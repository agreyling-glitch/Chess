const DB_NAME = 'ironwood-chess';
// Public Lichess API access; no Lichess code or artwork is bundled.
const explorerCache = new Map();
let explorerBusy = false, explorerRetryAt = 0, explorerToken = '', explorerGeneration = 0;
window.ironwoodExplorerToken = token => {
  explorerToken = token.trim(); explorerGeneration++; explorerCache.clear(); explorerRetryAt = 0;
};
export function openingExplorerUrl(fen, source, speed = '', ratings = '') {
  if (!['masters','lichess'].includes(source)) throw new Error('Invalid explorer source');
  if (speed && !['bullet','blitz','rapid','classical','correspondence'].includes(speed)) throw new Error('Invalid explorer speed');
  if (ratings && !ratings.split(',').every(r => ['0','1000','1200','1400','1600','1800','2000','2200','2500'].includes(r))) throw new Error('Invalid rating groups');
  const url = new URL(`https://explorer.lichess.org/${source}`);
  url.search = new URLSearchParams({fen,moves:'12',topGames:'0',recentGames:'0'});
  if (source === 'lichess') {
    url.searchParams.set('variant','standard');
    url.searchParams.set('speeds',speed || 'bullet,blitz,rapid,classical,correspondence');
    url.searchParams.set('ratings',ratings || '0,1000,1200,1400,1600,1800,2000,2200,2500');
  }
  return url.href;
}
window.ironwoodExplorer = (fen,source,speed,ratings) => {
  if (!explorerToken) return JSON.stringify({error:'Lichess requires an API token for Opening Explorer. Connect above to load statistics.'});
  const key = openingExplorerUrl(fen,source,speed,ratings);
  if (explorerCache.has(key)) return explorerCache.get(key);
  if (Date.now() < explorerRetryAt) return JSON.stringify({error:'Rate limited. Try again in one minute.'});
  if (!explorerBusy) {
    explorerBusy = true;
    const generation = explorerGeneration;
    void fetch(key,{credentials:'omit',headers:{Authorization:`Bearer ${explorerToken}`},signal:AbortSignal.timeout(20000)}).then(async response => {
      if (response.status === 401 || response.status === 403) throw new Error('Lichess rejected the API token. Check it or create a new token.');
      if (response.status === 429) { explorerRetryAt = Date.now()+60000; throw new Error('Rate limited. Try again in one minute.'); }
      if (!response.ok) throw new Error(`Opening database request failed (${response.status}).`);
      const data = await response.json();
      if (!Array.isArray(data.moves)) throw new Error('Unexpected opening database response.');
      if (explorerCache.size >= 100) explorerCache.delete(explorerCache.keys().next().value);
      if (generation === explorerGeneration) explorerCache.set(key,JSON.stringify(data));
    }).catch(error => {
      if (generation === explorerGeneration && (!explorerRetryAt || Date.now() >= explorerRetryAt)) explorerCache.set(key,JSON.stringify({error:error.name==='TimeoutError'?'Request timed out. Retry when ready.':error instanceof TypeError?'Could not load opening data. Check your connection and retry.':error.message}));
    }).finally(()=>{explorerBusy=false;});
  }
  return '';
};
window.ironwoodExplorerRetry = () => { for (const [key,value] of explorerCache) if (JSON.parse(value).error) explorerCache.delete(key); };
const STORE_NAME = 'games';
const ACTIVE_ID_KEY = 'ironwood.chess.active-game-id.v1';
const ACTIVE_CATEGORY_KEY = 'ironwood.chess.active-game-category.v1';
const CURRENT_GAME_KEY = 'ironwood.chess.game.v1';
const PREFERENCES_KEY = 'ironwood.chess.preferences.v1';
const TRAINING_KEY = 'ironwood.chess.training.v1';
const LIBRARY_RETURN_KEY = 'ironwood.chess.library-return.v1';
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

let lichessBusy = false;
let lichessRetryAt = 0;
let lichessStatus = '';
let lichessPgn = '';

