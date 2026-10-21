import { companionConnection, requireCompanionConnection, manageCompanionConnection } from './companion-connection.js';
import { CompanionArchiveClient } from "./companion-archive-client.js";
// Original API client; no Lichess server code or tablebase files are bundled.
export function tablebaseUrl(fen) {
  const fields = fen.trim().split(/\s+/);
  const count = (fields[0].match(/[prnbqk]/gi) || []).length;
  if (fields.length !== 6 || count < 2 || count > 7 || fields[2] !== '-')
    throw new Error('Use a position with at most seven pieces and no castling rights.');
  const url = new URL('https://tablebase.lichess.ovh/standard');
  url.searchParams.set('fen', fen);
  return url.href;
}

export function createTablebaseClient(fetcher = fetch, now = Date.now) {
  const cache = new Map();
  let busy = false, retryAt = 0;
  return (fen, retry = false, begin = false) => {
    let key;
    try { key = tablebaseUrl(fen); }
    catch (error) { return JSON.stringify({error: error.message}); }
    if (retry) cache.delete(key);
    if (cache.has(key)) {
      const entry = cache.get(key);
      // Repaint polling retains the original source; a new lookup is a cache hit.
      if (begin) entry.source = 'cache';
      return JSON.stringify({...entry.result, source: entry.source});
    }
    if (now() < retryAt) return JSON.stringify({error: 'Lichess rate limit: wait one minute before retrying.'});
    if (!busy) {
      busy = true;
      void (async () => {
        let result;
        try {
          const response = await fetcher(key, {credentials: 'omit', cache: 'no-store', referrerPolicy: 'no-referrer', signal: AbortSignal.timeout(15000)});
          if (response.status === 429) { retryAt = now() + 60000; throw new Error('Lichess rate limit: wait one minute before retrying.'); }
          if (response.status === 404) throw new Error('This position is not available in the tablebases.');
          if (!response.ok) throw new Error(`Tablebase request failed (${response.status}).`);
          result = await response.json();
          if (typeof result.category !== 'string' || !Array.isArray(result.moves) || result.moves.some(m => typeof m.uci !== 'string' || typeof m.san !== 'string' || typeof m.category !== 'string'))
            throw new Error('Unexpected tablebase response.');
        } catch (error) {
          result = {error: error.name === 'TimeoutError' ? 'Tablebase request timed out. Retry when ready.' : error instanceof TypeError ? 'Could not load tablebases. Check your connection and retry.' : error.message};
        } finally { busy = false; }
        if (cache.size >= 100) cache.delete(cache.keys().next().value);
        cache.set(key, {result, source: 'lichess'});
      })();
    }
    return '';
  };
}
export function createLocalTablebaseClient(configuration, fetcher = fetch) {
  let pending = '', result = '', serial = 0;
  return (fen, retry = false, begin = false) => {
    const config = configuration();
    if (!config) { serial++; pending = ''; result = ''; return JSON.stringify({error: 'Connect Companion using Manage connection, then look up this position again.'}); }
    const key = JSON.stringify([config.endpoint, config.credential, fen]);
    if (begin || retry || pending !== key) {
      pending = key; result = ''; const request = ++serial;
      void new CompanionArchiveClient(config.endpoint, config.credential, fetcher)
        .request(`tablebases/probe?fen=${encodeURIComponent(fen)}`)
        .then(value => { if (request === serial) result = JSON.stringify({...value, source: 'companion'}); })
        .catch(error => { if (request === serial) result = JSON.stringify({error: error.message}); });
    }
    return result;
  };
}
if (typeof window !== 'undefined') {
  const configuration = companionConnection;
  const local = createLocalTablebaseClient(configuration);
  const online = createTablebaseClient();
  async function connect() {
  if (window.ironwoodCompanionFeatureEnabled?.() === false) return;
    if (!await requireCompanionConnection()) return;
    if (document.getElementById('tablebase-connection')) return;
    if (!document.getElementById('companion-archive-style')) { const style = document.createElement('link'); style.id = 'companion-archive-style'; style.rel = 'stylesheet'; style.href = '/companion-archive.css'; document.head.append(style); }
    const dialog = document.createElement('dialog'); dialog.id = 'tablebase-connection';
    dialog.innerHTML = `<p class="eyebrow">IRONWOOD COMPANION</p><h2>Local endgame tablebases</h2><p data-connection></p><p role="status"></p><button type="button" data-manage>Manage connection</button> <button type="button" data-close>Close</button>`;
    dialog.querySelector('[data-connection]').textContent = `Connected to ${configuration().endpoint}`;
    dialog.querySelector('[data-close]').onclick = () => dialog.close();
    dialog.querySelector('[data-manage]').onclick = async () => { dialog.close(); await manageCompanionConnection(); };
    dialog.onclose = () => dialog.remove();
    const shared = configuration();
    void new CompanionArchiveClient(shared.endpoint, shared.credential).request('tablebases').then(coverage => {
      if (!dialog.isConnected || configuration()?.credential !== shared.credential || configuration()?.endpoint !== shared.endpoint) return;
      const materials = Object.entries(coverage.materials).map(([m, files]) => `${m}: ${files.join(', ')}`).join('; ');
      dialog.querySelector('[role=status]').textContent = `${materials || 'No table files registered.'} Rejected files: ${coverage.rejectedFiles}; unavailable directories: ${coverage.unavailableDirectories}. ${coverage.validation} Close this window and look up the position again.`;
    }).catch(error => { if (dialog.isConnected) dialog.querySelector('[role=status]').textContent = error.message; });
    document.body.append(dialog); dialog.showModal();
  }
  window.ironwoodTablebase = (fen, retry, begin, useLocal = false) => {
    if (useLocal && fen === '') { connect(); return ''; }
    if (useLocal && !configuration() && begin) connect();
    return useLocal ? local(fen, retry, begin) : online(fen, retry, begin);
  };
}
