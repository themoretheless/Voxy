// Chunked exact ray scheduling. No fields are quantized at stage boundaries.
override ARCHIVE_RAY_COHERENT:bool=false;
override ARCHIVE_RAY_QUEUE:bool=false;
override ARCHIVE_RAY_SPATIAL_KEY:bool=false;
override ARCHIVE_RAY_QUEUE_PROOF:bool=false;
struct ArchiveRayBins {header:array<atomic<u32>,8>,counts:array<atomic<u32>,256>}
@group(1) @binding(13) var<storage,read_write> staged_order:array<u32>;
@group(1) @binding(14) var<storage,read_write> staged_bins:ArchiveRayBins;
@group(1) @binding(15) var<storage,read_write> staged_seen:array<atomic<u32>>;
@group(1) @binding(16) var<storage,read_write> staged_errors:array<atomic<u32>>;
var<workgroup> ray_histogram:array<atomic<u32>,256>;
var<workgroup> ray_prefix:array<u32,256>;
var<workgroup> scatter_bins:array<u32,64>;
var<workgroup> scatter_bases:array<u32,64>;
fn archive_ray_bin(segment:DdaProjectedSegment)->u32 {
 if segment.state!=1u {return select(0u,1u,segment.state==2u);}
 let delta=segment.end.xy-segment.begin.xy;
 let length=max(abs(delta.x),abs(delta.y));
 let exponent=i32((bitcast<u32>(length)>>23u)&255u)-127;
 let bucket=u32(clamp((exponent+1)/2,0,7));
 let negative_z=projection_pair_less(vec2<f32>(segment.end.z,segment.end_low.z),vec2<f32>(segment.begin.z,segment.begin_low.z));
 let direction=select(0u,1u,delta.x<0.)|select(0u,2u,delta.y<0.)|select(0u,4u,abs(delta.y)>abs(delta.x))|select(0u,8u,negative_z);
 if ARCHIVE_RAY_SPATIAL_KEY {
  // Coarse origin + direction key. Only scheduling uses rounded coordinates;
  // the complete high/low segment remains unchanged for intersection tests.
  let origin=clamp(segment.begin.xy/camera.viewport.xy,vec2<f32>(0.),vec2<f32>(1.));
  let quadrant=select(0u,1u,origin.x>=0.5)|select(0u,2u,origin.y>=0.5);
  let long_ray=select(0u,1u,length>=256.);
  return 2u+quadrant*32u+long_ray*16u+direction;
 }
 return 2u+bucket*16u+direction;
}
var<workgroup> ray_order:array<u32,64>;
fn archive_trace_staged(begin:DdaVectorPair,end:DdaVectorPair)->DepthDdaPairHit {
 if ARCHIVE_STACK_TRAVERSAL {return trace_depth_hierarchy_candidate(scene_depth,begin,end);}
 return trace_depth_dda_paired(scene_depth,begin,end);
}

