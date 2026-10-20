// Candidate only. Intersect the full-resolution depth image as closed pixel
// footprints at their stored NDC depth. No fixed ray-step budget or downsample.
// Screen-segment parameter is affine in NDC; world t requires perspective
// reconstruction by the caller before any production lighting integration.
struct DepthDdaHit { pixel:vec2<i32>, parameter:f32, visited:u32 }
fn dda_miss(visited:u32)->DepthDdaHit {return DepthDdaHit(vec2<i32>(-1),1e30,visited);}
fn dda_touch(image:texture_2d<f32>,point:vec2<f32>,level:f32,parameter:f32,visited:u32)->DepthDdaHit {
 let size=vec2<i32>(textureDimensions(image));let hi=vec2<i32>(floor(point));
 let lo=hi-vec2<i32>(select(0,1,point.x==floor(point.x)),select(0,1,point.y==floor(point.y)));
 // Row-major order resolves equal-depth edge/corner ties deterministically.
 for(var y=lo.y;y<=hi.y;y++) {for(var x=lo.x;x<=hi.x;x++) {
  if x<0||y<0||x>=size.x||y>=size.y {continue;}
  let depth=textureLoad(image,vec2<i32>(x,y),0).x;
  if depth>=0.&&depth<1.&&depth==level {return DepthDdaHit(vec2<i32>(x,y),parameter,visited);}
 }}
 return dda_miss(visited);
}
// Exact cancellation predicate for finite float32 coefficients. Integer
// mantissa products avoid any dependence on fused floating multiply-add,
// including overflow/underflow of the corresponding float32 products.
struct DdaProduct { words:vec2<u32>, exponent:i32, negative:bool, valid:bool }
fn dda_product(a:f32,b:f32)->DdaProduct {
 let ia=bitcast<u32>(a);let ib=bitcast<u32>(b);let ea=(ia>>23u)&255u;let eb=(ib>>23u)&255u;
 if ea==255u||eb==255u {return DdaProduct(vec2<u32>(0),0,false,false);}
 let ma=(ia&0x7fffffu)|select(0u,0x800000u,ea!=0u);let mb=(ib&0x7fffffu)|select(0u,0x800000u,eb!=0u);
 var exponent=select(-149,i32(ea)-150,ea!=0u)+select(-149,i32(eb)-150,eb!=0u);
 let p=(ma&65535u)*(mb&65535u);let cross=(ma>>16u)*(mb&65535u)+(ma&65535u)*(mb>>16u);
 var lo=p+(cross<<16u);var hi=(ma>>16u)*(mb>>16u)+(cross>>16u)+select(0u,1u,lo<p);
 if lo==0u&&hi==0u {return DdaProduct(vec2<u32>(0),0,false,true);}
 let zeros=select(32u+countTrailingZeros(hi),countTrailingZeros(lo),lo!=0u);
 if zeros>0u {if zeros<32u {lo=(lo>>zeros)|(hi<<(32u-zeros));hi>>=zeros;}else{lo=hi>>(zeros-32u);hi=0u;}}
 exponent+=i32(zeros);
 return DdaProduct(vec2<u32>(lo,hi),exponent,((ia^ib)>>31u)!=0u,true);
}
fn dda_exact_edge_zero(a:f32,b:f32,c:f32,d:f32)->bool {
 let p=dda_product(a,b);let q=dda_product(c,d);
 if !p.valid||!q.valid {return false;}
 if all(p.words==vec2<u32>(0)) {return all(q.words==vec2<u32>(0));}
 return p.negative!=q.negative&&p.exponent==q.exponent&&all(p.words==q.words);
}
fn dda_plane_point(begin:vec3<f32>,delta:vec3<f32>,depth:f32,parameter:f32)->vec2<f32> {
 var point=begin.xy+delta.xy*parameter;
 for(var axis=0u;axis<2u;axis++) {
  let edge=round(point[axis]);
  if dda_exact_edge_zero(begin[axis]-edge,delta.z,delta[axis],depth-begin.z) {point[axis]=edge;}
 }
 return point;
}
fn trace_depth_dda(image:texture_2d<f32>,begin:vec3<f32>,end:vec3<f32>)->DepthDdaHit {
 let size=vec2<i32>(textureDimensions(image));let limit=vec3<f32>(vec2<f32>(size),1.);
 let delta=end-begin;
 // Numerical domains unsupported by this candidate are distinct from misses;
 // a future renderer caller must retain its existing complete fallback.
 if any((bitcast<vec3<u32>>(begin)&vec3<u32>(0x7f800000u))==vec3<u32>(0x7f800000u))||any((bitcast<vec3<u32>>(end)&vec3<u32>(0x7f800000u))==vec3<u32>(0x7f800000u))||any((bitcast<vec3<u32>>(delta)&vec3<u32>(0x7f800000u))==vec3<u32>(0x7f800000u)) {return dda_miss(0xffffffffu);}
 var low=0.;var high=1.;
 for(var axis=0u;axis<3u;axis++) {
  if delta[axis]==0. {if begin[axis]<0.||begin[axis]>limit[axis] {return dda_miss(0u);}continue;}
  let a=-begin[axis]/delta[axis];let b=(limit[axis]-begin[axis])/delta[axis];
  low=max(low,min(a,b));high=min(high,max(a,b));
 }
 if low>high {return dda_miss(0u);}
 var cell=clamp(vec2<i32>(floor(begin.xy+delta.xy*low)),vec2<i32>(0),size-1);
 let direction=vec2<i32>(select(-1,1,delta.x>=0.),select(-1,1,delta.y>=0.));
 var entry=low;var visited=0u;
 for(var step=0u;step<u32(size.x+size.y+4);step++) {
  if any(cell<vec2<i32>(0))||any(cell>=size) {break;}
  visited+=1u;
  let at_entry=dda_touch(image,begin.xy+delta.xy*entry,begin.z+delta.z*entry,entry,visited);
  if at_entry.pixel.x>=0 {return at_entry;}
  var next_x=1e30;var next_y=1e30;
  if delta.x!=0. {next_x=(f32(cell.x+select(0,1,delta.x>0.))-begin.x)/delta.x;}
  if delta.y!=0. {next_y=(f32(cell.y+select(0,1,delta.y>0.))-begin.y)/delta.y;}
  let exit=max(entry,min(high,min(next_x,next_y)));
  // A ray parallel to a grid edge belongs to both closed footprints for
  // its entire interval, not merely when entering/leaving the cell.
  let left=cell-vec2<i32>(select(0,1,delta.x==0.&&begin.x==floor(begin.x)),select(0,1,delta.y==0.&&begin.y==floor(begin.y)));
  var closest=dda_miss(visited);
  for(var y=left.y;y<=cell.y;y++) {for(var x=left.x;x<=cell.x;x++) {
   if x<0||y<0||x>=size.x||y>=size.y {continue;}
   let depth=textureLoad(image,vec2<i32>(x,y),0).x;
   if depth>=0.&&depth<1.&&delta.z!=0. {
    let parameter=(depth-begin.z)/delta.z;
    let point=dda_plane_point(begin,delta,depth,parameter);
    if parameter>=entry&&parameter<=exit&&all(point>=vec2<f32>(f32(x),f32(y)))&&all(point<=vec2<f32>(f32(x+1),f32(y+1))) {
     let hit=dda_touch(image,point,depth,parameter,visited);
     let index=hit.pixel.y*size.x+hit.pixel.x;let old_index=closest.pixel.y*size.x+closest.pixel.x;
     if hit.pixel.x>=0&&(closest.pixel.x<0||hit.parameter<closest.parameter||(hit.parameter==closest.parameter&&index<old_index)) {closest=hit;}
    }
   }
  }}
  if closest.pixel.x>=0 {return closest;}
  // Explicitly visit point-only neighbours at mixed-direction grid corners.
  var boundary=begin.xy+delta.xy*exit;
  if next_x<=exit {boundary.x=f32(cell.x+select(0,1,delta.x>0.));}
  if next_y<=exit {boundary.y=f32(cell.y+select(0,1,delta.y>0.));}
  let touch=dda_touch(image,boundary,begin.z+delta.z*exit,exit,visited);
  if touch.pixel.x>=0 {return touch;}
  if exit>=high {break;}
  if next_x<=exit {cell.x+=direction.x;}
  if next_y<=exit {cell.y+=direction.y;}
  entry=exit;
 }
 return dda_miss(visited);
}
