import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { runInNewContext } from 'node:vm';

const source = readFileSync(new URL('../web/piece-themes.js', import.meta.url), 'utf8')
  .replace("import { pieceThemes } from './piece-themes/manifest.js';", 'const pieceThemes = {};');
const { createPieceThemeLoader, pieceNames } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function fixture() {
  const bodies = new Map(), manifest = {};
  for (const [themeIndex, theme] of ['sculpted-marble', 'hybrid-wood', 'hybrid-glass'].entries()) {
    manifest[theme] = {};
    for (const [pieceIndex, piece] of pieceNames.entries()) {
      const bytes = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, themeIndex, pieceIndex]);
      const path = `/piece-themes/${theme}/${sha(bytes)}/${piece}.png`;
      manifest[theme][piece] = path;
      bodies.set(path, bytes);
    }
  }
  return { bodies, manifest };
}
function memoryCaches(origin = 'https://ironwood.test') {
  const stores = new Map();
  const key = value => new URL(typeof value === 'string' ? value : value.url, origin).href;
  return {
    stores,
    async keys() { return [...stores.keys()]; },
    async delete(name) { return stores.delete(name); },
    async has(name) { return stores.has(name); },
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const records = stores.get(name);
      return {
        async match(request, options) {
          if (!options?.ignoreSearch) return records.get(key(request))?.clone();
          const url = new URL(key(request));
          return [...records].find(([stored]) => new URL(stored).pathname === url.pathname)?.[1].clone();
        },
        async put(request, response) { records.set(key(request), response.clone()); },
        async delete(request) { return records.delete(key(request)); },
        async addAll(urls) { for (const url of urls) records.set(key(url), new Response('shell')); },
      };
    },
  };
}

test('a requested theme loads all twelve pieces in stable order, with no default or other-theme downloads', async () => {
  const { bodies, manifest } = fixture();
  const cacheStorage = memoryCaches(), calls = [];
  const load = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async (url, options) => {
    calls.push(url);
    assert.equal(options.credentials, 'same-origin');
    return new Response(bodies.get(url));
  } });
  assert.deepEqual(calls, []);
  const first = load('hybrid-wood');
  assert.equal(load('hybrid-wood'), first, 'concurrent requests share the same complete-theme promise');
  const result = await first;
  assert.equal(result.length, 12);
  assert.deepEqual(result.map(bytes => bytes[9]), Array.from({ length: 12 }, (_, index) => index));
  assert.ok(calls.every(url => url.startsWith('/piece-themes/hybrid-wood/')));
  assert.equal(calls.length, 12);
  assert.equal(await load('hybrid-wood'), result);
  assert.equal(calls.length, 12);
});

test('visited themes work offline in a fresh loader, while unvisited themes fail explicitly', async () => {
  const { bodies, manifest } = fixture();
  const cacheStorage = memoryCaches();
  await createPieceThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) })('sculpted-marble');
  let offlineRequests = 0;
  const offline = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async () => {
    offlineRequests++; throw new TypeError('offline');
  } });
  assert.equal((await offline('sculpted-marble')).length, 12);
  assert.equal(offlineRequests, 0, 'cached artwork must not revalidate or redownload');
  await assert.rejects(offline('hybrid-glass'), /Could not load Glass pieces.*Connect to the internet and retry/);
  assert.ok(offlineRequests > 0);
});

test('changed content hashes cannot reuse stale bytes; failed packs retry after reconnecting', async () => {
  const { bodies, manifest } = fixture();
  const cacheStorage = memoryCaches();
  await createPieceThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) })('hybrid-wood');
  const changed = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 99]);
  const path = `/piece-themes/hybrid-wood/${sha(changed)}/wK.png`;
  manifest['hybrid-wood'].wK = path;
  let online = false, calls = 0;
  const load = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async url => {
    calls++; assert.equal(url, path);
    if (!online) throw new TypeError('offline');
    return new Response(changed);
  } });
  await assert.rejects(load('hybrid-wood'), /Could not load Wood pieces/);
  online = true;
  assert.deepEqual((await load('hybrid-wood'))[0], changed);
  assert.equal(calls, 2);
});

