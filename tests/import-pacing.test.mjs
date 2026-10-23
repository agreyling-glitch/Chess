import {test} from 'node:test';
import assert from 'node:assert/strict';
import {ImportPacer,ArchiveLibrary} from '../web/companion-library.js';
import {CompanionArchiveClient} from '../web/companion-archive-client.js';
test('adaptive import sends healthy batches immediately and reserves shared capacity',async()=>{
 let now=0;const sleeps=[],p=new ImportPacer(()=>now,async ms=>{sleeps.push(ms);now+=ms;});
 await p.wait({limit:240,remaining:239,resetAt:60000});assert.deepEqual(sleeps,[]);
 await p.wait({limit:240,remaining:60,resetAt:60000});assert.deepEqual(sleeps,[60050]);
 await p.wait({limit:1,remaining:1,resetAt:120000});assert.equal(sleeps.length,1);
 await p.wait({limit:1,remaining:0,resetAt:120000});assert.equal(now,120050);
});
test('busy engines get breathing room and old servers retain a conservative budget',async()=>{
 let now=0;const sleeps=[],p=new ImportPacer(()=>now,async ms=>{sleeps.push(ms);now+=ms;});
 p.latency=600;await p.wait({limit:240,remaining:200,resetAt:60000,busy:true});assert.deepEqual(sleeps,[300]);
 for(let i=1;i<120;i++)await p.wait();assert.equal(sleeps.length,1);
 await p.wait();assert.equal(now,60050);
});
test('client reads budget headers and rejects malformed feedback',async()=>{
 let headers=new Headers({'x-ironwood-operations-limit':'240','x-ironwood-operations-remaining':'230','x-ironwood-operations-reset-ms':'50000','x-ironwood-engine-busy':'1'});
 const client=new CompanionArchiveClient('http://local:8790','test',async()=>({ok:true,headers,json:async()=>({})}));
 await client.request('capabilities');assert.equal(client.importBudget.remaining,230);assert.equal(client.importBudget.busy,true);
 headers.set('x-ironwood-operations-remaining','999');await client.request('capabilities');assert.equal(client.importBudget,undefined);
});
test('concurrent archive transfers share one in-flight import and recover after failure',async()=>{
 let active=0,max=0,calls=0;const client={endpoint:'http://local:8790',credential:'test',request:async()=>{active++;max=Math.max(max,active);await new Promise(r=>setTimeout(r,5));active--;if(++calls===1)throw Error('failed');return {};}};
 const make=()=>new ArchiveLibrary(client,{connection:()=>client});const a=make(),b=make();
 const results=await Promise.allSettled([a.request('archive/import',{}),b.request('archive/import',{})]);
 assert.equal(max,1);assert.equal(results[0].status,'rejected');assert.equal(results[1].status,'fulfilled');
});
