import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, readFile, writeFile, rm, rename } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { once } from 'node:events';

const execute = promisify(execFile);
const binary = process.env.COMPANION_BINARY || resolve(`target/debug/ironwood-server${process.platform === 'win32' ? '.exe' : ''}`);
const cli = args => execute(binary, args, { windowsHide: true, timeout: 15000 });

test('update validation pauses clients, then custom-config archive recovery preserves data and credential', { skip: process.platform !== 'win32', timeout: 40000 }, async () => {
  const directory = await mkdtemp(resolve('tmp/companion-update-lifecycle-'));
  const children = [];
  try {
    await cli(['init', '--data-dir', directory]);
    const defaultConfig = join(directory, 'config.json');
    const customConfig = join(directory, 'custom config.json');
    const config = JSON.parse(await readFile(defaultConfig, 'utf8'));
    config.listen = '127.0.0.1:0';
    config.archive = 'archive/custom.sqlite3';
    await writeFile(customConfig, JSON.stringify(config));
    const originalToken = await readFile(join(directory, 'owner-token'), 'utf8');
    const marker = join(directory, 'pending'); await writeFile(marker, 'pending');
    const headers = { Origin: config.allowed_origins[0], Authorization: `Bearer ${originalToken.trim()}`, 'X-Ironwood-Protocol': '1' };
    async function start(pending) {
      const args = ['--config', customConfig]; if (pending) args.push('--update-pending', marker);
      const child = spawn(binary, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); children.push(child);
      let output = '';
      const address = await new Promise((resolveAddress, reject) => {
        const timer = setTimeout(() => reject(Error('startup timed out')), 8000);
        child.once('error', error => { clearTimeout(timer); reject(error); });
        child.once('exit', () => { clearTimeout(timer); reject(Error('startup failed')); });
        child.stdout.on('data', data => { output += data; const match = output.match(/"address":"(127\.0\.0\.1:\d+)"/); if (match) { clearTimeout(timer); resolveAddress(`http://${match[1]}`); } });
      });
      return { child, address };
    }
    async function stop(server) {
      const done = once(server.child, 'close');
      assert.equal((await fetch(server.address + '/api/v1/shutdown', { method: 'POST', headers })).status, 202);
      await done;
    }
    let server = await start(true);
    assert.equal((await fetch(server.address + '/readyz')).status, 200);
    assert.equal((await fetch(server.address + '/api/v1/capabilities', { headers })).status, 503);
    assert.equal((await fetch(server.address + '/api/v1/tickets', { method: 'POST', headers })).status, 503);
    await rm(marker);
    assert.equal((await fetch(server.address + '/api/v1/capabilities', { headers })).status, 200);
    const updates = await fetch(server.address + '/api/v1/updates', { headers });
    assert.equal(updates.status, 200);
    assert.equal((await updates.json()).management, 'external');
    assert.equal((await fetch(server.address + '/api/v1/updates', { headers: { ...headers, Authorization: 'Bearer wrong' } })).status, 401);
    for (const command of ['check', 'install']) {
      const endpoint = server.address + '/api/v1/updates/' + command;
      assert.equal((await fetch(endpoint, { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: '{}' })).status, 409);
      assert.equal((await fetch(endpoint, { method: 'POST', headers: { ...headers, Authorization: 'Bearer wrong', 'Content-Type': 'application/json' }, body: '{}' })).status, 401);
      assert.equal((await fetch(endpoint, { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: '{"url":"https://evil.example"}' })).status, 400);
    }
    const imported = await fetch(server.address + '/api/v1/archive/import', { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify({ pgn: '[White "Recovery fixture"]\n[Black "Preserved"]\n[Result "*"]\n\n1. e4 e5 2. Nf3 Nc6 *' }) });
    assert.equal(imported.status, 200);
    assert.equal((await imported.json()).imported, 1);
    await stop(server);
    const snapshot = join(directory, 'snapshot.sqlite3');
    await cli(['archive-backup', '--config', customConfig, '--output', snapshot]);
    // Recovery preserves the failed database and restores to a new destination.
    await rename(join(directory, config.archive), join(directory, 'failed.sqlite3'));
    await cli(['archive-restore', '--config', customConfig, '--input', snapshot]);
    assert.equal(await readFile(join(directory, 'owner-token'), 'utf8'), originalToken);
    assert.deepEqual(JSON.parse(await readFile(customConfig, 'utf8')), config);
    server = await start(false);
    assert.equal((await fetch(server.address + '/api/v1/capabilities', { headers })).status, 200);
    assert.equal((await (await fetch(server.address + '/api/v1/archive/games', { headers })).json()).games.length, 1);
    await stop(server);
  } finally {
    for (const child of children) if (child.exitCode === null && child.signalCode === null) { const done = once(child, 'close'); child.kill(); await done; }
    await rm(directory, { recursive: true, force: true });
  }
});
