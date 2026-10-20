// Frozen previous kernel for GPU differential performance qualification only.
fn ray_before_refine_cache(origin:vec3<f32>,direction:vec3<f32>,history:bool,reach:f32,growth:f32,refines:u32,bounded:bool)->vec4<f32>{
 if history&&camera.surface.z<0.5 {return vec4<f32>(0.);}
 var t=0.08;var previous=0.;
 var cached_tile=vec2<i32>(-1);var cached_lower=0.;
 var cached_pixel=vec2<i32>(-1);var cached_depth=1.;
 for(var step=0u;step<80u;step++) {
  let q=camera.vp*vec4<f32>(origin+direction*t,1.);
  if q.w<=0. {return vec4<f32>(0.);}
  let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
  if any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){return vec4<f32>(0.);}
  let ray_pixel=vec2<i32>(uv*camera.viewport.xy);
  // The minimum is a rejection bound only. Potential hits retain the original
  // level-zero read, step recurrence, crossing and all binary refinements.
  var depth=1.;
  if bounded {
   let tile=ray_pixel/8;
   if any(tile!=cached_tile) {cached_lower=textureLoad(ray_depth_bounds,tile,0).x;cached_tile=tile;}
   if q.z/q.w>=cached_lower {
    if any(ray_pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,ray_pixel,0);cached_pixel=ray_pixel;}
    depth=cached_depth;
   }
  } else {depth=textureLoad(scene_depth,ray_pixel,0);}
  if depth<1.&&q.z/q.w>=depth {
   var lo=previous;var hi=t;
   for(var refine=0u;refine<refines;refine++) {
    let mid=(lo+hi)*0.5;let clip=camera.vp*vec4<f32>(origin+direction*mid,1.);
    let pixel=clamp(vec2<i32>((clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
    let z=textureLoad(scene_depth,pixel,0);
    if z<1.&&clip.z/clip.w>=z {hi=mid;} else {lo=mid;}
   }
   let point=origin+direction*hi;let clip=camera.vp*vec4<f32>(point,1.);
   let coord=clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5;
   let pixel=clamp(vec2<i32>(coord*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
   let z=textureLoad(scene_depth,pixel,0);
   let h=camera.inverse_vp*vec4<f32>((vec2<f32>(pixel)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),z,1.);
   if distance(h.xyz/h.w,point)<0.20 {
    var edge=min(min(coord.x,coord.y),min(1.-coord.x,1.-coord.y));
    var source=pixel;
    if history {
     let hit_world=h.xyz/h.w;
     let old=camera.previous_vp*vec4<f32>(hit_world,1.);
     if old.w<=0. {return vec4<f32>(0.);}
     let old_uv=old.xy/old.w*vec2<f32>(0.5,-0.5)+0.5;
     edge=min(edge,min(min(old_uv.x,old_uv.y),min(1.-old_uv.x,1.-old_uv.y)));
     if edge<=0. {return vec4<f32>(0.);}
     source=clamp(vec2<i32>(old_uv*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
     let stored_depth=textureLoad(opaque_history_depth,source,0);
     if stored_depth>=1. {return vec4<f32>(0.);}
     let old_ndc=(vec2<f32>(source)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
     let recovered=camera.previous_inverse_vp*vec4<f32>(old_ndc,stored_depth,1.);
     if recovered.w==0. || !archive_history_surface_matches(hit_world,recovered.xyz/recovered.w,old.w,camera.surface.w) {return vec4<f32>(0.);}

    }
    let fade=smoothstep(0.,0.04,edge)*(1.-smoothstep(0.75,1.,hi/reach));
    return vec4<f32>(textureLoad(background,source,0).rgb/camera.viewport.z,fade);
   }
   return vec4<f32>(0.);
  }
  previous=t;t+=growth*(0.04+0.06*t);
  if t>reach {break;}
 }
 return vec4<f32>(0.);
}
