import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const html=readFileSync(new URL('../index.html',import.meta.url),'utf8');
const start=html.indexOf('      // Restore validated animated settings');
const end=html.indexOf('      // End animated settings restoration.',start);
assert.ok(start>=0 && end>start);
const block=html.slice(start,end);
function restore(query) {
  const values={'reflector-roughness':['0.1','0.35','1'],'reflector-metallic':['0','0.5','1'],'environment-intensity':['0','0.5','1','2'],exposure:['0.25','1','4'],light:['0','35','70'],shadows:['1','0'],'shadow-filter':['0','1','2'],roughness:['0.1','0.35','1'],metallic:['0','0.5','1']};
  const controls=Object.fromEntries(Object.entries(values).map(([id,options])=>[id,{value:options[1],options:options.map(value=>({value}))}]));
  const calls=[];
  const canvas={dataset:{}};
  const context={time:0,paused:false,URLSearchParams,location:{search:query},canvas,document:{querySelector(selector){return controls[selector.slice(1)];}},engine:{
    set_animated_environment_intensity(value){calls.push(['environment',value]);},set_animated_exposure(value){calls.push(['exposure',value]);},set_animated_light(value){calls.push(['light',value]);},set_animated_shadows(value){calls.push(['shadows',value]);},set_animated_shadow_filter(value){calls.push(['filter',value]);},
  },updateReflector(){calls.push(['reflector',Number(controls['reflector-roughness'].value),Number(controls['reflector-metallic'].value)]);},updateMaterial(){calls.push(['material',Number(controls.roughness.value),Number(controls.metallic.value)]);}};
  vm.runInNewContext(block,context);
  return {controls,calls,canvas,time:context.time,paused:context.paused};
}
const result=restore('?exposure=4&lightIntensity=70&shadows=0&shadowFilter=2&roughness=1&metallic=0');
assert.deepEqual(result.calls.slice(0,4),[['exposure',4],['light',70],['shadows',false],['filter',2]]);
assert.deepEqual(result.calls.at(-1),['material',1,0]);
assert.equal(result.controls.roughness.value,'1');
assert.equal(result.controls.metallic.value,'0');
assert.deepEqual({...result.canvas.dataset},{exposure:'4',lightIntensity:'70',shadows:'0',shadowFilter:'2',roughness:'1',metallic:'0'});
assert.equal(restore('').calls.length,0);
for(const query of ['?exposure=NaN','?roughness=-1','?metallic=Infinity','?shadows=yes','?shadowFilter=99']) assert.throws(()=>restore(query),/unsupported/);
console.log('animated restoration: all settings applied, defaults retained, unsupported URL values rejected');

const held=restore('?animationTime=17.8912&userPaused=1');
assert.equal(held.time,17.8912); assert.equal(held.paused,true);
assert.equal(restore('?animationTime=0&userPaused=0').paused,false);
for(const query of ['?animationTime=NaN','?animationTime=-1','?animationTime=1e300','?animationTime=','?userPaused=yes']) assert.throws(()=>restore(query),/invalid animation/);
console.log('animated restoration: pose time/manual pause retained, nonfinite/negative/overflow times rejected');

const environment=restore('?environmentIntensity=2');
assert.deepEqual(environment.calls,[['environment',2]]);
assert.equal(environment.canvas.dataset.environmentIntensity,'2');
for(const query of ['?environmentIntensity=NaN','?environmentIntensity=-1','?environmentIntensity=Infinity']) assert.throws(()=>restore(query),/unsupported/);
console.log('environment intensity: restore and invalid settings rejection passed');

const reflector=restore('?reflectorRoughness=1&reflectorMetallic=0');
assert.deepEqual(reflector.calls.at(-1),['reflector',1,0]);
assert.equal(reflector.canvas.dataset.reflectorRoughness,'1');
assert.throws(()=>restore('?reflectorMetallic=NaN'),/unsupported/);
console.log('reflector material: independent restoration and invalid setting rejection passed');
