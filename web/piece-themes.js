import { pieceThemes } from './piece-themes/manifest.js';

export const pieceNames = Object.freeze(['wK', 'wQ', 'wR', 'wB', 'wN', 'wP', 'bK', 'bQ', 'bR', 'bB', 'bN', 'bP']);
const CACHE_NAME = 'ironwood-piece-themes-v1';
const labels = { 'sculpted-marble': 'Marble', 'hybrid-wood': 'Wood', 'hybrid-glass': 'Glass' };
const pngSignature = [137, 80, 78, 71, 13, 10, 26, 10];

export function createPieceThemeLoader({
  manifest = pieceThemes, fetcher = fetch, cacheStorage = globalThis.caches, digest = bytes => crypto.subtle.digest('SHA-256', bytes),
} = {}) {
  const loaded = new Map();
  const verifiedBytes = async (response, expectedHash) => {
    if (!response.ok) throw new Error(`Artwork request failed (${response.status})`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (bytes.length <= pngSignature.length || !pngSignature.every((value, index) => bytes[index] === value))
      throw new Error('The piece image is not a PNG');
    const actualHash = Array.from(new Uint8Array(await digest(bytes)), value => value.toString(16).padStart(2, '0')).join('');
    if (actualHash !== expectedHash) throw new Error('The piece image does not match its content hash');
    return bytes;
  };
  const piece = async (url, expectedHash) => {
    let cache;
    try { cache = await cacheStorage?.open(CACHE_NAME); } catch { /* Online use still works without persistent caching. */ }
    if (cache) {
      let cached;
      try { cached = await cache.match(url); } catch { /* Try the network if cache reads fail. */ }
      if (cached) {
        try { return await verifiedBytes(cached, expectedHash); }
        catch { await cache.delete(url).catch(() => {}); }
      }
    }
    // Verified Cache Storage entries already cover normal reuse. Bypass the
    // HTTP cache here so a cached error page or corrupt download cannot poison
    // every explicit retry after its runtime-cache entry has been removed.
    const response = await fetcher(url, { credentials: 'same-origin', cache: 'reload', signal: AbortSignal.timeout(30000) });
    let bytes;
    try { bytes = await verifiedBytes(response, expectedHash); }
    catch (error) {
      // The service worker may have cached this response before validation.
      await cache?.delete(url).catch(() => {});
      throw error;
    }
    if (cache) await cache.put(url, new Response(bytes, { headers: { 'Content-Type': 'image/png' } })).catch(() => {});
    return bytes;
  };
  return theme => {
    if (!Object.hasOwn(labels, theme) || !Object.hasOwn(manifest, theme))
      return Promise.reject(new Error('This piece theme is not available for download.'));
    if (!loaded.has(theme)) {
      const request = Promise.resolve().then(() => {
        // Fetch all twelve pieces, including pieces absent from the current
        // position, so promotion, history and board copies work offline later.
        const files = pieceNames.map(name => {
          const url = manifest[theme][name];
          const match = typeof url === 'string' && url.match(new RegExp(`^/piece-themes/${theme}/([a-f0-9]{64})/${name}\\.png$`));
          if (!match) throw new Error(`Missing or invalid ${name} artwork`);
          return { url, hash: match[1] };
        });
        return Promise.all(files.map(file => piece(file.url, file.hash)));
      }).catch(error => {
        loaded.delete(theme);
        throw new Error(`Could not load ${labels[theme]} pieces. Connect to the internet and retry. ${error.message}`);
      });
      loaded.set(theme, request);
    }
    return loaded.get(theme);
  };
}

export const loadPieceTheme = createPieceThemeLoader();
