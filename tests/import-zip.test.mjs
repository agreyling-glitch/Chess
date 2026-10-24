import test from 'node:test';import assert from 'node:assert/strict';import {zip} from './helpers/zip-fixture.mjs';import {readPgnZip} from '../web/import-zip.js';
test('ZIP import combines stored and deflated PGNs in folders and ignores unrelated files',async()=>{
 const first='[White "Alice"]\n[Black "Bob"]\n\n1. e4 e5 *',second='[White "Chloé"]\n[Black "Dan"]\n\n1. d4 d5 *';
 const archive=zip([{name:'one.pgn',text:first,method:0},{name:'folder/two.PGN',text:second,flags:8},{name:'readme.txt',text:'ignored'},{name:'__MACOSX/._one.pgn',text:'ignored'}]);
 assert.equal(await readPgnZip(archive),first+'\n\n'+second);
});
test('ZIP import rejects invalid files, empty archives and archives without PGN files',async()=>{
 for(const bytes of [Buffer.from('not a zip'),zip([{name:'game.pgn',text:'*'}]).subarray(0,-1)])await assert.rejects(readPgnZip(bytes),/damaged/);
 await assert.rejects(readPgnZip(zip([])),/does not contain any PGN/);await assert.rejects(readPgnZip(zip([{name:'analysis.json',text:'{}'}])),/does not contain any PGN/);
});
test('ZIP import bounds expanded data and entry count before extraction',async()=>{
 await assert.rejects(readPgnZip(zip([{name:'one.pgn',declaredSize:65*1024*1024}])),/64 MiB/);
 await assert.rejects(readPgnZip(zip([{name:'one.pgn',text:'*'},{name:'two.pgn',text:'*'}]),{maxEntries:1}),/too many files/);
 await assert.rejects(readPgnZip(zip([{name:'one.pgn',text:'123'},{name:'two.pgn',text:'456'}]),{maxBytes:7}),/64 MiB/);
 await assert.rejects(readPgnZip(zip([{name:'bomb.pgn',text:'A'.repeat(100000),declaredSize:1}])),/expands beyond/);
});
test('ZIP import rejects encrypted, unsupported and corrupt PGNs without returning partial results',async()=>{
 await assert.rejects(readPgnZip(zip([{name:'locked.pgn',flags:1}])),/Password-protected/);
 await assert.rejects(readPgnZip(zip([{name:'method.pgn',method:99}])),/compression method/);
 await assert.rejects(readPgnZip(zip([{name:'okay.pgn',text:'*'},{name:'corrupt.pgn',text:'bad',checksum:0}])),/integrity check/);
 const split=zip([{name:'game.pgn'}]);split.writeUInt16LE(1,split.length-18);await assert.rejects(readPgnZip(split),/Multi-part/);
 const mismatch=zip([{name:'game.pgn'}]);mismatch[30]^=1;await assert.rejects(readPgnZip(mismatch),/damaged/);
});
