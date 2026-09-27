import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('board image uses the supplied position, orientation, artwork, and PNG format', async () => {
  const draws = [], labels = [];
  const context = { fillRect(){}, save(){}, restore(){}, drawImage(image,...args){draws.push([image.src,...args]);}, fillText(...args){labels.push(args);} };
  const canvas = { getContext:()=>context, toBlob(callback,type){assert.equal(type,'image/png');callback(new Blob(['png'],{type}));} };
  globalThis.document = {createElement:()=>canvas};
  globalThis.window = {};
  globalThis.Image = class {set src(value){this.url=value;queueMicrotask(()=>this.onload());}get src(){return this.url;}};
  const source = await readFile(new URL('../web/board-clipboard.js',import.meta.url),'utf8');
  const {boardImage} = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  const fen = '7k/8/8/8/8/8/8/K7 w - - 0 1';
  const blob = await boardImage(fen,{pieceSet:'RoyalRascals',frame:true,coordinates:true});
  assert.equal(blob.type,'image/png'); assert.equal(canvas.width,1024); assert.equal(canvas.height,1024);
  assert.equal(draws.length,2); assert.match(draws[0][0],/royal-rascals\/bK.svg$/);
  assert.ok(draws[0][1]>draws[1][1]); assert.ok(draws[0][2]<draws[1][2]);
  assert.equal(labels[0][0],'a');
  draws.length=0; labels.length=0;
  await boardImage(fen,{pieceSet:'Merida',flipped:true,frame:true,coordinates:true});
  assert.ok(draws[0][1]<draws[1][1]); assert.ok(draws[0][2]>draws[1][2]);
  assert.equal(labels[0][0],'h');
  await assert.rejects(boardImage('invalid'),/Invalid board/);
});
