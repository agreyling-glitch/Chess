import {test} from 'node:test';
import assert from 'node:assert/strict';
import {ArchiveLibrary,importWithRetry} from '../web/companion-library.js';
const connection={endpoint:'http://local:8790',credential:'owner'};
const document=id=>({format:'ironwood-companion-game',version:1,pgn:'[Result "*"]\n\n1. e4 *',metadata:{White:'Player',Result:'*'},record:{id:`browser-${id}`,updatedAt:123,category:'training',favorite:true,custom:{keep:'original'},json:JSON.stringify({live_moves:['e4'],move_notes:['note'],game_analysis:[{score:12}],training:{profile_id:'p'}})}});
const details=()=>({white:'Player',black:'Opponent',result:'*',trainingProfileId:'p'});
function library(handler,receive=async()=>({received:1,duplicates:0,ids:['destination']})){
 return new ArchiveLibrary({...connection,request:handler},{details,prepare:()=>'{"review_pgn":"original"}',exporter:()=>'[Result "*"]\n\n1. e4 *',receive,connection:()=>connection});
}
test('archive library uses complete records for cards and revision-checked updates without losing original IDs',async()=>{
 const source=document('a'),calls=[];
 const store=library(async(path,body)=>{calls.push({path,body});if(path.startsWith('archive/games?'))return {games:[{id:'a',revision:7,updated_at:400}],next:null};if(path==='archive/export')return {documents:[source]};if(path.endsWith('/update'))return {id:'a',revision:8,document:body.document};return {id:'a',revision:7,document:source};});
 const rows=await store.list();assert.equal(rows[0].id,'a');assert.equal(rows[0].favorite,true);assert.equal(rows[0].category,'training');assert.equal(rows[0].json,source.record.json);
 const fresh=await store.read('a'),updated=await store.write({...fresh,favorite:false});
 assert.equal(updated._archive.revision,8);assert.equal(calls.at(-1).body.revision,7);assert.equal(calls.at(-1).body.document.record.id,'browser-a');assert.deepEqual(calls.at(-1).body.document.record.custom,{keep:'original'});assert.equal(calls.at(-1).body.document.pgn,source.pgn);
});
test('moves to Browser commit every destination before deleting archive sources, and copy failure deletes nothing',async()=>{
 const events=[];let fail=true;
 const store=library(async(path,body)=>{if(path.endsWith('/delete')){events.push(`delete:${path}`);assert.equal(body.revision,4);return {}; }return {id:path.split('/').at(-1),revision:4,document:document(path)};},async(docs)=>{events.push('copy');assert.equal(docs[0].record.favorite,true);if(fail)throw Error('quota');return {received:2,duplicates:0,ids:['one','two']};});
 await assert.rejects(store.moveToBrowser(['a','b']),/quota/);assert.deepEqual(events,['copy']);
 events.length=0;fail=false;const result=await store.moveToBrowser(['a','b']);assert.deepEqual(result.ids,['one','two']);assert.equal(events[0],'copy');assert.equal(events.length,3);
});
test('failed archive deletion leaves browser copies available and browser-to-archive import preserves the complete record',async()=>{
 let copied=false;const source=document('a').record;
 const store=library(async(path,body)=>{if(path==='archive/import'){assert.deepEqual(body.documents[0].record,source);return {ids:['a'],duplicates:1,imported:0};}if(path.endsWith('/delete')){assert.ok(copied);throw Error('Revision changed');}return {id:'a',revision:2,document:document('a')};},async()=>{copied=true;return {received:0,duplicates:1,ids:['existing']};});
 assert.deepEqual(await store.importRecords([source]),['a']);await assert.rejects(store.moveToBrowser(['a']),/Revision changed/);assert.equal(copied,true);
});

test('bulk PGN archive imports use bounded paced batches and retain acknowledged counts on later failure',async()=>{
 const requests=[],progress=[];
 const store=library(async(path,body)=>{
   assert.equal(path,'archive/import');requests.push(body.documents.length);
   if(requests.length===2)throw Error('Archive disconnected');
   assert.equal(body.documents[0].record.custom.keep,'original');
   return {imported:126,duplicates:2};
 });
 const records=Array.from({length:140},(_,i)=>document(i).record);
 await assert.rejects(store.importBatchRecords(records,(result,count)=>progress.push({result,count})),/disconnected/);
 assert.deepEqual(requests,[128,12]);
 assert.deepEqual(progress,[{result:{imported:126,duplicates:2},count:128}]);
});

