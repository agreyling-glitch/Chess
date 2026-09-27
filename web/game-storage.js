const DB_NAME = 'ironwood-chess';
const STORE_NAME = 'games';
const ACTIVE_ID_KEY = 'ironwood.chess.active-game-id.v1';
const ACTIVE_CATEGORY_KEY = 'ironwood.chess.active-game-category.v1';
const CURRENT_GAME_KEY = 'ironwood.chess.game.v1';
const PREFERENCES_KEY = 'ironwood.chess.preferences.v1';
const BACKUP_FORMAT = 'ironwood-backup';
const BACKUP_VERSION = 1;

const observationSession = newId();
let databasePromise;
let pendingWrite = Promise.resolve();
let importedFingerprints = new Set();
let fingerprintIndexReady = false;
let fingerprintIndexPromise;
let batchImport = null;
let storageStatus = 'Checking browser storage…';

async function refreshStorageStatus() {
  if (!navigator.storage?.estimate) {
    storageStatus = 'Browser storage estimate unavailable';
    return;
  }
  try {
    const { usage, quota } = await navigator.storage.estimate();
    if (typeof usage !== 'number' || typeof quota !== 'number') throw new Error('Incomplete estimate');
    const format = bytes => `${(bytes / 1048576).toFixed(1)} MB`;
    storageStatus = `Browser storage estimate: ${format(usage)} used of ${format(quota)} quota`;
  } catch {
    storageStatus = 'Browser storage estimate unavailable';
  }
}

window.ironwoodStorageStatus = () => storageStatus;
window.ironwoodRefreshStorageStatus = () => { void refreshStorageStatus(); };
window.ironwoodBeginBatchImport = () => {
  batchImport = { pending: 0, saved: 0, failed: 0, firstError: null };
};
window.ironwoodBatchImportStatus = () => JSON.stringify(batchImport || { pending: 0, saved: 0, failed: 0, firstError: null });
window.ironwoodEndBatchImport = () => {
  batchImport = null;
  void refreshStorageStatus();
};
window.ironwoodImportedIndexReady = () => fingerprintIndexReady;
window.ironwoodEnsureImportedIndex = () => { void loadFingerprintIndex(); };

export function importFingerprint(pgn) {
  if (!pgn) return null;
  const header = name => (pgnTag(pgn, name) || '').trim().toLowerCase();
  const body = pgn.split(/\r?\n/).filter(line => !line.trimStart().startsWith('[')).join('\n');
  let clean = '';
  let braces = 0;
  let variations = 0;
  let lineComment = false;
  for (const character of body) {
    if (lineComment) {
      if (character === '\n') { lineComment = false; clean += ' '; }
      continue;
    }
    if (character === ';' && !braces && !variations) { lineComment = true; continue; }
    if (character === '{') { braces++; continue; }
    if (character === '}' && braces) { braces--; continue; }
    if (braces) continue;
    if (character === '(') { variations++; continue; }
    if (character === ')' && variations) { variations--; continue; }
    if (!variations) clean += character;
  }
  const moves = clean.split(/\s+/).map(token => token.replace(/^\d+\.(?:\.\.)?/, '').replace(/[!?]+$/g, ''))
    .filter(token => token && token !== '...' && !token.startsWith('$') &&
      !['1-0', '0-1', '1/2-1/2', '*'].includes(token));
  if (!moves.length) return null;
  const identity = [header('White'), header('Black'), header('Date'), header('FEN'), ...moves].join('\x1f');
  let hash = 0xcbf29ce484222325n;
  for (let index = 0; index < identity.length; index++) {
    hash = BigInt.asUintN(64, (hash ^ BigInt(identity.charCodeAt(index))) * 0x100000001b3n);
  }
  return hash.toString(16).padStart(16, '0');
}

function storedFingerprint(game) {
  if (game.fingerprint) return game.fingerprint;
  try { return importFingerprint(JSON.parse(game.json).review_pgn); }
  catch { return null; }
}

window.ironwoodIsImportedDuplicate = pgn => {
  try {
    const fingerprint = importFingerprint(pgn);
    return fingerprint !== null && importedFingerprints.has(fingerprint);
  } catch (error) {
    console.warn('Ironwood could not check this PGN for duplicates:', error);
    return false;
  }
};

function openDatabase() {
  if (!('indexedDB' in window)) return Promise.reject(new Error('IndexedDB is unavailable'));
  if (!databasePromise) {
    databasePromise = new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, 1);
      request.onupgradeneeded = () => {
        if (!request.result.objectStoreNames.contains(STORE_NAME)) {
          request.result.createObjectStore(STORE_NAME, { keyPath: 'id' });
        }
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error || new Error('Could not open saved games'));
    }).catch(error => {
      databasePromise = undefined;
      throw error;
    });
  }
  return databasePromise;
}

async function readGame(id) {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const request = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME).get(id);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error('Could not read saved game'));
  });
}

async function listGames() {
  await pendingWrite;
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const games = [];
    const request = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME).openCursor();
    request.onsuccess = () => {
      const cursor = request.result;
      if (!cursor) {
        resolve(games.sort((a, b) => b.updatedAt - a.updatedAt));
        return;
      }
      const { json, ...summary } = cursor.value;
      const details = gameDetails(json);
      games.push({ ...summary, ...details, category: summary.category || details.category, source: summary.source, favorite: Boolean(summary.favorite), fingerprint: storedFingerprint(cursor.value) });
      cursor.continue();
    };
    request.onerror = () => reject(request.error || new Error('Could not list saved games'));
  });
}

async function readAllGameRecords() {
  await pendingWrite;
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const request = db.transaction(STORE_NAME, 'readonly').objectStore(STORE_NAME).getAll();
    request.onsuccess = () => resolve(request.result || []);
    request.onerror = () => reject(request.error || new Error('Could not read saved games'));
  });
}

