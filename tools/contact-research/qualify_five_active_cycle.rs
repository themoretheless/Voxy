//! Automatic five-component GPU active-set qualification on an ORIGINAL operator.
//! Device arithmetic owns QR, gaps, entering selection, release and interpolation.
mod joint_contact_dot{pub use voxy_render::JointDotError;}
#[path="../../crates/voxy_render/src/contact_exact_five.rs"]mod contact_exact_five;
pub use contact_exact_five::ExperimentalExactFiveContactInput;
#[path="../../crates/voxy_render/src/contact_five_qr.rs"]mod contact_five_qr;
pub use contact_five_qr::{ExperimentalFiveQrInput,ExperimentalFiveQrValue};
#[path="../../crates/voxy_render/src/contact_five_gaps.rs"]mod contact_five_gaps;
#[path="../../crates/voxy_render/src/contact_five_select.rs"]mod contact_five_select;
#[path="../../crates/voxy_render/src/contact_five_release.rs"]mod contact_five_release;
#[path="contact_fixture.rs"]mod fixture;
use contact_five_gaps::ExperimentalFiveGapInput as Gaps;
use contact_five_select::{ExperimentalFiveSelectionInput as Select,ExperimentalFiveSelection};
use contact_five_release::{ExperimentalFiveReleaseInput as Release,ExperimentalFiveRelease};
fn zero()->ExperimentalFiveQrValue{ExperimentalFiveQrValue{words:[0;5],exponent:0,error_bound:0.}}
fn scale(values:&mut[ExperimentalFiveQrValue],scale:f64){assert!(scale.is_normal()&&scale>0.&&scale.to_bits()&0x000f_ffff_ffff_ffff==0);let shift=((scale.to_bits()>>52)&2047)as i32-1023;for v in values{v.exponent+=shift;}}
fn native(v:&ExperimentalFiveQrValue)->physics::hair::HairResponseCoefficient{assert!((-1022..=1023).contains(&v.exponent));physics::hair::HairResponseCoefficient::from_expansion_terms(v.words.map(|w|f64::from(f32::from_bits(w))),[1.,2f64.powi(v.exponent)]).unwrap()}
fn json(values:&[ExperimentalFiveQrValue])->Vec<serde_json::Value>{values.iter().map(|v|serde_json::json!({"words":v.words,"exponent":v.exponent,"error_bound_word":v.error_bound.to_bits()})).collect()}
fn dispatch(device:&wgpu::Device,queue:&wgpu::Queue,program:&voxy_render::ComputeProgram,input:&[u8],steps:usize,groups:[u32;3])->Vec<u8>{
 let job=program.create_job(device,input).unwrap();let mut encoder=device.create_command_encoder(&Default::default());for _ in 0..steps{job.encode_step(&mut encoder,groups).unwrap();}let snapshot=job.encode_snapshot(&mut encoder).unwrap();let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None}).unwrap();read.try_read().unwrap().unwrap()
}
fn run()->Result<serde_json::Value,Box<dyn std::error::Error>>{
 let f=fixture::read(std::path::Path::new(&std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").ok_or("missing original fixture")?));
 let mut columns=vec![Vec::new();f.rows];for request in &f.requests{for(col,values)in columns.iter_mut().zip(request.whiten_loads_expanded()?){col.extend(values.iter().map(|v|v.parts()));}}
 let instance=voxy_render::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);let(device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
 let qr=pollster::block_on(voxy_render::ComputeProgram::new(&device,&ExperimentalFiveQrInput::shader_source()))?;
 let gaps=pollster::block_on(voxy_render::ComputeProgram::new(&device,&Gaps::shader_source()))?;
 let select=pollster::block_on(voxy_render::ComputeProgram::new(&device,&Select::shader_source()))?;
 let release=pollster::block_on(voxy_render::ComputeProgram::new(&device,&Release::shader_source()))?;
 let mut active:Vec<usize>=Vec::new();let mut state=vec![zero();f.width];let mut reactions=vec![zero();f.rows];let mut need_qr=false;let mut trace=Vec::new();let mut completed=false;
 for iteration in 0..512usize.max(f.rows+1){
  eprintln!("FIVE ACTIVE iteration={iteration} active={active:?} qr={need_qr}");
  if need_qr{
   let selected=active.iter().map(|&i|columns[i].clone()).collect::<Vec<_>>();let targets=active.iter().map(|&i|f.bounds[i]).collect::<Vec<_>>();
   let input=ExperimentalFiveQrInput::new(&selected,&targets,f.tolerance)?;let mut candidate=input.candidate(&dispatch(&device,&queue,&qr,input.bytes(),input.steps(),[1,1,1]))?;
   scale(&mut candidate.coordinates,input.state_scale());scale(&mut candidate.reactions,input.reaction_scale());
   let current=active.iter().map(|&i|reactions[i].clone()).collect::<Vec<_>>();let step=Release::new_with_state(&current,&candidate.reactions,&state,&candidate.coordinates)?;
   match step.result(&dispatch(&device,&queue,&release,step.bytes(),1,[1,1,1]))?{
    ExperimentalFiveRelease::Accept=>{state=candidate.coordinates;reactions.fill(zero());for(&id,value)in active.iter().zip(candidate.reactions){reactions[id]=value;}need_qr=false;},
    ExperimentalFiveRelease::Release{row,fraction,reactions:updated,coordinates}=>{
     let id=active[row];state=coordinates;for(&id,value)in active.iter().zip(updated){reactions[id]=value;}active.remove(row);reactions[id]=zero();need_qr=!active.is_empty();
     trace.push(serde_json::json!({"iteration":iteration,"release":id,"fraction":json(&[fraction])}));continue;
    },ExperimentalFiveRelease::Uncertain=>return Err(format!("uncertain release at iteration {iteration}, active {active:?}").into()),
   }
  }
  let input=Gaps::new_with_runtime_state(&columns,&state,1.,&f.bounds,f.tolerance)?;let bytes=dispatch(&device,&queue,&gaps,input.bytes(),1,input.workgroups());let residuals=input.residuals(&bytes)?;let decisions=input.decisions(&bytes)?.iter().map(|v|*v as u32).collect::<Vec<_>>();let mut flags=vec![false;f.rows];for &id in &active{flags[id]=true;if decisions[id]!=3{return Err(format!("active equality not certified at row {id}, iteration {iteration}").into());}}
  let input=Select::new(&residuals,&decisions,&flags)?;
  match input.selection(&dispatch(&device,&queue,&select,input.bytes(),1,[1,1,1]))?{
   ExperimentalFiveSelection::Enter(id)=>{trace.push(serde_json::json!({"iteration":iteration,"enter":id}));active.push(id);active.sort_unstable();need_qr=true;},
   ExperimentalFiveSelection::Feasible=>{completed=true;break;},ExperimentalFiveSelection::Uncertain=>return Err(format!("uncertain entering row at iteration {iteration}").into()),
  }
 }
 if !completed{return Err("unchanged active-set iteration budget exhausted".into());}
 let coordinates=state.iter().map(native).collect::<Vec<_>>();let duals=reactions.iter().map(|v|{let[h,l]=native(v).parts();h+l}).collect::<Vec<_>>();let mut offset=0;let mut responses=Vec::new();for request in &f.requests{let end=offset+request.system.rhs.len();let values=request.backtransform_expanded(&coordinates[offset..end])?;responses.push(values.iter().map(|v|{let[h,l]=v.parts();h+l}).collect::<Vec<_>>());offset=end;}
 physics::hair::HairResponseSystem::validate_joint_solution(&f.requests,&f.bounds,&responses,&duals,f.tolerance)?;
 Ok(serde_json::json!({"scope":"Automatic five-component GPU active-set selection and original physical admission on one supplied operator; host controls dispatch and indices; not resident performance or trajectory proof","original_rows":f.rows,"width":f.width,"active_rows":active,"original_tolerance_bits":f.tolerance.to_bits(),"original_physical_admission":true,"native_contact_fallbacks":0,"trace":trace,"coordinates":json(&state),"reactions":json(&reactions)}))
}
fn main(){let result=match run(){Ok(v)=>v,Err(e)=>{eprintln!("FIVE ACTIVE FAILURE {e}");serde_json::json!({"status":"failed","error":e.to_string(),"full_cycle_passed":false})}};let failed=result["status"]=="failed";serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(std::env::var_os("VOXY_FIVE_ACTIVE_EXPORT").unwrap()).unwrap(),&result).unwrap();if failed{std::process::exit(1);}}
