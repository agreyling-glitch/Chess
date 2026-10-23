import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';import vm from 'node:vm';
const source=readFileSync(new URL('../src/wrangler-worker.js',import.meta.url),'utf8').replace('import { connect } from "cloudflare:sockets";','').replace('export default','globalThis.worker =');
function fixture(reply=()=>new Response('engine bytes')){
 const calls=[],local=[];const context=vm.createContext({URL,Request,Response,Headers,fetch:async request=>{calls.push(request);return reply(request);},connect:()=>assert.fail('engine requests must not create FICS connections')});vm.runInContext(source,context);
 const env={ENGINE_ORIGIN:'https://engine.example',ASSETS:{fetch:async request=>{local.push(request);return new Response('local asset');}}};
 return {calls,local,env,run:(path,options)=>context.worker.fetch(new Request('https://ironwoodchess.com'+path,options),env)};
}
test('engine proxy rejects authority escapes and unrecognized paths without any upstream request',async()=>{
 const f=fixture();for(const path of ['/engine//evil.example/payload.js','/engine///evil.example/payload.wasm','/engine/\\evil.example/payload.js','/engine/%2f%2fevil.example/payload.js','/engine/%5cevil.example/payload.js','/engine/https://evil.example/payload.js','/engine/stockfish-19.wasm/extra','/engine/unknown.js']){
  const response=await f.run(path);assert.equal(response.status,404,path);assert.equal(response.headers.get('cache-control'),'no-store');
 }assert.equal(f.calls.length,0);assert.equal(f.local.length,0);
 // URL normalization can move dot segments out of /engine; they stay in local assets.
 await f.run('/engine/%2e%2e/other.js');assert.equal(f.calls.length,0);
});
test('real engine assets use only the configured origin and do not forward credentials',async()=>{
 const f=fixture(()=>new Response('wasm',{status:206,headers:{'Content-Range':'bytes 0-3/100','ETag':'"engine"','Service-Worker-Allowed':'/','Set-Cookie':'secret=1','Content-Type':'text/html'}}));
 const response=await f.run('/engine/stockfish-19.wasm?v=anything',{headers:{Range:'bytes=0-3',Authorization:'Bearer private',Cookie:'private=1','If-Range':'"engine"'}});
 assert.equal(f.calls[0].url,'https://engine.example/stockfish-19.wasm');assert.equal(f.calls[0].redirect,'manual');assert.equal(f.calls[0].headers.get('range'),'bytes=0-3');assert.equal(f.calls[0].headers.get('if-range'),'"engine"');assert.equal(f.calls[0].headers.get('authorization'),null);assert.equal(f.calls[0].headers.get('cookie'),null);
 assert.equal(response.status,206);assert.equal(response.headers.get('content-type'),'application/wasm');assert.equal(response.headers.get('content-range'),'bytes 0-3/100');assert.equal(response.headers.get('service-worker-allowed'),null);assert.equal(response.headers.get('set-cookie'),null);assert.equal(response.headers.get('x-content-type-options'),'nosniff');assert.equal(await response.text(),'wasm');
});
test('engine redirects and upstream error pages are rejected without caching or reflecting headers',async()=>{
 for(const status of [301,302,303,307,308,404,500]){
  const f=fixture(()=>new Response('untrusted markup',{status,headers:{Location:'https://evil.example/script.js','Service-Worker-Allowed':'/','Cache-Control':'public,max-age=999999'}}));
  const response=await f.run('/engine/stockfish-19.js');assert.equal(f.calls.length,1);assert.equal(f.calls[0].redirect,'manual');assert.equal(response.status,status===404?404:502);assert.equal(response.headers.get('cache-control'),'no-store');assert.equal(response.headers.get('location'),null);assert.equal(response.headers.get('service-worker-allowed'),null);assert.doesNotMatch(await response.text(),/untrusted markup/);
 }
});
test('engine proxy preserves HEAD, conditional responses, local development and packaged wrapper',async()=>{
 const f=fixture(()=>new Response(null,{status:304,headers:{ETag:'"v1"'}}));let response=await f.run('/engine/stockfish-19.js',{headers:{'If-None-Match':'"v1"'}});assert.equal(response.status,304);assert.equal(f.calls[0].headers.get('if-none-match'),'"v1"');assert.equal(await response.text(),'');
 f.env.ENGINE_ORIGIN='http://127.0.0.1:8789';response=await f.run('/engine/stockfish-19.wasm',{method:'HEAD'});assert.equal(f.calls[1].url,'http://127.0.0.1:8789/stockfish-19.wasm');assert.equal(f.calls[1].method,'HEAD');assert.equal(await response.text(),'');
 await f.run('/engine/stockfish-19-ironwood-pool-v1.js');assert.equal(f.local.length,1);assert.equal(f.calls.length,2);
});
test('invalid engine configuration, unsupported methods and network failures fail closed',async()=>{
 const f=fixture();for(const origin of [undefined,'not a URL','ftp://engine.example','https://user:pass@engine.example','https://engine.example/other/']){f.env.ENGINE_ORIGIN=origin;assert.equal((await f.run('/engine/stockfish-19.js')).status,503);}assert.equal(f.calls.length,0);
 f.env.ENGINE_ORIGIN='https://engine.example';assert.equal((await f.run('/engine/stockfish-19.js',{method:'POST',body:'private'})).status,405);assert.equal(f.calls.length,0);
 const failed=fixture(()=>{throw Error('private network details');});const response=await failed.run('/engine/stockfish-19.wasm');assert.equal(response.status,502);assert.equal(response.headers.get('cache-control'),'no-store');assert.doesNotMatch(await response.text(),/private network details/);
});
