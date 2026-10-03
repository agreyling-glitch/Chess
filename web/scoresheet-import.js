import { scoresheetPgn } from './scoresheet-pgn.js';
import { draftStore } from './scoresheet-draft.js';
let dialog,pending='',entries=[],preview,selected=0,photoUrl,zoom=1,loading=0;
let photoFile,restoring=false,saveQueue=Promise.resolve(),saveRevision=0;
const storage=draftStore(),detailIds=['white','black','event','date','result'];
function saveDraft(){
 if(restoring)return;
 const revision=++saveRevision;
 const draft={version:1,entries:[...entries],details:Object.fromEntries(detailIds.map(id=>[id,$(id).value])),photo:photoFile};
 $('saved').textContent='Saving draft…';
 saveQueue=saveQueue.catch(()=>{}).then(()=>storage.write(draft)).then(()=>{if(revision===saveRevision)$('saved').textContent='Draft saved on this device.';},()=>{if(revision===saveRevision)$('saved').textContent='Draft could not be saved. Keep this window open or download PGN.';});
}
async function restoreDraft(){
 restoring=true;dialog.querySelectorAll('input,select,button').forEach(element=>element.disabled=true);$('saved').textContent='Restoring draft…';
 try{const draft=await storage.read();if(draft?.version===1&&Array.isArray(draft.entries)){
 const values=draft.entries.slice(0,1000).map(value=>typeof value==='string'?value.slice(0,20):'');
 if(values.length>entries.length)addRows(Math.ceil(values.length/2)-entries.length/2);
 values.forEach((value,index)=>{entries[index]=value;$('table').querySelector(`[data-index="${index}"]`).value=value;});
 for(const id of detailIds)if(typeof draft.details?.[id]==='string')$(id).value=draft.details[id];
 if(draft.photo)await load(draft.photo);
 $('saved').textContent='Saved draft restored.';
 }else $('saved').textContent='Drafts save automatically on this device.';
 }catch{$('saved').textContent='Saved draft could not be opened. Local saving may be unavailable.';}
 finally{restoring=false;dialog.querySelectorAll('input,select,button').forEach(element=>element.disabled=false);update(false);}
}
const START='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const $=id=>dialog.querySelector(`#ss-${id}`);
const labels={valid:'Legal move',incomplete:'Finish typing this move',invalid:'Illegal or unrecognized move',missing:'Enter the missing move',blocked:'Correct the earlier move first',empty:''};
function board(){
 const highlight=selected?preview?.highlights?.[selected-1]:null;
 const fen=preview?.positions[Math.min(selected,preview.positions.length-1)]||START;$('board').replaceChildren();let index=0;
 for(const char of fen.split(' ')[0].replaceAll('/','')){const count=/[1-8]/.test(char)?Number(char):1;for(let n=0;n<count;n++,index++){
 const square=document.createElement('div'),name='abcdefgh'[index%8]+(8-Math.floor(index/8));square.className=(Math.floor(index/8)+index%8)%2?'dark':'light';square.setAttribute('role','img');square.setAttribute('aria-label',name+': '+(/[1-8]/.test(char)?'empty':char));
 if(name===highlight?.from||name===highlight?.to){square.classList.add('last-move');square.setAttribute('aria-label',square.getAttribute('aria-label')+(name===highlight.from?', last move from':', last move to'));}
 if(!/[1-8]/.test(char)){const piece=document.createElement('img');piece.src=`/pieces/cburnett/${char===char.toUpperCase()?'w':'b'}${char.toUpperCase()}.svg`;piece.alt='';square.append(piece);}
 if(index%8===0){const rank=document.createElement('small');rank.textContent=String(8-Math.floor(index/8));square.append(rank);}if(index>=56){const file=document.createElement('small');file.className='file';file.textContent=name[0];square.append(file);}$('board').append(square);
 }}$('position').textContent=selected?`After ${Math.ceil(selected/2)}${selected%2?'.':'…'} ${preview.moves[selected-1]}`:'Starting position';
}
function moveList(reveal=true){
 const previousScroll=$('list').scrollTop;
 $('list').replaceChildren();const start=document.createElement('button');start.textContent='Starting position';start.onclick=()=>{selected=0;board();moveList();};$('list').append(start);
 for(let i=0;i<preview.moves.length;i+=2){const row=document.createElement('div');row.className='ss-move-row';const number=document.createElement('span');number.textContent=`${i/2+1}.`;row.append(number);for(let j=i;j<Math.min(i+2,preview.moves.length);j++){const button=document.createElement('button');button.textContent=preview.moves[j];button.classList.toggle('selected',selected===j+1);button.onclick=()=>{selected=j+1;board();moveList();};row.append(button);}$('list').append(row);}
 const list=$('list');list.scrollTop=previousScroll;
 if(reveal){const target=selected?list.querySelector('.selected'):start;const bounds=list.getBoundingClientRect(),item=target.getBoundingClientRect();
 if(item.bottom>bounds.bottom)list.scrollTop+=item.bottom-bounds.bottom;
 else if(item.top<bounds.top)list.scrollTop+=item.top-bounds.top;}
}
function update(follow=true){
 $('use').disabled=$('download').disabled=true;if(!window.ironwoodPreviewScoresheet){$('status').textContent='Chess validation is loading…';return;}
 try{preview=JSON.parse(window.ironwoodPreviewScoresheet(JSON.stringify(entries)));}catch(error){$('status').textContent=String(error.message||error);return;}
 const last=entries.findLastIndex(value=>value.trim()),bad=preview.statuses.findIndex((value,index)=>index<=last&&value!=='valid');
 for(const input of $('table').querySelectorAll('input')){const index=Number(input.dataset.index),state=preview.statuses[index]||'empty';input.dataset.state=state;input.setAttribute('aria-invalid',['invalid','missing'].includes(state)?'true':'false');input.title=labels[state];input.closest('td').querySelector('small').textContent=labels[state];}
 $('status').textContent=bad<0?(last<0?'Enter White’s first move. Use Tab to move through the sheet.':`${preview.moves.length} legal half-moves. Ready to import.`):`${Math.floor(bad/2)+1}${bad%2?'… Black':'. White'}: ${labels[preview.statuses[bad]]}. Later moves will be checked after this is corrected.`;
 const ready=last>=0&&bad<0;$('use').disabled=$('download').disabled=!ready;selected=follow?preview.moves.length:Math.min(selected,preview.moves.length);board();moveList(follow);
}
function addRows(count=10){
 const first=entries.length/2;for(let rowIndex=first;rowIndex<Math.min(first+count,500);rowIndex++){const row=document.createElement('tr'),number=document.createElement('th');number.scope='row';number.textContent=String(rowIndex+1);row.append(number);
 for(let side=0;side<2;side++){const index=entries.length;entries.push('');const cell=document.createElement('td'),input=document.createElement('input'),hint=document.createElement('small');input.dataset.index=String(index);input.autocomplete='off';input.spellcheck=false;input.maxLength=20;input.setAttribute('aria-label',`Move ${rowIndex+1} ${side?'Black':'White'}`);hint.id=`ss-hint-${index}`;input.setAttribute('aria-describedby',hint.id);
 input.oninput=()=>{entries[index]=input.value;if(index>=entries.length-2&&input.value.trim())addRows();update();saveDraft();};input.onfocus=()=>{selected=Math.min(index,preview?.moves.length||0);board();if(preview)moveList();};input.onkeydown=event=>{if(event.key==='Enter'){event.preventDefault();$('table').querySelector(`[data-index="${index+1}"]`)?.focus();}};cell.append(input,hint);row.append(cell);}$('table').append(row);}
}
function game(){update(false);if($('use').disabled)return null;const moves=preview.moves.map((move,index)=>`${index%2?'':`${index/2+1}. `}${move}`).join(' ');return scoresheetPgn(moves,{White:$('white').value,Black:$('black').value,Event:$('event').value,Date:$('date').value.replaceAll('-','.')||'????.??.??',Result:$('result').value});}
async function load(file){
 if(!file)return;if(!['image/jpeg','image/png','image/webp'].includes(file.type)||file.size>20*1024*1024){$('photo-status').textContent='Choose a JPG, PNG, or WebP photo smaller than 20 MB.';return;}const version=++loading,url=URL.createObjectURL(file);
 try{const image=new Image();image.src=url;await image.decode();if(version!==loading){URL.revokeObjectURL(url);return;}if(photoUrl)URL.revokeObjectURL(photoUrl);photoUrl=url;photoFile=file;$('photo').src=url;$('photo').hidden=false;zoom=1;photoZoom();$('photo-status').textContent='Use zoom and drag to read the sheet. The photo stays on this device.';saveDraft();}catch{URL.revokeObjectURL(url);$('photo-status').textContent='This photo could not be opened.';}
}
function photoZoom(){$('photo').style.width=`${zoom*100}%`;$('zoom-value').textContent=`${Math.round(zoom*100)}%`;}
function create(){
 const css=document.createElement('link');css.rel='stylesheet';css.href='/scoresheet-import.css';document.head.append(css);dialog=document.createElement('dialog');dialog.id='ironwood-scoresheet';
 dialog.innerHTML=`<header><h2>Enter a scoresheet</h2><button id="ss-close" aria-label="Close scoresheet entry">×</button></header>
 <div class="ss-details"><label>White<input id="ss-white" maxlength="200"></label><label>Black<input id="ss-black" maxlength="200"></label><label>Event<input id="ss-event" maxlength="200"></label><label>Date<input id="ss-date" type="date"></label><label>Result<select id="ss-result"><option value="*">Unknown / unfinished</option><option value="1-0">White won</option><option value="0-1">Black won</option><option value="1/2-1/2">Draw</option></select></label></div>
 <div class="ss-grid"><section class="ss-panel"><h3>Board preview</h3><div id="ss-board" aria-label="Chess board preview"></div><p id="ss-position">Starting position</p></section>
 <section class="ss-panel"><h3>Move list</h3><p>Click a move to preview its position.</p><div id="ss-list" class="ss-scroll" aria-label="Validated moves"></div></section>
 <section class="ss-panel" id="ss-reference"><h3>Scoresheet photo</h3><label>Choose photo<input id="ss-file" type="file" accept="image/jpeg,image/png,image/webp"></label><div class="ss-tools"><button id="ss-zoom-out" aria-label="Zoom out">−</button><span id="ss-zoom-value">100%</span><button id="ss-zoom-in" aria-label="Zoom in">+</button><button id="ss-fit">Fit photo</button><button id="ss-to-entry" class="ss-mobile">Go to entry</button></div><div id="ss-photo-frame"><img id="ss-photo" alt="Scoresheet reference photo" hidden draggable="false"></div><p id="ss-photo-status">Choose, paste, or drop a scoresheet photo. You can also enter moves without one.</p></section>
 <section class="ss-panel" id="ss-entry"><h3>Enter moves</h3><p>Use notation such as e4, Nf3, O-O, or e8=Q. Tab advances to the next cell.</p><button id="ss-to-photo" class="ss-mobile">Go to photo</button><div class="ss-scroll"><table><thead><tr><th scope="col">Move</th><th scope="col">White</th><th scope="col">Black</th></tr></thead><tbody id="ss-table"></tbody></table></div></section></div>
 <footer><div><p id="ss-status" role="status" aria-live="polite"></p><p id="ss-saved" role="status" aria-live="polite"></p></div><div class="ss-tools"><button id="ss-new">Discard draft</button><button id="ss-download" disabled>Download PGN</button><button id="ss-use" disabled>Import game</button></div></footer>`;
 document.body.append(dialog);addRows(30);$('close').onclick=()=>dialog.close();$('file').onchange=event=>{load(event.target.files[0]);event.target.value='';};
 dialog.addEventListener('paste',event=>{const file=[...(event.clipboardData?.items||[])].find(item=>item.type.startsWith('image/'))?.getAsFile();if(file){event.preventDefault();load(file);}});dialog.ondragover=event=>{event.preventDefault();event.stopPropagation();};dialog.ondrop=event=>{event.preventDefault();event.stopPropagation();load(event.dataTransfer.files[0]);};
 $('zoom-in').onclick=()=>{zoom=Math.min(5,zoom+.25);photoZoom();};$('zoom-out').onclick=()=>{zoom=Math.max(.5,zoom-.25);photoZoom();};$('fit').onclick=()=>{zoom=1;photoZoom();};let pan;
 $('photo-frame').onpointerdown=event=>{if(!photoUrl)return;pan={x:event.clientX,y:event.clientY,left:$('photo-frame').scrollLeft,top:$('photo-frame').scrollTop};$('photo-frame').setPointerCapture(event.pointerId);};$('photo-frame').onpointermove=event=>{if(pan){$('photo-frame').scrollLeft=pan.left-event.clientX+pan.x;$('photo-frame').scrollTop=pan.top-event.clientY+pan.y;}};$('photo-frame').onpointerup=$('photo-frame').onpointercancel=()=>{pan=null;};
 for(const id of ['white','black','event','date','result'])$(id).oninput=()=>{update(false);saveDraft();};$('to-entry').onclick=()=>$('entry').scrollIntoView({block:'start',behavior:'smooth'});$('to-photo').onclick=()=>$('reference').scrollIntoView({block:'start',behavior:'smooth'});
 $('new').onclick=()=>{if(!window.confirm('Discard the saved draft, including moves, details, and photo?'))return;const revision=++saveRevision;$('saved').textContent='Discarding draft…';loading++;entries=[];$('table').replaceChildren();addRows(30);for(const id of detailIds)$(id).value=id==='result'?'*':'';selected=0;photoFile=undefined;if(photoUrl)URL.revokeObjectURL(photoUrl);photoUrl=undefined;$('photo').removeAttribute('src');$('photo').hidden=true;zoom=1;photoZoom();$('photo-status').textContent='Choose, paste, or drop a scoresheet photo.';update();saveQueue=saveQueue.catch(()=>{}).then(()=>storage.clear()).then(()=>{if(revision===saveRevision)$('saved').textContent='Draft discarded.';},()=>{if(revision===saveRevision)$('saved').textContent='Saved draft could not be discarded. Try again.';});};
 $('use').onclick=()=>{const pgn=game();if(pgn){pending=pgn;dialog.close();}};$('download').onclick=()=>{const pgn=game();if(!pgn)return;const url=URL.createObjectURL(new Blob([pgn],{type:'application/x-chess-pgn'}));const link=document.createElement('a');link.href=url;link.download='scoresheet.pgn';document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);};
}
window.ironwoodOpenScoresheet=()=>{if(!dialog){create();restoreDraft();}if(!dialog.open)dialog.showModal();if(!restoring)update(false);};
window.ironwoodPollScoresheet=()=>{const game=pending;pending='';return game;};
