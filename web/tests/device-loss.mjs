import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../index.html',import.meta.url),'utf8');
const start=source.indexOf('  let hdrLoader=null;');
const end=source.indexOf('    syncPause();',start);
assert.ok(start>=0 && end>start);
let freed=0,aborted=0,submissions=0;
let contextLost;
const controls=[{disabled:false},{disabled:false}];
const recovery={hidden:true};
const context={animated:false,environmentBusy:false,engine:{device_failure(){return 'GPU device lost (Destroyed): acceptance';},free(){freed++;}},
 window:{voxyDemo:{frames:30}},canvas:{dataset:{},addEventListener(name,handler){assert.equal(name,"webglcontextlost");contextLost=handler;}},status:{textContent:''},
 lifecycle:{abort(){aborted++;}},URL,location:{href:"http://localhost/?backend=webgpu&voxel=1&deviceLossCheck=1"},
 document:{querySelectorAll(){return controls;},querySelector(){return recovery;}}};
vm.createContext(context);
vm.runInContext(source.slice(start,end)+'submissionsMustNotRun();\n}\nframe(123);',Object.assign(context,{submissionsMustNotRun(){submissions++;}}));
assert.equal(recovery.hidden,false);
assert.equal(recovery.href,"http://localhost/?backend=webgpu&voxel=1");
let prevented=0; contextLost({preventDefault(){prevented++;}});
vm.runInContext('frame(456)',context);
assert.equal(prevented,1);
assert.equal(freed,1);assert.equal(aborted,1);assert.equal(submissions,0);
assert.ok(controls.every(control=>control.disabled));
assert.equal(context.window.voxyDemo.frames,30);
assert.equal(context.canvas.dataset.deviceError,context.status.textContent);
assert.equal(context.window.voxyDemo.error,context.status.textContent);
console.log('device loss: diagnostic, disabled controls, cleanup and no subsequent submissions passed');

// Other terminal frame errors must use the same owner, including after a
// callback has already stopped the engine. The first diagnostic is retained.
const firstDiagnostic=context.status.textContent;
vm.runInContext("stopEngine('later rendering failure'); frame(789)",context);
assert.equal(freed,1);assert.equal(aborted,1);
assert.equal(context.status.textContent,firstDiagnostic);
console.log('terminal frame failures: shared owner and first-diagnostic preservation passed');

// Loss recovery carries the current HDR settings rather than stale URL defaults.
context.canvas.dataset={scene:'animated-skeleton',hdr:'true',exposure:'4',environmentIntensity:'2',reflectorRoughness:'0.1',reflectorMetallic:'1',lightIntensity:'70',shadows:'0',shadowFilter:'2',roughness:'1',metallic:'0',time:'17.8912',userPaused:'true'};
context.location.href='http://localhost/?backend=webgpu&animated=1&deviceLossCheck=1&roughness=0.1';
context.canvas.dataset.temporalGuides='enabled';
vm.runInContext('stopped=false; stopEngine("acceptance restart");',context);
const resumed=new URL(recovery.href);
assert.equal(resumed.searchParams.has('deviceLossCheck'),false);
assert.equal(resumed.searchParams.get('temporalGuides'),'1');
for(const key of ['exposure','lightIntensity','environmentIntensity','reflectorRoughness','reflectorMetallic','shadows','shadowFilter','roughness','metallic']) assert.equal(resumed.searchParams.get(key),context.canvas.dataset[key]);
console.log('animated recovery: current HDR/material/light/shadow settings retained, injection flag removed');

assert.equal(resumed.searchParams.get('animationTime'),'17.8912');
assert.equal(resumed.searchParams.get('userPaused'),'1');
console.log('animated recovery: pose time and manual pause snapshot retained');
context.canvas.dataset.temporalGuides='disabled';
context.location.href='http://localhost/?backend=webgpu&animated=1&temporalGuides=1';
vm.runInContext('stopped=false; stopEngine("temporal disabled");',context);
assert.equal(new URL(recovery.href).searchParams.has('temporalGuides'),false);
console.log('temporal recovery: disabled mode removes stale enabled URL flag');


// A frame scheduled during an async HDR import must not touch the borrowed engine.
const priorFreed=freed,priorSubmissions=submissions;
let scheduled=0;context.requestAnimationFrame=()=>scheduled++;context.last=123;
vm.runInContext('stopped=false;environmentBusy=true;frame(789);',context);
assert.equal(scheduled,1);assert.equal(context.last,null);assert.equal(freed,priorFreed);assert.equal(submissions,priorSubmissions);
vm.runInContext('stopped=true;environmentBusy=false;',context);
console.log('HDR loading: RAF defers GPU diagnostics and submissions during async borrow');