function validateBackup(value) {
  if (!value || value.format !== BACKUP_FORMAT || value.version !== BACKUP_VERSION || !Array.isArray(value.games)) {
    throw new Error('This is not a supported Ironwood backup');
  }
  const games = value.games.map((record, index) => {
    if (!record || typeof record.id !== 'string' || typeof record.json !== 'string') {
      throw new Error(`Saved game ${index + 1} is incomplete`);
    }
    gameDetails(record.json);
    return {
      ...record,
      id: record.id,
      category: ['imported', 'observed'].includes(record.category) ? record.category : 'mine',
      favorite: Boolean(record.favorite),
      updatedAt: Number(record.updatedAt) || Date.now(),
    };
  });
  if (value.preferences !== null && value.preferences !== undefined &&
      (typeof value.preferences !== 'object' || Array.isArray(value.preferences))) {
    throw new Error('The backup preferences are invalid');
  }
  return { games, preferences: value.preferences ?? null };
}

export function backupSummary(value) {
  const backup = validateBackup(value);
  return {
    games: backup.games.length,
    mine: backup.games.filter(game => game.category === 'mine').length,
    imported: backup.games.filter(game => game.category === 'imported').length,
    observed: backup.games.filter(game => game.category === 'observed').length,
    favorites: backup.games.filter(game => game.favorite).length,
    hasPreferences: Boolean(backup.preferences),
  };
}

async function replaceOrMergeGames(games, replace) {
  await pendingWrite;
  const db = await openDatabase();
  const existingIds = replace ? new Set() : new Set((await readAllGameRecords()).map(game => game.id));
  await new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    if (replace) store.clear();
    for (const game of games) {
      let id = game.id;
      while (existingIds.has(id)) id = newId();
      existingIds.add(id);
      store.put({ ...game, id });
    }
    transaction.oncomplete = resolve;
    transaction.onerror = () => reject(transaction.error || new Error('Could not restore saved games'));
    transaction.onabort = () => reject(transaction.error || new Error('Restore was interrupted'));
  });
}

async function clearSavedGames() {
  await pendingWrite;
  const db = await openDatabase();
  await new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    transaction.objectStore(STORE_NAME).clear();
    transaction.oncomplete = resolve;
    transaction.onerror = () => reject(transaction.error || new Error('Could not clear saved games'));
    transaction.onabort = () => reject(transaction.error || new Error('Clear was interrupted'));
  });
  localStorage.removeItem(CURRENT_GAME_KEY);
  localStorage.removeItem(ACTIVE_ID_KEY);
  localStorage.removeItem(ACTIVE_CATEGORY_KEY);
  importedFingerprints = new Set();
  fingerprintIndexReady = false;
  fingerprintIndexPromise = undefined;
}

function downloadJson(filename, value) {
  const blob = new Blob([`${JSON.stringify(value, null, 2)}\n`], { type: 'application/json;charset=utf-8' });
  downloadBlob(filename, blob);
}

function downloadBlob(filename, blob) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