test('archive import retries preflight/network failures, respects rate-limit delays and preserves exact documents',async()=>{
 const documents=[document('retry')],delays=[],calls=[];
 const failures=[new TypeError('Failed to fetch'),Object.assign(Error('rate limited'),{status:429,retryAfterMs:1200}),new DOMException('timeout','TimeoutError')];
 const storage={check(){},async request(path,body){assert.equal(path,'archive/import');calls.push(body.documents);if(failures.length)throw failures.shift();return {imported:0,duplicates:1};}};
 assert.deepEqual(await importWithRetry(storage,documents,async delay=>delays.push(delay)),{imported:0,duplicates:1});
 assert.deepEqual(delays,[1000,1200,15000]);assert.ok(calls.every(value=>value===documents));
 let attempts=0;
 await assert.rejects(importWithRetry({check(){},async request(){attempts++;throw Object.assign(Error('unauthorized'),{status:401});}},documents,async()=>{}),/unauthorized/);
 assert.equal(attempts,1);
 await assert.rejects(importWithRetry({check(){throw Error('Connection changed');},async request(){throw new TypeError('Failed to fetch');}},documents,async()=>{}),/Connection changed/);
});

test('large archive listing loads 2129 complete records in 44 requests and reports progress',async()=>{
 const rows=Array.from({length:2129},(_,i)=>({id:String(i),revision:i+1,updated_at:i}));
 let requests=0;const progress=[];
 const store=library(async(path,body)=>{
   requests++;
   if(path.startsWith('archive/games?')) {
     const after=new URL('http://local/'+path).searchParams.get('after');
     const offset=Number(after||0),games=rows.slice(offset,offset+100);
     return {games,next:offset+100<rows.length?String(offset+100):null};
   }
   assert.equal(path,'archive/export');assert.ok(body.ids.length<=100);
   return {documents:body.ids.map(document)};
 });
 const records=await store.list(()=>true,(count,message)=>{if(!message)progress.push(count);});
 assert.equal(records.length,2129);assert.equal(requests,44);
 assert.equal(records.at(-1).id,'2128');assert.equal(records.at(-1)._archive.revision,2129);
 assert.equal(progress.at(-1),2129);assert.equal(records[0].json,document('0').record.json);
});

test('archive listing splits oversized responses without dropping or reordering games',async()=>{
 const rows=Array.from({length:100},(_,i)=>({id:String(i),revision:1})),sizes=[];
 const store=library(async(path,body)=>{
   if(path.startsWith('archive/games?'))return {games:rows,next:null};
   sizes.push(body.ids.length);
   if(body.ids.length>40)throw Object.assign(Error('Export exceeds 16 MiB; select a smaller batch'),{status:400});
   return {documents:body.ids.map(document)};
 });
 assert.deepEqual((await store.list()).map(record=>record.id),rows.map(row=>row.id));
 assert.deepEqual(sizes,[100,50,25,25,50,25,25]);
});

test('archive pages and matching selection fetch summaries only, with filters encoded',async()=>{
 const requests=[];
 const store=library(async(path,body)=>{
   requests.push({path,body});const url=new URL('http://local/'+path);assert.equal(url.pathname,'/archive/library');
   assert.equal(url.searchParams.get('player'),'A & B');assert.equal(url.searchParams.get('search'),'é4');assert.equal(url.searchParams.get('special'),'short-games');
   return {games:[{id:'id',revision:3,title:'A & B vs Opponent'}],total:1,page:0,counts:{all:1},players:['A & B'],profiles:[]};
 });
 const filters={category:'all',player:'A & B',search:'é4',special:'short-games',sort:'recent',page:0};
 const page=await store.page(filters);assert.equal(page.games[0]._archive.revision,3);assert.equal(page.games[0].json,undefined);
 const selected=await store.selection(filters);assert.equal(selected[0].id,'id');assert.equal(requests.length,2);assert.ok(requests.every(request=>request.body===undefined));
});

