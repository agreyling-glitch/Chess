import test from 'node:test';
import assert from 'node:assert/strict';
import { parseStyle12, parseFicsChunk, parseFicsMoveRow, squareAt } from '../web/fics/protocol.js';

test('style 12 identifies board, players, turn, clocks, and relation', () => {
  const line = '<12> rnbqkb-r pppppppp -----n-- -------- ----P--- -------- PPPPKPPP RNBQ-BNR B -1 0 0 1 1 0 7 Newton Einstein 1 2 12 39 39 119 122 2 K/e1-e2 (0:06) Ke2 0';
  assert.deepEqual(parseStyle12(line), {
    ranks: ['rnbqkb-r', 'pppppppp', '-----n--', '--------', '----P---', '--------', 'PPPPKPPP', 'RNBQ-BNR'],
    turn: 'B', game: 7, white: 'Newton', black: 'Einstein', relation: 1,
    whiteTime: 119, blackTime: 122, moveNumber: 2,
    lastMove: 'K/e1-e2', lastSan: 'Ke2', flipped: false,
  });
  assert.equal(parseStyle12('<12> invalid'), null);
});

test('partial TCP chunks preserve a complete style 12 line', () => {
  const first = parseFicsChunk('', 'Welcome\r\n<12> rnb');
  assert.deepEqual(first.lines, ['Welcome']);
  const second = parseFicsChunk(first.rest, 'qkbnr\r\nlogin: ');
  assert.deepEqual(second.lines, ['<12> rnbqkbnr']);
  assert.equal(second.rest, 'login: ');
  assert.equal(squareAt(0, 0), 'a8');
  assert.equal(squareAt(7, 7), 'h1');
});

test('terminal control characters do not appear in server text', () => {
  const parsed = parseFicsChunk('', 'fics% \x07\n  ASCII art\tkeeps spacing\r\n');
  assert.deepEqual(parsed.lines, ['fics% ', '  ASCII art\tkeeps spacing']);
});

test('FICS move rows yield SAN without elapsed times', () => {
  assert.deepEqual(parseFicsMoveRow('  1.  e4      (0:04)     e6      (0:12)'), ['e4', 'e6']);
  assert.deepEqual(parseFicsMoveRow('  2.  Nf3     (0:03)'), ['Nf3']);
  assert.equal(parseFicsMoveRow('Move  White  Black'), null);
});
