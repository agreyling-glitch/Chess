import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {CompanionEngineAdapter} from '../web/companion-engine-adapter.js';
const flush = () => new Promise(resolve => setImmediate(resolve));
const capabilities = {product:'Ironwood Companion',protocol_version:'1',services:{archive:true,uci:true}};

test('named providers route independently, survive reload, and keep one default archive', async () => {
  const dom = new JSDOM('<body></body>',{url:'http://localhost/play/'});
  const previous = {window:globalThis.window,fetch:globalThis.fetch};
  globalThis.window = dom.window;
  let enabled = true, failed = false;
  window.ironwoodCompanionFeatureEnabled = () => enabled;
  const calls = [];
  globalThis.fetch = async (url,init) => {
    const endpoint = new URL(url).origin;
    calls.push({url:String(url),authorization:init.headers.Authorization});
    assert.equal(init.headers.Authorization,`Bearer ${endpoint.includes('nas') ? 'archive-token' : endpoint.includes('second') ? 'second-token' : 'gpu-token'}`);
    if (failed && endpoint.includes('second')) throw Error('offline');
    return {ok:true,json:async () => String(url).endsWith('capabilities') ? capabilities : [{id:'lc0',label:'Lc0'}]};
  };
  const methods = {connect:CompanionEngineAdapter.prototype.connect,release:CompanionEngineAdapter.prototype.release,close:CompanionEngineAdapter.prototype.close};
  let selected;
  CompanionEngineAdapter.prototype.connect = async config => { selected = config; return {name:'Lc0',options:[]}; };
  CompanionEngineAdapter.prototype.release = async () => {};
  CompanionEngineAdapter.prototype.close = () => {};
  try {
    const connections = await import('../web/companion-connection.js');
    let archiveChanges = 0;
    const unsubscribe = connections.onCompanionConnectionChange(() => archiveChanges++);
    await connections.connectCompanion('http://nas:8790','archive-token',{name:'QNAP Archive',archive:true,engines:false,remember:true});
    await connections.connectCompanion('http://gpu:8790','gpu-token',{name:'Windows GPU',archive:false,engines:true,remember:true});
    await connections.connectCompanion('http://second:8790','second-token',{name:'Second PC',archive:false,engines:true,remember:true});
    assert.equal(archiveChanges,1,'adding engine providers must not reset archive clients or cached pages');
    unsubscribe();
    assert.equal(connections.companionConnection().endpoint,'http://nas:8790');
    assert.equal(connections.companionConnections().filter(item => item.archive).length,1);
    const reloaded = await import('../web/companion-connection.js?named-reload');
    assert.equal(reloaded.companionConnections().length,3);
    assert.equal(reloaded.companionConnection().endpoint,'http://nas:8790');
    assert.equal(reloaded.companionEngineConfiguration('http://gpu:8790/lc0').credential,'gpu-token');
    assert.throws(() => reloaded.companionEngineConfiguration('http://nas:8790/lc0'),/different Companion/);
    await import('../web/companion-engines.js');
    await window.ironwoodRefreshCompanionEngines();
    let catalog = JSON.parse(window.ironwoodCompanionEngineCatalog());
    assert.deepEqual(catalog.engines.map(item => item.label),['Lc0 — Windows GPU','Lc0 — Second PC']);
    assert.equal(calls.filter(item => item.url === 'http://nas:8790/api/v1/engines').length,0);
    await window.ironwoodSelectCompanionEngine('http://gpu:8790/lc0');
    assert.equal(selected.credential,'gpu-token');
    assert.equal(selected.endpoint,'http://gpu:8790');
    assert.equal(JSON.parse(window.ironwoodCompanionPollChange()).identity,'http://gpu:8790/lc0');
    failed = true;
    await window.ironwoodRefreshCompanionEngines();
    catalog = JSON.parse(window.ironwoodCompanionEngineCatalog());
    assert.equal(catalog.engines.length,1);
    assert.match(catalog.status,/Second PC.*offline/);
    connections.disconnectCompanion('http://nas:8790');
    assert.equal(connections.companionConnection(),null);
    assert.equal(connections.companionEngineConfiguration('http://gpu:8790/lc0').credential,'gpu-token');
    enabled = false;
    const count = calls.length;
    await window.ironwoodRefreshCompanionEngines();
    assert.equal(calls.length,count);
    assert.deepEqual(connections.companionConnections(),[]);
    assert.throws(() => connections.companionEngineConfiguration('http://gpu:8790/lc0'),/Retry connection/);
    enabled = true;
    connections.removeCompanionConnection('http://gpu:8790');
    assert.equal(connections.companionEngineConnections().length,1);
    connections.forgetCompanionConnection();
  } finally {
    Object.assign(CompanionEngineAdapter.prototype,methods);
    globalThis.window = previous.window; globalThis.fetch = previous.fetch; dom.window.close();
  }
});

