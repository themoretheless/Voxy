// Exact cursor arithmetic is generated from the original DDA owner.
// Only work assignment changes. Every original ID, inactive ray and fallback
// remains represented; final radiance resolves in original pixel order.
@group(1) @binding(17) var<storage,read_write> persistent_next:atomic<u32>;
@compute @workgroup_size(64) fn cs_ray_traverse_persistent() {
 let count=atomicLoad(&staged_bins.header[0]);
 var cursor:DdaCursor;var index=0u;var ray_pending=false;
 loop {
  if !ray_pending {
   let position=atomicAdd(&persistent_next,1u);
   if position<count {
    index=staged_order[position];
    if index<count {
     if ARCHIVE_RAY_QUEUE_PROOF {
      if atomicAdd(&staged_seen[index],1u)!=0u {atomicAdd(&staged_errors[1],1u);}
      atomicAdd(&staged_errors[4],1u);
     }
     let segment=staged_segments[index];
     if segment.state==1u {
      if ARCHIVE_RAY_QUEUE_PROOF {atomicAdd(&staged_errors[6],1u);}
      cursor=dda_cursor_init(scene_depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));ray_pending=!cursor.done;
      if !ray_pending {staged_hits[index]=cursor.hit;}
     }else {staged_hits[index]=precise_miss(0u);}
    }else {if ARCHIVE_RAY_QUEUE_PROOF {atomicAdd(&staged_errors[0],1u);}}
   }
  }
  if ray_pending {
   for(var step=0u;step<16u;step++) {if cursor.done {break;}dda_cursor_step(scene_depth,&cursor);}
   if cursor.done {staged_hits[index]=cursor.hit;ray_pending=false;}
  }
  if !subgroupAny(ray_pending||atomicLoad(&persistent_next)<count) {break;}
 }
}
