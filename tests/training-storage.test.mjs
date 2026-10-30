import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { gameDetails, gameMatchesCategory, backupSummary, mergeTrainingProfiles, validateTrainingProfiles, buildTrainingProfileExport, validateTrainingProfileImport, removeTrainingProfiles, newTrainingGames } =
  await import(`data:text/javascript,${encodeURIComponent(source)}`);
const session = (id, side = 'White') => ({ id, profile_id:'p', profile_name:'Player <One>', side,
  opponent_elo:1320, rating_before:1320, rating_after:1336,
  started_at:'2026-10-03T12:00:00.000Z', completed_at:'2026-10-03T12:05:00.000Z' });
const result = (id, side = 'White') => ({ session:session(id,side), result:side === 'White' ? '1-0':'0-1', score:1, timed_out:false });
const profiles = results => ({ selected_id:'p', profiles:[{ id:'p', name:'Player <One>', results }] });

test('lc0 profiles preserve node difficulty, merge levels once, and export separately from Elo', () => {
  const engine={kind:'lc0',identity:'http://localhost:8790/lc0',name:'Lc0',options:{Backend:'cuda'}};
  const lc0 = id => {
    const entry=result(id);
    Object.assign(entry.session,{engine,node_budget:16,difficulty_level:3,opponent_elo:0,rating_before:3,rating_after:4});
    return {selected_id:'p',profiles:[{id:'p',name:'Lc0 practice',engine,starting_level:3,starting_elo:1320,results:[entry]}]};
  };
  const first=lc0('one'), second=lc0('two');
  const merged=mergeTrainingProfiles(first,second);
  assert.deepEqual(merged.profiles[0].results.map(r=>r.session.rating_after),[4,5]);
  assert.equal(merged.profiles[0].results[1].session.node_budget,16,'played difficulty remains frozen when the ledger is replayed');
  assert.equal(mergeTrainingProfiles(merged,second).profiles[0].results.length,2);
  const bundle=buildTrainingProfileExport(first,[],['p']);
  assert.deepEqual(validateTrainingProfileImport(bundle).trainingProfiles,first);
  const details=gameDetails(JSON.stringify({player_side:'White',training:first.profiles[0].results[0].session}));
  assert.equal(details.black,'Lc0');assert.equal(details.trainingOpponentLevel,3);assert.equal(details.trainingNodeBudget,16);
  assert.equal(details.trainingEngineKind,'lc0');
  const wrong=lc0('wrong');wrong.profiles[0].results[0].session.node_budget=999;
  assert.throws(()=>validateTrainingProfiles(wrong),/Invalid training result/);
  assert.throws(()=>mergeTrainingProfiles(profiles([]),first),/engine or starting difficulty differs/);
});

test('training games use profile names, color ratings, and a separate category', () => {
  const json = JSON.stringify({ board:'start', player_side:'White', training:session('g'), live_moves:['e4'], result:'1-0' });
  const details = gameDetails(json);
  assert.equal(details.category,'training');
  assert.equal(details.white,'Player <One>');
  assert.equal(details.black,'Stockfish');
  assert.equal(details.trainingProfileId,'p');
  assert.equal(details.ratingAfter,1336);
  assert.ok(gameMatchesCategory(details,'training'));
  assert.equal(gameMatchesCategory(details,'mine'),false);
  assert.ok(gameMatchesCategory({...details,favorite:true},'favorites'));
  const black = gameDetails(JSON.stringify({ board:'start', player_side:'Black', training:session('b','Black') }));
  assert.equal(black.black,'Player <One>');
  assert.equal(black.white,'Stockfish');
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
    assert.equal(details.title, 'Stockfish vs New name');
    assert.equal(details.trainingProfileId, 'p');
    globalThis.localStorage = { getItem: () => { throw new Error('unavailable'); } };
    assert.equal(gameDetails(json).black, 'Player <One>');
  } finally {
    delete globalThis.localStorage;
  }
});

const saved = (id, profileId = 'p', sessionId = id) => ({ id, category:'training', json:JSON.stringify({board:'start', training:{...session(sessionId), profile_id:profileId}}) });

test('profile export includes only selected profiles and linked games and round trips', () => {
  const current = profiles([result('g')]);
  current.profiles.push({id:'q', name:'Other', results:[]});
  const bundle = buildTrainingProfileExport(current, [saved('g'),saved('h','q')], ['p']);
  assert.equal(bundle.training_profiles.profiles.length,1);
  assert.deepEqual(bundle.games.map(g=>g.id),['g']);
  const restored = validateTrainingProfileImport(JSON.parse(JSON.stringify(bundle)));
  assert.deepEqual(restored.trainingProfiles,profiles([result('g')]));
  assert.equal(restored.games.length,1);
  assert.throws(()=>buildTrainingProfileExport(current,[],[]),/Select/);
  bundle.games.push(saved('bad','q'));
  assert.throws(()=>validateTrainingProfileImport(bundle),/outside/);
});

