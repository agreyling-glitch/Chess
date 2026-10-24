import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CompanionEngineAdapter } from '../web/companion-engine-adapter.js';
function fixture() {
  const lines=[],sent=[],errors=[];
  const adapter=new CompanionEngineAdapter(line=>lines.push(line),error=>errors.push(error));
  adapter.session='session';adapter.details={options:[{name:'Threads',kind:'spin',min:1,max:4},{name:'MultiPV',kind:'spin',min:1,max:4}]};
  adapter.socket={readyState:1,send:message=>sent.push(JSON.parse(message)),close:()=>{}};
  return {adapter,lines,sent,errors};
}

test('discovery invokes injected fetch without an adapter receiver',async()=>{
  let adapter;
  class Socket {
    readyState=1;
    constructor(){queueMicrotask(()=>this.onopen());}
    send(raw){const request=JSON.parse(raw);queueMicrotask(()=>this.onmessage({data:JSON.stringify(request.type==='authenticate'?{type:'ready',protocol_version:'1'}:{type:'engine_opened',session_id:'session',engine:{name:'Fixture',options:[]}})}));}
    close(){}
  }
  adapter=new CompanionEngineAdapter(()=>{},()=>{}, {Socket,fetcher:async function(){assert.equal(this,undefined);return {ok:true,json:async()=>({ticket:'test'})};}});
  try {assert.equal((await adapter.connect({endpoint:'http://127.0.0.1:8790',credential:'test',engineId:'fixture'})).name,'Fixture');}
  finally {adapter.close();}
});
test('typed search ignores browser resource defaults, filters unsupported options and preserves explicit UCI values',async()=>{
  const {adapter,sent}=fixture();
  await adapter.command('setoption name Threads value 8');
  await adapter.command('setoption name UCI_Elo value 1320');
  await adapter.command('setoption name Missing value 42');
  await adapter.command('position startpos moves e2e4');
  await adapter.command('go depth 12');
  assert.equal(sent[0].options.Threads,undefined);assert.equal(sent[0].options.UCI_Elo,undefined);assert.equal(sent[0].options.Missing,undefined);
  assert.deepEqual(sent[0].moves,['e2e4']);assert.deepEqual(sent[0].limit,{kind:'depth',plies:12});
  adapter.message({type:'bestmove',session_id:'session',search_id:'search-1',position_id:'position-1',bestmove:'e7e5'});
  adapter.overrides={Threads:'99'};await adapter.command('go movetime 100');assert.equal(sent[1].options.Threads,'99');adapter.close();
});
test('old identities and duplicate bestmoves are ignored; cancellation drains before readiness',async()=>{
  const {adapter,lines,sent}=fixture();
  await adapter.command('go infinite');
  const base={session_id:'session',search_id:'search-1',position_id:'position-1'};
  adapter.message({...base,type:'bestmove',position_id:'stale-position',bestmove:'e2e4'});assert.equal(lines.length,0);
  const stop=adapter.command('stop');const ready=adapter.command('isready');
  await Promise.resolve();
  adapter.message({...base,type:'info',info:{depth:10,score:{kind:'cp',value:400,perspective:'side_to_move'},pv:['e2e4']}});
  assert.equal(lines.length,0);
  adapter.message({...base,type:'cancelled'});await stop;await ready;
  assert.deepEqual(lines,['bestmove 0000','readyok']);assert.equal(sent[1].type,'cancel');
  await adapter.command('go movetime 50');
  adapter.message({...base,type:'bestmove',bestmove:'e2e4'});assert.equal(lines.length,2);
  const next={session_id:'session',search_id:'search-2',position_id:'position-2',type:'bestmove',bestmove:'e2e4'};
  adapter.message(next);adapter.message(next);assert.equal(lines.filter(line=>line==='bestmove e2e4').length,1);
  adapter.close();
});
test('disconnect fails closed and does not replay pending commands',async()=>{
  const {adapter,sent,errors}=fixture();await adapter.command('go infinite');adapter.fatal('connection lost');
  await adapter.command('go depth 8');assert.equal(sent.length,1);assert.equal(errors.length,1);assert.equal(adapter.active,null);
});

test('advertised string options and one-shot UCI actions use typed messages and timed searches',async()=>{
  const emitted=[],sent=[];let socket;
  class Socket {
    readyState=1;
    constructor(){socket=this;queueMicrotask(()=>this.onopen());}
    send(raw){const message=JSON.parse(raw);sent.push(message);queueMicrotask(()=>{
      if(message.type==='authenticate')this.onmessage({data:JSON.stringify({type:'ready',protocol_version:'1'})});
      if(message.type==='open_engine')this.onmessage({data:JSON.stringify({type:'engine_opened',session_id:'session',engine:{name:'Fixture',options:[{name:'BackendOptions',kind:'string',default:'',choices:[]},{name:'Clear Hash',kind:'button',default:'',choices:[]}]}})});
      if(message.type==='set_option')this.onmessage({data:JSON.stringify({type:'option_set',session_id:'session',request_id:message.request_id})});
    });}
    close(){}
  }
  const adapter=new CompanionEngineAdapter(line=>emitted.push(line),error=>assert.fail(error),{Socket,fetcher:async()=>({ok:true,json:async()=>({ticket:'test'})})});
  try {
    await adapter.connect({endpoint:'http://localhost:8790',credential:'test',engineId:'fixture'});
    assert.ok(emitted.includes('option name BackendOptions type string default <empty>'));
    await adapter.command('setoption name BackendOptions value gpu=0');
    await adapter.command('setoption name Clear Hash');
    await adapter.command('go movetime 5000');
    const action=sent.find(message=>message.type==='set_option'),search=sent.find(message=>message.type==='search');
    assert.equal(action.name,'Clear Hash');assert.equal(action.value,'');
    assert.deepEqual(search.limit,{kind:'time',milliseconds:5000});assert.equal(search.options.BackendOptions,'gpu=0');assert.equal(search.options['Clear Hash'],undefined);
  }finally{adapter.close();}
});


test('Maia Elo overrides reach each play search without browser Stockfish strength defaults',async()=>{
  const {adapter,sent}=fixture();
  adapter.details.options=[
    {name:'Elo',kind:'spin',min:0,max:5000},
    {name:'SelfElo',kind:'spin',min:0,max:5000},
    {name:'OppoElo',kind:'spin',min:0,max:5000},
    {name:'MultiPV',kind:'spin',min:1,max:20},
  ];
  adapter.overrides={Elo:'900',SelfElo:'900',OppoElo:'900'};
  await adapter.command('setoption name UCI_Elo value 1320');
  await adapter.command('setoption name UCI_LimitStrength value false');
  await adapter.command('setoption name MultiPV value 1');
  await adapter.command('position startpos');
  await adapter.command('go movetime 650');
  assert.deepEqual(sent[0].options,{MultiPV:'1',Elo:'900',SelfElo:'900',OppoElo:'900'});
  adapter.close();
});
