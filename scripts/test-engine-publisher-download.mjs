// Explicit opt-in network smoke test. Uses an isolated local Companion data dir.
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {once} from 'node:events';
import assert from 'node:assert/strict';
const binary=resolve(process.env.COMPANION_BINARY||'target/debug/ironwood-server.exe');
const root=await mkdtemp(resolve('output/engine-publisher-smoke-'));
assert.equal(spawnSync(binary,['init','--data-dir',root],{windowsHide:true}).status,0);
const cfg=JSON.parse(await readFile(join(root,'config.json'),'utf8'));cfg.listen='127.0.0.1:0';cfg.archive=null;cfg.automatic_import=false;await writeFile(join(root,'config.json'),JSON.stringify(cfg));
const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();
let child,address;
async function start(){child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});let output='';child.stdout.on('data',bytes=>output+=bytes);child.stderr.on('data',bytes=>output+=bytes);address=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('Startup timeout: '+output)),15000);child.once('error',reject);child.stdout.on('data',()=>{const found=output.match(/"address":"(127\.0\.0\.1:\d+)"/);if(found){clearTimeout(timer);resolve('http://'+found[1]);}});});}
async function request(path,body){const response=await fetch(address+'/api/v1/'+path,{method:body===undefined?'GET':'POST',headers:{Origin:'http://localhost:8788',Authorization:'Bearer '+credential,'X-Ironwood-Protocol':'1',...(body===undefined?{}:{'Content-Type':'application/json'})},body:body===undefined?undefined:JSON.stringify(body)});const value=await response.json();assert.ok(response.ok,JSON.stringify(value));return value;}
async function shutdown(){const closed=once(child,'close');const response=await fetch(address+'/api/v1/shutdown',{method:'POST',headers:{Origin:'http://localhost:8788',Authorization:'Bearer '+credential,'X-Ironwood-Protocol':'1'}});assert.equal(response.status,202);await closed;child=null;}
try{
 await start();const catalog=await request('engine-download-catalog'),p=process.env.ENGINE_PACKAGE?catalog.packages.find(p=>p.id===process.env.ENGINE_PACKAGE):catalog.packages[0];assert.ok(p);assert.match(p.url,/^https:\/\/github.com\//);
 // Seed a genuine prefix, then require the downloader to resume from the publisher.
 let original;try{original=await readFile(resolve('output/stockfish-19-official.zip'));}catch(error){if(error.code!=='ENOENT')throw error;}
 if(original&&p.id.startsWith('stockfish-')){const directory=join(root,'engines','.downloads',p.id);await mkdir(directory,{recursive:true});await writeFile(join(directory,'package.zip.part'),original.subarray(0,1024*1024));}
 await request('engine-downloads',{package:p.id});let result,lastPhase='';
 for(let i=0;i<180;i++){const state=await request('engine-downloads');result=state.jobs.find(j=>j.id===p.id);if(result.phase!==lastPhase){console.log(result.phase,result.downloaded);lastPhase=result.phase;}if(['ready','paused','cancelled'].includes(result.phase))break;await new Promise(resolve=>setTimeout(resolve,1000));}
 assert.equal(result.phase,'ready',JSON.stringify(result));const engines=await request('engines');assert.ok(JSON.stringify(engines).includes(result.engine_id));
 const settings=await request(`engine-management/${result.engine_id}`);assert.equal(settings.can_uninstall,true);
 const executable=settings.executable;
 const probe=spawn(executable,settings.args,{cwd:join(root,'engines',p.id),windowsHide:true,stdio:['pipe','pipe','pipe']});
 await new Promise((yes,no)=>{let output='';const timer=setTimeout(()=>{probe.kill();no(Error('No bestmove: '+output));},60000);probe.once('error',no);probe.stdout.on('data',b=>{output+=b;if(/bestmove [a-h][1-8][a-h][1-8]/.test(output)){clearTimeout(timer);probe.stdin.end('quit\n');yes();}});probe.stdin.write('uci\nisready\nposition startpos\ngo depth 1\n');});
 await shutdown();await start();const persisted=(await request('engine-downloads')).jobs.find(j=>j.id===p.id);assert.equal(persisted.phase,'ready');assert.equal(persisted.engine_id,result.engine_id);
 console.log(JSON.stringify({root,package:p.id,downloaded:result.downloaded,registered:result.engine_id,restart:'ready',publisher:p.url}));
}finally{if(child)await shutdown();}
