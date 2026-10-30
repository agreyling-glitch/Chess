import {test} from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {connectCompanion, disconnectCompanion} from '../web/companion-connection.js';
import {CompanionEngineAdapter} from '../web/companion-engine-adapter.js';

test('settings engine selector validates selections and discards results after disconnect', async () => {
  const dom = new JSDOM('<body></body>', {url:'http://localhost:8788'});
  const originals = {window:globalThis.window,fetch:globalThis.fetch};
  const methods = {connect:CompanionEngineAdapter.prototype.connect,release:CompanionEngineAdapter.prototype.release,close:CompanionEngineAdapter.prototype.close};
  globalThis.window = dom.window;
  window.ironwoodCompanionFeatureEnabled = () => true;
  const calls = [];
  globalThis.fetch = async (url, init) => {
    calls.push({url:String(url),init});
    return {ok:true,json:async () => String(url).endsWith('capabilities')
      ? {product:'Ironwood Companion',protocol_version:'1',services:{uci:true}}
      : [{id:'lc0',label:'Lc0'}]};
  };
  let finish, received;
  CompanionEngineAdapter.prototype.connect = async function(config) {
    received = config;
    return {name:'Lc0',options:[{name:'Threads',kind:'spin',default:'2',min:1,max:8}]};
  };
  CompanionEngineAdapter.prototype.release = async function() {};
  CompanionEngineAdapter.prototype.close = function() {};
  try {
    await import('../web/companion-engines.js');
    await window.ironwoodRefreshCompanionEngines();
    assert.equal(calls.length,0);
    assert.match(JSON.parse(window.ironwoodCompanionEngineCatalog()).status,/Companion connection/);
    await connectCompanion('http://localhost:8790','test-token');
    assert.equal(JSON.parse(window.ironwoodCompanionEngineCatalog()).busy,true);
    await new Promise(resolve => setImmediate(resolve));
    const catalog = JSON.parse(window.ironwoodCompanionEngineCatalog());
    assert.equal(catalog.engines[0].identity,'http://localhost:8790/lc0');
    await connectCompanion('http://localhost:8790','test-token',{name:'Win11'});
    window.ironwoodCompanionEngineCatalog();
    await new Promise(resolve => setImmediate(resolve));
    const renamed = JSON.parse(window.ironwoodCompanionEngineCatalog()).engines[0];
    assert.equal(renamed.label,'Lc0 — Win11');
    assert.equal(renamed.identity,catalog.engines[0].identity);
    const reads = calls.length;
    window.ironwoodCompanionEngineCatalog();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(calls.length,reads,'catalog polling must not repeatedly request engines');
    await window.ironwoodSelectCompanionEngine(catalog.engines[0].identity);
    assert.equal(received.credential,'test-token');
    assert.equal(received.engineId,'lc0');
    const selected = JSON.parse(window.ironwoodCompanionPollChange());
    assert.equal(selected.provider,'companion');
    assert.equal(selected.uci_options[0].name,'Threads');
    assert.equal(dom.window.document.querySelector('dialog'),null);
    await window.ironwoodSelectCompanionEngine('');
    assert.equal(JSON.parse(window.ironwoodCompanionPollChange()).provider,'builtin');
    // Refresh invalidates the capability cache so this selection opens a session.
    await window.ironwoodRefreshCompanionEngines();
    CompanionEngineAdapter.prototype.connect = () => new Promise(resolve => {finish = resolve;});
    const pending = window.ironwoodSelectCompanionEngine(catalog.engines[0].identity);
    disconnectCompanion();
    finish({name:'Late engine',options:[]});
    await pending;
    assert.equal(window.ironwoodCompanionPollChange(),'');
    assert.equal(JSON.parse(window.ironwoodCompanionEngineCatalog()).busy,false);
    window.ironwoodCompanionFeatureEnabled = () => false;
    await window.ironwoodSelectCompanionEngine('');
    assert.equal(window.ironwoodCompanionPollChange(),'');
  } finally {
    disconnectCompanion();
    Object.assign(CompanionEngineAdapter.prototype,methods);
    globalThis.window = originals.window; globalThis.fetch = originals.fetch;
    dom.window.close();
  }
});
