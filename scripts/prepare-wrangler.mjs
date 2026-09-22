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
const engineVersion = createHash('sha256')
  .update(readFileSync(join(source, 'engine', 'stockfish-19.js')))
  .update(readFileSync(join(source, 'engine', 'stockfish-19.wasm')))
  .digest('hex')
  .slice(0, 16);

function hashTree(directory, hash, prefix = '') {
  for (const entry of readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const path = join(directory, entry.name);
    const name = `${prefix}${entry.name}`;
    if (entry.isDirectory()) hashTree(path, hash, `${name}/`);
    else if (name !== 'service-worker.js' && name !== 'app-version.json') {
      hash.update(name).update(readFileSync(path));
    }
  }
}

const shellHash = createHash('sha256');
hashTree(target, shellHash);
const shellVersion = shellHash.digest('hex').slice(0, 20);

const pwaPath = join(target, 'pwa.js');
writeFileSync(pwaPath, readFileSync(pwaPath, 'utf8')
  .replaceAll('__APP_SHELL_VERSION__', shellVersion)
  .replaceAll('__ENGINE_CACHE_VERSION__', engineVersion));
writeFileSync(join(target, 'app-version.json'), `${JSON.stringify({
  version: shellVersion,
  package_version: packageVersion,
  engine: { name: 'Stockfish', release: 19, cache_version: engineVersion },
})}\n`);

const serviceWorkerPath = join(target, 'service-worker.js');
const serviceWorker = readFileSync(serviceWorkerPath, 'utf8')
  .replaceAll('__APP_SHELL_VERSION__', shellVersion)
  .replaceAll('__ENGINE_CACHE_VERSION__', engineVersion);
writeFileSync(serviceWorkerPath, serviceWorker);
console.log(`App version: ${shellVersion} · engine cache: ${engineVersion}`);
