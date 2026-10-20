// Reciprocal is admitted only by the existing outward back-multiplication proof.
// Its interval encloses 1/abs(D) for every exact paired-source direction value.
var<private> nearest_inverse:array<WorldInterval,3>;
var<private> nearest_inverse_ready:bool;
fn nearest_prepare_reciprocal() {
 nearest_inverse_ready=true;
 for(var axis=0u;axis<3u;axis++) {
  var direction=nearest_node_direction[axis];
  nearest_inverse[axis]=wi_bad();
  if direction.valid==0u {nearest_inverse_ready=false;continue;}
  if direction.lo==0.&&direction.hi==0. {nearest_inverse[axis]=wi_scalar(0.);continue;}
  if wi_negative(direction) {direction=wi_neg(direction);}
  if !wi_positive(direction) {nearest_inverse_ready=false;continue;}
  let inverse=nearest_div_interval(wi_scalar(1.),direction);
  nearest_inverse[axis]=inverse;
  if inverse.valid==0u||!wi_positive(inverse) {nearest_inverse_ready=false;}
 }
}
// Callers admit both operands as finite normal-or-zero values before reaching
// these helpers. Only the one required outward endpoint is constructed. Results
// still reject FTZ-sensitive cancellation, underflow and overflow explicitly.
struct NearestAdmittedEndpoint {value:f32,valid:u32}
fn nearest_admitted_round(v:f32,upper:bool)->NearestAdmittedEndpoint {
 if !wi_normal_or_zero(v)||v==0. {return NearestAdmittedEndpoint(0.,0u);}
 let endpoint=select(wi_down(v),wi_up(v),upper);
 return NearestAdmittedEndpoint(endpoint,select(0u,1u,wi_normal_or_zero(endpoint)));
}
fn nearest_admitted_add(a:f32,b:f32,upper:bool)->NearestAdmittedEndpoint {
 if a==0. {return NearestAdmittedEndpoint(b,1u);}if b==0. {return NearestAdmittedEndpoint(a,1u);}
 if a== -b {return NearestAdmittedEndpoint(0.,1u);}
 return nearest_admitted_round(projection_add_scalar(a,b),upper);
}
fn nearest_admitted_mul(a:f32,b:f32,upper:bool)->NearestAdmittedEndpoint {
 if a==0.||b==0. {return NearestAdmittedEndpoint(0.,1u);}
 return nearest_admitted_round(projection_mul_scalar(a,b),upper);
}
fn nearest_node_miss_cached(base:u32,ray:u32,far:f32)->bool {
 if !nearest_inverse_ready {return nearest_node_miss_before(base,ray,far);}
 var entry=0.;var exit=far;
 for(var axis=0u;axis<3u;axis++) {
  let origin=nearest_node_origin[axis];let direction=nearest_node_direction[axis];
  let bound_lo=nearest_node_bound(nearest_active[base+axis],false);
  let bound_hi=nearest_node_bound(nearest_active[base+3u+axis],true);
  if (bound_lo.valid&bound_hi.valid&origin.valid&direction.valid)==0u {return nearest_node_miss_before(base,ray,far);}
  // Only these extrema can constrain entry/exit. Every scalar operation still
  // returns the same outward rounding certificate used by the general interval.
  let low=nearest_admitted_add(bound_lo.lo,-origin.hi,false);
  let high=nearest_admitted_add(bound_hi.hi,-origin.lo,true);
  if (low.valid&high.valid)==0u {return nearest_node_miss_before(base,ray,far);}
  var lo=low.value;var hi=high.value;
  if direction.lo==0.&&direction.hi==0. {if lo>0.||hi<0. {return true;}continue;}
  if wi_negative(direction) {let old=lo;lo=-hi;hi=-old;}
  let inverse=nearest_inverse[axis];
  // Inverse is strictly positive. Minimum/maximum products use a single corner
  // selected by the numerator sign; other corners cannot tighten these bounds.
  let lower=nearest_admitted_mul(lo,select(inverse.lo,inverse.hi,lo<0.),false);
  let upper=nearest_admitted_mul(hi,select(inverse.hi,inverse.lo,hi<0.),true);
  if (lower.valid&upper.valid)==0u {return nearest_node_miss_before(base,ray,far);}
  entry=max(entry,lower.value);exit=min(exit,upper.value);
  if entry>exit {return true;}
 }
 return false;
}