export function importFilters(options = {}) {
  if (typeof options === 'string') options = JSON.parse(options);
  const limit = Number(options.limit ?? 20);
  if (![20,50,100,200].includes(limit)) throw new Error('Choose 20, 50, 100, or 200 games.');
  const speed = options.speed || '';
  if (!['','bullet','blitz','rapid','classical','correspondence','daily'].includes(speed)) throw new Error('Choose a valid time control.');
  const date = (value, end) => {
    if (!value) return null;
    const time = Date.parse(`${value}T00:00:00Z`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || !Number.isFinite(time)
      || new Date(time).toISOString().slice(0,10) !== value) throw new Error('Use valid dates in YYYY-MM-DD format.');
    return time + (end ? 86400000 - 1 : 0);
  };
  const since = date(options.from, false), until = date(options.to, true);
  if (since !== null && until !== null && since > until) throw new Error('The start date must be on or before the end date.');
  return {limit,speed,since,until};
}

export function lichessExportUrl(input, options = {}) {
  const value = input.trim();
  let path;
  if (/^https?:\/\//i.test(value)) {
    const url = new URL(value);
    if (url.protocol !== 'https:' || url.hostname !== 'lichess.org' || url.port) {
      throw new Error('Use an https://lichess.org game link.');
    }
    const match = url.pathname.match(/^\/([a-zA-Z0-9]{8})(?:[a-zA-Z0-9]{4})?(?:\/(?:white|black))?\/?$/);
    if (!match) throw new Error('Paste a Lichess game link, or enter a username.');
    path = `/game/export/${match[1]}`;
  } else {
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(value)) throw new Error('Enter a valid Lichess username or game link.');
    path = `/api/games/user/${encodeURIComponent(value)}`;
  }
  const url = new URL(path, 'https://lichess.org');
  url.search = new URLSearchParams({ max: '20', ongoing: 'false', finished: 'true',
    moves: 'true', tags: 'true', clocks: 'true', opening: 'true', evals: 'false' });
  if (path.startsWith('/api/games/user/')) {
    const filters = importFilters(options);
    url.searchParams.set('max', String(filters.limit));
    url.searchParams.set('perfType', filters.speed || 'ultraBullet,bullet,blitz,rapid,classical,correspondence,chess960');
    if (filters.since !== null) url.searchParams.set('since', String(filters.since));
    if (filters.until !== null) url.searchParams.set('until', String(filters.until));
  }
  return url.href;
}