// Approximate keys change lane assignment only. Full paired segments and hit
// destinations retain their original IDs, including inactive/fallback rays.
fn archive_ray_schedule_key(segment:DdaProjectedSegment)->u32 {
 if segment.state!=1u {return segment.state;}
 let delta=segment.end.xy-segment.begin.xy;
 let length=max(abs(delta.x),abs(delta.y));
 let exponent=(bitcast<u32>(length)>>23u)&255u;
 let direction=select(0u,1u,delta.x<0.)|select(0u,2u,delta.y<0.)|select(0u,4u,abs(delta.y)>abs(delta.x));
 return 4u+exponent*8u+direction;
}
struct ArchiveRayTile {offset:vec2<u32>,extent:vec2<u32>}
@group(1) @binding(10) var<storage,read_write> staged_segments:array<DdaProjectedSegment>;
@group(1) @binding(11) var<storage,read_write> staged_hits:array<DepthDdaPairHit>;
@group(1) @binding(12) var<uniform> ray_tile:ArchiveRayTile;
struct ArchiveRayInput {origin:vec3<f32>,direction:vec3<f32>,history:bool,admitted:bool}
fn archive_ray_input(pixel:vec2<u32>)->ArchiveRayInput {
 let world=textureLoad(gi_world,vec2<i32>(pixel),0);let normal=textureLoad(gi_normal,vec2<i32>(pixel),0);
 if normal.w==0. {return ArchiveRayInput(vec3<f32>(0.),vec3<f32>(0.),true,false);}
 if ARCHIVE_TRACE_SSR {
  let history=normal.w>0.;let admitted=world.w>0.&&!fx_off(2u)&&!(history&&camera.surface.z<0.5);
  let view=normalize(camera.eye.xyz-world.xyz);
  return ArchiveRayInput(world.xyz+normal.xyz*0.03,reflect(-view,normal.xyz),history,admitted);
 }
 return ArchiveRayInput(world.xyz+normal.xyz*0.05,archive_gi_direction(vec2<f32>(pixel)+0.5,normal.xyz),true,!fx_off(1u)&&!(camera.surface.z<0.5));
}
@compute @workgroup_size(8,8) fn cs_ray_project(@builtin(global_invocation_id) id:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
 if ARCHIVE_RAY_QUEUE {
  for(var bin=lane;bin<256u;bin+=64u) {atomicStore(&ray_histogram[bin],0u);}
  workgroupBarrier();
 }
 if all(id.xy<ray_tile.extent) {
  let index=id.y*ray_tile.extent.x+id.x;let ray_input=archive_ray_input(ray_tile.offset+id.xy);
  var segment=dda_projection_empty(0u);
  if ray_input.admitted {segment=dda_project_segment(camera.vp,ray_input.origin,ray_input.origin+ray_input.direction*60.,camera.viewport.xy);}
  if ARCHIVE_RAY_FORCE_FALLBACK&&ray_input.admitted {segment=dda_projection_empty(2u);}
  segment.padding0=select(0u,1u,ray_input.history);staged_segments[index]=segment;
  if ARCHIVE_RAY_QUEUE {atomicAdd(&ray_histogram[archive_ray_bin(segment)],1u);}
  if ARCHIVE_RAY_QUEUE_PROOF {atomicStore(&staged_seen[index],0u);}
 }
 if ARCHIVE_RAY_QUEUE {
  workgroupBarrier();
  for(var bin=lane;bin<256u;bin+=64u) {
   let count=atomicLoad(&ray_histogram[bin]);if count!=0u {atomicAdd(&staged_bins.counts[bin],count);}
  }
 }
}
@compute @workgroup_size(256) fn cs_ray_queue_prefix(@builtin(local_invocation_index) lane:u32) {
 ray_prefix[lane]=atomicLoad(&staged_bins.counts[lane]);workgroupBarrier();
 for(var stride=1u;stride<256u;stride*=2u) {
  var add=0u;if lane>=stride {add=ray_prefix[lane-stride];}
  let own=ray_prefix[lane];workgroupBarrier();ray_prefix[lane]=own+add;workgroupBarrier();
 }
 var offset=0u;if lane>0u {offset=ray_prefix[lane-1u];}
 atomicStore(&staged_bins.counts[lane],offset);
 if lane==0u {
  atomicStore(&staged_bins.header[0],ray_prefix[255]);
  if ARCHIVE_RAY_QUEUE_PROOF&&!ARCHIVE_TRACE_SSR&&all(ray_tile.offset==vec2<u32>(0u)) {atomicAdd(&staged_errors[7],1u);}
 }
}
@compute @workgroup_size(64) fn cs_ray_queue_scatter(@builtin(global_invocation_id) id:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
 let count=ray_tile.extent.x*ray_tile.extent.y;let valid=id.x<count;
 var bin=0xffffffffu;if valid {bin=archive_ray_bin(staged_segments[id.x]);}
 scatter_bins[lane]=bin;workgroupBarrier();
 var first=lane;var rank=0u;var total=0u;
 for(var j=0u;j<64u;j++) {if scatter_bins[j]==bin {total+=1u;first=min(first,j);if j<lane {rank+=1u;}}}
 if valid&&first==lane {scatter_bases[lane]=atomicAdd(&staged_bins.counts[bin],total);}
 workgroupBarrier();
 if valid {staged_order[scatter_bases[first]+rank]=id.x;}
}
@compute @workgroup_size(64) fn cs_ray_traverse_queue(@builtin(global_invocation_id) id:vec3<u32>) {
 let count=atomicLoad(&staged_bins.header[0]);if id.x>=count||id.x>=ray_tile.extent.x*ray_tile.extent.y {return;}
 let index=staged_order[id.x];
 if ARCHIVE_RAY_QUEUE_PROOF {
  if index>=ray_tile.extent.x*ray_tile.extent.y {atomicAdd(&staged_errors[0],1u);return;}
  if atomicAdd(&staged_seen[index],1u)!=0u {atomicAdd(&staged_errors[1],1u);}
  atomicAdd(&staged_errors[4],1u);
 }
 let segment=staged_segments[index];var hit=precise_miss(0u);
 if segment.state==1u {
  if ARCHIVE_RAY_QUEUE_PROOF {atomicAdd(&staged_errors[6],1u);}
  hit=archive_trace_staged(DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));
 }
 staged_hits[index]=hit;
}
@compute @workgroup_size(64) fn cs_ray_queue_check(@builtin(global_invocation_id) id:vec3<u32>) {
 if !ARCHIVE_RAY_QUEUE_PROOF {return;}
 let count=ray_tile.extent.x*ray_tile.extent.y;if id.x>=count {return;}
 // Recompute in original-ID order independently of queue assignment. Compare
 // every hit word, including the low parameter component and signed zeros.
 let segment=staged_segments[id.x];var reference=precise_miss(0u);
 if segment.state==1u {reference=trace_depth_dda_paired(scene_depth,DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}
 let actual=staged_hits[id.x];
 if any(actual.pixel!=reference.pixel)||any(bitcast<vec2<u32>>(actual.parameter)!=bitcast<vec2<u32>>(reference.parameter))||(actual.visited!=reference.visited&&!ARCHIVE_STACK_TRAVERSAL)||actual.padding0!=reference.padding0||actual.padding1!=reference.padding1||actual.padding2!=reference.padding2 {atomicAdd(&staged_errors[8],1u);}
 if atomicLoad(&staged_seen[id.x])!=1u {atomicAdd(&staged_errors[2],1u);}
 atomicAdd(&staged_errors[5],1u);
 if id.x==0u&&atomicLoad(&staged_bins.header[0])!=count {atomicAdd(&staged_errors[3],1u);}
}
@compute @workgroup_size(8,8) fn cs_ray_traverse(@builtin(global_invocation_id) id:vec3<u32>,@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
 var pixel=id.xy;
 if ARCHIVE_RAY_COHERENT {
  var packed=0xffffff00u|lane;
  if all(pixel<ray_tile.extent) {packed=(archive_ray_schedule_key(staged_segments[pixel.y*ray_tile.extent.x+pixel.x])<<6u)|lane;}
  ray_order[lane]=packed;workgroupBarrier();
  for(var width=2u;width<=64u;width*=2u) {
   for(var stride=width/2u;stride>0u;stride/=2u) {
    let own=ray_order[lane];let peer=ray_order[lane^stride];
    let take_min=((lane&stride)==0u)==((lane&width)==0u);
    let chosen=select(max(own,peer),min(own,peer),take_min);
    // All reads finish before any lane writes the next network stage.
    workgroupBarrier();ray_order[lane]=chosen;workgroupBarrier();
   }
  }
  let original_lane=ray_order[lane]&63u;
  pixel=group.xy*8u+vec2<u32>(original_lane%8u,original_lane/8u);
 }
 if any(pixel>=ray_tile.extent){return;}
 let index=pixel.y*ray_tile.extent.x+pixel.x;let segment=staged_segments[index];var hit=precise_miss(0u);
 if segment.state==1u {hit=archive_trace_staged(DdaVectorPair(segment.begin,segment.begin_low),DdaVectorPair(segment.end,segment.end_low));}
 staged_hits[index]=hit;
}
@compute @workgroup_size(8,8) fn cs_ray_resolve(@builtin(global_invocation_id) id:vec3<u32>) {
 if any(id.xy>=ray_tile.extent){return;}
 let pixel=ray_tile.offset+id.xy;let index=id.y*ray_tile.extent.x+id.x;let segment=staged_segments[index];var colour=vec4<f32>(0.);
 if segment.state==1u {colour=screen_reflection_paired_hit(segment,segment.padding0!=0u,staged_hits[index]);}
 else if segment.state==2u {let ray_input=archive_ray_input(pixel);colour=screen_reflection_original(ray_input.origin,ray_input.direction,ray_input.history,60.,1.,8u,ARCHIVE_RAY_BOUNDS);}
 textureStore(gi_output,vec2<i32>(pixel),colour);
}
