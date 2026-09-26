import {STANDARD,parseFen,placement,basicError} from './model.js';
let draggedSquare=null;
let dialog, state, selected='P', flipped=false, worker, busy=false, pending='', confidence=[], scanVersion=0;
const names={k:'King',q:'Queen',r:'Rook',b:'Bishop',n:'Knight',p:'Pawn'};
const $=id=>dialog.querySelector('#pe-'+id);
function fen(){return `${placement(state.board)} ${$('turn').value} ${$('rights').value.trim()||'-'} ${$('ep').value.trim()||'-'} ${$('half').value} ${$('full').value}`;}
function sync(){ $('fen').value=fen(); const error=basicError(state.board); $('error').textContent=error; $('use').disabled=!!error; }
function render(){
 const board=$('board');board.replaceChildren();
 for(let cell=0;cell<64;cell++){
  const index=flipped?63-cell:cell,piece=state.board[index],square='abcdefgh'[index%8]+(8-Math.floor(index/8));
  const button=document.createElement('button');button.type='button';button.className=`square ${(Math.floor(cell/8)+cell%8)%2?'dark':'light'}`;
  if(confidence[index]<.7)button.classList.add('uncertain');
  button.setAttribute('aria-label',`${square}: ${piece?(piece===piece.toUpperCase()?'White ':'Black ')+names[piece.toLowerCase()]:'empty'}`);
  button.title=button.getAttribute('aria-label');
  if(piece){const image=document.createElement('img');image.src=`/pieces/cburnett/${piece===piece.toUpperCase()?'w':'b'}${piece.toUpperCase()}.svg`;image.alt='';image.draggable=false;button.append(image);button.draggable=true;}
  const label=document.createElement('small');label.textContent=square;button.append(label);
  button.onclick=()=>{state.board[index]=selected;confidence[index]=1;scanVersion++;render();};
  button.oncontextmenu=e=>{e.preventDefault();state.board[index]='';confidence[index]=1;scanVersion++;render();};
  button.ondragstart=e=>{draggedSquare=index;e.dataTransfer.effectAllowed='move';e.dataTransfer.setData('application/x-ironwood-piece',`square:${index}`);e.dataTransfer.setData('text/plain',`square:${index}`);};
  button.ondragend=()=>{draggedSquare=null;$('trash').classList.remove('drop-ready');};
  button.ondragover=e=>e.preventDefault();button.ondrop=e=>{const value=e.dataTransfer.getData('application/x-ironwood-piece');if(!value)return;e.preventDefault();e.stopPropagation();if(value.startsWith('square:')){const source=Number(value.slice(7));if(Number.isInteger(source)&&source>=0&&source<64){state.board[index]=state.board[source];if(source!==index)state.board[source]='';}}else if(/^[prnbqkPRNBQK]$/.test(value))state.board[index]=value;confidence[index]=1;scanVersion++;render();};
  board.append(button);
 }
 sync();
}
function load(f){state=parseFen(f);for(const key of ['turn','rights','ep','half','full'])$(key).value=state[key];confidence=[];scanVersion++;render();}
function ensureWorker(){if(worker)return;worker=new Worker('/position-editor/vendor/recognition-worker.js',{type:'module'});worker.onmessage=({data})=>{
 if(data.type==='ready')return;
 if(!busy)return;
 busy=false;$('upload').disabled=false;
 if(data.type==='error'){$('status').textContent=`Recognition failed: ${data.message}. You can still edit manually.`;return;}
 if(!dialog.open || worker.version!==scanVersion)return;
 if(!data.result){$('status').textContent='No board found. Try a screenshot cropped closely around the board.';return;}
 const {result,orientation}=data;
 load(`${orientation.placement} w - - 0 1`);
 confidence=Array.from({length:64},(_,i)=>{const j=orientation.orientation==='black'?63-i:i;return result.confidences[(7-Math.floor(j/8))*8+j%8];});render();
 $('status').textContent=`Recognized in ${(data.timing.total/1000).toFixed(1)}s. ${result.reliable?'Review every piece.':'Uncertain squares are outlined in orange.'} Confirm orientation, turn and castling rights. Screenshot import does not infer game history.`;
 };worker.onerror=()=>{busy=false;$('upload').disabled=false;$('status').textContent='Recognition worker failed. Manual editing remains available.';worker.terminate();worker=null;};worker.postMessage({type:'warmup'});}
