@group(0) @binding(8) var nearest_scene:acceleration_structure;



var<private> nearest_best:ExParameters;
var<private> nearest_slot:u32;
var<private> nearest_primitive:u32;
var<private> nearest_candidates:u32;
var<private> nearest_ties:u32;
var<private> nearest_unknown:u32;
// Exact affine interval of the original local source box, relative to the ray.
// No f32 arithmetic, inverse transform, padding assumption or rounded t is used.
fn nearest_instance_bound(slot:u32,axis:u32,upper:bool,ray:u32)->ExInt {
 let mesh=exact_layout[slot].x*2u;
 var value=ex_float(exact_instances[slot*28u+12u+axis],-298);
 for(var k=0u;k<3u;k+=1u) {
  let matrix_bits=exact_instances[slot*28u+k*4u+axis];
  let low=nearest_counts.values[mesh][k+1u];let high=nearest_counts.values[mesh+1u][k];
  let point=select(low,high,upper!=(matrix_bits>>31u!=0u));
  value=ex_add(value,ex_mul(ex_float(matrix_bits,-149),ex_float(point,-149)));
 }
 value=ex_sub(value,ex_float(exact_ray_word(ray+axis),-298));
 return ex_sub(value,ex_float(exact_ray_word(ray+4u+axis),-298));
}
fn nearest_instance_possible(slot:u32,ray:u32,d:ExVec)->bool {
 // Intersect the three closed slab intervals using unrounded rational bounds.
 // Slots 7..10 retain the current [begin,end]; candidate temporaries start at 11.
 ex_cursor=11u;let zero=ex_zero();let one=ex_one();
 var begin_n=exact_copy_parameter(zero,7u);var begin_d=exact_copy_parameter(one,8u);
 var finish_n=exact_copy_parameter(one,9u);var finish_d=exact_copy_parameter(one,10u);
 for(var axis=0u;axis<3u;axis+=1u) {
  ex_cursor=11u;
  var lo=nearest_instance_bound(slot,axis,false,ray);var hi=nearest_instance_bound(slot,axis,true,ray);
  var den=ex_shl(ex_component(d,axis),149u);
  if (lo.valid&hi.valid&den.valid)==0u {return true;}
  if den.length==0u {
   if (lo.negative==0u&&lo.length!=0u)||hi.negative!=0u {return false;}continue;
  }
  if den.negative!=0u {den=ex_neg(den);let old_lo=lo;lo=ex_neg(hi);hi=ex_neg(old_lo);}
  if hi.negative!=0u||ex_cmp(lo,den)>0 {return false;}
  if lo.negative!=0u {lo=ex_zero();}if ex_cmp(hi,den)>0 {hi=den;}
  let lower=ex_cmp_products(lo,begin_d,begin_n,den);
  let upper=ex_cmp_products(hi,finish_d,finish_n,den);
  if (lower.valid&upper.valid)==0u {return true;}
  if lower.order>0 {begin_n=exact_copy_parameter(lo,7u);begin_d=exact_copy_parameter(den,8u);}
  if upper.order<0 {finish_n=exact_copy_parameter(hi,9u);finish_d=exact_copy_parameter(den,10u);}
  let order=ex_cmp_products(begin_n,finish_d,finish_n,begin_d);
  if order.valid==0u {return true;}if order.order>0 {return false;}
 }
 return true;
}

var<private> nearest_seen:array<vec2<u32>,64>;
var<private> nearest_seen_count:u32;

