import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const source = readFileSync(new URL('../web/board-themes.js', import.meta.url), 'utf8')
  .replace("import { boardThemes } from './board-themes/manifest.js';", 'const boardThemes = {};');
const { createBoardThemeLoader, boardAssetNames } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function fixture() {
  const bodies = new Map(), manifest = {};
  for (const [theme, names] of Object.entries(boardAssetNames)) {
    manifest[theme] = {};
    for (const [index, name] of names.entries()) {
      let bytes;
      if (name.endsWith('.glb')) {
        bytes = new Uint8Array(32);
        bytes.set([103, 108, 84, 70]);
        const view = new DataView(bytes.buffer);
        view.setUint32(4, 2, true); view.setUint32(8, bytes.length, true);
        bytes[31] = index;
      } else bytes = new Uint8Array([255, 216, 255, 224, 0, 8, index, 0, 255, 217]);
      const path = `/board-themes/${theme}/${sha(bytes)}/${name}`;
      manifest[theme][name] = path;
      bodies.set(path, bytes);
    }
  }
  return { bodies, manifest };
}
function memoryCaches() {
  const records = new Map();
  return {
    records,
    async open(name) {
      assert.equal(name, 'ironwood-board-themes-v1');
      return {
        async match(url) { return records.get(url)?.clone(); },
        async put(url, response) { records.set(url, response.clone()); },
        async delete(url) { return records.delete(url); },
      };
    },
  };
}

test('3D packs load only on request, preserve the Rust order and share in-flight and completed requests', async () => {
  const { bodies, manifest } = fixture(), requests = [];
  const load = createBoardThemeLoader({ manifest, cacheStorage: memoryCaches(), fetcher: async (url, options) => {
    requests.push(url); assert.equal(options.credentials, 'same-origin');
    return new Response(bodies.get(url));
  } });
  assert.deepEqual(requests, []);
  const first = load('wood');
  assert.equal(load('wood'), first);
  const wood = await first;
  assert.equal(wood.length, 14);
  assert.deepEqual(wood, boardAssetNames.wood.map(name => bodies.get(manifest.wood[name])));
  assert.ok(requests.every(url => url.startsWith('/board-themes/wood/')));
  assert.equal(requests.length, 14);
  assert.equal(await load('wood'), wood);
  assert.equal(requests.length, 14);
  const marble = await load('marble');
  assert.deepEqual(marble, boardAssetNames.marble.map(name => bodies.get(manifest.marble[name])));
  assert.equal(requests.length, 28);
});

test('a pack resolves only after every geometry and texture response is verified', async () => {
  const { bodies, manifest } = fixture();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let finished = false;
  const lastPath = manifest.wood['pieces_black_nor.jpg'];
  const load = createBoardThemeLoader({ manifest, cacheStorage: memoryCaches(), fetcher: async url => {
    if (url === lastPath) await gate;
    return new Response(bodies.get(url));
  } });
  const pending = load('wood').then(bytes => { finished = true; return bytes; });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(finished, false, 'thirteen files must not expose a partial theme');
  release();
  assert.equal((await pending).length, 14);
});

test('visited 3D packs work offline in a new loader without network revalidation', async () => {
  const { bodies, manifest } = fixture(), cacheStorage = memoryCaches();
  const online = createBoardThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) });
  await online('wood');
  let requests = 0;
  const offline = createBoardThemeLoader({ manifest, cacheStorage, fetcher: async () => { requests++; throw new TypeError('offline'); } });
  assert.equal((await offline('wood')).length, 14);
  assert.equal(requests, 0);
  await assert.rejects(offline('marble'), /Marble and Glass.*Connect to the internet and retry/);
  assert.ok(requests > 0);
});

