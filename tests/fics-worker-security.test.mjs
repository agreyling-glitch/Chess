import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {randomUUID} from 'node:crypto';

const source=readFileSync(new URL('../src/wrangler-worker.js',import.meta.url),'utf8')
 .replace('import { connect } from "cloudflare:sockets";','')
 .replace('export default','globalThis.worker =')
 .replace('export class FicsBridge','globalThis.FicsBridge = class FicsBridge');
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function fixture({stall=false,fail=false}={}) {
 let now=0,nextTimer=0;const timers=new Map(),sockets=[];
 class FakeWebSocket {
  constructor(){this.handlers={};this.sent=[];this.bufferedAmount=0;this.closed=false;}
  accept(){} addEventListener(name,fn){this.handlers[name]=fn;}
  send(data){this.sent.push(data);} close(){this.closed=true;}
  message(data){this.handlers.message({data});}
 }
 const context=vm.createContext({URL,Request,Headers,TextEncoder,TextDecoder,crypto:{randomUUID},
  Date:class extends Date {static now(){return now;}},
  Response:class {constructor(body,options){if(options?.status===101){Object.assign(this,options);return;}return new Response(body,options);}},
  WebSocketPair:class {constructor(){this[0]=new FakeWebSocket();this[1]=new FakeWebSocket();}},
  setTimeout:(fn,ms)=>{const id=++nextTimer;timers.set(id,{fn,ms});return id;},
  setInterval:(fn,ms)=>{const id=++nextTimer;timers.set(id,{fn,ms,interval:true});return id;},
  clearTimeout:id=>timers.delete(id),clearInterval:id=>timers.delete(id),
  connect:()=>{
   if(fail)throw Error('unavailable');
   let controller;const socket={writes:[],readable:new ReadableStream({start(c){controller=c;}}),
    writable:new WritableStream({write(data){socket.writes.push(new TextDecoder().decode(data));if(stall)return new Promise(()=>{});}}),
    close(){if(!socket.closed){socket.closed=true;controller.close();}},
    output(text){controller.enqueue(new TextEncoder().encode(text));}};
   sockets.push(socket);return socket;
  }
 });
 vm.runInContext(source,context);const bridge=new context.FicsBridge();
 const request=(ip='192.0.2.1')=>new Request('https://ironwoodchess.com/fics/socket',{headers:{Upgrade:'websocket',Origin:'https://ironwoodchess.com','CF-Connecting-IP':ip}});
 const open=(ip)=>{const result=bridge.fetch(request(ip));if(result.status===101){const socket=sockets.at(-1);socket.server=result.webSocket;/* capture server through pair below */}return result;};
 // Capture each server half separately from the client response.
 const pairs=[];context.WebSocketPair=class {constructor(){this[0]=new FakeWebSocket();this[1]=new FakeWebSocket();pairs.push(this);}};
 return {bridge,request,open,sockets,pairs,timers,advance(ms){now+=ms;},
  fire(ms){for(const [id,timer] of [...timers])if(timer.ms===ms){if(!timer.interval)timers.delete(id);timer.fn();}},
  close(){for(const pair of pairs)pair[1].handlers.close();}};
}

test('global and per-client admission refuse excess connections before TCP allocation',()=>{
 const f=fixture();try {
  for(let i=0;i<4;i++)assert.equal(f.open().status,101);
  assert.equal(f.open().status,429);assert.equal(f.sockets.length,4);
  for(let i=4;i<128;i++)assert.equal(f.open(`192.0.2.${i}`).status,101);
  assert.equal(f.open('198.51.100.1').status,429);assert.equal(f.sockets.length,128);
 }finally{f.close();}
});
test('login/idle/lifetime deadlines and failed upstream release admission',async()=>{
 const f=fixture();f.open();assert.equal(f.bridge.active.size,1);f.fire(120_000);assert.equal(f.bridge.active.size,0);assert.ok(f.sockets[0].closed);
 f.open();f.sockets[1].output('fics% ');await tick();assert.ok(![...f.timers.values()].some(t=>t.ms===120_000));
 f.advance(31*60_000);f.fire(10_000);assert.equal(f.bridge.active.size,0);
 f.open();f.fire(6*60*60_000);assert.equal(f.bridge.active.size,0);f.close();
 const failed=fixture({fail:true});assert.equal(failed.open().status,502);assert.equal(failed.bridge.active.size,0);
});
test('oversized frames, command floods, slow writes and slow readers are bounded',async()=>{
 for(const mode of ['frame','flood','write','reader','output']){
  const f=fixture({stall:mode==='write'});f.open();const server=f.pairs[0][1],socket=f.sockets[0];
  if(mode==='frame')server.message('{'+ 'x'.repeat(1024));
  if(mode==='flood')for(let i=0;i<121;i++)server.message('');
  if(mode==='write'){server.message('guest');await tick();f.fire(10_000);await tick();}
  if(mode==='reader'){server.bufferedAmount=1024*1024+1;socket.output('login: ');await tick();}
  if(mode==='output'){socket.output('x'.repeat(128*1024+1));await tick();}
  assert.equal(f.bridge.active.size,0,mode);assert.ok(server.closed,mode);f.close();
 }
});
test('guest and registered login, console commands and read flow remain available',async()=>{
 const f=fixture();try{
  f.open();const server=f.pairs[0][1],socket=f.sockets[0];
  server.message('Alice');await tick();socket.output('Password: ');await tick();
  server.message(JSON.stringify({type:'password',value:'unique-test-password'}));await tick();
  socket.output('fics% ');await tick();server.message(JSON.stringify({type:'command',value:'observe 42'}));await tick();
  assert.deepEqual(socket.writes,['Alice\n','unique-test-password\n','observe 42\n']);assert.equal(f.bridge.active.size,1);
  server.message(JSON.stringify({type:'password',value:'again'}));assert.equal(f.bridge.active.size,0);
  f.open('198.51.100.2');f.pairs[1][1].message('guest');await tick();assert.equal(f.sockets[1].writes[0],'guest\n');
  f.pairs[1][1].message(JSON.stringify({type:'password',value:'not-allowed'}));assert.equal(f.bridge.active.size,0);
 }finally{f.close();}
});

test('unread output requires real receipts, stays bounded and expires promptly',async()=>{
 const f=fixture();try{
  f.open();const socket=f.sockets[0],server=f.pairs[0][1];socket.output('login: ');await tick();
  const envelope=JSON.parse(server.sent[0]);assert.equal(envelope.type,'fics-data');
  server.message(JSON.stringify({type:'ack',receipt:envelope.receipt}));
  f.advance(31_000);f.fire(10_000);assert.equal(f.bridge.active.size,1,'Acknowledged output must not time out');
  socket.output('still here');await tick();f.advance(31_000);f.fire(10_000);assert.equal(f.bridge.active.size,0);
  f.open('192.0.2.55');f.pairs[1][1].message(JSON.stringify({type:'ack',receipt:'guessed'}));assert.equal(f.bridge.active.size,0);
  f.open('192.0.2.56');for(let i=0;i<6;i++){f.sockets[2].output('x'.repeat(100*1024));await tick();f.advance(1000);}
  assert.equal(f.bridge.active.size,0,'Unacknowledged output cannot grow past 512 KiB');
 }finally{f.close();}
});
