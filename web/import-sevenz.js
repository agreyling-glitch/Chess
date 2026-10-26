const MAX=64*1024*1024*1024,PGN_MAX=64*1024*1024;
export function planSevenZip(manifest){
  if(!manifest||!Array.isArray(manifest.entries)||manifest.entries.length>1000||!Number.isSafeInteger(manifest.total)||manifest.total<0||manifest.total>MAX)throw Error('Invalid or oversized 7z archive.');
  const names=new Set(),files=[];let total=0;
  for(const entry of manifest.entries){
    const name=entry.name?.replaceAll('\\','/');
    if(!name||name.startsWith('/')||name.includes(':')||/[\x00-\x1f\x7f]/.test(name)||name.split('/').some(s=>s==='..'||s==='.')||names.has(name.toLowerCase()))throw Error('The 7z contains unsafe or duplicate file paths.');
    names.add(name.toLowerCase());if(!Number.isSafeInteger(entry.size)||entry.size<0||(total+=entry.size)>MAX)throw Error('The 7z expands beyond the supported size.');
    if(!entry.directory&&/\.(pgn|s[ing][45])$/i.test(name))files.push({...entry,name});
  }
  if(total!==manifest.total)throw Error('Invalid 7z expanded size.');
  if(!files.length)throw Error('The 7z contains no PGN or Scid 4/5 files.');
  const scid=files.filter(f=>!/\.pgn$/i.test(f.name)),pgn=files.filter(f=>/\.pgn$/i.test(f.name));
  if(scid.length){
    if(pgn.length)throw Error('This 7z mixes PGN and Scid databases. Extract it first and choose which database to import.');
    const base=scid[0].name.replace(/\.s[ing][45]$/i,'').toLowerCase(),version=scid[0].name.at(-1);
    if(scid.length!==3||scid.some(f=>f.name.replace(/\.s[ing][45]$/i,'').toLowerCase()!==base||f.name.at(-1)!==version)||!['si','sn','sg'].every(ext=>scid.some(f=>f.name.toLowerCase().endsWith(`.${ext}${version}`))))throw Error('The 7z must contain one complete matching Scid database (.si, .sn and .sg files).');
    if(scid.some(f=>/\.sn[45]$/i.test(f.name)&&f.size>128*1024*1024))throw Error('The Scid name file exceeds 128 MiB.');
    if(version==='4'&&scid.reduce((n,f)=>n+f.size,0)>PGN_MAX)throw Error('This Scid 4 database exceeds the browser import limit. Use the Companion Import folder.');
  }else if(pgn.reduce((n,f)=>n+f.size,0)>PGN_MAX)throw Error('The PGNs inside this 7z exceed 64 MiB. Use the Companion Import folder for larger PGNs.');
  return {kind:scid.length?`scid${scid[0].name.at(-1)}`:'pgn',files,bytes:files.reduce((n,f)=>n+f.size,0)};
}
export async function extractSevenZip(file,{target=window,onProgress=()=>{},onStatus=()=>{},signal,workerFactory=()=>new target.Worker('/import-sevenz-worker.js',{type:'module'})}={}){
  if(!file||!Number.isSafeInteger(file.size)||file.size<32||file.size>MAX)throw Error('Choose a .7z archive no larger than 64 GiB.');
  if(!target.navigator.storage?.getDirectory)throw Error('Temporary browser storage is unavailable. Extract the .7z first or use the Companion Import folder.');
  const parent=await (await target.navigator.storage.getDirectory()).getDirectoryHandle('ironwood-sevenz-imports',{create:true});
  const session=target.crypto.randomUUID(),directory=await parent.getDirectoryHandle(session,{create:true});
  let cleaned=false,worker;
  const cleanup=async()=>{if(cleaned)return;for(let attempt=0;;attempt++){try{await parent.removeEntry(session,{recursive:true});cleaned=true;target.removeEventListener?.('pagehide',leave);return;}catch(error){if(attempt>=2)throw error;await new Promise(resolve=>setTimeout(resolve,50));}}};
  const leave=event=>{if(!event.persisted){worker?.terminate();void cleanup().catch(()=>{});}};
  target.addEventListener?.('pagehide',leave);
  try{
    worker=workerFactory();
    const plan=await new Promise((resolve,reject)=>{
      const abort=()=>{worker.terminate();reject(Error('7z import cancelled.'));};
      const finish=(error,value)=>{signal?.removeEventListener('abort',abort);error?reject(error):resolve(value);};
      worker.onerror=event=>finish(Error(event.message||'Could not decode 7z archive.'));
      worker.onmessage=async event=>{const message=event.data;try{
        if(message.type==='manifest'){
          const plan=planSevenZip(message.manifest),estimate=await target.navigator.storage.estimate?.();
          if(estimate?.quota&&plan.bytes>Math.max(0,estimate.quota-(estimate.usage||0)-16*1024*1024))throw Error('Not enough temporary browser storage to extract this 7z. Use the Companion Import folder.');
          worker.postMessage({type:'extract',plan});
        }else if(message.type==='stage')onStatus(message.message);
        else if(message.type==='progress')onProgress(message.done,message.total);
        else if(message.type==='ready')finish(null,message.plan);
        else if(message.type==='error')finish(Error(message.message));
      }catch(error){finish(error);}};
      if(signal?.aborted){abort();return;}signal?.addEventListener('abort',abort,{once:true});worker.postMessage({type:'open',file,directory});
    });
    worker.terminate();worker=null;
    if(signal?.aborted)throw Error('7z import cancelled.');
    const files=[];for(let i=0;i<plan.files.length;i++){const staged=await (await directory.getFileHandle(`entry-${i}`)).getFile();files.push(new target.File([staged],plan.files[i].name,{lastModified:0}));}
    return {kind:plan.kind,files,cleanup};
  }catch(error){worker?.terminate();await cleanup().catch(()=>{});throw error;}
}
export async function openSevenZip(target,file,{onProgress=()=>{},signal,readScid=files=>target.ironwoodReadScidFiles(files)}={}){
  const document=target.document,dialog=document.createElement('dialog');dialog.id='sevenz-import-progress';
  dialog.style.cssText='background:#191d20;color:#e1d6b5;border:3px ridge #c5a45a;border-radius:8px;padding:24px;max-width:85vw;min-width:320px';
  const title=document.createElement('h3');title.textContent='Opening .7z archive';const status=document.createElement('p');status.textContent='Preparing temporary browser storage…';const bar=document.createElement('progress');bar.style.cssText='width:100%;height:20px;accent-color:#c5a45a';
  const cancel=document.createElement('button');cancel.textContent='Cancel';const controller=new target.AbortController();cancel.onclick=()=>controller.abort();dialog.addEventListener('cancel',event=>{event.preventDefault();controller.abort();});
  const abort=()=>controller.abort();signal?.addEventListener('abort',abort,{once:true});if(signal?.aborted)abort();
  dialog.append(title,status,bar,cancel);document.body.append(dialog);dialog.showModal();
  let extracted,leased=false,keepError=false;
  try{
    extracted=await extractSevenZip(file,{target,signal:controller.signal,onStatus:message=>{status.textContent=message;onProgress(message);},onProgress:(done,total)=>{bar.max=total||1;bar.value=done;const text=`Extracting ${Math.floor(done/1024/1024).toLocaleString('en-US')} of ${Math.ceil(total/1024/1024).toLocaleString('en-US')} MiB…`;status.textContent=text;onProgress(text);}});
    dialog.close();dialog.remove();
    if(extracted.kind==='scid5'){
      const scid=await target.ironwoodOpenScid5Files(extracted.files);scid.addEventListener('close',()=>void extracted.cleanup().catch(()=>{}),{once:true});leased=true;return null;
    }
    if(extracted.kind==='scid4')return await readScid(extracted.files);
    return (await Promise.all(extracted.files.map(file=>file.text()))).join('\n\n');
  }catch(error){
    if(!controller.signal.aborted){
      keepError=true;title.textContent='Could not open .7z archive';status.setAttribute('role','alert');status.textContent=error.message||String(error);bar.hidden=true;cancel.textContent='Close';
      const dismiss=()=>{dialog.close();dialog.remove();};cancel.onclick=dismiss;dialog.addEventListener('cancel',dismiss,{once:true});
      if(!dialog.isConnected){document.body.append(dialog);dialog.showModal();}
    }
    throw error;
  }finally{
    signal?.removeEventListener('abort',abort);if(!keepError){dialog.close();dialog.remove();}
    if(extracted&&!leased)await extracted.cleanup();
  }
}
