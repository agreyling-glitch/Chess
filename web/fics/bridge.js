import { parseStyle12, parseFicsChunk, parseFicsMoveRow } from './protocol.js';

let socket = null;
let buffer = '';
let loginSent = false;
let guestConfirmed = false;
let passwordSent = false;
let ready = false;
let currentGame = null;
const observedGames = new Map();
const moveRequests = [];
let activeMoveRequest = null;
let guestName = '';
let loginMode = 'guest';
let pendingPassword = '';
let loginError = '';
const events = [];

function emit(event) {
  events.push(JSON.stringify(event));
  if (events.length > 150) events.splice(0, events.length - 150);
}

function send(command) {
  if (socket?.readyState === WebSocket.OPEN) socket.send(command);
}

function failLogin(message) {
  loginError = message;
  pendingPassword = '';
  socket?.close();
}

function handleLoginPrompt(text) {
  if (!loginSent && /login:\s*$/i.test(text)) {
    loginSent = true;
    send(loginMode === 'registered' ? guestName : 'guest');
  }
  if (loginMode === 'registered' && loginSent && !passwordSent && /password:\s*$/i.test(text)) {
    passwordSent = true;
    send(JSON.stringify({ type: 'password', value: pendingPassword }));
    pendingPassword = '';
  }
  if (loginMode === 'guest' && loginSent && !guestConfirmed && /(?:press|hit)\s+(?:return|enter)/i.test(text)) {
    guestConfirmed = true;
    send('');
  }
}

function fenFromStyle12(board, fields) {
  const ranks = board.ranks.map(rank => {
    let result = '', empty = 0;
    for (const piece of rank) {
      if (piece === '-') empty++;
      else {
        if (empty) result += String(empty);
        result += piece;
        empty = 0;
      }
    }
    if (empty) result += String(empty);
    return result;
  });
  const castling = ['K', 'Q', 'k', 'q'].filter((_, i) => fields[11 + i] === '1').join('') || '-';
  const epFile = Number(fields[10]);
  const ep = epFile >= 0 && epFile < 8 ? 'abcdefgh'[epFile] + (board.turn === 'W' ? '6' : '3') : '-';
  return `${ranks.join('/')} ${board.turn.toLowerCase()} ${castling} ${ep} ${fields[15]} ${fields[26]}`;
}

function lastCoordinateMove(value) {
  const match = value.match(/([a-h][1-8])-([a-h][1-8])(?:=([QRBN]))?/i);
  return match ? match[1] + match[2] + (match[3] || '').toLowerCase() : '';
}

function requestObservedMoves(game) {
  moveRequests.push(game);
  startNextMoveRequest();
}

function startNextMoveRequest() {
  if (activeMoveRequest || !moveRequests.length || !ready) return;
  activeMoveRequest = { game: moveRequests.shift(), moves: [], sawRows: false };
  send(JSON.stringify({ type: 'command', value: `moves ${activeMoveRequest.game}` }));
}

function finishMoveRequest() {
  if (!activeMoveRequest) return;
  const { game, moves, sawRows } = activeMoveRequest;
  activeMoveRequest = null;
  if (sawRows && observedGames.has(game)) emit({ type: 'history', game, moves });
  startNextMoveRequest();
}

function handleLine(line) {
  handleLoginPrompt(line);
  if (loginMode === 'registered' && !ready) {
    if (/(?:invalid|incorrect|wrong).*(?:password|login)|(?:password|login).*(?:invalid|incorrect)/i.test(line)) {
      failLogin('FICS rejected the username or password.');
    }
    if (!/fics%/i.test(line)) return;
  }
  const name = line.match(/\bGuest[A-Z]{4}\b/i);
  if (name && /(?:logged|session|guest|enter)/i.test(line) && !guestName) guestName = name[0];
  if (!ready && /fics%/i.test(line)) finishLogin();
  const promptPrefixed = /^\s*fics%\s*/i.test(line);
  if (promptPrefixed) line = line.replace(/^\s*fics%\s*/i, '');
  if (!line.trim()) return;
  const board = parseStyle12(line);
  if (board && (Math.abs(board.relation) === 1 || board.relation === 0)) {
    const observing = board.relation === 0;
    if (observing && observedGames.get(board.game)?.finished) return;
    if (observing && !observedGames.has(board.game) && observedGames.size >= 10) return;
    const fields = line.trim().split(/\s+/);
    const side = guestName && board.black.toLowerCase() === guestName.toLowerCase()
      ? 'black' : guestName && board.white.toLowerCase() === guestName.toLowerCase()
      ? 'white' : board.flipped ? 'black' : 'white';
    if (observing) {
      if (!observedGames.has(board.game)) requestObservedMoves(board.game);
      observedGames.set(board.game, { white: board.white, black: board.black, finished: false, ended: false });
    }
    else currentGame = board.game;
    emit({ type: 'board', fen: fenFromStyle12(board, fields), game: board.game,
      white: board.white, black: board.black, side, observing,
      last_move: lastCoordinateMove(board.lastMove),
      last_san: board.lastSan,
      move_number: board.moveNumber,
      white_time: board.whiteTime, black_time: board.blackTime });
    return;
  }
  if (activeMoveRequest) {
    const row = parseFicsMoveRow(line);
    if (row) {
      activeMoveRequest.moves.push(...row);
      activeMoveRequest.sawRows = true;
    } else if (/^\s*\{[^}]*\}\s+(?:1-0|0-1|1\/2-1\/2|\*)\s*$/.test(line)) {
      finishMoveRequest();
    }
  }
  const removed = line.match(/^Removing game (\d+) from observation list\./i);
  if (removed && observedGames.has(Number(removed[1]))) {
    const game = Number(removed[1]);
    if (!observedGames.get(game).finished) {
      observedGames.get(game).finished = true;
      emit({ type: 'observationstopped', game });
    }
  }
  const ended = line.match(/^\{Game (\d+) .*\} (?:1-0|0-1|1\/2-1\/2|\*)/);
  if (ended && Number(ended[1]) === currentGame) {
    currentGame = null;
    emit({ type: 'end', message: line.trim() });
  } else if (ended && observedGames.has(Number(ended[1]))) {
    const game = Number(ended[1]);
    if (!observedGames.get(game).ended) {
      observedGames.get(game).finished = true;
      observedGames.get(game).ended = true;
      emit({ type: 'end', message: line.trim(), observing: true, game });
    }
  } else {
    emit({ type: 'line', message: line });
  }
}

