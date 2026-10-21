import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, readFile, writeFile, rm, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { once } from 'node:events';
import WebSocket from 'ws';

const execute = promisify(execFile);
const binary = process.env.COMPANION_BINARY || resolve(`target/debug/ironwood-server${process.platform === 'win32' ? '.exe' : ''}`);
const origin = 'http://localhost:8788';
const cli = args => execute(binary, args, { windowsHide: true, timeout: 10000 });

async function start(directory) {
  const child = spawn(binary, ['--data-dir', directory], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.on('data', data => { output += data; });
  child.stderr.on('data', data => { output += data; });
  const address = await new Promise((resolveAddress, reject) => {
    const timer = setTimeout(() => reject(new Error('Companion startup timed out')), 8000);
    child.once('error', error => { clearTimeout(timer); reject(error); });
    child.once('exit', () => { clearTimeout(timer); reject(new Error('Companion exited before listening')); });
    child.stdout.on('data', () => {
      const match = output.match(/"address":"(127\.0\.0\.1:\d+)"/);
      if (match) { clearTimeout(timer); resolveAddress(`http://${match[1]}`); }
    });
  });
  return { child, address, output: () => output };
}

test('native provisioning, persistence, access control, rotation, health and graceful shutdown', { timeout: 40000 }, async () => {
  const parent = await mkdtemp(resolve('tmp/companion-lifecycle-'));
  const directory = join(parent, 'Companion data ü with spaces');
  const children = [];
  const sockets = [];
  const logs = [];
  try {
    const initialized = await cli(['init', '--data-dir', directory]);
    logs.push(initialized.stdout, initialized.stderr);
    const original = (await readFile(join(directory, 'owner-token'), 'utf8')).trim();
    assert.match(original, /^[a-f0-9]{64}$/);
    if (process.platform === 'win32') {
      const acl = await execute('icacls', [directory], { windowsHide: true });
      assert.doesNotMatch(acl.stdout, /\(I\)/); // No inherited access entries.
      assert.match(acl.stdout, /\(OI\)\(CI\)\(F\)/);
    } else {
      assert.equal((await stat(directory)).mode & 0o777, 0o700);
      assert.equal((await stat(join(directory, 'owner-token'))).mode & 0o777, 0o600);
    }
    // Idempotent initialization must preserve existing credentials and settings.
    await cli(['init', '--data-dir', directory]);
    assert.equal((await readFile(join(directory, 'owner-token'), 'utf8')).trim(), original);
    const config = JSON.parse(await readFile(join(directory, 'config.json'), 'utf8'));
    assert.equal(config.archive,'archive/archive.sqlite3');
    assert.equal((await stat(join(directory,'engines'))).isDirectory(),true);
    assert.equal((await stat(join(directory,'archive'))).isDirectory(),true);
    assert.equal(config.listen, '127.0.0.1:8790');
    config.listen = '127.0.0.1:0';
    config.limits.max_connections = 2;
    config.limits.max_pending_tickets = 1;
    config.limits.max_request_bytes = 256;
    await writeFile(join(directory, 'config.json'), JSON.stringify(config));
    let server = await start(directory); children.push(server.child);
    await assert.rejects(cli(['--start-if-needed','--data-dir',directory]),error=>{
      assert.equal(error.code,20);assert.match(error.stdout,/Companion is already running/);
      assert.doesNotMatch(error.stdout+error.stderr,/startup_failed/);return true;
    });
    const headers = token => ({ Origin: origin, Authorization: `Bearer ${token}`, 'X-Ironwood-Protocol': '1' });
    const request = (path, options = {}) => fetch(server.address + path, { headers: headers(original), ...options });
    assert.equal((await fetch(server.address + '/healthz')).status, 200);
    assert.equal((await fetch(server.address + '/readyz')).status, 200);
    await cli(['healthcheck', '--address', new URL(server.address).host]);
    await assert.rejects(cli(['rotate-token', '--data-dir', directory]));
    assert.equal((await readFile(join(directory, 'owner-token'), 'utf8')).trim(), original);
    const large = await request('/api/v1/tickets', { method: 'POST', body: 'x'.repeat(257) });
    assert.equal(large.status, 413);
    assert.equal((await large.json()).code, 'request_too_large');
    const invalid = await request('/api/v1/tickets', { method: 'POST', body: 'x' });
    assert.equal(invalid.status, 400);
    const first = await request('/api/v1/tickets', { method: 'POST' });
    const { ticket } = await first.json();
    assert.equal((await request('/api/v1/tickets', { method: 'POST' })).status, 429);
    const ws = new WebSocket(server.address.replace('http:', 'ws:') + '/api/v1/socket', { origin });
    ws.on('error', () => {}); sockets.push(ws);
    await once(ws, 'open');
    const ready = once(ws, 'message'); ws.send(JSON.stringify({ type: 'authenticate', ticket }));
    assert.equal(JSON.parse((await ready)[0]).type, 'ready');
    const waiting = new WebSocket(server.address.replace('http:', 'ws:') + '/api/v1/socket', { origin });
    waiting.on('error', () => {}); sockets.push(waiting);
    await once(waiting, 'open');
    const excess = new WebSocket(server.address.replace('http:', 'ws:') + '/api/v1/socket', { origin });
    excess.on('error', () => {}); sockets.push(excess);
    const [excessRequest, excessResponse] = await once(excess, 'unexpected-response');
    assert.equal(excessResponse.statusCode, 429);
    excessResponse.resume(); excessRequest.destroy();
    const waitingClosed = once(waiting, 'close');
    const closed = once(ws, 'close');
    // close follows exit and also waits for stdout/stderr to finish draining.
    const exited = once(server.child, 'close');
    assert.equal((await request('/api/v1/shutdown', { method: 'POST' })).status, 202);
    await closed;
    await waitingClosed;
    const [code] = await exited;
    assert.equal(code, 0);
    logs.push(server.output());
    assert.match(server.output(), /"event":"stopped"/);
    assert.doesNotMatch(server.output(), /shutdown_deadline/);
    server = await start(directory); children.push(server.child);
    assert.equal((await request('/api/v1/capabilities')).status, 200);
    assert.deepEqual((await (await request('/api/v1/capabilities')).json()).services,
      { uci: false, archive: true, explorer: true, tablebases: false });
    let stopped = once(server.child, 'exit');
    await request('/api/v1/shutdown', { method: 'POST' }); await stopped; logs.push(server.output());
    await cli(['rotate-token', '--data-dir', directory]);
    const rotated = (await readFile(join(directory, 'owner-token'), 'utf8')).trim();
    assert.notEqual(rotated, original);
    server = await start(directory); children.push(server.child);
    assert.equal((await request('/api/v1/capabilities')).status, 401);
    assert.equal((await request('/api/v1/capabilities', { headers: headers(rotated) })).status, 200);
    stopped = once(server.child, 'exit');
    await request('/api/v1/shutdown', { method: 'POST', headers: headers(rotated) }); await stopped; logs.push(server.output());
    const combined = logs.join('\n');
    assert.ok(!combined.includes(original) && !combined.includes(rotated) && !combined.includes(ticket));
    for (const line of combined.split('\n').filter(Boolean)) assert.equal(typeof JSON.parse(line).event, 'string');
  } finally {
    for (const socket of sockets) socket.terminate();
    for (const child of children) if (child.exitCode === null && child.signalCode === null) {
      const exited = once(child, 'exit'); child.kill(); await exited;
    }
    // The parent is an explicitly created disposable fixture inside workspace tmp.
    await rm(parent, { recursive: true, force: true });
  }
});
