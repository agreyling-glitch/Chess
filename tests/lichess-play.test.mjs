import test from 'node:test';
import assert from 'node:assert/strict';
import { createClient, readNdjson } from '../web/lichess/client.js';

const tick = () => new Promise(resolve => setImmediate(resolve));
const json = (body, status = 200, headers) => new Response(JSON.stringify(body), { status, headers });
function fixture() {
  const calls = [], events = [], streams = new Map(), replies = new Map(), retries = [];
  let clock = 1000;
  const client = createClient({ emit: event => events.push(structuredClone(event)), now: () => clock,
    delay: (_ms, signal) => new Promise(resolve => { retries.push(resolve); signal.addEventListener('abort', resolve, { once: true }); }),
    fetch: async (url, options) => {
      const path = new URL(url).pathname;
      calls.push({ path, options });
      if (replies.has(path)) return replies.get(path)(options);
      if (path === '/api/account') return json({ id: 'alice', username: 'Alice' });
      if (path === '/api/account/playing') return json({ nowPlaying: [] });
      if (path === '/api/challenge') return json({ in: [], out: [] });
      if (path.startsWith('/game/export/')) return json({ createdAt: 1791475200000 });
      if (path === '/api/stream/event' || path.includes('/stream/') || path === '/api/board/seek') {
        let controller;
        const body = new ReadableStream({ start(c) { controller = c; } });
        options.signal.addEventListener('abort', () => {
          try { controller.error(new DOMException('Aborted', 'AbortError')); } catch {}
        }, { once: true });
        streams.set(path, { send: event => controller.enqueue(new TextEncoder().encode(JSON.stringify(event) + '\n')),
          end: () => controller.close() });
        return new Response(body);
      }
      return json({ ok: true });
    } });
  return { client, calls, events, replies, streams, retries, advance: ms => { clock += ms; } };
}
const full = (moves = '', status = 'started') => ({ type: 'gameFull', id: 'abcd1234', initialFen: 'startpos',
  variant: { key: 'standard' }, speed: 'rapid', white: { id: 'alice', name: 'Alice' },
  black: { id: 'bob', name: 'Bob' }, state: { type: 'gameState', moves, status, wtime: 600000, btime: 600000 } });
async function playing(t, moves = '') {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  f.streams.get('/api/stream/event').send({ type: 'gameStart', game: { id: 'abcd1234' } });
  await tick(); f.streams.get('/api/board/game/stream/abcd1234').send(full(moves)); await tick();
  return f;
}

test('NDJSON handles arbitrary UTF-8 chunks, heartbeats, CRLF, and an unterminated final record', async () => {
  const bytes = new TextEncoder().encode('\n{"text":"♞"}\r\n\n{"last":true}');
  const result = [];
  await readNdjson(new Response(new ReadableStream({ start(c) {
    for (const byte of bytes) c.enqueue(new Uint8Array([byte])); c.close();
  } })), event => result.push(event));
  assert.deepEqual(result, [{ text: '♞' }, { last: true }]);
});

test('account connection opens the event stream and authenticates directly with Lichess', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  assert.equal(f.client.snapshot().connected, true);
  assert.deepEqual(f.calls.map(c => c.path), ['/api/account', '/api/account/playing', '/api/challenge', '/api/stream/event']);
  assert.ok(f.calls.every(c => c.options.headers.Authorization === 'Bearer secret' && c.options.cache === 'no-store'));
  assert.ok(!JSON.stringify(f.events).includes('secret'));
});

test('seeks stay open until canceled, and blitz matchmaking is rejected before a request', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  await assert.rejects(f.client.seek({ time: 3, increment: 0 }), /rapid/);
  const task = f.client.seek({ time: 10, increment: 5, rated: false }); await tick();
  assert.equal(f.client.snapshot().seeking, true);
  const call = f.calls.at(-1);
  assert.equal(call.options.body.get('variant'), 'standard');
  f.client.cancelSeek(); await task.catch(() => {});
  assert.equal(call.options.signal.aborted, true);
  assert.equal(f.client.snapshot().seeking, false);
});

