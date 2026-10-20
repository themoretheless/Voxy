override ARCHIVE_DDA_FAR_BOUND:bool=false;
// Candidate bridge: retain world-coordinate and clipping residuals end-to-end.
// Consumers must not collapse high/low world positions before reprojection.
struct DdaVectorPair { high:vec4<f32>, low:vec4<f32> }
struct DdaProjectedSegment {
 begin:vec4<f32>,end:vec4<f32>,world_begin:vec4<f32>,world_end:vec4<f32>,
 world_begin_low:vec4<f32>,world_end_low:vec4<f32>,begin_low:vec4<f32>,end_low:vec4<f32>,range:vec4<f32>,clip_w:vec4<f32>,
 state:u32,padding0:u32,padding1:u32,padding2:u32,
}
fn dda_projection_empty(state:u32)->DdaProjectedSegment {
 return DdaProjectedSegment(vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),vec4<f32>(0.),state,0u,0u,0u);
}
fn dda_finite4(v:vec4<f32>)->bool {return !any((bitcast<vec4<u32>>(v)&vec4<u32>(0x7f800000u))==vec4<u32>(0x7f800000u));}
fn dda_stable_pair_lerp(a:DdaVectorPair,b:DdaVectorPair,weights:vec4<f32>)->DdaVectorPair {
 var high=vec4<f32>(0.);var low=vec4<f32>(0.);
 for(var component=0u;component<4u;component++) {
  let av=vec2<f32>(a.high[component],a.low[component]);let bv=vec2<f32>(b.high[component],b.low[component]);var value=vec2<f32>(0.);
  if !projection_pair_less(weights.xy,weights.zw) {value=projection_pair_add(av,projection_pair_mul(projection_pair_add(bv,-av),weights.zw));}
  else {value=projection_pair_add(bv,projection_pair_mul(projection_pair_add(av,-bv),weights.xy));}
  high[component]=value.x;low[component]=value.y;
 }
 return DdaVectorPair(high,low);
}
fn dda_project_world_pair(matrix:mat4x4<f32>,point:DdaVectorPair)->DdaVectorPair {
 var high=vec4<f32>(0.);var low=vec4<f32>(0.);
 for(var row=0u;row<4u;row++) {var sum=vec2<f32>(0.);for(var column=0u;column<4u;column++) {
  sum=projection_pair_add(sum,projection_pair_mul(vec2<f32>(matrix[column][row],0.),vec2<f32>(point.high[column],point.low[column])));
 }high[row]=sum.x;low[row]=sum.y;}
 return DdaVectorPair(high,low);
}
fn dda_project_endpoint(matrix:mat4x4<f32>,point:vec4<f32>)->DdaVectorPair {
 var high=vec4<f32>(0.);var low=vec4<f32>(0.);
 for(var row=0u;row<4u;row++) {
  let coefficients=vec4<f32>(matrix[0][row],matrix[1][row],matrix[2][row],matrix[3][row]);
  var sum=vec2<f32>(0.);var magnitude=0.;
  for(var component=0u;component<4u;component++) {sum=projection_pair_add(sum,projection_pair_mul(vec2<f32>(coefficients[component],0.),vec2<f32>(point[component],0.)));magnitude+=abs(coefficients[component]*point[component]);}
  // This condition chooses stronger arithmetic; it never enlarges a surface
  // or relaxes a hit test. Near-camera cancellation takes the exact sum path.
  if abs(projection_add_scalar(sum.x,sum.y))<0.01*magnitude {sum=projection_dot4_expanded(coefficients,point);}
  high[row]=sum.x;low[row]=sum.y;
 }
 return DdaVectorPair(high,low);
}
fn dda_clip_ndc(clip:DdaVectorPair)->vec3<f32> {
 var ndc=vec3<f32>(0.);for(var axis=0u;axis<3u;axis++) {let value=projection_pair_div(vec2<f32>(clip.high[axis],clip.low[axis]),vec2<f32>(clip.high.w,clip.low.w));ndc[axis]=projection_add_scalar(value.x,value.y);}return ndc;
}
fn dda_plane_distance(matrix:mat4x4<f32>,point:vec4<f32>,plane:u32)->vec2<f32> {
 let axis=min(plane/2u,2u);var distance=vec2<f32>(0.);var magnitude=0.;
 for(var component=0u;component<4u;component++) {
  var coefficient=projection_pair_add(vec2<f32>(matrix[component].w,0.),vec2<f32>(select(matrix[component][axis],-matrix[component][axis],(plane&1u)!=0u),0.));
  if plane==4u {coefficient=vec2<f32>(matrix[component].z,0.);}
  distance=projection_pair_add(distance,projection_pair_mul(coefficient,vec2<f32>(point[component],0.)));
  magnitude+=abs(matrix[component].w*point[component])+abs(matrix[component][axis]*point[component]);
 }
 if abs(projection_add_scalar(distance.x,distance.y))<0.01*magnitude {
  let row=vec4<f32>(matrix[0][axis],matrix[1][axis],matrix[2][axis],matrix[3][axis]);
  if plane==4u {return projection_dot4_expanded(row,point);}
  let rw=vec4<f32>(matrix[0].w,matrix[1].w,matrix[2].w,matrix[3].w);
  return projection_dot_rows_expanded(rw,select(row,-row,(plane&1u)!=0u),point);
 }
 return distance;
}
fn dda_project_segment(matrix:mat4x4<f32>,world_begin:vec3<f32>,world_end:vec3<f32>,viewport:vec2<f32>)->DdaProjectedSegment {
 if !dda_finite4(vec4<f32>(world_begin,1.))||!dda_finite4(vec4<f32>(world_end,1.))||!dda_finite4(vec4<f32>(viewport,0.,0.))||!all(viewport>vec2<f32>(0.)) {return dda_projection_empty(2u);}
 for(var column=0u;column<4u;column++) {if !dda_finite4(matrix[column]) {return dda_projection_empty(2u);}}
 var low=vec2<f32>(0.);var high=vec2<f32>(1.,0.);var begin_weights=vec4<f32>(1.,0.,0.,0.);var end_weights=vec4<f32>(0.,0.,1.,0.);
 for(var plane=0u;plane<6u;plane++) {
  // Prove admission only. Any ambiguous bound takes the unchanged expanded
  // plane calculation and clipping path, including exact boundary cases.
  if ARCHIVE_DDA_FAR_BOUND&&plane==5u&&dda_far_plane_inside(matrix,vec4<f32>(world_begin,1.))&&dda_far_plane_inside(matrix,vec4<f32>(world_end,1.)) {continue;}
  let a=dda_plane_distance(matrix,vec4<f32>(world_begin,1.),plane);let b=dda_plane_distance(matrix,vec4<f32>(world_end,1.),plane);
  if !dda_finite4(vec4<f32>(a,b)) {return dda_projection_empty(2u);}
  let negative_a=projection_pair_less(a,vec2<f32>(0.));let negative_b=projection_pair_less(b,vec2<f32>(0.));
  if negative_a&&negative_b {return dda_projection_empty(0u);}
  if negative_a||negative_b {
   let abs_a=select(a,-a,negative_a);let abs_b=select(b,-b,negative_b);let sum=projection_pair_add(abs_a,abs_b);
   if all(sum==vec2<f32>(0.))||!dda_finite4(vec4<f32>(sum,0.,0.)) {return dda_projection_empty(2u);}
   let wa=projection_pair_div(abs_b,sum);let wb=projection_pair_div(abs_a,sum);let weights=vec4<f32>(wa,wb);
   if negative_a&&projection_pair_less(low,wb) {low=wb;begin_weights=weights;}
   if negative_b&&projection_pair_less(wb,high) {high=wb;end_weights=weights;}
  }
 }
 if projection_pair_less(high,low) {return dda_projection_empty(0u);}
 let input_a=DdaVectorPair(vec4<f32>(world_begin,1.),vec4<f32>(0.));let input_b=DdaVectorPair(vec4<f32>(world_end,1.),vec4<f32>(0.));
 let a=dda_stable_pair_lerp(input_a,input_b,begin_weights);let b=dda_stable_pair_lerp(input_a,input_b,end_weights);
 let original_qa=dda_project_endpoint(matrix,vec4<f32>(world_begin,1.));let original_qb=dda_project_endpoint(matrix,vec4<f32>(world_end,1.));
 let qa=dda_stable_pair_lerp(original_qa,original_qb,begin_weights);let qb=dda_stable_pair_lerp(original_qa,original_qb,end_weights);
 if !dda_finite4(qa.high)||!dda_finite4(qa.low)||!dda_finite4(qb.high)||!dda_finite4(qb.low)||!projection_pair_less(vec2<f32>(0.),vec2<f32>(qa.high.w,qa.low.w))||!projection_pair_less(vec2<f32>(0.),vec2<f32>(qb.high.w,qb.low.w)) {return dda_projection_empty(2u);}
 var begin=vec4<f32>(0.);var begin_low=vec4<f32>(0.);var end=vec4<f32>(0.);var end_low=vec4<f32>(0.);
 for(var axis=0u;axis<3u;axis++) {
  var va=projection_pair_div(vec2<f32>(qa.high[axis],qa.low[axis]),vec2<f32>(qa.high.w,qa.low.w));var vb=projection_pair_div(vec2<f32>(qb.high[axis],qb.low[axis]),vec2<f32>(qb.high.w,qb.low.w));
  if axis<2u {let scale=select(0.5,-0.5,axis==1u);va=projection_pair_mul(projection_pair_add(projection_pair_mul(va,vec2<f32>(scale,0.)),vec2<f32>(0.5,0.)),vec2<f32>(viewport[axis],0.));vb=projection_pair_mul(projection_pair_add(projection_pair_mul(vb,vec2<f32>(scale,0.)),vec2<f32>(0.5,0.)),vec2<f32>(viewport[axis],0.));}
  begin[axis]=va.x;begin_low[axis]=va.y;end[axis]=vb.x;end_low[axis]=vb.y;
 }
 begin.w=qa.high.w;begin_low.w=qa.low.w;end.w=qb.high.w;end_low.w=qb.low.w;
 return DdaProjectedSegment(begin,end,a.high,b.high,a.low,b.low,begin_low,end_low,vec4<f32>(low,high),vec4<f32>(qa.high.w,qa.low.w,qb.high.w,qb.low.w),1u,0u,0u,0u);
}
fn dda_segment_weights_paired(segment:DdaProjectedSegment,parameter:vec2<f32>)->vec4<f32> {
 let inverse=projection_pair_add(vec2<f32>(1.,0.),-parameter);
 let a=projection_pair_mul(inverse,segment.clip_w.zw);let b=projection_pair_mul(parameter,segment.clip_w.xy);let sum=projection_pair_add(a,b);
 return vec4<f32>(projection_pair_div(a,sum),projection_pair_div(b,sum));
}
fn dda_reconstruct_world_paired(segment:DdaProjectedSegment,parameter:vec2<f32>)->DdaVectorPair {
 return dda_stable_pair_lerp(DdaVectorPair(segment.world_begin,segment.world_begin_low),DdaVectorPair(segment.world_end,segment.world_end_low),dda_segment_weights_paired(segment,parameter));
}
fn dda_reconstruct_parameter_paired(segment:DdaProjectedSegment,parameter:vec2<f32>)->vec2<f32> {
 let weights=dda_segment_weights_paired(segment,parameter);
 if !projection_pair_less(weights.xy,weights.zw) {return projection_pair_add(segment.range.xy,projection_pair_mul(projection_pair_add(segment.range.zw,-segment.range.xy),weights.zw));}
 return projection_pair_add(segment.range.zw,projection_pair_mul(projection_pair_add(segment.range.xy,-segment.range.zw),weights.xy));
}

