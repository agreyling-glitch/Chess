import { installCompanionFeature } from './companion-feature.js';
installCompanionFeature(window);
if (!window.ironwoodCompanionFeatureEnabled()) {
  document.querySelector('main').innerHTML = '<h1>Diagnostics disabled</h1><p>Enable Companion in Ironwood’s Help → About → Version diagnostics first.</p><a href="/play/">Back to Ironwood</a>';
}
import { requireCompanionConnection, manageCompanionConnection } from './companion-connection.js';
if (document.querySelector('#manage')) document.querySelector('#manage').onclick = () => manageCompanionConnection();
// Standalone feasibility probe. Does not touch game storage or the engine worker.
const form = document.querySelector('#connection');
const results = document.querySelector('#results');
const button = document.querySelector('#check');

async function exchange(base, ticket, expectReady = true) {
  const address = new URL('/api/v1/socket', base);
  address.protocol = address.protocol === 'https:' ? 'wss:' : 'ws:';
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(address);
    const timer = setTimeout(() => finish(new Error('WebSocket check timed out')), 8000);
    let finished = false;
    function finish(error) {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      socket.close();
      if (error) reject(error); else resolve();
    }
    socket.onopen = () => socket.send(JSON.stringify({ type: 'authenticate', ticket }));
    socket.onerror = () => finish(new Error('WebSocket connection failed; check browser permissions and server origin settings'));
    socket.onclose = () => { if (!finished) finish(new Error('Connection closed before completing checks')); };
    socket.onmessage = event => {
      try {
        const message = JSON.parse(event.data);
        if (!expectReady) {
          if (message.type === 'error' && message.code === 'unauthorized') finish();
          else finish(new Error('A reused ticket was accepted'));
        } else if (message.type === 'ready' && message.protocol_version === '1') {
          socket.send(JSON.stringify({ type: 'ping', request_id: 'phase-0' }));
        } else if (message.type === 'pong' && message.request_id === 'phase-0') finish();
        else finish(new Error('Unexpected Companion response'));
      } catch { finish(new Error('Invalid Companion response')); }
    };
  });
}

form?.addEventListener('submit', async event => {
  event.preventDefault();
  button.disabled = true;
  results.textContent = '';
  const log = text => { results.textContent += `${text}\n`; };
  let credential = '';
  try {
    const shared = await requireCompanionConnection();
    if (!shared) return;
    credential = shared.credential;
    const base = new URL(shared.endpoint);
    if (!['http:', 'https:'].includes(base.protocol) || base.username || base.password || base.pathname !== '/' || base.search || base.hash) {
      throw new Error('Enter an HTTP(S) origin without a path, credentials, query, or fragment');
    }
    log(`Page origin: ${location.origin}`);
    log(`Secure context: ${isSecureContext}; cross-origin isolated: ${crossOriginIsolated}`);
    const headers = { Authorization: `Bearer ${credential}`, 'X-Ironwood-Protocol': '1' };
    const request = async (path, overrides = {}) => fetch(new URL(path, base), {
      headers, cache: 'no-store', credentials: 'omit', redirect: 'error', signal: AbortSignal.timeout(8000), ...overrides,
    });
    const capabilities = await request('/api/v1/capabilities');
    if (!capabilities.ok) throw new Error(`Capabilities rejected (${capabilities.status}); check credential, origin, and protocol`);
    const caps = await capabilities.json();
    if (caps.protocol_version !== '1' || caps.product !== 'Ironwood Companion') throw new Error('Unexpected server or protocol');
    log('PASS authenticated capabilities + CORS');
    log(`Services: ${JSON.stringify(caps.services)}`);
    const wrongCredential = await request('/api/v1/capabilities', { headers: { ...headers, Authorization: 'Bearer invalid' } });
    if (wrongCredential.status !== 401) throw new Error('Wrong credential was not rejected');
    log('PASS wrong credential rejected');
    const wrongVersion = await request('/api/v1/capabilities', { headers: { ...headers, 'X-Ironwood-Protocol': '999' } });
    if (wrongVersion.status !== 426) throw new Error('Unsupported protocol was not rejected');
    log('PASS unsupported protocol rejected');
    const response = await request('/api/v1/tickets', { method: 'POST' });
    if (!response.ok) throw new Error(`Connection ticket rejected (${response.status})`);
    const { ticket } = await response.json();
    await exchange(base, ticket);
    log('PASS authenticated WebSocket + correlated ping');
    await exchange(base, ticket, false);
    log('PASS ticket replay rejected');
    log('Connection checks complete. No game or engine state changed.');
  } catch (error) {
    log(`FAIL ${error.message}`);
    log('If the browser blocked local-network access or mixed content, use the documented same-origin/local probe or an HTTPS endpoint. Do not disable browser security.');
  } finally { credential = ''; button.disabled = false; }
});
