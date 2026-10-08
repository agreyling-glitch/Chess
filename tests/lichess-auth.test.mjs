import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';

const tick = () => new Promise(resolve => setImmediate(resolve));
let sequence = 0;
async function setup(href, saved) {
  const data = new Map(Object.entries(saved || {})), calls = [];
  let assigned, replaced;
  globalThis.window = {};
  globalThis.sessionStorage = { getItem: key => data.get(key) || null, setItem: (key, value) => data.set(key, value), removeItem: key => data.delete(key) };
  globalThis.location = { href, origin: new URL(href).origin, assign: value => { assigned = value; } };
  globalThis.history = { replaceState: (_a, _b, value) => { replaced = value; } };
  Object.defineProperty(globalThis, 'crypto', { configurable: true, value: webcrypto });
  globalThis.fetch = async (url, options = {}) => {
    calls.push({ url, options });
    const path = new URL(url).pathname;
    if (path === '/api/token' && options.method === 'POST') return Response.json({ access_token: 'private-token', expires_in: 31536000 });
    if (path === '/api/token' && options.method === 'DELETE') return new Response(null, { status: 204 });
    if (path === '/api/account') return Response.json({ id: 'alice', username: 'Alice' });
    if (path === '/api/account/playing') return Response.json({ nowPlaying: [] });
    if (path === '/api/challenge') return Response.json({ in: [], out: [] });
    if (path === '/api/stream/event') return new Response(new ReadableStream({ start(controller) {
      options.signal.addEventListener('abort', () => controller.error(new DOMException('Aborted', 'AbortError')), { once: true });
    } }));
    throw new Error(`Unexpected request ${path}`);
  };
  await import(`../web/lichess/bridge.js?test=${++sequence}`); await tick();
  return { data, calls, window: globalThis.window, assigned: () => assigned, replaced: () => replaced,
    events: () => { const result = []; for (let raw; (raw = window.ironwoodLichessPoll());) result.push(JSON.parse(raw)); return result; } };
}

test('sign-in generates a unique S256 PKCE request with only the required scopes', async t => {
  const f = await setup('https://ironwoodchess.com/play/'); t.after(() => f.window.ironwoodLichessDisconnect());
  await f.window.ironwoodLichessSignIn();
  const url = new URL(f.assigned()), saved = JSON.parse(f.data.get('ironwood.lichess.pkce'));
  assert.equal(url.origin, 'https://lichess.org'); assert.equal(url.pathname, '/oauth');
  assert.equal(url.searchParams.get('code_challenge_method'), 'S256');
  assert.equal(url.searchParams.get('scope'), 'board:play challenge:read challenge:write');
  assert.equal(url.searchParams.get('state'), saved.state); assert.ok(saved.verifier.length >= 43);
  const hash = Buffer.from(await webcrypto.subtle.digest('SHA-256', new TextEncoder().encode(saved.verifier))).toString('base64url');
  assert.equal(url.searchParams.get('code_challenge'), hash);
  assert.equal(url.searchParams.get('redirect_uri'), 'https://ironwoodchess.com/play/');
  assert.equal(f.data.has('ironwood.lichess.token'), false);
});

test('a valid callback exchanges the one-use code, strips it from history, and revokes on sign-out', async t => {
  const saved = { verifier: 'v'.repeat(43), state: 'correct-state', redirect: 'https://ironwoodchess.com/play/', created: Date.now() };
  const f = await setup('https://ironwoodchess.com/play/?code=one-use-code&state=correct-state', { 'ironwood.lichess.pkce': JSON.stringify(saved) });
  t.after(() => f.window.ironwoodLichessDisconnect()); await tick();
  assert.equal(f.replaced(), '/play/'); assert.equal(f.data.has('ironwood.lichess.pkce'), false);
  assert.equal(f.data.get('ironwood.lichess.token'), 'private-token');
  assert.equal(f.calls[0].options.body.get('code_verifier'), saved.verifier);
  assert.equal(f.calls[0].options.body.get('redirect_uri'), saved.redirect);
  assert.equal(f.calls[0].options.cache, 'no-store');
  assert.ok(!JSON.stringify(f.events()).includes('private-token'));
  await f.window.ironwoodLichessSignOut();
  assert.equal(f.data.has('ironwood.lichess.token'), false);
  assert.equal(f.calls.at(-1).options.method, 'DELETE');
  assert.equal(f.calls.at(-1).options.headers.Authorization, 'Bearer private-token');
});

test('a forged state or expired verifier never sends an authorization code to the token endpoint', async () => {
  for (const [state, age] of [['forged', 0], ['correct', 600001]]) {
    const f = await setup(`https://ironwoodchess.com/play/?code=secret-code&state=${state}`, {
      'ironwood.lichess.pkce': JSON.stringify({ verifier: 'v'.repeat(43), state: 'correct', created: Date.now() - age }) });
    assert.equal(f.calls.length, 0); assert.equal(f.replaced(), '/play/');
    assert.match(f.events().find(e => e.type === 'error').message, /could not be verified/);
  }
});

test('denied sign-in removes callback credentials and leaves no session token', async () => {
  const f = await setup('https://ironwoodchess.com/play/?error=access_denied&state=correct', {
    'ironwood.lichess.pkce': JSON.stringify({ verifier: 'v'.repeat(43), state: 'correct', created: Date.now() }) });
  assert.equal(f.calls.length, 0); assert.equal(f.replaced(), '/play/');
  assert.match(f.events().find(e => e.type === 'error').message, /canceled or denied/);
});
