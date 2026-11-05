import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
globalThis.window=globalThis;
const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8')
  .replaceAll("import('./lichess/requests.js')", `import(${JSON.stringify(new URL('../web/lichess/requests.js',import.meta.url).href)})`);
await import(`data:text/javascript,${encodeURIComponent(source)}`);
const pgn='[Event "Study"]\n[StudyName "Study"]\n[ChapterName "One"]\n\n1. e4 *';
test('study refresh fetches public PGN without credentials and preserves error details',async()=>{
  let status=200,calls=0,body=pgn;
  globalThis.fetch=async(url,options)=>{calls++;assert.match(url,/^https:\/\/lichess.org\/api\/study\/abcd1234.pgn\?/);assert.equal(options.credentials,'omit');return {status,ok:status===200,text:async()=>body};};
  await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');
  assert.deepEqual(JSON.parse(window.ironwoodPollStudyRefresh()),{pgn,url:'https://lichess.org/study/abcd1234'});
  assert.equal(window.ironwoodPollStudyRefresh(),'');
  for(const code of [403,404,500]){status=code;await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.match(JSON.parse(window.ironwoodPollStudyRefresh()).error,/local copy was kept/);}
  status=200;body='<html>error</html>';await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.match(JSON.parse(window.ironwoodPollStudyRefresh()).error,/invalid study/);
  const before=calls;await window.ironwoodRefreshStudy('https://evil.com/study/abcd1234');assert.ok(JSON.parse(window.ironwoodPollStudyRefresh()).error);assert.equal(calls,before);
  status=429;await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.match(JSON.parse(window.ironwoodPollStudyRefresh()).error,/full minute/);
  const limited=calls;await window.ironwoodRefreshStudy('https://lichess.org/study/abcd1234');assert.equal(calls,limited);assert.match(JSON.parse(window.ironwoodPollStudyRefresh()).error,/full minute/);
});
