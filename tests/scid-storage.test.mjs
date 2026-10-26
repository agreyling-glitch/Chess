import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';

test('Scid storage migrates legacy fingerprints, deduplicates on disk, and keeps failed batches atomic',async()=>{
 const dom=new JSDOM('',{url:'http://localhost/'}),target=dom.window;
 Object.assign(globalThis,{window:target,localStorage:target.localStorage});
 const pgn=white=>`[White "${white}"]\n[Black "Ben"]\n[Date "2026.10.24"]\n\n1. e4 e5 2. Nf3 Nc6 *`;
 const records=new Map([['legacy',{id:'legacy',category:'imported',json:JSON.stringify({review_pgn:pgn('Ada')})}]]);
 let indexed=false,fail=false,lookups=0,version;
 const db={objectStoreNames:{contains:()=>true},close(){},transaction(_name,mode){
  let pending=0,aborted=false,timer;
  const draft=mode==='readwrite'?new Map(structuredClone([...records])):records;
  const tx={error:null,abort(){aborted=true;clearTimeout(timer);queueMicrotask(()=>tx.onabort?.());}};
  const finish=()=>{clearTimeout(timer);timer=setTimeout(()=>{if(aborted||pending)return;if(mode==='readwrite'){records.clear();for(const [k,v]of draft)records.set(k,v);}tx.oncomplete?.();},0);};
  const request=action=>{const r={};pending++;queueMicrotask(()=>{if(aborted)return;try{r.result=action();r.onsuccess?.();}catch(error){tx.error=error;tx.abort();}pending--;finish();});return r;};
  const store={indexNames:{contains:()=>indexed},createIndex(){indexed=true;},
   index(){assert.equal(indexed,true);return {getKey:fingerprint=>{lookups++;return request(()=>[...draft.values()].find(r=>r.fingerprint===fingerprint)?.id);}};},
   put(record){if(fail){tx.error=Error('Quota exceeded');tx.abort();return;}draft.set(record.id,structuredClone(record));},
   openCursor(){const r={},rows=[...draft.values()];let n=0;pending++;
    const advance=()=>queueMicrotask(()=>{r.result=n<rows.length?{value:rows[n++],update:record=>draft.set(record.id,record),continue:advance}:null;r.onsuccess?.();if(!r.result){pending--;finish();}});advance();return r;
   },
  };
  tx.objectStore=()=>store;finish();return tx;
 }};
 target.indexedDB=globalThis.indexedDB={open(_name,v){version=v;const r={};queueMicrotask(()=>{
  r.result=db;r.transaction=db.transaction('games','readwrite');r.onupgradeneeded();r.transaction.oncomplete=()=>r.onsuccess();
 });return r;}};
 target.ironwoodPrepareArchivedPgn=text=>{if(text==='bad')throw Error('Invalid PGN');return JSON.stringify({review_pgn:text});};
 const {saveScidBatch,importFingerprint}=await import('../web/game-storage.js');
 try{
  const result=await saveScidBatch([pgn('Ada'),pgn('Cara'),pgn('Cara'),'bad','1. e4 e5 *']);
  assert.equal(version,2);assert.equal(records.get('legacy').fingerprint,importFingerprint(pgn('Ada')));
  assert.deepEqual(result,{saved:1,duplicates:2,discarded:1,failed:1,firstError:'Invalid PGN'});
  assert.equal(records.size,2);assert.ok(lookups>=2);
  assert.equal((await saveScidBatch([pgn('Cara')])).duplicates,1);
  const before=structuredClone([...records]);fail=true;
  await assert.rejects(saveScidBatch([pgn('Dina')]),/Quota/);assert.deepEqual([...records],before);
  fail=false;assert.equal((await saveScidBatch([pgn('Dina')])).saved,1);
  await assert.rejects(saveScidBatch(Array(33).fill(pgn('Ada'))),/32/);
  const browserBefore=structuredClone([...records]);let imported=0;
  await assert.rejects(saveScidBatch([pgn('Eve')],{destination:'archive',endpoint:'http://archive'}),/disabled/);
  target.ironwoodCompanionFeatureEnabled=()=>true;
  let connections=0;
  target.ironwoodCompanionLibrary=async()=>{connections++;return {endpoint:'http://archive',check(){},
   request:async path=>path==='archive/stats'?{games:1382310}:{services:{archive:true}},
   importPgnBatch:async(batch,progress,onStatus)=>{assert.ok(batch.every(pgn=>typeof pgn==='string'));onStatus?.('Connection interrupted. Retrying automatically in 1 seconds…');imported+=batch.length;progress({imported:1,duplicates:batch.length-1},batch.length);onStatus?.('');},
  };};
  assert.equal(await target.ironwoodScidArchiveDestination('http://archive'),'http://archive');
  assert.equal(await target.ironwoodScidArchiveTotal('http://archive'),1382310);
  await assert.rejects(target.ironwoodScidArchiveTotal('http://other'),/connection changed/);
  target.ironwoodPrepareArchivedPgn=()=>{throw Error('Archive imports must bypass Browser record preparation');};
  const archived=await saveScidBatch([pgn('Eve'),pgn('Eve')],{destination:'archive',endpoint:'http://archive'});
  assert.equal(archived.saved,1);assert.equal(archived.duplicates,1);assert.equal(imported,2);
  const connected=connections,messages=[];
  const larger=await saveScidBatch(Array(128).fill(pgn('Eve')),{destination:'archive',endpoint:'http://archive',onStatus:message=>messages.push(message)});
  assert.equal(connections,connected,'Archive batches must reuse the client and pacing budget');
  assert.equal(larger.duplicates,127);assert.ok(messages.some(message=>/Retrying/.test(message)));assert.equal(messages.at(-1),'');
  assert.deepEqual([...records],browserBefore,'Archive imports must not create Browser copies');
  await assert.rejects(saveScidBatch([pgn('Eve')],{destination:'archive',endpoint:'http://other'}),/connection changed/);
  target.ironwoodCompanionFeatureEnabled=()=>false;
  assert.equal(await target.ironwoodScidArchiveDestination('http://archive'),null);
  assert.equal(await target.ironwoodScidArchiveTotal('http://archive'),null);
  await assert.rejects(saveScidBatch([pgn('Eve')],{destination:'archive',endpoint:'http://archive'}),/disabled/);
 }finally{dom.window.close();}
});
