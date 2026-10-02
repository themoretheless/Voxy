import assert from 'node:assert/strict';
import {createHdrLoader} from '../hdr-environment.mjs';
let release;let calls=0;const events=[];
const loader=createHdrLoader({load:async bytes=>{calls++;assert.equal(bytes.length,4);await new Promise(resolve=>release=resolve);},busyChanged:b=>events.push(b),result:(ok,message)=>events.push([ok,message])});
const file={name:'studio.hdr',size:4,arrayBuffer:async()=>new Uint8Array(4).buffer};
const pending=loader.run(file);await Promise.resolve();await Promise.resolve();
assert.equal(loader.busy,true);assert.equal(await loader.run(file),false);assert.equal(calls,1);
release();assert.equal(await pending,true);assert.equal(loader.busy,false);assert.deepEqual(events,[true,[true,'studio.hdr'],false]);
let reads=0;
assert.equal(await loader.run({size:33*1024*1024,arrayBuffer:()=>reads++}),false);assert.equal(reads,0);
const bad=createHdrLoader({load:async()=>{throw new Error('bad RGBE');},busyChanged:b=>events.push(b),result:(ok,message)=>{assert.equal(ok,false);assert.match(message,/bad RGBE/);}});
assert.equal(await bad.run(file),false);assert.equal(bad.busy,false);
console.log('HDR loader: serialization, source limit, success and failure release passed');

let imported;let rejectImport=false;let sourceReads=0;
const recovery=createHdrLoader({
  load:async bytes=>{if(rejectImport) throw new Error('invalid environment'); imported=Array.from(bytes);bytes.fill(99);},
  busyChanged:()=>{},result:()=>{},
});
assert.equal(await recovery.restore(),false);
assert.equal(await recovery.run({name:'retained.hdr',size:3,arrayBuffer:async()=>{sourceReads++;return new Uint8Array([2,4,8]).buffer;}}),true);
assert.equal(recovery.hasEnvironment,true);
rejectImport=true;
assert.equal(await recovery.run({name:'bad.hdr',size:1,arrayBuffer:async()=>new Uint8Array([0]).buffer}),false);
assert.equal(await recovery.restore(async bytes=>{imported=Array.from(bytes);bytes.fill(0);}),true);
assert.deepEqual(imported,[2,4,8]);
assert.equal(sourceReads,1);
assert.equal(await recovery.restore(async()=>{throw new Error('replacement GPU lost');}),false);
assert.equal(recovery.busy,false);
assert.equal(await recovery.restore(async bytes=>{imported=Array.from(bytes);}),true);
assert.deepEqual(imported,[2,4,8]);
let releaseRestore;
const pendingRestore=recovery.restore(async()=>new Promise(resolve=>releaseRestore=resolve));
assert.equal(recovery.busy,true);
assert.equal(await recovery.restore(),false);
assert.equal(await recovery.run(file),false);
releaseRestore();assert.equal(await pendingRestore,true);
console.log('HDR recovery: retained ownership, failed replacement preservation, retry and serialization passed');
