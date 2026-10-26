// Optional scheduling tail in the SAME source hierarchy buffer. Original
// binary nodes and material offsets remain available to the full exact pass.
override WORLD_WIDE_BASE:u32=0u;
override WORLD_WIDE_NEAR_FIRST:bool=false;
fn visibility_fast_walk(ray:u32)->bool {
 if WORLD_WIDE_BASE==0u {return visibility_walk(ray);}
 let length=arrayLength(&nearest_active);let start=WORLD_WIDE_BASE;
 if start>length {visibility_unknown|=64u;return false;}
 if length-start<12u {visibility_unknown|=64u;return false;}
 let count=nearest_active[start+1u];let capacity=nearest_active[start+2u];
 if nearest_active[start]!=0x38595657u||count>(length-start-12u)/68u {visibility_unknown|=64u;return false;}
 // Admission occurs before visiting any leaf. Deep trees retain the complete
 // binary path rather than clipping a frontier to the private stack capacity.
 if capacity>64u {return visibility_walk(ray);}
 let present=nearest_active[start+3u];
 if present!=1u||capacity==0u {visibility_unknown|=64u;return false;}
 var stack:array<u32,64>;stack[0]=start+4u;var size=1u;var visited=0u;
 while size>0u {
  size-=1u;let base=stack[size];visited+=1u;visibility_nodes+=1u;
  if base>length||length-base<8u||visited>count*8u+1u {visibility_unknown|=64u;return false;}
  if visibility_box_valid {
   var overlaps=true;
   for(var axis=0u;axis<3u;axis++) {overlaps=overlaps&&nearest_ordered(nearest_active[base+axis])<=nearest_box[axis].y&&nearest_ordered(nearest_active[base+3u+axis])>=nearest_box[axis].x;}
   if !overlaps {continue;}
  }
  if visibility_node_miss(base,ray) {continue;}
  let left=nearest_active[base+6u];let right=nearest_active[base+7u];
  if right<4294967294u {
   if right!=0u||left>=count {visibility_unknown|=64u;return false;}
   let group=start+12u+left*68u;let children=nearest_active[group];
   if children<2u||children>8u||size+children>64u {visibility_unknown|=64u;return false;}
   for(var lane=0u;lane<children;lane++) {stack[size+lane]=group+4u+lane*8u;}
   if WORLD_WIDE_NEAR_FIRST&&WORLD_FAST_DEFER_NONCLEAR&&WORLD_FAST_CAN_BOUND {
    // Descending keys on the LIFO stack visit nearer hints first. Every child
    // remains present; this hint never proves an intersection or rejects a node.
    var keys:array<f32,8>;
    for(var lane=0u;lane<children;lane++) {keys[lane]=visibility_bound_key(stack[size+lane]);}
    for(var lane=1u;lane<children;lane++) {
     let child=stack[size+lane];let key=keys[lane];var at=lane;
     while at>0u {if keys[at-1u]>=key {break;}keys[at]=keys[at-1u];stack[size+at]=stack[size+at-1u];at-=1u;}
     keys[at]=key;stack[size+at]=child;
    }
   }
   size+=children;continue;
  }
  let leaf=source_leaf_range(left,right);if leaf.valid==0u {visibility_unknown|=64u;return false;}
  let slot=leaf.slot;if nearest_active[4u+slot]==0u {continue;}
  let material=visibility_material(exact_layout[slot].w);if material==0u {continue;}
  let end=leaf.first+leaf.count;
  for(var primitive=leaf.first;primitive<end;primitive++) {if visibility_triangle(slot,primitive,ray,material) {return true;}}
 }
 return false;
}
