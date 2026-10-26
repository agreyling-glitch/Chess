// Owner-local source reads; all HTTP writes go to a newly initialized disposable archive.
import {readFile,writeFile,mkdir,mkdtemp,open,stat} from 'node:fs/promises';
import {readFileSync} from 'node:fs';
import {spawn,spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {resolve,join} from 'node:path';
import {JSDOM} from 'jsdom';
import {openScid5,importScidRange} from '../web/import-scid5.js';
import {ArchiveLibrary} from '../web/companion-library.js';
import {CompanionArchiveClient} from '../web/companion-archive-client.js';
const mode=process.argv[2]||'legacy',count=Number(process.argv[3]||512);
if(!['legacy','direct','serial-stream','pipeline'].includes(mode)||!Number.isInteger(count)||count<1||count>2048)throw Error('Use legacy|direct|serial-stream|pipeline and 1–2048 games.');
const streaming=['serial-stream','pipeline'].includes(mode);
const folder=resolve('output/scid5-validation'),binary=resolve('output/companion/current/ironwood-server.exe');
const dom=new JSDOM('',{url:'http://localhost/saved-games/'});
globalThis.window=dom.window;globalThis.Window=dom.window.Window;
const wasm=await import('../web/pkg/battle_chess.js');
await wasm.default({module_or_path:readFileSync(resolve('web/pkg/battle_chess_bg.wasm'))});
window.ironwoodExportSavedGame=wasm.export_saved_game;
const {gameDetails,importFingerprint}=await import('../web/game-storage.js');
const metrics={mode,games:count,namesMs:0,indexMs:0,decodeMs:0,prepareMs:0,detailsAndFingerprintMs:0,documentConversionMs:0,requestMs:0,payloadBytes:0,requests:0,imported:0,duplicates:0,discarded:0};
const handles=[];let child,source;
await mkdir(resolve('output/scid-import-benchmark'),{recursive:true});
const root=await mkdtemp(resolve('output/scid-import-benchmark/archive-'));
try{
 const files=await Promise.all(['si5','sn5','sg5'].map(async ext=>{
  const path=join(folder,`LumbrasGigaBase_OTB.${ext}`),info=await stat(path),file=await open(path);handles.push(file);
  return {name:`sample.${ext}`,size:info.size,slice(start,end){return {async arrayBuffer(){const bytes=Buffer.alloc(end-start);const result=await file.read(bytes,0,bytes.length,start);return bytes.subarray(0,result.bytesRead);}};}};
 }));
 let start=performance.now();source=await openScid5(files,names=>new wasm.Scid5Reader(names));metrics.namesMs=performance.now()-start;
 const pgns=[];
 if(!streaming){
  start=performance.now();const entries=[];for await(const entry of source.entries(0,count))entries.push(entry);metrics.indexMs=performance.now()-start;
  start=performance.now();for(const entry of entries){const pgn=await source.pgn(entry);if(pgn)pgns.push(pgn);}metrics.decodeMs=performance.now()-start;
 }
 if(spawnSync(binary,['init','--data-dir',root],{windowsHide:true,stdio:'ignore'}).status!==0)throw Error('Disposable archive initialization failed.');
 const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';config.allowed_origins=['http://localhost:8788'];await writeFile(join(root,'config.json'),JSON.stringify(config));
 child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});child.stderr.resume();
 const endpoint=await new Promise((yes,no)=>{let log='';const timer=setTimeout(()=>no(Error('Archive startup timeout')),10000);child.stdout.on('data',part=>{log+=part;const match=log.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes('http://'+match[1]);}});child.once('error',no);});
 const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();
 const client=new CompanionArchiveClient(endpoint,credential,(url,options)=>fetch(url,{...options,headers:{...options.headers,Origin:'http://localhost:8788'}}));
 const request=client.request.bind(client);client.request=async(path,body)=>{
  const stamp=performance.now();if(path==='archive/import'){metrics.requests++;metrics.payloadBytes+=Buffer.byteLength(JSON.stringify(body));}
  try{return await request(path,body);}finally{if(path==='archive/import')metrics.requestMs+=performance.now()-stamp;}
 };
 const timeConversion=fn=>(...args)=>{const stamp=performance.now();try{return fn(...args);}finally{metrics.documentConversionMs+=performance.now()-stamp;}};
 const library=new ArchiveLibrary(client,{details:timeConversion(gameDetails),exporter:timeConversion(wasm.export_saved_game),connection:()=>({endpoint,credential})});
 const result=value=>{metrics.imported+=value.imported;metrics.duplicates+=value.duplicates;metrics.discarded+=value.discarded||0;};
 start=performance.now();
 if(streaming){
  const timed={count:source.count,entries:(first,end)=>source.entries(first,end),async pgn(entry){const stamp=performance.now();try{return await source.pgn(entry);}finally{metrics.decodeMs+=performance.now()-stamp;}}};
  await importScidRange(timed,{end:count,batchSize:128,prefetch:mode==='pipeline'},{saveBatch:async batch=>{
   const summary={saved:0,duplicates:0,discarded:0,failed:0};
   await library.importPgnBatch(batch,value=>{result(value);summary.saved+=value.imported;summary.duplicates+=value.duplicates;summary.discarded+=value.discarded||0;summary.failed+=value.failed||0;});return summary;
  }});
 }
 for(let offset=0;offset<pgns.length;offset+=128){
  const batch=pgns.slice(offset,offset+128);
  if(mode==='direct'){await library.importPgnBatch(batch,result);continue;}
  const records=[];
  for(const pgn of batch){
   // Match production cooperative yields in both record preparation and the
   // real importBatchRecords implementation (timer cost is platform-specific).
   if(records.length%8===0)await new Promise(resolve=>setTimeout(resolve,0));
   let stamp=performance.now();const json=wasm.prepare_archived_pgn(pgn);metrics.prepareMs+=performance.now()-stamp;
   stamp=performance.now();const details=gameDetails(json),fingerprint=importFingerprint(pgn);metrics.detailsAndFingerprintMs+=performance.now()-stamp;
   if(details.moves<4){metrics.discarded++;continue;}
   records.push({id:`bench-${offset}-${records.length}`,...details,category:'imported',fingerprint,updatedAt:1,favorite:false,json});
  }
  await library.importBatchRecords(records,result);
 }
 metrics.saveWallMs=performance.now()-start;
 metrics.totalMs=metrics.namesMs+(streaming?0:metrics.indexMs+metrics.decodeMs)+metrics.saveWallMs;
 metrics.gamesPerSecond=count*1000/metrics.totalMs;
 metrics.pacingAndOtherMs=streaming?null:metrics.saveWallMs-metrics.prepareMs-metrics.detailsAndFingerprintMs-metrics.documentConversionMs-metrics.requestMs;
 const stats=await request('archive/stats');metrics.archiveStats=stats;
 for(const [key,value]of Object.entries(metrics))if(typeof value==='number')metrics[key]=Math.round(value*100)/100;
 await writeFile(resolve(`output/scid-import-benchmark/${mode}.json`),JSON.stringify(metrics,null,2));console.log(JSON.stringify(metrics,null,2));
}finally{
 source?.free();await Promise.all(handles.map(file=>file.close()));dom.window.close();
 if(child&&child.exitCode===null){const closed=once(child,'close');child.kill();await closed;}
}
