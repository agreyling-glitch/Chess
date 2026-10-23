import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {once} from 'node:events';
import {browserDocuments} from '../web/companion-archive-client.js';

const records=new Map(),local=new Map();
let failWrite=false,networkCalls=0,enabled=false;
const dom=new JSDOM('<body></body>',{url:'http://localhost:8788/play/'});
globalThis.window=dom.window;globalThis.document=dom.window.document;
globalThis.localStorage={getItem:key=>local.get(key)||null,setItem:(key,value)=>local.set(key,String(value)),removeItem:key=>local.delete(key)};
window.ironwoodCompanionFeatureEnabled=()=>enabled;
window.ironwoodCompanionLibrary=async()=>{networkCalls++;throw Error('Companion must not be contacted for Browser analyses');};
dom.window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
dom.window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new dom.window.Event('close'));};
const db={transaction(){
  let stopped=false;const staged=new Map();
  const tx={abort(){stopped=true;queueMicrotask(()=>tx.onabort?.());},objectStore(){return {
    get(id){const request={};setImmediate(()=>{
      request.result=structuredClone(records.get(id));request.onsuccess?.();
      queueMicrotask(()=>{
        if(stopped)return;
        if(failWrite&&staged.size){failWrite=false;tx.error=Error('Quota exceeded');tx.onabort?.();return;}
        for(const [key,value]of staged)records.set(key,value);
        tx.oncomplete?.();
      });
    });return request;},
    put(record){staged.set(record.id,structuredClone(record));},
    getAll(){const request={};setImmediate(()=>{request.result=structuredClone([...records.values()]);request.onsuccess?.();});return request;}
  };}};return tx;
}};
globalThis.indexedDB={open(){const request={};queueMicrotask(()=>{request.result=db;request.onsuccess();});return request;}};
window.indexedDB=globalThis.indexedDB;
let storage,serial=0;
async function reload(){storage=await import(new URL(`../web/game-storage.js?browser-test=${serial++}`,import.meta.url));}
const currentKey='ironwood.chess.game.v1',idKey='ironwood.chess.active-game-id.v1';
const settle=()=>new Promise(resolve=>setTimeout(resolve,25));
const metadata=engine=>({engine_name:engine,engine_identity:engine,engine_options:{Threads:'2',Hash:'64'},settings_label:'1 second per position',config_key:engine,position_fens:['start','e4','e5']});
const snapshot=(run,score)=>({board:'8/8/8/8/8/8/8/K6k w - - 0 1',history:[],live_moves:['e4','e5'],live_positions:['start','e4','e5'],review_pgn:'[White "Alice"]\n[Black "Bob"]\n\n1. e4 e5 *',analysis_run:run,game_analysis:[{eval_cp:score},{eval_cp:score+1},{eval_cp:score+2}],verification_targets:[],verification_results:[],verification_nodes:0,verification_time_ms:0,move_notes:['Keep these notes'],drawings:[{note:'Keep drawings'}]});
async function save(saved){local.set(currentKey,JSON.stringify(saved));window.ironwoodStoreCurrentGame(JSON.stringify(saved));await storage.companionBrowserRecords();await settle();}
async function state(){window.ironwoodArchiveAnalysisRuns();await settle();return JSON.parse(window.ironwoodArchiveAnalysisRuns());}

