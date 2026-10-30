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
      calls.push({ path, url, options });
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



test('profile statistics share the account request and are cleared on disconnect',async t=>{
  const f = fixture(); t.after(()=>f.client.disconnect());
  const profile = {count:{all:13,win:3,draw:0,loss:10,rated:0},perfs:{rapid:{games:0,rating:1500,prov:true}},playTime:{total:5640}};
  f.replies.set('/api/account',()=>json({id:'alice',username:'Alice',...profile,profile:{bio:'not needed'}}));
  await f.client.connect('secret'); await tick();
  assert.deepEqual(f.client.snapshot().profile,profile);
  assert.equal(f.calls.filter(c=>c.path === '/api/account').length,1);
  f.client.disconnect();
  assert.equal(f.client.snapshot().profile,null);
});


test('rating history is authenticated, lazy, cached, and refreshable', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  let requests = 0;
  const history = [{ name: 'rapid', points: [[2026, 0, 1, 1500], [2026, 1, 1, 1550]] }];
  f.replies.set('/api/user/Alice/rating-history', () => { requests++; return json(history); });
  await f.client.connect('secret'); await tick();
  assert.equal(requests, 0);
  await f.client.ratingHistory(); await f.client.ratingHistory();
  assert.equal(requests, 1);
  assert.deepEqual(f.client.snapshot().ratingHistory, history);
  await f.client.ratingHistory(true); assert.equal(requests, 2);
  assert.equal(f.calls.at(-1).options.headers.Authorization, 'Bearer secret');
});

test('rating history errors stay local and stale replies cannot cross sessions', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  f.replies.set('/api/user/Alice/rating-history', () => json({ error: 'Unavailable' }, 503));
  await f.client.ratingHistory();
  assert.equal(f.client.snapshot().connected, true);
  assert.equal(f.client.snapshot().historyError, 'Unavailable');
  let resolve;
  f.replies.set('/api/user/Alice/rating-history', () => new Promise(r => { resolve = r; }));
  const pending = f.client.ratingHistory(); await tick();
  f.client.disconnect(); resolve(json([{ name: 'rapid', points: [[2026,0,1,1500]] }]));
  await pending;
  assert.equal(f.client.snapshot().ratingHistory, null);
  assert.equal(f.client.snapshot().historyLoading, false);
});


test('another player history replaces the cache without changing signed-in account', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  f.replies.set('/api/user/Alice/rating-history', () => json([{name:'rapid',points:[[2026,0,1,1500]]}]));
  f.replies.set('/api/user/DrNykterstein/rating-history', () => json([{name:'blitz',points:[[2026,0,1,3000]]}]));
  await f.client.ratingHistory();
  await f.client.ratingHistory(false, 'DrNykterstein');
  assert.equal(f.client.snapshot().account, 'Alice');
  assert.equal(f.client.snapshot().historyUsername, 'DrNykterstein');
  assert.equal(f.client.snapshot().ratingHistory[0].name, 'blitz');
  await f.client.ratingHistory();
  assert.equal(f.client.snapshot().historyUsername, 'Alice');
  await f.client.ratingHistory(false, '../bad');
  assert.match(f.client.snapshot().historyError, /valid Lichess username/);
});


test('player profile loads public data, lowercase history and bounded NDJSON games', async t => {
  const f = fixture(); t.after(() => f.client.disconnect());
  await f.client.connect('secret'); await tick();
  f.replies.set('/api/user/cutemouse83', () => json({username:'cutemouse83',title:'GM',profile:{bio:'Biography'},count:{all:19642}}));
  f.replies.set('/api/user/cutemouse83/rating-history', () => json([{name:'blitz',points:[[2026,9,14,2997]]}]));
  f.replies.set('/api/games/user/cutemouse83', () => new Response(Array.from({length:15},(_,i)=>JSON.stringify({id:`game${i}`,speed:'blitz'})).join('\n')));
  await f.client.playerProfile(false, 'cutemouse83');
  const state=f.client.snapshot();
  assert.equal(state.playerProfile.profile.bio,'Biography');
  assert.equal(state.profileUsername,'cutemouse83'); assert.equal(state.account,'Alice');
  assert.equal(state.recentGames.length,12); assert.equal(state.profileLoading,false);
  const call=f.calls.at(-1);
  assert.equal(call.options.headers.Accept,'application/x-ndjson');
  assert.equal(new URL(call.url).searchParams.get('clocks'),'true');
  assert.equal(new URL(call.url).searchParams.get('pgnInJson'),'true');
  const count=f.calls.length; await f.client.playerProfile(false,'cutemouse83'); assert.equal(f.calls.length,count);
});

