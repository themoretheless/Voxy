override ARCHIVE_DDA_TILE_EDGE_CACHE:bool=false;
// Optional qualification counter; dead code in every runtime pipeline.
override ARCHIVE_DDA_TILE_EDGE_STATS:bool=false;
var<private> dda_tile_edge_reuses:u32;
override ARCHIVE_DDA_INTERVAL_CACHE:bool=false;
override ARCHIVE_DDA_PLANE_CACHE:bool=false;
override ARCHIVE_DDA_POINT_CACHE:bool=false;
override ARCHIVE_DDA_EDGE_CACHE:bool=false;
override ARCHIVE_DDA_HIERARCHY:bool=false;
override ARCHIVE_DDA_CELL_BOUNDS:bool=false;
override ARCHIVE_DDA_TILES:bool=false;
// Candidate full-resolution traversal retaining projected coordinate residuals.
struct DepthDdaPairHit {pixel:vec2<i32>,parameter:vec2<f32>,visited:u32,padding0:u32,padding1:u32,padding2:u32}
fn precise_miss(visited:u32)->DepthDdaPairHit {return DepthDdaPairHit(vec2<i32>(-1),vec2<f32>(1e30,0.),visited,0u,0u,0u);}
fn precise_equal(a:vec2<f32>,b:vec2<f32>)->bool {return all(a==b);}
fn precise_le(a:vec2<f32>,b:vec2<f32>)->bool {return !projection_pair_less(b,a);}
fn precise_min(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {return select(b,a,projection_pair_less(a,b));}
fn precise_max(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {return select(a,b,projection_pair_less(a,b));}
fn precise_axis(point:DdaVectorPair,axis:u32)->vec2<f32>{return vec2<f32>(point.high[axis],point.low[axis]);}
fn precise_floor(value:vec2<f32>)->i32 {let f=floor(value.x);return i32(f)-select(0,1,value.x==f&&value.y<0.);}
fn precise_integer(value:vec2<f32>)->bool{return value.y==0.&&value.x==floor(value.x);}
fn precise_point(begin:DdaVectorPair,delta:DdaVectorPair,parameter:vec2<f32>)->DdaVectorPair {
 let x=projection_pair_add(precise_axis(begin,0u),projection_pair_mul(precise_axis(delta,0u),parameter));
 let y=projection_pair_add(precise_axis(begin,1u),projection_pair_mul(precise_axis(delta,1u),parameter));
 let z=projection_pair_add(precise_axis(begin,2u),projection_pair_mul(precise_axis(delta,2u),parameter));
 return DdaVectorPair(vec4<f32>(x.x,y.x,z.x,0.),vec4<f32>(x.y,y.y,z.y,0.));
}
// Stored Depth32Float values have an identically zero low component.
// This only admits the original exact touch path; no geometric epsilon is used.
fn precise_touch_possible(level:vec2<f32>)->bool{return level.y==0.&&level.x>=0.&&level.x<1.;}
fn precise_touch(image:texture_2d<f32>,point:DdaVectorPair,level:vec2<f32>,parameter:vec2<f32>,visited:u32)->DepthDdaPairHit {
 // Stored depth is (float32, 0). A nonzero residual cannot satisfy the
 // existing exact equality, so these texture loads have no possible hit.
 if level.y!=0.||level.x<0.||level.x>=1. {return precise_miss(visited);}

 let size=vec2<i32>(textureDimensions(image));let px=precise_axis(point,0u);let py=precise_axis(point,1u);let hi=vec2<i32>(precise_floor(px),precise_floor(py));let lo=hi-vec2<i32>(select(0,1,precise_integer(px)),select(0,1,precise_integer(py)));
 for(var y=lo.y;y<=hi.y;y++){for(var x=lo.x;x<=hi.x;x++){if x<0||y<0||x>=size.x||y>=size.y{continue;}let depth=textureLoad(image,vec2<i32>(x,y),0).x;if depth>=0.&&depth<1.&&precise_equal(vec2<f32>(depth,0.),level){return DepthDdaPairHit(vec2<i32>(x,y),parameter,visited,0u,0u,0u);}}}
 return precise_miss(visited);
}
fn trace_depth_dda_paired(image:texture_2d<f32>,begin:DdaVectorPair,end:DdaVectorPair)->DepthDdaPairHit {
 let size=vec2<i32>(textureDimensions(image));var dh=vec4<f32>(0.);var dl=vec4<f32>(0.);
 for(var axis=0u;axis<3u;axis++){let v=projection_pair_add(precise_axis(end,axis),-precise_axis(begin,axis));dh[axis]=v.x;dl[axis]=v.y;}
 let delta=DdaVectorPair(dh,dl);var low=vec2<f32>(0.);var high=vec2<f32>(1.,0.);let limit=vec3<f32>(vec2<f32>(size),1.);
 for(var axis=0u;axis<3u;axis++) {
  let b=precise_axis(begin,axis);let d=precise_axis(delta,axis);
  if precise_equal(d,vec2<f32>(0.)){if projection_pair_less(b,vec2<f32>(0.))||projection_pair_less(vec2<f32>(limit[axis],0.),b){return precise_miss(0u);}continue;}
  let a=projection_pair_div(-b,d);let z=projection_pair_div(projection_pair_add(vec2<f32>(limit[axis],0.),-b),d);low=precise_max(low,precise_min(a,z));high=precise_min(high,precise_max(a,z));
 }
 if projection_pair_less(high,low){return precise_miss(0u);}
 let first=precise_point(begin,delta,low);var cell=clamp(vec2<i32>(precise_floor(precise_axis(first,0u)),precise_floor(precise_axis(first,1u))),vec2<i32>(0),size-1);var entry=low;var visited=0u;var checked_tile=vec2<i32>(-1);var hierarchy_level=0;
 let dx=precise_axis(delta,0u);let dy=precise_axis(delta,1u);let dz=precise_axis(delta,2u);let bx=precise_axis(begin,0u);let by=precise_axis(begin,1u);let bz=precise_axis(begin,2u);
 let rejection_ray=dda_interval_ray(begin,end);
 let z_begin_interval=dda_interval_pair(bz);
 let z_delta_interval=dda_interval_add(dda_interval_pair(precise_axis(end,2u)),dda_interval_neg(z_begin_interval));
 let positive_x=!projection_pair_less(dx,vec2<f32>(0.));let positive_y=!projection_pair_less(dy,vec2<f32>(0.));let direction=vec2<i32>(select(-1,1,positive_x),select(-1,1,positive_y));
 // Cache only identical divisions: a boundary depends on its own cell axis.
 // Entry equals the previous exit/jump parameter. Preserve its depth pair;
 // XY reconstruction is unnecessary when an exact stored-depth touch is impossible.
 var cached_entry_z=precise_axis(first,2u);
 // Depth bits, origin and dz completely determine this compensated quotient.
 var cached_depth_bits=0xffffffffu;var cached_parameter=vec2<f32>(0.);
 var cached_cell=vec2<i32>(-2147483647);var cached_x=vec2<f32>(1e30,0.);var cached_y=vec2<f32>(1e30,0.);
 for(var step=0u;step<u32(size.x+size.y+4);step++) {
  if any(cell<vec2<i32>(0))||any(cell>=size){break;}visited+=1u;
  if !ARCHIVE_DDA_POINT_CACHE||precise_touch_possible(cached_entry_z) {
   let at_entry=precise_point(begin,delta,entry);let start_hit=precise_touch(image,at_entry,precise_axis(at_entry,2u),entry,visited);if start_hit.pixel.x>=0{return start_hit;}
  }
  if ARCHIVE_DDA_TILES {
   let tile=cell/8;
   if any(tile!=checked_tile) {
    checked_tile=tile;
// Ascend after a proven skip; descend near possible geometry.
    let level_count=select(1u,dda_tile_levels(),ARCHIVE_DDA_HIERARCHY);
    var jumped=false;
    for(var level=min(hierarchy_level,i32(level_count)-1);level>=0;level--) {
    let stride=8<<u32(level);let ancestor=cell/stride;var depth_range=vec2<f32>(1.,0.);
    let left_tile=ancestor-vec2<i32>(select(0,1,precise_equal(dx,vec2<f32>(0.))&&precise_integer(bx)&&i32(bx.x)%stride==0),select(0,1,precise_equal(dy,vec2<f32>(0.))&&precise_integer(by)&&i32(by.x)%stride==0));
    for(var ty=max(0,left_tile.y);ty<=ancestor.y;ty++){for(var tx=max(0,left_tile.x);tx<=ancestor.x;tx++){let r=dda_tile_range(vec2<i32>(tx,ty),level);depth_range=vec2<f32>(min(depth_range.x,r.x),max(depth_range.y,r.y));}}
    let edge=vec2<f32>(vec2<i32>(min(size.x,(ancestor.x+select(0,1,positive_x))*stride),min(size.y,(ancestor.y+select(0,1,positive_y))*stride)));
    let moving=vec2<bool>(!precise_equal(dx,vec2<f32>(0.)),!precise_equal(dy,vec2<f32>(0.)));
    // The bound owner maps invalid depth to zero; minimum==1 means
    // every represented texel is sky, which cannot be a depth hit.
    var skip=depth_range.x==1.;
    if !skip {
     if ARCHIVE_DDA_INTERVAL_CACHE {skip=dda_interval_can_skip_cached(rejection_ray,entry,edge,moving,depth_range);}
     else {skip=dda_interval_can_skip(begin,end,entry,edge,moving,depth_range);}
    }
    if skip {
     var tx=vec2<f32>(1e30,0.);var ty=vec2<f32>(1e30,0.);
     if moving.x {
      if ARCHIVE_DDA_TILE_EDGE_CACHE&&ARCHIVE_DDA_EDGE_CACHE&&cached_cell.x==i32(edge.x) {tx=cached_x;if ARCHIVE_DDA_TILE_EDGE_STATS {dda_tile_edge_reuses+=1u;}}
      else {tx=projection_pair_div(projection_pair_add(vec2<f32>(edge.x,0.),-bx),dx);if ARCHIVE_DDA_TILE_EDGE_CACHE&&ARCHIVE_DDA_EDGE_CACHE {cached_x=tx;cached_cell.x=i32(edge.x);}}
     }
     if moving.y {
      if ARCHIVE_DDA_TILE_EDGE_CACHE&&ARCHIVE_DDA_EDGE_CACHE&&cached_cell.y==i32(edge.y) {ty=cached_y;if ARCHIVE_DDA_TILE_EDGE_STATS {dda_tile_edge_reuses+=1u;}}
      else {ty=projection_pair_div(projection_pair_add(vec2<f32>(edge.y,0.),-by),dy);if ARCHIVE_DDA_TILE_EDGE_CACHE&&ARCHIVE_DDA_EDGE_CACHE {cached_y=ty;cached_cell.y=i32(edge.y);}}
     }
     let jump=precise_min(high,precise_min(tx,ty));
     if projection_pair_less(entry,jump) {
      var boundary=precise_point(begin,delta,jump);
      if ARCHIVE_DDA_POINT_CACHE {cached_entry_z=precise_axis(boundary,2u);}
      if precise_le(tx,jump){boundary.high.x=edge.x;boundary.low.x=0.;}
      if precise_le(ty,jump){boundary.high.y=edge.y;boundary.low.y=0.;}
      let touch=precise_touch(image,boundary,precise_axis(boundary,2u),jump,visited);if touch.pixel.x>=0{return touch;}
      if precise_le(high,jump){return precise_miss(visited);}
      cell=clamp(vec2<i32>(precise_floor(precise_axis(boundary,0u)),precise_floor(precise_axis(boundary,1u))),vec2<i32>(0),size-1);
      if precise_le(tx,jump)&&!positive_x {cell.x-=1;}if precise_le(ty,jump)&&!positive_y {cell.y-=1;}
      entry=jump;hierarchy_level=min(level+1,i32(level_count)-1);jumped=true;break;
     }
    }
    }
    if jumped {continue;}
    hierarchy_level=0;
   }
  }
  // One cache per axis, keyed by the exact integer boundary coordinate.
  // Both pixel and hierarchy expressions divide identical endpoint words.
  let cache_cell=cell+select(vec2<i32>(0),vec2<i32>(select(0,1,positive_x),select(0,1,positive_y)),ARCHIVE_DDA_TILE_EDGE_CACHE);
  var nx=vec2<f32>(1e30,0.);var ny=vec2<f32>(1e30,0.);
  if ARCHIVE_DDA_EDGE_CACHE&&cached_cell.x==cache_cell.x {nx=cached_x;}else{if !precise_equal(dx,vec2<f32>(0.)){nx=projection_pair_div(projection_pair_add(vec2<f32>(f32(cell.x+select(0,1,positive_x)),0.),-bx),dx);}cached_x=nx;cached_cell.x=cache_cell.x;}
  if ARCHIVE_DDA_EDGE_CACHE&&cached_cell.y==cache_cell.y {ny=cached_y;}else{if !precise_equal(dy,vec2<f32>(0.)){ny=projection_pair_div(projection_pair_add(vec2<f32>(f32(cell.y+select(0,1,positive_y)),0.),-by),dy);}cached_y=ny;cached_cell.y=cache_cell.y;}
  let exit=precise_max(entry,precise_min(high,precise_min(nx,ny)));
  // Reject impossible depth planes before compensated division and XY
  // reconstruction. Outward intervals affect rejection only; survivors use
  // exactly the original paired hit test and tie ordering.
  var plane_range=DdaInterval(vec2<f32>(0.,1.),false);
  if ARCHIVE_DDA_CELL_BOUNDS {
   let entry_range=dda_interval_pair(entry);let exit_range=dda_interval_pair(exit);
   let parameters=DdaInterval(vec2<f32>(entry_range.bounds.x,exit_range.bounds.y),entry_range.valid&&exit_range.valid);
   plane_range=dda_interval_add(z_begin_interval,dda_interval_mul(z_delta_interval,parameters));
  }
  let left=cell-vec2<i32>(select(0,1,precise_equal(dx,vec2<f32>(0.))&&precise_integer(bx)),select(0,1,precise_equal(dy,vec2<f32>(0.))&&precise_integer(by)));var closest=precise_miss(visited);
  for(var y=left.y;y<=cell.y;y++){for(var x=left.x;x<=cell.x;x++){
   if x<0||y<0||x>=size.x||y>=size.y{continue;}let depth=textureLoad(image,vec2<i32>(x,y),0).x;
   if depth>=0.&&depth<1.&&!precise_equal(dz,vec2<f32>(0.))&&(!ARCHIVE_DDA_CELL_BOUNDS||!plane_range.valid||(depth>=plane_range.bounds.x&&depth<=plane_range.bounds.y)) {
    var parameter=vec2<f32>(0.);
    let depth_bits=bitcast<u32>(depth);
    if ARCHIVE_DDA_PLANE_CACHE&&depth_bits==cached_depth_bits {parameter=cached_parameter;}
    else{parameter=projection_pair_div(projection_pair_add(vec2<f32>(depth,0.),-bz),dz);if ARCHIVE_DDA_PLANE_CACHE&&dda_finite4(vec4<f32>(parameter,0.,0.)) {cached_parameter=parameter;cached_depth_bits=depth_bits;}}
    // The original exact parameter bounds reject this candidate independently
    // of XY. Reconstruct survivors with the identical original operations.
    if !ARCHIVE_DDA_PLANE_CACHE||(precise_le(entry,parameter)&&precise_le(parameter,exit)) {
    let point=precise_point(begin,delta,parameter);let px=precise_axis(point,0u);let py=precise_axis(point,1u);
    if precise_le(entry,parameter)&&precise_le(parameter,exit)&&precise_le(vec2<f32>(f32(x),0.),px)&&precise_le(px,vec2<f32>(f32(x+1),0.))&&precise_le(vec2<f32>(f32(y),0.),py)&&precise_le(py,vec2<f32>(f32(y+1),0.)) {
     let hit=precise_touch(image,point,vec2<f32>(depth,0.),parameter,visited);let index=hit.pixel.y*size.x+hit.pixel.x;let old_index=closest.pixel.y*size.x+closest.pixel.x;
     if hit.pixel.x>=0&&(closest.pixel.x<0||projection_pair_less(hit.parameter,closest.parameter)||(precise_equal(hit.parameter,closest.parameter)&&index<old_index)){closest=hit;}
    }
    }
   }
  }}
  if closest.pixel.x>=0{return closest;}
  if ARCHIVE_DDA_POINT_CACHE {cached_entry_z=projection_pair_add(bz,projection_pair_mul(dz,exit));}
  if !ARCHIVE_DDA_POINT_CACHE||precise_touch_possible(cached_entry_z) {
   var boundary=precise_point(begin,delta,exit);
   if precise_le(nx,exit){boundary.high.x=f32(cell.x+select(0,1,positive_x));boundary.low.x=0.;}
   if precise_le(ny,exit){boundary.high.y=f32(cell.y+select(0,1,positive_y));boundary.low.y=0.;}
   let touch=precise_touch(image,boundary,precise_axis(boundary,2u),exit,visited);if touch.pixel.x>=0{return touch;}
  }
  if precise_le(high,exit){break;}if precise_le(nx,exit){cell.x+=direction.x;}if precise_le(ny,exit){cell.y+=direction.y;}entry=exit;
 }
 return precise_miss(visited);
}
