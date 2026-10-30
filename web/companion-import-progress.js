import {companionConnection} from './companion-connection.js';
import {CompanionArchiveClient} from './companion-archive-client.js';

const number=value=>Math.max(0,Number(value)||0).toLocaleString('en-US',{maximumFractionDigits:0});
function duration(value){const seconds=Math.max(0,Math.ceil(value));return seconds>=86400?`${Math.floor(seconds/86400)}d ${Math.floor(seconds%86400/3600)}h`:seconds>=3600?`${Math.floor(seconds/3600)}h ${Math.floor(seconds%3600/60)}m`:seconds>=60?`${Math.floor(seconds/60)}m ${seconds%60}s`:`${seconds}s`;}
export function folderImportView(value){
  const s=value.stats||{},processed=Number(value.processed)||0,total=value.total_games;
  const rate=Number(value.games_per_second)||0,phase=value.phase||'idle';
  const speed=rate>0?`${rate.toLocaleString('en-US',{maximumFractionDigits:1})} games/s`:'— games/s';
  const remaining=value.eta_seconds;
  const eta=phase==='complete'?'Complete':phase==='failed'?'Needs attention':phase==='waiting'?'Waiting — estimate paused':phase==='unpacking'?'Unpacking — estimate available when importing':phase==='resuming'?'Resuming from the last saved batch…':phase==='counting'||phase==='preparing'?'Counting games…':phase==='verifying'?'Finishing source verification':Number.isFinite(remaining)?`About ${duration(remaining)} remaining`:value.active?'Estimating time…':'Waiting for files';
  return {filename:value.filename||'',phase,message:value.message||'Waiting for files in Companion’s Import folder',counts:`Read ${number(processed)}${Number.isFinite(total)?` of ${number(total)}`:''} · Saved ${number(s.imported)} · Duplicates ${number(s.duplicates)} · Short ${number(s.discarded)} · Deleted ${number(s.deleted)} · Failed ${number(s.failed)}`,timing:`${speed} · ${eta}`,fraction:Number.isFinite(total)&&total>0?Math.min(1,processed/total):null,active:!!value.active};
}

export function installCompanionImportProgress(target,connectionProvider=companionConnection){
  let cached=null,key=null,client=null,pending=null,last=0,revision=0;
  target.ironwoodCompanionImportProgress=()=>{
    const connection=target.ironwoodCompanionFeatureEnabled?.()===true?connectionProvider():null;
    if(!connection){revision++;cached=null;key=null;client=null;return '';}
    const next=connection.endpoint+'\n'+connection.credential;
    if(next!==key){key=next;revision++;cached=null;last=0;client=new CompanionArchiveClient(connection.endpoint,connection.credential);}
    if(!pending&&!target.document.hidden&&Date.now()-last>=2000){
      const epoch=revision,current=client;
      pending=current.request('archive/import-progress').then(value=>{if(epoch===revision)cached=folderImportView(value);}).catch(error=>{
        if(epoch===revision)cached={filename:'',counts:'',fraction:null,active:false,phase:'unavailable',message:error.status===404?'Update Companion to show Import folder progress.':'Folder progress unavailable — reconnecting…',timing:''};
        if(epoch===revision&&error.status===404)last=Date.now()+28000;
      }).finally(()=>{pending=null;last=Math.max(last,Date.now());});
    }
    return cached?JSON.stringify(cached):'';
  };
}

export function mountCompanionImportProgress(target,parent){
  if(!target.ironwoodCompanionImportProgress)installCompanionImportProgress(target);
  const d=target.document,box=d.createElement('section');box.className='companion-folder-progress';box.hidden=true;
  const title=d.createElement('h3');title.textContent='Companion Import folder';
  const name=d.createElement('strong'),status=d.createElement('p'),counts=d.createElement('p'),row=d.createElement('div'),bar=d.createElement('progress'),timing=d.createElement('output');
  status.setAttribute('role','status');bar.max=1;bar.setAttribute('aria-label','Companion folder import progress');timing.setAttribute('aria-label','Folder import speed and estimated time remaining');row.append(bar,timing);box.append(title,name,status,counts,row);parent.append(box);
  if(!d.getElementById('companion-folder-progress-style')){const style=d.createElement('style');style.id='companion-folder-progress-style';style.textContent='.companion-folder-progress{padding:14px;margin:12px 0;background:#10151a;color:#ecebe4;border:1px solid #816b42;border-radius:7px}.companion-folder-progress[hidden]{display:none!important}.companion-folder-progress h3{color:#f1ce83;margin:0 0 8px}.companion-folder-progress p{margin:8px 0}.companion-folder-progress>div{display:flex;align-items:center;gap:14px;flex-wrap:wrap}.companion-folder-progress progress{height:28px;flex:1;min-width:160px;accent-color:#d3ad62}.companion-folder-progress output{color:#eeddbb}.companion-folder-progress progress::-webkit-progress-bar{background:#10151a;border:1px solid #816b42;border-radius:5px;box-shadow:inset 0 2px 5px #000}.companion-folder-progress progress::-webkit-progress-value{background:linear-gradient(#f4dfab,#c69a4c 52%,#a87a31);border-radius:4px}';d.head.append(style);}
  let timer,stopped=false;
  const tick=()=>{
    if(stopped||!box.isConnected)return;
    const raw=target.ironwoodCompanionImportProgress();box.hidden=!raw;
    if(raw){const value=JSON.parse(raw);name.textContent=value.filename;status.textContent=value.message;counts.textContent=value.counts;timing.textContent=value.timing;if(value.fraction===null)bar.removeAttribute('value');else bar.value=value.fraction;}
    timer=target.setTimeout(tick,1000);
  };
  timer=target.setTimeout(tick,0);
  return ()=>{stopped=true;target.clearTimeout(timer);box.remove();};
}