function storedZip(files) {
  const encoder = new TextEncoder();
  const chunks = [], directory = [];
  let offset = 0;
  const crc32 = bytes => {
    let crc = 0xffffffff;
    for (const byte of bytes) {
      crc ^= byte;
      for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
    return (crc ^ 0xffffffff) >>> 0;
  };
  for (const file of files) {
    const name = encoder.encode(file.name), data = encoder.encode(file.content);
    const crc = crc32(data);
    const local = new Uint8Array(30 + name.length), l = new DataView(local.buffer);
    l.setUint32(0, 0x04034b50, true); l.setUint16(4, 20, true); l.setUint16(6, 0x800, true);
    l.setUint16(12, 33, true); l.setUint32(14, crc, true);
    l.setUint32(18, data.length, true); l.setUint32(22, data.length, true); l.setUint16(26, name.length, true);
    local.set(name, 30);
    const central = new Uint8Array(46 + name.length), c = new DataView(central.buffer);
    c.setUint32(0, 0x02014b50, true); c.setUint16(4, 20, true); c.setUint16(6, 20, true);
    c.setUint16(8, 0x800, true); c.setUint16(14, 33, true); c.setUint32(16, crc, true);
    c.setUint32(20, data.length, true); c.setUint32(24, data.length, true);
    c.setUint16(28, name.length, true); c.setUint32(42, offset, true); central.set(name, 46);
    chunks.push(local, data); directory.push(central); offset += local.length + data.length;
  }
  const directorySize = directory.reduce((sum, chunk) => sum + chunk.length, 0);
  if (files.length > 65535 || offset + directorySize > 0xffffffff) throw new Error('Selection is too large for one ZIP; export smaller batches');
  const end = new Uint8Array(22), e = new DataView(end.buffer);
  e.setUint32(0, 0x06054b50, true); e.setUint16(8, files.length, true); e.setUint16(10, files.length, true);
  e.setUint32(12, directorySize, true); e.setUint32(16, offset, true);
  const output = new Uint8Array(offset + directorySize + end.length);
  let cursor = 0;
  for (const chunk of [...chunks, ...directory, end]) { output.set(chunk, cursor); cursor += chunk.length; }
  return output;
}

export function buildGameExport(records, format, exporter) {
  if (!records.length) throw new Error('Select at least one game');
  if (typeof exporter !== 'function') throw new Error('The game is still loading. Try again shortly');
  const contents = records.map(record => exporter(record.json, format));
  const base = records.length === 1 ? 'ironwood-game' : 'ironwood-games';
  if (format === 'json' && records.length > 1) {
    return { filename: `${base}-analysis.zip`, type: 'application/zip',
      content: storedZip(contents.map((content, i) => ({ name: `game-${i + 1}.json`, content }))) };
  }
  return { filename: `${base}${format === 'annotated' ? '-annotated' : ''}.${format === 'json' ? 'json' : 'pgn'}`,
    type: format === 'json' ? 'application/json;charset=utf-8' : 'application/x-chess-pgn;charset=utf-8',
    content: contents.map(value => value.trim()).join('\n\n') + '\n' };
}

function storageDialog(titleText) {
  if (!document.getElementById('ironwood-storage-styles')) {
    const style = document.createElement('style');
    style.id = 'ironwood-storage-styles';
    style.textContent = `
      #storage-management-dialog{width:min(560px,calc(100vw - 24px));box-sizing:border-box;padding:20px;border:1px solid #a8874d;border-radius:8px;background:#191d23;color:#ecebe4;box-shadow:0 20px 70px #000a;font:14px/1.45 Inter,system-ui,sans-serif}
      #storage-management-dialog::backdrop{background:#0009}
      #storage-management-dialog .game-library-heading{display:flex;align-items:center;justify-content:space-between;gap:12px}
      #storage-management-dialog .game-library-heading h2{margin:0 0 14px;font-size:21px;font-weight:600}
      #storage-management-dialog .game-library-heading button{border:0;background:transparent;color:#ecebe4;font-size:25px;cursor:pointer}
      #storage-management-dialog .storage-guidance{color:#b9bdb8}
      #storage-management-dialog .storage-actions{display:flex;justify-content:flex-end;gap:9px;margin-top:20px}
      #storage-management-dialog .storage-actions button{padding:8px 12px;border:1px solid #586068;border-radius:4px;color:#ecebe4;background:#293037;cursor:pointer}
      #storage-management-dialog .storage-actions button.danger{border-color:#a85454;color:#ffb2ad}
      #storage-management-dialog .storage-info-grid{display:grid;grid-template-columns:minmax(130px,max-content) 1fr;gap:9px 20px;align-items:baseline;padding:4px 0}
      #storage-management-dialog .storage-info-grid strong{color:#b9bdb8;font-weight:600}
      #storage-management-dialog .storage-info-grid span{min-width:0;overflow-wrap:anywhere}
      @media(max-width:520px){#storage-management-dialog .storage-info-grid{grid-template-columns:1fr;gap:2px}#storage-management-dialog .storage-info-grid span{margin-bottom:8px}}
    `;
    document.head.append(style);
  }
  let dialog = document.getElementById('storage-management-dialog');
  if (!dialog) {
    dialog = document.createElement('dialog');
    dialog.id = 'storage-management-dialog';
    document.body.append(dialog);
  }
  dialog.replaceChildren();
  const heading = document.createElement('div');
  heading.className = 'game-library-heading';
  const title = document.createElement('h2');
  title.textContent = titleText;
  const close = document.createElement('button');
  close.type = 'button';
  close.textContent = '×';
  close.setAttribute('aria-label', `Close ${titleText.toLowerCase()}`);
  close.addEventListener('click', () => dialog.close());
  const windowControls = document.createElement('div');
  windowControls.className = 'game-library-window-controls';
  const maximize = document.createElement('button');
  maximize.type = 'button';
  const updateMaximize = () => {
    const maximized = dialog.classList.contains('maximized');
    maximize.textContent = maximized ? '❐' : '□';
    maximize.title = maximized ? 'Restore saved games window' : 'Maximize saved games window';
    maximize.setAttribute('aria-label', maximize.title);
    maximize.setAttribute('aria-pressed', String(maximized));
  };
  maximize.addEventListener('click', () => {
    dialog.classList.toggle('maximized');
    updateMaximize();
  });
  updateMaximize();
  windowControls.append(maximize, close);
  heading.append(title, windowControls);
  dialog.append(heading);
  return dialog;
}

window.ironwoodBackupStorage = async () => {
  try {
    const games = await readAllGameRecords();
    let preferences = null;
    try { preferences = JSON.parse(localStorage.getItem(PREFERENCES_KEY)); } catch { /* Preserve games even if preferences are damaged. */ }
    const backup = {
      format: BACKUP_FORMAT,
      version: BACKUP_VERSION,
      exported_at: new Date().toISOString(),
      app: { name: 'Ironwood Chess', version: document.querySelector('meta[name="application-version"]')?.content || null },
      games,
      preferences,
    };
    const date = new Date().toISOString().slice(0, 10);
    downloadJson(`ironwood-backup-${date}.json`, backup);
  } catch (error) {
    alert(`Backup failed: ${error.message}`);
  }
};

window.ironwoodRestoreStorage = () => {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json,application/json';
  input.addEventListener('change', async () => {
    const file = input.files?.[0];
    if (!file) return;
    try {
      const raw = JSON.parse(await file.text());
      const backup = validateBackup(raw);
      const summary = backupSummary(raw);
      const dialog = storageDialog('Restore backup');
      const message = document.createElement('p');
      message.textContent = `${file.name} contains ${summary.games} games (${summary.mine} personal, ${summary.imported} imported, ${summary.observed} observed, ${summary.favorites} favorites)${summary.hasPreferences ? ' and display preferences' : ''}.`;
      const guidance = document.createElement('p');
      guidance.className = 'storage-guidance';
      guidance.textContent = 'Merge keeps existing games and adds the backup. Replace removes existing games first. Both options restore included preferences.';
      const actions = document.createElement('div');
      actions.className = 'storage-actions';
      for (const [label, replace, danger] of [['Cancel', null, false], ['Merge', false, false], ['Replace existing', true, true]]) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = label;
        if (danger) button.className = 'danger';
        button.addEventListener('click', async () => {
          if (replace === null) { dialog.close(); return; }
          for (const child of actions.children) child.disabled = true;
          message.textContent = 'Restoring backup…';
          try {
            await replaceOrMergeGames(backup.games, replace);
            if (backup.preferences) localStorage.setItem(PREFERENCES_KEY, JSON.stringify(backup.preferences));
            localStorage.removeItem(CURRENT_GAME_KEY);
            localStorage.removeItem(ACTIVE_ID_KEY);
            localStorage.removeItem(ACTIVE_CATEGORY_KEY);
            location.reload();
          } catch (error) {
            message.textContent = `Restore failed: ${error.message}`;
            for (const child of actions.children) child.disabled = false;
          }
        });
        actions.append(button);
      }
      dialog.append(message, guidance, actions);
      dialog.showModal();
    } catch (error) {
      alert(`This backup could not be restored: ${error.message}`);
    }
  }, { once: true });
  input.click();
};

