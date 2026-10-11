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
if (typeof window !== 'undefined') window.ironwoodTablebase = createTablebaseClient();
