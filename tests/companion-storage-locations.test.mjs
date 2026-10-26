import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
test('named archive clients use their own credentials without changing the default archive',async()=>{
 const dom=new JSDOM('<body/>',{url:'http://localhost:8788'}),original={window:globalThis.window,fetch:globalThis.fetch};globalThis.window=dom.window;window.ironwoodCompanionFeatureEnabled=()=>true;
 const first={endpoint:'http://localhost:8790',name:'Win11',credential:'windows-owner'},second={endpoint:'http://192.168.0.2:8790',name:'NAS4',credential:'nas-owner'};
 window.localStorage.setItem('ironwood.companion.connection.v1',JSON.stringify({version:2,connections:[first,second],archiveId:first.endpoint}));const calls=[];
 globalThis.fetch=async(url,init)=>{calls.push([url,init.headers.Authorization]);return {ok:true,json:async()=>url.endsWith('capabilities')?{product:'Ironwood Companion',protocol_version:'1',services:{archive:true}}:{games:2000}};};
 try{
  await import('../web/companion-library.js');const {companionConnection,connectCompanion}=await import('../web/companion-connection.js');
  const selected=await window.ironwoodCompanionLibrary({endpoint:second.endpoint,interactive:false});assert.equal(selected.endpoint,second.endpoint);assert.equal((await selected.stats()).games,2000);
  assert.ok(calls.every(([url,auth])=>url.startsWith(second.endpoint)&&auth==='Bearer nas-owner'));assert.equal(companionConnection().endpoint,first.endpoint);
  const defaultStore=await window.ironwoodCompanionLibrary({interactive:false});assert.equal(defaultStore.endpoint,first.endpoint);
  await assert.rejects(window.ironwoodCompanionLibrary({endpoint:'http://unknown:8790',interactive:false}),/Connect to Companion/);
  await connectCompanion(second.endpoint,'changed-owner',{name:'NAS4',archive:false});assert.throws(()=>selected.check(),/connection changed/);assert.equal(companionConnection().endpoint,first.endpoint);
 }finally{Object.assign(globalThis,original);dom.window.close();}
});
