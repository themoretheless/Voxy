enable wgpu_ray_query;
// A temporary primitive ID only. The following source pass re-certifies the
// original triangle and still traverses every non-excluded source node.
override HINT_STRIDE:u32=5u;
@group(0) @binding(0) var<storage,read> rays:array<u32>;
@group(0) @binding(5) var<storage,read_write> hints:array<vec4<u32>>;
@group(0) @binding(8) var scene:acceleration_structure;
fn normal_or_zero(word:u32)->bool {
 let exponent=(word>>23u)&255u;
 return exponent!=255u&&(exponent!=0u||(word&0x7fffffffu)==0u);
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;let ray=index*20u;if ray+19u>=arrayLength(&rays) {return;}
 let out=index*HINT_STRIDE;
 hints[out]=vec4<u32>(4u,0u,0u,0u);
 hints[out+1u]=vec4<u32>(rays[ray+18u],rays[ray+19u],0xffffffffu,0xffffffffu);
 if rays[ray+3u]!=1u {return;}
 var origin:vec3<f32>;var direction:vec3<f32>;
 for(var axis=0u;axis<3u;axis++) {
  let o=rays[ray+axis];let d=rays[ray+8u+axis];
  if !normal_or_zero(o)||!normal_or_zero(d) {return;}
  origin[axis]=bitcast<f32>(o);direction[axis]=bitcast<f32>(d);
 }
 if all(direction==vec3<f32>(0.)) {return;}
 var query:ray_query;
 rayQueryInitialize(&query,scene,RayDesc(0u,255u,0.,1.,origin,direction));
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);
  if candidate.kind!=3u {continue;}
  hints[out+1u]=vec4<u32>(rays[ray+18u],rays[ray+19u],candidate.instance_custom_data,candidate.primitive_index);
  hints[out].x=1u;return;
 }
}
