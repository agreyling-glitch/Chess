import test from 'node:test';
import assert from 'node:assert/strict';
import { JSDOM } from 'jsdom';

test('Companion settings edits the selected connection inline and clears the token on close', async () => {
  const dom = new JSDOM('<canvas id="chess_canvas"></canvas>', {url:'http://localhost/play/'});
  const previous = {window:globalThis.window, document:globalThis.document, fetch:globalThis.fetch};
  globalThis.window = dom.window;
  globalThis.document = dom.window.document;
  try {
    window.localStorage.setItem('ironwood.companion.connection.v1', JSON.stringify({
      version: 2,
      archiveId: 'http://archive:8790',
      connections: [
        {name:'Archive', endpoint:'http://archive:8790', credential:'archive-test'},
        {name:'Engines', endpoint:'http://engines:8790', credential:'engines-test'},
      ],
    }));
    globalThis.fetch = async (url, options) => {
      assert.equal(new URL(url).origin, 'http://engines:8790');
      assert.equal(options.headers.Authorization, 'Bearer engines-test');
      return {ok:true, json:async () => ({product:'Ironwood Companion', protocol_version:'1'})};
    };
    const {layoutCompanionSettings, companionConnections} = await import('../web/companion-connection.js');
    layoutCompanionSettings({x:220, y:70, width:550, height:520});
    const editor = document.getElementById('companion-settings-editor');
    const form = editor.querySelector('form');
    assert.equal(document.querySelector('dialog'), null);
    assert.equal(editor.style.left, '220px');
    assert.equal(form.elements.connectionName.value, 'Archive');
    editor.querySelectorAll('.connection-list button')[1].click();
    assert.equal(form.elements.connectionName.value, 'Engines');
    assert.equal(form.elements.credential.type, 'password');
    assert.equal(form.elements.credential.value, 'engines-test');
    form.elements.connectionName.value = 'Renamed engines';
    await form.onsubmit(new window.Event('submit', {cancelable:true}));
    assert.equal(companionConnections().find(item => item.engines && !item.archive).name, 'Renamed engines');
    assert.match(editor.querySelector('[aria-pressed=true]').textContent, /Renamed engines/);
    layoutCompanionSettings(null);
    assert.equal(editor.isConnected, false);
    assert.equal(form.elements.credential.value, '');
  } finally {
    Object.assign(globalThis, previous);
    dom.window.close();
  }
});
