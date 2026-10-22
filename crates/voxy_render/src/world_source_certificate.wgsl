override WORLD_SOURCE_MATERIAL_SIDES:bool=false;
// Use the prepared original material word, never shading normals or a rounded
// native front-face flag. Winding comes from exact transformed source vertices.
fn source_material_sides(material:u32)->u32 {
 let header=nearest_material_base();
 if nearest_active[header+1u]!=1u||material>=nearest_active[header] {return 0u;}
 let word=nearest_active[header+4u+material*24u+21u];
 if word>1u {return 0u;}return word+1u;
}

// One original-source material and triangle proof authority, shared by software and hardware candidate passes.
fn visibility_material(material:u32)->u32 {
 let header=nearest_material_base();if nearest_active[header+1u]==0u {return 1u;}
 if material>=nearest_active[header] {return 3u;}
 let base=header+4u+material*24u;
 let alpha=nearest_active[base+3u];let am=alpha&0x7fffffffu;
 let transmission=nearest_active[base+14u];let tm=transmission&0x7fffffffu;
 let cutoff=nearest_active[base+15u];let cm=cutoff&0x7fffffffu;
 if am>0x3f800000u||((alpha>>31u)!=0u&&am!=0u)||tm>=0x7f800000u||((transmission>>31u)!=0u&&tm!=0u)||cm>=0x7f800000u {return 3u;}
 let image=nearest_active[base+20u]!=0xffffffffu;
 let negative_cutoff=(cutoff>>31u)!=0u&&cm!=0u;
 if !negative_cutoff {if image {return 3u;}if am<cm {return 0u;}}
 if cutoff==0xc0000000u {
  if image {
   let workflow=nearest_active[base+17u];let wm=workflow&0x7fffffffu;
   if wm>=0x7f800000u||(workflow>>31u)!=0u||wm<=0x3fc00000u {return 3u;}
  }
  if am==0u&&tm==0u {return 0u;}
  if tm!=0u||am!=0x3f800000u {return 2u;}
 }else if tm!=0u {return 2u;}
 return 1u;
}
struct SourceTriangleCertificate {intersection:WorldTriangleCertificate,t:WorldInterval}
fn source_triangle_certificate(slot:u32,first:u32,ray:u32,order:bool)->SourceTriangleCertificate {
 let info=exact_layout[slot];
 let va=info.y+exact_indices[first];let vb=info.y+exact_indices[first+1u];let vc=info.y+exact_indices[first+2u];
 var e1:array<WorldInterval,3>;var e2:array<WorldInterval,3>;var delta:array<WorldInterval,3>;
 for(var axis=0u;axis<3u;axis++) {
  var x=wi_scalar(0.);var y=wi_scalar(0.);
  // Correlated common world translation cancels algebraically in the edges.
  // Subtract source translation from ray origin before adding local products.
  var z=wi_add(wi_sub(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_instances[slot*28u+12u+axis])),wi_source_word(exact_ray_word(ray+4u+axis)));
  for(var k=0u;k<3u;k++) {
   let matrix=wi_source_word(exact_instances[slot*28u+k*4u+axis]);
   let a=wi_source_word(exact_vertices[va*12u+k]);let b=wi_source_word(exact_vertices[vb*12u+k]);let c=wi_source_word(exact_vertices[vc*12u+k]);
   x=wi_add(x,wi_mul(matrix,wi_sub(b,a)));y=wi_add(y,wi_mul(matrix,wi_sub(c,a)));z=wi_sub(z,wi_mul(matrix,a));
  }
  e1[axis]=x;e2[axis]=y;delta[axis]=z;
 }
 let direction=WorldInterval3(nearest_node_direction[0],nearest_node_direction[1],nearest_node_direction[2]);
 let edge1=WorldInterval3(e1[0],e1[1],e1[2]);let edge2=WorldInterval3(e2[0],e2[1],e2[2]);let relative=WorldInterval3(delta[0],delta[1],delta[2]);
 let normal=wi3_cross(edge1,edge2);let start=wi3_dot(normal,relative);let finish=wi_add(start,wi3_dot(normal,direction));
 // Strict same-side separation also proves parallel misses. Zero-area and
 // endpoint-touch cases remain inconclusive and retain exact fallback.
 if (wi_positive(start)&&wi_positive(finish))||(wi_negative(start)&&wi_negative(finish)) {return SourceTriangleCertificate(WorldTriangleCertificate(0u,0u,0u),wi_bad());}
 let certificate=world_triangle_certificate(direction,edge1,edge2,relative,wi_scalar(0.),wi_scalar(1.));
 if WORLD_SOURCE_MATERIAL_SIDES&&certificate.state!=0u {
  let sides=source_material_sides(info.w);
  if sides==0u {return SourceTriangleCertificate(WorldTriangleCertificate(2u,16u,certificate.reverse),wi_bad());}
  if sides==1u&&(certificate.reason&1u)==0u&&certificate.reverse!=0u {
   // reverse is set only after a strictly negative determinant interval.
   // Boundary/invalid origin tests cannot invalidate that orientation proof.
   return SourceTriangleCertificate(WorldTriangleCertificate(0u,0u,certificate.reverse),wi_bad());
  }
 }

 var t=wi_bad();
 if order&&certificate.state==1u {
  var denominator=wi3_dot(edge1,wi3_cross(direction,edge2));
  var numerator=wi3_dot(edge2,wi3_cross(relative,edge1));
  if wi_negative(denominator) {denominator=wi_neg(denominator);numerator=wi_neg(numerator);}
  if wi_positive(denominator) {t=nearest_div_interval(numerator,denominator);}
 }
 return SourceTriangleCertificate(certificate,t);
}
fn visibility_certificate(slot:u32,first:u32,ray:u32)->WorldTriangleCertificate {
 return source_triangle_certificate(slot,first,ray,false).intersection;
}
