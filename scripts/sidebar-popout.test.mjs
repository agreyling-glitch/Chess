import test from 'node:test';
import assert from 'node:assert/strict';
import { JSDOM } from 'jsdom';
import { installSidebarPopout } from '../web/sidebar-popout.js';
import { backingSize, clipPixels } from '../web/sidebar-renderer.js';

test('GPU backing size and clipping use the popup native pixel density',()=>{
  assert.deepEqual(backingSize(800,600,2),[1600,1200]);
  assert.deepEqual(clipPixels([10,20,200,100],2,1600,1200),[20,1000,380,160]);
  assert.deepEqual(clipPixels([-10,-10,2000,2000],2,1600,1200),[0,0,1600,1200]);
});

test('independent rendering, local input, resizing, and docking keep one game and never resize or read the main canvas',async()=>{
  const parent=new JSDOM('<canvas id="chess_canvas" style="width:100%"></canvas>',{url:'http://localhost:8788/play/'}).window;
  const popup=new JSDOM('',{url:'http://localhost:8788/play/'}).window;
  const mainCanvas=parent.document.getElementById('chess_canvas');
  mainCanvas.captureStream=()=>assert.fail('Image streaming must not be used');
  mainCanvas.getContext=()=>assert.fail('The main renderer must not be copied');
  let nextFrame,opens=0,focused=0,docked=0,destroyed=0,width=800;
  const requests=[],paints=[],sizes=[],game={selectedMove:7};
  parent.open=()=>{opens++;return popup;};parent.alert=()=>assert.fail('Unexpected warning');parent.focus=()=>{};
  popup.closed=false;popup.focus=()=>focused++;popup.close=()=>{popup.dispatchEvent(new popup.Event('beforeunload'));popup.closed=true;};
  popup.requestAnimationFrame=callback=>{nextFrame=callback;};
  Object.defineProperty(popup,'devicePixelRatio',{value:2});
  parent.ironwoodBeginStudyPanel=()=>{};parent.ironwoodDockStudyPanel=()=>docked++;
  parent.ironwoodRenderStudyPanel=json=>{
    const input=JSON.parse(json);requests.push(input);
    if(input.events.some(event=>event.type==='button'&&event.pressed))game.selectedMove++;
    return {ready:true,commands:[],cursor:'Default',editingText:false,selectedMove:game.selectedMove,pixelsPerPoint:input.dpr*input.zoom};
  };
  installSidebarPopout(parent,{createRenderer:()=>({textureLimit:8192,resize:(...args)=>sizes.push(args),paint:frame=>paints.push(frame),destroy:()=>destroyed++})});
  assert.equal(parent.ironwoodOpenSidebarPopout(),true);
  assert.equal(parent.ironwoodSidebarPopoutOpen(),false,'Docked panel remains until the popup renders');
  popup.document.getElementById('surface').getBoundingClientRect=()=>({width,height:600});
  nextFrame();
  assert.equal(parent.ironwoodSidebarPopoutOpen(),true);
  assert.deepEqual(sizes[0],[800,600,2]);
  assert.equal(mainCanvas.style.width,'100%');assert.equal(mainCanvas.width,300);
  const canvas=popup.document.getElementById('panel');canvas.setPointerCapture=()=>{};
  canvas.getBoundingClientRect=()=>({left:10,top:50,width,height:600});
  canvas.dispatchEvent(new popup.MouseEvent('pointerdown',{clientX:100,clientY:300,button:0}));
  nextFrame();
  assert.deepEqual(requests.at(-1).events.map(e=>[e.type,e.x,e.y]),[['move',90,250],['button',90,250]]);
  assert.equal(game.selectedMove,8);assert.equal(paints.at(-1).selectedMove,8);
  width=520;popup.document.getElementById('zoom').value='1.25';nextFrame();
  assert.deepEqual(sizes.at(-1),[520,600,2]);assert.equal(requests.at(-1).width,520);assert.equal(paints.at(-1).pixelsPerPoint,2.5);
  assert.equal(mainCanvas.style.width,'100%');assert.equal(mainCanvas.width,300);
  assert.equal(parent.ironwoodOpenSidebarPopout(),true);assert.equal(opens,1);assert.equal(focused,1);
  parent.ironwoodDockSidebarPopout();
  assert.equal(parent.ironwoodSidebarPopoutOpen(),true,"Docking waits until rendering returns");
  await new Promise(resolve=>popup.setTimeout(resolve,10));
  assert.equal(parent.ironwoodSidebarPopoutOpen(),false);assert.equal(destroyed,1);assert.equal(docked,1);
  assert.equal(game.selectedMove,8);assert.equal(mainCanvas.style.width,'100%');
});

test('blocked or unavailable popup renderer leaves the main panel docked',()=>{
  const parent=new JSDOM('',{url:'http://localhost:8788/play/'}).window;
  let warning='';parent.open=()=>null;parent.alert=text=>{warning=text;};
  installSidebarPopout(parent);
  assert.equal(parent.ironwoodOpenSidebarPopout(),false);assert.match(warning,/Allow pop-ups/);
  assert.equal(parent.ironwoodSidebarPopoutOpen(),false);
  const popup=new JSDOM('',{url:'http://localhost:8788/play/'}).window;popup.closed=false;
  parent.open=()=>popup;
  installSidebarPopout(parent,{createRenderer:()=>{throw Error('Renderer unavailable');}});
  assert.equal(parent.ironwoodOpenSidebarPopout(),true);assert.equal(parent.ironwoodSidebarPopoutOpen(),false);
  assert.match(popup.document.getElementById('status').textContent,/Renderer unavailable/);
});
