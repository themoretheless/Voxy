//! Qualification-only baseline active rows, admitted by the original physics.
use std::io::{Read, Write};
use physics::hair::{HairLinearSolver, HairLinearSystem, HairResponseSystem, HairResponseCoefficient as C, HairExpandedCoordinateRequest as R};
fn integer(input: &mut impl Read)->usize {let mut b=[0;4];input.read_exact(&mut b).unwrap();u32::from_le_bytes(b) as usize}
fn scalar(input: &mut impl Read)->f64 {let mut b=[0;8];input.read_exact(&mut b).unwrap();f64::from_le_bytes(b)}
struct Baseline {device:wgpu::Device,queue:wgpu::Queue,program:voxy_render::ComputeProgram,last_error:Option<String>}
impl HairLinearSolver for Baseline {
 fn solve(&mut self,_:&[HairLinearSystem])->Result<Vec<Vec<f64>>,&'static str>{Err("structural solve not owned by baseline")}
 fn joint_contact_expanded_enabled(&self)->bool{true}
 fn solve_joint_expanded_batch(&mut self,requests:&[R<'_>])->Option<Vec<(Vec<C>,Vec<f64>)>> {
  let run=||->Result<Vec<(Vec<C>,Vec<f64>)>,Box<dyn std::error::Error>> {
   let inputs=requests.iter().map(|r| {
    let columns=r.columns.iter().map(|c|c.iter().map(|v|v.parts()).collect()).collect::<Vec<Vec<_>>>();
    voxy_render::ExperimentalTripleActiveContactInput::new_expanded(&columns,r.bounds,r.tolerance)
   }).collect::<Result<Vec<_>,_>>()?;
   let batch=voxy_render::ExperimentalTripleActiveContactBatch::new(inputs)?;
   let job=self.program.create_job(&self.device,batch.bytes())?;
   let mut encoder=self.device.create_command_encoder(&Default::default());job.encode_step(&mut encoder,batch.workgroups())?;
   let snapshot=job.encode_snapshot(&mut encoder)?;let submission=self.queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
   self.device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None})?;
   let bytes=read.try_read()?.ok_or("missing GPU baseline readback")?;
   fn decode(v:voxy_render::ExperimentalTripleExpansion)->Result<Vec<C>,&'static str>{let(w,s)=v.into_parts();w.into_iter().map(|w|C::from_scaled_terms(w.map(|x|f64::from(f32::from_bits(x))),s)).collect()}
   batch.expanded_candidates(&bytes,false)?.into_iter().map(|candidate| {
    let(x,r)=candidate.into_parts();Ok((decode(x)?,decode(r)?.iter().map(|v|{let[h,l]=v.parts();h+l}).collect()))
   }).collect()
  };
  match run(){Ok(v)=>Some(v),Err(e)=>{self.last_error=Some(e.to_string());None}}
 }
}
fn main(){
 let args=std::env::args().collect::<Vec<_>>();assert_eq!(args.len(),3,"fixture output.json");
 let mut input=std::io::Cursor::new(std::fs::read(&args[1]).unwrap());let mut magic=[0;4];input.read_exact(&mut magic).unwrap();assert_eq!(&magic,b"VQC1");
 let rows=integer(&mut input);let width=integer(&mut input);let systems=integer(&mut input);integer(&mut input);let tolerance=scalar(&mut input);
 assert!(rows>0&&rows<=1024&&width<=65536&&systems<=512);
 let bounds=(0..rows).map(|_|scalar(&mut input)).collect::<Vec<_>>();for _ in 0..rows+rows*width{scalar(&mut input);}
 let mut requests=Vec::new();for _ in 0..systems{
  let n=integer(&mut input);let band=integer(&mut input);let lo=integer(&mut input);let hi=integer(&mut input);assert!(n<=65536&&band==9&&lo<=hi&&hi<=n);
  requests.push(HairResponseSystem{system:HairLinearSystem{band_width:band,matrix:(0..n*band).map(|_|scalar(&mut input)).collect(),rhs:(0..n).map(|_|scalar(&mut input)).collect(),active:lo..hi},loads:(0..rows).map(|_|(0..n).map(|_|scalar(&mut input)).collect()).collect()});
 }
 assert_eq!(input.position() as usize,input.get_ref().len());
 let instance=voxy_render::GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
 let(device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
 let program=pollster::block_on(voxy_render::ComputeProgram::new(&device,&voxy_render::ExperimentalTripleActiveContactBatch::shader_source())).unwrap();
 let mut backend=Baseline{device,queue,program,last_error:None};
 let(responses,reactions,used)=HairResponseSystem::solve_joint_load_inequalities_expanded(&requests,&bounds,tolerance,&mut backend).unwrap();
 HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
 let active=reactions.iter().enumerate().filter_map(|(i,&v)|(v>0.).then_some(i)).collect::<Vec<_>>();assert!(!active.is_empty());
 let report=serde_json::json!({"scope":"Baseline active row selection validated against all original physical constraints; not five-component QR or trajectory proof","rows":rows,"width":width,"systems":systems,"active_rows":active,"original_bounds_bits":bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"baseline_reactions_bits":reactions.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"tolerance_bits":tolerance.to_bits(),"baseline_used_gpu":used,"baseline_gpu_error":backend.last_error,"original_physical_admission":true});
 let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&args[2]).unwrap();file.write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes()).unwrap();
 println!("original_rows={rows} width={width} active_rows={} original_physical_admission=true baseline_used_gpu={used}",active.len());
}