test('profile retains successful sections on recent-game errors and clears data on user change', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  f.replies.set('/api/user/Alice',()=>json({username:'Alice'}));
  f.replies.set('/api/user/Alice/rating-history',()=>json([]));
  f.replies.set('/api/games/user/Alice',()=>json({error:'Unavailable'},503));
  await f.client.playerProfile();
  assert.equal(f.client.snapshot().playerProfile.username,'Alice');
  assert.match(f.client.snapshot().profileError,/Recent games/); assert.equal(f.client.snapshot().connected,true);
  f.replies.set('/api/user/Missing',()=>json({error:'No such player'},404));
  await f.client.playerProfile(false,'Missing');
  assert.equal(f.client.snapshot().playerProfile,null); assert.equal(f.client.snapshot().recentGames,null);
  assert.equal(f.client.snapshot().ratingHistory,null); assert.match(f.client.snapshot().profileError,/No such player/);
});

test('disconnect prevents a delayed profile response from restoring another session', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  let resolve; f.replies.set('/api/user/Alice',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.playerProfile(); await tick(); f.client.disconnect(); resolve(json({username:'Alice'}));
  await pending; assert.equal(f.client.snapshot().playerProfile,null); assert.equal(f.client.snapshot().profileLoading,false);
});


test('following list loads all players beyond recent-game limit and is cached', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  const users=Array.from({length:25},(_,i)=>({id:`player${i}`,username:`Player${i}`,title:'GM'}));
  f.replies.set('/api/rel/following',()=>new Response(users.map(u=>JSON.stringify(u)).join('\n')));
  await f.client.loadFollowing(); assert.equal(f.client.snapshot().following.length,25);
  assert.equal(f.calls.at(-1).options.headers.Accept,'application/x-ndjson');
  const count=f.calls.length; await f.client.loadFollowing(); assert.equal(f.calls.length,count);
  f.client.disconnect(); assert.equal(f.client.snapshot().following,null);
});

test('follow and unfollow update server relation and cached list only after success', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  f.replies.set('/api/rel/following',()=>new Response(''));
  await f.client.loadFollowing();
  f.replies.set('/api/user/Bob',()=>json({id:'bob',username:'Bob',following:false,followable:true}));
  f.replies.set('/api/user/Bob/rating-history',()=>json([]));
  f.replies.set('/api/games/user/Bob',()=>new Response(''));
  await f.client.playerProfile(false,'Bob');
  await f.client.setFollowing('Bob',true);
  assert.equal(f.calls.at(-1).path,'/api/rel/follow/Bob'); assert.equal(f.calls.at(-1).options.method,'POST');
  assert.equal(f.client.snapshot().playerProfile.following,true); assert.equal(f.client.snapshot().following[0].username,'Bob');
  f.replies.set('/api/rel/unfollow/Bob',()=>json({error:'Unavailable'},503));
  await f.client.setFollowing('Bob',false);
  assert.equal(f.client.snapshot().playerProfile.following,true); assert.equal(f.client.snapshot().following.length,1);
  f.replies.set('/api/rel/unfollow/Bob',()=>json({ok:true}));
  await f.client.setFollowing('Bob',false);
  assert.equal(f.client.snapshot().playerProfile.following,false); assert.equal(f.client.snapshot().following.length,0);
  const count=f.calls.length; await f.client.setFollowing('Alice',true); await f.client.setFollowing('../bad',true);
  assert.equal(f.calls.length,count);
});