test('authoritative moves, clocks, and completion come from the game stream', async t => {
  const f = await playing(t, 'e2e4 e7e5');
  await f.client.move('g1f3');
  assert.equal(f.calls.at(-1).path, '/api/board/game/abcd1234/move/g1f3');
  await assert.rejects(f.client.move('g1f3'), /Cannot send/);
  const stream = f.streams.get('/api/board/game/stream/abcd1234');
  stream.send({ type: 'gameState', moves: 'e2e4 e7e5', status: 'started', bdraw: true, wtime: 599000, btime: 597000 }); await tick();
  await assert.rejects(f.client.move('g1f3'), /Cannot send/);
  stream.send({ type: 'gameState', moves: 'e2e4 e7e5 g1f3', status: 'resign', winner: 'white', wtime: 598000, btime: 597000 }); await tick();
  const game = f.events.filter(e => e.type === 'game').at(-1);
  assert.equal(game.state.wtime, 598000); assert.equal(game.state.winner, 'white');
  assert.equal(game.pendingMove, false); assert.deepEqual(f.client.snapshot().games, []);
  await assert.rejects(f.client.move('b8c6'), /Cannot send/);
});

test('rejected or uncertain moves trigger a fresh full-game synchronization without retrying the move', async t => {
  const f = await playing(t);
  f.replies.set('/api/board/game/abcd1234/move/e2e4', () => json({ error: 'Not your turn' }, 400));
  await assert.rejects(f.client.move('e2e4'), /Not your turn/); await tick();
  assert.ok(f.events.some(event => event.type === 'moveRejected' && event.id === 'abcd1234'));
  assert.equal(f.calls.filter(c => c.path.endsWith('/move/e2e4')).length, 1);
  assert.equal(f.calls.filter(c => c.path.includes('/game/stream/')).length, 2);
  f.streams.get('/api/board/game/stream/abcd1234').send(full('e2e4 e7e5')); await tick();
  await f.client.move('g1f3');
});

test('rate limits block finite requests for at least a minute', async t => {
  const f = await playing(t);
  f.replies.set('/api/board/game/abcd1234/draw/yes', () => json({}, 429, { 'Retry-After': '90' }));
  await assert.rejects(f.client.action('draw/yes'), /rate limit/);
  const count = f.calls.length;
  await assert.rejects(f.client.action('resign'), /rate limit/); assert.equal(f.calls.length, count);
  f.advance(60000); await assert.rejects(f.client.action('resign'), /rate limit/);
  f.advance(30001); await f.client.action('resign');
});

test('game chat remains separate from spectator chat and encodes text in the POST body', async t => {
  const f = await playing(t);
  const stream = f.streams.get('/api/board/game/stream/abcd1234');
  stream.send({ type: 'chatLine', username: 'Bob', text: '<script>hello</script>', room: 'player' });
  stream.send({ type: 'chatLine', username: 'Spectator', text: 'private', room: 'spectator' }); await tick();
  assert.deepEqual(f.client.snapshot().chat, [{ user: 'Bob', text: '<script>hello</script>' }]);
  await f.client.action('chat', 'hello & goodbye');
  assert.equal(f.calls.at(-1).options.body.get('text'), 'hello & goodbye');
  assert.equal(f.calls.at(-1).options.body.get('room'), 'player');
});

test('incoming challenges can be accepted or declined; outgoing challenges can be canceled', async t => {
  const f = fixture(); t.after(() => f.client.disconnect()); await f.client.connect('secret'); await tick();
  f.streams.get('/api/stream/event').send({ type: 'challenge', challenge: { id: 'chal1234', challenger: { id: 'bob' } } }); await tick();
  assert.equal(f.client.snapshot().challenges.length, 1);
  await f.client.challengeAction('chal1234', 'decline'); assert.equal(f.client.snapshot().challenges.length, 0);
  f.replies.set('/api/challenge/Bob', () => json({ challenge: { id: 'outg1234' } }));
  await f.client.challenge('Bob', { 'clock.limit': 300, 'clock.increment': 5 });
  await f.client.challengeAction('outg1234', 'cancel');
  assert.equal(f.calls.at(-1).path, '/api/challenge/outg1234/cancel');
});

