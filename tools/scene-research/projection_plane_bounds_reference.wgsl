fn research_plane_inside_interval(matrix:mat4x4<f32>,point:vec4<f32>,plane:u32)->bool {
 if plane==5u {return dda_far_plane_inside(matrix,point);}
 let axis=min(plane/2u,2u);var distance=DdaInterval(vec2<f32>(0.),true);
 for(var column=0u;column<4u;column++) {
  let row=matrix[column][axis];
  let bits=bitcast<vec3<u32>>(vec3<f32>(matrix[column].w,row,point[column]));
  if any(((bits&vec3<u32>(0x7f800000u))==vec3<u32>(0u))&((bits&vec3<u32>(0x007fffffu))!=vec3<u32>(0u))) {return false;}
  var coefficient=DdaInterval(vec2<f32>(row),true);
  if plane!=4u {coefficient=dda_interval_expand(vec2<f32>(projection_add_scalar(matrix[column].w,select(row,-row,(plane&1u)!=0u))),1u);}
  distance=dda_interval_add(distance,dda_interval_mul(coefficient,DdaInterval(vec2<f32>(point[column]),true)));
 }
 return distance.valid&&distance.bounds.x>0.;
}

// Research correction: a positive exact interval alone does not guarantee the
// old expanded calculation is finite. Keep ample headroom for every original
// product, coefficient sum, expansion carry and magnitude calculation.
fn research_plane_inside_bound(matrix:mat4x4<f32>,point:vec4<f32>,plane:u32)->bool {
 if !research_plane_inside_interval(matrix,point,plane) {return false;}
 let axis=min(plane/2u,2u);let limit=bitcast<f32>(0x7d7fffffu);var magnitude=0.;
 for(var column=0u;column<4u;column++) {
  if abs(matrix[column].w)>limit||abs(matrix[column][axis])>limit {return false;}
  let a=abs(projection_mul_scalar(matrix[column].w,point[column]));
  let b=abs(projection_mul_scalar(matrix[column][axis],point[column]));
  magnitude=projection_add_scalar(magnitude,projection_add_scalar(a,b));
 }
 return dda_interval_finite(vec2<f32>(magnitude))&&magnitude<=limit;
}
