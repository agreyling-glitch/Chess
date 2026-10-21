import {JSDOM} from 'jsdom';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import test from 'node:test';

test('archive UI pages remotely, keeps cross-page selection, filters, previews, and ignores stale replies',async()=>{
 const dom=new JSDOM('<body></body>',{url:'https://test.local'}),{window}=dom;
 Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};
 const browserRecords=Array.from({length:25},(_,i)=>({id:`browser-${i}`,category:'mine',updatedAt:100-i,json:JSON.stringify({board:'8/8/8/8/8/8/8/K6k w - - 0 1',history:[],review_pgn:`[White "Browser${i}"]\n[Black "Opponent"]\n\n1. e4 *`,live_moves:Array(i+1).fill('e4')})}));
 globalThis.indexedDB=window.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){return {objectStore(){return {openCursor(){const cursor={},rows=structuredClone(browserRecords);let index=0;const advance=()=>queueMicrotask(()=>{cursor.result=index<rows.length?{value:rows[index++],continue:advance}:null;cursor.onsuccess();});advance();return cursor;}};}};}};request.onsuccess();});return request;}};
 const fen='8/8/8/8/8/8/8/K6k w - - 0 1';
 const cards=Array.from({length:25},(_,i)=>({id:String(i),title:`Player${i} vs Opponent`,white:`Player${i}`,black:'Opponent',category:'imported',favorite:false,moves:i+1,analyzed:0,analysisStatus:'Not analyzed',finalFen:fen,updatedAt:100-i,_archive:{endpoint:'http://local:8790',id:String(i),revision:1}}));
 const queries=[],reads=[];let finishSlow,failNext=false;
 const storage={endpoint:'http://local:8790',check(){},async list(){throw Error('Full archive listing must not be called');},async page(query){queries.push({...query});if(failNext){failNext=false;throw Error("offline");}if(query.search==='slow')await new Promise(resolve=>finishSlow=resolve);let matching=cards.filter(row=>row.title.toLowerCase().includes(query.search.toLowerCase())&&(query.player==='*'||row.white===query.player));if(query.sort==='moves-desc')matching=matching.toReversed();return {games:structuredClone(matching.slice(query.page*10,query.page*10+10)),total:matching.length,page:query.page,counts:{all:25,imported:25},players:cards.map(row=>row.white),profiles:[]};},async selection(query){assert.equal(query.category,'all');return structuredClone(cards);},async read(id){reads.push(id);return {...cards.find(row=>row.id===id),json:JSON.stringify({board:fen,history:[],live_positions:[fen,fen],live_moves:['e4']})};}};
 window.ironwoodCompanionFeatureEnabled=()=>true;window.ironwoodCompanionLibrary=async()=>storage;
 const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');await import(`data:text/javascript,${encodeURIComponent(source)}`);
 await window.ironwoodOpenGameLibrary();
 const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent===text);
 const click=text=>{assert.ok(button(text),text);button(text).click();};
 const settle=()=>new Promise(resolve=>setTimeout(resolve,200));
 const browserSearch=document.querySelector('[aria-label="Search saved games"]');browserSearch.value='Browser';browserSearch.dispatchEvent(new window.Event('input'));
 const browserPlayer=document.querySelector('label select');browserPlayer.value='Opponent';browserPlayer.dispatchEvent(new window.Event('change'));
 const browserAnalysis=[...document.querySelectorAll('label')].find(label=>label.textContent.startsWith('Analysis ')).querySelector('select');browserAnalysis.value='unanalyzed';browserAnalysis.dispatchEvent(new window.Event('change'));
 const browserSort=document.querySelector('[aria-label="Sort saved games"]');browserSort.value='moves-desc';browserSort.dispatchEvent(new window.Event('change'));
 click('Next ▶');document.querySelector('.game-library-select').click();
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);
 click('Archive');await settle();
 assert.equal(document.querySelector('[aria-label="Search saved games"]').value,'');
 assert.equal(document.querySelector('[aria-label="Sort saved games"]').value,'recent');

 assert.equal(document.querySelectorAll('.game-library-entry').length,10);assert.equal(reads.length,0);assert.equal(queries.length,1);
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 1 of 3 · 25 games/);assert.ok(button('All Games (25)'));
 await new Promise(resolve=>setTimeout(resolve,150));
 assert.equal(queries.filter(query=>query.page===1&&!query.search).length,1);
 document.querySelector('.game-library-select').click();click('Next ▶');
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);
 assert.equal(document.querySelectorAll('.game-library-entry').length,10);
 await settle();
 assert.equal(queries.filter(query=>query.page===1&&!query.search).length,1);

 assert.ok(queries.some(query=>query.page===1));assert.ok(button('Delete selected (1)'));assert.match(document.querySelector('.game-library-main strong').textContent,/Player10/);
 document.querySelector('.game-library-select').click();assert.ok(button('Delete selected (2)'));
 click('Browser');
 assert.equal(document.querySelector('[aria-label="Search saved games"]').value,'Browser');
 assert.equal(document.querySelector('[aria-label="Sort saved games"]').value,'moves-desc');
 assert.equal(document.querySelector('label select').value,'Opponent');
 assert.equal([...document.querySelectorAll('label')].find(label=>label.textContent.startsWith('Analysis ')).querySelector('select').value,'unanalyzed');
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);assert.ok(button('Delete selected (1)'));
 assert.match(document.querySelector('.game-library-main strong').textContent,/Browser14/);
 click('Archive');
 // Cached archive cards and selection are restored synchronously, before refresh settles.
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);assert.ok(button('Delete selected (2)'));
 assert.match(document.querySelector('.game-library-main strong').textContent,/Player10/);
 await settle();

 // A failed background refresh keeps the cached page and selection usable.
 click('Browser');failNext=true;click('Archive');
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);
 await settle();assert.equal(document.querySelectorAll('.game-library-entry').length,10);
 assert.match(document.querySelector('.game-library-notice').textContent,/cached games.*offline/);
 click('Refresh');await settle();assert.ok(button('Delete selected (2)'));
 click('Select all 25 matching');await settle();assert.ok(button('Delete selected (25)'));click('Delete selected (25)');assert.match(document.querySelector('#game-delete-dialog').textContent,/Delete 25 saved games/);document.querySelector('#game-delete-dialog').close();
 const sort=document.querySelector('[aria-label="Sort saved games"]');sort.value='moves-desc';sort.dispatchEvent(new window.Event('change'));await settle();assert.equal(queries.at(-1).sort,'moves-desc');assert.match(document.querySelector('.game-library-main strong').textContent,/Player24/);
 document.querySelector('.game-library-preview').click();await new Promise(resolve=>setTimeout(resolve,10));assert.deepEqual(reads,['24']);assert.equal([...document.querySelectorAll('#game-preview-dialog button')].find(b=>b.textContent==='Auto play').disabled,false);document.querySelector('#game-preview-dialog').close();
 const search=value=>{const input=document.querySelector('[aria-label="Search saved games"]');input.value=value;input.dispatchEvent(new window.Event('input'));};
 search('slow');await settle();assert.ok(finishSlow);search('Player2');await settle();assert.match(document.querySelector('.game-library-pager').textContent,/6 games/);finishSlow();await new Promise(resolve=>setTimeout(resolve,20));assert.match(document.querySelector('.game-library-pager').textContent,/6 games/);assert.equal(queries.at(-1).search,'Player2');
 assert.equal(reads.length,1);
 document.querySelector('.game-library-select').click();assert.ok(button('Delete selected (1)'));
 click('Browser');cards.splice(cards.findIndex(card=>card.id==='24'),1);
 window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'archive',endpoint:storage.endpoint,deletedIds:['24']}}));
 click('Archive');assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();
 assert.match(document.querySelector('.game-library-pager').textContent,/5 games/);assert.ok(button('Delete selected (0)'));
 assert.doesNotMatch(document.querySelector('.game-library-list').textContent,/Player24/);
 // A connection change discards summaries and selection, including cached pages.
 document.querySelector('.game-library-select').click();window.dispatchEvent(new window.Event('ironwood-companion-connection-changed'));
 assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();assert.ok(button('Delete selected (0)'));
 browserRecords.push({id:'browser-new',category:'mine',updatedAt:999,json:JSON.stringify({board:fen,history:[],review_pgn:'[White "Browser new"]\n[Black "Opponent"]\n\n1. e4 *',live_moves:['e4']})});
 window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'browser'}}));
 click('Browser');assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3 · 26 games/);assert.ok(button('Delete selected (1)'));
 document.querySelector('#game-library-dialog').close();
 await window.ironwoodOpenGameLibrary();
 assert.equal(document.querySelector('[aria-label="Search saved games"]').value,'Browser');assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);assert.ok(button('Delete selected (1)'));
 document.querySelector('#game-library-dialog').close();dom.window.close();
});