test('Browser runs persist atomically, survive reload and switch with Companion off',async()=>{
  await reload();local.set(idKey,'browser-game');
  const sf=JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(metadata('Stockfish 19'))));
  await save(snapshot(sf,21));
  const lc0=JSON.parse(window.ironwoodBeginArchiveAnalysisRun(JSON.stringify(metadata('Lc0'))));
  await save(snapshot(lc0,88));
  assert.equal(records.get('browser-game').archiveAnalysisRuns.length,2);
  assert.equal((await state()).runs.length,2);
  await reload();
  const restored=await state();assert.equal(restored.available,true);assert.equal(restored.offline,false);
  assert.deepEqual(new Set(restored.runs.map(run=>run.metadata.engine_name)),new Set(['Stockfish 19','Lc0']));
  assert.ok(restored.runs.every(run=>!run.payload));
  window.ironwoodSelectArchiveAnalysisRun(sf.id);await settle();
  const selected=JSON.parse(window.ironwoodTakeArchiveAnalysisRun());
  assert.equal(selected.game_analysis[0].eval_cp,21);assert.equal(selected.analysis_run.id,sf.id);
  assert.ok(!('move_notes' in selected));assert.equal(window.ironwoodTakeArchiveAnalysisRun(),'');
  await save({...snapshot(lc0,88),...selected});
  assert.equal(records.get('browser-game').archiveAnalysisRuns.length,2,'viewing a saved run does not add another run');
  assert.equal(networkCalls,0);
});

test('verification updates one run and transfers retain every payload and engine setting',async()=>{
  const record=records.get('browser-game'),sf=record.archiveAnalysisRuns.find(run=>run.metadata.engine_name==='Stockfish 19');
  const saved={...JSON.parse(record.json),...sf.payload};saved.analysis_run.verification=true;saved.verification_targets=[2];saved.verification_results=[null,null,{eval_cp:40}];saved.game_analysis[2].eval_cp=40;
  await save(saved);
  const updated=records.get('browser-game');assert.equal(updated.archiveAnalysisRuns.length,2);
  const verified=updated.archiveAnalysisRuns.find(run=>run.id===sf.id);assert.equal(verified.metadata.verification,true);assert.equal(verified.payload.verification_results[2].eval_cp,40);
  const unchangedRevision=verified.revision;await save(saved);assert.equal(records.get('browser-game').archiveAnalysisRuns.find(run=>run.id===sf.id).revision,unchangedRevision);
  const transfer=browserDocuments([records.get('browser-game')],()=> '1. e4 e5 *',storage.gameDetails)[0];
  assert.equal(transfer.analysis_runs.length,2);assert.ok(!('archiveAnalysisRuns' in transfer.record));
  assert.equal(transfer.analysis_runs.find(run=>run.id===sf.id).payload.game_analysis[2].eval_cp,40);
  assert.ok(transfer.analysis_runs.every(run=>run.revision>0&&run.metadata.engine_options.Threads==='2'));
  assert.deepEqual(JSON.parse(transfer.record.json).move_notes,['Keep these notes']);
});

test('Browser record imports every run into a real archive; retries do not duplicate or replace results',{timeout:20000},async()=>{
  const root=await mkdtemp(resolve('tmp/browser-analysis-transfer-'));
  const binary=resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);let child;
  try{
    assert.equal(spawnSync(binary,['init','--data-dir',root],{windowsHide:true}).status,0);
    const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';await writeFile(join(root,'config.json'),JSON.stringify(config));
    child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});
    const endpoint=await new Promise((yes,no)=>{let text='';const timer=setTimeout(()=>no(Error('Startup timed out')),5000);child.once('error',error=>{clearTimeout(timer);no(error);});child.stdout.on('data',part=>{text+=part;const match=text.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes('http://'+match[1]);}});});
    const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();
    const {ArchiveLibrary}=await import('../web/companion-library.js');
    const {CompanionArchiveClient}=await import('../web/companion-archive-client.js');
    enabled=true;
    const client=new CompanionArchiveClient(endpoint,credential,(url,options)=>fetch(url,{...options,headers:{...options.headers,Origin:'http://localhost:8788'}}));
    const library=new ArchiveLibrary(client,{details:storage.gameDetails,exporter:()=> '[White "Alice"]\n[Black "Bob"]\n\n1. e4 e5 *',connection:()=>({endpoint,credential})});
    const record=structuredClone(records.get('browser-game'));
    const ids=await library.importRecords([record]);assert.equal(ids.length,1);
    assert.deepEqual(await library.importRecords([record]),ids);
    const remote=await library.request(`archive/games/${ids[0]}/analysis-runs`);assert.equal(remote.runs.length,2);
    for(const expected of record.archiveAnalysisRuns){const actual=await library.request(`archive/games/${ids[0]}/analysis-runs/${expected.id}`);assert.deepEqual(actual.payload,expected.payload);assert.deepEqual(actual.metadata,expected.metadata);}
    const copied=await library.read(ids[0]);assert.deepEqual(JSON.parse(copied.json).move_notes,['Keep these notes']);
    assert.equal(records.get('browser-game').archiveAnalysisRuns.length,2,'source remains intact until a confirmed move removes it');
    const exited=once(child,'close');
    const shutdown=await fetch(endpoint+'/api/v1/shutdown',{method:'POST',headers:{Origin:'http://localhost:8788',Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1'}});
    assert.equal(shutdown.status,202);await exited;
  }finally{enabled=false;if(child&&child.exitCode===null){const exited=once(child,'close');child.kill();await exited;}await rm(root,{recursive:true,force:true});}
});

