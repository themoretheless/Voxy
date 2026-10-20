// Work classes affect scheduling only. Geometry still uses the original paired
// traversal. Colour reprojects the same immutable ray/guide inputs, retaining
// every field without transporting unused projection fields through storage.
struct SortedConsumerSegment {begin:vec4<f32>,end:vec4<f32>,begin_low:vec4<f32>,end_low:vec4<f32>}
@group(1) @binding(23) var<storage,read_write> sorted_segments:array<SortedConsumerSegment>;
@group(1) @binding(26) var<storage,read_write> sorted_hits:array<DepthDdaPairHit>;
// Header: histogram[32], offsets[32], cursors[32], total, fault, padding.
// Followed by capacity keys and capacity original indices. All words are exact.
@group(1) @binding(27) var<storage,read_write> sorted_work:array<atomic<u32>>;
var<workgroup> sorted_histogram:array<atomic<u32>,32>;
fn sorted_capacity()->u32{return (arrayLength(&sorted_work)-128u)/2u;}
fn sorted_key_at(index:u32)->u32{return 128u+index;}
fn sorted_queue_at(index:u32)->u32{return 128u+sorted_capacity()+index;}
fn sorted_fault(){atomicStore(&sorted_work[97u],1u);atomicAdd(&world_resolve_audit[10u],1u);}
fn sorted_class(s:DdaProjectedSegment)->u32 {
 if !dda_finite4(s.begin)||!dda_finite4(s.end){return 31u;}
 // Clamp before subtraction so even extreme finite coordinates cannot
 // overflow the scheduling estimate. This estimate never rejects a ray.
 let a=clamp(s.begin.xy,vec2<f32>(0.),vec2<f32>(65535.));
 let b=clamp(s.end.xy,vec2<f32>(0.),vec2<f32>(65535.));
 let span=abs(b.x-a.x)+abs(b.y-a.y);
 return min(31u,32u-countLeadingZeros(u32(max(1.,ceil(span)))));
}
@compute @workgroup_size(8,8)
fn cs_consumer_sorted_project(@builtin(global_invocation_id) id:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
 if lane<32u {atomicStore(&sorted_histogram[lane],0u);}
 workgroupBarrier();
 if all(id.xy<ray_tile.extent) {
  let index=id.y*ray_tile.extent.x+id.x;let s=consumer_project(archive_ray_input(ray_tile.offset+id.xy));
  sorted_hits[index]=precise_miss(0u);
  var key=0xffffffffu;
  if s.state==1u {
   key=sorted_class(s);sorted_segments[index]=SortedConsumerSegment(s.begin,s.end,s.begin_low,s.end_low);
   atomicAdd(&sorted_histogram[key],1u);
  }
  atomicStore(&sorted_work[sorted_key_at(index)],key);
 }
 workgroupBarrier();
 if lane<32u {let n=atomicLoad(&sorted_histogram[lane]);if n>0u {atomicAdd(&sorted_work[lane],n);}}
}
@compute @workgroup_size(1)
fn cs_consumer_sorted_prefix() {
 var total=0u;
 for(var bucket=0u;bucket<32u;bucket++) {
  atomicStore(&sorted_work[32u+bucket],total);atomicStore(&sorted_work[64u+bucket],0u);
  total+=atomicLoad(&sorted_work[bucket]);
 }
 atomicStore(&sorted_work[96u],total);
 if total>sorted_capacity(){sorted_fault();}
}
@compute @workgroup_size(8,8)
fn cs_consumer_sorted_scatter(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent)||atomicLoad(&sorted_work[97u])!=0u {return;}
 let index=id.y*ray_tile.extent.x+id.x;let key=atomicLoad(&sorted_work[sorted_key_at(index)]);
 if key==0xffffffffu {return;}
 if key>=32u {sorted_fault();return;}
 let slot=atomicLoad(&sorted_work[32u+key])+atomicAdd(&sorted_work[64u+key],1u);
 if slot>=atomicLoad(&sorted_work[96u])||slot>=sorted_capacity(){sorted_fault();return;}
 atomicStore(&sorted_work[sorted_queue_at(slot)],index);
}
@compute @workgroup_size(64)
fn cs_consumer_sorted_traverse(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=atomicLoad(&sorted_work[96u])||atomicLoad(&sorted_work[97u])!=0u{return;}
 let index=atomicLoad(&sorted_work[sorted_queue_at(id.x)]);
 if index>=ray_tile.extent.x*ray_tile.extent.y {sorted_fault();return;}
 if WORLD_CONSUMER_CACHE_CHECK {
  let key=atomicExchange(&sorted_work[sorted_key_at(index)],0xfffffffeu);
  if key>=32u {sorted_fault();return;}
 }
 let s=sorted_segments[index];
 sorted_hits[index]=trace_depth_dda_paired(scene_depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));
}
fn sorted_segment_same(a:SortedConsumerSegment,b:DdaProjectedSegment)->bool {
 return all(bitcast<vec4<u32>>(a.begin)==bitcast<vec4<u32>>(b.begin))&&all(bitcast<vec4<u32>>(a.end)==bitcast<vec4<u32>>(b.end))&&
 all(bitcast<vec4<u32>>(a.begin_low)==bitcast<vec4<u32>>(b.begin_low))&&all(bitcast<vec4<u32>>(a.end_low)==bitcast<vec4<u32>>(b.end_low));
}
@compute @workgroup_size(8,8)
fn cs_consumer_sorted_colour(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent){return;}
 let index=id.y*ray_tile.extent.x+id.x;let ray=archive_ray_input(ray_tile.offset+id.xy);let s=consumer_project(ray);let hit=sorted_hits[index];
 var failed=atomicLoad(&sorted_work[97u])!=0u;
 if WORLD_CONSUMER_CACHE_CHECK&&s.state==1u {
  if atomicLoad(&sorted_work[sorted_key_at(index)])!=0xfffffffeu||!sorted_segment_same(sorted_segments[index],s) {sorted_fault();failed=true;}
  let expected=trace_depth_dda_paired(scene_depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));
  if any(hit.pixel!=expected.pixel)||any(bitcast<vec2<u32>>(hit.parameter)!=bitcast<vec2<u32>>(expected.parameter))||hit.visited!=expected.visited||hit.padding0!=expected.padding0||hit.padding1!=expected.padding1||hit.padding2!=expected.padding2 {sorted_fault();failed=true;}
 }
 var colour=vec4<f32>(0.);
 // A malformed work queue retains the complete original query, never zeroes
 // or drops a contribution. The diagnostic error counter still rejects it.
 if failed {if ray.admitted {colour=archive_world_query_fallback(ray);}}
 else if s.state==1u {colour=screen_reflection_paired_hit(s,ray.history,hit);}
 else if s.state==2u&&ray.admitted {colour=screen_reflection_original(ray.origin,ray.direction,ray.history,60.,1.,8u,ARCHIVE_RAY_BOUNDS);}
 world_consumer_store(index,ray,colour);
}
