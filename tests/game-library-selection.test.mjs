import { JSDOM } from 'jsdom';
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import test from 'node:test';

test('saved-game bulk selection, confirmation, atomic failure, and pagination', async () => {
  const dom = new JSDOM('<body></body>', { url: 'https://test.local' });
  const { window } = dom;
  Object.assign(globalThis, { window, document: window.document, localStorage: window.localStorage });
  window.HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  window.HTMLDialogElement.prototype.close = function (value = '') {
    this.returnValue = value; this.open = false; this.dispatchEvent(new window.Event('close'));
  };
  const records = new Map(Array.from({ length: 12 }, (_, i) => [String(i), {
    id: String(i), category: i === 11 ? 'imported' : 'mine', updatedAt: 100 - i,
    json: JSON.stringify({ live_moves: Array(i + 1).fill('e4'), move_notes: i === 0 ? ['Opening plan\n<img src=x onerror=alert(1)>'] : [], board: '8/8/8/8/8/8/8/8 w - - 0 1', review_pgn: `[White "Player${i}"]\n[Black "Opponent"]\n[Date "2026.09.27"]\n\n1. e4 *` }),
  }]));
  let failDelete = false;
  let deletes = [];
  const db = { transaction(_store, mode) {
    let aborted=false;
    const tx = { objectStore() { return {
      openCursor() {
        const request = {};
        const values = [...records.values()];
        let index = 0;
        const advance = () => queueMicrotask(() => {
          request.result = index < values.length ? { value: values[index++], continue: advance } : null;
          request.onsuccess();
        });
        advance(); return request;
      },
      getAll() {const request={};queueMicrotask(()=>{request.result=structuredClone([...records.values()]);request.onsuccess();});return request;},
      get(id) {const request={};queueMicrotask(()=>{request.result=structuredClone(records.get(id));request.onsuccess();});return request;},
      delete(id) { deletes.push(id); },
    }; } };
    tx.abort=()=>{aborted=true;deletes=[];tx.onabort();};
    if (mode === 'readwrite') setTimeout(() => {
      if(aborted)return;
      if (failDelete) { tx.error = new Error('simulated failure'); tx.onabort(); }
      else { deletes.forEach(id => records.delete(id)); tx.oncomplete(); }
      deletes = [];
    }, 0);
    return tx;
  } };
  globalThis.indexedDB = window.indexedDB = { open() {
    const request = {};
    queueMicrotask(() => { request.result = db; request.onsuccess(); }); return request;
  } };
  const source = await readFile(new URL('../web/game-storage.js', import.meta.url), 'utf8');
  const { buildGameExport } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
  await window.ironwoodOpenGameLibrary();
  const button = text => [...document.querySelectorAll('button')].find(b => b.textContent === text);
  const click = text => { const b = button(text); assert.ok(b, text); b.click(); };
  const settle = () => new Promise(resolve => setTimeout(resolve, 15));
  assert.equal(document.querySelectorAll('.game-library-select').length, 10);
  const savedWindow=document.querySelector('#game-library-dialog'), heading=savedWindow.querySelector('.game-library-heading');
  savedWindow.getBoundingClientRect=()=>({left:50,top:60,width:600,height:500});
  heading.dispatchEvent(new window.MouseEvent('pointerdown',{bubbles:true,button:0,clientX:60,clientY:70}));
  heading.dispatchEvent(new window.MouseEvent('pointermove',{bubbles:true,clientX:200,clientY:140}));
  assert.equal(savedWindow.style.left,'190px');assert.equal(savedWindow.style.top,'130px');
  heading.dispatchEvent(new window.MouseEvent('pointercancel'));
  heading.dispatchEvent(new window.MouseEvent('pointermove',{clientX:999,clientY:999}));
  assert.equal(savedWindow.style.left,'190px');assert.equal(heading.classList.contains('dragging'),false);
  const sourceButtons=document.querySelector('.game-library-sources');
  assert.equal(sourceButtons.hidden,true);
  let companion=true;window.ironwoodCompanionFeatureEnabled=()=>companion;
  window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));
  assert.equal(sourceButtons.hidden,false);
  let archiveRecords=[], failImport=true, mutateSource=false;
  const archiveStore={endpoint:'http://localhost:8790',check(){},async list(){return structuredClone(archiveRecords);},async importRecords(incoming){if(failImport)throw Error('simulated archive failure');if(mutateSource)records.get(incoming[0].id).updatedAt++;const next=incoming.map(record=>({...structuredClone(record),id:'remote-'+record.id,_archive:{endpoint:this.endpoint,id:'remote-'+record.id,revision:1}}));archiveRecords=[...new Map([...archiveRecords,...next].map(record=>[record.id,record])).values()];return next.map(record=>record.id);},async moveToBrowser(ids){const moved=archiveRecords.filter(record=>ids.includes(record.id));for(const record of moved){const {_archive,...data}=record;records.set(record.id.replace('remote-',''),{...data,id:record.id.replace('remote-','')});}archiveRecords=archiveRecords.filter(record=>!ids.includes(record.id));return {received:moved.length,duplicates:0,ids:moved.map(record=>record.id.replace('remote-',''))};}};
  window.ironwoodCompanionLibrary=async()=>archiveStore;
  // The same game cards and controls are used on each source.
  const row=document.querySelector('.game-library-entry');
  [...row.querySelectorAll('button')].find(button=>button.textContent==='Move to Archive').click();await settle();
  assert.equal(records.size,12);assert.match(document.querySelector('.game-library-notice').textContent,/simulated archive failure/);
  failImport=false;mutateSource=true;
  [...document.querySelector('.game-library-entry').querySelectorAll('button')].find(button=>button.textContent==='Move to Archive').click();await settle();
  assert.equal(records.size,12);assert.match(document.querySelector('.game-library-notice').textContent,/changed during transfer/);
  mutateSource=false;
  [...document.querySelector('.game-library-entry').querySelectorAll('button')].find(button=>button.textContent==='Move to Archive').click();await settle();

  assert.equal(records.size,11);assert.equal(archiveRecords.length,1);
  for(const index of [0,1]){const input=document.querySelectorAll('.game-library-select')[index];input.checked=true;input.dispatchEvent(new window.Event('change'));}
  click('Move to Archive');await settle();assert.equal(records.size,9);assert.equal(archiveRecords.length,3);
  click('Archive');await settle();
  assert.equal(document.querySelector('.game-library-browser').hidden,false);
  assert.equal(document.querySelectorAll('.game-library-select').length,3);
  assert.equal(button('Archive').getAttribute('aria-pressed'),'true');
  assert.equal(document.querySelectorAll('.game-library-controls').length,1);
  // Archive previews load moves from Companion rather than IndexedDB.
  let archiveReads = 0;
  const previewFen = '8/8/8/8/8/8/8/K6k w - - 0 1';
  archiveStore.read = async id => {
    archiveReads++;
    const record = archiveRecords.find(record => record.id === id);
    assert.ok(record);
    return {...record, json: JSON.stringify({live_positions: [previewFen, previewFen, previewFen], live_moves: ['e4', 'e5']})};
  };
  document.querySelector('.game-library-preview').click();await settle();
  const archivePreview = document.querySelector('#game-preview-dialog');
  const previewButton = label => [...archivePreview.querySelectorAll('button')].find(button => button.textContent === label);
  assert.equal(archiveReads, 1);
  assert.doesNotMatch(archivePreview.textContent, /not found|Move preview unavailable/);
  assert.equal(previewButton('Auto play').disabled, false);
  previewButton('First').click();
  assert.match(archivePreview.querySelector('[aria-label^="Position"]').getAttribute('aria-label'), /Position 0 of 2/);
  previewButton('Auto play').click();
  assert.equal(previewButton('Pause').getAttribute('aria-pressed'), 'true');
  await new Promise(resolve => setTimeout(resolve, 550));
  assert.match(archivePreview.querySelector('[aria-label^="Position"]').getAttribute('aria-label'), /Position 1 of 2/);
  archivePreview.close();
  click('Select all 3 matching');
  click('Move to Browser');await settle();
  assert.equal(records.size,12);assert.equal(archiveRecords.length,0);
  click('Browser');await settle();assert.equal(document.querySelectorAll('.game-library-select').length,10);
  click('Archive');await settle();companion=false;window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));await settle();
  assert.equal(sourceButtons.hidden,true);
  assert.equal(document.querySelectorAll('.game-library-select').length,10);

  const chooseSort = value => {
    const select = document.querySelector('[aria-label="Sort saved games"]');
    assert.ok(select); select.value = value; select.dispatchEvent(new window.Event('change'));
  };
  chooseSort('moves-desc');
  assert.match(document.querySelector('.game-library-main strong').textContent, /Player11 vs/);
  assert.equal(localStorage.getItem('ironwood.chess.library-sort.v1'), 'moves-desc');
  click('Next ▶');
  assert.match(document.querySelector('.game-library-main strong').textContent, /Player1 vs/);
  chooseSort('moves-asc');
  assert.match(document.querySelector('.game-library-pager').textContent, /Page 1 of 2/);
  assert.match(document.querySelector('.game-library-main strong').textContent, /Player0 vs/);
  chooseSort('recent');

  const notePreview = document.querySelector('.game-starting-note-preview');
  assert.equal(notePreview.textContent, 'Opening plan\n<img src=x onerror=alert(1)>');
  assert.equal(document.querySelectorAll('.game-starting-note-preview').length, 1);
  assert.equal(notePreview.querySelector('img'), null);
  document.querySelector('.game-library-preview').click();
  const fullNote = document.querySelector('#game-preview-dialog .game-starting-note-full');
  assert.equal(fullNote.textContent, notePreview.textContent);
  assert.equal(fullNote.querySelector('img'), null);
  document.querySelector('#game-preview-dialog').close();
  assert.equal(button('Clear selection'), undefined);
  assert.equal(button('Export ▾').disabled, true);
  const samples = [{json:'first'}, {json:'second'}];
  const exporter = (json, format) => format === 'json' ? JSON.stringify({game:json}) : `[Event "${json}"]\n\n1. e4 *\n`;
  assert.equal(buildGameExport(samples,'pgn',exporter).content.split('[Event').length,3);
  assert.equal(buildGameExport([samples[0]],'json',exporter).filename,'ironwood-game.json');
  const archive = buildGameExport(samples,'json',exporter);
  assert.equal(archive.filename,'ironwood-games-analysis.zip');
  // Independently open and CRC-check the archive with Python's standard ZIP reader.
  const { spawnSync } = await import('node:child_process');
  const checked = spawnSync('python', ['-c', 'import sys,io,zipfile,json; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); assert z.testzip() is None; assert z.namelist()==["game-1.json","game-2.json"]; assert json.loads(z.read("game-2.json"))["game"]=="second"'], {input:archive.content});
  assert.equal(checked.status,0,checked.stderr.toString());
  document.querySelector('.game-library-select').click();
  click('Export ▾');
  assert.equal(button('Annotated PGN').closest('.game-library-export-options').hidden,false);
  button('Annotated PGN').dispatchEvent(new window.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
  assert.equal(button('Annotated PGN').closest('.game-library-export-options').hidden,true);
  click('Next ▶');
  document.querySelector('.game-library-select').click();
  assert.equal(button('Delete selected (2)').disabled, false);
  click('Delete selected (2)'); await settle();
  assert.equal(document.querySelector('#game-delete-dialog h2').textContent, 'Delete 2 saved games?');
  click('Cancel'); await settle();
  assert.equal(records.size, 12);
  click('Delete selected (2)'); await settle();
  click('Delete 2 games'); await settle();
  assert.equal(records.size, 10);
  assert.equal(records.has('0'), false);
  assert.equal(records.has('10'), false);
  assert.match(document.querySelector('.game-library-pager').textContent, /Page 1 of 1/);
  click('Select all 10 matching');
  const search = document.querySelector('input[type=search]');
  search.value = 'Player11'; search.dispatchEvent(new window.Event('input'));
  assert.equal(button('Delete selected (0)').disabled, true);
  click('Select all 1 matching');
  failDelete = true;
  click('Delete selected (1)'); await settle();
  click('Delete game'); await settle();
  assert.equal(records.size, 10);
  assert.match(document.querySelector('[role=alert]').textContent, /simulated failure/);
  failDelete = false;
  click('Delete selected (1)'); await settle();
  click('Delete game'); await settle();
  assert.equal(records.has('11'), false);
  assert.equal(records.size, 9);
  assert.equal(button('Delete selected (0)').disabled, true);
  assert.match(document.querySelector('.game-library-list').textContent, /No games match/);
  records.set('puzzle',{id:'puzzle',category:'imported',updatedAt:200,json:JSON.stringify({board:'8/8/8/8/8/8/8/8 w - - 0 1',review_pgn:'[Event "Lichess puzzle"]\n[PuzzleId "abc12"]\n\n1. e4 *'})});
  document.getElementById('game-library-dialog').close();
  await window.ironwoodOpenGameLibrary();
  const restoredSearch=document.querySelector('input[type=search]');
  assert.equal(restoredSearch.value,'Player11');restoredSearch.value='';restoredSearch.dispatchEvent(new window.Event('input'));
  click('Puzzles (1)');
  assert.equal(document.querySelectorAll('.game-library-select').length,1);
  assert.equal(document.querySelector('.game-library-main strong').textContent,'Lichess puzzle abc12');
  assert.equal(button('Move to My Games').hidden,true);
  assert.match(document.querySelector('.game-library-result').textContent,/Puzzle move history/);
  document.getElementById('game-library-dialog').close();
  window.HTMLCanvasElement.prototype.getContext=()=>null;
  let statsCalls=0,offline=false;
  archiveStore.stats=async()=>{statsCalls++;if(offline)throw Error('server offline');return {server_version:'0.1.0',schema_version:2,games:42,categories:{mine:30,imported:12},favorites:4,collections:2,database_bytes:4096,wal_bytes:8192,shared_memory_bytes:32768,total_bytes:45056,allocated_bytes:12288,reusable_bytes:4096,index:{indexed:40,total:42,pending:2,failed:0}};};
  await window.ironwoodOpenStorageInfo();await settle();assert.equal(statsCalls,0);assert.ok(!document.querySelector('#storage-management-dialog').textContent.includes('Server archive'));
  companion=true;window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));await settle();assert.equal(statsCalls,0);
  const info=document.querySelector('#storage-management-dialog');
  assert.ok(info.querySelector('.game-library-title-group .game-library-sources'));
  const infoHeading=info.querySelector('.game-library-heading');
  info.getBoundingClientRect=()=>({left:50,top:60,width:600,height:500});
  infoHeading.dispatchEvent(new window.MouseEvent('pointerdown',{bubbles:true,button:0,clientX:60,clientY:70}));
  infoHeading.dispatchEvent(new window.MouseEvent('pointermove',{bubbles:true,clientX:160,clientY:140}));
  assert.equal(info.style.left,'150px');assert.equal(info.style.top,'130px');
  infoHeading.dispatchEvent(new window.MouseEvent('pointercancel'));assert.equal(infoHeading.classList.contains('dragging'),false);
  infoHeading.dispatchEvent(new window.MouseEvent('pointermove',{clientX:999,clientY:999}));assert.equal(info.style.left,'150px');
  const infoTab=label=>[...info.querySelectorAll('[role=tab]')].find(tab=>tab.textContent===label);
  assert.equal(infoTab('Browser').getAttribute('aria-selected'),'true');
  assert.equal(info.querySelector('#storage-info-archive-panel').hidden,true);
  infoTab('Archive').click();await settle();assert.equal(statsCalls,1);
  assert.equal(info.querySelector('#storage-info-browser-panel').hidden,true);
  assert.equal(info.querySelector('#storage-info-archive-panel').hidden,false);
  const toolbar=info.querySelector('.storage-info-toolbar');
  assert.equal(toolbar.hidden,false);assert.equal(toolbar.querySelectorAll('button').length,2);
  assert.ok(toolbar.compareDocumentPosition(info.querySelector('.storage-info-body'))&window.Node.DOCUMENT_POSITION_FOLLOWING);
  assert.equal(info.querySelector('#storage-info-archive-panel .storage-actions'),null);

  const serverPanel=[...document.querySelectorAll('.storage-info-panel')].find(panel=>panel.querySelector('h3')?.textContent==='Server archive');
  assert.ok(serverPanel.textContent.includes('Saved games42'));assert.ok(serverPanel.textContent.includes('Total archive files44.0 KiB'));
  assert.ok(document.querySelector('#storage-management-dialog').textContent.includes('Game library'));
  offline=true;click('Refresh server');await settle();assert.match(document.querySelector('#storage-management-dialog').textContent,/server offline/);
  assert.ok(document.querySelector('#storage-management-dialog').textContent.includes('Game library'));
  infoTab('Browser').click();assert.equal(info.querySelector('#storage-info-archive-panel').hidden,true);assert.equal(info.querySelector('#storage-info-browser-panel').hidden,false);assert.equal(info.querySelector('.storage-info-toolbar').hidden,true);
  assert.match(info.querySelector('#storage-info-browser-panel').textContent,/Game library.*Browser storage/s);
  infoTab('Browser').dispatchEvent(new window.KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}));await settle();assert.equal(infoTab('Archive').getAttribute('aria-selected'),'true');
  companion=false;window.dispatchEvent(new window.Event('ironwood-companion-feature-changed'));await settle();assert.ok(!document.querySelector('#storage-management-dialog').textContent.includes('Server archive'));assert.equal(statsCalls,3);
  assert.equal(info.querySelector('.game-library-sources').hidden,true);assert.equal(info.querySelector('#storage-info-browser-panel').hidden,false);
  document.getElementById('storage-management-dialog').close();
  dom.window.close();
});
