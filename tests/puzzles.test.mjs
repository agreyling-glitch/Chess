import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('../web/puzzles.js', import.meta.url), 'utf8');
function harness(fetch) {
  let now = 100000;
  const saved = new Map();
  const context = {window: {}, URL, AbortSignal, Date: {now: () => now}, fetch,
    localStorage: {getItem: key => saved.get(key), setItem: (key, value) => saved.set(key, value)}};
  vm.runInNewContext(source, context);
  return {call: context.window.ironwoodPuzzles, advance: ms => {now += ms;}, context};
}
const tick = () => new Promise(resolve => setTimeout(resolve, 0));

test('practice filters, anonymous requests, and concurrent request protection', async () => {
  let finish, calls = 0;
  const h = harness((url, options) => {
    calls++;
    assert.equal(url.origin, 'https://lichess.org');
    assert.equal(url.pathname, '/api/puzzle/next');
    assert.equal(url.searchParams.get('angle'), 'fork');
    assert.equal(url.searchParams.get('difficulty'), 'harder');
    assert.equal(options.credentials, 'omit');
    return new Promise(resolve => {finish = resolve;});
  });
  h.call('request', JSON.stringify({theme: 'fork', difficulty: 'harder'}));
  h.call('request', '{}');
  assert.equal(calls, 1);
  finish({ok: true, json: async () => ({puzzle: {id: 'test'}})});
  await tick();
  assert.equal(JSON.parse(h.call('poll')).data.puzzle.id, 'test');
});

test('429 cooldown prevents requests and allows retry after one minute', async () => {
  let calls = 0;
  const h = harness(async url => {
    calls++;
    assert.equal(url.pathname, '/api/puzzle/daily');
    return {status: 429, ok: false};
  });
  h.call('request', '{"daily":true}');
  await tick();
  assert.match(JSON.parse(h.call('poll')).error, /rate limiting/);
  h.advance(59999);
  h.call('request', '{"daily":true}');
  assert.equal(calls, 1);
  h.advance(1);
  h.call('request', '{"daily":true}');
  assert.equal(calls, 2);
  await tick();
});

test('history persists and unavailable storage produces a visible warning', () => {
  const h = harness(async () => {throw new Error('offline');});
  assert.equal(h.call('history'), '[]');
  assert.equal(h.call('save', '[{"solved":true}]'), '');
  assert.equal(h.call('history'), '[{"solved":true}]');
  h.context.localStorage.setItem = () => {throw new Error('blocked');};
  assert.match(h.call('save', '[]'), /could not be saved/);
});
