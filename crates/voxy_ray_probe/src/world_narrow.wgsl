enable wgpu_ray_query;
struct WorldRay {origin_hi:vec4<f32>,origin_lo:vec4<f32>,direction_hi:vec4<f32>,direction_lo:vec4<f32>,expected:vec4<u32>}
@group(0) @binding(0) var scene:acceleration_structure;
@group(0) @binding(1) var<storage,read> rays:array<WorldRay>;
@group(0) @binding(2) var<storage,read> vertices:array<f32>;
@group(0) @binding(3) var<storage,read> indices:array<u32>;
@group(0) @binding(4) var<storage,read> instances:array<f32>;
@group(0) @binding(5) var<storage,read> source_layout:array<vec4<u32>>;
@group(0) @binding(6) var<storage,read_write> output:array<vec4<u32>>;
struct WorldNarrowWorkQueue {count:atomic<u32>,ids:array<u32>}
@group(0) @binding(7) var<storage,read_write> narrow_work:WorldNarrowWorkQueue;
fn narrow_enqueue(index:u32) {let slot=atomicAdd(&narrow_work.count,1u);if slot<arrayLength(&narrow_work.ids) {narrow_work.ids[slot]=index;}}
struct WorldVertex {pair:WorldPair3,bounds:WorldInterval3,relative_bounds:WorldInterval3}
fn source_world_vertex(slot:u32,vertex:u32,origin:WorldPair3)->WorldVertex {
 let base=vertex*12u;let p=vec4<f32>(vertices[base],vertices[base+1u],vertices[base+2u],1.);
 let ib=slot*28u;
 let x=vec4<f32>(instances[ib],instances[ib+4u],instances[ib+8u],instances[ib+12u]);
 let y=vec4<f32>(instances[ib+1u],instances[ib+5u],instances[ib+9u],instances[ib+13u]);
 let z=vec4<f32>(instances[ib+2u],instances[ib+6u],instances[ib+10u],instances[ib+14u]);
 var ix=wi_scalar(0.);var iy=wi_scalar(0.);var iz=wi_scalar(0.);
 for(var k=0u;k<4u;k+=1u) {let value=wi_scalar(p[k]);ix=wi_add(ix,wi_mul(wi_scalar(x[k]),value));iy=wi_add(iy,wi_mul(wi_scalar(y[k]),value));iz=wi_add(iz,wi_mul(wi_scalar(z[k]),value));}
 let rx=world_dot4_minus_origin(x,p,origin.x);let ry=world_dot4_minus_origin(y,p,origin.y);let rz=world_dot4_minus_origin(z,p,origin.z);
 return WorldVertex(WorldPair3(rx.pair,ry.pair,rz.pair),WorldInterval3(ix,iy,iz),WorldInterval3(rx.bounds,ry.bounds,rz.bounds));
}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&rays) {return;}let ray=rays[id.x];
 if all(ray.direction_hi.xyz==vec3<f32>(0.)) {
  output[id.x*2u]=vec4<u32>(2u,0u,0u,0u);output[id.x*2u+1u]=vec4<u32>(0u,0u,0u,32u);narrow_enqueue(id.x);return;
 }

 let origin=WorldPair3(vec2<f32>(ray.origin_hi.x,ray.origin_lo.x),vec2<f32>(ray.origin_hi.y,ray.origin_lo.y),vec2<f32>(ray.origin_hi.z,ray.origin_lo.z));
 let direction=WorldPair3(vec2<f32>(ray.direction_hi.x,ray.direction_lo.x),vec2<f32>(ray.direction_hi.y,ray.direction_lo.y),vec2<f32>(ray.direction_hi.z,ray.direction_lo.z));
 var query:ray_query;rayQueryInitialize(&query,scene,RayDesc(0u,255u,0.,1.,ray.origin_hi.xyz,ray.direction_hi.xyz));
 var result=WorldTriangleResult(3u,0u,vec2<f32>(0.),vec2<f32>(0.),vec2<f32>(0.));
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);
  if candidate.kind!=3u||candidate.instance_custom_data!=ray.expected.x||candidate.primitive_index!=ray.expected.y {continue;}
  let source_info=source_layout[candidate.instance_custom_data];
  let start=source_info.z+candidate.primitive_index*3u;
  let p0=source_world_vertex(candidate.instance_custom_data,source_info.y+indices[start],origin);
  let p1=source_world_vertex(candidate.instance_custom_data,source_info.y+indices[start+1u],origin);
  let p2=source_world_vertex(candidate.instance_custom_data,source_info.y+indices[start+2u],origin);
  var ie1=wi3_sub(p1.bounds,p0.bounds);var ie2=wi3_sub(p2.bounds,p0.bounds);var delta=wi3_sub(wi_vec(origin),p0.bounds);
  let valid=p0.relative_bounds.x.valid&p0.relative_bounds.y.valid&p0.relative_bounds.z.valid&p1.relative_bounds.x.valid&p1.relative_bounds.y.valid&p1.relative_bounds.z.valid&p2.relative_bounds.x.valid&p2.relative_bounds.y.valid&p2.relative_bounds.z.valid;
  if valid!=0u {
   ie1=wi3_sub(p1.relative_bounds,p0.relative_bounds);ie2=wi3_sub(p2.relative_bounds,p0.relative_bounds);
   delta=WorldInterval3(wi_neg(p0.relative_bounds.x),wi_neg(p0.relative_bounds.y),wi_neg(p0.relative_bounds.z));
  }
  result=world_triangle_pair(wp_scalar(vec3<f32>(0.)),direction,p0.pair,p1.pair,p2.pair,ie1,ie2,delta,vec2<f32>(0.),vec2<f32>(1.,0.));
 }
 if result.state==2u {narrow_enqueue(id.x);}
 output[id.x*2u]=vec4<u32>(result.state,bitcast<u32>(result.t.x),bitcast<u32>(result.t.y),bitcast<u32>(result.u.x));
 output[id.x*2u+1u]=vec4<u32>(bitcast<u32>(result.u.y),bitcast<u32>(result.v.x),bitcast<u32>(result.v.y),result.reason);
}