// Directed binary32 rounding of an integer multiple of 2^-149. No float
// arithmetic or division; all discarded integer bits participate in rounding.
fn nearest_bound_bits(n:ExInt,upper:bool)->ExRounded {
 if n.valid==0u {return ExRounded(0u,0u);}let size=ex_bits(n);let sign=n.negative<<31u;
 if size==0u {return ExRounded(0u,1u);}
 if size<=24u {return ExRounded(sign|ex_word(n,0u)|(ex_word(n,1u)<<16u),1u);}
 var exponent=i32(size)-150;let ceil=upper!=(n.negative!=0u);
 if exponent>127 {return ExRounded(sign|select(2139095039u,2139095040u,ceil),1u);}
 let shift=size-24u;let whole=shift/16u;let part=shift%16u;
 var mantissa=(ex_word(n,whole)>>part)|(ex_word(n,whole+1u)<<(16u-part));
 if part>8u {mantissa|=ex_word(n,whole+2u)<<(32u-part);}mantissa&=16777215u;
 var tail=(ex_word(n,whole)&((1u<<part)-1u))!=0u;
 for(var k=0u;k<whole;k+=1u) {tail=tail||ex_word(n,k)!=0u;}
 if tail&&ceil {mantissa+=1u;}
 if mantissa==16777216u {mantissa>>=1u;exponent+=1;}
 if exponent>127 {return ExRounded(sign|2139095040u,1u);}
 return ExRounded(sign|(u32(exponent+127)<<23u)|(mantissa&8388607u),1u);
}
fn nearest_make_box(ray:u32,d:ExVec)->bool {
 for(var axis=0u;axis<3u;axis+=1u) {
  if (nearest_counts.values[4095u].w&2u)!=0u {
   nearest_node_origin[axis]=wi_add(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_ray_word(ray+4u+axis)));
   nearest_node_direction[axis]=wi_add(wi_source_word(exact_ray_word(ray+8u+axis)),wi_source_word(exact_ray_word(ray+12u+axis)));
  }
  ex_cursor=7u;
  let origin=ex_add(ex_float(exact_ray_word(ray+axis),-149),ex_float(exact_ray_word(ray+4u+axis),-149));
  let end=ex_add(origin,ex_component(d,axis));
  if (origin.valid&end.valid)==0u {return false;}
  var lo=origin;var hi=end;if ex_cmp(origin,end)>0 {lo=end;hi=origin;}
  let lower=nearest_bound_bits(lo,false);let upper=nearest_bound_bits(hi,true);
  if (lower.valid&upper.valid)==0u {return false;}
  nearest_box[axis]=vec2<u32>(nearest_ordered(lower.bits),nearest_ordered(upper.bits));
 }
 return true;
}
var<private> nearest_node_rejected:u32;


// A quotient bracket is usable only after outward multiplication proves both
// endpoints. No division-accuracy assumption or epsilon authorizes rejection.


// Source boxes may contain subnormal outward padding around zero. Enlarge that
// padding to a normal-or-zero endpoint; never move a bound inward or edit geometry.


fn nearest_source_guard(ray:u32,d:ExVec) {
 var stack:array<u32,64>;var size=0u;var visited=0u;
 if nearest_active[3]==4294967295u {return;}
 stack[0]=nearest_active[3];size=1u;
 while size>0u {
  size-=1u;let node=stack[size];visited+=1u;
  if node>=nearest_active[1]||visited>nearest_active[1] {nearest_unknown=1u;return;}
  let base=nearest_active[2]+node*8u;var overlaps=true;
  for(var axis=0u;axis<3u;axis+=1u) {
   overlaps=overlaps&&nearest_ordered(nearest_active[base+axis])<=nearest_box[axis].y&&nearest_ordered(nearest_active[base+3u+axis])>=nearest_box[axis].x;
  }
  if !overlaps {continue;}
  if (nearest_counts.values[4095u].w&2u)!=0u&&nearest_node_miss(base,ray) {nearest_node_rejected+=1u;continue;}
  let left=nearest_active[base+6u];let right=nearest_active[base+7u];
  if right<4294967294u {
   if size+2u>64u {nearest_unknown=1u;return;}
   stack[size]=left;stack[size+1u]=right;size+=2u;continue;
  }
  let leaf=source_leaf_range(left,right);if leaf.valid==0u {nearest_unknown=1u;return;}
  let slot=leaf.slot;if nearest_active[4u+slot]==0u {continue;}
  if !nearest_instance_possible(slot,ray,d) {continue;}
  let end=leaf.first+leaf.count;
  for(var primitive=leaf.first;primitive<end;primitive+=1u) {
   var seen=false;for(var k=0u;k<nearest_seen_count;k+=1u) {seen=seen||all(nearest_seen[k]==vec2<u32>(slot,primitive));}
   if !seen {nearest_consider(slot,primitive,ray,d);}
  }
 }
}
var<private> nearest_kind:u32;
var<private> nearest_material_unknown:u32;