test('resume loads an existing game; disconnect aborts streams and ignores stale session events', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  f.replies.set('/api/account/playing', () => json({ nowPlaying: [{ gameId: 'abcd1234', variant: { key: 'standard' } }] }));
  await f.client.connect('secret'); await tick();
  assert.equal(f.client.snapshot().game, null);
  assert.equal(f.streams.has('/api/board/game/stream/abcd1234'), false);
  f.streams.get('/api/stream/event').send({ type: 'gameStart', game: { id: 'abcd1234' } });
  await tick();
  assert.equal(f.streams.has('/api/board/game/stream/abcd1234'), false);
  f.client.openGame('abcd1234'); await tick();
  f.streams.get('/api/board/game/stream/abcd1234').send(full('e2e4')); await tick();
  assert.equal(f.client.snapshot().game.state.moves, 'e2e4');
  f.client.disconnect(); await tick();
  assert.equal(f.client.snapshot().connected, false); assert.equal(f.client.snapshot().game, null);
  assert.ok(f.calls.filter(c => c.path.includes('/stream/')).every(c => c.options.signal.aborted));
});

test('unsupported variants are surfaced without allowing moves', async t => {
  const f = await playing(t);
  const event = full(); event.variant.key = 'atomic';
  f.streams.get('/api/board/game/stream/abcd1234').send(event); await tick();
  assert.match(f.client.snapshot().status, /not supported/);
  await assert.rejects(f.client.move('e2e4'), /Cannot send/);
});

test('event and game streams reconnect and restore the full game after a dropped connection', async t => {
  const f = await playing(t, 'e2e4');
  f.streams.get('/api/board/game/stream/abcd1234').end(); await tick();
  f.retries.shift()(); await tick();
  f.streams.get('/api/board/game/stream/abcd1234').send(full('e2e4 e7e5 g1f3')); await tick();
  assert.equal(f.client.snapshot().game.state.moves, 'e2e4 e7e5 g1f3');
  f.streams.get('/api/stream/event').end(); await tick();
  assert.equal(f.client.snapshot().connected, false);
  f.retries.shift()(); await tick(); assert.equal(f.client.snapshot().connected, true);
});

test('expired authorization disconnects all streams rather than repeatedly retrying', async t => {
  const f = await playing(t);
  f.replies.set('/api/board/game/abcd1234/draw/yes', () => json({}, 401));
  await assert.rejects(f.client.action('draw/yes'), /sign-in expired/); await tick();
  assert.equal(f.client.snapshot().connected, false);
  assert.equal(f.client.snapshot().game, null);
  assert.equal(f.retries.length, 0);
});

test('unsupported ongoing games stay in the lobby without opening a Board API stream', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  f.replies.set('/api/account/playing', () => json({ nowPlaying: [{ gameId: 'bullet12', variant: { key: 'standard' }, speed: 'bullet', compat: { board: false } }] }));
  await f.client.connect('secret'); await tick();
  assert.equal(f.client.snapshot().games.length, 1);
  assert.equal(f.calls.some(c => c.path.includes('/game/stream/')), false);
  assert.throws(() => f.client.openGame('bullet12'), /must be played on Lichess/);
});

test('Lichess AI starts a casual game with the selected level and time control', async t => {
  const f = fixture(); t.after(() => f.client.disconnect()); await f.client.connect('secret'); await tick();
  f.replies.set('/api/challenge/ai', () => json({ id: 'abcd1234' }, 201));
  await f.client.challengeAi({ level: 3, 'clock.limit': 600, 'clock.increment': 5, color: 'random' }); await tick();
  const request = f.calls.find(c => c.path === '/api/challenge/ai');
  assert.equal(request.options.body.get('level'), '3'); assert.equal(request.options.body.get('clock.limit'), '600');
  assert.equal(request.options.body.has('rated'), false);
  assert.ok(f.streams.has('/api/board/game/stream/abcd1234'));
});

