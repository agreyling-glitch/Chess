// Isolated local HTTP benchmark. Never opens the configured user archive.
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {spawn,spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
import {CompanionArchiveClient} from '../web/companion-archive-client.js';
import {ArchiveLibrary} from '../web/companion-library.js';
const count=Number(process.argv[2]||256),baselinePath=process.argv[3];
if(!baselinePath)throw Error('Usage: node scripts/benchmark-archive-import.mjs 256 <baseline-module-path>');
let baseline=await readFile(baselinePath,'utf8');
for(const name of ['companion-archive-client.js','companion-connection.js'])baseline=baseline.replaceAll(`'./${name}'`,JSON.stringify(pathToFileURL(resolve('web',name)).href));
const Before=(await import('data:text/javascript;base64,'+Buffer.from(baseline).toString('base64'))).ArchiveLibrary;
const binary=resolve('output/companion/current/ironwood-server.exe');
const records=Array.from({length:count},(_,i)=>({id:`game-${i}`,updatedAt:1,category:'imported',json:JSON.stringify({board:'8/8/8/8/8/8/8/K6k w - - 0 1',history:[],live_moves:['e4','c5'],review_pgn:`[White "Benchmark ${i}"]\n[Black "Opponent"]\n\n1. e4 {Keep note ${i}} c5 *`,move_notes:[`Keep note ${i}`]}),archiveAnalysisRuns:['Stockfish','Lc0'].map((engine,j)=>({id:`run-${j}`,revision:1,created_at:j+1,metadata:{engine_name:engine,engine_options:{Threads:'2'},settings_label:'Benchmark settings'},payload:{game_analysis:[{eval_cp:i}],verification_results:[]}}))}));
const results=[];
for(const [name,Implementation] of [['before',Before],['after',ArchiveLibrary]]){
 const root=await mkdtemp(resolve('tmp/import-benchmark-'));let child;
 try{
  assert.equal(spawnSync(binary,['init','--data-dir',root],{windowsHide:true}).status,0);
  const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';await writeFile(join(root,'config.json'),JSON.stringify(config));
  child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});child.stderr.resume();
  const endpoint=await new Promise((yes,no)=>{let log='';const timer=setTimeout(()=>no(Error('Startup timeout')),10000);child.stdout.on('data',part=>{log+=part;const match=log.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes('http://'+match[1]);}});child.once('error',no);});
  const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();
  const client=new CompanionArchiveClient(endpoint,credential,(url,options)=>fetch(url,{...options,headers:{...options.headers,Origin:'http://localhost:8788'}}));
  let converted=0,acknowledged=0,peakPrepared=0,requests=0,firstRequestMs;
  const start=performance.now();
  const tracked={endpoint,credential,async request(path,body){if(path==='archive/import'){requests++;firstRequestMs??=performance.now()-start;peakPrepared=Math.max(peakPrepared,converted-acknowledged);}return client.request(path,body);}};
  const library=new Implementation(tracked,{details:raw=>({white:'Benchmark '+JSON.parse(raw).move_notes[0].split(' ').at(-1),black:'Opponent',result:'*'}),exporter:raw=>{converted++;return JSON.parse(raw).review_pgn;},connection:()=>({endpoint,credential})});
  let imported=0,duplicates=0;
  await library.importBatchRecords(records,(result,n)=>{acknowledged+=n;imported+=result.imported;duplicates+=result.duplicates;});
  const elapsedMs=performance.now()-start;
  const importRequests=requests,peakPreparedGames=peakPrepared;
  assert.equal(imported,count);assert.equal(duplicates,0);
  await library.importBatchRecords(records,(result)=>{duplicates+=result.duplicates;assert.equal(result.imported,0);});assert.equal(duplicates,count);
  const page={games:[]};let cursor='';do{const part=await client.request(`archive/games?limit=100&after=${cursor}`);page.games.push(...part.games);cursor=part.next||'';}while(cursor);assert.equal(page.games.length,count);
  for(let offset=0;offset<page.games.length;offset+=100){const batch=page.games.slice(offset,offset+100),bundle=await client.request('archive/export',{ids:batch.map(game=>game.id),format:'json'});for(const doc of bundle.documents){const saved=JSON.parse(doc.record.json),original=records.find(record=>record.json===doc.record.json);assert.ok(original);assert.deepEqual(saved.move_notes,JSON.parse(original.json).move_notes);assert.deepEqual(doc.analysis_runs.toSorted((a,b)=>a.id.localeCompare(b.id)),original.archiveAnalysisRuns.toSorted((a,b)=>a.id.localeCompare(b.id)));}}
  results.push({name,games:count,elapsedMs:Math.round(elapsedMs),gamesPerSecond:Math.round(count*1000/elapsedMs),importRequests,peakPreparedGames,firstRequestMs:Math.round(firstRequestMs),duplicatesVerified:duplicates,analysisRunsVerified:count*2});
  const closed=once(child,'close');await fetch(endpoint+'/api/v1/shutdown',{method:'POST',headers:{Origin:'http://localhost:8788',Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1'}});await closed;
 }finally{if(child&&child.exitCode===null){const closed=once(child,'close');child.kill();await closed;}await rm(root,{recursive:true,force:true});}
}
const report={scope:`${count} short PGNs, notes and two saved runs each; real isolated local Archive HTTP, synthetic JS exporter (not browser WASM conversion/rendering)`,results,speedup:Math.round(results[0].elapsedMs/results[1].elapsedMs*100)/100};
await writeFile(resolve('tmp/archive-import-benchmark.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
