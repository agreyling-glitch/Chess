import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createTablebaseClient, tablebaseUrl} from '../web/tablebase.js';
const fen = '7k/8/5KQ1/8/8/8/8/8 w - - 98 1';
const tick = () => new Promise(resolve => setImmediate(resolve));
test('eligibility and full FEN including clock', () => {
  assert.equal(new URL(tablebaseUrl(fen)).searchParams.get('fen'), fen);
  assert.throws(() => tablebaseUrl('8/8/8/8/8/8/8/K6k w K - 0 1'));
  assert.throws(() => tablebaseUrl('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1'));
});
test('serial requests, position-specific cache, no authentication', async () => {
  let finish, calls = 0;
  const client = createTablebaseClient(async (url, options) => {
    calls++;
    assert.equal(options.credentials, 'omit');
    assert.equal(options.headers, undefined);
    await new Promise(resolve => { finish = resolve; });
    return {ok:true, json:async () => ({category:'win', moves:[]})};
  });
  assert.equal(client(fen), '');
  assert.equal(client(fen.replace('98', '99')), '');
  assert.equal(calls, 1);
  finish(); await tick();
  assert.equal(JSON.parse(client(fen)).category, 'win');
  assert.equal(calls, 1);
  client(fen.replace('98', '99')); assert.equal(calls, 2);
  finish(); await tick();
});
test('rate limits block retries for a minute then recover', async () => {
  let time = 0, calls = 0;
  const client = createTablebaseClient(async () => {
    calls++;
    return calls === 1 ? {status:429} : {ok:true, json:async () => ({category:'draw', moves:[]})};
  }, () => time);
  client(fen); await tick();
  assert.match(JSON.parse(client(fen, true)).error, /rate limit/);
  assert.equal(calls, 1);
  time = 60001; client(fen, true); await tick();
  assert.equal(JSON.parse(client(fen)).category, 'draw');
});
test('source distinguishes new cache lookups, polling and forced refresh', async () => {
  let calls = 0;
  const client = createTablebaseClient(async () => {
    calls++;
    return {ok:true, json:async () => ({category:'win', moves:[]})};
  });
  client(fen, false, true); await tick();
  assert.equal(JSON.parse(client(fen)).source, 'lichess');
  assert.equal(JSON.parse(client(fen)).source, 'lichess');
  assert.equal(JSON.parse(client(fen, false, true)).source, 'cache');
  assert.equal(JSON.parse(client(fen)).source, 'cache');
  assert.equal(calls, 1);
  client(fen, true, true); await tick();
  assert.equal(JSON.parse(client(fen)).source, 'lichess');
  assert.equal(calls, 2);
});
test('offline, missing tables and malformed responses are retryable', async () => {
  for (const fetcher of [async () => {throw new TypeError('offline');}, async () => ({status:404}), async () => ({ok:true,json:async () => ({category:'win',moves:[{}]})})]) {
    const client = createTablebaseClient(fetcher);
    client(fen); await tick();
    assert.ok(JSON.parse(client(fen)).error);
    assert.equal(client(fen, true), ''); await tick();
  }
});
import {createLocalTablebaseClient} from '../web/tablebase.js';

test('local tablebases authenticate only to Companion and refresh without an online fallback', async () => {
  let calls = 0;
  const local = createLocalTablebaseClient(() => ({endpoint:'http://127.0.0.1:8790',credential:'owner'}), async (url, options) => {
    calls++;
    assert.equal(new URL(url).origin, 'http://127.0.0.1:8790');
    assert.equal(new URL(url).searchParams.get('fen'), fen);
    assert.equal(options.headers.Authorization, 'Bearer owner');
    return {ok:true,json:async()=>({category:'draw', moves:[], source:'companion'})};
  });
  assert.equal(local(fen, false, true), ''); await tick();
  assert.equal(JSON.parse(local(fen)).source, 'companion');
  local(fen, true, true); await tick(); assert.equal(calls, 2);
  const offline = createLocalTablebaseClient(() => ({endpoint:'http://127.0.0.1:8790',credential:'owner'}), async () => { throw new Error('missing local tables'); });
  offline(fen); await tick(); assert.match(JSON.parse(offline(fen)).error, /missing local tables/);
  assert.match(JSON.parse(createLocalTablebaseClient(() => null)(fen)).error, /Connect Companion/);
});

test('local source ignores stale responses after switching position or credentials', async () => {
  const finish = [];
  let config = {endpoint:'http://127.0.0.1:8790',credential:'first'};
  const local = createLocalTablebaseClient(() => config, () => new Promise(resolve => finish.push(resolve)));
  local(fen); config = {...config, credential:'second'}; local(fen);
  finish[1]({ok:true,json:async()=>({category:'draw',moves:[]})}); await tick();
  finish[0]({ok:true,json:async()=>({category:'win',moves:[]})}); await tick();
  assert.equal(JSON.parse(local(fen)).category,'draw');
});
