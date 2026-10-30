import { createSidebarRenderer } from './sidebar-renderer.js';

// Separate UI/layout/font/GPU resources, over the single main game's state.
export function installSidebarPopout(target,{createRenderer=createSidebarRenderer}={}) {
  let child,detached=false,active=false,dockButton,clean=()=>{},requestDock=()=>{};
  const live=()=>active&&!!child&&!child.closed;
  const cursors={Default:'default',None:'none',PointingHand:'pointer',Text:'text',VerticalText:'vertical-text',Grab:'grab',Grabbing:'grabbing',Crosshair:'crosshair',ResizeHorizontal:'ew-resize',ResizeVertical:'ns-resize',ResizeNwSe:'nwse-resize',ResizeNeSw:'nesw-resize',NotAllowed:'not-allowed',Wait:'wait',Progress:'progress'};
  target.ironwoodDockSidebarPopout=()=>{if(live())child.setTimeout(requestDock,0);};
  target.ironwoodSidebarPopoutOpen=()=>{if(detached&&!live())clean();return detached&&live();};
  target.ironwoodOpenSidebarPopout=()=>{
    if(live()){child.focus();return true;}
    child=target.open('about:blank','ironwood-sidebar','popup,width=1000,height=900,resizable=yes,scrollbars=no');
    if(!child){target.alert('Allow pop-ups for Ironwood to open the right-hand panel.');return false;}
    const popup=child,doc=popup.document;active=true;
    doc.open();
    doc.write('<!doctype html><html><head><title>Ironwood — Study panel</title><meta name="viewport" content="width=device-width,initial-scale=1"><style>html,body{height:100%;margin:0;overflow:hidden;background:#111418;color:#e8e5d6;font:14px "Segoe UI",sans-serif}body{display:flex;flex-direction:column}header{display:flex;align-items:center;flex-wrap:wrap;gap:10px;padding:8px 12px;background:#181c21;border-bottom:1px solid #584a32;flex:none}header span{flex:1}button,select{background:#242a2e;color:#e8e5d6;border:1px solid #75603a;border-radius:4px;padding:5px 8px}button{cursor:pointer}#surface{position:relative;flex:1;min-height:0}canvas{display:block;width:100%;height:100%;outline:none;touch-action:none}#status{position:absolute;inset:0;padding:16px;background:#111418;color:#d3ad62}textarea{position:fixed;left:-1000px;top:0;width:1px;height:1px;opacity:0}</style></head><body><header><span>Ironwood · Study panel</span><label>Text size <select id="zoom"><option value="1">100%</option><option value="1.1">110%</option><option value="1.25">125%</option><option value="1.5">150%</option></select></label><button id="main">Show main window</button><button id="dock">Dock panel</button></header><div id="surface"><canvas id="panel" tabindex="0" aria-label="Ironwood study panel"></canvas><div id="status">Opening study panel…</div></div><textarea id="input" aria-label="Study panel text input" autocomplete="off" autocapitalize="off" spellcheck="false"></textarea></body></html>');
    doc.close();
    const canvas=doc.getElementById('panel'),surface=doc.getElementById('surface'),status=doc.getElementById('status');
    const agent=doc.getElementById('input'),zoom=doc.getElementById('zoom');
    let renderer,events=[],ready=false,stopped=false,composing=false,ignoreCompositionInput=false,lastPoint={x:0,y:0};
    const pressed=new Set();
    const restore=()=>{
      if(stopped)return;stopped=true;active=false;detached=false;dockButton?.remove();dockButton=undefined;
      renderer?.destroy();try{target.ironwoodDockStudyPanel?.();}catch{}
    };
    clean=restore;
    const dock=()=>{restore();popup.close();target.focus();};
    requestDock=dock;
    doc.getElementById('main').onclick=()=>target.focus();doc.getElementById('dock').onclick=dock;
    popup.addEventListener('beforeunload',restore);
    try{renderer=createRenderer(canvas);}catch(error){status.textContent=error.message;active=false;return true;}
    const enqueue=event=>{if(stopped||events.length>=512)return;if(event.type==='move'&&events.at(-1)?.type==='move')events.pop();events.push(event);};
    const modifiers=e=>({alt:e.altKey,ctrl:e.ctrlKey,shift:e.shiftKey,meta:e.metaKey});
    const point=e=>{const box=canvas.getBoundingClientRect();return lastPoint={x:e.clientX-box.left,y:e.clientY-box.top};};
    canvas.addEventListener('pointermove',e=>enqueue({type:'move',...point(e),...modifiers(e)}));
    canvas.addEventListener('pointerdown',e=>{
      if(e.button>2)return;canvas.focus();canvas.setPointerCapture(e.pointerId);pressed.add(e.button);
      enqueue({type:'move',...point(e),...modifiers(e)});enqueue({type:'button',...lastPoint,button:e.button,pressed:true,...modifiers(e)});e.preventDefault();
    });
    canvas.addEventListener('pointerup',e=>{if(e.button>2)return;enqueue({type:'button',...point(e),button:e.button,pressed:false,...modifiers(e)});pressed.delete(e.button);e.preventDefault();});
    const release=()=>{for(const button of pressed)enqueue({type:'button',...lastPoint,button,pressed:false});pressed.clear();enqueue({type:'gone'});};
    canvas.addEventListener('pointercancel',release);popup.addEventListener('blur',release);
    canvas.addEventListener('pointerleave',()=>{if(!pressed.size)enqueue({type:'gone'});});
    canvas.addEventListener('contextmenu',e=>e.preventDefault());
    canvas.addEventListener('wheel',e=>{enqueue({type:'move',...point(e),...modifiers(e)});enqueue({type:'wheel',x:-e.deltaX,y:-e.deltaY,mode:e.deltaMode,...modifiers(e)});e.preventDefault();},{passive:false});
    const editing=()=>doc.activeElement===canvas||doc.activeElement===agent;
    for(const type of ['keydown','keyup'])doc.addEventListener(type,e=>{
      if(!editing()||e.isComposing)return;
      enqueue({type:'key',key:e.key,pressed:type==='keydown',repeat:e.repeat,...modifiers(e)});
      const character=[...e.key].length===1;
      if(type==='keydown'&&character&&doc.activeElement===canvas&&!e.ctrlKey&&!e.metaKey&&!e.altKey)enqueue({type:'text',text:e.key});
      if(!e.ctrlKey&&!e.metaKey&&!e.altKey&&(!character||doc.activeElement===canvas))e.preventDefault();
    });
    agent.addEventListener('compositionstart',()=>{composing=true;});
    agent.addEventListener('compositionend',e=>{composing=false;ignoreCompositionInput=true;agent.value='';if(e.data)enqueue({type:'text',text:e.data});popup.setTimeout(()=>{ignoreCompositionInput=false;},0);});
    agent.addEventListener('input',()=>{if(composing)return;if(!ignoreCompositionInput&&agent.value)enqueue({type:'text',text:agent.value});agent.value='';});
    for(const type of ['copy','cut','paste'])doc.addEventListener(type,e=>{if(!editing())return;enqueue(type==='paste'?{type,text:e.clipboardData.getData('text/plain')}:{type});e.preventDefault();});
    const commands=async frame=>{
      for(const command of frame.commands){
        if(command.type==='copy')await popup.navigator.clipboard.writeText(command.text);
        else if(command.type==='url')popup.open(command.url,command.newTab?'_blank':'_self');
        else if(command.type==='copyImage'){
          const image=doc.createElement('canvas');image.width=command.width;image.height=command.height;
          image.getContext('2d').putImageData(new popup.ImageData(new Uint8ClampedArray(command.pixels),command.width,command.height),0,0);
          const blob=new Promise(resolve=>image.toBlob(resolve,'image/png'));
          await popup.navigator.clipboard.write([new popup.ClipboardItem({'image/png':blob})]);
        }
      }
    };
    const draw=()=>{
      if(stopped||child!==popup||popup.closed){restore();return;}
      if(target.closed){restore();status.hidden=false;status.textContent='The main game window is closed.';return;}
      try{
        if(!ready)target.ironwoodBeginStudyPanel?.();
        const box=surface.getBoundingClientRect(),width=Math.max(100,box.width),height=Math.max(100,box.height),dpr=popup.devicePixelRatio||1;
        renderer.resize(width,height,dpr);const batch=events;events=[];
        const frame=target.ironwoodRenderStudyPanel?.(JSON.stringify({width,height,dpr,zoom:Number(zoom.value),time:target.performance.now()/1000,
          textureLimit:renderer.textureLimit,focused:doc.hasFocus(),events:batch}));
        if(frame?.ready){
          renderer.paint(frame);status.hidden=true;canvas.style.cursor=cursors[frame.cursor]||'default';
          if(!ready){
            ready=true;detached=true;dockButton=target.document.createElement('button');dockButton.textContent='Dock study panel';dockButton.type='button';
            dockButton.setAttribute('aria-label','Return study panel to the main window');
            Object.assign(dockButton.style,{position:'fixed',right:'12px',top:'8px',zIndex:'10000',background:'#242a2e',color:'#e8e5d6',border:'1px solid #75603a',borderRadius:'4px',padding:'7px 10px',cursor:'pointer'});
            dockButton.onclick=dock;target.document.body.append(dockButton);
          }
          if(frame.editingText&&doc.activeElement===canvas)agent.focus({preventScroll:true});
          else if(!frame.editingText&&doc.activeElement===agent)canvas.focus({preventScroll:true});
          void commands(frame).catch(()=>{status.hidden=false;status.textContent='The browser could not complete the clipboard or link action.';});
        }
      }catch(error){restore();status.hidden=false;status.textContent=`Study panel stopped: ${error.message}. The panel is available in the main window.`;return;}
      popup.requestAnimationFrame(draw);
    };
    popup.requestAnimationFrame(draw);canvas.focus();return true;
  };
  target.addEventListener?.('beforeunload',()=>{if(live())child.close();clean();});
}
