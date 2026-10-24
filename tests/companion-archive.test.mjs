import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, writeFile, readFile, rm, mkdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { once } from 'node:events';
import net from 'node:net';
const binary=process.env.COMPANION_BINARY||resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);
const token='c'.repeat(64),origin='http://localhost:8788',cli=promisify(execFile);
const pgn='[White "Ada"]\n[Black "Grace"]\n[Date "2026.10.17"]\n[ECO "C20"]\n[Result "*"]\n\n1. e4 {a note [%cal Ge2e4]} (1. d4 d5) e5 $1 *';
async function start(root){
  const child=spawn(binary,['--config',join(root,'config.json')],{windowsHide:true,stdio:['ignore','pipe','pipe']});let output='';child.stdout.on('data',p=>{output+=p;});child.stderr.on('data',p=>{output+=p;});
  const address=await new Promise((yes,no)=>{const timer=setTimeout(()=>no(new Error(`Startup timeout: ${output}`)),8000);child.once('error',e=>{clearTimeout(timer);no(e);});child.once('exit',()=>{clearTimeout(timer);no(new Error(`Startup failed: ${output}`));});child.stdout.on('data',()=>{const match=output.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes(`http://${match[1]}`);}});});
  const request=async(path,body,credential=token)=>{const response=await fetch(address+'/api/v1/'+path,{method:body===undefined?'GET':'POST',headers:{Origin:origin,Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1','Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});return {status:response.status,value:await response.json()};};
  const stop=async()=>{const closed=once(child,'close');assert.equal((await request('shutdown',{})).status,400);const response=await fetch(address+'/api/v1/shutdown',{method:'POST',headers:{Origin:origin,Authorization:`Bearer ${token}`,'X-Ironwood-Protocol':'1'}});assert.equal(response.status,202);assert.equal((await closed)[0],0);};
  return {child,address,request,stop};
}
test('persistent archive HTTP round trips, transactions, conflicts, collections, search and recovery',{timeout:40000},async()=>{
  const root=await mkdtemp(resolve('tmp/companion-archive-http-'));const children=[];
  async function directory(name){const dir=join(root,name);await mkdir(dir);await writeFile(join(dir,'owner-token'),token);await writeFile(join(dir,'config.json'),JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token'}));return dir;}
  try{
    const dir=await directory('owner');let server=await start(dir);children.push(server.child);
    assert.equal((await server.request('capabilities')).value.services.archive,true);
    assert.equal((await server.request('archive/games',undefined,'wrong')).status,401);
    const record={id:'browser-original',json:JSON.stringify({board:'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 17 42',history:[],move_notes:['private note'],board_marks:[[{color:'Gold',from:'e2',to:'e4'}]],game_analysis:[{score_cp:23,pv:['e2e4']}],engine_config:{opponent:'Companion',companion_name:'My engine'},training:{profile_id:'profile-one',id:'session-one'}}),category:'training',favorite:true,source:'original source',updatedAt:1234,custom:{retained:true}};
    const document={format:'ironwood-companion-game',version:1,record,pgn,metadata:{White:'Ada',Black:'Grace',Date:'2026.10.17',Result:'*',ECO:'C20',TrainingProfileId:'profile-one'}};
    const group=(await server.request('archive/collections',{action:'create',name:'Research'})).value;const collection=group.id;
    const imported=await server.request('archive/import',{documents:[document],collection});assert.equal(imported.status,200);assert.equal(imported.value.imported,1);const id=imported.value.ids[0];
    assert.equal((await server.request('archive/import',{documents:[document]})).value.duplicates,1);
    assert.deepEqual((await server.request(`archive/games/${id}`)).value.document,document);
    const queries=['white=ad','black=grace','date_from=2026.10.01&date_to=2026.10.31','result=*','opening=C20','profile=profile-one',`collection=${collection}`];
    for(const query of queries)assert.equal((await server.request(`archive/games?${query}`)).value.games.length,1,query);
    assert.equal((await server.request('archive/games?white=missing')).value.games.length,0);
    assert.equal((await server.request('archive/games?white=%27%20OR%201%3D1--')).value.games.length,0);
    assert.equal((await server.request('archive/import',{documents:[{...document,metadata:{White:'new'}},{...document,version:999}]})).status,400);
    assert.equal((await server.request('archive/import',{pgn:'1. d4 *\n\n1. e5 *'})).status,400);
    // A disconnected, incomplete request must not import even its first game.
    const socket=net.connect(new URL(server.address).port,'127.0.0.1');await once(socket,'connect');socket.write(`POST /api/v1/archive/import HTTP/1.1\r\nHost: localhost\r\nOrigin: ${origin}\r\nAuthorization: Bearer ${token}\r\nX-Ironwood-Protocol: 1\r\nContent-Type: application/json\r\nContent-Length: 9999\r\n\r\n{"pgn":"1. d4 *`);socket.destroy();
    assert.equal((await server.request('archive/games')).value.games.length,1);
    const runPath=`archive/games/${id}/analysis-runs`;
    for(const [run,engine,score]of [['sf','Stockfish 19',21],['lc0','Lc0 v0.32.1',82]]){
      const body={expected_revision:0,metadata:{engine_name:engine,settings_label:'1 second per position'},payload:{game_analysis:[{eval_cp:score}]}};
      assert.equal((await server.request(`${runPath}/${run}`,body,'wrong')).status,401);
      assert.equal((await server.request(`${runPath}/${run}`,body)).status,200);
      assert.equal((await server.request(`${runPath}/${run}`,body)).status,409);
    }
    assert.equal((await server.request(`archive/games/${id}/delete`,{revision:1,analysis_runs:[['sf',1]]})).status,409);
    const catalog=(await server.request(runPath)).value;assert.equal(catalog.runs.length,2);assert.ok(catalog.runs.every(run=>!('payload' in run)));
    assert.equal((await server.request(`${runPath}/lc0`)).value.payload.game_analysis[0].eval_cp,82);
    assert.equal((await server.request(`${runPath}/sf`)).value.payload.game_analysis[0].eval_cp,21);
    const edited={...document,metadata:{...document.metadata,Opening:'King pawn'}};
    const writes=await Promise.all([server.request(`archive/games/${id}/update`,{revision:1,document:edited}),server.request(`archive/games/${id}/update`,{revision:1,document:edited})]);assert.deepEqual(writes.map(r=>r.status).sort(),[200,409]);
    assert.equal((await server.request(`archive/games/${id}/delete`,{revision:1})).status,409);
    const version=(await server.request(`archive/games/${id}`)).value;assert.equal(version.revision,2);
    const pg=(await server.request('archive/export',{ids:[id],format:'pgn'})).value.pgn;assert.equal(pg,pgn);
    assert.deepEqual((await server.request('archive/export',{ids:[id],format:'json'})).value.documents[0].record,record);
    await server.stop();server=await start(dir);children.push(server.child);assert.deepEqual((await server.request(`archive/games/${id}`)).value,version);
    const backup=(await server.request('archive/backup')).value;assert.equal(backup.schema_version,3);assert.equal(backup.games[0].document.analysis_runs.length,2);
    assert.equal((await server.request('archive/restore',backup)).status,409);
    const restored=await start(await directory('restored'));children.push(restored.child);assert.equal((await restored.request('archive/restore',{...backup,schema_version:999})).status,400);assert.equal((await restored.request('archive/restore',backup)).value.restored,1);assert.deepEqual((await restored.request('archive/backup')).value,backup);
    assert.equal((await server.request(`${runPath}/delete`,{runs:[['lc0',1]]},'wrong')).status,401);
    assert.equal((await server.request(`${runPath}/delete`,{runs:[['lc0',1],['sf',99]]})).status,409);
    assert.equal((await server.request(runPath)).value.runs.length,2);
    assert.equal((await server.request(`${runPath}/delete`,{runs:[['lc0',1]]})).status,200);
    assert.equal((await server.request(`${runPath}/lc0`)).status,404);
    assert.equal((await server.request(`${runPath}/sf`)).value.payload.game_analysis[0].eval_cp,21);
    assert.deepEqual((await server.request(`archive/games/${id}`)).value,version);
    const collectionRevision=(await server.request('archive/collections')).value[0].revision;
    assert.equal((await server.request(`archive/collections/${collection}`,{action:'delete',revision:collectionRevision})).status,200);assert.equal((await server.request('archive/games')).value.games.length,1);
    assert.equal((await server.request(`archive/games/${id}/delete`,{revision:2})).status,200);assert.equal((await server.request('archive/games')).value.games.length,0);
    const batch=await server.request('archive/import',{pgn:'[White "First"]\n\n1. d4 d5 2. c4 e6 *\n\n[White "Second"]\n\n1. c4 e5 2. Nc3 Nf6 *\n\n[White "Short"]\n\n1. e4 e5 *'});assert.equal(batch.value.imported,2);assert.equal(batch.value.discarded,1);const first=(await server.request('archive/games?limit=1')).value;assert.equal(first.games.length,1);assert.ok(first.next);const second=(await server.request(`archive/games?limit=1&after=${first.next}`)).value;assert.notEqual(first.games[0].id,second.games[0].id);assert.equal(second.next,null);
    await server.stop();await restored.stop();
    const snapshot=join(root,'snapshot.sqlite3');await cli(binary,['archive-backup','--data-dir',join(root,'restored'),'--output',snapshot],{windowsHide:true});
    const nativeDir=join(root,'native-restore');await cli(binary,['archive-restore','--data-dir',nativeDir,'--input',snapshot],{windowsHide:true});
    const restoredConfig=JSON.parse(await readFile(join(nativeDir,'config.json')));restoredConfig.listen='127.0.0.1:0';await writeFile(join(nativeDir,'config.json'),JSON.stringify(restoredConfig));await writeFile(join(nativeDir,'owner-token'),token);
    const native=await start(nativeDir);children.push(native.child);assert.deepEqual((await native.request('archive/backup')).value,backup);await native.stop();
  }finally{for(const child of children)if(child.exitCode===null&&child.signalCode===null){const closed=once(child,'close');child.kill();await closed;}await rm(root,{recursive:true,force:true});}
});

test('unified Saved games provider round-trips complete records through the real archive API',{timeout:15000},async()=>{
 const {ArchiveLibrary}=await import('../web/companion-library.js');
 const root=await mkdtemp(resolve('tmp/companion-library-http-'));let server;
 try {
  await writeFile(join(root,'owner-token'),token);await writeFile(join(root,'config.json'),JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token'}));
  server=await start(root);const endpoint=server.address,current={endpoint,credential:token},browser=[];
  const record={id:'browser-record',category:'imported',favorite:true,source:'original',updatedAt:123,json:JSON.stringify({board:'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',history:[],review_pgn:pgn,move_notes:['keep note'],game_analysis:[{score_cp:45}]}),custom:{keep:true}};
  const store=new ArchiveLibrary({endpoint,credential:token,request:async(path,body)=>{const response=await server.request(path,body);if(response.status!==200)throw Error(response.value.message);return response.value;}},{connection:()=>current,details:()=>({white:'Ada',black:'Grace',result:'*'}),exporter:()=>pgn,prepare:()=>record.json,receive:async documents=>{browser.push(...structuredClone(documents));return {received:documents.length,duplicates:0,ids:['restored-browser']};}});
  const [id]=await store.importRecords([record]);const rows=await store.list();assert.equal(rows.length,1);assert.equal(rows[0].json,record.json);assert.equal(rows[0].favorite,true);
  const stats=await store.stats();assert.equal(stats.games,1);assert.equal(stats.favorites,1);assert.equal(stats.categories.imported,1);assert.equal(stats.schema_version,3);assert.equal(stats.collections,0);assert.ok(stats.database_bytes>0);assert.equal(stats.total_bytes,stats.database_bytes+stats.wal_bytes+stats.shared_memory_bytes);assert.ok(stats.reusable_bytes<=stats.allocated_bytes);assert.ok(!JSON.stringify(stats).includes(root));assert.equal((await server.request('archive/stats',undefined,'wrong')).status,401);
  const updated=await store.write({...await store.read(id),favorite:false});assert.equal(updated._archive.revision,2);assert.equal(updated.category,'imported');
  await store.moveToBrowser([id]);assert.equal(browser[0].record.favorite,false);assert.equal(browser[0].record.json,record.json);assert.deepEqual(browser[0].record.custom,{keep:true});assert.equal((await store.list()).length,0);assert.equal((await store.stats()).games,0);
  await server.stop();
 }finally{if(server?.child.exitCode===null){const exited=once(server.child,'close');server.child.kill();await exited;}await rm(root,{recursive:true,force:true});}
});

test('durable analysis outbox synchronizes multiple offline runs with the real archive API',{timeout:15000},async()=>{
 const root=await mkdtemp(resolve('tmp/companion-offline-analysis-'));let server;
 const previous={window:globalThis.window,localStorage:globalThis.localStorage};
 try{
  await writeFile(join(root,'owner-token'),token);await writeFile(join(root,'config.json'),JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token'}));server=await start(root);
  const values=new Map(),entries=new Map();globalThis.window=globalThis;
  globalThis.localStorage={getItem:key=>values.get(key)||null,setItem:(key,value)=>values.set(key,value),removeItem:key=>values.delete(key)};
  window.ironwoodCompanionFeatureEnabled=()=>false;
  const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');
  const module=await import(`data:text/javascript,${encodeURIComponent(source+'\n// offline-http')}`);
  Object.assign(module.analysisLocalCache,{
   async get(id){return structuredClone(entries.get(id));},async list(index,value){return structuredClone([...entries.values()].filter(entry=>entry[index]===value));},
   async write(put=[],remove=[]){put.forEach(entry=>entries.set(entry.id,structuredClone(entry)));remove.forEach(id=>entries.delete(id));},
   async mutate(changes){for(const [id,change]of changes){const value=change(structuredClone(entries.get(id)));value?entries.set(id,structuredClone(value)):entries.delete(id);}}
  });
  const fen='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
  const original={board:fen,history:[],game_analysis:[],move_notes:['Original notes'],board_marks:[['e4']]};
  const document={format:'ironwood-companion-game',version:1,pgn:'*',record:{id:'offline-original',json:JSON.stringify(original),favorite:true},metadata:{}};
  const imported=await server.request('archive/import',{documents:[document]});assert.equal(imported.status,200);const id=imported.value.ids[0],active={endpoint:server.address,id,revision:1};
  const snapshots=[];
  for(const [engine,score]of [['Stockfish 19',21],['Lc0',88]]){
   const context=JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify({engine_name:engine,engine_identity:engine,settings_label:'1 second per position',position_fens:[fen],engine_options:{Threads:'2'}})));
   const saved={...original,analysis_run:context,game_analysis:[{eval_cp:score,depth:12,nodes:1000,pv:'e4'}]};snapshots.push(saved);await module.queueArchiveAnalysis(active,JSON.stringify(saved));
  }
  assert.equal((await server.request(`archive/games/${id}/analysis-runs`)).value.runs.length,0);
  const storage={endpoint:server.address,check(){},record:game=>({...game.document.record,id:game.id,_archive:{endpoint:server.address,id:game.id,revision:game.revision,document:game.document}}),
   async request(path,body){const result=await server.request(path,body);if(result.status!==200)throw Object.assign(Error(result.value.message),{status:result.status});return result.value;},
   async write(record){const document=structuredClone(record._archive.document);document.record={...document.record,json:record.json};return this.record(await this.request(`archive/games/${record.id}/update`,{revision:record._archive.revision,document}));}
  };
  const queue=await module.analysisLocalCache.get(`${server.address}/${id}/pending`);await module.syncArchiveAnalyses(storage,queue);
  const runs=(await server.request(`archive/games/${id}/analysis-runs`)).value.runs;assert.equal(runs.length,2);
  for(const saved of snapshots){const run=(await server.request(`archive/games/${id}/analysis-runs/${saved.analysis_run.id}`)).value;assert.equal(run.metadata.engine_name,saved.analysis_run.engine_name);assert.deepEqual(run.payload.game_analysis,saved.game_analysis);}
  const final=(await server.request(`archive/games/${id}`)).value;assert.equal(final.document.record.favorite,true);assert.equal(final.document.pgn,'*');assert.deepEqual(JSON.parse(final.document.record.json).move_notes,original.move_notes);assert.deepEqual(JSON.parse(final.document.record.json).board_marks,original.board_marks);
  assert.equal(await module.analysisLocalCache.get(queue.id),undefined);await server.stop();
 }finally{if(server?.child.exitCode===null){const closed=once(server.child,'close');server.child.kill();await closed;}globalThis.window=previous.window;globalThis.localStorage=previous.localStorage;await rm(root,{recursive:true,force:true});}
});
