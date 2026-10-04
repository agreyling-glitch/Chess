import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const body = source.slice(source.indexOf('    const removeGames = async targets => {') + '    const removeGames = async targets => {'.length, source.indexOf('    const render = () => {'));

test('deleting the active game restores the saved-games view after reset', async () => {
  const session = new Map();
  const local = new Map([['active', 'test-game'], ['current', '{}'], ['category', 'mine']]);
  const games = [{ id:'test-game' }, { id:'keep' }];
  let reloaded = 0;
  const context = vm.createContext({
    deleting:false, games, selectedIds:new Set(['test-game']), importedFingerprints:new Set(),
    category:'training', profileFilter:'profile', playerFilter:'*', searchQuery:'test', analysisFilter:'complete', page:2,
    ACTIVE_ID_KEY:'active', CURRENT_GAME_KEY:'current', ACTIVE_CATEGORY_KEY:'category', LIBRARY_RETURN_KEY:'return',
    localStorage:{getItem:key=>local.get(key),removeItem:key=>local.delete(key)},
    sessionStorage:{setItem:(key,value)=>session.set(key,value)},
    confirmDeleteGames:async()=>true, deleteGames:async ids=>assert.deepEqual([...ids],['test-game']),
    storedFingerprint:()=>null, dialog:{classList:{contains:()=>true}},
    location:{reload:()=>reloaded++}, render:()=>assert.fail('active game must reset first'),
    selection:{append:()=>assert.fail('unexpected deletion error')}, document:{createElement:()=>assert.fail('unexpected error')},
  });
  await vm.runInContext(`(async targets => {${body.replace(/};\s*$/, '')}})([games[0]])`,context);
  assert.equal(reloaded,1);
  assert.deepEqual(games,[{id:'keep'}]);
  assert.equal(local.size,0);
  const restored = JSON.parse(session.get('return'));
  assert.equal(restored.category,'training');
  assert.equal(restored.page,2);
  assert.equal(restored.maximized,true);
  let opened;
  const startup = source.slice(source.indexOf("if (typeof document !== 'undefined')", source.indexOf('// Deleting the active game resets')));
  vm.runInNewContext(startup, {
    document:{readyState:'complete'}, LIBRARY_RETURN_KEY:'return',
    sessionStorage:{getItem:key=>session.get(key),removeItem:key=>session.delete(key)},
    window:{ironwoodOpenGameLibrary:state=>opened=state},
  });
  assert.equal(opened.profileFilter,'profile');
  assert.equal(session.size,0);
});
