const APP_SHELL_VERSION = '__APP_SHELL_VERSION__';
const ENGINE_CACHE_VERSION = '__ENGINE_CACHE_VERSION__';
const SHELL_CACHE = `ironwood-shell-${APP_SHELL_VERSION}`;
const ENGINE_CACHE_PREFIX = 'ironwood-engine-stockfish-19-';
const ENGINE_CACHE = `${ENGINE_CACHE_PREFIX}${ENGINE_CACHE_VERSION}`;
const LEGACY_ENGINE_CACHE = 'ironwood-engine-stockfish-19';
const LOCAL_DEVELOPMENT = ['127.0.0.1', 'localhost'].includes(self.location.hostname);
const APP_PACKAGE_URLS = [
  '/pkg/battle_chess.js',
  '/pkg/battle_chess_bg.wasm',
];
const ENGINE_URLS = [
  '/engine/stockfish-19.js',
  '/engine/stockfish-19.wasm',
];
const SHELL_URLS = [
  '/',
  '/play/',
  '/3d/',
  '/3d/main.js',
  '/3d/style.css',
  '/3d/licenses/three-mit.txt',
  '/3d/models/board.glb',
  '/3d/models/chess_set_board_diff_1k.jpg',
  '/3d/models/chess_set_board_nor_gl_1k.jpg',
  '/3d/models/chess_set_board_arm_1k.jpg',
  '/3d/models/pawn.glb',
  '/3d/models/rook.glb',
  '/3d/models/knight.glb',
  '/3d/models/bishop.glb',
  '/3d/models/queen.glb',
  '/3d/models/king.glb',
  '/fics/',
  '/fics/bridge.js',
  '/fics/protocol.js',
  '/features/',
  '/blog/',
  '/blog/stockfish-in-your-browser/',
  '/blog/private-analysis-portable-results/',
  '/blog/local-first-library-and-safe-updates/',
  '/changelog/',
  '/changelog/2026-09-21-initial-public-build/',
  '/changelog/2026-09-22-analysis-workspace/',
  '/changelog/2026-09-24-saved-games-and-storage/',
  '/open-source-notices.html',
  '/404.html',
  '/landing.css',
  '/styles.css',
  '/site.css',
  '/pwa.js',
  '/game-storage.js',
  '/scoresheet-import.js',
  '/scoresheet-pgn.js',
  '/scoresheet-draft.js',
  '/scoresheet-import.css',
  '/position-editor/editor.js',
  '/position-editor/model.js',
  '/position-editor/editor.css',
  '/engine-worker.js',
  '/board-clipboard.js',
  '/favicon.ico',
  '/favicon-16x16.png',
  '/favicon-32x32.png',
  '/favicon-48x48.png',
  '/brand/ironwood-logo-192.webp',
  '/brand/ironwood-logo-256.webp',
  '/brand/ironwood-logo-384.webp',
  '/brand/ironwood-logo-768.webp',
  '/android-chrome-maskable-512x512.png',
  '/favicon.svg',
  '/apple-touch-icon.png',
  '/android-chrome-192x192.png',
  '/android-chrome-512x512.png',
  '/site.webmanifest',
];

self.addEventListener('install', event => {
  event.waitUntil((async () => {
    const existingCaches = await caches.keys();
    const cache = await caches.open(SHELL_CACHE);
    await cache.addAll(SHELL_URLS);
    // The original Ironwood worker used a 16-character package-only cache key
    // and immediately replaced itself. Preserve that behavior once so existing
    // installations can move to the user-confirmed update flow without Ctrl+F5.
    if (LOCAL_DEVELOPMENT || existingCaches.some(name => /^ironwood-shell-[a-f0-9]{16}$/.test(name))) {
      await self.skipWaiting();
    }
  })());
});

self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    const responses = await Promise.all(APP_PACKAGE_URLS.map(async url => {
      const response = await fetch(url, { cache: 'reload' });
      if (!response.ok) throw new Error(`Unable to refresh ${url}`);
      return response;
    }));
    const cache = await caches.open(SHELL_CACHE);
    await Promise.all(APP_PACKAGE_URLS.map((url, index) => cache.put(url, responses[index])));

    const names = await caches.keys();
    await Promise.all(names
      .filter(name => (name.startsWith('ironwood-shell-') && name !== SHELL_CACHE) ||
        name.startsWith('ironwood-engine-lc0-'))
      .map(name => caches.delete(name)));
    await self.clients.claim();
  })());
});

async function networkFirst(request) {
  const cache = await caches.open(SHELL_CACHE);
  try {
    const response = await fetch(request);
    if (response.ok) await cache.put(request, response.clone());
    return response;
  } catch {
    return (await cache.match(request, { ignoreSearch: true })) ||
      (await cache.match(new URL(request.url).pathname)) ||
      (await cache.match('/404.html'));
  }
}

async function cachedAsset(request, event) {
  const shell = await caches.open(SHELL_CACHE);
  const cached = await shell.match(request, { ignoreSearch: true });
  const refresh = fetch(request).then(async response => {
    if (response.ok) await shell.put(request, response.clone());
    return response;
  }).catch(() => null);
  event.waitUntil(refresh.then(() => undefined));
  return cached || await refresh || Response.error();
}

