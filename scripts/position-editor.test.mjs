import {test} from 'node:test';
import assert from 'node:assert/strict';
import {parseFen,placement,basicError,STANDARD} from '../web/position-editor/model.js';
test('FEN round trip retains position and special game state',()=>{for(const fen of [STANDARD,'4k3/8/8/3pP3/8/8/8/4K3 w - d6 7 25','4k3/8/8/8/8/8/8/5KR1 w G - 0 1']){const p=parseFen(fen);assert.equal(`${placement(p.board)} ${p.turn} ${p.rights} ${p.ep} ${p.half} ${p.full}`,fen);}});
test('invalid input is rejected without replacing editor state',()=>{for(const fen of ['8/8 w - - 0 1',STANDARD.replace(' w ',' z '),STANDARD.replace(' 0 1',' 0 0')])assert.throws(()=>parseFen(fen));});
test('empty boards and pawns on back ranks cannot be submitted',()=>{assert.match(basicError(Array(64).fill('')),/king/);const p=parseFen(STANDARD);p.board[0]='P';assert.match(basicError(p.board),/Pawns/);assert.equal(basicError(parseFen(STANDARD).board),'');});