window.ironwoodLichessStatus = () => lichessStatus;
window.ironwoodPollLichess = () => {
  const pgn = lichessPgn;
  lichessPgn = '';
  return pgn;
};
window.ironwoodFetchLichess = async (input, options = {}) => {
  if (lichessBusy || lichessPgn) return;
  if (Date.now() < lichessRetryAt) {
    lichessStatus = 'Lichess rate limit: wait a full minute before trying again.';
    return;
  }
  lichessBusy = true;
  lichessStatus = 'Fetching completed games from Lichess…';
  try {
    const response = await fetch(lichessExportUrl(input, options), {
      headers: { Accept: 'application/x-chess-pgn' }, credentials: 'omit',
      signal: AbortSignal.timeout(30000), cache: 'no-store',
    });
    if (response.status === 429) {
      lichessRetryAt = Date.now() + 60000;
      throw new Error('Lichess rate limit: wait a full minute before trying again.');
    }
    if (response.status === 404) throw new Error('Lichess user or game not found.');
    if (!response.ok) throw new Error(`Lichess request failed (${response.status}). Try again later.`);
    const pgn = await response.text();
    if (!pgn.trim()) throw new Error('No completed games match these filters. Try a wider date range or another time control.');
    if (!/^\[Event\s/m.test(pgn)) throw new Error('Lichess returned an unexpected game format.');
    if (/^\[Result "\*"\]/m.test(pgn)) throw new Error('This game is still in progress. Import it after it finishes.');
    lichessPgn = pgn;
    lichessStatus = 'Games fetched. Continue to choose games to import.';
  } catch (error) {
    lichessStatus = error.name === 'TimeoutError' ? 'Lichess request timed out. Try again.' :
      error instanceof TypeError ? 'Could not reach Lichess. Check your connection and try again.' : error.message;
  } finally {
    lichessBusy = false;
  }
};

let chessComBusy = false;
let chessComRetryAt = 0;
let chessComStatus = '';
let chessComPgn = '';

export function chessComUsername(input) {
  const value = input.trim();
  if (!/^[a-zA-Z0-9_-]{2,30}$/.test(value)) throw new Error('Enter a Chess.com username, rather than a game link.');
  return value.toLowerCase();
}

export async function fetchChessComGames(input, request = fetch, options = {}) {
  const filters = importFilters(options);
  const username = chessComUsername(input);
  const base = `https://api.chess.com/pub/player/${encodeURIComponent(username)}/games`;
  const get = async url => {
    const response = await request(url, { headers: { Accept: 'application/json' }, credentials: 'omit', signal: AbortSignal.timeout(30000) });
    if (response.status === 429) throw new Error('Chess.com rate limit: wait a full minute before trying again.');
    if (response.status === 404) throw new Error('Chess.com user or game archive not found.');
    if (!response.ok) throw new Error(`Chess.com request failed (${response.status}). Try again later.`);
    return response.json();
  };
  const listing = await get(`${base}/archives`);
  if (!Array.isArray(listing.archives)) throw new Error('Chess.com returned an unexpected archive format.');
  // Never follow arbitrary URLs from a remote response.
  const archives = [...new Set(listing.archives)].filter(url => typeof url === 'string' && url.startsWith(`${base}/`)
    && /^\d{4}\/(?:0[1-9]|1[0-2])$/.test(url.slice(base.length + 1))).sort().reverse();
  const games = [];
  const seen = new Set();
  for (const archive of archives) {
    const monthStart = Date.parse(`${archive.slice(-7).replace('/','-')}-01T00:00:00Z`);
    const nextMonth = new Date(monthStart); nextMonth.setUTCMonth(nextMonth.getUTCMonth()+1);
    if (filters.until !== null && monthStart > filters.until) continue;
    if (filters.since !== null && nextMonth.getTime() <= filters.since) break;
    const month = await get(archive); // Serial requests respect the Published Data API guidance.
    if (!Array.isArray(month.games)) throw new Error('Chess.com returned an unexpected game format.');
    for (const game of month.games) {
      if (!['chess', 'chess960'].includes(game.rules) || typeof game.pgn !== 'string'
        || !Number.isFinite(game.end_time) || !/^\[Event\s/m.test(game.pgn)
        || !/^\[Result "(?:1-0|0-1|1\/2-1\/2)"\]/m.test(game.pgn)
        || (filters.since !== null && game.end_time*1000 < filters.since)
        || (filters.until !== null && game.end_time*1000 > filters.until)
        || (filters.speed && game.time_class !== filters.speed)) continue;
      const key = typeof game.url === 'string' ? game.url : game.pgn;
      if (!seen.has(key)) { seen.add(key); games.push(game); }
    }
    games.sort((a,b) => b.end_time - a.end_time);
    if (games.length >= filters.limit) break;
  }
  if (!games.length) throw new Error('No completed standard chess or Chess960 games match these filters.');
  return games.slice(0,filters.limit).map(game => game.pgn.trim()).join('\n\n');
}

window.ironwoodChessComStatus = () => chessComStatus;
window.ironwoodPollChessCom = () => {
  const pgn = chessComPgn;
  chessComPgn = '';
  return pgn;
};
window.ironwoodFetchChessCom = async (input, options = {}) => {
  if (chessComBusy || chessComPgn) return;
  if (Date.now() < chessComRetryAt) { chessComStatus = 'Chess.com rate limit: wait a full minute before trying again.'; return; }
  chessComBusy = true;
  chessComStatus = 'Fetching latest completed games from Chess.com…';
  try {
    chessComPgn = await fetchChessComGames(input, fetch, options);
    chessComStatus = 'Games fetched. Continue to choose games to import.';
  } catch (error) {
    if (error.message?.includes('rate limit')) chessComRetryAt = Date.now() + 60000;
    chessComStatus = error.name === 'TimeoutError' ? 'Chess.com request timed out. Try again.' :
      error instanceof TypeError ? 'Could not reach Chess.com. Check your connection and try again.' : error.message;
  } finally { chessComBusy = false; }
};

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
      games.push({ ...summary, ...details, category: details.trainingProfileId ? 'training' : summary.category || details.category, source: summary.source, favorite: Boolean(summary.favorite), fingerprint: storedFingerprint(cursor.value) });
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

export function validateTrainingProfiles(value) {
  if (!value || !Array.isArray(value.profiles) || typeof value.selected_id !== 'string') throw new Error('Invalid training profiles');
  const ids = new Set();
  for (const profile of value.profiles) {
    if (!profile || typeof profile.id !== 'string' || !profile.id || ids.has(profile.id) ||
        typeof profile.name !== 'string' || !profile.name.trim() || !Array.isArray(profile.results)) throw new Error('Invalid training profile');
    ids.add(profile.id);
    const sessions = new Set();
    for (const entry of profile.results) {
      const s = entry?.session;
      const expectedScore = entry?.result === '1/2-1/2' ? 0.5 :
        (entry?.result === '1-0' && s?.side === 'White') || (entry?.result === '0-1' && s?.side === 'Black') ? 1 : 0;
      if (!s || typeof s.id !== 'string' || !s.id || sessions.has(s.id) || s.profile_id !== profile.id ||
          typeof s.profile_name !== 'string' || !['White','Black'].includes(s.side) ||
          !Number.isInteger(s.opponent_elo) || s.opponent_elo < 1320 || s.opponent_elo > 3190 ||
          !Number.isInteger(s.rating_before) || !Number.isInteger(s.rating_after) ||
          typeof s.started_at !== 'string' || typeof s.completed_at !== 'string' ||
          !['1-0','0-1','1/2-1/2'].includes(entry.result) || entry.score !== expectedScore || typeof entry.timed_out !== 'boolean') throw new Error('Invalid training result');
      sessions.add(s.id);
      const a = entry.analysis;
      if (a && (!Array.isArray(a.phases) || a.phases.length !== 3 ||
          ![a.total_player_moves,a.missed_mates,a.allowed_mates,...a.phases.flatMap(p => [p.moves,p.cp_moves ?? 0,p.loss_cp,p.mistakes,p.blunders])].every(n => Number.isSafeInteger(n) && n >= 0))) throw new Error('Invalid training analysis');
    }
  }
  return structuredClone(value);
}

export function mergeTrainingProfiles(current, incoming) {
  const result = current ? validateTrainingProfiles(current) : { selected_id: '', profiles: [] };
  const backup = validateTrainingProfiles(incoming);
  for (const p of backup.profiles) {
    let target = result.profiles.find(q => q.id === p.id);
    if (!target) { target = { ...p, results: [] }; result.profiles.push(target); }
    for (const entry of p.results) {
      const existing = target.results.find(r => r.session.id === entry.session.id);
      if (!existing) target.results.push(entry);
      else if ((entry.analysis?.phases.reduce((n,p) => n+p.moves,0) || 0) >
               (existing.analysis?.phases.reduce((n,p) => n+p.moves,0) || 0)) existing.analysis = entry.analysis;
    }
    target.results.sort((a,b) => a.session.completed_at.localeCompare(b.session.completed_at) || a.session.id.localeCompare(b.session.id));
    const ratings = { White: 1320, Black: 1320 };
    for (const entry of target.results) {
      const s = entry.session;
      s.rating_before = ratings[s.side];
      const expected = 1/(1+10**((s.opponent_elo-s.rating_before)/400));
      const delta = 32*(entry.score-expected);
      // Rust rounds negative half-points away from zero.
      s.rating_after = s.rating_before + Math.sign(delta)*Math.round(Math.abs(delta));
      ratings[s.side] = s.rating_after;
    }
  }
  if (!result.profiles.some(p => p.id === result.selected_id)) result.selected_id = backup.selected_id;
  return result;
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
      category: gameDetails(record.json).trainingProfileId ? 'training' : ['imported', 'observed'].includes(record.category) ? record.category : 'mine',
      favorite: Boolean(record.favorite),
      updatedAt: Number(record.updatedAt) || Date.now(),
    };
  });
  if (value.preferences !== null && value.preferences !== undefined &&
      (typeof value.preferences !== 'object' || Array.isArray(value.preferences))) {
    throw new Error('The backup preferences are invalid');
  }
  const trainingProfiles = value.training_profiles == null ? null : validateTrainingProfiles(value.training_profiles);
  return { games, preferences: value.preferences ?? null, trainingProfiles };
}

