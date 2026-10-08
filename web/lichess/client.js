// Original Ironwood client. Uses the public Lichess API; no SDK or assets bundled.
export const HOST = 'https://lichess.org';
export const SCOPES = 'board:play challenge:read challenge:write';
export const CLIENT_ID = 'ironwood-chess';
const ACTIVE = new Set(['created', 'started']);
export const isPlaying = state => ACTIVE.has(state?.status);
export const isCompatible = game => game.compat?.board !== false
  && !['bullet', 'ultraBullet'].includes(game.speed)
  && ['standard', 'fromPosition', 'chess960'].includes(game.variant?.key || 'standard');

export async function readNdjson(response, onEvent, signal, onChunk = () => {}) {
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  try {
    while (!signal?.aborted) {
      const { value, done } = await reader.read();
      onChunk();
      buffer += decoder.decode(value, { stream: !done });
      let newline;
      while ((newline = buffer.indexOf('\n')) >= 0) {
        const line = buffer.slice(0, newline).trim();
        buffer = buffer.slice(newline + 1);
        if (line) onEvent(JSON.parse(line));
      }
      if (buffer.length > 1048576) throw new Error('Lichess stream record is too large');
      if (done) {
        if (buffer.trim()) onEvent(JSON.parse(buffer));
        break;
      }
    }
  } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
}

