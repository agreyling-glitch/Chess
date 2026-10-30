// Owner credentials are held only by this in-memory client.
export function archiveCredential(endpoint, typed, archive, engine) {
  const origin = new URL(endpoint).origin;
  const credential = typed || (archive?.endpoint === origin ? archive.credential : '') || (engine?.endpoint === origin ? engine.credential : '');
  if (!credential) throw new Error('Enter the owner credential for this Companion address.');
  return credential;
}
export class CompanionArchiveClient {
  constructor(endpoint, credential, fetcher = fetch) {
    const url = new URL(endpoint);
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.pathname !== '/' || url.search || url.hash) throw new Error('Enter a Companion origin such as http://127.0.0.1:8790');
    this.endpoint = url.origin; this.credential = credential; this.fetcher = fetcher;
  }
  async request(path, body) {
    const response = await (0, this.fetcher)(`${this.endpoint}/api/v1/${path}`, {
      method: body === undefined ? 'GET' : 'POST',
      headers: { Authorization: `Bearer ${this.credential}`, 'X-Ironwood-Protocol': '1', ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
      body: body === undefined ? undefined : JSON.stringify(body), credentials: 'omit', cache: 'no-store', redirect: 'error', signal: AbortSignal.timeout(30000),
    });
    const number=name=>{const raw=response.headers?.get(name);return raw!==null&&raw!==undefined&&/^\d+$/.test(raw)?Number(raw):NaN;};
    const limit=number('x-ironwood-operations-limit'),remaining=number('x-ironwood-operations-remaining'),reset=number('x-ironwood-operations-reset-ms');
    this.importBudget=Number.isSafeInteger(limit)&&limit>0&&limit<=1000000&&remaining>=0&&remaining<=limit&&reset>=0&&reset<=60000
      ? {limit,remaining,resetAt:Date.now()+reset,busy:response.headers?.get('x-ironwood-engine-busy')==='1'} : undefined;
    let value;
    try { value = await response.json(); }
    catch { if (response.ok) throw new Error('Companion returned an invalid response.'); value = {}; }
    if (!response.ok) {
      const error = new Error(value.code === 'conflict' ? `${value.message}. Your changes were not saved.` : value.message || `Companion returned ${response.status}`);
      error.status = response.status; error.code=value.code;
      const retry = response.headers?.get('retry-after');
      if (retry) {
        const milliseconds = /^\d+$/.test(retry) ? Number(retry) * 1000 : Date.parse(retry) - Date.now();
        if (Number.isFinite(milliseconds)) error.retryAfterMs = Math.min(120000, Math.max(1000, milliseconds));
      }
      throw error;
    }
    if(body!==undefined && /^archive\/(import|restore|games\/[^/]+\/(update|delete)|collections(?:\/[^/]+)?)$/.test(path) && globalThis.window?.CustomEvent && window.dispatchEvent) {
      const deleted=path.match(/^archive\/games\/([^/]+)\/delete$/);
      window.dispatchEvent(new window.CustomEvent('ironwood-game-library-changed',{detail:{source:'archive',endpoint:this.endpoint,...(deleted?{deletedIds:[deleted[1]]}:{})}}));
    }
    return value;
  }
}
export function browserDocuments(records, exporter, details) {
  return records.map(record => {
    const info = details(record.json), game = JSON.parse(record.json);
    const metadata = { White: info.white, Black: info.black, Result: info.result };
    if (info.playedAt) metadata.Date = info.playedAt.replaceAll('-', '.');
    if (info.trainingProfileId) metadata.TrainingProfileId = info.trainingProfileId;
    for (const name of ['ECO', 'Opening', 'Variant', 'FEN', 'WhiteElo', 'BlackElo']) {
      const match = game.review_pgn?.match(new RegExp(`\\[${name}\\s+"((?:[^"\\\\]|\\\\.)*)"\\]`));
      if (match) metadata[name] = match[1];
    }
    Object.assign(metadata,record.metadata || {});
    const {archiveAnalysisRuns,collectionIds,...original}=record;
    return { format: 'ironwood-companion-game', version: 1, record: structuredClone(original), pgn: exporter(record.json, 'annotated'), metadata,...(archiveAnalysisRuns?.length?{analysis_runs:archiveAnalysisRuns}:{}) };
  });
}
export function transferDocuments(value) {
  if (value?.format !== 'ironwood-companion-transfer' || value.version !== 1 || !Array.isArray(value.documents)) throw new Error('Unsupported Companion transfer version. Choose a Companion JSON export or a PGN file.');
  if (!value.documents.length || value.documents.length > 1000) throw new Error('Import 1 to 1000 games per batch.');
  if (value.documents.some(d => d?.format !== 'ironwood-companion-game' || d.version !== 1)) throw new Error('Unsupported game version.');
  return value.documents;
}
