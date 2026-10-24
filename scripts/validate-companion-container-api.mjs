import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import WebSocket from 'ws';
const mode=process.argv[2]||'exercise',data=process.env.COMPANION_TEST_DATA||'/data';
if(mode==='seed'){
 const file=data+'/config.json',config=JSON.parse(await readFile(file,'utf8'));
 config.engines=[{id:'fixture',label:'Original container fixture',executable:'/bin/sh',args:[data+'/test-engine.sh']}];
 config.tablebase_directories=[data+'/tables'];await mkdir(data+'/tables',{recursive:true});
 await writeFile(data+'/test-engine.sh',`#!/bin/sh
while IFS= read -r line; do
 case "$line" in
 uci) printf '%s\\n' 'id name Ironwood Container Fixture' 'option name SyzygyPath type string default <empty>' 'uciok';;
 isready) printf '%s\\n' 'readyok';;
 'go '*) printf '%s\\n' 'info depth 1 score cp 12 pv e2e4' 'bestmove e2e4';;
 quit) exit 0;;
 esac
done
`);await writeFile(file,JSON.stringify(config));console.log('Seeded original container fixture.');
}else{
 const address=process.env.COMPANION_ADDRESS||'http://companion:8790',token=(await readFile(data+'/owner-token','utf8')).trim();
 const checks=[],report={format:'ironwood-production-container-api',version:1,mode,status:'running',checks};
 const request=async(path,body,credential=token,origin='http://localhost:8788',method=body===undefined?'GET':'POST')=>{
  const r=await fetch(address+'/api/v1/'+path,{method,headers:{Origin:origin,Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1',...(body===undefined?{}:{'Content-Type':'application/json'})},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(10000)});
  return {status:r.status,value:await r.json()};
 };
 try{
  assert.equal((await request('capabilities',undefined,'wrong')).status,401);
  assert.equal((await request('capabilities',undefined,token,'https://invalid.example')).status,403);
  const caps=(await request('capabilities')).value;for(const name of ['uci','archive','explorer','tablebases'])assert.equal(caps.services[name],true);
  checks.push('authentication-origin-capabilities');
  if(mode==='exercise'){
   assert.equal((await request('archive/import',{pgn:'[White "Container Validation"]\n[Black "Fixture"]\n\n1. e4 e5 2. Nf3 Nc6 *'})).value.imported,1);
  }
  const games=(await request('archive/games?white=Container%20Validation')).value.games;assert.equal(games.length,1);
  assert.match((await request(`archive/games/${games[0].id}`)).value.document.pgn,/1\. e4 (?:1\.\.\. )?e5/);
  let indexed=false;for(let i=0;i<100;i++){if((await request('archive/explorer/status')).value.pending===0){indexed=true;break;}await new Promise(r=>setTimeout(r,50));}assert.ok(indexed);
  const fen='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
  assert.equal((await request('archive/explorer?'+new URLSearchParams({fen}))).value.total,1);
  const draw='8/8/8/8/8/2k5/8/K7 w - - 99 1';assert.equal((await request('tablebases/probe?'+new URLSearchParams({fen:draw}))).value.category,'draw');
  checks.push(mode==='exercise'?'archive-explorer-local-probe':'persistent-archive-explorer-after-container-replacement');
  if(mode==='exercise'){
   const {ticket}=(await request('tickets',undefined,token,'http://localhost:8788','POST')).value;
   const socket=new WebSocket(address.replace('http:','ws:')+'/api/v1/socket',{origin:'http://localhost:8788'}),pending=[],messages=[];
   socket.on('error',()=>{});socket.on('message',raw=>{const value=JSON.parse(raw);const i=pending.findIndex(p=>p.type===value.type||value.type==='error');if(i>=0){const p=pending.splice(i,1)[0];clearTimeout(p.timer);value.type==='error'?p.reject(Error('Engine protocol error')):p.resolve(value);}else messages.push(value);});
   const wait=type=>new Promise((resolve,reject)=>{const i=messages.findIndex(m=>m.type===type);if(i>=0)return resolve(messages.splice(i,1)[0]);const p={type,resolve,reject,timer:setTimeout(()=>{pending.splice(pending.indexOf(p),1);reject(Error('Missing protocol event '+type));},10000)};pending.push(p);});
   try{
    await new Promise((yes,no)=>{socket.once('open',yes);socket.once('error',no)});
    socket.send(JSON.stringify({type:'authenticate',ticket}));await wait('ready');
    socket.send(JSON.stringify({type:'open_engine',request_id:'open',engine_id:'fixture'}));const {session_id}=await wait('engine_opened');
    socket.send(JSON.stringify({type:'search',session_id,search_id:'search',position_id:'initial',fen,limit:{kind:'depth',plies:1}}));assert.equal((await wait('bestmove')).bestmove,'e2e4');
    socket.send(JSON.stringify({type:'close_engine',session_id}));await wait('engine_closed');checks.push('uci-handshake-search-close');
   }finally{socket.terminate();for(const p of pending)clearTimeout(p.timer);}
  }
  report.status='passed';
 }catch(error){report.status='failed';report.error=error.message;process.exitCode=1;}
 await mkdir('tmp',{recursive:true});await writeFile(`tmp/container-${mode}-report.json`,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
}
