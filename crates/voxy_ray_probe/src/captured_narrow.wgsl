struct Grid {tiles:vec2<u32>,viewport:vec2<u32>,first:u32,span:u32,p0:u32,p1:u32}
@group(0) @binding(0) var scene:acceleration_structure;
@group(0) @binding(1) var<storage,read> inputs:array<DdaProjectedSegment>;
@group(0) @binding(2) var source_depth:texture_depth_2d;
@group(0) @binding(3) var tile_depth:texture_2d<f32>;
@group(0) @binding(4) var<storage,read_write> outputs:array<DepthDdaPairHit>;
@group(0) @binding(5) var<storage,read_write> modes:array<vec4<u32>>;
@group(0) @binding(6) var<uniform> grid:Grid;
@group(0) @binding(7) var<storage,read> original_order:array<u32>;
fn dda_tile_range(tile:vec2<i32>,level:i32)->vec2<f32> {return textureLoad(tile_depth,tile,level).xy;}
fn dda_tile_levels()->u32 {return textureNumLevels(tile_depth);}
struct NarrowRay {begin:DdaVectorPair,delta:DdaVectorPair,lo:vec2<f32>,hi:vec2<f32>,interval:DdaIntervalRay,inverse_z:DdaInterval}
struct NarrowResult {hit:DepthDdaPairHit,reason:u32,tests:u32,pair_divisions:u32}
fn overlaps(a:DdaInterval,b:DdaInterval)->bool {return a.bounds.x<=b.bounds.y&&b.bounds.x<=a.bounds.y;}
fn touches_grid(a:DdaInterval)->bool {return ceil(a.bounds.x)<=floor(a.bounds.y);}
// Outward intervals reject candidates only. Every survivor retains the original
// paired quotient and closed-edge/tie tests; invalid intervals reject nothing.
fn coarse_possible(ray:NarrowRay,depth:f32,x:i32,y:i32,lo:DdaInterval,hi:DdaInterval)->bool {
 let numerator=dda_interval_add(DdaInterval(vec2<f32>(depth),true),dda_interval_neg(ray.interval.begin[2]));
 let parameter=dda_interval_mul(numerator,ray.inverse_z);
 if !parameter.valid||!lo.valid||!hi.valid {return true;}
 if parameter.bounds.y<lo.bounds.x||parameter.bounds.x>hi.bounds.y {return false;}
 let xi=dda_interval_add(ray.interval.begin[0],dda_interval_mul(ray.interval.delta[0],parameter));
 let yi=dda_interval_add(ray.interval.begin[1],dda_interval_mul(ray.interval.delta[1],parameter));
 if !xi.valid||!yi.valid {return true;}
 return xi.bounds.y>=f32(x)&&xi.bounds.x<=f32(x+1)&&yi.bounds.y>=f32(y)&&yi.bounds.x<=f32(y+1);
}
fn narrow_tile(ray:NarrowRay,tile:u32,previous:NarrowResult)->NarrowResult {
 var result=previous;let origin=vec2<i32>(i32(tile%grid.tiles.x),i32(tile/grid.tiles.x))*8;
 let end=min(origin+vec2<i32>(8),vec2<i32>(grid.viewport));
 let bz=precise_axis(ray.begin,2u);let dz=precise_axis(ray.delta,2u);
 let lo=dda_interval_pair(ray.lo);let hi=dda_interval_pair(ray.hi);
 for(var y=origin.y;y<end.y;y++){for(var x=origin.x;x<end.x;x++) {
  let depth=textureLoad(source_depth,vec2<i32>(x,y),0);if depth<0.||depth>=1. {continue;}
  result.tests+=1u;
  if !coarse_possible(ray,depth,x,y,lo,hi) {continue;}
  result.pair_divisions+=1u;
  let parameter=projection_pair_div(projection_pair_add(vec2<f32>(depth,0.),-bz),dz);
  if !dda_finite4(vec4<f32>(parameter,0.,0.)) {result.reason|=2u;return result;}
  let pi=dda_interval_pair(parameter);
  if !pi.valid||!lo.valid||!hi.valid {result.reason|=4u;return result;}
  if pi.bounds.y<lo.bounds.x||pi.bounds.x>hi.bounds.y {continue;}
  let xi=dda_interval_add(ray.interval.begin[0],dda_interval_mul(ray.interval.delta[0],pi));
  let yi=dda_interval_add(ray.interval.begin[1],dda_interval_mul(ray.interval.delta[1],pi));
  if !xi.valid||!yi.valid {result.reason|=4u;return result;}
  if xi.bounds.y<f32(x)||xi.bounds.x>f32(x+1)||yi.bounds.y<f32(y)||yi.bounds.x>f32(y+1) {continue;}
  // Ambiguous closed edges retain the original DDA event/touch arithmetic.
  // These outward intervals route work only; they never admit an approximate hit.
  if touches_grid(xi)||touches_grid(yi) {result.reason|=8u;return result;}
  if overlaps(pi,lo)||overlaps(pi,hi) {result.reason|=16u;return result;}
  if !precise_le(ray.lo,parameter)||!precise_le(parameter,ray.hi) {continue;}
  let point=precise_point(ray.begin,ray.delta,parameter);let px=precise_axis(point,0u);let py=precise_axis(point,1u);
  if !precise_le(vec2<f32>(f32(x),0.),px)||!precise_le(px,vec2<f32>(f32(x+1),0.))||!precise_le(vec2<f32>(f32(y),0.),py)||!precise_le(py,vec2<f32>(f32(y+1),0.)) {continue;}
  let hit=precise_touch(source_depth,point,vec2<f32>(depth,0.),parameter,0u);
  let index=hit.pixel.y*i32(grid.viewport.x)+hit.pixel.x;let old=result.hit.pixel.y*i32(grid.viewport.x)+result.hit.pixel.x;
  if hit.pixel.x>=0&&(result.hit.pixel.x<0||projection_pair_less(hit.parameter,result.hit.parameter)||(precise_equal(hit.parameter,result.hit.parameter)&&index<old)) {result.hit=hit;}
 }}
 return result;
}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let position=grid.first+id.x;if id.x>=grid.span||position>=arrayLength(&inputs) {return;}let index=original_order[position];
 let s=inputs[index];var hit=precise_miss(0u);if s.state!=1u {outputs[index]=hit;modes[index]=vec4<u32>(0u);return;}
 let begin=DdaVectorPair(s.begin,s.begin_low);let end=DdaVectorPair(s.end,s.end_low);
 let cursor=dda_cursor_init(source_depth,begin,end);
 if cursor.done {outputs[index]=cursor.hit;modes[index]=vec4<u32>(3u,0u,0u,0u);return;}
 let hardware_delta=s.end.xyz-s.begin.xyz;
 if all(hardware_delta==vec3<f32>(0.))||!dda_finite4(vec4<f32>(hardware_delta,0.))||precise_equal(precise_axis(cursor.delta,2u),vec2<f32>(0.)) {
  outputs[index]=trace_depth_dda_paired(source_depth,begin,end);modes[index]=vec4<u32>(2u,0u,1u,0u);return;
 }
 let rejection=cursor.rejection_ray;
 let inverse_z=dda_interval_div(DdaInterval(vec2<f32>(1.),true),rejection.delta[2]);
 let ray=NarrowRay(begin,cursor.delta,cursor.entry,cursor.high,rejection,inverse_z);
 var result=NarrowResult(hit,0u,0u,0u);var candidates=0u;var query:ray_query;
 rayQueryInitialize(&query,scene,RayDesc(0u,255u,0.,1.,s.begin.xyz,hardware_delta));
 while rayQueryProceed(&query) {
  let candidate=rayQueryGetCandidateIntersection(&query);
  if candidate.kind!=3u||candidate.primitive_index>=grid.tiles.x*grid.tiles.y||candidate.instance_custom_data!=0u {result.reason|=32u;}
  else {candidates+=1u;if result.reason==0u {
   let previous=result.hit;
   result=narrow_tile(ray,candidate.primitive_index,result);
   if result.reason==0u&&result.hit.pixel.x>=0&&(previous.pixel.x<0||any(result.hit.pixel!=previous.pixel)||!precise_equal(result.hit.parameter,previous.parameter)) {
    // Commit an outward upper distance only to prune farther candidate volumes.
    // Exact output retains BOTH parameter words, never this Float32 bound.
    let bound=dda_interval_pair(result.hit.parameter);
    if bound.valid {let committed_distance=min(1.,bound.bounds.y);rayQueryGenerateIntersection(&query,committed_distance);}
   }
  }}
 }
 if result.reason!=0u {hit=trace_depth_dda_paired(source_depth,begin,end);}else{hit=result.hit;hit.visited=result.pair_divisions;}
 outputs[index]=hit;modes[index]=vec4<u32>(select(1u,2u,result.reason!=0u),candidates,result.reason,result.tests);
}

// Matched original DDA control uses the same payload, dispatches and outputs.
@compute @workgroup_size(64) fn original_control(@builtin(global_invocation_id) id:vec3<u32>) {
 let position=grid.first+id.x;if id.x>=grid.span||position>=arrayLength(&inputs) {return;}let index=original_order[position];
 let s=inputs[index];var hit=precise_miss(0u);var mode=0u;
 if s.state==1u {hit=trace_depth_dda_paired(source_depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));mode=2u;}
 outputs[index]=hit;modes[index]=vec4<u32>(mode,0u,0u,0u);
}
