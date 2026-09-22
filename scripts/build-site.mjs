import { rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const source = join(root, 'site');
const destination = join(root, '.hugo-public');

rmSync(destination, { recursive: true, force: true });
const result = spawnSync('hugo', [
  '--source', source,
  '--destination', destination,
  '--baseURL', 'https://ironwoodchess.com/',
  '--minify',
], {
  cwd: root,
  env: process.env,
  stdio: 'inherit',
});

if (result.error) {
  console.error('Hugo is required to build the blog and changelog.');
  throw result.error;
}
if (result.status !== 0) process.exit(result.status ?? 1);