test('invalid PNGs, hash mismatches, unknown themes and unexpected asset URLs are rejected', async () => {
  const { bodies, manifest } = fixture();
  for (const payload of [new TextEncoder().encode('<!doctype html>'), new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 255])]) {
    await assert.rejects(createPieceThemeLoader({ manifest, cacheStorage: undefined, fetcher: async () => new Response(payload) })('hybrid-glass'), /not a PNG|content hash/);
  }
  let requests = 0;
  const load = createPieceThemeLoader({ manifest, cacheStorage: undefined, fetcher: async url => { requests++; return new Response(bodies.get(url)); } });
  await assert.rejects(load('hybrid-egyptian'), /not available/);
  manifest['sculpted-marble'].wK = 'https://unrelated.invalid/piece.png';
  await assert.rejects(load('sculpted-marble'), /invalid wK artwork/);
  assert.equal(requests, 0);
});

test('corrupt runtime entries are replaced only after PNG and content-hash validation succeeds', async () => {
  const { bodies, manifest } = fixture();
  const cacheStorage = memoryCaches(), cache = await cacheStorage.open('ironwood-piece-themes-v1');
  await cache.put(manifest['hybrid-glass'].wK, new Response('<html>bad</html>'));
  const load = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async url => new Response(bodies.get(url)) });
  const pieces = await load('hybrid-glass');
  assert.deepEqual(new Uint8Array(await (await cache.match(manifest['hybrid-glass'].wK)).arrayBuffer()), pieces[0]);
});

test('retry bypasses an invalid response left in the browser HTTP cache', async () => {
  const { bodies, manifest } = fixture();
  const cacheStorage = memoryCaches(), httpCache = new Map();
  const badPath = manifest['hybrid-glass'].wK;
  let badPathRequests = 0;
  const load = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async (url, options) => {
    if (url === badPath) badPathRequests++;
    // The first server response is a temporary HTML error, which the browser
    // may retain even though PNG validation rejects it in the application.
    if (url === badPath && badPathRequests === 1) {
      httpCache.set(url, new Response('<html>temporarily unavailable</html>'));
      return httpCache.get(url).clone();
    }
    if (httpCache.has(url) && options.cache !== 'reload') return httpCache.get(url).clone();
    const response = new Response(bodies.get(url));
    httpCache.set(url, response.clone());
    return response;
  } });
  await assert.rejects(load('hybrid-glass'), /not a PNG/);
  const pieces = await load('hybrid-glass');
  assert.deepEqual(pieces[0], bodies.get(badPath));
  assert.equal(badPathRequests, 2);
  assert.deepEqual(new Uint8Array(await httpCache.get(badPath).arrayBuffer()), bodies.get(badPath));
});

test('unavailable or full persistent storage does not prevent a complete online theme load', async () => {
  const { bodies, manifest } = fixture();
  for (const cacheStorage of [
    { async open() { throw new Error('Storage unavailable'); } },
    { async open() { return { async match() { throw new Error('Read denied'); }, async put() { throw new Error('Quota exceeded'); } }; } },
  ]) {
    let requests = 0;
    const load = createPieceThemeLoader({ manifest, cacheStorage, fetcher: async url => {
      requests++;
      return new Response(bodies.get(url));
    } });
    const pieces = await load('sculpted-marble');
    assert.equal(pieces.length, 12);
    assert.equal(requests, 12);
    assert.equal(await load('sculpted-marble'), pieces, 'the in-memory theme remains usable without persistent storage');
    assert.equal(requests, 12);
  }
});

function worker(cacheStorage, fetcher) {
  const listeners = new Map();
  const context = {
    self: { location: new URL('https://ironwood.test/service-worker.js'), addEventListener: (type, handler) => listeners.set(type, handler),
      clients: { claim: async () => {}, matchAll: async () => [] }, skipWaiting: async () => {} },
    caches: cacheStorage, fetch: fetcher, URL, Response, Request,
  };
  runInNewContext(readFileSync(new URL('../web/service-worker.js', import.meta.url), 'utf8'), context);
  return {
    async lifecycle(type) {
      let pending;
      listeners.get(type)({ waitUntil: promise => { pending = promise; } });
      await pending;
    },
    async request(path, { navigation = false } = {}) {
      let response;
      const pending = [];
      const request = new Request(new URL(path, context.self.location.origin));
      if (navigation) Object.defineProperty(request, 'mode', { value: 'navigate' });
      listeners.get('fetch')({ request,
        respondWith: promise => { response = promise; }, waitUntil: promise => pending.push(promise) });
      const result = await response;
      await Promise.all(pending);
      return result;
    },
  };
}

