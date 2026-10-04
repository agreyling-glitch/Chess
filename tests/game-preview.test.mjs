import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { JSDOM } from 'jsdom';
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const preview = source.slice(source.indexOf('async function showGamePreview'), source.indexOf('function confirmDeleteGames'));
test('preview navigation and 500ms autoplay stop at end and on close', async () => {
  const { window } = new JSDOM('<button id="trigger">Preview</button>');
  window.HTMLDialogElement.prototype.showModal = function() { this.open = true; };
  window.HTMLDialogElement.prototype.close = function() { this.open = false; this.dispatchEvent(new window.Event('close')); };
  let tick; let interval; let cleared = 0;
  window.ironwoodExportSavedGame = () => JSON.stringify({positions:['start','e4','e5'], moves:['e4','e5']});
  const context = vm.createContext({window, document:window.document, readGame:async()=>({json:'{}'}),
    finalBoard:fen=>{const el=window.document.createElement('div');el.textContent=fen;return el;},
    startingNoteElement:()=>null, setInterval:(fn,ms)=>{tick=fn;interval=ms;return 1;},clearInterval:()=>cleared++});
  vm.runInContext(preview,context);
  await context.showGamePreview({id:'test',title:'Test',finalFen:'e5'},window.document.querySelector('#trigger'));
  const dialog = window.document.querySelector('dialog');
  const buttons = [...dialog.querySelectorAll('.game-preview-controls button')];
  buttons[2].click(); assert.equal(interval,500); assert.match(dialog.textContent,/Starting position/);
  tick(); assert.match(dialog.textContent,/Move 1 of 2/);
  tick(); assert.match(dialog.textContent,/Move 2 of 2/); assert.equal(buttons[2].textContent,'Auto play');
  assert.equal(cleared,1);
  dialog.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowLeft',bubbles:true,cancelable:true}));
  assert.match(dialog.textContent,/Move 1 of 2/);
  dialog.dispatchEvent(new window.KeyboardEvent('keydown', {key:'ArrowRight',bubbles:true,cancelable:true}));
  assert.match(dialog.textContent,/Move 2 of 2/);
  buttons[1].click(); assert.match(dialog.textContent,/Move 1 of 2/);
  buttons[2].click(); dialog.close(); assert.equal(cleared,2); assert.equal(window.document.querySelector('dialog'),null);
});
