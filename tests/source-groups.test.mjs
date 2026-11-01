import {test} from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';

test('source categories group trusted hosts without modifying original links',async()=>{
 const {window}=new JSDOM('<body></body>',{url:'https://test.local'});
 Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 const {sourceGroup,savedGameSourceLabel}=await import('../web/game-storage.js?source-group-test');
 for(const [url,label] of [
  ['https://lichess.org/abcd1234','Lichess games'],['https://lichess.org/efgh5678/black','Lichess games'],
  ['https://lichess.org/training/bDo83','Lichess puzzles'],['https://www.lichess.org/study/abcd','Lichess studies'],
  ['https://www.chess.com/game/live/123','Chess.com'],['Lichess','Lichess games'],
  ['https://lichess.org.evil.example/abc','https://lichess.org.evil.example/abc'],['Club tournament','Club tournament']
 ]) {
  const game={source:url};assert.equal(sourceGroup(url),label);assert.equal(savedGameSourceLabel(game),label);assert.equal(game.source,url);
  assert.equal(savedGameSourceLabel({venue:url}),label);
 }
 window.close();
});
