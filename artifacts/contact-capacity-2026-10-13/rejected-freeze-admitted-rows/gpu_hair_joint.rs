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
        Self::new_bounded(columns,bounds,tolerance,compact,output,Self::MAX_BYTES)
    }
    fn new_bounded(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,compact:bool,output:&(Vec<f64>,Vec<f64>),limit:usize)->Option<Self> {
        let words=columns.iter().try_fold(bounds.len(),|n,c|n.checked_add(c.len()))?
            .checked_add(output.0.len())?.checked_add(output.1.len())?;
        if words.checked_mul(8)?>limit {return None;}
        Some(Self {columns:columns.to_vec(),bounds:bounds.to_vec(),tolerance:tolerance.to_bits(),compact,output:output.clone()})
    }
    fn matches(&self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,compact:bool)->bool {
        fn exact(a:&[f64],b:&[f64])->bool {a.len()==b.len()&&a.iter().zip(b).all(|(a,b)|a.to_bits()==b.to_bits())}
        self.tolerance==tolerance.to_bits()&&self.compact==compact&&exact(&self.bounds,bounds)
            &&self.columns.len()==columns.len()&&self.columns.iter().zip(columns).all(|(a,b)|exact(a,b))
    }
}
// Complete coordinate candidates only. Original physical admission remains mandatory.
#[cfg(test)]
#[derive(Debug,Default)]
pub(super) struct ResidentCandidateCache { entries:Vec<JointResultCache>, bytes:usize }
#[cfg(test)]
impl ResidentCandidateCache {
    const MAX_BYTES:usize=16*1024*1024;
    fn get(&mut self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Option<(Vec<f64>,Vec<f64>)> {
        let i=self.entries.iter().position(|e|e.matches(columns,bounds,tolerance,false))?;
        let entry=self.entries.remove(i);let output=entry.output.clone();self.entries.push(entry);Some(output)
    }
    // Reuse only active-row guesses from a bit-exact retained column prefix.
    // Bounds/tolerance may differ: this is never an accepted-result cache hit.
    fn active_row_hint(&self,columns:&[Vec<f64>])->Option<Vec<f64>> {
        let entry=self.entries.iter().rev().filter(|e|!e.columns.is_empty()&&e.columns.len()<=columns.len()
            &&e.output.1.len()==e.columns.len()&&e.output.1.iter().any(|v|*v>0.)
            &&e.columns.iter().zip(columns).all(|(a,b)|a.len()==b.len()&&a.iter().zip(b).all(|(x,y)|x.to_bits()==y.to_bits())))
            .max_by_key(|e|e.columns.len())?;
        let mut hints=vec![0.;columns.len()];
        for (hint,&reaction) in hints.iter_mut().zip(&entry.output.1) {if reaction>0. {*hint=1.;}}
        Some(hints)
    }
    fn size(e:&JointResultCache)->usize {
        std::mem::size_of::<JointResultCache>()+e.columns.capacity()*std::mem::size_of::<Vec<f64>>()
            +8*(e.bounds.capacity()+e.output.0.capacity()+e.output.1.capacity()
                +e.columns.iter().map(|c|c.capacity()).sum::<usize>())
    }
    fn insert(&mut self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,output:&(Vec<f64>,Vec<f64>)) {
        let Some(entry)=JointResultCache::new_bounded(columns,bounds,tolerance,false,output,Self::MAX_BYTES) else {return;};
        let bytes=Self::size(&entry);if bytes>Self::MAX_BYTES {return;}
        while self.entries.len()>=32 || self.bytes+bytes>Self::MAX_BYTES {
            let old=self.entries.remove(0);self.bytes-=Self::size(&old);
        }
        self.bytes+=bytes;self.entries.push(entry);
    }
}
#[derive(Clone,Copy)]
struct EqualityOptions {
    compact:bool,prefix_reuse:bool,release:bool,early_release:bool,
    full_refinement_readback:bool,separate_qr_passes:bool,refinement_limit:usize,refinement_trace:bool,
    stage_profile:bool,resident_columns:bool,
}
#[derive(Default)]
struct EqualityStageProfile {
    prepare_wall_ms:f64,encode_wall_ms:f64,submit_map_wall_ms:f64,
    wait_wall_ms:f64,decode_refine_wall_ms:f64,total_wall_ms:f64,
    readback_payload_bytes:u64,
}
// Scope-exit diagnostics include failed batches, never interpret scope exit as
// physical admission. Disabled profiling performs no clock reads or logging.
#[cfg(test)]
struct ResidentStageProfile {
    profile:EqualityStageProfile,total:Option<std::time::Instant>,stage:usize,backend: &'static str,submissions:usize,
    stage_start:Option<std::time::Instant>,owners:usize,cached:usize,submitted:usize,
    creations:usize,reuses:usize,dispatches:usize,errors:usize,timestamp_resolve_wall_ms:f64,factor_readback_wall_ms:f64,factor_readback_bytes:u64,factor_readback_submissions:usize,
    submitted_shapes:Vec<(usize,usize)>,completed_iterations:Vec<u32>,complete_workspace_payload_bytes:u64,
    incoming_valid_seeded_owners:usize,incoming_positive_seed_rows:usize,seeded_upload_owners:usize,cached_active_hint_owners:usize,factor_reuse_owners:usize,factor_cache_entries:usize,factor_cache_bytes:usize,
}
#[cfg(test)]
impl ResidentStageProfile {
    fn new(owners:usize)->Self {
        let enabled=std::env::var_os("VOXY_HAIR_JOINT_STAGE_PROFILE").is_some();
        Self {profile:Default::default(),total:EqualityStageProfile::start(enabled),stage:0,backend:"complete_resident_active_set",submissions:0,
            stage_start:EqualityStageProfile::start(enabled),owners,cached:0,submitted:0,
            creations:0,reuses:0,dispatches:0,errors:owners,timestamp_resolve_wall_ms:0.,factor_readback_wall_ms:0.,factor_readback_bytes:0,factor_readback_submissions:0,submitted_shapes:Vec::new(),completed_iterations:Vec::new(),complete_workspace_payload_bytes:0,incoming_valid_seeded_owners:0,incoming_positive_seed_rows:0,seeded_upload_owners:0,cached_active_hint_owners:0,factor_reuse_owners:0,factor_cache_entries:0,factor_cache_bytes:0}
    }
    fn next(&mut self,stage:usize) {
        let elapsed=EqualityStageProfile::elapsed(self.stage_start.take());
        match self.stage {0=>self.profile.prepare_wall_ms+=elapsed,1=>self.profile.encode_wall_ms+=elapsed,
            2=>self.profile.submit_map_wall_ms+=elapsed,3=>self.profile.wait_wall_ms+=elapsed,
            _=>self.profile.decode_refine_wall_ms+=elapsed}
        self.stage=stage;self.stage_start=EqualityStageProfile::start(self.total.is_some());
    }
}
#[cfg(test)]
impl Drop for ResidentStageProfile {
    fn drop(&mut self) {
        if self.total.is_none() {return;}
        self.next(self.stage);self.profile.total_wall_ms=EqualityStageProfile::elapsed(self.total);
        eprintln!("HAIR JOINT STAGE PROFILE {}",serde_json::json!({
            "schema":"voxy-joint-host-stage-profile-v1",
            "scope":"equality_tasks_host_wall_times_not_gpu_timestamps_or_rendered_fps",
            "backend":self.backend,"owners":self.owners,
            "gpu_timestamp_profile_overhead_wall_ms":self.timestamp_resolve_wall_ms,
            "completed_factor_readback_wall_ms":self.factor_readback_wall_ms,"completed_factor_readback_bytes":self.factor_readback_bytes,"completed_factor_readback_submissions":self.factor_readback_submissions,
            "cached_owners":self.cached,"submitted_owners":self.submitted,
            "submitted_operator_shapes_rows_coordinates":self.submitted_shapes,
            "completed_active_set_iterations":self.completed_iterations,
            "complete_workspace_payload_bytes":self.complete_workspace_payload_bytes,
            "incoming_valid_seeded_owners":self.incoming_valid_seeded_owners,
            "incoming_positive_seed_rows":self.incoming_positive_seed_rows,
            "seeded_upload_owners":self.seeded_upload_owners,"cached_active_hint_owners":self.cached_active_hint_owners,
            "factor_reuse_owners":self.factor_reuse_owners,"factor_cache_entries":self.factor_cache_entries,"factor_cache_bytes":self.factor_cache_bytes,
            "dispatches":self.dispatches,"submissions":if self.backend=="triple_active_set" {self.submissions} else {self.dispatches},
            "workspace_creations":self.creations,"workspace_reuses":self.reuses,"errors":self.errors,
            "stages":{"prepare_wall_ms":self.profile.prepare_wall_ms,"encode_wall_ms":self.profile.encode_wall_ms,
                "submit_map_wall_ms":self.profile.submit_map_wall_ms,"wait_wall_ms":self.profile.wait_wall_ms,
                "decode_refine_wall_ms":self.profile.decode_refine_wall_ms,"total_wall_ms":self.profile.total_wall_ms,
                "readback_payload_bytes":self.profile.readback_payload_bytes}
        }));
    }
}
fn experimental_resident_columns()->bool {
    #[cfg(test)] {std::env::var_os("VOXY_HAIR_JOINT_QR_RESIDENT_COLUMNS").is_some()
        && std::env::var_os("VOXY_HAIR_JOINT_QR_SHADER_FIXTURE").is_some()}
    #[cfg(not(test))] {false}
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
pub(super) struct EqualityWorkspacePool {jobs:Vec<ComputeJob>,bytes:u64,reuses:usize,creations:usize,
    #[cfg(test)] geometric_growth:bool,
    #[cfg(test)] pub(super) prefer_large_completed:bool}
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
        #[cfg(test)] let growth=self.geometric_growth;
        #[cfg(not(test))] let growth=false;
        let limits=device.limits();
        let capacity=if growth {bytes.len().checked_next_power_of_two().filter(|&n|
            n as u64<=Self::MAX_BYTES && n as u64<=limits.max_buffer_size
                && n as u64<=limits.max_storage_buffer_binding_size).unwrap_or(bytes.len())} else {bytes.len()};
        let job=if capacity>bytes.len() {
            let mut padded=vec![0;capacity];padded[..bytes.len()].copy_from_slice(bytes);
            match qr.create_job(device,&padded) {
                Ok(job)=>job,
                // Exact-size admission remains available under a strict budget;
                // a failed reservation neither drops cached jobs nor dispatches.
                Err(voxy_render::ComputeError::MemoryBudget)=>qr.create_job(device,bytes)?,
                Err(error)=>return Err(error),
            }
        } else {qr.create_job(device,bytes)?};
        self.creations+=1;Ok(job)
    }
    fn retain_completed(&mut self,job:ComputeJob) {
        let size=job.buffer().size();
        #[cfg(test)]
        if self.prefer_large_completed && size<=Self::MAX_BYTES {
            let smaller_count=self.jobs.iter().filter(|old|old.buffer().size()<size).count();
            let reclaimable:u64=self.jobs.iter().map(|old|old.buffer().size()).filter(|&old|old<size).sum();
            // Plan before retiring anything: smaller jobs may be insufficient
            // while larger retained jobs occupy the pool. Preserve that cache.
            let can_fit=size<=Self::MAX_BYTES-(self.bytes-reclaimable)
                && self.jobs.len()-smaller_count<Self::MAX_JOBS;
            while can_fit && (self.jobs.len()>=Self::MAX_JOBS || size>Self::MAX_BYTES-self.bytes) {
                let Some((index,old_size))=self.jobs.iter().enumerate()
                    .map(|(i,old)|(i,old.buffer().size())).filter(|(_,old)|*old<size)
                    .min_by_key(|(_,old)|*old) else {break;};
                self.bytes-=old_size;
                // Cache eviction retires storage through its existing owner.
                // It does not release the device-wide accounting charge here.
                drop(self.jobs.swap_remove(index));
            }
        }
        if self.jobs.len()<Self::MAX_JOBS && size<=Self::MAX_BYTES-self.bytes {
            self.bytes+=size;self.jobs.push(job);
        }
    }
}
struct EqualityTask<'a> {
    columns:&'a [Vec<f64>],bounds:&'a [f64],tolerance:f64,
    rhs:Vec<f64>,reactions:Vec<f64>,coordinates:Vec<f64>,supports:Option<Vec<Vec<usize>>>,
    job:ComputeJob,payload_bytes:u64,timestamp_supported:bool,initial:Option<ResidentContactEqualityInput>,
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
            supports:None,job,payload_bytes,
            timestamp_supported:device.features().contains(wgpu::Features::TIMESTAMP_QUERY),initial:Some(input),validated:None,
            prefix,count,reused,refinement:0})
    }
    fn snapshot_range(&self,options:EqualityOptions)->(u64,u64) {
        if self.refinement==0 || options.full_refinement_readback {(0,self.payload_bytes)}
        else {let (offset,bytes)=self.validated.as_ref().unwrap().equality_update();(offset,bytes.len() as u64)}
    }
    fn encode(&mut self,queue:&wgpu::Queue,solve:&ComputeProgram,
        encoder:&mut wgpu::CommandEncoder,options:EqualityOptions)->Result<(),Box<dyn std::error::Error>> {
        self.encode_with_timestamps(queue,solve,encoder,options,None,None)
    }
    fn encode_with_timestamps(&mut self,queue:&wgpu::Queue,solve:&ComputeProgram,
        encoder:&mut wgpu::CommandEncoder,options:EqualityOptions,
        qr_timestamps:Option<wgpu::ComputePassTimestampWrites<'_>>,
        equality_timestamps:Option<wgpu::ComputePassTimestampWrites<'_>>,
    )->Result<(),Box<dyn std::error::Error>> {
        if options.resident_columns && options.separate_qr_passes {
            return Err("resident-column QR requires one grouped dispatch".into());
        }
        if (qr_timestamps.is_some() || equality_timestamps.is_some()) && !self.timestamp_supported {
            return Err(voxy_render::ComputeError::Unsupported.into());
        }
        if qr_timestamps.is_some() && (self.refinement>0 || self.count==self.reused || options.separate_qr_passes) {
            return Err("QR timestamps require a nonempty initial grouped factorization".into());
        }
        if self.refinement>0 {
            let input=self.validated.as_mut().unwrap();input.update_bounds(&self.rhs)?;
            let (offset,bytes)=input.equality_update();queue.write_buffer(self.job.buffer(),offset,bytes);
        } else {
            if options.separate_qr_passes {
                for _ in self.reused..self.count {self.job.encode_step(encoder,[1,1,1])?;}
            } else if self.count>self.reused {
                let repetitions=if options.resident_columns {1} else {u32::try_from(self.count-self.reused)?};
                if let Some(timestamps)=qr_timestamps {
                    self.job.encode_repeated_steps_with_timestamps(encoder,[1,1,1],repetitions,timestamps)?;
                } else {self.job.encode_repeated_steps(encoder,[1,1,1],repetitions)?;}
            }
            self.job.use_program(solve)?;
        }
        if let Some(timestamps)=equality_timestamps {
            self.job.encode_step_with_timestamps(encoder,[1,1,1],timestamps)?;
        } else {self.job.encode_step(encoder,[1,1,1])?;}
        Ok(())
    }

    fn consume(&mut self,bytes:&[u8],options:EqualityOptions,stats:&mut EqualityStats)
        ->Result<Option<physics::hair::HairContactEqualityProposal>,Box<dyn std::error::Error>> {
        let refinement=self.refinement;let full_refinement_readback=options.full_refinement_readback;
        let refinement_trace=options.refinement_trace;
        let columns=self.columns;let bounds=self.bounds;let tolerance=self.tolerance;
        let reactions=&mut self.reactions;let coordinates=&mut self.coordinates;
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
                    // Early-release proposals need no original-column dot
                    // products. Build exact support only for admitted-coordinate
                    // refinement, retaining it across that task's iterations.
                    let supports=self.supports.get_or_insert_with(||
                        columns.iter().map(|c|original_support(c)).collect());
                    rhs=columns.iter().zip(supports.iter()).zip(bounds).map(|((column,ids),&bound)|
                        bound-original_products(ids.iter().map(|&i|(column[i],coordinates[i])))).collect();
                    if rhs.iter().any(|v|!v.is_finite()) {return Err("original GPU equality refinement overflow".into());}
                    // Physical feasibility alone can accept a small equality
                    // defect which an ill-conditioned operator amplifies into
                    // visible motion. Refine to original f64 backward accuracy
                    // as well; this never increases the physical tolerance.
                    let precise=columns.iter().zip(supports.iter()).zip(bounds).zip(&rhs).all(|(((column,ids),&bound),&defect)| {
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
    /// Experimental complete operators share one submission. The physical owner
    /// still reconstructs and admits every result; incomplete batches fail whole.
    #[cfg(test)]
    fn solve_resident_active_batch(&mut self, requests:&[physics::hair::HairContactCoordinateRequest<'_>])
        ->Result<Vec<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        let warm=std::env::var("VOXY_HAIR_RESIDENT_WARM_START").as_deref()==Ok("1");
        let mut seeded_submission=false;
        let result=self.solve_resident_active_batch_with_seeds(requests,warm,&mut seeded_submission);
        if seeded_submission && result.is_err() {
            // Retry the complete request, never publish a partial warm batch.
            // All immutable current operators and original admission are kept.
            eprintln!("RESIDENT WARM RETRY complete_batch=true cold_retry=true owners={}",requests.len());
            self.solve_resident_active_batch_with_seeds(requests,false,&mut false)
        } else {result}
    }
    #[cfg(test)]
    fn solve_resident_active_batch_with_seeds(&mut self,requests:&[physics::hair::HairContactCoordinateRequest<'_>],warm:bool,seeded_submission:&mut bool)
        ->Result<Vec<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        let mut profile=ResidentStageProfile::new(requests.len());
        if profile.total.is_some() {
            for request in requests {
                if !request.seeds.is_empty() && request.seeds.len()==request.columns.len()
                    && request.seeds.iter().all(|v|v.is_finite()&&*v>=0.) {
                    profile.incoming_valid_seeded_owners+=1;
                    profile.incoming_positive_seed_rows+=request.seeds.iter().filter(|&&v|v>0.).count();
                }
            }
        }
        let program=self.joint_resident_active_program.as_ref().ok_or("resident program missing")?;
        let mut outputs=vec![None;requests.len()];let mut missing=Vec::new();
        let mut inputs:Vec<voxy_render::ExperimentalResidentContactInput>=Vec::new();
        let mut aliases=Vec::new();
        let mut packed_keys=std::collections::HashMap::<u64,Vec<usize>>::new();
        for (i,r) in requests.iter().enumerate() {
            if self.joint_resident_operator_reuse {
                if let Some(output)=self.joint_resident_candidates.get(r.columns,r.bounds,r.tolerance) {
                    outputs[i]=Some(output);self.joint_result_cache_hits+=1;profile.cached+=1;continue;
                }
            }
            let input=if warm {
                voxy_render::ExperimentalResidentContactInput::new_seeded(r.columns,r.bounds,r.tolerance,r.seeds)?
            } else {voxy_render::ExperimentalResidentContactInput::new(r.columns,r.bounds,r.tolerance)?};
            if self.joint_resident_batch_dedup {
                use std::hash::{Hash,Hasher};
                let mut hasher=std::collections::hash_map::DefaultHasher::new();
                // Hash original numeric input only, not the large zero-filled
                // resident QR workspace. A mismatch is only a cache miss.
                r.columns.len().hash(&mut hasher);
                for column in r.columns {
                    column.len().hash(&mut hasher);
                    for value in column {value.to_bits().hash(&mut hasher);}
                }
                for value in r.bounds {value.to_bits().hash(&mut hasher);}
                r.tolerance.to_bits().hash(&mut hasher);
                if warm {for value in r.seeds {value.to_bits().hash(&mut hasher);}}
                let key=hasher.finish();
                // A hash only narrows the search. Complete bit-exact packed
                // inputs, including warm hints, must match before sharing work.
                if let Some(source)=packed_keys.get(&key).and_then(|ids|ids.iter().copied()
                    .find(|&j|inputs[j].bytes()==input.bytes())) {
                    aliases.push((i,missing[source]));
                    self.joint_result_cache_hits+=1;profile.cached+=1;continue;
                }
                packed_keys.entry(key).or_default().push(inputs.len());
            }
            inputs.push(input);
            if profile.total.is_some() {
                profile.seeded_upload_owners+=usize::from(inputs.last().unwrap().has_seed_hint());
                profile.submitted_shapes.push((r.columns.len(),r.columns.first().map_or(0,Vec::len)));
            }
            missing.push(i);
        }
        if inputs.is_empty() {profile.errors=0;return Ok(outputs.into_iter().map(Option::unwrap).collect());}
        profile.submitted=inputs.len();
        let has_seeded_input=inputs.iter().any(voxy_render::ExperimentalResidentContactInput::has_seed_hint);
        let batch=voxy_render::ExperimentalResidentContactBatch::new(inputs)?;
        profile.complete_workspace_payload_bytes=batch.bytes().len() as u64;
        let job=if self.joint_resident_workspace_reuse {
            let created=self.joint_workspaces.creations;let reused=self.joint_workspaces.reuses;
            let result=self.joint_workspaces.acquire(&self.device,&self.queue,program,batch.bytes());
            self.joint_workspace_creations+=self.joint_workspaces.creations-created;
            self.joint_workspace_reuses+=self.joint_workspaces.reuses-reused;
            profile.creations=self.joint_workspaces.creations-created;profile.reuses=self.joint_workspaces.reuses-reused;
            result?
        } else {
            let job=program.create_job(&self.device,batch.bytes())?;
            self.joint_workspace_creations+=1;profile.creations=1;job
        };
        profile.next(1);
        let mut encoder=self.device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder,batch.workgroups())?;
        // Reused storage can exceed the current complete batch. Copy every
        // audited byte of this batch, excluding only unused backing capacity.
        #[cfg(test)] let compact_readback=std::env::var("VOXY_HAIR_RESIDENT_COMPACT_READBACK").as_deref()==Ok("1");
        #[cfg(not(test))] let compact_readback=false;
        let mut readback_bytes=batch.bytes().len() as u64;
        let snapshot=if compact_readback {
            let mut size=0;
            let ranges=batch.candidate_readback_ranges().into_iter().map(|range| {
                let length=range.end-range.start;let target=size;size+=length;
                (range.start,target,length)
            }).collect::<Vec<_>>();
            readback_bytes=size;
            voxy_render::ComputeDispatch::gather_buffer(&self.device,&mut encoder,job.buffer(),&ranges,size)?
        } else {voxy_render::ComputeDispatch::copy_buffer(&self.device,&mut encoder,
            job.buffer(),0,readback_bytes)?};
        let command=encoder.finish();profile.next(2);
        let submission=self.queue.submit([command]);profile.dispatches=1;
        *seeded_submission=has_seeded_input;
        profile.profile.readback_payload_bytes=readback_bytes;
        self.joint_equality_submissions+=1;
        // One dispatch launches one independent workgroup per complete operator.
        self.joint_equality_dispatches+=1;
        let mut read=snapshot.begin_read();profile.next(3);
        self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None})?;
        profile.next(4);
        let bytes=read.try_read()?.ok_or("resident active-set readback pending")?;
        // A reused allocation may be larger than this complete payload. The
        // fresh header bounds every workgroup; unused backing is never addressed.
        let result=if compact_readback {batch.candidate_from_compact_readback(&bytes)?}
            else {batch.candidate(bytes.get(..batch.bytes().len()).ok_or("short resident readback")?)?};
        if self.joint_resident_workspace_reuse {self.joint_workspaces.retain_completed(job);}
        for (i,output) in missing.into_iter().zip(result) {
            if self.joint_resident_operator_reuse {
                let r=&requests[i];self.joint_resident_candidates.insert(r.columns,r.bounds,r.tolerance,&output);
            }
            outputs[i]=Some(output);
        }
        for (target,source) in aliases {
            outputs[target]=Some(outputs[source].as_ref().ok_or("missing shared resident candidate")?.clone());
        }
        profile.errors=0;
        Ok(outputs.into_iter().map(Option::unwrap).collect())
    }
    #[cfg(test)]
    fn solve_triple_active_batch(&mut self,requests:&[physics::hair::HairContactCoordinateRequest<'_>])
        ->Result<Vec<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        self.solve_triple_active_batch_attempt(requests,true)
    }
    #[cfg(test)]
    fn solve_triple_active_batch_attempt(&mut self,requests:&[physics::hair::HairContactCoordinateRequest<'_>],allow_cached_hints:bool)
        ->Result<Vec<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        let mut profile=ResidentStageProfile::new(requests.len());profile.backend="triple_active_set";
        let batched=std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1");
        profile.factor_cache_entries=self.joint_triple_factors.len();
        profile.factor_cache_bytes=self.joint_triple_factors.iter().map(|f|f.retained_bytes()).sum();
        if requests.is_empty() {return Err("empty triple contact batch".into());}
        if batched&&requests.len()>65535 {return Err(voxy_render::JointDotError::Capacity.into());}
        // Aggregate workspace is bounded before even packing the first operator.
        requests.iter().try_fold(if batched {(requests.len()+2)*4} else {0usize},|sum,r| {
            let size=voxy_render::ExperimentalTripleActiveContactInput::required_bytes(
                r.columns.len(),r.columns.first().map_or(0,Vec::len))?;
            sum.checked_add(size).filter(|&bytes|bytes<=64*1024*1024).ok_or(voxy_render::JointDotError::Capacity)
        })?;
        let completed_factors=std::env::var("VOXY_HAIR_TRIPLE_COMPLETED_FACTORS").as_deref()==Ok("1");
        let warm=std::env::var("VOXY_HAIR_TRIPLE_WARM_START").as_deref()==Ok("1");
        let factor_reuse=allow_cached_hints&&warm
            &&std::env::var("VOXY_HAIR_TRIPLE_PARALLEL").as_deref()==Ok("1")
            &&std::env::var("VOXY_HAIR_TRIPLE_FACTOR_REUSE").as_deref()==Ok("1");
        let valid_hint=|r:&physics::hair::HairContactCoordinateRequest<'_>|warm&&r.seeds.len()==r.columns.len()
            &&r.seeds.iter().all(|v|v.is_finite()&&*v>=0.)&&r.seeds.iter().any(|v|*v>0.);
        let cache_hints_enabled=allow_cached_hints&&warm&&self.joint_resident_operator_reuse
            &&std::env::var("VOXY_HAIR_TRIPLE_CACHE_HINTS").as_deref()==Ok("1");
        let cached_hints=requests.iter().map(|r| {
            if cache_hints_enabled&&!valid_hint(r) {self.joint_resident_candidates.active_row_hint(r.columns)} else {None}
        }).collect::<Vec<_>>();
        let hint=|i:usize|if valid_hint(&requests[i]) {requests[i].seeds} else {cached_hints[i].as_deref().unwrap_or(&[])};
        let inputs=requests.iter().enumerate().map(|(i,r)| {
            let packed=if !hint(i).is_empty() {
                if valid_hint(r) {profile.incoming_valid_seeded_owners+=1;profile.incoming_positive_seed_rows+=r.seeds.iter().filter(|v|**v>0.).count();}
                else {profile.cached_active_hint_owners+=1;}
                voxy_render::ExperimentalTripleActiveContactInput::new_seeded(r.columns,r.bounds,r.tolerance,hint(i))
            } else {voxy_render::ExperimentalTripleActiveContactInput::new(r.columns,r.bounds,r.tolerance)};
            if let Err(error)=&packed {
                if let Some(directory)=std::env::var_os("VOXY_HAIR_TRIPLE_PACKING_FAILURE_DIR") {
                    // Coordinate-only evidence: original physical loads remain native-owned.
                    // Unique create_new captures preserve earlier failures across retries.
                    static SEQUENCE:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
                    let write=||->Result<(),Box<dyn std::error::Error>> {
                        let directory=std::path::PathBuf::from(directory);std::fs::create_dir_all(&directory)?;
                        let sequence=SEQUENCE.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
                        let path=directory.join(format!("packing-{}-{sequence}.json",std::process::id()));
                        let report=serde_json::json!({"schema":"voxy-triple-packing-failure-v1","error":error.to_string(),
                            "request_index":i,"columns_bits":r.columns.iter().map(|c|c.iter().map(|v|v.to_bits()).collect::<Vec<_>>()).collect::<Vec<_>>(),
                            "bounds_bits":r.bounds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"tolerance_bits":r.tolerance.to_bits(),
                            "seeds_bits":hint(i).iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"scope":"exact_coordinate_input_not_original_physical_load_capture"});
                        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
                        serde_json::to_writer_pretty(file,&report)?;Ok(())
                    };
                    if let Err(error)=write() {eprintln!("TRIPLE PACKING FAILURE CAPTURE ERROR {error}");}
                }
            }
            packed
        }).collect::<Result<Vec<_>,_>>()?;
        let mut output=vec![None;requests.len()];let mut missing:Vec<(usize,voxy_render::ExperimentalTripleActiveContactInput)>=Vec::new();
        let mut aliases=Vec::new();let mut keys=std::collections::HashMap::<u64,Vec<usize>>::new();
        for (index,mut input) in inputs.into_iter().enumerate() {
            let request=&requests[index];
            if self.joint_resident_operator_reuse {
                if let Some(candidate)=self.joint_resident_candidates.get(request.columns,request.bounds,request.tolerance) {
                    output[index]=Some(candidate);self.joint_result_cache_hits+=1;profile.cached+=1;continue;
                }
            }
            // Canonicalize retained scratch before full-payload deduplication;
            // otherwise a cached first request cannot match an identical alias.
            let reused_factor=factor_reuse&&self.joint_triple_factors.iter().rev()
                .any(|factors|if completed_factors {input.reuse_completed_parallel_factors(factors).is_ok()} else {input.reuse_parallel_factors(factors).is_ok()});
            if self.joint_resident_batch_dedup {
                use std::hash::{Hash,Hasher};
                let mut hasher=std::collections::hash_map::DefaultHasher::new();
                request.columns.len().hash(&mut hasher);
                for column in request.columns {column.len().hash(&mut hasher);for value in column {value.to_bits().hash(&mut hasher);}}
                for value in request.bounds {value.to_bits().hash(&mut hasher);}
                request.tolerance.to_bits().hash(&mut hasher);
                // Hash collisions cannot share work: full lossless payload must match.
                // Complete payload includes active-row hints, so different
                // hinted attempts never share a dispatch.
                let key=hasher.finish();
                if let Some(source)=keys.get(&key).and_then(|ids|ids.iter().copied().find(|&j|missing[j].1.bytes()==input.bytes())) {
                    aliases.push((index,missing[source].0));profile.cached+=1;continue;
                }
                keys.entry(key).or_default().push(missing.len());
            }
            if reused_factor {self.joint_triple_factor_reuses+=1;profile.factor_reuse_owners+=1;}
            if !hint(index).is_empty() {profile.seeded_upload_owners+=1;}
            missing.push((index,input));
        }
        if missing.is_empty() {profile.errors=0;return Ok(output.into_iter().map(Option::unwrap).collect());}
        let seeded=missing.iter().any(|(i,_)|!hint(*i).is_empty());
        profile.submitted=missing.len();
        if profile.total.is_some() {
            profile.submitted_shapes=missing.iter().map(|(i,_)|(requests[*i].columns.len(),requests[*i].columns[0].len())).collect();
            profile.complete_workspace_payload_bytes=missing.iter().map(|(_,input)|input.bytes().len() as u64).sum();
            profile.profile.readback_payload_bytes=profile.complete_workspace_payload_bytes;
        }
        let (indices,mut singles):(Vec<_>,Vec<_>)=missing.into_iter().unzip();
        let batch=if batched {Some(voxy_render::ExperimentalTripleActiveContactBatch::new(std::mem::take(&mut singles))?)} else {None};
        let inputs=batch.as_ref().map_or(singles.as_slice(),|batch|batch.inputs());
        let missing=indices.iter().copied().zip(inputs).collect::<Vec<_>>();
        let program=self.joint_triple_active_program.as_ref().ok_or("triple contact program missing")?;
        // Complete preflight and job creation precede the only queue submission.
        let workspace_reuse=std::env::var("VOXY_HAIR_TRIPLE_WORKSPACE_REUSE").as_deref()==Ok("1");
        let compact_readback=std::env::var("VOXY_HAIR_TRIPLE_COMPACT_READBACK").as_deref()==Ok("1");
        let payloads=if let Some(batch)=&batch {vec![batch.bytes()]} else {inputs.iter().map(|input|input.bytes()).collect()};
        profile.complete_workspace_payload_bytes=payloads.iter().map(|p|p.len() as u64).sum();
        let timestamp_enabled=std::env::var("VOXY_HAIR_TRIPLE_GPU_TIMESTAMPS").as_deref()==Ok("1");
        let timestamp_queries=if timestamp_enabled {
            if !self.device.features().contains(wgpu::Features::TIMESTAMP_QUERY)
                ||payloads.len()>wgpu::QUERY_SET_MAX_QUERIES as usize/2 {return Err("triple GPU timestamp capability/capacity unavailable".into());}
            Some(self.device.create_query_set(&wgpu::QuerySetDescriptor {
                label:Some("triple contact pass timestamps"),ty:wgpu::QueryType::Timestamp,count:(2*payloads.len()) as u32}))
        } else {None};
        let mut jobs=Vec::with_capacity(payloads.len());
        for payload in &payloads {
            let job=if workspace_reuse {
                let created=self.joint_workspaces.creations;let reused=self.joint_workspaces.reuses;
                let result=self.joint_workspaces.acquire(&self.device,&self.queue,program,payload);
                profile.creations+=self.joint_workspaces.creations-created;profile.reuses+=self.joint_workspaces.reuses-reused;
                self.joint_workspace_creations+=self.joint_workspaces.creations-created;
                self.joint_workspace_reuses+=self.joint_workspaces.reuses-reused;
                result?
            } else {
                let job=program.create_job(&self.device,payload)?;
                profile.creations+=1;self.joint_workspace_creations+=1;job
            };
            jobs.push(job);
        }
        profile.next(1);
        let mut encoder=self.device.create_command_encoder(&Default::default());let mut snapshots=Vec::new();
        let mut readback_bytes=0;
        for (i,job) in jobs.iter().enumerate() {
            let workgroups=batch.as_ref().map_or([1,1,1],|b|b.workgroups());
            if let Some(queries)=&timestamp_queries {
                job.encode_step_with_timestamps(&mut encoder,workgroups,wgpu::ComputePassTimestampWrites {
                    query_set:queries,beginning_of_pass_write_index:Some((2*i) as u32),end_of_pass_write_index:Some((2*i+1) as u32)})?;
            } else {job.encode_step(&mut encoder,workgroups)?;}
            if compact_readback {
                let mut size=0;
                let source_ranges=if let Some(batch)=&batch {batch.compact_readback_ranges()} else {inputs[i].compact_readback_ranges()};
                let ranges=source_ranges.into_iter().map(|range| {
                    let offset=size;let length=range.end-range.start;size+=length;(range.start,offset,length)
                }).collect::<Vec<_>>();
                snapshots.push(voxy_render::ComputeDispatch::gather_buffer(&self.device,&mut encoder,job.buffer(),&ranges,size)?);
                readback_bytes+=size;
            } else {
                snapshots.push(voxy_render::ComputeDispatch::copy_buffer(&self.device,&mut encoder,
                    job.buffer(),0,payloads[i].len() as u64)?);
                readback_bytes+=payloads[i].len() as u64;
            }
        }
        profile.profile.readback_payload_bytes=readback_bytes;
        profile.next(2);
        let submission=self.queue.submit([encoder.finish()]);
        let mut reads=snapshots.into_iter().map(|snapshot|snapshot.begin_read()).collect::<Vec<_>>();
        self.joint_equality_dispatches+=jobs.len();self.joint_equality_submissions+=1;
        profile.dispatches=jobs.len();profile.submissions=1;profile.next(3);
        self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None})?;
        profile.next(4);
        let mut readbacks=Vec::with_capacity(reads.len());
        for read in &mut reads {readbacks.push(read.try_read()?.ok_or("triple contact readback incomplete")?);}
        let operator_bytes=if let Some(batch)=&batch {batch.operator_readbacks(&readbacks[0],compact_readback)?} else {readbacks.iter().map(|b|b.as_slice()).collect()};
        let mut decoded=Vec::new();let mut factors_to_commit=Vec::new();let mut factor_plans=Vec::new();
        for (operator_index,((index,input),bytes)) in missing.iter().zip(operator_bytes).enumerate() {
            let decoded_candidate=if compact_readback {input.candidate_from_compact_readback(&bytes)} else {input.candidate(&bytes)};
            let candidate=match decoded_candidate {
                Ok(candidate)=>candidate,
                Err(_) if seeded=> {
                    // Reject the complete hinted attempt; no decoded prefix is
                    // cached or published. A seed-free retry cannot recurse here.
                    let cold=requests.iter().map(|r|physics::hair::HairContactCoordinateRequest {
                        columns:r.columns,bounds:r.bounds,tolerance:r.tolerance,seeds:&[]}).collect::<Vec<_>>();
                    // Close the rejected attempt before retry timing starts;
                    // otherwise nested profiles double-count cold GPU wait.
                    drop(profile);
                    return self.solve_triple_active_batch_attempt(&cold,false);
                },
                Err(error)=>return Err(error.into()),
            };
            if factor_reuse {
                if completed_factors&&compact_readback {
                    if let Ok(factors)=input.parallel_factors_from_compact_readback(&bytes) {factors_to_commit.push(factors);}
                    else if let Ok(plan)=input.completed_factor_readback_plan(&bytes) {factor_plans.push((operator_index,plan));}
                } else {
                    let factors=if completed_factors {input.completed_parallel_factors(&bytes)}
                        else if compact_readback {input.parallel_factors_from_compact_readback(&bytes)} else {input.parallel_factors(&bytes)};
                    if let Ok(factors)=factors {factors_to_commit.push(factors);}
                }
            }
            decoded.push((*index,candidate));
            if profile.total.is_some() {profile.completed_iterations.push(u32::from_le_bytes(bytes[12..16].try_into()?));}
        }
        if !factor_plans.is_empty() {
            // Keep every producing allocation owned until all selected QR reads
            // complete. No result or factor prefix has been committed yet.
            profile.next(4);let started=EqualityStageProfile::start(profile.total.is_some());
            let mut encoder=self.device.create_command_encoder(&Default::default());let mut snapshots=Vec::new();
            for (operator_index,plan) in &factor_plans {
                let (job,base)=if let Some(batch)=&batch {(&jobs[0],batch.operator_offset_bytes(*operator_index).ok_or("missing factor owner")?)} else {(&jobs[*operator_index],0)};
                let mut size=0;
                let ranges=plan.ranges().iter().map(|range| {let offset=size;let length=range.end-range.start;size+=length;(base+range.start,offset,length)}).collect::<Vec<_>>();
                snapshots.push(voxy_render::ComputeDispatch::gather_buffer(&self.device,&mut encoder,job.buffer(),&ranges,size)?);
                profile.factor_readback_bytes+=size;
            }
            let submission=self.queue.submit([encoder.finish()]);profile.factor_readback_submissions=1;let mut reads=snapshots.into_iter().map(|s|s.begin_read()).collect::<Vec<_>>();
            self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None})?;
            for ((_,plan),read) in factor_plans.into_iter().zip(&mut reads) {
                let bytes=read.try_read()?.ok_or("completed factor readback incomplete")?;factors_to_commit.push(plan.factors(&bytes)?);
            }
            profile.factor_readback_submissions=1;profile.factor_readback_wall_ms=EqualityStageProfile::elapsed(started);
            profile.stage_start=EqualityStageProfile::start(profile.total.is_some());
        }
        if let Some(queries)=timestamp_queries {
            // Resolve after the producing submission completes: Metal counter
            // availability must not be inferred from encoder ordering.
            profile.next(4);
            let started=std::time::Instant::now();
            let count=(2*jobs.len()) as u32;
            let resolved=self.device.create_buffer(&wgpu::BufferDescriptor {
                label:Some("triple contact resolved timestamps"),size:u64::from(count)*8,
                usage:wgpu::BufferUsages::QUERY_RESOLVE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
            let mut encoder=self.device.create_command_encoder(&Default::default());
            encoder.resolve_query_set(&queries,0..count,&resolved,0);
            let snapshot=voxy_render::ComputeDispatch::copy_buffer(&self.device,&mut encoder,&resolved,0,u64::from(count)*8)?;
            let submission=self.queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
            self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None})?;
            let bytes=read.try_read()?.ok_or("triple timestamps incomplete")?;
            let period=f64::from(self.queue.get_timestamp_period());
            if bytes.len()!=count as usize*8||!period.is_finite()||period<=0. {return Err("invalid triple GPU timestamp data".into());}
            let mut milliseconds=Vec::new();let mut pairs=Vec::new();
            for pair in bytes.chunks_exact(16) {
                let begin=u64::from_le_bytes(pair[..8].try_into()?);let end=u64::from_le_bytes(pair[8..].try_into()?);
                if end<=begin {return Err("unwritten or nonpositive triple GPU timestamp pair".into());}
                milliseconds.push((end-begin) as f64*period/1e6);pairs.push((begin,end));
            }
            profile.timestamp_resolve_wall_ms=started.elapsed().as_secs_f64()*1000.;
            profile.stage_start=EqualityStageProfile::start(profile.total.is_some());
            eprintln!("TRIPLE GPU PASS PROFILE {}",serde_json::json!({"schema":"voxy-triple-gpu-pass-profile-v1",
                "scope":"GPU compute pass boundaries; excludes queue wait before pass, host preparation, readback, physical reconstruction and rendering; concurrent GPU activity may affect execution",
                "single_dispatch":batched,"submitted_operators":missing.len(),"compute_dispatches":jobs.len(),
                "gpu_pass_ms":milliseconds,"timestamp_pairs_begin_end":pairs,"timestamp_period_ns":period,
                "profiling_resolve_submissions":1,"profiling_resolve_wall_ms":profile.timestamp_resolve_wall_ms,
                "all_coordinate_candidates_decoded":true,"original_physical_admission":"caller_still_required"}));
        }
        if workspace_reuse {for job in jobs {self.joint_workspaces.retain_completed(job);}}
        // Factor scratch is bounded and committed only after every candidate decodes.
        // Native physical rejection clears this scratch together with result hints.
        for factors in factors_to_commit {
            let size=factors.retained_bytes();if size>16*1024*1024 {continue;}
            self.joint_triple_factors.retain(|previous|!previous.same_basis(&factors));
            while self.joint_triple_factors.len()>=32||self.joint_triple_factors.iter().map(|f|f.retained_bytes()).sum::<usize>()+size>16*1024*1024 {
                self.joint_triple_factors.remove(0);
            }
            self.joint_triple_factors.push(factors);
        }
        profile.factor_cache_entries=self.joint_triple_factors.len();
        profile.factor_cache_bytes=self.joint_triple_factors.iter().map(|f|f.retained_bytes()).sum();
        // Publish cache entries only after the complete missing batch decodes.
        for (index,candidate) in decoded {
            if self.joint_resident_operator_reuse {
                let r=&requests[index];self.joint_resident_candidates.insert(r.columns,r.bounds,r.tolerance,&candidate);
            }
            output[index]=Some(candidate);
        }
        for (target,source) in aliases {
            output[target]=Some(output[source].as_ref().ok_or("missing triple shared candidate")?.clone());
        }
        profile.errors=0;
        Ok(output.into_iter().map(Option::unwrap).collect())
    }
    /// Cooperative GPU coordinate solving. Original physical island admission
    /// remains the caller's responsibility; no partial batch is published.
    pub(super) fn solve_joint_batch_checked(&mut self,requests:&[physics::hair::HairContactCoordinateRequest<'_>])
        ->Result<Option<Vec<(Vec<f64>,Vec<f64>)>>,Box<dyn std::error::Error>> {
        if !self.joint_contact_qr {return Ok(None);}
        #[cfg(test)]
        if self.joint_triple_active_program.is_some() {return self.solve_triple_active_batch(requests).map(Some);}
        #[cfg(test)]
        if self.joint_resident_active_program.is_some() {
            return self.solve_resident_active_batch(requests).map(Some);
        }

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
        let options=EqualityOptions {resident_columns:{#[cfg(test)] {self.joint_resident_qr_columns} #[cfg(not(test))] {false}},compact:self.joint_support_compaction,prefix_reuse:self.joint_qr_prefix_reuse,
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
        #[cfg(test)]
        if self.joint_triple_active_program.is_some() {
            let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds};
            return self.solve_triple_active_batch(&[request]).map(|mut output|output.pop());
        }
        #[cfg(test)]
        if self.joint_resident_active_program.is_some() {
            let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds};
            return self.solve_resident_active_batch(&[request]).map(|mut result|result.pop());
        }

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
                    #[cfg(test)]
                    if std::env::var_os("VOXY_HAIR_JOINT_GPU_PASS_PROFILE").is_some()
                        && self.joint_profile_operator.as_ref().is_none_or(|(saved,_,_)|columns.len()>saved.len())
                        && columns.iter().try_fold(bounds.len(),|n,c|n.checked_add(c.len()))
                            .and_then(|n|n.checked_mul(8)).is_some_and(|bytes|bytes<=2*1024*1024) {
                        self.joint_profile_operator=Some((columns.to_vec(),bounds.to_vec(),tolerance));
                    }
                    let request=physics::hair::HairContactEqualityRequest {operator_index:0,columns,bounds,tolerance};
                    let options=EqualityOptions {resident_columns:{#[cfg(test)] {self.joint_resident_qr_columns} #[cfg(not(test))] {false}},compact:self.joint_support_compaction,prefix_reuse:self.joint_qr_prefix_reuse,
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
    fn gpu_large_completed_workspace_keeps_retired_storage_charged() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mib=1024*1024u64;
        let budget=voxy_render::ComputeMemoryBudget::configure(&device,16*mib).unwrap();
        let program=pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER)).unwrap();
        let mut pool=EqualityWorkspacePool::default();pool.prefer_large_completed=true;
        for size in [4*mib,6*mib] {
            let bytes=vec![0;size as usize];
            let job=pool.acquire(&device,&queue,&program,&bytes).unwrap();
            pool.retain_completed(job);
        }
        assert_eq!(pool.jobs.len(),1);assert_eq!(pool.bytes,6*mib);
        assert_eq!(budget.stats().allocated_bytes,10*mib);
        assert_eq!(budget.stats().retired_buffers,1);
        let before=(pool.bytes,pool.jobs.len(),pool.creations,pool.reuses,budget.stats());
        assert!(matches!(pool.acquire(&device,&queue,&program,&vec![0;9*mib as usize]),
            Err(voxy_render::ComputeError::MemoryBudget)));
        assert_eq!((pool.bytes,pool.jobs.len(),pool.creations,pool.reuses,budget.stats()),before);
        let payload=vec![3u8;16];
        let job=pool.acquire(&device,&queue,&program,&payload).unwrap();
        let mut encoder=device.create_command_encoder(&Default::default());
        let snapshot=voxy_render::ComputeDispatch::copy_buffer(&device,&mut encoder,job.buffer(),0,16).unwrap();
        let submission=queue.submit([encoder.finish()]);let mut read=snapshot.begin_read();
        device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
        assert_eq!(read.try_read().unwrap().unwrap(),payload);
        pool.retain_completed(job);
        let mut retirement=budget.begin_retirement(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert_eq!(budget.stats().allocated_bytes,10*mib);
        assert!(retirement.try_finish().unwrap());
        assert_eq!(budget.stats().allocated_bytes,6*mib);
        drop(pool);
        let mut retirement=budget.begin_retirement(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert!(retirement.try_finish().unwrap());assert_eq!(budget.stats().allocated_bytes,0);
        // A 3 MiB incoming job cannot coexist with retained 6 MiB even
        // after removing 1 MiB. An unsuccessful admission must preserve both.
        let mut pool=EqualityWorkspacePool::default();pool.prefer_large_completed=true;
        for size in [6*mib,mib,3*mib] {
            let job=program.create_job(&device,&vec![0;size as usize]).unwrap();
            pool.retain_completed(job);
        }
        assert_eq!(pool.jobs.len(),2);assert_eq!(pool.bytes,7*mib);
        let mut sizes:Vec<_>=pool.jobs.iter().map(|job|job.buffer().size()).collect();
        sizes.sort_unstable();assert_eq!(sizes,vec![mib,6*mib]);
        assert_eq!(budget.stats().allocated_bytes,10*mib);
        assert_eq!(budget.stats().retired_buffers,1);
        drop(pool);
        let mut retirement=budget.begin_retirement(&queue);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert!(retirement.try_finish().unwrap());assert_eq!(budget.stats().allocated_bytes,0);
        eprintln!("LARGE WORKSPACE eviction_retired_bytes={} current_payload_preserved=true budget_refusal_preserved_cache=true impossible_admission_preserved_cache=true explicit_retirement=true",4*mib);
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
        let mut pool=EqualityWorkspacePool::default();pool.geometric_growth=true;
        for full_readback in [true,false] {
            let options=EqualityOptions {resident_columns:false,compact:false,prefix_reuse:false,release:false,early_release:false,
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
        let options=EqualityOptions {resident_columns:false,compact:false,prefix_reuse:false,release:false,early_release:false,
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
    fn profile_captured_equality_passes(device:&wgpu::Device,queue:&wgpu::Queue,qr:&ComputeProgram,solve:&ComputeProgram,
        columns:&[Vec<f64>],bounds:&[f64],tolerance:f64) {
        let scope=device.push_error_scope(wgpu::ErrorFilter::Validation);
        let reference=pollster::block_on(ComputeProgram::new(device,JOINT_CONTACT_QR_SHADER)).unwrap();
        let queries=device.create_query_set(&wgpu::QuerySetDescriptor {label:Some("captured hair QR and equality"),ty:wgpu::QueryType::Timestamp,count:4});
        let resolved=device.create_buffer(&wgpu::BufferDescriptor {label:Some("captured hair intervals"),size:256,
            usage:wgpu::BufferUsages::QUERY_RESOLVE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
        let options=EqualityOptions {resident_columns:false,compact:true,prefix_reuse:false,release:false,early_release:false,
            full_refinement_readback:true,separate_qr_passes:false,refinement_limit:8,refinement_trace:false,stage_profile:false};
        let period=f64::from(queue.get_timestamp_period());assert!(period.is_finite() && period>0.);
        let mut pool=EqualityWorkspacePool::default();let mut samples=Vec::new();
        let run=|pool:&mut EqualityWorkspacePool,candidate:bool,timed:bool| {
            let program=if candidate {qr} else {&reference};
            let options=EqualityOptions {resident_columns:candidate && experimental_resident_columns(),..options};
            let mut task=EqualityTask::new(device,queue,program,columns,bounds,tolerance,pool,None,options,&mut EqualityStats::default()).unwrap();
            let mut encoder=device.create_command_encoder(&Default::default());
            if timed {
                let writes=|i|wgpu::ComputePassTimestampWrites {query_set:&queries,beginning_of_pass_write_index:Some(i),end_of_pass_write_index:Some(i+1)};
                task.encode_with_timestamps(queue,solve,&mut encoder,options,Some(writes(0)),Some(writes(2))).unwrap();
            } else {task.encode(queue,solve,&mut encoder,options).unwrap();}
            let dispatch=task.job.encode_snapshot(&mut encoder).unwrap();
            let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
            device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
            let bytes=read.try_read().unwrap().unwrap();
            task.initial.as_ref().unwrap().decode(&bytes).unwrap();pool.retain_completed(task.job);
            let intervals=if timed {
                let mut encoder=device.create_command_encoder(&Default::default());
                encoder.resolve_query_set(&queries,0..4,&resolved,0);
                let dispatch=voxy_render::ComputeDispatch::gather_buffers(device,&mut encoder,&[(&resolved,0,0,32)],32).unwrap();
                let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
                device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
                let bytes=read.try_read().unwrap().unwrap();
                let ticks:Vec<_>=bytes.chunks_exact(8).map(|b|u64::from_le_bytes(b.try_into().unwrap())).collect();
                assert!(ticks[1]>ticks[0] && ticks[3]>ticks[2],"unwritten or nonpositive GPU intervals");
                Some(((ticks[1]-ticks[0]) as f64*period,(ticks[3]-ticks[2]) as f64*period))
            } else {None};
            (bytes,intervals)
        };
        let mut boundary_widths=Vec::new();
        if std::env::var_os("VOXY_HAIR_JOINT_QR_SHADER_FIXTURE").is_some() {
            let mut checks:Vec<(Vec<Vec<f64>>,Vec<f64>,bool)>=Vec::new();
            for width in [1919,1920,1921,4096] {
                let columns:Vec<Vec<f64>>=(0..3).map(|j|(0..width).map(|i|match j {
                    0=>1.,1=>if i%2==0 {0.25} else {-0.25},_=>if i%3==0 {0.7} else {-0.2},
                }).collect()).collect();
                checks.push((columns,vec![0.3,0.7,-0.2],true));boundary_widths.push(width);
            }
            checks.push((vec![vec![1.,0.],vec![1.,1e-20]],vec![0.5,0.5],true));
            checks.push((vec![vec![1.,0.],vec![1.,0.]],vec![0.5,0.5],false));
            for (columns,bounds,accepted) in checks {
                let mut check_options=options;check_options.compact=false;
                let execute=|pool:&mut EqualityWorkspacePool,program:&ComputeProgram,candidate:bool| {
                    let check_options=EqualityOptions {resident_columns:candidate && experimental_resident_columns(),..check_options};
                    let mut task=EqualityTask::new(device,queue,program,&columns,&bounds,tolerance,pool,None,check_options,&mut EqualityStats::default()).unwrap();
                    let mut encoder=device.create_command_encoder(&Default::default());
                    task.encode(queue,solve,&mut encoder,check_options).unwrap();
                    let dispatch=task.job.encode_snapshot(&mut encoder).unwrap();
                    let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
                    device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
                    let bytes=read.try_read().unwrap().unwrap();
                    // The tiny positive pivot qualifies QR, not an equality solve:
                    // its squared diagonal may underflow in the equality shader.
                    let qr_input=voxy_render::ResidentContactQrInput::new_with_support_compaction(&columns,false).unwrap();
                    let decoded=qr_input.decode(&bytes[..qr_input.bytes().len()]);
                    assert_eq!(decoded.is_ok(),accepted,"QR rank/underflow gate changed: width={} columns={} error={:?}",columns[0].len(),columns.len(),decoded.err());
                    if columns[0].len()>2 {
                        task.initial.as_ref().unwrap().decode(&bytes).expect("boundary equality rejected");
                    }
                    pool.retain_completed(task.job);bytes
                };
                assert_eq!(execute(&mut pool,&reference,false),execute(&mut pool,qr,true),"cache/storage boundary or rank gate changed");
            }
        }
        let expected=run(&mut pool,false,false).0;
        for sample in 0..9 {
            let order=if sample%2==0 {[false,true]} else {[true,false]};
            let first=run(&mut pool,order[0],true);let second=run(&mut pool,order[1],true);
            assert_eq!(first.0,expected,"first QR/equality storage differs at sample {sample}");
            assert_eq!(second.0,expected,"second QR/equality storage differs at sample {sample}");
            let (base,candidate)=if order[0] {(second.1.unwrap(),first.1.unwrap())} else {(first.1.unwrap(),second.1.unwrap())};
            samples.push(serde_json::json!({"sample":sample,"candidate_first":order[0],
                "baseline_qr_ns":base.0,"candidate_qr_ns":candidate.0,
                "baseline_equality_ns":base.1,"candidate_equality_ns":candidate.1}));
        }
        assert!(pollster::block_on(scope.pop()).is_none(),"profiled equality GPU validation failed");
        assert_eq!(voxy_render::ComputeReadbackPool::for_device(device).stats().quarantined_buffers,0);
        let report=serde_json::json!({"schema":"voxy-original-equality-gpu-pass-v3","supported":true,"selected_equality_rows":columns.len(),
            "original_coordinate_width":columns[0].len(),"absolute_equality_tolerance":tolerance,"capture_budget_bytes":2*1024*1024,
            "period_ns":period,"timing_diagnostic_submissions":1+samples.len()*4,"baseline_qr_dispatches_per_sample":columns.len(),"candidate_qr_dispatches_per_sample":if experimental_resident_columns() {1} else {columns.len()},
            "uncollapsed_boundary_widths_checked":boundary_widths,"rank_and_underflow_checks":!boundary_widths.is_empty(),
            "samples":samples,"exact_unprofiled_full_storage":true,"staging_quarantine":0,
            "comparison":"production QR versus selected QR; alternating timed pairs, identical equality shader; unprofiled production storage reference",
            "scope":"largest captured serial active equality within 2 MiB budget; initial fresh QR/equality passes only; excludes subsequent residual/active-set iterations, transfer, CPU work and rendering",
            "concurrent_system_load_controlled":false,"full_trajectory_qualified":false,"rendered_fps_measured":false});
        eprintln!("JOINT GPU PASS PROFILE {report}");
        if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_GPU_PASS_PROFILE_EXPORT") {std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();}
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
            let options=EqualityOptions {resident_columns:false,compact:true,prefix_reuse:true,release:true,early_release:true,
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
        let (device,queue)=pollster::block_on(adapter.request_device(&super::super::qualification_device_descriptor(&adapter))).unwrap();
        let mut serial_solver=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        let mut batch_solver=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        for solver in [&mut serial_solver,&mut batch_solver] {
            solver.joint_contact_qr=true;solver.joint_qr_prefix_reuse=true;
            solver.joint_release_directions=true;solver.joint_early_release_directions=true;
            solver.residual_refinements=1;solver.contact_response_batches=true;
            if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {
                solver.joint_triple_active_program=Some(pollster::block_on(ComputeProgram::new(&device,
                    &voxy_render::ExperimentalTripleActiveContactBatch::shader_source())).unwrap());
            }
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
        if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {
            assert!(batch_solver.joint_triple_active_program.is_some(),"single-dispatch qualification requires the triple backend");
            assert_eq!(batch_solver.joint_equality_submissions,batch_solver.joint_equality_dispatches,
                "single-dispatch path issued multiple GPU dispatches per submission");
            assert!(batch_solver.joint_equality_dispatches<serial_solver.joint_equality_dispatches,
                "physical islands never shared a GPU dispatch");
        } else {
            assert!(batch_solver.joint_equality_submissions<batch_solver.joint_equality_dispatches,
                "physical islands never shared a GPU submission");
        }
        assert_eq!(serial_solver.joint_native_fallbacks,0);assert_eq!(batch_solver.joint_native_fallbacks,0);
        eprintln!("JOINT PHYSICAL ISLAND GPU BATCH frames=4 guides=4 equality_dispatches={} submissions={} serial_dispatches={} exact_serial_pose_and_rotation=true native_fallbacks=0 scope=small_physical_scene_not_full_model_or_fps",batch_solver.joint_equality_dispatches,batch_solver.joint_equality_submissions,serial_solver.joint_equality_dispatches);
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
    fn resident_candidate_cache_preserves_exact_keys_order_and_budget() {
        let mut cache=ResidentCandidateCache::default();let columns=vec![vec![1.,0.]];
        for i in 0..40 {let b=vec![i as f64];cache.insert(&columns,&b,1e-14,&(b.clone(),b.clone()));}
        assert_eq!(cache.entries.len(),32);assert!(cache.bytes<=ResidentCandidateCache::MAX_BYTES);
        assert!(cache.get(&columns,&[0.],1e-14).is_none());
        assert_eq!(cache.get(&columns,&[39.],1e-14).unwrap().0,vec![39.]);
        assert!(cache.get(&[vec![1.,-0.]],&[39.],1e-14).is_none());
        assert!(cache.get(&columns,&[39.],2e-14).is_none());
        assert!(cache.get(&columns,&[f64::from_bits(39f64.to_bits()+1)],1e-14).is_none());
    }
    #[test]
    fn resident_active_row_hint_requires_exact_column_prefix() {
        let mut cache=ResidentCandidateCache::default();let columns=vec![vec![1.,0.],vec![0.,1.]];
        cache.insert(&columns,&[1.,2.],1e-14,&(vec![1.,2.],vec![1.,0.]));
        let mut extended=columns.clone();extended.push(vec![1.,1.]);
        assert_eq!(cache.active_row_hint(&extended),Some(vec![1.,0.,0.]));
        assert_eq!(cache.active_row_hint(&columns),Some(vec![1.,0.]));
        let mut changed=extended.clone();changed[0][0]=f64::from_bits(1f64.to_bits()+1);assert!(cache.active_row_hint(&changed).is_none());
        let mut signed=columns.clone();signed[0][1]=-0.;assert!(cache.active_row_hint(&signed).is_none());
        assert!(cache.active_row_hint(&[vec![1.]]).is_none());assert!(cache.active_row_hint(&[]).is_none());
    }

    #[test]
    #[ignore="requires GPU and original full physical fixture"]
    fn gpu_resident_candidate_reuse_requires_original_physical_admission() {
        let (requests,bounds,tolerance,_,_,_)=captured_original_physical_fixture();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;gpu.joint_resident_operator_reuse=true;
        gpu.joint_resident_active_program=Some(pollster::block_on(ComputeProgram::new(
            &device,voxy_render::EXPERIMENTAL_RESIDENT_CONTACT_SHADER)).unwrap());
        let changed=bounds.iter().map(|b|b*1.0001).collect::<Vec<_>>();let mut original=None;
        for (targets,expected_submissions) in [(&bounds,1),(&bounds,0),(&changed,1),(&bounds,0)] {
            let before=gpu.joint_equality_submissions;
            let (responses,reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,targets,tolerance,|columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
            assert!(admitted,"{:?}",gpu.last_error);
            HairResponseSystem::validate_joint_solution(&requests,targets,&responses,&reactions,tolerance).unwrap();
            assert_eq!(gpu.joint_equality_submissions-before,expected_submissions);
            if std::ptr::eq(targets,&bounds) {
                let bits=responses.iter().flatten().chain(reactions.iter()).map(|v|v.to_bits()).collect::<Vec<_>>();
                if let Some(first)=&original {assert_eq!(&bits,first);} else {original=Some(bits);}
            }
        }
        assert_eq!(gpu.joint_result_cache_hits,2);
        // Mix hits around a fresh complete operator; output slots must not follow
        // the smaller GPU batch's numbering. Identity operators have x=lambda=b.
        let columns=vec![vec![1.]];let one=vec![1.];let two=vec![2.];
        let request=|bounds|physics::hair::HairContactCoordinateRequest {columns:&columns,bounds,tolerance,seeds:&[]};
        let cold=gpu.solve_resident_active_batch(&[request(&one)]).unwrap();
        let before=gpu.joint_equality_submissions;let hits=gpu.joint_result_cache_hits;
        let mixed=gpu.solve_resident_active_batch(&[request(&one),request(&two),request(&one)]).unwrap();
        assert_eq!(gpu.joint_equality_submissions-before,1);
        assert_eq!(gpu.joint_result_cache_hits-hits,2);
        assert_eq!(mixed[0],cold[0]);assert_eq!(mixed[2],cold[0]);
        assert!((mixed[1].0[0]-2.).abs()<=tolerance);
        assert!((mixed[1].1[0]-2.).abs()<=tolerance);
        let before=gpu.joint_equality_submissions;
        assert!(gpu.solve_resident_active_batch(&[request(&one),request(&[f64::NAN])]).is_err());
        assert_eq!(gpu.joint_equality_submissions,before);
        physics::hair::HairLinearSolver::joint_contact_result(&mut gpu,false);
        assert!(gpu.joint_resident_candidates.entries.is_empty());
        eprintln!("RESIDENT EXACT CANDIDATE REUSE original_physical_admission=true submissions=1,0,1,0 hits=2 mixed_order=true invalid_batch_rejected=true physical_rejection_clears=true");
    }

    #[test]
    #[ignore="requires actual GPU and original VQC1 systems; triple candidate original physical admission"]
    fn gpu_triple_active_admits_captured_original_physics() {
        let (requests,bounds,tolerance,rows,width,systems)=captured_original_physical_fixture();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&super::super::qualification_device_descriptor(&adapter))).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();gpu.joint_contact_qr=true;
        gpu.joint_triple_active_program=Some(pollster::block_on(ComputeProgram::new(&device,
            &if let Some(path)=std::env::var_os("VOXY_HAIR_TRIPLE_SHADER_FIXTURE") {std::fs::read_to_string(path).unwrap()} else if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {voxy_render::ExperimentalTripleActiveContactBatch::shader_source()} else if std::env::var_os("VOXY_HAIR_TRIPLE_PARALLEL").is_some() {voxy_render::ExperimentalTripleActiveContactInput::parallel_shader_source()} else {voxy_render::ExperimentalTripleActiveContactInput::shader_source()})).unwrap());
        gpu.joint_resident_operator_reuse=std::env::var("VOXY_HAIR_TRIPLE_OPERATOR_REUSE").as_deref()==Ok("1");
        let duplicate=std::env::var("VOXY_HAIR_TRIPLE_BATCH_DEDUP").as_deref()==Ok("1");
        gpu.joint_resident_batch_dedup=duplicate;
        let started=std::time::Instant::now();
        let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
            &requests,&bounds,tolerance,|columns,bounds,tolerance| {
                if !duplicate {return gpu.solve_joint_coordinates(columns,bounds,tolerance);}
                let before=gpu.joint_equality_dispatches;
                let request=||physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds:&[]};
                let mut outputs=gpu.solve_triple_active_batch(&[request(),request()]).unwrap();
                assert_eq!(outputs[0],outputs[1]);assert_eq!(gpu.joint_equality_dispatches,before+1);
                outputs.pop()
            }).unwrap();
        gpu.joint_contact_result(accelerated);
        eprintln!("TRIPLE PHYSICAL rows={rows} width={width} systems={systems} accelerated={accelerated} submissions={} elapsed_ms={} error={:?} scope=one_original_physical_operator_not_trajectory_or_FPS",gpu.joint_equality_submissions,started.elapsed().as_secs_f64()*1000.,gpu.last_error);
        assert!(accelerated,"original physical owner rejected triple candidate: {:?}",gpu.last_error);
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
        assert_eq!(gpu.joint_native_fallbacks,0);
        if std::env::var("VOXY_HAIR_TRIPLE_WARM_START").as_deref()==Ok("1") {
            let began=std::time::Instant::now();
            let (again,lambda,used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&bounds,tolerance,|columns,bounds,tolerance| {
                    gpu.solve_joint_seeded_checked(columns,bounds,tolerance,&reactions).unwrap()
                }).unwrap();
            assert!(used);HairResponseSystem::validate_joint_solution(&requests,&bounds,&again,&lambda,tolerance).unwrap();
            eprintln!("TRIPLE WARM PHYSICAL rows={rows} width={width} original_admission=true elapsed_ms={} scope=same_operator_hint_not_changed_trajectory_or_FPS",began.elapsed().as_secs_f64()*1000.);
        }
        if std::env::var("VOXY_HAIR_TRIPLE_FACTOR_REUSE").as_deref()==Ok("1") {
            assert!(!gpu.joint_triple_factors.is_empty(),"warm solve must retain checked factors");
            let before=gpu.joint_triple_factor_reuses;
            let changed=bounds.iter().map(|b|b*1.000001).collect::<Vec<_>>();
            let changed_result=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&changed,tolerance,|columns,bounds,tolerance|gpu.solve_joint_seeded_checked(columns,bounds,tolerance,&reactions).unwrap());
            if changed_result.is_err() {
                let baseline=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,&changed,tolerance,|columns,bounds,tolerance| {
                        let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds:&reactions};
                        gpu.solve_triple_active_batch_attempt(&[request],false).ok().and_then(|mut values|values.pop())
                    });
                eprintln!("TRIPLE FACTOR FAILURE reused_error={:?} freshly_factored_original_physical_error={:?}",changed_result.as_ref().err(),baseline.as_ref().err());
            }
            let (again,lambda,used)=changed_result.unwrap();
            assert!(used);HairResponseSystem::validate_joint_solution(&requests,&changed,&again,&lambda,tolerance).unwrap();
            assert!(gpu.joint_triple_factor_reuses>before,"changed-bound solve did not reuse factors");
            eprintln!("TRIPLE FACTORS original_physical_admission=true changed_bounds=true reused={} scope=original_operator_replay_not_trajectory_or_FPS",gpu.joint_triple_factor_reuses-before);
            let mut extended_requests=requests.clone();
            for request in &mut extended_requests {request.loads.push(vec![0.;request.system.rhs.len()]);}
            let mut extended_bounds=changed.clone();extended_bounds.push(-1.);
            let mut extended_seeds=lambda.clone();extended_seeds.push(0.);
            let before=gpu.joint_triple_factor_reuses;
            let (extended_responses,extended_reactions,extended_used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &extended_requests,&extended_bounds,tolerance,|columns,bounds,tolerance|gpu.solve_joint_seeded_checked(columns,bounds,tolerance,&extended_seeds).unwrap()).unwrap();
            assert!(extended_used);HairResponseSystem::validate_joint_solution(&extended_requests,&extended_bounds,&extended_responses,&extended_reactions,tolerance).unwrap();
            assert!(gpu.joint_triple_factor_reuses>before,"appended inactive original load prevented factor reuse");
            eprintln!("TRIPLE ACTIVE BASIS appended_inactive_original_load=true original_physical_admission=true reused={}",gpu.joint_triple_factor_reuses-before);
            gpu.joint_contact_result(false);assert!(gpu.joint_triple_factors.is_empty());
        }
        if std::env::var("VOXY_HAIR_TRIPLE_CACHE_HINTS").as_deref()==Ok("1") {
            let changed=bounds.iter().map(|b|b*1.000001).collect::<Vec<_>>();assert_ne!(changed,bounds);
            let before=gpu.joint_equality_submissions;let began=std::time::Instant::now();
            let (again,lambda,used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&changed,tolerance,|columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
            assert!(used);HairResponseSystem::validate_joint_solution(&requests,&changed,&again,&lambda,tolerance).unwrap();
            assert!(gpu.joint_equality_submissions>before,"changed bounds must be recomputed");
            eprintln!("TRIPLE CACHE HINT PHYSICAL changed_bounds=true original_admission=true elapsed_ms={} scope=changed_bounds_same_columns_not_trajectory_or_FPS",began.elapsed().as_secs_f64()*1000.);
        }
        if gpu.joint_resident_operator_reuse {
            let submissions=gpu.joint_equality_submissions;let began=std::time::Instant::now();
            let (again,reactions_again,used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&bounds,tolerance,|columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
            gpu.joint_contact_result(used);assert!(used);
            HairResponseSystem::validate_joint_solution(&requests,&bounds,&again,&reactions_again,tolerance).unwrap();
            assert_eq!(again,responses);assert_eq!(reactions_again,reactions);
            assert_eq!(gpu.joint_equality_submissions,submissions);assert!(gpu.joint_result_cache_hits>0);
            eprintln!("TRIPLE EXACT REUSE original_physical_admission=true additional_submissions=0 elapsed_ms={} hits={} scope=same_original_operator_only_not_changed_trajectory_or_FPS",began.elapsed().as_secs_f64()*1000.,gpu.joint_result_cache_hits);
        }
    }

    #[test]
    #[ignore="requires actual GPU; triple batch complete preflight and atomic decode"]
    fn gpu_triple_batch_preflight_and_complete_decode() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&super::super::qualification_device_descriptor(&adapter))).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();gpu.joint_contact_qr=true;
        gpu.joint_triple_active_program=Some(pollster::block_on(ComputeProgram::new(&device,
            &if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {voxy_render::ExperimentalTripleActiveContactBatch::shader_source()} else {voxy_render::ExperimentalTripleActiveContactInput::parallel_shader_source()})).unwrap());
        let columns=vec![vec![1.]];let bad=vec![vec![f64::NAN]];let one=[1.];let two=[2.];
        let request=|columns,bounds|physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance:1e-14,seeds:&[]};
        assert!(gpu.solve_triple_active_batch(&[request(&columns,&one),request(&bad,&two)]).is_err());
        assert_eq!(gpu.joint_equality_submissions,0);assert_eq!(gpu.joint_equality_dispatches,0);
        let wide=vec![vec![1.;1024]];
        let oversized=(0..2000).map(|_|request(&wide,&one)).collect::<Vec<_>>();
        assert!(gpu.solve_triple_active_batch(&oversized).is_err());assert_eq!(gpu.joint_equality_submissions,0);
        let output=gpu.solve_triple_active_batch(&[request(&columns,&one),request(&columns,&two)]).unwrap();
        assert_eq!(output,vec![(vec![1.],vec![1.]),(vec![2.],vec![2.])]);
        assert_eq!(gpu.joint_equality_submissions,1);assert_eq!(gpu.joint_equality_dispatches,if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {1} else {2});
        let impossible=vec![vec![1.],vec![-1.]];let contradictory=[1.,1.];
        assert!(gpu.solve_triple_active_batch(&[request(&columns,&one),request(&impossible,&contradictory)]).is_err());
        assert_eq!(gpu.joint_equality_submissions,2);
        gpu.joint_resident_operator_reuse=true;
        let first=gpu.solve_triple_active_batch(&[request(&columns,&one)]).unwrap();let submitted=gpu.joint_equality_submissions;
        assert_eq!(gpu.solve_triple_active_batch(&[request(&columns,&one)]).unwrap(),first);
        assert_eq!(gpu.joint_equality_submissions,submitted);assert!(gpu.joint_result_cache_hits>0);
        let mixed=gpu.solve_triple_active_batch(&[request(&columns,&two),request(&columns,&one)]).unwrap();
        assert_eq!(mixed,vec![(vec![2.],vec![2.]),(vec![1.],vec![1.])]);assert_eq!(gpu.joint_equality_submissions,submitted+1);
        let changed=vec![vec![f64::from_bits(1f64.to_bits()+1)]];
        assert!(gpu.solve_triple_active_batch(&[request(&changed,&one)]).is_ok());assert_eq!(gpu.joint_equality_submissions,submitted+2);
        let changed_tolerance=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&one,
            tolerance:f64::from_bits(1e-14f64.to_bits()+1),seeds:&[]};
        assert!(gpu.solve_triple_active_batch(&[changed_tolerance]).is_ok());assert_eq!(gpu.joint_equality_submissions,submitted+3);
        let entries=gpu.joint_resident_candidates.entries.len();let hits=gpu.joint_result_cache_hits;
        assert!(gpu.solve_triple_active_batch(&[request(&columns,&one),request(&bad,&one)]).is_err());
        assert_eq!(gpu.joint_equality_submissions,submitted+3);assert_eq!(gpu.joint_result_cache_hits,hits);
        assert_eq!(gpu.joint_resident_candidates.entries.len(),entries);
        let three=[3.];
        assert!(gpu.solve_triple_active_batch(&[request(&columns,&one),request(&columns,&three),request(&impossible,&contradictory)]).is_err());
        assert_eq!(gpu.joint_resident_candidates.entries.len(),entries);
        assert!(gpu.joint_resident_candidates.get(&columns,&three,1e-14).is_none(),"failed batch cached a valid prefix");
        physics::hair::HairLinearSolver::joint_contact_result(&mut gpu,false);
        assert!(gpu.joint_resident_candidates.entries.is_empty());
        gpu.joint_resident_operator_reuse=false;gpu.joint_resident_batch_dedup=true;
        let before=gpu.joint_equality_dispatches;let submits=gpu.joint_equality_submissions;
        let shared=gpu.solve_triple_active_batch(&[request(&columns,&two),request(&columns,&one),request(&columns,&two)]).unwrap();
        assert_eq!(shared,vec![(vec![2.],vec![2.]),(vec![1.],vec![1.]),(vec![2.],vec![2.])]);
        assert_eq!(gpu.joint_equality_dispatches,before+if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {1} else {2});assert_eq!(gpu.joint_equality_submissions,submits+1);
        assert!(gpu.solve_triple_active_batch(&[request(&columns,&two),request(&columns,&two),request(&impossible,&contradictory)]).is_err());
        assert!(gpu.joint_resident_candidates.entries.is_empty());
        if std::env::var("VOXY_HAIR_TRIPLE_WARM_START").as_deref()==Ok("1") {
            let consistent=[1.,-1.];let hints=[1.,1.];let before=gpu.joint_equality_submissions;
            let hinted=physics::hair::HairContactCoordinateRequest {columns:&impossible,bounds:&consistent,tolerance:1e-14,seeds:&hints};
            let retry=gpu.solve_triple_active_batch(&[hinted]).unwrap();assert_eq!(retry[0].0,vec![1.]);
            assert_eq!(gpu.joint_equality_submissions,before+2);assert!(gpu.joint_resident_candidates.entries.is_empty());
        }
        if std::env::var("VOXY_HAIR_TRIPLE_CACHE_HINTS").as_deref()==Ok("1") {
            gpu.joint_resident_operator_reuse=true;gpu.joint_resident_candidates=ResidentCandidateCache::default();
            let prefix=vec![vec![1.,0.]];let extended=vec![vec![1.,0.],vec![0.,1.]];
            gpu.solve_triple_active_batch(&[request(&prefix,&one)]).unwrap();
            assert_eq!(gpu.joint_resident_candidates.active_row_hint(&extended),Some(vec![1.,0.]));
            let before=gpu.joint_equality_submissions;
            let result=gpu.solve_triple_active_batch(&[request(&extended,&[1.,2.])]).unwrap();
            assert_eq!(result[0].0,vec![1.,2.]);assert_eq!(gpu.joint_equality_submissions,before+1);
            physics::hair::HairLinearSolver::joint_contact_result(&mut gpu,false);
            assert!(gpu.joint_resident_candidates.active_row_hint(&extended).is_none());
        }
        if std::env::var("VOXY_HAIR_TRIPLE_FACTOR_REUSE").as_deref()==Ok("1") {
            gpu.joint_resident_operator_reuse=false;gpu.joint_resident_batch_dedup=true;gpu.joint_triple_factors.clear();
            let hint=[1.];
            let seeded=|bounds|physics::hair::HairContactCoordinateRequest {columns:&columns,bounds,tolerance:1e-14,seeds:&hint};
            gpu.solve_triple_active_batch(&[seeded(&one)]).unwrap();assert_eq!(gpu.joint_triple_factors.len(),1);
            let before=gpu.joint_equality_dispatches;let reused=gpu.joint_triple_factor_reuses;
            let output=gpu.solve_triple_active_batch(&[seeded(&two),seeded(&two)]).unwrap();
            assert_eq!(output,vec![(vec![2.],vec![2.]);2]);assert_eq!(gpu.joint_equality_dispatches,before+1);
            assert_eq!(gpu.joint_triple_factor_reuses,reused+1);assert_eq!(gpu.joint_triple_factors.len(),1,"same basis duplicated in cache");
            let retained=gpu.joint_triple_factors[0].retained_bytes();
            assert!(gpu.solve_triple_active_batch(&[seeded(&three),physics::hair::HairContactCoordinateRequest {columns:&impossible,bounds:&contradictory,tolerance:1e-14,seeds:&[]}]).is_err());
            assert_eq!(gpu.joint_triple_factors.len(),1);assert_eq!(gpu.joint_triple_factors[0].retained_bytes(),retained);
            physics::hair::HairLinearSolver::joint_contact_result(&mut gpu,false);assert!(gpu.joint_triple_factors.is_empty());
            eprintln!("TRIPLE FACTOR CACHE compact_basis_dedup=true shared_dispatch_after_reuse=true failed_batch_no_commit=true physical_rejection_clears=true");
        }
        if std::env::var("VOXY_HAIR_TRIPLE_COMPLETED_FACTORS").as_deref()==Ok("1")
            &&std::env::var("VOXY_HAIR_TRIPLE_FACTOR_REUSE").as_deref()==Ok("1") {
            gpu.joint_resident_operator_reuse=false;gpu.joint_resident_batch_dedup=false;gpu.joint_triple_factors.clear();
            let pair_columns=vec![vec![1.,0.],vec![0.,1.]];let pair_bounds=[1.,2.];
            let pair=physics::hair::HairContactCoordinateRequest {columns:&pair_columns,bounds:&pair_bounds,tolerance:1e-14,seeds:&[]};
            let scalar_one=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&one,tolerance:1e-14,seeds:&[]};
            let scalar_two=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&two,tolerance:1e-14,seeds:&[]};
            let mixed=gpu.solve_triple_active_batch(&[scalar_one,pair,scalar_two]).unwrap();
            assert_eq!(mixed,vec![(vec![1.],vec![1.]),(vec![1.,2.],vec![1.,2.]),(vec![2.],vec![2.])]);
            assert_eq!(gpu.joint_triple_factors.len(),2,"cold multi-iteration bases were not retained");
            let mut ordered=voxy_render::ExperimentalTripleActiveContactInput::new_seeded(&pair_columns,&pair_bounds,1e-14,&[1.,1.]).unwrap();
            assert!(gpu.joint_triple_factors.iter().all(|f|ordered.reuse_parallel_factors(f).is_err()),"fixture did not require restoring a different active order");
            let before=gpu.joint_triple_factor_reuses;
            let changed=[2.,3.];let seeds=[1.,1.];
            let warm=physics::hair::HairContactCoordinateRequest {columns:&pair_columns,bounds:&changed,tolerance:1e-14,seeds:&seeds};
            let result=gpu.solve_triple_active_batch(&[warm]).unwrap();assert_eq!(result,vec![(vec![2.,3.],vec![2.,3.])]);
            assert_eq!(gpu.joint_triple_factor_reuses,before+1);
            physics::hair::HairLinearSolver::joint_contact_result(&mut gpu,false);assert!(gpu.joint_triple_factors.is_empty());
            eprintln!("TRIPLE COMPLETED CACHE cold_multi_iteration_retained=true mixed_operator_offsets=true explicit_final_row_order_restored=true changed_bounds=true physical_rejection_clears=true");
        }
        if std::env::var("VOXY_HAIR_TRIPLE_WORKSPACE_REUSE").as_deref()==Ok("1") {
            gpu.joint_resident_operator_reuse=false;gpu.joint_resident_batch_dedup=false;
            gpu.joint_workspaces=EqualityWorkspacePool::default();
            let large=vec![vec![1.,0.],vec![0.,1.]];
            let large_request=physics::hair::HairContactCoordinateRequest {columns:&large,bounds:&[1.,2.],tolerance:1e-14,seeds:&[]};
            gpu.solve_triple_active_batch(&[large_request]).unwrap();
            assert_eq!(gpu.joint_workspaces.jobs.len(),1);
            assert!(gpu.joint_workspaces.jobs[0].buffer().size()>voxy_render::ExperimentalTripleActiveContactInput::required_bytes(1,1).unwrap() as u64);
            let before=gpu.joint_workspace_reuses;
            let small_request=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&two,tolerance:1e-14,seeds:&[]};
            let output=gpu.solve_triple_active_batch(&[small_request]).unwrap();assert_eq!(output,vec![(vec![2.],vec![2.])]);
            assert!(gpu.joint_workspace_reuses>before);
            let retained=gpu.joint_workspaces.jobs.len();let bytes=gpu.joint_workspaces.bytes;
            let invalid=physics::hair::HairContactCoordinateRequest {columns:&bad,bounds:&one,tolerance:1e-14,seeds:&[]};
            assert!(gpu.solve_triple_active_batch(&[invalid]).is_err());
            assert_eq!(gpu.joint_workspaces.jobs.len(),retained);assert_eq!(gpu.joint_workspaces.bytes,bytes);
            assert!(bytes<=EqualityWorkspacePool::MAX_BYTES);
            eprintln!("TRIPLE WORKSPACE smaller_payload_in_larger_allocation=true complete_fresh_upload=true exact_readback_length=true invalid_preflight_preserves_pool=true bounded_completed_jobs=true");
        }
        eprintln!("TRIPLE BATCH DEDUP original_order=true exact_payload_required=true failed_batch_withholds_aliases=true");
        eprintln!("TRIPLE EXACT CACHE changed_coefficient_misses=true changed_bound_misses=true changed_tolerance_misses=true mixed_order=true invalid_preflight_preserves_cache=true failed_decode_no_prefix_commit=true physical_rejection_clears=true");
        eprintln!("TRIPLE BATCH later_NaN_preflight=true aggregate_capacity_preflight=true valid_order=true one_submission_two_operators=true rejected_later_output_withholds_whole_batch=true scope=transport_not_physical_admission");
    }

    #[test]
    #[ignore="requires GPU and VOXY_HAIR_TRIPLE_GPU_TIMESTAMPS=1; missing capability must reject before submission"]
    fn gpu_triple_timestamp_capability_preflight() {
        assert_eq!(std::env::var("VOXY_HAIR_TRIPLE_GPU_TIMESTAMPS").as_deref(),Ok("1"));
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        assert!(!device.features().contains(wgpu::Features::TIMESTAMP_QUERY));
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        let batched=std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1");
        let source=if batched {voxy_render::ExperimentalTripleActiveContactBatch::shader_source()}
            else {voxy_render::ExperimentalTripleActiveContactInput::parallel_shader_source()};
        gpu.joint_triple_active_program=Some(pollster::block_on(ComputeProgram::new(&device,&source)).unwrap());
        let columns=vec![vec![1.]];let bounds=[1.];
        let request=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&bounds,tolerance:1e-14,seeds:&[]};
        let error=gpu.solve_triple_active_batch(&[request]).unwrap_err();
        assert!(error.to_string().contains("timestamp capability/capacity"));
        assert_eq!(gpu.joint_equality_submissions,0);assert_eq!(gpu.joint_equality_dispatches,0);
        assert_eq!(gpu.joint_workspace_creations,0);assert!(gpu.joint_resident_candidates.entries.is_empty());
    }

    #[test]
    #[ignore="requires GPU, original VQC1 fixture and completed/warm/parallel factor flags"]
    fn gpu_triple_completed_cache_original_physical_sequence() {
        for flag in ["VOXY_HAIR_TRIPLE_COMPLETED_FACTORS","VOXY_HAIR_TRIPLE_WARM_START","VOXY_HAIR_TRIPLE_PARALLEL","VOXY_HAIR_TRIPLE_FACTOR_REUSE"] {
            assert_eq!(std::env::var(flag).as_deref(),Ok("1"),"missing sequence qualification flag {flag}");
        }
        let steps=std::env::var("VOXY_HAIR_TRIPLE_SEQUENCE_STEPS").ok().map(|s|s.parse::<usize>().unwrap()).unwrap_or(32);
        assert!((16..=256).contains(&steps));
        let (requests,bounds,tolerance,rows,width,systems)=captured_original_physical_fixture();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&super::super::qualification_device_descriptor(&adapter))).unwrap();
        let source=if std::env::var("VOXY_HAIR_TRIPLE_SINGLE_DISPATCH").as_deref()==Ok("1") {voxy_render::ExperimentalTripleActiveContactBatch::shader_source()}
            else {voxy_render::ExperimentalTripleActiveContactInput::parallel_shader_source()};
        let mut cached=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        let mut fresh=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        for solver in [&mut cached,&mut fresh] {
            solver.joint_contact_qr=true;solver.joint_resident_operator_reuse=false;
            solver.joint_triple_active_program=Some(pollster::block_on(ComputeProgram::new(&device,&source)).unwrap());
        }
        let mut seeds=Vec::new();let mut max_response_difference=0f64;let mut max_reaction_difference=0f64;
        for step in 1..=steps {
            let phase=((step*7)%31) as f64/30.-0.5;
            let current=bounds.iter().map(|v|v*(1.+phase*2e-6)).collect::<Vec<_>>();
            let (response,reactions,used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&current,tolerance,|columns,bounds,tolerance|cached.solve_joint_coordinates_seeded(columns,bounds,tolerance,&seeds)).unwrap();
            cached.joint_contact_result(used);
            HairResponseSystem::validate_joint_solution(&requests,&current,&response,&reactions,tolerance).unwrap();
            let (reference,reference_reactions,reference_used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &requests,&current,tolerance,|columns,bounds,tolerance| {
                    let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds:&[]};
                    fresh.solve_triple_active_batch_attempt(&[request],false).ok().and_then(|mut values|values.pop())
                }).unwrap();
            fresh.joint_contact_result(reference_used);
            if !used||!reference_used {
                // Diagnostic only: freshly factor the prior admitted active-row
                // hints in their canonical sorted order, with no retained QR.
                let (sorted_response,sorted_reactions,sorted_used)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,&current,tolerance,|columns,bounds,tolerance| {
                        let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds:&seeds};
                        fresh.solve_triple_active_batch_attempt(&[request],false).ok().and_then(|mut values|values.pop())
                    }).unwrap();
                HairResponseSystem::validate_joint_solution(&requests,&current,&sorted_response,&sorted_reactions,tolerance).unwrap();
                let report=serde_json::json!({"step":step,"rows":rows,"coordinates":width,"systems":systems,
                    "fresh_sorted_seeded_original_physical_admission":sorted_used,
                    "cached_original_physical_admission":used,"fresh_original_physical_admission":reference_used,
                    "cached_error":cached.last_error,"bounds_bits":current.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                    "incoming_seeds_bits":seeds.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                    "scope":"sequence failure diagnostic; native fallback output never qualifies GPU admission"});
                eprintln!("TRIPLE COMPLETED SEQUENCE FAILURE {report}");
                if let Some(path)=std::env::var_os("VOXY_HAIR_TRIPLE_SEQUENCE_FAILURE_EXPORT") {
                    let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();serde_json::to_writer_pretty(file,&report).unwrap();
                }
            }
            assert!(used,"cached sequence step={step} used native fallback: {:?}",cached.last_error);
            assert!(reference_used,"fresh sequence step={step} used native fallback");
            HairResponseSystem::validate_joint_solution(&requests,&current,&reference,&reference_reactions,tolerance).unwrap();
            assert!(fresh.joint_triple_factors.is_empty(),"fresh comparison reused a factor cache");
            for (a,b) in response.iter().flatten().zip(reference.iter().flatten()) {max_response_difference=max_response_difference.max((a-b).abs());}
            for (a,b) in reactions.iter().zip(&reference_reactions) {max_reaction_difference=max_reaction_difference.max((a-b).abs());}
            seeds=reactions;
            eprintln!("TRIPLE COMPLETED SEQUENCE STEP step={step}/{steps} rows={rows} coordinates={width} both_original_physical_admissions=true cached_factor_reuses={} scope=changing_bounds_fixed_original_operator_not_jump_trajectory",cached.joint_triple_factor_reuses);
        }
        assert!(cached.joint_triple_factor_reuses>0,"sequence never used completed factors");
        assert_eq!(cached.joint_native_fallbacks,0);assert_eq!(fresh.joint_native_fallbacks,0);
        eprintln!("TRIPLE COMPLETED SEQUENCE {}",serde_json::json!({"rows":rows,"coordinates":width,"systems":systems,"steps":steps,
            "cached_factor_reuses":cached.joint_triple_factor_reuses,"cached_compute_submissions":cached.joint_equality_submissions,
            "fresh_compute_submissions":fresh.joint_equality_submissions,"native_fallbacks":0,"all_original_physical_gates":true,
            "max_response_difference":max_response_difference,"max_reaction_difference":max_reaction_difference,
            "scope":"bounded changing-bound sequence on fixed original matrices/loads; both paths independently pass unchanged native gates; not bit identity, full jump, timing benchmark or FPS"}));
    }

    fn captured_original_physical_fixture()->(Vec<HairResponseSystem>,Vec<f64>,f64,usize,usize,usize) {
        let path=std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap();
        let mut input=std::io::Cursor::new(std::fs::read(path).unwrap());
        fn integer(input:&mut std::io::Cursor<Vec<u8>>)->usize {let mut b=[0;4];input.read_exact(&mut b).unwrap();u32::from_le_bytes(b) as usize}
        fn scalar(input:&mut std::io::Cursor<Vec<u8>>)->f64 {let mut b=[0;8];input.read_exact(&mut b).unwrap();f64::from_le_bytes(b)}
        let mut magic=[0;4];input.read_exact(&mut magic).unwrap();assert_eq!(&magic,b"VQC1");
        let rows=integer(&mut input);let width=integer(&mut input);let systems=integer(&mut input);let _=integer(&mut input);
        let tolerance=scalar(&mut input);assert!(rows>0&&rows<=1024&&width<=65536&&systems<=512);
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
        (requests,bounds,tolerance,rows,width,systems)
    }

    #[test]
    #[ignore="requires GPU; exact duplicate operators share one workgroup"]
    fn gpu_resident_batch_deduplicates_exact_inputs_and_preserves_warm_hints() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_resident_operator_reuse=true;gpu.joint_resident_batch_dedup=true;
        gpu.joint_resident_active_program=Some(pollster::block_on(ComputeProgram::new(
            &device,voxy_render::EXPERIMENTAL_RESIDENT_CONTACT_SHADER)).unwrap());
        let columns=vec![vec![1.]];let one=[1.];let two=[2.];
        let request=|bounds|physics::hair::HairContactCoordinateRequest {columns:&columns,bounds,tolerance:1e-12,seeds:&[]};
        let output=gpu.solve_resident_active_batch_with_seeds(
            &[request(&one),request(&two),request(&one)],false,&mut false).unwrap();
        assert_eq!(output.len(),3);assert_eq!(output[0],output[2]);
        assert!((output[0].0[0]-1.).abs()<1e-12);assert!((output[1].0[0]-2.).abs()<1e-12);
        assert_eq!(gpu.joint_result_cache_hits,1);
        assert_eq!(gpu.joint_resident_candidates.entries.len(),2);
        assert_eq!(gpu.joint_equality_submissions,1);
        gpu.joint_resident_candidates=ResidentCandidateCache::default();
        gpu.joint_resident_active_program=Some(pollster::block_on(ComputeProgram::new(
            &device,voxy_render::EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER)).unwrap());
        let seeded=|seeds|physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&one,tolerance:1e-12,seeds};
        let hits=gpu.joint_result_cache_hits;
        let mut seeded_submission=false;
        let output=gpu.solve_resident_active_batch_with_seeds(
            &[seeded(&one),seeded(&two),seeded(&one)],true,&mut seeded_submission).unwrap();
        assert!(seeded_submission);
        assert_eq!(gpu.joint_result_cache_hits-hits,1,"different hints were merged");
        assert_eq!(gpu.joint_resident_candidates.entries.len(),2);
        for candidate in output {
            assert!((candidate.0[0]-1.).abs()<1e-12);
            assert!((candidate.1[0]-1.).abs()<1e-12);
        }
        gpu.joint_resident_candidates=ResidentCandidateCache::default();
        let before=gpu.joint_equality_submissions;
        assert!(gpu.solve_resident_active_batch_with_seeds(
            &[request(&one),request(&one),request(&[f64::NAN])],false,&mut false).is_err());
        assert_eq!(gpu.joint_equality_submissions,before,"invalid later owner launched earlier work");
        assert!(gpu.joint_resident_candidates.entries.is_empty());
        if std::env::var_os("VOXY_HAIR_DEDUP_BENCHMARK").is_some() {
            let columns=(0..8).map(|i| {
                let mut column=vec![0.;128];column[i]=1.;column
            }).collect::<Vec<_>>();
            gpu.joint_resident_workspace_reuse=true;
            for unique in [false,true] {
            let bounds=(0..16).map(|i|vec![if unique {1.+i as f64/16.} else {1.};8]).collect::<Vec<_>>();
            let requests=bounds.iter().map(|bounds|physics::hair::HairContactCoordinateRequest {
                columns:&columns,bounds,tolerance:1e-12,seeds:&[]}).collect::<Vec<_>>();
            let mut reference=None;
            for pair in 0..6 {
                for reuse in if pair%2==0 {[false,true]} else {[true,false]} {
                    gpu.joint_resident_operator_reuse=true;
                    gpu.joint_resident_batch_dedup=reuse;
                    gpu.joint_resident_candidates=ResidentCandidateCache::default();
                    let start=std::time::Instant::now();
                    let output=gpu.solve_resident_active_batch_with_seeds(&requests,false,&mut false).unwrap();
                    let elapsed=start.elapsed().as_secs_f64()*1000.;
                    if let Some(expected)=&reference {assert_eq!(&output,expected);} else {reference=Some(output);}
                    eprintln!("RESIDENT DEDUP BENCH pair={pair} reuse={reuse} elapsed_ms={elapsed} owners=16 rows=8 width=128 unique={unique} exact_output_match=true scope=operator_fixture_not_full_model_or_FPS");
                }
            }
            }
        }
    }

    #[test]
    #[ignore="requires GPU; resident operator workspace freshness across changed shapes and failures"]
    fn gpu_resident_workspace_reuse_preserves_current_operator() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&super::super::qualification_device_descriptor(&adapter))).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;gpu.joint_resident_workspace_reuse=true;
        let source=std::env::var_os("VOXY_HAIR_RESIDENT_ACTIVE_SHADER_FIXTURE")
            .map(|path|std::fs::read_to_string(path).expect("read resident workspace qualification shader"));
        gpu.joint_resident_active_program=Some(pollster::block_on(ComputeProgram::new(
            &device,source.as_deref().unwrap_or(voxy_render::EXPERIMENTAL_RESIDENT_CONTACT_SHADER))).unwrap());
        let large=vec![vec![1.;128]];
        let first=gpu.solve_joint_seeded_checked(&large,&[128.],1e-12,&[]).unwrap().unwrap();
        let creations=gpu.joint_workspace_creations;
        let small=vec![vec![1.,0.],vec![0.,1.]];
        for bounds in [[2.,3.],[-1.,4.],[0.,0.],[2.,3.]] {
            let actual=gpu.solve_joint_seeded_checked(&small,&bounds,1e-12,&[]).unwrap().unwrap();
            // Independent closed form for C=I: x=lambda=max(bounds,0).
            let positive=bounds.map(|value|value.max(0.)).to_vec();
            let reference=(positive.clone(),positive);
            for (a,b) in actual.0.iter().chain(&actual.1).zip(reference.0.iter().chain(&reference.1)) {
                assert!((a-b).abs()<=1e-12,"reused workspace changed current operator: {a} vs {b}");
            }
            assert_eq!(gpu.joint_workspace_creations,creations,"smaller operator allocated instead of reusing storage");
        }
        assert_eq!(gpu.solve_joint_seeded_checked(&large,&[128.],1e-12,&[]).unwrap().unwrap(),first);
        assert!(gpu.joint_workspace_reuses>=5);
        let submissions=gpu.joint_equality_submissions;
        assert!(gpu.solve_joint_seeded_checked(&[vec![f64::NAN]],&[1.],1e-12,&[]).is_err());
        assert_eq!(gpu.joint_equality_submissions,submissions,"invalid operator was submitted");
        // A failed complete candidate must never enter the completed cache.
        assert!(gpu.solve_joint_seeded_checked(&[vec![1.],vec![-1.]],&[1.,1.],1e-12,&[]).is_err());
        let recovered=gpu.solve_joint_seeded_checked(&small,&[5.,6.],1e-12,&[]).unwrap().unwrap();
        assert!(recovered.0.iter().zip([5.,6.]).all(|(a,b)|(a-b).abs()<=1e-12));
        eprintln!("RESIDENT WORKSPACE current_operator_admitted=true creations={} reuses={} changed_shape_and_failure_recovery=true",gpu.joint_workspace_creations,gpu.joint_workspace_reuses);
    }

    #[test]
    #[ignore="requires GPU and full original VQC1 fixture; experimental resident candidate"]
    fn gpu_resident_active_set_admits_original_physical_operator() {
        let (requests,bounds,tolerance,rows,width,systems)=captured_original_physical_fixture();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        eprintln!("RESIDENT PHYSICAL ADAPTER {:?}",adapter.get_info());
        let timestamp_profile=std::env::var_os("VOXY_HAIR_RESIDENT_GPU_TIMESTAMPS").is_some();
        let descriptor=super::super::qualification_device_descriptor(&adapter);
        let (device,queue)=pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.joint_contact_qr=true;
        let source=std::env::var_os("VOXY_HAIR_RESIDENT_ACTIVE_SHADER_FIXTURE")
            .map(|path|std::fs::read_to_string(path).expect("read resident active-set qualification shader"));
        gpu.joint_resident_active_program=Some(pollster::block_on(ComputeProgram::new(
            &device,source.as_deref().unwrap_or(if std::env::var("VOXY_HAIR_RESIDENT_WARM_START").as_deref()==Ok("1") {
                voxy_render::experimental_seeded_resident_contact_shader(&device.limits()).unwrap()
            } else {voxy_render::EXPERIMENTAL_RESIDENT_CONTACT_SHADER}))).unwrap());
        gpu.joint_resident_workspace_reuse=std::env::var("VOXY_HAIR_RESIDENT_WORKSPACE_REUSE").as_deref()==Ok("1");
        let mut captured=None;let began=std::time::Instant::now();
        let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
            &requests,&bounds,tolerance,|columns,bounds,tolerance| {
                captured=Some((columns.to_vec(),bounds.to_vec(),tolerance));
                gpu.solve_joint_coordinates(columns,bounds,tolerance)
            }).unwrap();
        let submissions=gpu.joint_equality_submissions;
        let elapsed_ms=began.elapsed().as_secs_f64()*1000.;
        assert!(accelerated,"resident candidate used native fallback: {:?}",gpu.last_error);
        assert_eq!(submissions,1);
        let (columns,coordinate_bounds,coordinate_tolerance)=captured.unwrap();
        let request=physics::hair::HairContactCoordinateRequest {
            columns:&columns,bounds:&coordinate_bounds,tolerance:coordinate_tolerance,seeds:&[]};
        let before=gpu.joint_equality_submissions;
        let batch=gpu.solve_joint_batch_checked(&[physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&coordinate_bounds,tolerance:coordinate_tolerance,seeds:&[]},request]).unwrap().unwrap();
        assert_eq!(gpu.joint_equality_submissions-before,1);
        assert_eq!(batch.len(),2);assert_eq!(batch[0],batch[1]);
        if timestamp_profile {
            let changed_profile=std::env::var("VOXY_HAIR_RESIDENT_TIMESTAMP_CHANGED_BOUNDS").as_deref()==Ok("1");
            let targets=if changed_profile {bounds.iter().map(|b|b*1.0001).collect::<Vec<_>>()} else {bounds.clone()};
            let mut profile_operator=None;let mut cold_reference=None;
            if changed_profile {
                let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,&targets,tolerance,|c,b,t| {
                        profile_operator=Some((c.to_vec(),b.to_vec(),t));
                        let candidate=gpu.solve_joint_coordinates(c,b,t);
                        cold_reference=candidate.clone();candidate
                    }).unwrap();
                assert!(admitted,"changed timing fixture failed original physical admission");
                HairResponseSystem::validate_joint_solution(&requests,&targets,&current,&current_reactions,tolerance).unwrap();
            }
            let (columns,coordinate_bounds,coordinate_tolerance)=profile_operator.unwrap_or_else(||
                (columns.clone(),coordinate_bounds.clone(),coordinate_tolerance));
            let cold_reference=cold_reference.unwrap_or_else(||batch[0].clone());
            let packed=voxy_render::ExperimentalResidentContactBatch::new(vec![
                voxy_render::ExperimentalResidentContactInput::new(&columns,&coordinate_bounds,coordinate_tolerance).unwrap()
            ]).unwrap();
            let program=gpu.joint_resident_active_program.as_ref().unwrap();
            let period=f64::from(queue.get_timestamp_period());
            assert!(period.is_finite() && period>0.);
            let warm_profile=std::env::var("VOXY_HAIR_RESIDENT_WARM_START").as_deref()==Ok("1");
            let mut samples=Vec::new();let mut warm_samples=Vec::new();let mut timestamp_pairs=Vec::new();
            // Alternate order to avoid consistently charging the first case
            // with clock ramp or concurrent queue pressure.
            for seeded in [false,true,true,false,false,true] {
                if seeded && !warm_profile {continue;}
                let queries=device.create_query_set(&wgpu::QuerySetDescriptor {
                    label:Some("complete resident contact pass"),ty:wgpu::QueryType::Timestamp,count:2});
                let resolved=device.create_buffer(&wgpu::BufferDescriptor {label:Some("resident contact timestamps"),
                    size:256,usage:wgpu::BufferUsages::QUERY_RESOLVE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});

                let warm_packed=seeded.then(||voxy_render::ExperimentalResidentContactBatch::new(vec![
                    voxy_render::ExperimentalResidentContactInput::new_seeded(&columns,&coordinate_bounds,
                        coordinate_tolerance,&batch[0].1).unwrap()]).unwrap());
                let packed=warm_packed.as_ref().unwrap_or(&packed);
                let job=program.create_job(&device,packed.bytes()).unwrap();
                let mut encoder=device.create_command_encoder(&Default::default());
                job.encode_step_with_timestamps(&mut encoder,packed.workgroups(),wgpu::ComputePassTimestampWrites {
                    query_set:&queries,beginning_of_pass_write_index:Some(0),end_of_pass_write_index:Some(1)}).unwrap();
                let snapshot=job.encode_snapshot(&mut encoder).unwrap();
                let submission=queue.submit([encoder.finish()]);
                let mut read=snapshot.begin_read();
                device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
                // Resolve only after the producing command buffer has completed.
                // Metal counter availability must not be inferred from encoding order.
                let mut resolve_encoder=device.create_command_encoder(&Default::default());
                resolve_encoder.resolve_query_set(&queries,0..2,&resolved,0);
                let stamps=voxy_render::ComputeDispatch::copy_buffer(&device,&mut resolve_encoder,&resolved,0,16).unwrap();
                let resolve_submission=queue.submit([resolve_encoder.finish()]);
                let mut stamp_read=stamps.begin_read();
                device.poll(wgpu::PollType::Wait {submission_index:Some(resolve_submission),timeout:None}).unwrap();
                let candidate=packed.candidate(&read.try_read().unwrap().unwrap()).unwrap();
                if !seeded {assert_eq!(candidate[0],cold_reference,"timed cold pass changed validated candidate");}
                let (physical,physical_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,&targets,tolerance,|current,current_bounds,current_tolerance| {
                        assert_eq!(current,columns);assert_eq!(current_bounds,coordinate_bounds);
                        assert_eq!(current_tolerance,coordinate_tolerance);
                        Some(candidate[0].clone())
                    }).unwrap();
                assert!(admitted,"timed candidate failed original physical admission");
                HairResponseSystem::validate_joint_solution(&requests,&targets,&physical,&physical_reactions,tolerance).unwrap();
                let bytes=stamp_read.try_read().unwrap().unwrap();
                let begin=u64::from_le_bytes(bytes[0..8].try_into().unwrap());
                let end=u64::from_le_bytes(bytes[8..16].try_into().unwrap());
                assert!(end>begin,"unwritten or nonpositive resident timestamps: begin={begin} end={end}");
                assert!(!timestamp_pairs.iter().any(|&(_,b,e)|b==begin && e==end),"repeated timestamp pair across independent passes");
                timestamp_pairs.push((seeded,begin,end));
                let duration=(end-begin) as f64*period/1e6;
                if seeded {warm_samples.push(duration);} else {samples.push(duration);}
            }
            eprintln!("RESIDENT GPU PASS PROFILE {}",serde_json::json!({
                "scope":"complete single-operator GPU pass; excludes host preparation, queue wait, readback, physical reconstruction and rendering",
                "changed_physical_bounds":changed_profile,"rows":rows,"coordinates":width,"gpu_pass_ms":samples,"warm_gpu_pass_ms":warm_samples,"timestamp_pairs_seeded_begin_end":timestamp_pairs,"cold_candidate_equality_verified":true,"all_timed_candidates_original_physics_admitted":true}));
        }
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
        let (native,native_reactions)=HairResponseSystem::solve_joint_load_inequalities_native(&requests,&bounds,tolerance).unwrap();
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&native,&native_reactions,tolerance).unwrap();
        let component=native.iter().flatten().zip(responses.iter().flatten()).map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
        let reaction=native_reactions.iter().zip(&reactions).map(|(a,b)|(a-b).abs()).fold(0f64,f64::max);
        if gpu.joint_resident_workspace_reuse {
            let creations=gpu.joint_workspace_creations;
            let changed=bounds.iter().map(|b|b*1.0001).collect::<Vec<_>>();
            for (label,targets) in [("same",&bounds),("changed",&changed),("restored",&bounds)] {
                let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,targets,tolerance,|columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
                assert!(admitted,"reused {label} physical operator fell back: {:?}",gpu.last_error);
                HairResponseSystem::validate_joint_solution(&requests,targets,&current,&current_reactions,tolerance).unwrap();
                assert_eq!(gpu.joint_workspace_creations,creations,"warm operator created new storage");
                if label!="changed" {assert_eq!(current,responses);assert_eq!(current_reactions,reactions);}
            }
            assert!(gpu.joint_workspace_reuses>=3);
            eprintln!("RESIDENT ORIGINAL WORKSPACE changed_and_restored_physical_operators_admitted=true creations={} reuses={}",gpu.joint_workspace_creations,gpu.joint_workspace_reuses);
        }
        if std::env::var("VOXY_HAIR_RESIDENT_WARM_START").as_deref()==Ok("1") {
            let changed=bounds.iter().map(|b|b*1.0001).collect::<Vec<_>>();
            for targets in [&bounds,&changed] {
                let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &requests,targets,tolerance,|columns,bounds,tolerance| {
                        let request=physics::hair::HairContactCoordinateRequest {columns,bounds,tolerance,seeds:&batch[0].1};
                        Some(gpu.solve_resident_active_batch(&[request]).unwrap().remove(0))
                    }).unwrap();
                assert!(admitted,"warm original operator fell back");
                HairResponseSystem::validate_joint_solution(&requests,targets,&current,&current_reactions,tolerance).unwrap();
            }
            for direction in [-1.,1.] {
                let mut changed_systems=requests.clone();
                for (index,system) in changed_systems.iter_mut().enumerate() {
                    // Positive per-system scaling preserves SPD while changing
                    // the current physical factors and coordinate columns.
                    let scale=1.+direction*0.0001*(index+1) as f64;
                    for value in &mut system.system.matrix {*value*=scale;}
                }
                let mut saw_changed_columns=false;
                let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                    &changed_systems,&bounds,tolerance,|current_columns,current_bounds,current_tolerance| {
                        saw_changed_columns=current_columns!=columns;
                        let request=physics::hair::HairContactCoordinateRequest {columns:current_columns,
                            bounds:current_bounds,tolerance:current_tolerance,seeds:&batch[0].1};
                        Some(gpu.solve_resident_active_batch(&[request]).unwrap().remove(0))
                    }).unwrap();
                assert!(saw_changed_columns,"stiffness fixture did not change coordinate operator");
                assert!(admitted,"previous-operator hints failed changed physical stiffness");
                HairResponseSystem::validate_joint_solution(&changed_systems,&bounds,&current,&current_reactions,tolerance).unwrap();
            }
            eprintln!("RESIDENT WARM STIFFNESS changed_current_factors=true previous_operator_hints=true original_physical_admission=true directions=2");
            let mut reordered=requests.clone();let mut reordered_bounds=bounds.clone();
            reordered_bounds.rotate_left(7);
            for system in &mut reordered {system.loads.rotate_left(7);}
            let (current,current_reactions,admitted)=HairResponseSystem::solve_joint_load_inequalities_accelerated(
                &reordered,&reordered_bounds,tolerance,|c,b,t| {
                    // Deliberately retain the old order: hints are provisional,
                    // never an identity certificate for current contact rows.
                    let request=physics::hair::HairContactCoordinateRequest {columns:c,bounds:b,tolerance:t,seeds:&batch[0].1};
                    Some(gpu.solve_resident_active_batch(&[request]).unwrap().remove(0))
                }).unwrap();
            assert!(admitted,"reordered contacts with stale hints used native fallback");
            HairResponseSystem::validate_joint_solution(&reordered,&reordered_bounds,&current,&current_reactions,tolerance).unwrap();
            eprintln!("RESIDENT WARM ROW ORDER current_constraints_rotated=7 hints_unpermuted=true original_physical_admission=true");

            let dependent=vec![vec![1.],vec![1.]];let targets=[1.,1.];let hints=[1.,1.];
            let request=physics::hair::HairContactCoordinateRequest {columns:&dependent,bounds:&targets,tolerance:1e-12,seeds:&hints};
            let before=gpu.joint_equality_submissions;
            let original=physics::hair::HairContactCoordinateRequest {columns:&columns,bounds:&coordinate_bounds,
                tolerance:coordinate_tolerance,seeds:&batch[0].1};
            let recovered=gpu.solve_resident_active_batch(&[original,request]).unwrap();
            assert_eq!(gpu.joint_equality_submissions-before,2,"dependent warm set did not retry whole batch cold");
            assert_eq!(recovered.len(),2);
            assert_eq!(recovered[0],batch[0],"cold retry changed the complete original operator");
            assert!((recovered[1].0[0]-1.).abs()<=1e-12);
            // Requests that were already cold cannot benefit from a second
            // identical dispatch, even if the experimental warm mode is on.
            let impossible=vec![vec![1.],vec![-1.]];
            for seeds in [&[][..],&[0.,0.][..],&[f64::NAN,1.][..]] {
                let before=gpu.joint_equality_submissions;
                let request=physics::hair::HairContactCoordinateRequest {columns:&impossible,bounds:&[1.,1.],tolerance:1e-12,seeds};
                assert!(gpu.solve_resident_active_batch(&[request]).is_err());
                assert_eq!(gpu.joint_equality_submissions-before,1,"already-cold failure submitted twice");
            }
            eprintln!("RESIDENT WARM ORIGINAL current_and_changed_original_physics_admitted=true dependent_hint_cold_retry=true");
        }
        let report=serde_json::json!({"status":"original_physical_operator_admitted","systems":systems,"rows":rows,"coordinate_width":width,
            "absolute_physical_tolerance":tolerance,"submissions":submissions,"elapsed_ms":elapsed_ms,"native_fallback_used":!accelerated,
            "maximum_response_component_difference":component,"maximum_reaction_difference":reaction,"response_units":"unassigned",
            "scope":"one complete original physical operator, including native factor/response reconstruction and original admission; not trajectory or FPS"});
        eprintln!("RESIDENT PHYSICAL QUALIFICATION {report}");
        if let Some(path)=std::env::var_os("VOXY_HAIR_RESIDENT_PHYSICAL_EXPORT") {std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();}
    }

    #[test]
    #[ignore = "requires Metal/Vulkan hardware and original VQC1 physical contact fixture"]
    fn gpu_joint_backend_admits_captured_original_physics() {
        let (requests,bounds,tolerance,rows,width,systems)=captured_original_physical_fixture();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("JOINT BACKEND ADAPTER {:?}",adapter.get_info());
        let mut descriptor=wgpu::DeviceDescriptor::default();
        if std::env::var_os("VOXY_HAIR_JOINT_GPU_PASS_PROFILE").is_some()
            && adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            descriptor.required_features|=wgpu::Features::TIMESTAMP_QUERY;
        }
        if std::env::var_os("VOXY_HAIR_TRANSFER_ONE_STORAGE").is_some() {
            descriptor.required_limits.max_storage_buffers_per_shader_stage=1;
        }
        let (device,queue)=pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        eprintln!("JOINT BACKEND storage_limit={}",device.limits().max_storage_buffers_per_shader_stage);
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();gpu.joint_contact_qr=true;
        if let Some(path)=std::env::var_os("VOXY_HAIR_JOINT_QR_SHADER_FIXTURE") {
            let source=std::fs::read_to_string(path).unwrap();
            gpu.joint_programs=Some((pollster::block_on(ComputeProgram::new(&device,&source)).unwrap(),
                pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER)).unwrap()));
        }
        gpu.joint_resident_qr_columns=experimental_resident_columns();
        gpu.joint_workspaces.geometric_growth=std::env::var_os("VOXY_HAIR_JOINT_WORKSPACE_GEOMETRIC").is_some();
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
        if std::env::var_os("VOXY_HAIR_JOINT_GPU_PASS_PROFILE").is_some() {
            if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                let (columns,bounds,tolerance)=gpu.joint_profile_operator.take().expect("no equality operator within 2 MiB capture budget");
                let (qr,solve)=gpu.joint_programs.as_ref().unwrap();
                profile_captured_equality_passes(&device,&queue,qr,solve,&columns,&bounds,tolerance);
            } else {eprintln!("JOINT GPU PASS PROFILE supported=false measured=false");}
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
        let workspace_bench=std::env::var_os("VOXY_HAIR_JOINT_WORKSPACE_BENCH").is_some();
        let resident_bench=std::env::var_os("VOXY_HAIR_JOINT_RESIDENT_QR_BENCH").is_some();
        let tail_bench=std::env::var_os("VOXY_HAIR_JOINT_TAIL_BENCH").is_some();
        let pass_bench=std::env::var_os("VOXY_HAIR_JOINT_QR_PASS_BENCH").is_some();
        let prefix_bench=std::env::var_os("VOXY_HAIR_JOINT_QR_PREFIX_BENCH").is_some();
        assert!([pass_bench,tail_bench,prefix_bench,resident_bench,workspace_bench].into_iter().filter(|v|*v).count()<=1,"select one benchmark comparison");
        if workspace_bench || resident_bench || prefix_bench || pass_bench || tail_bench || std::env::var_os("VOXY_HAIR_JOINT_SUPPORT_BENCH").is_some() {
            assert!(!resident_bench || gpu.joint_resident_qr_columns,"resident benchmark requires explicit resident shader fixture flags");
            let mut reference_programs=resident_bench.then(||(
                pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_QR_SHADER)).unwrap(),
                pollster::block_on(ComputeProgram::new(&device,JOINT_CONTACT_EQUALITY_SHADER)).unwrap()));
            let mut candidate_selected=true;
            let _initial_workspaces=workspace_bench.then(||std::mem::take(&mut gpu.joint_workspaces));
            let mut other_workspaces=EqualityWorkspacePool::default();
            let mut workspace_candidate_selected=false;
            let mut run=|compact:bool| {
                if workspace_bench && workspace_candidate_selected!=compact {
                    std::mem::swap(&mut gpu.joint_workspaces,&mut other_workspaces);workspace_candidate_selected=compact;
                }
                if workspace_bench {gpu.joint_workspaces.geometric_growth=compact;}
                let workspaces_before=(gpu.joint_workspace_creations,gpu.joint_workspace_reuses);
                if resident_bench && candidate_selected!=compact {
                    std::mem::swap(&mut gpu.joint_programs,&mut reference_programs);candidate_selected=compact;
                }
                gpu.joint_resident_qr_columns=resident_bench && compact;
                gpu.joint_support_compaction=if workspace_bench || resident_bench || tail_bench || pass_bench || prefix_bench {true} else {compact};
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
                (elapsed,gpu.joint_equality_dispatches-before,responses,reactions,gpu.joint_qr_columns_reused-qr_before,gpu.joint_workspace_creations-workspaces_before.0,gpu.joint_workspace_reuses-workspaces_before.1)
            };
            for compact in [false,true] {let _=run(compact);}
            let mut dense=Vec::new();let mut compact_times=Vec::new();let mut records=Vec::new();
            let mut max_response_difference=0f64;let mut max_reaction_difference=0f64;
            for pair in 0..7 {
                let order=if pair%2==0 {[false,true]} else {[true,false]};
                let first=run(order[0]);let second=run(order[1]);
                let (d,c)=if order[0] {(&second,&first)} else {(&first,&second)};
                dense.push(d.0);compact_times.push(c.0);
                if workspace_bench || resident_bench || tail_bench || pass_bench || prefix_bench {
                    assert_eq!(d.1,c.1,"readback changed equality submission count");
                    assert_eq!(d.2,c.2,"readback changed original physical coordinates");
                    assert_eq!(d.3,c.3,"readback changed original physical reactions");
                }
                if workspace_bench || resident_bench {
                    assert!(d.2.iter().flatten().zip(c.2.iter().flatten()).all(|(a,b)|a.to_bits()==b.to_bits()),"resident QR changed physical response bits");
                    assert!(d.3.iter().zip(&c.3).all(|(a,b)|a.to_bits()==b.to_bits()),"resident QR changed physical reaction bits");
                }
                if prefix_bench {assert_eq!(d.4,0);assert!(c.4>0,"prefix benchmark did not reuse QR columns");}
                for (a,b) in d.2.iter().flatten().zip(c.2.iter().flatten()) {max_response_difference=max_response_difference.max((a-b).abs());}
                for (a,b) in d.3.iter().zip(&c.3) {max_reaction_difference=max_reaction_difference.max((a-b).abs());}
                records.push(serde_json::json!({"pair":pair,"dense_seconds":d.0,"compact_seconds":c.0,"dense_equality_dispatches":d.1,"compact_equality_dispatches":c.1,"baseline_qr_columns_reused":d.4,"candidate_qr_columns_reused":c.4,"baseline_workspace_creations":d.5,"candidate_workspace_creations":c.5,"baseline_workspace_reuses":d.6,"candidate_workspace_reuses":c.6}));
            }
            dense.sort_by(f64::total_cmp);compact_times.sort_by(f64::total_cmp);
            let mut result=serde_json::json!({"scope":"complete original physical projection, warmed shaders, seven alternating pairs; excludes device creation, fixture load, rendering and extra post-timing validation",
                "rows":rows,"coordinates":width,"systems":systems,"absolute_physical_tolerance":tolerance,"native_fallback_used":false,
                "median_dense_seconds":dense[3],"median_compact_seconds":compact_times[3],"speedup":dense[3]/compact_times[3],
                "maximum_response_difference":max_response_difference,"maximum_reaction_difference":max_reaction_difference,"pairs":records,
                "whole_animation_benchmark":false,"rendered_fps_measured":false,
                "concurrent_system_load_controlled":false});
            if workspace_bench {
                let report=result.as_object_mut().unwrap();
                report.insert("comparison".into(),serde_json::json!("exact-size versus geometric-capacity workspace allocation; separate warmed pools; production shader, support compaction and logical-payload readback in both"));
                for (old,new) in [("median_dense_seconds","median_exact_capacity_seconds"),("median_compact_seconds","median_geometric_capacity_seconds")] {
                    let value=report.remove(old).unwrap();report.insert(new.into(),value);
                }
                for pair in report.get_mut("pairs").unwrap().as_array_mut().unwrap() {
                    let pair=pair.as_object_mut().unwrap();
                    for (old,new) in [("dense_seconds","exact_capacity_seconds"),("compact_seconds","geometric_capacity_seconds"),("dense_equality_dispatches","exact_equality_dispatches"),("compact_equality_dispatches","geometric_equality_dispatches")] {
                        let value=pair.remove(old).unwrap();pair.insert(new.into(),value);
                    }
                }
            }
            if resident_bench {
                let report=result.as_object_mut().unwrap();
                report.insert("comparison".into(),serde_json::json!("production per-column QR dispatches versus one resident ordered-column dispatch; original full physical projection, support compaction and full readback in both"));
                for (old,new) in [("median_dense_seconds","median_baseline_seconds"),("median_compact_seconds","median_candidate_seconds")] {
                    let value=report.remove(old).unwrap();report.insert(new.into(),value);
                }
                for pair in report.get_mut("pairs").unwrap().as_array_mut().unwrap() {
                    let pair=pair.as_object_mut().unwrap();
                    for (old,new) in [("dense_seconds","baseline_seconds"),("compact_seconds","candidate_seconds"),("dense_equality_dispatches","baseline_equality_dispatches"),("compact_equality_dispatches","candidate_equality_dispatches")] {
                        let value=pair.remove(old).unwrap();pair.insert(new.into(),value);
                    }
                }
            }
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