test('service worker skips standalone 3D installation downloads and caches visited files for offline use', async () => {
  const cacheStorage = memoryCaches();
  const requests = [];
  let offline = false;
  const sw = worker(cacheStorage, async request => {
    const path = new URL(typeof request === 'string' ? request : request.url, 'https://ironwood.test').pathname;
    requests.push(path);
    if (offline) throw new TypeError('Offline');
    return new Response(`Downloaded ${path}`);
  });
  await sw.lifecycle('install');
  await sw.lifecycle('activate');
  const shell = cacheStorage.stores.get('ironwood-shell-__APP_SHELL_VERSION__');
  assert.ok([...shell.keys()].some(url => url.endsWith('/play/')), 'the game remains in the offline shell');
  assert.ok([...shell.keys()].some(url => url.endsWith('/pkg/battle_chess_bg.wasm')), 'activation still caches the app');
  assert.ok(![...shell.keys()].some(url => new URL(url).pathname.startsWith('/3d/')), 'installation must not cache viewer files');
  assert.ok(!requests.some(path => path.startsWith('/3d/')), 'installation and activation must not fetch the viewer');

  const visited = ['/3d/', '/3d/main.js', '/3d/style.css', '/3d/models/king.glb', '/3d/models/chess_set_pieces_white_diff_1k.jpg'];
  for (const path of visited) {
    assert.equal(await (await sw.request(path, { navigation: path === '/3d/' })).text(), `Downloaded ${path}`);
    assert.ok(shell.has(`https://ironwood.test${path}`), 'online visits cache each response');
  }
  assert.ok(!shell.has('https://ironwood.test/3d/models/queen.glb'), 'visiting one asset does not prefetch others');

  offline = true;
  for (const path of visited) {
    assert.equal(await (await sw.request(path, { navigation: path === '/3d/' })).text(), `Downloaded ${path}`);
  }
  assert.equal((await sw.request('/3d/models/queen.glb')).type, 'error', 'unvisited resources still require an online visit');
});

test('service worker loads the optional background only on use and keeps it available offline afterward', async () => {
  const cacheStorage = memoryCaches(), requests = [];
  const path = '/brand/ironwood-background-no-ring.png';
  let offline = false;
  const sw = worker(cacheStorage, async request => {
    const url = new URL(typeof request === 'string' ? request : request.url, 'https://ironwood.test');
    requests.push(url.pathname);
    if (offline) throw new TypeError('Offline');
    return new Response(`Downloaded ${url.pathname}`);
  });
  await sw.lifecycle('install');
  await sw.lifecycle('activate');
  const shell = cacheStorage.stores.get('ironwood-shell-__APP_SHELL_VERSION__');
  assert.ok(shell.has('https://ironwood.test/background-image.js'), 'background controls remain available offline');
  assert.ok(!shell.has(`https://ironwood.test${path}`), 'installation must not cache the optional image');
  assert.ok(!requests.includes(path), 'installation and activation must not fetch the optional image');

  offline = true;
  assert.equal((await sw.request(path)).type, 'error', 'first use requires an online image download');
  assert.ok(!shell.has(`https://ironwood.test${path}`), 'an unsuccessful request must not be cached');
  offline = false;
  assert.equal(await (await sw.request(path)).text(), `Downloaded ${path}`);
  assert.ok(shell.has(`https://ironwood.test${path}`), 'selecting the background online caches it');
  offline = true;
  assert.equal(await (await sw.request(path)).text(), `Downloaded ${path}`, 'the selected image remains usable offline');
});