test('bulk export loads only requested full records and splits large responses',async()=>{
 const calls=[];const store=library(async(path,body)=>{calls.push(body.ids);assert.equal(path,'archive/export');if(body.ids.length>2)throw Object.assign(Error('too large'),{status:413});return {documents:body.ids.map(document)};});
 const progress=[];const result=await store.readMany(['a','b','c'],(done,total)=>progress.push([done,total]));assert.deepEqual(progress,[[2,3],[3,3]]);assert.deepEqual(result.map(record=>record.id),['a','b','c']);assert.ok(result.every(record=>record.json));assert.deepEqual(calls,[['a','b','c'],['a','b'],['c']]);
});


test('archive filter choices reuse a revision across pages and refresh when the archive changes',async()=>{
 const hints=[];let revision=1;
 const store=library(async path=>{
   const url=new URL('http://local/'+path),hint=url.searchParams.get('facets_revision');hints.push(hint);
   return {games:[],page:0,total:revision,revision,...(hint===String(revision)?{}:{counts:{all:revision},players:[`Player ${revision}`],profiles:[]})};
 });
 const first=await store.page({category:'all',page:0});assert.deepEqual(first.players,['Player 1']);
 const second=await store.page({category:'all',page:1});assert.deepEqual(second.players,first.players);assert.deepEqual(hints,[null,'1']);
 revision=2;const changed=await store.page({category:'all',page:0});assert.equal(changed.counts.all,2);assert.deepEqual(changed.players,['Player 2']);
 await store.page({category:'favorites',page:0});assert.equal(hints.at(-1),null);
});

test('speculative page requests make one attempt and never start rate-limit retries',async()=>{
 let requests=0;
 const store=library(async()=>{requests++;throw Object.assign(Error('busy'),{status:429,retryAfterMs:60000});});
 await assert.rejects(store.page({category:'all',page:1},()=>true,{prefetch:true}),/busy/);assert.equal(requests,1);
});

test('conversion is incremental and later records are not converted before acknowledgement',async()=>{
 const records=Array.from({length:260},(_,i)=>document(i).record),original=structuredClone(records);
 let converted=0,requests=0,yielded=false;
 setTimeout(()=>{yielded=true;},0);
 const store=library(async(_path,body)=>{requests++;assert.ok(yielded);assert.equal(converted,Math.min(requests*128,260));return {imported:body.documents.length,duplicates:0};});
 store.exporter=()=>{converted++;return '1. e4 *';};
 await store.importBatchRecords(records,()=>{});
 assert.equal(requests,3);assert.deepEqual(records,original);
});

test('large selection uses 1000 lightweight rows, reports progress and rejects revision drift',async()=>{
 const calls=[],progress=[];let drift=false;
 const store=library(async path=>{const query=new URL(path,'http://local').searchParams;calls.push(query);const page=Number(query.get('page'));return {total:1001,revision:drift&&page?2:1,games:Array.from({length:page?1:1000},(_,i)=>({id:String(page*1000+i),revision:1,title:'Game'}))};});
 const rows=await store.selection({player:'A'},()=>true,(done,total)=>progress.push([done,total]));
 assert.equal(calls.length,2);assert.ok(calls.every(q=>q.get('limit')==='1000'&&q.get('player')==='A'));assert.equal(rows.length,1001);assert.deepEqual(progress,[[1000,1001],[1001,1001]]);assert.equal(rows[1000]._archive.id,'1000');
 drift=true;await assert.rejects(store.selection({}),/changed during selection/);
});
test('selection falls back once for older servers and ignores cancelled replies',async()=>{
 const limits=[];const store=library(async path=>{const query=new URL(path,'http://local').searchParams;limits.push(query.get('limit'));if(query.get('limit')==='1000')throw Object.assign(Error('Invalid filter'),{status:400});return {total:1,games:[{id:'a',revision:1}]};});
 assert.equal((await store.selection({})).length,1);await store.selection({});assert.deepEqual(limits,['1000','100','100']);
 let active=true;const cancelled=library(async()=>{active=false;return {total:1,games:[{id:'a',revision:1}]};});await assert.rejects(cancelled.selection({},()=>active),/cancelled/);
});
