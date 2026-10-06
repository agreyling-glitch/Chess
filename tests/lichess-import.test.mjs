import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { lichessExportUrl, importFilters } = await import(`data:text/javascript,${encodeURIComponent(source)}`);

test('filters validate dates and generate inclusive UTC bounds', () => {
  const url = new URL(lichessExportUrl('Ada',{limit:100,speed:'blitz',from:'2026-09-01',to:'2026-09-30'}));
  assert.equal(url.searchParams.get('max'),'100');
  assert.equal(url.searchParams.get('perfType'),'blitz');
  assert.equal(Number(url.searchParams.get('since')),Date.parse('2026-09-01T00:00:00Z'));
  assert.equal(Number(url.searchParams.get('until')),Date.parse('2026-10-01T00:00:00Z')-1);
  for (const bad of [{from:'2026-02-30'},{from:'2026-10-01',to:'2026-09-01'},{limit:999},{speed:'invalid'}]) assert.throws(()=>importFilters(bad));
  assert.doesNotThrow(()=>lichessExportUrl('https://lichess.org/abcd1234',{from:'invalid'}));
});

test('username exports are bounded to completed games', () => {
  const url = new URL(lichessExportUrl(' Ada '));
  assert.equal(url.pathname, '/api/games/user/Ada');
  assert.equal(url.searchParams.get('max'), '20');
  assert.equal(url.searchParams.get('ongoing'), 'false');
});
test('game URLs discard player secrets and reject other hosts', () => {
  assert.equal(new URL(lichessExportUrl('https://lichess.org/abcd1234WXYZ/black#12')).pathname, '/game/export/abcd1234');
  for (const input of ['https://evil.com/abcd1234', 'https://lichess.org@evil.com/abcd1234', 'https://lichess.org/study/abcd1234', 'bad name']) {
    assert.throws(() => lichessExportUrl(input));
  }
});
test('fetch hands PGN off once, rejects ongoing games, and honors rate limits', async () => {
  let calls = 0;
  let body = '[Event "Rated blitz"]\n[Result "1-0"]\n\n1. e4 1-0';
  let status = 200;
  globalThis.fetch = async (url, options) => {
    calls++;
    assert.equal(options.credentials, 'omit');
    assert.equal(options.headers.Accept, 'application/x-chess-pgn');
    return { ok: status === 200, status, text: async () => body };
  };
  await window.ironwoodFetchLichess('Ada');
  assert.equal(window.ironwoodPollLichess(), body);
  assert.equal(window.ironwoodPollLichess(), '');
  body = '[Event "Live"]\n[Result "*"]';
  await window.ironwoodFetchLichess('https://lichess.org/abcd1234');
  assert.equal(window.ironwoodPollLichess(), '');
  assert.match(window.ironwoodLichessStatus(), /still in progress/);
  status = 429;
  await window.ironwoodFetchLichess('Ada');
  const before = calls;
  await window.ironwoodFetchLichess('Ada');
  assert.equal(calls, before);
  assert.match(window.ironwoodLichessStatus(), /full minute/);
});
