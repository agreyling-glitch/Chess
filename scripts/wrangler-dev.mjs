import { spawn } from 'node:child_process';
import { join } from 'node:path';

const port = Number(process.argv[2] || 8788);
const enginePort = port + 1;
const engine = spawn(process.execPath, ['scripts/engine-server.mjs', String(enginePort)], { stdio: 'inherit' });
const wranglerCli = join('node_modules', 'wrangler', 'bin', 'wrangler.js');
const wrangler = spawn(process.execPath, [wranglerCli,
  'dev', '--port', String(port),
  '--var', `ENGINE_ORIGIN:http://127.0.0.1:${enginePort}`,
], { stdio: 'inherit' });

function stop(code = 0) {
  if (!engine.killed) engine.kill();
  if (!wrangler.killed) wrangler.kill();
  process.exit(code);
}
engine.on('error', error => {
  console.error('Engine preview server failed:', error);
  stop(1);
});
wrangler.on('error', error => {
  console.error('Wrangler failed to start:', error);
  stop(1);
});
engine.on('exit', (code, signal) => {
  console.error(`Engine preview server exited (code=${code}, signal=${signal || 'none'}).`);
  stop(code || 1);
});
wrangler.on('exit', (code, signal) => {
  console.error(`Wrangler exited (code=${code}, signal=${signal || 'none'}).`);
  stop(code || (signal ? 1 : 0));
});
process.on('SIGINT', () => stop(0));
process.on('SIGTERM', () => stop(0));
