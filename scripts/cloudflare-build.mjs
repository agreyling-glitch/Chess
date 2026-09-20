import { spawnSync } from 'node:child_process';
import { delimiter, join } from 'node:path';
import { homedir } from 'node:os';

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    env: process.env,
    stdio: 'inherit',
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function available(command) {
  const result = spawnSync(command, ['--version'], { env: process.env, stdio: 'ignore' });
  return !result.error && result.status === 0;
}

const cargoHome = process.env.CARGO_HOME || join(homedir(), '.cargo');
process.env.CARGO_HOME = cargoHome;
process.env.RUSTUP_HOME ||= join(homedir(), '.rustup');
process.env.PATH = `${join(cargoHome, 'bin')}${delimiter}${process.env.PATH || ''}`;

if (!available('rustup')) {
  run('sh', [
    '-c',
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable",
  ]);
}

run('rustup', ['target', 'add', 'wasm32-unknown-unknown']);

if (!available('wasm-pack')) {
  run('sh', [
    '-c',
    "curl --proto '=https' --tlsv1.2 -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh",
  ]);
}

function npmRun(script) {
  if (process.platform === 'win32') {
    run(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', `npm run ${script}`]);
  } else {
    run('npm', ['run', script]);
  }
}

npmRun('build');
npmRun('prepare:wrangler');
