// Fetch one immutable upstream snapshot, validate it, then replace bundled data.
import {readFile,writeFile,mkdir,mkdtemp,rm} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {resolve,join,sep} from 'node:path';
import {pathToFileURL} from 'node:url';
export const REPOSITORY='https://github.com/lichess-org/chess-openings';
const API='https://api.github.com/repos/lichess-org/chess-openings';
const LICENSE_SHA256='6d489af6292662d9e36d34ce49423784984a5f6e41d7b58f49b01264df59fa03';
const VOLUMES=['a','b','c','d','e'];
const digest=text=>createHash('sha256').update(text).digest('hex');
const normalize=text=>text.replaceAll('\r\n','\n');
export function validateVolume(text,volume,previousRows=0) {
  if(!text.endsWith('\n'))throw Error(`${volume}.tsv: missing final newline`);
  const lines=text.trimEnd().split('\n');
  if(lines.shift()!=='eco\tname\tpgn')throw Error(`${volume}.tsv: unexpected columns`);
  const seen=new Set();
  for(const [index,line] of lines.entries()){
    const fields=line.split('\t');
    if(fields.length!==3||!new RegExp(`^${volume.toUpperCase()}[0-9]{2}$`).test(fields[0])||!fields[1].trim()||!fields[2].trim()||/[\x00-\x08\x0b-\x1f\x7f]/.test(line))throw Error(`${volume}.tsv:${index+2}: invalid opening row`);
    if(seen.has(line))throw Error(`${volume}.tsv:${index+2}: duplicate row`);seen.add(line);
  }
  if(!lines.length||lines.length<previousRows*0.8)throw Error(`${volume}.tsv: unexpected loss of opening data; review upstream before releasing`);
  return lines.length;
}
async function get(url,fetcher,maxBytes=8*1024*1024) {
  const response=await fetcher(url,{headers:{'User-Agent':'Ironwood-opening-release-check','Accept':'application/vnd.github+json'},redirect:'error',signal:AbortSignal.timeout(20000)});
  if(!response.ok)throw Error(`Opening download failed (${response.status}): ${url}`);
  let size=0;const chunks=[];
  for await(const chunk of response.body){size+=chunk.length;if(size>maxBytes)throw Error('Opening response exceeds size limit');chunks.push(chunk);}
  return normalize(new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks)));
}
async function optional(path){try{return await readFile(path,'utf8');}catch(error){if(error.code==='ENOENT')return undefined;throw error;}}
function validateMoves(folder,root) {
  const result=spawnSync('cargo',['run','--quiet','--locked','--bin','validate-openings','--',folder],{cwd:root,stdio:'inherit',windowsHide:true});
  if(result.error)throw result.error;if(result.status!==0)throw Error('Opening move validation failed; bundled data was kept');
}
export async function updateOpenings({root=resolve(import.meta.dirname,'..'),fetcher=fetch,revision,validate=validateMoves,now=()=>new Date()}={}) {
  if(revision&&!/^[a-f0-9]{40}$/.test(revision))throw Error('Use a full 40-character upstream commit');
  const repo=JSON.parse(await get(API,fetcher,128*1024));
  if(repo.license?.spdx_id!=='CC0-1.0')throw Error('Upstream licence changed; review required before releasing');
  if(!revision){const commit=JSON.parse(await get(`${API}/git/ref/heads/${encodeURIComponent(repo.default_branch)}`,fetcher,1024*1024));revision=commit.object?.sha;}
  if(!/^[a-f0-9]{40}$/.test(revision))throw Error('Invalid upstream commit');
  const raw=`https://raw.githubusercontent.com/lichess-org/chess-openings/${revision}`;
  const files=Object.fromEntries(await Promise.all(['COPYING.txt','README.md',...VOLUMES.map(v=>`${v}.tsv`)].map(async name=>[name,await get(`${raw}/${name}`,fetcher)])));
  if(digest(files['COPYING.txt'].trim())!==LICENSE_SHA256||!/the work is released under the CC0 Public Domain Dedication\./.test(files['README.md'].replace(/\s+/g,' ')))throw Error('Dataset licence text changed; bundled data was kept');
  const destination=join(root,'assets','openings'),manifest={format:1,repository:REPOSITORY,revision,license:'CC0-1.0',files:{}};
  for(const volume of VOLUMES){
    const name=`${volume}.tsv`,previous=await optional(join(destination,name));
    const rows=validateVolume(files[name],volume,previous?normalize(previous).trimEnd().split('\n').length-1:0);
    manifest.files[name]={rows,sha256:digest(files[name])};
  }
  const tmpRoot=resolve(root,'tmp');await mkdir(tmpRoot,{recursive:true});const stage=await mkdtemp(join(tmpRoot,'openings-update-'));
  try {
    for(const volume of VOLUMES)await writeFile(join(stage,`${volume}.tsv`),files[`${volume}.tsv`]);
    await validate(stage,root);
    const oldManifest=await optional(join(destination,'SOURCE.json'));
    const sourceBody={...manifest,downloaded_at:now().toISOString()};
    if(oldManifest){const old=JSON.parse(oldManifest);if(old.revision===revision&&JSON.stringify(old.files)===JSON.stringify(manifest.files))sourceBody.downloaded_at=old.downloaded_at;}
    const updates=new Map(VOLUMES.map(volume=>[join(destination,`${volume}.tsv`),files[`${volume}.tsv`]]));
    updates.set(join(destination,'COPYING.txt'),files['COPYING.txt']);
    updates.set(join(root,'web','licenses','chess-openings-cc0.txt'),files['COPYING.txt']);
    updates.set(join(destination,'SOURCE.txt'),`Lichess chess-openings (CC0-1.0)\n${REPOSITORY}\nCommit: ${revision}\nDownloaded: ${sourceBody.downloaded_at.slice(0,10)}\n`);
    updates.set(join(destination,'SOURCE.json'),JSON.stringify(sourceBody,null,2)+'\n');
    const previous=new Map(),written=[];
    try {
      for(const [path,content] of updates){const old=await optional(path);if(old===content)continue;previous.set(path,old);await mkdir(resolve(path,'..'),{recursive:true});written.push(path);await writeFile(path,content);}
    } catch(error){
      for(const path of written.reverse()){const old=previous.get(path);if(old===undefined)await rm(path,{force:true});else await writeFile(path,old);}
      throw error;
    }
    return {...manifest,changed:written.length>0};
  } finally {
    if(!stage.startsWith(tmpRoot+sep))throw Error('Unsafe staging path');
    await rm(stage,{recursive:true,force:true});
  }
}
if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url){
  const args=process.argv.slice(2);if(args.length&&!(args.length===2&&args[0]==='--revision'))throw Error('Usage: node scripts/update-openings.mjs [--revision <commit>]');
  const result=await updateOpenings({revision:args[1]});
  console.log(`Opening dataset ${result.changed?'updated':'verified'} at ${result.revision} (${Object.values(result.files).reduce((sum,file)=>sum+file.rows,0)} lines, CC0-1.0).`);
}
