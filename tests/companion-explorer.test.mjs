import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { once } from 'node:events';
const binary=process.env.COMPANION_BINARY||resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`),origin='http://localhost:8788',token='d'.repeat(64);
const fen='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
test('background opening index remains consistent across archive edits, rebuild, restart and JSON restore',{timeout:30000},async()=>{
  const root=await mkdtemp(resolve('tmp/companion-explorer-'));const children=[];
  async function start(database='archive/archive.sqlite3'){
    await writeFile(join(root,'owner-token'),token);await writeFile(join(root,'config.json'),JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token',archive:database}));
    const child=spawn(binary,['--config',join(root,'config.json')],{windowsHide:true,stdio:['ignore','pipe','pipe']});children.push(child);let output='';child.stdout.on('data',part=>{output+=part;});
    const address=await new Promise((yes,no)=>{const timer=setTimeout(()=>no(new Error('Explorer startup timeout')),5000);child.once('error',no);child.once('exit',()=>{clearTimeout(timer);no(new Error(output));});child.stdout.on('data',()=>{const m=output.match(/"address":"(127\.0\.0\.1:\d+)"/);if(m){clearTimeout(timer);yes(`http://${m[1]}`);}});});
    const request=async(path,body,credential=token)=>{const response=await fetch(address+'/api/v1/'+path,{method:body===undefined?'GET':'POST',headers:{Origin:origin,Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1','Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});return {status:response.status,value:await response.json()};};
    const settled=async()=>{for(let i=0;i<100;i++){const progress=(await request('archive/explorer/status')).value;if(progress.pending===0)return progress;await new Promise(resolve=>setTimeout(resolve,30));}throw new Error('Index failed to finish');};
    const stop=async()=>{const closed=once(child,'close');await request('shutdown');const r=await fetch(address+'/api/v1/shutdown',{method:'POST',headers:{Origin:origin,Authorization:`Bearer ${token}`,'X-Ironwood-Protocol':'1'}});assert.equal(r.status,202);await closed;};
    return {request,settled,stop};
  }
  try{
    let server=await start();assert.equal((await server.request('capabilities')).value.services.explorer,true);
    const path='archive/explorer?'+new URLSearchParams({fen});assert.equal((await server.request(path,undefined,'wrong')).status,401);
    const pgn='[White "Ada"]\n[Black "Bob"]\n[WhiteElo "1800"]\n[Date "2026.10.17"]\n\n1. e4 e5 1-0\n\n[White "Other"]\n\n1. e4 c5 *';
    const imported=(await server.request('archive/import',{pgn})).value;assert.equal(imported.imported,2);await server.settled();const initial=(await server.request(path)).value;
    assert.equal(initial.total,2);assert.equal(initial.moves[0].total,2);assert.equal(initial.moves[0].unknown,1);assert.equal(initial.moves[0].percentages.white,100);assert.equal(initial.missing_metadata.white_elo,1);
    const filtered=(await server.request(path+'&player=Ada&color=white&white_elo_min=1750')).value;assert.equal(filtered.total,1);assert.equal(filtered.recentGames[0].id,imported.ids[0]);
    const game=(await server.request(`archive/games/${imported.ids[0]}`)).value;game.document.metadata.Result='0-1';await server.request(`archive/games/${game.id}/update`,{revision:game.revision,document:game.document});await server.settled();const changed=(await server.request(path)).value;assert.equal(changed.black,1);assert.ok(changed.generation>initial.generation);
    const queued=await server.request('archive/explorer/rebuild',{});assert.equal(queued.value.pending,2);assert.equal((await server.request('archive/games')).value.games.length,2);await server.settled();assert.deepEqual((await server.request(path)).value.moves,changed.moves);
    const backup=(await server.request('archive/backup')).value;await server.stop();server=await start();await server.settled();assert.deepEqual((await server.request(path)).value.moves,changed.moves);await server.stop();
    server=await start('restored.sqlite3');assert.equal((await server.request('archive/restore',backup)).value.restored,2);await server.settled();assert.deepEqual((await server.request(path)).value.moves,changed.moves);
    await server.request(`archive/games/${game.id}/delete`,{revision:2});assert.equal((await server.request(path)).value.total,1);await server.stop();
  }finally{for(const child of children)if(child.exitCode===null&&child.signalCode===null){const closed=once(child,'close');child.kill();await closed;}await rm(root,{recursive:true,force:true});}
});
