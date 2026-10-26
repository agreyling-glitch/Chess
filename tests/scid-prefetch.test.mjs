import test from 'node:test';
import assert from 'node:assert/strict';
import {importScidRange} from '../web/import-scid5.js';
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
async function until(condition){for(let i=0;i<100&&!condition();i++)await tick();assert.ok(condition(),'Pipeline did not reach expected state');}
function fixture(count=400,failAt=-1){
 let decoded=0,closed=false;
 return {count,get decoded(){return decoded;},get closed(){return closed;},
  async *entries(first,end){try{for(let number=first;number<end;number++){if(number===failAt)throw Error('Index read failed');yield {number,white:'Ada',black:'Ben'};}}finally{closed=true;}},
  async pgn(entry){decoded++;return String(entry.number);},
 };
}
const options={batchSize:128,prefetch:true};
const yieldToUi=async()=>{};

test('prepare next batch overlaps one save, remains bounded, and checkpoints only acknowledgements',async()=>{
 const source=fixture(),gate=deferred(),writes=[],checkpoints=[];let active=0,peak=0;
 const task=importScidRange(source,options,{yieldToUi,checkpoint:s=>checkpoints.push(s.next),saveBatch:async rows=>{
  writes.push(rows);active++;peak=Math.max(peak,active);if(writes.length===1)await gate.promise;active--;return {saved:rows.length};
 }});
 await until(()=>source.decoded===256);assert.equal(writes.length,1);assert.deepEqual(checkpoints,[]);
 await tick();assert.equal(source.decoded,256,'Only one ahead batch may be prepared');
 gate.resolve();const result=await task;
 assert.equal(peak,1);assert.equal(result.saved,400);assert.deepEqual(checkpoints,[128,256,384,400]);assert.equal(source.closed,true);
});

test('Stop finishes the in-flight batch and discards prepared look-ahead for safe resume',async()=>{
 const source=fixture(),gate=deferred();let stop=false,writes=[];
 const task=importScidRange(source,options,{yieldToUi,cancelled:()=>stop,saveBatch:async rows=>{writes.push(rows);await gate.promise;return {saved:rows.length};}});
 await until(()=>source.decoded===256);stop=true;gate.resolve();const result=await task;
 assert.equal(writes.length,1);assert.equal(result.next,128);assert.equal(result.saved,128);assert.equal(result.stopped,true);
 const resumed=await importScidRange(source,{...options,first:result.next},{yieldToUi,saveBatch:async rows=>{writes.push(rows);return {saved:rows.length};}});
 assert.equal(resumed.saved,272);assert.deepEqual(writes.flat(),Array.from({length:400},(_,n)=>String(n)));
});

test('save rejection never advances the checkpoint over look-ahead and closes the producer',async()=>{
 const source=fixture(),gate=deferred(),checkpoints=[];
 const task=importScidRange(source,options,{yieldToUi,checkpoint:s=>checkpoints.push(s.next),saveBatch:()=>gate.promise});
 const failure=assert.rejects(task,error=>{assert.equal(error.progress.next,0);return /disk full/.test(error.message);});
 await until(()=>source.decoded===256);gate.reject(Error('disk full'));await failure;
 assert.deepEqual(checkpoints,[]);assert.equal(source.closed,true);
});

test('look-ahead read errors wait for the current save before reporting its durable checkpoint',async()=>{
 const source=fixture(400,129),gate=deferred(),checkpoints=[];
 const task=importScidRange(source,options,{yieldToUi,checkpoint:s=>checkpoints.push(s.next),saveBatch:()=>gate.promise});
 const failure=assert.rejects(task,error=>{assert.equal(error.progress.next,128);return /Index read failed/.test(error.message);});
 await until(()=>source.closed);assert.deepEqual(checkpoints,[]);gate.resolve({saved:128});await failure;
 assert.deepEqual(checkpoints,[128]);
});
