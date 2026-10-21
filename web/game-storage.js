const DB_NAME = 'ironwood-chess';
// Session-only summaries and view state; no disk persistence or extra credentials.
const savedLibraryViews = {browser:null, archive:null};
const libraryVersions = {browser:0, archive:0};
const archivePageCache = new Map();
const archiveFacetCache = new Map();
let archiveConnectionVersion=0;
function notifyLibraryChanged(source, extra={}) {
  if(window.CustomEvent&&window.dispatchEvent)window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source,...extra}}));
  else invalidateLibraryCache(source);
}
function invalidateLibraryCache(source) {
  libraryVersions[source]++;
  if(source==='archive'){archivePageCache.clear();archiveFacetCache.clear();}
  if(savedLibraryViews[source])savedLibraryViews[source].valid=false;
}
if(typeof window!=='undefined') {
  window.addEventListener?.('ironwood-game-library-changed',event=>{
    if(!['browser','archive'].includes(event.detail?.source))return;
    invalidateLibraryCache(event.detail.source);
    if(event.detail.deletedIds&&savedLibraryViews[event.detail.source]) {
      const view=savedLibraryViews[event.detail.source],removed=new Set(event.detail.deletedIds);
      view.selected=view.selected.filter(id=>!removed.has(id));
      view.records=view.records.filter(record=>!removed.has(record.id));
    }
  });
  const forgetArchive=()=>{archiveConnectionVersion++;invalidateLibraryCache('archive');savedLibraryViews.archive=null;};
  window.addEventListener?.('ironwood-companion-connection-changed',forgetArchive);
  window.addEventListener?.('ironwood-companion-feature-changed',()=>{if(window.ironwoodCompanionFeatureEnabled?.()!==true)forgetArchive();});
}

// Public Lichess API access; no Lichess code or artwork is bundled.
const explorerCache = new Map();
let explorerBusy = false, explorerRetryAt = 0, explorerToken = '', explorerGeneration = 0;
let explorerController, explorerActiveKey = '';
function syncExplorerSession() {
  const token = sessionStorage.getItem('ironwood.lichess.token') || '';
  if (token !== explorerToken) {
    explorerController?.abort();
    explorerToken = token; explorerGeneration++; explorerCache.clear(); explorerRetryAt = 0;
  }
  return !!token;
}
window.ironwoodExplorerSignedIn = syncExplorerSession;
export function openingExplorerUrl(fen, source, speed = '', ratings = '', player = '', color = 'white', since = '', until = '') {
  if (!['masters','lichess','player'].includes(source)) throw new Error('Invalid explorer source');
  if (speed && !['bullet','blitz','rapid','classical','correspondence'].includes(speed)) throw new Error('Invalid explorer speed');
  if (ratings && !ratings.split(',').every(r => ['0','1000','1200','1400','1600','1800','2000','2200','2500'].includes(r))) throw new Error('Invalid rating groups');
  const url = new URL(`https://explorer.lichess.org/${source}`);
  url.search = new URLSearchParams({fen,moves:'12',topGames:'0',recentGames:'0'});
  if (source === 'lichess') {
    url.searchParams.set('variant','standard');
    url.searchParams.set('speeds',speed || 'bullet,blitz,rapid,classical,correspondence');
    url.searchParams.set('ratings',ratings || '0,1000,1200,1400,1600,1800,2000,2200,2500');
  }
  if (source === 'player') {
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(player)) throw new Error('Enter a valid Lichess username.');
    if (!['white','black'].includes(color)) throw new Error('Choose White or Black.');
    for (const month of [since, until]) if (month && !/^\d{4}-(0[1-9]|1[0-2])$/.test(month)) throw new Error('Use YYYY-MM for dates.');
    if (since && until && since > until) throw new Error('Start month must precede end month.');
    for (const [key,value] of Object.entries({player,color,variant:'standard',recentGames:'8',speeds:speed,since,until}))
      if (value) url.searchParams.set(key,value);
  }
  return url.href;
}
export async function readExplorerStream(response, publish) {
  const reader = response.body.getReader(), decoder = new TextDecoder();
  let buffer = '', latest;
  const consume = line => {
    if (!line.trim()) return;
    const data = JSON.parse(line);
    if (!Array.isArray(data.moves)) throw new Error('Unexpected opening database response.');
    latest = data; publish({...data, indexing:true});
  };
  try {
    while (true) {
      const {value,done} = await reader.read();
      buffer += decoder.decode(value,{stream:!done});
      if (buffer.length > 1048576) throw new Error('Opening database response is too large.');
      let end;
      while ((end = buffer.indexOf('\n')) >= 0) { consume(buffer.slice(0,end)); buffer = buffer.slice(end+1); }
      if (done) { consume(buffer); break; }
    }
    if (!latest) throw new Error('No opening statistics received. Retry when ready.');
    return latest;
  } finally { await reader.cancel().catch(()=>{}); reader.releaseLock(); }
}
window.ironwoodExplorer = (fen,source,speed,ratings,player,color,since,until) => {
  if (!syncExplorerSession()) return JSON.stringify({error:'Sign in through Lichess-Online to load opening statistics.'});
  let key;
  try { key = openingExplorerUrl(fen,source,speed,ratings,player,color,since,until); }
  catch (error) { return JSON.stringify({error:error.message}); }
  if (explorerBusy && explorerActiveKey !== key && new URL(explorerActiveKey).pathname === '/player') explorerController?.abort();
  if (explorerCache.has(key)) return explorerCache.get(key);
  if (Date.now() < explorerRetryAt) return JSON.stringify({error:'Rate limited. Try again in one minute.'});
  if (!explorerBusy) {
    explorerBusy = true;
    explorerActiveKey = key;
    explorerController = new AbortController();
    const generation = explorerGeneration;
    void fetch(key,{credentials:'omit',cache:'no-store',headers:{Authorization:`Bearer ${explorerToken}`},signal:AbortSignal.any([explorerController.signal,AbortSignal.timeout(source === 'player' ? 120000 : 20000)])}).then(async response => {
      if (response.status === 401 || response.status === 403) throw new Error('Lichess rejected this session. Sign out and sign in again through Lichess-Online.');
      if (response.status === 429) { explorerRetryAt = Date.now()+60000; throw new Error('Rate limited. Try again in one minute.'); }
      if (!response.ok) throw new Error(`Opening database request failed (${response.status}).`);
      const publish = data => {
        if (syncExplorerSession() && generation === explorerGeneration) {
          if (explorerCache.size >= 100 && !explorerCache.has(key)) explorerCache.delete(explorerCache.keys().next().value);
          explorerCache.set(key,JSON.stringify(data));
        }
      };
      const data = source === 'player' ? await readExplorerStream(response,publish) : await response.json();
      if (!Array.isArray(data.moves)) throw new Error('Unexpected opening database response.');
      if (explorerCache.size >= 100) explorerCache.delete(explorerCache.keys().next().value);
      if (syncExplorerSession() && generation === explorerGeneration) explorerCache.set(key,JSON.stringify(data));
    }).catch(error => {
      if (error.name === 'AbortError') { if (generation === explorerGeneration) explorerCache.delete(key); return; }
      if (generation === explorerGeneration && (!explorerRetryAt || Date.now() >= explorerRetryAt)) explorerCache.set(key,JSON.stringify({error:error.name==='TimeoutError'?'Request timed out. Retry when ready.':error instanceof TypeError?'Could not load opening data. Check your connection and retry.':error.message}));
    }).finally(()=>{explorerBusy=false;});
  }
  return '';
};
window.ironwoodExplorerRefresh = () => {
  syncExplorerSession();
  // An active stream already receives updates; do not restart indexing while queued.
  for (const [key,value] of explorerCache)
    if (new URL(key).pathname === '/player' && !JSON.parse(value).indexing) explorerCache.delete(key);
};
window.ironwoodExplorerRetry = () => { for (const [key,value] of explorerCache) if (JSON.parse(value).error) explorerCache.delete(key); };
const STORE_NAME = 'games';
const ACTIVE_ID_KEY = 'ironwood.chess.active-game-id.v1';
const ACTIVE_CATEGORY_KEY = 'ironwood.chess.active-game-category.v1';
const CURRENT_GAME_KEY = 'ironwood.chess.game.v1';
const ACTIVE_ARCHIVE_KEY = 'ironwood.chess.active-archive-game.v1';
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
let importArchive = null, archiveProbeBusy = false, archiveProbeAt = 0;
window.ironwoodArchiveImportAvailable = () => {
  if (window.ironwoodCompanionFeatureEnabled?.() !== true) { importArchive = null; return false; }
  try { importArchive?.check(); } catch { importArchive = null; }
  if (!archiveProbeBusy && Date.now() - archiveProbeAt > 5000) {
    archiveProbeBusy = true; archiveProbeAt = Date.now();
    void archiveLibrary(false).then(async storage => {
      const capabilities = await storage.request('capabilities');
      storage.check();
      importArchive = capabilities.services?.archive ? storage : null;
    }).catch(() => { importArchive = null; }).finally(() => { archiveProbeBusy = false; });
  }
  return !!importArchive;
};
window.ironwoodBeginBatchImport = (archive = false) => {
  batchImport = { pending: 0, saved: 0, failed: 0, duplicates: 0, firstError: null, archive, storage: archive ? importArchive : null, queue: [] };
};
window.ironwoodBatchImportStatus = () => {
  const {pending = 0, saved = 0, failed = 0, duplicates = 0, firstError = null, retryMessage = ''} = batchImport || {};
  return JSON.stringify({pending, saved, failed, duplicates, firstError, retryMessage});
};
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

function librarySummary(record) {
  const {json,archiveAnalysisRuns,...summary}=record, details=gameDetails(json);
  return {...summary,...details,category:details.category==='puzzles'?'puzzles':details.trainingProfileId?'training':summary.category||details.category,source:summary.source,favorite:Boolean(summary.favorite),fingerprint:storedFingerprint(record)};
}
async function archiveLibrary(interactive=true) {
  if(typeof window.ironwoodCompanionLibrary!=='function')throw Error('Archive storage is unavailable. Reload Ironwood.');
  return window.ironwoodCompanionLibrary({details:gameDetails,prepare:window.ironwoodPrepareArchivedPgn,exporter:window.ironwoodExportSavedGame,receive:receiveCompanionDocuments,interactive});
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
      games.push(librarySummary(cursor.value));
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

// Explicit Companion transfers never change browser originals or profile ratings.
export async function companionBrowserRecords() { return readAllGameRecords(); }
export async function receiveCompanionDocuments(documents, {returnIds=false} = {}) {
  if (!Array.isArray(documents) || !documents.length || documents.length > 1000) throw new Error('Choose 1 to 1000 archived games.');
  const records = documents.map(document => {
    if (document?.format !== 'ironwood-companion-game' || document.version !== 1) throw new Error('Unsupported Companion game version.');
    const record = document.record ? structuredClone(document.record) : {
      id: newId(), json: window.ironwoodPrepareArchivedPgn(document.pgn), category: 'imported', updatedAt: Date.now(), favorite: false,
    };
    if(document.analysis_runs?.length)record.archiveAnalysisRuns=structuredClone(document.analysis_runs);
    if (typeof record.id !== 'string' || typeof record.json !== 'string') throw new Error('Invalid Companion game record.');
    gameDetails(record.json);
    return record;
  });
  const existing = await readAllGameRecords();
  const key = record => {
    const {id, updatedAt, ...content} = record;
    const canonical = value => Array.isArray(value) ? value.map(canonical) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])) : value;
    return JSON.stringify(canonical(content));
  };
  const known = new Set(existing.map(key)), incoming = [];
  for (const record of records) if (!known.has(key(record))) { known.add(key(record)); incoming.push(record); }
  if (incoming.length) await replaceOrMergeGames(incoming, false);
  if(!returnIds)return {received:incoming.length,duplicates:records.length-incoming.length};
  const committed=await readAllGameRecords(), destinations=new Map(committed.map(record=>[key(record),record.id]));
  const ids=records.map(record=>destinations.get(key(record)));
  if(ids.some(id=>!id))throw Error('Could not verify every browser destination. Archive originals were kept.');
  return { received: incoming.length, duplicates: records.length - incoming.length, ids };
}

