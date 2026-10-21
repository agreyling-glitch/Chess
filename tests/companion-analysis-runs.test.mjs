import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {JSDOM} from 'jsdom';
globalThis.window=globalThis;
const local=new Map();globalThis.localStorage={getItem:key=>local.get(key)||null,setItem:(key,value)=>local.set(key,value),removeItem:key=>local.delete(key)};
let enabled=true;window.ironwoodCompanionFeatureEnabled=()=>enabled;
const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');
const {preserveArchiveAnalysis,archiveAnalysisSnapshot,analysisLocalCache,queueArchiveAnalysis,syncArchiveAnalyses}=await import(`data:text/javascript,${encodeURIComponent(source)}`);
const cacheEntries=new Map();
function memoryCache(cache){Object.assign(cache,{
 async get(id){return structuredClone(cacheEntries.get(id));},
 async list(index,value){return structuredClone([...cacheEntries.values()].filter(entry=>entry[index]===value));},
 async write(put=[],remove=[]){for(const entry of put)cacheEntries.set(entry.id,structuredClone(entry));for(const id of remove)cacheEntries.delete(id);},
 async mutate(changes){for(const [id,change]of changes){const value=change(structuredClone(cacheEntries.get(id)));if(value)cacheEntries.set(id,structuredClone(value));else cacheEntries.delete(id);}}
});}memoryCache(analysisLocalCache);
const runs=new Map(),calls=[];
const storage={endpoint:'http://local:8790',check(){},async request(path,body){calls.push({path,body});const id=path.split('/').at(-1);
 if(path.endsWith('/analysis-runs/delete')){for(const [id,revision]of body.runs)assert.equal(runs.get(id)?.revision,revision);for(const [id]of body.runs)runs.delete(id);return {deleted:body.runs.length};}
 if(path.endsWith('/analysis-runs'))return {runs:[...runs.values()].map(({payload,...run})=>run)};
 if(body){const previous=runs.get(id);assert.equal(body.expected_revision,previous?.revision||0);const run={id,revision:(previous?.revision||0)+1,created_at:123,metadata:structuredClone(body.metadata),payload:structuredClone(body.payload)};runs.set(id,run);return structuredClone(run);}
 if(!runs.has(id))throw Object.assign(Error('Not found'),{status:404});return structuredClone(runs.get(id));
}};
window.ironwoodCompanionLibrary=async()=>storage;
const activeKey='ironwood.chess.active-archive-game.v1';
const context=(id,engine)=>({id,engine_name:engine,settings_label:'1 second per position',config_key:engine,position_fens:['a','b'],created_at:123});
const game=(context,score)=>({analysis_run:context,game_analysis:[{eval_cp:score},null],verification_targets:[],verification_results:[],verification_nodes:0,verification_time_ms:0,move_notes:['Keep my notes']});
test('independent runs retain different engine scores, snapshots omit game annotations, and legacy results are preserved',async()=>{
 const old={game_analysis:[{eval_cp:15},null],move_notes:['Original notes']};
 const stockfish=game(context('sf','Stockfish 19'),21);await preserveArchiveAnalysis(storage,'game',old,stockfish);
 const lc0=game(context('lc0','Lc0 v0.32.1'),88);await preserveArchiveAnalysis(storage,'game',stockfish,lc0);
 assert.equal(runs.size,3);assert.equal(runs.get('sf').payload.game_analysis[0].eval_cp,21);assert.equal(runs.get('lc0').payload.game_analysis[0].eval_cp,88);assert.equal(runs.get('legacy-game').payload.game_analysis[0].eval_cp,15);
 assert.ok(!('move_notes' in archiveAnalysisSnapshot(stockfish)));const writes=calls.filter(call=>call.body).length;
 await preserveArchiveAnalysis(storage,'game',lc0,lc0);assert.equal(calls.filter(call=>call.body).length,writes);
 await preserveArchiveAnalysis(storage,'game',old,game(context('sf2','Stockfish 19'),32));assert.equal(runs.get('legacy-game').payload.game_analysis[0].eval_cp,15);
});
test('manager requires confirmation and deletes only checked runs without restoring deleted results',async()=>{
 const dom=new JSDOM('<body></body>');globalThis.document=dom.window.document;
 dom.window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 dom.window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new dom.window.Event('close'));};
 globalThis.window.addEventListener=()=>{};globalThis.window.removeEventListener=()=>{};
 enabled=true;local.set(activeKey,JSON.stringify({endpoint:storage.endpoint,id:'game',revision:1}));
 local.set('ironwood.chess.game.v1',JSON.stringify(game(context('lc0','Lc0 v0.32.1'),88)));
 const before=runs.get('sf');window.ironwoodArchiveAnalysisRuns();await new Promise(resolve=>setTimeout(resolve,30));
 window.ironwoodManageArchiveAnalyses();await new Promise(resolve=>setTimeout(resolve,30));
 const dialog=document.getElementById('analysis-runs-dialog');assert.ok(dialog);
 assert.deepEqual([...dialog.querySelectorAll('th')].slice(1).map(cell=>cell.textContent),['Engine','Settings','Status','Date']);
 const checkbox=[...dialog.querySelectorAll('tbody input')].find(input=>input.getAttribute('aria-label').includes('Lc0'));assert.ok(checkbox);
 checkbox.checked=true;checkbox.dispatchEvent(new dom.window.Event('change'));
 const count=runs.size;dialog.querySelector('[data-delete]').click();assert.equal(runs.size,count);assert.equal(dialog.querySelector('[data-confirm]').hidden,false);
 dialog.querySelector('[data-cancel-delete]').click();assert.equal(runs.size,count);
 dialog.querySelector('[data-delete]').click();dialog.querySelector('[data-confirm-delete]').click();await new Promise(resolve=>setTimeout(resolve,40));
 assert.equal(runs.has('lc0'),false);assert.deepEqual(runs.get('sf'),before);
 const cleared=JSON.parse(window.ironwoodTakeArchiveAnalysisRun());assert.equal(cleared.analysis_run,null);assert.deepEqual(cleared.game_analysis,[null,null]);
 assert.deepEqual(JSON.parse(local.get('ironwood.chess.game.v1')).move_notes,['Keep my notes']);
 await preserveArchiveAnalysis(storage,'game',{},game(context('lc0','Lc0'),88));assert.equal(runs.has('lc0'),false);
 dialog.querySelector('[data-done]').click();assert.equal(document.getElementById('analysis-runs-dialog'),null);
 delete globalThis.document;local.delete(activeKey);
});
test('selector lists summaries and loads only the selected run; gating hides all controls',async()=>{
 local.set(activeKey,JSON.stringify({endpoint:storage.endpoint,id:'game',revision:1}));
 window.ironwoodArchiveAnalysisRuns();await new Promise(resolve=>setTimeout(resolve,20));
 const state=JSON.parse(window.ironwoodArchiveAnalysisRuns());assert.equal(state.available,true);assert.equal(state.runs.length,3);assert.ok(state.runs.every(run=>!run.payload));
 window.ironwoodSelectArchiveAnalysisRun('sf');await new Promise(resolve=>setTimeout(resolve,20));
 const selected=JSON.parse(window.ironwoodTakeArchiveAnalysisRun());assert.equal(selected.analysis_run.id,'sf');assert.equal(selected.game_analysis[0].eval_cp,21);assert.equal(window.ironwoodTakeArchiveAnalysisRun(),'');
 enabled=false;assert.equal(JSON.parse(window.ironwoodArchiveAnalysisRuns()).available,false);
 const requests=calls.length;
 const provenance=JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify({engine_name:'Stockfish 19',settings_label:'1000000 nodes per position'})));
 assert.equal(provenance.engine_name,'Stockfish 19');assert.equal(provenance.settings_label,'1000000 nodes per position');assert.ok(provenance.id);assert.ok(provenance.created_at>0);assert.equal(calls.length,requests);
 enabled=true;local.delete(activeKey);assert.equal(JSON.parse(window.ironwoodArchiveAnalysisRuns()).available,false);
});
test('second-pass verification updates the original saved run rather than adding an entry',async()=>{
 const first=game(context('single-stockfish-run','Stockfish 19'),21);
 first.game_analysis=[{eval_cp:21},{eval_cp:30}];
 const count=runs.size;await preserveArchiveAnalysis(storage,'game',{},first);
 const initial=runs.get('single-stockfish-run');assert.equal(runs.size,count+1);
 const verified=structuredClone(first);verified.analysis_run.verification=true;
 verified.verification_targets=[1];verified.verification_results=[null,{eval_cp:36}];verified.game_analysis[1].eval_cp=36;
 await preserveArchiveAnalysis(storage,'game',first,verified);
 assert.equal(runs.size,count+1);const result=runs.get('single-stockfish-run');
 assert.equal(result.revision,initial.revision+1);assert.equal(result.created_at,initial.created_at);
 assert.equal(result.payload.game_analysis[1].eval_cp,36);assert.equal(result.metadata.verification,true);
});
test('flag-off provenance saves later with its known engine while archive access stays gated',async()=>{
 enabled=false;local.set(activeKey,JSON.stringify({endpoint:storage.endpoint,id:'game',revision:1}));
 const count=calls.length;
 const provenance=JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(context('unused','Stockfish 19'))));
 const saved=game(provenance,42);saved.game_analysis=[{eval_cp:42},{eval_cp:50}];
 window.ironwoodStoreCurrentGame(JSON.stringify(saved));assert.equal(calls.length,count);
 assert.equal(JSON.parse(window.ironwoodArchiveAnalysisRuns()).available,false);
 enabled=true;local.delete(activeKey);
 await preserveArchiveAnalysis(storage,'game',{},saved);
 assert.equal(runs.get(provenance.id).metadata.engine_name,'Stockfish 19');
 assert.equal(runs.get(provenance.id).metadata.settings_label,'1 second per position');
 assert.equal(runs.get(provenance.id).metadata.status,'Complete');
});

