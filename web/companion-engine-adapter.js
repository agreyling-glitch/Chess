// Translate the existing browser UCI boundary into the typed Companion protocol.
// Credentials remain in memory. No command line or engine path crosses this API.
const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
export class CompanionEngineAdapter {
  constructor(emit, fail, { fetcher = fetch, Socket = WebSocket } = {}) {
    Object.assign(this, { emit, fail, fetcher, Socket });
    this.queue = Promise.resolve(); this.options = {}; this.overrides = {}; this.position = { fen: START, moves: [] };
    this.counter = 0; this.active = null; this.closed = false;
  }
  async connect({ endpoint, credential, engineId, overrides = {} }) {
    const base = new URL(endpoint);
    if (!['http:', 'https:'].includes(base.protocol) || base.username || base.password || base.pathname !== '/' || base.search || base.hash) throw new Error('Invalid Companion address');
    const response = await (0, this.fetcher)(new URL('/api/v1/tickets', base), { method: 'POST', headers: { Authorization: `Bearer ${credential}`, 'X-Ironwood-Protocol': '1' }, credentials: 'omit', redirect: 'error', cache: 'no-store', signal: AbortSignal.timeout(8000) });
    credential = '';
    if (!response.ok) throw new Error(`Companion connection rejected (${response.status})`);
    const { ticket } = await response.json();
    const address = new URL('/api/v1/socket', base); address.protocol = base.protocol === 'https:' ? 'wss:' : 'ws:';
    this.socket = new this.Socket(address);
    this.overrides = { ...overrides };
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Engine discovery timed out')), 12000);
      this.socket.onopen = () => this.send({ type: 'authenticate', ticket });
      this.socket.onerror = () => { clearTimeout(timer); reject(new Error('Companion connection failed')); this.fatal('Companion connection failed'); };
      this.socket.onclose = () => { clearTimeout(timer); reject(new Error('Companion disconnected')); this.fatal('Companion disconnected; reconnect explicitly to resume'); };
      this.socket.onmessage = ({ data }) => {
        try {
          const message = JSON.parse(data);
          if (message.type === 'ready') {
            if (message.protocol_version !== '1') throw new Error('Unsupported Companion protocol');
            this.send({ type: 'open_engine', request_id: 'open', engine_id: engineId });
          }
          else if (message.type === 'engine_opened') {
            clearTimeout(timer); this.session = message.session_id; this.details = message.engine;
            this.heartbeat = setInterval(() => this.send({ type: 'ping', request_id: 'heartbeat' }), 10000);
            resolve(this.details);
          } else if (message.type === 'option_set' && message.session_id === this.session) {
            this.optionSet?.(message.request_id);
          } else if (message.type === 'error') {
            clearTimeout(timer); reject(new Error(message.message)); this.fatal(message.message);
          } else this.message(message);
        } catch { reject(new Error('Invalid Companion response')); this.fatal('Invalid Companion response'); }
      };
    });
    for (const option of this.details.options) {
      const parts = [`option name ${option.name} type ${option.kind}`];
      if (option.kind !== 'button') parts.push(`default ${option.default || '<empty>'}`);
      if (option.min != null) parts.push(`min ${option.min}`);
      if (option.max != null) parts.push(`max ${option.max}`);
      for (const choice of option.choices || []) parts.push(`var ${choice}`);
      this.emit(parts.join(' '));
    }
    this.emit('uciok');
    return this.details;
  }
  send(message) { if (!this.closed && this.socket?.readyState === 1) this.socket.send(JSON.stringify(message)); }
  fatal(message) {
    if (this.closed) return;
    this.close(); this.fail(`Companion worker failed: ${message}`);
  }
  close() {
    this.closed = true; clearInterval(this.heartbeat);
    this.cancelOption?.();
    this.active?.release?.(); this.active = null;
    this.socket?.close();
  }
  async release() {
    if (!this.session || this.closed) { this.close(); return; }
    await new Promise(resolve => {
      const timer = setTimeout(resolve, 4000);
      this.released = () => { clearTimeout(timer); resolve(); };
      this.send({ type: 'close_engine', session_id: this.session });
    });
    this.close();
  }
  message(message) {
    if (message.type === 'engine_closed' && message.session_id === this.session) { this.released?.(); return; }
    const active = this.active;
    if (!active || message.session_id !== this.session || message.search_id !== active.search_id) return;
    if (message.type === 'cancelled') {
      this.active = null; this.emit('bestmove 0000'); active.release?.(); return;
    }
    if (message.position_id !== active.position_id) return;
    if (message.type === 'info' && !active.suppressed) {
      const info = message.info;
      const parts = ['info'];
      for (const key of ['depth', 'nodes', 'multipv']) if (Number.isSafeInteger(info[key])) parts.push(key, info[key]);
      if (info.score?.perspective === 'side_to_move') {
        parts.push('score', info.score.kind, info.score.value);
        if (info.score.bound) parts.push(info.score.bound);
      }
      if (info.pv?.length) parts.push('pv', ...info.pv);
      this.emit(parts.join(' '));
    } else if (message.type === 'bestmove') {
      this.active = null;
      this.emit(active.suppressed ? 'bestmove 0000' : `bestmove ${message.bestmove || '0000'}`);
      active.release?.();
    }
  }
  command(line) {
    if (line === 'stop' && this.active) this.active.suppressed = true;
    this.queue = this.queue.then(() => this.dispatch(line)).catch(error => this.fatal(error.message));
    return this.queue;
  }
  async dispatch(line) {
    if (this.closed) return;
    if (line === 'stop') {
      if (this.active) {
        const active = this.active;
        const completed = new Promise(resolve => { active.release = resolve; });
        this.send({ type: 'cancel', session_id: this.session, search_id: active.search_id });
        await completed;
      }
    } else if (line === 'isready') this.emit('readyok');
    else if (line === 'ucinewgame') this.newGame = true;
    else if (line === 'quit') this.close();
    else if (line.startsWith('setoption name ')) {
      const button = this.details.options.find(option => option.kind === 'button' && line === `setoption name ${option.name}`);
      if (button) {
        if (this.active) throw new Error('Stop analysis before running an engine action');
        const request_id = `option-${++this.counter}`;
        await new Promise((resolve, reject) => {
          const timer = setTimeout(() => { this.optionSet = null; this.cancelOption = null; reject(new Error('Engine action timed out')); }, 4000);
          this.cancelOption = () => { clearTimeout(timer); this.optionSet = null; this.cancelOption = null; reject(new Error('Engine disconnected during action')); };
          this.optionSet = id => { if (id === request_id) { clearTimeout(timer); this.optionSet = null; this.cancelOption = null; resolve(); } };
          this.send({type:'set_option',session_id:this.session,request_id,name:button.name,value:''});
        });
        return;
      }
      const match = /^setoption name (.+?) value (.*)$/.exec(line);
      if (!match) return;
      if (match[1] === 'UCI_Chess960') { this.chess960 = match[2] === 'true'; return; }
      // Resource values for server engines come from their UCI settings, not
      // the browser Stockfish controls. Explicit overrides are applied below.
      if (['Threads', 'Hash'].includes(match[1])) return;
      // Stockfish's limiter defaults must never silently weaken another engine.
      if (['Skill Level', 'UCI_LimitStrength', 'UCI_Elo', 'Ponder'].includes(match[1])) return;
      if (this.details.options.some(option => option.name === match[1] && option.kind !== 'button')) this.options[match[1]] = match[2];
    } else if (line.startsWith('position ')) {
      const text = line.slice(9).replace(/^startpos/, `fen ${START}`);
      const [fen, moves = ''] = text.replace(/^fen /, '').split(' moves ');
      this.position = { fen, moves: moves.trim() ? moves.trim().split(/\s+/) : [] };
    } else if (line.startsWith('go ')) {
      if (this.active) throw new Error('Previous search has not drained');
      const words = line.split(/\s+/).slice(1);
      let limit;
      if (words[0] === 'infinite') limit = { kind: 'infinite' };
      else if (words[0] === 'movetime') limit = { kind: 'time', milliseconds: Number(words[1]) };
      else if (words[0] === 'depth') limit = { kind: 'depth', plies: Number(words[1]) };
      else if (words[0] === 'nodes') limit = { kind: 'nodes', nodes: Number(words[1]) };
      else if (words[0] === 'wtime') {
        const values = Object.fromEntries(Array.from({ length: words.length / 2 }, (_, i) => [words[i * 2], Number(words[i * 2 + 1])]));
        limit = { kind: 'clock', white_ms: values.wtime, black_ms: values.btime, white_increment_ms: values.winc || 0, black_increment_ms: values.binc || 0 };
      } else throw new Error('Unsupported search limit');
      const options = { ...this.options };
      // Adapt stock browser resource defaults to advertised ranges. Explicit
      // Companion option overrides are left intact and validated by the server.
      for (const name of ['Threads', 'Hash', 'MultiPV']) {
        const option = this.details.options.find(o => o.name === name);
        if (option && name in options) options[name] = String(Math.max(option.min ?? 1, Math.min(Number(options[name]), option.max ?? Number(options[name]))));
      }
      Object.assign(options, this.overrides);
      const search_id = `search-${++this.counter}`, position_id = `position-${this.counter}`;
      this.active = { search_id, position_id, suppressed: false };
      this.send({ type: 'search', session_id: this.session, search_id, position_id, ...this.position,
        chess960: Boolean(this.chess960), new_game: Boolean(this.newGame), options, limit });
      this.newGame = false;
    }
  }
}
