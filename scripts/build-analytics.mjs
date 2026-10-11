import { build } from 'esbuild';
import { copyFileSync, existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
const root = join(import.meta.dirname, '..');
const result = await build({
  entryPoints: [join(root, 'scripts/analytics-source.js')],
  outfile: join(root, 'web/analytics.js'),
  bundle: true, format: 'esm', splitting: false, minify: true, target: 'es2022',
  legalComments: 'inline', metafile: true,
});
copyFileSync(join(root, 'node_modules/posthog-js/LICENSE'), join(root, 'web/licenses/posthog-js.txt'));
const packages = new Set(Object.keys(result.metafile.inputs).filter(p => p.startsWith('node_modules/')).map(p => {
  const parts = p.split('/');
  return parts.slice(0, parts[1].startsWith('@') ? 3 : 2).join('/');
}));
const notices = [];
for (const path of packages) {
  const pkg = JSON.parse(readFileSync(join(root, path, 'package.json'), 'utf8'));
  if (!['MIT', 'Apache-2.0', '(Apache-2.0 AND MIT)', 'BSD-3-Clause', 'ISC'].includes(pkg.license)) throw new Error(`Review license: ${pkg.name}: ${pkg.license}`);
  for (const file of readdirSync(join(root, path)).filter(name => /^(license|copying|notice)/i.test(name))) {
    const full = join(root, path, file);
    if (existsSync(full)) notices.push(`${pkg.name} ${pkg.version} — ${file}\n${readFileSync(full, 'utf8')}`);
  }
}
writeFileSync(join(root, 'web/licenses/analytics-dependencies.txt'), notices.join('\n\n---\n\n'));