fn dda_reconstruct_world(segment:DdaProjectedSegment,parameter:f32)->DdaVectorPair{return dda_reconstruct_world_paired(segment,vec2<f32>(parameter,0.));}
fn dda_reconstruct_parameter(segment:DdaProjectedSegment,parameter:f32)->vec2<f32>{return dda_reconstruct_parameter_paired(segment,vec2<f32>(parameter,0.));}

fn dda_far_plane_inside(matrix:mat4x4<f32>,point:vec4<f32>)->bool {
 // A positive exact distance alone cannot bypass an unsupported ORIGINAL
 // expanded calculation. Bound its raw row products and all intermediate
 // coefficient sums/carries before admitting the interval shortcut.
 let w=abs(vec4<f32>(matrix[0].w,matrix[1].w,matrix[2].w,matrix[3].w));
 let z=abs(vec4<f32>(matrix[0].z,matrix[1].z,matrix[2].z,matrix[3].z));
 let largest=max(max(max(w.x,w.y),max(w.z,w.w)),max(max(z.x,z.y),max(z.z,z.w)));
 let p=abs(point);let extent=max(max(p.x,p.y),max(p.z,p.w));
 if largest>bitcast<f32>(0x7d7fffffu) {return false;}
 // For a finite nonnegative Float32 with biased exponent E, value <
 // 2^(E-126), including subnormals. E1+E2<=372 therefore guarantees
 // every original raw row product is <2^120, without multiplying first.
 // This leaves headroom for eight products and the expansion scratch sums.
 let matrix_exponent=(bitcast<u32>(largest)>>23u)&255u;
 let point_exponent=(bitcast<u32>(extent)>>23u)&255u;
 if matrix_exponent+point_exponent>372u {return false;}
 var distance=DdaInterval(vec2<f32>(0.),true);
 for(var column=0u;column<4u;column++) {
  // Float comparisons may flush subnormal operands. Inspect the encoding and
  // retain the complete original path for every such input.
  let bits=bitcast<vec3<u32>>(vec3<f32>(matrix[column].w,matrix[column].z,point[column]));
  if any(((bits&vec3<u32>(0x7f800000u))==vec3<u32>(0u))&((bits&vec3<u32>(0x007fffffu))!=vec3<u32>(0u))) {return false;}
  let coefficient=dda_interval_expand(vec2<f32>(projection_add_scalar(matrix[column].w,-matrix[column].z)),1u);
  distance=dda_interval_add(distance,dda_interval_mul(coefficient,DdaInterval(vec2<f32>(point[column]),true)));
 }
 return distance.valid&&distance.bounds.x>0.;
}
