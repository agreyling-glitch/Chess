import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';
import {JSDOM} from 'jsdom';
import {planSevenZip,extractSevenZip,openSevenZip} from '../web/import-sevenz.js';
const manifest=files=>({entries:files.map(([name,size=10])=>({name,size,directory:false})),total:files.reduce((sum,[,size=10])=>sum+size,0)});
test('7z planning accepts PGNs and complete Scid groups, rejects ambiguity and unsafe/oversized archives',()=>{
 assert.equal(planSevenZip(manifest([['a.pgn'],['folder/B.PGN']])).kind,'pgn');
 for(const version of [4,5])assert.equal(planSevenZip(manifest(['si','sn','sg'].map(ext=>[`folder/base.${ext}${version}`]))).kind,`scid${version}`);
 for(const files of [[['../a.pgn']],[['C:/a.pgn']],[['a.pgn'],['A.PGN']],[['base.si5']],[['base.si5'],['base.sn4'],['base.sg5']],[['a.pgn',65*1024*1024]]])assert.throws(()=>planSevenZip(manifest(files)));
 assert.throws(()=>planSevenZip(manifest([['readme.txt']])));
});
test('real browser WASM extracts 7z PGN and Scid fixtures with CRC checks and bounded reads',async()=>{
 const wasm=await import('../web/pkg/battle_chess.js');await wasm.default({module_or_path:await readFile(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
 for(const [archive,kind] of [['sample-pgn.7z','pgn'],['sample-si4.7z','scid4'],['sample-si5.7z','scid5']]){
  const input=await readFile(new URL(`./fixtures/scid/${archive}`,import.meta.url));let largest=0;
  const read=(offset,length)=>{largest=Math.max(largest,length);return input.subarray(offset,offset+length);};
  const plan=planSevenZip(JSON.parse(wasm.ironwoodSevenZipList(input.length,read)));assert.equal(plan.kind,kind);
  const parts=new Map();let done=0;
  wasm.ironwoodSevenZipExtract(input.length,read,(name,offset,bytes)=>{if(!parts.has(name))parts.set(name,Buffer.alloc(plan.files.find(f=>f.name===name)?.size||bytes.length));Buffer.from(bytes).copy(parts.get(name),offset);},value=>done=value);
  for(const entry of plan.files){const base=entry.name.split('/').at(-1);const expected=await readFile(new URL(`./fixtures/scid/${kind==='pgn'?'archive.pgn':base}`,import.meta.url));assert.deepEqual(parts.get(entry.name),expected);}
  assert.ok(largest<=1024*1024);assert.ok(done>0);
  const damaged=Buffer.from(input);damaged[Math.floor(damaged.length/2)]^=255;
  assert.throws(()=>{wasm.ironwoodSevenZipList(damaged.length,(o,n)=>damaged.subarray(o,o+n));wasm.ironwoodSevenZipExtract(damaged.length,(o,n)=>damaged.subarray(o,o+n),()=>{},()=>{});});
 }
});
test('worker failures, insufficient quota and cancellation terminate extraction and remove only temporary files',async()=>{
 let removed=0,terminated=0;const parent={getDirectoryHandle:async()=>({}),removeEntry:async()=>{removed++;}};
 const target={crypto:{randomUUID:()=> 'test-session'},navigator:{storage:{getDirectory:async()=>({getDirectoryHandle:async()=>parent}),estimate:async()=>({quota:1,usage:0})}}};
 const workerFactory=()=>({terminate(){terminated++;},postMessage(message){if(message.type==='open')queueMicrotask(()=>this.onmessage({data:{type:'manifest',manifest:manifest([['a.pgn']])}}));}});
 await assert.rejects(extractSevenZip({size:100},{target,workerFactory}),/Not enough/);assert.equal(removed,1);assert.equal(terminated,1);
 const controller=new AbortController();controller.abort();await assert.rejects(extractSevenZip({size:100},{target,workerFactory,signal:controller.signal}),/cancelled/);assert.equal(removed,2);
});
test('an extraction failure remains visible until the user closes it',async()=>{
 const dom=new JSDOM('<body></body>'),target=dom.window;
 target.HTMLDialogElement.prototype.showModal=function(){this.open=true;};target.HTMLDialogElement.prototype.close=function(){this.open=false;};
 await assert.rejects(openSevenZip(target,{size:100}),/storage is unavailable/);
 const dialog=target.document.querySelector('#sevenz-import-progress');assert.ok(dialog?.open);assert.match(dialog.textContent,/Could not open.*storage is unavailable/s);assert.equal(dialog.querySelector('progress').hidden,true);assert.equal(dialog.querySelector('button').textContent,'Close');dialog.querySelector('button').click();assert.equal(dialog.isConnected,false);dom.window.close();
});
test('7z decoder seeks beyond 4 GiB without loading or allocating the source whole',async()=>{
 const wasm=await import('../web/pkg/battle_chess.js');await wasm.default({module_or_path:await readFile(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
 const input=await readFile(new URL('./fixtures/scid/sample-si5.7z',import.meta.url)),oldHeader=32+Number(input.readBigUInt64LE(12)),headerSize=Number(input.readBigUInt64LE(20));
 const header=input.subarray(oldHeader,oldHeader+headerSize),headerAt=5*1024*1024*1024,first=Buffer.from(input.subarray(0,32));first.writeBigUInt64LE(BigInt(headerAt-32),12);
 let crc=0xffffffff;for(const value of first.subarray(12,32)){crc^=value;for(let b=0;b<8;b++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}first.writeUInt32LE((crc^0xffffffff)>>>0,8);
 let maxOffset=0,maxRead=0;const read=(offset,length)=>{maxOffset=Math.max(maxOffset,offset);maxRead=Math.max(maxRead,length);const out=Buffer.alloc(length);for(const [start,bytes] of [[0,first],[32,input.subarray(32)],[headerAt,header]]){const a=Math.max(offset,start),b=Math.min(offset+length,start+bytes.length);if(b>a)bytes.copy(out,a-offset,a-start,b-start);}return out;};
 const total=headerAt+header.length,plan=planSevenZip(JSON.parse(wasm.ironwoodSevenZipList(total,read)));assert.equal(plan.kind,'scid5');
 let written=0;wasm.ironwoodSevenZipExtract(total,read,(_name,_offset,bytes)=>written+=bytes.length,()=>{});
 assert.equal(written,plan.bytes);assert.ok(maxOffset>4*1024*1024*1024);assert.ok(maxRead<=1024*1024);
});
test('actual worker stages real archives and routes PGN/Scid through review, with Scid 5 files retained until close',async()=>{
 const wasm=await import('../web/pkg/battle_chess.js');await wasm.default({module_or_path:await readFile(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
 const code=(await readFile(new URL('../web/import-sevenz-worker.js',import.meta.url),'utf8')).replace(/^import[^\n]*\n/,'');
 for(const name of ['sample-pgn.7z','sample-si4.7z','sample-si5.7z']){
  const dom=new JSDOM('<body></body>'),target=dom.window;let removed=0,scidDialog;
  target.HTMLDialogElement.prototype.showModal=function(){this.open=true;};target.HTMLDialogElement.prototype.close=function(){this.open=false;};
  target.File=File;
  const buffers=new Map();
  const directory={async getFileHandle(name){return {async createSyncAccessHandle(){buffers.set(name,Buffer.alloc(0));return {write(bytes,{at}){const old=buffers.get(name),next=Buffer.alloc(Math.max(old.length,at+bytes.length));old.copy(next);Buffer.from(bytes).copy(next,at);buffers.set(name,next);return bytes.length;},truncate(size){buffers.set(name,buffers.get(name).subarray(0,size));},flush(){},close(){}};},async getFile(){return new File([buffers.get(name)],name);}};}};
  const parent={getDirectoryHandle:async()=>directory,removeEntry:async()=>{removed++;}};
  Object.defineProperty(target.navigator,'storage',{value:{getDirectory:async()=>({getDirectoryHandle:async()=>parent}),estimate:async()=>({quota:1024*1024*1024,usage:0})}});
  target.Worker=class{
   constructor(){const worker=this,self={postMessage:data=>queueMicrotask(()=>worker.onmessage({data}))};this.context=vm.createContext({self,fetch:async(url,options)=>{if(url==='/app-version.json'){assert.equal(options.cache,'no-store');return {ok:true,json:async()=>({package_version:'new-7z-decoder'})};}assert.equal(url,'/pkg/battle_chess_bg.wasm?v=new-7z-decoder');assert.equal(options.cache,'reload');return {ok:true};},init:async()=>{},ironwoodSevenZipList:wasm.ironwoodSevenZipList,ironwoodSevenZipExtract:wasm.ironwoodSevenZipExtract,FileReaderSync:class{readAsArrayBuffer(slice){return Uint8Array.from(slice.bytes).buffer;}},performance,Uint8Array,Map});vm.runInContext(code,this.context);}
   postMessage(data){void this.context.self.onmessage({data});}terminate(){}
  };
  target.ironwoodReadScidFiles=async files=>{assert.equal(files.length,3);assert.ok((await files[0].arrayBuffer()).byteLength>0);return 'Scid review text';};
  target.ironwoodOpenScid5Files=async files=>{assert.equal(files.length,3);assert.equal(removed,0);scidDialog=target.document.createElement('dialog');return scidDialog;};
  const bytes=await readFile(new URL(`./fixtures/scid/${name}`,import.meta.url));
  const result=await openSevenZip(target,{size:bytes.length,slice:(a,b)=>({bytes:bytes.subarray(a,b)})});
  if(name.includes('si5')){assert.equal(result,null);assert.equal(removed,0);scidDialog.dispatchEvent(new target.Event('close'));await new Promise(r=>setTimeout(r,0));}
  else if(name.includes('si4'))assert.equal(result,'Scid review text');else assert.match(result,/Seven Zip/);
  assert.equal(removed,1);assert.equal(target.document.querySelector('#sevenz-import-progress'),null);dom.window.close();
 }
});
