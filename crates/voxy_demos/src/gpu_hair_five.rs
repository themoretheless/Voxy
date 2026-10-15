//! Five-component GPU route under the existing physics-owned final admission.
use super::*;
use voxy_render::{ExperimentalFiveQrInput,ExperimentalFiveQrValue,ExperimentalFiveGapInput as Gaps,ExperimentalFiveSelectionInput as Select,ExperimentalFiveSelection,ExperimentalFiveReleaseInput as Release,ExperimentalFiveRelease};
#[derive(Debug)] pub(super) struct FivePrograms {qr:ComputeProgram,gaps:ComputeProgram,select:ComputeProgram,release:ComputeProgram,pub(super) qr_submissions:usize,pub(super) qr_dispatches:usize}
impl FivePrograms {
 pub(super) async fn new(device:&wgpu::Device)->Result<Self,voxy_render::ComputeError>{Ok(Self{qr:ComputeProgram::new(device,&ExperimentalFiveQrInput::shader_source()).await?,gaps:ComputeProgram::new(device,&Gaps::shader_source()).await?,select:ComputeProgram::new(device,&Select::shader_source()).await?,release:ComputeProgram::new(device,&Release::shader_source()).await?,qr_submissions:0,qr_dispatches:0})}
 pub(super) fn solve(&mut self,device:&wgpu::Device,queue:&wgpu::Queue,request:&physics::hair::HairExpandedCoordinateRequest<'_>)->Result<(Vec<physics::hair::HairResponseCoefficient>,Vec<f64>),Box<dyn std::error::Error>>{
 let columns=request.columns.iter().map(|c|c.iter().map(|v|v.parts()).collect::<Vec<_>>()).collect::<Vec<_>>();let bounds=request.bounds;let tolerance=request.tolerance;let rows=columns.len();let width=columns.first().map_or(0,Vec::len);
 let mut active:Vec<usize>=Vec::new();let mut state=vec![zero();width];let mut reactions=vec![zero();rows];let mut need_qr=false;let mut completed=false;
 for iteration in 0..512usize.max(rows+1){

  if need_qr{
   let selected=active.iter().map(|&i|columns[i].clone()).collect::<Vec<_>>();let targets=active.iter().map(|&i|bounds[i]).collect::<Vec<_>>();
   let input=ExperimentalFiveQrInput::new(&selected,&targets,tolerance)?;self.qr_submissions+=1;self.qr_dispatches+=input.steps();let mut candidate=input.candidate(&dispatch(&device,&queue,&self.qr,input.bytes(),input.steps(),[1,1,1])?)?;
   scale(&mut candidate.coordinates,input.state_scale())?;scale(&mut candidate.reactions,input.reaction_scale())?;
   let current=active.iter().map(|&i|reactions[i].clone()).collect::<Vec<_>>();let step=Release::new_with_state(&current,&candidate.reactions,&state,&candidate.coordinates)?;
   match step.result(&dispatch(&device,&queue,&self.release,step.bytes(),1,[1,1,1])?)?{
    ExperimentalFiveRelease::Accept=>{state=candidate.coordinates;reactions.fill(zero());for(&id,value)in active.iter().zip(candidate.reactions){reactions[id]=value;}},
    ExperimentalFiveRelease::Release{row,fraction:_,reactions:updated,coordinates}=>{
     let id=active[row];state=coordinates;for(&id,value)in active.iter().zip(updated){reactions[id]=value;}active.remove(row);reactions[id]=zero();need_qr=!active.is_empty();
     continue;
    },ExperimentalFiveRelease::Uncertain=>return Err(format!("uncertain release at iteration {iteration}, active {active:?}").into()),
   }
  }
  let input=Gaps::new_with_runtime_state(&columns,&state,1.,&bounds,tolerance)?;let bytes=dispatch(&device,&queue,&self.gaps,input.bytes(),1,input.workgroups())?;let residuals=input.residuals(&bytes)?;let decisions=input.decisions(&bytes)?.iter().map(|v|*v as u32).collect::<Vec<_>>();let mut flags=vec![false;rows];for &id in &active{flags[id]=true;if decisions[id]!=3{return Err(format!("active equality not certified at row {id}, iteration {iteration}").into());}}
  let input=Select::new(&residuals,&decisions,&flags)?;
  match input.selection(&dispatch(&device,&queue,&self.select,input.bytes(),1,[1,1,1])?)?{
   ExperimentalFiveSelection::Enter(id)=>{active.push(id);active.sort_unstable();need_qr=true;},
   ExperimentalFiveSelection::Feasible=>{completed=true;break;},ExperimentalFiveSelection::Uncertain=>return Err(format!("uncertain entering row at iteration {iteration}").into()),
  }
 }
 if !completed{return Err("unchanged active-set iteration budget exhausted".into());}
 let coordinates=state.iter().map(native).collect::<Result<Vec<_>,_>>()?;
 let reactions=reactions.iter().map(|v|native(v).map(|v|{let[h,l]=v.parts();h+l})).collect::<Result<Vec<_>,_>>()?;
 Ok((coordinates,reactions))
 }
}
fn zero()->ExperimentalFiveQrValue{ExperimentalFiveQrValue{words:[0;5],exponent:0,error_bound:0.}}
fn scale(values:&mut[ExperimentalFiveQrValue],scale:f64)->Result<(),Box<dyn std::error::Error>>{if !scale.is_normal()||scale<=0.||scale.to_bits()&0x000f_ffff_ffff_ffff!=0{return Err("invalid five-component binary scale".into());}let shift=((scale.to_bits()>>52)&2047)as i32-1023;for v in values{v.exponent=v.exponent.checked_add(shift).ok_or("five-component exponent overflow")?;}Ok(())}
fn native(v:&ExperimentalFiveQrValue)->Result<physics::hair::HairResponseCoefficient,&'static str>{if !(-1022..=1023).contains(&v.exponent){return Err("five-component recovery exponent range");}physics::hair::HairResponseCoefficient::from_expansion_terms(v.words.map(|w|f64::from(f32::from_bits(w))),[1.,2f64.powi(v.exponent)])}
fn dispatch(device:&wgpu::Device,queue:&wgpu::Queue,program:&voxy_render::ComputeProgram,input:&[u8],steps:usize,groups:[u32;3])->Result<Vec<u8>,Box<dyn std::error::Error>>{
 let job=program.create_job(device,input)?;let mut encoder=device.create_command_encoder(&Default::default());for _ in 0..steps{job.encode_step(&mut encoder,groups)?;}let snapshot=job.encode_snapshot(&mut encoder)?;let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();device.poll(wgpu::PollType::Wait{submission_index:Some(submission),timeout:None})?;Ok(read.try_read()?.ok_or("missing five-component GPU readback")?)
}
