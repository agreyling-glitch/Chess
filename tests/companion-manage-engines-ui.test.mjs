import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {manageCompanionEngines} from '../web/companion-manage-engines.js';
const flush=()=>new Promise(resolve=>setTimeout(resolve,0));
test('Manage engines edits launch settings and preserves existing engine defaults',async()=>{
 const dom=new JSDOM('<body></body>'),target=dom.window;target.ironwoodCompanionFeatureEnabled=()=>true;target.HTMLDialogElement.prototype.showModal=function(){this.open=true;};target.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new target.Event('close'));};
 const provider={endpoint:'http://local:8790',name:'Local Companion'},calls=[];let saved=0;
 const info={id:'test',label:'Test engine',executable:'C:/engines/test.exe',args:[],token:'before',values:{EvalFile:'original.nnue'},options:[{name:'Threads',kind:'spin',default:'1',min:1,max:4},{name:'Ponder',kind:'check',default:'false'},{name:'Playing Style',kind:'combo',default:'Normal',choices:['Normal','Very Aggressive']},{name:'EvalFile',kind:'string',default:''},{name:'Clear Hash',kind:'button',default:''}]};
 const client={async request(path,body){calls.push([path,body]);if(path==='engines')return [{id:'test',label:'Test engine'}];if(body)return {token:'after',message:'Saved.'};return info;}};
 const done=manageCompanionEngines([provider],{target,identity:provider.endpoint+'/test',clientFactory:()=>client,onSaved:()=>saved++});await flush();await flush();
 const dialog=target.document.querySelector('dialog'),form=dialog.querySelector('form');assert.equal(form.elements.executable.readOnly,true);assert.equal(dialog.querySelector('select'),null);assert.match(dialog.querySelector('h2').textContent,/Test engine/);assert.equal(calls[0][0],'engine-management/test');assert.equal(dialog.querySelector('[data-options]'),null);assert.equal(dialog.querySelector('[data-reset]'),null);
 form.elements.label.value='Edited';form.elements.args.value='["--model","network.pt"]';
 form.dispatchEvent(new target.Event('submit',{cancelable:true}));await flush();
 const body=calls.find(([,body])=>body)[1];assert.deepEqual(body,{token:'before',label:'Edited',args:['--model','network.pt'],options:{EvalFile:'original.nnue'}});assert.equal(saved,1);assert.match(dialog.textContent,/Reselect the engine/);
 dialog.querySelector('[data-close]').click();assert.equal(await done,true);dom.window.close();
});
test('manager stays behind the feature flag and keeps failed/conflicting edits unsaved',async()=>{
 const dom=new JSDOM('<body></body>'),target=dom.window;let enabled=false,saves=0;target.ironwoodCompanionFeatureEnabled=()=>enabled;target.HTMLDialogElement.prototype.showModal=function(){};target.HTMLDialogElement.prototype.close=function(){this.dispatchEvent(new target.Event('close'));};const provider={endpoint:'http://local'};
 assert.equal(await manageCompanionEngines([provider],{target}),false);assert.equal(target.document.querySelector('dialog'),null);enabled=true;
 const done=manageCompanionEngines([provider],{target,identity:provider.endpoint+'/e',clientFactory:()=>({async request(path,body){if(path==='engines')return [{id:'e',label:'Engine'}];if(body){saves++;throw Error('Configuration changed. Reload before saving.');}return {id:'e',label:'Engine',executable:'test',args:[],values:{},options:[],token:'old'};}})});await flush();await flush();const dialog=target.document.querySelector('dialog');dialog.querySelector('form').dispatchEvent(new target.Event('submit',{cancelable:true}));await flush();assert.equal(saves,1);assert.match(dialog.textContent,/Configuration changed/);assert.equal(dialog.querySelector('[data-close]').disabled,false);dialog.querySelector('[data-close]').click();assert.equal(await done,false);dom.window.close();
});
