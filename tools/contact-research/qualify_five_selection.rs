//! GPU-only entering-row arithmetic; current selector source and existing runtime.
mod joint_contact_dot {pub use voxy_render::JointDotError;}
#[derive(Debug,Clone)] pub struct ExperimentalFiveQrValue {pub words:[u32;5],pub exponent:i32,pub error_bound:f32}
#[path="../../crates/voxy_render/src/contact_five_select.rs"] mod contact_five_select;
use contact_five_select::{ExperimentalFiveSelectionInput as Input,ExperimentalFiveSelection as Selection};
fn decode(v:&serde_json::Value)->ExperimentalFiveQrValue{ExperimentalFiveQrValue{words:v["words"].as_array().unwrap().iter().map(|w|w.as_u64().unwrap() as u32).collect::<Vec<_>>().try_into().unwrap(),exponent:v["exponent"].as_i64().unwrap() as i32,error_bound:f32::from_bits(v["error_bound_word"].as_u64().unwrap() as u32)}}
fn scalar(hi:f32,lo:f32,error:f32)->ExperimentalFiveQrValue{ExperimentalFiveQrValue{words:[hi.to_bits(),lo.to_bits(),0,0,0],exponent:0,error_bound:error}}
fn main(){
 let source:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_FIVE_GAPS_SOURCE").unwrap()).unwrap()).unwrap();
 let qr:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var_os("VOXY_FIVE_QR_STATE").unwrap()).unwrap()).unwrap();
 let instance=voxy_render::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
 let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();let program=pollster::block_on(voxy_render::ComputeProgram::new(&device,&Input::shader_source())).unwrap();
 let mut cases=Vec::new();
 for c in source["cases"].as_array().unwrap(){
  let gaps=c["residuals"].as_array().unwrap().iter().map(decode).collect::<Vec<_>>();let decisions=c["decisions"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as u32).collect::<Vec<_>>();let mut active=vec![false;gaps.len()];
  if c["name"]=="admitted_qr"{for id in qr["active_rows"].as_array().unwrap(){active[id.as_u64().unwrap() as usize]=true;}}
  cases.push((c["name"].as_str().unwrap().to_string(),gaps,decisions,active));
 }
 cases.push(("low_component_and_active_exclusion".into(),vec![scalar(-1.,0.,0.),scalar(-1.,-2f32.powi(-60),0.),scalar(-2.,0.,0.)],vec![1,1,1],vec![false,false,true]));
 cases.push(("overlapping_error_intervals".into(),vec![scalar(-1.,0.,1e-20),scalar(-1.,-2f32.powi(-80),1e-20)],vec![1,1],vec![false,false]));
 cases.push(("exact_tie".into(),vec![scalar(-1.,0.,0.),scalar(-1.,0.,0.)],vec![1,1],vec![false,false]));
 let mut reports=Vec::new();
 for (name,gaps,decisions,active) in cases {
  let input=Input::new(&gaps,&decisions,&active).unwrap();let job=program.create_job(&device,input.bytes()).unwrap();let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,[1,1,1]).unwrap();let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();let bytes=read.try_read().unwrap().unwrap();
  let result=input.selection(&bytes).unwrap();
  match name.as_str(){"low_component_and_active_exclusion"=>assert_eq!(result,Selection::Enter(1)),"overlapping_error_intervals"=>assert_eq!(result,Selection::Uncertain),"exact_tie"=>assert_eq!(result,Selection::Enter(0)),_=>{}}
  let(status,row)=match result{Selection::Enter(i)=>("enter",Some(i)),Selection::Feasible=>("inactive_feasible",None),Selection::Uncertain=>("uncertain",None)};
  reports.push(serde_json::json!({"name":name,"gaps":gaps.iter().map(|v|serde_json::json!({"words":v.words,"exponent":v.exponent,"error_bound_word":v.error_bound.to_bits()})).collect::<Vec<_>>(),"decisions":decisions,"active":active,"status":status,"row":row}));
 }
 serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(std::env::var_os("VOXY_FIVE_SELECTION_EXPORT").unwrap()).unwrap(),&serde_json::json!({"scope":"certified GPU entering-row selection; not release/iteration or trajectory proof","cases":reports})).unwrap();
}
