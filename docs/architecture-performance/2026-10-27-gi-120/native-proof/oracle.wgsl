fn dda_finite4(v:vec4<f32>)->bool {return !any((bitcast<vec4<u32>>(v)&vec4<u32>(0x7f800000u))==vec4<u32>(0x7f800000u));}
fn gi_batch_in_front(origin:vec3<f32>,direction:vec3<f32>,t0:f32,t1:f32)->bool {
 if !dda_finite4(vec4<f32>(origin,t0))||!dda_finite4(vec4<f32>(direction,t1)){return false;}
 let p0=origin+direction*t0;let p1=origin+direction*t1;
 let world_error=(abs(origin)+abs(direction)*max(abs(t0),abs(t1)))*0.00000095367431640625+vec3<f32>(1e-30);
 let centre=(p0+p1)*0.5;let radius=abs(p1-p0)*0.5;
 let absolute=mat4x4<f32>(abs(camera.vp[0]),abs(camera.vp[1]),abs(camera.vp[2]),abs(camera.vp[3]));
 let clip=camera.vp*vec4<f32>(centre,1.);
 let spread=absolute*vec4<f32>(radius,0.);
 // 64 float32 epsilons cover endpoint-box arithmetic, dot-product
 // reassociation/FMA, and the outward clip endpoints. Underflow is padded.
 let error=(absolute*vec4<f32>(max(abs(p0),abs(p1)),1.))*0.00000762939453125
  +absolute*vec4<f32>(world_error,0.)+vec4<f32>(1e-30);
 let lo=clip-spread-error;let hi=clip+spread+error;
 if !dda_finite4(lo)||!dda_finite4(hi)||lo.w<=0. {return false;}
 let ndc_lo=min(lo.xyz/lo.w,lo.xyz/hi.w);
 let ndc_hi=max(hi.xyz/lo.w,hi.xyz/hi.w);
 let margin=(max(abs(ndc_lo),abs(ndc_hi))+1.)*0.000003814697265625;
 let lower=ndc_lo-margin;let upper=ndc_hi+margin;
 if !dda_finite4(vec4<f32>(lower,1.))||!dda_finite4(vec4<f32>(upper,1.)){return false;}
 let uv_lo=vec2<f32>(lower.x,-upper.y)*0.5+0.5;
 let uv_hi=vec2<f32>(upper.x,-lower.y)*0.5+0.5;
 if any(uv_lo<=vec2<f32>(0.))||any(uv_hi>=vec2<f32>(1.)){return false;}
 let a=vec2<u32>(floor(uv_lo*camera.viewport.xy))/8u;
 let b=vec2<u32>(floor(uv_hi*camera.viewport.xy))/8u;
 let level=32u-countLeadingZeros((a.x^b.x)|(a.y^b.y));
 if level>=textureNumLevels(ray_depth_bounds){return false;}
 let tile=a>>vec2<u32>(level);
 let minimum=textureLoad(ray_depth_bounds,vec2<i32>(tile),i32(level)).x;
 return upper.z<minimum;
}

struct Camera {vp:mat4x4<f32>,viewport:vec4<f32>}
var<private> camera:Camera;
struct Input {vp:mat4x4<f32>,origin:vec4<f32>,direction:vec4<f32>,times:vec4<f32>}
@group(0) @binding(0) var<storage,read> samples:array<Input>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
@group(0) @binding(2) var ray_depth_bounds:texture_2d<f32>;
@group(0) @binding(3) var original_depth:texture_2d<f32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=arrayLength(&samples){return;}let s=samples[id.x];
 camera=Camera(s.vp,vec4<f32>(259.,131.,0.,0.));
 var end=s.times.x;for(var i=1u;i<8u;i++){end+=0.04+0.06*end;}
 let accepted=gi_batch_in_front(s.origin.xyz,s.direction.xyz,s.times.x,end);
 var bad=0u;var t=s.times.x;
 if accepted {for(var i=0u;i<8u;i++){
  let q=camera.vp*vec4<f32>(s.origin.xyz+s.direction.xyz*t,1.);
  let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
  if !dda_finite4(q)||q.w<=0.||any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){bad+=1u;}
  else{let z=textureLoad(original_depth,vec2<i32>(uv*camera.viewport.xy),0).x;if z<1.&&q.z/q.w>=z{bad+=1u;}}
  t+=0.04+0.06*t;
 }}
 result[id.x]=vec4<u32>(u32(accepted),bad,0u,0u);
}