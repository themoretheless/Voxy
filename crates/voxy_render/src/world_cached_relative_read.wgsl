// Layout constants belong to the same retained Arc<RayScene> as all bindings.
struct CachedRelative {delta:array<WorldInterval,3>,valid:u32}
fn source_cached_relative(slot:u32,vertex:u32,ray:u32)->CachedRelative {
 var bad:CachedRelative;bad.valid=0u;
 let n=arrayLength(&exact_vertices);let h=CR_HEADER;
 // Subtraction checks precede every variable address addition/multiplication.
 if n!=CR_WORDS||h>n||n-h<17u||CR_SLOTS>n-h-17u||CR_RECORDS>n {return bad;}
 if exact_vertices[h]!=1u||exact_vertices[h+1u]!=CR_SLOTS||exact_vertices[h+2u]!=CR_VERTICES||exact_vertices[h+3u]!=CR_RECORDS||exact_vertices[h+4u]!=12u||exact_vertices[h+5u]!=0u||exact_vertices[h+6u]!=CR_VERTICES {return bad;}
 if exact_vertices[h+8u]!=CR_REVISION.x||exact_vertices[h+9u]!=CR_REVISION.y||slot>=CR_SLOTS||slot>=arrayLength(&exact_layout) {return bad;}
 if exact_vertices[h+16u]!=0u||exact_vertices[h+16u+CR_SLOTS]!=CR_VERTICES {return bad;}
 let first=exact_vertices[h+16u+slot];let end=exact_vertices[h+17u+slot];
 if first>end||end>CR_VERTICES||vertex<exact_layout[slot].y {return bad;}
 let local=vertex-exact_layout[slot].y;if local>=end-first {return bad;}
 let index=first+local;
 if (n-CR_RECORDS)/12u<=index {return bad;}
 let base=CR_RECORDS+index*12u;
 var result:CachedRelative;
 for(var axis=0u;axis<3u;axis++) {
  let at=base+axis*4u;
  let record=vec4<u32>(exact_vertices[at],exact_vertices[at+1u],exact_vertices[at+2u],exact_vertices[at+3u]);
  let value=relative_interval(record,vec2<u32>(exact_ray_word(ray+axis),exact_ray_word(ray+4u+axis)));
  if value.reason!=0u {return bad;}
  result.delta[axis]=value.bounds;
 }
 result.valid=1u;return result;
}