async function reloadStorage(tag){const module=await import(`data:text/javascript,${encodeURIComponent(source+`\n// reload ${tag}`)}`);memoryCache(module.analysisLocalCache);return module;}
const settle=()=>new Promise(resolve=>setTimeout(resolve,35));
function queueServer(endpoint,initial){
 let record={json:JSON.stringify(initial),_archive:{endpoint,id:'offline-game',revision:1}},lost=false;
 const savedRuns=new Map(),requests=[];
 const server={endpoint,check(){},record:value=>structuredClone(value),savedRuns,requests,
   get recordValue(){return structuredClone(record);},set recordValue(value){record=structuredClone(value);},loseNextResponse:false,
   async request(path,body){requests.push({path,body});const id=path.split('/').at(-1);
     if(path.includes('/analysis-runs/')){
       if(body){const prior=savedRuns.get(id);if(body.expected_revision!==(prior?.revision||0))throw Object.assign(Error('Revision conflict'),{status:409});const run={id,revision:(prior?.revision||0)+1,created_at:Date.now(),metadata:structuredClone(body.metadata),payload:structuredClone(body.payload)};savedRuns.set(id,run);if(server.loseNextResponse&&!lost){lost=true;throw TypeError('Response lost');}return structuredClone(run);}
       if(!savedRuns.has(id))throw Object.assign(Error('Not found'),{status:404});return structuredClone(savedRuns.get(id));
     }
     return structuredClone(record);
   },
   async write(next){assert.equal(next._archive.revision,record._archive.revision);record={...structuredClone(next),_archive:{...next._archive,revision:next._archive.revision+1}};return structuredClone(record);}
 };return server;
}

test('opening a game caches all runs without selecting them, surviving reload and offline switching',async()=>{
 enabled=true;cacheEntries.clear();local.set(activeKey,JSON.stringify({endpoint:storage.endpoint,id:'game',revision:1}));
 const online=await reloadStorage('cache-online');window.ironwoodArchiveAnalysisRuns();await settle();
 assert.equal(window.ironwoodTakeArchiveAnalysisRun(),'');
 const cached=await online.analysisLocalCache.list('game',`${storage.endpoint}/game`);assert.deepEqual(new Set(cached.filter(entry=>entry.kind==='run').map(entry=>entry.run.id)),new Set(runs.keys()));
 const reads=calls.filter(call=>call.path.includes('/analysis-runs/')&&!call.body).length;
 window.ironwoodRefreshArchiveAnalysisRuns();await settle();assert.equal(calls.filter(call=>call.path.includes('/analysis-runs/')&&!call.body).length,reads,'Unchanged cached runs are not downloaded again');
 const request=storage.request;storage.request=async()=>{throw TypeError('Companion is stopped');};
 try{
   await reloadStorage('cache-offline');window.ironwoodArchiveAnalysisRuns();await settle();
   const state=JSON.parse(window.ironwoodArchiveAnalysisRuns());assert.equal(state.offline,true);assert.match(state.message,/Companion offline/);assert.deepEqual(new Set(state.runs.map(run=>run.id)),new Set(runs.keys()));
   const count=calls.length;window.ironwoodSelectArchiveAnalysisRun('sf');await settle();assert.equal(JSON.parse(window.ironwoodTakeArchiveAnalysisRun()).game_analysis[0].eval_cp,21);
   window.ironwoodSelectArchiveAnalysisRun('sf2');await settle();assert.equal(JSON.parse(window.ironwoodTakeArchiveAnalysisRun()).game_analysis[0].eval_cp,32);assert.equal(calls.length,count);
 }finally{storage.request=request;local.delete(activeKey);}
});

test('disabling Companion stops background caching without committing an in-flight payload',async()=>{
 cacheEntries.clear();enabled=true;local.set(activeKey,JSON.stringify({endpoint:storage.endpoint,id:'game',revision:1}));
 const request=storage.request;let finish,details=0;
 storage.request=async(path,body)=>{if(path.includes('/analysis-runs/')&&!body){details++;const id=path.split('/').at(-1);return new Promise(resolve=>{finish=()=>resolve(structuredClone(runs.get(id)));});}return request.call(storage,path,body);};
 try{
   const module=await reloadStorage('cache-cancel');window.ironwoodArchiveAnalysisRuns();await settle();assert.equal(details,1);assert.equal(typeof finish,'function');
   enabled=false;assert.equal(JSON.parse(window.ironwoodArchiveAnalysisRuns()).available,false);finish();await settle();assert.equal(details,1);
   assert.equal((await module.analysisLocalCache.list('game',`${storage.endpoint}/game`)).filter(entry=>entry.kind==='run').length,0);
 }finally{storage.request=request;local.delete(activeKey);enabled=true;}
});

test('offline runs and latest game edits survive reload, sync once, and keep engine settings and notes',async()=>{
 enabled=false;cacheEntries.clear();const module=await reloadStorage('outbox-before');
 const endpoint='http://offline:8790',active={endpoint,id:'offline-game',revision:1};
 const sf=game(JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(context('','Stockfish 19')))),21);
 const lc0=game(JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(context('','Lc0')))),88);lc0.move_notes=['Retain this offline note'];
 await module.queueArchiveAnalysis(active,JSON.stringify(sf));await module.queueArchiveAnalysis(active,JSON.stringify(lc0));
 const key=`${endpoint}/offline-game`;assert.equal((await module.analysisLocalCache.list('game',key)).filter(entry=>entry.pending).length,2);
 const restored=await reloadStorage('outbox-after'),server=queueServer(endpoint,{game_analysis:[],move_notes:['Original note']});
 const queue=await restored.analysisLocalCache.get(`${key}/pending`);assert.equal(JSON.parse(queue.json).move_notes[0],'Retain this offline note');
 await restored.syncArchiveAnalyses(server,queue);assert.equal(server.savedRuns.size,2);
 assert.equal(server.savedRuns.get(sf.analysis_run.id).payload.game_analysis[0].eval_cp,21);assert.equal(server.savedRuns.get(lc0.analysis_run.id).payload.game_analysis[0].eval_cp,88);
 assert.equal(server.savedRuns.get(lc0.analysis_run.id).metadata.engine_name,'Lc0');assert.equal(server.savedRuns.get(lc0.analysis_run.id).metadata.settings_label,'1 second per position');
 assert.equal(JSON.parse(server.recordValue.json).move_notes[0],'Retain this offline note');assert.equal(await restored.analysisLocalCache.get(queue.id),undefined);
 const puts=server.requests.filter(request=>request.body).length;await restored.syncArchiveAnalyses(server,{...queue,baseRevision:2});assert.equal(server.requests.filter(request=>request.body).length,puts);
});

