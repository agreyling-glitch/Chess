import {test} from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
const dom=new JSDOM('<body></body>',{url:'https://test.local'});
Object.assign(globalThis,{window:dom.window,document:dom.window.document,localStorage:dom.window.localStorage});
const {buildGameExport,buildGameExportWithProgress,createGameExportProgress}=await import('../web/game-storage.js');
test('progress export yields during preparation and preserves all export formats byte-for-byte',async()=>{
 const records=Array.from({length:260},(_,i)=>({json:JSON.stringify({id:i,note:'Preserve me'})}));const before=JSON.stringify(records);
 for(const format of ['pgn','annotated','json']){
  const progress=[];let yields=0;const exporter=(json,kind)=>kind==='json'?json:`[Event "${JSON.parse(json).id}"]\n\n1. e4 *`;
  const actual=await buildGameExportWithProgress(records,format,exporter,(done,total)=>progress.push([done,total]),async()=>{yields++;});
  assert.deepEqual(actual,buildGameExport(records,format,exporter));assert.ok(yields>=3);assert.deepEqual(progress.at(-1),[260,260]);assert.ok(progress.every(([done,total],i)=>total===260&&done>(progress[i-1]?.[0]||0)));
 }
 assert.equal(JSON.stringify(records),before);
 await assert.rejects(buildGameExportWithProgress(records,'pgn',()=>{throw Error('conversion failed');}),/conversion failed/);
});
test('export progress exposes accessible stages and clamps the 3d fill',()=>{
 const progress=createGameExportProgress();document.body.append(progress.panel);assert.equal(progress.panel.hidden,true);
 progress.update(42,'Reading games — 100 of 240');const bar=progress.panel.querySelector('[role=progressbar]');assert.equal(bar.getAttribute('aria-valuenow'),'42');assert.equal(bar.firstChild.style.width,'42%');assert.match(progress.panel.textContent,/100 of 240/);
 progress.update(100,'Export ready. Download started.');assert.equal(bar.getAttribute('aria-valuenow'),'100');assert.equal(progress.panel.hidden,false);
});

test('completed export dismisses automatically without hiding a newer export or an error',t=>{
 t.mock.timers.enable({apis:['setTimeout']});
 const progress=createGameExportProgress();
 progress.update(100,'Download started');t.mock.timers.tick(1499);assert.equal(progress.panel.hidden,false);
 t.mock.timers.tick(1);assert.equal(progress.panel.hidden,true);
 progress.update(100,'Download started');t.mock.timers.tick(1000);
 progress.update(0,'Preparing next export');t.mock.timers.tick(1500);assert.equal(progress.panel.hidden,false);
 progress.update(40,'Export failed');t.mock.timers.tick(10000);assert.equal(progress.panel.hidden,false);
 progress.hide();assert.equal(progress.panel.hidden,true);
});
