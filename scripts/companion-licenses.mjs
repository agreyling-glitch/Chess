import { spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';

const packages = new Map();
for (const target of ['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu']) {
  const result = spawnSync('cargo', ['metadata', '--locked', '--format-version', '1', '--filter-platform', target], { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(result.stderr || 'cargo metadata failed');
  const metadata = JSON.parse(result.stdout);
  const root = metadata.packages.find(p => p.name === 'ironwood-server');
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  const reachable = new Set();
  function walk(id) {
    if (reachable.has(id)) return;
    reachable.add(id);
    for (const dependency of nodes.get(id).dependencies) walk(dependency);
  }
  walk(root.id);
  for (const pkg of metadata.packages.filter(pkg => reachable.has(pkg.id))) {
    if (!pkg.license) throw new Error(`Missing license metadata: ${pkg.name}@${pkg.version}`);
    const key = `${pkg.name}@${pkg.version}`;
    if (!packages.has(key)) packages.set(key, { name: pkg.name, version: pkg.version, license: pkg.license, repository: pkg.repository, targets: [] });
    packages.get(key).targets.push(target);
  }
}
const inventory = [...packages.values()].sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
writeFileSync('server/docs/dependency-licenses.json', JSON.stringify(inventory, null, 2) + '\n');
console.log(`Recorded ${inventory.length} dependency versions. License metadata is an inventory, not a substitute for release notices.`);
