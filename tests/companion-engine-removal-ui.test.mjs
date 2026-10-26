import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {manageCompanionEngines} from '../web/companion-manage-engines.js';
test('engine removal requires confirmation and preserves manual engine files',async()=>{
 const dom=new JSDOM('<body/>'),target=dom.window;target.ironwoodCompanionFeatureEnabled=()=>true;target.HTMLDialogElement.prototype.showModal=function(){};target.HTMLDialogElement.prototype.close=function(){this.dispatchEvent(new target.Event('close'));};
 const calls=[];let removed;
 const done=manageCompanionEngines([{endpoint:'http://local'}],{target,identity:'http://local/e',onSaved:value=>removed=value,clientFactory:()=>({async request(path,body){calls.push([path,body]);return body?{removed:true}:{id:'e',label:'Engine',executable:'test',args:[],values:{},token:'token',can_uninstall:false};}})});
 await new Promise(resolve=>setTimeout(resolve,0));const dialog=target.document.querySelector('dialog');
 assert.equal(dialog.querySelector('[data-delete-label]').hidden,true);assert.equal(dialog.querySelector('[data-remove-confirm]').hidden,true);
 dialog.querySelector('[data-remove]').click();assert.equal(calls.length,1);assert.equal(dialog.querySelector('[data-remove-confirm]').hidden,false);
 dialog.querySelector('[data-confirm-remove]').click();assert.equal(await done,true);assert.deepEqual(calls[1],['engine-management/e/remove',{token:'token',delete_files:false}]);assert.equal(removed.removed,true);dom.window.close();
});
