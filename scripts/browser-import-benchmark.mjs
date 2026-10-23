// Local-only browser/WASM benchmark with a disposable archive. No user data.
import {createServer} from 'node:http';
import {readFile,writeFile,mkdtemp,stat} from 'node:fs/promises';
import {spawn,spawnSync} from 'node:child_process';
import {resolve,join,extname,sep} from 'node:path';
const label=process.argv[2]||'before',binary=resolve(process.argv[3]||'tmp/import-parser-baseline.exe'),port=Number(process.argv[4]||8808);
const root=await mkdtemp(resolve('tmp/browser-pgn-'+label+'-'));
if(spawnSync(binary,['init','--data-dir',root],{windowsHide:true}).status!==0)throw Error('Init failed');
const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';
const busy=process.argv.includes('--engine-busy');
if(busy)config.engines=[{id:'benchmark-stockfish',label:'Stockfish 19 single-thread',executable:process.execPath,args:[resolve('node_modules/stockfish/bin/stockfish-19-lite-single.js')],options:{Threads:'1',Hash:'16'}}];config.allowed_origins=[`http://127.0.0.1:${port}`];await writeFile(join(root,'config.json'),JSON.stringify(config));
const child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});child.stderr.resume();
const endpoint=await new Promise((yes,no)=>{let log='';child.stdout.on('data',part=>{log+=part;const match=log.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match)yes('http://'+match[1]);});child.once('error',no);});
const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();
const html=`<!doctype html><meta charset="utf-8"><title>Browser PGN benchmark ${label}</title><style>body{background:#171d24;color:#eee;font:16px system-ui}iframe{width:1000px;height:600px}pre{white-space:pre-wrap}</style><h1>Real browser PGN import: ${label}</h1><button id="run" disabled>Run 512-game benchmark</button><pre id="result">Loading real Ironwood browser app…</pre><iframe id="app" src="/play/"></iframe><script type="module" src="/benchmark.js"></script>`;
const browser=String.raw`import {ArchiveLibrary} from '/companion-library.js';import {CompanionArchiveClient} from '/companion-archive-client.js';
const cfg=await(await fetch('/benchmark-config')).json(),app=document.querySelector('#app'),button=document.querySelector('#run'),output=document.querySelector('#result');
while(!app.contentWindow?.ironwoodPrepareArchivedPgn)await new Promise(resolve=>setTimeout(resolve,100));
button.disabled=false;output.textContent='Ready. Real WASM parsing and annotated export; isolated local archive.';
button.onclick=async()=>{button.disabled=true;try{
 const w=app.contentWindow,storage=await import('/game-storage.js');
 let engine,engineStarted=false;
 if(cfg.busy){
  const {CompanionEngineAdapter}=await import('/companion-engine-adapter.js');
  engine=new CompanionEngineAdapter(line=>{if(line.startsWith('info depth'))engineStarted=true;},message=>{output.textContent=message;});
  await engine.connect({...cfg,engineId:'benchmark-stockfish'});
  engine.command('position startpos');engine.command('go infinite');
  const deadline=Date.now()+15000;
  while(!engineStarted){if(Date.now()>deadline)throw Error('Engine search did not start');await new Promise(r=>setTimeout(r,25));}
 }

 const client=new CompanionArchiveClient(cfg.endpoint,cfg.credential);
 const fixture=await(await fetch('/fixture.pgn')).text(),pgns=fixture.trim().split(/\n\n(?=\[Event)/),records=[];
 let requests=0,requestMs=0;const originalRequest=client.request.bind(client);client.request=async(...args)=>{const start=performance.now();try{if(args[0]==='archive/import')requests++;return await originalRequest(...args);}finally{if(args[0]==='archive/import')requestMs+=performance.now()-start;}};
 const library=new ArchiveLibrary(client,{details:storage.gameDetails,exporter:w.ironwoodExportSavedGame,connection:()=>cfg});
 const start=performance.now();let preparationMs=0,importMs=0,imported=0;
 for(let offset=0;offset<pgns.length;offset+=128){const prep=performance.now(),batch=[];
  for(let i=offset;i<Math.min(offset+128,pgns.length);i++){
   const prepared=w.ironwoodPrepareArchivedPgn(pgns[i]);const saved=JSON.parse(prepared);saved.move_notes[1]='Preserve benchmark note '+i;const json=JSON.stringify(saved);
   const record={id:'benchmark-'+i,category:'imported',updatedAt:1,json,archiveAnalysisRuns:['Stockfish','Lc0'].map((name,j)=>({id:'run-'+j,revision:1,created_at:j+1,metadata:{engine_name:name,settings_label:'Benchmark settings'},payload:{game_analysis:saved.game_analysis}}))};records.push(record);batch.push(record);
   if(i%8===0)await new Promise(resolve=>setTimeout(resolve,0));
  }preparationMs+=performance.now()-prep;
  const transfer=performance.now();
  await library.importBatchRecords(batch,r=>{imported+=r.imported;});importMs+=performance.now()-transfer;
  output.textContent='Imported '+imported+' / '+pgns.length;
 }
 const totalMs=performance.now()-start;
 if(engine)await engine.release();
 let duplicates=0;
 await library.importBatchRecords(records,r=>{duplicates+=r.duplicates;});if(duplicates!==records.length)throw Error('Duplicate mismatch');
 const ids=[];let cursor='';do{const page=await client.request('archive/games?limit=100&after='+cursor);ids.push(...page.games.map(g=>g.id));cursor=page.next||'';}while(cursor);
 let notes=0,runs=0;for(let offset=0;offset<ids.length;offset+=50){const bundle=await client.request('archive/export',{ids:ids.slice(offset,offset+50),format:'json'});for(const doc of bundle.documents){const original=records.find(r=>r.id===doc.record.id);if(!original||original.json!==doc.record.json)throw Error('Record changed');if(doc.analysis_runs.length!==2)throw Error('Lost analysis');for(const run of doc.analysis_runs){const expected=original.archiveAnalysisRuns.find(r=>r.id===run.id);if(JSON.stringify(run.payload)!==JSON.stringify(expected.payload)||JSON.stringify(run.metadata)!==JSON.stringify(expected.metadata))throw Error('Analysis changed');}const noteCount=JSON.parse(doc.record.json).move_notes.filter(Boolean).length;if(noteCount<1)throw Error('Note missing');notes+=noteCount;runs+=doc.analysis_runs.length;}}
 const report={label:cfg.label,engineBusy:cfg.busy,engineSearchVerified:engineStarted,games:records.length,pliesPerGame:33,totalMs:Math.round(totalMs),preparationMs:Math.round(preparationMs),conversionAndImportMs:Math.round(importMs),requestMs:Math.round(requestMs),duplicatesVerified:duplicates,notesVerified:notes,analysisRunsVerified:runs,userAgent:navigator.userAgent};
 await fetch('/benchmark-result',{method:'POST',body:JSON.stringify(report)});output.textContent=JSON.stringify(report,null,2);
}catch(error){output.textContent=error.stack;await fetch('/benchmark-error',{method:'POST',body:String(error.stack)});}finally{button.disabled=false;}};`;
const line='1. e4 {Keep this note [%eval 0.2]} e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0';
const fixture=Array.from({length:512},(_,i)=>`[Event "Benchmark ${i}"]\n[White "White ${i}"]\n[Black "Black"]\n[Result "1-0"]\n\n${line}`).join('\n\n');
const assets=resolve('.wrangler-assets');
const server=createServer(async(req,res)=>{try{
 res.setHeader('Cross-Origin-Opener-Policy','same-origin');res.setHeader('Cross-Origin-Embedder-Policy','require-corp');res.setHeader('Cache-Control','no-store');
 const path=new URL(req.url,'http://localhost').pathname;
 if(path==='/benchmark-config'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify({label,endpoint,credential,busy}));return;}
 if(path==='/bench.html'){res.setHeader('Content-Type','text/html');res.end(html);return;}
 if(path==='/companion-library.js'&&process.argv.includes('--before')){res.setHeader('Content-Type','text/javascript');res.end(await readFile(resolve('tmp/adaptive-import-before-library.mjs')));return;}
 if(path==='/benchmark.js'){res.setHeader('Content-Type','text/javascript');res.end(browser);return;}
 if(path==='/fixture.pgn'){res.end(fixture);return;}
 if(path==='/benchmark-result'||path==='/benchmark-error'){let body='';for await(const part of req)body+=part;await writeFile(resolve('tmp/browser-pgn-'+label+(path.endsWith('error')?'-error.txt':'.json')),body);res.end('OK');return;}
 let file=resolve(assets,'.'+path);if(file!==assets&&!file.startsWith(assets+sep)){res.writeHead(403).end();return;}if((await stat(file)).isDirectory())file=join(file,'index.html');
 const mime={'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.json':'application/json','.png':'image/png','.svg':'image/svg+xml'};res.setHeader('Content-Type',mime[extname(file)]||'application/octet-stream');res.end(await readFile(file));
 }catch{res.writeHead(404).end('Not found');}}).listen(port,'127.0.0.1',()=>console.log(`Browser benchmark ${label}: http://127.0.0.1:${port}/bench.html; isolated archive only`));
process.on('SIGINT',()=>{child.kill();server.close(()=>process.exit());});
