import test from 'node:test';
import assert from 'node:assert/strict';

test('guest login produces an authoritative board event for the Ironwood board', async () => {
  let connection;
  class FakeWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.sent = []; connection = this; }
    send(command) { this.sent.push(command); }
    close() { this.readyState = 3; }
  }
  globalThis.window = {};
  globalThis.location = { protocol: 'http:', host: '127.0.0.1:8788' };
  globalThis.WebSocket = FakeWebSocket;
  await import('../web/fics/bridge.js');
  window.ironwoodFicsConnect();
  window.ironwoodFicsSendCommand('who');
  assert.deepEqual(connection.sent, []);
  connection.onmessage({ data: 'login: ' });
  assert.deepEqual(connection.sent, ['guest']);
  connection.onmessage({ data: '\nPress return to enter as GuestABCD\n' });
  assert.equal(connection.sent.at(-1), '');
  connection.onmessage({ data: 'fics% ' });
  assert.deepEqual(connection.sent.slice(-3), ['set style 12', 'set seek 0', 'sought']);
  window.ironwoodFicsSendCommand('who');
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'who' });
  window.ironwoodFicsSendCommand('who\nquit');
  assert.equal(connection.sent.length, 6);
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- -------- -------- PPPPPPPP RNBQKBNR W -1 1 1 1 1 0 12 GuestABCD Opponent 1 300 0 39 39 300 300 1 none (0:00) none 0\n' });
  const events = [];
  for (let json; (json = window.ironwoodFicsPoll());) events.push(JSON.parse(json));
  const board = events.find(event => event.type === 'board');
  assert.equal(board?.fen, 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
  assert.equal(board?.side, 'white');
  assert.equal(board?.game, 12);
  assert.equal(board?.last_san, 'none');
  connection.onmessage({ data: '{Game 12 (GuestABCD vs. Opponent) GuestABCD checkmated} 0-1\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), {
    type: 'end', message: '{Game 12 (GuestABCD vs. Opponent) GuestABCD checkmated} 0-1',
  });
  connection.onmessage({ data: '    /\\___\n  /  \\  \n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: '    /\\___' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: '  /  \\  ' });
  connection.onmessage({ data: 'fics% \x07\n' });
  assert.equal(window.ironwoodFicsPoll(), '');
});

test('registered login sends the password only at the password prompt', async () => {
  let connection;
  class FakeWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.sent = []; connection = this; }
    send(command) { this.sent.push(command); }
    close() { this.readyState = 3; this.onclose?.(); }
  }
  globalThis.window = {};
  globalThis.location = { protocol: 'https:', host: 'ironwoodchess.com' };
  globalThis.WebSocket = FakeWebSocket;
  await import('../web/fics/bridge.js?registered-login');
  window.ironwoodFicsConnectRegistered('Alice', 'unique-password');
  connection.onmessage({ data: 'login: ' });
  assert.deepEqual(connection.sent, ['Alice']);
  connection.onmessage({ data: 'Password: ' });
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'password', value: 'unique-password' });
  connection.onmessage({ data: 'fics% ' });
  assert.deepEqual(connection.sent.slice(-4,-1), ['set style 12', 'set seek 0', 'sought']);
  assert.deepEqual(JSON.parse(connection.sent.at(-1)),{type:'command',value:'finger Alice /bls r'});
  connection.onmessage({data:'Statistics for Alice On for: 10\nBlitz 1910 75.3 28 10 2 40 1936 (01-Jun-95)\nStandard 1838 92.3 13 6 1 20 1838 (03-Jun-95)\nLightning ---- 350.0 0 0 0 0\nfics% '});
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- -------- -------- PPPPPPPP RNBQKBNR W -1 1 1 1 1 0 12 Alice Opponent 1 300 0 39 39 300 300 1 none (0:00) none 0\n' });
  const events = [];
  for (let json; (json = window.ironwoodFicsPoll());) events.push(JSON.parse(json));
  assert.ok(events.some(event => event.type === 'status' && event.connected && event.registered));
  const profile = events.filter(event=>event.type === 'profile').at(-1);
  assert.match(profile.summary,/Alice.*Blitz 1910.*Standard 1838.*Lightning —/);
  assert.match(profile.details,/28 wins \/ 2 draws \/ 10 losses/);
  assert.match(profile.details,/best 1936/);
  connection.onmessage({data:'Statistics for Bob On for: 10\nBlitz 2200 50.0 90 10 0 100 2300\nfics% '});
  const later = [];
  for (let json; (json = window.ironwoodFicsPoll());) later.push(JSON.parse(json));
  assert.ok(!later.some(event=>event.type === 'profile'));
  assert.equal(events.find(event => event.type === 'board')?.side, 'white');
  assert.ok(!events.some(event => JSON.stringify(event).includes('unique-password')));
});