test('profile deletion supports preserving games, selecting a survivor, and deleting multiple profiles', () => {
  const current = profiles([result('g')]);
  current.profiles.push({id:'q',name:'Other',results:[]});
  const games = [saved('g'),saved('h','q')];
  const kept = removeTrainingProfiles(current,games,['p'],false);
  assert.equal(kept.profiles.selected_id,'q');
  assert.deepEqual(kept.deleteIds,[]);
  const removed = removeTrainingProfiles(current,games,['p'],true);
  assert.deepEqual(removed.deleteIds,['g']);
  const all = removeTrainingProfiles(current,games,['p','q'],true);
  assert.equal(all.profiles.profiles.length,0);
  assert.equal(all.profiles.selected_id,'');
  assert.deepEqual(all.deleteIds,['g','h']);
  assert.equal(current.profiles.length,2);
});

test('repeated profile imports deduplicate saved game sessions and remap unrelated ID collisions', () => {
  const existing = [saved('original','p','g')];
  assert.deepEqual(newTrainingGames(existing,[saved('different','p','g')]),[]);
  const imported = newTrainingGames(existing,[saved('original','p','h')]);
  assert.equal(imported.length,1);
  assert.notEqual(imported[0].id,'original');
  assert.equal(JSON.parse(imported[0].json).training.id,'h');
  assert.equal(newTrainingGames([...existing,...imported],[saved('original','p','h')]).length,0);
});


test('Maia3 profile backups preserve low Elo, color progress, engine options and saved game labels',()=>{
  const engine={kind:'maia3',identity:'http://localhost:8790/maia3',name:'Maia3 79M (GPU)',options:{Temperature:'0',Elo:'1500'}};
  const make=(id,side='White',start=800,win=true)=>{
    const entry=result(id,side);Object.assign(entry.session,{engine,node_budget:0,difficulty_level:0,opponent_elo:start,rating_before:start,rating_after:start+(win?16:-16)});
    if(!win){entry.result=side==='White'?'0-1':'1-0';entry.score=0;}return entry;
  };
  const a={selected_id:'p',profiles:[{id:'p',name:'Maia practice',engine,starting_elo:800,starting_level:1,results:[make('01-white'),make('02-black','Black',800,false)]}]};
  assert.deepEqual(validateTrainingProfiles(a),a);
  const bundle=buildTrainingProfileExport(a,[],['p']);assert.deepEqual(validateTrainingProfileImport(bundle).trainingProfiles,a);
  const b=structuredClone(a);b.profiles[0].results=[make('03-next','White',816)];
  const merged=mergeTrainingProfiles(a,b);assert.deepEqual(merged.profiles[0].results.map(e=>e.session.rating_after),[816,784,832]);
  assert.equal(mergeTrainingProfiles(merged,b).profiles[0].results.length,3);
  const game=gameDetails(JSON.stringify({board:'start',player_side:'White',training:a.profiles[0].results[0].session}));
  assert.equal(game.black,'Maia3 79M (GPU)');assert.equal(game.trainingOpponentElo,800);assert.equal(game.trainingEngineKind,'maia3');
  assert.equal(game.category,'training');
  const zero={selected_id:'p',profiles:[{...a.profiles[0],starting_elo:0,results:[]}]};assert.equal(mergeTrainingProfiles(zero,zero).profiles[0].starting_elo,0);
  const wrong=structuredClone(a);wrong.profiles[0].results[0].session.opponent_elo=5001;assert.throws(()=>validateTrainingProfiles(wrong),/Invalid training result/);
  wrong.profiles[0].results=[];wrong.profiles[0].starting_elo=-1;assert.throws(()=>validateTrainingProfiles(wrong),/Invalid starting training Elo/);
  assert.throws(()=>mergeTrainingProfiles(profiles([]),a),/engine or starting difficulty differs/);
});

test('Maia3 replay clamps rating boundaries and preserves zero instead of Stockfish defaults',()=>{
  const engine={kind:'maia3',identity:'http://localhost:8790/maia',name:'Maia3',options:{}};
  for(const [start,win,expected] of [[0,false,0],[5000,true,5000]]){
    const entry=result('edge');Object.assign(entry.session,{engine,opponent_elo:start,rating_before:start,rating_after:expected});if(!win){entry.result='0-1';entry.score=0;}
    const p={selected_id:'p',profiles:[{id:'p',name:'Edges',engine,starting_elo:start,results:[entry]}]};
    assert.equal(mergeTrainingProfiles(null,p).profiles[0].results[0].session.rating_after,expected);
    assert.equal(mergeTrainingProfiles(null,p).profiles[0].results[0].session.rating_before,start);
  }
});