function scan(file){if(busy||!file)return;if(!file.type.startsWith('image/')){$('status').textContent='Choose an image file.';return;}ensureWorker();busy=true;$('upload').disabled=true;$('status').textContent='Reading board locally… First use downloads the recognition model and runtime.';worker.version=++scanVersion;worker.postMessage({type:'scan',file});}
function create(){
 const css=document.createElement('link');css.rel='stylesheet';css.href='/position-editor/editor.css';document.head.append(css);
 dialog=document.createElement('dialog');dialog.id='ironwood-position-editor';dialog.innerHTML=`<header><h2>Position editor</h2><button id="pe-close" aria-label="Close position editor">×</button></header><section class="pe-body"><div><div class="pe-tools"><button id="pe-flip">Flip view</button><button id="pe-rotate">Rotate position</button><button id="pe-standard">Starting position</button><button id="pe-clear">Clear</button></div><div id="pe-board" aria-label="Position board"></div><div id="pe-palette" aria-label="Choose a piece"></div><p class="hint">Choose a piece, then click a square. Drag pieces to move them; right-click to erase, or drag a piece into the trash.</p></div><aside><h3>Import screenshot</h3><input id="pe-upload" type="file" accept="image/*"><p class="hint">Paste an image here or drop one into this window. Crop to the board for best results. Cburnett and Merida are the tested sets.</p><p id="pe-status" role="status">Recognition runs locally. Review the result before using it.</p><h3>Position details</h3><label>Side to move<select id="pe-turn"><option value="w">White</option><option value="b">Black</option></select></label><label>Castling rights<input id="pe-rights" placeholder="KQkq or -" aria-describedby="pe-castling-help"></label><small id="pe-castling-help">KQkq for standard chess; rook-file letters for Chess960. Use - for none.</small><label>En passant target<input id="pe-ep" placeholder="- or e3"></label><label>Halfmove clock<input id="pe-half" type="number" min="0"></label><label>Fullmove number<input id="pe-full" type="number" min="1"></label></aside></section><footer><label>FEN<textarea id="pe-fen" rows="2" spellcheck="false"></textarea></label><div class="pe-tools"><button id="pe-load">Load FEN</button><button id="pe-copy">Copy FEN</button><span id="pe-error" role="alert"></span><button id="pe-use" class="primary">Use in Ironwood</button></div></footer>`;
 document.body.append(dialog);
 $('close').onclick=()=>dialog.close();dialog.addEventListener('close',()=>{scanVersion++;});
 for(const p of 'KQRBNPkqrbnp '){const button=document.createElement('button');button.type='button';button.title=p===' '?'Erase':(p===p.toUpperCase()?'White ':'Black ')+names[p.toLowerCase()];button.setAttribute('aria-label',button.title);if(p===' '){
 button.id='pe-trash';
 button.title='Remove piece — click to erase squares, or drop a board piece here';
 button.setAttribute('aria-label',button.title);
 button.innerHTML='<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18M9 6V4h6v2M5 6l1 15h12l1-15M10 10v7M14 10v7"/></svg>';
 button.ondragover=e=>{if(draggedSquare!==null){e.preventDefault();e.dataTransfer.dropEffect='move';button.classList.add('drop-ready');}};
 button.ondragleave=e=>{if(!button.contains(e.relatedTarget))button.classList.remove('drop-ready');};
 button.ondrop=e=>{if(draggedSquare===null)return;e.preventDefault();e.stopPropagation();const index=draggedSquare;draggedSquare=null;state.board[index]='';confidence[index]=1;scanVersion++;button.classList.remove('drop-ready');render();$('status').textContent='Piece removed.';};
 }else{const img=document.createElement('img');img.src=`/pieces/cburnett/${p===p.toUpperCase()?'w':'b'}${p.toUpperCase()}.svg`;img.alt='';img.draggable=false;button.append(img);}button.draggable=p!==' ';button.ondragstart=e=>{e.dataTransfer.effectAllowed='copy';e.dataTransfer.setData('application/x-ironwood-piece',p);e.dataTransfer.setData('text/plain',p);};button.onclick=()=>{selected=p.trim();for(const b of $('palette').children)b.classList.remove('selected');button.classList.add('selected');};if(p==='P')button.classList.add('selected');$('palette').append(button);}
 $('flip').onclick=()=>{flipped=!flipped;render();};$('rotate').onclick=()=>{state.board.reverse();confidence.reverse();scanVersion++;render();};$('standard').onclick=()=>load(STANDARD);$('clear').onclick=()=>load('8/8/8/8/8/8/8/8 w - - 0 1');
 for(const key of ['turn','rights','ep','half','full'])$(key).oninput=sync;
 $('load').onclick=()=>{try{load($('fen').value);}catch(e){$('error').textContent=e.message;}};
 $('copy').onclick=async()=>{try{await navigator.clipboard.writeText($('fen').value);$('status').textContent='FEN copied.';}catch{$('status').textContent='Select the FEN text and copy it manually.';}};
 $('use').onclick=()=>{try{const parsed=parseFen($('fen').value);const error=basicError(parsed.board);if(error)throw Error(error);pending=$('fen').value.trim();dialog.close();}catch(e){$('error').textContent=e.message;}};
 $('upload').onchange=e=>{scan(e.target.files[0]);e.target.value='';};
 dialog.addEventListener('paste',e=>{const item=[...e.clipboardData.items].find(item=>item.type.startsWith('image/'));if(item){e.preventDefault();scan(item.getAsFile());}});
 dialog.ondragover=e=>{if(e.dataTransfer.types.includes('Files'))e.preventDefault();};dialog.ondrop=e=>{if(e.dataTransfer.types.includes('application/x-ironwood-piece')){e.preventDefault();return;}if(e.dataTransfer.files.length){e.preventDefault();scan(e.dataTransfer.files[0]);}};
}
window.ironwoodOpenPositionEditor=fen=>{if(!dialog)create();try{load(fen);}catch{load(STANDARD);}$('status').textContent='Arrange pieces manually, or import a screenshot. Images stay on your device.';dialog.showModal();ensureWorker();};
window.ironwoodPollPositionEditor=()=>{const value=pending;pending='';return value;};
