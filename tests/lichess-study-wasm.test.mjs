import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import test from 'node:test';
import init, {prepare_archived_pgn, export_saved_game} from '../web/pkg/battle_chess.js';

await init({module_or_path: await readFile(new URL('../web/pkg/battle_chess_bg.wasm', import.meta.url))});
const headers = '[Event "Study: Chapter"]\n[StudyName "Study"]\n[ChapterName "Chapter"]\n[ChapterURL "https://lichess.org/study/abcd1234/WXYZ5678"]\n[Result "*"]\n\n';

test('browser WASM keeps study source and supported annotations through saved records', () => {
  const pgn = headers + '{Introduction} 1. e4 $1 {King pawn [%eval 0.25] [%csl Ge4] [%cal Ge2e4]} (1. d4 {Side line}) *';
  const record = JSON.parse(prepare_archived_pgn(pgn));
  assert.equal(record.review_pgn, pgn);
  assert.deepEqual(record.move_notes, ['Introduction', 'King pawn']);
  assert.deepEqual(record.move_annotations[1], [1]);
  assert.equal(record.game_analysis[1].eval_cp, 25);
  assert.equal(record.board_marks[1].length, 2);
  const preview = JSON.parse(export_saved_game(JSON.stringify(record), 'preview'));
  assert.deepEqual(preview.moves, ['e4']);
  const exported = JSON.parse(export_saved_game(JSON.stringify(record), 'json'));
  assert.equal(exported.game.source_pgn, pgn);
});

test('browser WASM retains a position-only study chapter and its introduction', () => {
  const pgn = headers + '{Start here} *';
  const record = JSON.parse(prepare_archived_pgn(pgn));
  assert.equal(record.review_index, 0);
  assert.deepEqual(record.move_notes, ['Start here']);
  assert.equal(record.review_pgn, pgn);
  assert.equal(JSON.parse(export_saved_game(JSON.stringify(record), 'preview')).positions.length, 1);
});


test('browser WASM preserves multiline drawing directives in exported study PGN', () => {
  const pgn=headers+'1. e4 {Author explanation.\n\n[%csl Ge4][%cal Ge2e4]} e5 *';
  const record=JSON.parse(prepare_archived_pgn(pgn));
  assert.equal(record.move_notes[1],'Author explanation.');
  assert.equal(record.board_marks[1].length,2);
  record.study={name:'Study',chapter:0,node:0,line_end:0,chapters:[{source:pgn,name:'Chapter',url:'https://lichess.org/study/abcd1234/WXYZ5678',nodes:[]}]};
  const exported=export_saved_game(JSON.stringify(record),'annotated');
  const imported=JSON.parse(prepare_archived_pgn(exported));
  assert.equal(imported.move_notes[1],'Author explanation.');
  assert.equal(imported.board_marks[1].length,2);
  assert.deepEqual(JSON.parse(export_saved_game(JSON.stringify(imported),'preview')).moves,['e4','e5']);
});