test('legacy connection migrates without changing endpoint, credential or engine identity', async () => {
  const dom = new JSDOM('<body></body>',{url:'http://localhost/play/'});
  const previous = globalThis.window;
  globalThis.window = dom.window;
  window.localStorage.setItem('ironwood.companion.connection.v1',JSON.stringify({version:1,endpoint:'http://localhost:8790',credential:'legacy-token'}));
  try {
    const connection = await import('../web/companion-connection.js?named-migration');
    assert.equal(connection.companionConnections()[0].name,'Companion');
    assert.equal(connection.companionConnections()[0].archive,true);
    assert.equal(connection.companionEngineConfiguration('http://localhost:8790/lc0').credential,'legacy-token');
    window.ironwoodCompanionFeatureEnabled = () => false;
    window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));
    assert.equal(connection.companionConnection(),null);
    window.ironwoodCompanionFeatureEnabled = () => true;
    window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));
    assert.equal(connection.companionConnection().endpoint,'http://localhost:8790');
  } finally { globalThis.window = previous; dom.window.close(); }
});

test('connection manager edits roles and names, masks tokens and removes only the selected provider', async () => {
  const dom = new JSDOM('<body></body>',{url:'http://localhost/play/'});
  const previous = {window:globalThis.window,document:globalThis.document,fetch:globalThis.fetch};
  globalThis.window = dom.window; globalThis.document = dom.window.document;
  window.ironwoodCompanionFeatureEnabled = () => true;
  window.HTMLDialogElement.prototype.showModal = function() { this.open = true; };
  window.HTMLDialogElement.prototype.close = function() { this.open = false; this.dispatchEvent(new window.Event('close')); };
  globalThis.fetch = async () => ({ok:true,json:async () => capabilities});
  try {
    const connections = await import('../web/companion-connection.js?named-manager');
    await connections.connectCompanion('http://nas:8790','token-a',{name:'Archive',engines:false});
    await connections.connectCompanion('http://gpu:8790','token-b',{name:'GPU',archive:false});
    const closing = connections.manageCompanionConnection();
    const dialog = document.querySelector('#companion-connection'), form = dialog.querySelector('form');
    assert.equal(form.elements.credential.type,'password');
    form.elements.connection.value = 'http://gpu:8790';
    form.elements.connection.dispatchEvent(new window.Event('change'));
    form.elements.connectionName.value = '<b>Windows GPU</b>';
    form.dispatchEvent(new window.Event('submit',{cancelable:true}));
    await flush();
    assert.equal(connections.companionConnection().endpoint,'http://nas:8790');
    assert.match(form.elements.connection.selectedOptions[0].textContent,/<b>Windows GPU<\/b>/);
    assert.equal(dialog.querySelector('b'),null);
    dialog.querySelector('[data-disconnect]').click();
    assert.equal(connections.companionConnection().endpoint,'http://nas:8790');
    assert.equal(connections.companionEngineConnections().length,0);
    dialog.querySelector('[data-forget]').click();
    assert.equal(connections.companionConnections().length,1);
    dialog.querySelector('[data-close]').click();
    await closing;
    assert.equal(document.querySelector('#companion-connection'),null);
  } finally { Object.assign(globalThis,previous); dom.window.close(); }
});
