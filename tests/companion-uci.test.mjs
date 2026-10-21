import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { once } from 'node:events';
import WebSocket from 'ws';

const origin = 'http://localhost:8788';
const fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const token = 'b'.repeat(64);
const binary = process.env.COMPANION_BINARY || resolve(`target/debug/ironwood-server${process.platform === 'win32' ? '.exe' : ''}`);

test('UCI isolation, cancellation, option validation, crash/timeouts and process cleanup', { timeout: 30000 }, async () => {
  const root = await mkdtemp(resolve('tmp/companion-uci-'));
  const engines = ['partial', 'crash', 'no-handshake', 'ignore-stop', 'illegal', 'long-line', 'stderr'].map(mode => ({
    id: mode, label: `Test ${mode}`, executable: process.execPath,
    args: [resolve('tests/fixtures/companion-fake-engine.mjs'), mode, join(root, `${mode}.log`), join(root, `${mode}.pid`)],
  }));
  await writeFile(join(root, 'owner-token'), token);
  await writeFile(join(root, 'config.json'), JSON.stringify({ listen:'127.0.0.1:0', allowed_origins:[origin], token_file:'owner-token', engines, tablebase_directories:[root],
    engine_limits:{ max_processes:2, handshake_ms:300, stop_ms:150, max_search_ms:2000, max_threads:4, max_hash_mib:128 } }));
  const child = spawn(binary, ['--config',join(root,'config.json')], { windowsHide:true, stdio:['ignore','pipe','pipe'] });
  const clients = [];
  try {
    const address = await new Promise((yes,no) => {
      let data = ''; const timer = setTimeout(()=>no(new Error('Server startup timeout')),5000);
      child.once('error',no); child.stdout.on('data',part=>{ data+=part; const match=data.match(/"address":"(127\.0\.0\.1:\d+)"/); if(match){clearTimeout(timer);yes(`http://${match[1]}`);} });
    });
    const request = (path, method='GET') => fetch(address+path, {method,headers:{Origin:origin,Authorization:`Bearer ${token}`,'X-Ironwood-Protocol':'1'}});
    assert.equal((await (await request('/api/v1/capabilities')).json()).services.uci,true);
    const catalog=await (await request('/api/v1/engines')).json(); assert.equal(catalog.length,engines.length); assert.ok(!JSON.stringify(catalog).includes(process.execPath));
    async function connect() {
      const {ticket}=await (await request('/api/v1/tickets','POST')).json();
      const socket=new WebSocket(address.replace('http:','ws:')+'/api/v1/socket',{origin}); socket.on('error',()=>{}); clients.push(socket);
      const messages=[]; const waiters=[];
      socket.on('message',data=>{ const m=JSON.parse(data); const index=waiters.findIndex(w=>w.predicate(m)); if(index>=0){const [w]=waiters.splice(index,1);clearTimeout(w.timer);w.yes(m);}else messages.push(m); });
      const wait=predicate=>new Promise((yes,no)=>{ const index=messages.findIndex(predicate);if(index>=0){yes(messages.splice(index,1)[0]);return;}const waiter={predicate,yes,timer:setTimeout(()=>{const i=waiters.indexOf(waiter);if(i>=0)waiters.splice(i,1);no(new Error('Expected engine event did not arrive'));},4000)};waiters.push(waiter); });
      await once(socket,'open'); const send=m=>socket.send(JSON.stringify(m)); send({type:'authenticate',ticket});await wait(m=>m.type==='ready');
      return {socket,send,wait,messages};
    }
    const a=await connect(),b=await connect();
    a.send({type:'open_engine',request_id:'open-a',engine_id:'partial'});
    const opened=await a.wait(m=>m.type==='engine_opened'); assert.equal(opened.engine.name,'Ironwood Test Engine');
    const session=opened.session_id;
    assert.ok((await readFile(join(root,'partial.log'),'utf8')).includes(`setoption name SyzygyPath value ${root}`));
    b.send({type:'open_engine',request_id:'open-b',engine_id:'partial'}); const second=await b.wait(m=>m.type==='engine_opened');assert.notEqual(second.session_id,session);
    const c=await connect(); c.send({type:'open_engine',request_id:'limit',engine_id:'partial'}); assert.match((await c.wait(m=>m.type==='error')).message,/limit/);
    const search=(id,options={},limit={kind:'infinite'})=>({type:'search',session_id:session,search_id:id,position_id:`pos-${id}`,fen,moves:[],options,limit});
    for(const options of [{Threads:'99'},{Missing:'1'},{EvalFile:'C:/private/file\nquit'}]) {
      a.send(search(`bad-${Object.keys(options)[0]}`,options));assert.equal((await a.wait(m=>m.type==='error')).code,'invalid_request');
    }
    a.send({type:'set_option',session_id:session,request_id:'clear-hash',name:'Clear Hash',value:''});
    assert.equal((await a.wait(m=>m.type==='option_set')).request_id,'clear-hash');
    a.send(search('first',{Threads:'2','Playing Style':'Very Aggressive',BackendOptions:'gpu=0'}));await a.wait(m=>m.type==='search_started');
    a.send({type:'set_option',session_id:session,request_id:'busy-action',name:'Clear Hash',value:''});
    assert.match((await a.wait(m=>m.type==='error')).message,/Stop the current search/);
    const info=await a.wait(m=>m.type==='info');assert.equal(info.info.score.perspective,'side_to_move');assert.equal(info.search_id,'first');
    a.send(search('overlapping'));assert.equal((await a.wait(m=>m.type==='error')).code,'busy');
    a.send({type:'cancel',session_id:session,search_id:'first'});await a.wait(m=>m.type==='cancelled');
    a.send(search('next',{}, {kind:'time',milliseconds:200}));
    const best=await a.wait(m=>m.type==='bestmove');assert.equal(best.search_id,'next');assert.equal(best.position_id,'pos-next');assert.equal(best.bestmove,'e2e4');
    assert.ok(!a.messages.some(m=>m.type==='bestmove'&&m.search_id==='first'));
    for (const [id, terminal] of [
      ['mate','7k/6Q1/6K1/8/8/8/8/8 b - - 0 1'],
      ['stalemate','7k/5Q2/6K1/8/8/8/8/8 b - - 0 1']
    ]) {
      a.send({...search(id,{}, {kind:'time',milliseconds:1000}),fen:terminal});
      const result=await a.wait(m=>m.type==='bestmove');
      assert.equal(result.search_id,id);assert.equal(result.bestmove,null,'terminal engine placeholders are not playable moves');
    }
    a.send(search('after-terminal',{}, {kind:'time',milliseconds:1000}));
    assert.equal((await a.wait(m=>m.type==='bestmove')).bestmove,'e2e4','terminal completion leaves the session usable');
    b.send({...search('isolated',{}, {kind:'nodes',nodes:100}),session_id:second.session_id,fen:fen.replace(' w ',' b ')});
    assert.equal((await b.wait(m=>m.type==='bestmove')).bestmove,'e7e5');
    a.send({type:'close_engine',session_id:session}); await a.wait(m=>m.type==='engine_closed');
    b.send({type:'close_engine',session_id:second.session_id});await b.wait(m=>m.type==='engine_closed');
    const commands=await readFile(join(root,'partial.log'),'utf8');assert.ok(!commands.includes('Threads value 99'));assert.ok(!commands.includes('Missing'));assert.ok(!commands.includes('private/file'));
    assert.ok(commands.includes('setoption name BackendOptions value gpu=0'));
    assert.equal(commands.split('\n').filter(line=>line==='setoption name Clear Hash').length,1);
    for(const mode of ['crash','no-handshake','ignore-stop','illegal','long-line','stderr']) {
      c.send({type:'open_engine',request_id:`open-${mode}`,engine_id:mode});
      if(mode==='no-handshake') await c.wait(m=>m.type==='error'&&m.code==='engine_failed');
      else {
        const event=await c.wait(m=>m.type==='engine_opened');
        c.send({...search(`search-${mode}`,{}, {kind:mode==='ignore-stop'?'infinite':'time',milliseconds:100}),session_id:event.session_id});
        await c.wait(m=>m.type==='search_started');
        if(mode==='ignore-stop') c.send({type:'cancel',session_id:event.session_id,search_id:`search-${mode}`});
        if(mode==='stderr') {await c.wait(m=>m.type==='bestmove');c.send({type:'close_engine',session_id:event.session_id});await c.wait(m=>m.type==='engine_closed');}
        else await c.wait(m=>m.type==='error'&&m.code==='engine_failed');
      }
      // An engine_failed event precedes cleanup; opening retries are not automatic.
      await new Promise(resolve=>setTimeout(resolve,350));
      const pid=Number(await readFile(join(root,`${mode}.pid`),'utf8'));
      assert.throws(()=>process.kill(pid,0),`Leaked ${mode} engine process`);
    }
    c.send({type:'open_engine',request_id:'disconnect',engine_id:'partial'});
    const disconnect=await c.wait(m=>m.type==='engine_opened');
    c.send({...search('disconnect-active'),session_id:disconnect.session_id});await c.wait(m=>m.type==='search_started');
    const disconnectedPid=Number(await readFile(join(root,'partial.pid'),'utf8'));c.socket.close();
    await new Promise(resolve=>setTimeout(resolve,400));assert.throws(()=>process.kill(disconnectedPid,0),'Leaked engine on disconnect');
    a.send({type:'open_engine',request_id:'shutdown',engine_id:'partial'});
    const shutdown=await a.wait(m=>m.type==='engine_opened');
    a.send({...search('shutdown-active'),session_id:shutdown.session_id});await a.wait(m=>m.type==='search_started');
    const shutdownPid=Number(await readFile(join(root,'partial.pid'),'utf8'));
    const exited=once(child,'exit');await request('/api/v1/shutdown','POST');assert.equal((await exited)[0],0);
    assert.throws(()=>process.kill(shutdownPid,0),'Leaked engine on graceful shutdown');
  } finally {
    for(const socket of clients)socket.terminate();
    if(child.exitCode===null&&child.signalCode===null){const exited=once(child,'exit');child.kill();await exited;}
    await rm(root,{recursive:true,force:true});
  }
});
