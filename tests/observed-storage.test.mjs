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
const { gameDetails, sortSavedGames } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
const flush = () => new Promise(resolve => setImmediate(resolve));
const makeGame = analysis => JSON.stringify({ review_pgn: '[White "Ada"]\n[Black "Ben"]\n[Site "freechess.org"]\n\n1.e4 e5 1-0', game_analysis: analysis, game_analysis_paused: true });

test('personal game names use the saved opponent on either side, with legacy Stockfish fallback', () => {
  for (const player_side of ['White', 'Black']) {
    const details = gameDetails(JSON.stringify({ player_side, engine_config: { opponent: 'Lc0' } }));
    assert.equal(details.title, player_side === 'White' ? 'You vs Lc0 · Good Gyal' : 'Lc0 · Good Gyal vs You');
  }
  assert.equal(gameDetails(JSON.stringify({ player_side: 'Black' })).title, 'Stockfish 19 vs You');
  assert.equal(gameDetails(JSON.stringify({ engine_enabled: false, engine_config: { opponent: 'Lc0' } })).title, 'White vs Black');
  assert.equal(gameDetails(JSON.stringify({ player_side: 'White', engine_config: { opponent: 'Stockfish' } })).title, 'You vs Stockfish 19');
  assert.equal(gameDetails(JSON.stringify({ review_pgn: '[White "Ada"]\n[Black "Ben"]', engine_config: { opponent: 'Lc0' } })).title, 'Ada vs Ben');
});

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


test('zero-move games do not enter the library, including analysis-only positions', async () => {
  const before=records.size, beforeWrites=writes;
  const empty=JSON.stringify({live_moves:[],game_analysis:[{eval_cp:0},{eval_cp:10}]});
  ironwoodStoreCurrentGame(empty); ironwoodStoreObservedGame(empty,'empty-observed');
  ironwoodStoreImportedGame(JSON.stringify({review_pgn:'[White "Ada"]\n[Black "Ben"]\n\n*'}));
  await flush(); assert.equal(records.size,before); assert.equal(writes,beforeWrites);
  assert.equal(gameDetails(empty).moves,0);
  local.set('ironwood.chess.active-game-id.v1','first-move-game');
  ironwoodStoreCurrentGame(JSON.stringify({live_moves:['e4']})); await flush();
  assert.equal(records.get('first-move-game').moves,1);
});

test('move sorting is numeric, includes existing empty games and breaks ties by recency', () => {
  const games=[{id:'long',moves:100,updatedAt:1},{id:'short-old',moves:2,updatedAt:2},{id:'empty',moves:0,updatedAt:3},{id:'short-new',moves:2,updatedAt:4}];
  assert.deepEqual(sortSavedGames(games,'moves-asc').map(g=>g.id),['empty','short-new','short-old','long']);
  assert.deepEqual(sortSavedGames(games,'moves-desc').map(g=>g.id),['long','short-new','short-old','empty']);
  assert.deepEqual(sortSavedGames(games).map(g=>g.id),['short-new','empty','short-old','long']);
  assert.equal(games[0].id,'long');
});


test('puzzle autosaves update one attempt in Puzzles and empty attempts remain unsaved', async () => {
  const before=records.size;
  const pgn=moves=>JSON.stringify({review_pgn:`[Event "Lichess puzzle"]\n[PuzzleId "abc12"]\n\n${moves} *`});
  ironwoodStartNewStoredGame('puzzles');
  const id=local.get('ironwood.chess.active-game-id.v1');
  ironwoodStoreCurrentGame(pgn('')); await flush(); assert.equal(records.size,before);
  ironwoodStoreCurrentGame(pgn('1. e4 e5')); await flush();
  ironwoodStoreCurrentGame(pgn('1. e4 e5 2. Nf3')); await flush();
  assert.equal(records.size,before+1); assert.equal(records.get(id).category,'puzzles');
  assert.equal(records.get(id).moves,3); assert.equal(records.get(id).title,'Lichess puzzle abc12');
  assert.equal(local.get('ironwood.chess.active-game-category.v1'),'puzzles');
  ironwoodStartNewStoredGame('puzzles'); ironwoodStoreCurrentGame(pgn('1. e4')); await flush();
  assert.equal(records.size,before+2);
});
