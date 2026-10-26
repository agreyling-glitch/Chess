import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import worker from '../deploy/companion-releases/worker.js';
const token = 'private-fixture-token-'.repeat(3);
const origin = 'https://companion-releases.ironwoodchess.com';
const path = '/private-preview/windows-x64/manifest.json';
function setup() {
  const reads = [];
  const env = {RELEASE_READ_TOKEN:token,RELEASES:{
    get:async key => { reads.push(key); return {body:'signed-envelope',size:15,httpEtag:'"hash"'}; },
    head:async key => { reads.push(key); return {size:15,httpEtag:'"hash"'}; }
  }};
  const request = (suffix=path,options={}) => new Request(origin+suffix,{headers:{Authorization:`Bearer ${token}`},...options});
  return {env,reads,request};
}
test('release channel fails closed and never reads private objects without authentication',async () => {
  const {env,reads,request} = setup();
  assert.equal((await worker.fetch(new Request(origin+path),env)).status,401);
  assert.equal((await worker.fetch(request(path,{headers:{Authorization:'Bearer wrong'}}),env)).status,401);
  assert.equal((await worker.fetch(request(),{...env,RELEASE_READ_TOKEN:''})).status,503);
  assert.deepEqual(reads,[]);
});
test('authenticated release requests are read-only, bounded to release objects and never publicly cached',async () => {
  const {env,reads,request} = setup();
  for (const invalid of ['/','/private-preview/','/owner-token','/private-preview/windows-x64/manifest.json?token=bad','/private-preview/windows-x64/%6danifest.json']) {
    assert.ok([400,404].includes((await worker.fetch(request(invalid),env)).status));
  }
  assert.equal((await worker.fetch(request(path,{method:'POST'}),env)).status,405);
  assert.equal((await worker.fetch(new Request('http://companion-releases.ironwoodchess.com'+path,{headers:{Authorization:`Bearer ${token}`}}),env)).status,400);
  assert.deepEqual(reads,[]);
  const response = await worker.fetch(request(),env);
  assert.equal(await response.text(),'signed-envelope');
  assert.equal(response.headers.get('cache-control'),'private, no-store');
  assert.match(response.headers.get('x-robots-tag'),/noindex/);
  assert.equal(response.headers.get('access-control-allow-origin'),null);
  const head = await worker.fetch(request(path,{method:'HEAD'}),env);
  assert.equal(await head.text(),'');
  assert.equal(head.headers.get('content-length'),'15');
  const zip = '/private-preview/windows-x64/0.2.0/ironwood-companion-windows-x64.zip';
  assert.equal((await worker.fetch(request(zip),env)).headers.get('content-type'),'application/zip');
  assert.deepEqual(reads,[path.slice(1),path.slice(1),zip.slice(1)]);
  env.RELEASES.get = async () => null;
  assert.equal((await worker.fetch(request(),env)).status,404);
  env.RELEASES.get = async () => { throw Error('sensitive backend details'); };
  const failure = await worker.fetch(request(),env);
  assert.equal(failure.status,503);
  assert.doesNotMatch(await failure.text(),/sensitive/);
});
test('release Worker has no workers.dev or preview bypass and uses a dedicated bucket',() => {
  const config = JSON.parse(readFileSync(new URL('../deploy/companion-releases/wrangler.jsonc',import.meta.url),'utf8'));
  assert.equal(config.workers_dev,false);
  assert.equal(config.preview_urls,false);
  assert.equal(config.r2_buckets[0].bucket_name,'ironwood-companion-private');
  assert.equal(config.observability.enabled,false);
});

test('browser downloads allow exact Ironwood origins but still require release authentication', async () => {
  const {env,reads} = setup();
  const site = 'https://ironwoodchess.com';
  const preflight = originValue => new Request(origin+path,{method:'OPTIONS',headers:{Origin:originValue,'Access-Control-Request-Method':'GET','Access-Control-Request-Headers':'authorization'}});
  const allowed = await worker.fetch(preflight(site),env);
  assert.equal(allowed.status,204);
  assert.equal(allowed.headers.get('access-control-allow-origin'),site);
  assert.equal((await worker.fetch(preflight('https://other.example'),env)).status,403);
  const unauth = await worker.fetch(new Request(origin+path,{headers:{Origin:site}}),env);
  assert.equal(unauth.status,401);
  assert.equal(unauth.headers.get('access-control-allow-origin'),site);
  assert.deepEqual(reads,[]);
  const response = await worker.fetch(new Request(origin+path,{headers:{Origin:site,Authorization:`Bearer ${token}`}}),env);
  assert.equal(response.status,200);
  assert.equal(response.headers.get('access-control-allow-origin'),site);
  assert.equal((await worker.fetch(new Request(origin+path,{headers:{Origin:'https://ironwoodchess.com.evil.example',Authorization:`Bearer ${token}`}}),env)).status,403);
});


test('public Windows releases are opt-in and never expose the private channel',async()=>{
 const {env,reads}=setup();const url=origin+'/public/windows-x64/manifest.json';
 assert.equal((await worker.fetch(new Request(url),env)).status,503);assert.deepEqual(reads,[]);
 const publicEnv={...env,PUBLIC_WINDOWS_RELEASES:'1',RELEASE_READ_TOKEN:''};
 assert.equal((await worker.fetch(new Request(url),publicEnv)).status,200);
 assert.equal((await worker.fetch(new Request(origin+'/public/windows-x64/0.1.4/ironwood-companion-windows-x64.zip'),publicEnv)).status,200);
 assert.equal((await worker.fetch(new Request(origin+path),{...env,PUBLIC_WINDOWS_RELEASES:'1'})).status,401);
 for(const suffix of ['/public/linux-x64/manifest.json','/public/windows-x64/owner-token','/public/windows-x64/0.1.4/source/private-key.json']){
  assert.notEqual((await worker.fetch(new Request(origin+suffix),publicEnv)).status,200);
 }
 assert.equal((await worker.fetch(new Request(url,{method:'POST'}),publicEnv)).status,405);
 assert.deepEqual(reads,['public/windows-x64/manifest.json','public/windows-x64/0.1.4/ironwood-companion-windows-x64.zip']);
});