test('unknown registered handle returns a sign-in error instead of waiting for a password', async () => {
  let connection;
  class FakeWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.sent = []; connection = this; }
    send(command) { this.sent.push(command); }
    close() { this.readyState = 3; this.onclose?.(); }
  }
  globalThis.window = {};
  globalThis.location = { protocol: 'http:', host: '127.0.0.1:8788' };
  globalThis.WebSocket = FakeWebSocket;
  await import('../web/fics/bridge.js?unknown-registered-handle');
  window.ironwoodFicsConnectRegistered('Zyqnotreal', 'example-password');
  connection.onopen();
  connection.onmessage({ data: 'Welcome to FICS\n' });
  connection.onmessage({ data: 'login: ' });
  connection.onmessage({ data: '\n"Zyqnotreal" is not a registered name. You may use this name to play unrated games.\n' });
  assert.deepEqual(connection.sent, ['Zyqnotreal']);
  assert.equal(connection.readyState, 3);
  const events = [];
  for (let json; (json = window.ironwoodFicsPoll());) events.push(JSON.parse(json));
  assert.ok(events.some(event => event.type === 'status' && event.transport_open && !event.connected));
  assert.ok(events.some(event => event.type === 'line' && event.message === 'Welcome to FICS'));
  assert.ok(events.some(event => event.type === 'line' && event.message === 'login: '));
  assert.ok(events.some(event => event.type === 'line' && event.message.includes('not a registered name')));
  assert.deepEqual(events.at(-1), {
    type: 'status',
    message: 'FICS rejected the handle: it is not registered.',
    connected: false,
    registered: false,
  });
  assert.ok(!events.some(event => JSON.stringify(event).includes('example-password')));
});

