import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
globalThis.window=globalThis;
const session = new Map();
globalThis.sessionStorage = { getItem: key => session.get(key) ?? null, setItem: (key,value) => session.set(key,value), removeItem: key => session.delete(key) };
const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8');
const {openingExplorerUrl}=await import(`data:text/javascript,${encodeURIComponent(source)}`);
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
