import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { gameDetails, gameMatchesCategory, backupSummary, mergeTrainingProfiles, validateTrainingProfiles } =
  await import(`data:text/javascript,${encodeURIComponent(source)}`);
const session = (id, side = 'White') => ({ id, profile_id:'p', profile_name:'Player <One>', side,
  opponent_elo:1320, rating_before:1320, rating_after:1336,
  started_at:'2026-10-03T12:00:00.000Z', completed_at:'2026-10-03T12:05:00.000Z' });
const result = (id, side = 'White') => ({ session:session(id,side), result:side === 'White' ? '1-0':'0-1', score:1, timed_out:false });
const profiles = results => ({ selected_id:'p', profiles:[{ id:'p', name:'Player <One>', results }] });

test('training games use profile names, color ratings, and a separate category', () => {
  const json = JSON.stringify({ board:'start', player_side:'White', training:session('g'), live_moves:['e4'], result:'1-0' });
  const details = gameDetails(json);
  assert.equal(details.category,'training');
  assert.equal(details.white,'Player <One>');
  assert.equal(details.black,'Stockfish 1320 Elo');
  assert.equal(details.trainingProfileId,'p');
  assert.equal(details.ratingAfter,1336);
  assert.ok(gameMatchesCategory(details,'training'));
  assert.equal(gameMatchesCategory(details,'mine'),false);
  assert.ok(gameMatchesCategory({...details,favorite:true},'favorites'));
  const black = gameDetails(JSON.stringify({ board:'start', player_side:'Black', training:session('b','Black') }));
  assert.equal(black.black,'Player <One>');
  assert.equal(black.white,'Stockfish 1320 Elo');
});

test('backups preserve training category and profile ledger; old backups remain supported', () => {
  const json = JSON.stringify({board:'start',training:session('g')});
  const summary = backupSummary({format:'ironwood-backup',version:1,games:[{id:'g',json,category:'mine'}],training_profiles:profiles([result('g')])});
  assert.equal(summary.training,1);
  assert.equal(summary.profiles,1);
  assert.equal(summary.mine,0);
  assert.equal(backupSummary({format:'ironwood-backup',version:1,games:[]}).profiles,0);
});

test('merging repeated backups does not duplicate rating results or combine different profiles', () => {
  const white = result('w'); const black = result('b','Black');
  black.session.completed_at = '2026-10-03T12:15:00.000Z';
  const first = mergeTrainingProfiles(profiles([white]),profiles([white,black]));
  const second = mergeTrainingProfiles(first,profiles([white,black]));
  assert.equal(second.profiles[0].results.length,2);
  assert.equal(second.profiles[0].results[0].session.rating_after,1336);
  assert.equal(second.profiles[0].results[1].session.rating_after,1336);
  const other = {selected_id:'q',profiles:[{id:'q',name:'Other',results:[]}]};
  assert.equal(mergeTrainingProfiles(second,other).profiles.length,2);
  assert.equal(second.profiles[0].results.length,2);
});

test('training backup validation rejects contradictory results and duplicate settlement IDs', () => {
  const wrong = result('bad'); wrong.score = 0;
  assert.throws(()=>validateTrainingProfiles(profiles([wrong])),/Invalid training result/);
  assert.throws(()=>validateTrainingProfiles(profiles([result('g'),result('g')])),/Invalid training result/);
  const outOfRange = result('bad'); outOfRange.session.opponent_elo = 300;
  assert.throws(()=>validateTrainingProfiles(profiles([outOfRange])),/Invalid training result/);
});


test('saved training games display the renamed profile by stable ID', () => {
  const json = JSON.stringify({ player_side:'Black', training:session('renamed', 'Black') });
  globalThis.localStorage = { getItem: () => JSON.stringify({ profiles:[{ id:'p', name:'New name' }] }) };
  try {
    const details = gameDetails(json);
    assert.equal(details.trainingProfileName, 'New name');
    assert.equal(details.black, 'New name');
    assert.equal(details.title, 'Stockfish 1320 Elo vs New name');
    assert.equal(details.trainingProfileId, 'p');
    globalThis.localStorage = { getItem: () => { throw new Error('unavailable'); } };
    assert.equal(gameDetails(json).black, 'Player <One>');
  } finally {
    delete globalThis.localStorage;
  }
});
