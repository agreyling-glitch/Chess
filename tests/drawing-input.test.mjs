import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

test('canvas forwards only mouse chord transitions omitted by browser pointer events', () => {
  const html = readFileSync(new URL('../web/play/index.html', import.meta.url), 'utf8');
  const script = html.match(/<script>\s*\/\/ Pointer events[\s\S]*?<\/script>/)[0].replace(/<\/?script>/g, '');
  const canvasListeners = {}, documentListeners = {}, forwarded = [];
  const canvas = { addEventListener: (name, fn) => canvasListeners[name] = fn, dispatchEvent: event => forwarded.push(event) };
  vm.runInNewContext(script, {
    document: { getElementById: () => canvas, addEventListener: (name, fn) => documentListeners[name] = fn },
    window: { addEventListener() {} },
    PointerEvent: class { constructor(type, properties) { this.type = type; Object.assign(this, properties); } },
  });
  const mouse = (button, buttons) => ({ button, buttons, clientX: 150, clientY: 220, ctrlKey: false });
  canvasListeners.mousedown(mouse(0, 1));
  assert.equal(forwarded.length, 0);
  canvasListeners.mousedown(mouse(2, 3));
  assert.equal(forwarded.length, 1);
  assert.equal(forwarded[0].type, 'pointerdown');
  assert.equal(forwarded[0].button, 2);
  documentListeners.mouseup(mouse(2, 1));
  assert.equal(forwarded.length, 2);
  assert.equal(forwarded[1].type, 'pointerup');
  documentListeners.mouseup(mouse(0, 0));
  assert.equal(forwarded.length, 2);
  // Releasing left first must also reach egui while right remains held.
  canvasListeners.mousedown(mouse(0, 1));
  canvasListeners.mousedown(mouse(2, 3));
  documentListeners.mouseup(mouse(0, 2));
  assert.equal(forwarded.at(-1).button, 0);
  assert.equal(forwarded.at(-1).buttons, 2);
});
