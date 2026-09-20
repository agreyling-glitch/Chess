import { cpSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const source = join(root, 'web');
const target = join(root, '.wrangler-assets');
mkdirSync(target, { recursive: true });
for (const entry of ['index.html', 'open-source-notices.html', 'landing.css', 'styles.css', 'engine-worker.js', 'robots.txt', '_headers', 'play', 'pkg', 'licenses']) {
  cpSync(join(source, entry), join(target, entry), { recursive: true });
}
