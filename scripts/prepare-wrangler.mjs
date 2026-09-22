import { cpSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, relative } from 'node:path';

const root = join(import.meta.dirname, '..');
const source = join(root, 'web');
const hugoSource = join(root, '.hugo-public');
const target = join(root, '.wrangler-assets');

mkdirSync(target, { recursive: true });
for (const entry of readdirSync(target)) {
  rmSync(join(target, entry), { recursive: true, force: true });
}
cpSync(source, target, {
  recursive: true,
  filter(path) {
    const [topLevel] = relative(source, path).split(/[\\/]/);
    return topLevel !== 'engine';
  },
});
cpSync(hugoSource, target, {
  recursive: true,
  force: true,
  filter(path) {
    return relative(hugoSource, path).replaceAll('\\', '/') !== 'index.html';
  },
});

const packageVersion = createHash('sha256')
  .update(readFileSync(join(target, 'pkg', 'battle_chess.js')))
  .update(readFileSync(join(target, 'pkg', 'battle_chess_bg.wasm')))
  .digest('hex')
  .slice(0, 16);
const serviceWorkerPath = join(target, 'service-worker.js');
const serviceWorker = readFileSync(serviceWorkerPath, 'utf8')
  .replaceAll('__APP_PACKAGE_VERSION__', packageVersion);
writeFileSync(serviceWorkerPath, serviceWorker);
