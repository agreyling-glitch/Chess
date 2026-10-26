// Browser Files remain the backing store: never read a multi-GB file whole.
import {mountCompanionImportProgress} from './companion-import-progress.js';
const RECORD=56,CHUNK=1024,CACHE=4*1024*1024;
const pause=()=>new Promise(resolve=>setTimeout(resolve,0));

export function importTiming(scanned,total,elapsedMs,state='running'){
  const rate=elapsedMs>0?scanned/(elapsedMs/1000):0;
  const speed=rate>0?`${rate.toLocaleString(undefined,{maximumFractionDigits:1})} games/s`:'— games/s';
  if(state!=='running')return `${speed} · ${state==='complete'?'Complete':state==='stopped'?'Paused':'Interrupted'}`;
  if(!rate)return `${speed} · Estimating time…`;
  const seconds=Math.ceil(Math.max(0,total-scanned)/rate);
  const days=Math.floor(seconds/86400),hours=Math.floor(seconds%86400/3600),minutes=Math.floor(seconds%3600/60);
  const duration=days?`${days}d ${hours}h`:hours?`${hours}h ${minutes}m`:minutes?`${minutes}m ${seconds%60}s`:`${seconds}s`;
  return `${speed} · About ${duration} remaining`;
}

export function installScidWindowFrame(target,dialog,heading){
  let drag;
  const stop=()=>{drag=null;target.removeEventListener('pointermove',move);target.removeEventListener('pointerup',stop);target.removeEventListener('pointercancel',stop);};
  const move=event=>{
    if(!drag)return;
    const dx=event.clientX-drag.x,dy=event.clientY-drag.y;
    const vw=target.innerWidth,vh=target.innerHeight,minW=Math.min(400,vw),minH=Math.min(320,vh);
    let {left,top,width,height}=drag;
    if(drag.edge==='move'){left=Math.max(0,Math.min(vw-width,left+dx));top=Math.max(0,Math.min(vh-height,top+dy));}
    else{
      if(drag.edge.includes('e'))width=Math.max(minW,Math.min(vw-left,width+dx));
      if(drag.edge.includes('s'))height=Math.max(minH,Math.min(vh-top,height+dy));
      if(drag.edge.includes('w')){const right=left+width;left=Math.max(0,Math.min(right-minW,left+dx));width=right-left;}
      if(drag.edge.includes('n')){const bottom=top+height;top=Math.max(0,Math.min(bottom-minH,top+dy));height=bottom-top;}
    }
    Object.assign(dialog.style,{left:`${left}px`,top:`${top}px`,width:`${width}px`,height:`${height}px`});
  };
  const begin=(event,edge)=>{
    if(event.button!==0||event.target.closest('button,input,select,a'))return;
    stop();const rect=dialog.getBoundingClientRect();
    drag={edge,x:event.clientX,y:event.clientY,left:rect.left,top:rect.top,width:rect.width,height:rect.height};
    Object.assign(dialog.style,{position:'fixed',inset:'auto',margin:'0',left:`${rect.left}px`,top:`${rect.top}px`,width:`${rect.width}px`,height:`${rect.height}px`});
    target.addEventListener('pointermove',move);target.addEventListener('pointerup',stop);target.addEventListener('pointercancel',stop);event.preventDefault();
  };
  heading.title='Drag to move';heading.addEventListener('pointerdown',event=>begin(event,'move'));
  dialog.addEventListener('pointerdown',event=>{if(event.target===dialog)begin(event,'move');});
  for(const edge of ['n','s','e','w','ne','nw','se','sw']){
    const grip=dialog.ownerDocument.createElement('div');grip.className=`scid-resize scid-resize-${edge}`;grip.setAttribute('aria-hidden','true');
    grip.addEventListener('pointerdown',event=>begin(event,edge));dialog.append(grip);
  }
  dialog.addEventListener('close',stop,{once:true});
}
async function slice(file,start,end){
  if(ArrayBuffer.isView(file.bytes))return new Uint8Array(file.bytes.subarray(start,end));
  return new Uint8Array(await file.slice(start,end).arrayBuffer());
}
export async function openScid5(files,createReader){
  const parts=new Map();let stem;
  for(const file of files){
    const match=/^(.*)\.(si5|sn5|sg5)$/i.exec(file.name||'');
    if(!match)throw Error('Select only the matching .si5, .sn5 and .sg5 files together.');
    const base=match[1].toLowerCase(),ext=match[2].toLowerCase();
    if(stem!==undefined&&stem!==base||parts.has(ext))throw Error('Choose one matching set of Scid 5 files with the same database name.');
    if(!Number.isSafeInteger(file.size)||file.size<0)throw Error('Invalid Scid file size.');
    stem=base;parts.set(ext,file);
  }
  if(parts.size!==3)throw Error('Select the .si5 index together with its matching .sn5 names and .sg5 games files.');
  const index=parts.get('si5'),names=parts.get('sn5'),games=parts.get('sg5');
  if(!index.size||index.size%RECORD)throw Error('The .si5 index is empty or truncated (records must be 56 bytes).');
  if(names.size>128*1024*1024)throw Error('The Scid name file exceeds 128 MiB.');
  if(games.size>=2**47)throw Error('The .sg5 file exceeds Scid 5’s 47-bit offset range.');
  const reader=createReader(await slice(names,0,names.size));
  let cache=new Uint8Array(),cacheStart=0;
  return {
    count:index.size/RECORD,
    signature:JSON.stringify([...parts.values()].map(f=>[f.name,f.size,f.lastModified??0]).sort()),
    async *entries(first=0,end=this.count){
      if(!Number.isSafeInteger(first)||!Number.isSafeInteger(end)||first<0||end<first||end>this.count)throw Error('Choose a valid game range.');
      for(let n=first;n<end;n+=CHUNK){
        const limit=Math.min(end,n+CHUNK),bytes=await slice(index,n*RECORD,limit*RECORD);
        if(bytes.length!==(limit-n)*RECORD)throw Error('The .si5 index could not be read completely.');
        const entries=JSON.parse(reader.index_entries(bytes));
        for(let i=0;i<entries.length;i++){
          const entry=entries[i];
          if(!entry.deleted&&(!Number.isSafeInteger(entry.offset)||entry.offset<0||entry.length<1||entry.offset+entry.length>games.size))throw Error(`Scid game ${n+i+1} points outside the .sg5 game file.`);
          yield {...entry,number:n+i,record:bytes.subarray(i*RECORD,(i+1)*RECORD)};
        }
        await pause();
      }
    },
    async pgn(entry){
      if(entry.deleted)return '';
      const end=entry.offset+entry.length;
      if(entry.offset<cacheStart||end>cacheStart+cache.length){
        cacheStart=entry.offset;cache=await slice(games,cacheStart,Math.min(games.size,Math.max(end,cacheStart+CACHE)));
      }
      try{return reader.decode_game(entry.record,cache.subarray(entry.offset-cacheStart,end-cacheStart));}
      catch(error){throw Error(`Could not decode Scid game ${entry.number+1}: ${error.message||error}`);}
    },
    free(){cache=new Uint8Array();reader.free();},
  };
}

