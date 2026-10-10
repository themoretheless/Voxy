//! Resident GPU equality backend under physics-owned active-set selection.
use super::*;
use voxy_render::{ResidentContactEqualityInput,JOINT_CONTACT_QR_SHADER,JOINT_CONTACT_EQUALITY_SHADER};
// Evaluate equality defects against original f64 columns. Compensated
// products retain cancellation in coupled contact loads.
fn original_products(pairs:impl Iterator<Item=(f64,f64)>)->f64 {
    let mut sum=0f64;let mut correction=0f64;
    for (a,b) in pairs {
        let product=a*b;let next=sum+product;
        correction+=if sum.abs()>=product.abs() {(sum-next)+product} else {(product-next)+sum};
        correction+=a.mul_add(b,-product);sum=next;
    }
    sum+correction
}
fn original_support(column:&[f64])->Vec<usize> {
    column.iter().enumerate().filter_map(|(i,&v)|(v!=0.).then_some(i)).collect()
}
// One exact-input GPU result, bounded independently of scene size. Physical
// ownership still validates the returned candidate against original loads.
#[derive(Debug)]
pub(super) struct JointResultCache {
    columns:Vec<Vec<f64>>, bounds:Vec<f64>, tolerance:u64, compact:bool,
    output:(Vec<f64>,Vec<f64>),
}
impl JointResultCache {
    const MAX_BYTES:usize=2*1024*1024;
    fn new(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,compact:bool,output:&(Vec<f64>,Vec<f64>))->Option<Self> {
        let words=columns.iter().try_fold(bounds.len(),|n,c|n.checked_add(c.len()))?
            .checked_add(output.0.len())?.checked_add(output.1.len())?;
        if words.checked_mul(8)?>Self::MAX_BYTES {return None;}
        Some(Self {columns:columns.to_vec(),bounds:bounds.to_vec(),tolerance:tolerance.to_bits(),compact,output:output.clone()})
    }
    fn matches(&self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,compact:bool)->bool {
        fn exact(a:&[f64],b:&[f64])->bool {a.len()==b.len()&&a.iter().zip(b).all(|(a,b)|a.to_bits()==b.to_bits())}
        self.tolerance==tolerance.to_bits()&&self.compact==compact&&exact(&self.bounds,bounds)
            &&self.columns.len()==columns.len()&&self.columns.iter().zip(columns).all(|(a,b)|exact(a,b))
    }
}
#[derive(Clone,Copy)]
struct EqualityOptions {
    compact:bool,prefix_reuse:bool,release:bool,early_release:bool,
    full_refinement_readback:bool,separate_qr_passes:bool,refinement_limit:usize,refinement_trace:bool,
    stage_profile:bool,
}
#[derive(Default)]
struct EqualityStageProfile {
    prepare_wall_ms:f64,encode_wall_ms:f64,submit_map_wall_ms:f64,
    wait_wall_ms:f64,decode_refine_wall_ms:f64,total_wall_ms:f64,
    readback_payload_bytes:u64,
}
impl EqualityStageProfile {
    fn start(enabled:bool)->Option<std::time::Instant> {enabled.then(std::time::Instant::now)}
    fn elapsed(start:Option<std::time::Instant>)->f64 {
        start.map_or(0.,|start|start.elapsed().as_secs_f64()*1000.)
    }
}
#[derive(Default)]
struct EqualityStats {dispatches:usize,submissions:usize,reused:usize,release:usize,early_release:usize,workspace_creations:usize,workspace_reuses:usize}
// Owned by one solver/device. Only completed jobs enter this bounded cache;
// every acquired job receives a complete fresh payload before any dispatch.
#[derive(Debug,Default)]
pub(super) struct EqualityWorkspacePool {jobs:Vec<ComputeJob>,bytes:u64,reuses:usize,creations:usize}
impl EqualityWorkspacePool {
    const MAX_BYTES:u64=8*1024*1024;
    const MAX_JOBS:usize=32;
    fn acquire(&mut self,device:&wgpu::Device,queue:&wgpu::Queue,qr:&ComputeProgram,bytes:&[u8])
        ->Result<ComputeJob,voxy_render::ComputeError> {
        if let Some((i,_))=self.jobs.iter().enumerate()
            .filter(|(_,job)|job.buffer().size()>=bytes.len() as u64)
            .min_by_key(|(_,job)|job.buffer().size()) {
            let mut job=self.jobs.swap_remove(i);self.bytes-=job.buffer().size();
            job.use_program(qr)?;
            queue.write_buffer(job.buffer(),0,bytes);self.reuses+=1;
            return Ok(job);
        }
        // Dropping cached storage on a budget error only moves its charge to
        // configured retirement. Preserve useful jobs; never silently discard
        // their reuse opportunity or pretend the charge has been released.
        let job=qr.create_job(device,bytes)?;
        self.creations+=1;Ok(job)
    }
    fn retain_completed(&mut self,job:ComputeJob) {
        let size=job.buffer().size();
        if self.jobs.len()<Self::MAX_JOBS && size<=Self::MAX_BYTES-self.bytes {
            self.bytes+=size;self.jobs.push(job);
        }
    }
}
struct EqualityTask<'a> {
    columns:&'a [Vec<f64>],bounds:&'a [f64],tolerance:f64,
    rhs:Vec<f64>,reactions:Vec<f64>,coordinates:Vec<f64>,supports:Vec<Vec<usize>>,
    job:ComputeJob,payload_bytes:u64,initial:Option<ResidentContactEqualityInput>,
    validated:Option<voxy_render::ValidatedResidentContactEquality>,
    prefix:Option<voxy_render::ResidentContactQrPrefix>,count:usize,reused:usize,refinement:usize,
}
impl<'a> EqualityTask<'a> {
    fn new(device:&wgpu::Device,queue:&wgpu::Queue,qr:&ComputeProgram,columns:&'a [Vec<f64>],bounds:&'a [f64],tolerance:f64,
        workspaces:&mut EqualityWorkspacePool,
        prefix:Option<voxy_render::ResidentContactQrPrefix>,options:EqualityOptions,stats:&mut EqualityStats)
        ->Result<Self,Box<dyn std::error::Error>> {
        let mut input=ResidentContactEqualityInput::new_with_support_compaction(columns,bounds,options.compact)?;
        let reused=if options.prefix_reuse {prefix.as_ref().map_or(0,|p|input.reuse_qr_prefix(p))} else {0};
        stats.reused+=reused;
        let payload_bytes=input.bytes().len() as u64;
        let job=workspaces.acquire(device,queue,qr,input.bytes())?;let count=input.columns();
        Ok(Self {columns,bounds,tolerance,rhs:bounds.to_vec(),reactions:vec![0.;bounds.len()],coordinates:Vec::new(),
            supports:columns.iter().map(|c|original_support(c)).collect(),job,payload_bytes,initial:Some(input),validated:None,
            prefix,count,reused,refinement:0})
    }
    fn snapshot_range(&self,options:EqualityOptions)->(u64,u64) {
        if self.refinement==0 || options.full_refinement_readback {(0,self.payload_bytes)}
        else {let (offset,bytes)=self.validated.as_ref().unwrap().equality_update();(offset,bytes.len() as u64)}
    }
    fn encode(&mut self,queue:&wgpu::Queue,solve:&ComputeProgram,
        encoder:&mut wgpu::CommandEncoder,options:EqualityOptions)->Result<(),Box<dyn std::error::Error>> {
        if self.refinement>0 {
            let input=self.validated.as_mut().unwrap();input.update_bounds(&self.rhs)?;
            let (offset,bytes)=input.equality_update();queue.write_buffer(self.job.buffer(),offset,bytes);
        } else {
            if options.separate_qr_passes {
                for _ in self.reused..self.count {self.job.encode_step(encoder,[1,1,1])?;}
            } else if self.count>self.reused {
                self.job.encode_repeated_steps(encoder,[1,1,1],u32::try_from(self.count-self.reused)?)?;
            }
            self.job.use_program(solve)?;
        }
        self.job.encode_step(encoder,[1,1,1])?;
        Ok(())
    }

