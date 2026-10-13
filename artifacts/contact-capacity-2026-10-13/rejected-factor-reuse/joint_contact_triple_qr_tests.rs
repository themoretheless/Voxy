use super::*;
fn shader()->String {format!("{}\n{}",EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,include_str!("joint_contact_triple_qr.wgsl"))}
#[test]
fn triple_qr_shader_validates() {
 let module=naga::front::wgsl::parse_str(&shader()).unwrap();
 naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::empty()).validate(&module).unwrap();
}
fn compensated_dot(a:&[f64],b:&[f64])->f64 {
 let mut sum=0.;let mut correction=0.;
 for (&a,&b) in a.iter().zip(b) {let product=a*b;let error=a.mul_add(b,-product);let value=product-correction;let next=sum+value;correction=(next-sum)-value-error;sum=next;}
 sum-correction
}
#[test]
#[ignore="requires actual GPU and original rejected operator JSON; experimental QR only"]
fn gpu_triple_qr_original_active_operator() {
 let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap()).unwrap();
 let decode_bits=|v:&serde_json::Value|f64::from_bits(v.as_u64().unwrap());
 let original=fixture["columns_bits"].as_array().unwrap().iter().map(|row|row.as_array().unwrap().iter().map(decode_bits).collect::<Vec<_>>()).collect::<Vec<_>>();
 let original_bounds=fixture["bounds_bits"].as_array().unwrap().iter().map(decode_bits).collect::<Vec<_>>();
 let active=fixture["reactions_bits"].as_array().unwrap().iter().enumerate().filter_map(|(i,v)|(decode_bits(v)>0.).then_some(i)).collect::<Vec<_>>();
 assert_eq!(active,[2,26,27,48,53,60]);
 let columns=active.iter().map(|&i|original[i].clone()).collect::<Vec<_>>();
 let bounds=active.iter().map(|&i|original_bounds[i]).collect::<Vec<_>>();
 let tolerance=decode_bits(&fixture["tolerance_bits"]);
 let input=ExperimentalTripleContactInput::new(&columns,&bounds,tolerance).unwrap();
 let count=input.rows;let width=input.width;
 let mut words=vec![count as u32,width as u32];words.extend(input.columns.iter().flatten().copied());words.extend(input.bounds.iter().flatten().copied());
 let q=words.len();let r=q+3*count*width;let state=r+3*count*count;let reaction=state+3*width;let status=reaction+3*count;
 words.resize(status+1,0);
 let output=dispatch(&words);
 assert_eq!(&output[..q],&words[..q]);assert_eq!(output[status],0,"QR candidate rejected");
 let decode=|offset| {let a=f32::from_bits(output[offset]) as f64;let b=f32::from_bits(output[offset+1]) as f64;let c=f32::from_bits(output[offset+2]) as f64;let sum=a+b;let v=sum-a;sum+((a-(sum-v))+(b-v)+c)};
 let basis=(0..count).map(|i|(0..width).map(|j|decode(q+3*(i*width+j))).collect::<Vec<_>>()).collect::<Vec<_>>();
 let mut orthogonal=0f64;let mut reconstruction=0f64;
 for i in 0..count {
  for j in 0..count {orthogonal=orthogonal.max((compensated_dot(&basis[i],&basis[j])-if i==j {1.}else{0.}).abs());}
  for coord in 0..width {
   let coefficients=(0..count).map(|j|decode(r+3*(j*count+i))).collect::<Vec<_>>();
   let values=(0..count).map(|j|basis[j][coord]).collect::<Vec<_>>();
   reconstruction=reconstruction.max((compensated_dot(&coefficients,&values)-columns[i][coord]/input.column_scale).abs());
  }
 }
 let motion=(0..width).map(|i|decode(state+3*i)*input.bound_scale/input.column_scale).collect::<Vec<_>>();
 let lambda=(0..count).map(|i|(decode(reaction+3*i)*input.bound_scale/input.column_scale)/input.column_scale).collect::<Vec<_>>();
 let worst_gap=original.iter().zip(&original_bounds).map(|(column,bound)|bound-compensated_dot(column,&motion)).fold(0f64,f64::max);
 let equality=columns.iter().zip(&bounds).map(|(column,bound)|(bound-compensated_dot(column,&motion)).abs()).fold(0f64,f64::max);
 eprintln!("TRIPLE ORIGINAL QR active={active:?} width={width} orthogonality={orthogonal} reconstruction={reconstruction} original_worst_gap={worst_gap} active_equality={equality} min_reaction={} tolerance={tolerance} scope=fixed_active_QR_not_full_active_set_or_physical_admission",lambda.iter().copied().fold(f64::INFINITY,f64::min));
 assert!(orthogonal<=16.*f64::EPSILON);assert!(reconstruction<=16.*f64::EPSILON);
 assert!(lambda.iter().all(|v|v.is_finite()&&*v>=0.));assert!(motion.iter().all(|v|v.is_finite()));
 assert!(equality<=tolerance);assert!(worst_gap<=tolerance,"original inequalities reject candidate");
}

