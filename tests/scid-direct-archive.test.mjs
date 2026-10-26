import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,mkdtemp,rm} from 'node:fs/promises';
import {readFileSync} from 'node:fs';
import {spawn,spawnSync} from 'node:child_process';
import {once} from 'node:events';
import {resolve,join,sep} from 'node:path';
import {JSDOM} from 'jsdom';
import {ArchiveLibrary} from '../web/companion-library.js';
import {CompanionArchiveClient} from '../web/companion-archive-client.js';

test('direct Scid PGN imports round-trip annotations/FEN, isolate invalid games and deduplicate a lost reply',{timeout:30000},async()=>{
 const parent=resolve('output'),binary=resolve(process.env.IRONWOOD_TEST_SERVER||`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);
 await mkdir(parent,{recursive:true});const root=await mkdtemp(join(parent,'direct-pgn-test-'));let child;
 const dom=new JSDOM('',{url:'http://localhost/saved-games/'});globalThis.window=dom.window;globalThis.Window=dom.window.Window;
 window.ironwoodCompanionFeatureEnabled=()=>true;
 const wasm=await import('../web/pkg/battle_chess.js');await wasm.default({module_or_path:readFileSync(resolve('web/pkg/battle_chess_bg.wasm'))});
 try{
  assert.equal(spawnSync(binary,['init','--data-dir',root],{windowsHide:true,stdio:'ignore'}).status,0);
  const config=JSON.parse(await readFile(join(root,'config.json'),'utf8'));config.listen='127.0.0.1:0';config.allowed_origins=['http://localhost:8788'];await writeFile(join(root,'config.json'),JSON.stringify(config));
  child=spawn(binary,['--data-dir',root],{windowsHide:true,stdio:['ignore','pipe','pipe']});child.stderr.resume();
  const endpoint=await new Promise((yes,no)=>{let log='';const timer=setTimeout(()=>no(Error('Startup timeout')),10000);child.once('error',error=>{clearTimeout(timer);no(error);});child.stdout.on('data',part=>{log+=part;const match=log.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes('http://'+match[1]);}});});
  const credential=(await readFile(join(root,'owner-token'),'utf8')).trim();let dropReply=false;
  const client=new CompanionArchiveClient(endpoint,credential,async(url,options)=>{
   const response=await fetch(url,{...options,headers:{...options.headers,Origin:'http://localhost:8788'}});
   if(dropReply&&url.endsWith('/archive/import')&&response.ok){dropReply=false;await response.arrayBuffer();throw new TypeError('Simulated lost response after commit');}return response;
  });
  const library=new ArchiveLibrary(client,{prepare:wasm.prepare_archived_pgn,connection:()=>({endpoint,credential}),details(){throw Error('Browser metadata used');},exporter(){throw Error('Browser export used');}});
  const annotated='[White "Ada"]\n[Black "Ben"]\n[Date "2026.10.24"]\n\n1. e4 {[%clk 0:10:00] Plan} e5 2. Nf3 $1 (2. Nc3 Nc6) Nc6 *';
  const custom='[White "Custom"]\n[SetUp "1"]\n[FEN "8/8/8/8/8/8/4K3/7k w - - 47 63"]\n\n63. Kf3 Kh2 64. Kg4 Kg2 *';
  const totals={imported:0,duplicates:0,discarded:0,failed:0};
  await library.importPgnBatch([annotated,custom,'[White "Short"]\n\n1. e4 *','[White "Invalid"]\n\n1. e5 *'],r=>{for(const key of Object.keys(totals))totals[key]+=r[key]||0;});
  assert.deepEqual(totals,{imported:2,duplicates:0,discarded:1,failed:1});
  const rows=await client.request('archive/games?limit=100');assert.equal(rows.games.length,2);
  const bundle=await client.request('archive/export',{ids:rows.games.map(r=>r.id),format:'json'});
  assert.ok(bundle.documents.every(doc=>doc.record==null));
  const stored=bundle.documents.find(doc=>doc.pgn.includes('Ada'));assert.match(stored.pgn,/\[%clk 0:10:00\] Plan/);assert.match(stored.pgn,/\$1/);assert.match(stored.pgn,/\(\s*2\. Nc3\s+2\.\.\. Nc6\s*\)/);
  assert.ok(bundle.documents.some(doc=>doc.pgn.includes('47 63')));
  for(const row of rows.games){const saved=await library.read(row.id);assert.ok(JSON.parse(saved.json).review_pgn);}
  const retried='[White "Lost reply"]\n[Black "Retry"]\n\n1. d4 d5 2. c4 e6 *';let result;
  dropReply=true;await library.importPgnBatch([retried],r=>{result=r;});
  assert.equal(result.imported,0);assert.equal(result.duplicates,1);
  assert.equal((await client.request('archive/stats')).games,3);
 }finally{
  if(child&&child.exitCode===null){const closed=once(child,'close');child.kill();await closed;}
  dom.window.close();
  if(!root.startsWith(parent+sep))throw Error('Refusing to clean a test archive outside output');
  await rm(root,{recursive:true,force:true});
 }
});
