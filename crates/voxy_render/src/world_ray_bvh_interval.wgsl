// Optional aligned output tail; zero preserves the original query allocation.
override WORLD_HIT_POSITION_BASE:u32=0u;
fn nearest_clear_position(index:u32) {
 if WORLD_HIT_POSITION_BASE==0u {return;}
 let out=WORLD_HIT_POSITION_BASE+index*2u;
 exact_output[out]=vec4<u32>(0u);exact_output[out+1u]=vec4<u32>(0u);
}
// Shared conservative source-node rejection. Unproven bounds keep the node.
struct NearestCounts {values:array<vec4<u32>,4096>}
@group(0) @binding(9) var<uniform> nearest_counts:NearestCounts;
@group(0) @binding(10) var<storage,read> nearest_active:array<u32>;
var<private> nearest_box:array<vec2<u32>,3>;
var<private> nearest_node_origin:array<WorldInterval,3>;
var<private> nearest_node_direction:array<WorldInterval,3>;
fn nearest_ordered(bits:u32)->u32 {if (bits&2147483647u)==0u {return 2147483648u;}return select(bits^2147483648u,~bits,(bits>>31u)!=0u);}
fn nearest_div_endpoint(a:f32,b:f32)->WorldInterval {
 if !wi_normal_or_zero(a)||!wi_normal_or_zero(b)||b<=0. {return wi_bad();}
 if a==0. {return wi_scalar(0.);}
 let q=a/b;if !wi_normal_or_zero(q)||q==0. {return wi_bad();}
 var lo=q;var hi=q;for(var k=0u;k<4u;k+=1u) {lo=wi_down(lo);hi=wi_up(hi);}
 let low=wi_mul(wi_scalar(lo),wi_scalar(b));let high=wi_mul(wi_scalar(hi),wi_scalar(b));
 if (low.valid&high.valid)==0u||low.hi>a||high.lo<a {return wi_bad();}
 return WorldInterval(lo,hi,1u);
}
fn nearest_div_interval(a:WorldInterval,b:WorldInterval)->WorldInterval {
 if (a.valid&b.valid)==0u||b.lo<=0. {return wi_bad();}
 // Positive denominator: select the exact monotone endpoint pair by numerator sign.
 let p=nearest_div_endpoint(a.lo,select(b.hi,b.lo,a.lo<0.));let q=nearest_div_endpoint(a.hi,select(b.lo,b.hi,a.hi<0.));
 return WorldInterval(p.lo,q.hi,p.valid&q.valid);
}
fn nearest_node_bound(bits:u32,upper:bool)->WorldInterval {
 let absolute=bits&2147483647u;
 if absolute!=0u&&absolute<16777216u {
  if (bits>>31u)==0u {return wi_source_word(select(0u,16777216u,upper));}
  return wi_source_word(select(2164260864u,0u,upper));
 }
 return wi_source_word(bits);
}
fn nearest_node_miss_before(base:u32,ray:u32,far:f32)->bool {
 var entry=0.;var exit=far;
 for(var axis=0u;axis<3u;axis+=1u) {
  let origin=nearest_node_origin[axis];
  var direction=nearest_node_direction[axis];
  var lo=wi_sub(nearest_node_bound(nearest_active[base+axis],false),origin);var hi=wi_sub(nearest_node_bound(nearest_active[base+3u+axis],true),origin);
  if (lo.valid&hi.valid&direction.valid)==0u {return false;}
  if direction.lo==0.&&direction.hi==0. {if wi_positive(lo)||wi_negative(hi) {return true;}continue;}
  if wi_negative(direction) {direction=wi_neg(direction);let old_lo=lo;lo=wi_neg(hi);hi=wi_neg(old_lo);}
  if !wi_positive(direction) {return false;}
  let lower=nearest_div_interval(lo,direction);let upper=nearest_div_interval(hi,direction);
  if (lower.valid&upper.valid)==0u {return false;}
  entry=max(entry,lower.lo);exit=min(exit,upper.hi);
  if entry>exit {return true;}
 }
 return false;
}
fn nearest_material_base()->u32 {return nearest_active[2]+nearest_active[1]*8u;}
fn nearest_material_enabled()->bool {return nearest_active[nearest_material_base()+1u]!=0u;}
fn nearest_admission_result(index:u32,stride:u32,state:u32,reason:u32) {
 nearest_clear_position(index);
 let out=index*stride;let ray=index*20u;
 exact_output[out]=vec4<u32>(state,reason,2048u,1u);
 exact_output[out+1u]=vec4<u32>(exact_ray_word(ray+18u),exact_ray_word(ray+19u),4294967295u,4294967295u);
 exact_output[out+2u]=vec4<u32>(0u);exact_output[out+3u]=vec4<u32>(0u);
 exact_output[out+4u]=vec4<u32>(0u,4294967295u,0u,0u);
 if stride==6u||stride==8u {exact_output[out+5u]=vec4<u32>(0u,0u,nearest_counts.values[4095u].w,0u);}
 if stride>=7u {let uv=out+select(5u,6u,stride==8u);exact_output[uv]=vec4<u32>(0u);exact_output[uv+1u]=vec4<u32>(0u);}
}

fn nearest_node_miss(base:u32,ray:u32)->bool {return nearest_node_miss_before(base,ray,1.);}

struct SourceLeafRange {slot:u32,first:u32,count:u32,valid:u32}
fn source_leaf_range(left:u32,tag:u32)->SourceLeafRange {
 var slot=left;var first=0u;var count=0u;
 if tag==4294967294u {
  let begin=4u+nearest_active[0];let end=nearest_active[2];
  if end<begin||left>=(end-begin)/3u {return SourceLeafRange(0u,0u,0u,0u);}
  let at=begin+left*3u;slot=nearest_active[at];first=nearest_active[at+1u];count=nearest_active[at+2u];
 }
 if slot>=nearest_active[0] {return SourceLeafRange(0u,0u,0u,0u);}
 let total=nearest_counts.values[exact_layout[slot].x*2u].x;
 if tag==4294967295u {count=total;}
 if first>total||count>total-first {return SourceLeafRange(0u,0u,0u,0u);}
 return SourceLeafRange(slot,first,count,1u);
}