test('Manage analyses confirms local deletion, preserves notes and other runs, and cannot resurrect a deleted run',async()=>{
  await state();window.ironwoodManageArchiveAnalyses();await settle();
  const dialog=document.getElementById('analysis-runs-dialog');assert.ok(dialog);
  const sf=records.get('browser-game').archiveAnalysisRuns.find(run=>run.metadata.engine_name==='Stockfish 19');
  const stale=snapshot(sf.payload.analysis_run,99);
  const check=[...dialog.querySelectorAll('tbody input')].find(input=>input.getAttribute('aria-label').includes('Stockfish'));
  check.checked=true;check.dispatchEvent(new dom.window.Event('change'));
  const remove=dialog.querySelector('[data-delete]');assert.equal(remove.disabled,false);
  remove.click();assert.equal(records.get('browser-game').archiveAnalysisRuns.length,2);
  dialog.querySelector('[data-cancel-delete]').click();assert.equal(records.get('browser-game').archiveAnalysisRuns.length,2);
  remove.click();dialog.querySelector('[data-confirm-delete]').click();await settle();await settle();
  assert.equal(records.get('browser-game').archiveAnalysisRuns.length,1);
  assert.equal(JSON.parse(records.get('browser-game').json).analysis_run,null);
  assert.equal(records.get('browser-game').analyzed,0);
  assert.deepEqual(JSON.parse(local.get(currentKey)).move_notes,['Keep these notes']);
  assert.equal(JSON.parse(window.ironwoodTakeArchiveAnalysisRun()).analysis_run,null);
  await save(stale);assert.equal(records.get('browser-game').archiveAnalysisRuns.length,1);
  assert.equal(JSON.parse(records.get('browser-game').json).analysis_run,null);
  dialog.querySelector('[data-done]').click();
  await reload();assert.equal((await state()).runs.length,1);assert.equal(networkCalls,0);
});

test('quota failures leave prior run and game intact; legacy results migrate without attributing an engine',async()=>{
  local.set(idKey,'quota-game');await save(snapshot({id:'old',...metadata('Stockfish'),created_at:123},10));
  const before=structuredClone(records.get('quota-game'));
  failWrite=true;window.ironwoodStoreCurrentGame(JSON.stringify(snapshot({id:'new',...metadata('Lc0'),created_at:124},90)));
  await assert.rejects(storage.companionBrowserRecords(),/Quota/);assert.deepEqual(records.get('quota-game'),before);
  local.set(idKey,'legacy-game');const old=snapshot(null,5);await save(old);
  await save(snapshot({id:'known',...metadata('Stockfish'),created_at:125},20));
  const legacy=records.get('legacy-game').archiveAnalysisRuns;
  assert.equal(legacy.length,2);assert.equal(legacy.find(run=>run.id.startsWith('legacy-')).metadata.engine_name,'Previous analysis (engine unverified)');
  assert.equal(networkCalls,0);
  dom.window.close();
});



