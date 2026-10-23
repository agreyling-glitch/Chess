import {test} from 'node:test';import assert from 'node:assert/strict';import {JSDOM} from 'jsdom';
test('game actions menu supports keyboard navigation, dismissal and existing action handlers',async()=>{
 const {window}=new JSDOM('<body><dialog open></dialog><button id="outside">Outside</button></body>',{url:'https://test.local'});Object.assign(globalThis,{window,document:window.document,localStorage:window.localStorage});
 const {createGameActionMenu}=await import('../web/game-storage.js');const actions=document.createElement('div');actions.className='game-library-actions';let activated=0;
 for(const text of ['Open game','Favorite','Delete']){const item=document.createElement('button');item.textContent=text;item.onclick=()=>activated++;actions.append(item);}
 const trigger=createGameActionMenu(actions,'Alice vs Bob');document.querySelector('dialog').append(trigger,actions);
 assert.equal(actions.hidden,true);trigger.click();assert.equal(actions.hidden,false);assert.equal(document.activeElement.textContent,'Open game');
 actions.dispatchEvent(new window.KeyboardEvent('keydown',{key:'ArrowDown',bubbles:true}));assert.equal(document.activeElement.textContent,'Favorite');
 actions.dispatchEvent(new window.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));assert.equal(actions.hidden,true);assert.equal(document.activeElement,trigger);
 trigger.click();actions.querySelector('button').click();assert.equal(activated,1);assert.equal(actions.hidden,true);
 trigger.click();document.querySelector('#outside').dispatchEvent(new window.Event('pointerdown',{bubbles:true}));assert.equal(actions.hidden,true);assert.equal(trigger.getAttribute('aria-expanded'),'false');
});
