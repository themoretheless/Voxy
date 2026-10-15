// A screen-space history lookup is valid only for the same geometric surface.
// Compare reconstructed surfaces, not approximate ray points. Allow two
// projected pixels and a small world-space floor for float32 depth reprojection.
fn archive_history_surface_matches(point:vec3<f32>,recovered:vec3<f32>,clip_w:f32,pixel_angular_size:f32)->bool {
 let exponent=vec3<u32>(0x7f800000u);
 if any((bitcast<vec3<u32>>(point)&exponent)==exponent)||any((bitcast<vec3<u32>>(recovered)&exponent)==exponent)
  ||(bitcast<u32>(clip_w)&0x7f800000u)==0x7f800000u
  ||(bitcast<u32>(pixel_angular_size)&0x7f800000u)==0x7f800000u||clip_w<=0. {return false;}
 let tolerance=0.03+2.*max(pixel_angular_size,0.)*clip_w;
 let delta=recovered-point;
 return dot(delta,delta)<=tolerance*tolerance;
}
