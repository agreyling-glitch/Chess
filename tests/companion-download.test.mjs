import test from 'node:test';
import assert from 'node:assert/strict';
import {webcrypto,createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {downloadPrivateCompanion} from '../web/companion-download.js';
const token='a'.repeat(64);
const root='https://companion-releases.ironwoodchess.com/private-preview/windows-x64/';
function fixture(change={},verify=true) {
 const archive=new TextEncoder().encode('package fixture');
 const release={format:2,product:'Ironwood Companion',platform:'windows-x64',signing_mode:'private-preview-key',protocol:1,data_schema:1,version:'0.1.1',expires:'2099-01-01T00:00:00Z',url:root+'0.1.1/ironwood-companion-windows-x64.zip',sha256:createHash('sha256').update(archive).digest('hex'),...change};
 const payload=JSON.stringify(release),calls=[];
 const fetcher=async(url,options)=>{calls.push({url,options});return new Response(url.endsWith('manifest.json')?JSON.stringify({payload:Buffer.from(payload).toString('base64'),signature:'AA=='}):archive);};
 const cryptoApi={subtle:{importKey:async()=>({}),verify:async(_algorithm,_key,_signature,data)=>{assert.equal(new TextDecoder().decode(data),'Ironwood Companion private-preview release v1\n'+payload);return verify;},digest:(...args)=>webcrypto.subtle.digest(...args)}};
 return {calls,options:{fetcher,cryptoApi,allowed:()=>true}};
}
test('private download is gated, authenticated, pinned and verified before returning a package',async()=>{
 const {calls,options}=fixture();
 const result=await downloadPrivateCompanion(token,options);
 assert.equal(result.version,'0.1.1');assert.equal(calls.length,2);
 for(const {options} of calls){assert.equal(options.headers.Authorization,'Bearer '+token);assert.equal(options.redirect,'error');assert.equal(options.credentials,'omit');assert.equal(options.cache,'no-store');}
 const gated=fixture();await assert.rejects(downloadPrivateCompanion(token,{...gated.options,allowed:()=>false}),/Enable/);assert.equal(gated.calls.length,0);
});
test('bad signatures, expired feeds, wrong targets and changed packages are never returned',async()=>{
 for(const [change,verify,error] of [[{},false,/signature/],[{expires:'2000-01-01'},true,/expired/],[{url:'https://other.example/package.zip'},true,/address/],[{sha256:'0'.repeat(64)},true,/integrity/]]){
  const {calls,options}=fixture(change,verify);await assert.rejects(downloadPrivateCompanion(token,options),error);
  assert.equal(calls.length,change.sha256?2:1);
 }
});
test('unauthorized download explains release access without exposing credentials',async()=>{
 await assert.rejects(downloadPrivateCompanion(token,{allowed:()=>true,fetcher:async()=>new Response('',{status:401})}),/Release access was rejected/);
});
test('the real pinned public key rejects an unsigned manifest', async()=>{
 const {calls,options}=fixture();
 await assert.rejects(downloadPrivateCompanion(token,{...options,cryptoApi:webcrypto}),/signature/);
 assert.equal(calls.length,1);
});
test('download entry point remains in the Companion tab and uses no persistent token storage',()=>{
 const source=readFileSync(new URL('../web/companion-download.js',import.meta.url),'utf8');
 assert.doesNotMatch(source,/localStorage|sessionStorage/);
 const rust=readFileSync(new URL('../src/app.rs',import.meta.url),'utf8');
 const start=rust.indexOf('if companion_tab {',rust.indexOf('about_version_diagnostics'));
 assert.ok(start>=0 && rust.indexOf('Download Companion for Windows',start)>start);
});
