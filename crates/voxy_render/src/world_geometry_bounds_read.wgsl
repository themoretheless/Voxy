@group(0) @binding(11) var<storage,read> cached_bounds:array<u32>;
fn source_cached_geometry_bounds(slot:u32,first:u32)->SourceGeometryBounds {
 if cached_bounds[0u]!=1u||slot>=cached_bounds[1u]{return source_geometry_bounds(slot,first);}
 let info=exact_layout[slot];if first<info.z||(first-info.z)%3u!=0u{return source_geometry_bounds(slot,first);}
 let primitive=(first-info.z)/3u;let start=cached_bounds[16u+slot];let end=cached_bounds[17u+slot];
 if primitive>=end-start{return source_geometry_bounds(slot,first);}
 let base=cached_bounds[3u]+(start+primitive)*19u;let mask=cached_bounds[base+18u];
 var e1:array<WorldInterval,3>;var e2:array<WorldInterval,3>;var n:array<WorldInterval,3>;
 for(var axis=0u;axis<3u;axis++){
  let at=base+axis*4u;
  e1[axis]=WorldInterval(bitcast<f32>(cached_bounds[at]),bitcast<f32>(cached_bounds[at+1u]),(mask>>axis)&1u);
  e2[axis]=WorldInterval(bitcast<f32>(cached_bounds[at+2u]),bitcast<f32>(cached_bounds[at+3u]),(mask>>(axis+3u))&1u);
  n[axis]=WorldInterval(bitcast<f32>(cached_bounds[base+12u+axis*2u]),bitcast<f32>(cached_bounds[base+13u+axis*2u]),(mask>>(axis+6u))&1u);
 }
 return SourceGeometryBounds(e1,e2,WorldInterval3(n[0],n[1],n[2]));
}
