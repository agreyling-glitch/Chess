import {test} from 'node:test';import assert from 'node:assert/strict';import {JSDOM} from 'jsdom';import {rememberPopup} from '../web/library-window-state.js';
test('separate Saved Games window remembers resizing and movement and stops tracking on close',()=>{
 const {window}=new JSDOM('',{url:'https://test.local'});let tick,cleared=false;window.setInterval=fn=>{tick=fn;return 42;};window.clearInterval=id=>{cleared=id===42;};
 Object.assign(window,{innerWidth:920,innerHeight:650,screenX:160,screenY:90});rememberPopup(window);tick();
 assert.deepEqual(JSON.parse(window.localStorage.getItem('ironwood.saved-games.popup')),{width:920,height:650,left:160,top:90});
 window.innerWidth=760;window.dispatchEvent(new window.Event('resize'));assert.equal(JSON.parse(window.localStorage.getItem('ironwood.saved-games.popup')).width,760);
 window.screenX=240;window.dispatchEvent(new window.Event('pagehide'));assert.equal(JSON.parse(window.localStorage.getItem('ironwood.saved-games.popup')).left,240);assert.equal(cleared,true);
});
