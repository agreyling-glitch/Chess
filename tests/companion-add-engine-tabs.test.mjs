import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
test('Add engine separates automatic installation from manual registration',async()=>{
 const dom=new JSDOM('<body/>',{url:'http://localhost:8788'}),original={window:globalThis.window,document:globalThis.document,fetch:globalThis.fetch};globalThis.window=dom.window;globalThis.document=dom.window.document;
 const provider={endpoint:'http://localhost:8790',credential:'test',name:'Win11',engines:true};window.localStorage.setItem('ironwood.companion.connection.v1',JSON.stringify({version:2,connections:[provider]}));window.ironwoodCompanionFeatureEnabled=()=>true;
 window.HTMLDialogElement.prototype.showModal=function(){};window.HTMLDialogElement.prototype.close=function(){this.dispatchEvent(new window.Event('close'));};const calls=[];
 globalThis.fetch=async url=>{calls.push(String(url));return {ok:true,json:async()=>String(url).includes('engine-download-catalog')?{available:true,packages:[]}:String(url).endsWith('engine-downloads')?{jobs:[]}:{path:'engines',folder:'engines',entries:[]}};};
 try{
 const {addCompanionEngine}=await import('../web/companion-add-engine.js');const done=addCompanionEngine(provider);await new Promise(r=>setTimeout(r,0));const dialog=document.querySelector('dialog'),auto=dialog.querySelector('[data-tab=auto]'),manual=dialog.querySelector('[data-tab=manual]'),save=dialog.querySelector('[type=submit]');
 assert.equal(auto.getAttribute('aria-selected'),'true');assert.equal(dialog.querySelector('[data-manual]').hidden,true);assert.equal(save.hidden,true);assert.equal(calls.some(u=>u.includes('engine-files')),false);
 manual.click();await new Promise(r=>setTimeout(r,0));assert.equal(dialog.querySelector('[data-auto]').hidden,true);assert.equal(save.hidden,false);assert.equal(calls.filter(u=>u.includes('engine-files')).length,1);
 auto.click();manual.click();assert.equal(calls.filter(u=>u.includes('engine-files')).length,1);manual.dispatchEvent(new window.KeyboardEvent('keydown',{key:'ArrowLeft',bubbles:true}));assert.equal(auto.getAttribute('aria-selected'),'true');
 assert.equal(dialog.querySelector('[data-close]').textContent,'Close');dialog.querySelector('[data-close]').click();assert.equal(await done,false);assert.equal(calls.some(u=>u.includes('/cancel')),false);
 }finally{Object.assign(globalThis,original);dom.window.close();}
});
