import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

const source = readFileSync('scripts/analytics-source.js', 'utf8').replace("import('../node_modules/posthog-js/dist/module.mjs')", 'window.loadSDK()');
function setup(url) {
  const dom = new JSDOM('<body></body>', { url, runScripts: 'outside-only' });
  const w = dom.window;
  let tick;
  w.setInterval = callback => { tick = callback; return 1; };
  const events = [];
  let loads = 0, config, stopped = false;
  const sdk = {
    init(key, options) { config = options; return sdk; },
    opt_in_capturing() { stopped = false; },
    opt_out_capturing() { stopped = true; },
    capture(event, properties) {
      if (!stopped) {
        const result = config.before_send({ event, properties: { ...properties, $current_url: url, $title: 'Private player', $referrer: 'secret' } });
        if (result) events.push(result);
      }
    },
  };
  w.loadSDK = async () => { loads++; return { default: sdk }; };
  w.eval(source);
  return { dom, w, events, tick: () => tick(), loads: () => loads, config: () => config };
}
const settle = () => new Promise(resolve => setTimeout(resolve, 0));
test('expanding privacy details leaves consent unanswered and collection off', () => {
  const s = setup('http://127.0.0.1:8080/play/');
  const panel = s.w.document.querySelector('[data-analytics-panel]');
  const details = panel.querySelector('details');
  assert.equal(details.open, false);
  panel.querySelector('summary').click();
  assert.equal(details.open, true);
  assert.equal(s.w.localStorage.length, 0);
  assert.equal(s.loads(), 0);
  assert.equal(panel.hidden, false);
  assert.equal(panel.querySelector('a'), null);
  panel.querySelector('summary').click();
  assert.equal(details.open, false);
  s.dom.window.close();
});
test('other preview hosts remain off even with the old test parameter', () => {
  const s = setup('https://preview.example.com/play/?analytics-test=1');
  assert.equal(s.loads(), 0);
  assert.equal(s.w.document.querySelector('[data-analytics-panel]'), null);
  s.dom.window.close();
});
test('normal local URLs collect test events only after consent', async () => {
  for (const host of ['localhost', '127.0.0.1']) {
    const s = setup(`http://${host}:8788/play/`);
    assert.equal(s.loads(), 0);
    s.w.ironwoodTrackUsage('game_started');
    assert.equal(s.events.length, 0);
    s.w.document.querySelector('[data-accept]').click();
    await settle();
    assert.equal(s.events[0].properties.environment, 'local_test');
    assert.equal(s.w.localStorage.getItem('ironwood.analytics.test-consent'), 'accepted');
    s.dom.window.close();
  }
});
test('consent gates loading; decline stops events; sensitive context is removed', async () => {
  const s = setup('http://127.0.0.1:8080/play/?game=private');
  assert.equal(s.loads(), 0);
  s.w.document.querySelector('[data-accept]').click();
  await settle();
  assert.equal(s.loads(), 1);
  assert.equal(s.config().disable_session_recording, true);
  assert.equal(s.config().autocapture, false);
  assert.equal(s.events[0].event, '$pageview');
  assert.equal(s.events[0].properties.environment, 'local_test');
  assert.equal(s.events[0].properties.$current_url, 'http://127.0.0.1:8080/play/');
  assert.equal(s.events[0].properties.$title, undefined);
  assert.equal(s.events[0].properties.$referrer, undefined);
  s.w.ironwoodTrackUsage('game_started');
  const count = s.events.length;
  s.w.document.querySelector('[data-decline]').click();
  s.w.ironwoodTrackUsage('game_started');
  assert.equal(s.events.length, count);
  s.dom.window.close();
});
test('declining initially never loads the SDK', async () => {
  const s = setup('https://ironwoodchess.com/play/');
  s.w.document.querySelector('[data-decline]').click();
  await settle();
  assert.equal(s.loads(), 0);
  s.dom.window.close();
});
test('active time excludes hidden and unfocused app windows', async () => {
  const s = setup('http://127.0.0.1:8080/play/');
  s.w.document.querySelector('[data-accept]').click();
  await settle();
  Object.defineProperty(s.w.document, 'hidden', { configurable: true, value: false });
  s.w.document.hasFocus = () => true;
  s.tick();
  assert.equal(s.events.at(-1).event, 'active_usage');
  const count = s.events.length;
  s.w.document.hasFocus = () => false;
  s.tick();
  assert.equal(s.events.length, count);
  s.w.document.hasFocus = () => true;
  Object.defineProperty(s.w.document, 'hidden', { value: true });
  s.tick();
  assert.equal(s.events.length, count);
  s.dom.window.close();
});