    fn consume(&mut self,bytes:&[u8],options:EqualityOptions,stats:&mut EqualityStats)
        ->Result<Option<physics::hair::HairContactEqualityProposal>,Box<dyn std::error::Error>> {
        let refinement=self.refinement;let full_refinement_readback=options.full_refinement_readback;
        let refinement_trace=options.refinement_trace;
        let columns=self.columns;let bounds=self.bounds;let tolerance=self.tolerance;
        let supports=&self.supports;let reactions=&mut self.reactions;let coordinates=&mut self.coordinates;
        let rhs:Vec<f64>;
                    let output=if refinement==0 {
                        let (state,output)=self.initial.take().unwrap().into_validated(&bytes).map_err(|error| {
                            let header:Vec<_>=bytes.iter().copied().take(16).collect::<Vec<_>>().chunks_exact(4)
                                .map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect();
                            let status=bytes.get(bytes.len().saturating_sub(4)..).filter(|b|b.len()==4)
                                .map(|b|u32::from_le_bytes(b.try_into().unwrap()));
                            format!("GPU joint QR/equality publication rejected: {error}; refinement={refinement} header={header:?} equality_status={status:?}")
                        })?;
                        // Bound retained snapshot payload independently of scene size.
                        if options.prefix_reuse {
                            self.prefix=if bytes.len()<=2*1024*1024 {Some(state.qr_prefix(&bytes)?)} else {None};
                        }
                        self.validated=Some(state);output
                    } else {
                        let input=self.validated.as_ref().unwrap();
                        let tail=if full_refinement_readback {
                            let (offset,expected)=input.equality_update();
                            bytes.get(offset as usize..offset as usize+expected.len()).ok_or("truncated full equality snapshot")?
                        } else {&bytes};
                        input.decode_tail(tail)?
                    };
                    for (total,delta) in reactions.iter_mut().zip(output.reactions) {*total+=delta;}
                    if coordinates.is_empty() {*coordinates=output.coordinates;}
                    else {for (total,delta) in coordinates.iter_mut().zip(output.coordinates) {*total+=delta;}}
                    // Validate every coordinate before omitting exact-zero
                    // products; a nonfinite value on a zero axis must reject.
                    if coordinates.iter().any(|v|!v.is_finite()) || reactions.iter().any(|v|!v.is_finite()) {return Err("original GPU equality refinement overflow".into());}
                    // An opening dual direction only changes the active set;
                    // it cannot publish coordinates. The physics owner restores
                    // CURRENT C*lambda and retains all original final KKT gates.
                    if options.release && options.early_release
                        && reactions.iter().any(|v|*v<0.) {
                        stats.release+=1;
                        stats.early_release+=1;
                        return Ok(Some(physics::hair::HairContactEqualityProposal::ReleaseDirection(std::mem::take(reactions))));
                    }
                    rhs=columns.iter().zip(supports).zip(bounds).map(|((column,ids),&bound)|
                        bound-original_products(ids.iter().map(|&i|(column[i],coordinates[i])))).collect();
                    if rhs.iter().any(|v|!v.is_finite()) {return Err("original GPU equality refinement overflow".into());}
                    // Physical feasibility alone can accept a small equality
                    // defect which an ill-conditioned operator amplifies into
                    // visible motion. Refine to original f64 backward accuracy
                    // as well; this never increases the physical tolerance.
                    let precise=columns.iter().zip(supports).zip(bounds).zip(&rhs).all(|(((column,ids),&bound),&defect)| {
                        let magnitude=bound.abs()+original_products(ids.iter().map(|&i|(column[i].abs(),coordinates[i].abs())));
                        let numerical_limit=32.*f64::EPSILON*magnitude.max(f64::MIN_POSITIVE);
                        magnitude.is_finite() && defect.abs()<=tolerance.min(numerical_limit)
                    });
                    if refinement_trace {
                        let (row,defect)=rhs.iter().enumerate().max_by(|a,b|a.1.abs().total_cmp(&b.1.abs())).unwrap();
                        let minimum_reaction=reactions.iter().copied().fold(f64::INFINITY,f64::min);
                        let negative_reactions=reactions.iter().filter(|v|**v<0.).count();
                        eprintln!("JOINT EQUALITY REFINEMENT rows={} width={} refinement={refinement} worst_row={row} defect={defect:e} precise={precise} minimum_reaction={minimum_reaction:e} negative_reactions={negative_reactions}",columns.len(),coordinates.len());
                    }
        self.refinement+=1;
        if precise {return Ok(Some(physics::hair::HairContactEqualityProposal::Solution(
            std::mem::take(coordinates),std::mem::take(reactions))));}
        self.rhs=rhs;
        if self.refinement>=options.refinement_limit {
            if options.release && reactions.iter().all(|v|v.is_finite()) && reactions.iter().any(|v|*v<0.) {
                stats.release+=1;
                return Ok(Some(physics::hair::HairContactEqualityProposal::ReleaseDirection(std::mem::take(reactions))));
            }
            return Err("original GPU equality refinement did not converge".into());
        }
        Ok(None)
    }
}
// Every ready independent task is encoded before one submission and one wait.
// This owner stays on the calling thread; jobs retain independent storage.
fn run_equality_tasks(device:&wgpu::Device,queue:&wgpu::Queue,qr:&ComputeProgram,solve:&ComputeProgram,
    requests:&[physics::hair::HairContactEqualityRequest<'_>],
    prefixes:&mut [Option<voxy_render::ResidentContactQrPrefix>],workspaces:&mut EqualityWorkspacePool,options:EqualityOptions,stats:&mut EqualityStats)
    ->Vec<Result<physics::hair::HairContactEqualityProposal,Box<dyn std::error::Error>>> {
    let total_started=EqualityStageProfile::start(options.stage_profile);
    let mut profile=EqualityStageProfile::default();
    let dispatch_before=(stats.dispatches,stats.submissions);
    let workspace_before=(workspaces.creations,workspaces.reuses);
    let mut results:Vec<_>=(0..requests.len()).map(|_|None).collect();
    let mut tasks:Vec<_>=requests.iter().enumerate().map(|(i,r)| {
        match EqualityTask::new(device,queue,qr,r.columns,r.bounds,r.tolerance,workspaces,prefixes[i].take(),options,stats) {
            Ok(task)=>Some(task),Err(error)=>{results[i]=Some(Err(error));None}
        }
    }).collect();
    profile.prepare_wall_ms=EqualityStageProfile::elapsed(total_started);
    while tasks.iter().any(Option::is_some) {
        // Each ready owner advances once per round. Split storage pressure
        // before encoding; retrying an already encoded QR step would corrupt
        // its resident phase and dropping an unconfirmed copy quarantines it.
        let mut cursor=0;
        while cursor<tasks.len() {
            let encode_started=EqualityStageProfile::start(options.stage_profile);
            let capacity=voxy_render::ComputeReadbackPool::for_device(device).available_capacity();
            let mut remaining_bytes=capacity.max_bytes;
            let mut encoder=device.create_command_encoder(&Default::default());
            let mut dispatches=Vec::new();
            while cursor<tasks.len() {
                let i=cursor;
                if let Some(task)=tasks[i].as_mut() {
                    let (source_offset,size)=task.snapshot_range(options);
                    if size>remaining_bytes || capacity.max_buffers==0 {
                        if !dispatches.is_empty() {break;}
                        results[i]=Some(Err(format!("GPU equality readback capacity: requested_bytes={size} available={capacity:?}").into()));
                        cursor+=1;continue;
                    }
                    match task.encode(queue,solve,&mut encoder,options) {
                        Ok(())=>{
                            dispatches.push((i,source_offset,capacity.max_bytes-remaining_bytes,size));
                            remaining_bytes-=size;
                        },
                        Err(error)=>{results[i]=Some(Err(error));},
                    }
                }
                cursor+=1;
            }
            for i in 0..tasks.len() {if results[i].is_some() {if let Some(task)=tasks[i].take() {prefixes[i]=task.prefix;}}}
            if dispatches.is_empty() {
                profile.encode_wall_ms+=EqualityStageProfile::elapsed(encode_started);continue;
            }
            // Independent resident jobs share one contiguous staging lease.
            // No owner can overwrite another's storage; slices are consumed in
            // the original ready order after the one submission completes.
            let ranges:Vec<_>=dispatches.iter().map(|&(i,source,target,size)|
                (tasks[i].as_ref().unwrap().job.buffer(),source,target,size)).collect();
            let payload_bytes=capacity.max_bytes-remaining_bytes;
            let gathered=voxy_render::ComputeDispatch::gather_buffers(device,&mut encoder,&ranges,payload_bytes);
            drop(ranges);
            let dispatch=match gathered {
                Ok(dispatch)=>dispatch,
                Err(error)=>{
                    let message=error.to_string();
                    for &(i,_,_,_) in &dispatches {
                        results[i]=Some(Err(message.clone().into()));
                        prefixes[i]=tasks[i].take().unwrap().prefix;
                    }
                    profile.encode_wall_ms+=EqualityStageProfile::elapsed(encode_started);
                    continue;
                }
            };
            stats.dispatches+=dispatches.len();
            if options.stage_profile {profile.readback_payload_bytes=profile.readback_payload_bytes.saturating_add(payload_bytes);}
            let commands=encoder.finish();
            profile.encode_wall_ms+=EqualityStageProfile::elapsed(encode_started);
            let submit_started=EqualityStageProfile::start(options.stage_profile);
            let submission=queue.submit([commands]);stats.submissions+=1;
            let mut read=dispatch.begin_read();
            profile.submit_map_wall_ms+=EqualityStageProfile::elapsed(submit_started);
            let wait_started=EqualityStageProfile::start(options.stage_profile);
            let polled=device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None});
            profile.wait_wall_ms+=EqualityStageProfile::elapsed(wait_started);
            let decode_started=EqualityStageProfile::start(options.stage_profile);
            let bytes=if let Err(error)=&polled {Err(error.to_string())} else {
                read.try_read().map_err(|error|error.to_string())
                    .and_then(|bytes|bytes.ok_or_else(||"pending GPU joint contact readback".to_owned()))
            };
            for (i,_,target,size) in dispatches {
                let result=match &bytes {
                    Ok(bytes)=>tasks[i].as_mut().unwrap().consume(&bytes[target as usize..(target+size) as usize],options,stats),
                    Err(error)=>Err(error.clone().into()),
                };
                match result {
                    Ok(None)=>{},Ok(Some(proposal))=>results[i]=Some(Ok(proposal)),Err(error)=>results[i]=Some(Err(error)),
                }
                if results[i].is_some() {
                    let task=tasks[i].take().unwrap();prefixes[i]=task.prefix;
                    if polled.is_ok() {workspaces.retain_completed(task.job);}
                }
            }
            profile.decode_refine_wall_ms+=EqualityStageProfile::elapsed(decode_started);
        }
    }
    stats.workspace_creations+=workspaces.creations-workspace_before.0;
    stats.workspace_reuses+=workspaces.reuses-workspace_before.1;
    if options.stage_profile {
        profile.total_wall_ms=EqualityStageProfile::elapsed(total_started);
        eprintln!("HAIR JOINT STAGE PROFILE {}",serde_json::json!({
            "schema":"voxy-joint-host-stage-profile-v1","scope":"equality_tasks_host_wall_times_not_gpu_timestamps_or_rendered_fps",
            "owners":requests.len(),"dispatches":stats.dispatches-dispatch_before.0,
            "submissions":stats.submissions-dispatch_before.1,
            "workspace_creations":workspaces.creations-workspace_before.0,
            "workspace_reuses":workspaces.reuses-workspace_before.1,
            "errors":results.iter().filter(|result|matches!(result,Some(Err(_)))).count(),
            "stages":{
                "prepare_wall_ms":profile.prepare_wall_ms,"encode_wall_ms":profile.encode_wall_ms,
                "submit_map_wall_ms":profile.submit_map_wall_ms,"wait_wall_ms":profile.wait_wall_ms,
                "decode_refine_wall_ms":profile.decode_refine_wall_ms,"total_wall_ms":profile.total_wall_ms,
                "readback_payload_bytes":profile.readback_payload_bytes
            }
        }));
    }
    results.into_iter().map(|r|r.expect("every equality task completed or failed")).collect()
}

