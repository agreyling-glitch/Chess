import {test} from 'node:test';
import assert from 'node:assert/strict';
import {companionConnection, connectCompanion, disconnectCompanion, onCompanionConnectionChange} from '../web/companion-connection.js';
globalThis.window = globalThis;
const {ArchiveExplorer} = await import('../web/companion-explorer.js');
import {createLocalTablebaseClient} from '../web/tablebase.js';

test('one authenticated connection serves explorer and tablebases; failed changes and late authentication cannot replace it', async () => {
  const previous = globalThis.fetch, requests = [];
  let finish;
  globalThis.fetch = async (url, init) => {
    requests.push({url, headers:init.headers});
    if (url.includes('slow')) return new Promise(resolve => {finish = () => resolve({ok:true,json:async()=>({product:'Ironwood Companion',protocol_version:'1',services:{}})});});
    if (url.includes('rejected')) return {ok:false,json:async()=>({message:'Invalid credential'})};
    return {ok:true,json:async()=>url.endsWith('/capabilities') ? {product:'Ironwood Companion',protocol_version:'1',services:{archive:true}} : url.includes('explorer') ? {generation:1,moves:[]} : {category:'draw',moves:[]}};
  };
  const changes = [], unsubscribe = onCompanionConnectionChange(value => changes.push(value?.endpoint || null));
  try {
    await connectCompanion('http://first:8790', 'first-token');
    const explorer = new ArchiveExplorer(companionConnection);
    const local = createLocalTablebaseClient(companionConnection);
    explorer.poll('position'); local('position');
    await new Promise(resolve=>setImmediate(resolve));
    assert.equal(JSON.parse(explorer.poll('position')).generation,1);
    assert.equal(JSON.parse(local('position')).category,'draw');
    assert.ok(requests.every(r=>r.headers.Authorization==='Bearer first-token'));
    await assert.rejects(connectCompanion('http://rejected:8790','bad-token'), /Invalid credential/);
    assert.equal(companionConnection().endpoint,'http://first:8790');
    const stale = connectCompanion('http://slow:8790','late-token');
    disconnectCompanion(); finish();
    await assert.rejects(stale,/Connection changed/);
    assert.equal(companionConnection(),null);
    assert.match(JSON.parse(local('position')).error,/Connect Companion/);
    assert.match(JSON.parse(explorer.poll('position')).error,/Connect through/);
    assert.deepEqual(changes,['http://first:8790',null]);
    await assert.rejects(connectCompanion('http://first:8790/other','token'),/Companion origin/);
  } finally { unsubscribe(); disconnectCompanion(); globalThis.fetch = previous; }
});


test('remember is opt-in, restores after reload, and forget removes both saved fields', async () => {
  const previousWindow = globalThis.window, previousFetch = globalThis.fetch;
  const storage = new Map();
  globalThis.window = {localStorage:{getItem:key=>storage.get(key) ?? null,setItem:(key,value)=>storage.set(key,value),removeItem:key=>storage.delete(key)}};
  globalThis.fetch = async()=>({ok:true,json:async()=>({product:'Ironwood Companion',protocol_version:'1',services:{}})});
  try {
    const session = await import('../web/companion-connection.js?remember-test');
    await session.connectCompanion('http://localhost:8790','temporary');
    assert.equal(storage.size,0);
    await session.connectCompanion('http://localhost:8790','remembered',{remember:true});
    assert.equal(storage.size,1);
    const reloaded = await import('../web/companion-connection.js?reload-test');
    assert.deepEqual(reloaded.companionConnection(),{endpoint:'http://localhost:8790',credential:'remembered'});
    assert.deepEqual(reloaded.companionEngineConfiguration('http://localhost:8790/lc0', {Backend:'cuda'}),
      {endpoint:'http://localhost:8790',credential:'remembered',engineId:'lc0',overrides:{Backend:'cuda'}});
    assert.throws(() => reloaded.companionEngineConfiguration('http://other:8790/lc0'), /different Companion address/);
    assert.throws(() => reloaded.companionEngineConfiguration('http://localhost:8790/'), /different Companion address/);
    reloaded.disconnectCompanion();
    assert.equal(reloaded.companionConnection(),null);
    assert.throws(() => reloaded.companionEngineConfiguration('http://localhost:8790/lc0'), /Retry connection/);
    assert.equal(storage.size,1);
    reloaded.forgetCompanionConnection();
    assert.equal(storage.size,0);
    const forgotten = await import('../web/companion-connection.js?forgotten-test');
    assert.equal(forgotten.companionConnection(),null);
    await session.connectCompanion('http://localhost:8790','saved-again',{remember:true});
    await session.connectCompanion('http://localhost:8790','memory-only');
    assert.equal(storage.size,0);
    globalThis.window.localStorage.setItem = ()=>{throw Error('blocked');};
    await assert.rejects(session.connectCompanion('http://localhost:8790','blocked',{remember:true}),/storage is unavailable/);
    assert.equal(session.companionConnection().credential,'memory-only');
    await session.connectCompanion('http://localhost:8790','still-usable');
    assert.equal(session.companionConnection().credential,'still-usable');
  } finally {globalThis.window=previousWindow;globalThis.fetch=previousFetch;}
});
