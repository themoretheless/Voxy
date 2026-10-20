// Experimental exact traversal, off by default. Bounds reject
// candidates conservatively; surviving pixels use unchanged paired arithmetic.
override ARCHIVE_STACK_TRAVERSAL:bool=false;
override ARCHIVE_STACK_PIXEL_LEAVES:bool=false;
override ARCHIVE_STACK_RECIPROCAL_CACHE:bool=false;
override ARCHIVE_STACK_NEAR_FIRST:bool=false;
fn hierarchy_node_misses(ray:DdaIntervalRay,lower:vec3<f32>,upper:vec3<f32>,maximum:f32,stationary:vec3<bool>,inverse:array<DdaInterval,3>)->bool {
 var lo=0.;var hi=maximum;
 for(var axis=0u;axis<3u;axis++) {
  let origin=ray.begin[axis];let delta=ray.delta[axis];
  if !origin.valid||!delta.valid {continue;}
  if stationary[axis] {
   if origin.bounds.y<lower[axis]||origin.bounds.x>upper[axis] {return true;}
   continue;
  }
  let lower_offset=dda_interval_add(DdaInterval(vec2<f32>(lower[axis]),true),dda_interval_neg(origin));
  let upper_offset=dda_interval_add(DdaInterval(vec2<f32>(upper[axis]),true),dda_interval_neg(origin));
  var a:DdaInterval;var b:DdaInterval;
  if ARCHIVE_STACK_RECIPROCAL_CACHE {
   a=dda_interval_mul(lower_offset,inverse[axis]);b=dda_interval_mul(upper_offset,inverse[axis]);
  }else {a=dda_interval_div(lower_offset,delta);b=dda_interval_div(upper_offset,delta);}

  // Invalid division, including an interval spanning zero, cannot reject.
  if !a.valid||!b.valid {
   // A direction interval spanning zero cannot be divided, but its full
   // position hull can still exclude distant slabs. Keeping the entire hull
   // preserves almost-parallel low components instead of rounding them away.
   let hull=dda_interval_add(origin,dda_interval_mul(delta,DdaInterval(vec2<f32>(lo,hi),true)));
   if hull.valid&&(hull.bounds.y<lower[axis]||hull.bounds.x>upper[axis]) {return true;}
   continue;
  }
  lo=max(lo,min(a.bounds.x,b.bounds.x));hi=min(hi,max(a.bounds.y,b.bounds.y));
  if lo>hi {return true;}
 }
 return false;
}
fn hierarchy_choose(current:DepthDdaPairHit,candidate:DepthDdaPairHit,width:i32)->DepthDdaPairHit {
 if candidate.pixel.x<0 {return current;}
 let index=candidate.pixel.y*width+candidate.pixel.x;let previous=current.pixel.y*width+current.pixel.x;
 if current.pixel.x<0||projection_pair_less(candidate.parameter,current.parameter)||(precise_equal(candidate.parameter,current.parameter)&&index<previous) {return candidate;}
 return current;
}
fn hierarchy_plane_candidate(image:texture_2d<f32>,begin:DdaVectorPair,delta:DdaVectorPair,low:vec2<f32>,high:vec2<f32>,lower:vec2<i32>,upper:vec2<i32>,depth:f32,visited:u32)->DepthDdaPairHit {
 let parameter=projection_pair_div(projection_pair_add(vec2<f32>(depth,0.),-precise_axis(begin,2u)),precise_axis(delta,2u));
 if !precise_le(low,parameter)||!precise_le(parameter,high) {return precise_miss(visited);}
 let point=precise_point(begin,delta,parameter);let px=precise_axis(point,0u);let py=precise_axis(point,1u);
 if !precise_le(vec2<f32>(f32(lower.x),0.),px)||!precise_le(px,vec2<f32>(f32(upper.x),0.))||!precise_le(vec2<f32>(f32(lower.y),0.),py)||!precise_le(py,vec2<f32>(f32(upper.y),0.)) {return precise_miss(visited);}
 return precise_touch(image,point,vec2<f32>(depth,0.),parameter,visited);
}
fn trace_depth_hierarchy_candidate(image:texture_2d<f32>,begin:DdaVectorPair,end:DdaVectorPair)->DepthDdaPairHit {
 let size=vec2<i32>(textureDimensions(image));var dh=vec4<f32>(0.);var dl=vec4<f32>(0.);
 for(var axis=0u;axis<3u;axis++) {let d=projection_pair_add(precise_axis(end,axis),-precise_axis(begin,axis));dh[axis]=d.x;dl[axis]=d.y;}
 let delta=DdaVectorPair(dh,dl);let dz=precise_axis(delta,2u);
 // Coplanar rays retain the original exact boundary-touch behavior.
 let levels=hierarchy_candidate_levels();
 if precise_equal(dz,vec2<f32>(0.))||levels>21u {return trace_depth_dda_paired(image,begin,end);}
 var low=vec2<f32>(0.);var high=vec2<f32>(1.,0.);let limit=vec3<f32>(vec2<f32>(size),1.);
 for(var axis=0u;axis<3u;axis++) {
  let b=precise_axis(begin,axis);let d=precise_axis(delta,axis);
  if precise_equal(d,vec2<f32>(0.)) {if projection_pair_less(b,vec2<f32>(0.))||projection_pair_less(vec2<f32>(limit[axis],0.),b) {return precise_miss(0u);}continue;}
  let a=projection_pair_div(-b,d);let z=projection_pair_div(projection_pair_add(vec2<f32>(limit[axis],0.),-b),d);
  low=precise_max(low,precise_min(a,z));high=precise_min(high,precise_max(a,z));
 }
 if projection_pair_less(high,low) {return precise_miss(0u);}
 let ray=dda_interval_ray(begin,end);
 var inverse:array<DdaInterval,3>;
 if ARCHIVE_STACK_RECIPROCAL_CACHE {
  for(var axis=0u;axis<3u;axis++) {inverse[axis]=dda_interval_div(DdaInterval(vec2<f32>(1.),true),ray.delta[axis]);}
 }

 // This rounded key changes visitation order only. Original paired
 // coordinates, candidate coverage and exact hit/tie tests stay unchanged.
 var order_origin=vec2<f32>(0.);
 if ARCHIVE_STACK_NEAR_FIRST {order_origin=precise_point(begin,delta,low).high.xy;}
 let stationary=vec3<bool>(precise_equal(precise_axis(delta,0u),vec2<f32>(0.)),precise_equal(precise_axis(delta,1u),vec2<f32>(0.)),false);
 var stack:array<vec3<i32>,64>;var pending=1u;stack[0]=vec3<i32>(0,0,i32(levels)-1);
 var closest=precise_miss(0u);var visited=0u;
 loop {
  if pending==0u {break;}
  pending-=1u;let node=stack[pending];let stride=hierarchy_candidate_leaf_extent()<<u32(node.z);let start=node.xy*stride;
  if any(start>=size) {continue;}
  let stop=min(start+vec2<i32>(stride),size);let depths=hierarchy_candidate_range(node.xy,node.z);
  if depths.x==1. {continue;}
  var maximum=1.;if closest.pixel.x>=0 {let bound=dda_interval_pair(closest.parameter);if bound.valid {maximum=min(maximum,bound.bounds.y);}}
  if hierarchy_node_misses(ray,vec3<f32>(vec2<f32>(start),depths.x),vec3<f32>(vec2<f32>(stop),depths.y),maximum,stationary,inverse) {continue;}
  // Equal positive bounds mean all non-sky, valid represented planes have
  // identical Float32 depth. Test that plane once and still sample the actual
  // depth texture at its exact intersection, retaining holes and closed edges.
  // Zero is excluded to preserve any signed-zero distinction in stored input.
  if depths.x>0.&&depths.x==depths.y {
   visited+=1u;
   closest=hierarchy_choose(closest,hierarchy_plane_candidate(image,begin,delta,low,high,start,stop,depths.x,visited),size.x);
   continue;
  }
  if node.z>0 {
   // Stack capacity affects storage only. Overflow replays the complete
   // original traversal; no candidate is discarded or truncated.
   if pending+4u>64u {return trace_depth_dda_paired(image,begin,end);}
   var preferred=0;
   if ARCHIVE_STACK_NEAR_FIRST {
    let centre=vec2<f32>(start)+vec2<f32>(f32(stride)*0.5);
    preferred=select(0,1,order_origin.x>=centre.x)|select(0,2,order_origin.y>=centre.y);
   }
   for(var child=0;child<4;child++) {
    var quadrant=child;if ARCHIVE_STACK_NEAR_FIRST {quadrant=(3-child)^preferred;}
    stack[pending]=vec3<i32>(node.xy*2+vec2<i32>(quadrant&1,quadrant>>1),node.z-1);pending+=1u;
   }
   continue;
  }
  for(var y=start.y;y<stop.y;y++) {for(var x=start.x;x<stop.x;x++) {
   visited+=1u;let depth=textureLoad(image,vec2<i32>(x,y),0).x;
   if !(depth>=0.&&depth<1.) {continue;}
   closest=hierarchy_choose(closest,hierarchy_plane_candidate(image,begin,delta,low,high,vec2<i32>(x,y),vec2<i32>(x+1,y+1),depth,visited),size.x);
  }}
 }
 closest.visited=visited;return closest;
}
