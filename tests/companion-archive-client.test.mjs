import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { CompanionArchiveClient, browserDocuments, transferDocuments, archiveCredential } from '../web/companion-archive-client.js';
globalThis.window=globalThis;
const storage=await import(`data:text/javascript,${encodeURIComponent(await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8'))}`);

test('archive HTTP errors retain status and retry delay even when a proxy returns non-JSON',async()=>{
  const client=new CompanionArchiveClient('http://localhost:8790','test-only',async()=>({ok:false,status:429,headers:{get:()=> '2'},json:async()=>{throw new SyntaxError('not JSON');}}));
  await assert.rejects(client.request('archive/import',{documents:[]}),error=>{
    assert.equal(error.status,429);assert.equal(error.retryAfterMs,2000);assert.match(error.message,/429/);return true;
  });
  client.fetcher=async()=>({ok:true,json:async()=>{throw new SyntaxError('not JSON');}});
  await assert.rejects(client.request('capabilities'),/invalid response/);
});
test('archive transfer preserves complete browser records and actual engine identity',()=>{
  const record={id:'original',category:'mine',favorite:true,updatedAt:123,custom:{preserved:true},json:JSON.stringify({board:'fen',history:[],engine_config:{opponent:'Companion',companion_name:'Owner engine'},player_side:'White',live_moves:['e4'],move_notes:['note'],board_marks:[['drawing']],game_analysis:[{score:10}]})};
  const documents=browserDocuments([record],()=> '[White "You"]\n\n1. e4 *',storage.gameDetails);
  assert.deepEqual(documents[0].record,record);assert.equal(documents[0].metadata.Black,'Owner engine');
  documents[0].record.custom.preserved=false;assert.equal(record.custom.preserved,true);
  assert.throws(()=>transferDocuments({format:'ironwood-companion-transfer',version:99,documents}),/Unsupported/);
});
test('archive client authenticates bounded requests without saving or rebinding credentials',async()=>{
  const existing = {endpoint:'http://127.0.0.1:8790',credential:'private'};
  assert.equal(archiveCredential(existing.endpoint,'',existing), 'private');
  assert.throws(()=>archiveCredential('http://other-server:8790','',existing,existing),/Enter the owner credential/);
  let call;
  const client=new CompanionArchiveClient('http://127.0.0.1:8790','disposable-owner',async function(url,options){assert.equal(this,undefined);call={url,options};return {ok:true,json:async()=>({games:[]})};});
  await client.request('archive/import',{pgn:'1. e4 *'});assert.equal(call.options.headers.Authorization,'Bearer disposable-owner');assert.equal(call.options.credentials,'omit');assert.equal(call.options.redirect,'error');assert.ok(!call.url.includes('disposable-owner'));assert.equal(call.options.body,'{"pgn":"1. e4 *"}');
  for(const endpoint of ['file:///data','http://user:pass@localhost/','http://localhost/path','http://localhost/?token=x'])assert.throws(()=>new CompanionArchiveClient(endpoint,'x'));
  client.fetcher=async()=>({ok:false,json:async()=>({code:'conflict',message:'Revision changed'})});await assert.rejects(client.request('archive/games'),/Your changes were not saved/);
});
test('copying from Companion preserves existing browser records and never writes profile ratings',async()=>{
  const records=new Map([['collision',{id:'collision',json:JSON.stringify({board:'original',history:[]}),category:'mine',updatedAt:1}]]);
  const original=structuredClone(records.get('collision'));let writes=0;
  const db={transaction(_store,mode){const staged=[];const tx={objectStore(){return {getAll(){const req={};queueMicrotask(()=>{req.result=[...records.values()];req.onsuccess();});return req;},put(record){staged.push(record);}};}};if(mode==='readwrite')setTimeout(()=>{for(const record of staged)records.set(record.id,record);writes++;tx.oncomplete();},0);return tx;}};
  globalThis.indexedDB={open(){const req={};queueMicrotask(()=>{req.result=db;req.onsuccess();});return req;}};
  // Any attempt to modify profile/rating storage fails this test.
  globalThis.localStorage={getItem(){return null;},setItem(){throw new Error('Archive transfer modified local preferences or training ratings');}};
  const doc={format:'ironwood-companion-game',version:1,pgn:'1. e4 *',record:{id:'collision',json:JSON.stringify({board:'incoming',history:[],move_notes:['kept'],training:{profile_id:'p',profile_name:'Player',side:'White',opponent_elo:1320}}),category:'training',favorite:true,updatedAt:2,custom:{note:'retained'}}};
  assert.deepEqual(await storage.receiveCompanionDocuments([doc]),{received:1,duplicates:0});assert.deepEqual(records.get('collision'),original);assert.equal(records.size,2);assert.equal([...records.values()].find(r=>r.id!=='collision').custom.note,'retained');
  assert.deepEqual(await storage.receiveCompanionDocuments([doc]),{received:0,duplicates:1});assert.equal(writes,1);
  await assert.rejects(storage.receiveCompanionDocuments([doc,{...doc,version:9}]),/Unsupported/);assert.equal(writes,1);
});

