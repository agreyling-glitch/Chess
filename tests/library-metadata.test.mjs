import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {JSDOM} from 'jsdom';
const dom=new JSDOM('<body></body>',{url:'https://metadata.test.local'});
Object.assign(globalThis,{window:dom.window,document:dom.window.document,localStorage:dom.window.localStorage});
window.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
window.HTMLDialogElement.prototype.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));this.remove();};
const source=await readFile(new URL('../web/game-storage.js',import.meta.url),'utf8');
const {editLibraryMetadata,libraryMetadataDetails,libraryEventVenueLabel,validateLibraryMetadata,enginePlayerName,copyRecordsToArchive,validateBrowserCollections,mergeCollectionBackup}=await import(`data:text/javascript,${encodeURIComponent(source)}`);
const json=JSON.stringify({board:'8/8/8/8/8/8/8/K6k w - - 0 1',history:[],live_moves:['e4'],review_pgn:'[White "Original"]\n[Black "Opponent"]\n[Event "Original event"]\n\n1. e4 {note [%clk 0:01:00]} *',move_notes:['keep note'],game_analysis:[{score:5}]});
test('metadata editor saves readable fields while preserving original game data and unknown tags',async()=>{
 const record={id:'game',json,metadata:{White:'Corrected',CustomTag:'keep'}};
 let saved;
 const dialog=editLibraryMetadata(record,async value=>{saved=value;});
 assert.equal(dialog.querySelector('[aria-label="White player"]').value,'Corrected');
 dialog.querySelector('[aria-label="White player"]').value='New player';
 dialog.querySelector('[aria-label="Date (YYYY.MM.DD)"]').value='2026.10.30';
 await dialog.querySelector('form').onsubmit({preventDefault(){}});
 assert.equal(saved.White,'New player');assert.equal(saved.Date,'2026.10.30');assert.equal(saved.CustomTag,'keep');assert.equal(record.json,json);
 const details=libraryMetadataDetails({white:'Original',black:'Opponent'},saved);
 assert.equal(details.title,'New player vs Opponent');assert.equal(details.playedAt,'2026.10.30');
});
test('invalid fields and revision conflicts leave the editor open and preserve user input',async()=>{
 let attempts=0;
 const dialog=editLibraryMetadata({id:'game',json,_archive:{endpoint:'http://nas:8790',document:{metadata:{White:'NAS'}}}},async()=>{attempts++;throw Error('Revision changed; refresh before editing');});
 const input=dialog.querySelector('[aria-label="ECO"]');input.value='Z99';
 await dialog.querySelector('form').onsubmit({preventDefault(){}});
 assert.equal(attempts,0);assert.match(dialog.textContent,/A00 through E99/);
 input.value='B01';await dialog.querySelector('form').onsubmit({preventDefault(){}});
 assert.equal(attempts,1);assert.ok(dialog.open);assert.equal(input.value,'B01');assert.match(dialog.textContent,/Revision changed/);dialog.close();
 assert.throws(()=>validateLibraryMetadata({White:'bad\nname'}),/single line/);
});


test('saved Event is displayed separately from an unknown venue',()=>{
 const game=libraryMetadataDetails({white:'White',black:'Black',venue:null},{Event:'Testing'});
 assert.equal(game.event,'Testing');
 assert.equal(libraryEventVenueLabel(game),'Event: Testing · Venue unknown');
 const venue=libraryMetadataDetails(game,{Site:'Chicago'});
 assert.equal(libraryEventVenueLabel(venue),'Event: Testing · Venue: Chicago');
});


test('engine player identities group historical ratings and keep actual model names',()=>{
 for(const rating of [100,116,132,468])assert.equal(enginePlayerName(`Maia3 79M (GPU) ${rating} Elo`),'Maia3 79M (GPU)');
 assert.equal(enginePlayerName('Lc0 · Level 2 · 10 nodes'),'Lc0');
 assert.equal(enginePlayerName('Stockfish 1320 Elo'),'Stockfish');
 assert.equal(enginePlayerName('Maia3 59M (GPU)'),'Maia3 59M (GPU)');
 assert.equal(enginePlayerName('Benitez Villalba, Amaia Beatriz'),'Benitez Villalba, Amaia Beatriz');
});


test('copy to archive keeps source records intact on success and failure',async()=>{
 const records=[{id:'browser-one',json,metadata:{Event:'Testing'},favorite:true}];const before=structuredClone(records);let calls=0;
 assert.deepEqual(await copyRecordsToArchive(records,{async importRecords(value){calls++;assert.equal(value,records);return ['archive-one'];}}),['archive-one']);
 assert.equal(calls,1);assert.deepEqual(records,before);
 await assert.rejects(copyRecordsToArchive(records,{async importRecords(){throw Error('offline');}}),/offline/);
 assert.deepEqual(records,before);
 await assert.rejects(copyRecordsToArchive(records,{async importRecords(){return [];}}),/confirm every/);
});


test('collection backups remap conflicting IDs and retain memberships without duplicating repeated restores',()=>{
 const original=[{id:'one',name:'Openings'}],incoming=[{id:'one',name:'Endgames'}];const games=[{id:'game',json,collectionIds:['one','missing']}];
 const merged=mergeCollectionBackup(original,incoming,games,false,()=> 'new-id');
 assert.deepEqual(merged.collections,[...original,{id:'new-id',name:'Endgames'}]);assert.deepEqual(merged.games[0].collectionIds,['new-id']);assert.equal(merged.games[0].json,json);
 const repeated=mergeCollectionBackup(merged.collections,incoming,games,false,()=> 'must-not-add');assert.equal(repeated.collections.length,2);assert.deepEqual(repeated.games[0].collectionIds,['new-id']);
 const again=mergeCollectionBackup(original,[{id:'one',name:'Openings'}],games);assert.equal(again.collections.length,1);
 const replaced=mergeCollectionBackup(original,[],games,true);assert.deepEqual(replaced.collections,[]);assert.deepEqual(replaced.games[0].collectionIds,[]);
 assert.throws(()=>validateBrowserCollections([{id:'one',name:'A'},{id:'one',name:'B'}]),/Invalid/);
});