export function backupSummary(value) {
  const backup = validateBackup(value);
  return {
    games: backup.games.length,
    mine: backup.games.filter(game => game.category === 'mine').length,
    imported: backup.games.filter(game => game.category === 'imported').length,
    observed: backup.games.filter(game => game.category === 'observed').length,
    training: backup.games.filter(game => game.category === 'training').length,
    profiles: backup.trainingProfiles?.profiles.length || 0,
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
  windowControls.append(close);
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
      training_profiles: JSON.parse(localStorage.getItem(TRAINING_KEY) || 'null'),
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
      message.textContent = `${file.name} contains ${summary.games} games (${summary.mine} personal, ${summary.imported} imported, ${summary.observed} observed, ${summary.training} training, ${summary.favorites} favorites)${summary.hasPreferences ? ' and display preferences' : ''}.`;
      const guidance = document.createElement('p');
      guidance.className = 'storage-guidance';
      guidance.textContent = `Merge keeps existing games and adds the backup. Replace removes existing games first. Both options restore included preferences and training profiles (${summary.profiles}). Training results are merged by game ID so ratings are not counted twice.`;
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
            const profiles = backup.trainingProfiles ? mergeTrainingProfiles(
              replace ? null : JSON.parse(localStorage.getItem(TRAINING_KEY) || 'null'), backup.trainingProfiles) : null;
            await replaceOrMergeGames(backup.games, replace);
            if (profiles) localStorage.setItem(TRAINING_KEY, JSON.stringify(profiles));
            else if (replace) localStorage.removeItem(TRAINING_KEY);
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
      ['Training games', String(games.filter(game => game.category === 'training').length)],
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
  const training = game.training;
  const opponent = training ? `Stockfish ${training.opponent_elo} Elo` : game.engine_config?.opponent === 'Lc0' ? 'Lc0 · Good Gyal' : 'Stockfish 19';
  let profileName = training?.profile_name || null;
  if (training?.profile_id) {
    try {
      const profiles = JSON.parse(localStorage.getItem(TRAINING_KEY) || 'null');
      profileName = profiles?.profiles?.find(profile => profile.id === training.profile_id)?.name || profileName;
    } catch { /* Keep the recorded name if profile storage is unavailable. */ }
  }
  const human = profileName || 'You';
  const bothSides = game.engine_enabled === false;
  const white = training ? (training.side === 'Black' ? opponent : human) : imported ? pgnTag(game.review_pgn, 'White') || 'Unknown' : bothSides ? 'White' : game.player_side === 'Black' ? opponent : human;
  const black = training ? (training.side === 'Black' ? human : opponent) : imported ? pgnTag(game.review_pgn, 'Black') || 'Unknown' : bothSides ? 'Black' : game.player_side === 'Black' ? human : opponent;
  const rawDate = pgnTag(game.review_pgn, 'Date');
  const playedAt = training?.started_at?.slice(0, 10) || (rawDate && rawDate !== '????.??.??' ? rawDate : null);
  const rawSite = pgnTag(game.review_pgn, 'Site');
  const venue = rawSite && rawSite !== '-' && rawSite !== '?' ? rawSite : null;
  const recordedResult = pgnTag(game.review_pgn, 'Result') || game.result;
  const result = ['1-0', '0-1', '1/2-1/2', '*'].includes(recordedResult) ? recordedResult : '*';
  const moves = game.live_moves?.length || pgnMoveCount(game.review_pgn) || Math.max(0, (game.game_analysis?.length || 1) - 1);
  const analyzed = game.game_analysis?.filter(Boolean).length || 0;
  const analysisStatus = analyzed === 0 ? 'Not analyzed' : analyzed >= moves + 1 ? 'Complete' : 'Partial';
  const whiteFideId = pgnTag(game.review_pgn, 'WhiteFideId') || null;
  const blackFideId = pgnTag(game.review_pgn, 'BlackFideId') || null;
  const startingNote = typeof game.move_notes?.[0] === 'string' ? game.move_notes[0].trim() : '';
  return { startingNote, analysisStatus, title: `${white} vs ${black}`, white, black, whiteFideId, blackFideId, playedAt, venue, result, moves, analyzed,
    trainingProfileId: training?.profile_id || null, trainingProfileName: profileName, trainingSide: training?.side || null,
    trainingOpponentElo: training?.opponent_elo, ratingBefore: training?.rating_before, ratingAfter: training?.rating_after,
    finalFen: game.final_board || game.board, category: training ? 'training' : imported ? 'imported' : 'mine' };
}

export function fideProfileUrl(id) {
  const value = typeof id === 'string' ? id.trim() : '';
  return /^\d+$/.test(value) && /[1-9]/.test(value)
    ? `https://ratings.fide.com/profile/${value}` : null;
}

export function playerNameElement(player, fideId) {
  const url = fideProfileUrl(fideId);
  const element = document.createElement(url ? 'a' : 'span');
  element.textContent = player || 'Unknown';
  if (url) {
    element.href = url;
    element.target = '_blank';
    element.rel = 'noopener noreferrer';
    element.title = `Open FIDE profile (${fideId.trim()})`;
  }
  return element;
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
    const details = gameDetails(json);
    queueGameRecord(json, activeId(), details.trainingProfileId ? 'training' : localStorage.getItem(ACTIVE_CATEGORY_KEY) || details.category);
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
    localStorage.setItem(ACTIVE_CATEGORY_KEY, ['imported', 'observed', 'training'].includes(category) ? category : 'mine');
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

async function openSavedGame(game, startingPosition = false) {
  await pendingWrite;
  const selected = await readGame(game.id);
  if (!selected?.json) throw new Error('Saved game was not found');
  const saved = JSON.parse(selected.json);
  if (startingPosition) saved.review_index = 0;
  localStorage.setItem(ACTIVE_ID_KEY, game.id);
  localStorage.setItem(ACTIVE_CATEGORY_KEY, game.category);
  localStorage.setItem(CURRENT_GAME_KEY, JSON.stringify(saved));
  location.reload();
}

function startingNoteElement(game, compact = false) {
  if (!game.startingNote) return null;
  const section = document.createElement('section');
  section.className = 'game-starting-note';
  const heading = document.createElement('strong');
  heading.textContent = 'Starting note';
  const text = document.createElement(compact ? 'button' : 'p');
  text.textContent = game.startingNote;
  if (compact) {
    text.type = 'button';
    text.className = 'game-starting-note-preview';
    text.title = 'Open game at the starting position';
    text.addEventListener('click', async () => {
      try { await openSavedGame(game, true); }
      catch (error) { alert(`Could not open game: ${error.message}`); }
    });
  } else {
    text.className = 'game-starting-note-full';
  }
  section.append(heading, text);
  return section;
}

async function showGamePreview(game, trigger) {
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
  const boardHost = document.createElement('div');
  const controls = document.createElement('div');
  controls.className = 'game-preview-controls';
  let positions = [game.finalFen];
  let moves = [];
  let index = 0;
  let timer = null;
  const stop = () => {
    if (timer !== null) clearInterval(timer);
    timer = null;
    autoplay.textContent = 'Auto play';
    autoplay.setAttribute('aria-pressed', 'false');
  };
  const button = (label, action) => {
    const element = document.createElement('button');
    element.type = 'button'; element.textContent = label;
    element.addEventListener('click', action); controls.append(element);
    return element;
  };
  const first = button('First', () => { stop(); index = 0; render(); });
  const previous = button('Previous', () => { stop(); index = Math.max(0, index - 1); render(); });
  const autoplay = button('Auto play', () => {
    if (timer !== null) { stop(); return; }
    if (index === positions.length - 1) index = 0;
    autoplay.textContent = 'Pause'; autoplay.setAttribute('aria-pressed', 'true');
    render();
    timer = setInterval(() => {
      index = Math.min(index + 1, positions.length - 1);
      if (index === positions.length - 1) stop();
      render();
    }, 500);
  });
  autoplay.setAttribute('aria-pressed', 'false');
  const next = button('Next', () => { stop(); index = Math.min(positions.length - 1, index + 1); render(); });
  const last = button('Last', () => { stop(); index = positions.length - 1; render(); });
  dialog.addEventListener('keydown', event => {
    if (event.altKey || event.ctrlKey || event.metaKey || !['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    stop();
    index = Math.max(0, Math.min(positions.length - 1, index + (event.key === 'ArrowLeft' ? -1 : 1)));
    render();
  });
  const render = () => {
    const board = finalBoard(positions[index]);
    board.setAttribute('aria-label', `Position ${index} of ${positions.length - 1}: ${game.title}`);
    boardHost.replaceChildren(board);
    caption.textContent = `${positions.length > 1 ? index === 0 ? 'Starting position' : `Move ${index} of ${positions.length - 1} · ${moves[index - 1] || ''}` : 'Final position'} · ${game.result || '*'}${game.playedAt ? ` · ${game.playedAt}` : ''}`;
    first.disabled = previous.disabled = index === 0;
    next.disabled = last.disabled = index === positions.length - 1;
    autoplay.disabled = positions.length < 2;
  };
  render();
  dialog.append(heading, caption, boardHost, controls);
  const note = startingNoteElement(game);
  if (note) dialog.append(note);
  dialog.addEventListener('close', () => {
    stop();
    dialog.remove();
    if (trigger.isConnected) trigger.focus();
  }, { once: true });
  document.body.append(dialog);
  dialog.showModal();
  close.focus();
  try {
    const stored = await readGame(game.id);
    if (!dialog.open) return;
    if (!stored?.json) throw new Error('Saved game was not found');
    const saved = JSON.parse(stored.json);
    const preview = window.ironwoodExportSavedGame
      ? JSON.parse(window.ironwoodExportSavedGame(stored.json, 'preview'))
      : { positions: saved.live_positions, moves: saved.live_moves };
    if (Array.isArray(preview.positions) && preview.positions.length) {
      positions = preview.positions; moves = preview.moves || [];
      index = positions.length - 1;
      render();
    }
  } catch (error) {
    caption.textContent += ` · Move preview unavailable: ${error.message}`;
  }
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

window.ironwoodOpenGameLibrary = async (restore = null) => {
  let dialog = document.getElementById('game-library-dialog');
  if (!dialog) {
    dialog = document.createElement('dialog');
    dialog.id = 'game-library-dialog';
    document.body.append(dialog);
  }
  if (restore) dialog.classList.toggle('maximized', Boolean(restore.maximized));
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
    let category = restore?.category || 'all';
    let profileFilter = restore?.profileFilter || '*';
    let playerFilter = restore?.playerFilter || '*';
    let searchQuery = restore?.searchQuery || '';
    let analysisFilter = restore?.analysisFilter || '*';
    let page = Number.isSafeInteger(restore?.page) ? Math.max(0, restore.page) : 0;
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
          sessionStorage.setItem(LIBRARY_RETURN_KEY, JSON.stringify({
            category, profileFilter, playerFilter, searchQuery, analysisFilter, page,
            maximized: dialog.classList.contains('maximized'),
          }));
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
        ['training', 'Training'],
        ['observed', 'Observed'],
        ['imported', 'Imported Games'],
        ['favorites', 'Favorites'],
      ]) {
        const tab = document.createElement('button');
        tab.type = 'button';
        tab.textContent = `${label} (${games.filter(game => gameMatchesCategory(game, value)).length})`;
        tab.className = category === value ? 'selected' : '';
        tab.addEventListener('click', () => { category = value; playerFilter = '*'; profileFilter = '*'; selectedIds.clear(); page = 0; render(); });
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
      if (category === 'training') {
        const label = document.createElement('label'); label.textContent = 'Profile ';
        const select = document.createElement('select'); select.setAttribute('aria-label', 'Training profile');
        for (const [id, name] of [['*', 'All profiles'], ...new Map(games.filter(g => g.trainingProfileId).map(g => [g.trainingProfileId, g.trainingProfileName])).entries()]) {
          const option = document.createElement('option'); option.value = id; option.textContent = name; select.append(option);
        }
        select.value = profileFilter;
        select.addEventListener('change', () => { profileFilter = select.value; selectedIds.clear(); page = 0; render(); });
        label.append(select); filters.append(label);
      }
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
      const visible = games.filter(game => [game.title, game.white, game.black, game.source, game.venue, game.playedAt].join(' ').toLowerCase().includes(searchQuery.trim().toLowerCase()) && gameMatchesCategory(game, category) && (profileFilter === '*' || game.trainingProfileId === profileFilter) && (playerFilter === '*' ||
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
          category === 'training' ? 'Start Training vs AI from New game to add games under a profile.' :
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
        // listGames reads these from the original PGN, including older saved records.
        const fide = game;
        name.append(game.favorite ? '★ ' : '', playerNameElement(game.white, fide.whiteFideId),
          ' vs ', playerNameElement(game.black, fide.blackFideId));
        const players = document.createElement('div');
        players.className = 'game-library-players';
        for (const [side, player, fideId] of [['White', game.white, fide.whiteFideId], ['Black', game.black, fide.blackFideId]]) {
          const label = document.createElement('span');
          label.append(`${side}: `, playerNameElement(player, fideId));
          players.append(label);
        }
        const result = document.createElement('span');
        result.className = 'game-library-result';
        result.textContent = game.result === '1-0' ? `${game.white} won (White)` :
          game.result === '0-1' ? `${game.black} won (Black)` :
          game.result === '1/2-1/2' ? 'Draw' : 'Result unknown / unfinished';
        const details = document.createElement('span');
        details.textContent = `${game.playedAt ? `Played ${game.playedAt}` : 'Date played unknown'} · ${game.venue ? `Venue: ${game.venue}` : 'Venue unknown'} · Saved ${new Date(game.updatedAt).toLocaleString()} · ${game.moves} moves · ${game.analysisStatus} · ${game.analyzed} positions analyzed${game.source ? ` · ${game.source}` : ''}`;
        if (game.trainingProfileId) {
          details.textContent += ` · Training: ${game.trainingProfileName} · ${game.trainingSide} · Elo ${game.ratingBefore}`;
          if (Number.isFinite(game.ratingAfter)) details.textContent += ` → ${game.ratingAfter} (${game.ratingAfter-game.ratingBefore >= 0 ? '+' : ''}${game.ratingAfter-game.ratingBefore})`;
        }
        const actions = document.createElement('div');
        actions.className = 'game-library-actions';
        const open = document.createElement('button');
        open.type = 'button';
        open.textContent = 'Open game';
        open.addEventListener('click', async () => {
          try {
            await openSavedGame(game);
          } catch (error) {
            list.textContent = `Could not open game: ${error.message}`;
          }
        });
        const move = document.createElement('button');
        move.type = 'button';
        move.hidden = game.category === 'observed' || game.category === 'training';
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
        main.append(name, players, result, details);
        const note = startingNoteElement(game, true);
        if (note) main.append(note);
        main.append(actions);
        entry.append(main);
        list.append(entry);
      }
    };
    render();
  } catch (error) {
    list.textContent = `Saved games are unavailable: ${error.message}`;
  }
};


// Deleting the active game resets the WASM board via reload. Return to the library
// once that reset is complete, consuming the marker so later visits stay normal.
if (typeof document !== 'undefined') {
  const returnToLibrary = () => {
    try {
      const saved = sessionStorage.getItem(LIBRARY_RETURN_KEY);
      if (!saved) return;
      sessionStorage.removeItem(LIBRARY_RETURN_KEY);
      window.ironwoodOpenGameLibrary(JSON.parse(saved));
    } catch { /* An unavailable session store should not block app startup. */ }
  };
  if (document.readyState === 'complete') returnToLibrary();
  else window.addEventListener('load', returnToLibrary, { once: true });
}
