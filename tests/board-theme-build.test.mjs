import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { buildBoardThemes, boardThemeFiles, boardThemeSources } from '../scripts/build-board-themes.mjs';

test('3D packs preserve source bytes, use stable content identities, and reject damaged assets', () => {
  const temporaryRoot = resolve(tmpdir());
  const directory = mkdtempSync(join(temporaryRoot, 'ironwood-board-themes-'));
  const sourceRoot = join(directory, 'source'), outputRoot = join(directory, 'output');
  try {
    let total = 0;
    for (const [theme, files] of Object.entries(boardThemeFiles)) {
      const source = boardThemeSources[theme];
      mkdirSync(join(sourceRoot, source), { recursive: true });
      for (const file of files) {
        const data = readFileSync(new URL(`../assets/3d/${source}/${file}`, import.meta.url));
        writeFileSync(join(sourceRoot, source, file), data);
        total += data.length;
      }
    }
    const first = buildBoardThemes({ sourceRoot, outputRoot });
    assert.equal(first.bytes, total);
    for (const [theme, files] of Object.entries(boardThemeFiles)) {
      assert.deepEqual(Object.keys(first.boardThemes[theme]), files);
      for (const file of files) {
        const data = readFileSync(join(sourceRoot, boardThemeSources[theme], file));
        const hash = createHash('sha256').update(data).digest('hex');
        const url = first.boardThemes[theme][file];
        assert.equal(url, `/board-themes/${theme}/${hash}/${file}`);
        assert.deepEqual(readFileSync(join(outputRoot, url.slice('/board-themes/'.length))), data);
      }
    }
    assert.deepEqual(buildBoardThemes({ sourceRoot, outputRoot }), first);
    const changedPath = join(sourceRoot, boardThemeSources.wood, 'pieces_white_diff.jpg');
    writeFileSync(changedPath, readFileSync(join(sourceRoot, boardThemeSources.wood, 'pieces_black_diff.jpg')));
    const changed = buildBoardThemes({ sourceRoot, outputRoot });
    assert.notEqual(changed.boardThemes.wood['pieces_white_diff.jpg'], first.boardThemes.wood['pieces_white_diff.jpg']);
    for (const [theme, files] of Object.entries(boardThemeFiles)) for (const file of files) {
      if (theme === 'wood' && file === 'pieces_white_diff.jpg') continue;
      assert.equal(changed.boardThemes[theme][file], first.boardThemes[theme][file]);
    }
    assert.match(readFileSync(join(outputRoot, 'manifest.js'), 'utf8'), /export const boardThemes =/);
    const boardPath = join(sourceRoot, boardThemeSources.wood, 'board.glb');
    const validBoard = readFileSync(boardPath);
    const invalidBoard = Buffer.from(validBoard);
    invalidBoard.writeUInt32LE(validBoard.length + 8, 8);
    writeFileSync(boardPath, invalidBoard);
    assert.throws(() => buildBoardThemes({ sourceRoot, outputRoot }), /Invalid GLB/);
    writeFileSync(boardPath, validBoard);
    writeFileSync(changedPath, 'not a JPEG');
    assert.throws(() => buildBoardThemes({ sourceRoot, outputRoot }), /Invalid JPEG/);
  } finally {
    assert.ok(resolve(directory).startsWith(temporaryRoot + sep));
    rmSync(directory, { recursive: true, force: true });
  }
});
