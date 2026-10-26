import {JSDOM} from 'jsdom';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import test from 'node:test';

test('large Saved games counts are grouped while page and selection data remain numeric',async()=>{
 const dom=new JSDOM('<body></body>',{url:'https://counts.test.local'}),{window}=dom;
 Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 window.HTMLDialogElement.prototype.close=function(value=''){this.returnValue=value;this.open=false;this.dispatchEvent(new window.Event('close'));};
 const fen='8/8/8/8/8/8/8/K6k w - - 0 1',endpoint='http://local:8790';
 const card=i=>({id:`game-${i}`,title:'Ada vs Ben',white:'Ada',black:'Ben',category:'imported',favorite:false,moves:1234,analyzed:2345,analysisStatus:'Complete',finalFen:fen,updatedAt:1,_archive:{endpoint,id:`game-${i}`,revision:1}});
 const queries=[];let selectingText='';
 const storage={endpoint,check(){},async page(query){queries.push(query);return {games:Array.from({length:10},(_,i)=>card(i)),total:2068097,page:query.page,counts:{all:2068097,imported:2015659},players:[],profiles:[],sources:[],dates:[],openings:[]};},async selection(query,active,progress){assert.equal(query.page,1000);progress(1000,2068097);selectingText=document.querySelector('.game-library-selection').textContent;return Array.from({length:1001},(_,i)=>card(i));},async remove(){assert.fail('Formatting checks must not delete any games');}};
 window.ironwoodCompanionFeatureEnabled=()=>true;window.ironwoodCompanionConnections=()=>[{name:'Win11',endpoint:'http://local:8790',connected:true,archive:true}];window.ironwoodCompanionLibrary=async()=>storage;
 try {
  const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');await import(`data:text/javascript,${encodeURIComponent(source)}#large-counts`);
  await window.ironwoodOpenGameLibrary({windowState:{storageSource:'archive',settings:{category:'all',profile:'*',player:'*',source:'*',date:'*',analysis:'*',special:'*',sort:'recent',page:1000}}});
  await new Promise(resolve=>setTimeout(resolve,200));
  const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent===text);
  assert.match(document.querySelector('[aria-label="Category"]').textContent,/All Games \(2,068,097\)/);
  assert.match(document.querySelector('[aria-label="Category"]').textContent,/Imported Games \(2,015,659\)/);
  assert.match(document.querySelector('.game-library-pager').textContent,/Page 1,001 of 206,810 · 2,068,097 games/);
  assert.equal(queries[0].page,1000);assert.equal(typeof queries[0].page,'number');
  assert.match(document.querySelector('.game-library-entry').textContent,/1,234 moves.*2,345 positions analyzed/);
  button('Select all 2,068,097 matching').click();await new Promise(resolve=>setTimeout(resolve,20));
  assert.match(selectingText,/Selecting 1,000 of 2,068,097/);
  assert.ok(button('Delete selected (1,001)'));button('Delete selected (1,001)').click();
  const confirmation=document.querySelector('#game-delete-dialog');
  assert.match(confirmation.textContent,/Delete 1,001 saved games/);assert.match(confirmation.textContent,/1,001 selected games/);
  assert.ok([...confirmation.querySelectorAll('button')].some(b=>b.textContent==='Delete 1,001 games'));
  [...confirmation.querySelectorAll('button')].find(b=>b.textContent==='Cancel').click();
 } finally {document.querySelector('#game-delete-dialog')?.close('cancel');document.querySelector('#game-library-dialog')?.close();dom.window.close();}
});