test('finite game actions are serialized while the required event and game streams remain open', async t => {
  const f = await playing(t);
  let resolveFirst;
  f.replies.set('/api/board/game/abcd1234/draw/yes', () => new Promise(resolve => { resolveFirst = resolve; }));
  const draw = f.client.action('draw/yes'), takeback = f.client.action('takeback/yes'); await tick();
  assert.equal(f.calls.some(c => c.path.endsWith('/takeback/yes')), false);
  resolveFirst(json({ ok: true })); await Promise.all([draw, takeback]);
  assert.equal(f.calls.at(-1).path, '/api/board/game/abcd1234/takeback/yes');
});

test('correspondence can be left ongoing while another game opens', async t => {
  const f = await playing(t);
  assert.throws(() => f.client.leaveCorrespondence(), /correspondence/);
  f.streams.get('/api/board/game/stream/abcd1234').send({ ...full(''), speed: 'correspondence' });
  await tick();
  const oldStream = f.calls.find(c => c.path === '/api/board/game/stream/abcd1234');
  f.client.leaveCorrespondence();
  assert.equal(oldStream.options.signal.aborted, true);
  assert.equal(f.client.snapshot().game, null);
  assert.ok(f.client.snapshot().games.some(g => g.gameId === 'abcd1234'));
  assert.ok(f.events.some(e => e.type === 'parked'));
  assert.ok(!f.calls.some(c => /resign|abort$/.test(c.path)));
  f.streams.get('/api/stream/event').send({ type: 'gameStart', game: { id: 'efgh5678' } });
  await tick();
  assert.ok(f.streams.has('/api/board/game/stream/efgh5678'));
  f.client.openGame('abcd1234');
  await tick();
  assert.equal(f.calls.filter(c => c.path === '/api/board/game/stream/abcd1234').length, 2);
});

test('ongoing games gain start time and correspondence metadata without opening the board', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  f.replies.set('/api/account/playing', () => json({ nowPlaying: [{ gameId: 'abcd1234', speed: 'correspondence', isMyTurn: true }] }));
  f.replies.set('/game/export/abcd1234', () => json({ createdAt: 1791475200000, daysPerTurn: 2 }));
  await f.client.connect('secret'); await tick();
  const item = f.client.snapshot().games[0];
  assert.ok(item.startedLabel);
  assert.equal(item.daysPerTurn, 2);
  assert.equal(item.isMyTurn, true);
  assert.equal(f.client.snapshot().game, null);
});

test('missing export timestamp falls back to a short metadata stream without engaging the game', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  f.replies.set('/api/account/playing', () => json({ nowPlaying: [{ gameId: 'abcd1234', speed: 'correspondence' }] }));
  f.replies.set('/game/export/abcd1234', () => json({}));
  await f.client.connect('secret'); await tick();
  f.streams.get('/api/board/game/stream/abcd1234').send({ ...full(), createdAt: 1791475200000 });
  await tick();
  assert.ok(f.client.snapshot().games[0].startedLabel);
  assert.equal(f.client.snapshot().game, null);
  assert.ok(!f.events.some(e => e.type === 'loading' || e.type === 'game'));
  assert.ok(f.calls.find(c => c.path === '/api/board/game/stream/abcd1234').options.signal.aborted);
});

test('computer variants and custom FEN are sent without forcing standard or a clock', async t => {
  const f = fixture(); t.after(() => f.client.disconnect()); await f.client.connect('token');
  f.replies.set('/api/challenge/ai', () => json({ id: 'abcd1234' }));
  await f.client.challengeAi({ level: 4, variant: 'chess960', color: 'black' });
  let request = f.calls.filter(c => c.path === '/api/challenge/ai').at(-1);
  assert.equal(request.options.body.get('variant'), 'chess960');
  assert.equal(request.options.body.has('clock.limit'), false);
  const fen = '4k3/8/8/8/8/8/8/4K3 w - - 0 1';
  await f.client.challengeAi({ level: 2, variant: 'fromPosition', fen, days: 2 });
  request = f.calls.filter(c => c.path === '/api/challenge/ai').at(-1);
  assert.equal(request.options.body.get('fen'), fen);
  assert.equal(request.options.body.get('variant'), 'fromPosition');
  await assert.rejects(f.client.challengeAi({ level: 2, variant: 'fromPosition' }), /FEN/);
});

