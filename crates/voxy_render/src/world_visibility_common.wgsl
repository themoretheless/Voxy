// Shared source hierarchy, material policy and outward certificates.
// Both the certificate-only pass and the exact fallback use this authority.
var<private> visibility_candidates:u32;
var<private> visibility_misses:u32;
var<private> visibility_hits:u32;
var<private> visibility_exact:u32;
var<private> visibility_nodes:u32;
var<private> visibility_unknown:u32;
var<private> visibility_slot:u32;
var<private> visibility_primitive:u32;
var<private> visibility_route:u32;
var<private> visibility_box_valid:bool;
var<private> visibility_integer_box:bool;
fn visibility_walk(ray:u32)->bool {
 if nearest_active[3]==4294967295u {return false;}
 var stack:array<u32,64>;stack[0]=nearest_active[3];var size=1u;
 while size>0u {
  size-=1u;let node=stack[size];visibility_nodes+=1u;
  if node>=nearest_active[1]||visibility_nodes>nearest_active[1] {visibility_unknown|=64u;return false;}
  let base=nearest_active[2]+node*8u;
  if visibility_box_valid {
   var overlaps=true;
   for(var axis=0u;axis<3u;axis++) {overlaps=overlaps&&nearest_ordered(nearest_active[base+axis])<=nearest_box[axis].y&&nearest_ordered(nearest_active[base+3u+axis])>=nearest_box[axis].x;}
   if !overlaps {continue;}
  }
  if visibility_node_miss(base,ray) {continue;}
  let left=nearest_active[base+6u];let right=nearest_active[base+7u];
  if right<4294967294u {if size+2u>64u {visibility_unknown|=64u;return false;}if visibility_left_first(left,right,ray) {stack[size]=right;stack[size+1u]=left;}
   else {stack[size]=left;stack[size+1u]=right;}
   size+=2u;continue;}
  let leaf=source_leaf_range(left,right);if leaf.valid==0u {visibility_unknown|=64u;return false;}
  let slot=leaf.slot;if nearest_active[4u+slot]==0u {continue;}
  let material=visibility_material(exact_layout[slot].w);if material==0u {continue;}
  let end=leaf.first+leaf.count;
  for(var primitive=leaf.first;primitive<end;primitive++) {if visibility_triangle(slot,primitive,ray,material) {return true;}}
 }
 return false;
}
fn visibility_early(index:u32,stride:u32,state:u32,reason:u32) {
 nearest_admission_result(index,stride,state,reason);
 if stride==6u {exact_output[index*stride+5u]=vec4<u32>(0u);}
}

fn visibility_make_float_box()->bool {
 for(var axis=0u;axis<3u;axis++) {
  let origin=nearest_node_origin[axis];let end=wi_add(origin,nearest_node_direction[axis]);
  if (origin.valid&end.valid)==0u {return false;}
  nearest_box[axis]=vec2<u32>(nearest_ordered(bitcast<u32>(min(origin.lo,end.lo))),nearest_ordered(bitcast<u32>(max(origin.hi,end.hi))));
 }
 return true;
}