self.addEventListener('fetch', event => {
  const { request } = event;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  if (url.pathname === '/service-worker.js') return;

  if (url.pathname === '/site.webmanifest') {
    event.respondWith(networkFirst(request));
    return;
  }

  if (url.pathname === '/app-version.json') {
    event.respondWith(fetch(request, { cache: 'no-store' }));
    return;
  }

  // The saved-games dialog is a separate module from the WASM app. Serving its
  // stale copy first made local updates appear only after a second reload.
  if (url.pathname === '/game-storage.js' || url.pathname === '/styles.css' ||
      url.pathname === '/site.css' || url.pathname === '/help-search.js' ||
      LOCAL_DEVELOPMENT && url.pathname === '/pwa.js') {
    event.respondWith(networkFirst(request));
    return;
  }

  if (url.pathname.startsWith('/position-editor/')) {
    event.respondWith(networkFirst(request));
    return;
  }

  if (url.pathname.startsWith('/pkg/')) {
    if (LOCAL_DEVELOPMENT) {
      event.respondWith(networkFirst(request));
      return;
    }
    event.respondWith(caches.open(SHELL_CACHE).then(async cache =>
      (await cache.match(request, { ignoreSearch: true })) || fetch(request)
    ));
    return;
  }

  // Emscripten creates pthread Workers by loading this same script again with
  // a URL fragment. Let the browser's immutable HTTP cache handle that script;
  // replaying it from Cache Storage breaks the nested Worker bootstrap.
  if (url.pathname === '/engine/stockfish-19.js') return;

  if (url.pathname.startsWith('/engine/')) {
    event.respondWith(caches.open(ENGINE_CACHE).then(async cache =>
      (await cache.match(request, { ignoreSearch: true })) || fetch(request)
    ));
    return;
  }
  event.respondWith(request.mode === 'navigate' ? networkFirst(request) : cachedAsset(request, event));
});

async function broadcast(message) {
  const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
  for (const client of clients) client.postMessage(message);
}

async function engineIsCached() {
  await migrateLegacyEngineCache();
  const cache = await caches.open(ENGINE_CACHE);
  const matches = await Promise.all(ENGINE_URLS.map(url => cache.match(url)));
  return matches.every(Boolean);
}

async function migrateLegacyEngineCache() {
  if (!(await caches.has(LEGACY_ENGINE_CACHE)) || await caches.has(ENGINE_CACHE)) return;
  const legacy = await caches.open(LEGACY_ENGINE_CACHE);
  const matches = await Promise.all(ENGINE_URLS.map(url => legacy.match(url)));
  if (!matches.every(Boolean)) return;
  const current = await caches.open(ENGINE_CACHE);
  await Promise.all(ENGINE_URLS.map((url, index) => current.put(url, matches[index])));
  await caches.delete(LEGACY_ENGINE_CACHE);
}

async function cacheEngine() {
  const cache = await caches.open(ENGINE_CACHE);
  const totals = [];
  const loaded = [];

  for (let index = 0; index < ENGINE_URLS.length; index++) {
    const url = ENGINE_URLS[index];
    const response = await fetch(url, { cache: 'force-cache' });
    if (!response.ok || !response.body) throw new Error(`Unable to download ${url}`);
    totals[index] = Number(response.headers.get('Content-Length')) || 0;
    loaded[index] = 0;

    const cacheWrite = cache.put(url, response.clone());
    const reader = response.body.getReader();
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      loaded[index] += value.byteLength;
      await broadcast({
        type: 'OFFLINE_ENGINE_PROGRESS',
        loaded: loaded.reduce((sum, value) => sum + value, 0),
        total: totals.reduce((sum, value) => sum + value, 0),
      });
    }
    await cacheWrite;
  }
  const names = await caches.keys();
  await Promise.all(names
    .filter(name => (name === LEGACY_ENGINE_CACHE || name.startsWith(ENGINE_CACHE_PREFIX)) && name !== ENGINE_CACHE)
    .map(name => caches.delete(name)));
  await broadcast({ type: 'OFFLINE_ENGINE_READY' });
}

self.addEventListener('message', event => {
  if (event.data?.type === 'GET_APP_VERSION') {
    event.ports[0]?.postMessage({
      type: 'APP_VERSION',
      version: APP_SHELL_VERSION,
      engine_cache_version: ENGINE_CACHE_VERSION,
    });
  }
  if (event.data?.type === 'ACTIVATE_UPDATE') {
    event.waitUntil(self.skipWaiting());
  }
  if (event.data?.type === 'CHECK_OFFLINE_ENGINE') {
    event.waitUntil(engineIsCached().then(cached => event.source?.postMessage({
      type: 'OFFLINE_ENGINE_STATUS',
      cached,
    })));
  }
  if (event.data?.type === 'CACHE_OFFLINE_ENGINE') {
    event.waitUntil(cacheEngine().catch(async error => {
      await caches.delete(ENGINE_CACHE);
      await broadcast({ type: 'OFFLINE_ENGINE_ERROR', message: error.message });
    }));
  }
  if (event.data?.type === 'REMOVE_OFFLINE_ENGINE') {
    event.waitUntil((async () => {
      const names = await caches.keys();
      await Promise.all(names
        .filter(name => name === LEGACY_ENGINE_CACHE || name.startsWith(ENGINE_CACHE_PREFIX))
        .map(name => caches.delete(name)));
      await broadcast({ type: 'OFFLINE_ENGINE_STATUS', cached: false });
    })());
  }
});
