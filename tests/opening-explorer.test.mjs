import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
globalThis.window=globalThis;
const session = new Map();
globalThis.sessionStorage = { getItem: key => session.get(key) ?? null, setItem: (key,value) => session.set(key,value), removeItem: key => session.delete(key) };
const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8');
const {openingExplorerUrl,readExplorerStream}=await import(`data:text/javascript,${encodeURIComponent(source)}`);
const fen='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
test('source and filters build only trusted explorer URLs',()=>{
  const url=new URL(openingExplorerUrl(fen,'lichess','rapid','1600,1800,2000'));
  assert.equal(url.origin,'https://explorer.lichess.org');
  assert.equal(url.searchParams.get('fen'),fen);
  assert.equal(url.searchParams.get('speeds'),'rapid');
  assert.equal(url.searchParams.get('ratings'),'1600,1800,2000');
  assert.equal(new URL(openingExplorerUrl(fen,'masters')).pathname,'/masters');
  assert.throws(()=>openingExplorerUrl(fen,'evil'));
});
test('requests are serialized, cached per position, and errors can be retried',async()=>{
  sessionStorage.setItem('ironwood.lichess.token','test-token');
  let calls=0,resolve;
  globalThis.fetch=()=>{calls++;return new Promise(r=>{resolve=r;});};
  assert.equal(window.ironwoodExplorer(fen,'masters','',''),'');
  assert.equal(window.ironwoodExplorer(fen,'lichess','',''),'');
  assert.equal(calls,1);
  resolve({ok:true,json:async()=>({moves:[]})});
  await new Promise(r=>setTimeout(r,0));
  assert.deepEqual(JSON.parse(window.ironwoodExplorer(fen,'masters','','')).moves,[]);
  assert.equal(calls,1);
  globalThis.fetch=async()=>{calls++;throw new TypeError('offline');};
  window.ironwoodExplorer(fen,'lichess','','');
  await new Promise(r=>setTimeout(r,0));
  assert.match(JSON.parse(window.ironwoodExplorer(fen,'lichess','','')).error,/connection/);
  window.ironwoodExplorerRetry();
  assert.equal(window.ironwoodExplorer(fen,'lichess','',''),'');
  assert.equal(calls,3);
  await new Promise(r=>setTimeout(r,0));
});

test('missing and rejected credentials produce authentication guidance',async()=>{
  sessionStorage.removeItem('ironwood.lichess.token');
  assert.match(JSON.parse(window.ironwoodExplorer(fen,'masters','','')).error,/Sign in/);
  sessionStorage.setItem('ironwood.lichess.token','test-token');
  globalThis.fetch=async(url,options)=>{
    assert.equal(options.headers.Authorization,'Bearer test-token');
    return {ok:false,status:401};
  };
  window.ironwoodExplorer(fen,'masters','','');
  await new Promise(r=>setTimeout(r,0));
  assert.match(JSON.parse(window.ironwoodExplorer(fen,'masters','','')).error,/rejected/);
});

test('sign-out clears cached data and session replacement uses the new credential', async()=>{
  sessionStorage.setItem('ironwood.lichess.token','replacement-session');
  assert.equal(window.ironwoodExplorerSignedIn(),true);
  globalThis.fetch=async(url,options)=>{
    assert.equal(options.headers.Authorization,'Bearer replacement-session');
    return {ok:true,json:async()=>({moves:[]})};
  };
  window.ironwoodExplorer(fen,'masters','','');
  await new Promise(r=>setTimeout(r,0));
  assert.deepEqual(JSON.parse(window.ironwoodExplorer(fen,'masters','','')).moves,[]);
  sessionStorage.removeItem('ironwood.lichess.token');
  assert.equal(window.ironwoodExplorerSignedIn(),false);
  assert.match(JSON.parse(window.ironwoodExplorer(fen,'masters','','')).error,/Sign in/);
});


test('player filters use month boundaries and isolate color and player caches',()=>{
  const url = new URL(openingExplorerUrl(fen,'player','rapid','','Ada','black','2026-01','2026-09'));
  assert.equal(url.pathname,'/player');
  for (const [key,value] of Object.entries({player:'Ada',color:'black',since:'2026-01',until:'2026-09',recentGames:'8',speeds:'rapid'})) assert.equal(url.searchParams.get(key),value);
  assert.notEqual(url.href,openingExplorerUrl(fen,'player','rapid','','Ada','white','2026-01','2026-09'));
  assert.notEqual(url.href,openingExplorerUrl(fen,'player','rapid','','Bob','black','2026-01','2026-09'));
  for (const args of [['bad user','white','',''],['Ada','random','',''],['Ada','white','2026-13',''],['Ada','white','2026-09','2026-01']])
    assert.throws(()=>openingExplorerUrl(fen,'player','','',...args));
});

