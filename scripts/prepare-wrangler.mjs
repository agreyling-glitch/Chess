import { cpSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, relative } from 'node:path';

const root = join(import.meta.dirname, '..');
const source = join(root, 'web');
const hugoSource = join(root, '.hugo-public');
const target = join(root, '.wrangler-assets');
const changelogSource = join(root, 'site', 'content', 'changelog');

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

const newestChangelog = readdirSync(changelogSource)
  .filter(name => name.endsWith('.md') && name !== '_index.md')
  .map(name => {
    const contents = readFileSync(join(changelogSource, name), 'utf8');
    const date = contents.match(/^date:\s*(\S+)/m)?.[1];
    const slug = contents.match(/^slug:\s*["']?([^"'\r\n]+)["']?/m)?.[1];
    if (!date || !slug || Number.isNaN(Date.parse(date))) {
      throw new Error(`Invalid changelog metadata in ${name}`);
    }
    return { date, slug };
  })
  .sort((a, b) => Date.parse(b.date) - Date.parse(a.date))[0];
if (!newestChangelog) throw new Error('A changelog entry is required for the home-page update date');
const updatedIso = newestChangelog.date.slice(0, 10);
const updatedLabel = new Date(`${updatedIso}T12:00:00Z`).toLocaleDateString('en-US', {
  month: 'long', day: 'numeric', year: 'numeric', timeZone: 'UTC',
});
const homePath = join(target, 'index.html');
writeFileSync(homePath, readFileSync(homePath, 'utf8')
  .replaceAll('__LAST_UPDATED_ISO__', updatedIso)
  .replaceAll('__LAST_UPDATED_DATE__', updatedLabel)
  .replaceAll('__LATEST_CHANGELOG_URL__', `/changelog/${newestChangelog.slug}/`));

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
    else if (name !== 'app-version.json') {
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
  .replaceAll('__PACKAGE_VERSION__', packageVersion)
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
