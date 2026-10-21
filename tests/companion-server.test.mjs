import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { once } from 'node:events';
import WebSocket from 'ws';

// Run after cargo build -p ironwood-server. Ephemeral process has no user data.
test('native HTTP and WebSocket authentication, replay, origin, and protocol checks', { timeout: 20000 }, async () => {
  const directory = await mkdtemp(resolve('tmp/companion-test-'));
  const token = 'a'.repeat(64); // Test-only credential; never used by production setup.
  const origin = 'http://localhost:8788';
  await writeFile(join(directory, 'owner-token'), token);
  await writeFile(join(directory, 'config.json'), JSON.stringify({ listen: '127.0.0.1:0', allowed_origins: [origin], token_file: 'owner-token' }));
  const executable = process.env.COMPANION_BINARY || resolve(`target/debug/ironwood-server${process.platform === 'win32' ? '.exe' : ''}`);
  const child = spawn(executable, ['--config', join(directory, 'config.json')], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const sockets = [];
  try {
    const address = await new Promise((resolveAddress, reject) => {
      let output = '';
      const timeout = setTimeout(() => reject(new Error('Server startup timed out')), 5000);
      child.once('error', error => { clearTimeout(timeout); reject(error); });
      child.once('exit', () => { clearTimeout(timeout); reject(new Error('Server exited during startup')); });
      child.stdout.on('data', chunk => {
        output += chunk;
        const match = output.match(/"address":"(127\.0\.0\.1:\d+)"/);
        if (match) { clearTimeout(timeout); resolveAddress(`http://${match[1]}`); }
      });
    });
    const headers = { Origin: origin, Authorization: `Bearer ${token}`, 'X-Ironwood-Protocol': '1' };
    const request = (path, options = {}) => fetch(address + path, { headers, ...options });
    const response = await request('/api/v1/capabilities');
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('access-control-allow-origin'), origin);
    assert.equal((await response.json()).services.uci, false);
    assert.equal((await request('/api/v1/capabilities', { headers: { ...headers, Origin: 'https://evil.example' } })).status, 403);
    assert.equal((await request('/api/v1/capabilities', { headers: { ...headers, Authorization: 'Bearer wrong' } })).status, 401);
    assert.equal((await request('/api/v1/capabilities', { headers: { ...headers, 'X-Ironwood-Protocol': '2' } })).status, 426);
    assert.equal((await request('/api/v1/capabilities', { method: 'OPTIONS', headers: {
      Origin: origin, 'Access-Control-Request-Method': 'GET', 'Access-Control-Request-Headers': 'authorization,x-ironwood-protocol',
    } })).status, 204);
    assert.equal((await request('/api/v1/capabilities', { method: 'OPTIONS', headers: {
      Origin: origin, 'Access-Control-Request-Method': 'DELETE',
    } })).status, 403);
    const { ticket } = await (await request('/api/v1/tickets', { method: 'POST' })).json();
    const connect = async (connectionOrigin = origin) => {
      const socket = new WebSocket(address.replace('http:', 'ws:') + '/api/v1/socket', { origin: connectionOrigin });
      socket.on('error', () => {});
      sockets.push(socket);
      await once(socket, 'open');
      return socket;
    };
    const exchange = (socket, value) => {
      const reply = once(socket, 'message');
      socket.send(JSON.stringify(value));
      return reply.then(([data]) => JSON.parse(data.toString()));
    };
    const socket = await connect();
    assert.equal((await exchange(socket, { type: 'authenticate', ticket })).type, 'ready');
    assert.deepEqual(await exchange(socket, { type: 'ping', request_id: 'correlation' }), { type: 'pong', request_id: 'correlation' });
    const replay = await connect();
    assert.equal((await exchange(replay, { type: 'authenticate', ticket })).code, 'unauthorized');
    const unauthenticated = await connect();
    assert.equal((await exchange(unauthenticated, { type: 'ping', request_id: 'no-auth' })).code, 'unauthorized');
    const denied = new WebSocket(address.replace('http:', 'ws:') + '/api/v1/socket', { origin: 'https://evil.example' });
    denied.on('error', () => {});
    sockets.push(denied);
    const [deniedRequest, deniedResponse] = await once(denied, 'unexpected-response');
    assert.equal(deniedResponse.statusCode, 403);
    deniedResponse.resume();
    deniedRequest.destroy();
  } finally {
    for (const socket of sockets) socket.terminate();
    if (child.exitCode === null && child.signalCode === null) {
      const exited = once(child, 'exit');
      child.kill();
      await exited;
    }
    await rm(directory, { recursive: true, force: true });
  }
});
