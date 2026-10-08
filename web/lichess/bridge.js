import { createClient, HOST, SCOPES, CLIENT_ID } from './client.js';

// Session-only credentials: never included in game storage, exports, or the service worker cache.
const TOKEN_KEY = 'ironwood.lichess.token';
const PKCE_KEY = 'ironwood.lichess.pkce';
const events = [];
const client = createClient({ emit: event => {
  events.push(JSON.stringify(event));
  if (events.length > 250) events.splice(0, events.length - 250);
}, invalidToken: () => sessionStorage.removeItem(TOKEN_KEY) });
const run = operation => Promise.resolve().then(operation).catch(client.reportError);
const base64url = bytes => btoa(String.fromCharCode(...bytes)).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
const random = () => base64url(crypto.getRandomValues(new Uint8Array(32)));

async function signIn() {
  const verifier = random(), state = random(), redirect = new URL('/play/', location.origin).href;
  const challenge = base64url(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier))));
  sessionStorage.setItem(PKCE_KEY, JSON.stringify({ verifier, state, redirect, created: Date.now() }));
  const url = new URL('/oauth', HOST);
  url.search = new URLSearchParams({ response_type: 'code', client_id: CLIENT_ID, redirect_uri: redirect,
    scope: SCOPES, state, code_challenge: challenge, code_challenge_method: 'S256' }).toString();
  location.assign(url.href);
}

async function initialize() {
  const url = new URL(location.href);
  const code = url.searchParams.get('code'), oauthError = url.searchParams.get('error');
  if (code || oauthError) {
    const raw = sessionStorage.getItem(PKCE_KEY);
    sessionStorage.removeItem(PKCE_KEY);
    // Strip authorization codes before the app, history, or diagnostics can retain them.
    const state = url.searchParams.get('state');
    for (const name of ['code', 'state', 'error', 'error_description']) url.searchParams.delete(name);
    history.replaceState(null, '', url.pathname + url.search + url.hash);
    const saved = raw ? JSON.parse(raw) : null;
    if (!saved || state !== saved.state || Date.now() - saved.created > 600000)
      throw new Error('Lichess sign-in could not be verified. Please sign in again.');
    if (oauthError) throw new Error('Lichess sign-in was canceled or denied');
    const response = await fetch(HOST + '/api/token', { method: 'POST', credentials: 'omit', cache: 'no-store',
      body: new URLSearchParams({ grant_type: 'authorization_code', client_id: CLIENT_ID, code,
        code_verifier: saved.verifier, redirect_uri: saved.redirect }) });
    if (!response.ok) throw new Error('Lichess sign-in failed. Please sign in again.');
    const result = await response.json();
    if (!result.access_token || !result.scope?.split(' ').includes('board:play') && result.scope !== undefined)
      throw new Error('Lichess did not grant permission to play');
    sessionStorage.setItem(TOKEN_KEY, result.access_token);
  }
  const token = sessionStorage.getItem(TOKEN_KEY);
  if (token) await client.connect(token);
}

window.ironwoodLichessSignIn = () => run(signIn);
window.ironwoodLichessConnect = () => run(async () => {
  const token = sessionStorage.getItem(TOKEN_KEY);
  if (token) await client.connect(token); else await signIn();
});
window.ironwoodLichessDisconnect = () => client.disconnect();
window.ironwoodLichessSignOut = () => run(async () => {
  await client.revoke();
  sessionStorage.removeItem(TOKEN_KEY); client.disconnect();
});
window.ironwoodLichessPoll = () => events.shift() || '';
window.ironwoodLichessMove = move => run(() => client.move(move));
window.ironwoodLichessCommand = raw => run(async () => {
  const cmd = JSON.parse(raw);
  switch (cmd.type) {
    case 'seek': return client.seek(cmd.options);
    case 'cancelSeek': return client.cancelSeek();
    case 'open': return client.openGame(cmd.id);
    case 'leaveCorrespondence': return client.leaveCorrespondence();
    case 'challenge': return client.challenge(cmd.username, cmd.options);
    case 'ai': return client.challengeAi(cmd.options);
    case 'challengeAction': return client.challengeAction(cmd.id, cmd.action);
    case 'action': return client.action(cmd.action, cmd.text || '');
    default: throw new Error('Unknown Lichess command');
  }
});
void run(initialize);
