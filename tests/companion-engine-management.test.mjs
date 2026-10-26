import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {resolve,join,sep,basename} from 'node:path';
import {once} from 'node:events';
const binary=resolve(process.env.COMPANION_BINARY||`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`),token='e'.repeat(64),origin='http://localhost:8788';
test('registered engine settings validate, persist, reject conflicts and preserve other engines',{timeout:30000},async()=>{
 const root=await mkdtemp(resolve('output/engine-management-test-')),path=join(root,'config.json'),log=join(root,'engine.log'),args=[resolve('tests/fixtures/companion-fake-engine.mjs'),'normal',log];let child,address;
 const config={listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token',archive:null,automatic_import:false,engines:[{id:'managed',label:'Before',executable:process.execPath,args,options:{}},{id:'other',label:'Keep me',executable:process.execPath,args,options:{Hash:'16'}}]};
 await writeFile(join(root,'owner-token'),token);await writeFile(path,JSON.stringify(config));
 async function start(){child=spawn(binary,['--config',path],{windowsHide:true,stdio:['ignore','pipe','pipe']});let output='';child.stdout.on('data',b=>output+=b);child.stderr.on('data',b=>output+=b);address=await new Promise((yes,no)=>{const timer=setTimeout(()=>no(Error(output)),10000);child.once('error',no);child.stdout.on('data',()=>{const m=output.match(/"address":"(127\.0\.0\.1:\d+)"/);if(m){clearTimeout(timer);yes('http://'+m[1]);}});});}
 async function request(route,body,credential=token){const response=await fetch(address+'/api/v1/'+route,{method:body===undefined?'GET':'POST',headers:{Origin:origin,Authorization:'Bearer '+credential,'X-Ironwood-Protocol':'1','Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});const text=await response.text();let value;try{value=JSON.parse(text);}catch{value={message:text};}return {status:response.status,value};}
 async function stop(){const closed=once(child,'close');await fetch(address+'/api/v1/shutdown',{method:'POST',headers:{Origin:origin,Authorization:'Bearer '+token,'X-Ironwood-Protocol':'1'}});await closed;child=null;}
 try{
  await start();assert.equal((await request('engine-management/managed',undefined,'wrong')).status,401);
  const info=(await request('engine-management/managed')).value;assert.equal(info.label,'Before');assert.equal(info.can_uninstall,false);assert.equal(info.options,undefined);
  const body={token:info.token,label:'Edited engine',args,options:{Threads:'2',Hash:'64','Playing Style':'Very Aggressive',Ponder:'false',EvalFile:'custom.nnue'}};
  const saved=await request('engine-management/managed',body);assert.equal(saved.status,200,JSON.stringify(saved));
  let disk=JSON.parse(await readFile(path,'utf8'));assert.deepEqual(disk.engines[0].options,body.options);assert.equal(disk.engines[0].label,'Edited engine');assert.deepEqual(disk.engines[1],config.engines[1]);assert.equal(disk.automatic_import,false);
  assert.equal((await request('engine-management/managed',body)).status,409);
  let current=(await request('engine-management/managed')).value;
  for(const options of [{Threads:'99'},{Threads:'not a number'},{'Playing Style':'Unknown'},{Ponder:'maybe'},{'Clear Hash':''},{EvalFile:'line\nquit'},{Unknown:'1'}]){
   const before=await readFile(path,'utf8'),rejected=await request('engine-management/managed',{...body,token:current.token,options});assert.equal(rejected.status,400,JSON.stringify(rejected));assert.equal(await readFile(path,'utf8'),before);
  }
  assert.ok(!(await readFile(log,'utf8')).includes('setoption name Clear Hash'));
  const invalid=await request('engine-management/managed',{...body,token:current.token,executable:'C:/other.exe'});assert.ok([400,422].includes(invalid.status));
  await stop();await start();current=(await request('engine-management/managed')).value;assert.equal(current.label,'Edited engine');assert.equal(current.values.Threads,'2');assert.equal(current.values.EvalFile,'custom.nnue');
  assert.equal((await request('engine-management/managed/remove',{token:current.token,delete_files:true})).status,409);
  disk=JSON.parse(await readFile(path,'utf8'));disk.engines[1].label='External edit';await writeFile(path,JSON.stringify(disk));
  assert.equal((await request('engine-management/managed',{...body,token:current.token})).status,409);assert.equal(JSON.parse(await readFile(path,'utf8')).engines[1].label,'External edit');
  current=(await request('engine-management/managed')).value;const removed=await request('engine-management/managed/remove',{token:current.token,delete_files:false});assert.equal(removed.status,200,JSON.stringify(removed));assert.equal((await request('engines')).value.length,1);assert.equal(JSON.parse(await readFile(path,'utf8')).engines[0].label,'External edit');await stop();await start();assert.equal((await request('engines')).value.length,1);
 }finally{if(child)await stop();const absolute=resolve(root);assert.ok(absolute.startsWith(resolve('output')+sep)&&basename(absolute).startsWith('engine-management-test-'));await rm(root,{recursive:true,force:true});}
});
