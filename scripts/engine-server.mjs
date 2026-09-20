import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize } from 'node:path';

const port = Number(process.argv[2] || 8789);
const root = normalize(join(import.meta.dirname, '..', 'web', 'engine'));
const mime = { '.js': 'text/javascript', '.wasm': 'application/wasm' };

createServer((req, res) => {
  const pathname = decodeURIComponent((req.url || '/').split('?')[0]);
  const file = normalize(join(root, pathname));
  if (!file.startsWith(root) || !existsSync(file) || statSync(file).isDirectory()) {
    res.writeHead(404).end('Not found');
    return;
  }
  res.setHeader('Content-Type', mime[extname(file)] || 'application/octet-stream');
  res.setHeader('Cross-Origin-Resource-Policy', 'same-origin');
  res.setHeader('Cache-Control', 'public, max-age=31536000, immutable');
  const size = statSync(file).size;
  res.setHeader('Content-Length', String(size));
  res.setHeader('Accept-Ranges', 'bytes');
  if (req.method === 'HEAD') {
    res.end();
    return;
  }

  const stream = createReadStream(file);
  stream.pipe(res);
  stream.on('error', error => res.destroy(error));
}).listen(port, '127.0.0.1', () => console.log(`Full NNUE engine server: http://127.0.0.1:${port}`));
