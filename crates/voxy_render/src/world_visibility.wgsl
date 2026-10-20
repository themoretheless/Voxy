// Closed-segment opaque visibility. Float certificates prove hit/miss; every
// ambiguous triangle retains the existing integer predicate. No nearest t or
// rounded parameter is needed for a proven opaque blocker.
// Source parameters are single f32 words. Integer magnitude/sign ordering is
// exact, including signed zero and subnormals; no FP material epsilon or FTZ.


fn visibility_triangle(slot:u32,primitive:u32,ray:u32,material:u32)->bool {
 visibility_candidates+=1u;let info=exact_layout[slot];let first=info.z+primitive*3u;
 var certificate=WorldTriangleCertificate(2u,0u,0u);
 if (nearest_counts.values[4095u].w&16u)!=0u {certificate=visibility_certificate(slot,first,ray);}
 if certificate.state==0u {visibility_misses+=1u;return false;}
 var hit=certificate.state==1u;var route=256u;
 if hit {visibility_hits+=1u;}else{
  visibility_exact+=1u;ex_cursor=0u;ex_parameters.valid=0u;
  let a=exact_vertex(slot,info.y+exact_indices[first],ray);let b=exact_vertex(slot,info.y+exact_indices[first+1u],ray);let c=exact_vertex(slot,info.y+exact_indices[first+2u],ray);
  let d=ExVec(ex_add(ex_float(exact_ray_word(ray+8u),-149),ex_float(exact_ray_word(ray+12u),-149)),ex_add(ex_float(exact_ray_word(ray+9u),-149),ex_float(exact_ray_word(ray+13u),-149)),ex_add(ex_float(exact_ray_word(ray+10u),-149),ex_float(exact_ray_word(ray+14u),-149)));
  let decision=ex_triangle(a,b,c,d);
  if decision.state==2u {visibility_unknown|=64u;return false;}
  hit=decision.state==1u;route=512u;
 }
 if !hit {return false;}
 if material==1u {visibility_slot=slot;visibility_primitive=primitive;visibility_route=route;return true;}
 visibility_unknown|=select(1024u,256u,material==3u);return false;
}
fn visibility_make_box(ray:u32)->bool {
 if visibility_make_float_box() {return true;}
 // Preserve unsupported interval-domain words (including subnormals). Exact
 // endpoint rounding supplies only an outward broad-phase envelope.
 visibility_integer_box=true;ex_cursor=7u;
 let d=ExVec(ex_add(ex_float(exact_ray_word(ray+8u),-149),ex_float(exact_ray_word(ray+12u),-149)),ex_add(ex_float(exact_ray_word(ray+9u),-149),ex_float(exact_ray_word(ray+13u),-149)),ex_add(ex_float(exact_ray_word(ray+10u),-149),ex_float(exact_ray_word(ray+14u),-149)));
 let dx=exact_copy_parameter(d.x,4u);let dy=exact_copy_parameter(d.y,5u);let dz=exact_copy_parameter(d.z,6u);
 return nearest_make_box(ray,ExVec(dx,dy,dz));
}


fn visibility_process_worker(index:u32,stride:u32,worker:u32) {
 let ray=index*20u;if ray+19u>=exact_ray_word_count() {return;}
 if (nearest_counts.values[4095u].w&8u)!=0u {let admission=exact_ray_word(ray+3u);if admission==0u {visibility_early(index,stride,3u,0u);return;}if admission!=1u {visibility_early(index,stride,2u,512u);return;}}
 for(var axis=0u;axis<3u;axis++) {for(var part=0u;part<4u;part++) {
  if ((exact_ray_word(ray+axis+part*4u)>>23u)&255u)==255u {visibility_early(index,stride,2u,64u);return;}
 }}
 ex_begin=worker*EX_ARENA_WORDS;ex_cursor=0u;ex_peak=0u;
 visibility_candidates=0u;visibility_misses=0u;visibility_hits=0u;visibility_exact=0u;visibility_nodes=0u;visibility_unknown=0u;visibility_slot=0xffffffffu;visibility_primitive=0xffffffffu;visibility_route=0u;
 for(var axis=0u;axis<3u;axis++) {
  nearest_node_origin[axis]=wi_add(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_ray_word(ray+4u+axis)));
  nearest_node_direction[axis]=wi_add(wi_source_word(exact_ray_word(ray+8u+axis)),wi_source_word(exact_ray_word(ray+12u+axis)));
 }
 visibility_integer_box=false;visibility_box_valid=visibility_make_box(ray);
 let blocked=visibility_walk(ray);var state=select(0u,1u,blocked);var reason=0u;
 if !blocked&&visibility_unknown!=0u {state=2u;reason=visibility_unknown;}
 let out=index*stride;exact_output[out]=vec4<u32>(state,reason,2048u,max(1u,ex_peak));
 exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),visibility_slot,visibility_primitive);
 exact_output[out+2u]=vec4<u32>(0u);exact_output[out+3u]=vec4<u32>(0u);
 var material=0xffffffffu;if blocked {material=exact_layout[visibility_slot].w;}
 exact_output[out+4u]=vec4<u32>(visibility_candidates,material,visibility_route|(select(0u,1u,nearest_material_enabled())<<1u)|(u32(visibility_integer_box)<<10u),0u);
 if stride==6u {exact_output[out+5u]=vec4<u32>(visibility_misses,visibility_hits,visibility_exact,visibility_nodes);}
}
@compute @workgroup_size(1) fn main_visibility(@builtin(global_invocation_id) id:vec3<u32>) {visibility_process_worker(id.x,5u,id.x);}
@compute @workgroup_size(1) fn main_visibility_stats(@builtin(global_invocation_id) id:vec3<u32>) {visibility_process_worker(id.x,6u,id.x);}

// Fixed workspace per worker, reused only after the previous ray has completed.
// The input binding is restricted to the requested count by the host. Every ray
// in that view is visited exactly once, even when the batch exceeds worker count.
@compute @workgroup_size(1)
fn main_visibility_bounded(@builtin(global_invocation_id) id:vec3<u32>,
 @builtin(num_workgroups) groups:vec3<u32>) {
 let count=exact_ray_word_count()/20u;
 for(var index=id.x;index<count;index+=groups.x) {
  visibility_process_worker(index,5u,id.x);
 }
}

@compute @workgroup_size(1)
fn main_visibility_bounded_stats(@builtin(global_invocation_id) id:vec3<u32>,
 @builtin(num_workgroups) groups:vec3<u32>) {
 let count=exact_ray_word_count()/20u;
 for(var index=id.x;index<count;index+=groups.x) {
  visibility_process_worker(index,6u,id.x);
 }
}

// Monolithic visibility keeps its existing complete walk and candidate order.
fn visibility_node_miss(base:u32,ray:u32)->bool {return nearest_node_miss(base,ray);}
fn visibility_left_first(left:u32,right:u32,ray:u32)->bool {return false;}