// x: reject/accept/unresolved; y: opaque/masked/blend-or-transmission event.
// Mapped alpha is explicitly unresolved until source sampling is connected.
fn nearest_material_decision(material:u32)->vec2<u32> {
 let header=nearest_material_base();if nearest_active[header+1u]==0u {return vec2<u32>(1u,0u);}
 if material>=nearest_active[header] {return vec2<u32>(2u,0u);}
 let base=header+4u+material*24u;
 let alpha=ex_float(nearest_active[base+3u],-149);let cutoff=ex_float(nearest_active[base+15u],-149);let transmission=ex_float(nearest_active[base+14u],-149);
 let one=ex_float(1065353216u,-149);
 if (alpha.valid&cutoff.valid&transmission.valid&one.valid)==0u||alpha.negative!=0u||ex_cmp(alpha,one)>0||transmission.negative!=0u {return vec2<u32>(2u,0u);}
 let image=nearest_active[base+20u]!=4294967295u;var kind=0u;
 if cutoff.negative==0u {
  if image {return vec2<u32>(2u,0u);}
  if ex_cmp(alpha,cutoff)<0 {return vec2<u32>(0u,0u);}kind=1u;
 }
 let blend=nearest_active[base+15u]==3221225472u;
 if blend {
  if image {
   // Packed albedo A is AO, not opacity; the raster material uses diffuse.a.
   let workflow=ex_float(nearest_active[base+17u],-149);let limit=ex_float(1069547520u,-149);
   if (workflow.valid&limit.valid)==0u||ex_cmp(workflow,limit)<=0 {return vec2<u32>(2u,0u);}
  }
  if alpha.length==0u&&transmission.length==0u {return vec2<u32>(0u,0u);}kind=2u;
 }
 if transmission.length!=0u {kind=2u;}
 return vec2<u32>(1u,kind);
}
var<private> nearest_interval_rejected:u32;
var<private> nearest_exact_calls:u32;
var<private> nearest_candidate_t:WorldInterval;
var<private> nearest_best_t:WorldInterval;
fn nearest_interval_vertex(slot:u32,vertex:u32)->WorldInterval3 {
 var bounds:array<WorldInterval,3>;
 for(var axis=0u;axis<3u;axis+=1u) {
  var value=wi_source_word(exact_instances[slot*28u+12u+axis]);
  for(var k=0u;k<3u;k+=1u) {
   value=wi_add(value,wi_mul(wi_source_word(exact_instances[slot*28u+k*4u+axis]),wi_source_word(exact_vertices[vertex*12u+k])));
  }
  bounds[axis]=value;
 }
 return WorldInterval3(bounds[0],bounds[1],bounds[2]);
}
fn nearest_interval_miss(slot:u32,first:u32,ray:u32,order:bool)->bool {
 nearest_candidate_t=wi_bad();
 let info=exact_layout[slot];
 let a=nearest_interval_vertex(slot,info.y+exact_indices[first]);let b=nearest_interval_vertex(slot,info.y+exact_indices[first+1u]);let c=nearest_interval_vertex(slot,info.y+exact_indices[first+2u]);
 let origin=WorldInterval3(wi_add(wi_source_word(exact_ray_word(ray)),wi_source_word(exact_ray_word(ray+4u))),wi_add(wi_source_word(exact_ray_word(ray+1u)),wi_source_word(exact_ray_word(ray+5u))),wi_add(wi_source_word(exact_ray_word(ray+2u)),wi_source_word(exact_ray_word(ray+6u))));
 let direction=WorldInterval3(wi_add(wi_source_word(exact_ray_word(ray+8u)),wi_source_word(exact_ray_word(ray+12u))),wi_add(wi_source_word(exact_ray_word(ray+9u)),wi_source_word(exact_ray_word(ray+13u))),wi_add(wi_source_word(exact_ray_word(ray+10u)),wi_source_word(exact_ray_word(ray+14u))));
 let e1=wi3_sub(b,a);let e2=wi3_sub(c,a);let delta=wi3_sub(origin,a);
 let normal=wi3_cross(e1,e2);let start=wi3_dot(normal,delta);let finish=wi_add(start,wi3_dot(normal,direction));
 // Closed segment remains strictly on one side of the original triangle plane.
 // This also proves parallel misses, while zero-area planes remain inconclusive.
 if (wi_positive(start)&&wi_positive(finish))||(wi_negative(start)&&wi_negative(finish)) {return true;}
 let result=world_triangle_certificate(direction,e1,e2,delta,wi_scalar(0.),wi_scalar(1.));
 if result.state!=0u&&order {
  var determinant=wi3_dot(e1,wi3_cross(direction,e2));
  var numerator=wi3_dot(e2,wi3_cross(delta,e1));
  if wi_negative(determinant) {determinant=wi_neg(determinant);numerator=wi_neg(numerator);}
  // Every quotient endpoint is certified by outward back-multiplication.
  // No epsilon or native rounded t authorizes rejection.
  if wi_positive(determinant) {nearest_candidate_t=nearest_div_interval(numerator,determinant);}
 }
 return result.state==0u;
}
struct NearestSourceVertices {a:ExVec,b:ExVec,c:ExVec}
var<private> nearest_admitted_kind:u32;
fn nearest_source_vertices(slot:u32,primitive:u32,ray:u32)->NearestSourceVertices {
 let info=exact_layout[slot];let first=info.z+primitive*3u;
 let a=exact_vertex(slot,info.y+exact_indices[first],ray);let b=exact_vertex(slot,info.y+exact_indices[first+1u],ray);let c=exact_vertex(slot,info.y+exact_indices[first+2u],ray);
 return NearestSourceVertices(a,b,c);
}
fn nearest_admit_decision(material:u32,decision:ExDecision)->bool {
 if decision.state==2u {nearest_unknown=1u;return false;}if decision.state!=1u {return false;}
 if ex_parameters.valid==0u {nearest_unknown=1u;return false;}
 let admission=nearest_material_decision(material);
 if admission.x==2u {nearest_unknown=1u;nearest_material_unknown=1u;return false;}
 if admission.x==0u {return false;}
 nearest_admitted_kind=admission.y;return true;
}
fn nearest_save_parameters(slot:u32,primitive:u32) {
  let t=exact_copy_parameter(ex_parameters.t,0u);let u=exact_copy_parameter(ex_parameters.u,1u);let v=exact_copy_parameter(ex_parameters.v,2u);let den=exact_copy_parameter(ex_parameters.denominator,3u);
  nearest_best=ExParameters(t,u,v,den,t.valid&u.valid&v.valid&den.valid);nearest_slot=slot;nearest_primitive=primitive;nearest_kind=nearest_admitted_kind;nearest_best_t=nearest_candidate_t;
}
// Shared exact candidate update; callers own source enumeration and arena reset.
fn nearest_evaluate_and_update(slot:u32,primitive:u32,ray:u32,d:ExVec) {
 let info=exact_layout[slot];
 nearest_exact_calls+=1u;
 let triangle=nearest_source_vertices(slot,primitive,ray);
 let decision=ex_triangle(triangle.a,triangle.b,triangle.c,d);
 if !nearest_admit_decision(info.w,decision) {return;}
 var replace=nearest_best.valid==0u;
 if !replace {
  let cmp=ex_cmp_products(ex_parameters.t,nearest_best.denominator,nearest_best.t,ex_parameters.denominator);
  if cmp.valid==0u {nearest_unknown=1u;return;}
  if cmp.order==0 {nearest_ties+=1u;}
  replace=cmp.order<0||(cmp.order==0&&(slot<nearest_slot||(slot==nearest_slot&&primitive<nearest_primitive)));
 }
 if replace {
  nearest_save_parameters(slot,primitive);
 }
}
fn nearest_consider(slot:u32,primitive:u32,ray:u32,d:ExVec) {
 if slot>=nearest_active[0]||nearest_active[4u+slot]==0u {return;}
 nearest_candidates+=1u;ex_cursor=7u;ex_parameters.valid=0u;
 let info=exact_layout[slot];let first=info.z+primitive*3u;
 if (nearest_counts.values[4095u].w&1u)!=0u&&nearest_interval_miss(slot,first,ray,(nearest_counts.values[4095u].w&4u)!=0u) {nearest_interval_rejected+=1u;return;}
 if (nearest_counts.values[4095u].w&4u)!=0u&&nearest_best.valid!=0u&&nearest_best_t.valid!=0u&&nearest_candidate_t.valid!=0u&&nearest_candidate_t.lo>nearest_best_t.hi {
  // Preserve conservative unknown-material reporting even behind a known hit.
  let admitted=nearest_material_decision(info.w);ex_cursor=7u;
  if admitted.x==1u {nearest_interval_rejected+=1u;return;}
 }
 nearest_evaluate_and_update(slot,primitive,ray,d);
}

