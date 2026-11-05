import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
globalThis.window = globalThis;
const source = readFileSync(new URL('../web/game-storage.js', import.meta.url), 'utf8')
  .replaceAll("import('./lichess/requests.js')", `import(${JSON.stringify(new URL('../web/lichess/requests.js',import.meta.url).href)})`);
const { lichessExportUrl, importFilters, gameDetails, libraryMetadataDetails, importFingerprint } = await import(`data:text/javascript,${encodeURIComponent(source)}`);

test('filters validate dates and generate inclusive UTC bounds', () => {
  const url = new URL(lichessExportUrl('Ada',{limit:100,speed:'blitz',from:'2026-09-01',to:'2026-09-30'}));
  assert.equal(url.searchParams.get('max'),'100');
  assert.equal(url.searchParams.get('perfType'),'blitz');
  assert.equal(Number(url.searchParams.get('since')),Date.parse('2026-09-01T00:00:00Z'));
  assert.equal(Number(url.searchParams.get('until')),Date.parse('2026-10-01T00:00:00Z')-1);
  for (const bad of [{from:'2026-02-30'},{from:'2026-10-01',to:'2026-09-01'},{limit:999},{speed:'invalid'}]) assert.throws(()=>importFilters(bad));
  assert.doesNotThrow(()=>lichessExportUrl('https://lichess.org/abcd1234',{from:'invalid'}));
});

test('username exports are bounded to completed games', () => {
  const url = new URL(lichessExportUrl(' Ada '));
  assert.equal(url.pathname, '/api/games/user/Ada');
  assert.equal(url.searchParams.get('max'), '20');
  assert.equal(url.searchParams.get('ongoing'), 'false');
});
test('game URLs discard player secrets and reject other hosts', () => {
  assert.equal(new URL(lichessExportUrl('https://lichess.org/abcd1234WXYZ/black#12')).pathname, '/game/export/abcd1234');
  for (const input of ['https://evil.com/abcd1234', 'https://lichess.org@evil.com/abcd1234', 'https://lichess.org/study/short', 'bad name']) {
    assert.throws(() => lichessExportUrl(input));
  }
});
test('study links export all chapters or one chapter with annotations', () => {
  for (const [link,path] of [
    ['https://lichess.org/study/abcd1234','/api/study/abcd1234.pgn'],
    ['https://lichess.org/study/abcd1234/WXYZ5678#12','/api/study/abcd1234/WXYZ5678.pgn'],
  ]) {
    const url = new URL(lichessExportUrl(link,{from:'invalid'}));
    assert.equal(url.pathname,path);
    for (const option of ['comments','variations','clocks','orientation']) assert.equal(url.searchParams.get(option),'true');
    assert.equal(url.searchParams.has('ongoing'),false);
  }
  for (const link of ['https://lichess.org/study/abcd1234/short','https://lichess.org/study/abcd1234/WXYZ5678/extra','https://user@lichess.org/study/abcd1234']) assert.throws(()=>lichessExportUrl(link));
});

test('completed single games are cached, bounded, expiring and refreshable without caching studies or collections', async t => {
  const originalFetch=globalThis.fetch,originalNow=Date.now;let clock=originalNow(),calls=0;
  t.after(()=>{globalThis.fetch=originalFetch;Date.now=originalNow;});Date.now=()=>clock;
  const body='[Event "Completed"]\n[Result "1-0"]\n\n1. e4 e5 2. Nf3 Nc6 1-0';
  globalThis.fetch=async()=>{calls++;return new Response(body);};
  const load=async(input,options={})=>{await window.ironwoodFetchLichess(input,options);assert.equal(window.ironwoodPollLichess(),body);};
  await load('https://lichess.org/cache001');await load('https://lichess.org/cache001ABCD/black');assert.equal(calls,1);
  await load('https://lichess.org/cache001',{refresh:true});assert.equal(calls,2);
  clock+=30*60000+1;await load('https://lichess.org/cache001');assert.equal(calls,3);
  await load('Ada');await load('Ada');assert.equal(calls,5);
  await load('https://lichess.org/study/cache001');await load('https://lichess.org/study/cache001');assert.equal(calls,7);
  for(let i=0;i<32;i++)await load(`https://lichess.org/cache${String(i+10).padStart(3,'0')}`);
  const before=calls;await load('https://lichess.org/cache001');assert.equal(calls,before+1);
});
test('fetch hands PGN off once, rejects ongoing games, and honors rate limits', async () => {
  let calls = 0;
  let body = '[Event "Rated blitz"]\n[Result "1-0"]\n\n1. e4 1-0';
  let status = 200;
  globalThis.fetch = async (url, options) => {
    calls++;
    assert.equal(options.credentials, 'omit');
    assert.equal(options.headers.Accept, 'application/x-chess-pgn');
    return { ok: status === 200, status, text: async () => body };
  };
  await window.ironwoodFetchLichess('Ada');
  assert.equal(window.ironwoodPollLichess(), body);
  assert.equal(window.ironwoodPollLichess(), '');
  body = '[Event "Live"]\n[Result "*"]';
  await window.ironwoodFetchLichess('https://lichess.org/abcd1234');
  assert.equal(window.ironwoodPollLichess(), '');
  assert.match(window.ironwoodLichessStatus(), /still in progress/);
  body = '[Event "Study: Chapter"]\n[StudyName "Study"]\n[ChapterName "Chapter"]\n[Result "*"]\n\n1. e4 {note} (1. d4) *';
  await window.ironwoodFetchLichess('https://lichess.org/study/abcd1234');
  assert.equal(window.ironwoodPollLichess(),body);
  assert.match(window.ironwoodLichessStatus(),/Opening study/);
  status = 403;
  await window.ironwoodFetchLichess('https://lichess.org/study/abcd1234');
  assert.equal(window.ironwoodPollLichess(),'');
  assert.match(window.ironwoodLichessStatus(),/public study/);
  status = 429;
  await window.ironwoodFetchLichess('Ada');
  const before = calls;
  await window.ironwoodFetchLichess('Ada');
  assert.equal(calls, before);
  assert.match(window.ironwoodLichessStatus(), /full minute/);
});


test('saved chapters retain study titles and source metadata', () => {
  const pgn = '[Event "Study: Chapter"]\n[StudyName "My study"]\n[ChapterName "First chapter"]\n[ChapterURL "https://lichess.org/study/abcd1234/WXYZ5678"]\n[Result "*"]\n\n1. e4 *';
  const details = gameDetails(JSON.stringify({review_pgn:pgn,board:'',game_analysis:[],move_notes:['Introduction']}));
  assert.equal(details.title,'My study · 1 chapter');
  assert.equal(details.category,'studies');
  assert.equal(details.startingNote,'Introduction');
  assert.equal(details.chapterUrl,'https://lichess.org/study/abcd1234/WXYZ5678');
  assert.equal(libraryMetadataDetails(details,{}).title,details.title);
});


test('distinct study chapters are not discarded as duplicate games', () => {
  const chapter = name => `[StudyName "Study"]\n[ChapterName "${name}"]\n[Result "*"]\n\n*`;
  assert.ok(importFingerprint(chapter('Introduction')));
  assert.notEqual(importFingerprint(chapter('Introduction')),importFingerprint(chapter('Position')));
  assert.equal(importFingerprint(chapter('Introduction')),importFingerprint(chapter('Introduction')));
});
