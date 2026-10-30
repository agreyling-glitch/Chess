import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
test('retired Companion archive window has no menu, lazy loader or offline cache entry',async()=>{
 for(const name of ['../src/app.rs','../web/app-bootstrap.js','../web/service-worker.js']){
  const source=await readFile(new URL(name,import.meta.url),'utf8');
  assert.ok(!source.includes('ironwoodOpenCompanionArchive'),name);
  assert.ok(!source.includes("'/companion-archive.js'"),name);
 }
 const explorer=await readFile(new URL('../src/opening_explorer.rs',import.meta.url),'utf8');
 assert.ok(explorer.includes('js_name = ironwoodManageCompanionConnection'));
});
