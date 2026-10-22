@group(0) @binding(11) var<storage,read_write> prepared_bounds:array<atomic<u32>>;
fn bounds_word(at:u32)->u32{return atomicLoad(&prepared_bounds[at]);}
fn bounds_slot(index:u32)->u32 {
 var lo=0u;var hi=bounds_word(1u)+1u;
 while lo<hi {let mid=lo+(hi-lo)/2u;if bounds_word(16u+mid)<=index {lo=mid+1u;}else{hi=mid;}}
 return lo-1u;
}
fn bounds_original(index:u32)->SourceGeometryBounds {
 let slot=bounds_slot(index);let primitive=index-bounds_word(16u+slot);
 return source_geometry_bounds(slot,exact_layout[slot].z+primitive*3u);
}
fn bounds_words(e:SourceGeometryBounds)->array<u32,19> {
 var words:array<u32,19>;let n=array<WorldInterval,3>(e.normal.x,e.normal.y,e.normal.z);
 for(var axis=0u;axis<3u;axis++) {
  let at=axis*4u;words[at]=bitcast<u32>(e.e1[axis].lo);words[at+1u]=bitcast<u32>(e.e1[axis].hi);
  words[at+2u]=bitcast<u32>(e.e2[axis].lo);words[at+3u]=bitcast<u32>(e.e2[axis].hi);
  words[12u+axis*2u]=bitcast<u32>(n[axis].lo);words[13u+axis*2u]=bitcast<u32>(n[axis].hi);
  words[18u]|=(e.e1[axis].valid&1u)<<axis;words[18u]|=(e.e2[axis].valid&1u)<<(axis+3u);words[18u]|=(n[axis].valid&1u)<<(axis+6u);
 }
 return words;
}
@compute @workgroup_size(64) fn cs_bounds_prepare(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=bounds_word(2u){return;}let words=bounds_words(bounds_original(id.x));let base=bounds_word(3u)+id.x*19u;
 for(var k=0u;k<19u;k++){atomicStore(&prepared_bounds[base+k],words[k]);}
}
@compute @workgroup_size(64) fn cs_bounds_check(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=bounds_word(2u){return;}let words=bounds_words(bounds_original(id.x));let base=bounds_word(3u)+id.x*19u;var different=false;
 for(var k=0u;k<19u;k++){different=different||bounds_word(base+k)!=words[k];}
 if different{atomicAdd(&prepared_bounds[5u],1u);}atomicAdd(&prepared_bounds[6u],1u);
}
@compute @workgroup_size(1) fn cs_bounds_ready() {
 let checked=bounds_word(7u)==0u||bounds_word(6u)==bounds_word(2u);
 atomicStore(&prepared_bounds[0u],select(0u,1u,checked&&bounds_word(5u)==0u));
}
