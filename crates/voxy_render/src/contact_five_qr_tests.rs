use crate::{ExperimentalFiveQrInput,ExperimentalFiveQrValue};
#[path="../../../tools/contact-research/contact_fixture.rs"]
mod fixture;
fn dispatch(input:&ExperimentalFiveQrInput)->Vec<u8>{
    let instance=crate::GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program=pollster::block_on(crate::ComputeProgram::new(&device,&ExperimentalFiveQrInput::shader_source())).unwrap();
    let job=program.create_job(&device,input.bytes()).unwrap();
    let mut encoder=device.create_command_encoder(&Default::default());
    for _ in 0..input.steps(){job.encode_step(&mut encoder,[1,1,1]).unwrap();}
    let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
    device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();
    let bytes=read.try_read().unwrap().unwrap();
    if let Some(path)=std::env::var_os("VOXY_FIVE_QR_READBACK"){
        use std::io::Write;
        std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap().write_all(&bytes).unwrap();
    }
    let status=u32::from_le_bytes(bytes[24..28].try_into().unwrap());let step=u32::from_le_bytes(bytes[28..32].try_into().unwrap());
    assert_eq!(status,0,"five-component QR failed status={status} completed_columns={step}");bytes
}
fn native(value:&ExperimentalFiveQrValue,scale:f64)->physics::hair::HairResponseCoefficient{
    assert!((-1022..=1023).contains(&value.exponent));
    physics::hair::HairResponseCoefficient::from_expansion_terms(value.words.map(|v|f64::from(f32::from_bits(v))),[scale,2f64.powi(value.exponent)]).unwrap()
}
fn json_values(values:&[ExperimentalFiveQrValue])->Vec<serde_json::Value>{
    values.iter().map(|v|serde_json::json!({"words":v.words,"exponent":v.exponent,"error_bound_word":v.error_bound.to_bits()})).collect()
}
#[test]
#[ignore="requires physical GPU; exact low-part equality coordinates"]
fn gpu_five_qr_diagonal_low_parts(){
    let low=2f64.powi(-60);
    let input=ExperimentalFiveQrInput::new(&[vec![[1.,low],[0.,0.],[0.,0.]],vec![[0.,0.],[1.,-low],[0.,0.]]],&[1.,2.],1e-14).unwrap();
    let candidate=input.candidate(&dispatch(&input)).unwrap();
    let first=native(&candidate.coordinates[0],input.state_scale());
    let second=native(&candidate.coordinates[1],input.state_scale());
    assert_ne!(first.parts()[1],0.);assert_ne!(second.parts()[1],0.);
    assert_eq!(native(&candidate.coordinates[2],input.state_scale()).parts(),[0.,0.]);
}
#[test]
#[ignore="requires real GPU, original VQC1 operator and physics-admitted active row baseline"]
fn gpu_five_qr_original_physical_active_columns(){
    let f=fixture::read(std::path::Path::new(&std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()));
    let baseline:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_FIVE_QR_ACTIVE_ROWS").unwrap()).unwrap()).unwrap();
    assert_eq!(baseline["rows"].as_u64().unwrap() as usize,f.rows);assert_eq!(baseline["width"].as_u64().unwrap() as usize,f.width);
    assert_eq!(baseline["original_bounds_bits"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()).collect::<Vec<_>>(),f.bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>());
    assert_eq!(baseline["tolerance_bits"].as_u64().unwrap(),f.tolerance.to_bits());
    let active=baseline["active_rows"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as usize).collect::<Vec<_>>();
    assert!(!active.is_empty()&&active.windows(2).all(|w|w[0]<w[1])&&active.iter().all(|&i|i<f.rows));
    let mut columns=vec![Vec::new();active.len()];
    for request in &f.requests {
        let all=request.whiten_loads_expanded().unwrap();
        for (column,&id) in columns.iter_mut().zip(&active){column.extend(all[id].iter().map(|v|v.parts()));}
    }
    let bounds=active.iter().map(|&i|f.bounds[i]).collect::<Vec<_>>();
    let input=ExperimentalFiveQrInput::new(&columns,&bounds,f.tolerance).unwrap();
    let candidate=input.candidate(&dispatch(&input)).unwrap();
    let coordinates=candidate.coordinates.iter().map(|v|native(v,input.state_scale())).collect::<Vec<_>>();
    let mut reactions=vec![0.;f.rows];
    for (&id,value) in active.iter().zip(&candidate.reactions){let [hi,lo]=native(value,input.reaction_scale()).parts();reactions[id]=hi+lo;assert!(reactions[id]>=0.);}
    let mut offset=0;let mut responses=Vec::new();
    for request in &f.requests {
        let end=offset+request.system.rhs.len();
        let response=request.backtransform_expanded(&coordinates[offset..end]).unwrap();
        responses.push(response.iter().map(|v|{let [hi,lo]=v.parts();hi+lo}).collect::<Vec<_>>());offset=end;
    }
    assert_eq!(offset,f.width);
    physics::hair::HairResponseSystem::validate_joint_solution(&f.requests,&f.bounds,&responses,&reactions,f.tolerance).unwrap();
    let report=serde_json::json!({"scope":"Five-component QR on a baseline-admitted active set; all original physical inequalities checked; not automatic five-component active-set selection or trajectory proof","original_rows":f.rows,"width":f.width,"active_rows":active,"original_physical_admission":true,"original_tolerance_bits":f.tolerance.to_bits(),"column_parts_bits":columns.iter().map(|c|c.iter().map(|v|v.map(f64::to_bits)).collect::<Vec<_>>()).collect::<Vec<_>>(),"bounds_bits":bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"state_scale_bits":input.state_scale().to_bits(),"reaction_scale_bits":input.reaction_scale().to_bits(),"coordinates":json_values(&candidate.coordinates),"reactions":json_values(&candidate.reactions),"basis":json_values(&candidate.basis),"triangular":json_values(&candidate.triangular)});
    if let Some(path)=std::env::var_os("VOXY_FIVE_QR_EXPORT") {let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();serde_json::to_writer_pretty(file,&report).unwrap();}
    eprintln!("FIVE QR ORIGINAL PHYSICS rows={} active={} width={} original_physical_admission=true scope=fixed_admitted_active_set_not_automatic_selection_or_trajectory",f.rows,active.len(),f.width);
}

