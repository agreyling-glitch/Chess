const bytes=value=>`${(value/1024/1024).toLocaleString('en-US',{maximumFractionDigits:1})} MiB`;
function duration(seconds){const n=Math.ceil(seconds);return n>=3600?`${Math.floor(n/3600)}h ${Math.floor(n%3600/60)}m`:n>=60?`${Math.floor(n/60)}m ${n%60}s`:`${n}s`;}
export function downloadProgress(job){
 const amount=`${bytes(job.downloaded||0)} of ${bytes(job.total||0)}`;
 const timing=job.phase==='downloading'&&job.speed>0?` · ${bytes(job.speed)}/s${Number.isFinite(job.eta_seconds)?` · About ${duration(job.eta_seconds)} remaining`:''}`:'';
 return `${job.message||job.phase} · ${amount}${timing}`;
}
export function mountEngineDownloads(target,container,client,{isCurrent=()=>true,onInstalled=()=>{}}={}){
 if(target.ironwoodCompanionFeatureEnabled?.()!==true)return ()=>{};
 const document=target.document,section=document.createElement('section');section.dataset.engineDownloads='';
 const heading=document.createElement('h3');heading.textContent='Download an engine';const note=document.createElement('p');note.textContent='Companion downloads directly from the publisher, verifies the package and installs it. Downloads continue if this window closes.';
 const content=document.createElement('div'),status=document.createElement('p');status.setAttribute('role','status');status.textContent='Reading available downloads…';section.append(heading,note,content,status);container.prepend(section);
 let alive=true,timer,packages=[],working=false,initial=true;const seen=new Set();
 const current=()=>alive&&target.ironwoodCompanionFeatureEnabled?.()===true&&isCurrent();
 async function command(path,body){if(!current())return;working=true;try{await client.request(path,body);await poll();}catch(error){if(current())status.textContent=error.message;}finally{working=false;}}
 function render(jobs){
  if(initial){for(const job of jobs)if(job.phase==='ready')seen.add(job.engine_id);initial=false;}
  content.replaceChildren();status.textContent='';
  for(const p of packages){
   const job=jobs.find(j=>j.id===p.id),active=job&&!['ready','paused','cancelled'].includes(job.phase);
   const card=document.createElement('div');card.className='engine-download-card';
   const title=document.createElement('strong');title.textContent=`${p.name} ${p.version}`;
   const description=document.createElement('p');description.textContent=`${bytes(p.bytes)} · ${p.license} · ${p.networks}`;
   const link=document.createElement('a');link.textContent='Original publisher and source';if(/^https:\/\//.test(p.source)){link.href=p.source;link.target='_blank';link.rel='noopener noreferrer';}
   const networkLinks=document.createElement('p');networkLinks.hidden=!p.network_source;
   for(const [label,url] of [['Network source',p.network_source],['Network permission',p.network_permission]])if(/^https:\/\//.test(url||'')){const a=document.createElement('a');a.textContent=label;a.href=url;a.target='_blank';a.rel='noopener noreferrer';a.style.marginRight='12px';networkLinks.append(a);}
   const progress=document.createElement('progress');progress.max=job?.total||p.bytes;progress.value=job?.downloaded||0;progress.style.cssText='width:100%;height:18px;accent-color:#d3ad62';progress.hidden=!job;
   const text=document.createElement('p');text.setAttribute('role','status');text.textContent=job?downloadProgress(job):'Ready to download from the original publisher.';
   const action=document.createElement('button');action.type='button';action.textContent=job?.phase==='ready'?'Installed':job?'Resume download / install':'Download and install';action.disabled=working||active||job?.phase==='ready';action.onclick=()=>void command('engine-downloads',{package:p.id});
   const cancel=document.createElement('button');cancel.type='button';cancel.textContent='Cancel download';cancel.className='secondary';cancel.hidden=!active;cancel.disabled=working;cancel.onclick=()=>void command(`engine-downloads/${p.id}/cancel`,{});
   card.append(title,description,link,networkLinks,progress,text,action,cancel);content.append(card);
   if(job?.phase==='ready'&&!seen.has(job.engine_id)){seen.add(job.engine_id);onInstalled(job);}
  }
 }
 async function poll(){
  target.clearTimeout(timer);if(!current())return;
  try{const result=await client.request('engine-downloads');if(!current())return;render(result.jobs||[]);}
  catch(error){if(current())status.textContent=error.status===404?'Update Companion to use engine downloads.':error.message;}
  if(current())timer=target.setTimeout(()=>void poll(),1000);
 }
 void(async()=>{try{const result=await client.request('engine-download-catalog');if(!current())return;packages=result.packages||[];if(!result.available||!packages.length){status.textContent='Publisher downloads are not available for this Companion platform yet. You can still choose a local engine below.';return;}await poll();}catch(error){if(current())status.textContent=error.status===404?'Update Companion to use engine downloads. You can still select a local engine below.':error.message;}})();
 return ()=>{alive=false;target.clearTimeout(timer);section.remove();};
}
