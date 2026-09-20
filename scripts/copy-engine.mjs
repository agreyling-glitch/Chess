import { cpSync, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const source = join(import.meta.dirname, '..', 'node_modules', 'stockfish', 'bin');
const target = join(import.meta.dirname, '..', 'web', 'engine');
const licenseSource = join(import.meta.dirname, '..', 'node_modules', 'stockfish', 'Copying.txt');
const licenseTarget = join(import.meta.dirname, '..', 'web', 'licenses');
mkdirSync(target, { recursive: true });
mkdirSync(licenseTarget, { recursive: true });
for (const file of readdirSync(source)) {
  if (file === 'stockfish-19.js' || file === 'stockfish-19.wasm') {
    cpSync(join(source, file), join(target, file));
  }
}
if (!existsSync(join(target, 'stockfish-19.js'))) throw new Error('Stockfish 19 full engine was not found');
if (!existsSync(licenseSource)) throw new Error('The Stockfish GPLv3 license was not found');
cpSync(licenseSource, join(licenseTarget, 'stockfish-gpl-3.0.txt'));
