import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createCompanionDiagnostics } from '../web/companion-diagnostics.js';
const capabilities = { product: 'Ironwood Companion', protocol_version: '1', server_version: '0.2.0' };
const available = { state: 'available', message: 'Version 0.3.0 is ready.', management: 'native_windows', available_version: '0.3.0', can_check: true, can_install: true };
const connection = () => ({ endpoint: 'http://localhost:8790', credential: 'private-token' });

test('diagnostics stay private behind the flag and reuse the shared connection', async () => {
  let enabled = false; const calls = [];
  const diagnostics = createCompanionDiagnostics({ enabled: () => enabled, connection, client: current => {
    assert.equal(current.credential, 'private-token');
    return { request: async (path, body) => { calls.push([path, body]); return path === 'capabilities' ? capabilities : available; } };
  } });
  assert.equal(diagnostics.snapshot(), '{}'); await diagnostics.check(); await diagnostics.install();
  assert.equal(calls.length, 0);
  enabled = true; await diagnostics.refresh();
  let snapshot = JSON.parse(diagnostics.snapshot());
  assert.equal(snapshot.connection_status, 'Connected'); assert.equal(snapshot.version, '0.2.0');
  assert.equal(snapshot.available_version, '0.3.0'); assert.equal(snapshot.can_install, true);
  assert.doesNotMatch(diagnostics.snapshot(), /private-token|localhost/);
  await diagnostics.check(); assert.deepEqual(calls.at(-1), ['updates/check', {}]);
  await diagnostics.install(); assert.deepEqual(calls.at(-1), ['updates/install', {}]);
  enabled = false; diagnostics.reset(); assert.equal(diagnostics.snapshot(), '{}');
});

test('older servers retain the version and explain manual updates; disconnected and stale responses clear controls', async () => {
  let current = connection(), finish;
  const diagnostics = createCompanionDiagnostics({ enabled: () => true, connection: () => current, client: () => ({ request: async path => {
    if (path === 'capabilities') return capabilities;
    const error = Error('Unknown endpoint'); error.status = 404; throw error;
  } }) });
  await diagnostics.refresh();
  let snapshot = JSON.parse(diagnostics.snapshot());
  assert.equal(snapshot.version, '0.2.0'); assert.equal(snapshot.connection_status, 'Connected');
  assert.match(snapshot.message, /manually/); assert.equal(snapshot.can_install, false);
  current = null; diagnostics.reset(); await diagnostics.refresh();
  snapshot = JSON.parse(diagnostics.snapshot()); assert.equal(snapshot.connection_status, 'Disconnected'); assert.equal(snapshot.version, '');
  current = connection();
  const stale = createCompanionDiagnostics({ enabled: () => true, connection: () => current, client: () => ({ request: () => new Promise(resolve => { finish = resolve; }) }) });
  const request = stale.refresh(); current = null; stale.reset(); finish(capabilities); await request;
  assert.equal(JSON.parse(stale.snapshot()).connection_status, 'Disconnected');
});

test('busy engines disable installation, Docker guidance is preserved, and restart probes recover', async () => {
  let response = { ...available, can_install: false }, offline = false, now = 1000;
  const diagnostics = createCompanionDiagnostics({ enabled: () => true, connection, clock: () => now, client: () => ({ request: async path => {
    if (offline) throw Error('Connection lost');
    if (path === 'capabilities') return capabilities;
    return response;
  } }) });
  await diagnostics.refresh(); assert.equal(JSON.parse(diagnostics.snapshot()).can_install, false);
  response = { state: 'unsupported', management: 'container', message: 'Use Container Station to update the image.', can_check: false, can_install: false };
  await diagnostics.refresh(); assert.match(JSON.parse(diagnostics.snapshot()).message, /Container Station/);
  response = { ...available, state: 'installing', can_check: false, can_install: false };
  await diagnostics.install(); offline = true; now += 2000; await diagnostics.refresh();
  assert.equal(JSON.parse(diagnostics.snapshot()).connection_status, 'Restarting');
  offline = false; response = { ...available, state: 'up_to_date', available_version: null, can_install: false };
  await diagnostics.refresh(); assert.equal(JSON.parse(diagnostics.snapshot()).connection_status, 'Connected');
  offline = true; await diagnostics.refresh(); assert.equal(JSON.parse(diagnostics.snapshot()).connection_status, 'Disconnected');
});
