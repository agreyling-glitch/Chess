const APP_PACKAGE_VERSION = '__APP_PACKAGE_VERSION__';
const SHELL_CACHE = `ironwood-shell-${APP_PACKAGE_VERSION}`;
const ENGINE_CACHE = 'ironwood-engine-stockfish-19';
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
  '/blog/',
  '/blog/stockfish-in-your-browser/',
  '/changelog/',
  '/changelog/2026-09-21-initial-public-build/',
  '/open-source-notices.html',
  '/404.html',
  '/landing.css',
  '/styles.css',
  '/site.css',
  '/pwa.js',
  '/engine-worker.js',
  '/favicon.ico',
  '/favicon.svg',
  '/apple-touch-icon.png',
  '/android-chrome-192x192.png',
  '/android-chrome-512x512.png',
  '/site.webmanifest',
];

self.addEventListener('install', event => {
  event.waitUntil(caches.open(SHELL_CACHE).then(cache => cache.addAll(SHELL_URLS)));
  self.skipWaiting();
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
      .filter(name => name.startsWith('ironwood-shell-') && name !== SHELL_CACHE)
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

  if (url.pathname.startsWith('/pkg/')) {
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
  const cache = await caches.open(ENGINE_CACHE);
  const matches = await Promise.all(ENGINE_URLS.map(url => cache.match(url)));
  return matches.every(Boolean);
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
  await broadcast({ type: 'OFFLINE_ENGINE_READY' });
}

self.addEventListener('message', event => {
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
    event.waitUntil(caches.delete(ENGINE_CACHE).then(() => broadcast({
      type: 'OFFLINE_ENGINE_STATUS',
      cached: false,
    })));
  }
});
