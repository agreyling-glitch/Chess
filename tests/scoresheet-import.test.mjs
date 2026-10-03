import assert from 'node:assert/strict';
import test from 'node:test';
import {readFileSync} from 'node:fs';
const source = readFileSync(new URL('../web/scoresheet-pgn.js', import.meta.url), 'utf8');
const {normalizeMoves, scoresheetPgn} = await import(`data:text/javascript,${encodeURIComponent(source)}`);
test('Notation cleanup preserves unknown text and normalizes castling and move numbers', () => {
  assert.equal(normalizeMoves('1) e4 e5\n2 Nf3 Nc6 3. 0–0 ???'), '1. e4 e5 2. Nf3 Nc6 3. O-O ???');
});
test('PGN metadata is escaped, defaults are complete, and results agree', () => {
  const pgn = scoresheetPgn('1. e4 e5', {White:'A "B"\nC',Result:'1-0'});
  assert.match(pgn, /\[White "A \\"B\\" C"\]/);
  assert.match(pgn, /1\. e4 e5 1-0$/);
  assert.throws(() => scoresheetPgn('1. e4 e5 0-1', {Result:'1-0'}), /differs/);
});
test('missing rows, unknown words, and embedded comments cannot disappear during validation', () => {
  for (const text of ['1. e4 e5 3. Nf3 Nc6','1. e4 ???','1. e4 {missing} e5','1. e4 * e5','1. e4 2. Nf3']) assert.throws(() => scoresheetPgn(text));
  assert.doesNotThrow(() => scoresheetPgn('1. e4 e5 2. Nf3 Nc6'));
});
