import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

test('an early upload waits for saved background restoration before decoding and selecting its image', async () => {
  const records = new Map(), local = new Map(), decoded = [], revoked = [];
  let releaseInitialRead;
  const initialRead = new Promise(resolve => { releaseInitialRead = resolve; });
  let firstRead = true;
  const db = { transaction() {
    const transaction = { objectStore() { return {
      getAll() {
        const request = {};
        const wait = firstRead ? initialRead : Promise.resolve();
        firstRead = false;
        void wait.then(() => {
          request.result = [...records.values()]; transaction.oncomplete();
        });
        return request;
      },
      put(record) {
        const request = {};
        records.set(record.id, record);
        queueMicrotask(() => transaction.oncomplete());
        return request;
      },
    }; } };
    return transaction;
  } };
  const target = {
    indexedDB: { open() {
      const request = {};
      queueMicrotask(() => { request.result = db; request.onsuccess(); });
      return request;
    } },
    localStorage: { getItem: key => local.get(key) ?? null, setItem: (key, value) => local.set(key, value) },
    crypto: { randomUUID: () => 'new-image' }, Uint8Array,
    URL: { createObjectURL: () => 'blob:new-image', revokeObjectURL: url => revoked.push(url) },
    Image: class {
      naturalWidth = 1; naturalHeight = 1;
      async decode() { decoded.push(this.src); }
    },
    document: { createElement(tag) {
      assert.equal(tag, 'canvas');
      return { getContext: () => ({ drawImage() {}, getImageData: () => ({ data: [1, 2, 3, 255] }) }), toDataURL: () => 'data:image/jpeg,new' };
    } },
    alert: message => assert.fail(message),
  };
  target.window = target;
  const source = readFileSync(new URL('../web/background-image.js', import.meta.url), 'utf8').replace(/^export /gm, '');
  runInNewContext(`${source}\nglobalThis.upload = uploadBackgroundFile;`, target);
  const upload = target.upload({ name: 'new.png' });
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(decoded, [], 'the upload cannot race the pending restoration');
  releaseInitialRead();
  await upload;
  assert.deepEqual(decoded, ['blob:new-image']);
  assert.equal(local.get('ironwood-background-selected'), 'new-image');
  assert.equal(records.get('new-image').name, 'new.png');
  assert.equal(target.ironwoodTakeBackground().length, 4);
  assert.deepEqual(revoked, ['blob:new-image']);
});
