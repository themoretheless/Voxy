// Percentage-closer soft shadows for the orthographic sun map: a blocker
// search sizes the penumbra from the sun's angular radius, so contact
// shadows stay sharp and distant occluders blur (Fernando 2005).
const SUN_TAN_HALF_ANGLE:f32=0.00465;
const PCSS_TAPS:array<vec2<f32>,12>=array<vec2<f32>,12>(
 vec2<f32>(-0.326,-0.406),vec2<f32>(-0.840,-0.074),vec2<f32>(-0.696,0.457),vec2<f32>(-0.203,0.621),
 vec2<f32>(0.962,-0.195),vec2<f32>(0.473,-0.480),vec2<f32>(0.519,0.767),vec2<f32>(0.185,-0.893),
 vec2<f32>(0.507,0.064),vec2<f32>(0.896,0.412),vec2<f32>(-0.322,-0.933),vec2<f32>(-0.792,-0.598));
fn soft_sun_visibility(shadow:mat4x4<f32>,uv:vec2<f32>,depth:f32,noise:f32)->f32{
 let size=vec2<f32>(textureDimensions(shadow_map));
 // Orthographic light: world metres per uv unit and per depth unit.
 let metres_per_uv=2./length(vec3<f32>(shadow[0].x,shadow[1].x,shadow[2].x));
 let search=clamp(0.6/metres_per_uv,2./size.x,48./size.x);
 let rotation=noise*6.2831853;let c=cos(rotation);let s=sin(rotation);
 let spin=mat2x2<f32>(c,s,-s,c);
 let bias_depth=depth-0.00004;
 var blockers=0.;var blocker_depth=0.;
 for(var i=0;i<12;i++){
  let p=clamp(vec2<i32>((uv+spin*PCSS_TAPS[i]*search)*size),vec2<i32>(0),vec2<i32>(size)-1);
  let z=textureLoad(shadow_map,p,0);
  if z<bias_depth {blockers+=1.;blocker_depth+=z;}
 }
 if blockers==0. {return 1.;}
 blocker_depth/=blockers;
 let metres_per_depth=1./length(vec3<f32>(shadow[0].z,shadow[1].z,shadow[2].z));
 let penumbra_metres=(depth-blocker_depth)*metres_per_depth*2.*SUN_TAN_HALF_ANGLE;
 let radius=clamp(penumbra_metres/metres_per_uv,1./size.x,search);
 var visibility=0.;
 for(var i=0;i<12;i++){
  visibility+=textureSampleCompareLevel(shadow_map,shadow_sampler,uv+spin*PCSS_TAPS[i]*radius,bias_depth);
 }
 return visibility*(1./12.);
}