fn nearest_native_word(bits:u32)->bool {let absolute=bits&2147483647u;let e=absolute>>23u;return absolute==0u||(e!=0u&&e!=255u);}
fn nearest_canonical_pair(hi:u32,lo:u32)->bool {
 let h=hi&2147483647u;let l=lo&2147483647u;let e=h>>23u;
 if e==255u||(l>>23u)==255u {return false;}if l==0u {return true;}if e<=1u {return false;}
 var limit=0u;if e>=25u {limit=(e-24u)<<23u;}else {limit=1u<<(e-2u);}return l<=limit;
}
// Material coordinates use original UV words and unrounded barycentric ratios.
// Canonical parameter pairs are outputs only, never interpolation operands.
fn nearest_exact_uv(axis:u32)->ExPairBits {
 let info=exact_layout[nearest_slot];let first=info.z+nearest_primitive*3u;
 let a=ex_float(exact_vertices[(info.y+exact_indices[first])*12u+10u+axis],-149);
 let b=ex_float(exact_vertices[(info.y+exact_indices[first+1u])*12u+10u+axis],-149);
 let c=ex_float(exact_vertices[(info.y+exact_indices[first+2u])*12u+10u+axis],-149);
 let numerator=ex_add(ex_add(ex_mul(a,nearest_best.denominator),ex_mul(ex_sub(b,a),nearest_best.u)),ex_mul(ex_sub(c,a),nearest_best.v));
 return ex_ratio_pair(numerator,nearest_best.denominator,-149);
}
// Explicit guarded producer protocol: inactive is distinct from a miss;
// unsupported coefficients cannot turn into fabricated intersections.

