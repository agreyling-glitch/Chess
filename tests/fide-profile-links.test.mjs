import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { JSDOM } from 'jsdom';
globalThis.window = globalThis;
globalThis.document = new JSDOM('').window.document;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { fideProfileUrl, gameDetails, playerNameElement } = await import(`data:text/javascript,${encodeURIComponent(source)}`);

test('existing saved PGN exposes independent FIDE IDs for both players', () => {
  const game = gameDetails(JSON.stringify({review_pgn: '[White "Ada"]\n[Black "Ben"]\n[WhiteFideId "1503014"]\n[BlackFideId "123456"]\n\n1. e4 *'}));
  assert.equal(game.whiteFideId, '1503014');
  assert.equal(game.blackFideId, '123456');
  assert.equal(gameDetails(JSON.stringify({review_pgn: '[White "Ada"]'})).whiteFideId, null);
});
test('only positive digit IDs produce profile links', () => {
  assert.equal(fideProfileUrl(' 1503014 '), 'https://ratings.fide.com/profile/1503014');
  for (const id of [null, '', '0', '000', '?', '-1', '12/34', 'javascript:alert(1)', '1e5']) assert.equal(fideProfileUrl(id), null);
});
test('player links open safely in new tabs and preserve plain text names', () => {
  const link = playerNameElement('<Ada>', '1503014');
  assert.equal(link.tagName, 'A');
  assert.equal(link.textContent, '<Ada>');
  assert.equal(link.href, 'https://ratings.fide.com/profile/1503014');
  assert.equal(link.target, '_blank');
  assert.equal(link.rel, 'noopener noreferrer');
  assert.equal(link.children.length, 0);
  assert.equal(playerNameElement('Ben', null).tagName, 'SPAN');
});
