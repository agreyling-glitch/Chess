import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {folderImportView,installCompanionImportProgress,mountCompanionImportProgress} from '../web/companion-import-progress.js';
import {connectCompanion,forgetCompanionConnection} from '../web/companion-connection.js';

test('folder progress formats counts, speed, ETA and preparation states',()=>{
  const value={active:true,phase:'importing',filename:'db.7z',processed:20544,total_games:100000,games_per_second:375.2,eta_seconds:212,stats:{imported:20526,discarded:18}};
  const view=folderImportView(value);assert.match(view.counts,/20,544 of 100,000/);assert.match(view.counts,/Saved 20,526/);assert.match(view.timing,/375.2 games\/s.*3m 32s remaining/);assert.equal(view.fraction,0.20544);
  for(const [phase,expected] of [['resuming',/Resuming/],['unpacking',/Unpacking/],['counting',/Counting/],['waiting',/estimate paused/],['verifying',/verification/],['complete',/Complete/],['failed',/Needs attention/]])assert.match(folderImportView({...value,phase}).timing,expected);
  assert.equal(folderImportView({...value,total_games:null}).fraction,null);
});
test('disabled or disconnected Companion stays hidden and sends no requests',async()=>{
  const dom=new JSDOM('',{url:'http://localhost/play/'}),target=dom.window;
  target.ironwoodCompanionFeatureEnabled=()=>false;installCompanionImportProgress(target);
  assert.equal(target.ironwoodCompanionImportProgress(),'');
  const stop=mountCompanionImportProgress(target,target.document.body);
  await new Promise(resolve=>target.setTimeout(resolve,10));assert.equal(target.document.querySelector('.companion-folder-progress').hidden,true);
  target.ironwoodCompanionFeatureEnabled=()=>true;assert.equal(target.ironwoodCompanionImportProgress(),'');stop();assert.equal(target.document.querySelector('.companion-folder-progress'),null);dom.window.close();
});
test('polls are shared and disabling Companion clears cached progress',async()=>{
  const dom=new JSDOM('',{url:'http://localhost/play/',pretendToBeVisual:true}),target=dom.window;
  const oldWindow=globalThis.window,oldFetch=globalThis.fetch;globalThis.window=target;
  let enabled=true,calls=0;target.ironwoodCompanionFeatureEnabled=()=>enabled;
  globalThis.fetch=async url=>{const caps=url.endsWith('/capabilities');if(!caps)calls++;return new Response(JSON.stringify(caps?{product:'Ironwood Companion',protocol_version:'1',services:{archive:true}}:{active:true,phase:'importing',filename:'db.7z',processed:128,total_games:256,stats:{imported:128},games_per_second:64,eta_seconds:2}),{status:200,headers:{'content-type':'application/json'}});};
  try {
    await connectCompanion('http://localhost:8790','private-test-token');installCompanionImportProgress(target);
    target.ironwoodCompanionImportProgress();target.ironwoodCompanionImportProgress();await new Promise(resolve=>setTimeout(resolve,10));
    const value=JSON.parse(target.ironwoodCompanionImportProgress());assert.equal(calls,1);assert.equal(value.fraction,0.5);assert.match(value.timing,/64 games\/s/);assert.ok(!JSON.stringify(value).includes('private-test-token'));
    enabled=false;assert.equal(target.ironwoodCompanionImportProgress(),'');assert.equal(calls,1);
  } finally {forgetCompanionConnection();globalThis.window=oldWindow;globalThis.fetch=oldFetch;dom.window.close();}
});
