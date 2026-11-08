import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {openCompanionDownload} from '../web/companion-download.js';

test('Companion downloads open the public Windows page without requesting private releases',()=>{
 const original=globalThis.window,calls=[];
 globalThis.window={open:(...args)=>calls.push(args)};
 try {
  openCompanionDownload();
  assert.deepEqual(calls,[['/downloads/companion/','_blank','noopener,noreferrer']]);
 } finally {globalThis.window=original;}
});

test('download entry point remains in the Companion tab and uses no persistent token storage',()=>{
 const source=readFileSync(new URL('../web/companion-download.js',import.meta.url),'utf8');
 assert.doesNotMatch(source,/localStorage|sessionStorage/);
 const rust=readFileSync(new URL('../src/app.rs',import.meta.url),'utf8');
 const start=rust.indexOf('if companion_tab {',rust.indexOf('about_version_diagnostics'));
 assert.ok(start>=0 && rust.indexOf('Download Companion for Windows',start)>start);
});
