//! Independent f64 admission test for conservative rejection arithmetic.
use super::{Case,ResultRow};
use wgpu::util::DeviceExt;
pub(super) fn run(device:&wgpu::Device,queue:&wgpu::Queue,shader:&wgpu::ShaderModule,pipeline_layout:&wgpu::PipelineLayout,layout:&wgpu::BindGroupLayout,depth:&wgpu::TextureView,tiles:&wgpu::TextureView) {
 let mut state=0x494e54455256414cu64;let mut random=||{state=state.wrapping_mul(6364136223846793005).wrapping_add(1);(state>>32) as u32};let mut cases=Vec::new();
 for _ in 0..20000 {let a=f32::from_bits((random()&0x807fffff)|((27+random()%200)<<23));let b=f32::from_bits((random()&0x807fffff)|((27+random()%200)<<23));let matrix=std::array::from_fn(|_|f32::from_bits((random()&0x807fffff)|((27+random()%200)<<23)));cases.push(Case {matrix,begin:[a,a*f32::EPSILON*0.25,0.,0.],end:[b,-b*f32::EPSILON*0.25,0.,0.],sample:[0.;4]});}
 let edge=[0.,-0.,f32::from_bits(1),f32::from_bits(0x007fffff),f32::MIN_POSITIVE,f32::from_bits(0x00800001),1.,f32::from_bits(1f32.to_bits()-1),f32::from_bits(1f32.to_bits()+1),f32::MAX,-f32::MAX,f32::INFINITY,f32::NEG_INFINITY,f32::NAN];
 for a in edge {for b in edge {cases.push(Case {matrix:glam::Mat4::IDENTITY.to_cols_array(),begin:[a,0.,0.,0.],end:[b,0.,0.,0.],sample:[0.;4]});}}
 let input=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:None,contents:bytemuck::cast_slice(&cases),usage:wgpu::BufferUsages::STORAGE});let size=(cases.len()*80) as u64;let output=device.create_buffer(&wgpu::BufferDescriptor {label:None,size,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});let read=device.create_buffer(&wgpu::BufferDescriptor {label:None,size,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
 let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:Some("conservative interval arithmetic gate"),layout:Some(pipeline_layout),module:shader,entry_point:Some("interval_proof"),compilation_options:Default::default(),cache:None});
 let bind=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout,entries:&[wgpu::BindGroupEntry {binding:4,resource:wgpu::BindingResource::TextureView(tiles)},wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(depth)},wgpu::BindGroupEntry {binding:1,resource:input.as_entire_binding()},wgpu::BindGroupEntry {binding:2,resource:output.as_entire_binding()},wgpu::BindGroupEntry {binding:3,resource:wgpu::BindingResource::TextureView(tiles)}]});let mut encoder=device.create_command_encoder(&Default::default());{let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&pipeline);pass.set_bind_group(0,&bind,&[]);pass.dispatch_workgroups((cases.len() as u32).div_ceil(64),1,1);}encoder.copy_buffer_to_buffer(&output,0,&read,0,size);let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();let data=read.slice(..).get_mapped_range().unwrap();let mut valid=[0usize;3];let mut failures=0;let mut far_admitted=0;let mut paired_quotient_cases=0;let mut paired_quotient_failures=0;let mut paired_accuracy_cases=0;let mut paired_accuracy_failures=0;let mut min_normal_regressions=0;let(mut reciprocal_cases,mut reciprocal_paired_cases,mut reciprocal_failures)=(0,0,0);
 for (i,row) in data.chunks_exact(80).enumerate(){let row:ResultRow=bytemuck::pod_read_unaligned(row);let case=cases[i];
 if row.state&32!=0 {
  reciprocal_cases+=1;let bounds=&row.world_low[2..];
  let expected=(f64::from(case.begin[0])+f64::from(case.begin[1]))/(f64::from(case.end[0])+f64::from(case.end[1]));
  assert!(bounds.iter().all(|v|v.to_bits()&0x7f800000!=0||v.to_bits()&0x007fffff==0),"cached reciprocal endpoints must remain FTZ safe");
  let mut okay=expected.is_finite()&&f64::from(bounds[0])<=expected&&f64::from(bounds[1])>=expected;
  if row.state&16!=0 {
   reciprocal_paired_cases+=1;
   let less=|a:[f32;2],b:[f32;2]|a[0]<b[0]||(a[0]==b[0]&&a[1]<b[1]);
   okay=okay&&!less(row.world_parameter,[bounds[0],0.])&&!less([bounds[1],0.],row.world_parameter);
  }
  if !okay {if reciprocal_failures<10 {eprintln!("RECIPROCAL_INTERVAL_MISMATCH case={i} expected={expected} actual_pair={:?} bounds={bounds:?}",row.world_parameter);}reciprocal_failures+=1;}
 }
 if row.state&16!=0 {
  paired_quotient_cases+=1;
  let expected=(f64::from(case.begin[0])+f64::from(case.begin[1]))/(f64::from(case.end[0])+f64::from(case.end[1]));
  let actual=f64::from(row.world_parameter[0])+f64::from(row.world_parameter[1]);
  if expected.abs()>=2f64.powi(-60)&&expected.abs()<=2f64.powi(60) {
   paired_accuracy_cases+=1;
   // Preserve the established projected-depth reference error contract.
   let error=(actual-expected).abs()/expected.abs();
   if !error.is_finite()||error>4096.*f64::from(f32::EPSILON).powi(2) {
    if paired_accuracy_failures<10 {eprintln!("PAIRED_QUOTIENT_ACCURACY_MISMATCH case={i} expected={expected} actual={actual} relative_error={error}");}
    paired_accuracy_failures+=1;
   }
  }
  if case.begin[0].to_bits()==0x00800000&&case.end[0].to_bits()==0x00800001&&case.begin[1]==0.&&case.end[1]==0. {
   min_normal_regressions+=1;
   println!("GPU_PAIRED_DIVISION_MIN_NORMAL numerator_bits=00800000 denominator_bits=00800001 expected_f64={expected:.17} actual_pair={:?} actual_f64={actual:.17}",row.world_parameter);
  }
  let bounds=&row.world_low[..2];
  let less=|a:[f32;2],b:[f32;2]|a[0]<b[0]||(a[0]==b[0]&&a[1]<b[1]);
  // Match the original paired comparison and retain residuals even where
  // summing into f64 would round them away at a float32 endpoint.
  if row.world_parameter.iter().any(|v|!v.is_finite())||less(row.world_parameter,[bounds[0],0.])||less([bounds[1],0.],row.world_parameter) {
   if paired_quotient_failures<10 {eprintln!("PAIRED_QUOTIENT_INTERVAL_MISMATCH case={i} numerator={:?} denominator={:?} actual_pair={:?} bounds={bounds:?}",&case.begin[..2],&case.end[..2],row.world_parameter);}
   paired_quotient_failures+=1;
  }
 }
 if row.state&8!=0 {far_admitted+=1;let point=[case.begin[0],case.begin[1],case.begin[2],1.];let mut expansion=Vec::<f64>::new();for column in 0..4 {for term in [f64::from(case.matrix[column*4+3])*f64::from(point[column]),-f64::from(case.matrix[column*4+2])*f64::from(point[column])] {let mut carry=term;for part in &mut expansion {let sum=carry+*part;let v=sum-carry;let error=(carry-(sum-v))+(*part-v);*part=error;carry=sum;}expansion.push(carry);}}let sign=expansion.iter().rev().copied().find(|&v|v!=0.).unwrap_or(0.);if !sign.is_finite()||sign<=0. {if failures<10 {eprintln!("FAR_BOUND_MISMATCH case={i} expansion={expansion:?}");}failures+=1;}}let a=f64::from(case.begin[0])+f64::from(case.begin[1]);let b=f64::from(case.end[0])+f64::from(case.end[1]);let expected=[a+b,a*b,a/b];let intervals=[&row.world_high[..2],&row.world_high[2..],&row.world_low[..2]];for operation in 0..3 {if row.state&(1<<operation)!=0 {valid[operation]+=1;let bounds=intervals[operation];assert!(bounds.iter().all(|v|v.to_bits()&0x7f800000!=0||v.to_bits()&0x007fffff==0),"conservative endpoints must remain normal or zero under FTZ");if !expected[operation].is_finite()||f64::from(bounds[0])>expected[operation]||f64::from(bounds[1])<expected[operation] {if failures<10 {eprintln!("INTERVAL_MISMATCH case={i} operation={operation} a={a} b={b} expected={} bounds={bounds:?}",expected[operation]);}failures+=1;}}}}
 assert!(reciprocal_cases>10000&&reciprocal_paired_cases>1000);
 println!("GPU_RECIPROCAL_INTERVAL_CACHE valid_cases={reciprocal_cases} paired_cases={reciprocal_paired_cases} failures={reciprocal_failures} independent_f64_and_actual_paired_containment=true residual_lexicographic_comparison=true universal_proof=false");
 assert_eq!(reciprocal_failures,0,"cached reciprocal interval must contain both real and paired quotients");
 assert!(paired_accuracy_cases>1000&&min_normal_regressions==1,"accuracy domain and original regression must be exercised");
 println!("GPU_PAIRED_QUOTIENT_ACCURACY cases={paired_accuracy_cases} failures={paired_accuracy_failures} independent_f64_reference=true original_min_normal_regression=true established_relative_error_limit_4096_eps_squared=true");
 assert_eq!(paired_accuracy_failures,0,"interval containment alone does not prove quotient accuracy");
 assert!(paired_quotient_cases>1000,"paired quotient domain must be exercised");
 println!("GPU_PAIRED_QUOTIENT_INTERVAL cases={paired_quotient_cases} failures={paired_quotient_failures} domain_normalized_abs_high_at_most_two=true actual_gpu_paired_result_checked=true residual_lexicographic_comparison=true universal_proof=false");
 assert_eq!(paired_quotient_failures,0,"real-number containment alone does not qualify paired division");
 assert!(far_admitted>1000,"far gate must exercise admission");assert!(valid.iter().all(|&n|n>10000),"interval gate must exercise useful finite bounds");println!("GPU_DDA_INTERVALS cases={} operations=3 far_admitted={far_admitted} exact_dyadic_far_sign=true valid_add_mul_div={valid:?} failures={failures} independent_f64_containment=true subnormals_and_nonfinite=true endpoints_normal_or_zero=true geometry_tolerance_changed=false",cases.len());assert_eq!(failures,0);drop(data);read.unmap();
}