test('service worker keeps exact hashed artwork across shell updates without background refetch or precaching PNGs', async () => {
  const cacheStorage = memoryCaches(), { manifest } = fixture();
  const path = manifest['hybrid-wood'].wK;
  const cache = await cacheStorage.open('ironwood-piece-themes-v1');
  await cache.put(path, new Response('visited artwork'));
  await cacheStorage.open('ironwood-shell-obsolete');
  const requests = [];
  const sw = worker(cacheStorage, async request => {
    const url = typeof request === 'string' ? request : request.url;
    requests.push(url);
    return new Response('network');
  });
  assert.equal(await (await sw.request(path)).text(), 'visited artwork');
  assert.deepEqual(requests, []);
  assert.equal(await (await sw.request(path + '?different=1')).text(), 'network');
  assert.equal(requests.length, 1, 'query variants must not use ignoreSearch artwork matching');
  await sw.lifecycle('install');
  const shell = cacheStorage.stores.get('ironwood-shell-__APP_SHELL_VERSION__');
  assert.ok([...shell.keys()].some(url => url.endsWith('/piece-themes.js')));
  assert.ok([...shell.keys()].some(url => url.endsWith('/piece-themes/manifest.js')));
  assert.ok(![...shell.keys()].some(url => /\/piece-themes\/.*\.png/.test(url)), 'PNG packs are never install-time precaches');
  await sw.lifecycle('activate');
  assert.equal(await cacheStorage.has('ironwood-shell-obsolete'), false);
  assert.equal(await cacheStorage.has('ironwood-piece-themes-v1'), true);
  assert.equal(await (await cache.match(path)).text(), 'visited artwork');
});

test('service worker keeps exact 3D theme assets across updates without preloading models or textures', async () => {
  const cacheStorage = memoryCaches(), cache = await cacheStorage.open('ironwood-board-themes-v1');
  const models = ['board.glb', 'pawn.glb', 'rook.glb', 'knight.glb', 'bishop.glb', 'queen.glb', 'king.glb'];
  const files = {
    wood: [...models, 'pieces_white_diff.jpg', 'pieces_black_diff.jpg', 'board_diff.jpg', 'board_nor.jpg',
      'board_rough.jpg', 'pieces_white_nor.jpg', 'pieces_black_nor.jpg'],
    marble: [...models, 'chess_set_pieces_white_diff_1k.jpg', 'chess_set_pieces_black_diff_1k.jpg',
      'chess_set_board_diff_1k.jpg', 'chess_set_board_nor_gl_1k.jpg', 'chess_set_board_arm_1k.jpg',
      'chess_set_pieces_white_nor_gl_1k.jpg', 'chess_set_pieces_black_nor_gl_1k.jpg'],
  };
  const paths = Object.entries(files).flatMap(([theme, names]) => names.map(name => `/board-themes/${theme}/${'a'.repeat(64)}/${name}`));
  for (const path of paths) await cache.put(path, new Response('visited 3D artwork'));
  await cacheStorage.open('ironwood-shell-obsolete');
  const requests = [];
  const sw = worker(cacheStorage, async request => {
    requests.push(typeof request === 'string' ? request : request.url);
    return new Response('network');
  });
  for (const path of paths) assert.equal(await (await sw.request(path)).text(), 'visited 3D artwork');
  assert.deepEqual(requests, [], 'all 28 supported files use the exact cache without background refresh');
  assert.equal(await (await sw.request(paths[0] + '?different=1')).text(), 'network');
  assert.equal(requests.length, 1);
  await sw.lifecycle('install');
  const shell = cacheStorage.stores.get('ironwood-shell-__APP_SHELL_VERSION__');
  assert.ok([...shell.keys()].some(url => url.endsWith('/board-themes.js')));
  assert.ok([...shell.keys()].some(url => url.endsWith('/board-themes/manifest.js')));
  assert.ok(![...shell.keys()].some(url => /\/board-themes\/.*\.(?:glb|jpg)$/.test(url)));
  await sw.lifecycle('activate');
  assert.equal(await cacheStorage.has('ironwood-shell-obsolete'), false);
  assert.equal(await cacheStorage.has('ironwood-board-themes-v1'), true);
  for (const path of paths) assert.equal(await (await cache.match(path)).text(), 'visited 3D artwork');
});