test('indexing stream accepts split records, heartbeats, and a final unterminated update',async()=>{
  const snapshots = [];
  const chunks = ['\n{"moves":[],"wh','ite":1}\n\n','{"moves":[],"white":2}'];
  const body = new ReadableStream({ start(controller) { for (const chunk of chunks) controller.enqueue(new TextEncoder().encode(chunk)); controller.close(); } });
  const result = await readExplorerStream({body},data=>snapshots.push(data));
  assert.deepEqual(snapshots.map(data=>data.white),[1,2]);
  assert.equal(snapshots[0].indexing,true);
  assert.equal(result.white,2);
  assert.equal(result.indexing,undefined);
});

test('player request caches the final streamed snapshot and reports invalid filters without fetching',async()=>{
  sessionStorage.setItem('ironwood.lichess.token','stream-session');
  let calls = 0;
  globalThis.fetch = async()=> { calls++; return {ok:true,body:new ReadableStream({start(controller) {
    controller.enqueue(new TextEncoder().encode('{"moves":[],"recentGames":[],"white":3}\n')); controller.close();
  }})}; };
  assert.match(JSON.parse(window.ironwoodExplorer(fen,'player','','','bad user','white','','')).error,/username/);
  assert.equal(calls,0);
  assert.equal(window.ironwoodExplorer(fen,'player','','','Ada','white','',''),'');
  await new Promise(resolve=>setTimeout(resolve,0));
  const result = JSON.parse(window.ironwoodExplorer(fen,'player','','','Ada','white','',''));
  assert.equal(result.white,3);
  assert.equal(result.indexing,undefined);
  assert.equal(calls,1);
});


test('changing a player position aborts indexing and never reuses partial statistics',async()=>{
  sessionStorage.setItem('ironwood.lichess.token','cancel-session');
  let aborted = false, calls = 0;
  globalThis.fetch = async(url,{signal})=> {
    calls++;
    return {ok:true,body:new ReadableStream({start(controller) {
      controller.enqueue(new TextEncoder().encode('{"moves":[],"white":1}\n'));
      signal.addEventListener('abort',()=>{aborted=true;controller.error(new DOMException('Aborted','AbortError'));},{once:true});
    }})};
  };
  window.ironwoodExplorer(fen,'player','','','Ada','white','','');
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(JSON.parse(window.ironwoodExplorer(fen,'player','','','Ada','white','','')).indexing,true);
  assert.equal(window.ironwoodExplorer(fen,'player','','','Bob','white','',''),'');
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(aborted,true);
  globalThis.fetch = async()=>({ok:true,body:new ReadableStream({start(controller) {
    controller.enqueue(new TextEncoder().encode('{"moves":[],"white":2}\n')); controller.close();
  }})});
  assert.equal(window.ironwoodExplorer(fen,'player','','','Ada','white','',''),'');
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(JSON.parse(window.ironwoodExplorer(fen,'player','','','Ada','white','','')).white,2);
  assert.equal(calls,1);
});


test('refresh discards completed player statistics without restarting queued indexing',async()=>{
  sessionStorage.setItem('ironwood.lichess.token','refresh-session');
  let calls = 0, finish;
  globalThis.fetch = async()=> { calls++; return {ok:true,body:new ReadableStream({start(controller) {
    controller.enqueue(new TextEncoder().encode('{"moves":[],"white":0,"queuePosition":42}\n'));
    finish = ()=>controller.close();
  }})}; };
  window.ironwoodExplorer(fen,'player','','','Ada','white','','');
  await new Promise(resolve=>setTimeout(resolve,0));
  window.ironwoodExplorerRefresh();
  const queued = JSON.parse(window.ironwoodExplorer(fen,'player','','','Ada','white','',''));
  assert.equal(queued.indexing,true);
  assert.equal(queued.queuePosition,42);
  assert.equal(calls,1);
  finish();
  await new Promise(resolve=>setTimeout(resolve,0));
  window.ironwoodExplorerRefresh();
  assert.equal(window.ironwoodExplorer(fen,'player','','','Ada','white','',''),'');
  await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(calls,2);
  finish();
  await new Promise(resolve=>setTimeout(resolve,0));
});