test('missing follow permission gives reauthorization guidance without disconnecting play', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('old-token'); await tick();
  f.replies.set('/api/rel/following',()=>json({error:'Missing scope'},403));
  await f.client.loadFollowing();
  assert.equal(f.client.snapshot().followPermissionNeeded,true); assert.match(f.client.snapshot().followError,/Sign in again/);
  assert.equal(f.client.snapshot().connected,true); assert.equal(f.client.snapshot().followingLoading,false);
  f.replies.set('/api/rel/follow/Bob',()=>json({error:'Missing scope'},403));
  await f.client.setFollowing('Bob',true);
  assert.equal(f.client.snapshot().followPermissionNeeded,true); assert.equal(f.client.snapshot().followBusy,false);
});

test('in-flight follow requests cannot duplicate or leak into another session', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  let resolve; f.replies.set('/api/rel/follow/Bob',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.setFollowing('Bob',true); await tick();
  const count=f.calls.length; await f.client.setFollowing('Bob',true); assert.equal(f.calls.length,count);
  f.client.disconnect(); resolve(json({ok:true})); await pending;
  assert.equal(f.client.snapshot().following,null); assert.equal(f.client.snapshot().followBusy,false);
});


test('comparison windows load concurrently without replacing each other and share follow updates', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  for (const [username,rating] of [['Bob',2000],['Carol',2500]]) {
    f.replies.set(`/api/user/${username}`,()=>json({id:username.toLowerCase(),username,following:false}));
    f.replies.set(`/api/user/${username}/rating-history`,()=>json([{name:'blitz',points:[[2026,9,14,rating]]}]));
    f.replies.set(`/api/games/user/${username}`,()=>new Response(JSON.stringify({id:username+'game'})));
  }
  await Promise.all([f.client.playerProfile(false,'Bob','one'),f.client.playerProfile(false,'Carol','two')]);
  let windows=f.client.snapshot().profileWindows;
  assert.equal(windows.one.playerProfile.username,'Bob'); assert.equal(windows.two.playerProfile.username,'Carol');
  assert.equal(windows.one.ratingHistory[0].points[0][3],2000); assert.equal(windows.two.ratingHistory[0].points[0][3],2500);
  assert.equal(windows.one.recentGames[0].id,'Bobgame'); assert.equal(windows.two.recentGames[0].id,'Carolgame');
  await f.client.playerProfile(false,'Bob','three');
  await f.client.setFollowing('Bob',true);
  windows=f.client.snapshot().profileWindows;
  assert.equal(windows.one.playerProfile.following,true); assert.equal(windows.three.playerProfile.following,true);
  assert.equal(windows.two.playerProfile.following,false);
  f.client.closeProfileWindow('one'); assert.equal(f.client.snapshot().profileWindows.one,undefined);
  assert.equal(f.client.snapshot().profileWindows.two.playerProfile.username,'Carol');
});