const trainingNodeLevels = [1,4,16,64,256,1024,4096,16384,65536,262144];
function trainingEngine(value) {
  if (value == null || value.kind === 'stockfish') return {kind:'stockfish'};
  if (value.kind !== 'lc0' || typeof value.identity !== 'string' || typeof value.name !== 'string' || !value.name.trim()) throw Error('Invalid training engine');
  let address; try { address = new URL(value.identity); } catch { throw Error('Invalid training engine address'); }
  if (!['http:','https:'].includes(address.protocol) || address.username || address.password || address.search || address.hash || address.pathname === '/') throw Error('Invalid training engine address');
  const options = Object.entries(value.options || {}).sort(([a],[b])=>a.localeCompare(b));
  if (options.length > 64 || options.some(([name,text])=>name.length>256 || /[\r\n\0]/.test(name) || typeof text !== 'string' || text.length>4096 || /[\r\n\0]/.test(text))) throw Error('Invalid training engine options');
  return {kind:'lc0',identity:value.identity,name:value.name,options:Object.fromEntries(options)};
}
const sameTrainingEngine = (a,b) => JSON.stringify(trainingEngine(a)) === JSON.stringify(trainingEngine(b));
export function validateTrainingProfiles(value) {
  if (!value || !Array.isArray(value.profiles) || typeof value.selected_id !== 'string') throw new Error('Invalid training profiles');
  const ids = new Set();
  for (const profile of value.profiles) {
    if (!profile || typeof profile.id !== 'string' || !profile.id || ids.has(profile.id) ||
        typeof profile.name !== 'string' || !profile.name.trim() || !Array.isArray(profile.results)) throw new Error('Invalid training profile');
    ids.add(profile.id);
    const engine = trainingEngine(profile.engine), lc0 = engine.kind === 'lc0';
    if (lc0 && (!Number.isInteger(profile.starting_level) || profile.starting_level<1 || profile.starting_level>trainingNodeLevels.length)) throw Error('Invalid starting training level');
    if (profile.starting_elo != null && (!Number.isInteger(profile.starting_elo) || profile.starting_elo<1320 || profile.starting_elo>3190)) throw Error('Invalid starting training Elo');
    const sessions = new Set();
    for (const entry of profile.results) {
      const s = entry?.session;
      const expectedScore = entry?.result === '1/2-1/2' ? 0.5 :
        (entry?.result === '1-0' && s?.side === 'White') || (entry?.result === '0-1' && s?.side === 'Black') ? 1 : 0;
      if (!s || typeof s.id !== 'string' || !s.id || sessions.has(s.id) || s.profile_id !== profile.id ||
          typeof s.profile_name !== 'string' || !['White','Black'].includes(s.side) ||
          !sameTrainingEngine(profile.engine,s.engine) ||
          !Number.isInteger(s.opponent_elo) || (lc0 ? s.opponent_elo !== 0 : s.opponent_elo < 1320 || s.opponent_elo > 3190) ||
          (lc0 && (!Number.isInteger(s.difficulty_level) || s.difficulty_level<1 || s.difficulty_level>trainingNodeLevels.length || s.node_budget !== trainingNodeLevels[s.difficulty_level-1])) ||
          (lc0 && (s.rating_before<1 || s.rating_before>trainingNodeLevels.length || s.rating_after<1 || s.rating_after>trainingNodeLevels.length)) ||
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
    if (!sameTrainingEngine(target.engine,p.engine) || (target.starting_level || 1)!==(p.starting_level || 1) || (target.starting_elo || 1320)!==(p.starting_elo || 1320)) throw Error('Training profile engine or starting difficulty differs. Import it as a separate profile.');
    for (const entry of p.results) {
      const existing = target.results.find(r => r.session.id === entry.session.id);
      if (!existing) target.results.push(entry);
      else if ((entry.analysis?.phases.reduce((n,p) => n+p.moves,0) || 0) >
               (existing.analysis?.phases.reduce((n,p) => n+p.moves,0) || 0)) existing.analysis = entry.analysis;
    }
    target.results.sort((a,b) => a.session.completed_at.localeCompare(b.session.completed_at) || a.session.id.localeCompare(b.session.id));
    const lc0 = trainingEngine(target.engine).kind === 'lc0';
    const initial = lc0 ? target.starting_level : target.starting_elo || 1320;
    const ratings = { White: initial, Black: initial };
    for (const entry of target.results) {
      const s = entry.session;
      s.rating_before = ratings[s.side];
      const expected = 1/(1+10**((s.opponent_elo-s.rating_before)/400));
      const delta = 32*(entry.score-expected);
      // Rust rounds negative half-points away from zero.
      s.rating_after = lc0 ? Math.max(1,Math.min(trainingNodeLevels.length,s.rating_before+(entry.score===1?1:entry.score===0?-1:0))) : s.rating_before + Math.sign(delta)*Math.round(Math.abs(delta));
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
      category: gameDetails(record.json).category === 'puzzles' ? 'puzzles' : gameDetails(record.json).trainingProfileId ? 'training' : ['imported', 'observed', 'puzzles'].includes(record.category) ? record.category : 'mine',
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
    puzzles: backup.games.filter(game => game.category === 'puzzles').length,
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
    transaction.oncomplete = () => { notifyLibraryChanged('browser'); resolve(); };
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
    transaction.oncomplete = () => { notifyLibraryChanged('browser'); resolve(); };
    transaction.onerror = () => reject(transaction.error || new Error('Could not clear saved games'));
    transaction.onabort = () => reject(transaction.error || new Error('Clear was interrupted'));
  });
  localStorage.removeItem(CURRENT_GAME_KEY);
  localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
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

let storageGrainUrl;
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
      #storage-management-dialog.storage-information{width:min(620px,calc(100vw - 24px));padding:22px;background:#171a1e;border-radius:12px}
      #storage-management-dialog.storage-information .game-library-heading h2{margin:0;color:#ecebe4}
      #storage-management-dialog.storage-information .game-library-heading button{width:34px;height:34px;display:grid;place-items:center;padding:0;line-height:1;box-sizing:border-box;border:1px solid #887044;border-radius:5px;font-size:22px;background:linear-gradient(#30363c,#1d2329)}
      #storage-management-dialog.storage-information .game-library-title-group{display:flex;align-items:center;flex-wrap:wrap;gap:12px}
      #storage-management-dialog.storage-information .game-library-sources{display:flex;gap:6px}
      #storage-management-dialog.storage-information .game-library-heading .game-library-sources button{width:auto;height:auto;display:block;padding:8px 14px;border:1px solid;border-color:#77828a #11171c #11171c #77828a;border-radius:5px;background:linear-gradient(#48515a,#293139);color:#f0eee8;font:600 13px system-ui,sans-serif;box-shadow:inset 1px 1px 0 #ffffff26,inset -1px -1px 0 #0008,0 2px 2px #0008}
      #storage-management-dialog.storage-information .game-library-heading .game-library-sources button[aria-pressed=true]{background:linear-gradient(#606b74,#3a444c);outline:1px solid #d3ad62;outline-offset:1px}
      #storage-management-dialog.storage-information [hidden]{display:none!important}
      #storage-management-dialog .storage-info-intro{margin:10px 0 18px;color:#aeb7bd}
      #storage-management-dialog .storage-info-panel{margin-top:14px;padding:17px 18px;background:#0d1116;border:1px solid #30373c;border-top-color:#080b0e;border-left-color:#080b0e;border-bottom-color:#52595e;border-right-color:#52595e;border-radius:7px;box-shadow:inset 2px 3px 7px #0009,0 1px 0 #ffffff08}
      #storage-management-dialog .storage-info-panel h3{margin:0 0 13px;color:#d3ad62;font-size:12px;letter-spacing:.08em;text-transform:uppercase}
      #storage-management-dialog .storage-info-panel .storage-info-grid{gap:10px 24px}
      #storage-management-dialog .storage-info-panel .storage-info-grid strong{color:#aeb7bd;font-weight:400}
      #storage-management-dialog .storage-info-panel .storage-info-grid span{color:#ecebe4;font-variant-numeric:tabular-nums}
      #storage-management-dialog.storage-information{resize:both;min-width:min(440px,calc(100vw - 24px));min-height:min(340px,calc(100dvh - 24px));height:min(720px,calc(100dvh - 24px));max-width:calc(100vw - 24px);max-height:calc(100dvh - 24px);overflow:hidden}
      #storage-management-dialog.storage-information[open]{display:flex;flex-direction:column}
      #storage-management-dialog.storage-information .game-library-heading{flex:none;cursor:grab;touch-action:none;user-select:none}
      #storage-management-dialog.storage-information .game-library-heading.dragging{cursor:grabbing}
      #storage-management-dialog.storage-information .storage-info-intro{flex:none}
      #storage-management-dialog.storage-information .storage-info-toolbar{flex:none;justify-content:flex-start;flex-wrap:wrap;margin:0 0 12px}
      #storage-management-dialog.storage-information .storage-info-body{flex:1;min-height:0;overflow:auto;padding:0 4px 5px 0;scrollbar-gutter:stable}
      #storage-management-dialog.storage-information .storage-info-body .storage-info-panel:first-child{margin-top:0}
      #storage-management-dialog .storage-capacity{display:flex;align-items:center;gap:22px;margin:4px 0 20px}
      #storage-management-dialog .storage-pie{width:110px;height:110px;flex:0 0 110px;border-radius:50%;background:conic-gradient(#d3ad62 0 var(--used),#506675 var(--used) 100%);border:1px solid #73828a;box-shadow:0 4px 12px #0007}
      #storage-management-dialog .storage-capacity strong{display:block;font-size:22px;color:#ecebe4;font-variant-numeric:tabular-nums}
      #storage-management-dialog .storage-capacity p{margin:5px 0;color:#aeb7bd}
      #storage-management-dialog .storage-capacity-key{display:flex;gap:18px;margin-top:12px;color:#c2cbd0}
      #storage-management-dialog .storage-capacity-key span::before{content:'';display:inline-block;width:10px;height:10px;margin-right:7px;border-radius:2px;background:#d3ad62}
      #storage-management-dialog .storage-capacity-key span+span::before{background:#506675}
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
  if(dialog.classList.contains('storage-information')&&titleText!=='Storage information'){
    for(const property of ['position','margin','inset','left','top','width','height'])dialog.style.removeProperty(property);
  }
  dialog.classList.toggle('storage-information', titleText === 'Storage information');
  if (titleText === 'Storage information' && !storageGrainUrl) {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 256;
    const context = canvas.getContext('2d');
    if (context) {
      const pixels = context.createImageData(256, 256);
      let seed = 19741029;
      for (let i = 0; i < pixels.data.length; i += 4) {
        seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5;
        const light = seed & 1;
        pixels.data[i] = light ? 238 : 0;
        pixels.data[i + 1] = light ? 222 : 0;
        pixels.data[i + 2] = light ? 194 : 0;
        pixels.data[i + 3] = (seed >>> 8) % 18;
      }
      context.putImageData(pixels, 0, 0);
      storageGrainUrl = canvas.toDataURL();
    }
  }
  dialog.style.backgroundImage = titleText === 'Storage information' && storageGrainUrl ? `url("${storageGrainUrl}")` : '';
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

// Portable profile bundles keep stable profile/session IDs for safe repeated imports.
export function buildTrainingProfileExport(profiles, games, ids) {
  const validated = validateTrainingProfiles(profiles);
  const selected = new Set(ids);
  const chosen = validated.profiles.filter(profile => selected.has(profile.id));
  if (!chosen.length) throw new Error('Select at least one profile.');
  const chosenIds = new Set(chosen.map(profile => profile.id));
  return {
    format: 'ironwood-training-profiles', version: 1,
    exported_at: new Date().toISOString(),
    training_profiles: { selected_id: chosenIds.has(validated.selected_id) ? validated.selected_id : chosen[0].id, profiles: chosen },
    games: games.filter(game => chosenIds.has(JSON.parse(game.json).training?.profile_id)),
  };
}

export function validateTrainingProfileImport(raw) {
  if (raw?.format !== 'ironwood-training-profiles' || raw.version !== 1 || !raw.training_profiles) {
    throw new Error('Choose an Ironwood training profile export (.json).');
  }
  const backup = validateBackup({ ...raw, format: BACKUP_FORMAT });
  const ids = new Set(backup.trainingProfiles.profiles.map(profile => profile.id));
  if (!ids.has(backup.trainingProfiles.selected_id)) throw new Error('The export has no valid selected training profile.');
  if (!ids.size || backup.games.some(game => !ids.has(JSON.parse(game.json).training?.profile_id))) {
    throw new Error('The export contains games outside its training profiles.');
  }
  return backup;
}

export function removeTrainingProfiles(profiles, games, ids, deleteHistory) {
  const next = validateTrainingProfiles(profiles);
  const selected = new Set(ids);
  next.profiles = next.profiles.filter(profile => !selected.has(profile.id));
  if (!next.profiles.some(profile => profile.id === next.selected_id)) next.selected_id = next.profiles[0]?.id || '';
  return {
    profiles: next,
    deleteIds: deleteHistory ? games.filter(game => selected.has(JSON.parse(game.json).training?.profile_id)).map(game => game.id) : [],
  };
}

export function newTrainingGames(existing, incoming) {
  const sessions = new Set(existing.map(game => JSON.parse(game.json).training?.id).filter(Boolean));
  const ids = new Set(existing.map(game => game.id));
  return incoming.filter(game => {
    const session = JSON.parse(game.json).training?.id;
    if (session && sessions.has(session)) return false;
    if (!session && ids.has(game.id)) return false;
    if (session) sessions.add(session);
    return true;
  }).map(game => {
    let id = game.id;
    while (ids.has(id)) id = newId();
    ids.add(id);
    return { ...game, id };
  });
}

async function commitTrainingProfiles(profiles, games = [], deleteIds = []) {
  await pendingWrite;
  const db = await openDatabase();
  const previous = localStorage.getItem(TRAINING_KEY);
  await new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    for (const id of deleteIds) store.delete(id);
    for (const game of games) store.put(game);
    let saved = false;
    const rollback = () => {
      if (saved) {
        try { if (previous === null) localStorage.removeItem(TRAINING_KEY); else localStorage.setItem(TRAINING_KEY, previous); }
        catch (error) { console.error('Could not roll back profile storage:', error); }
      }
      reject(transaction.error || new Error('Profile update was interrupted.'));
    };
    transaction.oncomplete = () => { notifyLibraryChanged('browser'); resolve(); };
    transaction.onerror = rollback;
    transaction.onabort = rollback;
    try { localStorage.setItem(TRAINING_KEY, JSON.stringify(profiles)); saved = true; }
    catch (error) { transaction.abort(); reject(error); }
  });
}

let profileActionStatus = { busy: false };
window.ironwoodTrainingProfilesAction = (command, payload = '') => {
  if (command === 'poll') {
    const result = JSON.stringify(profileActionStatus);
    profileActionStatus.changed = false;
    return result;
  }
  if (profileActionStatus.busy) return JSON.stringify(profileActionStatus);
  profileActionStatus = { busy: true, message: 'Working…' };
  const run = async () => {
    if (command === 'import') {
      const file = await new Promise(resolve => {
        const input = document.createElement('input');
        input.type = 'file'; input.accept = '.json,application/json';
        input.addEventListener('change', () => resolve(input.files?.[0]), { once: true });
        input.addEventListener('cancel', () => resolve(null), { once: true });
        input.click();
      });
      if (!file) return { message: 'Import cancelled.' };
      if (file.size > 100 * 1024 * 1024) throw new Error('Profile export exceeds 100 MB.');
      const bundle = validateTrainingProfileImport(JSON.parse(await file.text()));
      const current = JSON.parse(localStorage.getItem(TRAINING_KEY) || 'null');
      const profiles = mergeTrainingProfiles(current, bundle.trainingProfiles);
      const games = newTrainingGames(await readAllGameRecords(), bundle.games);
      await commitTrainingProfiles(profiles, games);
      return { changed: true, message: `Imported ${bundle.trainingProfiles.profiles.length} profiles and ${games.length} new saved games. Existing results were merged by session ID.` };
    }
    const options = JSON.parse(payload);
    const profiles = validateTrainingProfiles(JSON.parse(localStorage.getItem(TRAINING_KEY) || '{"selected_id":"","profiles":[]}'));
    const games = await readAllGameRecords();
    if (command === 'export') {
      const bundle = buildTrainingProfileExport(profiles, games, options.ids);
      downloadJson(`ironwood-training-profiles-${new Date().toISOString().slice(0,10)}.json`, bundle);
      return { message: `Exported ${bundle.training_profiles.profiles.length} profiles with ${bundle.games.length} saved games.` };
    }
    if (command === 'delete') {
      const next = removeTrainingProfiles(profiles, games, options.ids, options.deleteGames);
      await commitTrainingProfiles(next.profiles, [], next.deleteIds);
      let current;
      try { current = JSON.parse(localStorage.getItem(CURRENT_GAME_KEY) || 'null'); } catch { /* A damaged board snapshot must not hide a completed deletion. */ }
      if (options.ids.includes(current?.training?.profile_id)) {
        localStorage.removeItem(CURRENT_GAME_KEY);
        localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
        localStorage.removeItem(ACTIVE_ID_KEY);
        localStorage.removeItem(ACTIVE_CATEGORY_KEY);
      }
      return { changed: true, message: options.deleteGames ? `Profiles and ${next.deleteIds.length} saved games deleted.` : 'Profiles deleted. Saved game history kept.' };
    }
    throw new Error('Unknown profile action.');
  };
  // Start file picking synchronously, preserving the user's click activation.
  void run().then(result => { profileActionStatus = { busy: false, ...result }; })
    .catch(error => { profileActionStatus = { busy: false, error: error.message || 'Profile operation failed.' }; });
  return JSON.stringify(profileActionStatus);
};

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
      message.textContent = `${file.name} contains ${summary.games} games (${summary.mine} personal, ${summary.imported} imported, ${summary.observed} observed, ${summary.training} training, ${summary.puzzles} puzzles, ${summary.favorites} favorites)${summary.hasPreferences ? ' and display preferences' : ''}.`;
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
            localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
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
  const heading=dialog.querySelector('.game-library-heading'),title=heading.querySelector('h2');
  const titleGroup=document.createElement('div');titleGroup.className='game-library-title-group';
  const tabs=document.createElement('div');tabs.className='game-library-sources';tabs.setAttribute('role','tablist');tabs.setAttribute('aria-label','Storage source');
  const browserTab=document.createElement('button'),archiveTab=document.createElement('button');
  for(const [tab,name] of [[browserTab,'Browser'],[archiveTab,'Archive']]){
    tab.type='button';tab.textContent=name;tab.id=`storage-info-${name.toLowerCase()}-tab`;tab.setAttribute('role','tab');tab.setAttribute('aria-controls',`storage-info-${name.toLowerCase()}-panel`);tabs.append(tab);
  }
  titleGroup.append(title,tabs);heading.prepend(titleGroup);
  heading.title='Drag to move';let drag=null;
  const stopDragging=()=>{drag=null;heading.classList.remove('dragging');};
  heading.addEventListener('pointerdown',event=>{
    if(event.button!==0||event.target.closest('button,input,select,a'))return;
    const rect=dialog.getBoundingClientRect();
    drag={pointerId:event.pointerId,x:event.clientX,y:event.clientY,left:rect.left,top:rect.top,width:rect.width,height:rect.height};
    Object.assign(dialog.style,{position:'fixed',margin:'0',inset:'auto',left:`${rect.left}px`,top:`${rect.top}px`,width:`${rect.width}px`,height:`${rect.height}px`});
    heading.classList.add('dragging');heading.setPointerCapture?.(event.pointerId);event.preventDefault();
  });
  heading.addEventListener('pointermove',event=>{
    if(!drag||drag.pointerId!==event.pointerId)return;
    dialog.style.left=`${Math.max(0,Math.min(window.innerWidth-drag.width,drag.left+event.clientX-drag.x))}px`;
    dialog.style.top=`${Math.max(0,Math.min(window.innerHeight-drag.height,drag.top+event.clientY-drag.y))}px`;
  });
  for(const type of ['pointerup','pointercancel','lostpointercapture'])heading.addEventListener(type,stopDragging);
  const keepVisible=()=>{
    if(dialog.style.position!=='fixed')return;
    const rect=dialog.getBoundingClientRect();
    dialog.style.left=`${Math.max(0,Math.min(window.innerWidth-rect.width,rect.left))}px`;
    dialog.style.top=`${Math.max(0,Math.min(window.innerHeight-rect.height,rect.top))}px`;
  };
  const resizeObserver=window.ResizeObserver?new window.ResizeObserver(keepVisible):null;
  resizeObserver?.observe(dialog);window.addEventListener('resize',keepVisible);
  dialog.addEventListener('close',()=>{stopDragging();resizeObserver?.disconnect();window.removeEventListener('resize',keepVisible);},{once:true});

  let selected='browser',serverLoaded=false;

  const intro = document.createElement('p');
  intro.className = 'storage-info-intro';
  intro.textContent = 'Your game library and browser storage on this device.';
  dialog.append(intro);
  const content = document.createElement('div');
  content.id='storage-info-browser-panel';content.setAttribute('role','tabpanel');content.setAttribute('aria-labelledby',browserTab.id);
  content.textContent = 'Checking storage…';
  const archiveToolbar=document.createElement('div');archiveToolbar.className='storage-actions storage-info-toolbar';
  const body=document.createElement('div');body.className='storage-info-body';body.append(content);
  dialog.append(archiveToolbar,body);
  const serverPanel=document.createElement('section');serverPanel.className='storage-info-panel';
  serverPanel.id='storage-info-archive-panel';serverPanel.setAttribute('role','tabpanel');serverPanel.setAttribute('aria-labelledby',archiveTab.id);
  let serverEpoch=0;
  const serverEnabled=()=>window.ironwoodCompanionFeatureEnabled?.()===true;
  async function refreshServer(interactive=false) {
    const epoch=++serverEpoch;serverLoaded=false;
    if(!serverEnabled()||selected!=='archive'||!dialog.contains(serverPanel))return;
    serverPanel.replaceChildren();
    const heading=document.createElement('h3');heading.textContent='Server archive';serverPanel.append(heading);
    const status=document.createElement('p');status.setAttribute('role','status');status.textContent='Checking server storage…';serverPanel.append(status);
    const actions=document.createElement('div');actions.className='storage-actions';
    const connect=document.createElement('button');connect.textContent='Manage connection';connect.type='button';connect.onclick=async()=>{try{if(typeof window.ironwoodManageCompanionConnection==='function')await window.ironwoodManageCompanionConnection();else await archiveLibrary(true);await refreshServer(false);}catch(error){status.textContent=error.message;}};
    const refresh=document.createElement('button');refresh.type='button';refresh.textContent='Refresh server';refresh.onclick=()=>void refreshServer(false);actions.append(connect,refresh);archiveToolbar.replaceChildren(...actions.childNodes);
    try {
      const storage=await archiveLibrary(interactive),stats=await storage.stats();
      if(epoch!==serverEpoch||!dialog.open||!serverEnabled()||selected!=='archive')return;
      serverLoaded=true;
      status.textContent=`Connected to ${storage.endpoint}`;
      const format=value=>typeof value==='number'?value<1048576?`${(value/1024).toFixed(1)} KiB`:`${(value/1048576).toFixed(1)} MiB`:'Unavailable';
      const count=value=>Number.isFinite(value)?String(value):'Unavailable';
      const grid=document.createElement('div');grid.className='storage-info-grid';
      const rows=[['Companion version',stats.server_version||'Unavailable'],['Database',`SQLite · schema ${stats.schema_version}`],['Saved games',count(stats.games)],
        ['My games',count(stats.categories?.mine??0)],['Imported games',count(stats.categories?.imported??0)],['Observed games',count(stats.categories?.observed??0)],['Training games',count(stats.categories?.training??0)],['Puzzles',count(stats.categories?.puzzles??0)],['Favorites',count(stats.favorites)],['Collections',count(stats.collections)],
        ['Database file',format(stats.database_bytes)],['Write-ahead log',format(stats.wal_bytes)],['Shared-memory file',format(stats.shared_memory_bytes)],['Total archive files',format(stats.total_bytes)],['SQLite allocation',format(stats.allocated_bytes)],['Reusable SQLite space',format(stats.reusable_bytes)],
        ['Opening index',`${count(stats.index?.indexed)} / ${count(stats.index?.total)} games`],['Pending indexing',count(stats.index?.pending)],['Indexing failures',count(stats.index?.failed)],['Other indexed variants',count(stats.index?.unsupported_variants)]];
      for(const [label,value]of rows){const term=document.createElement('strong'),detail=document.createElement('span');term.textContent=label;detail.textContent=value;grid.append(term,detail);}
      serverPanel.append(grid);
      const scope=document.createElement('p');scope.className='storage-guidance';scope.textContent='Archive database files only. Reusable SQLite space is inside the database, not free disk space. Engines and tablebase files are not included.';serverPanel.append(scope);
    }catch(error){if(epoch===serverEpoch&&dialog.open&&serverEnabled()&&selected==='archive')status.textContent=error.status===404?'Server storage statistics require an updated Companion build.':`Server storage is unavailable: ${error.message}`;}
  }
  function updateTabs(){
    if(!serverEnabled())selected='browser';
    tabs.hidden=!serverEnabled();content.hidden=selected!=='browser';serverPanel.hidden=selected!=='archive';archiveToolbar.hidden=selected!=='archive';
    intro.textContent=selected==='browser'?'Your game library and browser storage on this device.':'Storage used by your connected server archive.';
    for(const [tab,value] of [[browserTab,'browser'],[archiveTab,'archive']]){
      const active=selected===value;tab.setAttribute('aria-selected',String(active));tab.setAttribute('aria-pressed',String(active));tab.tabIndex=active?0:-1;
    }
  }
  function chooseTab(value){
    if(value==='archive'&&!serverEnabled())return;
    if(selected!==value)serverEpoch++;
    selected=value;updateTabs();
    if(value==='archive'&&!serverLoaded)void refreshServer(false);
  }
  browserTab.onclick=()=>chooseTab('browser');archiveTab.onclick=()=>chooseTab('archive');
  tabs.addEventListener('keydown',event=>{
    if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;
    event.preventDefault();const value=event.key==='Home'?'browser':event.key==='End'?'archive':selected==='browser'?'archive':'browser';
    chooseTab(value);(value==='browser'?browserTab:archiveTab).focus();
  });
  const updateServer=()=>{
    serverEpoch++;serverLoaded=false;
    if(!serverEnabled())serverPanel.replaceChildren();
    updateTabs();if(selected==='archive')void refreshServer(false);
  };
  body.append(serverPanel);updateTabs();
  window.addEventListener('ironwood-companion-feature-changed',updateServer);
  window.addEventListener('ironwood-companion-connection-changed',updateServer);
  dialog.addEventListener('close',()=>{serverEpoch++;window.removeEventListener('ironwood-companion-feature-changed',updateServer);window.removeEventListener('ironwood-companion-connection-changed',updateServer);},{once:true});
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
      ['Puzzles', String(games.filter(game => game.category === 'puzzles').length)],
      ['Favorites', String(games.filter(game => game.favorite).length)],
      ['Browser usage', format(estimate?.usage)],
      ['Browser quota', format(estimate?.quota)],
      ['Persistent storage', persisted === true ? 'Granted' : persisted === false ? 'Not granted' : 'Unavailable'],
      ['Offline engine', 'Managed separately by the offline-play control'],
    ];
    content.replaceChildren();
    for (const [title, start, end] of [['Game library', 0, 8], ['Browser storage', 8, 11], ['Offline play', 11, 12]]) {
      const panel = document.createElement('section');
      panel.className = 'storage-info-panel';
      const heading = document.createElement('h3');
      heading.textContent = title;
      const grid = document.createElement('div');
      grid.className = 'storage-info-grid';
      for (const [label, value] of rows.slice(start, end)) {
      const term = document.createElement('strong');
      term.textContent = label;
      const detail = document.createElement('span');
      detail.textContent = value;
        grid.append(term, detail);
      }
      panel.append(heading);
      if (title === 'Browser storage' && Number.isFinite(estimate?.usage) && Number.isFinite(estimate?.quota) && estimate.quota > 0) {
        const percent = Math.max(0, Math.min(100, estimate.usage / estimate.quota * 100));
        const available = Math.max(0, estimate.quota - estimate.usage);
        const capacity = document.createElement('div');
        capacity.className = 'storage-capacity';
        const pie = document.createElement('div');
        pie.className = 'storage-pie';
        pie.style.setProperty('--used', `${percent}%`);
        pie.setAttribute('role', 'img');
        const percentage = percent > 0 && percent < 0.01 ? '<0.01' : percent.toFixed(2);
        pie.setAttribute('aria-label', `Browser storage: ${percentage}% used, ${format(available)} available within the browser quota`);
        const summary = document.createElement('div');
        const amount = document.createElement('strong');
        amount.textContent = `${percentage}% used`;
        const remaining = document.createElement('p');
        remaining.textContent = `${format(available)} available`;
        const key = document.createElement('div');
        key.className = 'storage-capacity-key';
        for (const label of ['Used', 'Available']) {
          const item = document.createElement('span'); item.textContent = label; key.append(item);
        }
        summary.append(amount, remaining, key);
        capacity.append(pie, summary);
        panel.append(capacity);
      }
      panel.append(grid);
      content.append(panel);
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
    clearTimeout(analysisSyncTimer);await localAnalysisWrites;await analysisSyncPromise?.catch(()=>{});
    const analysisDb=await analysisLocalCache.database?.catch(()=>null);analysisDb?.close();analysisLocalCache.database=null;
    if(globalThis.indexedDB)await new Promise((resolve,reject)=>{const request=indexedDB.deleteDatabase('ironwood-analysis-cache');request.onsuccess=()=>resolve();request.onerror=()=>reject(request.error);request.onblocked=()=>reject(Error('Close other Ironwood tabs before resetting local analysis data.'));});
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
      transaction.oncomplete = () => { notifyLibraryChanged('browser'); resolve(); };
      transaction.onerror = () => reject(transaction.error || new Error('Could not save game'));
      transaction.onabort = () => reject(transaction.error || new Error('Game save was interrupted'));
    });
  });
  pendingWrite.catch(error => console.warn('Ironwood game library save failed:', error));
  return pendingWrite;
}