#[test]
#[ignore="requires physical GPU, original operator and verified five-component QR state"]
fn gpu_five_gaps_all_original_constraints(){
    let f=fixture::read(std::path::Path::new(&std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()));
    let state:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_FIVE_QR_STATE").unwrap()).unwrap()).unwrap();
    assert_eq!(state["width"].as_u64().unwrap() as usize,f.width);
    let scale=f64::from_bits(state["state_scale_bits"].as_u64().unwrap());
    let solved=state["coordinates"].as_array().unwrap().iter().map(|v|{
        let words=v["words"].as_array().unwrap().iter().map(|w|w.as_u64().unwrap() as u32).collect::<Vec<_>>().try_into().unwrap();
        native(&ExperimentalFiveQrValue{words,exponent:v["exponent"].as_i64().unwrap() as i32,error_bound:f32::from_bits(v["error_bound_word"].as_u64().unwrap() as u32)},scale).parts()
    }).collect::<Vec<_>>();
    let mut columns=vec![Vec::new();f.rows];
    for request in &f.requests {for(column,values) in columns.iter_mut().zip(request.whiten_loads_expanded().unwrap()){column.extend(values.iter().map(|v|v.parts()));}}
    let instance=crate::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
    let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program=pollster::block_on(crate::ComputeProgram::new(&device,&crate::ExperimentalFiveGapInput::shader_source())).unwrap();
    let mut cases=Vec::new();
    for (name,coordinates) in [("zero",vec![[0.,0.];f.width]),("admitted_qr",solved)] {
        let input=crate::ExperimentalFiveGapInput::new(&columns,&coordinates,&f.bounds,f.tolerance).unwrap();
        let job=program.create_job(&device,input.bytes()).unwrap();let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,input.workgroups()).unwrap();
        let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();
        let bytes=read.try_read().unwrap().unwrap();let residuals=input.residuals(&bytes).unwrap();let decisions=input.decisions(&bytes).unwrap();assert_eq!(residuals.len(),f.rows);
        cases.push(serde_json::json!({"name":name,"state_parts_bits":coordinates.iter().map(|v|v.map(f64::to_bits)).collect::<Vec<_>>(),"residual_scale_bits":input.scale().to_bits(),"residuals":json_values(&residuals),"decisions":decisions.iter().map(|v|*v as u32).collect::<Vec<_>>()}));
    }
    let report=serde_json::json!({"scope":"GPU residuals for every original inequality, zero and admitted QR state; not active-set or trajectory proof","rows":f.rows,"width":f.width,"column_parts_bits":columns.iter().map(|c|c.iter().map(|v|v.map(f64::to_bits)).collect::<Vec<_>>()).collect::<Vec<_>>(),"tolerance_bits":f.tolerance.to_bits(),"bounds_bits":f.bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"cases":cases});
    let file=std::fs::OpenOptions::new().write(true).create_new(true).open(std::env::var_os("VOXY_FIVE_GAP_EXPORT").unwrap()).unwrap();serde_json::to_writer(file,&report).unwrap();
}
