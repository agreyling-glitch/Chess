import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm,mkdir,copyFile,chmod,symlink} from 'node:fs/promises';
import {resolve,join,dirname} from 'node:path';
import {once} from 'node:events';
import WebSocket from 'ws';
const binary=process.env.COMPANION_BINARY||resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);
const origin='http://localhost:8788',token='c'.repeat(64);
const sleep=ms=>new Promise(r=>setTimeout(r,ms));

test('browse, automatic UCI validation and explicit Save persist live without terminal approval', {timeout:45000},async()=>{
 const root=await mkdtemp(resolve('tmp/companion-registration-'));
 const configPath=join(root,'config.json'),engines=join(root,'engines');
 await mkdir(engines);await mkdir(join(engines,'subfolder'));
 const selected=join(engines,process.platform==='win32'?'fixture.exe':'fixture');
 await copyFile(process.execPath,selected);await chmod(selected,0o755);await writeFile(join(engines,'readme.txt'),'not an engine');
 await symlink(dirname(root),join(engines,'outside-link'),process.platform==='win32'?'junction':'dir');
 await writeFile(join(root,'owner-token'),token);
 await writeFile(configPath,JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token',archive:null,engine_limits:{handshake_ms:500,stop_ms:150}}));
 let child,address;
 async function start(mounted=false){
  const env={...process.env};delete env.IRONWOOD_ENGINE_DIRECTORY;if(mounted)env.IRONWOOD_ENGINE_DIRECTORY=engines;
  child=spawn(binary,['--config',configPath],{stdio:['ignore','pipe','pipe'],windowsHide:true,env});
  address=await new Promise((yes,no)=>{let log='';const t=setTimeout(()=>no(Error('startup timeout')),5000);child.once('error',no);child.stdout.on('data',data=>{log+=data;const m=log.match(/"address":"(127\.0\.0\.1:\d+)"/);if(m){clearTimeout(t);yes(`http://${m[1]}`);}});});
 }
 const request=(path,body,auth=true)=>fetch(`${address}/api/v1/${path}`,{method:body===undefined&&!['shutdown','tickets'].includes(path)?'GET':'POST',headers:{Origin:origin,Authorization:`Bearer ${auth?token:'bad'}`,'X-Ironwood-Protocol':'1',...(body===undefined?{}:{'Content-Type':'application/json'})},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(5000)});
 async function propose(mode='partial',executable=selected){
  const response=await request('engine-registrations',{executable,args:[resolve('tests/fixtures/companion-fake-engine.mjs'),mode,join(root,`${mode}.log`),join(root,`${mode}.marker`)]});
  assert.equal(response.status,202);return response.json();
 }
 async function done(id){for(let i=0;i<80;i++){const value=await(await request(`engine-registrations/${id}`)).json();if(['validated','failed','cancelled'].includes(value.status))return value;await sleep(50);}throw Error('validation timeout');}
 async function stop(){const exited=once(child,'exit');assert.equal((await request('shutdown')).status,202);assert.equal((await exited)[0],0);}
 try{
  await start();
  assert.equal((await request('engine-files',undefined,false)).status,401);
  assert.equal((await request('engine-registrations',{executable:selected},false)).status,401);
  assert.equal((await request('engine-registrations',{executable:'relative.exe'})).status,400);
  const listing=await(await request(`engine-files?path=${encodeURIComponent(engines)}`)).json();
  assert.equal(listing.parent,null);assert.ok(listing.folder.endsWith('engines'));
  assert.equal((await request(`engine-files?path=${encodeURIComponent(root)}`)).status,403);
  assert.equal((await request('engine-registrations',{executable:process.execPath})).status,400);
  assert.ok(listing.entries.some(e=>e.name==='subfolder'&&e.directory));
  assert.ok(listing.entries.some(e=>e.name=== (process.platform==='win32'?'fixture.exe':'fixture')&&!e.directory));
  assert.ok(!listing.entries.some(e=>e.name==='readme.txt'));
  const first=await propose('cwd');assert.equal(first.status,'validating');
  assert.equal((await request(`engine-registrations/${first.request}/save`,{},false)).status,401);
  const result=await done(first.request);assert.equal(result.status,'validated');assert.equal(result.name,'Ironwood Test Engine');
  assert.equal((await readFile(join(root,'cwd.marker'),'utf8')).toLowerCase(),engines.toLowerCase());
  assert.deepEqual(await(await request('engines')).json(),[]);
  assert.equal(JSON.parse(await readFile(configPath,'utf8')).engines,undefined,'validation alone must not save');
  assert.equal((await request(`engine-registrations/${first.request}/save`,{label:'Friendly engine'})).status,200);
  assert.equal((await request(`engine-registrations/${first.request}/save`,{})).status,409,'no duplicate saves');
  assert.equal((await(await request('capabilities')).json()).services.uci,true);
  const catalog=await(await request('engines')).json();assert.equal(catalog[0].label,'Friendly engine');
  const duplicate=await request('engine-registrations',{executable:selected,args:[resolve('tests/fixtures/companion-fake-engine.mjs'),'cwd',join(root,'cwd.log'),join(root,'cwd.marker')],label:'Another name'});
  assert.equal(duplicate.status,409,'same executable and arguments cannot be registered under another name');
  assert.equal((await duplicate.json()).code,'already_registered');
  assert.equal((await(await request('engines')).json()).length,1);
  const {ticket}=await(await request('tickets')).json();
  const socket=new WebSocket(address.replace('http:','ws:')+'/api/v1/socket',{origin});
  await once(socket,'open');let received=once(socket,'message');socket.send(JSON.stringify({type:'authenticate',ticket}));await received;
  received=once(socket,'message');socket.send(JSON.stringify({type:'open_engine',request_id:'new-registration',engine_id:catalog[0].id}));
  const opened=JSON.parse((await received)[0]);assert.equal(opened.type,'engine_opened');assert.equal(opened.engine.id,catalog[0].id);
  received=once(socket,'message');socket.send(JSON.stringify({type:'close_engine',session_id:opened.session_id}));await received;socket.close();await once(socket,'close');
  assert.equal(JSON.parse(await readFile(configPath,'utf8')).engines.length,1);
  await stop();await start();assert.equal((await(await request('engines')).json()).length,1);
  const bad=await propose('no-handshake');assert.equal((await done(bad.request)).status,'failed');assert.equal((await request(`engine-registrations/${bad.request}/save`,{})).status,409);
  const cancelled=await propose();await done(cancelled.request);await request(`engine-registrations/${cancelled.request}/cancel`,{});assert.equal((await done(cancelled.request)).status,'cancelled');assert.equal((await request(`engine-registrations/${cancelled.request}/save`,{})).status,409);
  const changed=await propose();await done(changed.request);
  const edited=JSON.parse(await readFile(configPath,'utf8'));edited.allowed_origins.push('http://localhost:9999');await writeFile(configPath,JSON.stringify(edited));
  const conflict=await request(`engine-registrations/${changed.request}/save`,{});assert.equal(conflict.status,409);assert.match((await conflict.json()).message,/Configuration changed/);assert.deepEqual(JSON.parse(await readFile(configPath,'utf8')),edited);
  await stop();await start(true);
  const mounted=await(await request('engine-files')).json();assert.ok(mounted.folder.endsWith('engines'));assert.equal(mounted.parent,null);assert.ok(!mounted.entries.some(e=>e.name==='outside-link'));
  assert.equal((await request(`engine-files?path=${encodeURIComponent(root)}`)).status,403);
  assert.equal((await request(`engine-files?path=${encodeURIComponent(join(engines,'..'))}`)).status,403);
  assert.equal((await request('engine-registrations',{executable:process.execPath})).status,400);
  const inside=await propose();assert.equal((await done(inside.request)).status,'validated');assert.equal((await request(`engine-registrations/${inside.request}/save`,{})).status,200);
  assert.equal(JSON.parse(await readFile(configPath,'utf8')).engines.length,2);
  const pending=await propose('no-handshake');await stop();
  await start(true);assert.equal((await request(`engine-registrations/${pending.request}`)).status,404);await stop();
 }finally{if(child?.exitCode===null){child.kill();await once(child,'exit');}await rm(root,{recursive:true,force:true});}
});
