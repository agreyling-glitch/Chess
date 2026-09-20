import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    env: process.env,
    stdio: 'inherit',
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function npmRun(script) {
  if (process.platform === 'win32') {
    run(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', `npm run ${script}`]);
  } else {
    run('npm', ['run', script]);
  }
}

const requiredArtifacts = [
  join('web', 'pkg', 'battle_chess.js'),
  join('web', 'pkg', 'battle_chess_bg.wasm'),
  join('web', 'licenses', 'stockfish-gpl-3.0.txt'),
];

const missingArtifacts = requiredArtifacts.filter((path) => !existsSync(path));
if (missingArtifacts.length > 0) {
  console.error(`Missing committed deployment artifacts:\n${missingArtifacts.join('\n')}`);
  console.error('Run `npm run build`, then commit the updated web/pkg and license files.');
  process.exit(1);
}

npmRun('prepare:wrangler');
