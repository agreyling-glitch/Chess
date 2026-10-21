import { test } from 'node:test';
import assert from 'node:assert/strict';
globalThis.window=globalThis;
const { ArchiveExplorer }=await import('../web/companion-explorer.js');
const flush=()=>new Promise(resolve=>setImmediate(resolve));
test('old positions, filters and connection responses never replace the current explorer view',async()=>{
  let connection={endpoint:'http://first:8790',credential:'private'},time=0;const requests=[];
  const explorer=new ArchiveExplorer(()=>connection,{clock:()=>time,createClient:()=>({request:path=>new Promise(resolve=>requests.push({path,resolve}))})});
  assert.equal(explorer.poll('position A'),'');assert.equal(explorer.poll('position B'),'');
  requests[0].resolve({generation:1,moves:[{san:'stale'}]});await flush();assert.equal(explorer.poll('position B'),'');
  requests[1].resolve({generation:2,moves:[{san:'current'}]});await flush();assert.equal(JSON.parse(explorer.poll('position B')).moves[0].san,'current');
  explorer.setFilters({player:'Ada',color:'white'});assert.equal(explorer.poll('position B'),'');assert.ok(requests[2].path.includes('player=Ada'));
  connection={endpoint:'http://second:8790',credential:'another'};assert.equal(explorer.poll('position B'),'');requests[2].resolve({generation:3,moves:[{san:'old filters and server'}]});await flush();assert.equal(explorer.poll('position B'),'');
  requests[3].resolve({generation:1,moves:[]});await flush();assert.deepEqual(JSON.parse(explorer.poll('position B')).moves,[]);
  assert.ok(requests.every(r=>!r.path.includes('private')&&!r.path.includes('another')));
});
test('generation changes reset matching pages, partial results refresh and disconnected views fail clearly',async()=>{
  let time=0,connection={endpoint:'http://local:8790',credential:'test'},generation=1,indexing=true;const paths=[];
  const explorer=new ArchiveExplorer(()=>connection,{clock:()=>time,createClient:()=>({request:async path=>{paths.push(path);return {generation,indexing,moves:[],next:'second-page',recentGames:[]};}})});
  explorer.poll('fen');await flush();assert.equal(JSON.parse(explorer.poll('fen')).indexing,true);explorer.page(true);explorer.poll('fen');await flush();assert.ok(paths.at(-1).includes('game_after=second-page'));
  generation=2;indexing=false;time=1000;explorer.poll('fen');await flush();assert.equal(explorer.cursor,'');assert.equal(explorer.data,null);explorer.poll('fen');await flush();assert.equal(JSON.parse(explorer.poll('fen')).generation,2);
  connection=null;assert.match(JSON.parse(explorer.poll('fen')).error,/Connect through Storage/);
});
