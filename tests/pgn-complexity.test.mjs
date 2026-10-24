import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
globalThis.window=globalThis;
const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8');
const {pgnMoveCount}=await import(`data:text/javascript,${encodeURIComponent(source)}`);

test('move counting handles headers, comments, move numbers and nested variations in one pass',()=>{
 assert.equal(pgnMoveCount('[White "A (B)"]\n\n1.e4 {ignored (and nested {comment})} e5 (1...c5 (2.Nf3)) 2.Nf3 $1 Nc6 ; ignored e4\n*'),4);
 assert.equal(pgnMoveCount('1...e5 2.Nf3 *'),2);
 assert.equal(pgnMoveCount('[Event "Empty"]\n\n*'),0);
 for(const depth of [1,1000,50000]){
  assert.equal(pgnMoveCount('1. e4 '+ '('.repeat(depth)+'1. d4 d5'+')'.repeat(depth)+' e5 2. Nf3 Nc6 *'),4);
 }
});