test('closing a loading comparison window ignores its late reply', async t => {
  const f=fixture(); t.after(()=>f.client.disconnect()); await f.client.connect('secret'); await tick();
  let resolve; f.replies.set('/api/user/Bob',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.playerProfile(false,'Bob','closed'); await tick();
  f.client.closeProfileWindow('closed'); resolve(json({id:'bob',username:'Bob'})); await pending;
  assert.equal(f.client.snapshot().profileWindows.closed,undefined);
  assert.ok(!f.calls.some(call=>call.path==='/api/user/Bob/rating-history'));
});

async function performanceProfile(f, username = 'Bob', windowId = 'perf') {
  f.replies.set(`/api/user/${username}`, () => json({id:username.toLowerCase(),username}));
  f.replies.set(`/api/user/${username}/rating-history`, () => json([]));
  f.replies.set(`/api/games/user/${username}`, () => new Response(''));
  await f.client.playerProfile(false, username, windowId);
}
const perfStats = () => ({perf:{glicko:{rating:2000}},rank:10,percentile:99,stat:{
  highest:{int:2200,at:'2026-10-01T00:00:00Z'},count:{all:50,win:30,loss:15,draw:5},
  bestWins:{results:[{gameId:'best1234',opId:{name:'Carol'},opRating:2500}]},
  worstLosses:{results:[{gameId:'loss1234',opId:{name:'Dave'},opRating:1500},{gameId:'best1234'}]}
}});
test('performance batch-exports notable games once without authorization and caches by time control', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  f.replies.set('/api/user/Bob/perf/blitz',()=>json(perfStats()));
  f.replies.set('/api/user/Bob/perf/rapid',()=>json({...perfStats(),rank:20}));
  f.replies.set('/api/games/export/_ids',options=>{
    assert.equal(options.method,'POST');assert.equal(options.body,'best1234,loss1234');
    assert.equal(options.headers.Accept,'application/x-ndjson');assert.ok(!('Authorization' in options.headers));
    return new Response(['loss1234','best1234'].map(id=>JSON.stringify({id,status:'mate',pgn:'1. e4 e5 2. Nf3 *'})).join('\n'));
  });
  await f.client.performance('Bob','blitz','perf');
  let entry=f.client.snapshot().profileWindows.perf.performance.blitz;
  assert.equal(entry.loading,false);assert.equal(entry.error,'');assert.equal(entry.data.rank,10);
  assert.equal(Object.keys(entry.games).length,2);assert.match(entry.games.best1234.pgn,/Nf3/);
  assert.equal(f.calls.filter(c=>c.path==='/api/games/export/_ids').length,1);
  assert.equal(new URL(f.calls.find(c=>c.path==='/api/games/export/_ids').url).searchParams.get('clocks'),'true');
  assert.ok(!f.calls.some(c=>c.path.startsWith('/game/export/')));
  const count=f.calls.length;await f.client.performance('Bob','blitz','perf');assert.equal(f.calls.length,count);
  await f.client.performance('Bob','rapid','perf');
  assert.equal(f.client.snapshot().profileWindows.perf.performance.blitz.data.rank,10);
  assert.equal(f.client.snapshot().profileWindows.perf.performance.rapid.data.rank,20);
  await f.client.performance('Bob','blitz','perf',true);assert.equal(f.calls.filter(c=>c.path==='/api/user/Bob/perf/blitz').length,2);
});
test('late performance responses cannot replace another player or reopen a closed window', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  let resolve;f.replies.set('/api/user/Bob/perf/blitz',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.performance('Bob','blitz','perf');await tick();const change=performanceProfile(f,'Carol');await tick();
  resolve(json(perfStats()));await Promise.all([pending,change]);
  assert.equal(f.client.snapshot().profileWindows.perf.profileUsername,'Carol');
  assert.deepEqual(f.client.snapshot().profileWindows.perf.performance,{});
  assert.ok(!f.calls.some(c=>c.path==='/api/games/export/_ids'));
  f.replies.set('/api/user/Carol/perf/blitz',()=>json(perfStats()));
  f.replies.set('/api/games/export/_ids',()=>new Promise(r=>{resolve=r;}));
  const closed=f.client.performance('Carol','blitz','perf');await tick();f.client.closeProfileWindow('perf');
  resolve(new Response(JSON.stringify({id:'best1234',status:'mate',pgn:'1. e4 *'})));await closed;
  assert.equal(f.client.snapshot().profileWindows.perf,undefined);
});
test('missing batch games retain statistics and successful previews; failed batches are retryable', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  f.replies.set('/api/user/Bob/perf/blitz',()=>json(perfStats()));
  f.replies.set('/api/games/export/_ids',()=>new Response(JSON.stringify({id:'best1234',status:'mate',pgn:'1. e4 *'})));
  await f.client.performance('Bob','blitz','perf');let entry=f.client.snapshot().profileWindows.perf.performance.blitz;
  assert.equal(entry.data.rank,10);assert.ok(entry.games.best1234);assert.match(entry.gameErrors.loss1234,/unavailable/);
  f.replies.set('/api/games/export/_ids',()=>{throw new TypeError('Failed to fetch');});
  await f.client.performance('Bob','blitz','perf',true);entry=f.client.snapshot().profileWindows.perf.performance.blitz;
  assert.equal(entry.loading,false);assert.equal(entry.data.rank,10);assert.match(entry.error,/Refresh/);
  const count=f.calls.filter(c=>c.path==='/api/games/export/_ids').length;
  await f.client.performance('Bob','blitz','perf');assert.equal(f.calls.filter(c=>c.path==='/api/games/export/_ids').length,count+1);
});
test('empty performance skips exports; rate limits and disconnects preserve session isolation', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  f.replies.set('/api/user/Bob/perf/blitz',()=>json({perf:{},stat:{bestWins:{results:[]},worstLosses:{results:[]}}}));
  await f.client.performance('Bob','blitz','perf');assert.ok(!f.calls.some(c=>c.path==='/api/games/export/_ids'));
  const count=f.calls.length;await f.client.performance('Bob','puzzle','perf');assert.equal(f.calls.length,count);
  f.replies.set('/api/user/Bob/perf/rapid',()=>json(perfStats()));
  f.replies.set('/api/games/export/_ids',()=>json({},429));await f.client.performance('Bob','rapid','perf');
  assert.match(f.client.snapshot().profileWindows.perf.performance.rapid.error,/rate limit/);
  assert.equal(f.client.snapshot().profileWindows.perf.performance.rapid.loading,false);
  f.advance(60001);let resolve;f.replies.set('/api/games/export/_ids',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.performance('Bob','rapid','perf',true);await tick();f.client.disconnect();resolve(new Response(''));await pending;
  assert.deepEqual(f.client.snapshot().profileWindows,{});
});

