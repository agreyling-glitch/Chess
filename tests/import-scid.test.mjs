import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {JSDOM} from 'jsdom';
import {readScidFiles} from '../web/import-scid.js';
import {installImportFilePicker} from '../web/app-bootstrap.js';

const dom=new JSDOM('',{url:'http://localhost/saved-games/'});
globalThis.window=dom.window;globalThis.Window=dom.window.Window;
const app=await import('../web/pkg/battle_chess.js');
await app.default({module_or_path:readFileSync(new URL('../web/pkg/battle_chess_bg.wasm',import.meta.url))});
const {importFingerprint}=await import('../web/game-storage.js');
after(()=>dom.window.close());
const fixture=()=>['si4','sn4','sg4'].map(extension=>{
 const bytes=readFileSync(new URL(`./fixtures/scid/sample.${extension}`,import.meta.url));
 return {name:`sample.${extension}`,size:bytes.length,bytes};
});
const create=(...parts)=>new app.ScidImport(...parts);
const convert=files=>readScidFiles(files,create);

test('browser File.bytes method is not mistaken for an in-memory byte array',async()=>{
 const files=fixture().map(({name,bytes})=>new File([bytes],name));
 assert.equal(typeof files[0].bytes,'function');
 assert.match(await convert(files),/Ada/);
});

test('real browser WASM converts si4 to existing PGN records and duplicate fingerprints without changing inputs',async()=>{
 const files=fixture(),before=files.map(file=>Buffer.from(file.bytes));let progress=0;
 const text=await readScidFiles(files,create,{onProgress:()=>progress++});
 assert.equal(text.match(/\[White "Ada"\]/g).length,2);assert.ok(progress);
 const records=text.trim().split(/\n\n(?=\[Event)/);
 assert.equal(importFingerprint(records[0]),importFingerprint(records[1]));
 assert.equal(importFingerprint(records[0]),importFingerprint('[White "Ada"]\n[Black "Ben"]\n[Date "2026.10.24"]\n\n1. e4 e5 2. Nf3 Nc6 1/2-1/2'));
 const saved=JSON.parse(app.prepare_archived_pgn(records[0]));
 assert.match(saved.review_pgn,/\(2\. Nc3 Nc6\)/);assert.match(saved.review_pgn,/Develop a knight/);assert.match(saved.review_pgn,/\$1/);
 files.forEach((file,i)=>assert.deepEqual(file.bytes,before[i]));
});

test('missing, mixed, duplicate, unsupported and oversized Scid selections fail before decoding',async()=>{
 const files=fixture();
 await assert.rejects(convert(files.slice(0,1)),/together/);
 await assert.rejects(convert([files[0],{...files[1],name:'other.sn4'},files[2]]),/same database name/);
 await assert.rejects(convert([...files,files[0]]),/one .si4/);
 await assert.rejects(convert([{name:'sample.si5'}]),/Only Scid .si4/);
 await assert.rejects(convert([...files,{name:'readme.txt'}]),/Select only/);
 await assert.rejects(convert(files.map(file=>({...file,size:64*1024*1024}))),/64 MiB/);
});

test('corrupt index, mismatched names and invalid move bytes never return partial PGN',async()=>{
 for(const mutate of [
  files=>{files[0].bytes=files[0].bytes.subarray(0,200);},
  files=>{files[1].bytes=files[1].bytes.subarray(0,36);},
  files=>{files[2].bytes[2]=14;},
  files=>{files[2].bytes[files[2].bytes.length-1]=65;},
 ]){
  const files=fixture();mutate(files);await assert.rejects(convert(files));
 }
});

test('conversion cancellation releases the WASM database',async()=>{
 let cancelled=false,freed=false,decoded=0;
 await assert.rejects(readScidFiles(fixture(),()=>({game_count:()=>16,game_pgn:()=>{decoded++;return '*';},free:()=>{freed=true;}}),{
  yieldToUi:async()=>{cancelled=true;},cancelled:()=>cancelled,
 }),/cancelled/);
 assert.equal(freed,true);assert.equal(decoded,0);
});

test('Browse Scid only fills the paste field; dropped files share the same converter',async()=>{
 const target=dom.window;installImportFilePicker(target);target.ironwoodCreateScidImport=create;
 target.HTMLInputElement.prototype.click=function(){};
 let saves=0;target.ironwoodSaveGame=()=>saves++;
 target.ironwoodBrowseImportFile();const input=target.document.querySelector('input');
 assert.equal(input.multiple,true);assert.match(input.accept,/\.si4/);
 Object.defineProperty(input,'files',{value:fixture()});input.dispatchEvent(new target.Event('change'));
 let result;for(let i=0;i<100&&!result;i++){await new Promise(resolve=>setTimeout(resolve,10));result=target.ironwoodPollImportFile();}
 assert.ok(result);const {text}=JSON.parse(result);assert.match(text,/Ada/);assert.equal(saves,0);
 assert.equal(target.ironwoodImportFileStatus(),'');assert.equal(input.isConnected,false);
 assert.equal(await target.ironwoodReadScidFiles(fixture()),text);
 assert.equal(saves,0);
});

test('Browse cancellation clears progress and does not publish partial contents',async()=>{
 const target=dom.window;installImportFilePicker(target);target.ironwoodCreateScidImport=create;
 target.ironwoodBrowseImportFile();const input=target.document.querySelector('input');
 Object.defineProperty(input,'files',{value:fixture()});input.dispatchEvent(new target.Event('change'));
 target.ironwoodCancelImportFile();await new Promise(resolve=>setTimeout(resolve,40));
 assert.equal(target.ironwoodPollImportFile(),'');assert.equal(target.ironwoodImportFileStatus(),'');
});
