import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

globalThis.window = globalThis;
const records = new Map();
const local = new Map();
let failProfileWrite = false;
let failTransaction = false;
globalThis.localStorage = {
  getItem: key => local.get(key) ?? null,
  setItem(key, value) {
    if (failProfileWrite && key === 'ironwood.chess.training.v1') throw new Error('Quota exceeded');
    local.set(key,value);
  },
  removeItem: key => local.delete(key),
};
const db = {
  transaction(_name, mode) {
    const draft = new Map(records);
    const tx = {
      objectStore: () => ({
        getAll() {
          const request = {};
          queueMicrotask(() => { request.result = [...records.values()]; request.onsuccess(); });
          return request;
        },
        delete: id => draft.delete(id),
        put: record => draft.set(record.id,structuredClone(record)),
      }),
      abort() { tx.aborted = true; queueMicrotask(()=>tx.onabort?.()); },
    };
    if (mode === 'readwrite') queueMicrotask(() => {
      if (tx.aborted) return;
      if (failTransaction) { tx.error = new Error('IDB failed'); tx.onabort?.(); return; }
      records.clear(); for (const [id,record] of draft) records.set(id,record);
      tx.oncomplete?.();
    });
    return tx;
  },
};
globalThis.indexedDB = { open() {
  const request = {};
  queueMicrotask(() => { request.result = db; request.onsuccess(); });
  return request;
}};
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url),'utf8');
await import(`data:text/javascript,${encodeURIComponent(source)}`);
const flush = () => new Promise(resolve=>setImmediate(resolve));
const seed = () => {
  records.clear(); local.clear(); failProfileWrite = false; failTransaction = false;
  local.set('ironwood.chess.training.v1',JSON.stringify({selected_id:'p',profiles:[
    {id:'p',name:'Delete me',results:[]},{id:'q',name:'Keep me',results:[]}
  ]}));
  for (const id of ['p','q']) records.set(id,{id,json:JSON.stringify({board:'start',training:{profile_id:id}})});
};

test('delete bridge keeps unrelated data and defaults to preserving game history', async () => {
  seed();
  ironwoodTrainingProfilesAction('delete',JSON.stringify({ids:['p'],deleteGames:false}));
  await flush();
  const status = JSON.parse(ironwoodTrainingProfilesAction('poll',''));
  assert.equal(status.changed,true);
  assert.equal(records.size,2);
  assert.equal(JSON.parse(local.get('ironwood.chess.training.v1')).selected_id,'q');
  assert.equal(JSON.parse(ironwoodTrainingProfilesAction('poll','')).changed,false);
});

test('confirmed history deletion removes only selected profiles games', async () => {
  seed();
  ironwoodTrainingProfilesAction('delete',JSON.stringify({ids:['p'],deleteGames:true}));
  await flush();
  assert.equal(JSON.parse(ironwoodTrainingProfilesAction('poll','')).changed,true);
  assert.deepEqual([...records.keys()],['q']);
});

test('quota failure aborts game deletion and leaves the profile ledger unchanged', async () => {
  seed(); const before = local.get('ironwood.chess.training.v1'); failProfileWrite = true;
  ironwoodTrainingProfilesAction('delete',JSON.stringify({ids:['p'],deleteGames:true}));
  await flush();
  assert.match(JSON.parse(ironwoodTrainingProfilesAction('poll','')).error,/Quota/);
  assert.equal(local.get('ironwood.chess.training.v1'),before);
  assert.equal(records.size,2);
});

test('IDB failure rolls profile changes back and reports failure', async () => {
  seed(); const before = local.get('ironwood.chess.training.v1'); failTransaction = true;
  ironwoodTrainingProfilesAction('delete',JSON.stringify({ids:['p'],deleteGames:true}));
  await flush();
  assert.match(JSON.parse(ironwoodTrainingProfilesAction('poll','')).error,/IDB/);
  assert.equal(local.get('ironwood.chess.training.v1'),before);
  assert.equal(records.size,2);
});
