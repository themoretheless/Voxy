override WORLD_EXACT_GROUP:u32=1u;
// Two entry-point families isolate strict primitive work from the complete
// hierarchy/tie/coplanar predicate. Both reuse the same bounded integer arenas.
fn nearest_winner_process(index:u32,stride:u32,worker:u32) {
 let out=index*stride;let source=exact_output[out+1u];
 let hints=exact_output[out].w;let hint_flags=select(0u,16384u,(hints&0x80000000u)!=0u)|select(0u,32768u,(hints&0x40000000u)!=0u);
 if source.z<nearest_active[0]&&nearest_active[4u+source.z]!=0u {
  let info=exact_layout[source.z];
  if source.w<nearest_counts.values[info.x*2u].x {
   let direction=nearest_start(index,stride,worker);
   nearest_candidates=1u;nearest_exact_calls=1u;ex_cursor=nearest_temporary_begin();ex_parameters.valid=0u;
   let triangle=nearest_source_vertices(source.z,source.w,index*20u);
   let decision=ex_triangle_certified(triangle.a,triangle.b,triangle.c,direction);
   if nearest_admit_decision(info.w,decision) {
    nearest_save_parameters(source.z,source.w);nearest_finish(index,stride,false,direction);
    exact_output[out+4u].z|=4096u|hint_flags;return;
   }
  }
 }
 // A violated hint is never a fabricated miss. The following full pass retries.
 nearest_admission_result(index,stride,4u,0u);
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_winner(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*5u].x==5u {nearest_winner_process(index,5u,id.x);}
 }
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_uv_winner(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*7u].x==5u {nearest_winner_process(index,7u,id.x);}
 }
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_deferred(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*5u].x==4u {nearest_process_worker(index,5u,id.x);}
 }
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_uv_deferred(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*7u].x==4u {nearest_process_worker(index,7u,id.x);}
 }
}

// The fast pass stores at most 6 (20-word) or 10 (28-word) candidates directly
// in the not-yet-final output payload. Copy before initialization overwrites it.
fn nearest_candidates_process(index:u32,stride:u32,worker:u32) {
 let out=index*stride;let hints=exact_output[out].w;let hint_flags=select(0u,16384u,(hints&0x80000000u)!=0u)|select(0u,32768u,(hints&0x40000000u)!=0u);let count=exact_output[out].y;let limit=(stride*4u-8u)/2u;
 if count==0u||count>limit {nearest_admission_result(index,stride,4u,0u);return;}
 var candidates:array<vec2<u32>,10>;
 for(var i=0u;i<count;i++) {
  let value=exact_output[out+2u+i/2u];candidates[i]=select(value.xy,value.zw,i%2u!=0u);
 }
 let direction=nearest_start(index,stride,worker);
 for(var i=0u;i<count;i++) {
  let source=candidates[i];
  if source.x>=nearest_active[0] {nearest_admission_result(index,stride,4u,0u);return;}
  let info=exact_layout[source.x];
  if source.y>=nearest_counts.values[info.x*2u].x {nearest_admission_result(index,stride,4u,0u);return;}
  if nearest_active[4u+source.x]==0u {continue;}
  nearest_candidates+=1u;ex_cursor=nearest_temporary_begin();ex_parameters.valid=0u;
  nearest_evaluate_and_update(source.x,source.y,index*20u,direction);
 }
 nearest_finish(index,stride,false,direction);exact_output[out+4u].z|=8192u|hint_flags;
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_candidates(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*5u].x==6u {nearest_candidates_process(index,5u,id.x);}
 }
}
@compute @workgroup_size(WORLD_EXACT_GROUP)
fn main_nearest_uv_candidates(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}
 loop {let index=deferred_take();if index==0xffffffffu {return;}
  if exact_output[index*7u].x==6u {nearest_candidates_process(index,7u,id.x);}
 }
}
