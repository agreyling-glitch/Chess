import {test} from 'node:test';import assert from 'node:assert/strict';import {JSDOM} from 'jsdom';
test('Saved Games source/date dropdowns combine, normalize dates, handle missing values and restore filters',async()=>{
 const {window}=new JSDOM('<body></body>',{url:'https://test.local'});Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};window.ironwoodCompanionFeatureEnabled=()=>false;
 const records=[['A','Lichess','2026.10.20'],['B',null,'2026.10.21'],['C','Lichess','????.??.??']].map(([name,source,date],index)=>({id:name,source,category:'imported',updatedAt:index,json:JSON.stringify({board:'8/8/8/8/8/8/8/K6k w - - 0 1',live_moves:['e4'],review_pgn:`[White "${name}"]\n[Black "Opponent"]\n[Date "${date}"]\n\n1. e4 *`})}));
 const before=JSON.stringify(records);globalThis.indexedDB=window.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){return {objectStore(){return {openCursor(){const cursor={};let index=0;const next=()=>queueMicrotask(()=>{cursor.result=index<records.length?{value:records[index++],continue:next}:null;cursor.onsuccess();});next();return cursor;}};}};}};request.onsuccess();});return request;}};
 const {savedGameSourceLabel}=await import('../web/game-storage.js?source-date-test');assert.equal(savedGameSourceLabel({source:null,venue:'Chess club'}),'Chess club');assert.equal(savedGameSourceLabel({source:'Lichess',venue:'Chess club'}),'Lichess');await window.ironwoodOpenGameLibrary();
 assert.equal(document.querySelector('[aria-label="Search saved games"]'),null);assert.equal(document.querySelector('[placeholder="Search players, source, or date"]'),null);
 const names=()=>[...document.querySelectorAll('.game-library-main strong')].map(node=>node.textContent).sort();
 const choose=(kind,text,query=text)=>{document.querySelector(`[aria-label="${kind}"]`).click();const input=document.querySelector(`[aria-label="Find a ${kind.toLowerCase()}"]`);input.value=query;input.dispatchEvent(new window.Event('input'));const option=[...document.querySelectorAll('.library-player-option')].find(option=>option.textContent===text);assert.ok(option,text);option.click();};
 choose('Source','Lichess');assert.deepEqual(names(),['A vs Opponent','C vs Opponent']);
 choose('Date','2026-10-20','2026.10');assert.deepEqual(names(),['A vs Opponent']);
 document.querySelector('.game-library-select').click();choose('Date','Unknown dates');assert.deepEqual(names(),['C vs Opponent']);assert.equal([...document.querySelectorAll('button')].find(button=>button.textContent==='Export ▾').disabled,true);
 document.querySelector('#game-library-dialog').close();await window.ironwoodOpenGameLibrary();assert.equal(document.querySelector('[aria-label="Source"]').textContent,'Lichess');assert.equal(document.querySelector('[aria-label="Date"]').textContent,'Unknown dates');
 choose('Date','All dates');choose('Source','Unknown sources');assert.deepEqual(names(),['B vs Opponent']);
 choose('Source','All sources');document.querySelector('[aria-label="Date"]').click();assert.deepEqual([...document.querySelectorAll('.library-player-option')].map(option=>option.textContent),['All dates','2026-10-21','2026-10-20','Unknown dates']);
 assert.equal(JSON.stringify(records),before);document.querySelector('#game-library-dialog').close();
});
