//! Physical GPU release arithmetic qualification; not an active-set solver.
mod joint_contact_dot{pub use voxy_render::JointDotError;}
#[derive(Debug,Clone)]pub struct ExperimentalFiveQrValue{pub words:[u32;5],pub exponent:i32,pub error_bound:f32}
#[path="../../crates/voxy_render/src/contact_five_release.rs"]mod contact_five_release;
use contact_five_release::{ExperimentalFiveReleaseInput as Input,ExperimentalFiveRelease as Release};
fn scalar(hi:f32,lo:f32,error:f32)->ExperimentalFiveQrValue{ExperimentalFiveQrValue{words:[hi.to_bits(),lo.to_bits(),0,0,0],exponent:0,error_bound:error}}
fn json(v:&ExperimentalFiveQrValue)->serde_json::Value{serde_json::json!({"words":v.words,"exponent":v.exponent,"error_bound_word":v.error_bound.to_bits()})}
fn main(){
 let instance=voxy_render::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
 let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();let program=pollster::block_on(voxy_render::ComputeProgram::new(&device,&Input::shader_source())).unwrap();
 let one=scalar(1.,0.,0.);let minus_one=scalar(-1.,0.,0.);
 let cases=vec![
  ("different_ratios",vec![one.clone(),one.clone()],vec![minus_one.clone(),scalar(-3.,0.,0.)],Some(1)),
  ("exact_tie",vec![one.clone(),one.clone()],vec![minus_one.clone(),minus_one.clone()],Some(0)),
  ("low_component_winner",vec![one.clone(),one.clone()],vec![minus_one.clone(),scalar(-1.,-2f32.powi(-60),0.)],Some(1)),
  ("zero_current",vec![scalar(0.,0.,0.),one.clone()],vec![minus_one.clone(),minus_one.clone()],Some(0)),
  ("nonnegative_candidate",vec![one.clone()],vec![one.clone()],None),
  ("uncertain_candidate_sign",vec![one.clone()],vec![scalar(-1e-20,0.,1e-19)],None),
 ];let mut reports=Vec::new();
 for(name,current,candidate,expected_row)in cases{
  let state=vec![scalar(1.,0.,0.),scalar(-1.,0.,0.),scalar(1.,2f32.powi(-60),0.)];
  let next_state=vec![scalar(-1.,0.,0.),scalar(3.,0.,0.),scalar(-1.,0.,0.)];
  let input=Input::new_with_state(&current,&candidate,&state,&next_state).unwrap();let job=program.create_job(&device,input.bytes()).unwrap();let mut encoder=device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,[1,1,1]).unwrap();let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();let bytes=read.try_read().unwrap().unwrap();let result=input.result(&bytes).unwrap();
  let output=match result{
   Release::Release{row,fraction,reactions,coordinates}=>{assert_eq!(Some(row),expected_row,"{name}");serde_json::json!({"status":"release","row":row,"fraction":json(&fraction),"reactions":reactions.iter().map(json).collect::<Vec<_>>(),"coordinates":coordinates.iter().map(json).collect::<Vec<_>>()})},
   Release::Accept=>{assert_eq!(name,"nonnegative_candidate");serde_json::json!({"status":"accept"})},
   Release::Uncertain=>{assert_eq!(name,"uncertain_candidate_sign","{name} unexpectedly uncertain");serde_json::json!({"status":"uncertain"})},
  };
  reports.push(serde_json::json!({"name":name,"current":current.iter().map(json).collect::<Vec<_>>(),"candidate":candidate.iter().map(json).collect::<Vec<_>>(),"state":state.iter().map(json).collect::<Vec<_>>(),"next_state":next_state.iter().map(json).collect::<Vec<_>>(),"output":output}));
 }
 serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(std::env::var_os("VOXY_FIVE_RELEASE_EXPORT").unwrap()).unwrap(),&serde_json::json!({"scope":"GPU release fraction and updated duals on selected cases; not automatic active set or trajectory","cases":reports})).unwrap();
}