window.ironwoodOpenStorageInfo = async () => {
  const dialog = storageDialog('Storage information');
  const content = document.createElement('div');
  content.textContent = 'Checking storage…';
  dialog.append(content);
  dialog.showModal();
  try {
    const games = await readAllGameRecords();
    const estimate = await navigator.storage?.estimate?.();
    const persisted = await navigator.storage?.persisted?.();
    const format = value => typeof value === 'number' ? `${(value / 1048576).toFixed(1)} MB` : 'Unavailable';
    const rows = [
      ['Database', `${DB_NAME} · schema 1`],
      ['Saved games', String(games.length)],
      ['My games', String(games.filter(game => game.category === 'mine').length)],
      ['Imported games', String(games.filter(game => game.category === 'imported').length)],
      ['Observed games', String(games.filter(game => game.category === 'observed').length)],
      ['Favorites', String(games.filter(game => game.favorite).length)],
      ['Browser usage', format(estimate?.usage)],
      ['Browser quota', format(estimate?.quota)],
      ['Persistent storage', persisted === true ? 'Granted' : persisted === false ? 'Not granted' : 'Unavailable'],
      ['Offline engine', 'Managed separately by the offline-play control'],
    ];
    content.replaceChildren();
    content.className = 'storage-info-grid';
    for (const [label, value] of rows) {
      const term = document.createElement('strong');
      term.textContent = label;
      const detail = document.createElement('span');
      detail.textContent = value;
      content.append(term, detail);
    }
  } catch (error) {
    content.textContent = `Storage information is unavailable: ${error.message}`;
  }
};

window.ironwoodClearSavedGames = async () => {
  const games = await readAllGameRecords().catch(() => []);
  if (!confirm(`Clear ${games.length} saved games? Preferences and the offline engine will be kept. This cannot be undone.`)) return;
  try {
    await clearSavedGames();
    location.reload();
  } catch (error) {
    alert(`Saved games could not be cleared: ${error.message}`);
  }
};

window.ironwoodResetLocalData = async () => {
  if (!confirm('Reset all Ironwood data on this device? This removes saved games, preferences, and downloaded offline data. This cannot be undone.')) return;
  try {
    await clearSavedGames();
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith('ironwood')) localStorage.removeItem(key);
    }
    if ('caches' in window) await Promise.all((await caches.keys()).filter(name => name.startsWith('ironwood-')).map(name => caches.delete(name)));
    if ('serviceWorker' in navigator) await Promise.all((await navigator.serviceWorker.getRegistrations()).filter(item => new URL(item.scope).origin === location.origin).map(item => item.unregister()));
    location.reload();
  } catch (error) {
    alert(`Local data could not be reset: ${error.message}`);
  }
};

function loadFingerprintIndex() {
  if (!fingerprintIndexPromise) {
    fingerprintIndexPromise = (async () => {
      await pendingWrite;
      const db = await openDatabase();
      const fingerprints = new Set();
      let afterKey;
      while (true) {
        const chunk = await new Promise((resolve, reject) => {
          const transaction = db.transaction(STORE_NAME, 'readonly');
          const range = afterKey === undefined ? undefined : IDBKeyRange.lowerBound(afterKey, true);
          const request = transaction.objectStore(STORE_NAME).openCursor(range);
          let count = 0;
          request.onsuccess = () => {
            const cursor = request.result;
            if (!cursor) { resolve({ done: true }); return; }
            const fingerprint = storedFingerprint(cursor.value);
            if (fingerprint) fingerprints.add(fingerprint);
            afterKey = cursor.key;
            if (++count >= 16) { resolve({ done: false }); return; }
            cursor.continue();
          };
          request.onerror = () => reject(request.error || new Error('Could not index saved games'));
        });
        if (chunk.done) break;
        // Release the main thread between chunks so the board and dialogs stay responsive.
        await new Promise(resolve => setTimeout(resolve, 0));
      }
      for (const fingerprint of importedFingerprints) fingerprints.add(fingerprint);
      importedFingerprints = fingerprints;
      fingerprintIndexReady = true;
    })().catch(error => {
      fingerprintIndexPromise = undefined;
      console.warn('Ironwood could not index imported games for duplicate checking:', error);
    });
  }
  return fingerprintIndexPromise;
}

function putGame(record) {
  pendingWrite = pendingWrite.catch(() => {}).then(async () => {
    const db = await openDatabase();
    await new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, 'readwrite');
      const store = transaction.objectStore(STORE_NAME);
      const request = store.get(record.id);
      request.onsuccess = () => {
        const existing = request.result;
        store.put({ ...existing, ...record, favorite: record.favorite ?? existing?.favorite ?? false });
      };
      request.onerror = () => transaction.abort();
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error || new Error('Could not save game'));
      transaction.onabort = () => reject(transaction.error || new Error('Game save was interrupted'));
    });
  });
  pendingWrite.catch(error => console.warn('Ironwood game library save failed:', error));
  return pendingWrite;
}

async function deleteGames(ids) {
  await pendingWrite;
  const db = await openDatabase();
  await new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    for (const id of ids) store.delete(id);
    transaction.oncomplete = resolve;
    transaction.onerror = () => reject(transaction.error || new Error('Could not delete game'));
    transaction.onabort = () => reject(transaction.error || new Error('Game deletion was interrupted'));
  });
}

function newId() {
  return crypto.randomUUID?.() || `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function activeId() {
  let id = localStorage.getItem(ACTIVE_ID_KEY);
  if (!id) {
    id = newId();
    localStorage.setItem(ACTIVE_ID_KEY, id);
  }
  return id;
}

function pgnTag(pgn, name) {
  return pgn?.match(new RegExp(`^\\[${name} "([^"]+)"\\]`, 'm'))?.[1];
}

function pgnMoveCount(pgn) {
  if (!pgn) return 0;
  let body = pgn.split(/\r?\n/).filter(line => !line.trimStart().startsWith('[')).join('\n');
  body = body.replace(/\{[^}]*\}|;[^\n]*/g, ' ');
  while (/\([^()]*\)/.test(body)) body = body.replace(/\([^()]*\)/g, ' ');
  return body.replace(/\d+\.(?:\.\.)?/g, ' ').split(/\s+/)
    .filter(token => token && token !== '...' && !token.startsWith('$') && !['*', '1-0', '0-1', '1/2-1/2'].includes(token)).length;
}

