import {companionConnection} from './companion-connection.js';
import {CompanionArchiveClient} from './companion-archive-client.js';

const count=new Intl.NumberFormat('en-US');
function duration(seconds){const n=Math.ceil(seconds);return n>=86400?`${Math.floor(n/86400)}d ${Math.floor(n%86400/3600)}h`:n>=3600?`${Math.floor(n/3600)}h ${Math.floor(n%3600/60)}m`:n>=60?`${Math.floor(n/60)}m ${n%60}s`:`${n}s`;}
export function createCompanionIndexProgress({connection=companionConnection,enabled=()=>window.ironwoodCompanionFeatureEnabled?.()===true,visible=()=>!window.document.hidden,client=value=>new CompanionArchiveClient(value.endpoint,value.credential),clock=Date.now}={}){
  let key='',generation=0,state={},samples=[],pending=null,last=-Infinity,interval=2000;
  function reset(){generation++;key='';state={};samples=[];pending=null;last=-Infinity;interval=2000;}
  function current(){
    const value=enabled()?connection():null;
    if(!value){if(key)reset();return null;}
    const next=value.endpoint+'\n'+value.credential;
    if(next!==key){reset();key=next;}
    return value;
  }
  async function refresh(){
    const value=current();if(!value||!visible()||pending)return;
    const epoch=generation;last=clock();
    const task=(async()=>{
      try {
        const result=await client(value).request('archive/explorer/status');
        if(!current()||epoch!==generation)return;
        const valid=n=>Number.isSafeInteger(n)&&n>=0;
        if(![result.indexed,result.pending,result.total].every(valid))throw Error('Invalid index progress');
        const now=clock(),previous=samples.at(-1);
        if(previous&&(result.indexed<previous.indexed||result.generation<previous.generation||result.pending>0&&state.remaining===0))samples=[];
        samples.push({at:now,indexed:result.indexed,generation:result.generation});
        while(samples.length>2&&samples[1].at<now-30000)samples.shift();
        const first=samples[0],elapsed=(now-first.at)/1000;
        const speed=elapsed>0?Math.max(0,result.indexed-first.indexed)/elapsed:0;
        const failed=valid(result.failed)?result.failed:0,unsupported=valid(result.unsupported_variants)?result.unsupported_variants:0;
        const finished=result.pending===0,eta=!finished&&speed>0?Math.ceil(result.pending/speed):null;
        const rate=speed>0?`${count.format(Math.round(speed*10)/10)} games/s`:'— games/s';
        state={available:true,indexed:result.indexed,total:result.total,remaining:result.pending,speed,eta_seconds:eta,failed,unsupported,
          counts:`Opening index: ${count.format(result.indexed)} indexed · ${count.format(result.pending)} remaining`,
          timing:`${rate} · ${finished?(failed?'Finished — needs attention':'Up to date'):eta!==null?`About ${duration(eta)} remaining`:'Estimating time…'}`,
          detail:`${count.format(result.indexed)} of ${count.format(result.total)} saved games indexed. ${count.format(failed)} indexing failures; ${count.format(unsupported)} unsupported variants. Speed is measured over recent updates. The estimate covers the current backlog; continuing imports can add more games.`};
        // Older Companions calculate totals with a full query. Poll those less
        // often; the versioned counter endpoint is cheap enough for live updates.
        interval=result.progress_version===1?2000:30000;
      }catch(error){if(current()&&epoch===generation){const busy=error.code==='busy'||[429,503].includes(error.status);state={...state,available:true,speed:0,eta_seconds:null,counts:busy?'Opening index: waiting for archive':'Opening index unavailable',timing:busy?'Retrying…':'Reconnecting…',detail:error.status===404?'Update Companion to show opening-index progress.':busy?'Waiting for archive capacity; progress will refresh automatically.':'Waiting for the Companion connection to return.'};samples=[];interval=Math.max(5000,Math.min(60000,error.retryAfterMs||0));}}
    })();
    pending=task;
    try{await task;}finally{if(pending===task)pending=null;}
  }
  function snapshot(){if(!current())return '{}';if(!pending&&visible()&&clock()-last>=interval)void refresh();return JSON.stringify(state);}
  return {snapshot,refresh,reset};
}
if(typeof window!=='undefined'){
  const progress=createCompanionIndexProgress();
  window.ironwoodCompanionIndexProgress=progress.snapshot;
}