export async function importScidRange(source,{first=0,end=source.count,player='',batchSize=32,prefetch=false}={},
  {saveBatch,onProgress=()=>{},checkpoint=()=>{},cancelled=()=>false,yieldToUi=pause}={}){
  if(prefetch)return importScidRangeAhead(source,{first,end,player,batchSize},{saveBatch,onProgress,checkpoint,cancelled,yieldToUi});
  let pending=[],through=first;
  if(![32,128].includes(batchSize))throw Error('Choose a supported Scid batch size.');
  const counts={scanned:0,saved:0,duplicates:0,discarded:0,deleted:0,filtered:0,failed:0,firstError:'',next:first,stopped:false};
  const flush=async()=>{
    if(pending.length){const saved=await saveBatch(pending);for(const key of ['saved','duplicates','discarded','failed'])counts[key]+=saved[key]||0;counts.firstError||=saved.firstError||'';if(Number.isSafeInteger(saved.archiveTotal)&&saved.archiveTotal>=0)counts.archiveTotal=saved.archiveTotal;else delete counts.archiveTotal;pending=[];}
    counts.next=through;checkpoint({...counts});onProgress({...counts});await yieldToUi();
  };
  const needle=player.trim().toLowerCase();
  try {
    for await(const entry of source.entries(first,end)){
      if(cancelled()){counts.stopped=true;break;}
      counts.scanned++;through=entry.number+1;
      if(entry.deleted)counts.deleted++;
      else if(needle&&!`${entry.white}\n${entry.black}`.toLowerCase().includes(needle))counts.filtered++;
      else {
        try{pending.push(await source.pgn(entry));}
        catch(error){counts.failed++;counts.firstError||=error.message||String(error);}
      }
      if(pending.length>=batchSize||counts.scanned%1024===0)await flush();
    }
    await flush();return counts;
  } catch(error){
    // A rejected storage batch is atomic. Keep the last durable checkpoint,
    // so restarting never jumps over unsaved games.
    error.progress={...counts};throw error;
  }
}

