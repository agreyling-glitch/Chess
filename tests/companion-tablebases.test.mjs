import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {once} from 'node:events';
const binary=process.env.COMPANION_BINARY||resolve(`target/debug/ironwood-server${process.platform==='win32'?'.exe':''}`);
test('local Syzygy API: authentication, coverage, corrupt/missing data and replacement', {timeout:15000}, async()=>{
  const root=await mkdtemp(resolve('tmp/companion-tablebases-'));
  const token='e'.repeat(64), origin='http://localhost:8788';
  await writeFile(join(root,'owner-token'),token);
  await writeFile(join(root,'config.json'),JSON.stringify({listen:'127.0.0.1:0',allowed_origins:[origin],token_file:'owner-token',archive:null,tablebase_directories:['.']}));
  const child=spawn(binary,['--config',join(root,'config.json')],{windowsHide:true,stdio:['ignore','pipe','pipe']});
  try {
    const address=await new Promise((yes,no)=>{let data='';const timer=setTimeout(()=>no(Error('Startup timeout')),5000);child.once('error',no);child.stdout.on('data',part=>{data+=part;const match=data.match(/"address":"(127\.0\.0\.1:\d+)"/);if(match){clearTimeout(timer);yes(`http://${match[1]}`);}});});
    const request=async(path,credential=token)=>{const r=await fetch(`${address}/api/v1/${path}`,{headers:{Origin:origin,Authorization:`Bearer ${credential}`,'X-Ironwood-Protocol':'1'}});return {status:r.status,value:await r.json()};};
    assert.equal((await request('tablebases','wrong')).status,401);
    assert.equal((await request('capabilities')).value.services.tablebases,true);
    assert.equal((await request('tablebases')).value.configured,true);
    const probe=fen=>request(`tablebases/probe?fen=${encodeURIComponent(fen)}`);
    const draw=await probe('8/8/8/8/8/2k5/8/K7 w - - 99 1');
    assert.equal(draw.status,200);assert.equal(draw.value.category,'draw');assert.equal(draw.value.source,'companion');assert.ok(draw.value.moves.every(m=>m.category==='draw'));
    assert.equal((await probe('4k3/8/8/8/8/8/8/4K2R w K - 0 1')).value.code,'unsupported_position');
    const fen='8/8/8/8/8/2k5/8/KQ6 w - - 0 1';
    assert.equal((await probe(fen)).value.code,'missing_table');
    // Original invalid fixture, deliberately not tablebase data.
    await writeFile(join(root,'KQvK.rtbw'),Buffer.alloc(80));
    assert.deepEqual((await request('tablebases')).value.materials.KQvK,['rtbw']);
    assert.ok((await request('capabilities')).value.tablebase_coverage.includes('KQvK.rtbw'));
    assert.equal((await probe(fen)).value.code,'corrupt_table');
    await rm(join(root,'KQvK.rtbw'));
    assert.equal((await probe(fen)).value.code,'missing_table');
    const closed=once(child,'close');
    await fetch(`${address}/api/v1/shutdown`,{method:'POST',headers:{Origin:origin,Authorization:`Bearer ${token}`,'X-Ironwood-Protocol':'1'}});await closed;
  } finally {if(child.exitCode===null&&child.signalCode===null){const closed=once(child,'close');child.kill();await closed;}await rm(root,{recursive:true,force:true});}
});
