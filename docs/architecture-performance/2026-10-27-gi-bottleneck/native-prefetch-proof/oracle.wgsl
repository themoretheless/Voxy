
const ARCHIVE_REFINE_CACHE:bool=false;
struct Camera {vp:mat4x4<f32>,inverse_vp:mat4x4<f32>,previous_vp:mat4x4<f32>,previous_inverse_vp:mat4x4<f32>,viewport:vec4<f32>,surface:vec4<f32>}
var<private> camera:Camera;
struct Input {origin:vec4<f32>,direction:vec4<f32>}
struct Result {a:vec4<f32>,b:vec4<f32>}
@group(0) @binding(0) var scene_depth:texture_2d<f32>;
@group(0) @binding(1) var ray_depth_bounds:texture_2d<f32>;
@group(0) @binding(2) var background:texture_2d<f32>;
@group(0) @binding(3) var<storage,read> inputs:array<Input>;
@group(0) @binding(4) var<storage,read_write> results:array<Result>;
@group(0) @binding(5) var<uniform> matrices:array<mat4x4<f32>,2>;

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

fn screen_reflection_original(origin:vec3<f32>,direction:vec3<f32>,history:bool,reach:f32,growth:f32,refines:u32,bounded:bool)->vec4<f32>{
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
    if any(ray_pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,ray_pixel,0).x;cached_pixel=ray_pixel;}
    depth=cached_depth;
   }
  } else {depth=textureLoad(scene_depth,ray_pixel,0).x;}
  if depth<1.&&q.z/q.w>=depth {
   var lo=previous;var hi=t;
   for(var refine=0u;refine<refines;refine++) {
    let mid=(lo+hi)*0.5;let clip=camera.vp*vec4<f32>(origin+direction*mid,1.);
    let pixel=clamp(vec2<i32>((clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
    var z=1.;
    if bounded && ARCHIVE_REFINE_CACHE {
     if any(pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,pixel,0).x;cached_pixel=pixel;}
     z=cached_depth;
    } else {z=textureLoad(scene_depth,pixel,0).x;}
    if z<1.&&clip.z/clip.w>=z {hi=mid;} else {lo=mid;}
   }
   let point=origin+direction*hi;let clip=camera.vp*vec4<f32>(point,1.);
   let coord=clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5;
   let pixel=clamp(vec2<i32>(coord*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
   var z=1.;
   if bounded && ARCHIVE_REFINE_CACHE {
    if any(pixel!=cached_pixel) {cached_depth=textureLoad(scene_depth,pixel,0).x;cached_pixel=pixel;}
    z=cached_depth;
   } else {z=textureLoad(scene_depth,pixel,0).x;}
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
     let stored_depth=textureLoad(scene_depth,source,0).x;
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
// Four independent original samples in flight; admission stays sequential.
struct ArchivePrefetch {q:vec4<f32>,uv:vec2<f32>,depth:f32}
struct ArchiveProbeResult {done:bool,colour:vec4<f32>}
fn archive_prefetch(origin:vec3<f32>,direction:vec3<f32>,t:f32)->ArchivePrefetch {
 let q=camera.vp*vec4<f32>(origin+direction*t,1.);
 let uv=q.xy/q.w*vec2<f32>(0.5,-0.5)+0.5;
 // Speculative reads are clamped. Original viewport/reach tests precede use.
 let pixel=clamp(vec2<i32>(uv*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
 return ArchivePrefetch(q,uv,textureLoad(scene_depth,pixel,0).x);
}
fn archive_admit_prefetch(origin:vec3<f32>,direction:vec3<f32>,history:bool,t:f32,previous:f32,probe:ArchivePrefetch)->ArchiveProbeResult {
 let reach=60.;let refines=8u;let q=probe.q;let uv=probe.uv;let depth=probe.depth;
 if t>reach||q.w<=0.||any(uv<=vec2<f32>(0.))||any(uv>=vec2<f32>(1.)){return ArchiveProbeResult(true,vec4<f32>(0.));}
  if depth<1.&&q.z/q.w>=depth {
   var lo=previous;var hi=t;
   for(var refine=0u;refine<refines;refine++) {
    let mid=(lo+hi)*0.5;let clip=camera.vp*vec4<f32>(origin+direction*mid,1.);
    let pixel=clamp(vec2<i32>((clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5)*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
    let z=textureLoad(scene_depth,pixel,0).x;
    if z<1.&&clip.z/clip.w>=z {hi=mid;} else {lo=mid;}
   }
   let point=origin+direction*hi;let clip=camera.vp*vec4<f32>(point,1.);
   let coord=clip.xy/clip.w*vec2<f32>(0.5,-0.5)+0.5;
   let pixel=clamp(vec2<i32>(coord*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
   let z=textureLoad(scene_depth,pixel,0).x;
   let h=camera.inverse_vp*vec4<f32>((vec2<f32>(pixel)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),z,1.);
   if distance(h.xyz/h.w,point)<0.20 {
    var edge=min(min(coord.x,coord.y),min(1.-coord.x,1.-coord.y));
    var source=pixel;
    if history {
     let hit_world=h.xyz/h.w;
     let old=camera.previous_vp*vec4<f32>(hit_world,1.);
     if old.w<=0. {return ArchiveProbeResult(true,vec4<f32>(0.));}
     let old_uv=old.xy/old.w*vec2<f32>(0.5,-0.5)+0.5;
     edge=min(edge,min(min(old_uv.x,old_uv.y),min(1.-old_uv.x,1.-old_uv.y)));
     if edge<=0. {return ArchiveProbeResult(true,vec4<f32>(0.));}
     source=clamp(vec2<i32>(old_uv*camera.viewport.xy),vec2<i32>(0),vec2<i32>(camera.viewport.xy)-1);
     let stored_depth=textureLoad(scene_depth,source,0).x;
     if stored_depth>=1. {return ArchiveProbeResult(true,vec4<f32>(0.));}
     let old_ndc=(vec2<f32>(source)+0.5)/camera.viewport.xy*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.);
     let recovered=camera.previous_inverse_vp*vec4<f32>(old_ndc,stored_depth,1.);
     if recovered.w==0. || !archive_history_surface_matches(hit_world,recovered.xyz/recovered.w,old.w,camera.surface.w) {return ArchiveProbeResult(true,vec4<f32>(0.));}

    }
    let fade=smoothstep(0.,0.04,edge)*(1.-smoothstep(0.75,1.,hi/reach));
    return ArchiveProbeResult(true,vec4<f32>(textureLoad(background,source,0).rgb/camera.viewport.z,fade));
   }
   return ArchiveProbeResult(true,vec4<f32>(0.));
  }
 return ArchiveProbeResult(false,vec4<f32>(0.));
}
fn archive_gi_prefetched(origin:vec3<f32>,direction:vec3<f32>)->vec4<f32> {
 if camera.surface.z<0.5 {return vec4<f32>(0.);}
 var t=0.08;var previous=0.;
 for(var group=0u;group<20u;group++) {
  let t0=t;let t1=t0+1.*(0.04+0.06*t0);let t2=t1+1.*(0.04+0.06*t1);let t3=t2+1.*(0.04+0.06*t2);
  let p0=archive_prefetch(origin,direction,t0);
  let p1=archive_prefetch(origin,direction,t1);
  let p2=archive_prefetch(origin,direction,t2);
  let p3=archive_prefetch(origin,direction,t3);
  let a=archive_admit_prefetch(origin,direction,true,t0,previous,p0);if a.done{return a.colour;}
  let b=archive_admit_prefetch(origin,direction,true,t1,t0,p1);if b.done{return b.colour;}
  let c=archive_admit_prefetch(origin,direction,true,t2,t1,p2);if c.done{return c.colour;}
  let d=archive_admit_prefetch(origin,direction,true,t3,t2,p3);if d.done{return d.colour;}
  previous=t3;t=t3+1.*(0.04+0.06*t3);
  if t>60. {break;}
 }
 return vec4<f32>(0.);
}


@compute @workgroup_size(64) fn check(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs){return;}
 camera=Camera(matrices[0],matrices[1],matrices[0],matrices[1],vec4<f32>(259.,131.,1.,1.),vec4<f32>(0.,0.,1.,1.5/131.));
 let p=inputs[id.x].origin.xyz;let d=inputs[id.x].direction.xyz;
 results[id.x]=Result(screen_reflection_original(p,d,true,60.,1.,8u,true),archive_gi_prefetched(p,d));
}