fn dispatch(words:&[u32])->Vec<u32> {dispatch_shader(words,&shader())}
fn dispatch_shader(words:&[u32],source:&str)->Vec<u32> {
 let instance=crate::GraphicsOptions::default().create_instance();
 let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
 let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
 let program=pollster::block_on(crate::ComputeProgram::new(&device,source)).unwrap();
 let job=program.create_job(&device,bytemuck::cast_slice(words)).unwrap();
 let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,[1,1,1]).unwrap();
 let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
 device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
 let bytes=read.try_read().unwrap().unwrap();let output=bytes.chunks_exact(4).map(|v|u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>();
 output
}

#[test]
#[ignore="requires actual GPU; exact dependence and tiny independent directions"]
fn gpu_triple_qr_rejects_exact_dependencies_without_rank_cutoff() {
 let cases=[
  (vec![vec![1.,0.],vec![1.,0.]],8),
  (vec![vec![1.,0.],vec![-1.,0.]],8),
  (vec![vec![1.,0.],vec![0.,0.]],2),
  (vec![vec![1.,0.],vec![1.,1e-12]],0),
 ];
 for (columns,expected_status) in cases {
  let input=ExperimentalTripleContactInput::new(&columns,&[1.,1.],1e-14).unwrap();
  let count=input.rows;let width=input.width;
  let mut words=vec![count as u32,width as u32];words.extend(input.columns.iter().flatten().copied());words.extend(input.bounds.iter().flatten().copied());
  let immutable=words.len();let status=immutable+3*count*width+3*count*count+3*width+3*count;
  words.resize(status+1,0);let output=dispatch(&words);
  assert_eq!(&output[..immutable],&words[..immutable]);
  assert_eq!(output[status],expected_status,"columns={columns:?}");
 }
 eprintln!("TRIPLE QR DEPENDENCE duplicate=reject opposite=reject zero=reject independent_angle_1e_minus12=accepted no_rank_threshold=true scope=qualification_only");
}

fn active_shader()->String {if std::env::var_os("VOXY_TRIPLE_TEST_PARALLEL").is_some() {ExperimentalTripleActiveContactInput::parallel_shader_source()} else {ExperimentalTripleActiveContactInput::shader_source()}}
#[test]
fn triple_active_shader_validates() {
 let module=naga::front::wgsl::parse_str(&active_shader()).unwrap();
 naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::empty()).validate(&module).unwrap();
}
#[test]
#[ignore="requires actual GPU and original rejected operator; experimental automatic active set"]
fn gpu_triple_automatic_original_operator() {
 let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap()).unwrap();
 let decode_bits=|v:&serde_json::Value|f64::from_bits(v.as_u64().unwrap());
 let columns=fixture["columns_bits"].as_array().unwrap().iter().map(|row|row.as_array().unwrap().iter().map(decode_bits).collect::<Vec<_>>()).collect::<Vec<_>>();
 let bounds=fixture["bounds_bits"].as_array().unwrap().iter().map(decode_bits).collect::<Vec<_>>();
 let tolerance=decode_bits(&fixture["tolerance_bits"]);
 let input=ExperimentalTripleContactInput::new(&columns,&bounds,tolerance).unwrap();let count=input.rows;let width=input.width;
 let mut words=vec![count as u32,width as u32,4*(count*count) as u32,0];
 words.extend(input.columns.iter().flatten().copied());words.extend(input.bounds.iter().flatten().copied());words.extend(input.tolerance);
 let immutable=words.len();let q=immutable;let r=q+3*count*width;let candidate=r+3*count*count;
 let candidate_reactions=candidate+3*width;let state=candidate_reactions+3*count;let reactions=state+3*width;let status=reactions+3*count;
 words.resize(status+1,0);
 let typed=ExperimentalTripleActiveContactInput::new(&columns,&bounds,tolerance).unwrap();
 words=typed.bytes().chunks_exact(4).map(|v|u32::from_le_bytes(v.try_into().unwrap())).collect();
 let output=dispatch_shader(&words,&active_shader());
 let (typed_motion,typed_reactions)=typed.candidate(bytemuck::cast_slice(&output)).unwrap();
 assert_eq!(&output[..3],&words[..3]);assert_eq!(&output[4..immutable],&words[4..immutable]);
 eprintln!("TRIPLE AUTOMATIC status={} iterations={} rows={count} width={width}",output[status],output[3]);
 assert_eq!(output[status],0,"automatic active set rejected");
 let decode=|offset| {let a=f32::from_bits(output[offset]) as f64;let b=f32::from_bits(output[offset+1]) as f64;let c=f32::from_bits(output[offset+2]) as f64;let sum=a+b;let v=sum-a;sum+((a-(sum-v))+(b-v)+c)};
 let motion=(0..width).map(|i|decode(state+3*i)*input.bound_scale/input.column_scale).collect::<Vec<_>>();
 let lambda=(0..count).map(|i|(decode(reactions+3*i)*input.bound_scale/input.column_scale)/input.column_scale).collect::<Vec<_>>();
 assert_eq!(typed_motion,motion);assert_eq!(typed_reactions,lambda);
 let mut worst=0f64;let mut equality=0f64;let mut active=Vec::new();
 for row in 0..count {
  let gap=compensated_dot(&columns[row],&motion)-bounds[row];worst=worst.max(-gap);
  assert!(lambda[row].is_finite()&&lambda[row]>=0.);
  if lambda[row]>0. {active.push(row);equality=equality.max(gap.abs());}
 }
 assert!(motion.iter().all(|v|v.is_finite()));
 eprintln!("TRIPLE AUTOMATIC ORIGINAL active={active:?} worst_gap={worst} equality={equality} tolerance={tolerance} scope=GPU_candidate_not_native_physical_admission_or_FPS");
 assert!(worst<=tolerance&&equality<=tolerance,"original-coordinate contact gate rejected");
}

