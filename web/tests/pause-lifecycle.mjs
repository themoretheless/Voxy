import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Exercise the actual page lifecycle handler without a RAF callback. Browser
// suspension can stop RAF altogether, so a frame-only pause is insufficient.
const page=readFileSync(new URL('../index.html',import.meta.url),'utf8');
const start=page.indexOf('  const lifecycle = new AbortController();');
const end=page.indexOf('  let stopped=false;',start);
assert.ok(start>=0 && end>start);
let visibilityHandler;
const calls=[];
const state={environmentBusy:false,paused:false,appliedPause:null,last:123,jump:true,
  keys:new Set(['KeyW']),canvas:{dataset:{}},AbortController,
  document:{hidden:false,addEventListener(name,handler){
    assert.equal(name,'visibilitychange');visibilityHandler=handler;
  }},engine:{set_voxel_paused(value){calls.push(value);return 1;}}};
vm.createContext(state);
vm.runInContext(page.slice(start,end),state);
vm.runInContext('syncPause()',state);
assert.deepEqual(calls,[false]);
state.document.hidden=true;visibilityHandler();
assert.deepEqual(calls,[false,true]);
assert.equal(state.last,null);assert.equal(state.jump,false);
assert.equal(state.keys.size,0);assert.equal(state.canvas.dataset.paused,'true');
visibilityHandler();assert.equal(calls.length,2);
state.paused=true;state.document.hidden=false;visibilityHandler();
assert.equal(calls.length,2); // Returning to foreground preserves manual pause.
assert.equal(state.canvas.dataset.userPaused,'true');
assert.equal(state.canvas.dataset.visibilityPaused,'false');
state.paused=false;vm.runInContext('syncPause()',state);
assert.deepEqual(calls,[false,true,false]);
state.document.hidden=true;visibilityHandler();
state.document.hidden=false;visibilityHandler();
assert.deepEqual(calls,[false,true,false,true,false]);
console.log('pause lifecycle: RAF-independent hide, input reset, manual pause preservation, resume passed');

state.environmentBusy=true;state.document.hidden=true;
const before=calls.length;visibilityHandler();assert.equal(calls.length,before);
state.environmentBusy=false;vm.runInContext('syncPause()',state);assert.equal(calls.length,before+1);
console.log('pause lifecycle: no engine borrow during HDR loading passed');
