import {build} from 'esbuild';
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs';
import sharp from 'sharp';
const out='output/fenshot-training';mkdirSync(out,{recursive:true});
await build({stdin:{contents:"export {extractTiles,rgbaToGray} from '@scoriiu/fenshot';",resolveDir:process.cwd()},outfile:`${out}/tiles.mjs`,bundle:true,platform:'node',format:'esm',packages:'bundle',external:['onnxruntime-web/wasm']});
const {extractTiles,rgbaToGray}=await import(`../${out}/tiles.mjs`);
const count=Number(process.argv[2]||200),sets=['cburnett','merida','royal-rascals','undead-court'];
if(!Number.isInteger(count)||count<1||count>20000)throw Error('Board count must be 1–20000');
let seed=12345;const rand=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
const labels='1KQRBNPkqrbnp',chunks=[],lines=[],manifest=[];
for(let n=0;n<count;n++){
 const set=sets[n%4],size=[256,384,512][n%3],tile=size/8,board=Array.from({length:64},()=>rand()<.45?'1':labels[1+Math.floor(rand()*12)]);
 const light=['#dee5d2','#efd6b0','#cccccc'][n%3],dark=['#527763','#976b4c','#777777'][n%3];
 const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}">${board.map((_,i)=>`<rect x="${i%8*tile}" y="${Math.floor(i/8)*tile}" width="${tile}" height="${tile}" fill="${(Math.floor(i/8)+i%8)%2?dark:light}"/>`).join('')}</svg>`;
 const overlays=[];for(let i=0;i<64;i++){const p=board[i];if(p==='1')continue;const input=await sharp(readFileSync(`web/pieces/${set}/${p===p.toUpperCase()?'w':'b'}${p.toUpperCase()}.svg`)).resize(tile,tile).png().toBuffer();overlays.push({input,left:i%8*tile,top:Math.floor(i/8)*tile});}
 const png=await sharp(Buffer.from(svg)).composite(overlays).png().toBuffer();
 const degraded=await sharp(png).jpeg({quality:65+Math.floor(rand()*30)}).toBuffer();
 const {data,info}=await sharp(degraded).ensureAlpha().raw().toBuffer({resolveWithObject:true});
 const tiles=extractTiles(rgbaToGray(data,info.width,info.height),{x0:0,y0:0,x1:size,y1:size});
 chunks.push(Buffer.from(Uint8Array.from(tiles,v=>Math.round(v*255))));
 lines.push(Array.from({length:64},(_,i)=>board[(7-Math.floor(i/8))*8+i%8]).join(''));
 if(n<8)writeFileSync(`${out}/example-${n}-${set}.png`,png);
 manifest.push({board:n,set,size});
}
writeFileSync(`${out}/shard-000.bin`,Buffer.concat(chunks));writeFileSync(`${out}/shard-000.labels`,lines.join('\n')+'\n');writeFileSync(`${out}/manifest.json`,JSON.stringify(manifest,null,2));console.log(`Generated ${count} boards / ${count*64} labeled tiles in ${out}`);