test('opening an archive game keeps subsequent saves in that archive and fails closed on concurrent edits',async()=>{
  const before=await storage.companionBrowserRecords(), values=new Map();
  globalThis.localStorage={getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value),removeItem:key=>values.delete(key)};
  const key='ironwood.chess.active-archive-game.v1',endpoint='http://localhost:8790';
  localStorage.setItem(key,JSON.stringify({endpoint,id:'active',revision:2}));
  const old=JSON.stringify({board:'before',live_moves:['e4']}), next=JSON.stringify({board:'after',live_moves:['e4','e5'],move_notes:['retained']});
  let revision=2,writes=0;
  const entries=new Map();Object.assign(storage.analysisLocalCache,{
    async get(id){return structuredClone(entries.get(id));},async list(index,value){return structuredClone([...entries.values()].filter(entry=>entry[index]===value));},
    async write(put=[],remove=[]){put.forEach(entry=>entries.set(entry.id,structuredClone(entry)));remove.forEach(id=>entries.delete(id));},
    async mutate(changes){for(const [id,change]of changes){const value=change(structuredClone(entries.get(id)));value?entries.set(id,structuredClone(value)):entries.delete(id);}}
  });
  const driver={endpoint,check(){},record:record=>record,request:async()=>({id:'active',json:old,_archive:{endpoint,id:'active',revision}}),write:async record=>{writes++;assert.equal(record.json,next);return {...record,_archive:{endpoint,id:'active',revision:3}};}};
  globalThis.ironwoodCompanionLibrary=async()=>driver;
  window.ironwoodStoreCurrentGame(next);await storage.companionBrowserRecords();
  assert.equal(writes,0);await storage.syncArchiveAnalyses(driver,await storage.analysisLocalCache.get(`${endpoint}/active/pending`));
  assert.equal(writes,1);assert.equal(JSON.parse(localStorage.getItem(key)).revision,3);assert.deepEqual(await storage.companionBrowserRecords(),before);
  revision=4;const warn=console.warn;console.warn=()=>{};
  try{window.ironwoodStoreCurrentGame(next);await storage.companionBrowserRecords();await assert.rejects(storage.syncArchiveAnalyses(driver,await storage.analysisLocalCache.get(`${endpoint}/active/pending`)),/changed elsewhere/);}finally{console.warn=warn;}
  assert.equal(writes,1);assert.deepEqual(await storage.companionBrowserRecords(),before);
});

test('successful archive mutations invalidate summaries without exposing credentials; reads and failed writes do not',async()=>{
 const previous=globalThis.window,events=[];
 globalThis.window={CustomEvent:globalThis.CustomEvent,dispatchEvent:event=>events.push(event.detail)};
 try {
   const client=new CompanionArchiveClient('http://local:8790','private-token',async()=>({ok:true,json:async()=>({})}));
   await client.request('archive/library?limit=10');await client.request('archive/export',{ids:['one'],format:'json'});assert.equal(events.length,0);
   await client.request('archive/import',{documents:[]});await client.request('archive/games/one/delete',{revision:1});
   assert.equal(events.length,2);assert.deepEqual(events[1],{source:'archive',endpoint:'http://local:8790',deletedIds:['one']});assert.ok(!JSON.stringify(events).includes('private-token'));
   const rejected=new CompanionArchiveClient('http://local:8790','private-token',async()=>({ok:false,status:409,json:async()=>({code:'conflict',message:'Changed'})}));
   await assert.rejects(rejected.request('archive/games/one/update',{}));assert.equal(events.length,2);
 }finally{globalThis.window=previous;}
});
