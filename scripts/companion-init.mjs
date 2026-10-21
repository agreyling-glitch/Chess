// Compatibility helper: provisioning now belongs to the native executable.
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
const directory = resolve(process.argv[2] || 'tmp/companion');
const executable = resolve(`target/debug/ironwood-server${process.platform === 'win32' ? '.exe' : ''}`);
const result = spawnSync(executable, ['init', '--data-dir', directory, ...(process.argv.includes('--docker') ? ['--docker'] : [])], { stdio: 'inherit', windowsHide: true });
if (result.error) console.error('Build ironwood-server first, then use its native init command.');
process.exit(result.status ?? 1);