test('observed games update the board without becoming a played game', async () => {
  let connection;
  class FakeWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.sent = []; connection = this; }
    send(command) { this.sent.push(command); }
    close() { this.readyState = 3; }
  }
  globalThis.window = {};
  globalThis.location = { protocol: 'http:', host: '127.0.0.1:8788' };
  globalThis.WebSocket = FakeWebSocket;
  await import('../web/fics/bridge.js?observation');
  window.ironwoodFicsConnect();
  connection.onmessage({ data: 'login: ' });
  connection.onmessage({ data: '\nPress return to enter as GuestABCD\n' });
  connection.onmessage({ data: 'fics% ' });
  while (window.ironwoodFicsPoll()) {}
  window.ironwoodFicsSendCommand('observe 42');
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'observe 42' });
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- -------- -------- PPPPPPPP RNBQKBNR W -1 1 1 1 1 0 42 Alice Bob 0 300 0 39 39 280 270 1 none (0:00) none 0\n' });
  const board = JSON.parse(window.ironwoodFicsPoll());
  assert.equal(board.type, 'board');
  assert.equal(board.game, 42);
  assert.equal(board.observing, true);
  assert.equal(board.move_number, 1);
  assert.equal(board.white_time, 280);
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'moves 42' });
  connection.onmessage({ data: 'fics% ' });
  assert.equal(window.ironwoodFicsPoll(), '');
  connection.onmessage({ data: 'Movelist for game 42:\nAlice (1500) vs. Bob (1600)\nMove  Alice  Bob\n----  -----  ---\n  1.  e4      (0:04)     e5      (0:12)\n     {Still in progress} *\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Movelist for game 42:' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Alice (1500) vs. Bob (1600)' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Move  Alice  Bob' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: '----  -----  ---' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: '  1.  e4      (0:04)     e5      (0:12)' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'history', game: 42, moves: ['e4', 'e5'] });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: '     {Still in progress} *' });
  window.ironwoodFicsSendCommand('limits');
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'limits' });
  connection.onmessage({ data: 'Current hardcoded limits\nObserved games: 30\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Current hardcoded limits' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Observed games: 30' });
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- ----P--- -------- PPPP-PPP RNBQKBNR B 4 1 1 1 1 0 42 Alice Bob 0 300 0 39 39 280 270 1 P/e2-e4 (0:01) e4 0\n' });
  const nextBoard = JSON.parse(window.ironwoodFicsPoll());
  assert.equal(nextBoard.type, 'board');
  assert.equal(nextBoard.game, 42);
  assert.equal(nextBoard.observing, true);
  assert.equal(nextBoard.last_san, 'e4');
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- -------- -------- PPPPPPPP RNBQKBNR W -1 1 1 1 1 0 43 Carol Dan 0 300 0 39 39 280 270 1 none (0:00) none 0\n' });
  assert.equal(JSON.parse(window.ironwoodFicsPoll()).game, 43);
  connection.onmessage({ data: 'fics% Removing game 42 from observation list.\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'observationstopped', game: 42 });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Removing game 42 from observation list.' });
  connection.onmessage({ data: '{Game 42 (Alice vs. Bob) Bob checkmated} 1-0\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), {
    type: 'end', message: '{Game 42 (Alice vs. Bob) Bob checkmated} 1-0', observing: true, game: 42,
  });
  window.ironwoodFicsSendCommand('unobserve 42');
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'unobserved', game: 42 });
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'unobserve 42' });
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- -------- -------- PPPPPPPP RNBQKBNR W -1 1 1 1 1 0 43 Carol Dan 0 300 0 39 39 275 270 1 none (0:00) none 0\n' });
  assert.equal(JSON.parse(window.ironwoodFicsPoll()).game, 43);
  connection.onmessage({ data: '{Game 43 (Carol vs. Dan) Carol resigns} 0-1\nRemoving game 43 from observation list.\n' });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), {
    type: 'end', message: '{Game 43 (Carol vs. Dan) Carol resigns} 0-1', observing: true, game: 43,
  });
  assert.deepEqual(JSON.parse(window.ironwoodFicsPoll()), { type: 'line', message: 'Removing game 43 from observation list.' });
});

test('move backfill starts when the observe prompt arrives before the board', async () => {
  let connection;
  class FakeWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.sent = []; connection = this; }
    send(command) { this.sent.push(command); }
    close() { this.readyState = 3; }
  }
  globalThis.window = {};
  globalThis.location = { protocol: 'http:', host: '127.0.0.1:8788' };
  globalThis.WebSocket = FakeWebSocket;
  await import('../web/fics/bridge.js?observation-prompt-first');
  window.ironwoodFicsConnect();
  connection.onmessage({ data: 'login: ' });
  connection.onmessage({ data: '\nPress return to enter as GuestABCD\n' });
  connection.onmessage({ data: 'fics% ' });
  window.ironwoodFicsSendCommand('observe 42');
  connection.onmessage({ data: 'fics% You are now observing game 42.\n' });
  connection.onmessage({ data: '<12> rnbqkbnr pppppppp -------- -------- ----P--- -------- PPPP-PPP RNBQKBNR B 4 1 1 1 1 0 42 Alice Bob 0 300 0 39 39 280 270 1 P/e2-e4 (0:01) e4 0\n' });
  assert.deepEqual(JSON.parse(connection.sent.at(-1)), { type: 'command', value: 'moves 42' });
});