function finishLogin() {
  if (ready) return;
  if (loginMode === 'registered' && !passwordSent) {
    failLogin('FICS did not request a password for that account.');
    return;
  }
  ready = true;
  pendingPassword = '';
  send('set style 12');
  send('set seek 0');
  send('sought');
  emit({ type: 'status', message: guestName ? `Connected as ${guestName}` : 'Connected as FICS guest', connected: true, registered: loginMode === 'registered' });
}

function connect(mode, username = '', password = '') {
  if (socket) socket.close();
  events.length = 0;
  buffer = ''; loginSent = guestConfirmed = passwordSent = ready = false;
  currentGame = null; observedGames.clear(); moveRequests.length = 0; activeMoveRequest = null; guestName = username; loginMode = mode;
  pendingPassword = password; loginError = '';
  const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const connection = new WebSocket(`${scheme}//${location.host}/fics/socket`);
  socket = connection;
  emit({ type: 'status', message: 'Connecting to FICS…', connected: false });
  connection.onmessage = event => {
    if (socket !== connection) return;
    const parsed = parseFicsChunk(buffer, String(event.data));
    buffer = parsed.rest;
    for (const line of parsed.lines) handleLine(line);
    handleLoginPrompt(buffer);
    if (ready && /^\s*fics%\s*$/i.test(buffer)) buffer = '';
    if (!ready && /fics%\s*$/i.test(buffer)) { finishLogin(); buffer = ''; }
  };
  connection.onerror = () => emit({ type: 'line', message: 'FICS connection failed.' });
  connection.onclose = () => {
    if (socket !== connection) return;
    socket = null; ready = false; currentGame = null; observedGames.clear(); moveRequests.length = 0; activeMoveRequest = null; pendingPassword = '';
    emit({ type: 'status', message: loginError || 'Disconnected from FICS', connected: false, registered: false });
  };
}
window.ironwoodFicsConnect = () => connect('guest');
window.ironwoodFicsConnectRegistered = (username, password) => {
  if (!/^[A-Za-z]{3,17}$/.test(username) || !password || /[\r\n]/.test(password)) return;
  connect('registered', username, password);
};
window.ironwoodFicsDisconnect = () => { socket?.close(); socket = null; ready = false; currentGame = null; observedGames.clear(); moveRequests.length = 0; activeMoveRequest = null; pendingPassword = ''; };
window.ironwoodFicsSend = command => send(command);
window.ironwoodFicsSendCommand = command => {
  if (ready && /^[\x20-\x7e]{1,256}$/.test(command)) {
    const observe = command.match(/^observe\s+(.+)$/i);
    if (observe && observedGames.size >= 10 && !observedGames.has(Number(observe[1]))) {
      emit({ type: 'line', message: 'Ironwood can observe at most 10 games.' });
      return;
    }
    const unobserve = command.match(/^unobserve(?:\s+(.*))?$/i);
    if (unobserve) {
      const target = unobserve[1]?.trim();
      if (!target) {
        if (observedGames.size) emit({ type: 'unobserved' });
        observedGames.clear();
        moveRequests.length = 0;
        activeMoveRequest = null;
      } else {
        const game = [...observedGames].find(([id, players]) =>
          String(id) === target || players.white.toLowerCase() === target.toLowerCase() ||
          players.black.toLowerCase() === target.toLowerCase())?.[0];
        if (game !== undefined) {
          observedGames.delete(game);
          const pending = moveRequests.indexOf(game);
          if (pending >= 0) moveRequests.splice(pending, 1);
          emit({ type: 'unobserved', game });
        }
      }
    }
    send(JSON.stringify({ type: 'command', value: command }));
  }
};
window.ironwoodFicsPoll = () => events.shift() || '';
