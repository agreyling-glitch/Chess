import {test} from 'node:test';
import assert from 'node:assert/strict';

test('capability cache is session-scoped, shows pending choices, and never caches failures or stale results', async () => {
  const {JSDOM} = await import('jsdom');
  const dom = new JSDOM('<body></body>', {url:'http://localhost:8788'});
  const previous = {window:globalThis.window, document:globalThis.document, fetch:globalThis.fetch};
  globalThis.window = dom.window; globalThis.document = dom.window.document;
  globalThis.fetch = async address => ({ok:true,json:async()=>String(address).endsWith('/capabilities')
    ? {product:'Ironwood Companion',protocol_version:'1',services:{}}
    : [{id:'a',label:'Engine A'},{id:'b',label:'Engine B'}]});
  const {connectCompanion, disconnectCompanion} = await import('../web/companion-connection.js');
  const {CompanionEngineAdapter} = await import('../web/companion-engine-adapter.js');
  const originalConnect = CompanionEngineAdapter.prototype.connect;
  const originalRelease = CompanionEngineAdapter.prototype.release;
  let starts = 0, release, fail = false;
  let delayNext = false;
  CompanionEngineAdapter.prototype.connect = async function({engineId}) {
    starts++;
    if (delayNext) { delayNext = false; await new Promise(resolve => { release = resolve; }); }
    if (fail) throw Error('Engine unavailable');
    return {name:`Engine ${engineId}`,options:[{name:'Threads',kind:'spin',default:'2',min:1,max:8}]};
  };
  CompanionEngineAdapter.prototype.release = async function() {};
  try {
    await import('../web/companion-engines.js');
    await connectCompanion('http://localhost:8790','first-token');
    await window.ironwoodRefreshCompanionEngines();
    const catalog = () => JSON.parse(window.ironwoodCompanionEngineCatalog());
    const identity = id => catalog().engines.find(engine=>engine.id===id).identity;
    const a = identity('a'), b = identity('b');
    delayNext = true;
    const cold = window.ironwoodSelectCompanionEngine(a);
    assert.equal(catalog().pending_identity,a);
    assert.equal(catalog().busy,true);
    assert.match(catalog().status,/Preparing Engine A/);
    assert.equal(window.ironwoodCompanionPollChange(),'');
    release(); await cold;
    const first = JSON.parse(window.ironwoodCompanionPollChange());
    assert.equal(first.identity,a);
    assert.equal(first.uci_options[0].name,'Threads');
    assert.equal(catalog().pending_identity,null);
    await window.ironwoodSelectCompanionEngine(b);
    window.ironwoodCompanionPollChange();
    await window.ironwoodSelectCompanionEngine(a);
    assert.equal(starts,2,'returning to a discovered engine must skip the probe');
    assert.deepEqual(JSON.parse(window.ironwoodCompanionPollChange()),first);
    assert.equal(catalog().busy,false);
    assert.equal(window.localStorage.length,0,'capabilities must not be persisted');

    await window.ironwoodRefreshCompanionEngines();
    await window.ironwoodSelectCompanionEngine(a);
    window.ironwoodCompanionPollChange();
    assert.equal(starts,3,'explicit refresh must invalidate capabilities');
    await connectCompanion('http://localhost:8790','rotated-token');
    await window.ironwoodRefreshCompanionEngines();
    await window.ironwoodSelectCompanionEngine(a);
    window.ironwoodCompanionPollChange();
    assert.equal(starts,4,'credential changes must invalidate capabilities');

    fail = true;
    await window.ironwoodSelectCompanionEngine(b);
    assert.equal(window.ironwoodCompanionPollChange(),'');
    assert.equal(catalog().pending_identity,null);
    assert.match(catalog().status,/Engine unavailable/);
    fail = false;
    await window.ironwoodSelectCompanionEngine(b);
    window.ironwoodCompanionPollChange();
    assert.equal(starts,6,'failed discovery must be retried');

    await window.ironwoodRefreshCompanionEngines();
    delayNext = true;
    const stale = window.ironwoodSelectCompanionEngine(a);
    disconnectCompanion();
    release(); await stale;
    assert.equal(window.ironwoodCompanionPollChange(),'','disconnected results must not select an engine');
    await connectCompanion('http://localhost:8790','rotated-token');
    await window.ironwoodRefreshCompanionEngines();
    await window.ironwoodSelectCompanionEngine(a);
    assert.equal(starts,8,'stale discovery must not populate the new connection cache');
    window.ironwoodCompanionPollChange();
    await window.ironwoodSelectCompanionEngine('');
    assert.deepEqual(JSON.parse(window.ironwoodCompanionPollChange()),{provider:'builtin'});
  } finally {
    disconnectCompanion();
    CompanionEngineAdapter.prototype.connect = originalConnect;
    CompanionEngineAdapter.prototype.release = originalRelease;
    Object.assign(globalThis,previous); dom.window.close();
  }
});
