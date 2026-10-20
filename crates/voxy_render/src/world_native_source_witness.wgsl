// Hardware enumeration has no coverage authority. Only a strict original-source
// certificate may publish temporary status 7 for this encode's history consumer.
override HINT_STRIDE:u32=5u;
@group(0) @binding(8) var witness_scene:acceleration_structure;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;let ray=index*20u;if ray+19u>=exact_ray_word_count() {return;}
 nearest_admission_result(index,HINT_STRIDE,4u,0u);
 if exact_ray_word(ray+3u)!=1u {return;}
 var origin:vec3<f32>;var direction:vec3<f32>;
 for(var axis=0u;axis<3u;axis++) {
  for(var part=0u;part<4u;part++) {if ((exact_ray_word(ray+part*4u+axis)>>23u)&255u)==255u {return;}}
  let o=exact_ray_word(ray+axis);let d=exact_ray_word(ray+8u+axis);
  if !wi_normal_bits(o)||!wi_normal_bits(d) {return;}
  origin[axis]=bitcast<f32>(o);direction[axis]=bitcast<f32>(d);
  nearest_node_origin[axis]=wi_add(wi_source_word(o),wi_source_word(exact_ray_word(ray+4u+axis)));
  nearest_node_direction[axis]=wi_add(wi_source_word(d),wi_source_word(exact_ray_word(ray+12u+axis)));
  if (nearest_node_origin[axis].valid&nearest_node_direction[axis].valid)==0u {return;}
 }
 if all(direction==vec3<f32>(0.)) {return;}
 var query:ray_query;
 rayQueryInitialize(&query,witness_scene,RayDesc(0u,255u,0.,1.,origin,direction));
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);if candidate.kind!=3u {continue;}
  let slot=candidate.instance_custom_data;let primitive=candidate.primitive_index;
  if slot>=nearest_active[0]||nearest_active[4u+slot]==0u {continue;}
  let info=exact_layout[slot];if primitive>=nearest_counts.values[info.x*2u].x {continue;}
  let material=visibility_material(info.w);if material!=1u&&material!=2u {continue;}
  let proof=source_triangle_certificate(slot,info.z+primitive*3u,ray,false);
  if proof.intersection.state!=1u {continue;}
  let out=index*HINT_STRIDE;
  exact_output[out]=vec4<u32>(7u,8192u,2048u,1u);
  exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),slot,primitive);
  return;
 }
 // The following complete source pass handles absence and every inconclusive
 // candidate. A high-only hardware ray never certifies a paired-source miss.
}
