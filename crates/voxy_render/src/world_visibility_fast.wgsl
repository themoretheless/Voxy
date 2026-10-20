// Nearest mode proves misses or a unique winner; uncertain/tied candidates
// retain exact evaluation, with a complete source scan on bounded-list overflow.
override WORLD_FAST_WORK_STATS:bool=false;
override WORLD_FAST_DEFER_NONCLEAR:bool=false;
override WORLD_FAST_STRIDE:u32=5u;
// Only a prepared all-known material table may bound nearest traversal. Unknown
// alpha must still be visited even behind a nearer known hit (legacy policy).
override WORLD_FAST_CAN_BOUND:bool=false;
override WORLD_FAST_PREVIOUS_SEED:bool=false;
override WORLD_FAST_SEPARATE_SEED:bool=false;
override WORLD_FAST_HISTORY_ANY_HIT:bool=false;
override WORLD_FAST_NATIVE_WITNESS:bool=false;
override WORLD_FAST_NODE_RECIPROCAL:bool=false;
var<private> fast_history_witness:bool;
var<private> fast_work_triangles:u32;
// This module has no arbitrary-width integer arena or predicate. Unproven
// intersections are deferred; an uncertain test never authorizes a clear ray.
var<private> visibility_pending:bool;
var<private> fast_winner_slot:u32;
var<private> fast_winner_primitive:u32;
var<private> fast_winner_t:WorldInterval;
var<private> fast_candidate_count:u32;
var<private> fast_candidate_overflow:bool;
var<private> fast_index:u32;
var<private> fast_seed_slot:u32;
var<private> fast_seed_primitive:u32;
fn fast_candidate_append(slot:u32,primitive:u32) {
 let limit=(WORLD_FAST_STRIDE*4u-8u)/2u;
 if fast_candidate_count>=limit {fast_candidate_overflow=true;return;}
 let out=fast_index*WORLD_FAST_STRIDE+2u+fast_candidate_count/2u;
 let value=exact_output[out];
 if fast_candidate_count%2u==0u {exact_output[out]=vec4<u32>(slot,primitive,value.zw);}
 else {exact_output[out]=vec4<u32>(value.xy,slot,primitive);}
 fast_candidate_count+=1u;
}
// One authority for cached and native IDs: neither historic distance nor UV
// is reused. Re-certify the current original source triangle and exact ray.
fn visibility_try_seed(slot:u32,primitive:u32,ray:u32)->bool {
 if !WORLD_FAST_DEFER_NONCLEAR||!WORLD_FAST_CAN_BOUND {return false;}
 if slot>=nearest_active[0]||nearest_active[4u+slot]==0u {return false;}
 let info=exact_layout[slot];if primitive>=nearest_counts.values[info.x*2u].x {return false;}
 let material=visibility_material(info.w);if material!=1u&&material!=2u {return false;}
 if WORLD_FAST_WORK_STATS {fast_work_triangles=min(fast_work_triangles,0xfffffffeu)+1u;}
 let proof=source_triangle_certificate(slot,info.z+primitive*3u,ray,!WORLD_FAST_HISTORY_ANY_HIT);
 if proof.intersection.state!=1u {return false;}
 if WORLD_FAST_HISTORY_ANY_HIT {fast_history_witness=true;return true;}
 if proof.t.valid==0u {return false;}
 fast_seed_slot=slot;fast_seed_primitive=primitive;
 fast_winner_slot=slot;fast_winner_primitive=primitive;fast_winner_t=proof.t;
 visibility_candidates+=1u;visibility_hits+=1u;return true;
}
fn fast_seed_tag()->u32 {return select(0u,select(0x80000000u,0x40000000u,WORLD_FAST_PREVIOUS_SEED&&!WORLD_FAST_SEPARATE_SEED),fast_seed_slot!=0xffffffffu);}
fn visibility_triangle(slot:u32,primitive:u32,ray:u32,material:u32)->bool {
 if WORLD_FAST_DEFER_NONCLEAR&&slot==fast_seed_slot&&primitive==fast_seed_primitive {return false;}
 visibility_candidates+=1u;
 if WORLD_FAST_WORK_STATS {fast_work_triangles=min(fast_work_triangles,0xfffffffeu)+1u;}
 let source=source_triangle_certificate(slot,exact_layout[slot].z+primitive*3u,ray,WORLD_FAST_DEFER_NONCLEAR&&!WORLD_FAST_HISTORY_ANY_HIT);
 let certificate=source.intersection;
 if certificate.state==0u {visibility_misses+=1u;return false;}
 if certificate.state!=1u {
  if WORLD_FAST_DEFER_NONCLEAR {fast_candidate_append(slot,primitive);return fast_candidate_overflow;}
  visibility_pending=true;return false;
 }
 visibility_hits+=1u;
 if WORLD_FAST_HISTORY_ANY_HIT {
  if material==1u||material==2u {fast_history_witness=true;return true;}
  // Unknown opacity still requires exact evaluation; no nearest distance or
  // winner ordering is used by this existence-only consumer admission.
  if material!=0u {fast_candidate_append(slot,primitive);return fast_candidate_overflow;}
  return false;
 }
 if WORLD_FAST_DEFER_NONCLEAR {
  if material==3u||source.t.valid==0u {fast_candidate_append(slot,primitive);return fast_candidate_overflow;}
  if fast_winner_slot==0xffffffffu {
   fast_winner_slot=slot;fast_winner_primitive=primitive;fast_winner_t=source.t;
  } else if source.t.hi<fast_winner_t.lo {
   fast_winner_slot=slot;fast_winner_primitive=primitive;fast_winner_t=source.t;
  } else if source.t.lo<=fast_winner_t.hi {
   if source.t.hi<fast_winner_t.hi {
    fast_candidate_append(fast_winner_slot,fast_winner_primitive);
    fast_winner_slot=slot;fast_winner_primitive=primitive;fast_winner_t=source.t;
   }else{fast_candidate_append(slot,primitive);}
  }
  // Overflow stops only the certificate walk; the complete exact path follows.
  return fast_candidate_overflow;
 }
 if material==1u {visibility_slot=slot;visibility_primitive=primitive;visibility_route=256u;return true;}
 visibility_unknown|=select(1024u,256u,material==3u);return false;
}
fn visibility_defer(index:u32) {
 visibility_early(index,WORLD_FAST_STRIDE,4u,0u);
 // One append per ray, at most count entries. On violated capacity admission,
 // expose unresolved numeric failure rather than an incomplete clear result.
 if !deferred_append(index) {visibility_early(index,WORLD_FAST_STRIDE,2u,64u);}
}
fn visibility_fast_process(index:u32) {
 let ray=index*20u;if ray+19u>=exact_ray_word_count() {return;}
 if (nearest_counts.values[4095u].w&8u)!=0u {
  let admission=exact_ray_word(ray+3u);
  if admission==0u {visibility_early(index,WORLD_FAST_STRIDE,3u,0u);return;}
  if admission!=1u {visibility_early(index,WORLD_FAST_STRIDE,2u,512u);return;}
 }
 for(var axis=0u;axis<3u;axis++) {for(var part=0u;part<4u;part++) {
  if ((exact_ray_word(ray+axis+part*4u)>>23u)&255u)==255u {
   if WORLD_FAST_DEFER_NONCLEAR {visibility_defer(index);}else{visibility_early(index,WORLD_FAST_STRIDE,2u,64u);}return;
  }
 }}
 // Only the paired source-proof pass in THIS encode may publish status 7.
 // It shares immutable source ownership and the same protected ray words.
 if WORLD_FAST_NATIVE_WITNESS&&WORLD_FAST_HISTORY_ANY_HIT {
  let head=exact_output[index*WORLD_FAST_STRIDE];let ids=exact_output[index*WORLD_FAST_STRIDE+1u];
  if head.x==7u&&head.y==8192u&&ids.x==exact_ray_word(ray+18u)&&ids.y==exact_ray_word(ray+19u) {
   visibility_early(index,WORLD_FAST_STRIDE,2u,8192u);
   exact_output[index*WORLD_FAST_STRIDE+4u].z=65536u;return;
  }
 }
 var previous=vec4<u32>(0u);var previous_valid=false;
 if WORLD_FAST_PREVIOUS_SEED {
  previous=exact_output[index*WORLD_FAST_STRIDE+1u];
  previous_valid=exact_output[index*WORLD_FAST_STRIDE].x==1u&&previous.x==exact_ray_word(ray+18u)&&previous.y==exact_ray_word(ray+19u);
 }
 visibility_candidates=0u;visibility_misses=0u;visibility_hits=0u;visibility_exact=0u;
 visibility_nodes=0u;visibility_unknown=0u;visibility_slot=0xffffffffu;
 visibility_primitive=0xffffffffu;visibility_route=0u;visibility_pending=false;
 fast_winner_slot=0xffffffffu;fast_winner_primitive=0xffffffffu;fast_winner_t=wi_bad();
 fast_seed_slot=0xffffffffu;fast_seed_primitive=0xffffffffu;
 fast_index=index;fast_candidate_count=0u;fast_candidate_overflow=false;fast_history_witness=false;
 if WORLD_FAST_DEFER_NONCLEAR {visibility_early(index,WORLD_FAST_STRIDE,4u,0u);}
 for(var axis=0u;axis<3u;axis++) {
  nearest_node_origin[axis]=wi_add(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_ray_word(ray+4u+axis)));
  nearest_node_direction[axis]=wi_add(wi_source_word(exact_ray_word(ray+8u+axis)),wi_source_word(exact_ray_word(ray+12u+axis)));
 }
 if WORLD_FAST_NODE_RECIPROCAL {nearest_prepare_reciprocal();}
 visibility_box_valid=visibility_make_float_box();
 if !visibility_box_valid {visibility_defer(index);return;}
 visibility_seed(ray);
 if WORLD_FAST_PREVIOUS_SEED&&previous_valid {let accepted=visibility_try_seed(previous.z,previous.w,ray);}
 var blocked=false;
 if !fast_history_witness {blocked=visibility_walk(ray);}
 // A source-certified admitted surface exists. All possible winners use the
 // same complete consumer fallback; no nearest identity or distance is needed.
 // Absence of a witness still takes the original complete miss/exact path.
 if fast_history_witness {visibility_early(index,WORLD_FAST_STRIDE,2u,8192u);return;}
 if WORLD_FAST_DEFER_NONCLEAR {
  if blocked||fast_candidate_overflow||visibility_unknown!=0u {visibility_defer(index);return;}
  if fast_candidate_count!=0u {
   if fast_winner_slot!=0xffffffffu {fast_candidate_append(fast_winner_slot,fast_winner_primitive);}
   if fast_candidate_overflow {visibility_defer(index);return;}
   exact_output[index*WORLD_FAST_STRIDE]=vec4<u32>(6u,fast_candidate_count,2048u,1u|fast_seed_tag());
   if !deferred_append(index) {visibility_early(index,WORLD_FAST_STRIDE,2u,64u);}
   return;
  }
  if fast_winner_slot!=0xffffffffu {
   visibility_early(index,WORLD_FAST_STRIDE,5u,0u);
   exact_output[index*WORLD_FAST_STRIDE].w|=fast_seed_tag();
   exact_output[index*WORLD_FAST_STRIDE+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),fast_winner_slot,fast_winner_primitive);
   if !deferred_append(index) {visibility_early(index,WORLD_FAST_STRIDE,2u,64u);}
   return;
  }
  visibility_early(index,WORLD_FAST_STRIDE,0u,0u);
  exact_output[index*WORLD_FAST_STRIDE+4u]=vec4<u32>(visibility_candidates,0xffffffffu,2049u|(select(0u,1u,nearest_material_enabled())<<1u),0u);
  return;
 }
 if !blocked&&(visibility_pending||(visibility_unknown&64u)!=0u) {visibility_defer(index);return;}
 var state=select(0u,1u,blocked);var reason=0u;
 if !blocked&&visibility_unknown!=0u {state=2u;reason=visibility_unknown;}
 let out=index*5u;exact_output[out]=vec4<u32>(state,reason,2048u,1u);
 exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),visibility_slot,visibility_primitive);
 exact_output[out+2u]=vec4<u32>(0u);exact_output[out+3u]=vec4<u32>(0u);
 var material=0xffffffffu;if blocked {material=exact_layout[visibility_slot].w;}
 exact_output[out+4u]=vec4<u32>(visibility_candidates,material,visibility_route|(select(0u,1u,nearest_material_enabled())<<1u),0u);
}

