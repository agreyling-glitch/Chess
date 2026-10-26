import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {mountEngineDownloads,downloadProgress} from '../web/companion-engine-downloads.js';
const flush=()=>new Promise(resolve=>setTimeout(resolve,0));
test('publisher download controls remain private and closing the window does not cancel the Companion job',async()=>{
 const dom=new JSDOM('<main></main>'),target=dom.window,container=target.document.querySelector('main');let enabled=false,tick,state,installed=0;const calls=[];
 target.ironwoodCompanionFeatureEnabled=()=>enabled;target.setTimeout=callback=>{tick=callback;return 1;};target.clearTimeout=()=>{};
 const client={async request(path,body){calls.push([path,body]);if(path==='engine-download-catalog')return {available:true,packages:[{id:'stockfish-19-windows-x64',name:'Stockfish',version:'19',bytes:81431614,license:'GPL-3.0',source:'https://github.com/official-stockfish/Stockfish/releases/tag/sf_19',networks:'Included'}]};if(body){state={id:body.package,phase:'downloading',downloaded:1024*1024,total:81431614,speed:1024*1024,eta_seconds:77,message:'Downloading directly from the publisher'};return state;}return {jobs:state?[state]:[]};}};
 mountEngineDownloads(target,container,client);assert.equal(calls.length,0);enabled=true;
 const dispose=mountEngineDownloads(target,container,client,{onInstalled:()=>installed++});await flush();
 assert.match(container.textContent,/Stockfish 19/);const download=[...container.querySelectorAll('button')].find(b=>b.textContent==='Download and install');download.click();await flush();await tick();
 assert.deepEqual(calls.find(([path,body])=>path==='engine-downloads'&&body)[1],{package:'stockfish-19-windows-x64'});assert.match(container.textContent,/MiB\/s.*remaining/);
 state={...state,phase:'ready',engine_id:'new-engine',message:'Installed and ready to use'};await tick();await tick();assert.equal(installed,1);assert.ok([...container.querySelectorAll('button')].some(b=>b.textContent==='Installed'&&b.disabled));
 dispose();const before=calls.length;await tick();assert.equal(calls.length,before);assert.ok(!calls.some(([path])=>path.endsWith('/cancel')));dom.window.close();
});
test('download speed and ETA disappear while verifying or paused',()=>{
 const base={downloaded:1048576,total:81431614,speed:1048576,eta_seconds:77,message:'Downloading'};
 assert.match(downloadProgress({...base,phase:'downloading'}),/1 MiB\/s.*1m 17s/);
 assert.doesNotMatch(downloadProgress({...base,phase:'verifying'}),/MiB\/s|remaining/);
});
