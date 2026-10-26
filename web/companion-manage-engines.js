import {CompanionArchiveClient} from './companion-archive-client.js';
export async function manageCompanionEngines(providers,{target=window,identity='',isCurrent=()=>true,onSaved=()=>{},clientFactory=provider=>new CompanionArchiveClient(provider.endpoint,provider.credential)}={}){
 if(target.ironwoodCompanionFeatureEnabled?.()!==true||!providers.length)return false;
 const provider=providers.find(p=>identity.startsWith(p.endpoint+'/')&&identity.length>p.endpoint.length+1);if(!provider)return false;
 const engineId=identity.slice(provider.endpoint.length+1);
 const document=target.document;if(document.getElementById('companion-manage-engines'))return false;
 if(!document.getElementById('companion-engines-style')){const link=document.createElement('link');link.id='companion-engines-style';link.rel='stylesheet';link.href='/companion-engines.css';document.head.append(link);}
 return new Promise(resolve=>{
  const dialog=document.createElement('dialog');dialog.id='companion-manage-engines';dialog.setAttribute('aria-label','Manage engines');
  dialog.innerHTML=`<form><header><p class="eyebrow">IRONWOOD COMPANION</p><h2>Manage engines</h2></header><div class="engine-dialog-body"><div data-details hidden><label>Engine name<input name="label" maxlength="128" required></label><section class="engine-launch-settings"><h3>Launch settings</h3><label>Executable<input name="executable" readonly></label><label>Arguments (JSON array)<textarea name="args" rows="2" spellcheck="false"></textarea></label></section><p>Playing strength and analysis options are available in Engine Settings → Engine options.</p><section><h3>Remove engine</h3><p>Training profiles, ratings and saved games are kept. Profiles using this engine cannot start until it is restored.</p><button type="button" data-remove class="secondary">Remove from Companion…</button><div data-remove-confirm hidden><p>Remove this engine from Companion?</p><label data-delete-label hidden><input type="checkbox" data-delete-files> Also delete files installed by Companion</label><p data-keep-files>Engine files will be kept.</p><button type="button" data-confirm-remove>Confirm removal</button><button type="button" data-cancel-remove class="secondary">Keep engine</button></div></section></div></div><p role="status" aria-live="polite">Reading registered engines…</p><footer class="actions"><button type="submit" disabled>Save settings</button><button type="button" data-reload class="secondary">Reload settings</button><button type="button" data-close class="secondary">Close</button></footer></form>`;
  const form=dialog.querySelector('form'),details=form.querySelector('[data-details]'),status=form.querySelector('[role=status]'),save=form.querySelector('[type=submit]');
  let alive=true,generation=0,data,client,saving=false,changed=false;
  const current=turn=>alive&&turn===generation&&target.ironwoodCompanionFeatureEnabled?.()===true&&isCurrent(provider);
  function render(value){
   data=value;form.querySelector('[data-remove-confirm]').hidden=true;form.querySelector('[data-delete-label]').hidden=!value.can_uninstall;form.querySelector('[data-delete-files]').checked=false;form.querySelector('[data-keep-files]').textContent=value.can_uninstall?'Only this Companion-installed package can be deleted.':'This engine was added manually. Its files will be kept.';form.querySelector('h2').textContent=`Manage engine — ${value.label}`;details.hidden=false;form.elements.label.value=value.label;form.elements.executable.value=value.executable;form.elements.args.value=JSON.stringify(value.args||[]);status.textContent=value.probe_error?`The engine could not start: ${value.probe_error}. Correct the launch arguments and save to validate again.`:'Changes apply to new sessions. Reselect the engine after saving.';save.disabled=false;
  }
  async function loadEngine(){
   const turn=++generation;data=null;save.disabled=true;details.hidden=true;if(!current(turn))return;status.textContent='Reading launch settings…';
   try{const value=await client.request(`engine-management/${encodeURIComponent(engineId)}`);if(current(turn))render(value);}
   catch(error){if(current(turn))status.textContent=error.status===404?'Update Companion to manage engines.':error.message;}
  }
  client=clientFactory(provider);form.querySelector('[data-reload]').onclick=()=>void loadEngine();
  form.querySelector('[data-remove]').onclick=()=>{form.querySelector('[data-remove-confirm]').hidden=false;};
  form.querySelector('[data-cancel-remove]').onclick=()=>{form.querySelector('[data-remove-confirm]').hidden=true;};
  form.querySelector('[data-confirm-remove]').onclick=async()=>{
   const turn=generation;if(!data||saving||!current(turn))return;
   saving=true;for(const node of form.querySelectorAll('input,textarea,select,button'))node.disabled=true;status.textContent='Removing engine…';
   try{const result=await client.request(`engine-management/${encodeURIComponent(data.id)}/remove`,{token:data.token,delete_files:data.can_uninstall===true&&form.querySelector('[data-delete-files]').checked});if(!current(turn))return;changed=true;onSaved({...data,removed:true});dialog.close();}
   catch(error){if(current(turn))status.textContent=error.message;}
   finally{saving=false;if(alive)for(const node of form.querySelectorAll('input,textarea,select,button'))node.disabled=false;}
  };
  form.onsubmit=async event=>{
   event.preventDefault();const turn=generation;if(!data||saving||!current(turn))return;
   let args;try{args=JSON.parse(form.elements.args.value);if(!Array.isArray(args)||args.some(a=>typeof a!=='string'))throw Error('Arguments must be a JSON array of strings.');}catch(error){status.textContent=error.message;return;}
   const values={...(data.values||{})};
   const body={token:data.token,label:form.elements.label.value.trim(),args,options:values};saving=true;for(const node of form.querySelectorAll('input,textarea,select,button'))node.disabled=true;status.textContent='Validating and saving engine settings…';
   try{const result=await client.request(`engine-management/${encodeURIComponent(data.id)}`,body);if(!current(turn))return;data={...data,token:result.token,label:body.label,args,values};changed=true;form.querySelector('h2').textContent=`Manage engine — ${body.label}`;status.textContent=result.message+' Reselect the engine to start a new session.';onSaved(data);}
   catch(error){if(current(turn))status.textContent=error.message;}
   finally{saving=false;if(alive){for(const node of form.querySelectorAll('input,textarea,select,button'))node.disabled=false;}}
  };
  form.querySelector('[data-close]').onclick=()=>dialog.close();dialog.addEventListener('cancel',event=>{if(saving)event.preventDefault();});
  dialog.addEventListener('close',()=>{alive=false;++generation;dialog.remove();resolve(changed);},{once:true});
  let drag;const header=form.querySelector('header');header.style.touchAction='none';header.onpointerdown=event=>{if(event.button!==0)return;const rect=dialog.getBoundingClientRect();drag={x:event.clientX,y:event.clientY,left:rect.left,top:rect.top};dialog.style.margin='0';header.setPointerCapture?.(event.pointerId);event.preventDefault();};header.onpointermove=event=>{if(drag){dialog.style.left=`${Math.max(0,Math.min(target.innerWidth-dialog.offsetWidth,drag.left+event.clientX-drag.x))}px`;dialog.style.top=`${Math.max(0,Math.min(target.innerHeight-50,drag.top+event.clientY-drag.y))}px`;}};header.onpointerup=header.onpointercancel=()=>{drag=null;};
  document.body.append(dialog);dialog.showModal();void loadEngine();
 });
}
