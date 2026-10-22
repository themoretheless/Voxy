// EX_VERTEX_CACHE_HEADER is injected only for this retained immutable owner.
fn exact_axis(slot:u32,vertex:u32,axis:u32,origin_hi:u32,origin_lo:u32)->ExInt {
 let h=EX_VERTEX_CACHE_HEADER;
 if h+16u>=arrayLength(&exact_vertices)||exact_vertices[h]!=1u||slot>=exact_vertices[h+1u] {return exact_axis_original(slot,vertex,axis,origin_hi,origin_lo);}
 let local_base=exact_layout[slot].y;
 if vertex<local_base {return exact_axis_original(slot,vertex,axis,origin_hi,origin_lo);}
 let first=exact_vertices[h+16u+slot];let end=exact_vertices[h+17u+slot];let local=vertex-local_base;
 if local>=end-first {return exact_axis_original(slot,vertex,axis,origin_hi,origin_lo);}
 let base=exact_vertices[h+3u]+(first+local)*12u;
 if base+11u>=arrayLength(&exact_vertices) {return exact_axis_original(slot,vertex,axis,origin_hi,origin_lo);}
 // A non-admitted record falls back as a whole vertex, never partial geometry.
 for(var a=0u;a<3u;a++) {
  let descriptor=exact_vertices[base+a*4u];let live=(descriptor>>8u)&7u;let low=descriptor&255u;
  if (descriptor&32768u)==0u||(descriptor&0xffff3800u)!=0u||live>6u||low+live>EX_WORDS {return exact_axis_original(slot,vertex,axis,origin_hi,origin_lo);}
 }
 let at=base+axis*4u;let descriptor=exact_vertices[at];let live=(descriptor>>8u)&7u;let low=descriptor&255u;
 var value=ex_zero();if value.valid==0u {return value;}
 if live!=0u {
  for(var k=0u;k<low;k++){exact_scratch[value.offset+k]=0u;}
  for(var k=0u;k<live;k++){exact_scratch[value.offset+low+k]=(exact_vertices[at+1u+k/2u]>>((k%2u)*16u))&65535u;}
  value.length=low+live;value.negative=(descriptor>>14u)&1u;
 }
 value=ex_sub(value,ex_float(origin_hi,-298));return ex_sub(value,ex_float(origin_lo,-298));
}