// One writer and one look-ahead producer. Prepared games are never included in
// committed counters/checkpoints until that specific batch is acknowledged.
async function importScidRangeAhead(source,{first,end,player,batchSize},{saveBatch,onProgress,checkpoint,cancelled,yieldToUi}){
  if(![32,128].includes(batchSize))throw Error('Choose a supported Scid batch size.');
  const iterator=source.entries(first,end)[Symbol.asyncIterator](),needle=player.trim().toLowerCase();
  const counts={scanned:0,saved:0,duplicates:0,discarded:0,deleted:0,filtered:0,failed:0,firstError:'',next:first,stopped:false};
  let done=false,halt=false,through=first,ahead;
  const settled=promise=>promise.then(value=>({value}),error=>({error}));
  const prepare=async()=>{
    const batch={pgns:[],scanned:0,deleted:0,filtered:0,failed:0,firstError:'',next:through};
    while(!done&&!halt&&!cancelled()&&batch.pgns.length<batchSize&&batch.scanned<1024){
      const item=await iterator.next();if(item.done){done=true;break;}
      const entry=item.value;batch.scanned++;through=entry.number+1;batch.next=through;
      if(entry.deleted)batch.deleted++;
      else if(needle&&!`${entry.white}\n${entry.black}`.toLowerCase().includes(needle))batch.filtered++;
      else{try{batch.pgns.push(await source.pgn(entry));}catch(error){batch.failed++;batch.firstError||=error.message||String(error);}}
      // Let HTTP acknowledgements, Stop and rendering run during preparation.
      if(batch.scanned%32===0)await yieldToUi();
    }
    return batch;
  };
  try{
    let current=await prepare();
    while(current.scanned){
      if(cancelled()){counts.stopped=true;break;}
      const saving=settled(current.pgns.length?Promise.resolve().then(()=>saveBatch(current.pgns)):Promise.resolve({}));
      // Start only one following batch, and attach a rejection handler now.
      ahead=settled(prepare());
      const saved=await saving;
      if(saved.error)throw saved.error;
      for(const key of ['scanned','deleted','filtered','failed'])counts[key]+=current[key];
      for(const key of ['saved','duplicates','discarded','failed'])counts[key]+=saved.value[key]||0;
      if(Number.isSafeInteger(saved.value.archiveTotal)&&saved.value.archiveTotal>=0)counts.archiveTotal=saved.value.archiveTotal;else delete counts.archiveTotal;
      counts.firstError||=current.firstError||saved.value.firstError||'';
      counts.next=current.next;checkpoint({...counts});onProgress({...counts});
      if(cancelled()){counts.stopped=true;halt=true;await ahead;break;}
      const prepared=await ahead;ahead=null;if(prepared.error)throw prepared.error;current=prepared.value;
    }
    counts.stopped ||= cancelled();return counts;
  }catch(error){halt=true;error.progress={...counts};throw error;}
  finally{
    halt=true;
    // No file reads may outlive the source's WASM reader after close/error.
    if(ahead)await ahead;
    await iterator.return?.();
  }
}