fn visibility_node_miss(base:u32,ray:u32)->bool {
 if WORLD_FAST_DEFER_NONCLEAR&&WORLD_FAST_CAN_BOUND&&fast_winner_t.valid!=0u {
  if WORLD_FAST_NODE_RECIPROCAL {return nearest_node_miss_cached(base,ray,min(1.,fast_winner_t.hi));}
  return nearest_node_miss_before(base,ray,min(1.,fast_winner_t.hi));
 }
 if WORLD_FAST_NODE_RECIPROCAL {return nearest_node_miss_cached(base,ray,1.);}
 return nearest_node_miss(base,ray);
}
// This is a scheduling hint only. It never certifies a hit or rejects a node.
fn visibility_node_key(node:u32)->f32 {
 let base=nearest_active[2]+node*8u;var rank=0.;
 for(var axis=0u;axis<3u;axis++) {
  let center=bitcast<f32>(nearest_active[base+axis])*.5+bitcast<f32>(nearest_active[base+3u+axis])*.5;
  let origin=nearest_node_origin[axis].lo*.5+nearest_node_origin[axis].hi*.5;
  let direction=nearest_node_direction[axis].lo*.5+nearest_node_direction[axis].hi*.5;
  rank+=(center-origin)*direction;
 }
 if !wi_finite(rank) {return 0.;}return rank;
}
fn visibility_left_first(left:u32,right:u32,ray:u32)->bool {
 if !WORLD_FAST_DEFER_NONCLEAR||!WORLD_FAST_CAN_BOUND {return false;}
 if left>=nearest_active[1]||right>=nearest_active[1] {return false;}
 return visibility_node_key(left)<visibility_node_key(right);
}

// Diagnostic fields use only non-header padding. Exact queue owns ray 0 count/
// cursor and every word-15 link; never write those here. No output is changed.
@compute @workgroup_size(64)
fn main_visibility_fast(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;if index>=arrayLength(&deferred_rays) {return;}
 visibility_nodes=0u;visibility_candidates=0u;fast_work_triangles=0u;
 visibility_fast_process(index);
 if WORLD_FAST_WORK_STATS&&index>0u {
  atomicStore(&deferred_rays[index].count,visibility_nodes);
  atomicStore(&deferred_rays[index].cursor,fast_work_triangles);
 }
}
