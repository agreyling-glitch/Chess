import test from 'node:test';
import assert from 'node:assert/strict';
import { JSDOM } from 'jsdom';
const dom = new JSDOM('<body></body>', {url:'http://localhost/play/'});
globalThis.window = dom.window;
const {openHistoryBackup} = await import('../web/history-backup.js');

test('backup window names each Companion and restores only to the selected endpoint', async () => {
  const target = dom.window;
  target.HTMLDialogElement.prototype.showModal = function() { this.open = true; };
  target.HTMLDialogElement.prototype.close = function() { this.dispatchEvent(new target.Event('close')); };
  const entries = [
    {name:'Office',endpoint:'http://office:8790',credential:'office-secret',connected:true},
    {name:'Study',endpoint:'http://study:8790',credential:'study-secret',connected:true},
    {name:'Offline',endpoint:'http://offline:8790',connected:false},
  ];
  const calls = [];
  class Client {
    constructor(endpoint, credential) { this.endpoint=endpoint;this.credential=credential; }
    async request(path, body) {
      calls.push({endpoint:this.endpoint,credential:this.credential,path,body});
      return body ? {started:true} : {folder:'/nas/backups',files:[{name:'ironwood-123-abcd.sqlite3',bytes:90000000000}],job:{state:'idle'}};
    }
  }
  const dialog = openHistoryBackup(target,{connections:()=>entries,subscribe:()=>()=>{},Client});
  const [action,location] = dialog.querySelectorAll('select');
  assert.deepEqual([...location.options].map(option=>option.textContent),['Browser','Office','Study','Offline (disconnected)']);
  assert.ok(location.options[3].disabled);
  assert.ok(!dialog.textContent.includes('secret'));
  action.value='restore';location.value='http://study:8790';location.onchange();
  await new Promise(resolve=>setImmediate(resolve));
  const file=dialog.querySelector('select:last-of-type');
  const run=[...dialog.querySelectorAll('button')].at(-1);
  assert.ok(!run.disabled);
  await run.onclick();
  const writes=calls.filter(call=>call.body);
  assert.equal(writes.length,1);assert.equal(writes[0].endpoint,'http://study:8790');assert.equal(writes[0].credential,'study-secret');assert.equal(writes[0].path,'archive/backups');
  assert.deepEqual(writes[0].body,{action:'restore',file:'ironwood-123-abcd.sqlite3'});
  assert.ok(!calls.some(call=>call.path==='archive/restore'||call.path==='archive/backup'));
  assert.match(dialog.textContent,/Backup folder on Study: \/nas\/backups/);
  let browserRestore=0;target.ironwoodRestoreStorage=()=>browserRestore++;
  location.value='browser';location.onchange();await run.onclick();
  assert.equal(browserRestore,1);assert.equal(calls.filter(call=>call.body).length,1);
});