test('activity is lazy, sorted newest first, cached per profile window and refreshable', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  assert.ok(!f.calls.some(c=>c.path.endsWith('/activity')));
  const days=[{interval:{start:100},puzzles:{score:{win:2,loss:1,rp:{before:1500,after:1510}}}},{interval:{start:200},games:{blitz:{win:3,loss:2,draw:1,rp:{before:2000,after:2010}}}}];
  f.replies.set('/api/user/Bob/activity',()=>json(days));await f.client.activity('Bob','perf');
  assert.deepEqual(f.client.snapshot().profileWindows.perf.activity.days.map(d=>d.interval.start),[200,100]);
  const count=f.calls.length;await f.client.activity('Bob','perf');assert.equal(f.calls.length,count);
  await f.client.activity('Bob','perf',true);assert.equal(f.calls.length,count+1);
  await performanceProfile(f,'Carol','other');f.replies.set('/api/user/Carol/activity',()=>json([]));await f.client.activity('Carol','other');
  assert.equal(f.client.snapshot().profileWindows.perf.activity.days.length,2);
  assert.deepEqual(f.client.snapshot().profileWindows.other.activity.days,[]);
});
test('late activity cannot cross players or sessions, and failed loads can be refreshed', async t => {
  const f=fixture();t.after(()=>f.client.disconnect());await f.client.connect('secret');await tick();await performanceProfile(f);
  let resolve;f.replies.set('/api/user/Bob/activity',()=>new Promise(r=>{resolve=r;}));
  const pending=f.client.activity('Bob','perf');await tick();const change=performanceProfile(f,'Carol');
  resolve(json([{interval:{start:1}}]));await Promise.all([pending,change]);
  assert.equal(f.client.snapshot().profileWindows.perf.activity,null);
  f.replies.set('/api/user/Carol/activity',()=>json({error:'Unavailable'},503));await f.client.activity('Carol','perf');
  assert.match(f.client.snapshot().profileWindows.perf.activity.error,/Unavailable/);
  assert.equal(f.client.snapshot().profileWindows.perf.activity.loading,false);
  f.replies.set('/api/user/Carol/activity',()=>json([]));await f.client.activity('Carol','perf',true);
  assert.deepEqual(f.client.snapshot().profileWindows.perf.activity.days,[]);
  f.replies.set('/api/user/Carol/activity',()=>new Promise(r=>{resolve=r;}));
  const closed=f.client.activity('Carol','perf',true);await tick();f.client.disconnect();resolve(json([]));await closed;
  assert.deepEqual(f.client.snapshot().profileWindows,{});
});
