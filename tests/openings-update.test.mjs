import {test} from 'node:test';import assert from 'node:assert/strict';
import {readFile,mkdtemp,mkdir,rm,readdir} from 'node:fs/promises';import {resolve,join,sep} from 'node:path';
import {updateOpenings,validateVolume} from '../scripts/update-openings.mjs';
const license=await readFile(new URL('../assets/openings/COPYING.txt',import.meta.url),'utf8');
const revision='a'.repeat(40),other='b'.repeat(40);
function upstream({sha=revision,badLicense=false,badVolume=false,network=false}={}) {
 const calls=[];const fetcher=async url=>{
  calls.push(url);if(network)return new Response('',{status:503});
  if(url.endsWith('/chess-openings'))return Response.json({default_branch:'master',license:{spdx_id:badLicense?'UNKNOWN':'CC0-1.0'}});
  if(url.includes('/git/ref/heads/'))return Response.json({object:{sha}});
  assert.ok(url.includes('/'+sha+'/'),'Every data file must use the same commit');
  const name=url.split('/').at(-1);
  return new Response(name==='COPYING.txt'?license:name==='README.md'?'The work is released elsewhere. Insofar as that qualifies for copyright, the work is released under the CC0 Public Domain Dedication.':`eco\tname\tpgn\n${badVolume?'Z':name[0].toUpperCase()}00\tOpening ${name[0]}\t1. e4\n`);
 };return {fetcher,calls};
}
async function fixture(t){const tmp=resolve('tmp');await mkdir(tmp,{recursive:true});const root=await mkdtemp(join(tmp,'opening-update-test-'));t.after(async()=>{assert.ok(root.startsWith(tmp+sep));await rm(root,{recursive:true,force:true});});return root;}
async function snapshot(root){const dir=join(root,'assets','openings');return Object.fromEntries(await Promise.all((await readdir(dir)).sort().map(async name=>[name,await readFile(join(dir,name),'utf8')])));}
test('release update pins one commit, validates before writing and is idempotent',async t=>{
 const root=await fixture(t),remote=upstream();let validated=0;
 const options={root,fetcher:remote.fetcher,validate:async stage=>{validated++;assert.match(await readFile(join(stage,'a.tsv'),'utf8'),/A00/);},now:()=>new Date('2026-10-23T12:00:00Z')};
 const first=await updateOpenings(options);assert.equal(first.changed,true);assert.equal(first.revision,revision);assert.equal(Object.keys(first.files).length,5);assert.equal(remote.calls.filter(url=>url.includes('raw.githubusercontent.com')).length,7);
 const before=await snapshot(root);const second=await updateOpenings({...options,now:()=>new Date('2026-10-24T12:00:00Z')});assert.equal(second.changed,false);assert.equal(validated,2);assert.deepEqual(await snapshot(root),before);
 const pinned=upstream({sha:other});await updateOpenings({...options,fetcher:pinned.fetcher,revision:other});assert.ok(pinned.calls.every(url=>!url.includes('/git/ref/')));assert.equal(JSON.parse((await snapshot(root))['SOURCE.json']).revision,other);
});
test('licence, format, network and move-validation failures preserve the bundled snapshot',async t=>{
 const root=await fixture(t);await updateOpenings({root,...upstream(),validate:async()=>{}});const before=await snapshot(root);
 for(const remote of [upstream({badLicense:true}),upstream({badVolume:true}),upstream({network:true})]){
  await assert.rejects(updateOpenings({root,...remote,validate:async()=>assert.fail('Invalid download must not reach move validation')}));assert.deepEqual(await snapshot(root),before);
 }
 await assert.rejects(updateOpenings({root,...upstream({sha:other}),validate:async()=>{throw Error('Illegal move');}}),/Illegal move/);assert.deepEqual(await snapshot(root),before);
 const altered=upstream();const original=altered.fetcher;altered.fetcher=async url=>url.endsWith('/COPYING.txt')?new Response('CC0 with extra restrictions'):original(url);
 await assert.rejects(updateOpenings({root,...altered,validate:async()=>assert.fail()}),/licence/);assert.deepEqual(await snapshot(root),before);
});
test('opening TSV checks reject wrong columns, empty data, duplicate rows and unexpected shrinkage',()=>{
 const row='A00\tAmar Opening\t1. Nh3\n';assert.equal(validateVolume('eco\tname\tpgn\n'+row,'a'),1);
 for(const input of ['eco\tname\n'+row,'eco\tname\tpgn\n','eco\tname\tpgn\n'+row+row])assert.throws(()=>validateVolume(input,'a'));
 assert.throws(()=>validateVolume('eco\tname\tpgn\n'+row,'a',10),/loss/);
});
