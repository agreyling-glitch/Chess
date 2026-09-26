import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// Exercise the storage bridge and serialized writes with an asynchronous IDB fixture.
const records = new Map();
let writes = 0;
const db = {
  transaction() {
    const transaction = {
      objectStore() {
        return {
          get(id) {
            const request = {};
            queueMicrotask(() => {
              request.result = structuredClone(records.get(id));
              request.onsuccess();
              queueMicrotask(() => transaction.oncomplete());
            });
            return request;
          },
          put(record) { records.set(record.id, structuredClone(record)); writes++; },
        };
      },
    };
    return transaction;
  },
};
globalThis.window = globalThis;
globalThis.indexedDB = { open() {
  const request = {};
  queueMicrotask(() => { request.result = db; request.onsuccess(); });
  return request;
} };
const local = new Map([['ironwood.chess.active-game-id.v1', 'personal-game'], ['ironwood.chess.active-game-category.v1', 'mine']]);
globalThis.localStorage = { getItem: key => local.get(key), setItem: (key, value) => local.set(key, value) };
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
await import(`data:text/javascript,${encodeURIComponent(source)}`);
const flush = () => new Promise(resolve => setImmediate(resolve));
const makeGame = analysis => JSON.stringify({ review_pgn: '[White "Ada"]\n[Black "Ben"]\n[Site "freechess.org"]\n\n1.e4 e5 1-0', game_analysis: analysis, game_analysis_paused: true });

test('observed autosaves update one record without replacing the active personal game', async () => {
  ironwoodStoreObservedGame(makeGame([]), 'game-1');
  ironwoodStoreObservedGame(makeGame([{eval_cp: 12}, null, null]), 'game-1');
  await flush();
  assert.equal(writes, 2);
  assert.equal(records.size, 1);
  const saved = [...records.values()][0];
  assert.equal(saved.category, 'observed');
  assert.equal(saved.source, 'FICS');
  assert.equal(saved.analysisStatus, 'Partial');
  assert.equal(saved.analyzed, 1);
  assert.equal(local.get('ironwood.chess.active-game-id.v1'), 'personal-game');
  assert.equal(local.get('ironwood.chess.game.v1'), undefined);
  saved.favorite = true;
  ironwoodStoreObservedGame(makeGame([{eval_cp: 12}, {eval_cp: 20}, {eval_cp: 5}]), 'game-1');
  await flush();
  assert.equal(records.size, 1);
  assert.equal(records.get(saved.id).favorite, true);
  assert.equal(records.get(saved.id).analysisStatus, 'Complete');
});

test('reopened observed games keep their category and source when analysis is saved', async () => {
  const saved = [...records.values()][0];
  local.set('ironwood.chess.active-game-id.v1', saved.id);
  local.set('ironwood.chess.active-game-category.v1', 'observed');
  ironwoodStoreCurrentGame(makeGame([{eval_cp: 18}, {eval_cp: 22}, {eval_cp: 7}]));
  await flush();
  assert.equal(records.size, 1);
  assert.equal(records.get(saved.id).source, 'FICS');
  assert.equal(records.get(saved.id).category, 'observed');
  assert.equal(records.get(saved.id).favorite, true);
  assert.equal(JSON.parse(records.get(saved.id).json).game_analysis[0].eval_cp, 18);
});
