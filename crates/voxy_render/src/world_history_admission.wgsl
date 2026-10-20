// Render-consumer admission ONLY, after complete source coverage has proved a
// strictly separated nearest hit. Never authorizes a hit/miss or weakens the
// general exact-nearest contract. Returning a reason selects the SAME complete
// legacy query that archive_world_ray_resolve uses for these source results.
fn history_mod_point(slot:u32,vertex:u32,p:u32)->vec3<u32> {
 var result=vec3<u32>(0u);
 for(var axis=0u;axis<3u;axis++) {
  var value=history_mod_word(exact_instances[slot*28u+12u+axis],p);
  for(var k=0u;k<3u;k++) {value+=history_mod_word(exact_instances[slot*28u+k*4u+axis],p)*history_mod_word(exact_vertices[vertex*12u+k],p);}
  result[axis]=value%p;
 }
 return result;
}
fn history_tail_proved(slot:u32,primitive:u32,ray:u32)->bool {
 let info=exact_layout[slot];let first=info.z+primitive*3u;
 // Keep unsupported values inconclusive. These guards also document the finite
 // field's domain independently of the preceding floating interval certificate.
 for(var axis=0u;axis<3u;axis++) {
  for(var part=0u;part<4u;part++) {if ((exact_ray_word(ray+part*4u+axis)>>23u)&255u)==255u {return false;}}
  for(var k=0u;k<4u;k++) {if ((exact_instances[slot*28u+k*4u+axis]>>23u)&255u)==255u {return false;}}
  for(var corner=0u;corner<3u;corner++) {if ((exact_vertices[(info.y+exact_indices[first+corner])*12u+axis]>>23u)&255u)==255u {return false;}}
 }
 for(var which=0u;which<2u;which++) {
  let p=select(3u,5u,which==1u);
  let a=history_mod_point(slot,info.y+exact_indices[first],p);
  let b=history_mod_point(slot,info.y+exact_indices[first+1u],p);
  let c=history_mod_point(slot,info.y+exact_indices[first+2u],p);
  var origin=vec3<u32>(0u);var direction=vec3<u32>(0u);
  for(var axis=0u;axis<3u;axis++) {
   origin[axis]=(history_mod_word(exact_ray_word(ray+axis),p)+history_mod_word(exact_ray_word(ray+4u+axis),p))%p;
   direction[axis]=(history_mod_word(exact_ray_word(ray+8u+axis),p)+history_mod_word(exact_ray_word(ray+12u+axis),p))%p;
  }
  let e1=history_mod_sub(b,a,p);let e2=history_mod_sub(c,a,p);
  let denominator=history_mod_dot(e1,history_mod_cross(direction,e2,p),p);
  let numerator=history_mod_dot(e2,history_mod_cross(history_mod_sub(origin,a,p),e1,p),p);
  // A nonzero numerator and zero denominator in Fp prove that the reduced
  // rational t contains this odd prime. No pair of finite f32 values is exact.
  if denominator==0u&&numerator!=0u {return true;}
 }
 return false;
}
fn history_admission_reason(slot:u32,primitive:u32,ray:u32)->u32 {
 let material_id=exact_layout[slot].w;let header=nearest_material_base();
 if nearest_active[header+1u]!=1u||nearest_active[header+2u]!=24u||material_id>=nearest_active[header] {return 0u;}
 let base=header+4u+material_id*24u;
 if nearest_active[base+15u]==0xc0000000u||(nearest_active[base+14u]&0x7fffffffu)!=0u {return 2048u;}
 if nearest_active[base+21u]==0u {return 1024u;}
 if history_tail_proved(slot,primitive,ray) {return 4096u;}
 return 0u;
}

// The traversal's private stack/registers stay out of this integer-only kernel.
// Recompaction retains states 4/5/6 and changes only reserved input padding.
@compute @workgroup_size(64)
fn main_history_admission(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;let ray=index*20u;if ray+19u>=exact_ray_word_count() {return;}
 let out=index*5u;var state=exact_output[out].x;
 if state==5u {
  let source=exact_output[out+1u];let reason=history_admission_reason(source.z,source.w,ray);
  if reason!=0u {nearest_admission_result(index,5u,2u,reason);state=2u;}
 }
 if state==4u||state==5u||state==6u {if !deferred_append(index) {nearest_admission_result(index,5u,2u,64u);}}
}
