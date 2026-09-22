import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize } from 'node:path';

const root = normalize(join(import.meta.dirname, '..', 'web'));
const mime = {'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.nnue':'application/octet-stream'};
createServer((req,res) => {
  const urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
  let file = normalize(join(root, urlPath === '/' ? 'index.html' : urlPath));
  if (!file.startsWith(root) || !existsSync(file) || statSync(file).isDirectory()) file = join(root, 'index.html');
  res.setHeader('Cross-Origin-Opener-Policy','same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy','require-corp');
  res.setHeader('Cross-Origin-Resource-Policy','same-origin');
  res.setHeader('Content-Type', mime[extname(file)] || 'application/octet-stream');
  res.setHeader('Cache-Control', 'no-cache');
  if (urlPath === '/service-worker.js') {
    const wasm = statSync(join(root, 'pkg', 'battle_chess_bg.wasm'));
    const revision = `${wasm.size}-${Math.trunc(wasm.mtimeMs)}`;
    const worker = readFileSync(file, 'utf8').replaceAll('__APP_PACKAGE_VERSION__', revision);
    res.end(worker);
    return;
  }
  createReadStream(file).pipe(res);
}).listen(8080, '127.0.0.1', () => console.log('Ironwood Chess: http://127.0.0.1:8080'));
