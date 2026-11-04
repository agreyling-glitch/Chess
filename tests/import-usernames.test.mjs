import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

globalThis.window = globalThis;
const values = new Map();
globalThis.localStorage = {
  getItem: key => values.get(key) ?? null,
  setItem: (key, value) => values.set(key, value),
};
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const moduleUrl = `data:text/javascript,${encodeURIComponent(source)}`;
const { recentImportUsernames: recent, rememberImportUsername: remember } = await import(moduleUrl);

test('keeps the latest 20 distinct usernames per provider across reloads', async () => {
  values.clear();
  for (let i=0;i<25;i++) remember('lichess', `Player${i}`);
  remember('chesscom', 'MagnusCarlsen');
  remember('lichess', ' player10 ');
  assert.equal(recent('lichess').length, 20);
  assert.equal(recent('lichess')[0], 'player10');
  assert.equal(recent('lichess').filter(name => name.toLowerCase() === 'player10').length, 1);
  assert.ok(!recent('lichess').includes('Player4'));
  assert.deepEqual(recent('chesscom'), ['MagnusCarlsen']);
  const reloaded = await import(moduleUrl + '#reload');
  assert.deepEqual(reloaded.recentImportUsernames('lichess'), recent('lichess'));
  assert.deepEqual(JSON.parse(window.ironwoodRecentImportUsernames('chesscom')), ['MagnusCarlsen']);
});

test('ignores links and invalid names and tolerates damaged or unavailable storage', () => {
  values.clear();
  for (const input of ['https://lichess.org/abcdefgh', 'bad name', '', 'a', null]) remember('lichess', input);
  remember('unknown', 'Player');
  assert.deepEqual(recent('lichess'), []);
  assert.equal(values.size, 0);
  for (const stored of ['bad json', '{}', 'null']) {
    values.set('ironwood.import-usernames.lichess', stored);
    assert.deepEqual(recent('lichess'), []);
  }
  values.set('ironwood.import-usernames.lichess', JSON.stringify(['Ada','ada',null,'bad name','Other']));
  assert.deepEqual(recent('lichess'), ['Ada','Other']);
  const storage = globalThis.localStorage;
  globalThis.localStorage = {getItem() {throw Error('blocked');}, setItem() {throw Error('full');}};
  try { assert.deepEqual(recent('lichess'), []); assert.doesNotThrow(() => remember('lichess','Player')); }
  finally {globalThis.localStorage = storage;}
});