export async function deleteUnchangedBrowserRecords(records) {
  await pendingWrite;
  const db=await openDatabase();
  await new Promise((resolve,reject)=>{
    const tx=db.transaction(STORE_NAME,'readwrite'),store=tx.objectStore(STORE_NAME);let failure;
    tx.oncomplete=()=>{notifyLibraryChanged('browser',{deletedIds:records.map(record=>record.id)});resolve();};tx.onerror=tx.onabort=()=>reject(failure||tx.error||Error('Browser removal failed. Archive copies were kept.'));
    for(const record of records){const get=store.get(record.id);get.onsuccess=()=>{
      if(failure)return;
      if(JSON.stringify(get.result)!==JSON.stringify(record)){failure=Error('A browser game changed during transfer. Browser originals were kept.');tx.abort();return;}
      store.delete(record.id);
    };}
  });
}
function getArchiveActive() {try{return JSON.parse(localStorage.getItem(ACTIVE_ARCHIVE_KEY)||'null');}catch{return null;}}
function setArchiveActive(record) {
  const archive=record._archive;
  localStorage.setItem(ACTIVE_ARCHIVE_KEY,JSON.stringify({endpoint:archive.endpoint,id:archive.id,revision:archive.revision}));
  localStorage.setItem(ACTIVE_ID_KEY,archive.id);
}
async function deleteGames(ids) {
  await pendingWrite;
  const db = await openDatabase();
  await new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    for (const id of ids) store.delete(id);
    transaction.oncomplete = () => { notifyLibraryChanged('browser'); resolve(); };
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
  const puzzle = pgnTag(game.review_pgn, 'Event') === 'Lichess puzzle';
  const puzzleId = puzzle ? pgnTag(game.review_pgn, 'PuzzleId') : null;
  // Preserve historical opponent names for games saved before the engine was removed.
  const training = game.training;
  const opponent = training ? training.engine?.kind === 'lc0' ? `${training.engine.name} · Level ${training.difficulty_level} · ${training.node_budget} nodes` : `Stockfish ${training.opponent_elo} Elo` : game.engine_config?.opponent === 'Companion' ? game.engine_config.companion_name || 'Companion engine' : game.engine_config?.opponent === 'Lc0' ? 'Lc0 · Good Gyal' : 'Stockfish 19';
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
  const moves = game.live_moves?.length || pgnMoveCount(game.review_pgn);
  const analyzed = game.game_analysis?.filter(Boolean).length || 0;
  const analysisStatus = analyzed === 0 ? 'Not analyzed' : analyzed >= moves + 1 ? 'Complete' : 'Partial';
  const whiteFideId = pgnTag(game.review_pgn, 'WhiteFideId') || null;
  const blackFideId = pgnTag(game.review_pgn, 'BlackFideId') || null;
  const startingNote = typeof game.move_notes?.[0] === 'string' ? game.move_notes[0].trim() : '';
  return { startingNote, analysisStatus, puzzleId, title: puzzle ? `Lichess puzzle${puzzleId ? ` ${puzzleId}` : ''}` : `${white} vs ${black}`, white, black, whiteFideId, blackFideId, playedAt, venue, result, moves, analyzed,
    trainingProfileId: training?.profile_id || null, trainingProfileName: profileName, trainingSide: training?.side || null,
    trainingOpponentElo: training?.opponent_elo, trainingEngineKind:training?.engine?.kind || 'stockfish', ratingBefore: training?.rating_before, ratingAfter: training?.rating_after,
    finalFen: game.final_board || game.board, category: puzzle ? 'puzzles' : training ? 'training' : imported ? 'imported' : 'mine' };
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

export function sortSavedGames(games, order = 'recent') {
  return [...games].sort((a, b) => {
    const moves = (a.moves || 0) - (b.moves || 0);
    if (order === 'moves-asc' && moves) return moves;
    if (order === 'moves-desc' && moves) return -moves;
    return (b.updatedAt || 0) - (a.updatedAt || 0) || String(a.id).localeCompare(String(b.id));
  });
}

export function gameMatchesCategory(game, category) {
  return category === 'all' || (category === 'favorites' ? Boolean(game.favorite) : game.category === category);
}

async function flushArchiveImport(batch) {
  if(batch.flushing||!batch.queue.length)return;
  clearTimeout(batch.timer);batch.timer=null;batch.flushing=true;
  const records=batch.queue.splice(0,32);let accounted=0;
  try {
    if(!batch.storage||window.ironwoodCompanionFeatureEnabled?.()!==true)throw Error('Companion archive is unavailable. Reconnect before importing.');
    await batch.storage.importBatchRecords(records,(result,count)=>{
      batch.saved+=result.imported;batch.duplicates+=result.duplicates;batch.pending-=count;accounted+=count;
    },message=>{batch.retryMessage=message;});
  } catch(error) {
    const remaining=records.length-accounted;batch.failed+=remaining;batch.pending-=remaining;
    batch.retryMessage='';
    batch.firstError ||= error.message||String(error);
  } finally {
    batch.flushing=false;
    if(batch.queue.length)batch.timer=setTimeout(()=>void flushArchiveImport(batch),0);
  }
}
function queueGameRecord(json, id, category, trackBatch = false, source) {
  const batch = trackBatch ? batchImport : null;
  try {
    const details = gameDetails(json);
    // Session restoration is separate; only played games belong in the library.
    if (details.moves === 0) return;
    const fingerprint = importFingerprint(JSON.parse(json).review_pgn);
    if (batch?.archive) {
      batch.pending++;
      batch.queue.push({id,...details,category,updatedAt:Date.now(),json});
      if(batch.queue.length>=32)void flushArchiveImport(batch);
      else if(!batch.timer)batch.timer=setTimeout(()=>{batch.timer=null;void flushArchiveImport(batch);},500);
      return;
    }
    if (fingerprint) importedFingerprints.add(fingerprint);
    if (batch) batch.pending++;
    const write = putGame({ id, ...details, category: details.category === 'puzzles' ? 'puzzles' : category, fingerprint, ...(source ? { source } : {}), updatedAt: Date.now(), json });
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

// Device-local payload cache and durable outbox. Credentials never enter this DB.
export const analysisLocalCache={
  database:null,
  async open(){
    if(!globalThis.indexedDB)throw Error('Browser storage is unavailable; this analysis is only in the current board snapshot.');
    if(!this.database)this.database=new Promise((resolve,reject)=>{const request=indexedDB.open('ironwood-analysis-cache',1);request.onupgradeneeded=()=>{const store=request.result.createObjectStore('entries',{keyPath:'id'});store.createIndex('game','game');store.createIndex('kind','kind');};request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);}).catch(error=>{this.database=null;throw error;});
    return this.database;
  },
  async get(id){const db=await this.open();return new Promise((resolve,reject)=>{const request=db.transaction('entries').objectStore('entries').get(id);request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});},
  async list(index,value){const db=await this.open();return new Promise((resolve,reject)=>{const request=db.transaction('entries').objectStore('entries').index(index).getAll(value);request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});},
  async write(put=[],remove=[]){const db=await this.open();return new Promise((resolve,reject)=>{const tx=db.transaction('entries','readwrite'),store=tx.objectStore('entries');put.forEach(value=>store.put(value));remove.forEach(id=>store.delete(id));tx.oncomplete=()=>resolve();tx.onerror=tx.onabort=()=>reject(tx.error||Error('Local analysis save failed.'));});}
  ,async mutate(changes){const db=await this.open();return new Promise((resolve,reject)=>{const tx=db.transaction('entries','readwrite'),store=tx.objectStore('entries');for(const [id,change]of changes){const request=store.get(id);request.onsuccess=()=>{try{const value=change(request.result);value?store.put(value):store.delete(id);}catch(error){tx.abort();reject(error);}};}tx.oncomplete=()=>resolve();tx.onerror=tx.onabort=()=>reject(tx.error||Error('Local analysis save failed.'));});}
};
const runCacheId=(key,id)=>`${key}/run/${id}`;
const locallyCreatedAnalysisRuns=new Set();
const runLabel=run=>{const date=new Date(run.created_at);return `${run.metadata.engine_name}; ${date.toLocaleDateString()}; ${date.toLocaleTimeString()}; ${run.metadata.status||'Partial'}`;};
function localRun(saved){
  const context=saved.analysis_run;if(!context)return null;
  const {id,position_fens,created_at,...details}=context,payload=archiveAnalysisSnapshot(saved),completed=(saved.game_analysis||[]).filter(Boolean).length;
  return {id,created_at:created_at||Date.now(),metadata:{...details,completed_positions:completed,total_positions:position_fens?.length||saved.game_analysis.length,status:completed>0&&completed===(position_fens?.length||saved.game_analysis.length)?'Complete':'Partial'},payload};
}
let localAnalysisWrites=Promise.resolve(),analysisSyncPromise=null,analysisSyncTimer=null,analysisSyncStarted=false;
async function cacheOpenedRun(key,run){
  const id=runCacheId(key,run.id);await analysisLocalCache.mutate([[id,existing=>existing?.pending?existing:{id,game:key,kind:'run',run,pending:false}]]);
}
async function cachedRunList(key,offline=false){
  const entries=await analysisLocalCache.list('game',key),catalog=entries.find(entry=>entry.kind==='catalog')?.runs||[];
  const map=new Map((offline?[]:catalog).map(run=>[run.id,run]));
  for(const entry of entries.filter(entry=>entry.kind==='run'))if(offline||entry.pending||!map.has(entry.run.id))map.set(entry.run.id,{...entry.run,payload:undefined,cached:true,pending:entry.pending});
  return [...map.values()].sort((a,b)=>b.created_at-a.created_at).map(run=>({...run,label:runLabel(run)}));
}
export async function queueArchiveAnalysis(game,json){
  const key=runGameKey(game),saved=JSON.parse(json),run=localRun(saved),changes=[];
  if(run&&!deletedAnalysisRuns.has(`${key}/${run.id}`)){
    const id=runCacheId(key,run.id);changes.push([id,existing=>{
      if(existing&&analysisSignature(existing.run.payload)===analysisSignature(run.payload)&&analysisSignature(existing.run.metadata)===analysisSignature(run.metadata))return existing;
      return {id,game:key,kind:'run',pending:true,run:{...run,revision:existing?.run.revision??runWrites.get(`${key}/${run.id}`)?.revision??(locallyCreatedAnalysisRuns.has(run.id)?0:null)}};
    }]);
  }
  const stamp=crypto.randomUUID();changes.push([`${key}/pending`,queue=>({id:`${key}/pending`,game:key,kind:'pending-game',endpoint:game.endpoint,gameId:game.id,baseRevision:queue?.baseRevision??(runGameKey(getArchiveActive())===key?Math.max(game.revision,getArchiveActive().revision):game.revision),json,stamp})]);
  await analysisLocalCache.mutate(changes);
  if(archiveRuns.key===key){archiveRuns.cache_error='';archiveRuns.pending=true;archiveRuns.runs=await cachedRunList(key,archiveRuns.offline);}
}
export async function syncArchiveAnalyses(storage,queue){
  storage.check();if(storage.endpoint!==queue.endpoint)return;
  const key=queue.game,entries=await analysisLocalCache.list('game',key);
  for(const entry of entries.filter(entry=>entry.kind==='run'&&entry.pending)){
    let remote=null;try{remote=await storage.request(`archive/games/${queue.gameId}/analysis-runs/${encodeURIComponent(entry.run.id)}`);}catch(error){if(error.status!==404)throw error;}
    const same=remote&&analysisSignature({metadata:remote.metadata,payload:remote.payload})===analysisSignature({metadata:entry.run.metadata,payload:entry.run.payload});
    if(!remote&&entry.run.revision===null)throw Object.assign(Error('The original archived run could not be verified. Your local analysis is preserved; reopen the game before syncing.'),{status:409});
    if(!same&&remote&&(entry.run.revision===null||remote.revision!==entry.run.revision))throw Object.assign(Error('This analysis changed in the archive. Your local results are preserved; reopen the archive game to resolve the conflict.'),{status:409});
    const saved=same?remote:await storage.request(`archive/games/${queue.gameId}/analysis-runs/${encodeURIComponent(entry.run.id)}`,{expected_revision:entry.run.revision??0,metadata:entry.run.metadata,payload:entry.run.payload});
    await analysisLocalCache.mutate([[entry.id,latest=>{if(!latest?.pending)return latest;const unchanged=analysisSignature(latest.run.payload)===analysisSignature(entry.run.payload)&&analysisSignature(latest.run.metadata)===analysisSignature(entry.run.metadata);return {...latest,pending:!unchanged,run:{...latest.run,revision:saved.revision}};}]]);
    runWrites.set(`${key}/${saved.id}`,{revision:saved.revision,signature:analysisSignature({metadata:saved.metadata,payload:saved.payload})});
  }
  // A game revision protects annotations independently of analysis-run revisions.
  const record=storage.record(await storage.request(`archive/games/${queue.gameId}`));
  let gameRevision=record._archive.revision;
  if(record.json!==queue.json){
    if(record._archive.revision!==queue.baseRevision)throw Object.assign(Error('The archive game changed elsewhere. Your local analysis is preserved; reopen it before syncing game edits.'),{status:409});
    const previous=JSON.parse(record.json);if(previous.game_analysis?.some(Boolean)&&!previous.analysis_run)await preserveArchiveAnalysis(storage,queue.gameId,previous,{...JSON.parse(queue.json),analysis_run:null});
    const updated=await storage.write({...record,json:queue.json,updatedAt:Date.now()});
    gameRevision=updated._archive.revision;
    if(runGameKey(getArchiveActive())===key)setArchiveActive(updated);
  }else if(runGameKey(getArchiveActive())===key)setArchiveActive(record);
  await analysisLocalCache.mutate([[queue.id,latest=>latest?.stamp===queue.stamp?null:latest?{...latest,baseRevision:gameRevision}:null]]);
}
async function flushAnalysisQueue(){
  if(window.ironwoodCompanionFeatureEnabled?.()!==true)return;
  if(analysisSyncPromise)return analysisSyncPromise;
  analysisSyncPromise=(async()=>{await localAnalysisWrites;const storage=await archiveLibrary(false),queues=await analysisLocalCache.list('kind','pending-game');
    for(const queue of queues){if(queue.endpoint!==storage.endpoint)continue;try{await syncArchiveAnalyses(storage,queue);if(archiveRuns.key===queue.game){archiveRuns.offline=false;archiveRuns.save_error='';archiveRuns.error='';archiveRuns.pending=Boolean(await analysisLocalCache.get(queue.id));}}catch(error){if(archiveRuns.key===queue.game){archiveRuns.offline=!error.status;archiveRuns.save_error=error.status?error.message:'';archiveRuns.pending=true;}throw error;}}
  })().finally(()=>{analysisSyncPromise=null;});
  return analysisSyncPromise;
}
function scheduleAnalysisSync(){clearTimeout(analysisSyncTimer);analysisSyncTimer=setTimeout(()=>void flushAnalysisQueue().catch(()=>{}),800);analysisSyncTimer.unref?.();}
function startAnalysisSync(){if(analysisSyncStarted)return;analysisSyncStarted=true;window.addEventListener('online',scheduleAnalysisSync);window.addEventListener('ironwood-companion-connection-changed',scheduleAnalysisSync);window.addEventListener('ironwood-companion-feature-changed',scheduleAnalysisSync);const timer=setInterval(()=>{if(window.ironwoodCompanionFeatureEnabled?.()===true){void flushAnalysisQueue().catch(()=>{});void refreshArchiveRuns();}},15000);timer.unref?.();scheduleAnalysisSync();}

// Metadata stays in memory; opened payloads and pending saves survive reloads.
let archiveRuns={key:'',runs:[],busy:false,error:'',save_error:'',generation:0},pendingRun=null;
function analysisSignature(value){
  const canonical=item=>Array.isArray(item)?item.map(canonical):item&&typeof item==='object'?Object.fromEntries(Object.keys(item).sort().map(key=>[key,canonical(item[key])])):item;
  return JSON.stringify(canonical(value));
}
const runWrites=new Map();
const deletedAnalysisRuns=new Set();
function clearedAnalysis(saved){return {...archiveAnalysisSnapshot(saved),analysis_run:null,game_analysis:(saved.game_analysis||[]).map(()=>null),verification_targets:[],verification_results:[],verification_nodes:0,verification_time_ms:0};}
const resetArchiveRuns=()=>{archiveRuns={key:'',runs:[],busy:false,error:'',save_error:'',generation:archiveRuns.generation+1};pendingRun=null;runWrites.clear();};
window.addEventListener?.('ironwood-companion-connection-changed',resetArchiveRuns);
window.addEventListener?.('ironwood-companion-feature-changed',()=>{if(window.ironwoodCompanionFeatureEnabled?.()!==true)resetArchiveRuns();});

function activeRunGame(){const game=getArchiveActive();return game&&window.ironwoodCompanionFeatureEnabled?.()===true?game:null;}
function runGameKey(game){return game?`${game.endpoint}/${game.id}`:'';}
function runState(){
  const game=activeRunGame(),key=runGameKey(game);
  if(key!==archiveRuns.key){archiveRuns={key,runs:[],busy:false,error:'',save_error:'',generation:archiveRuns.generation+1};pendingRun=null;runWrites.clear();if(key)void refreshArchiveRuns();}
  return game;
}
async function cacheGameAnalysisRuns(storage,game,runs,generation){
  const key=runGameKey(game);
  const current=()=>archiveRuns.key===key&&archiveRuns.generation===generation&&runGameKey(activeRunGame())===key;
  if(!current()||archiveRuns.caching)return;
  archiveRuns.caching=true;archiveRuns.cache_total=runs.length;archiveRuns.cache_done=0;
  try{
    // One request at a time keeps background caching out of the interactive path.
    for(const summary of runs){
      if(!current())return;
      if(deletedAnalysisRuns.has(`${key}/${summary.id}`))continue;
      let cached;try{cached=await analysisLocalCache.get(runCacheId(key,summary.id));}catch(error){archiveRuns.cache_error=error.message;return;}
      if(!current())return;
      if(!cached?.pending&&cached?.run.revision!==summary.revision){
        let run;try{storage.check();run=await storage.request(`archive/games/${game.id}/analysis-runs/${encodeURIComponent(summary.id)}`);}
        catch(error){if(!current())return;if(error.status===404)continue;archiveRuns.offline=!error.status;archiveRuns.error=error.status?error.message:'';return;}
        if(!current())return;
        if(!deletedAnalysisRuns.has(`${key}/${summary.id}`)){
          try{await cacheOpenedRun(key,run);}catch(error){archiveRuns.cache_error=error.message;return;}
        }
      }
      if(current())archiveRuns.cache_done++;
    }
  }finally{
    if(current()){archiveRuns.caching=false;try{archiveRuns.runs=await cachedRunList(key,archiveRuns.offline);}catch(error){archiveRuns.cache_error=error.message;}}
  }
}
async function refreshArchiveRuns(){
  const game=activeRunGame();if(!game)return;
  const key=runGameKey(game),generation=archiveRuns.generation;
  if(archiveRuns.refreshing)return;archiveRuns.refreshing=true;
  try{const runs=await cachedRunList(key,archiveRuns.offline),pending=Boolean(await analysisLocalCache.get(`${key}/pending`));if(archiveRuns.key!==key||generation!==archiveRuns.generation)return;archiveRuns.runs=runs;archiveRuns.pending=pending;}catch(error){if(archiveRuns.key===key&&generation===archiveRuns.generation)archiveRuns.cache_error=error.message;}
  try {await flushAnalysisQueue();const storage=await archiveLibrary(false);storage.check();if(storage.endpoint!==game.endpoint)throw Error('Reconnect to this game’s archive.');
    const result=await storage.request(`archive/games/${game.id}/analysis-runs`);
    if(archiveRuns.key!==key||generation!==archiveRuns.generation||runGameKey(activeRunGame())!==key)return;
    archiveRuns.runs=result.runs.map(run=>({...run,label:runLabel(run)}));archiveRuns.error='';archiveRuns.offline=false;
    try{
      const entries=await analysisLocalCache.list('game',key),remote=new Map(result.runs.map(run=>[run.id,run]));
      const remove=entries.filter(entry=>entry.kind==='run'&&!entry.pending&&(!remote.has(entry.run.id)||remote.get(entry.run.id).revision!==entry.run.revision)).map(entry=>entry.id);
      await analysisLocalCache.write([{id:`${key}/catalog`,game:key,kind:'catalog',runs:result.runs}],remove);
      archiveRuns.runs=await cachedRunList(key);
      void cacheGameAnalysisRuns(storage,game,result.runs,generation);
    }catch(error){archiveRuns.cache_error=error.message;}
  }catch(error){if(archiveRuns.key===key){archiveRuns.offline=!error.status;archiveRuns.error=error.status===404?'Update Companion to save multiple analysis runs.':error.status?error.message:'';try{archiveRuns.runs=await cachedRunList(key,true);}catch{};}}
  finally{if(archiveRuns.key===key&&generation===archiveRuns.generation)archiveRuns.refreshing=false;}
}
window.ironwoodArchiveAnalysisRuns=()=>{const game=runState();const message=archiveRuns.offline?`Companion offline — showing analyses saved on this device.${archiveRuns.pending?' New results are saved locally and will sync when Companion returns.':''}`:archiveRuns.pending?'Analysis saved locally — waiting to sync with Companion.':archiveRuns.caching?`Caching analyses for offline use… ${archiveRuns.cache_done} of ${archiveRuns.cache_total}`:'';return JSON.stringify({available:Boolean(game),...archiveRuns,message,error:archiveRuns.cache_error||archiveRuns.save_error||archiveRuns.error,runs:archiveRuns.runs});};
window.ironwoodBeginArchiveAnalysisRun=raw=>{
  // Local provenance is independent of Companion availability. Creating this
  // context performs no archive reads, connections or credential access.
  const metadata=JSON.parse(raw),id=crypto.randomUUID();locallyCreatedAnalysisRuns.add(id);return JSON.stringify({...metadata,id,created_at:Date.now()});
};
window.ironwoodRefreshArchiveAnalysisRuns=()=>{runState();void refreshArchiveRuns();};
window.ironwoodSelectArchiveAnalysisRun=id=>{
  const game=runState();if(!game||archiveRuns.busy)return;
  const key=archiveRuns.key,generation=archiveRuns.generation;archiveRuns.busy=true;archiveRuns.error='';
  void (async()=>{try{await localAnalysisWrites;const cached=await analysisLocalCache.get(runCacheId(key,id)).catch(()=>null);
    let run=cached?.run;if(!run){const storage=await archiveLibrary(false);if(storage.endpoint!==game.endpoint)throw Error('Reconnect to this game’s archive.');run=await storage.request(`archive/games/${game.id}/analysis-runs/${encodeURIComponent(id)}`);await cacheOpenedRun(key,run).catch(error=>{archiveRuns.cache_error=error.message;});}
    if(generation!==archiveRuns.generation||runGameKey(activeRunGame())!==key)return;
    pendingRun=JSON.stringify(run.payload);runWrites.set(`${key}/${id}`,{revision:run.revision,signature:analysisSignature({metadata:run.metadata,payload:run.payload})});
  }catch(error){if(archiveRuns.key===key){archiveRuns.offline=!error.status;archiveRuns.error=error.status?error.message:'This analysis is not cached on this device. Start Companion to open it.';}}finally{if(archiveRuns.key===key)archiveRuns.busy=false;}})();
};
window.ironwoodTakeArchiveAnalysisRun=()=>{if(!runState())return '';const result=pendingRun||'';pendingRun=null;return result;};
export async function deleteArchiveAnalysisRuns(storage,game,runs){
  await localAnalysisWrites;await flushAnalysisQueue();storage.check();
  const result=await storage.request(`archive/games/${game.id}/analysis-runs/delete`,{runs:runs.map(run=>[run.id,run.revision])});
  const key=runGameKey(game);
  for(const run of runs){deletedAnalysisRuns.add(`${key}/${run.id}`);runWrites.delete(`${key}/${run.id}`);}
  await analysisLocalCache.write([],runs.map(run=>runCacheId(key,run.id)));
  const catalog=await analysisLocalCache.get(`${key}/catalog`);
  if(catalog)await analysisLocalCache.write([{...catalog,runs:catalog.runs.filter(run=>!runs.some(deleted=>deleted.id===run.id))}]);
  if(runGameKey(activeRunGame())===key){
    const saved=JSON.parse(localStorage.getItem(CURRENT_GAME_KEY)||'null');
    if(result.game) setArchiveActive(storage.record(result.game));
    if(saved&&runs.some(run=>run.id===saved.analysis_run?.id||(!saved.analysis_run&&run.id===`legacy-${game.id}`))){
      const cleared=clearedAnalysis(saved);pendingRun=JSON.stringify(cleared);
      localStorage.setItem(CURRENT_GAME_KEY,JSON.stringify({...saved,...cleared,game_analysis_running:false,game_analysis_paused:false}));
    }
  }
  notifyLibraryChanged('archive');
  return result;
}
window.ironwoodManageArchiveAnalyses=()=>{void (async()=>{
  const game=runState();if(!game||archiveRuns.busy||document.getElementById('analysis-runs-dialog'))return;
  const key=runGameKey(game),generation=archiveRuns.generation;
  await refreshArchiveRuns();
  if(archiveRuns.key!==key||generation!==archiveRuns.generation||!activeRunGame())return;
  const dialog=document.createElement('dialog');dialog.id='analysis-runs-dialog';
  dialog.setAttribute('aria-labelledby','analysis-runs-title');
  dialog.innerHTML='<header><h2 id="analysis-runs-title">Manage analyses</h2><button type="button" data-close aria-label="Close Manage analyses">Close</button></header><p>Saved analysis runs for this game. Deleting runs keeps the game, notes and other analyses.</p><div class="analysis-runs-table"><table><thead><tr><th><input type="checkbox" aria-label="Select all analysis runs"></th><th>Engine</th><th>Settings</th><th>Status</th><th>Date</th></tr></thead><tbody></tbody></table></div><p data-status role="status"></p><div data-confirm hidden><p>Delete the selected analysis runs permanently? This cannot be undone.</p><button type="button" data-confirm-delete>Delete permanently</button> <button type="button" data-cancel-delete>Cancel</button></div><footer><button type="button" data-delete disabled>Delete selected (0)</button><button type="button" data-done>Done</button></footer>';
  const selected=new Set(),body=dialog.querySelector('tbody'),all=dialog.querySelector('thead input'),remove=dialog.querySelector('[data-delete]'),confirmation=dialog.querySelector('[data-confirm]'),status=dialog.querySelector('[data-status]');
  let runs=archiveRuns.runs.slice(),busy=false;
  const valid=()=>runGameKey(activeRunGame())===key&&archiveRuns.generation===generation;
  const update=()=>{remove.textContent=`Delete selected (${selected.size})`;remove.disabled=busy||!selected.size||archiveRuns.offline;remove.title=archiveRuns.offline?'Start Companion to delete archived analyses.':'';all.checked=runs.length>0&&selected.size===runs.length;all.indeterminate=selected.size>0&&selected.size<runs.length;};
  const render=()=>{body.replaceChildren();for(const run of runs){
    const row=document.createElement('tr'),cell=document.createElement('td'),check=document.createElement('input');check.type='checkbox';check.checked=selected.has(run.id);check.setAttribute('aria-label',`Select ${run.label}`);check.onchange=()=>{check.checked?selected.add(run.id):selected.delete(run.id);confirmation.hidden=true;update();};cell.append(check);row.append(cell);
    for(const [index,value] of [run.metadata.engine_name,run.metadata.settings_label||'Saved settings',run.metadata.status||'Partial',new Date(run.created_at).toLocaleString()].entries()){
      const td=document.createElement('td');td.textContent=value;
      if(index===1){
        const details=document.createElement('details'),summary=document.createElement('summary');summary.textContent='Saved run’s engine and settings';details.append(summary);
        const settings=document.createElement('dl');
        const fields=[['Engine',run.metadata.engine_name||'Unknown engine'],['Analysis budget',run.metadata.settings_label||'Original settings unavailable'],['Verification',run.metadata.verification?'Critical-move verification included':'Full-game analysis']];
        for(const [name,value] of Object.entries(run.metadata.engine_options||{}))fields.push([name,String(value)]);
        for(const [name,value] of fields){const term=document.createElement('dt'),description=document.createElement('dd');term.textContent=name;description.textContent=value;settings.append(term,description);}
        details.append(settings);td.append(details);
      }
      row.append(td);
    }body.append(row);
  }update();if(!runs.length)status.textContent='No saved analysis runs.';};
  all.onchange=()=>{selected.clear();if(all.checked)runs.forEach(run=>selected.add(run.id));confirmation.hidden=true;render();};
  remove.onclick=()=>{confirmation.hidden=false;dialog.querySelector('[data-cancel-delete]').focus();};
  dialog.querySelector('[data-cancel-delete]').onclick=()=>{confirmation.hidden=true;remove.focus();};
  dialog.querySelector('[data-confirm-delete]').onclick=async()=>{
    if(busy||!valid())return;busy=true;dialog.querySelectorAll('button,input').forEach(control=>control.disabled=true);status.textContent='Deleting selected analyses…';
    try{const storage=await archiveLibrary(false);if(!valid()||storage.endpoint!==game.endpoint)throw Error('Game or Companion connection changed. Close this window and try again.');
      const chosen=runs.filter(run=>selected.has(run.id));await deleteArchiveAnalysisRuns(storage,game,chosen);await refreshArchiveRuns();
      if(!valid()){dialog.close();return;}runs=archiveRuns.runs.slice();selected.clear();confirmation.hidden=true;status.textContent=`Deleted ${chosen.length} analysis run${chosen.length===1?'':'s'}.`;render();
    }catch(error){status.textContent=`Could not delete analyses: ${error.message}`;}
    finally{busy=false;dialog.querySelectorAll('button,input').forEach(control=>control.disabled=false);update();}
  };
  dialog.querySelectorAll('[data-close],[data-done]').forEach(button=>button.onclick=()=>dialog.close());
  const heading=dialog.querySelector('header');heading.title='Drag to move';let drag=null;
  const stopDragging=()=>{drag=null;heading.classList.remove('dragging');};
  heading.addEventListener('pointerdown',event=>{
    if(event.button!==0||event.target.closest('button,input,select,a'))return;
    const rect=dialog.getBoundingClientRect();
    drag={pointerId:event.pointerId,x:event.clientX,y:event.clientY,left:rect.left,top:rect.top,width:rect.width,height:rect.height};
    Object.assign(dialog.style,{position:'fixed',margin:'0',inset:'auto',left:`${rect.left}px`,top:`${rect.top}px`,width:`${rect.width}px`,height:`${rect.height}px`});
    heading.classList.add('dragging');heading.setPointerCapture?.(event.pointerId);event.preventDefault();
  });
  heading.addEventListener('pointermove',event=>{
    if(!drag||drag.pointerId!==event.pointerId)return;
    dialog.style.left=`${Math.max(0,Math.min(window.innerWidth-drag.width,drag.left+event.clientX-drag.x))}px`;
    dialog.style.top=`${Math.max(0,Math.min(window.innerHeight-drag.height,drag.top+event.clientY-drag.y))}px`;
  });
  for(const type of ['pointerup','pointercancel','lostpointercapture'])heading.addEventListener(type,stopDragging);
  const keepVisible=()=>{
    if(dialog.style.position!=='fixed')return;const rect=dialog.getBoundingClientRect();
    dialog.style.left=`${Math.max(0,Math.min(window.innerWidth-rect.width,rect.left))}px`;
    dialog.style.top=`${Math.max(0,Math.min(window.innerHeight-rect.height,rect.top))}px`;
  };
  const observer=window.ResizeObserver?new window.ResizeObserver(keepVisible):null;observer?.observe(dialog);window.addEventListener('resize',keepVisible);
  dialog.addEventListener('close',()=>{stopDragging();observer?.disconnect();window.removeEventListener('resize',keepVisible);},{once:true});
  dialog.addEventListener('cancel',event=>{if(busy)event.preventDefault();});
  const invalidate=()=>{if(!busy)dialog.close();};
  window.addEventListener('ironwood-companion-connection-changed',invalidate);window.addEventListener('ironwood-companion-feature-changed',invalidate);
  dialog.addEventListener('close',()=>{dialog.remove();window.removeEventListener('ironwood-companion-connection-changed',invalidate);window.removeEventListener('ironwood-companion-feature-changed',invalidate);},{once:true});
  render();if(archiveRuns.offline)status.textContent='Companion offline — cached analyses are available to view. Start Companion to delete archived runs.';document.body.append(dialog);dialog.showModal();
})().catch(error=>{archiveRuns.error=error.message;});};
export function archiveAnalysisSnapshot(saved){
  const payload={};for(const field of ['analysis_run','game_analysis','verification_targets','verification_results','verification_nodes','verification_time_ms'])payload[field]=saved[field]??(field==='analysis_run'?null:field.endsWith('nodes')||field.endsWith('ms')?0:[]);
  return payload;
}
export async function preserveArchiveAnalysis(storage,game,previous,saved){
  const key=`${storage.endpoint}/${game}`;
  const store=async snapshot=>{
    const context=snapshot.analysis_run;if(!context||deletedAnalysisRuns.has(`${key}/${context.id}`))return;
    const payload=archiveAnalysisSnapshot(snapshot),{id,position_fens,created_at,...details}=context;
    const completed=(snapshot.game_analysis||[]).filter(Boolean).length;
    const metadata={...details,completed_positions:completed,total_positions:position_fens?.length||snapshot.game_analysis.length,status:completed>0&&completed===(position_fens?.length||snapshot.game_analysis.length)?'Complete':'Partial'};
    const signature=analysisSignature({metadata,payload}),cacheKey=`${key}/${id}`;
    let existing=runWrites.get(cacheKey);
    if(existing?.signature===signature)return;
    if(!existing){try{const run=await storage.request(`archive/games/${game}/analysis-runs/${id}`);existing={revision:run.revision,signature:analysisSignature({metadata:run.metadata,payload:run.payload})};}catch(error){if(error.status!==404)throw error;existing={revision:0};}}
    if(id.startsWith('legacy-')&&existing.revision>0)return;
    if(existing.signature===signature){runWrites.set(cacheKey,existing);return;}
    const result=await storage.request(`archive/games/${game}/analysis-runs/${id}`,{expected_revision:existing.revision,metadata,payload});
    runWrites.set(cacheKey,{revision:result.revision,signature});
  };
  if(previous.game_analysis?.some(Boolean)&&!previous.analysis_run){
    await store({...previous,analysis_run:{id:`legacy-${game}`,engine_name:'Previous analysis (engine unverified)',settings_label:'Original saved results',position_fens:saved.analysis_run?.position_fens||previous.live_positions||[],created_at:Date.now()}});
  }
  if(saved.analysis_run)await store(saved);
  if(archiveRuns.key===key){archiveRuns.error='';setTimeout(()=>void refreshArchiveRuns(),0);}
}

window.ironwoodStoreCurrentGame = json => {
  const active=getArchiveActive();
  if(active){
    const candidate=JSON.parse(json);
    if(candidate.analysis_run&&deletedAnalysisRuns.has(`${runGameKey(active)}/${candidate.analysis_run.id}`))json=JSON.stringify({...candidate,...clearedAnalysis(candidate),game_analysis_running:false,game_analysis_paused:false});
    const write=localAnalysisWrites.then(()=>queueArchiveAnalysis(active,json));
    localAnalysisWrites=write.catch(error=>{if(runGameKey(getArchiveActive())===runGameKey(active))archiveRuns.cache_error=`Could not save analysis locally: ${error.message}`;});
    pendingWrite=localAnalysisWrites;
    void write.then(scheduleAnalysisSync,()=>{});
    return;
  }
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
    localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
    localStorage.setItem(ACTIVE_ID_KEY, newId());
    localStorage.setItem(ACTIVE_CATEGORY_KEY, ['imported', 'observed', 'training', 'puzzles'].includes(category) ? category : 'mine');
  } catch (error) {
    console.warn('Ironwood could not start a new saved game:', error);
  }
};

export async function initializeGameStorage() {
  startAnalysisSync();
  void refreshStorageStatus();
  try {
    const current = localStorage.getItem(CURRENT_GAME_KEY);
    if(!current&&getArchiveActive())localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
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

async function savedGameSource(game) {
  const storage = game._archive ? await archiveLibrary() : null;
  if (storage && storage.endpoint !== game._archive.endpoint) {
    throw Error('Reconnect to the archive containing this game.');
  }
  return storage;
}

async function openSavedGame(game, startingPosition = false) {
  await pendingWrite;
  const storage = await savedGameSource(game);
  const selected = storage ? await storage.read(game.id) : await readGame(game.id);
  if (!selected?.json) throw new Error('Saved game was not found');
  const saved = JSON.parse(selected.json);
  if (startingPosition) saved.review_index = 0;
  if(storage)setArchiveActive(selected);else localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
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
  document.getElementById('game-preview-dialog')?.close();
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
  const players = document.createElement('div');
  players.className = 'game-preview-players';
  for (const [color, label] of [['white', 'White'], ['black', 'Black']]) {
    const player = document.createElement('div');
    player.className = `game-preview-player ${color}`;
    const side = document.createElement('strong'); side.textContent = label;
    const name = document.createElement('span'); name.textContent = game[color] || 'Unknown player';
    player.append(side, name); players.append(player);
  }
  const outcome = document.createElement('p'); outcome.className = 'game-preview-outcome';
  outcome.textContent = game.result === '1-0' ? `${game.white || 'White'} won as White · 1–0`
    : game.result === '0-1' ? `${game.black || 'Black'} won as Black · 0–1`
    : game.result === '1/2-1/2' ? 'Draw · ½–½' : 'Unfinished / result unavailable';
  const caption = document.createElement('p');
  const boardHost = document.createElement('div');
  boardHost.className = 'game-preview-board-tray';
  const controls = document.createElement('div');
  controls.className = 'game-preview-controls';
  let positions = game.preview?.positions || [game.finalFen];
  let moves = game.preview?.moves || [];
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
  const info = document.createElement('section');
  info.className = 'game-preview-info';
  info.append(heading, players, outcome, caption);
  dialog.append(info, boardHost, controls);
  const note = startingNoteElement(game);
  if (note) dialog.append(note);
  dialog.addEventListener('close', () => {
    stop();
    dialog.remove();
    if (trigger?.isConnected) trigger.focus();
  }, { once: true });
  document.body.append(dialog);
  dialog.showModal();
  close.focus();
  if (game.preview) return;
  try {
    const storage = await savedGameSource(game);
    const stored = storage ? await storage.read(game.id) : await readGame(game.id);
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

window.ironwoodShowProfileGamePreview = raw => showGamePreview(JSON.parse(raw), document.getElementById('chess_canvas'));

function confirmDeleteGames(games, active) {
  return new Promise(resolve => {
    const dialog = document.createElement('dialog');
    dialog.id = 'game-delete-dialog';
    const eyebrow = document.createElement('p');
    eyebrow.className = 'game-delete-eyebrow';
    eyebrow.textContent = 'SAVED GAMES';
    const title = document.createElement('h2');
    title.id = 'game-delete-title';
    dialog.setAttribute('aria-labelledby', title.id);
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
    dialog.append(eyebrow, title, name, warning, actions);
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
  dialog.querySelector('#companion-archive')?.close();
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
  const titleGroup = document.createElement('div'); titleGroup.className = 'game-library-title-group';
  const sources = document.createElement('div'); sources.className = 'game-library-sources'; sources.setAttribute('role','group'); sources.setAttribute('aria-label','Game storage source');
  const browserSource = document.createElement('button'), archiveSource = document.createElement('button');
  browserSource.type = archiveSource.type = 'button'; browserSource.textContent = 'Browser'; archiveSource.textContent = 'Archive';
  sources.append(browserSource,archiveSource); titleGroup.append(title,sources); heading.append(titleGroup, windowControls);
  heading.title = 'Drag to move';
  let drag = null;
  const stopDragging = () => { drag = null; heading.classList.remove('dragging'); };
  heading.addEventListener('pointerdown', event => {
    if (event.button !== 0 || dialog.classList.contains('maximized') || event.target.closest('button,input,select,a')) return;
    const rect = dialog.getBoundingClientRect();
    drag = {pointerId:event.pointerId,x:event.clientX,y:event.clientY,left:rect.left,top:rect.top,width:rect.width,height:rect.height};
    Object.assign(dialog.style,{position:'fixed',margin:'0',inset:'auto',left:`${rect.left}px`,top:`${rect.top}px`,width:`${rect.width}px`,height:`${rect.height}px`});
    heading.classList.add('dragging');heading.setPointerCapture?.(event.pointerId);event.preventDefault();
  });
  heading.addEventListener('pointermove', event => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    dialog.style.left = `${Math.max(0,Math.min(window.innerWidth-drag.width,drag.left+event.clientX-drag.x))}px`;
    dialog.style.top = `${Math.max(0,Math.min(window.innerHeight-drag.height,drag.top+event.clientY-drag.y))}px`;
  });
  for (const type of ['pointerup','pointercancel','lostpointercapture']) heading.addEventListener(type,stopDragging);
  dialog.addEventListener('close',stopDragging,{once:true});

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
  const controls = document.createElement('section');
  controls.className = 'game-library-controls';
  controls.setAttribute('aria-label', 'Find and manage saved games');
  controls.append(tabs, filters, selection);
  const browserContent = document.createElement('div'); browserContent.className = 'game-library-browser'; browserContent.append(controls,list,pager);
  const notice=document.createElement('p');notice.className='game-library-notice';notice.setAttribute('role','status');notice.hidden=true;
  let source='browser', sourceEpoch=0, ready=false, operationBusy=false, loadSource=async()=>{}, renderCurrent=()=>{}, saveCurrentView=()=>{};
  const companionVisible = () => window.ironwoodCompanionFeatureEnabled?.() === true;
  function updateSourceButtons() {
    sources.hidden=!companionVisible();
    browserSource.setAttribute('aria-pressed',String(source==='browser'));archiveSource.setAttribute('aria-pressed',String(source==='archive'));
    browserSource.disabled=archiveSource.disabled=!ready||operationBusy;
  }
  async function chooseSource(value) {
    if(!ready||operationBusy||value===source||value==='archive'&&!companionVisible())return;
    saveCurrentView();source=value;updateSourceButtons();await loadSource();
  }
  browserSource.onclick=()=>void chooseSource('browser');archiveSource.onclick=()=>void chooseSource('archive');
  const updateSources=()=>{if(!companionVisible()&&source==='archive'){source='browser';void loadSource();}updateSourceButtons();if(ready)renderCurrent();};
  updateSourceButtons();
  window.addEventListener('ironwood-companion-feature-changed',updateSources);
  dialog.addEventListener('close',()=>{saveCurrentView();sourceEpoch++;window.removeEventListener('ironwood-companion-feature-changed',updateSources);},{once:true});
  dialog.append(heading,notice,browserContent);
  dialog.showModal();
  try {
    let games = [], provider=null;
    let archivePage=null, loadedQuery='', requestedQuery='', queryEpoch=0, queryTimer=null;
    const selectedRecords=new Map();
    const remote=()=>Boolean(provider?.page);
    const query=()=>({category,player:playerFilter,profile:profileFilter,search:searchQuery,analysis:analysisFilter,sort:sortOrder,page});
    const selectedGames=()=>remote()?[...selectedIds].map(id=>selectedRecords.get(id)).filter(Boolean):games.filter(game=>selectedIds.has(game.id));
    let browserLoadedVersion=-1, refreshTimer=null, sourceLoading=true, prefetchTimer=null;
    const cacheKey=key=>`${archiveConnectionVersion}:${provider?.endpoint}:${key}`;
    const rememberPage=(key,result,prefetched=false)=>{
      const id=cacheKey(key),facetId=cacheKey(JSON.stringify({category:JSON.parse(key).category||'all'}));
      const {counts,players,profiles,...summary}=result;
      archiveFacetCache.delete(facetId);archiveFacetCache.set(facetId,{revision:result.revision,counts,players,profiles});
      while(archiveFacetCache.size>8)archiveFacetCache.delete(archiveFacetCache.keys().next().value);
      archivePageCache.delete(id);
      archivePageCache.set(id,{version:libraryVersions.archive,result:structuredClone(summary),facetId,prefetched,at:Date.now()});
      while(archivePageCache.size>8)archivePageCache.delete(archivePageCache.keys().next().value);
    };
    const cachedPage=key=>{
      const id=cacheKey(key),cached=archivePageCache.get(id),facets=cached&&archiveFacetCache.get(cached.facetId);
      if(!cached||cached.version!==libraryVersions.archive||!facets||facets.revision!==cached.result.revision)return null;
      archivePageCache.delete(id);archivePageCache.set(id,cached);
      return {...structuredClone(cached.result),...structuredClone(facets)};
    };
    const prefetchNext=(key,result,storage=provider)=>{
      clearTimeout(prefetchTimer);
      const filters=JSON.parse(key),next={...filters,page:result.page+1};
      if(next.page*pageSize>=result.total)return;
      const nextKey=JSON.stringify(next);
      if(cachedPage(nextKey))return;
      const epoch=queryEpoch,sourceVersion=sourceEpoch,version=libraryVersions.archive;
      const active=()=>dialog.open&&source==='archive'&&provider===storage&&sourceEpoch===sourceVersion&&queryEpoch===epoch&&libraryVersions.archive===version&&JSON.stringify(query())===key;
      prefetchTimer=setTimeout(async()=>{
        if(!active())return;
        try{
          const page=await storage.page(next,active,{prefetch:true});
          if(active()&&page.page===next.page)rememberPage(nextKey,page,true);
        }catch{/* Speculative requests never interrupt browsing or start a retry loop. */}
      },100);
    };
    const applyArchivePage=result=>{
      archivePage=result;page=result.page;games=result.games;loadedQuery=JSON.stringify(query());requestedQuery=loadedQuery;
      for(const id of selectedRecords.keys())if(!selectedIds.has(id))selectedRecords.delete(id);
      for(const game of games)if(selectedIds.has(game.id))selectedRecords.set(game.id,game);
    };
    const loadArchivePage=async(key,storage=provider,background=false)=>{
      const epoch=++queryEpoch, sourceVersion=sourceEpoch, version=libraryVersions.archive;
      const active=()=>dialog.open&&provider===storage&&source==='archive'&&sourceEpoch===sourceVersion&&queryEpoch===epoch&&libraryVersions.archive===version&&JSON.stringify(query())===key;
      try {
        const result=await storage.page(JSON.parse(key),active);
        if(!active())return;
        applyArchivePage(result);rememberPage(loadedQuery,result);message('');render();prefetchNext(loadedQuery,result,storage);
      }catch(error){if(active()){
        requestedQuery='';
        if(background&&archivePage){message(`Showing cached games. Refresh failed: ${error.message}`);return;}
        list.inert=false;list.textContent=`Saved games are unavailable: ${error.message}`;const retry=document.createElement('button');retry.textContent='Retry';retry.onclick=()=>render();list.append(retry);
      }}
    };
    let category = restore?.category || 'all';
    let profileFilter = restore?.profileFilter || '*';
    let playerFilter = restore?.playerFilter || '*';
    let searchQuery = restore?.searchQuery || '';
    let analysisFilter = restore?.analysisFilter || '*';
    let sortOrder = restore?.sortOrder || localStorage.getItem('ironwood.chess.library-sort.v1') || 'recent';
    if (!['recent', 'moves-asc', 'moves-desc'].includes(sortOrder)) sortOrder = 'recent';
    let page = Number.isSafeInteger(restore?.page) ? Math.max(0, restore.page) : 0;
    const pageSize = 10;
    const selectedIds = new Set();
    const initialView=()=>({category:'all',profile:'*',player:'*',search:'',analysis:'*',sort:'recent',page:0});
    const restoreView=view=>{
      const settings=view?.settings||initialView();
      category=settings.category;profileFilter=settings.profile;playerFilter=settings.player;searchQuery=settings.search;
      analysisFilter=settings.analysis;sortOrder=settings.sort;page=settings.page;
      selectedIds.clear();selectedRecords.clear();
      for(const id of view?.selected||[])selectedIds.add(id);
      for(const record of view?.records||[])selectedRecords.set(record.id,record);
    };
    saveCurrentView=()=>{
      if(source==='archive'&&!companionVisible())return;
      savedLibraryViews[source]={settings:query(),selected:[...selectedIds],records:[...selectedRecords.values()].map(record=>({id:record.id,title:record.title,_archive:record._archive?{endpoint:record._archive.endpoint,id:record.id,revision:record._archive.revision}:undefined})),
        games:source==='browser'?games:undefined,provider:source==='archive'?provider:null,
        valid:source==='browser'?browserLoadedVersion===libraryVersions.browser:true,version:libraryVersions[source]};
    };
    if(!restore&&savedLibraryViews.browser)restoreView(savedLibraryViews.browser);
    const changes=event=>{
      if(!['browser','archive'].includes(event.detail?.source))return;
      if(event.detail.source===source){
        for(const id of event.detail.deletedIds||[]){selectedIds.delete(id);selectedRecords.delete(id);}
        clearTimeout(refreshTimer);
        refreshTimer=setTimeout(()=>{if(dialog.open&&!operationBusy)void loadSource({background:true,force:true});},80);
      }
    };
    const connectionChanged=()=>{
      if(source!=='archive')return;
      selectedIds.clear();selectedRecords.clear();queryEpoch++;provider=null;archivePage=null;
      void loadSource({force:true});
    };
    window.addEventListener('ironwood-game-library-changed',changes);
    window.addEventListener('ironwood-companion-connection-changed',connectionChanged);
    dialog.addEventListener('close',()=>{queryEpoch++;clearTimeout(queryTimer);clearTimeout(refreshTimer);clearTimeout(prefetchTimer);window.removeEventListener('ironwood-game-library-changed',changes);window.removeEventListener('ironwood-companion-connection-changed',connectionChanged);},{once:true});
    let deleting = false;
    const busy=value=>{operationBusy=value;browserContent.inert=value;updateSourceButtons();};
    const message=text=>{notice.hidden=!text;notice.textContent=text;};
    const readSource=id=>provider?provider.read(id):readGame(id);
    const writeSource=async record=>{if(!record._archive)return putGame(record);const storage=await archiveLibrary(false);if(storage.endpoint!==record._archive.endpoint)throw Error('Archive connection changed.');return storage.write(record);};
    const refreshGames=async()=>{clearTimeout(refreshTimer);invalidateLibraryCache('browser');invalidateLibraryCache('archive');selectedIds.clear();selectedRecords.clear();if(remote()){loadedQuery='';requestedQuery='';await loadArchivePage(JSON.stringify(query()));}else{games=provider?(await provider.list(()=>dialog.open&&source==='archive')).map(librarySummary):await listGames();browserLoadedVersion=libraryVersions.browser;render();}};
    const moveGames=async targets=>{
      if(operationBusy||!targets.length||!companionVisible())return;
      const from=source, storage=provider;busy(true);message('Moving games…');
      try {
        await pendingWrite;
        if(from==='browser') {
          const records=await Promise.all(targets.map(game=>readGame(game.id)));
          if(records.some(record=>!record?.json))throw Error('A browser game is no longer available. Refresh and try again.');
          const destination=await archiveLibrary();
          const ids=await destination.importRecords(records);
          const active=records.findIndex(record=>!getArchiveActive()&&record.id===localStorage.getItem(ACTIVE_ID_KEY));
          const saved=active>=0?await destination.read(ids[active]):null;
          destination.check();await deleteUnchangedBrowserRecords(records);
          if(saved)setArchiveActive(saved);
        } else {
          const result=await storage.moveToBrowser(targets.map(game=>game.id));
          const active=getArchiveActive();
          if(active?.endpoint===storage.endpoint){const index=targets.findIndex(game=>game.id===active.id);if(index>=0){localStorage.removeItem(ACTIVE_ARCHIVE_KEY);if(result.ids?.[index])localStorage.setItem(ACTIVE_ID_KEY,result.ids[index]);}}
        }
        await refreshGames();message(`${targets.length} game${targets.length===1?'':'s'} moved to ${from==='browser'?'Archive':'Browser'}.`);
      } catch(error) {
        try{await refreshGames();}catch{}
        message(`Move not completed: ${error.message} Any source that could not be removed was kept; destination copies may already exist.`);
      } finally{busy(false);}
    };

    const removeGames = async targets => {
      if (deleting || operationBusy || !targets.length) return;
      deleting = true;
      try {
        const active = targets.some(game => provider ? getArchiveActive()?.endpoint===provider.endpoint&&getArchiveActive()?.id===game.id : !getArchiveActive()&&localStorage.getItem(ACTIVE_ID_KEY) === game.id);
        if (!await confirmDeleteGames(targets, active)) return;
        busy(true);
        if(provider)await provider.remove(targets);else await deleteGames(targets.map(game => game.id));
        const removed = new Set(targets.map(game => game.id));
        for (let index = games.length - 1; index >= 0; index--) {
          if (removed.has(games[index].id)) games.splice(index, 1);
        }
        removed.forEach(id => selectedIds.delete(id));
        importedFingerprints = new Set(games.map(storedFingerprint).filter(Boolean));
        if (active) {
          localStorage.removeItem(CURRENT_GAME_KEY);
          localStorage.removeItem(ACTIVE_ARCHIVE_KEY);
          localStorage.removeItem(ACTIVE_ID_KEY);
          localStorage.removeItem(ACTIVE_CATEGORY_KEY);
          sessionStorage.setItem(LIBRARY_RETURN_KEY, JSON.stringify({
            category, profileFilter, playerFilter, searchQuery, analysisFilter, sortOrder, page,
            maximized: dialog.classList.contains('maximized'),
          }));
          location.reload();
        } else if(remote())await refreshGames();else render();
      } catch (error) {
        if(provider)try{await refreshGames();}catch{}
        const message = document.createElement('p');
        message.setAttribute('role', 'alert');
        message.textContent = `Could not delete games: ${error.message}`;
        selection.append(message);
      } finally { deleting = false; busy(false); }
    };
    const render = () => {
      tabs.replaceChildren();
      for (const [value, label] of [
        ['all', 'All Games'],
        ['mine', 'My Games'],
        ['training', 'Training'],
        ['puzzles', 'Puzzles'],
        ['observed', 'Observed'],
        ['imported', 'Imported Games'],
        ['favorites', 'Favorites'],
      ]) {
        const tab = document.createElement('button');
        tab.type = 'button';
        tab.textContent = `${label} (${remote()?archivePage?.counts?.[value]||0:games.filter(game => gameMatchesCategory(game, value)).length})`;
        tab.className = category === value ? 'selected' : '';
        tab.addEventListener('click', () => { category = value; playerFilter = '*'; profileFilter = '*'; selectedIds.clear(); page = 0; render(); });
        tabs.append(tab);
      }
      filters.replaceChildren();
      const refresh=document.createElement('button');refresh.type='button';refresh.textContent='Refresh';refresh.onclick=()=>{if(!operationBusy)void loadSource({force:true});};filters.append(refresh);
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
        for (const [id, name] of [['*', 'All profiles'], ...(remote()?archivePage?.profiles||[]:[...new Map(games.filter(g => g.trainingProfileId).map(g => [g.trainingProfileId, g.trainingProfileName])).entries()])]) {
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
        const names = remote()?(archivePage?.players||[]).filter(name=>name!=='Unknown'):[...new Set(categoryGames.flatMap(game => [game.white, game.black]).filter(name => name && name !== 'Unknown'))].sort();
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
      const sortLabel = document.createElement('label'); sortLabel.textContent = 'Sort ';
      const sortSelect = document.createElement('select'); sortSelect.setAttribute('aria-label', 'Sort saved games');
      for (const [value, text] of [['recent', 'Recently saved'], ['moves-asc', 'Fewest moves first'], ['moves-desc', 'Most moves first']]) {
        const option = document.createElement('option'); option.value = value; option.textContent = text; sortSelect.append(option);
      }
      sortSelect.value = sortOrder;
      sortSelect.addEventListener('change', () => {
        sortOrder = sortSelect.value; localStorage.setItem('ironwood.chess.library-sort.v1', sortOrder);
        page = 0; render(); list.scrollTop = 0;
      });
      sortLabel.append(sortSelect); filters.append(sortLabel);
      list.replaceChildren();
      pager.replaceChildren();
      if(sourceLoading){selection.replaceChildren();list.textContent='Loading games…';list.inert=selection.inert=pager.inert=true;return;}
      const key=JSON.stringify(query());
      const loading=remote()&&loadedQuery!==key;
      list.inert=selection.inert=pager.inert=loading;
      if(loading) {
        const cached=cachedPage(key);
        if(cached){
          const entry=archivePageCache.get(cacheKey(key));
          applyArchivePage(cached);render();clearTimeout(queryTimer);
          if(entry?.prefetched&&Date.now()-entry.at<10000){entry.prefetched=false;prefetchNext(JSON.stringify(query()),cached);}
          else void loadArchivePage(JSON.stringify(query()),provider,true);
          return;
        }
        clearTimeout(prefetchTimer);selection.replaceChildren();list.textContent='Loading archive games…';
        if(requestedQuery!==key){requestedQuery=key;clearTimeout(queryTimer);queryTimer=setTimeout(()=>void loadArchivePage(key),150);}
        return;
      }
      const visible = remote()?games:sortSavedGames(games.filter(game => [game.title, game.white, game.black, game.source, game.venue, game.playedAt].join(' ').toLowerCase().includes(searchQuery.trim().toLowerCase()) && gameMatchesCategory(game, category) && (profileFilter === '*' || game.trainingProfileId === profileFilter) && (playerFilter === '*' ||
        playerFilter === '?' && [game.white, game.black].includes('Unknown') ||
        [game.white, game.black].includes(playerFilter)) && (analysisFilter === '*' ||
        analysisFilter === 'complete' && game.analysisStatus === 'Complete' ||
        analysisFilter === 'partial' && game.analysisStatus === 'Partial' ||
        analysisFilter === 'unanalyzed' && !(game.analyzed > 0))), sortOrder);
      // Selection stays across pages, but never includes games hidden by filters.
      const matchingIds = new Set(visible.map(game => game.id));
      if(!remote())for (const id of selectedIds) if (!matchingIds.has(id)) selectedIds.delete(id);
      for(const id of selectedRecords.keys())if(!selectedIds.has(id))selectedRecords.delete(id);
      for(const game of games)if(selectedIds.has(game.id))selectedRecords.set(game.id,game);
      const matchingCount=remote()?archivePage.total:visible.length;
      page = Math.max(0, Math.min(page, Math.ceil(matchingCount / pageSize) - 1));
      const pageGames = remote()?games:visible.slice(page * pageSize, (page + 1) * pageSize);
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
      selectAll.textContent = `Select all ${matchingCount} matching`;
      selectAll.disabled = !matchingCount || selectedIds.size === matchingCount;
      selectAll.addEventListener('click', async () => {
        if(!remote()){visible.forEach(game=>selectedIds.add(game.id));render();return;}
        const storage=provider,key=JSON.stringify(query()),sourceVersion=sourceEpoch;
        const active=()=>dialog.open&&provider===storage&&sourceEpoch===sourceVersion&&JSON.stringify(query())===key&&companionVisible();
        selectAll.disabled=true;
        try {const rows=await storage.selection(query(),active);if(!active())return;for(const game of rows){selectedIds.add(game.id);selectedRecords.set(game.id,game);}render();}
        catch(error){if(active()){message(`Could not select games: ${error.message}`);render();}}
      });
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
          const ids = [...selectedIds];
          exportChoices.hidden = true; exportButton.setAttribute('aria-expanded', 'false');
          exportButton.disabled = true;
          try {
            await pendingWrite;
            const records = provider?.readMany ? await provider.readMany(ids) : await Promise.all(ids.map(readSource));
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
      removeSelected.addEventListener('click', () => removeGames(selectedGames()));
      selection.append(selectPageLabel, selectAll, exportMenu, removeSelected);
      if(companionVisible()) {
        const transfer=document.createElement('button');transfer.type='button';transfer.textContent=source==='archive'?'Move to Browser':'Move to Archive';transfer.disabled=!selectedIds.size||operationBusy;
        transfer.onclick=()=>void moveGames(selectedGames());selection.append(transfer);
      }
      if (!visible.length) {
        list.textContent = games.some(game => gameMatchesCategory(game, category)) ? 'No games match these filters.' :
          category === 'all' ? 'No saved games yet.' :
          category === 'training' ? 'Start Training vs AI from New game to add games under a profile.' :
          category === 'puzzles' ? 'Puzzle move history appears here after the first move or a revealed solution.' :
          category === 'observed' ? 'Finished games you watch are saved here automatically.' :
          category === 'mine' ? 'No games here yet. Finished online games and games against Stockfish are saved automatically.' :
            category === 'imported' ? 'No imported games here yet. Import a PGN or analysis JSON to add one.' :
              'No favorite games yet. Mark a game as a favorite to see it here.';
        return;
      }
      const pageCount = Math.ceil(matchingCount / pageSize);
      page = Math.min(page, pageCount - 1);
      const previous = document.createElement('button');
      previous.type = 'button';
      previous.textContent = '◀ Previous';
      previous.disabled = page === 0;
      previous.addEventListener('click', () => { page--; render(); list.scrollTop = 0; });
      const count = document.createElement('span');
      count.textContent = `Page ${page + 1} of ${pageCount} · ${matchingCount} games`;
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
        if (game.category === 'puzzles') name.append(game.favorite ? '★ ' : '', game.title);
        else name.append(game.favorite ? '★ ' : '', playerNameElement(game.white, fide.whiteFideId),
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
        result.textContent = game.category === 'puzzles' ? 'Puzzle move history' : game.result === '1-0' ? `${game.white} won (White)` :
          game.result === '0-1' ? `${game.black} won (Black)` :
          game.result === '1/2-1/2' ? 'Draw' : 'Result unknown / unfinished';
        const details = document.createElement('span');
        details.textContent = `${game.playedAt ? `Played ${game.playedAt}` : 'Date played unknown'} · ${game.venue ? `Venue: ${game.venue}` : 'Venue unknown'} · Saved ${new Date(game.updatedAt).toLocaleString()} · ${game.moves} moves · ${game.analysisStatus} · ${game.analyzed} positions analyzed${game.source ? ` · ${game.source}` : ''}`;
        if (game.trainingProfileId) {
          details.textContent += ` · Training: ${game.trainingProfileName} · ${game.trainingSide} · ${game.trainingEngineKind==='lc0'?'Level':'Elo'} ${game.ratingBefore}`;
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
        move.hidden = ['observed', 'training', 'puzzles'].includes(game.category);
        move.textContent = game.category === 'mine' ? 'Move to Imported' : 'Move to My Games';
        move.addEventListener('click', async () => {
          const oldCategory = game.category;
          game.category = oldCategory === 'mine' ? 'imported' : 'mine';
          try {
            const stored = await readSource(game.id);
            if (!stored) throw new Error('Saved game was not found');
            const updated=await writeSource({ ...stored, category: game.category });
            if(updated?._archive){game._archive=updated._archive;const active=getArchiveActive();if(active?.endpoint===updated._archive.endpoint&&active?.id===updated.id)setArchiveActive(updated);}
            if (localStorage.getItem(ACTIVE_ID_KEY) === game.id) localStorage.setItem(ACTIVE_CATEGORY_KEY, game.category);
            if(remote())await refreshGames();else render();
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
            const stored = await readSource(game.id);
            if (!stored) throw new Error('Saved game was not found');
            const updated=await writeSource({ ...stored, favorite: game.favorite });
            if(updated?._archive){game._archive=updated._archive;const active=getArchiveActive();if(active?.endpoint===updated._archive.endpoint&&active?.id===updated.id)setArchiveActive(updated);}
            if(remote())await refreshGames();else render();
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
        if(companionVisible()) {
          const transfer=document.createElement('button');transfer.type='button';transfer.textContent=source==='archive'?'Move to Browser':'Move to Archive';
          transfer.onclick=()=>void moveGames([game]);actions.append(transfer);
        }
        main.append(name, players, result, details);
        const note = startingNoteElement(game, true);
        if (note) main.append(note);
        main.append(actions);
        entry.append(main);
        list.append(entry);
      }
    };
    loadSource=async({force=false,background=false,restoreState=true}={})=>{
      const switching=!background;sourceLoading=false;
      if(force)invalidateLibraryCache(source);
      const epoch=++sourceEpoch, selectedSource=source;
      queryEpoch++;clearTimeout(queryTimer);clearTimeout(refreshTimer);clearTimeout(prefetchTimer);requestedQuery='';message('');
      const view=savedLibraryViews[source];
      if(switching&&restoreState){
        // A forced refresh keeps the current view; source changes restore the other view.
        if(!force&&view)restoreView(view);
        else if(!force&&source==='archive')restoreView(null);
      }
      const oldProvider=provider;
      provider=selectedSource==='archive'?(view?.provider||oldProvider?.endpoint&&oldProvider):null;
      let validProvider=provider;
      if(validProvider){try{validProvider.check();}catch{validProvider=null;provider=null;archivePageCache.clear();selectedIds.clear();selectedRecords.clear();}}
      let shown=false;
      if(selectedSource==='browser'&&background){render();shown=true;}
      if(selectedSource==='browser'&&!force&&view?.valid&&view.version===libraryVersions.browser){games=view.games||[];browserLoadedVersion=view.version;loadedQuery='';archivePage=null;render();shown=true;}
      if(selectedSource==='archive'&&validProvider){
        const cached=!force&&cachedPage(JSON.stringify(query()));
        if(cached){applyArchivePage(cached);render();shown=true;}
        else if(background&&archivePage){render();shown=true;}
      }
      if(!shown){loadedQuery='';archivePage=null;games=[];sourceLoading=true;if(source==='browser')browserLoadedVersion=-1;render();}
      try {
        const storage=selectedSource==='archive'?(validProvider||await archiveLibrary()):null;
        const active=()=>epoch===sourceEpoch&&dialog.open;
        if(!active())return;
        if(storage?.page){provider=storage;sourceLoading=false;
          if(shown){await loadArchivePage(JSON.stringify(query()),storage,true);}
          else {render();}
          return;
        }
        const version=libraryVersions[selectedSource];
        const records=storage?await storage.list(active):null;
        const next=storage?records.map(librarySummary):await listGames();
        if(!active()||version!==libraryVersions[selectedSource])return;
        provider=storage;games=next;browserLoadedVersion=libraryVersions.browser;sourceLoading=false;render();
      }catch(error){if(epoch===sourceEpoch&&dialog.open){
        sourceLoading=false;
        if(shown){message(`Showing cached games. Refresh failed: ${error.message}`);return;}
        list.inert=false;list.textContent=`Saved games are unavailable: ${error.message}`;const retry=document.createElement('button');retry.textContent='Retry';retry.onclick=()=>void loadSource({force:true});list.append(retry);
      }}
    };
    renderCurrent=render;ready=true;updateSourceButtons();render();await loadSource({restoreState:!restore});
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
