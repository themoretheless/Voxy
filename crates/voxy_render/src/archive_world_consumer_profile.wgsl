// All 44 segment words are retained in three independent storage bindings.
// Chunking the record, not its precision, admits full-width 3840x432 strips.
@group(1) @binding(23) var<storage,read_write> consumer_segment_a:array<vec4<u32>>;
@group(1) @binding(24) var<storage,read_write> consumer_segment_b:array<vec4<u32>>;
@group(1) @binding(25) var<storage,read_write> consumer_segment_c:array<vec4<u32>>;
@group(1) @binding(26) var<storage,read_write> consumer_hits:array<DepthDdaPairHit>;
fn consumer_segment_store(index:u32,s:DdaProjectedSegment) {
 let a=index*4u;let c=index*3u;
 consumer_segment_a[a]=bitcast<vec4<u32>>(s.begin);consumer_segment_a[a+1u]=bitcast<vec4<u32>>(s.end);
 consumer_segment_a[a+2u]=bitcast<vec4<u32>>(s.world_begin);consumer_segment_a[a+3u]=bitcast<vec4<u32>>(s.world_end);
 consumer_segment_b[a]=bitcast<vec4<u32>>(s.world_begin_low);consumer_segment_b[a+1u]=bitcast<vec4<u32>>(s.world_end_low);
 consumer_segment_b[a+2u]=bitcast<vec4<u32>>(s.begin_low);consumer_segment_b[a+3u]=bitcast<vec4<u32>>(s.end_low);
 consumer_segment_c[c]=bitcast<vec4<u32>>(s.range);consumer_segment_c[c+1u]=bitcast<vec4<u32>>(s.clip_w);
 consumer_segment_c[c+2u]=vec4<u32>(s.state,s.padding0,s.padding1,s.padding2);
}
fn consumer_segment_load(index:u32)->DdaProjectedSegment {
 let a=index*4u;let c=index*3u;let header_bits=consumer_segment_c[c+2u];
 return DdaProjectedSegment(bitcast<vec4<f32>>(consumer_segment_a[a]),bitcast<vec4<f32>>(consumer_segment_a[a+1u]),
 bitcast<vec4<f32>>(consumer_segment_a[a+2u]),bitcast<vec4<f32>>(consumer_segment_a[a+3u]),
 bitcast<vec4<f32>>(consumer_segment_b[a]),bitcast<vec4<f32>>(consumer_segment_b[a+1u]),
 bitcast<vec4<f32>>(consumer_segment_b[a+2u]),bitcast<vec4<f32>>(consumer_segment_b[a+3u]),
 bitcast<vec4<f32>>(consumer_segment_c[c]),bitcast<vec4<f32>>(consumer_segment_c[c+1u]),header_bits.x,header_bits.y,header_bits.z,header_bits.w);
}
// Only scheduling changes. Existing projection, paired traversal and radiance
// authorities retain every Float32 high/low word at stage boundaries.
@compute @workgroup_size(8,8)
fn cs_consumer_profile_project(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 consumer_segment_store(id.y*ray_tile.extent.x+id.x,consumer_project(archive_ray_input(ray_tile.offset+id.xy)));
}
@compute @workgroup_size(8,8)
fn cs_consumer_profile_traverse(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let index=id.y*ray_tile.extent.x+id.x;let segment=consumer_segment_load(index);var hit=precise_miss(0u);
 if segment.state==1u {hit=trace_depth_dda_paired(scene_depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}
 consumer_hits[index]=hit;
}
fn consumer_segment_same(a:DdaProjectedSegment,b:DdaProjectedSegment)->bool {
 return all(bitcast<vec4<u32>>(a.begin)==bitcast<vec4<u32>>(b.begin))&&all(bitcast<vec4<u32>>(a.end)==bitcast<vec4<u32>>(b.end))&&
 all(bitcast<vec4<u32>>(a.world_begin)==bitcast<vec4<u32>>(b.world_begin))&&all(bitcast<vec4<u32>>(a.world_end)==bitcast<vec4<u32>>(b.world_end))&&
 all(bitcast<vec4<u32>>(a.world_begin_low)==bitcast<vec4<u32>>(b.world_begin_low))&&all(bitcast<vec4<u32>>(a.world_end_low)==bitcast<vec4<u32>>(b.world_end_low))&&
 all(bitcast<vec4<u32>>(a.begin_low)==bitcast<vec4<u32>>(b.begin_low))&&all(bitcast<vec4<u32>>(a.end_low)==bitcast<vec4<u32>>(b.end_low))&&
 all(bitcast<vec4<u32>>(a.range)==bitcast<vec4<u32>>(b.range))&&all(bitcast<vec4<u32>>(a.clip_w)==bitcast<vec4<u32>>(b.clip_w))&&
 a.state==b.state&&a.padding0==b.padding0&&a.padding1==b.padding1&&a.padding2==b.padding2;
}
@compute @workgroup_size(8,8)
fn cs_consumer_profile_colour(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let index=id.y*ray_tile.extent.x+id.x;let ray=archive_ray_input(ray_tile.offset+id.xy);let segment=consumer_segment_load(index);let hit=consumer_hits[index];
 if WORLD_CONSUMER_CACHE_CHECK {
  // Recompute directly in this pass, independently of serialized projection.
  var reference=dda_projection_empty(0u);
  if ray.admitted&&!(ray.history&&camera.surface.z<0.5) {
   if ARCHIVE_PAIRED_DDA&&!ARCHIVE_RAY_FORCE_FALLBACK {reference=dda_project_segment(camera.vp,ray.origin,ray.origin+ray.direction*60.,camera.viewport.xy);}
   else {reference=dda_projection_empty(2u);}
  }
  reference.padding0=select(0u,1u,ray.history);
  var expected=precise_miss(0u);if reference.state==1u {expected=trace_depth_dda_paired(scene_depth,DdaVectorPair(reference.begin,reference.begin_low),DdaVectorPair(reference.end,reference.end_low));}
  if !consumer_segment_same(segment,reference)||any(hit.pixel!=expected.pixel)||any(bitcast<vec2<u32>>(hit.parameter)!=bitcast<vec2<u32>>(expected.parameter))||hit.visited!=expected.visited||hit.padding0!=expected.padding0||hit.padding1!=expected.padding1||hit.padding2!=expected.padding2 {atomicAdd(&world_resolve_audit[10],1u);}
 }
 var colour=vec4<f32>(0.);
 if segment.state==1u {colour=screen_reflection_paired_hit(segment,ray.history,hit);}
 else if segment.state==2u&&ray.admitted {colour=screen_reflection_original(ray.origin,ray.direction,ray.history,60.,1.,8u,ARCHIVE_RAY_BOUNDS);}
 world_consumer_store(index,ray,colour);
}
