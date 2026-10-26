import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {JSDOM} from 'jsdom';
import {openScid5,importScidRange,showScid5Import,installScidWindowFrame,importTiming} from '../web/import-scid5.js';
const dom=new JSDOM('',{url:'http://localhost/saved-games/'});
globalThis.window=dom.window;globalThis.Window=dom.window.Window;
const app=await import('../web/pkg/battle_chess.js');
await app.default({module_or_path:readFileSync(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
after(()=>dom.window.close());
function fixture(){return ['si5','sn5','sg5'].map(ext=>{
 const bytes=readFileSync(new URL(`./fixtures/scid/sample.${ext}`,import.meta.url));
 return {name:`sample.${ext}`,size:bytes.length,bytes};
});}
const create=names=>new app.Scid5Reader(names);

test('import speed and ETA include waiting time and replace ETA when stopped or complete',()=>{
 assert.match(importTiming(0,100,0),/Estimating/);
 assert.equal(importTiming(100,200,10000),'10 games/s · About 10s remaining');
 assert.equal(importTiming(100,200,20000),'5 games/s · About 20s remaining');
 assert.equal(importTiming(100,100,10000,'complete'),'10 games/s · Complete');
 assert.match(importTiming(100,200,10000,'stopped'),/Paused$/);
 assert.match(importTiming(100,200,10000,'interrupted'),/Interrupted$/);
 assert.match(importTiming(1,86401,1000),/1d 0h/);
});

test('browser File.bytes is a method; Scid 5 uses bounded slices instead',async()=>{
 const files=fixture().map(({name,size,bytes})=>({name,size,
  bytes(){throw Error('Whole-file bytes() must never be called');},
  arrayBuffer(){throw Error('Whole-file arrayBuffer() must never be called');},
  slice(start,end){return {arrayBuffer:async()=>bytes.subarray(start,end)};},
 }));
 const source=await openScid5(files,create);
 try{for await(const entry of source.entries(0,1))assert.match(await source.pgn(entry),/Ada/);}
 finally{source.free();}
});

test('Scid 5 opens real File objects with the native bytes() method',async()=>{
 const files=fixture().map(({name,bytes})=>new File([bytes],name));
 assert.equal(typeof files[0].bytes,'function');
 const source=await openScid5(files,create);
 try{for await(const entry of source.entries(0,1))assert.match(await source.pgn(entry),/Ada/);}
 finally{source.free();}
});

test('si5 reader decodes pages and streams batches using the rebuilt browser decoder',async()=>{
 const files=fixture(),source=await openScid5(files,create),batches=[],checkpoints=[];
 try{
  assert.equal(source.count,3);
  const result=await importScidRange(source,{}, {saveBatch:async pgns=>{batches.push(pgns);return {saved:pgns.length};},checkpoint:state=>checkpoints.push(state)});
  assert.equal(result.saved,2);assert.equal(result.deleted,1);assert.equal(result.next,3);
  assert.match(batches[0][0],/Develop a knight/);assert.match(batches[0][0],/\(2\. Nc3 Nc6\)/);
  assert.ok(app.prepare_archived_pgn(batches[0][0]));assert.equal(checkpoints.at(-1).next,3);
 }finally{source.free();}
});

test('large virtual files use File.slice with offsets beyond 4 GiB and never read index or games whole',async()=>{
 const files=fixture(),index=Buffer.from(files[0].bytes.subarray(0,56)),blob=files[2].bytes,offset=2**32+12;
 index.writeUInt32LE((blob.length<<15)|1,32);index.writeUInt32LE(12,36);
 const count=12_294_947,reads=[];
 files[0]={name:'sample.si5',size:count*56,arrayBuffer(){throw Error('whole index read');},slice(start,end){reads.push(end-start);return {arrayBuffer:async()=>Buffer.concat(Array.from({length:(end-start)/56},()=>index))};}};
 files[2]={name:'sample.sg5',size:offset+blob.length,arrayBuffer(){throw Error('whole games read');},slice(start,end){assert.equal(start,offset);reads.push(end-start);return {arrayBuffer:async()=>blob};}};
 const source=await openScid5(files,create);
 try{for await(const entry of source.entries(count-1,count)){assert.equal(entry.offset,offset);assert.match(await source.pgn(entry),/Ada/);}
 assert.ok(Math.max(...reads)<=4*1024*1024);assert.equal(source.count,count);
 }finally{source.free();}
});

test('mixed versions, names, missing parts, partial index and out-of-bounds offsets are rejected',async()=>{
 const files=fixture();
 await assert.rejects(openScid5(files.slice(0,1),create),/together/);
 await assert.rejects(openScid5([{...files[0],name:'sample.si4'},...files.slice(1)],create),/Select only/);
 await assert.rejects(openScid5([{...files[0],name:'wrong.si5'},...files.slice(1)],create),/same database/);
 await assert.rejects(openScid5([{...files[0],size:55},...files.slice(1)],create),/truncated/);
 const source=await openScid5([files[0],files[1],{...files[2],size:1}],create);
 try{await assert.rejects(async()=>{for await(const entry of source.entries())void entry;},/outside/);}finally{source.free();}
});

test('filtering, failed games, cancellation and resume checkpoints stay bounded and durable',async()=>{
 const entries=Array.from({length:100},(_,number)=>({number,white:'Ada',black:'Ben',deleted:false}));
 const source={count:100,async *entries(first,end){yield*entries.slice(first,end);},async pgn(e){if(e.number===7)throw Error('bad game 8');return String(e.number);}};
 let stop=false,checkpoint,batches=[];
 const result=await importScidRange(source,{}, {saveBatch:async rows=>{batches.push(rows);stop=true;return {saved:rows.length};},cancelled:()=>stop,checkpoint:state=>{checkpoint=state;}});
 assert.equal(result.saved,32);assert.equal(result.failed,1);assert.equal(result.next,33);assert.equal(result.stopped,true);
 const resumed=await importScidRange(source,{first:checkpoint.next},{saveBatch:async rows=>({saved:rows.length})});
 assert.equal(resumed.saved,67);assert.equal(resumed.next,100);
 const filtered=await importScidRange(source,{player:'Nobody'},{saveBatch:()=>{throw Error('unexpected save');}});
 assert.equal(filtered.filtered,100);
 let committed;
 await assert.rejects(importScidRange(source,{}, {checkpoint:state=>{committed=state;},saveBatch:async()=>{throw Error('QuotaExceededError');}}),/Quota/);
 assert.equal(committed,undefined);
});

test('archive streaming prepares bounded batches of 128 games',async()=>{
 const source={count:300,async *entries(first,end){for(let number=first;number<end;number++)yield {number,white:'Ada',black:'Ben'};},pgn:async entry=>String(entry.number)};
 const batches=[];
 const result=await importScidRange(source,{batchSize:128},{saveBatch:async rows=>{batches.push(rows.length);return {saved:rows.length};}});
 assert.deepEqual(batches,[128,128,44]);assert.equal(result.saved,300);assert.equal(result.next,300);
});

test('Scid dialog reviews checked games through the existing import handoff',async()=>{
 const target=dom.window;target.ironwoodCreateScid5Reader=create;
 target.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 target.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new target.Event('close'));};
 let text;target.ironwoodScidSelectionReady=value=>{text=value;};
 const dialog=await showScid5Import(target,fixture());
 const closeButton=dialog.querySelector('.scid-heading .scid-close');
 assert.ok(dialog.querySelector('.scid-progress-row progress'));
 assert.ok(dialog.querySelector('.scid-progress-row .scid-timing'));
 assert.equal(closeButton.textContent,'×');assert.equal(closeButton.getAttribute('aria-label'),'Close Scid import');
 assert.ok(dialog.querySelector('.scid-actions').compareDocumentPosition(dialog.querySelector('[aria-live]'))&target.Node.DOCUMENT_POSITION_FOLLOWING);
 assert.equal(dialog.querySelector('.scid-actions .scid-close'),null);
 assert.deepEqual([...dialog.querySelector('.scid-content>.scid-actions').children].map(button=>button.textContent),['Browser','Import range','Resume last import','Stop']);
 assert.deepEqual([...dialog.querySelector('.scid-browse-actions').children].map(button=>button.textContent),['Preview first 100','Previous 100','Next 100','Review selected games']);
 assert.ok(dialog.querySelector('.scid-game-list').compareDocumentPosition(dialog.querySelector('.scid-browse-actions'))&target.Node.DOCUMENT_POSITION_FOLLOWING);
 const through=dialog.querySelectorAll('input[type=number]')[1];
 through.value='';through.dispatchEvent(new target.Event('change'));assert.equal(through.value,'3');
 through.value=''; // Actions also normalize an empty field before blur/change.
 const button=label=>[...dialog.querySelectorAll('button')].find(b=>b.textContent===label);
 button('Preview first 100').click();await new Promise(resolve=>setTimeout(resolve,30));
 assert.equal(dialog.querySelectorAll('input[type=checkbox]').length,2);
 assert.equal(through.value,'3');
 dialog.querySelector('input[type=checkbox]').checked=true;button('Review selected games').click();
 await new Promise(resolve=>setTimeout(resolve,30));assert.match(text,/Ada/);assert.equal(dialog.isConnected,false);
});

test('Scid frame drags, resizes at edges/corners and releases pointer listeners on close',()=>{
 const target=dom.window,dialog=target.document.createElement('dialog'),heading=target.document.createElement('h2');
 dialog.append(heading);target.document.body.append(dialog);
 dialog.getBoundingClientRect=()=>({left:100,top:80,width:500,height:400});
 installScidWindowFrame(target,dialog,heading);
 const event=(element,type,x,y)=>element.dispatchEvent(new target.MouseEvent(type,{bubbles:true,button:0,clientX:x,clientY:y}));
 event(heading,'pointerdown',110,90);event(target,'pointermove',210,190);
 assert.equal(dialog.style.left,'200px');assert.equal(dialog.style.top,'180px');event(target,'pointerup',210,190);
 event(dialog.querySelector('.scid-resize-se'),'pointerdown',600,480);event(target,'pointermove',680,540);
 assert.equal(dialog.style.width,'580px');assert.equal(dialog.style.height,'460px');event(target,'pointerup',680,540);
 event(dialog.querySelector('.scid-resize-w'),'pointerdown',100,100);event(target,'pointermove',150,100);
 assert.equal(dialog.style.left,'150px');assert.equal(dialog.style.width,'450px');
 dialog.dispatchEvent(new target.Event('close'));event(target,'pointermove',200,100);assert.equal(dialog.style.width,'450px');dialog.remove();
});

test('named archive destinations preserve the selected endpoint and count',async()=>{
 const target=dom.window;target.ironwoodCreateScid5Reader=create;
 let selected='browser',chosen,finished,total=1382310;
 target.ironwoodImportLocationCatalog=()=>JSON.stringify({selected,locations:[{value:'browser',label:'Browser',enabled:true},{value:'http://win11',label:'Win11',enabled:true},{value:'http://qnap',label:'QNAP',enabled:true}]});
 target.ironwoodSetImportLocation=value=>{selected=value;return true;};
 target.ironwoodScidArchiveDestination=async endpoint=>endpoint;
 target.ironwoodScidArchiveTotal=async()=>total;
 target.ironwoodImportScidBatch=async(pgns,options)=>{chosen=options;total+=pgns.length;return {saved:pgns.length,archiveTotal:total};};
 target.ironwoodFinishScidImport=destination=>{finished=destination;};
 const dialog=await showScid5Import(target,fixture());await new Promise(resolve=>setTimeout(resolve,10));
 const selector=dialog.querySelector('select');assert.deepEqual([...selector.options].map(o=>o.textContent),['Browser','Win11','QNAP']);
 selector.value='http://qnap';selector.dispatchEvent(new target.Event('change'));await new Promise(resolve=>setTimeout(resolve,10));
 assert.equal(dialog.querySelector('.scid-archive-total').textContent,'QNAP: 1,382,310 games');
 [...dialog.querySelectorAll('button')].find(b=>b.textContent==='Import range').click();
 await new Promise(resolve=>setTimeout(resolve,50));
 assert.equal(chosen.destination,'archive');assert.equal(chosen.endpoint,'http://qnap');assert.equal(finished,'archive');
 assert.equal(dialog.querySelector('.scid-archive-total').textContent,'QNAP: 1,382,312 games');
 dialog.close();for(const key of ['ironwoodImportLocationCatalog','ironwoodSetImportLocation','ironwoodScidArchiveDestination','ironwoodScidArchiveTotal'])delete target[key];
});