test('changed content hashes cannot return old geometry and a failed pack can retry', async () => {
  const { bodies, manifest } = fixture(), cacheStorage = memoryCaches();
  await createBoardThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) })('wood');
  const changed = bodies.get(manifest.wood['king.glb']).slice();
  changed[31] = 99;
  const changedPath = `/board-themes/wood/${sha(changed)}/king.glb`;
  manifest.wood['king.glb'] = changedPath;
  let online = false, requests = 0;
  const load = createBoardThemeLoader({ manifest, cacheStorage, fetcher: async url => {
    requests++; assert.equal(url, changedPath);
    if (!online) throw new TypeError('offline');
    return new Response(changed);
  } });
  await assert.rejects(load('wood'), /Could not load the Wood 3D board/);
  online = true;
  assert.deepEqual((await load('wood'))[6], changed);
  assert.equal(requests, 2);
});

test('GLB headers, JPEG headers, full content hashes and HTTP status are validated', async () => {
  const { bodies, manifest } = fixture();
  const badGlb = bodies.get(manifest.wood['board.glb']).slice();
  badGlb[8] = 31;
  const changedGlb = bodies.get(manifest.wood['board.glb']).slice();
  changedGlb[31] = 99;
  for (const [name, response, error] of [
    ['board.glb', new Response('<html>error</html>'), /not a GLB/],
    ['board.glb', new Response(badGlb), /invalid GLB header/],
    ['board.glb', new Response(changedGlb), /content hash/],
    ['board_diff.jpg', new Response('<html>error</html>'), /not a JPEG/],
    ['board_diff.jpg', new Response('Unavailable', { status: 503 }), /503/],
  ]) {
    const badPath = manifest.wood[name];
    const load = createBoardThemeLoader({ manifest, cacheStorage: null, fetcher: async url =>
      url === badPath ? response.clone() : new Response(bodies.get(url)) });
    await assert.rejects(load('wood'), error);
  }
});

test('unknown themes and missing or unexpected asset paths reject before any request', async () => {
  const { manifest } = fixture();
  let requests = 0;
  const load = createBoardThemeLoader({ manifest, cacheStorage: null, fetcher: async () => { requests++; return new Response('unexpected'); } });
  await assert.rejects(load('egyptian'), /not available/);
  delete manifest.wood['king.glb'];
  await assert.rejects(load('wood'), /invalid king.glb artwork/);
  manifest.marble['board.glb'] = 'https://unrelated.invalid/board.glb';
  await assert.rejects(load('marble'), /invalid board.glb artwork/);
  assert.equal(requests, 0);
});

test('corrupt runtime and HTTP-cache entries do not poison explicit retries', async () => {
  const { bodies, manifest } = fixture(), cacheStorage = memoryCaches(), httpCache = new Map();
  const badPath = manifest.wood['board.glb'];
  cacheStorage.records.set(badPath, new Response('<html>stale</html>'));
  let attempts = 0;
  const load = createBoardThemeLoader({ manifest, cacheStorage, fetcher: async (url, options) => {
    if (url === badPath && ++attempts === 1) {
      const bad = new Response('<html>temporarily unavailable</html>');
      httpCache.set(url, bad.clone());
      // A service worker can also cache the unvalidated network response.
      cacheStorage.records.set(url, bad.clone());
      return bad;
    }
    if (httpCache.has(url) && options.cache !== 'reload') return httpCache.get(url).clone();
    return new Response(bodies.get(url));
  } });
  await assert.rejects(load('wood'), /not a GLB/);
  assert.equal(cacheStorage.records.has(badPath), false, 'invalid network bytes are removed from the shared runtime cache');
  assert.deepEqual((await load('wood'))[0], bodies.get(badPath));
  assert.equal(attempts, 2);
});

test('unavailable or full persistent storage does not block usable online 3D packs', async () => {
  const { bodies, manifest } = fixture();
  for (const cacheStorage of [
    { async open() { throw new Error('Storage unavailable'); } },
    { async open() { return { async match() { throw new Error('Read denied'); }, async put() { throw new Error('Quota exceeded'); } }; } },
  ]) {
    const load = createBoardThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) });
    assert.equal((await load('wood')).length, 14);
  }
});
