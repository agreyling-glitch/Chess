import { boardThemes } from './board-themes/manifest.js';

// This order is the binary contract with the Rust 3D theme loader.
const modelNames = ['board.glb', 'pawn.glb', 'rook.glb', 'knight.glb', 'bishop.glb', 'queen.glb', 'king.glb'];
export const boardAssetNames = Object.freeze({
  wood: Object.freeze([...modelNames,
    'pieces_white_diff.jpg', 'pieces_black_diff.jpg', 'board_diff.jpg', 'board_nor.jpg',
    'board_rough.jpg', 'pieces_white_nor.jpg', 'pieces_black_nor.jpg',
  ]),
  marble: Object.freeze([...modelNames,
    'chess_set_pieces_white_diff_1k.jpg', 'chess_set_pieces_black_diff_1k.jpg',
    'chess_set_board_diff_1k.jpg', 'chess_set_board_nor_gl_1k.jpg', 'chess_set_board_arm_1k.jpg',
    'chess_set_pieces_white_nor_gl_1k.jpg', 'chess_set_pieces_black_nor_gl_1k.jpg',
  ]),
});
const CACHE_NAME = 'ironwood-board-themes-v1';
const labels = { wood: 'Wood', marble: 'Marble and Glass' };

export function createBoardThemeLoader({
  manifest = boardThemes, fetcher = fetch, cacheStorage = globalThis.caches, digest = bytes => crypto.subtle.digest('SHA-256', bytes),
} = {}) {
  const loaded = new Map();
  const verifiedBytes = async (response, expectedHash, name) => {
    if (!response.ok) throw new Error(`3D artwork request failed (${response.status})`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (name.endsWith('.glb')) {
      if (bytes.length < 20 || bytes[0] !== 103 || bytes[1] !== 108 || bytes[2] !== 84 || bytes[3] !== 70)
        throw new Error(`${name} is not a GLB model`);
      const header = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
      if (header.getUint32(4, true) !== 2 || header.getUint32(8, true) !== bytes.length)
        throw new Error(`${name} has an invalid GLB header`);
    } else if (bytes.length < 4 || bytes[0] !== 255 || bytes[1] !== 216 || bytes[2] !== 255) {
      throw new Error(`${name} is not a JPEG image`);
    }
    const actualHash = Array.from(new Uint8Array(await digest(bytes)), value => value.toString(16).padStart(2, '0')).join('');
    if (actualHash !== expectedHash) throw new Error(`${name} does not match its content hash`);
    return bytes;
  };
  const asset = async ({ url, hash, name }) => {
    let cache;
    try { cache = await cacheStorage?.open(CACHE_NAME); } catch { /* Persistent storage is optional for online use. */ }
    if (cache) {
      let cached;
      try { cached = await cache.match(url); } catch { /* Fall back to the network if cache reads fail. */ }
      if (cached) {
        try { return await verifiedBytes(cached, hash, name); }
        catch { await cache.delete(url).catch(() => {}); }
      }
    }
    // Validated Cache Storage handles reuse. Bypass HTTP-cache errors on retry.
    const response = await fetcher(url, { credentials: 'same-origin', cache: 'reload', signal: AbortSignal.timeout(30000) });
    let bytes;
    try { bytes = await verifiedBytes(response, hash, name); }
    catch (error) {
      // The service worker may have stored this response before validation.
      await cache?.delete(url).catch(() => {});
      throw error;
    }
    if (cache) await cache.put(url, new Response(bytes, {
      headers: { 'Content-Type': name.endsWith('.glb') ? 'model/gltf-binary' : 'image/jpeg' },
    })).catch(() => {});
    return bytes;
  };
  return theme => {
    if (!Object.hasOwn(boardAssetNames, theme) || !Object.hasOwn(manifest, theme))
      return Promise.reject(new Error('This 3D board theme is not available for download.'));
    if (!loaded.has(theme)) {
      const request = Promise.resolve().then(() => {
        const files = boardAssetNames[theme].map(name => {
          const url = manifest[theme][name];
          const match = typeof url === 'string' && url.match(new RegExp(`^/board-themes/${theme}/([a-f0-9]{64})/${name.replace('.', '\\.')}$`));
          if (!match) throw new Error(`Missing or invalid ${name} artwork`);
          return { url, hash: match[1], name };
        });
        // Rust receives the complete pack together, never a partially loaded board.
        return Promise.all(files.map(asset));
      }).catch(error => {
        loaded.delete(theme);
        throw new Error(`Could not load the ${labels[theme]} 3D board. Connect to the internet and retry. ${error.message}`);
      });
      loaded.set(theme, request);
    }
    return loaded.get(theme);
  };
}

export const loadBoardTheme = createBoardThemeLoader();