impl GpuHairLinearSolver {
    /// Cooperative GPU coordinate solving. Original physical island admission
    /// remains the caller's responsibility; no partial batch is published.
    pub(super) fn solve_joint_batch_checked(&mut self,requests:&[physics::hair::HairContactCoordinateRequest<'_>])
        ->Result<Option<Vec<(Vec<f64>,Vec<f64>)>>,Box<dyn std::error::Error>> {
        if !self.joint_contact_qr {return Ok(None);}
        if self.joint_programs.is_none() {
            let qr=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_QR_SHADER))?;
            let solve=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_EQUALITY_SHADER))?;
            self.joint_programs=Some((qr,solve));
        }
        let (qr,solve)=self.joint_programs.as_ref().unwrap();
        #[cfg(test)] let full_refinement_readback=self.joint_full_refinement_readback;
        #[cfg(not(test))] let full_refinement_readback=true;
        #[cfg(test)] let separate_qr_passes=self.joint_separate_qr_passes;
        #[cfg(not(test))] let separate_qr_passes=false;
        #[cfg(test)] let refinement_limit=std::env::var("VOXY_HAIR_JOINT_REFINEMENT_LIMIT").ok().map(|v|v.parse::<usize>().unwrap()).unwrap_or(8);
        #[cfg(not(test))] let refinement_limit=8;
        assert!((1..=256).contains(&refinement_limit));
        let options=EqualityOptions {compact:self.joint_support_compaction,prefix_reuse:self.joint_qr_prefix_reuse,
            release:self.joint_release_directions,early_release:self.joint_early_release_directions,
            full_refinement_readback,separate_qr_passes,refinement_limit,
            refinement_trace:std::env::var_os("VOXY_HAIR_JOINT_REFINEMENT_TRACE").is_some(),
            stage_profile:std::env::var_os("VOXY_HAIR_JOINT_STAGE_PROFILE").is_some()};
        let requests:Vec<_>=requests.iter().map(|r|physics::hair::HairContactCoordinateRequest {
            columns:r.columns,bounds:r.bounds,tolerance:r.tolerance,seeds:if self.joint_dual_hints {r.seeds} else {&[]}
        }).collect();
        self.joint_dual_hint_attempts+=requests.iter().filter(|r|!r.seeds.is_empty()).count();
        let mut prefixes:Vec<_>=(0..requests.len()).map(|_|None).collect();let mut error=None;
        let mut workspaces=std::mem::take(&mut self.joint_workspaces);
        let output=HairResponseSystem::solve_contact_coordinate_batch_with_proposals(&requests,|ready| {
            let mut selected:Vec<_>=ready.iter().map(|r|prefixes[r.operator_index].take()).collect();
            let mut stats=EqualityStats::default();
            let results=run_equality_tasks(&self.device,&self.queue,qr,solve,ready,&mut selected,&mut workspaces,options,&mut stats);
            for (r,prefix) in ready.iter().zip(selected) {prefixes[r.operator_index]=prefix;}
            self.joint_workspace_creations+=stats.workspace_creations;self.joint_workspace_reuses+=stats.workspace_reuses;
            self.joint_equality_dispatches+=stats.dispatches;self.joint_equality_submissions+=stats.submissions;
            self.joint_qr_columns_reused+=stats.reused;self.joint_release_direction_calls+=stats.release;
            self.joint_early_release_direction_calls+=stats.early_release;
            Some(results.into_iter().map(|r|match r {Ok(p)=>Some(p),Err(e)=>{error=Some(e);None}}).collect())
        });
        self.joint_workspaces=workspaces;
        if output.is_none() {if let Some(error)=error {return Err(error);}}
        Ok(output)
    }
    pub(super) fn solve_joint_seeded_checked(&mut self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64])
        ->Result<Option<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        if !self.joint_contact_qr {return Ok(None);}
        if !self.joint_result_reuse {self.joint_result_cache=None;}
        if let Some(cached)=&self.joint_result_cache {
            if cached.matches(columns,bounds,tolerance,self.joint_support_compaction) {
                self.joint_result_cache_hits+=1;
                return Ok(Some(cached.output.clone()));
            }
        }
        if self.joint_programs.is_none() {
            let qr=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_QR_SHADER))?;
            let solve=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_EQUALITY_SHADER))?;
            self.joint_programs=Some((qr,solve));
        }
        let (qr,solve)=self.joint_programs.as_ref().unwrap();let mut error=None;
        #[cfg(test)]
        let full_refinement_readback=self.joint_full_refinement_readback;
        #[cfg(not(test))]
        // Partial staging copies did not improve the measured Metal fixture.
        // Keep full transfers until a representative benchmark proves otherwise.
        let full_refinement_readback=true;
        #[cfg(test)]
        let separate_qr_passes=self.joint_separate_qr_passes;
        #[cfg(not(test))]
        let separate_qr_passes=false;
        let seeds=if self.joint_dual_hints {seeds} else {&[]};
        if !seeds.is_empty() {self.joint_dual_hint_attempts+=1;}
        // Prefix factors never leave this coordinate owner's active-set solve.
        let mut qr_prefix=None;
        let mut workspaces=std::mem::take(&mut self.joint_workspaces);
        let original_columns=columns;
        let refinement_trace=std::env::var_os("VOXY_HAIR_JOINT_REFINEMENT_TRACE").is_some();
        let stage_profile=std::env::var_os("VOXY_HAIR_JOINT_STAGE_PROFILE").is_some();
        #[cfg(test)]
        let refinement_limit=std::env::var("VOXY_HAIR_JOINT_REFINEMENT_LIMIT").ok().map(|v|v.parse::<usize>().unwrap()).unwrap_or(8);
        #[cfg(not(test))]
        let refinement_limit=8;
        assert!((1..=256).contains(&refinement_limit));
        let output=HairResponseSystem::solve_contact_coordinates_with_proposals(columns,bounds,tolerance,seeds,
            |columns,bounds,tolerance| {
                let result=(||->Result<_,Box<dyn std::error::Error>> {
                    if refinement_trace {
                        let rows:Vec<_>=columns.iter().map(|column|original_columns.iter().position(|original|original==column)).collect();
                        eprintln!("JOINT EQUALITY SELECTED original_rows={rows:?}");
                    }
                    let request=physics::hair::HairContactEqualityRequest {operator_index:0,columns,bounds,tolerance};
                    let options=EqualityOptions {compact:self.joint_support_compaction,prefix_reuse:self.joint_qr_prefix_reuse,
                        release:self.joint_release_directions,early_release:self.joint_early_release_directions,
                        full_refinement_readback,separate_qr_passes,refinement_limit,refinement_trace,stage_profile};
                    let mut stats=EqualityStats::default();let mut prefixes=[qr_prefix.take()];
                    let output=run_equality_tasks(&self.device,&self.queue,qr,solve,&[request],&mut prefixes,&mut workspaces,options,&mut stats)
                        .pop().unwrap();
                    qr_prefix=prefixes[0].take();
                    self.joint_workspace_creations+=stats.workspace_creations;self.joint_workspace_reuses+=stats.workspace_reuses;
                    self.joint_equality_dispatches+=stats.dispatches;self.joint_equality_submissions+=stats.submissions;self.joint_qr_columns_reused+=stats.reused;
                    self.joint_release_direction_calls+=stats.release;self.joint_early_release_direction_calls+=stats.early_release;
                    output

                })();
                match result {Ok(output)=>Some(output),Err(reason)=>{error=Some(reason);None}}
            });
        self.joint_workspaces=workspaces;
        // A rejected hint may be followed by a successful canonical retry.
        if output.is_none() {
            if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_FAILURE_COORDINATES_EXPORT") {
                // Exact float bits preserve signed zero, subnormals and invalid
                // hints. create_new prevents overwriting earlier failure proof.
                if let Ok(file)=std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
                    let report=serde_json::json!({"schema":"voxy-joint-coordinate-failure-v1",
                        "columns_bits":columns.iter().map(|c|c.iter().map(|v|v.to_bits()).collect::<Vec<_>>()).collect::<Vec<_>>(),
                        "bounds_bits":bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                        "seeds_bits":seeds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                        "tolerance_bits":tolerance.to_bits(),"support_compaction":self.joint_support_compaction,
                        "qr_prefix_reuse":self.joint_qr_prefix_reuse,"release_directions":self.joint_release_directions,"early_release_directions":self.joint_early_release_directions,"reason":error.as_ref().map(|e|e.to_string())});
                    let _=serde_json::to_writer(std::io::BufWriter::new(file),&report);
                }
            }
            if let Some(error)=error {return Err(error);}
        }
        if output.is_none() {self.last_error=Some("GPU joint active contacts did not converge".into());}
        self.joint_result_cache=if self.joint_result_reuse {
            output.as_ref().and_then(|output|JointResultCache::new(columns,bounds,tolerance,self.joint_support_compaction,output))
        } else {None};
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    #[ignore = "requires GPU; solver-owned workspace reuse across changed physical inputs"]
    fn gpu_solver_workspaces_survive_calls_without_retaining_numerical_inputs() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;gpu.joint_support_compaction=false;
        let a=vec![vec![2.,0.,0.,0.],vec![0.,4.,0.,0.]];
        let b=vec![vec![0.,2.,0.,0.],vec![4.,0.,0.,0.]];
        let mut fresh=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        fresh.joint_contact_qr=true;fresh.joint_support_compaction=false;
        let changed_expected=fresh.solve_joint_seeded_checked(&b,&[6.,8.],1e-12,&[]).unwrap().unwrap();
        let first=gpu.solve_joint_seeded_checked(&a,&[2.,8.],1e-12,&[]).unwrap().unwrap();
        assert_eq!(first,(vec![1.,2.,0.,0.],vec![0.5,0.5]));
        let creations=gpu.joint_workspace_creations;
        for _ in 0..3 {
            let changed=gpu.solve_joint_seeded_checked(&b,&[6.,8.],1e-12,&[]).unwrap().unwrap();
            assert_eq!(changed,changed_expected,"retained workspace changed fresh GPU result bits");
            assert!((2.*changed.0[1]-6.).abs()<=1e-12);
            assert!((4.*changed.0[0]-8.).abs()<=1e-12);
            assert_eq!(gpu.solve_joint_seeded_checked(&a,&[2.,8.],1e-12,&[]).unwrap().unwrap(),first);
        }
        assert_eq!(gpu.joint_workspace_creations,creations,"warm compatible calls allocated new GPU storage");
        assert!(gpu.joint_workspace_reuses>0);
        let before=(gpu.joint_workspaces.jobs.len(),gpu.joint_workspaces.bytes,gpu.joint_equality_submissions);
        assert!(gpu.solve_joint_seeded_checked(&[vec![f64::NAN;4]],&[1.],1e-12,&[]).unwrap().is_none());
        assert_eq!((gpu.joint_workspaces.jobs.len(),gpu.joint_workspaces.bytes,gpu.joint_equality_submissions),before,
            "invalid input consumed or replaced completed workspaces");
        gpu.clear_joint_workspace_cache();
        assert!(gpu.joint_workspaces.jobs.is_empty());assert_eq!(gpu.joint_workspaces.bytes,0);
        assert_eq!(gpu.solve_joint_seeded_checked(&a,&[2.,8.],1e-12,&[]).unwrap().unwrap(),first);
        assert!(gpu.joint_workspace_creations>creations);
    }
    #[test]
    #[ignore = "requires GPU; persistent solver cache under exact managed storage budget"]
    fn gpu_solver_cache_budget_rejection_and_explicit_retirement_preserve_ownership() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let large=vec![vec![2.,0.,0.,0.,0.,0.,0.]];
        let small=vec![vec![2.,0.]];
        let oversized=vec![vec![2.;11]];
        let bytes=ResidentContactEqualityInput::new_with_support_compaction(&large,&[3.],false).unwrap().bytes().len() as u64;
        let budget=voxy_render::ComputeMemoryBudget::configure(&device,bytes).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;gpu.joint_support_compaction=false;
        assert!(gpu.solve_joint_seeded_checked(&large,&[3.],1e-12,&[]).unwrap().is_some());
        assert_eq!(gpu.solve_joint_seeded_checked(&small,&[4.],1e-12,&[]).unwrap().unwrap(),
            (vec![2.,0.],vec![1.]));
        let before=(budget.stats(),gpu.joint_workspaces.bytes,gpu.joint_workspaces.jobs.len(),gpu.joint_equality_submissions);
        let error=gpu.solve_joint_seeded_checked(&oversized,&[1.],1e-12,&[]).unwrap_err();
        assert!(error.to_string().contains("MemoryBudget"),"{error}");
        assert_eq!((budget.stats(),gpu.joint_workspaces.bytes,gpu.joint_workspaces.jobs.len(),gpu.joint_equality_submissions),before);
        assert_eq!(gpu.solve_joint_seeded_checked(&small,&[6.],1e-12,&[]).unwrap().unwrap(),
            (vec![3.,0.],vec![1.5]));
        assert_eq!(gpu.joint_workspace_creations,1);
        gpu.clear_joint_workspace_cache();
        assert_eq!(budget.stats().allocated_bytes,bytes);
        assert_eq!(budget.stats().retired_buffers,1);
        let mut retirement=budget.begin_retirement(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert_eq!(budget.stats().allocated_bytes,bytes,"unobserved completion released managed charge");
        assert!(retirement.try_finish().unwrap());assert_eq!(budget.stats().allocated_bytes,0);
        assert_eq!(gpu.solve_joint_seeded_checked(&small,&[4.],1e-12,&[]).unwrap().unwrap(),
            (vec![2.,0.],vec![1.]));
        assert_eq!(gpu.joint_workspace_creations,2);
        assert_eq!(voxy_render::ComputeReadbackPool::for_device(&device).stats().quarantined_buffers,0);
    }
    #[test]
    #[ignore = "requires GPU; strict storage budget, oversized backing and explicit retirement"]
    fn gpu_workspace_reuses_larger_storage_and_preserves_budget_after_rejection() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let large=vec![vec![2.,0.,0.,0.,0.,0.,0.]];
        let small=vec![vec![2.,0.]];
        let oversized=vec![vec![2.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.]];
        let size=ResidentContactEqualityInput::new_with_support_compaction(&large,&[3.],false).unwrap().bytes().len() as u64;
        let budget=voxy_render::ComputeMemoryBudget::configure(&device,size).unwrap();
        let qr=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER)).unwrap();
        let solve=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER)).unwrap();
        let mut pool=EqualityWorkspacePool::default();
        for full_readback in [true,false] {
            let options=EqualityOptions {compact:false,prefix_reuse:false,release:false,early_release:false,
                full_refinement_readback:full_readback,separate_qr_passes:false,refinement_limit:8,refinement_trace:false,stage_profile:false};
            let run=|pool:&mut EqualityWorkspacePool,columns:&[Vec<f64>],bound:f64| {
                let bounds=[bound];
                let request=physics::hair::HairContactEqualityRequest {operator_index:0,columns,bounds:&bounds,tolerance:1e-12};
                run_equality_tasks(&device,&queue,&qr,&solve,&[request],&mut [None],pool,options,&mut EqualityStats::default())
                    .pop().unwrap()
            };
            for (columns,bound) in [(&large,3.),(&small,4.)] {
                let physics::hair::HairContactEqualityProposal::Solution(coordinates,reactions)=run(&mut pool,columns,bound).unwrap()
                    else {panic!("unexpected release direction")};
                let mut expected=vec![0.;columns[0].len()];expected[0]=bound/2.;
                assert_eq!(coordinates,expected);assert_eq!(reactions,vec![bound/4.]);
                assert_eq!(budget.stats().allocated_bytes,size);
                assert_eq!(budget.stats().retired_buffers,0);
            }
            let before=(pool.bytes,pool.jobs.len(),pool.creations,pool.reuses,budget.stats());
            let error=run(&mut pool,&oversized,1.).unwrap_err();
            assert!(error.to_string().contains("MemoryBudget"),"{error}");
            assert_eq!((pool.bytes,pool.jobs.len(),pool.creations,pool.reuses,budget.stats()),before);
            assert!(run(&mut pool,&small,6.).is_ok(),"rejection lost usable storage");
        }
        assert_eq!(pool.creations,1);assert_eq!(pool.reuses,5);
        assert_eq!(voxy_render::ComputeReadbackPool::for_device(&device).stats().quarantined_buffers,0);
        drop(pool);
        assert_eq!(budget.stats().allocated_bytes,size);
        assert_eq!(budget.stats().retired_buffers,1);
        let mut retirement=budget.begin_retirement(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert_eq!(budget.stats().allocated_bytes,size,"charge released without observing retirement");
        assert!(retirement.try_finish().unwrap());
        assert_eq!(budget.stats().allocated_bytes,0);
        eprintln!("GPU WORKSPACE STRICT BUDGET bytes={size} creations=1 reuses=5 oversized_rejection_preserves_pool=true oversized_backing_uses_payload_snapshot=true explicit_retirement=true");
    }
    #[test]
    #[ignore = "requires GPU; reused equality storage must reset the full operator"]
    fn gpu_equality_workspace_resets_changed_columns_bounds_and_rejected_input() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let qr=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER)).unwrap();
        let solve=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER)).unwrap();
        let options=EqualityOptions {compact:false,prefix_reuse:false,release:false,early_release:false,
            full_refinement_readback:true,separate_qr_passes:false,refinement_limit:8,refinement_trace:false,stage_profile:false};
        let run=|pool:&mut EqualityWorkspacePool,columns:&[Vec<f64>],bounds:&[f64]| {
            let request=physics::hair::HairContactEqualityRequest {operator_index:0,columns,bounds,tolerance:1e-12};
            let result=run_equality_tasks(&device,&queue,&qr,&solve,&[request],&mut [None],pool,options,&mut EqualityStats::default())
                .pop().unwrap()?;
            let physics::hair::HairContactEqualityProposal::Solution(coordinates,reactions)=result else {panic!("unexpected release direction")};
            Ok::<_,Box<dyn std::error::Error>>((coordinates,reactions))
        };
        let snapshot=|pool:&EqualityWorkspacePool| {
            assert_eq!(pool.jobs.len(),1);
            let mut encoder=device.create_command_encoder(&Default::default());
            let dispatch=pool.jobs[0].encode_snapshot(&mut encoder).unwrap();
            let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
            device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
            read.try_read().unwrap().unwrap()
        };
        let mut reused=EqualityWorkspacePool::default();
        for case in 0..24 {
            let columns=vec![vec![1.+case as f64*0.01,0.03],vec![-0.02,1.3-case as f64*0.005]];
            let bounds=[0.7+case as f64*0.01,1.1-case as f64*0.002];
            let mut fresh=EqualityWorkspacePool::default();
            let expected=run(&mut fresh,&columns,&bounds).unwrap();
            let actual=run(&mut reused,&columns,&bounds).unwrap();
            assert_eq!(actual,expected);
            assert_eq!(snapshot(&reused),snapshot(&fresh),"stale storage at case {case}");
            let before=(reused.jobs.len(),reused.bytes,reused.reuses,reused.creations);
            assert!(run(&mut reused,&columns,&[f64::NAN,1.]).is_err());
            assert_eq!((reused.jobs.len(),reused.bytes,reused.reuses,reused.creations),before);
        }
        assert_eq!(reused.creations,1);assert_eq!(reused.reuses,23);
        // Equal byte counts can have different QR/equality layouts. The size
        // key must never stand in for operator identity or retained contents.
        let mut layouts=EqualityWorkspacePool::default();
        let cases=[(vec![vec![1.,0.2,0.,0.,0.,0.,0.]],vec![0.7]),
            (vec![vec![1.,0.03,0.],vec![-0.02,1.2,0.]],vec![0.7,1.1])];
        for (columns,bounds) in cases {
            let mut fresh=EqualityWorkspacePool::default();
            assert_eq!(run(&mut layouts,&columns,&bounds).unwrap(),run(&mut fresh,&columns,&bounds).unwrap());
            assert_eq!(snapshot(&layouts),snapshot(&fresh),"equal-size layout was not fully reset");
        }
        assert_eq!(layouts.creations,1);assert_eq!(layouts.reuses,1);
        drop(layouts);
        let pool=voxy_render::ComputeReadbackPool::for_device(&device);
        assert_eq!(pool.stats().quarantined_buffers,0);
        let before=voxy_render::ComputeMemoryBudget::for_device(&device).stats().allocated_bytes;
        assert!(before>=reused.bytes);
        let retained_bytes=reused.bytes;drop(reused);
        assert_eq!(voxy_render::ComputeMemoryBudget::for_device(&device).stats().allocated_bytes,before-retained_bytes);
        eprintln!("GPU EQUALITY WORKSPACE changed_operators=24 exact_full_storage=true creations=1 reuses=23 equal_size_different_layouts=true invalid_input_preserves_pool=true scoped_memory_released=true quarantine=0");
    }
    #[test]
    #[ignore = "requires actual GPU; bounded staging across many independent owners"]
    fn gpu_equality_batch_splits_readback_capacity_without_changing_results() {
        let columns=vec![vec![1.,0.],vec![0.,1.]];
        let bounds=[1.,2.];
        let snapshot_bytes=ResidentContactEqualityInput::new_with_support_compaction(&columns,&bounds,true).unwrap().bytes().len() as u64;
        for byte_limited in [false,true] {
            let instance=voxy_render::GraphicsOptions::default().create_instance();
            let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
            let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
            let pool=voxy_render::ComputeReadbackPool::configure(&device,voxy_render::ComputeReadbackLimits {
                max_bytes:if byte_limited {snapshot_bytes*2} else {snapshot_bytes*65},
                max_buffers:if byte_limited {8} else {1},
            }).unwrap();
            let qr=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER)).unwrap();
            let solve=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER)).unwrap();
            let options=EqualityOptions {compact:true,prefix_reuse:true,release:true,early_release:true,
                full_refinement_readback:true,separate_qr_passes:false,refinement_limit:8,refinement_trace:false,stage_profile:false};
            let request=physics::hair::HairContactEqualityRequest {operator_index:0,columns:&columns,bounds:&bounds,tolerance:1e-12};
            let solution=|proposal|match proposal {
                physics::hair::HairContactEqualityProposal::Solution(coordinates,reactions)=>(coordinates,reactions),
                physics::hair::HairContactEqualityProposal::ReleaseDirection(_)=>panic!("positive identity operator returned a release direction"),
            };
            let expected=solution(run_equality_tasks(&device,&queue,&qr,&solve,&[request],&mut [None],&mut EqualityWorkspacePool::default(),options,&mut EqualityStats::default()).pop().unwrap().unwrap());
            let requests:Vec<_>=(0..65).map(|i|physics::hair::HairContactEqualityRequest {
                operator_index:i,columns:&columns,bounds:&bounds,tolerance:1e-12}).collect();
            let mut workspaces=EqualityWorkspacePool::default();
            for _ in 0..2 {
                let mut prefixes=(0..requests.len()).map(|_|None).collect::<Vec<_>>();
                let mut stats=EqualityStats::default();
                let results=run_equality_tasks(&device,&queue,&qr,&solve,&requests,&mut prefixes,&mut workspaces,options,&mut stats);
                for result in results {assert_eq!(solution(result.unwrap()),expected);}
                assert_eq!(stats.dispatches,65);assert_eq!(stats.submissions,if byte_limited {33} else {1});
                assert_eq!(pool.stats().quarantined_buffers,0);
                assert_eq!(pool.available_capacity(),pool.limits());
                assert!(pool.stats().allocated_bytes<=pool.limits().max_bytes);
                assert!(pool.stats().allocated_buffers<=pool.limits().max_buffers);
            }
            assert!(workspaces.reuses>0);
            assert!(workspaces.bytes<=EqualityWorkspacePool::MAX_BYTES);
            assert!(workspaces.jobs.len()<=EqualityWorkspacePool::MAX_JOBS);
            eprintln!("GPU BOUNDED EQUALITY byte_limited={byte_limited} owners=65 repeated=2 exact_serial=true quarantine=0 stats={:?}",pool.stats());
        }
    }
    #[test]
    #[ignore = "requires actual GPU physical island batching"]
    fn gpu_independent_physical_islands_batch_without_state_drift() {
        use physics::hair::{HairRod,HairMaterial,HairSystem,RootPose,TriangleMesh};
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut serial_solver=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        let mut batch_solver=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        for solver in [&mut serial_solver,&mut batch_solver] {
            solver.joint_contact_qr=true;solver.joint_qr_prefix_reuse=true;
            solver.joint_release_directions=true;solver.joint_early_release_directions=true;
            solver.residual_refinements=1;solver.contact_response_batches=true;
        }
        batch_solver.joint_coordinate_batches=true;
        let curves:Vec<Vec<_>>=(0..4).map(|rod|(0..=8).map(|point|
            [rod as f64*0.02,0.00061,point as f64*0.01]).collect()).collect();
        let roots:Vec<_>=curves.iter().map(|curve|RootPose {position:curve[0],rotation:[0.,0.,0.,1.]}).collect();
        let rods=curves.into_iter().map(|curve|HairRod::new(curve,HairMaterial::default()).unwrap()).collect();
        let mut serial=HairSystem::new(rods).unwrap();
        serial.self_collision=false;serial.iterations=3;serial.substeps=1;
        serial.contact_radius=0.0006;serial.joint_contact_positions=true;serial.recover_friction_pressure=true;
        serial.joint_contact_velocities=true;
        let mut batch=serial.clone();
        let floor=TriangleMesh::new(&[[-1.,0.,-1.],[1.,0.,-1.],[1.,0.,1.],[-1.,0.,1.]],&[[0,2,1],[0,3,2]]).unwrap();
        for frame in 1..=4 {
            physics::hair::observe_contact_frame(frame,|| {
            serial.step_with_solver(1./240.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut serial_solver)
                .unwrap_or_else(|error|panic!("serial physical scene: {error}; {:?}",serial_solver.last_error));
            batch.step_with_solver(1./240.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut batch_solver)
                .unwrap_or_else(|error|panic!("batch physical scene: {error}; {:?}",batch_solver.last_error));
            });
            for (a,b) in serial.rods().iter().zip(batch.rods()) {
                assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
            }
        }
        assert!(batch_solver.joint_coordinate_calls>0,"fixture never exercised contact coordinates");
        assert!(batch_solver.joint_equality_submissions<batch_solver.joint_equality_dispatches,
            "physical islands never shared a GPU submission");
        assert_eq!(serial_solver.joint_native_fallbacks,0);assert_eq!(batch_solver.joint_native_fallbacks,0);
        eprintln!("JOINT PHYSICAL ISLAND GPU BATCH frames=4 guides=4 equality_dispatches={} submissions={} exact_serial_pose_and_rotation=true native_fallbacks=0 scope=small_physical_scene_not_full_model_or_fps",batch_solver.joint_equality_dispatches,batch_solver.joint_equality_submissions);
    }
    #[test]
    #[ignore = "requires an actual GPU and VOXY_HAIR_JOINT_FAILURE_FIXTURE exact captured coordinate input"]
    fn replay_rejected_gpu_joint_coordinates() {
        let path=std::env::var_os("VOXY_HAIR_JOINT_FAILURE_FIXTURE").expect("missing failure fixture");
        let report:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(report["schema"],"voxy-joint-coordinate-failure-v1");
        let decode=|values:&serde_json::Value|values.as_array().unwrap().iter()
            .map(|v|f64::from_bits(v.as_u64().unwrap())).collect::<Vec<_>>();
        let columns:Vec<_>=report["columns_bits"].as_array().unwrap().iter().map(decode).collect();
        let bounds=decode(&report["bounds_bits"]);let seeds=decode(&report["seeds_bits"]);
        let tolerance=f64::from_bits(report["tolerance_bits"].as_u64().unwrap());
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("JOINT FAILURE REPLAY ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;gpu.joint_dual_hints=true;
        gpu.joint_support_compaction=report["support_compaction"].as_bool().unwrap();
        gpu.joint_qr_prefix_reuse=report["qr_prefix_reuse"].as_bool().unwrap();
        gpu.joint_release_directions=report["release_directions"].as_bool().unwrap_or(false) || std::env::var_os("VOXY_HAIR_JOINT_RELEASE_DIRECTIONS").is_some();
        gpu.joint_early_release_directions=report["early_release_directions"].as_bool().unwrap_or(false) || std::env::var_os("VOXY_HAIR_JOINT_EARLY_RELEASE_DIRECTIONS").is_some();
        let seeds=if std::env::var_os("VOXY_HAIR_JOINT_REPLAY_COLD").is_some() {&[][..]} else {&seeds};
        let result=gpu.solve_joint_seeded_checked(&columns,&bounds,tolerance,seeds);
        eprintln!("JOINT FAILURE REPLAY rows={} width={} seeds={} submissions={} error={:?}",columns.len(),columns[0].len(),seeds.len(),gpu.joint_equality_dispatches,result.as_ref().err());
        let (coordinates,reactions)=result.unwrap().expect("coordinate active set rejected captured input");
        assert!(coordinates.iter().all(|v|v.is_finite()));
        assert!(reactions.iter().all(|v|v.is_finite()&&*v>=0.));
        for ((column,&bound),&reaction) in columns.iter().zip(&bounds).zip(&reactions) {
            let gap=original_products(column.iter().copied().zip(coordinates.iter().copied()))-bound;
            assert!(if reaction>0. {gap.abs()<=tolerance} else {gap>=-tolerance});
        }
        eprintln!("JOINT FAILURE REPLAY coordinate_gates=true release_directions={} original_physical_publication_still_required=true",gpu.joint_release_direction_calls);
    }

    #[test]
    #[ignore = "requires an actual GPU; seeded QR retry and current-operator admission"]
    fn gpu_seeded_joint_releases_and_retries_dependent_hints() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        assert!(!gpu.joint_dual_hints);
        gpu.joint_contact_qr=true;
        gpu.joint_dual_hints=true;
        let columns=vec![vec![1.,0.],vec![0.,1.]];
        let cold=gpu.solve_joint_coordinates(&columns,&[1.,2.],1e-12).unwrap();
        let before=gpu.joint_equality_dispatches;
        let seeded=gpu.solve_joint_coordinates_seeded(&columns,&[1.,2.],1e-12,&cold.1).unwrap();
        assert_eq!(cold,seeded);
        assert_eq!(gpu.joint_equality_dispatches-before,1);
        let released=gpu.solve_joint_coordinates_seeded(&columns,&[1.,-2.],1e-12,&cold.1).unwrap();
        assert_eq!(released,(vec![1.,0.],vec![1.,0.]));
        let dependent=vec![vec![1.],vec![1.]];
        let result=gpu.solve_joint_coordinates_seeded(&dependent,&[1.,1.],1e-12,&[1.,1.]).unwrap();
        assert_eq!(result.0,vec![1.]);
        assert_eq!(result.1.iter().sum::<f64>(),1.);
        assert!(gpu.last_error.is_none(),"rejected seed poisoned successful cold retry");
        assert_eq!(gpu.joint_native_fallbacks,0);
        assert_eq!(gpu.joint_dual_hint_attempts,3);
        gpu.joint_release_directions=true;gpu.joint_early_release_directions=true;gpu.joint_qr_prefix_reuse=true;
        let negative=[1.,-2.];let positive=[1.,2.];let equal=[1.,1.];let seed=[1.,2.];let dependent_seed=[1.,1.];
        let requests=[physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&negative,tolerance:1e-12,seeds:&seed},
            physics::hair::HairContactCoordinateRequest {columns:&dependent,bounds:&equal,tolerance:1e-12,seeds:&dependent_seed},
            physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&positive,tolerance:1e-12,seeds:&[]}];
        let expected:Vec<_>=requests.iter().map(|r|gpu.solve_joint_coordinates_seeded(r.columns,r.bounds,r.tolerance,r.seeds).unwrap()).collect();
        let before=(gpu.joint_coordinate_calls,gpu.joint_equality_dispatches,gpu.joint_equality_submissions);
        let actual=gpu.solve_joint_coordinates_batch(&requests).unwrap();
        assert_eq!(actual,expected);
        assert_eq!(gpu.joint_coordinate_calls-before.0,requests.len());
        assert!(gpu.joint_equality_submissions-before.2<gpu.joint_equality_dispatches-before.1);
        assert!(gpu.last_error.is_none(),"failed hinted task poisoned successful batch retry");
        assert_eq!(gpu.joint_native_fallbacks,0);
        eprintln!("JOINT HETEROGENEOUS GPU BATCH seeded_release=true dependent_hint_cold_retry=true exact_serial_match=true");

    }

    #[test]
    fn original_support_products_match_dense_cancellation_and_tiny_terms() {
        let column=[0.,1e16,-0.,1.,-1e16,f64::from_bits(1),1e-300];
        let values=[1.,1.,-1.,1.,1.,1e300,1e200];
        let ids=original_support(&column);
        assert_eq!(ids,vec![1,3,4,5,6]);
        assert_eq!(original_products(column.iter().copied().zip(values)).to_bits(),
            original_products(ids.iter().map(|&i|(column[i],values[i]))).to_bits());
        let mut seed=0x19e9_7cc1_2a84_61d3u64;
        let mut next=|| {seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;seed};
        for _ in 0..256 {
            let mut column=Vec::new();let mut values=Vec::new();
            for _ in 0..512 {
                for target in [&mut column,&mut values] {
                    let bits=next();let exponent=623+(bits%801);
                    let value=f64::from_bits((bits&(1u64<<63))|(exponent<<52)|(bits&((1u64<<52)-1)));
                    target.push(if bits%4!=0 {0f64.copysign(value)} else {value});
                }
            }
            let ids=original_support(&column);
            for absolute in [false,true] {
                let product=|a:f64,b:f64|if absolute {(a.abs(),b.abs())} else {(a,b)};
                let dense=original_products(column.iter().zip(&values).map(|(&a,&b)|product(a,b)));
                let sparse=original_products(ids.iter().map(|&i|product(column[i],values[i])));
                assert!(dense.is_finite());assert_eq!(dense.to_bits(),sparse.to_bits());
            }
        }
    }
    #[test]
    fn joint_result_cache_requires_exact_operator_and_is_bounded() {
        let columns=vec![vec![1.,0.],vec![0.,1.]];
        let bounds=vec![1.,2.];let output=(bounds.clone(),bounds.clone());
        let cache=JointResultCache::new(&columns,&bounds,1e-14,true,&output).unwrap();
        assert!(cache.matches(&columns,&bounds,1e-14,true));
        let mut changed=columns.clone();changed[0][0]=f64::from_bits(1f64.to_bits()+1);
        assert!(!cache.matches(&changed,&bounds,1e-14,true));
        changed=columns.clone();changed[0][1]=-0.;
        assert!(!cache.matches(&changed,&bounds,1e-14,true));
        assert!(!cache.matches(&columns,&[1.,3.],1e-14,true));
        assert!(!cache.matches(&columns,&bounds,2e-14,true));
        assert!(!cache.matches(&columns,&bounds,1e-14,false));
        assert!(!cache.matches(&columns[..1],&bounds,1e-14,true));
        let oversized=vec![vec![0.;JointResultCache::MAX_BYTES/8+1]];
        assert!(JointResultCache::new(&oversized,&bounds,1e-14,true,&output).is_none());
    }

    #[test]
    #[ignore = "requires Metal/Vulkan hardware and original VQC1 physical contact fixture"]
    fn gpu_joint_backend_admits_captured_original_physics() {
        let path=std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap();
        let mut input=std::io::Cursor::new(std::fs::read(path).unwrap());
        fn integer(input:&mut std::io::Cursor<Vec<u8>>)->usize {let mut b=[0;4];input.read_exact(&mut b).unwrap();u32::from_le_bytes(b) as usize}
        fn scalar(input:&mut std::io::Cursor<Vec<u8>>)->f64 {let mut b=[0;8];input.read_exact(&mut b).unwrap();f64::from_le_bytes(b)}
        let mut magic=[0;4];input.read_exact(&mut magic).unwrap();assert_eq!(&magic,b"VQC1");
        let rows=integer(&mut input);let width=integer(&mut input);let systems=integer(&mut input);let _=integer(&mut input);
        let tolerance=scalar(&mut input);assert!(rows>0&&rows<=512&&width<=65536&&systems<=512);
        let bounds:Vec<_>=(0..rows).map(|_|scalar(&mut input)).collect();
        for _ in 0..rows+rows*width {let _=scalar(&mut input);}
        let mut requests=Vec::new();
        for _ in 0..systems {
            let n=integer(&mut input);let band=integer(&mut input);let lo=integer(&mut input);let hi=integer(&mut input);
            assert!(n<=65536&&band==9&&lo<=hi&&hi<=n);
            let matrix=(0..n*band).map(|_|scalar(&mut input)).collect();let rhs=(0..n).map(|_|scalar(&mut input)).collect();
            let loads=(0..rows).map(|_|(0..n).map(|_|scalar(&mut input)).collect()).collect();
            requests.push(HairResponseSystem {system:HairLinearSystem {matrix,rhs,band_width:band,active:lo..hi},loads});
        }
        assert_eq!(input.position() as usize,input.get_ref().len());
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("JOINT BACKEND ADAPTER {:?}",adapter.get_info());
        let mut descriptor=wgpu::DeviceDescriptor::default();
        if std::env::var_os("VOXY_HAIR_TRANSFER_ONE_STORAGE").is_some() {
            descriptor.required_limits.max_storage_buffers_per_shader_stage=1;
        }
        let (device,queue)=pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        eprintln!("JOINT BACKEND storage_limit={}",device.limits().max_storage_buffers_per_shader_stage);
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();gpu.joint_contact_qr=true;
        assert!(!gpu.joint_result_reuse,"unmeasured exact reuse enabled by default");
        assert!(!gpu.joint_qr_prefix_reuse);
        gpu.joint_qr_prefix_reuse=std::env::var_os("VOXY_HAIR_JOINT_QR_PREFIX_REUSE").is_some();
        assert!(!gpu.joint_release_directions);
        assert!(!gpu.joint_early_release_directions);
        gpu.joint_release_directions=std::env::var_os("VOXY_HAIR_JOINT_RELEASE_DIRECTIONS").is_some();
        gpu.joint_early_release_directions=std::env::var_os("VOXY_HAIR_JOINT_EARLY_RELEASE_DIRECTIONS").is_some();
        gpu.joint_result_reuse=true;
        let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
            |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
        gpu.joint_contact_result(accelerated);
        if !accelerated {
            let active:Vec<_>=reactions.iter().enumerate().filter(|(_,v)|**v>0.).map(|(i,&v)|(i,v)).collect();
            eprintln!("JOINT REJECTED GPU native_recovery_active_reactions={active:?}");
        }
        assert!(accelerated,"GPU was not admitted: {:?}",gpu.last_error);
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
        if std::env::var_os("VOXY_HAIR_JOINT_NATIVE_COMPARE").is_some() {
            let (native,native_reactions)=HairResponseSystem::solve_joint_load_inequalities_native(&requests,&bounds,tolerance).unwrap();
            HairResponseSystem::validate_joint_solution(&requests,&bounds,&native,&native_reactions,tolerance).unwrap();
            let mut translation=0f64;let mut angular=0f64;
            for (a,b) in native.iter().zip(&responses) {
                for (i,(&a,&b)) in a.iter().zip(b).enumerate() {
                    if i%6<3 {translation=translation.max((a-b).abs());}
                    else {angular=angular.max((a-b).abs());}
                }
            }
            let reaction=native_reactions.iter().zip(&reactions).map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
            // The same linear operator codec also captures contact velocity
            // projection. Units come from the caller, never from matrix shape
            // or tolerance (neither identifies the physical quantity). Mixed
            // full-scene captures must remain unassigned without provenance.
            let quantity=std::env::var("VOXY_HAIR_JOINT_RESPONSE_QUANTITY")
                .unwrap_or_else(|_|"unspecified".into());
            let (linear_unit,angular_unit)=match quantity.as_str() {
                "unspecified"=>("unassigned","unassigned"),
                "displacement"=>("m","rad"),
                "velocity"=>("m/s","rad/s"),
                _=>panic!("unknown captured joint response quantity: {quantity}"),
            };
            let mut report=serde_json::json!({"scope":"same complete captured physical operator; not whole trajectory",
                "systems":systems,"rows":rows,"coordinates":width,"absolute_physical_tolerance":tolerance,
                "response_quantity":quantity,"linear_response_unit":linear_unit,"angular_response_unit":angular_unit,
                "maximum_linear_response_component_difference":translation,
                "maximum_angular_response_component_difference":angular,
                "maximum_reaction_difference":reaction,"native_fallback_used":!accelerated});
            if quantity=="displacement" {
                report["maximum_translation_difference_m"]=serde_json::json!(translation);
                report["maximum_angular_increment_difference_rad"]=serde_json::json!(angular);
            }
            if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_RESPONSE_EXPORT") {
                let data=serde_json::json!({"schema":"voxy-same-input-responses-v1",
                    "comparison":report,"native_responses":native,"gpu_responses":responses,
                    "native_reactions":native_reactions,"gpu_reactions":reactions,
                    "limits":"Original captured operator response only; global rod ordering requires input sidecar. Not a full-state trajectory replay."});
                std::fs::write(path,serde_json::to_vec_pretty(&data).unwrap()).unwrap();
            }
            eprintln!("JOINT NATIVE COMPARISON {report}");
            if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_NATIVE_COMPARE_EXPORT") {std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();}
            assert!(translation<1e-6,"captured GPU linear response differs from native: {translation} {linear_unit}");
        }
        eprintln!("JOINT GPU WORKSPACE creations={} reuses={} scope=original_physical_operator_not_full_model_or_fps",gpu.joint_workspace_creations,gpu.joint_workspace_reuses);
        assert!(gpu.joint_coordinate_calls>0&&gpu.joint_equality_dispatches>0);
        assert_eq!(gpu.joint_admitted,1);assert_eq!(gpu.joint_native_fallbacks,0);
        eprintln!("JOINT RELEASE DIRECTIONS {} early={}",gpu.joint_release_direction_calls,gpu.joint_early_release_direction_calls);
        eprintln!("JOINT BACKEND ORIGINAL systems={systems} rows={rows} width={width} tolerance={tolerance:e} coordinate_calls={} equality_dispatches={} admitted={} fallbacks={}",gpu.joint_coordinate_calls,gpu.joint_equality_dispatches,gpu.joint_admitted,gpu.joint_native_fallbacks);
        if gpu.joint_result_cache.is_some() {
            let before=gpu.joint_equality_dispatches;let hits=gpu.joint_result_cache_hits;
            let (repeat,repeat_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
                |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
            assert!(admitted);
            HairResponseSystem::validate_joint_solution(&requests,&bounds,&repeat,&repeat_reactions,tolerance).unwrap();
            assert_eq!(responses,repeat);assert_eq!(reactions,repeat_reactions);
            assert_eq!(gpu.joint_equality_dispatches,before,"identical operator resubmitted GPU work");
            assert!(gpu.joint_result_cache_hits>hits);
            eprintln!("JOINT EXACT REUSE physically_admitted=true additional_equality_submissions=0 cache_hits={}",gpu.joint_result_cache_hits-hits);
            let changed:Vec<_>=bounds.iter().map(|b|b*1.0001).collect();
            if changed.iter().zip(&bounds).any(|(a,b)|a.to_bits()!=b.to_bits()) {
                for (label,targets) in [("changed",&changed),("restored",&bounds)] {
                    let before=gpu.joint_equality_dispatches;let hits=gpu.joint_result_cache_hits;
                    let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,targets,tolerance,
                        |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
                    assert!(admitted,"{label} bounds fell back: {:?}",gpu.last_error);
                    HairResponseSystem::validate_joint_solution(&requests,targets,&current,&current_reactions,tolerance).unwrap();
                    assert!(gpu.joint_equality_dispatches>before,"{label} bounds incorrectly reused cached work");
                    assert_eq!(gpu.joint_result_cache_hits,hits);
                    if label=="restored" {assert_eq!(responses,current);assert_eq!(reactions,current_reactions);}
                    eprintln!("JOINT EXACT INVALIDATION bounds={label} physically_admitted=true equality_submissions={}",gpu.joint_equality_dispatches-before);
                }
            }
        }
        gpu.joint_result_reuse=false;
        let before=gpu.joint_equality_dispatches;let hits=gpu.joint_result_cache_hits;
        let (fresh,fresh_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
            |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
        assert!(admitted);assert!(gpu.joint_equality_dispatches>before);
        assert_eq!(gpu.joint_result_cache_hits,hits);assert!(gpu.joint_result_cache.is_none());
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&fresh,&fresh_reactions,tolerance).unwrap();
        assert_eq!(responses,fresh);assert_eq!(reactions,fresh_reactions);
        eprintln!("JOINT EXACT REUSE DISABLED physically_admitted=true retained_cache=false");
        if std::env::var_os("VOXY_HAIR_JOINT_DUAL_HINTS").is_some() {
            gpu.joint_dual_hints=true;
            let mut seeds=Vec::new();
            let changed:Vec<_>=bounds.iter().map(|b|b*1.0001).collect();
            for (label,targets) in [("cold",&bounds),("seeded",&bounds),("changed",&changed),("restored",&bounds)] {
                let before=gpu.joint_equality_dispatches;
                let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,targets,tolerance,
                    |columns,bounds,tolerance| {
                        let output=gpu.solve_joint_coordinates_seeded(columns,bounds,tolerance,&seeds);
                        if let Some((_,reactions))=&output {seeds.clone_from(reactions);}
                        output
                    }).unwrap();
                assert!(admitted,"{label} hints fell back: {:?}",gpu.last_error);
                HairResponseSystem::validate_joint_solution(&requests,targets,&current,&current_reactions,tolerance).unwrap();
                let (reference,_)=HairResponseSystem::solve_joint_load_inequalities_native(&requests,targets,tolerance).unwrap();
                let difference=current.iter().flatten().zip(reference.iter().flatten())
                    .map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
                assert!(difference<1e-10,"hint changed physical response: {difference}");
                eprintln!("JOINT PHYSICS OWNED HINTS mode={label} physically_admitted=true equality_submissions={} maximum_response_difference={difference:e}",gpu.joint_equality_dispatches-before);
            }
            gpu.joint_dual_hints=false;
        }
        if gpu.joint_qr_prefix_reuse {
            assert!(gpu.joint_qr_columns_reused>0,"fixture did not exercise prefix reuse");
            eprintln!("JOINT QR PREFIX REUSE physically_admitted=true reused_columns={}",gpu.joint_qr_columns_reused);
        }
        if std::env::var_os("VOXY_HAIR_JOINT_BATCH_COMPARE").is_some() {
            let mut captured=None;
            let (_,_,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
                |columns,bounds,tolerance| {
                    if captured.is_none() {captured=Some((columns.to_vec(),bounds.to_vec(),tolerance));}
                    gpu.solve_joint_coordinates(columns,bounds,tolerance)
                }).unwrap();
            assert!(admitted);
            let (columns,original,tolerance)=captured.unwrap();
            let targets:Vec<Vec<f64>>=[1.,1.0001,0.9,0.].into_iter()
                .map(|scale|original.iter().map(|b|b*scale).collect()).collect();
            let coordinate_requests:Vec<_>=targets.iter().map(|bounds|physics::hair::HairContactCoordinateRequest {
                columns:&columns,bounds,tolerance,seeds:&[]
            }).collect();
            let expected:Vec<_>=coordinate_requests.iter().map(|r|
                gpu.solve_joint_seeded_checked(r.columns,r.bounds,r.tolerance,r.seeds).unwrap().unwrap()).collect();
            let before=(gpu.joint_equality_dispatches,gpu.joint_equality_submissions);
            let actual=gpu.solve_joint_batch_checked(&coordinate_requests).unwrap().unwrap();
            assert_eq!(actual,expected,"cooperative GPU round changed coordinate/reaction bits");
            let dispatches=gpu.joint_equality_dispatches-before.0;
            let submissions=gpu.joint_equality_submissions-before.1;
            assert!(submissions<dispatches,"independent jobs were still submitted serially");
            assert_eq!(gpu.joint_native_fallbacks,0);
            eprintln!("JOINT COOPERATIVE GPU BATCH operators={} original_rows={} original_coordinates={} equality_dispatches={dispatches} submissions={submissions} exact_serial_match=true native_fallbacks=0 scope=coordinate_operators_not_full_trajectory",coordinate_requests.len(),columns.len(),columns[0].len());
            let invalid=vec![vec![f64::NAN;columns[0].len()]];
            let malformed=[physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&original,tolerance,seeds:&[]},
                physics::hair::HairContactCoordinateRequest {columns:&invalid,bounds:&[1.],tolerance,seeds:&[]}];
            let before=gpu.joint_equality_submissions;
            assert!(gpu.solve_joint_batch_checked(&malformed).unwrap().is_none());
            assert_eq!(gpu.joint_equality_submissions,before,"invalid later input submitted an earlier owner");
        }
        let tail_bench=std::env::var_os("VOXY_HAIR_JOINT_TAIL_BENCH").is_some();
        let pass_bench=std::env::var_os("VOXY_HAIR_JOINT_QR_PASS_BENCH").is_some();
        let prefix_bench=std::env::var_os("VOXY_HAIR_JOINT_QR_PREFIX_BENCH").is_some();
        assert!([pass_bench,tail_bench,prefix_bench].into_iter().filter(|v|*v).count()<=1,"select one benchmark comparison");
        if prefix_bench || pass_bench || tail_bench || std::env::var_os("VOXY_HAIR_JOINT_SUPPORT_BENCH").is_some() {
            let mut run=|compact:bool| {
                gpu.joint_support_compaction=if tail_bench || pass_bench || prefix_bench {true} else {compact};
                gpu.joint_full_refinement_readback=!tail_bench || !compact;
                gpu.joint_separate_qr_passes=pass_bench && !compact;
                gpu.joint_qr_prefix_reuse=prefix_bench && compact;
                let qr_before=gpu.joint_qr_columns_reused;
                gpu.joint_result_cache=None; // Benchmark actual GPU work in both modes.
                let before=gpu.joint_equality_dispatches;
                let started=std::time::Instant::now();
                let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
                    |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
                let elapsed=started.elapsed().as_secs_f64();
                assert!(accelerated,"benchmark fell back: {:?}",gpu.last_error);
                HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
                (elapsed,gpu.joint_equality_dispatches-before,responses,reactions,gpu.joint_qr_columns_reused-qr_before)
            };
            for compact in [false,true] {let _=run(compact);}
            let mut dense=Vec::new();let mut compact_times=Vec::new();let mut records=Vec::new();
            let mut max_response_difference=0f64;let mut max_reaction_difference=0f64;
            for pair in 0..7 {
                let order=if pair%2==0 {[false,true]} else {[true,false]};
                let first=run(order[0]);let second=run(order[1]);
                let (d,c)=if order[0] {(&second,&first)} else {(&first,&second)};
                dense.push(d.0);compact_times.push(c.0);
                if tail_bench || pass_bench || prefix_bench {
                    assert_eq!(d.1,c.1,"readback changed equality submission count");
                    assert_eq!(d.2,c.2,"readback changed original physical coordinates");
                    assert_eq!(d.3,c.3,"readback changed original physical reactions");
                }
                if prefix_bench {assert_eq!(d.4,0);assert!(c.4>0,"prefix benchmark did not reuse QR columns");}
                for (a,b) in d.2.iter().flatten().zip(c.2.iter().flatten()) {max_response_difference=max_response_difference.max((a-b).abs());}
                for (a,b) in d.3.iter().zip(&c.3) {max_reaction_difference=max_reaction_difference.max((a-b).abs());}
                records.push(serde_json::json!({"pair":pair,"dense_seconds":d.0,"compact_seconds":c.0,"dense_equality_dispatches":d.1,"compact_equality_dispatches":c.1,"baseline_qr_columns_reused":d.4,"candidate_qr_columns_reused":c.4}));
            }
            dense.sort_by(f64::total_cmp);compact_times.sort_by(f64::total_cmp);
            let mut result=serde_json::json!({"scope":"complete original physical projection, warmed shaders, seven alternating pairs; excludes device creation, fixture load, rendering and extra post-timing validation",
                "rows":rows,"coordinates":width,"systems":systems,"absolute_physical_tolerance":tolerance,"native_fallback_used":false,
                "median_dense_seconds":dense[3],"median_compact_seconds":compact_times[3],"speedup":dense[3]/compact_times[3],
                "maximum_response_difference":max_response_difference,"maximum_reaction_difference":max_reaction_difference,"pairs":records,
                "whole_animation_benchmark":false,"rendered_fps_measured":false,
                "concurrent_system_load_controlled":false});
            if tail_bench {
                let report=result.as_object_mut().unwrap();
                report.insert("comparison".into(),serde_json::json!("full snapshot versus validated tail; exact-zero support compaction enabled in both"));
                for (old,new) in [("median_dense_seconds","median_full_readback_seconds"),("median_compact_seconds","median_tail_readback_seconds")] {
                    let value=report.remove(old).unwrap();report.insert(new.into(),value);
                }
                for pair in report.get_mut("pairs").unwrap().as_array_mut().unwrap() {
                    let pair=pair.as_object_mut().unwrap();
                    for (old,new) in [("dense_seconds","full_readback_seconds"),("compact_seconds","tail_readback_seconds"),("dense_equality_dispatches","full_equality_dispatches"),("compact_equality_dispatches","tail_equality_dispatches")] {
                        let value=pair.remove(old).unwrap();pair.insert(new.into(),value);
                    }
                }
            }
            if pass_bench {
                let report=result.as_object_mut().unwrap();
                report.insert("comparison".into(),serde_json::json!("separate QR passes versus one ordered QR pass; full readback and support compaction in both"));
                for (old,new) in [("median_dense_seconds","median_separate_pass_seconds"),("median_compact_seconds","median_grouped_pass_seconds")] {
                    let value=report.remove(old).unwrap();report.insert(new.into(),value);
                }
                for pair in report.get_mut("pairs").unwrap().as_array_mut().unwrap() {
                    let pair=pair.as_object_mut().unwrap();
                    for (old,new) in [("dense_seconds","separate_pass_seconds"),("compact_seconds","grouped_pass_seconds"),("dense_equality_dispatches","separate_equality_dispatches"),("compact_equality_dispatches","grouped_equality_dispatches")] {
                        let value=pair.remove(old).unwrap();pair.insert(new.into(),value);
                    }
                }
            }
            if prefix_bench {
                let report=result.as_object_mut().unwrap();
                report.insert("comparison".into(),serde_json::json!("fresh QR versus validated identical-prefix reuse; full readback, grouped passes and support compaction in both; no dual hints"));
                for (old,new) in [("median_dense_seconds","median_fresh_qr_seconds"),("median_compact_seconds","median_prefix_qr_seconds")] {
                    let value=report.remove(old).unwrap();report.insert(new.into(),value);
                }
                for pair in report.get_mut("pairs").unwrap().as_array_mut().unwrap() {
                    let pair=pair.as_object_mut().unwrap();
                    for (old,new) in [("dense_seconds","fresh_qr_seconds"),("compact_seconds","prefix_qr_seconds"),("dense_equality_dispatches","fresh_equality_dispatches"),("compact_equality_dispatches","prefix_equality_dispatches")] {
                        let value=pair.remove(old).unwrap();pair.insert(new.into(),value);
                    }
                }
            }
            eprintln!("JOINT SUPPORT PAIRED {result}");
            if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_SUPPORT_BENCH_EXPORT") {std::fs::write(path,serde_json::to_vec_pretty(&result).unwrap()).unwrap();}
        }
    }
}
