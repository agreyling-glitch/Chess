import { JSDOM } from 'jsdom';
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import test from 'node:test';

test('saved-game bulk selection, confirmation, atomic failure, and pagination', async () => {
  const dom = new JSDOM('<body></body>', { url: 'https://test.local' });
  const { window } = dom;
  Object.assign(globalThis, { window, document: window.document, localStorage: window.localStorage });
  window.HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  window.HTMLDialogElement.prototype.close = function (value = '') {
    this.returnValue = value; this.open = false; this.dispatchEvent(new window.Event('close'));
  };
  const records = new Map(Array.from({ length: 12 }, (_, i) => [String(i), {
    id: String(i), category: i === 11 ? 'imported' : 'mine', updatedAt: 100 - i,
    json: JSON.stringify({ move_notes: i === 0 ? ['Opening plan\n<img src=x onerror=alert(1)>'] : [], board: '8/8/8/8/8/8/8/8 w - - 0 1', review_pgn: `[White "Player${i}"]\n[Black "Opponent"]\n[Date "2026.09.27"]\n\n1. e4 *` }),
  }]));
  let failDelete = false;
  let deletes = [];
  const db = { transaction(_store, mode) {
    const tx = { objectStore() { return {
      openCursor() {
        const request = {};
        const values = [...records.values()];
        let index = 0;
        const advance = () => queueMicrotask(() => {
          request.result = index < values.length ? { value: values[index++], continue: advance } : null;
          request.onsuccess();
        });
        advance(); return request;
      },
      delete(id) { deletes.push(id); },
    }; } };
    if (mode === 'readwrite') setTimeout(() => {
      if (failDelete) { tx.error = new Error('simulated failure'); tx.onabort(); }
      else { deletes.forEach(id => records.delete(id)); tx.oncomplete(); }
      deletes = [];
    }, 0);
    return tx;
  } };
  globalThis.indexedDB = window.indexedDB = { open() {
    const request = {};
    queueMicrotask(() => { request.result = db; request.onsuccess(); }); return request;
  } };
  const source = await readFile(new URL('../web/game-storage.js', import.meta.url), 'utf8');
  const { buildGameExport } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  await window.ironwoodOpenGameLibrary();
  const button = text => [...document.querySelectorAll('button')].find(b => b.textContent === text);
  const click = text => { const b = button(text); assert.ok(b, text); b.click(); };
  const settle = () => new Promise(resolve => setTimeout(resolve, 15));
  assert.equal(document.querySelectorAll('.game-library-select').length, 10);
  const notePreview = document.querySelector('.game-starting-note-preview');
  assert.equal(notePreview.textContent, 'Opening plan\n<img src=x onerror=alert(1)>');
  assert.equal(document.querySelectorAll('.game-starting-note-preview').length, 1);
  assert.equal(notePreview.querySelector('img'), null);
  document.querySelector('.game-library-preview').click();
  const fullNote = document.querySelector('#game-preview-dialog .game-starting-note-full');
  assert.equal(fullNote.textContent, notePreview.textContent);
  assert.equal(fullNote.querySelector('img'), null);
  document.querySelector('#game-preview-dialog').close();
  assert.equal(button('Clear selection'), undefined);
  assert.equal(button('Export ▾').disabled, true);
  const samples = [{json:'first'}, {json:'second'}];
  const exporter = (json, format) => format === 'json' ? JSON.stringify({game:json}) : `[Event "${json}"]\n\n1. e4 *\n`;
  assert.equal(buildGameExport(samples,'pgn',exporter).content.split('[Event').length,3);
  assert.equal(buildGameExport([samples[0]],'json',exporter).filename,'ironwood-game.json');
  const archive = buildGameExport(samples,'json',exporter);
  assert.equal(archive.filename,'ironwood-games-analysis.zip');
  // Independently open and CRC-check the archive with Python's standard ZIP reader.
  const { spawnSync } = await import('node:child_process');
  const checked = spawnSync('python', ['-c', 'import sys,io,zipfile,json; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); assert z.testzip() is None; assert z.namelist()==["game-1.json","game-2.json"]; assert json.loads(z.read("game-2.json"))["game"]=="second"'], {input:archive.content});
  assert.equal(checked.status,0,checked.stderr.toString());
  document.querySelector('.game-library-select').click();
  click('Export ▾');
  assert.equal(button('Annotated PGN').closest('.game-library-export-options').hidden,false);
  button('Annotated PGN').dispatchEvent(new window.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
  assert.equal(button('Annotated PGN').closest('.game-library-export-options').hidden,true);
  click('Next ▶');
  document.querySelector('.game-library-select').click();
  assert.equal(button('Delete selected (2)').disabled, false);
  click('Delete selected (2)'); await settle();
  assert.equal(document.querySelector('#game-delete-dialog h2').textContent, 'Delete 2 saved games?');
  click('Cancel'); await settle();
  assert.equal(records.size, 12);
  click('Delete selected (2)'); await settle();
  click('Delete 2 games'); await settle();
  assert.equal(records.size, 10);
  assert.equal(records.has('0'), false);
  assert.equal(records.has('10'), false);
  assert.match(document.querySelector('.game-library-pager').textContent, /Page 1 of 1/);
  click('Select all 10 matching');
  const search = document.querySelector('input[type=search]');
  search.value = 'Player11'; search.dispatchEvent(new window.Event('input'));
  assert.equal(button('Delete selected (0)').disabled, true);
  click('Select all 1 matching');
  failDelete = true;
  click('Delete selected (1)'); await settle();
  click('Delete game'); await settle();
  assert.equal(records.size, 10);
  assert.match(document.querySelector('[role=alert]').textContent, /simulated failure/);
  failDelete = false;
  click('Delete selected (1)'); await settle();
  click('Delete game'); await settle();
  assert.equal(records.has('11'), false);
  assert.equal(records.size, 9);
  assert.equal(button('Delete selected (0)').disabled, true);
  assert.match(document.querySelector('.game-library-list').textContent, /No games match/);
  dom.window.close();
});
