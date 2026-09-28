import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { build } from 'esbuild';

const root = join(import.meta.dirname, '..');
const source = join(root, 'prototypes', '3d');
const output = join(root, 'web', '3d');
rmSync(output, { recursive: true, force: true });
mkdirSync(join(output, 'models'), { recursive: true });
mkdirSync(join(output, 'licenses'), { recursive: true });
cpSync(join(source, 'index.html'), join(output, 'index.html'));
cpSync(join(source, 'style.css'), join(output, 'style.css'));
cpSync(join(root, 'node_modules', 'three', 'LICENSE'), join(output, 'licenses', 'three-mit.txt'));
cpSync(join(root, 'assets', '3d', 'polyhaven-chess-set'), join(output, 'models'), {
  recursive: true,
  filter: path => !path.endsWith('README.md'),
});
await build({
  entryPoints: [join(source, 'main.js')],
  outfile: join(output, 'main.js'),
  bundle: true,
  format: 'esm',
  minify: true,
  target: 'es2022',
});
console.log(`3D board built at ${output}`);
