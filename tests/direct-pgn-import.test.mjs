import test from 'node:test';
import assert from 'node:assert/strict';
import {ArchiveLibrary} from '../web/companion-library.js';
const connection={endpoint:'http://local:8790',credential:'test'};
const pgn=(name,body='1. e4 e5 2. Nf3 Nc6 *')=>`[Event "${name}"]\n\n${body}`;
const store=handler=>new ArchiveLibrary({...connection,request:handler},{connection:()=>connection,
 details(){throw Error('Browser metadata conversion called');},exporter(){throw Error('Browser annotated export called');}});

test('direct PGN sends original annotations and counts server-discarded short games without Browser conversion',async()=>{
 const games=[pgn('Annotated','1. e4 {[%clk 0:10:00] Plan} e5 2. Nf3 $1 (2. Nc3 Nc6) Nc6 *'),pgn('Short','1. e4 *')],results=[];
 const library=store(async(path,body)=>{assert.equal(path,'archive/import');assert.deepEqual(body,{pgn:games.join('\n\n')});return {imported:1,duplicates:0,discarded:1};});
 await library.importPgnBatch(games,(result,n)=>results.push({result,n}));
 assert.equal(results[0].n,2);assert.equal(results[0].result.discarded,1);
});

test('invalid PGN is isolated, while authentication and storage failures stop without splitting',async()=>{
 const results=[],requests=[];
 const library=store(async(path,body)=>{requests.push(body.pgn);if(body.pgn.includes('BAD'))throw Object.assign(Error('Malformed PGN'),{status:400,code:'invalid_request'});
  return {imported:(body.pgn.match(/\[Event/g)||[]).length,duplicates:0,discarded:0};});
 await library.importPgnBatch([pgn('A'),pgn('BAD'),pgn('B')],(r,n)=>results.push({r,n}));
 assert.equal(results.reduce((n,x)=>n+x.n,0),3);assert.equal(results.reduce((n,x)=>n+(x.r.failed||0),0),1);
 assert.equal(results.reduce((n,x)=>n+x.r.imported,0),2);
 let calls=0;const denied=store(async()=>{calls++;throw Object.assign(Error('Denied'),{status:401});});
 await assert.rejects(denied.importPgnBatch([pgn('A'),pgn('B')],()=>{}),/Denied/);assert.equal(calls,1);
});

test('direct PGN splits by JSON byte size and verifies complete acknowledgements',async()=>{
 const sizes=[],library=store(async(path,body)=>{sizes.push(Buffer.byteLength(JSON.stringify(body)));return {imported:(body.pgn.match(/\[Event/g)||[]).length,duplicates:0,discarded:0};});
 const games=Array.from({length:12},(_,i)=>pgn(String(i),`1. e4 {${'\\'.repeat(800000)}} e5 2. Nf3 Nc6 *`));
 let confirmed=0;await library.importPgnBatch(games,(_r,n)=>{confirmed+=n;});
 assert.equal(confirmed,12);assert.ok(sizes.length>1);assert.ok(sizes.every(n=>n<=15*1024*1024));
 await assert.rejects(store(async()=>({imported:0,duplicates:0,discarded:0})).importPgnBatch([pgn('A')],()=>{}),/confirm/);
});
