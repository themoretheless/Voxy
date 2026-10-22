enable wgpu_ray_query;
// A temporary primitive ID only. The following source pass re-certifies the
// original triangle and still traverses every non-excluded source node.
override HINT_STRIDE:u32=5u;
@group(0) @binding(0) var<storage,read> rays:array<u32>;
@group(0) @binding(1) var<storage,read> vertices:array<u32>;
@group(0) @binding(2) var<storage,read> indices:array<u32>;
@group(0) @binding(3) var<storage,read> instances:array<u32>;
@group(0) @binding(4) var<storage,read> source_layout:array<vec4<u32>>;
@group(0) @binding(5) var<storage,read_write> hints:array<vec4<u32>>;
@group(0) @binding(8) var scene:acceleration_structure;
struct HintCounts {values:array<vec4<u32>,4096>}
@group(0) @binding(9) var<uniform> counts:HintCounts;
fn normal_or_zero(word:u32)->bool {
 let exponent=(word>>23u)&255u;
 return exponent!=255u&&(exponent!=0u||(word&0x7fffffffu)==0u);
}
// Experimental, default-off: rounded arithmetic ranks optional IDs, never certifies a hit.
fn finite_scalar(x:f32)->bool {return (bitcast<u32>(x)&0x7f800000u)!=0x7f800000u;}
fn hint_vertex(slot:u32,vertex:u32)->vec3<f32> {
 var p:vec3<f32>;
 for(var axis=0u;axis<3u;axis++) {
  var value=bitcast<f32>(instances[slot*28u+12u+axis]);
  for(var k=0u;k<3u;k++) {
   value+=bitcast<f32>(instances[slot*28u+k*4u+axis])*bitcast<f32>(vertices[vertex*12u+k]);
  }
  p[axis]=value;
 }
 return p;
}
fn approximate_t(slot:u32,primitive:u32,o:vec3<f32>,d:vec3<f32>)->f32 {
 if slot>=arrayLength(&source_layout)||slot>=arrayLength(&instances)/28u {return -1.;}
 let info=source_layout[slot];
 if info.x>=2047u {return -1.;}
 if primitive>=counts.values[info.x*2u].x {return -1.;}
 let count=arrayLength(&indices);
 if info.z>count {return -1.;}
 if primitive>=(count-info.z)/3u {return -1.;}
 let first=info.z+primitive*3u;var points:array<vec3<f32>,3>;
 let vertex_count=arrayLength(&vertices)/12u;
 if info.y>=vertex_count {return -1.;}
 for(var k=0u;k<3u;k++) {
  let local=indices[first+k];if local>=vertex_count-info.y {return -1.;}
  points[k]=hint_vertex(slot,info.y+local);
 }
 let e=points[1]-points[0];let f=points[2]-points[0];
 let p=cross(d,f);let determinant=dot(e,p);
 if determinant==0.||!finite_scalar(determinant) {return -1.;}
 let delta=o-points[0];let u=dot(delta,p)/determinant;
 let q=cross(delta,e);let v=dot(d,q)/determinant;let t=dot(f,q)/determinant;
 if !finite_scalar(u)||!finite_scalar(v)||!finite_scalar(t) {return -1.;}
 if u<0.||v<0.||u+v>1.||t<0.||t>1. {return -1.;}
 return t;
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
 var best=2.;
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);
  if candidate.kind!=3u {continue;}
  let t=approximate_t(candidate.instance_custom_data,candidate.primitive_index,origin,direction);
  if t<0.||t>=best {continue;}best=t;
  hints[out+1u]=vec4<u32>(rays[ray+18u],rays[ray+19u],candidate.instance_custom_data,candidate.primitive_index);
  hints[out].x=1u;
 }
}
