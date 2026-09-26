import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync('web/position-editor/recognition-worker.js','utf8').replace(/^import .*;\r?\n/gm,'').replaceAll('import.meta.url',JSON.stringify('https://example.test/worker.js'));
function harness(fail=false){let decodes=0,loads=0;const messages=[];const context={self:{},URL,Blob,performance,postMessage:m=>messages.push(m),ort:{env:{wasm:{}},InferenceSession:{create:async()=>{loads++;if(fail)throw Error('offline');return {};}}},createImageBitmap:async()=>{decodes++;throw Error('bad image');}};vm.runInNewContext(source,context);return {send:data=>context.self.onmessage({data}),messages,counts:()=>({decodes,loads})};}
test('runtime messages and warmup never decode images',async()=>{const h=harness();await h.send(undefined);await h.send({type:'runtime'});await h.send({});assert.equal(h.counts().loads,0);await h.send({type:'warmup'});assert.equal(h.counts().decodes,0);assert.equal(h.messages[0].type,'ready');});
test('warmup failures remain silent for manual editing',async()=>{const h=harness(true);await h.send({type:'warmup'});assert.equal(h.messages.length,0);});
test('only explicit image scans report decoding failures',async()=>{const h=harness();await h.send({type:'scan',file:new Blob(['bad'],{type:'image/png'})});assert.equal(h.counts().decodes,1);assert.equal(h.messages[0].type,'error');});