export function createClient({ fetch: request = globalThis.fetch, emit, now = Date.now,
  delay = (ms, signal) => new Promise(resolve => {
    if (signal?.aborted) return resolve();
    const timer = setTimeout(finish, ms);
    function finish() { clearTimeout(timer); signal?.removeEventListener('abort', finish); resolve(); }
    signal?.addEventListener('abort', finish, { once: true });
  }), invalidToken = () => {} }) {
  let token = '', account = null, generation = 0, cooldown = 0;
  let eventStream = null, gameStream = null, seekStream = null, game = null;
  let tail = Promise.resolve(), pendingMove = false, outgoing = [];
  let sessionController = new AbortController();
  const state = { connected: false, account: '', status: 'Sign in to play on Lichess',
    seeking: false, challenges: [], outgoing: [], games: [], chat: [], game: null };
  const publish = () => emit({ type: 'lobby', ...state, outgoing: [...outgoing] });
  const headers = () => ({ Authorization: `Bearer ${token}` });
  async function checked(path, options = {}) {
    if (now() < cooldown) throw new Error('Lichess rate limit: wait at least one minute before retrying');
    const response = await request(HOST + path, { credentials: 'omit', cache: 'no-store', ...options,
      headers: { ...headers(), ...options.headers } });
    if (response.status === 429) {
      cooldown = now() + Math.max(60000, (Number(response.headers.get('Retry-After')) || 0) * 1000);
    }
    if (response.status === 401) { invalidToken(); disconnect(); }
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      const error = new Error(response.status === 429 ? 'Lichess rate limit: wait at least one minute before retrying'
        : response.status === 401 ? 'Lichess sign-in expired. Sign in again.'
        : body.error || `Lichess request failed (${response.status})`);
      error.status = response.status;
      throw error;
    }
    return response;
  }
  // Serialize finite requests. The account/game/seek streams must remain open concurrently.
  function api(path, params, method = 'POST') {
    const session = generation;
    const task = tail.then(async () => {
      if (session !== generation || !token) throw new Error('Lichess session changed');
      const response = await checked(path, { method, headers: { Accept: 'application/json' },
        signal: sessionController.signal,
        ...(params ? { body: new URLSearchParams(params) } : {}) });
      if (session !== generation) throw new Error('Lichess session changed');
      return response.status === 204 ? {} : response.json();
    });
    tail = task.catch(() => {});
    return task;
  }
  function error(err) {
    if (err.name === 'AbortError') return;
    state.status = err.message; emit({ type: 'error', message: err.message }); publish();
  }
  async function stream(path, controller, handle, isCurrent, onReady = () => {}) {
    let backoff = 1000;
    while (!controller.signal.aborted && isCurrent()) {
      // Lichess sends heartbeat lines. Recover even if a broken network leaves fetch pending.
      const attempt = new AbortController();
      const abort = () => attempt.abort();
      controller.signal.addEventListener('abort', abort, { once: true });
      let watchdog;
      const heartbeat = () => { clearTimeout(watchdog); watchdog = setTimeout(abort, 45000); };
      heartbeat();
      try {
        const response = await checked(path, { signal: attempt.signal });
        if (!isCurrent()) break;
        onReady();
        await readNdjson(response, event => { if (isCurrent()) handle(event); }, attempt.signal, heartbeat);
        backoff = 1000;
      } catch (err) {
        if (!controller.signal.aborted && isCurrent()) {
          error(err);
          if ([400, 401, 403, 404].includes(err.status)) { controller.abort(); return; }
        }
      } finally {
        clearTimeout(watchdog); controller.signal.removeEventListener('abort', abort);
        attempt.abort();
      }
      if (controller.signal.aborted || !isCurrent()) break;
      state.status = 'Reconnecting to Lichess…'; publish();
      if (controller === eventStream) { state.connected = false; publish(); }
      await delay(Math.max(backoff, cooldown - now()), controller.signal);
      backoff = Math.min(30000, backoff * 2);
    }
  }
  function gameEvent(event) {
    if (event.type === 'gameFull') {
      if (event.variant?.key !== 'standard' && event.variant?.key !== 'fromPosition' && event.variant?.key !== 'chess960') {
        state.status = 'This Lichess variant is not supported by Ironwood. Open it on Lichess.';
        game = null; state.game = null;
        gameStream?.abort(); publish(); return;
      }
      game = { id: event.id, variant: event.variant, initialFen: event.initialFen, white: event.white, black: event.black,
        clock: event.clock, date: new Date(event.createdAt || now()).toISOString().slice(0, 10).replaceAll('-', '.'),
        rated: !!event.rated, speed: event.speed, state: event.state,
        side: event.black?.id?.toLowerCase() === account.id.toLowerCase() ? 'black' : 'white' };
      state.game = game;
      const listed = state.games.find(g => g.gameId === game.id);
      if (listed) {
        listed.color = game.side; listed.rated = game.rated; listed.speed = game.speed;
        if (Number.isFinite(event.createdAt)) listed.startedLabel = new Date(event.createdAt).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
      }
    } else if (event.type === 'gameState' && game) game.state = event;
    else if (event.type === 'chatLine' && event.room === 'player') {
      state.chat.push({ user: event.username, text: event.text });
      state.chat = state.chat.slice(-100); publish(); return;
    } else if (event.type === 'opponentGone') {
      state.opponentGone = event;
      state.status = event.gone ? 'Opponent disconnected' : 'Opponent returned'; publish(); return;
    } else return;
    if (pendingMove && (game.state.moves !== pendingMove.moves || !isPlaying(game.state))) pendingMove = false;
    emit({ type: 'game', ...game, pendingMove: !!pendingMove });
    if (!isPlaying(game.state)) {
      state.games = state.games.filter(item => item.gameId !== game.id);
      gameStream?.abort(); state.status = `Game finished: ${game.state.status}`;
    } else {
      const count = game.state.moves.trim() ? game.state.moves.trim().split(/\s+/).length : 0;
      const startsBlack = game.initialFen !== 'startpos' && game.initialFen?.split(' ')[1] === 'b';
      const turn = (count + (startsBlack ? 1 : 0)) % 2 ? 'black' : 'white';
      const listed = state.games.find(g => g.gameId === game.id);
      if (listed) listed.isMyTurn = turn === game.side;
      state.status = pendingMove ? 'Waiting for Lichess to confirm move…' : turn === game.side ? 'Your move' : "Opponent's move";
    }
    publish();
  }
  function readOngoingMetadata(id) {
    const session = generation;
    const task = tail.then(async () => {
      if (session !== generation || !token) throw new Error('Lichess session changed');
      const controller = new AbortController();
      const signal = AbortSignal.any([controller.signal, sessionController.signal, AbortSignal.timeout(20000)]);
      let details;
      try {
        const response = await checked(`/api/board/game/stream/${id}`, { signal });
        await readNdjson(response, event => {
          if (event.type === 'gameFull') { details = event; controller.abort(); }
        }, signal);
      } catch (error) { if (!details) throw error; }
      finally { controller.abort(); }
      if (session !== generation || !details) throw new Error('Could not read game details');
      return details;
    });
    tail = task.catch(() => {});
    return task;
  }
  async function enrichOngoingGames() {
    const session = generation;
    for (const item of [...state.games]) {
      if (session !== generation || !token || item.startedLabel) continue;
      try {
        let details;
        try { details = await api(`/game/export/${item.gameId}?moves=false&clocks=false&evals=false&opening=false&division=false`, null, 'GET'); }
        catch (error) { if (error.status === 401 || now() < cooldown || session !== generation) throw error; }
        if (!Number.isFinite(details?.createdAt)) details = await readOngoingMetadata(item.gameId);
        if (session !== generation) return;
        const current = state.games.find(g => g.gameId === item.gameId);
        if (!current) continue;
        if (Number.isFinite(details.createdAt)) current.startedLabel = new Date(details.createdAt).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
        if (details.daysPerTurn) current.daysPerTurn = details.daysPerTurn;
        if (details.clock) current.clock = details.clock;
        publish();
      } catch { if (session !== generation || now() < cooldown) return; }
    }
  }
  function openGame(id) {
    if (!/^[a-zA-Z0-9]{8}$/.test(id)) throw new Error('Invalid Lichess game ID');
    const listed = state.games.find(g => g.gameId === id);
    if (listed && !isCompatible(listed)) throw new Error('This game must be played on Lichess');
    if (game?.id === id && isPlaying(game.state) && gameStream && !gameStream.signal.aborted) return;
    gameStream?.abort(); game = null; state.game = null; state.chat = []; state.opponentGone = null; pendingMove = false;
    cancelSeek();
    const controller = gameStream = new AbortController(), session = generation;
    state.status = 'Loading Lichess game…'; emit({ type: 'loading', id }); publish();
    void stream(`/api/board/game/stream/${id}`, controller, gameEvent,
      () => generation === session && gameStream === controller && !controller.signal.aborted);
  }
  function leaveCorrespondence() {
    if (!game || game.speed !== 'correspondence' || pendingMove) throw new Error('Wait for a confirmed correspondence move before returning to the lobby');
    gameStream?.abort(); gameStream = null; game = null; state.game = null;
    state.chat = []; state.opponentGone = null;
    state.status = 'Connected to Lichess';
    emit({ type: 'parked' }); publish();
  }
  function accountEvent(event) {
    if (event.type === 'gameStart') {
      const item = event.game;
      const id = item.gameId || item.id;
      const alreadyOngoing = state.games.some(g => g.gameId === id);
      if (!alreadyOngoing) state.games.push({ ...item, gameId: id });
      if (!alreadyOngoing) void enrichOngoingGames();
      outgoing = outgoing.filter(c => c.id !== id);
      state.challenges = state.challenges.filter(c => c.id !== id);
      if (!alreadyOngoing && isCompatible(item) && (!game || !isPlaying(game.state))) openGame(id);
      else if (!isCompatible(item)) state.status = 'This game must be played on Lichess';
    } else if (event.type === 'gameFinish') {
      state.games = state.games.filter(g => g.gameId !== (event.game.gameId || event.game.id));
      // The game stream carries the authoritative final move list and result.
    } else if (event.type === 'challenge') {
      const item = event.challenge;
      if (item.challenger?.id?.toLowerCase() !== account.id.toLowerCase()
        && !state.challenges.some(c => c.id === item.id)) state.challenges.push(item);
    } else if (event.type === 'challengeCanceled' || event.type === 'challengeDeclined') {
      state.challenges = state.challenges.filter(c => c.id !== event.challenge.id);
      outgoing = outgoing.filter(c => c.id !== event.challenge.id);
      state.status = event.type === 'challengeDeclined' ? 'Challenge declined' : 'Challenge canceled';
    }
    publish();
  }
  async function connect(accessToken) {
    disconnect(); token = accessToken;
    const session = generation;
    try {
      const user = await api('/api/account', null, 'GET');
      if (session !== generation) return;
      account = user; state.account = user.username; state.status = 'Connecting to Lichess…'; publish();
      const playing = await api('/api/account/playing', null, 'GET');
      if (session !== generation) return;
      state.games = playing.nowPlaying || [];
      const challenges = await api('/api/challenge', null, 'GET');
      if (session !== generation) return;
      state.challenges = challenges.in || []; outgoing = challenges.out || [];
      const controller = eventStream = new AbortController();
      void stream('/api/stream/event', controller, accountEvent,
        () => session === generation && eventStream === controller,
        () => { state.connected = true; state.status = 'Connected to Lichess'; publish(); });
      void enrichOngoingGames();
    } catch (err) { if (session === generation) error(err); }
  }
  function cancelSeek() { seekStream?.abort(); seekStream = null; state.seeking = false; publish(); }
  function disconnect() {
    generation++; eventStream?.abort(); gameStream?.abort(); seekStream?.abort();
    sessionController.abort(); sessionController = new AbortController();
    eventStream = gameStream = seekStream = null; token = ''; account = null; game = null;
    pendingMove = false; outgoing = [];
    Object.assign(state, { connected: false, account: '', seeking: false, challenges: [], outgoing: [],
      games: [], chat: [], game: null, opponentGone: null, status: 'Disconnected from Lichess' }); publish();
  }
  async function seek(options) {
    if (!state.connected || state.seeking || isPlaying(game?.state)) throw new Error('Connect before finding a new game');
    if (!options.days && (Number(options.time) * 60 + Number(options.increment) * 40 < 480))
      throw new Error('Lichess matchmaking requires rapid or slower (at least 8 minutes estimated duration)');
    const controller = seekStream = new AbortController(), session = generation;
    state.seeking = true; state.status = 'Looking for an opponent…'; publish();
    try {
      const response = await checked('/api/board/seek', { method: 'POST', signal: controller.signal,
        body: new URLSearchParams({ ...options, variant: 'standard' }) });
      if (options.days) {
        await response.json();
        state.status = 'Correspondence seek created. It remains on Lichess until matched.';
      } else await readNdjson(response, () => {}, controller.signal);
    } finally {
      if (session === generation && seekStream === controller) {
        state.seeking = false; seekStream = null;
        if (state.status === 'Looking for an opponent…') state.status = 'Seek ended';
        publish();
      }
    }
  }
  async function move(uci) {
    if (!game || !isPlaying(game.state) || pendingMove || !/^[a-h][1-8][a-h][1-8][qrbn]?$/.test(uci)) {
      emit({ type: 'moveRejected', id: game?.id });
      throw new Error('Cannot send this move');
    }
    const id = game.id, moves = game.state.moves;
    pendingMove = { moves };
    state.status = 'Waiting for Lichess to confirm move…'; publish();
    try { await api(`/api/board/game/${id}/move/${uci}`); }
    catch (err) {
      pendingMove = false;
      emit({ type: 'moveRejected', id });
      // A network failure may happen after the move was accepted: resync instead of retrying it.
      if (game?.id === id && game.state.moves === moves) {
        gameStream?.abort(); openGame(id);
      }
      throw err;
    }
  }
  async function challenge(username, options) {
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(username)) throw new Error('Enter a valid Lichess username');
    const result = await api(`/api/challenge/${encodeURIComponent(username)}`, { ...options, variant: 'standard' });
    if (result.challenge) outgoing.push(result.challenge);
    state.status = `Challenge sent to ${username}`; publish();
  }
  async function challengeAction(id, action) {
    if (!/^[a-zA-Z0-9]{8}$/.test(id) || !['accept', 'decline', 'cancel'].includes(action)) throw new Error('Invalid challenge');
    await api(`/api/challenge/${id}/${action}`);
    state.challenges = state.challenges.filter(c => c.id !== id);
    outgoing = outgoing.filter(c => c.id !== id); publish();
  }
  async function challengeAi(options) {
    const variant = options.variant || 'standard';
    if (!['standard', 'chess960', 'fromPosition'].includes(variant)) throw new Error('Unsupported computer variant');
    if (variant === 'fromPosition' && !options.fen?.trim()) throw new Error('Enter a FEN position');
    const result = await api('/api/challenge/ai', { ...options, variant });
    // gameStart normally arrives first; openGame is idempotent while that stream is active.
    if (result.id && (!game || !isPlaying(game.state))) openGame(result.id);
  }
  async function action(action, text) {
    if (!game || !isPlaying(game.state)) throw new Error('No active Lichess game');
    if (!['resign', 'abort', 'draw/yes', 'draw/no', 'takeback/yes', 'takeback/no', 'claim-victory', 'claim-draw', 'chat'].includes(action))
      throw new Error('Unsupported game action');
    await api(`/api/board/game/${game.id}/${action}`, action === 'chat' ? { room: 'player', text: text.slice(0, 140) } : null);
  }
  return { connect, disconnect, seek, cancelSeek, openGame, leaveCorrespondence, move, challenge, challengeAi, challengeAction, action,
    revoke: () => api('/api/token', null, 'DELETE'), reportError: error, snapshot: () => structuredClone(state) };
}