#[test]
#[ignore="requires actual GPU; automatic solver rejection and release paths"]
fn gpu_triple_automatic_rejections_and_release() {
 let cases=[
  ("exhaustion before work",vec![vec![1.]],vec![1.],0u32,14u32,None),
  ("exhaustion after enter",vec![vec![1.]],vec![1.],1,14,None),
  ("contradictory opposite",vec![vec![1.],vec![-1.]],vec![1.,1.],16,8,None),
  ("violated zero row",vec![vec![0.],vec![1.]],vec![1.,0.],16,2,None),
  ("inactive constraints",vec![vec![1.]],vec![-1.],1,0,Some(vec![0.])),
  ("redundant stricter row",vec![vec![1.],vec![1.]],vec![1.,2.],16,0,Some(vec![2.])),
  ("release negative reaction",vec![vec![1.,0.],vec![2.,1.]],vec![2.,3.],16,0,Some(vec![2.,0.])),
 ];
 for (name,columns,bounds,limit,expected_status,expected_motion) in cases {
  let input=ExperimentalTripleContactInput::new(&columns,&bounds,1e-14).unwrap();let count=input.rows;let width=input.width;
  let mut words=vec![count as u32,width as u32,limit,0];
  words.extend(input.columns.iter().flatten().copied());words.extend(input.bounds.iter().flatten().copied());words.extend(input.tolerance);
  let immutable=words.len();let state=immutable+3*count*width+3*count*count+3*width+3*count;let reactions=state+3*width;let status=reactions+3*count;
  words.resize(status+1,0);let output=dispatch_shader(&words,&active_shader());
  assert_eq!(&output[..3],&words[..3]);assert_eq!(&output[4..immutable],&words[4..immutable]);
  let expected_status=if std::env::var_os("VOXY_TRIPLE_TEST_PARALLEL").is_some()&&matches!(expected_status,2|8) {0x104} else {expected_status};
  assert_eq!(output[status],expected_status,"{name}: iterations={}",output[3]);assert!(output[3]<=limit);
  if let Some(expected) = expected_motion {
   let decode=|offset| {let a=f32::from_bits(output[offset]) as f64;let b=f32::from_bits(output[offset+1]) as f64;let c=f32::from_bits(output[offset+2]) as f64;let sum=a+b;let v=sum-a;sum+((a-(sum-v))+(b-v)+c)};
   let motion=(0..width).map(|i|decode(state+3*i)*input.bound_scale/input.column_scale).collect::<Vec<_>>();
   assert_eq!(motion,expected,"{name}");
   for row in 0..count {
    let reaction=(decode(reactions+3*row)*input.bound_scale/input.column_scale)/input.column_scale;
    assert!(reaction>=0.&&reaction.is_finite(),"{name}");
    let gap=compensated_dot(&columns[row],&motion)-bounds[row];assert!(gap>= -1e-14,"{name}");
    if reaction>0. {assert!(gap.abs()<=1e-14,"{name}");}
   }
   if name=="release negative reaction" {assert!(output[3]>=4,"release path was not exercised");assert_eq!(decode(reactions+3),0.);}
  }
  eprintln!("TRIPLE AUTOMATIC CASE name={name:?} status={} iterations={} scope=qualification_only",output[status],output[3]);
 }
}

