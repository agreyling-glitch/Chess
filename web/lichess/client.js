// Original Ironwood client. Uses the public Lichess API; no SDK or assets bundled.
export const HOST = 'https://lichess.org';
export const SCOPES = 'board:play challenge:read challenge:write follow:read follow:write';
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
  const state = { connected: false, account: '', profile: null, status: 'Sign in to play on Lichess',
    seeking: false, challenges: [], outgoing: [], games: [], chat: [], game: null, ratingHistory: null, historyUsername: '', historyLoading: false, historyError: '', playerProfile: null, recentGames: null, profileUsername: '', profileLoading: false, profileError: '', following: null, followingLoading: false, followBusy: false, followError: '', followPermissionNeeded: false, profileWindows: {} };
  const publish = () => emit({ type: 'lobby', ...state, outgoing: [...outgoing] });
  const headers = () => ({ Authorization: `Bearer ${token}` });
  async function checked(path, options = {}) {
    // Public game exports reject OPTIONS preflights, so never attach the OAuth header.
    const publicExport = path.startsWith('/game/export/') || path.startsWith('/api/games/export/_ids');
    if (now() < cooldown) throw new Error('Lichess rate limit: wait at least one minute before retrying');
    const response = await request(HOST + path, { credentials: 'omit', cache: 'no-store', ...options,
      headers: { ...(publicExport ? {} : headers()), ...options.headers } });
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
  function api(path, params, method = 'POST', accept = 'application/json', maxRecords = 12) {
    const session = generation;
    const task = tail.then(async () => {
      if (session !== generation || !token) throw new Error('Lichess session changed');
      const response = await checked(path, { method, headers: { Accept: accept },
        signal: sessionController.signal,
        ...(params ? { body: typeof params === 'string' ? params : new URLSearchParams(params) } : {}) });
      if (session !== generation) throw new Error('Lichess session changed');
      if (accept === 'application/x-ndjson') {
        const records = [];
        await readNdjson(response, record => { if (records.length < maxRecords) records.push(record); }, sessionController.signal);
        return records;
      }
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
      account = user; state.account = user.username;
      state.profile = { count: user.count || {}, perfs: user.perfs || {}, playTime: user.playTime || {} };
      state.status = 'Connecting to Lichess…'; publish();
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
  const followedPlayer = user => ({ id: user.id || (user.username || user.name || '').toLowerCase(),
    username: user.username || user.name || user.id || '', title: user.title || '', realName: user.profile?.realName || '' });
  function followFailure(err) {
    state.followPermissionNeeded = err.status === 403;
    state.followError = err.status === 403 ? 'Sign in again to grant access to follow and unfollow players.' : err.message;
  }
  async function loadFollowing(refresh = false) {
    if (!account || state.followingLoading || state.followBusy || (state.following !== null && !refresh)) return;
    const session = generation;
    state.followingLoading = true; state.followError = ''; state.followPermissionNeeded = false; publish();
    try {
      const users = await api('/api/rel/following', null, 'GET', 'application/x-ndjson', Infinity);
      if (session !== generation) return;
      state.following = users.map(followedPlayer).filter(user => user.username)
        .sort((a,b) => a.username.localeCompare(b.username));
      for (const target of [state, ...Object.values(state.profileWindows)]) {
        if (target.playerProfile) target.playerProfile.following = state.following.some(user => user.username.toLowerCase() === target.playerProfile.username.toLowerCase());
      }
    } catch (err) { if (session === generation) followFailure(err); }
    finally { if (session === generation) { state.followingLoading = false; publish(); } }
  }
  async function setFollowing(username, follow) {
    if (!account || state.followBusy || state.followingLoading) return;
    username = String(username || '').trim();
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(username) || typeof follow !== 'boolean') {
      state.followError = 'Enter a valid Lichess username.'; publish(); return;
    }
    if (username.toLowerCase() === account.username.toLowerCase()) {
      state.followError = 'You cannot follow your own account.'; publish(); return;
    }
    const session = generation;
    const profile = [state, ...Object.values(state.profileWindows)].find(target => target.playerProfile?.username?.toLowerCase() === username.toLowerCase())?.playerProfile;
    const viewed = followedPlayer(profile || {username});
    state.followBusy = true; state.followError = ''; state.followPermissionNeeded = false; publish();
    try {
      await api(`/api/rel/${follow ? 'follow' : 'unfollow'}/${encodeURIComponent(username)}`);
      if (session !== generation) return;
      for (const target of [state, ...Object.values(state.profileWindows)]) {
        if (target.playerProfile?.username?.toLowerCase() === username.toLowerCase()) target.playerProfile.following = follow;
      }
      if (state.following !== null) {
        state.following = state.following.filter(user => user.username.toLowerCase() !== username.toLowerCase());
        if (follow) state.following.push(viewed);
        state.following.sort((a,b) => a.username.localeCompare(b.username));
      }
    } catch (err) { if (session === generation) followFailure(err); }
    finally { if (session === generation) { state.followBusy = false; publish(); } }
  }
  async function playerProfile(refresh = false, username = '', windowId = '') {
    windowId = String(windowId || '');
    const target = windowId ? state.profileWindows[windowId] ||= {playerProfile:null,recentGames:null,ratingHistory:null,profileUsername:'',historyUsername:'',profileLoading:false,historyLoading:false,profileError:'',historyError:''} : state;
    if (!account || target.profileLoading || target.historyLoading) return;
    username = String(username || account.username).trim();
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(username)) {
      target.profileError = 'Enter a valid Lichess username.'; publish(); return;
    }
    if (!refresh && target.profileUsername.toLowerCase() === username.toLowerCase()
      && target.playerProfile && target.recentGames !== null && target.ratingHistory !== null && !target.profileError && !target.historyError) return;
    const session = generation;
    const current = () => session === generation && (!windowId || state.profileWindows[windowId] === target);
    const changed = target.profileUsername.toLowerCase() !== username.toLowerCase();
    if (changed) target.performance = {};
    if (changed || refresh) target.activity = null;
    if (changed) { target.playerProfile = null; target.recentGames = null; target.ratingHistory = null; }
    target.profileUsername = username; target.historyUsername = username;
    target.profileLoading = true; target.profileError = ''; target.historyError = ''; publish();
    // Keep requests sequential, and preserve successful sections if another section fails.
    try {
      const profile = await api(`/api/user/${encodeURIComponent(username)}?fideId=true`, null, 'GET');
      if (!current()) return;
      if (!profile.username) throw new Error('Invalid player profile response');
      target.playerProfile = profile; publish();
      await ratingHistory(refresh, username, target);
      if (!current()) return;
      try {
        const games = await api(`/api/games/user/${encodeURIComponent(username)}?max=12&opening=true&lastFen=true&moves=true&pgnInJson=true&clocks=true&ongoing=false`, null, 'GET', 'application/x-ndjson');
        if (!current()) return;
        target.recentGames = games;
      } catch (err) { if (current()) target.profileError = `Recent games: ${err.message}`; }
    } catch (err) { if (current()) target.profileError = err.message; }
    finally { if (current()) { target.profileLoading = false; publish(); } }
  }
  async function activity(username, windowId, refresh = false) {
    const target = state.profileWindows[String(windowId)];
    username = String(username || '').trim();
    if (!account || !target || target.profileUsername.toLowerCase() !== username.toLowerCase()) return;
    if (target.activity?.loading || (!refresh && target.activity?.days && !target.activity.error)) return;
    const entry = target.activity = { loading: true, error: '', days: null };
    const session = generation;
    const current = () => session === generation && state.profileWindows[String(windowId)] === target
      && target.profileUsername.toLowerCase() === username.toLowerCase() && target.activity === entry;
    publish();
    try {
      const days = await api(`/api/user/${encodeURIComponent(username)}/activity`, null, 'GET');
      if (!current()) return;
      if (!Array.isArray(days)) throw new Error('Invalid player activity response.');
      entry.days = days.sort((a,b) => (b.interval?.start || 0) - (a.interval?.start || 0));
    } catch (err) { if (current()) entry.error = err.message; }
    finally { if (current()) { entry.loading = false; publish(); } }
  }
  async function performance(username, perf, windowId, refresh = false) {
    const target = state.profileWindows[String(windowId)];
    username = String(username || '').trim();
    if (!account || !target || target.profileUsername.toLowerCase() !== username.toLowerCase()) return;
    if (!['bullet', 'blitz', 'rapid', 'classical', 'correspondence'].includes(perf)) return;
    target.performance ||= {};
    const previous = target.performance[perf];
    if (previous?.loading || (!refresh && previous?.data && !previous.error && !Object.keys(previous.gameErrors || {}).length)) return;
    const entry = target.performance[perf] = { loading: true, error: '', data: null, games: {}, gameErrors: {} };
    const session = generation;
    const current = () => session === generation && state.profileWindows[String(windowId)] === target
      && target.profileUsername.toLowerCase() === username.toLowerCase() && target.performance?.[perf] === entry;
    publish();
    try {
      const data = await api(`/api/user/${encodeURIComponent(username)}/perf/${perf}`, null, 'GET');
      if (!current()) return;
      if (!data.stat || !data.perf) throw new Error('No performance statistics available for this time control.');
      entry.data = data; publish();
      const ids = [...new Set(['bestWins', 'worstLosses'].flatMap(key =>
        (data.stat[key]?.results || []).slice(0, 5).map(result => result.gameId)))].filter(id => /^[a-zA-Z0-9]{8}$/.test(id));
      if (ids.length) {
        // The /api route supports browser requests and returns all notable games at once.
        const games = await api('/api/games/export/_ids?moves=true&pgnInJson=true&opening=true&clocks=true',
          ids.join(','), 'POST', 'application/x-ndjson', ids.length);
        if (!current()) return;
        for (const game of games) {
          if (ids.includes(game.id) && game.pgn && !['started', 'created'].includes(game.status)) entry.games[game.id] = game;
        }
        for (const id of ids) {
          if (!entry.games[id]) entry.gameErrors[id] = 'Completed game moves are unavailable.';
        }
        publish();
      }
    } catch (err) { if (current()) entry.error = err.name === 'TypeError' ? 'Game previews could not be loaded. Select Refresh to retry.' : err.message; }
    finally { if (current()) { entry.loading = false; publish(); } }
  }
  async function ratingHistory(refresh = false, username = '', target = state) {
    if (!account || target.historyLoading) return;
    username = String(username || account.username).trim();
    if (!/^[a-zA-Z0-9_-]{2,30}$/.test(username)) {
      target.historyError = 'Enter a valid Lichess username.'; publish(); return;
    }
    if (target.ratingHistory !== null && target.historyUsername.toLowerCase() === username.toLowerCase() && !refresh) return;
    if (target.historyUsername.toLowerCase() !== username.toLowerCase()) target.ratingHistory = null;
    target.historyUsername = username;
    const session = generation;
    target.historyLoading = true; target.historyError = ''; publish();
    try {
      const data = await api(`/api/user/${encodeURIComponent(username)}/rating-history`, null, 'GET');
      if (session !== generation) return;
      if (!Array.isArray(data)) throw new Error('Invalid rating history response');
      target.ratingHistory = data;
    } catch (err) {
      if (session !== generation) return;
      target.historyError = err.message;
    } finally {
      if (session === generation) { target.historyLoading = false; publish(); }
    }
  }
  function cancelSeek() { seekStream?.abort(); seekStream = null; state.seeking = false; publish(); }
  function disconnect() {
    generation++; eventStream?.abort(); gameStream?.abort(); seekStream?.abort();
    sessionController.abort(); sessionController = new AbortController();
    eventStream = gameStream = seekStream = null; token = ''; account = null; game = null;
    pendingMove = false; outgoing = [];
    Object.assign(state, { connected: false, account: '', profile: null, seeking: false, challenges: [], outgoing: [],
      games: [], chat: [], game: null, opponentGone: null, ratingHistory: null, historyUsername: '', historyLoading: false, historyError: '', playerProfile: null, recentGames: null, profileUsername: '', profileLoading: false, profileError: '', following: null, followingLoading: false, followBusy: false, followError: '', followPermissionNeeded: false, profileWindows: {}, status: 'Disconnected from Lichess' }); publish();
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
  return { connect, disconnect, closeProfileWindow: id => { delete state.profileWindows[String(id)]; publish(); }, loadFollowing, setFollowing, playerProfile, activity, performance, ratingHistory, seek, cancelSeek, openGame, leaveCorrespondence, move, challenge, challengeAi, challengeAction, action,
    revoke: () => api('/api/token', null, 'DELETE'), reportError: error, snapshot: () => structuredClone(state) };
}
