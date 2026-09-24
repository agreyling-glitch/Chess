import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { backupSummary, gameMatchesCategory, importFingerprint } = await import(`data:text/javascript,${encodeURIComponent(source)}`);

test('same game ignores comments, variations and whitespace', () => {
  const plain = '[White "Ada"]\n[Black "Ben"]\n[Date "2026.09.24"]\n\n1. e4 e5 2. Nf3 Nc6 1-0';
  const annotated = '[White "Ada"]\n[Black "Ben"]\n[Date "2026.09.24"]\n\n1.e4 {first move} e5 2.Nf3 (2. Bc4) Nc6 {[%eval 0.2]} 1-0';
  assert.equal(importFingerprint(plain), importFingerprint(annotated));
});

test('different players, dates or moves remain distinct', () => {
  const base = '[White "Ada"]\n[Black "Ben"]\n[Date "2026.09.24"]\n\n1. e4 e5';
  assert.notEqual(importFingerprint(base), importFingerprint(base.replace('Ada', 'Cara')));
  assert.notEqual(importFingerprint(base), importFingerprint(base.replace('2026.09.24', '2026.09.25')));
  assert.notEqual(importFingerprint(base), importFingerprint(base.replace('e5', 'c5')));
});

test('favorites remain a view across original game categories', () => {
  const mine = { category: 'mine', favorite: true };
  const imported = { category: 'imported', favorite: true };
  const ordinary = { category: 'mine', favorite: false };
  assert.equal(gameMatchesCategory(mine, 'favorites'), true);
  assert.equal(gameMatchesCategory(imported, 'favorites'), true);
  assert.equal(gameMatchesCategory(ordinary, 'favorites'), false);
  assert.equal(gameMatchesCategory(mine, 'mine'), true);
  assert.equal(gameMatchesCategory(imported, 'imported'), true);
});

test('backup validation summarizes restorable data', () => {
  const json = JSON.stringify({ board: 'start', player_side: 'White' });
  const summary = backupSummary({
    format: 'ironwood-backup',
    version: 1,
    preferences: { piece_set: 'Merida' },
    games: [
      { id: 'one', category: 'mine', favorite: true, json },
      { id: 'two', category: 'imported', favorite: false, json },
    ],
  });
  assert.deepEqual(summary, { games: 2, mine: 1, imported: 1, favorites: 1, hasPreferences: true });
});

test('backup validation rejects unknown formats and incomplete games', () => {
  assert.throws(() => backupSummary({ format: 'unknown', version: 1, games: [] }));
  assert.throws(() => backupSummary({ format: 'ironwood-backup', version: 1, games: [{ id: 'one' }] }));
});
