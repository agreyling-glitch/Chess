import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequestCoordinator, lichessRequests} from '../web/lichess/requests.js';
import {createClient} from '../web/lichess/client.js';

const tick=()=>new Promise(resolve=>setImmediate(resolve));

test('queue holds the lane until body consumption and survives failures and cancelled work',async()=>{
  const queue=createRequestCoordinator();let release;const calls=[];
  const first=queue.run(async()=>{calls.push('first');await new Promise(resolve=>{release=resolve;});});
  const controller=new AbortController();
  const cancelled=queue.run(()=>calls.push('cancelled'),{signal:controller.signal});
  const rejected=assert.rejects(cancelled,{name:'AbortError'});controller.abort();
  const second=queue.run(()=>calls.push('second'));
  await tick();assert.deepEqual(calls,['first']);release();await Promise.all([first,rejected,second]);
  assert.deepEqual(calls,['first','second']);
  await assert.rejects(queue.run(()=>{throw Error('failed');}),/failed/);
  await queue.run(()=>calls.push('recovered'));assert.equal(calls.at(-1),'recovered');
});

test('429 cooldown covers queued requests and stream handshakes, honors seconds and HTTP dates',async()=>{
  let clock=Date.parse('2026-11-05T12:00:00Z'),calls=0;
  const queue=createRequestCoordinator({now:()=>clock});
  const limited=header=>queue.fetch('https://lichess.org/api/test',{},async()=>{calls++;return new Response('',{status:429,headers:{'Retry-After':header}});});
  await queue.run(()=>limited('120'));
  const blocked=()=>assert.rejects(queue.run(()=>{calls++;}),/rate limit/);
  await blocked();await assert.rejects(queue.fetch('https://lichess.org/api/stream/event',{},async()=>{calls++;}),/rate limit/);
  clock+=60001;await blocked();assert.equal(calls,1);
  clock+=60000;await queue.run(()=>{calls++;});assert.equal(calls,2);
  await limited(new Date(clock+180000).toUTCString());clock+=120000;await blocked();clock+=60000;
  await limited('invalid');clock+=59999;await blocked();clock++;queue.check();
});

test('imports, study refresh and the online client share a queue and cooldown while streams stay independent',async t=>{
  const saved={fetch:globalThis.fetch,window:globalThis.window,now:Date.now};
  let clock=Date.now(),release,calls=[];const controllers=[];
  Date.now=()=>clock;globalThis.window=globalThis;
  const game='[Event "Game"]\n[Result "1-0"]\n\n1. e4 e5 2. Nf3 Nc6 1-0';
  const study='[StudyName "Study"]\n[ChapterName "One"]\n\n1. e4 *';
  globalThis.fetch=async(url,options)=>{
    const path=new URL(url).pathname;calls.push(path);
    if(path==='/api/games/user/Ada')return {ok:true,status:200,text:()=>new Promise(resolve=>{release=()=>resolve(game);})};
    if(path.startsWith('/api/study/'))return new Response(study);
    if(path==='/api/account')return Response.json({id:'alice',username:'Alice'});
    if(path==='/api/account/playing')return Response.json({nowPlaying:[]});
    if(path==='/api/challenge')return Response.json({in:[],out:[]});
    if(path==='/api/stream/event')return new Response(new ReadableStream({start(controller){controllers.push(controller);options.signal.addEventListener('abort',()=>{try{controller.error(new DOMException('Aborted','AbortError'));}catch{}},{once:true});}}));
    if(path==='/api/user/Bob')return new Response('',{status:429,headers:{'Retry-After':'120'}});
    throw Error(`Unexpected request ${path}`);
  };
  const client=createClient({emit:()=>{}});
  t.after(()=>{client.disconnect();globalThis.fetch=saved.fetch;globalThis.window=saved.window;Date.now=saved.now;});
  const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8')
    .replaceAll("import('./lichess/requests.js')",`import(${JSON.stringify(new URL('../web/lichess/requests.js',import.meta.url).href)})`);
  await import(`data:text/javascript,${encodeURIComponent(source)}`);
  const importing=window.ironwoodFetchLichess('Ada');await tick();
  const refreshing=window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');
  const connecting=client.connect('token');await tick();
  assert.deepEqual(calls,['/api/games/user/Ada']);
  release();await Promise.all([importing,refreshing,connecting]);await tick();
  assert.equal(window.ironwoodPollLichess(),game);assert.equal(JSON.parse(window.ironwoodPollStudyRefresh()).pgn,study);
  assert.equal(client.snapshot().connected,true);assert.equal(controllers.length,1);
  // An open account stream has not blocked another finite request.
  await client.playerProfile(false,'Bob','test');const count=calls.length;
  await window.ironwoodFetchLichess('Ada');await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');
  assert.equal(calls.length,count);assert.match(window.ironwoodLichessStatus(),/rate limit/);
  assert.match(JSON.parse(window.ironwoodPollStudyRefresh()).error,/rate limit/);
  client.disconnect();clock+=60001;
  await window.ironwoodFetchLichess('Ada');assert.equal(calls.length,count);
  // A stream-level 429 uses the same limiter (exercised through its public transport).
  clock=Math.max(clock,lichessRequests.retryAt());
  await lichessRequests.fetch('https://lichess.org/api/stream/event',{},async()=>new Response('',{status:429}));
  await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.equal(calls.length,count);
  clock=lichessRequests.retryAt();await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.equal(calls.length,count+1);
});
