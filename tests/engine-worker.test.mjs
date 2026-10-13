import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { patchThreadPool } from '../scripts/stockfish-thread-pool.mjs';

function engineWorkerHarness() {
  const workers = [];
  const context = vm.createContext({
    URL, Date, SharedArrayBuffer, crossOriginIsolated: true,
    self: { location: { href: 'https://ironwoodchess.com/engine-worker.js' } },
    postMessage() {},
    MessageChannel: class { port1 = {}; port2 = {}; },
    Worker: class {
      commands = [];
      constructor() { workers.push(this); }
      postMessage(command) { this.commands.push(command); }
    },
  });
  vm.runInContext(readFileSync(new URL('../web/engine-worker.js', import.meta.url), 'utf8'), context);
  return {
    init() {
      context.onmessage({ data: { type: 'init', threads: 8, hash: 64 } });
      return workers.at(-1);
    },
    command(command) { context.onmessage({ data: { type: 'command', command } }); },
    ready(worker) {
      worker.onmessage({ data: 'uciok' });
      worker.onmessage({ data: 'ironwood-thread-pool-ready' });
    },
  };
}

test('engine readiness waits for the asynchronous pthread pool', () => {
  const output = [];
  const commands = [];
  let engine;
  const context = vm.createContext({
    URL, Date, SharedArrayBuffer, crossOriginIsolated: true,
    self: { location: { href: 'http://127.0.0.1:8788/engine-worker.js' } },
    postMessage: message => output.push(message),
    MessageChannel: class { port1 = {}; port2 = {}; },
    Worker: class { constructor(url) { engine = this; this.url = url; } postMessage(command) { commands.push(command); } },
  });
  vm.runInContext(readFileSync(new URL('../web/engine-worker.js', import.meta.url), 'utf8'), context);
  vm.runInContext('onmessage({data:{type:"init",threads:8,hash:64}})', context);
  assert.equal(new URL(engine.url).pathname, '/engine/stockfish-19-ironwood-pool-v1.js');
  engine.onmessage({ data: 'uciok' });
  assert.equal(output.some(message => message.line === 'uciok'), false);
  assert.equal(commands.some(command => typeof command === 'string' && command.startsWith('setoption')), false);
  assert.equal(commands.at(-1).prepareThreads, 8);
  vm.runInContext('onmessage({data:{type:"command",command:"isready"}})', context);
  engine.onmessage({ data: 'ironwood-thread-pool-ready' });
  assert.ok(commands.includes('setoption name Threads value 8'));
  assert.equal(commands.at(-1), 'isready');
  assert.equal(output.at(-1).line, 'uciok');
});

test('queued searches reuse initial resource settings after pool preparation', () => {
  const harness = engineWorkerHarness();
  const worker = harness.init();
  for (const command of [
    'stop',
    'setoption name Threads value 8',
    'setoption name Hash value 64',
    'position startpos',
    'isready',
    'go nodes 1000',
  ]) harness.command(command);
  assert.deepEqual(worker.commands.filter(command => typeof command === 'string'), ['uci']);
  harness.ready(worker);
  assert.deepEqual(worker.commands.filter(command => typeof command === 'string'), [
    'uci',
    'setoption name Threads value 8',
    'setoption name Hash value 64',
    'setoption name Skill Level value 20',
    'setoption name UCI_LimitStrength value false',
    'setoption name UCI_Elo value 1800',
    'setoption name Ponder value false',
    'stop',
    'position startpos',
    'isready',
    'go nodes 1000',
  ]);
});

test('resource changes apply once while search and reset commands retain their order', () => {
  const harness = engineWorkerHarness();
  const worker = harness.init();
  harness.ready(worker);
  worker.commands.length = 0;
  for (const command of [
    'setoption name Threads value 8',
    'setoption name Hash value 64',
    'setoption name Threads value 4',
    'setoption name Hash value 128',
    'setoption name Threads value 4',
    'setoption name Hash value 128',
    'setoption name Threads value 8',
    'setoption name Hash value 64',
  ]) harness.command(command);
  const searchCommands = [
    'stop', 'stop',
    'ucinewgame', 'ucinewgame',
    'setoption name Clear Hash', 'setoption name Clear Hash',
    'isready', 'isready',
    'position startpos',
    'go nodes 1000', 'go nodes 1000',
  ];
  for (const command of searchCommands) harness.command(command);
  harness.command('setoption name Threads value 8');
  harness.command('setoption name Hash value 64');
  assert.deepEqual(worker.commands, [
    'setoption name Threads value 4',
    'setoption name Hash value 128',
    'setoption name Threads value 8',
    'setoption name Hash value 64',
    ...searchCommands,
  ]);
});

test('a replacement engine applies its initial resource settings again', () => {
  const harness = engineWorkerHarness();
  harness.ready(harness.init());
  const replacement = harness.init();
  harness.command('setoption name Threads value 8');
  harness.command('setoption name Hash value 64');
  assert.deepEqual(replacement.commands.filter(command => typeof command === 'string'), ['uci']);
  harness.ready(replacement);
  assert.deepEqual(replacement.commands.filter(command => typeof command === 'string'
    && /^setoption name (Threads|Hash) /.test(command)), [
    'setoption name Threads value 8',
    'setoption name Hash value 64',
  ]);
});

test('production serves patched glue from app assets and WASM from engine origin', async () => {
  const source = readFileSync(new URL('../src/wrangler-worker.js', import.meta.url), 'utf8')
    .replace('import { connect } from "cloudflare:sockets";', '')
    .replace('export default {', 'globalThis.handler = {');
  const upstream = [];
  const assets = [];
  const context = vm.createContext({ URL, Request, Response, Headers,
    fetch: async request => { upstream.push(request.url); return new Response('wasm'); },
  });
  vm.runInContext(source, context);
  const env = { ENGINE_ORIGIN: 'https://engine.example.com', ASSETS: {
    fetch: async request => { assets.push(request.url); return new Response('patched glue'); },
  } };
  const response = await context.handler.fetch(new Request('https://ironwoodchess.com/engine/stockfish-19-ironwood-pool-v1.js'), env);
  assert.equal(await response.text(), 'patched glue');
  assert.equal(assets.length, 1);
  assert.equal(upstream.length, 0);
  assert.equal(response.headers.get('Cross-Origin-Embedder-Policy'), 'require-corp');
  await context.handler.fetch(new Request('https://ironwoodchess.com/engine/stockfish-19.wasm'), env);
  assert.deepEqual(upstream, ['https://engine.example.com/stockfish-19.wasm']);
});

test('pinned engine adapter waits for all idle workers to finish loading', async () => {
  const source = readFileSync(new URL('../node_modules/stockfish/bin/stockfish-19.js', import.meta.url), 'utf8');
  const patched = patchThreadPool(source);
  const handler = patched.slice(patched.indexOf('else if(e.data.prepareThreads){'), patched.indexOf('else if(f.processCommand?'));
  const callbacks = [];
  const output = [];
  const pool = { ea: [], ja: [{}], Aa() { this.ea.push({}); }, Ha(worker, callback) { callbacks.push(callback); } };
  const context = vm.createContext({ f: { PThread: pool }, postMessage: line => output.push(line), e: { data: { prepareThreads: 8 } } });
  vm.runInContext('if(false){}' + handler, context);
  assert.equal(callbacks.length, 7);
  assert.equal(output.length, 0);
  callbacks.slice(0, 6).forEach(callback => callback());
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(output.length, 0);
  callbacks[6]();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(output, ['ironwood-thread-pool-ready']);
  assert.throws(() => patchThreadPool('different engine'), /needs review/);
});
