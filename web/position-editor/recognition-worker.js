import { recognizeGray, extractTiles, rgbaToGray, probsToPlacement, resolveOrientation } from '@scoriiu/fenshot';
import * as ort from 'onnxruntime-web/wasm';
ort.env.wasm.wasmPaths = new URL('./', import.meta.url).href;
ort.env.wasm.numThreads = 1;
let session;
const ready = () => session ??= ort.InferenceSession.create(new URL('./chess-tiles-v2.onnx', import.meta.url).href, { executionProviders: ['wasm'] });
self.onmessage = async ({data}) => {
  // Runtime messages are not image requests. Only explicit scans may decode a file.
  if (data?.type !== 'warmup' && data?.type !== 'scan') return;
  if (data.type === 'scan' && !(data.file instanceof Blob)) {
    postMessage({type:'error',message:'Choose or paste an image to recognize.'});
    return;
  }
  try {
    const start = performance.now();
    const model = await ready();
    if (data.type === 'warmup') { postMessage({type:'ready'}); return; }
    const loaded = performance.now();
    const image = await createImageBitmap(data.file);
    const scale = Math.min(1, 1200 / Math.max(image.width, image.height));
    const canvas = new OffscreenCanvas(Math.round(image.width*scale), Math.round(image.height*scale));
    const context = canvas.getContext('2d', {willReadFrequently:true});
    context.drawImage(image, 0, 0, canvas.width, canvas.height); image.close();
    const gray = rgbaToGray(context.getImageData(0,0,canvas.width,canvas.height).data,canvas.width,canvas.height);
    let inferenceMs = 0;
    const result = await recognizeGray(gray, async corners => {
      const tiles = extractTiles(gray,corners), t = performance.now();
      const output = await model.run({tiles:new ort.Tensor('float32',tiles,[64,1024])});
      inferenceMs += performance.now()-t;
      return probsToPlacement(output.probs.data);
    });
    postMessage({type:'result',result,orientation:result && resolveOrientation(result.placement),timing:{load:loaded-start,total:performance.now()-start,inference:inferenceMs}});
  } catch(error) {
    session = undefined;
    // Optional preloading must not display a recognition failure during manual editing.
    if (data.type === 'scan') postMessage({type:'error',message:error.message});
  }
};