fn nearest_start(index:u32,stride:u32,worker:u32)->ExVec {
 let ray=index*20u;
 nearest_candidate_t=wi_bad();nearest_best_t=wi_bad();
 ex_begin=worker*EX_ARENA_WORDS;ex_cursor=7u;ex_peak=7u;nearest_best.valid=0u;nearest_slot=4294967295u;nearest_primitive=4294967295u;nearest_candidates=0u;nearest_ties=0u;nearest_unknown=0u;nearest_seen_count=0u;nearest_kind=0u;nearest_material_unknown=0u;nearest_interval_rejected=0u;nearest_exact_calls=0u;nearest_node_rejected=0u;
 let out=index*stride;
 exact_output[out]=vec4<u32>(2u,128u,2048u,0u);exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),4294967295u,4294967295u);
 let d=ExVec(ex_add(ex_float(exact_ray_word(ray+8u),-149),ex_float(exact_ray_word(ray+12u),-149)),ex_add(ex_float(exact_ray_word(ray+9u),-149),ex_float(exact_ray_word(ray+13u),-149)),ex_add(ex_float(exact_ray_word(ray+10u),-149),ex_float(exact_ray_word(ray+14u),-149)));
 // Persist direction operands outside candidate temporaries.
 let dx=exact_copy_parameter(d.x,4u);let dy=exact_copy_parameter(d.y,5u);let dz=exact_copy_parameter(d.z,6u);
 return ExVec(dx,dy,dz);
}
fn nearest_finish(index:u32,stride:u32,native:bool) {
 let ray=index*20u;let out=index*stride;
 var t=ExPairBits(0u,0u,0u,0u);var u=t;var v=t;
 if nearest_best.valid!=0u&&nearest_unknown==0u {ex_cursor=7u;t=ex_ratio_pair(nearest_best.t,nearest_best.denominator,-149);ex_cursor=7u;u=ex_ratio_pair(nearest_best.u,nearest_best.denominator,0);ex_cursor=7u;v=ex_ratio_pair(nearest_best.v,nearest_best.denominator,0);if (t.valid&u.valid&v.valid)==0u {nearest_unknown=1u;}}
 var uv_x=ExPairBits(0u,0u,0u,0u);var uv_y=uv_x;
 if stride>=7u&&nearest_best.valid!=0u&&nearest_unknown==0u {
  ex_cursor=7u;uv_x=nearest_exact_uv(0u);ex_cursor=7u;uv_y=nearest_exact_uv(1u);
 }
 var state=select(0u,1u,nearest_best.valid!=0u);if nearest_unknown!=0u {state=2u;}
 if nearest_unknown!=0u {nearest_slot=4294967295u;nearest_primitive=4294967295u;nearest_kind=0u;t=ExPairBits(0u,0u,0u,0u);u=t;v=t;}
 var reason=select(0u,64u,nearest_unknown!=0u);if nearest_material_unknown!=0u {reason=256u;}
 exact_output[out]=vec4<u32>(state,reason,2048u,ex_peak);
 exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),nearest_slot,nearest_primitive);
 exact_output[out+2u]=vec4<u32>(t.hi,t.lo,u.hi,u.lo);exact_output[out+3u]=vec4<u32>(v.hi,v.lo,t.valid&u.valid&v.valid,t.residual_nonzero|(u.residual_nonzero<<1u)|(v.residual_nonzero<<2u));
 var material=4294967295u;if nearest_best.valid!=0u&&nearest_unknown==0u {material=exact_layout[nearest_slot].w;}
 exact_output[out+4u]=vec4<u32>(nearest_candidates,material,select(1u,0u,native)|(select(0u,1u,nearest_material_enabled())<<1u)|(nearest_kind<<2u)|(nearest_material_unknown<<4u),nearest_ties);
 if stride==6u||stride==8u {exact_output[out+5u]=vec4<u32>(nearest_interval_rejected,nearest_exact_calls,nearest_counts.values[4095u].w,nearest_node_rejected);}
 if stride>=7u {
  let uv=out+select(5u,6u,stride==8u);let valid=uv_x.valid|(uv_y.valid<<1u);
  exact_output[uv]=vec4<u32>(uv_x.hi,uv_x.lo,uv_y.hi,uv_y.lo);
  exact_output[uv+1u]=vec4<u32>(valid,uv_x.residual_nonzero|(uv_y.residual_nonzero<<1u),select(0u,64u,state==1u&&valid!=3u),0u);
 }
}
fn nearest_process_worker(index:u32,stride:u32,worker:u32) {
 let ray=index*20u;if ray+19u>=exact_ray_word_count() {return;}
 if (nearest_counts.values[4095u].w&8u)!=0u {
  let admission=exact_ray_word(ray+3u);
  if admission==0u {nearest_admission_result(index,stride,3u,0u);return;}
  if admission!=1u {nearest_admission_result(index,stride,2u,512u);return;}
 }

 let direction=nearest_start(index,stride,worker);
 // Candidate temporaries begin after all seven persistent operands.
 var native=ex_vec_valid(direction)!=0u&&!ex_vec_zero(direction);
 for(var k=0u;k<3u;k+=1u) {native=native&&nearest_native_word(exact_ray_word(ray+k))&&nearest_native_word(exact_ray_word(ray+8u+k))&&nearest_canonical_pair(exact_ray_word(ray+k),exact_ray_word(ray+4u+k))&&nearest_canonical_pair(exact_ray_word(ray+8u+k),exact_ray_word(ray+12u+k));}
 // A canonical low word is not covered by a fixed local BLAS padding.
 // Never substitute a high-only ray for an exact nonzero low origin/direction.
 for(var k=0u;k<3u;k+=1u) {native=native&&(exact_ray_word(ray+4u+k)&2147483647u)==0u&&(exact_ray_word(ray+12u+k)&2147483647u)==0u;}

 if ex_vec_valid(direction)==0u {nearest_unknown=1u;}
 else if native {
  let origin=vec3<f32>(bitcast<f32>(exact_ray_word(ray)),bitcast<f32>(exact_ray_word(ray+1u)),bitcast<f32>(exact_ray_word(ray+2u)));
  let high=vec3<f32>(bitcast<f32>(exact_ray_word(ray+8u)),bitcast<f32>(exact_ray_word(ray+9u)),bitcast<f32>(exact_ray_word(ray+10u)));
  var query:ray_query;rayQueryInitialize(&query,nearest_scene,RayDesc(0u,255u,0.,1.,origin,high));
  while rayQueryProceed(&query) {let candidate=rayQueryGetCandidateIntersection(&query);if candidate.kind==3u {nearest_consider(candidate.instance_custom_data,candidate.primitive_index,ray,direction);if nearest_seen_count<64u {nearest_seen[nearest_seen_count]=vec2<u32>(candidate.instance_custom_data,candidate.primitive_index);nearest_seen_count+=1u;}}}
 }
 // Always supplement native candidates with conservative original-source traversal.
 // Native broad-phase completeness is never assumed, including zero-low rays.
 if ex_vec_valid(direction)!=0u {
  if nearest_make_box(ray,direction) {nearest_source_guard(ray,direction);}
  else {
   for(var slot=0u;slot<nearest_active[0];slot+=1u) {if nearest_active[4u+slot]==0u {continue;}if !nearest_instance_possible(slot,ray,direction) {continue;}let count=nearest_counts.values[exact_layout[slot].x*2u].x;for(var primitive=0u;primitive<count;primitive+=1u) {nearest_consider(slot,primitive,ray,direction);}}
  }
 }
 nearest_finish(index,stride,native);
}
fn nearest_process(index:u32,stride:u32) {nearest_process_worker(index,stride,index);}
@compute @workgroup_size(1) fn main_nearest(@builtin(global_invocation_id) id:vec3<u32>) {nearest_process(id.x,5u);}
@compute @workgroup_size(1) fn main_nearest_uv(@builtin(global_invocation_id) id:vec3<u32>) {nearest_process(id.x,7u);}
@compute @workgroup_size(1) fn main_nearest_uv_stats(@builtin(global_invocation_id) id:vec3<u32>) {nearest_process(id.x,8u);}
@compute @workgroup_size(1) fn main_nearest_stats(@builtin(global_invocation_id) id:vec3<u32>) {nearest_process(id.x,6u);}

