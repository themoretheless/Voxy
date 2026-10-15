//! Qualification runner: current residual sources, existing GPU runtime.
mod joint_contact_dot {pub use voxy_render::JointDotError;}
#[path="../../crates/voxy_render/src/contact_exact_five.rs"] mod contact_exact_five;
pub use contact_exact_five::ExperimentalExactFiveContactInput;
#[derive(Debug)] pub struct ExperimentalFiveQrValue {pub words:[u32;5],pub exponent:i32,pub error_bound:f32}
#[path="../../crates/voxy_render/src/contact_five_gaps.rs"] mod contact_five_gaps;
pub use contact_five_gaps::ExperimentalFiveGapInput;
#[path="contact_fixture.rs"] mod fixture;
fn native(value:&ExperimentalFiveQrValue,scale:f64)->physics::hair::HairResponseCoefficient{
 physics::hair::HairResponseCoefficient::from_expansion_terms(value.words.map(|v|f64::from(f32::from_bits(v))),[scale,2f64.powi(value.exponent)]).unwrap()
}
fn json_values(values:&[ExperimentalFiveQrValue])->Vec<serde_json::Value>{values.iter().map(|v|serde_json::json!({"words":v.words,"exponent":v.exponent,"error_bound_word":v.error_bound.to_bits()})).collect()}
fn main(){
 let low=2f64.powi(-60);let tails=[low,-low,low/4.,-low/4.,0.,low/2.,-low/2.];
 let columns=tails.iter().map(|&v|vec![[1.,v]]).collect::<Vec<_>>();let state=vec![[1.,0.]];let bounds=vec![1.;tails.len()];let tolerance=low/2.;
 let input=ExperimentalFiveGapInput::new(&columns,&state,&bounds,tolerance).unwrap();
 let instance=voxy_render::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
 let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();let program=pollster::block_on(voxy_render::ComputeProgram::new(&device,&ExperimentalFiveGapInput::shader_source())).unwrap();
 let job=program.create_job(&device,input.bytes()).unwrap();let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,input.workgroups()).unwrap();let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();
 let bytes=read.try_read().unwrap().unwrap();let residuals=input.residuals(&bytes).unwrap();let decisions=input.decisions(&bytes).unwrap().iter().map(|d|*d as u32).collect::<Vec<_>>();
 assert_eq!(&decisions[..5],&[2,1,3,3,3]);assert!(decisions[5..].contains(&4));
 let report=serde_json::json!({"rows":tails.len(),"width":1,"column_parts_bits":columns.iter().map(|c|c.iter().map(|v|v.map(f64::to_bits)).collect::<Vec<_>>()).collect::<Vec<_>>(),"bounds_bits":bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"tolerance_bits":tolerance.to_bits(),"cases":[{"name":"low_component_and_tolerance_boundaries","state_parts_bits":state.iter().map(|v|v.map(f64::to_bits)).collect::<Vec<_>>(),"residual_scale_bits":input.scale().to_bits(),"residuals":json_values(&residuals),"decisions":decisions}]});
 serde_json::to_writer(std::fs::OpenOptions::new().write(true).create_new(true).open(std::env::var_os("VOXY_FIVE_GAP_EXPORT").unwrap()).unwrap(),&report).unwrap();
 println!("GPU boundary decisions {decisions:?}");
}
