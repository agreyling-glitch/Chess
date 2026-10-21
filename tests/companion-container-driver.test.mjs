import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {once} from 'node:events';
const binary=process.env.COMPANION_BINARY||resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);
test('container API driver exercises real server lifecycle without claiming Docker validation',{timeout:30000},async()=>{
 const root=await mkdtemp(resolve('tmp/companion-container-driver-'));let child;
 async function start(){
  child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});let data='';
  return new Promise((yes,no)=>{const timer=setTimeout(()=>no(Error('Startup timeout')),5000);child.once('error',no);child.stdout.on('data',part=>{data+=part;const m=data.match(/"address":"(127\.0\.0\.1:\d+)"/);if(m){clearTimeout(timer);yes('http://'+m[1]);}});});
 }
 async function stop(address){const closed=once(child,'close');await fetch(address+'/api/v1/shutdown',{method:'POST',headers:{Origin:'http://localhost:8788',Authorization:`Bearer ${(await readFile(join(root,'owner-token'),'utf8')).trim()}`,'X-Ironwood-Protocol':'1'}});assert.equal((await closed)[0],0);}
 try{
  assert.equal(spawnSync(binary,['init','--data-dir',root],{windowsHide:true}).status,0);
  const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';config.tablebase_directories=[root];config.engines=[{id:'fixture',label:'Original test fixture',executable:process.execPath,args:[resolve('tests/fixtures/companion-fake-engine.mjs')]}];await writeFile(join(root,'config.json'),JSON.stringify(config));
  for(const mode of ['exercise','persisted']){
   const address=await start();
   const result=spawnSync(process.execPath,[resolve('scripts/validate-companion-container-api.mjs'),mode],{cwd:root,env:{...process.env,COMPANION_ADDRESS:address,COMPANION_TEST_DATA:root},encoding:'utf8',windowsHide:true,timeout:15000});
   // The server is another OS process, so a synchronous driver does not block it.
   assert.equal(result.status,0,result.stdout+result.stderr);
   assert.equal(JSON.parse(await readFile(join(root,`tmp/container-${mode}-report.json`),'utf8')).status,'passed');
   await stop(address);
  }
 }finally{if(child&&child.exitCode===null&&child.signalCode===null){const closed=once(child,'close');child.kill();await closed;}await rm(root,{recursive:true,force:true});}
});
