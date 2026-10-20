// Renderer-only, admitted by the prepared all-material fallback policy.
@group(1) @binding(22) var<storage,read_write> world_consumer_colours:array<vec4<u32>>;
override WORLD_CONSUMER_CACHE_CHECK:bool=false;
fn world_consumer_check(index:u32,ray:ArchiveRayInput) {
 if WORLD_CONSUMER_CACHE_CHECK {
  let original=bitcast<vec4<u32>>(archive_world_query_fallback(ray));
  atomicAdd(&world_resolve_audit[9],1u);
  if any(original!=world_consumer_colours[index]) {atomicAdd(&world_resolve_audit[10],1u);}
 }
}
fn world_consumer_cached(index:u32,ray:ArchiveRayInput)->vec4<f32> {
 world_consumer_check(index,ray);return bitcast<vec4<f32>>(world_consumer_colours[index]);
}
fn world_consumer_pruned(index:u32,admission:u32,ray:ArchiveRayInput)->bool {
 if admission!=0u||any(world_consumer_colours[index]!=vec4<u32>(0u)) {return false;}
 world_consumer_check(index,ray);return true;
}
@compute @workgroup_size(8,8)
fn cs_world_consumer_cache(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let index=id.y*ray_tile.extent.x+id.x;
 let ray=archive_ray_input(ray_tile.offset+id.xy);var colour=vec4<f32>(0.);
 if ray.admitted {colour=archive_world_query_fallback(ray);}
 world_consumer_store(index,ray,colour);
}
fn world_consumer_store(index:u32,ray:ArchiveRayInput,colour:vec4<f32>) {
 let base=index*20u;
 let bits=bitcast<vec4<u32>>(colour);world_consumer_colours[index]=bits;
 if ray.admitted&&world_exact_ray_words[base+3u]==1u&&all(bits==vec4<u32>(0u)) {
  // Consumer-irrelevant, not a geometric miss. Preserve all coefficients/IDs.
  world_exact_ray_words[base+3u]=0u;
  if WORLD_CONSUMER_CACHE_CHECK||ARCHIVE_WORLD_RESOLVE_AUDIT {atomicAdd(&world_resolve_audit[8],1u);}
 }
 if ray.admitted&&any((bits&vec4<u32>(0x7fffffffu))>=vec4<u32>(0x7f800000u)) {atomicAdd(&world_resolve_audit[11],1u);}
}

// Shared projection authority for prepared consumer schedules.
fn consumer_project(ray:ArchiveRayInput)->DdaProjectedSegment {
 var segment=dda_projection_empty(0u);
 if ray.admitted&&!(ray.history&&camera.surface.z<0.5) {
  segment=dda_projection_empty(2u);
  if ARCHIVE_PAIRED_DDA&&!ARCHIVE_RAY_FORCE_FALLBACK {segment=dda_project_segment(camera.vp,ray.origin,ray.origin+ray.direction*60.,camera.viewport.xy);}
 }
 segment.padding0=select(0u,1u,ray.history);return segment;
}
