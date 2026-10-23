import {test} from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
const initial=new JSDOM('<body></body>',{url:'https://test.local'});
Object.assign(globalThis,{window:initial.window,document:initial.window.document,localStorage:initial.window.localStorage,sessionStorage:initial.window.sessionStorage});
const {createPlayerPicker}=await import('../web/game-storage.js');
test('large player picker keeps rendered rows bounded while scrolling and selecting',()=>{
 const {window}=new JSDOM('<body></body>');globalThis.document=window.document;
 const names=Array.from({length:55000},(_,i)=>`Player ${String(i).padStart(5,'0')}`);let selected;
 const picker=createPlayerPicker(names,'*',value=>selected=value);document.body.append(picker);
 const trigger=picker.querySelector('button');trigger.click();
 const list=picker.querySelector('[role=listbox]'),search=picker.querySelector('input');
 assert.ok(list.querySelectorAll('[role=option]').length<=14);assert.equal(list.querySelector('[role=option]').textContent,'All players');
 list.scrollTop=320000;list.dispatchEvent(new window.Event('scroll'));
 assert.ok(list.querySelectorAll('[role=option]').length<=14);assert.match(list.textContent,/Player 0999/);
 search.value='PLAYER 54321';search.dispatchEvent(new window.Event('input'));assert.equal(list.querySelectorAll('[role=option]').length,1);
 search.dispatchEvent(new window.KeyboardEvent('keydown',{key:'Enter',bubbles:true}));assert.equal(selected,'Player 54321');assert.equal(trigger.textContent,'Player 54321');
 trigger.click();assert.equal(list.querySelector('[aria-selected=true]').textContent,'Player 54321');
 search.value='no such player';search.dispatchEvent(new window.Event('input'));assert.equal(list.querySelectorAll('[role=option]').length,0);assert.equal(picker.querySelector('.library-player-empty').hidden,false);
 search.dispatchEvent(new window.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));assert.equal(trigger.getAttribute('aria-expanded'),'false');
 trigger.click();assert.equal(picker.querySelector('.library-player-empty').hidden,true);
});
test('player picker keyboard navigation and unknown selection preserve exact values',()=>{
 const {window}=new JSDOM('<body></body>');globalThis.document=window.document;let selected;
 const picker=createPlayerPicker(['A <B>','C'],'*',value=>selected=value);document.body.append(picker);picker.querySelector('button').click();const search=picker.querySelector('input');
 search.dispatchEvent(new window.KeyboardEvent('keydown',{key:'ArrowDown',bubbles:true}));search.dispatchEvent(new window.KeyboardEvent('keydown',{key:'Enter',bubbles:true}));assert.equal(selected,'A <B>');assert.equal(picker.querySelector('b'),null);
 picker.querySelector('button').click();search.value='unknown';search.dispatchEvent(new window.Event('input'));picker.querySelector('[role=option]').click();assert.equal(selected,'?');
});