test('archive UI pages remotely, keeps cross-page selection, filters, previews, and ignores stale replies',async()=>{
 const dom=new JSDOM('<body></body>',{url:'https://test.local'}),{window}=dom;
 Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};
 const browserRecords=Array.from({length:25},(_,i)=>({id:`browser-${i}`,category:'mine',source:'Browser',updatedAt:100-i,json:JSON.stringify({board:'8/8/8/8/8/8/8/K6k w - - 0 1',history:[],review_pgn:`[White "Browser${i}"]\n[Black "Opponent"]\n\n1. e4 *`,live_moves:Array(i+1).fill('e4')})}));
 globalThis.indexedDB=window.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){return {objectStore(){return {openCursor(){const cursor={},rows=structuredClone(browserRecords);let index=0;const advance=()=>queueMicrotask(()=>{cursor.result=index<rows.length?{value:rows[index++],continue:advance}:null;cursor.onsuccess();});advance();return cursor;}};}};}};request.onsuccess();});return request;}};
 const fen='8/8/8/8/8/8/8/K6k w - - 0 1';
 const cards=Array.from({length:25},(_,i)=>({id:String(i),title:`Player${i} vs Opponent`,white:`Player${i}`,black:'Opponent',category:'imported',favorite:false,moves:i+1,analyzed:0,analysisStatus:'Not analyzed',finalFen:fen,updatedAt:100-i,_archive:{endpoint:'http://local:8790',id:String(i),revision:1}}));
 const queries=[],reads=[];let finishSlow,failNext=false;
 const storage={endpoint:'http://local:8790',check(){},async list(){throw Error('Full archive listing must not be called');},async page(query){queries.push({...query});if(failNext){failNext=false;throw Error("offline");}if(query.source==='slow')await new Promise(resolve=>finishSlow=resolve);let matching=cards.filter(row=>(query.source==='*'||row.title.includes(query.source))&&(query.player==='*'||row.white===query.player));if(query.sort==='moves-desc')matching=matching.toReversed();return {games:structuredClone(matching.slice(query.page*10,query.page*10+10)),total:matching.length,page:query.page,counts:{all:25,imported:25},players:cards.map(row=>row.white),profiles:[],sources:['Archive','Player2','slow'],dates:[]};},async selection(query){assert.equal(query.category,'all');return structuredClone(cards);},async read(id){reads.push(id);return {...cards.find(row=>row.id===id),json:JSON.stringify({board:fen,history:[],live_positions:[fen,fen],live_moves:['e4']})};}};
 window.ironwoodCompanionFeatureEnabled=()=>true;window.ironwoodCompanionConnections=()=>[{name:'Win11',endpoint:'http://local:8790',connected:true,archive:true}];window.ironwoodCompanionLibrary=async()=>storage;
 const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');await import(`data:text/javascript,${encodeURIComponent(source)}`);
 await window.ironwoodOpenGameLibrary();
 const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent===text);
 const click=text=>{if(['Browser','Archive'].includes(text)){const field=document.querySelector('[aria-label="Game storage location"]');field.value=text==='Browser'?'browser':'http://local:8790';field.dispatchEvent(new window.Event('change'));return;}assert.ok(button(text),text);button(text).click();};
 const settle=()=>new Promise(resolve=>setTimeout(resolve,200));
 const chooseSource=value=>{document.querySelector('[aria-label="Source"]').click();const input=document.querySelector('[aria-label="Find a source"]');input.value=value;input.dispatchEvent(new window.Event('input'));[...document.querySelectorAll('.library-player-option')].find(option=>option.textContent===value).click();};
 chooseSource('Browser');
 document.querySelector('.library-player-trigger').click();const browserPlayer=document.querySelector('[aria-label="Find a player"]');browserPlayer.value='Opponent';browserPlayer.dispatchEvent(new window.Event('input'));document.querySelector('.library-player-option').click();
 const browserAnalysis=[...document.querySelectorAll('label')].find(label=>label.textContent.startsWith('Analysis ')).querySelector('select');browserAnalysis.value='unanalyzed';browserAnalysis.dispatchEvent(new window.Event('change'));
 const browserSort=document.querySelector('[aria-label="Sort saved games"]');browserSort.value='moves-desc';browserSort.dispatchEvent(new window.Event('change'));
 click('Next ▶');document.querySelector('.game-library-select').click();
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);
 click('Archive');await settle();
 assert.equal(document.querySelector('[aria-label="Source"]').textContent,'All sources');
 assert.equal(document.querySelector('[aria-label="Sort saved games"]').value,'recent');

 assert.equal(document.querySelectorAll('.game-library-entry').length,10);assert.equal(reads.length,0);assert.equal(queries.length,1);
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 1 of 3 · 25 games/);assert.match(document.querySelector('[aria-label="Category"]').textContent,/All Games \(25\)/);
 await new Promise(resolve=>setTimeout(resolve,150));
 assert.equal(queries.filter(query=>query.page===1&&query.source==='*').length,1);
 document.querySelector('.game-library-select').click();click('Next ▶');
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);
 assert.equal(document.querySelectorAll('.game-library-entry').length,10);
 await settle();
 assert.equal(queries.filter(query=>query.page===1&&query.source==='*').length,1);

 assert.ok(queries.some(query=>query.page===1));assert.ok(button('Delete selected (1)'));assert.match(document.querySelector('.game-library-main strong').textContent,/Player10/);
 document.querySelector('.game-library-select').click();assert.ok(button('Delete selected (2)'));
 click('Browser');
 assert.equal(document.querySelector('[aria-label="Source"]').textContent,'Browser');
 assert.equal(document.querySelector('[aria-label="Sort saved games"]').value,'moves-desc');
 assert.equal(document.querySelector('.library-player-trigger').textContent,'Opponent');
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
 document.querySelector('[aria-label="Refresh saved games"]').click();await settle();assert.ok(button('Delete selected (2)'));
 click('Select all 25 matching');await settle();assert.ok(button('Delete selected (25)'));click('Clear selection');assert.ok(button('Delete selected (0)').disabled);assert.ok(button('Export ▾').disabled);assert.ok(button('Clear selection').disabled);assert.ok([...document.querySelectorAll('.game-library-select')].every(box=>!box.checked));assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);click('Select all 25 matching');await settle();click('Delete selected (25)');assert.match(document.querySelector('#game-delete-dialog').textContent,/Delete 25 saved games/);document.querySelector('#game-delete-dialog').close();
 const sort=document.querySelector('[aria-label="Sort saved games"]');sort.value='moves-desc';sort.dispatchEvent(new window.Event('change'));await settle();assert.equal(queries.at(-1).sort,'moves-desc');assert.match(document.querySelector('.game-library-main strong').textContent,/Player24/);
 document.querySelector('.game-library-preview').click();await new Promise(resolve=>setTimeout(resolve,10));assert.deepEqual(reads,['24']);assert.equal([...document.querySelectorAll('#game-preview-dialog button')].find(b=>b.textContent==='Play').disabled,false);document.querySelector('#game-preview-dialog').close();
 const search=chooseSource;
 search('slow');await settle();assert.ok(finishSlow);search('Player2');await settle();assert.match(document.querySelector('.game-library-pager').textContent,/6 games/);finishSlow();await new Promise(resolve=>setTimeout(resolve,20));assert.match(document.querySelector('.game-library-pager').textContent,/6 games/);assert.equal(queries.at(-1).source,'Player2');
 assert.equal(reads.length,1);
 document.querySelector('.game-library-select').click();assert.ok(button('Delete selected (1)'));
 click('Browser');cards.splice(cards.findIndex(card=>card.id==='24'),1);
 window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'archive',endpoint:storage.endpoint,deletedIds:['24']}}));
 click('Archive');assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();
 assert.match(document.querySelector('.game-library-pager').textContent,/5 games/);assert.ok(button('Delete selected (0)'));
 assert.doesNotMatch(document.querySelector('.game-library-list').textContent,/Player24/);
 // A connection change discards summaries and selection, including cached pages.
 document.querySelector('.game-library-select').click();window.dispatchEvent(new window.Event('ironwood-companion-connections-changed'));
 assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();assert.ok(button('Delete selected (0)'));
 browserRecords.push({id:'browser-new',category:'mine',source:'Browser',updatedAt:999,json:JSON.stringify({board:fen,history:[],review_pgn:'[White "Browser new"]\n[Black "Opponent"]\n\n1. e4 *',live_moves:['e4']})});
 window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'browser'}}));
 click('Browser');assert.equal(document.querySelectorAll('.game-library-entry').length,0);await settle();
 assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3 · 26 games/);assert.ok(button('Delete selected (1)'));
 document.querySelector('#game-library-dialog').close();
 await window.ironwoodOpenGameLibrary();
 assert.equal(document.querySelector('[aria-label="Source"]').textContent,'Browser');assert.match(document.querySelector('.game-library-pager').textContent,/Page 2 of 3/);assert.ok(button('Delete selected (1)'));
 document.querySelector('#game-library-dialog').close();dom.window.close();
});