test('Archive preview steps through moves without importing or changing the current game',async(t)=>{
  await reload();
  dom.window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
  dom.window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new dom.window.Event('close'));};
  const fen='8/8/8/8/8/8/8/K6k w - - 0 1';
  const game=snapshot(null,0);
  const priorRecords=JSON.stringify([...records]),priorLocal=JSON.stringify([...local]);
  let loads=0;
  window.ironwoodLoadSavedGame=()=>{loads++;};
  window.ironwoodExportSavedGame=()=>JSON.stringify({positions:[fen,fen,fen],moves:['e4','e5']});
  await storage.previewCompanionDocument({format:'ironwood-companion-game',version:1,record:{json:JSON.stringify(game)}});
  const dialog=document.querySelector('#game-preview-dialog');
  assert.ok(dialog.open);
  assert.match(dialog.querySelector('.game-preview-board-tray').firstChild.getAttribute('aria-label'),/Position 0 of 2/);
  [...dialog.querySelectorAll('button')].find(button=>button.getAttribute('aria-label')==='Next').click();
  assert.match(dialog.querySelector('.game-preview-board-tray').firstChild.getAttribute('aria-label'),/Position 1 of 2/);
  assert.equal(dialog.querySelector('.game-preview-move.current').textContent,'e4');
  const moveList=dialog.querySelector('.game-preview-move-list'),row=dialog.querySelector('.game-preview-move').parentElement;
  Object.defineProperty(moveList,'clientHeight',{value:100});Object.defineProperty(row,'offsetTop',{value:400});Object.defineProperty(row,'offsetHeight',{value:30});
  dialog.querySelectorAll('.game-preview-move')[1].click();
  assert.equal(moveList.scrollTop,365,'the active row is centered, not just kept at the bottom edge');
  assert.equal(dialog.querySelector('.game-preview-move.current').textContent,'e5');
  assert.equal(dialog.querySelectorAll('[aria-current="step"]').length,1);
  assert.match(dialog.querySelector('.game-preview-board-tray').firstChild.getAttribute('aria-label'),/Position 2 of 2/);
  const realInterval=globalThis.setInterval,realClear=globalThis.clearInterval;
  t.after(()=>{globalThis.setInterval=realInterval;globalThis.clearInterval=realClear;});
  let intervalDelay,advance,clears=0;
  globalThis.setInterval=(callback,delay)=>{advance=callback;intervalDelay=delay;return 77;};globalThis.clearInterval=()=>{clears++;};
  const autoplay=dialog.querySelector('.game-preview-controls [aria-pressed]');
  autoplay.click();assert.equal(intervalDelay,2000);assert.equal(autoplay.getAttribute('aria-pressed'),'true');
  autoplay.dispatchEvent(new dom.window.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:100,clientY:100}));
  const menu=dialog.querySelector('[role="menu"]');assert.equal(menu.hidden,false);
  assert.equal(menu.querySelector('[aria-checked="true"]').dataset.delay,'2000');
  assert.equal(menu.querySelectorAll('[role="menuitemradio"]').length,5);
  menu.querySelector('[data-delay="500"]').click();assert.equal(intervalDelay,500);assert.equal(menu.hidden,true);assert.ok(clears>0);
  advance();assert.equal(dialog.querySelector('.game-preview-move.current').textContent,'e4');
  advance();assert.equal(autoplay.getAttribute('aria-pressed'),'false');
  dialog.close();assert.equal(document.querySelector('#game-preview-dialog'),null);
  assert.equal(local.get('ironwood.chess.preview-autoplay-delay.v1'),'500');
  await reload();
  await storage.previewCompanionDocument({format:'ironwood-companion-game',version:1,record:{json:JSON.stringify(game)}});
  const reopened=document.querySelector('#game-preview-dialog');
  assert.equal(reopened.querySelector('[aria-checked="true"]').dataset.delay,'500','speed survives reopening and module reload');
  reopened.querySelector('.game-preview-controls [aria-pressed]').click();assert.equal(intervalDelay,500);reopened.close();
  assert.equal(loads,0);assert.equal(JSON.stringify([...records]),priorRecords);
  assert.equal(JSON.stringify([...local].filter(([key])=>key!=='ironwood.chess.preview-autoplay-delay.v1')),priorLocal);
});
