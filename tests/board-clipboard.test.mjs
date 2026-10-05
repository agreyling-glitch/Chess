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

test('annotated board draws selected markings with flipped coordinates and projected 3D outlines', async () => {
  const lines = [], squares = [], fills = [];
  const context = { fillRect(){}, save(){}, restore(){}, drawImage(){}, fillText(){},
    beginPath(){}, moveTo(...p){lines.push(['move', ...p]);}, lineTo(...p){lines.push(['line', ...p]);},
    closePath(){}, stroke(){}, strokeRect(...p){squares.push(p);}, fill(){fills.push(this.fillStyle);} };
  const canvas = { getContext:()=>context, toBlob(callback){callback(new Blob(['png'],{type:'image/png'}));} };
  globalThis.document = {createElement:()=>canvas}; globalThis.window = {};
  const source = await readFile(new URL('../web/board-clipboard.js',import.meta.url),'utf8');
  const {boardImage, paintDrawings} = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  const fen = '8/8/8/8/8/8/8/8 w - - 0 1';
  await boardImage(fen, {boardMarks:[{color:'G',from:'e2',to:'e4'},{color:'R',from:'f6',to:'f6'}]});
  assert.deepEqual(lines[0], ['move',576,832]);
  assert.equal(squares.length,1); assert.deepEqual(fills,['#4ac474']);
  lines.length=0; squares.length=0;
  await boardImage(fen,{flipped:true,boardMarks:[{color:'Y',from:'e2',to:'e4'}]});
  assert.deepEqual(lines[0],['move',448,192]);
  lines.length=0;
  paintDrawings(context,[{color:'R',from:[100,100],to:[100,100],outline:[[80,90],[110,80],[120,110],[90,120]]}],100);
  assert.deepEqual(lines[0],['move',80,90]);
  assert.equal(lines.length,4);
  lines.length=0;
  paintDrawings(context,[{color:'G',from:null,to:[1,2]},{color:'X',from:[1,1],to:[2,2]}]);
  assert.equal(lines.length,0);
});

test('styled arrows use dashed shafts and mirrored curves with solid arrowheads', async () => {
  const source = await readFile(new URL('../web/board-clipboard.js', import.meta.url), 'utf8');
  const { paintDrawings } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  const dashes = [], curves = [];
  let currentDash = [];
  const context = { save(){}, restore(){}, beginPath(){}, moveTo(){}, lineTo(){}, stroke(){}, closePath(){},
    fill(){ assert.deepEqual(currentDash, []); },
    setLineDash(value){ currentDash = value; dashes.push(value); },
    quadraticCurveTo(...args){ curves.push(args); }
  };
  for (const style of ['dashed', 'curve-left', 'curve-right']) {
    paintDrawings(context, [{ color:'B', from:[0,0], to:[200,0], style }], 100);
  }
  assert.deepEqual(dashes, [[20,13],[]]);
  assert.equal(curves.length, 2);
  assert.equal(curves[0][1], 70);
  assert.equal(curves[1][1], -70);
});

test('circle and dotted shape copies preserve geometry and round dot spacing', async () => {
  const source = await readFile(new URL('../web/board-clipboard.js', import.meta.url), 'utf8');
  const { paintDrawings } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  const arcs = [], dashes = [], squares = [], lines = [];
  const context = { save(){}, restore(){}, beginPath(){}, stroke(){}, closePath(){},
    moveTo(...p){ lines.push(p); }, lineTo(...p){ lines.push(p); },
    arc(...p){ arcs.push(p); }, setLineDash(p){ dashes.push(p); }, strokeRect(...p){ squares.push(p); }
  };
  for (const style of ['circle','dotted-circle','dotted-square']) {
    paintDrawings(context, [{color:'R',from:[100,100],to:[100,100],style}], 100);
  }
  assert.equal(arcs.length, 2); assert.equal(arcs[0][2], 43);
  assert.equal(squares.length, 1);
  assert.equal(context.lineCap, 'round');
  assert.deepEqual(dashes, [[0, 6*2.8], [], [0,6*2.8], []]);
  paintDrawings(context, [{color:'R',from:[100,100],to:[100,100],style:'circle',outline:[[1,1],[2,1],[3,2],[2,3],[1,2]]}], 100);
  assert.equal(lines.length, 5); assert.equal(arcs.length, 2);
});