test('Saved games isolates two archive locations with identical game IDs',async()=>{
 const dom=new JSDOM('<body/>',{url:'https://locations.test.local'}),{window}=dom;Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};window.HTMLCanvasElement.prototype.getContext=()=>null;
 const browserRecords=[];globalThis.indexedDB=window.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){return {objectStore(){return {openCursor(){const cursor={};queueMicrotask(()=>{cursor.result=null;cursor.onsuccess();});return cursor;}};}};}};request.onsuccess();});return request;}};
 const locations=[{name:'Win11',endpoint:'http://win:8790',connected:true,archive:true},{name:'QNAP',endpoint:'http://nas:8790',connected:true,archive:false}];window.ironwoodCompanionFeatureEnabled=()=>true;window.ironwoodCompanionConnections=()=>locations;
 const calls=[];let finishWin;const fen='8/8/8/8/8/8/8/K6k w - - 0 1';
 const stores=new Map(locations.map(location=>{const card={id:'same-id',title:location.name+' game',white:location.name,black:'Opponent',category:'imported',moves:10,favorite:false,finalFen:fen,_archive:{endpoint:location.endpoint,id:'same-id',revision:1}};return [location.endpoint,{endpoint:location.endpoint,check(){},async page(query){calls.push(['page',location.endpoint]);if(location.name==='Win11'&&query.source==='slow')await new Promise(r=>finishWin=r);return {games:[card],page:0,total:1,counts:{all:1},players:[],profiles:[],sources:['slow'],dates:[],openings:[]};},async read(id){calls.push(['read',location.endpoint,id]);return {...card,json:JSON.stringify({board:fen,live_moves:['e4']})};},async write(record){calls.push(['write',location.endpoint,record.id]);return record;}}];}));
 window.ironwoodCompanionLibrary=async options=>{assert.ok(options.endpoint,'record operations must specify their archive');return stores.get(options.endpoint);};
 const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');await import(`data:text/javascript,${encodeURIComponent(source)}#two-locations`);
 try{
  await window.ironwoodOpenGameLibrary();const selector=document.querySelector('[aria-label="Game storage location"]');const settle=()=>new Promise(r=>setTimeout(r,250));const choose=value=>{selector.value=value;selector.dispatchEvent(new window.Event('change'));};
  assert.deepEqual([...selector.options].map(o=>o.textContent),['Browser','Win11','QNAP']);choose(locations[0].endpoint);await settle();assert.match(document.querySelector('.game-library-list').textContent,/Win11 vs Opponent/);
  document.querySelector('.game-library-select').click();choose(locations[1].endpoint);await settle();assert.match(document.querySelector('.game-library-list').textContent,/QNAP vs Opponent/);assert.equal(document.querySelector('.game-library-select').checked,false);
  document.querySelector('.game-library-preview').click();await settle();assert.deepEqual(calls.at(-1),['read',locations[1].endpoint,'same-id']);document.querySelector('#game-preview-dialog').close();
  document.querySelector('.game-library-select').click();const state=await window.ironwoodLibraryPopupState();assert.equal(state.storageEndpoint,locations[1].endpoint);assert.deepEqual(state.selected,['same-id']);
  choose(locations[0].endpoint);await settle();assert.equal(document.querySelector('.game-library-select').checked,true);assert.match(document.querySelector('.game-library-list').textContent,/Win11 vs Opponent/);assert.equal(locations[0].archive,true);
  choose('browser');await settle();assert.equal(document.querySelectorAll('.game-library-select').length,0);
 }finally{document.querySelector('#game-library-dialog')?.close();dom.window.close();}
});
