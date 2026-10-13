import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { buildPieceThemes, pieceNames, themeNames } from '../scripts/build-piece-themes.mjs';

test('theme output preserves every source PNG and immutable paths change only with content', () => {
  const temporaryRoot = resolve(tmpdir());
  const directory = mkdtempSync(join(temporaryRoot, 'ironwood-piece-themes-'));
  const sourceRoot = join(directory, 'source'), outputRoot = join(directory, 'output');
  try {
    let originalBytes = 0;
    for (const theme of themeNames) {
      mkdirSync(join(sourceRoot, theme), { recursive: true });
      for (const piece of pieceNames) {
        const bytes = readFileSync(new URL(`../web/pieces/${theme}/${piece}.png`, import.meta.url));
        writeFileSync(join(sourceRoot, theme, `${piece}.png`), bytes);
        originalBytes += bytes.length;
      }
    }
    const first = buildPieceThemes({ sourceRoot, outputRoot });
    assert.equal(first.bytes, originalBytes);
    assert.deepEqual(Object.keys(first.pieceThemes), themeNames);
    for (const theme of themeNames) {
      assert.deepEqual(Object.keys(first.pieceThemes[theme]), pieceNames);
      for (const piece of pieceNames) {
        const source = readFileSync(join(sourceRoot, theme, `${piece}.png`));
        const path = first.pieceThemes[theme][piece];
        const hash = createHash('sha256').update(source).digest('hex');
        assert.equal(path, `/piece-themes/${theme}/${hash}/${piece}.png`);
        assert.deepEqual(readFileSync(join(outputRoot, path.slice('/piece-themes/'.length))), source);
      }
    }
    assert.deepEqual(buildPieceThemes({ sourceRoot, outputRoot }), first);
    // Replace one sprite with another valid PNG. Unchanged artwork must retain
    // its existing offline/cache identity across builds.
    writeFileSync(join(sourceRoot, themeNames[0], 'wK.png'), readFileSync(join(sourceRoot, themeNames[0], 'wQ.png')));
    const changed = buildPieceThemes({ sourceRoot, outputRoot });
    assert.notEqual(changed.pieceThemes[themeNames[0]].wK, first.pieceThemes[themeNames[0]].wK);
    for (const theme of themeNames) for (const piece of pieceNames) {
      if (theme === themeNames[0] && piece === 'wK') continue;
      assert.equal(changed.pieceThemes[theme][piece], first.pieceThemes[theme][piece]);
    }
    assert.match(readFileSync(join(outputRoot, 'manifest.js'), 'utf8'), /export const pieceThemes =/);
    writeFileSync(join(sourceRoot, themeNames[0], 'wK.png'), 'missing image');
    assert.throws(() => buildPieceThemes({ sourceRoot, outputRoot }), /Invalid PNG/);
  } finally {
    assert.ok(resolve(directory).startsWith(temporaryRoot + sep));
    rmSync(directory, { recursive: true, force: true });
  }
});
