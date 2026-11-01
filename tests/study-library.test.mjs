import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {JSDOM} from 'jsdom';
import init,{prepare_archived_pgn,merge_saved_studies,export_saved_game} from '../web/pkg/battle_chess.js';

await init({module_or_path:await readFile(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
const dom=new JSDOM('<body></body>',{url:'http://localhost:8788/play/'});
Object.assign(globalThis,{window:dom.window,document:dom.window.document,localStorage:dom.window.localStorage});
window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};
window.ironwoodCompanionFeatureEnabled=()=>false;
window.ironwoodMergeSavedStudies=merge_saved_studies;
window.ironwoodExportSavedGame=export_saved_game;
const records=new Map();let failWrite=false;
globalThis.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){
  let aborted=false;const staged=new Map();
  const tx={abort(){aborted=true;queueMicrotask(()=>tx.onabort?.());},objectStore(){
    const complete=()=>{if(aborted)return;if(failWrite&&staged.size){failWrite=false;tx.error=Error('Quota exceeded');tx.onabort?.();return;}for(const [id,value]of staged)value===null?records.delete(id):records.set(id,value);tx.oncomplete?.();};
    const read=value=>{const req={};setImmediate(()=>{req.result=structuredClone(value());req.onsuccess?.();queueMicrotask(complete);});return req;};
    return {get:id=>read(()=>records.get(id)),getAll:()=>read(()=>[...records.values()]),put:record=>staged.set(record.id,structuredClone(record)),delete:id=>staged.set(id,null),openCursor(){const req={};const values=[...records.values()];let index=0;const next=()=>setImmediate(()=>{req.result=index<values.length?{value:structuredClone(values[index++]),continue:next}:null;req.onsuccess?.();});next();return req;}};
  }};return tx;
}};request.onsuccess();});return request;}};
window.indexedDB=globalThis.indexedDB;
const storage=await import('../web/game-storage.js?study-library');
const pgn=(chapter,moves='1. e4 e5',id='abcd1234')=>`[Event "Study: ${chapter}"]\n[StudyName "Study"]\n[StudyAuthor "Ada"]\n[ChapterName "${chapter}"]\n[ChapterURL "https://lichess.org/study/${id}/${chapter}"]\n[Result "*"]\n\n${moves} *`;
const legacy=(id,chapter,time,moves)=>({id,category:'imported',updatedAt:time,json:prepare_archived_pgn(pgn(chapter,moves))});

test('study identity uses source IDs, never shared titles',()=>{
 assert.equal(storage.savedStudyId(prepare_archived_pgn(pgn('FIRST001'))),'study-lichess-abcd1234');
 assert.notEqual(storage.savedStudyId(prepare_archived_pgn(pgn('FIRST001','1.d4','other123'))),'study-lichess-abcd1234');
 assert.equal(storage.savedStudyId(JSON.stringify({review_pgn:'[StudyName "Study"]\n\n1.e4 *'})),null);
});

test('history consolidates chapter saves atomically, shows Studies, and keeps later saves together',async()=>{
 const one=legacy('old-one','FIRST001',1),two=legacy('old-two','SECOND02',2,'1.d4 d5');
 const annotated=JSON.parse(one.json);annotated.move_notes[1]='My note';one.json=JSON.stringify(annotated);one.favorite=true;one.collectionIds=['opening'];
 records.set(one.id,one);records.set(two.id,two);
 records.set('ordinary',{id:'ordinary',category:'imported',updatedAt:3,json:prepare_archived_pgn('[White "Alice"]\n[Black "Bob"]\n\n1.e4 e5 *')});
 await window.ironwoodOpenGameLibrary();
 assert.equal(records.size,2,document.body.textContent);
 const combined=records.get('study-lichess-abcd1234');
 assert.equal(combined.category,'studies');assert.equal(combined.favorite,true);assert.deepEqual(combined.collectionIds,['opening']);
 assert.equal(combined.studySavedCopies.length,2);assert.equal(combined.studySavedCopies[0].json,one.json);
 const study=JSON.parse(combined.json).study;assert.equal(study.chapters.length,2);assert.equal(study.chapters[0].nodes[1].local_note,'My note');
 for(let i=0;i<30&&!document.querySelector('[aria-label="Show"] option[value="studies"]')?.textContent.includes('(1)');i++)await new Promise(resolve=>setTimeout(resolve,10));
 const category=document.querySelector('[aria-label="Show"]');assert.ok([...category.options].some(option=>option.value==='studies'&&option.textContent==='Studies (1)'));
 category.value='studies';category.dispatchEvent(new window.Event('change'));
 assert.equal(document.querySelectorAll('.game-library-entry').length,1);
 assert.match(document.querySelector('.game-library-title').textContent,/Study · 2 chapters/);
 assert.equal(document.querySelector('.game-library-players').hidden,true);
 assert.equal(document.querySelector('.game-library-result').textContent,'Lichess study');
 window.ironwoodStoreCurrentGame(two.json);await storage.companionBrowserRecords();
 assert.equal(records.size,2);assert.equal(JSON.parse(records.get(combined.id).json).study.chapters.length,2);
 document.querySelector('#game-library-dialog').close();
});

test('a failed consolidation keeps every original record',async()=>{
 const extra=legacy('old-third','THIRD003',4,'1.c4');records.set(extra.id,extra);
 const before=JSON.stringify([...records]);failWrite=true;
 window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'browser'}}));
 // Opening reports the storage failure; the transaction must leave all saves intact.
 await window.ironwoodOpenGameLibrary();
 assert.equal(JSON.stringify([...records]),before);
 document.querySelector('#game-library-dialog')?.close();
});