#[test]
fn triple_parallel_shader_validates() {
 let source=ExperimentalTripleActiveContactInput::parallel_shader_source();
 let module=naga::front::wgsl::parse_str(&source).unwrap();
 naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::empty()).validate(&module).unwrap();
}

#[test]
#[ignore="requires actual GPU and original operator JSON; warm hints remain candidates"]
fn gpu_triple_seeded_original_operator() {
 let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_TRIPLE_QR_FIXTURE").unwrap()).unwrap()).unwrap();
 let value=|v:&serde_json::Value|f64::from_bits(v.as_u64().unwrap());
 let columns=fixture["columns_bits"].as_array().unwrap().iter().map(|r|r.as_array().unwrap().iter().map(value).collect::<Vec<_>>()).collect::<Vec<_>>();
 let bounds=fixture["bounds_bits"].as_array().unwrap().iter().map(value).collect::<Vec<_>>();let tolerance=value(&fixture["tolerance_bits"]);
 let source=ExperimentalTripleActiveContactInput::parallel_shader_source();
 let solve=|input:&ExperimentalTripleActiveContactInput| {let words=input.bytes().chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();dispatch_shader(&words,&source)};
 let cold=ExperimentalTripleActiveContactInput::new(&columns,&bounds,tolerance).unwrap();let cold_output=solve(&cold);let (_,seeds)=cold.candidate(bytemuck::cast_slice(&cold_output)).unwrap();
 let warm=ExperimentalTripleActiveContactInput::new_seeded(&columns,&bounds,tolerance,&seeds).unwrap();let warm_output=solve(&warm);let (motion,reactions)=warm.candidate(bytemuck::cast_slice(&warm_output)).unwrap();
 for row in 0..bounds.len() {let gap=compensated_dot(&columns[row],&motion)-bounds[row];assert!(gap>= -tolerance);if reactions[row]>0. {assert!(gap.abs()<=tolerance);}}
 assert_eq!(warm_output[3],1);assert!(cold_output[3]>warm_output[3]);
 let factors=warm.parallel_factors(bytemuck::cast_slice(&warm_output)).unwrap();
 let changed_bounds=bounds.iter().map(|v|v*1.000001).collect::<Vec<_>>();
 let mut reused=ExperimentalTripleActiveContactInput::new_seeded(&columns,&changed_bounds,tolerance,&seeds).unwrap();
 reused.reuse_parallel_factors(&factors).unwrap();
 let reused_output=solve(&reused);let (motion,reactions)=reused.candidate(bytemuck::cast_slice(&reused_output)).unwrap();
 for row in 0..bounds.len() {let gap=compensated_dot(&columns[row],&motion)-changed_bounds[row];assert!(gap>= -tolerance);if reactions[row]>0. {assert!(gap.abs()<=tolerance,"factor row={row} gap={gap:e} tolerance={tolerance:e}");}}
 assert_eq!(reused_output[3],1);
 eprintln!("TRIPLE FACTOR REUSE changed_bounds_original_coordinate_gate=true");
 // Arbitrary overcomplete hints may fail. A fresh cold solve must remain usable.
 let bad=ExperimentalTripleActiveContactInput::new_seeded(&[vec![1.],vec![-1.]],&[1.,-1.],1e-14,&[1.,1.]).unwrap();assert!(bad.candidate(bytemuck::cast_slice(&solve(&bad))).is_err());
 let retry=ExperimentalTripleActiveContactInput::new(&[vec![1.],vec![-1.]],&[1.,-1.],1e-14).unwrap();assert!(retry.candidate(bytemuck::cast_slice(&solve(&retry))).is_ok());
 eprintln!("TRIPLE WARM cold_iterations={} warm_iterations={} original_coordinate_gate=true dependent_hint_rejected=true fresh_cold_retry_valid=true scope=coordinate_candidate_only_not_physical_admission",cold_output[3],warm_output[3]);
}
