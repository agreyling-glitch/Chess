import { build } from 'esbuild';
import { cpSync, mkdirSync } from 'node:fs';
const out = 'web/position-editor/vendor';
mkdirSync(out, { recursive: true });
await build({ entryPoints: ['web/position-editor/recognition-worker.js'], outfile: `${out}/recognition-worker.js`, bundle: true, format: 'esm', platform: 'browser', minify: true });
cpSync('node_modules/@scoriiu/fenshot/model/chess-tiles-v2.onnx', `${out}/chess-tiles-v2.onnx`);
for (const name of ['ort-wasm-simd-threaded.mjs', 'ort-wasm-simd-threaded.wasm']) cpSync(`node_modules/onnxruntime-web/dist/${name}`, `${out}/${name}`);
cpSync('node_modules/@scoriiu/fenshot/LICENSE', `${out}/FENSHOT-LICENSE`);
cpSync('web/licenses/onnxruntime-mit.txt', `${out}/ONNX-LICENSE`);
