import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { patchThreadPool } from '../scripts/stockfish-thread-pool.mjs';

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