export function gameDetails(json) {
  const game = JSON.parse(json);
  const imported = Boolean(game.review_pgn);
  // Preserve historical opponent names for games saved before the engine was removed.
  const opponent = game.engine_config?.opponent === 'Lc0' ? 'Lc0 · Good Gyal' : 'Stockfish 19';
  const bothSides = game.engine_enabled === false;
  const white = imported ? pgnTag(game.review_pgn, 'White') || 'Unknown' : bothSides ? 'White' : game.player_side === 'Black' ? opponent : 'You';
  const black = imported ? pgnTag(game.review_pgn, 'Black') || 'Unknown' : bothSides ? 'Black' : game.player_side === 'Black' ? 'You' : opponent;
  const rawDate = pgnTag(game.review_pgn, 'Date');
  const playedAt = rawDate && rawDate !== '????.??.??' ? rawDate : null;
  const rawSite = pgnTag(game.review_pgn, 'Site');
  const venue = rawSite && rawSite !== '-' && rawSite !== '?' ? rawSite : null;
  const recordedResult = pgnTag(game.review_pgn, 'Result') || game.result;
  const result = ['1-0', '0-1', '1/2-1/2', '*'].includes(recordedResult) ? recordedResult : '*';
  const moves = game.live_moves?.length || pgnMoveCount(game.review_pgn) || Math.max(0, (game.game_analysis?.length || 1) - 1);
  const analyzed = game.game_analysis?.filter(Boolean).length || 0;
  const analysisStatus = analyzed === 0 ? 'Not analyzed' : analyzed >= moves + 1 ? 'Complete' : 'Partial';
  return { analysisStatus, title: `${white} vs ${black}`, white, black, playedAt, venue, result, moves, analyzed,
    finalFen: game.final_board || game.board, category: imported ? 'imported' : 'mine' };
}

export function gameMatchesCategory(game, category) {
  return category === 'all' || (category === 'favorites' ? Boolean(game.favorite) : game.category === category);
}

function queueGameRecord(json, id, category, trackBatch = false, source) {
  const batch = trackBatch ? batchImport : null;
  try {
    const details = gameDetails(json);
    const fingerprint = importFingerprint(JSON.parse(json).review_pgn);
    if (fingerprint) importedFingerprints.add(fingerprint);
    if (batch) batch.pending++;
    const write = putGame({ id, ...details, category, fingerprint, ...(source ? { source } : {}), updatedAt: Date.now(), json });
    if (batch) write.then(() => {
      batch.pending--;
      batch.saved++;
    }, error => {
      batch.pending--;
      batch.failed++;
      batch.firstError ||= error?.message || String(error);
      if (fingerprint) importedFingerprints.delete(fingerprint);
    });
  } catch (error) {
    if (batch) {
      batch.failed++;
      batch.firstError ||= error?.message || String(error);
    }
    console.warn('Ironwood game library save failed:', error);
  }
}

window.ironwoodStoreCurrentGame = json => {
  try {
    queueGameRecord(json, activeId(), localStorage.getItem(ACTIVE_CATEGORY_KEY) || gameDetails(json).category);
  } catch (error) {
    console.warn('Ironwood game library save failed:', error);
  }
};

window.ironwoodStoreObservedGame = (json, id) => {
  queueGameRecord(json, `observed-${observationSession}-${id}`, 'observed', false, 'FICS');
};

window.ironwoodStoreImportedGame = json => {
  queueGameRecord(json, newId(), 'imported', true);
};

window.ironwoodStartNewStoredGame = category => {
  try {
    localStorage.setItem(ACTIVE_ID_KEY, newId());
    localStorage.setItem(ACTIVE_CATEGORY_KEY, ['imported', 'observed'].includes(category) ? category : 'mine');
  } catch (error) {
    console.warn('Ironwood could not start a new saved game:', error);
  }
};

export async function initializeGameStorage() {
  void refreshStorageStatus();
  try {
    const current = localStorage.getItem(CURRENT_GAME_KEY);
    if (current) {
      // The old local save is the most recent copy; importing it also migrates existing users.
      window.ironwoodStoreCurrentGame(current);
    } else {
      const id = localStorage.getItem(ACTIVE_ID_KEY);
      const record = id && await readGame(id);
      if (record?.json) {
        localStorage.setItem(CURRENT_GAME_KEY, record.json);
        localStorage.setItem(ACTIVE_CATEGORY_KEY, record.category || gameDetails(record.json).category);
      }
    }
  } catch (error) {
    console.warn('Ironwood saved games are unavailable; local save remains active:', error);
  }
  // The duplicate index is loaded on demand when an import starts. A large
  // library must not compete with the board's first render or normal play.
}

function finalBoard(fen) {
  const board = document.createElement('span');
  board.className = 'game-library-board';
  board.setAttribute('role', 'img');
  board.setAttribute('aria-label', 'Final board position');
  const ranks = fen?.split(' ')[0]?.split('/');
  if (ranks?.length !== 8) {
    board.textContent = 'No preview';
    return board;
  }
  for (const [rankIndex, rank] of ranks.entries()) {
    let file = 0;
    for (const piece of rank) {
      const count = Number(piece);
      if (Number.isInteger(count) && count >= 1 && count <= 8) {
        for (let empty = 0; empty < count; empty++) {
          const square = document.createElement('span');
          square.className = `game-library-square ${(rankIndex + file++) % 2 ? 'dark' : 'light'}`;
          board.append(square);
        }
      } else if (/[prnbqkPRNBQK]/.test(piece)) {
        const square = document.createElement('span');
        square.className = `game-library-square ${(rankIndex + file++) % 2 ? 'dark' : 'light'}`;
        const image = document.createElement('img');
        image.src = `/pieces/merida/${piece === piece.toUpperCase() ? 'w' : 'b'}${piece.toUpperCase()}.svg`;
        image.alt = '';
        square.append(image);
        board.append(square);
      }
    }
    if (file !== 8) {
      board.replaceChildren('No preview');
      break;
    }
  }
  return board;
}

function showGamePreview(game, trigger) {
  const dialog = document.createElement('dialog');
  dialog.id = 'game-preview-dialog';
  dialog.setAttribute('aria-labelledby', 'game-preview-title');
  const heading = document.createElement('div');
  heading.className = 'game-preview-heading';
  const title = document.createElement('h2');
  title.id = 'game-preview-title';
  title.textContent = game.title;
  const close = document.createElement('button');
  close.type = 'button'; close.textContent = '×';
  close.setAttribute('aria-label', 'Close board preview');
  close.addEventListener('click', () => dialog.close());
  heading.append(title, close);
  const caption = document.createElement('p');
  caption.textContent = `Final position · ${game.result || '*'}${game.playedAt ? ` · ${game.playedAt}` : ''}`;
  const board = finalBoard(game.finalFen);
  board.setAttribute('aria-label', `Final position: ${game.title}`);
  dialog.append(heading, caption, board);
  dialog.addEventListener('close', () => {
    dialog.remove();
    if (trigger.isConnected) trigger.focus();
  }, { once: true });
  document.body.append(dialog);
  dialog.showModal();
  close.focus();
}

