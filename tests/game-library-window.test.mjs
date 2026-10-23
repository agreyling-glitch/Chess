import {test} from 'node:test';import assert from 'node:assert/strict';import {JSDOM} from 'jsdom';
import {installLibraryWindow,startLibraryWindow} from '../web/saved-games-window.js';
function environment(){const {window}=new JSDOM('<p id="library-loading">Loading</p>',{url:'https://test.local/saved-games/'});return window;}
test('new library window initializes only export helpers and restores the originating view',async()=>{
 const target=environment();let reloads=0,focus=0,appLoads=0,opened;
 const state={storageSource:'browser',settings:{category:'all',player:'Alice',source:'*',date:'*',opening_choice:'',analysis:'*',sort:'recent',page:1},selected:['id'],records:[{id:'id',title:'Alice vs Bob'}]};
 target.opener={closed:false,location:{origin:'https://test.local',reload(){reloads++;}},focus(){focus++;},ironwoodLibraryPopupState:async()=>state};
 target.ironwoodOpenGameLibrary=async restore=>{opened=restore;};
 const exporter=()=>'',prepare=()=>'';
 await startLibraryWindow(target,{loadApp:async()=>({default:async()=>appLoads++,export_saved_game:exporter,prepare_archived_pgn:prepare}),loadStorage:async()=>{}});
 assert.equal(appLoads,1);assert.equal(target.ironwoodExportSavedGame,exporter);assert.deepEqual(opened,{windowState:state});assert.equal(target.document.querySelector('#library-loading').hidden,true);
 target.ironwoodLibraryWindowNavigate();assert.equal(reloads,1);assert.equal(focus,1);target.ironwoodLibraryWindowNavigate(false);assert.equal(reloads,2);assert.equal(focus,1);
});
test('private integrations stay disabled when opening a library window with the feature off',async()=>{
 const target=environment();let accessed=0;target.opener={closed:false,location:{origin:'https://test.local'},ironwoodCompanionConnection(){accessed++;return {credential:'must-not-be-read'};}};
 installLibraryWindow(target);assert.equal(accessed,0);await assert.rejects(target.ironwoodCompanionLibrary(),/disabled/);
 let changed=0;target.addEventListener('ironwood-companion-feature-changed',()=>changed++);target.dispatchEvent(new target.StorageEvent('storage',{key:'ironwood.experimental.companion'}));assert.equal(changed,1);
});
test('standalone library startup failures leave a clear error message',async()=>{
 const target=environment();await startLibraryWindow(target,{loadApp:async()=>{throw Error('offline');}});assert.match(target.document.querySelector('#library-loading').textContent,/offline/);assert.equal(target.document.querySelector('#library-loading').getAttribute('role'),'alert');
});
import {readFile} from 'node:fs/promises';
test('library selection tray is separate and the popout carries current selection without URL data',async()=>{
 const target=environment(),fen='8/8/8/8/8/8/8/K6k w - - 0 1';
 Object.assign(globalThis,{window:target,document:target.document,localStorage:target.localStorage});
 target.HTMLDialogElement.prototype.showModal=function(){this.open=true;};target.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new target.Event('close'));};target.ironwoodCompanionFeatureEnabled=()=>false;
 const record={id:'one',category:'imported',json:JSON.stringify({board:fen,live_moves:[],review_pgn:'[White "Alice"]\n[Black "Bob"]\n\n*'})};
 globalThis.indexedDB=target.indexedDB={open(){const request={};queueMicrotask(()=>{request.result={transaction(){return {objectStore(){return {openCursor(){const cursor={};let read=false;const next=()=>queueMicrotask(()=>{cursor.result=read?null:{value:record,continue:next};read=true;cursor.onsuccess();});next();return cursor;}};}};}};request.onsuccess();});return request;}};
 await import('../web/game-storage.js?popout-ui');await target.ironwoodOpenGameLibrary();
 const dialog=target.document.querySelector('#game-library-dialog'),selection=dialog.querySelector('.game-library-selection');assert.equal(selection.closest('.game-library-controls'),null);assert.equal(selection.getAttribute('aria-label'),'Game selection');
 dialog.querySelector('.game-library-select').click();let opened,focused=0;target.open=(...args)=>{opened=args;return {focus(){focused++;}};};
 dialog.querySelector('[aria-label="Open saved games in a new window"]').click();assert.equal(opened[0],'/saved-games/');assert.match(opened[2],/resizable=yes/);assert.equal(focused,1);assert.equal(dialog.open,false);
 const state=await target.ironwoodLibraryPopupState();assert.deepEqual(state.selected,['one']);assert.equal(state.storageSource,'browser');assert.equal(state.records[0].json,undefined);
 await target.ironwoodOpenGameLibrary();target.open=()=>null;dialog.querySelector('[aria-label="Open saved games in a new window"]').click();assert.equal(dialog.open,true);assert.match(dialog.querySelector('.game-library-notice').textContent,/Allow pop-ups/);dialog.close();
});
