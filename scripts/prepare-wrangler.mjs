import { cpSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = join(import.meta.dirname, '..');
const source = join(root, 'web');
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
