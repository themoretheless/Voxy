fn archive_gi_negative_duration(value:f32,slope:f32,error:f32,error_slope:f32,duration:f32)->f32 {
 let upper_a=value+error;let upper_b=slope+error_slope;
 if !(upper_a<0.) {return 0.;}
 if upper_b>0. {return min(duration,(-upper_a/upper_b)*0.999755859375);}
 return duration;
}
// Scheduling/rejection hint only. The original march retains its own samples.
// Bound both the original point+MVP evaluation and the affine hint, including
// coefficient arithmetic; never use the affine approximation for a hit test.
fn archive_gi_tile_skip_until(vp:mat4x4<f32>,viewport:vec2<f32>,origin:vec3<f32>,direction:vec3<f32>,t:f32,reach:f32,tile:vec2<i32>,minimum:f32,q:vec4<f32>)->f32 {
 if !(minimum>0.&&minimum<=1.)||q.w<0.0001||any(abs(origin)>vec3<f32>(1000000.)) {return t;}
 let l1=abs(vp[0])+abs(vp[1])+abs(vp[2]);
 let scale=max(max(l1.x,l1.y),max(l1.z,l1.w));
 let translation=max(max(abs(vp[3].x),abs(vp[3].y)),max(abs(vp[3].z),abs(vp[3].w)));
 let origin_radius=max(max(abs(origin.x),abs(origin.y)),abs(origin.z));
 let direction_radius=max(max(abs(direction.x),abs(direction.y)),abs(direction.z));
 let b=vp*vec4<f32>(direction,0.);
 let s0=scale*(origin_radius+direction_radius*t)+translation;
 if any(abs(q)>vec4<f32>(1e20))||any(abs(b)>vec4<f32>(1e20))||s0>1e20 {return t;}
 let epsilon=0.000003814697265625;
 let e0=(s0+2.*max(max(abs(q.x),abs(q.y)),max(abs(q.z),abs(q.w))))*epsilon+1e-30;
 let ed=(scale*direction_radius+2.*max(max(abs(b.x),abs(b.y)),max(abs(b.z),abs(b.w))))*epsilon+1e-30;
 // Shrink the pixel rectangle to cover Float32 division/UV/viewport rounding.
 let begin=vec2<f32>(tile*8)+0.25;
 let end=min(vec2<f32>((tile+1)*8),viewport)-0.25;
 let left=2.*begin.x/viewport.x-1.;let right=2.*end.x/viewport.x-1.;
 let top=1.-2.*begin.y/viewport.y;let bottom=1.-2.*end.y/viewport.y;
 // Four depth ULPs cover the quotient comparison; this is not a hit threshold.
 let d=bitcast<f32>(u32(max(i32(bitcast<u32>(minimum))-4,0)));
 // Explicit two-component planes avoid a dynamically indexed private array.
 var duration=max(reach-t,0.);
 duration=archive_gi_negative_duration(left*q.w-q.x,left*b.w-b.x,e0*(1.+abs(left)),ed*(1.+abs(left)),duration);
 duration=archive_gi_negative_duration(q.x-right*q.w,b.x-right*b.w,e0*(1.+abs(right)),ed*(1.+abs(right)),duration);
 duration=archive_gi_negative_duration(q.y-top*q.w,b.y-top*b.w,e0*(1.+abs(top)),ed*(1.+abs(top)),duration);
 duration=archive_gi_negative_duration(bottom*q.w-q.y,bottom*b.w-b.y,e0*(1.+abs(bottom)),ed*(1.+abs(bottom)),duration);
 duration=archive_gi_negative_duration(-q.w,-b.w,e0,ed,duration);
 duration=archive_gi_negative_duration(q.z-d*q.w,b.z-d*b.w,e0*(1.+d),ed*(1.+d),duration);
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