function confirmDeleteGames(games, active) {
  return new Promise(resolve => {
    const dialog = document.createElement('dialog');
    dialog.id = 'game-delete-dialog';
    const title = document.createElement('h2');
    title.textContent = games.length === 1 ? 'Delete saved game?' : `Delete ${games.length} saved games?`;
    const name = document.createElement('p');
    name.className = 'game-delete-name';
    name.textContent = games.length === 1 ? games[0].title : `${games.length} selected games will be permanently deleted, including their saved analysis.`;
    const warning = document.createElement('p');
    warning.className = 'game-delete-warning';
    warning.textContent = active
      ? 'This cannot be undone. The current board will reset.'
      : 'This cannot be undone.';
    const actions = document.createElement('div');
    actions.className = 'game-delete-actions';
    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.textContent = 'Cancel';
    cancel.addEventListener('click', () => dialog.close('cancel'));
    const remove = document.createElement('button');
    remove.type = 'button';
    remove.className = 'danger';
    remove.textContent = games.length === 1 ? 'Delete game' : `Delete ${games.length} games`;
    remove.addEventListener('click', () => dialog.close('delete'));
    actions.append(cancel, remove);
    dialog.append(title, name, warning, actions);
    document.body.append(dialog);
    dialog.addEventListener('close', () => {
      const confirmed = dialog.returnValue === 'delete';
      dialog.remove();
      resolve(confirmed);
    }, { once: true });
    dialog.showModal();
    cancel.focus();
  });
}

