import test from 'node:test';
import assert from 'node:assert/strict';
import { parseStyle12, parseFicsChunk, parseFicsMoveRow, parseFicsTimedMoveRow, parseFicsTimeControl, ficsHistoryPgn, squareAt } from '../web/fics/protocol.js';

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


test('finger rating rows preserve category records, RD, and missing ratings', async()=>{
  const {parseFicsRating} = await import('../web/fics/protocol.js');
  assert.deepEqual(parseFicsRating('Blitz 1910 75.3 28 10 2 40 1936 (01-Jun-95)'),{category:'blitz',rating:'1910',rd:75.3,wins:28,losses:10,draws:2,total:40,best:1936});
  assert.equal(parseFicsRating('Lightning ---- 350.0 0 0 0 0').rating,'----');
  assert.equal(parseFicsRating('Standard 1500P 100.2 1 2 0 3').rating,'1500P');
  assert.equal(parseFicsRating('Alice tells you: Blitz 1910 75.3 28 10 2 40'),null);
});


test('timed FICS rows preserve fractional and hour times without changing the SAN-only API',()=>{
  const row=parseFicsTimedMoveRow('19. Rxd5+ (0:16.6) Kc7 (1:02:03.250)');
  assert.deepEqual(row,{number:19,moves:[{san:'Rxd5+',elapsed:16.6,ply:36},{san:'Kc7',elapsed:3723.25,ply:37}]});
  assert.deepEqual(parseFicsMoveRow('1. e4 (0:01.250) e5 (0:02.75)'),['e4','e5']);
  assert.equal(parseFicsTimedMoveRow('Move White Black'),null);
  assert.equal(parseFicsTimedMoveRow('1. e4 (0:99) e5').moves[0].elapsed,null);
  assert.equal(parseFicsTimedMoveRow('1. e4 (bad) e5 (0:02)').moves[0].elapsed,null);
  assert.equal(parseFicsTimedMoveRow('2. O-O (0:00) *').moves.length,1);
});

test('FICS history clocks apply elapsed time and increment independently to each player',()=>{
  const control=parseFicsTimeControl('Unrated Blitz match, initial time: 2 minutes, increment: 2 seconds.');
  assert.deepEqual(control,{initial:120,increment:2});
  assert.deepEqual(parseFicsTimeControl('initial time: 90 seconds, increment: 0.5 seconds.'),{initial:90,increment:0.5});
  const moves=['1. e4 (0:01.250) e5 (0:02.75)','2. Nf3 (0:00)'].flatMap(line=>parseFicsTimedMoveRow(line).moves);
  const pgn=ficsHistoryPgn({white:'Alice',black:'Bob',timeControl:control,moves},'*');
  assert.match(pgn,/e4 \{ \[%emt 0:00:01.250\] \[%clk 0:02:00.750\] \}/);
  assert.match(pgn,/e5 \{ \[%emt 0:00:02.750\] \[%clk 0:01:59.250\] \}/);
  assert.match(pgn,/Nf3 \{ \[%emt 0:00:00.000\] \[%clk 0:02:02.750\] \}/);
  assert.match(pgn,/\[TimeControl "120\+2"\]/);
});

test('unknown or untimed controls retain elapsed times without invented clocks',()=>{
  const moves=parseFicsTimedMoveRow('1. e4 (0:03.5) e5 (0:04)').moves;
  for(const timeControl of [null,{initial:0,increment:0}]){
    const pgn=ficsHistoryPgn({white:'Alice',black:'Bob',timeControl,moves},'*');
    assert.ok(pgn.includes('[%emt 0:00:03.500]'));assert.ok(!pgn.includes('[%clk'));
  }
  assert.equal(parseFicsTimeControl('Untimed match'),null);
  assert.equal(parseFicsTimeControl('initial time: 2 minutes, increment: unknown seconds.'),null);
});

test('missing move times or rows break only the calculable clock history',()=>{
  const moves=['1. e4 e5 (0:02)','2. Nf3 (0:03) Nc6 (0:04)'].flatMap(line=>parseFicsTimedMoveRow(line).moves);
  const pgn=ficsHistoryPgn({white:'Alice',black:'Bob',timeControl:{initial:60,increment:1},moves},'*');
  assert.ok(!pgn.includes('e4 {'));assert.ok(pgn.includes('Nf3 { [%emt 0:00:03.000] }'));
  assert.ok(pgn.includes('Nc6 { [%emt 0:00:04.000] [%clk 0:00:56.000] }'));
  const gap=ficsHistoryPgn({white:'Alice',black:'Bob',timeControl:{initial:60,increment:1},moves:parseFicsTimedMoveRow('19. e4 (0:03) e5 (0:04)').moves},'*');
  assert.ok(!gap.includes('[%clk'));
});
