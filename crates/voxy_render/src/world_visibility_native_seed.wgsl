// Hardware supplies only a source primitive ID hint. Its float ray, distance,
// traversal order and absence are never authoritative for exact coverage.
@group(0) @binding(8) var fast_seed_scene:acceleration_structure;
fn visibility_seed(ray:u32) {
 if !WORLD_FAST_DEFER_NONCLEAR||!WORLD_FAST_CAN_BOUND {return;}
 var origin:vec3<f32>;var direction:vec3<f32>;
 for(var axis=0u;axis<3u;axis++) {
  origin[axis]=bitcast<f32>(exact_ray_word(ray+axis));direction[axis]=bitcast<f32>(exact_ray_word(ray+8u+axis));
  if !wi_normal_or_zero(origin[axis])||!wi_normal_or_zero(direction[axis]) {return;}
 }
 if all(direction==vec3<f32>(0.)) {return;}
 var query:ray_query;rayQueryInitialize(&query,fast_seed_scene,RayDesc(0u,255u,0.,1.,origin,direction));
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);if candidate.kind!=3u {continue;}
  let slot=candidate.instance_custom_data;let primitive=candidate.primitive_index;
  if visibility_try_seed(slot,primitive,ray) {return;}
 }
}