export async function showScid5Import(target,files){
  if(target.document.getElementById('scid-import-dialog'))throw Error('Close the current Scid import before opening another database.');
  const source=await openScid5(files,names=>target.ironwoodCreateScid5Reader(names));
  const document=target.document,dialog=document.createElement('dialog');dialog.id='scid-import-dialog';
  if(!document.getElementById('scid-import-styles')){const link=document.createElement('link');link.id='scid-import-styles';link.rel='stylesheet';link.href='/import-scid5.css';document.head.append(link);}
  dialog.setAttribute('aria-label','Import Scid database');
  const make=(tag,text)=>{const e=document.createElement(tag);if(text)e.textContent=text;return e;};
  const heading=make('h2','Import Scid database'),description=make('p',`${source.count.toLocaleString('en-US')} games. Files are read locally in small chunks and remain unchanged.`);
  const archiveTotal=make('p');archiveTotal.className='scid-archive-total';archiveTotal.hidden=true;
  const controls=make('div');controls.className='scid-controls';
  const field=(label,type,value)=>{const wrapper=make('label',label),input=make('input');input.type=type;input.value=value;wrapper.append(input);controls.append(wrapper);return input;};
  const first=field('From game','number','1'),end=field('Through game','number',String(source.count)),player=field('Player name contains','search','');
  for(const input of [first,end]){input.min='1';input.max=String(source.count);input.step='1';}
  const resetEmptyRange=()=>{if(!first.value.trim())first.value='1';if(!end.value.trim())end.value=String(source.count);};
  // Treat blank fields as their natural bounds without interrupting typing.
  first.placeholder='1';end.placeholder=String(source.count);
  for(const input of [first,end]){input.addEventListener('change',resetEmptyRange);input.addEventListener('blur',resetEmptyRange);}
  const status=make('p','Choose a range to preview, or import the whole database. Storage use can exceed the source size.');status.setAttribute('aria-live','polite');
  const activity=make('p');activity.className='scid-activity';activity.hidden=true;activity.setAttribute('aria-live','polite');
  const progress=make('progress');progress.max=source.count;progress.value=0;
  const timing=make('output','— games/s · Estimating time…');timing.className='scid-timing';timing.setAttribute('aria-label','Import speed and estimated time remaining');
  timing.title='Average games checked per second in this run, including duplicates, skipped games, waits and retries. Time remaining is an estimate.';
  const progressRow=make('div');progressRow.className='scid-progress-row';progressRow.append(progress,timing);
  const table=make('table'),body=make('tbody'),list=make('div');list.className='scid-game-list';table.append(body);list.append(table);
  const actions=make('div');actions.className='scid-actions';
  const preview=make('button','Preview first 100'),back=make('button','Previous 100'),next=make('button','Next 100'),review=make('button','Review selected games'),start=make('button','Import range'),resume=make('button','Resume last import'),stop=make('button','Stop'),close=make('button','Close');
  const destinationSelect=make('select');destinationSelect.setAttribute('aria-label','Import destination');destinationSelect.className='scid-location-select';
  actions.append(destinationSelect,start,resume,stop);
  const browseActions=make('div');browseActions.className='scid-actions scid-browse-actions';
  browseActions.append(preview,back,next,review);
  close.textContent='×';close.className='scid-close';close.setAttribute('aria-label','Close Scid import');close.title='Close';
  const header=make('header');header.className='scid-heading';header.append(heading,close);
  const note=make('p','Deleted games and games shorter than two full moves are skipped. Existing games are checked for duplicates. Undecodable games are counted as failed; storage failures stop the import. Custom tags, pre-game comments and Scid-specific flags are not imported. Browse or drop .7z archives directly. ZIP archives containing Scid files must be extracted first.');note.className='scid-note';
  const content=make('div');content.className='scid-content';content.append(description,archiveTotal,controls,actions,status,activity,progressRow,list,note);
  dialog.append(header,content,browseActions);
  const folderProgressHost=make('div');content.insertBefore(folderProgressHost,controls);
  const stopFolderProgress=mountCompanionImportProgress(target,folderProgressHost);
  dialog.addEventListener('close',stopFolderProgress,{once:true});
  installScidWindowFrame(target,dialog,header);
  let busy=false,cancelled=false,page=[],pageStart=0,checkpointValue,pageHistory=[];
  let key='ironwood.scid5.resume',archiveEndpoint=null,destinationEpoch=0;
  let archiveCount=null,totalPending=null,lastTotalCheck=0;
  const showArchiveTotal=()=>{archiveTotal.hidden=!archiveEndpoint;archiveTotal.textContent=archiveCount===null?'Archive total unavailable':`${destinationSelect.selectedOptions[0]?.textContent||'Archive'}: ${archiveCount.toLocaleString('en-US')} games`;};
  const refreshArchiveTotal=async(force=false)=>{
    if(!archiveEndpoint||busy&&!force)return;
    if(totalPending){await totalPending;if(force)return refreshArchiveTotal(true);return;}
    const endpoint=archiveEndpoint;
    archiveTotal.hidden=false;if(archiveCount===null)archiveTotal.textContent='Checking Archive total…';
    const task=(async()=>{
      try{const total=await target.ironwoodScidArchiveTotal?.(endpoint);
        if(archiveEndpoint!==endpoint||!dialog.isConnected)return;
        archiveCount=Number.isSafeInteger(total)&&total>=0?total:null;showArchiveTotal();
      }catch{if(archiveEndpoint===endpoint&&dialog.isConnected){archiveCount=null;showArchiveTotal();}}
      finally{lastTotalCheck=Date.now();}
    })();
    totalPending=task;await task;if(totalPending===task)totalPending=null;
  };
  const checkpointKey=(destination,endpoint)=>destination==='archive'?`ironwood.scid5.resume.archive.${encodeURIComponent(endpoint||'')}`:'ironwood.scid5.resume';
  const loadCheckpoint=()=>{
    const candidates=[];
    for(const destination of destinationSelect.value==='browser'?['browser']:archiveEndpoint?['archive']:[]){
      try{const saved=JSON.parse(target.localStorage.getItem(checkpointKey(destination,archiveEndpoint)));
        if(saved?.signature===source.signature&&(saved.destination||'browser')===destination&&(destination!=='archive'||saved.endpoint===archiveEndpoint)&&saved.next<saved.end)candidates.push(saved);
      }catch{}
    }
    checkpointValue=candidates.sort((a,b)=>(b.updatedAt||0)-(a.updatedAt||0))[0]||null;
    resume.textContent=checkpointValue?`Resume ${(checkpointValue.destination||'browser')==='archive'?(destinationSelect.selectedOptions[0]?.textContent||'Archive'):'Browser'} import`:'Resume last import';
    resume.disabled=busy||!checkpointValue||checkpointValue.next>=checkpointValue.end;
  };
  loadCheckpoint();
  const refreshDestination=async()=>{
    if(busy)return;const epoch=++destinationEpoch;
    let catalog;try{catalog=JSON.parse(target.ironwoodImportLocationCatalog?.()||'{"selected":"browser","locations":[{"value":"browser","label":"Browser","enabled":true}]}');}catch{return;}
    const selected=destinationSelect.value||catalog.selected||'browser';destinationSelect.replaceChildren();
    for(const location of catalog.locations||[]){const option=make('option',location.label);option.value=location.value;option.disabled=location.enabled===false;destinationSelect.append(option);}
    if(![...destinationSelect.options].some(option=>option.value===selected)){const option=make('option','Unavailable Companion');option.value=selected;option.disabled=true;destinationSelect.append(option);}
    destinationSelect.value=selected;const previous=archiveEndpoint;start.disabled=selected!=='browser';
    const available=selected==='browser'?null:await target.ironwoodScidArchiveDestination?.(selected)||null;
    if(epoch!==destinationEpoch||busy)return;archiveEndpoint=available;start.disabled=selected!=='browser'&&!available;
    if(previous!==archiveEndpoint){archiveCount=null;showArchiveTotal();}loadCheckpoint();
    if(archiveEndpoint&&(previous!==archiveEndpoint||Date.now()-lastTotalCheck>=30000))void refreshArchiveTotal();
  };
  destinationSelect.onchange=()=>{target.ironwoodSetImportLocation?.(destinationSelect.value);archiveEndpoint=null;archiveCount=null;showArchiveTotal();void refreshDestination();};
  void refreshDestination();const destinationTimer=target.setInterval(()=>void refreshDestination(),5000);
  target.addEventListener('ironwood-companion-connections-changed',refreshDestination);
  target.addEventListener('ironwood-companion-feature-changed',refreshDestination);
  const setBusy=value=>{busy=value;for(const button of [preview,back,next,review,start,resume])button.disabled=value;for(const input of [first,end,player,destinationSelect])input.disabled=value;start.disabled=value||(destinationSelect.value!=='browser'&&!archiveEndpoint);stop.disabled=!value;close.disabled=value;};
  const range=()=>{resetEmptyRange();const a=Number(first.value)-1,b=Number(end.value);if(!Number.isSafeInteger(a)||!Number.isSafeInteger(b)||a<0||b<=a||b>source.count)throw Error('Choose a valid inclusive game range.');return {first:a,end:b,player:player.value};};
  const report=error=>{status.textContent=error.message||String(error);};
  const showPage=async offset=>{
    setBusy(true);cancelled=false;body.replaceChildren();page=[];
    try{
      const options=range();pageStart=Math.max(options.first,offset);let scanned=0;
      for await(const entry of source.entries(pageStart,options.end)){
        if(cancelled)break;scanned++;
        if(scanned%1024===0){status.textContent=`Searching game ${(entry.number+1).toLocaleString('en-US')}…`;await pause();}
        if(entry.deleted||options.player.trim()&&!`${entry.white}\n${entry.black}`.toLowerCase().includes(options.player.trim().toLowerCase()))continue;
        page.push(entry);const row=make('tr'),cell=make('td'),checkbox=make('input');checkbox.type='checkbox';checkbox.setAttribute('aria-label',`Select game ${entry.number+1}`);entry.checkbox=checkbox;cell.append(checkbox);row.append(cell);
        for(const text of [(entry.number+1).toLocaleString('en-US'),entry.white,entry.black,entry.date,['*','1–0','0–1','½–½'][entry.result]])row.append(make('td',String(text)));
        body.append(row);if(page.length===100)break;
      }
      status.textContent=`Showing ${page.length} games. Select games to use the usual import review, or import the chosen range directly.`;
    }catch(error){report(error);}finally{setBusy(false);resume.disabled=!checkpointValue;}
  };
  preview.onclick=()=>{resetEmptyRange();pageHistory=[];void showPage(Number(first.value)-1);};
  next.onclick=()=>{pageHistory.push(pageStart);void showPage(page.length?page.at(-1).number+1:pageStart+100);};
  back.onclick=()=>void showPage(pageHistory.pop()??Number(first.value)-1);
  stop.onclick=()=>{cancelled=true;status.textContent='Stopping after the current storage batch…';};
  close.onclick=()=>dialog.close();
  dialog.addEventListener('cancel',event=>{if(busy){event.preventDefault();stop.click();}});
  dialog.addEventListener('close',()=>{target.clearInterval(destinationTimer);target.removeEventListener('ironwood-companion-connections-changed',refreshDestination);target.removeEventListener('ironwood-companion-feature-changed',refreshDestination);source.free();dialog.remove();},{once:true});
  review.onclick=async()=>{
    const selected=page.filter(e=>e.checkbox.checked);if(!selected.length){status.textContent='Select at least one game to review.';return;}
    setBusy(true);
    try{const texts=[];for(const entry of selected)texts.push(await source.pgn(entry));target.ironwoodScidSelectionReady(texts.join('\n\n'));dialog.close();}
    catch(error){report(error);setBusy(false);}
  };
  const run=async(destination,saved)=>{
    let options;try{options={...(saved?{first:saved.next,end:saved.end,player:saved.player}:range()),destination,endpoint:destination==='archive'?archiveEndpoint:null};
      if(options.destination==='archive'&&!options.endpoint)throw Error('Reconnect to Companion before importing to Archive.');
      if(saved&&destination==='archive'&&saved.endpoint!==options.endpoint)throw Error('Reconnect to the original archive before resuming.');
    }catch(error){report(error);return;}
    key=checkpointKey(options.destination,options.endpoint);checkpointValue=saved||null;
    setBusy(true);cancelled=false;progress.max=options.end-options.first;progress.value=0;
    const startedAt=target.performance.now();let scanned=0,runState='running';
    let archiveBaseline=null;
    const updateTiming=()=>{timing.textContent=importTiming(scanned,options.end-options.first,target.performance.now()-startedAt,runState);};
    updateTiming();const timingTimer=target.setInterval(updateTiming,1000);
    const onStatus=message=>{activity.textContent=message||'Saving games to Archive…';activity.hidden=false;};
    onStatus('Preparing import…');options.batchSize=options.destination==='archive'?128:32;options.prefetch=options.destination==='archive';
    let checkpointAt=0;
    try{
      if(typeof target.ironwoodImportScidBatch!=='function')throw Error('Game storage is still loading. Try again shortly.');
      target.ironwoodBeginScidImport?.();
      if(options.destination==='archive'){await refreshArchiveTotal(true);archiveBaseline=archiveCount;}
      if(options.destination==='browser'){
        const estimate=await target.navigator.storage?.estimate?.();
        if(estimate?.quota&&estimate.quota-estimate.usage<32*1024*1024)throw Error('Browser storage has less than 32 MiB free. Free space before importing.');
        await target.navigator.storage?.persist?.();
      }
      const counts=await importScidRange(source,options,{
        saveBatch:pgns=>target.ironwoodImportScidBatch(pgns,{destination:options.destination,endpoint:options.endpoint,onStatus}),cancelled:()=>cancelled,
        checkpoint:state=>{
          checkpointValue={signature:source.signature,end:options.end,player:options.player,destination:options.destination,endpoint:options.endpoint,updatedAt:Date.now(),...state};
          if(Date.now()-checkpointAt>1000){try{target.localStorage.setItem(key,JSON.stringify(checkpointValue));}catch{}checkpointAt=Date.now();}
        },
        onProgress:state=>{scanned=state.scanned;progress.value=state.scanned;updateTiming();
          if(options.destination==='archive'){
            if(Number.isSafeInteger(state.archiveTotal))archiveBaseline=state.archiveTotal-state.saved;
            if(archiveBaseline!==null){archiveCount=archiveBaseline+state.saved;showArchiveTotal();}
          }
          status.textContent=`Read ${state.scanned.toLocaleString('en-US')} · Saved ${state.saved.toLocaleString('en-US')} · Duplicates ${state.duplicates.toLocaleString('en-US')} · Short ${state.discarded.toLocaleString('en-US')} · Deleted ${state.deleted.toLocaleString('en-US')} · Failed ${state.failed.toLocaleString('en-US')}${state.firstError?' · First error: '+state.firstError:''}`;},
      });
      runState=counts.stopped?'stopped':'complete';
      status.textContent+=counts.stopped?' · Stopped. Resume is available.':' · Complete.';
      if(!counts.stopped){checkpointValue=null;try{target.localStorage.removeItem(key);}catch{}}
    }catch(error){runState='interrupted';report(error);status.textContent+=' Previously committed batches are kept. Resume starts after the last saved batch.';}
    finally{target.clearInterval(timingTimer);updateTiming();activity.hidden=true;if(checkpointValue){try{target.localStorage.setItem(key,JSON.stringify(checkpointValue));}catch{}}target.ironwoodFinishScidImport?.(options.destination);setBusy(false);loadCheckpoint();void refreshArchiveTotal();}
  };
  start.onclick=()=>void run(destinationSelect.value==='browser'?'browser':'archive');resume.onclick=()=>{if(checkpointValue)void run(checkpointValue.destination||'browser',checkpointValue);};
  document.body.append(dialog);dialog.showModal();setBusy(false);resume.disabled=!checkpointValue;
  status.textContent=`${source.count.toLocaleString('en-US')} games ready. Preview or import a range. Imports can be stopped and resumed after selecting the same files again.`;
  return dialog;
}