test('lost save responses retry without duplicate runs, and concurrent archive notes are never overwritten',async()=>{
 enabled=false;cacheEntries.clear();const module=await reloadStorage('ambiguous');
 const endpoint='http://lost-response:8790',active={endpoint,id:'offline-game',revision:1},key=`${endpoint}/offline-game`;
 const saved=game(JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(context('','Stockfish 19')))),50);
 await module.queueArchiveAnalysis(active,JSON.stringify(saved));const queue=await module.analysisLocalCache.get(`${key}/pending`),server=queueServer(endpoint,{game_analysis:[],move_notes:['Original note']});server.loseNextResponse=true;
 await assert.rejects(module.syncArchiveAnalyses(server,queue),/Response lost/);assert.ok(await module.analysisLocalCache.get(queue.id));
 const updated=server.recordValue;updated._archive.revision=2;updated.json=JSON.stringify({game_analysis:[],move_notes:['Newer notes in another browser']});server.recordValue=updated;
 await assert.rejects(module.syncArchiveAnalyses(server,queue),/changed elsewhere/);assert.equal(server.savedRuns.size,1);assert.equal(server.requests.filter(request=>request.body).length,1);
 assert.equal(JSON.parse(server.recordValue.json).move_notes[0],'Newer notes in another browser');assert.ok(await module.analysisLocalCache.get(queue.id));
});
