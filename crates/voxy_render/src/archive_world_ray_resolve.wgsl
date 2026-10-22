// Original-source admission feeds the actual opaque GI/reflection output.
// Unknown material layers and rational tails retain the complete depth query.
@group(1) @binding(18) var<storage,read> resolve_ray_words:array<u32>;
@group(1) @binding(19) var<storage,read> resolve_hit_words:array<u32>;
@group(1) @binding(21) var<storage,read> resolve_source_table:array<u32>;
fn world_resolve_two_sided(material_id:u32)->bool {
 let base=resolve_source_table[2u]+resolve_source_table[1u]*8u;
 if base+3u>=arrayLength(&resolve_source_table)||resolve_source_table[base+1u]!=1u||resolve_source_table[base+2u]!=24u||material_id>=resolve_source_table[base] {return false;}
 let word=base+4u+material_id*24u+21u;
 return word<arrayLength(&resolve_source_table)&&resolve_source_table[word]==1u;
}
override ARCHIVE_WORLD_HIT_WORDS:u32=20u;
override ARCHIVE_WORLD_POSITION_BASE:u32=0u;
override ARCHIVE_WORLD_MATERIAL_SIDES:bool=false;
fn world_resolve_exact_position(index:u32)->bool {
 if ARCHIVE_WORLD_POSITION_BASE==0u {return false;}
 let at=ARCHIVE_WORLD_POSITION_BASE+index*8u;
 return at+7u<arrayLength(&resolve_hit_words)&&resolve_hit_words[at+6u]==7u&&resolve_hit_words[at+7u]==0u;
}
override ARCHIVE_WORLD_RESOLVE_AUDIT:bool=false;
@group(1) @binding(20) var<storage,read_write> world_resolve_audit:array<atomic<u32>,12>;
fn world_resolve_count(index:u32) {if ARCHIVE_WORLD_RESOLVE_AUDIT {atomicAdd(&world_resolve_audit[index],1u);}}
fn archive_world_query_fallback(ray:ArchiveRayInput)->vec4<f32> {
 return screen_reflection(ray.origin,ray.direction,ray.history,60.,1.,8u,ARCHIVE_RAY_BOUNDS);
}
@compute @workgroup_size(8,8)
fn cs_world_ray_resolve(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let pixel=ray_tile.offset+id.xy;let index=id.y*ray_tile.extent.x+id.x;
 let ray=archive_ray_input(pixel);var colour=vec4<f32>(0.);
 if ray.admitted {
  let source=index*20u;let hit=index*ARCHIVE_WORLD_HIT_WORDS;
  if source+3u<arrayLength(&resolve_ray_words)&&world_consumer_pruned(index,resolve_ray_words[source+3u],ray) {
   world_resolve_count(0u);textureStore(gi_output,vec2<i32>(pixel),colour);return;
  }
  var fallback=true;var reason=7u;
  if source+19u<arrayLength(&resolve_ray_words)&&hit+19u<arrayLength(&resolve_hit_words) {
   let matching=resolve_hit_words[hit+4u]==resolve_ray_words[source+18u]&&resolve_hit_words[hit+5u]==resolve_ray_words[source+19u];
   let state=resolve_hit_words[hit];
   if matching&&state==0u {fallback=false;world_resolve_count(1u);}
   if !matching {reason=6u;}else if state==2u {reason=5u;}else if state==1u {
    if (resolve_hit_words[hit+15u]&1u)!=0u {reason=3u;}
    if ((resolve_hit_words[hit+18u]>>2u)&3u)>=2u {reason=4u;}
   }
   // A point formed from the unrounded rational may be exact even when t has
   // a remaining tail. Otherwise retain the complete original query. Optical
   // layers still require their original layered handling.
   let kind=(resolve_hit_words[hit+18u]>>2u)&3u;
   let exact_position=world_resolve_exact_position(index);
   let parameter_usable=(resolve_hit_words[hit+15u]&1u)==0u;
   let position_usable=select(parameter_usable,exact_position,ARCHIVE_WORLD_POSITION_BASE!=0u);
   let side_usable=ARCHIVE_WORLD_MATERIAL_SIDES||world_resolve_two_sided(resolve_hit_words[hit+17u]);
   if ARCHIVE_WORLD_POSITION_BASE!=0u&&state==1u&&!exact_position {reason=3u;}
   if matching&&state==1u&&resolve_hit_words[hit+14u]==1u&&position_usable&&kind<2u&&side_usable {
    var t=bitcast<vec2<f32>>(vec2<u32>(resolve_hit_words[hit+8u],resolve_hit_words[hit+9u]));
    // The high parameter word is RN-even of the full rational. Adding its
    // rounded low word could double-round a non-dyadic t at a midpoint.
    if ARCHIVE_WORLD_POSITION_BASE!=0u {t.y=0.;}
    var point=DdaVectorPair(vec4<f32>(0.,0.,0.,1.),vec4<f32>(0.));
    for(var axis=0u;axis<3u;axis++) {
     if ARCHIVE_WORLD_POSITION_BASE!=0u {
      let at=ARCHIVE_WORLD_POSITION_BASE+index*8u+axis*2u;
      point.high[axis]=bitcast<f32>(resolve_hit_words[at]);point.low[axis]=bitcast<f32>(resolve_hit_words[at+1u]);
     }else {
      let origin=bitcast<vec2<f32>>(vec2<u32>(resolve_ray_words[source+axis],resolve_ray_words[source+4u+axis]));
      let direction=bitcast<vec2<f32>>(vec2<u32>(resolve_ray_words[source+8u+axis],resolve_ray_words[source+12u+axis]));
      let value=projection_pair_add(origin,projection_pair_mul(direction,t));
      point.high[axis]=value.x;point.low[axis]=value.y;
     }
    }
    let clip=dda_project_world_pair(camera.vp,point);
    let clip_w=vec2<f32>(clip.high.w,clip.low.w);let clip_z=vec2<f32>(clip.high.z,clip.low.z);
    if dda_finite4(point.high)&&dda_finite4(point.low)&&dda_finite4(clip.high)&&dda_finite4(clip.low)&&!precise_le(clip_w,vec2<f32>(0.))&&!projection_pair_less(clip_z,vec2<f32>(0.))&&!projection_pair_less(clip_w,clip_z) {
     let uv=dda_clip_ndc(clip).xy*vec2<f32>(0.5,-0.5)+0.5;
     if all(uv>=vec2<f32>(0.))&&all(uv<vec2<f32>(1.)) {
      let sample_pixel=vec2<i32>(uv*camera.viewport.xy);
      colour=archive_world_history_radiance(point,t,sample_pixel,ray.history);fallback=false;world_resolve_count(2u);
     }
    }
   }
  }
  if fallback {colour=world_consumer_cached(index,ray);world_resolve_count(reason);}
 }else {world_resolve_count(0u);}
 textureStore(gi_output,vec2<i32>(pixel),colour);
}

// Diagnostic only: the complete fallback result is evaluated independently.
// No ray record, hit record or output texture is changed by this entry point.
@compute @workgroup_size(8,8)
fn cs_world_consumer_zero_audit(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent) {return;}
 let ray=archive_ray_input(ray_tile.offset+id.xy);if !ray.admitted {return;}
 let colour=archive_world_query_fallback(ray);let bits=bitcast<vec4<u32>>(colour);
 atomicAdd(&world_resolve_audit[8],1u);
 if all(bits==vec4<u32>(0u)) {atomicAdd(&world_resolve_audit[9],1u);}
 else if all((bits&vec4<u32>(0x7fffffffu))<vec4<u32>(0x7f800000u)) {atomicAdd(&world_resolve_audit[10],1u);}
 else {atomicAdd(&world_resolve_audit[11],1u);}
}