window.ironwoodOpenGameLibrary = async () => {
  let dialog = document.getElementById('game-library-dialog');
  if (!dialog) {
    dialog = document.createElement('dialog');
    dialog.id = 'game-library-dialog';
    document.body.append(dialog);
  }
  dialog.replaceChildren();
  const heading = document.createElement('div');
  heading.className = 'game-library-heading';
  const title = document.createElement('h2');
  title.textContent = 'Saved games';
  const close = document.createElement('button');
  close.type = 'button';
  close.textContent = '×';
  close.setAttribute('aria-label', 'Close saved games');
  close.addEventListener('click', () => dialog.close());
  const windowControls = document.createElement('div');
  windowControls.className = 'game-library-window-controls';
  const maximize = document.createElement('button');
  maximize.type = 'button';
  const updateMaximize = () => {
    const maximized = dialog.classList.contains('maximized');
    maximize.textContent = maximized ? '❐' : '□';
    maximize.title = maximized ? 'Restore saved games window' : 'Maximize saved games window';
    maximize.setAttribute('aria-label', maximize.title);
    maximize.setAttribute('aria-pressed', String(maximized));
  };
  maximize.addEventListener('click', () => {
    dialog.classList.toggle('maximized');
    updateMaximize();
  });
  updateMaximize();
  windowControls.append(maximize, close);
  heading.append(title, windowControls);
  const tabs = document.createElement('div');
  tabs.className = 'game-library-tabs';
  const filters = document.createElement('div');
  filters.className = 'game-library-filters';
  const list = document.createElement('div');
  list.className = 'game-library-list';
  list.textContent = 'Loading games…';
  const pager = document.createElement('div');
  pager.className = 'game-library-pager';
  const selection = document.createElement('div');
  selection.className = 'game-library-selection';
  dialog.append(heading, tabs, filters, selection, list, pager);
  dialog.showModal();
  try {
    const games = await listGames();
    let category = 'all';
    let playerFilter = '*';
    let searchQuery = '';
    let analysisFilter = '*';
    let page = 0;
    const pageSize = 10;
    const selectedIds = new Set();
    let deleting = false;
    const removeGames = async targets => {
      if (deleting || !targets.length) return;
      deleting = true;
      try {
        const active = targets.some(game => localStorage.getItem(ACTIVE_ID_KEY) === game.id);
        if (!await confirmDeleteGames(targets, active)) return;
        await deleteGames(targets.map(game => game.id));
        const removed = new Set(targets.map(game => game.id));
        for (let index = games.length - 1; index >= 0; index--) {
          if (removed.has(games[index].id)) games.splice(index, 1);
        }
        removed.forEach(id => selectedIds.delete(id));
        importedFingerprints = new Set(games.map(storedFingerprint).filter(Boolean));
        if (active) {
          localStorage.removeItem(CURRENT_GAME_KEY);
          localStorage.removeItem(ACTIVE_ID_KEY);
          localStorage.removeItem(ACTIVE_CATEGORY_KEY);
          location.reload();
        } else render();
      } catch (error) {
        const message = document.createElement('p');
        message.setAttribute('role', 'alert');
        message.textContent = `Could not delete games: ${error.message}`;
        selection.append(message);
      } finally { deleting = false; }
    };
    const render = () => {
      tabs.replaceChildren();
      for (const [value, label] of [
        ['all', 'All Games'],
        ['mine', 'My Games'],
        ['observed', 'Observed'],
        ['imported', 'Imported Games'],
        ['favorites', 'Favorites'],
      ]) {
        const tab = document.createElement('button');
        tab.type = 'button';
        tab.textContent = `${label} (${games.filter(game => gameMatchesCategory(game, value)).length})`;
        tab.className = category === value ? 'selected' : '';
        tab.addEventListener('click', () => { category = value; playerFilter = '*'; selectedIds.clear(); page = 0; render(); });
        tabs.append(tab);
      }
      filters.replaceChildren();
      const search = document.createElement('input');
      search.type = 'search';
      search.placeholder = 'Search players, source, or date';
      search.setAttribute('aria-label', 'Search saved games');
      search.value = searchQuery;
      search.addEventListener('input', () => {
        const cursor = search.selectionStart;
        searchQuery = search.value;
        selectedIds.clear();
        page = 0;
        render();
        const replacement = filters.querySelector('input[type=search]');
        replacement.focus();
        replacement.setSelectionRange(cursor, cursor);
      });
      filters.append(search);
      {
        const label = document.createElement('label');
        label.textContent = 'Player ';
        const select = document.createElement('select');
        const categoryGames = games.filter(game => gameMatchesCategory(game, category));
        const names = [...new Set(categoryGames.flatMap(game => [game.white, game.black]).filter(name => name && name !== 'Unknown'))].sort();
        for (const [value, text] of [['*', 'All players'], ...names.map(name => [name, name]), ['?', 'Unknown players']]) {
          const option = document.createElement('option');
          option.value = value;
          option.textContent = text;
          select.append(option);
        }
        select.value = playerFilter;
        select.addEventListener('change', () => { playerFilter = select.value; selectedIds.clear(); page = 0; render(); });
        label.append(select);
        filters.append(label);
      }
      const analysisLabel = document.createElement('label');
      analysisLabel.textContent = 'Analysis ';
      const analysisSelect = document.createElement('select');
      for (const [value, text] of [['*', 'All games'], ['complete', 'Complete'], ['partial', 'Partial'], ['unanalyzed', 'Not analyzed']]) {
        const option = document.createElement('option');
        option.value = value;
        option.textContent = text;
        analysisSelect.append(option);
      }
      analysisSelect.value = analysisFilter;
      analysisSelect.addEventListener('change', () => { analysisFilter = analysisSelect.value; selectedIds.clear(); page = 0; render(); });
      analysisLabel.append(analysisSelect);
      filters.append(analysisLabel);
      list.replaceChildren();
      pager.replaceChildren();
      const visible = games.filter(game => [game.title, game.white, game.black, game.source, game.venue, game.playedAt].join(' ').toLowerCase().includes(searchQuery.trim().toLowerCase()) && gameMatchesCategory(game, category) && (playerFilter === '*' ||
        playerFilter === '?' && [game.white, game.black].includes('Unknown') ||
        [game.white, game.black].includes(playerFilter)) && (analysisFilter === '*' ||
        analysisFilter === 'complete' && game.analysisStatus === 'Complete' ||
        analysisFilter === 'partial' && game.analysisStatus === 'Partial' ||
        analysisFilter === 'unanalyzed' && !(game.analyzed > 0)));
      // Selection stays across pages, but never includes games hidden by filters.
      const matchingIds = new Set(visible.map(game => game.id));
      for (const id of selectedIds) if (!matchingIds.has(id)) selectedIds.delete(id);
      page = Math.max(0, Math.min(page, Math.ceil(visible.length / pageSize) - 1));
      const pageGames = visible.slice(page * pageSize, (page + 1) * pageSize);
      selection.replaceChildren();
      const selectPageLabel = document.createElement('label');
      const selectPage = document.createElement('input');
      selectPage.type = 'checkbox';
      selectPage.checked = pageGames.length > 0 && pageGames.every(game => selectedIds.has(game.id));
      selectPage.indeterminate = !selectPage.checked && pageGames.some(game => selectedIds.has(game.id));
      selectPage.disabled = !pageGames.length;
      selectPage.addEventListener('change', () => {
        for (const game of pageGames) {
          if (selectPage.checked) selectedIds.add(game.id); else selectedIds.delete(game.id);
        }
        render();
      });
      selectPageLabel.append(selectPage, ' Select page');
      const selectAll = document.createElement('button');
      selectAll.type = 'button';
      selectAll.textContent = `Select all ${visible.length} matching`;
      selectAll.disabled = !visible.length || selectedIds.size === visible.length;
      selectAll.addEventListener('click', () => { visible.forEach(game => selectedIds.add(game.id)); render(); });
      const exportMenu = document.createElement('div');
      exportMenu.className = 'game-library-export';
      const exportButton = document.createElement('button');
      exportButton.type = 'button';
      exportButton.textContent = 'Export ▾';
      exportButton.disabled = !selectedIds.size;
      exportButton.setAttribute('aria-expanded', 'false');
      const exportChoices = document.createElement('div');
      exportChoices.className = 'game-library-export-options';
      exportChoices.hidden = true;
      exportButton.addEventListener('click', () => {
        exportChoices.hidden = !exportChoices.hidden;
        exportButton.setAttribute('aria-expanded', String(!exportChoices.hidden));
        if (!exportChoices.hidden) exportChoices.querySelector('button').focus();
      });
      exportMenu.addEventListener('keydown', event => {
        if (event.key === 'Escape' && !exportChoices.hidden) {
          event.preventDefault(); event.stopPropagation();
          exportChoices.hidden = true; exportButton.setAttribute('aria-expanded', 'false'); exportButton.focus();
        }
      });
      exportMenu.addEventListener('focusout', event => {
        if (!exportMenu.contains(event.relatedTarget)) {
          exportChoices.hidden = true; exportButton.setAttribute('aria-expanded', 'false');
        }
      });
      for (const [format, label] of [['pgn', 'PGN'], ['annotated', 'Annotated PGN'], ['json', 'Analysis JSON']]) {
        const choice = document.createElement('button');
        choice.type = 'button'; choice.textContent = label;
        choice.addEventListener('click', async () => {
          const ids = games.filter(game => selectedIds.has(game.id)).map(game => game.id);
          exportChoices.hidden = true; exportButton.setAttribute('aria-expanded', 'false');
          exportButton.disabled = true;
          try {
            await pendingWrite;
            const records = await Promise.all(ids.map(readGame));
            if (records.some(record => !record?.json)) throw new Error('A selected game is no longer available');
            const payload = buildGameExport(records, format, window.ironwoodExportSavedGame);
            downloadBlob(payload.filename, new Blob([payload.content], { type: payload.type }));
          } catch (error) {
            const message = document.createElement('p');
            message.setAttribute('role', 'alert'); message.textContent = `Could not export games: ${error.message}`;
            selection.append(message);
          } finally { exportButton.disabled = !selectedIds.size; }
        });
        exportChoices.append(choice);
      }
      exportMenu.append(exportButton, exportChoices);
      const removeSelected = document.createElement('button');
      removeSelected.type = 'button';
      removeSelected.className = 'danger';
      removeSelected.textContent = `Delete selected (${selectedIds.size})`;
      removeSelected.disabled = !selectedIds.size;
      removeSelected.addEventListener('click', () => removeGames(games.filter(game => selectedIds.has(game.id))));
      selection.append(selectPageLabel, selectAll, exportMenu, removeSelected);
      if (!visible.length) {
        list.textContent = games.some(game => gameMatchesCategory(game, category)) ? 'No games match these filters.' :
          category === 'all' ? 'No saved games yet.' :
          category === 'observed' ? 'Finished games you watch are saved here automatically.' :
          category === 'mine' ? 'No games here yet. Finished online games and games against Stockfish are saved automatically.' :
            category === 'imported' ? 'No imported games here yet. Import a PGN or analysis JSON to add one.' :
              'No favorite games yet. Mark a game as a favorite to see it here.';
        return;
      }
      const pageCount = Math.ceil(visible.length / pageSize);
      page = Math.min(page, pageCount - 1);
      const previous = document.createElement('button');
      previous.type = 'button';
      previous.textContent = '◀ Previous';
      previous.disabled = page === 0;
      previous.addEventListener('click', () => { page--; render(); list.scrollTop = 0; });
      const count = document.createElement('span');
      count.textContent = `Page ${page + 1} of ${pageCount} · ${visible.length} games`;
      count.setAttribute('aria-live', 'polite');
      const next = document.createElement('button');
      next.type = 'button';
      next.textContent = 'Next ▶';
      next.disabled = page + 1 >= pageCount;
      next.addEventListener('click', () => { page++; render(); list.scrollTop = 0; });
      pager.append(previous, count, next);
      for (const game of pageGames) {
        const entry = document.createElement('div');
        entry.className = `game-library-entry${selectedIds.has(game.id) ? ' selected' : ''}`;
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.className = 'game-library-select';
        checkbox.setAttribute('aria-label', `Select ${game.title}`);
        checkbox.checked = selectedIds.has(game.id);
        checkbox.addEventListener('change', () => {
          if (checkbox.checked) selectedIds.add(game.id); else selectedIds.delete(game.id);
          render();
          list.querySelectorAll('.game-library-select')[pageGames.indexOf(game)]?.focus();
        });
        entry.append(checkbox);
        const preview = document.createElement('button');
        preview.type = 'button';
        preview.className = 'game-library-preview';
        preview.setAttribute('aria-label', `Enlarge board preview: ${game.title}`);
        preview.title = 'Enlarge board preview';
        const previewBoard = finalBoard(game.finalFen);
        preview.append(previewBoard);
        preview.disabled = previewBoard.children.length !== 64;
        preview.addEventListener('click', () => showGamePreview(game, preview));
        entry.append(preview);
        const main = document.createElement('div');
        main.className = 'game-library-main';
        const name = document.createElement('strong');
        name.textContent = `${game.favorite ? '★ ' : ''}${game.title}`;
        const players = document.createElement('div');
        players.className = 'game-library-players';
        for (const [side, player] of [['White', game.white], ['Black', game.black]]) {
          const label = document.createElement('span');
          label.textContent = `${side}: ${player || 'Unknown'}`;
          players.append(label);
        }
        const result = document.createElement('span');
        result.className = 'game-library-result';
        result.textContent = game.result === '1-0' ? `${game.white} won (White)` :
          game.result === '0-1' ? `${game.black} won (Black)` :
          game.result === '1/2-1/2' ? 'Draw' : 'Result unknown / unfinished';
        const details = document.createElement('span');
        details.textContent = `${game.playedAt ? `Played ${game.playedAt}` : 'Date played unknown'} · ${game.venue ? `Venue: ${game.venue}` : 'Venue unknown'} · Saved ${new Date(game.updatedAt).toLocaleString()} · ${game.moves} moves · ${game.analysisStatus} · ${game.analyzed} positions analyzed${game.source ? ` · ${game.source}` : ''}`;
        const actions = document.createElement('div');
        actions.className = 'game-library-actions';
        const open = document.createElement('button');
        open.type = 'button';
        open.textContent = 'Open game';
        open.addEventListener('click', async () => {
          try {
            await pendingWrite;
            const selected = await readGame(game.id);
            if (!selected?.json) throw new Error('Saved game was not found');
            localStorage.setItem(ACTIVE_ID_KEY, game.id);
            localStorage.setItem(ACTIVE_CATEGORY_KEY, game.category);
            localStorage.setItem(CURRENT_GAME_KEY, selected.json);
            location.reload();
          } catch (error) {
            list.textContent = `Could not open game: ${error.message}`;
          }
        });
        const move = document.createElement('button');
        move.type = 'button';
        move.hidden = game.category === 'observed';
        move.textContent = game.category === 'mine' ? 'Move to Imported' : 'Move to My Games';
        move.addEventListener('click', async () => {
          const oldCategory = game.category;
          game.category = oldCategory === 'mine' ? 'imported' : 'mine';
          try {
            const stored = await readGame(game.id);
            if (!stored) throw new Error('Saved game was not found');
            await putGame({ ...stored, category: game.category });
            if (localStorage.getItem(ACTIVE_ID_KEY) === game.id) localStorage.setItem(ACTIVE_CATEGORY_KEY, game.category);
            render();
          } catch (error) {
            game.category = oldCategory;
            list.textContent = `Could not move game: ${error.message}`;
          }
        });
        const favorite = document.createElement('button');
        favorite.type = 'button';
        favorite.className = 'favorite';
        favorite.setAttribute('aria-pressed', String(Boolean(game.favorite)));
        favorite.textContent = game.favorite ? '★ Favorited' : '☆ Favorite';
        favorite.addEventListener('click', async () => {
          const oldFavorite = Boolean(game.favorite);
          game.favorite = !oldFavorite;
          try {
            const stored = await readGame(game.id);
            if (!stored) throw new Error('Saved game was not found');
            await putGame({ ...stored, favorite: game.favorite });
            render();
          } catch (error) {
            game.favorite = oldFavorite;
            list.textContent = `Could not update favorite: ${error.message}`;
          }
        });
        const remove = document.createElement('button');
        remove.type = 'button';
        remove.className = 'danger';
        remove.textContent = 'Delete';
        remove.addEventListener('click', () => removeGames([game]));
        actions.append(open, favorite, move, remove);
        main.append(name, players, result, details, actions);
        entry.append(main);
        list.append(entry);
      }
    };
    render();
  } catch (error) {
    list.textContent = `Saved games are unavailable: ${error.message}`;
  }
};
