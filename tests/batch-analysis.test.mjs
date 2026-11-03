import test from 'node:test';
import assert from 'node:assert/strict';
import {parseBatchInfo,runBatchAnalysis} from '../web/batch-analysis.js';
const white='8/8/8/8/8/8/K7/7k w - - 0 1',black=white.replace(' w ',' b ');
test('batch UCI scores use White perspective and ignore bounds and extra variations',()=>{
  assert.equal(parseBatchInfo('info depth 12 score cp 32 nodes 300 pv a2a3',black).eval_cp,-32);
  assert.equal(parseBatchInfo('info depth 12 score mate -3 nodes 300 pv a2a3',black).mate,3);
  assert.equal(parseBatchInfo('info score cp 40 lowerbound',white),null);
  assert.equal(parseBatchInfo('info multipv 2 score cp 40',white),null);
});
function fixture(options={}) {
  const controller=new AbortController(),saved=[],events=[];let closed=false,searches=0;
  const analysis={eval_cp:10,mate:null,depth:4,nodes:100,best_move:'a2a3',pv:'a2a3'};
  return {controller,saved,events,get closed(){return closed;},async run(){return runBatchAnalysis({
    ids:['a','b'],engine:{label:'Test engine'},signal:controller.signal,
    read:async id=>({id,title:id,json:JSON.stringify({move_notes:['keep me'],analysis_run:{id:'old'},analysis_config:{},full_game_analysis_config:{}})}),
    prepare:()=>({positions:[{fen:white},{fen:black},{fen:white,terminal:'draw'}]}),normalize:json=>json,
    save:async(record,json)=>{if(options.saveError)throw Error('Storage full');saved.push({id:record.id,game:JSON.parse(json)});},
    verify:options.verify!==false,identifyCritical:()=>options.targets||[],
    onProgress:p=>events.push(p),createEngine:async()=>({newGame(){},async search(fen,budget=1000){searches++;if(options.cancel&&searches===2||options.cancelPass2&&budget===5000){controller.abort();throw new DOMException('Cancelled','AbortError');}return {...analysis,depth:budget===5000?20:4};},async close(){closed=true;}})
  });}};
}
test('batch analyzes sequentially, preserves annotations and records a new run',async()=>{
  const f=fixture(),r=await f.run();assert.equal(r.completed,2);assert.equal(r.failed,0);assert.ok(f.closed);
  assert.deepEqual(f.saved.map(x=>x.id),['a','b']);assert.deepEqual(f.saved[0].game.move_notes,['keep me']);
  assert.equal(f.saved[0].game.analysis_run.parent_id,'old');assert.notEqual(f.saved[0].game.analysis_run.id,'old');
  assert.equal(f.saved[0].game.game_analysis[2].eval_cp,0);assert.equal(f.saved[0].game.game_analysis[2].best_move,null);
  assert.equal(f.events.at(-1).gameIndex,2);
});
test('cancel saves completed positions as a partial run and does not start another game',async()=>{
  const f=fixture({cancel:true}),r=await f.run();assert.equal(r.cancelled,true);assert.equal(r.partial,1);assert.equal(r.failed,0);
  assert.equal(f.saved.length,1);assert.equal(f.saved[0].game.game_analysis[1],null);assert.ok(f.closed);
});
test('failed saves are reported and never counted as completed games',async()=>{
  const f=fixture({saveError:true}),r=await f.run();assert.equal(r.completed,0);assert.equal(r.failed,2);assert.match(r.errors[0],/Storage full/);assert.ok(f.closed);
});
test('critical pass searches both targets with a larger budget and saves verified results',async()=>{
  const f=fixture({targets:[0,1]}),r=await f.run();assert.equal(r.completed,2);
  assert.deepEqual(f.saved[0].game.verification_targets,[0,1]);assert.equal(f.saved[0].game.verification_time_ms,5000);
  assert.equal(f.saved[0].game.game_analysis[0].depth,20);assert.equal(f.saved[0].game.verification_results[1].depth,20);
  assert.ok(f.events.some(event=>event.pass===2));
});
test('cancelling second pass preserves first pass and reports partial verification',async()=>{
  const f=fixture({targets:[0,1],cancelPass2:true}),r=await f.run();assert.equal(r.partial,1);assert.equal(r.completed,0);assert.equal(r.cancelled,true);
  assert.equal(f.saved[0].game.game_analysis[0].depth,4);assert.equal(f.saved[0].game.verification_results[0],null);
});
test('second pass can be switched off',async()=>{
  const f=fixture({targets:[0,1],verify:false}),r=await f.run();assert.equal(r.completed,2);assert.deepEqual(f.saved[0].game.verification_targets,[]);assert.ok(!f.events.some(event=>event.pass===2));
});
