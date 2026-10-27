// Scheduling/rejection hint only. The original march retains its own samples.
// Bound both the original point+MVP evaluation and the affine hint, including
// coefficient arithmetic; never use the affine approximation for a hit test.
fn archive_gi_tile_skip_until(vp:mat4x4<f32>,viewport:vec2<f32>,origin:vec3<f32>,direction:vec3<f32>,t:f32,reach:f32,tile:vec2<i32>,minimum:f32,q:vec4<f32>)->f32 {
 if !(minimum>0.&&minimum<=1.)||q.w<0.0001||any(abs(origin)>vec3<f32>(1000000.)) {return t;}
 let magnitude=mat4x4<f32>(abs(vp[0]),abs(vp[1]),abs(vp[2]),abs(vp[3]));
 let b=vp*vec4<f32>(direction,0.);
 let s0=magnitude*vec4<f32>(abs(origin)+abs(direction)*t,1.);
 let sd=magnitude*vec4<f32>(abs(direction),0.);
 if any(abs(q)>vec4<f32>(1e20))||any(abs(b)>vec4<f32>(1e20))||any(s0>vec4<f32>(1e20))||any(sd>vec4<f32>(1e20)) {return t;}
 let epsilon=0.000003814697265625; // 64 Float32 unit roundoffs.
 let e0=(s0+abs(q))*epsilon+vec4<f32>(1e-30);
 let ed=(sd+abs(b))*epsilon+vec4<f32>(1e-30);
 // Shrink the pixel rectangle to cover Float32 division/UV/viewport rounding.
 let begin=vec2<f32>(tile*8)+0.25;
 let end=min(vec2<f32>((tile+1)*8),viewport)-0.25;
 let left=2.*begin.x/viewport.x-1.;let right=2.*end.x/viewport.x-1.;
 let top=1.-2.*begin.y/viewport.y;let bottom=1.-2.*end.y/viewport.y;
 // Four depth ULPs cover the quotient comparison; this is not a hit threshold.
 let d=bitcast<f32>(u32(max(i32(bitcast<u32>(minimum))-4,0)));
 let planes=array<vec4<f32>,6>(vec4<f32>(-1.,0.,0.,left),vec4<f32>(1.,0.,0.,-right),vec4<f32>(0.,1.,0.,-top),vec4<f32>(0.,-1.,0.,bottom),vec4<f32>(0.,0.,0.,-1.),vec4<f32>(0.,0.,1.,-d));
 var duration=max(reach-t,0.);
 for(var i=0u;i<6u;i++) {
  let p=planes[i];let a=dot(p,q);let slope=dot(p,b);
  let upper_a=a+dot(abs(p),e0)+epsilon*dot(abs(p),abs(q));
  let upper_b=slope+dot(abs(p),ed)+epsilon*dot(abs(p),abs(b));
  if !(upper_a<0.) {return t;}
  if upper_b>0. {duration=min(duration,(-upper_a/upper_b)*0.999755859375);}
 }
 return max(t,min(reach,t+duration*0.999755859375-max(0.000001,t*0.00001)));
}


struct Input {vp:mat4x4<f32>,origin:vec4<f32>,direction:vec4<f32>,times:vec4<f32>}
@group(0) @binding(0) var<storage,read> samples:array<Input>;
@group(0) @binding(1) var<storage,read_write> results:array<vec4<u32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=arrayLength(&samples){return;}
 let s=samples[id.x];let viewport=s.times.yz;let q=s.vp*vec4<f32>(s.origin.xyz+s.direction.xyz*s.times.x,1.);
 let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;let pixel=vec2<i32>(uv*viewport);let tile=pixel/8;
 let minimum=clamp(q.z/q.w+s.times.w,0.,1.);
 let end=archive_gi_tile_skip_until(s.vp,viewport,s.origin.xyz,s.direction.xyz,s.times.x,60.,tile,minimum,q);
 var t=s.times.x;var checked=0u;var bad=0u;
 for(var i=0u;i<80u;i++) {
  t+=0.04+0.06*t;if !(t<end){break;}
  checked++;let clip=s.vp*vec4<f32>(s.origin.xyz+s.direction.xyz*t,1.);
  let coord=clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5;
  if !(clip.w>0.)||any(coord<=vec2<f32>(0.))||any(coord>=vec2<f32>(1.))||any(vec2<i32>(coord*viewport)/8!=tile)||!(clip.z/clip.w<minimum){bad++;}
 }
 results[id.x]=vec4<u32>(select(0u,1u,checked>0u),checked,bad,0u);
}