// Independent target-primitive admission audit: exact output plus the certificate.
@compute @workgroup_size(1) fn main_interval_probe(@builtin(global_invocation_id) id:vec3<u32>) {
 let ray=id.x*20u;if ray+19u>=exact_ray_word_count() {return;}
 let slot=exact_ray_word(ray+16u);let first=exact_layout[slot].z+exact_ray_word(ray+17u)*3u;
 let miss=nearest_interval_miss(slot,first,ray,false);
 ex_begin=id.x*EX_ARENA_WORDS;
 exact_process(id.x,id.x,5u,true);
 exact_output[id.x*5u+4u]=vec4<u32>(select(0u,1u,miss),0u,0u,0u);
}

// Qualification-only target query: expose outward t bounds, never upload gold.
@compute @workgroup_size(1) fn main_order_interval_probe(@builtin(global_invocation_id) id:vec3<u32>) {
 let ray=id.x*20u;if ray+19u>=exact_ray_word_count() {return;}
 let slot=exact_ray_word(ray+16u);let first=exact_layout[slot].z+exact_ray_word(ray+17u)*3u;
 let miss=nearest_interval_miss(slot,first,ray,true);
 let bound=nearest_candidate_t;
 ex_begin=id.x*EX_ARENA_WORDS;
 exact_process(id.x,id.x,5u,true);
 exact_output[id.x*5u+4u]=vec4<u32>(select(0u,1u,miss),bound.valid,bitcast<u32>(bound.lo),bitcast<u32>(bound.hi));
}

// Audit the exact-source certificate actually used for unique-winner selection.
@compute @workgroup_size(1)
fn main_source_order_interval_probe(@builtin(global_invocation_id) id:vec3<u32>) {
 let ray=id.x*20u;if ray+19u>=exact_ray_word_count() {return;}
 for(var axis=0u;axis<3u;axis++) {
  nearest_node_origin[axis]=wi_add(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_ray_word(ray+4u+axis)));
  nearest_node_direction[axis]=wi_add(wi_source_word(exact_ray_word(ray+8u+axis)),wi_source_word(exact_ray_word(ray+12u+axis)));
 }
 let slot=exact_ray_word(ray+16u);let first=exact_layout[slot].z+exact_ray_word(ray+17u)*3u;
 let proof=source_triangle_certificate(slot,first,ray,true);
 ex_begin=id.x*EX_ARENA_WORDS;
 exact_process(id.x,id.x,5u,true);
 exact_output[id.x*5u+4u]=vec4<u32>(select(0u,1u,proof.intersection.state==0u),proof.t.valid,bitcast<u32>(proof.t.lo),bitcast<u32>(proof.t.hi));
}
