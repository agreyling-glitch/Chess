import * as ort from 'onnxruntime-web/wasm';
import {readFileSync} from 'node:fs';
ort.env.wasm.numThreads=1;
const session=await ort.InferenceSession.create(new Uint8Array(readFileSync('web/position-editor/vendor/chess-tiles-v2.onnx')),{executionProviders:['wasm']});
const bytes=readFileSync('output/fenshot-training/shard-000.bin').subarray(0,65536);
const result=await session.run({tiles:new ort.Tensor('float32',Float32Array.from(bytes,v=>v/255),[64,1024])});
if(result.probs.dims.join(',')!=='64,13')throw Error('Unexpected model shape');
console.log('ONNX model inference passed: 64 squares, 13 classes');
