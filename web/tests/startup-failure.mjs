import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../index.html',import.meta.url),'utf8');
const start=source.indexOf('let startupEngine=null;');
const end=source.indexOf('const keys=new Set();',start);
assert.ok(start>=0 && end>start);
for(const created of [false,true]) {
 let freed=0;
 const controls=[{disabled:false}];
 const context={window:{voxyDemo:{}},canvas:{dataset:{}},status:{},
  owner:{free(){freed++;}},document:{querySelectorAll(){return controls;}}};
 vm.createContext(context);vm.runInContext(source.slice(start,end),context);
 if(created) vm.runInContext('startupEngine=owner',context);
 vm.runInContext("failStartup('startup rejected'); failStartup('startup rejected');",context);
 assert.equal(freed,Number(created));assert.equal(controls[0].disabled,true);
 assert.equal(context.canvas.dataset.startupError,'startup rejected');
 assert.equal(context.status.textContent,context.window.voxyDemo.error);
}
console.log('startup failure: before/after creation, disabled controls and single cleanup passed');
