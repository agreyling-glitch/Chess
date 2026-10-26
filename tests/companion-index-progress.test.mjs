import test from 'node:test';
import assert from 'node:assert/strict';
import {createCompanionIndexProgress} from '../web/companion-index-progress.js';
const status=(indexed,pending,extra={})=>({indexed,pending,total:indexed+pending,generation:100,failed:0,unsupported_variants:0,progress_version:1,...extra});
test('index progress groups counts and measures recent speed/ETA as the backlog changes',async()=>{
 let now=1000,value=status(1997623,2024294);const calls=[];
 const monitor=createCompanionIndexProgress({enabled:()=>true,visible:()=>true,connection:()=>({endpoint:'http://private',credential:'secret-token'}),clock:()=>now,client:()=>({request:async path=>{calls.push(path);return value;}})});
 await monitor.refresh();let p=JSON.parse(monitor.snapshot());assert.match(p.counts,/1,997,623 indexed.*2,024,294 remaining/);assert.equal(p.eta_seconds,null);
 now+=2000;value=status(1997823,2024094);await monitor.refresh();p=JSON.parse(monitor.snapshot());assert.equal(p.speed,100);assert.equal(p.eta_seconds,20241);assert.match(p.timing,/100 games\/s/);assert.doesNotMatch(monitor.snapshot(),/secret-token|private/);
 now+=2000;value=status(1998023,2030000);await monitor.refresh();p=JSON.parse(monitor.snapshot());assert.equal(p.remaining,2030000);assert.equal(p.eta_seconds,20300);assert.match(p.detail,/continuing imports/);
 now+=2000;value=status(0,4000000,{generation:101});await monitor.refresh();p=JSON.parse(monitor.snapshot());assert.equal(p.speed,0);assert.equal(p.eta_seconds,null);
 now+=2000;value=status(4000000,0);await monitor.refresh();assert.match(JSON.parse(monitor.snapshot()).timing,/Up to date/);assert.ok(calls.every(p=>p==='archive/explorer/status'));
});
test('gates polling, coalesces requests, backs off for old servers and rejects stale connections',async()=>{
 let now=1000,allowed=false,shown=true,connection={endpoint:'http://one',credential:'private'},resolve,calls=0;
 let request=()=>new Promise(yes=>resolve=yes);
 const monitor=createCompanionIndexProgress({enabled:()=>allowed,visible:()=>shown,connection:()=>connection,clock:()=>now,client:()=>({request:()=>{calls++;return request();}})});
 assert.equal(monitor.snapshot(),'{}');assert.equal(calls,0);allowed=true;shown=false;monitor.snapshot();assert.equal(calls,0);shown=true;
 monitor.snapshot();monitor.snapshot();assert.equal(calls,1);const oldResolve=resolve;
 connection={endpoint:'http://two',credential:'different'};monitor.snapshot();assert.equal(calls,2);resolve(status(20,80));await new Promise(r=>setTimeout(r,0));oldResolve(status(999,1));await new Promise(r=>setTimeout(r,0));assert.equal(JSON.parse(monitor.snapshot()).indexed,20);
 request=async()=>status(21,79,{progress_version:undefined});now+=2000;await monitor.refresh();const before=calls;now+=2000;monitor.snapshot();assert.equal(calls,before);
 now+=30000;request=async()=>{throw Object.assign(Error('busy'),{status:429,retryAfterMs:10000});};await monitor.refresh();let p=JSON.parse(monitor.snapshot());assert.match(p.counts,/waiting for archive/);assert.equal(p.eta_seconds,null);
 allowed=false;assert.equal(monitor.snapshot(),'{}');
});
