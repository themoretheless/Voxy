// Geometry-only original interval operations. Translation and the source ray
// are deliberately excluded; their correlated subtraction stays per query.
struct SourceGeometryBounds {e1:array<WorldInterval,3>,e2:array<WorldInterval,3>,normal:WorldInterval3}
fn source_geometry_bounds(slot:u32,first:u32)->SourceGeometryBounds {
 let info=exact_layout[slot];
 let va=info.y+exact_indices[first];let vb=info.y+exact_indices[first+1u];let vc=info.y+exact_indices[first+2u];
 var e1:array<WorldInterval,3>;var e2:array<WorldInterval,3>;
 for(var axis=0u;axis<3u;axis++) {
  var x=wi_scalar(0.);var y=wi_scalar(0.);
  for(var k=0u;k<3u;k++) {
   let matrix=wi_source_word(exact_instances[slot*28u+k*4u+axis]);
   let a=wi_source_word(exact_vertices[va*12u+k]);let b=wi_source_word(exact_vertices[vb*12u+k]);let c=wi_source_word(exact_vertices[vc*12u+k]);
   x=wi_add(x,wi_mul(matrix,wi_sub(b,a)));y=wi_add(y,wi_mul(matrix,wi_sub(c,a)));
  }
  e1[axis]=x;e2[axis]=y;
 }
 let normal=wi3_cross(WorldInterval3(e1[0],e1[1],e1[2]),WorldInterval3(e2[0],e2[1],e2[2]));
 return SourceGeometryBounds(e1,e2,normal);
}
