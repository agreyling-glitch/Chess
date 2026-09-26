import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';
import { STANDARD,parseFen,placement,basicError } from '../web/position-editor/model.js';
test('editor opens, closes, reopens, and returns a manually edited FEN',()=>{
 const dom=new JSDOM('<!doctype html><body></body>',{url:'https://ironwood.test',runScripts:'outside-only'});
 const w=dom.window;
 const requests=[];
 Object.assign(w,{STANDARD,parseFen,placement,basicError,Worker:class {postMessage(message){requests.push(message);} terminate(){}}});
 w.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
 w.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new w.Event('close'));};
 const source=readFileSync('web/position-editor/editor.js','utf8').replace(/^import .*;\r?\n/,'');
 w.eval(source);
 assert.doesNotThrow(()=>w.ironwoodOpenPositionEditor(STANDARD));
 assert.equal(w.document.querySelectorAll('#pe-board .square').length,64);
 assert.match(w.document.querySelector('#pe-status').textContent,/Arrange pieces manually/);
 w.document.querySelector('#pe-close').click();
 assert.doesNotThrow(()=>w.ironwoodOpenPositionEditor(STANDARD));

 const palette=w.document.querySelector('#pe-palette button');
 assert.equal(palette.querySelector('img').draggable,false);
 const values=new Map();
 const transfer={setData:(k,v)=>values.set(k,v),getData:k=>values.get(k)||'',get types(){return [...values.keys(),'Files'];},files:[new w.File(['svg'], 'piece.svg',{type:'image/svg+xml'})]};
 palette.ondragstart({dataTransfer:transfer});
 const drop=new w.Event('drop',{bubbles:true,cancelable:true});
 Object.defineProperty(drop,'dataTransfer',{value:transfer});
 w.document.querySelectorAll('#pe-board .square')[32].dispatchEvent(drop);
 assert.equal(parseFen(w.document.querySelector('#pe-fen').value).board[32],'K');
 assert.equal(requests.filter(r=>r.type==='scan').length,0);
 // Remove the extra test king and continue the normal manual editing flow.
 w.document.querySelector('#pe-trash').click();
 w.document.querySelectorAll('#pe-board .square')[32].click();

 w.document.querySelectorAll('#pe-board .square')[8].click();
 w.document.querySelector('#pe-use').click();
 assert.match(w.ironwoodPollPositionEditor(),/^rnbqkbnr\/1ppppppp\//);
 assert.equal(w.ironwoodPollPositionEditor(),'');
 dom.window.close();
});
