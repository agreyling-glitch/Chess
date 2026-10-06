import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8');
const { chessComUsername, fetchChessComGames } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
const base = 'https://api.chess.com/pub/player/ada/games';
const game = (n, rules = 'chess', result = '1-0') => ({rules,end_time:n,url:`https://www.chess.com/game/live/${n}`,pgn:`[Event "Game ${n}"]\n[Result "${result}"]\n\n1. e4 ${result}`});
function requestFor(routes, calls) {return async (url,options) => {
  calls.push(url);assert.equal(options.credentials,'omit');assert.equal(options.headers.Accept,'application/json');
  const body=routes[url];assert.notEqual(body,undefined,url);
  return {ok:true,status:200,json:async()=>body};
};}
test('usernames are normalized and external URLs are rejected',()=>{
  assert.equal(chessComUsername(' Ada '),'ada');
  for(const value of ['https://chess.com/member/ada','bad name','../ada','a'])assert.throws(()=>chessComUsername(value));
});
test('reads latest months sequentially, filters variants/unfinished games, limits latest 20',async()=>{
  const calls=[];let inFlight=0;
  const routes={
    [`${base}/archives`]:{archives:[`${base}/2026/08`,`${base}/2026/09`,'https://evil.com/month',`${base}/2026/13`]},
    [`${base}/2026/09`]:{games:[...Array.from({length:8},(_,i)=>game(i+21)),game(100,'bughouse'),game(101,'chess','*'),game(29,'chess960')]},
    [`${base}/2026/08`]:{games:Array.from({length:20},(_,i)=>game(i+1))},
  };
  const mock=requestFor(routes,calls);
  const pgn=await fetchChessComGames('Ada',async(...args)=>{
    assert.equal(inFlight,0);inFlight++;await Promise.resolve();const result=await mock(...args);inFlight--;return result;
  });
  assert.deepEqual(calls,[`${base}/archives`,`${base}/2026/09`,`${base}/2026/08`]);
  assert.equal((pgn.match(/\[Event /g)||[]).length,20);
  assert.ok(pgn.startsWith('[Event "Game 29"]'));assert.ok(pgn.includes('[Event "Game 10"]'));
  assert.ok(!pgn.includes('Game 9"'));assert.ok(!pgn.includes('Game 100'));
});
test('stops when newest month contains enough games and skips duplicate games',async()=>{
  const calls=[];const pgn=await fetchChessComGames('ada',requestFor({
    [`${base}/archives`]:{archives:[`${base}/2026/08`,`${base}/2026/09`,`${base}/2026/09`]},
    [`${base}/2026/09`]:{games:[...Array.from({length:25},(_,i)=>game(i+1)),game(25)]},
  },calls));assert.equal(calls.length,2);assert.equal((pgn.match(/\[Event /g)||[]).length,20);
});
test('empty, malformed and error responses produce useful messages',async()=>{
  await assert.rejects(fetchChessComGames('Ada',requestFor({[`${base}/archives`]:{archives:[]}},[])),/No completed/);
  await assert.rejects(fetchChessComGames('Ada',async()=>({ok:true,status:200,json:async()=>({})})),/unexpected archive/);
  for(const status of [404,429,500])await assert.rejects(fetchChessComGames('Ada',async()=>({ok:false,status})),status===404?/not found/:status===429?/rate limit/:/500/);
});
test('date and speed filters skip irrelevant months and include the entire end date',async()=>{
  const calls=[];
  const timed = (date,speed='blitz') => ({...game(Date.parse(date)/1000),time_class:speed});
  const pgn=await fetchChessComGames('Ada',requestFor({
    [`${base}/archives`]:{archives:['2026/08','2026/09','2026/10'].map(month=>`${base}/${month}`)},
    [`${base}/2026/09`]:{games:[timed('2026-09-30T23:59:59Z'),timed('2026-09-01T00:00:00Z'),timed('2026-09-20T00:00:00Z','rapid'),timed('2026-08-31T23:59:59Z')]},
  },calls),{limit:50,speed:'blitz',from:'2026-09-01',to:'2026-09-30'});
  assert.deepEqual(calls,[`${base}/archives`,`${base}/2026/09`]);
  assert.equal((pgn.match(/\[Event /g)||[]).length,2);
});

test('larger collections are capped at the chosen count',async()=>{
  const pgn=await fetchChessComGames('Ada',requestFor({
    [`${base}/archives`]:{archives:[`${base}/2026/09`]},
    [`${base}/2026/09`]:{games:Array.from({length:75},(_,i)=>game(i+1))},
  },[]),{limit:50});
  assert.equal((pgn.match(/\[Event /g)||[]).length,50);
});

test('window handoff is one-shot and rate limits prevent immediate retries',async()=>{
  let calls=0;
  globalThis.fetch=requestFor({[`${base}/archives`]:{archives:[`${base}/2026/09`]},[`${base}/2026/09`]:{games:[game(1)]}},[]);
  await window.ironwoodFetchChessCom('Ada');assert.match(window.ironwoodPollChessCom(),/Game 1/);assert.equal(window.ironwoodPollChessCom(),'');
  globalThis.fetch=async()=>{calls++;return {ok:false,status:429};};
  await window.ironwoodFetchChessCom('Ada');await window.ironwoodFetchChessCom('Ada');assert.equal(calls,1);assert.match(window.ironwoodChessComStatus(),/full minute/);
});
