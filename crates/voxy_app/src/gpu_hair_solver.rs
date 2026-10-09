//! Checked hybrid bridge. Matrix assembly/contact ownership stays in physics.
//! Blocking correction readback is a qualification path, not a real-time claim.
use physics::hair::{HairLinearSolver, HairLinearSystem, HairResponseSystem};
use voxy_render::{BandedSolveInput,BandedSystem,ComputeJob,ComputeProgram,BANDED_SOLVE_SHADER};
use wgpu::util::DeviceExt;
#[path="gpu_hair_response_dispatch.rs"]
mod response_dispatch;
#[derive(Debug)]
pub struct GpuHairLinearSolver {
    device:wgpu::Device,queue:wgpu::Queue,program:ComputeProgram,resident:Option<ComputeJob>,
    pub reference_audit:bool,
    pub max_linear_error:[f64;2],
    pub max_packing_error:[f64;2],
    /// Correction errors from equilibration, input packing and GPU arithmetic.
    pub max_stage_error:[[f64;2];3],
    /// Qualification-only mixed-precision iterations; never enabled implicitly.
    pub residual_refinements:usize,
    /// Contact-only precision experiment; None follows structural refinement.
    pub contact_residual_refinements:Option<usize>,
    pub refinement_dispatches:usize,
    /// Reuse same-call immutable factors for residual RHS corrections.
    /// Layout/coefficient equivalence is checked before updating the GPU job.
    pub reuse_refinement_factors:bool,
    pub reused_factor_dispatches:usize,
    pub contact_response_batches:bool,
    pub batch_response_waves:bool,
    pub compact_response_readback:bool,
    pub gpu_response_transport:bool,
    rhs_transfer_program:Option<voxy_render::BandedTransferProgram>,
    pub response_transfer_dispatches:usize,
    pub response_submissions:usize,
    pub response_calls:usize,
    pub response_dispatches:usize,
    pub calls:usize,pub elapsed_ms:f64,pub last_error:Option<String>,
}
impl GpuHairLinearSolver {
    pub async fn new(device:&wgpu::Device,queue:&wgpu::Queue)->Result<Self,voxy_render::ComputeError> {
        Self::new_with_extra_division_refinement(device,queue,false).await
    }
    /// Qualify a second compensated division residual update. This changes
    /// arithmetic accuracy only; the physical matrices and residual gates stay fixed.
    pub async fn new_with_extra_division_refinement(device:&wgpu::Device,queue:&wgpu::Queue,extra:bool)->Result<Self,voxy_render::ComputeError> {
        Self::new_with_refinements(device,queue,extra,false).await
    }
    /// Independent precision experiments preserve physical matrices and gates.
    pub async fn new_with_refinements(device:&wgpu::Device,queue:&wgpu::Queue,extra_division:bool,extra_root:bool)->Result<Self,voxy_render::ComputeError> {
        let mut source=std::borrow::Cow::Borrowed(BANDED_SOLVE_SHADER);
        for (declaration,replacement,enabled) in [
            ("const division_refinements:u32=1u;","const division_refinements:u32=2u;",extra_division),
            ("const root_refinements:u32=1u;","const root_refinements:u32=2u;",extra_root),
        ] {
            if enabled {
                if source.matches(declaration).count()!=1 {
                    return Err(voxy_render::ComputeError::Validation("missing unique arithmetic refinement declaration".into()));
                }
                source=std::borrow::Cow::Owned(source.replace(declaration,replacement));
            }
        }
        let program=ComputeProgram::new(device,&source).await?;
        Ok(Self {device:device.clone(),queue:queue.clone(),program,resident:None,reference_audit:false,max_linear_error:[0.;2],max_packing_error:[0.;2],max_stage_error:[[0.;2];3],residual_refinements:0,contact_residual_refinements:None,refinement_dispatches:0,reuse_refinement_factors:true,reused_factor_dispatches:0,contact_response_batches:false,batch_response_waves:false,compact_response_readback:false,gpu_response_transport:false,rhs_transfer_program:None,response_transfer_dispatches:0,response_submissions:0,response_calls:0,response_dispatches:0,calls:0,elapsed_ms:0.,last_error:None})
    }
    fn solve_checked(&mut self,systems:&[HairLinearSystem])->Result<Vec<Vec<f64>>,Box<dyn std::error::Error>> {
        if self.residual_refinements>3 {return Err("hair residual refinements must be in 0..3".into());}
        let first=systems.first().ok_or("empty hair accelerator batch")?;
        if systems.iter().any(|s|s.band_width!=9 || s.active!=first.active) {return Err("inconsistent hair accelerator batch".into());}
        let rows:Vec<_>=systems.iter().map(|s|BandedSystem {matrix:&s.matrix,rhs:&s.rhs}).collect();
        let input=BandedSolveInput::new_with_underflow_tolerance(&rows,first.active.clone(),1e-40)?;
        let packing_error=input.packing_error();
        for kind in 0..2 {self.max_packing_error[kind]=self.max_packing_error[kind].max(packing_error[kind]);}
        let mut corrections=self.dispatch_input(&input,None)?;
        if self.reference_audit {
            let mut errors=[0f64;2];let mut worst=[0usize;2];
            let mut stages=[[0f64;2];3];
            for (index,(system,correction)) in systems.iter().zip(&corrections).enumerate() {
                let native=system.solve_native()?;
                let packed=input.packed_reference(index).ok_or("missing packed reference")?;
                let n=system.rhs.len();
                let mut normalized=HairLinearSystem {band_width:9,matrix:vec![0.;n*9],rhs:vec![0.;n],active:system.active.clone()};
                for i in 0..n {
                    normalized.rhs[i]=system.rhs[i]*packed.scales[i];
                    for offset in 0..=8.min(i) {
                        normalized.matrix[i*9+offset]=system.matrix[i*9+offset]*packed.scales[i]*packed.scales[i-offset];
                    }
                }
                let equilibrated=normalized.solve_native()?;
                normalized.matrix=packed.matrix;normalized.rhs=packed.rhs;
                let represented=normalized.solve_native()?;
                for i in system.active.clone() {
                    let kind=usize::from(i%6>=3);let error=(native[i]-correction[i]).abs();
                    if error>errors[kind] {errors[kind]=error;worst[kind]=index;}
                    let balanced=equilibrated[i]*packed.scales[i];
                    let represented=represented[i]*packed.scales[i];
                    for (stage,error) in [(0,(native[i]-balanced).abs()),(1,(balanced-represented).abs()),(2,(represented-correction[i]).abs())] {
                        stages[stage][kind]=stages[stage][kind].max(error);
                    }
                }
            }
            for kind in 0..2 {self.max_linear_error[kind]=self.max_linear_error[kind].max(errors[kind]);}
            for stage in 0..3 {for kind in 0..2 {self.max_stage_error[stage][kind]=self.max_stage_error[stage][kind].max(stages[stage][kind]);}}
            eprintln!("GPU HAIR LINEAR AUDIT call={} position_correction_error_m={} rotation_correction_error_rad={} worst_rods={:?} packing_errors={:?} stage_errors={:?} before_residual_refinement=true",self.calls,errors[0],errors[1],worst,packing_error,stages);
        }
        for _ in 0..self.residual_refinements {
            // Physics owns the original f64 matrix product; the correction
            // equation is still solved on the GPU, without a native factorization.
            let residuals=systems.iter().zip(&corrections).map(|(system,correction)|system.correction_residual(correction)).collect::<Result<Vec<_>,_>>()?;
            // The residual changes only RHS values. Keep the exact admitted
            // coefficient words/scales rather than re-equilibrating the matrix.
            let residual_columns:Vec<_>=residuals.iter().map(Vec::as_slice).collect();
            let increment_input=input.with_rhs(&residual_columns,1e-40)?;
            let increments=self.dispatch_input(&increment_input,if self.reuse_refinement_factors {Some(&input)} else {None})?;
            self.refinement_dispatches+=1;
            if self.reuse_refinement_factors {self.reused_factor_dispatches+=1;}
            for ((system,correction),increment) in systems.iter().zip(&mut corrections).zip(increments) {
                for i in system.active.clone() {correction[i]+=increment[i];}
                // Revalidate the combined solution against the original matrix.
                system.validate_correction(correction)?;
            }
        }
        Ok(corrections)
    }
    fn dispatch_input(&mut self,input:&BandedSolveInput,factored_input:Option<&BandedSolveInput>)->Result<Vec<Vec<f64>>,Box<dyn std::error::Error>> {
        let mut rhs_upload=None;
        if let Some(factored_input)=factored_input {
            let job=self.resident.as_ref().ok_or("missing resident hair factors")?;
            if job.buffer().size()!=input.bytes().len() as u64 {return Err("resident hair factor layout changed".into());}
            let updates=input.factored_rhs_updates(factored_input)?;
            let mut payload=Vec::with_capacity(updates.iter().map(|(_,bytes)|bytes.len()).sum());
            let mut copies=Vec::with_capacity(updates.len());
            for (target,bytes) in updates {
                copies.push((payload.len() as u64,target,bytes.len() as u64));
                payload.extend_from_slice(&bytes);
            }
            let upload=self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label:Some("hair coalesced residual RHS"),contents:&payload,usage:wgpu::BufferUsages::COPY_SRC,
            });
            rhs_upload=Some((upload,copies));
        } else if self.resident.as_ref().is_none_or(|job|job.buffer().size()!=input.bytes().len() as u64) {
            self.resident=Some(self.program.create_job(&self.device,input.bytes())?);
        } else {self.queue.write_buffer(self.resident.as_ref().unwrap().buffer(),0,input.bytes());}
        let job=self.resident.as_ref().unwrap();
        let mut encoder=self.device.create_command_encoder(&Default::default());
        if let Some((upload,copies))=&rhs_upload {
            for &(source,target,size) in copies {encoder.copy_buffer_to_buffer(upload,source,job.buffer(),target,size);}
        }
        job.encode_step(&mut encoder,input.dispatch())?;
        let full_snapshot=if self.reference_audit {Some(job.encode_snapshot(&mut encoder)?)} else {None};
        let limits=self.device.limits();
        let groups=(input.compact_output_size()/4).div_ceil(64);
        let shader_gather=limits.max_storage_buffers_per_shader_stage>=2
            && limits.max_compute_workgroup_size_x>=64 && limits.max_compute_invocations_per_workgroup>=64
            && groups<=limits.max_compute_workgroups_per_dimension as u64;
        let dispatch=if shader_gather {
            if self.rhs_transfer_program.is_none() {
                self.rhs_transfer_program=Some(pollster::block_on(voxy_render::BandedTransferProgram::new(&self.device))?);
            }
            let compact=self.device.create_buffer(&wgpu::BufferDescriptor {
                label:Some("hair checked compact correction"),size:input.compact_output_size(),
                usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false,
            });
            self.rhs_transfer_program.as_ref().unwrap().encode(&mut encoder,job.buffer(),&compact,input,false)?;
            voxy_render::ComputeDispatch::copy_buffer(&self.device,&mut encoder,&compact,0,input.compact_output_size())?
        } else {
            // Preserve one-storage-buffer devices: ordered copies transport
            // exactly the same RHS/status bits without a second storage binding.
            voxy_render::ComputeDispatch::gather_buffer(&self.device,&mut encoder,job.buffer(),&input.compact_output_ranges(),input.compact_output_size())?
        };
        self.queue.submit([encoder.finish()]);
        let mut read=dispatch.begin_read();
        let mut full_read=full_snapshot.map(|snapshot|snapshot.begin_read());
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes=read.try_read()?.ok_or("hair accelerator readback pending")?;
        let corrections=input.decode_compact_checked(&bytes,1e-8)?;
        if let Some(read)=&mut full_read {
            let bytes=read.try_read()?.ok_or("hair full reference readback pending")?;
            let reference=input.decode_checked(&bytes,1e-8)?;
            if corrections.iter().flatten().zip(reference.iter().flatten()).any(|(a,b)|a.to_bits()!=b.to_bits()) {
                return Err("compact hair correction differs from full snapshot".into());
            }
        }
        Ok(corrections)
    }
    fn solve_responses_checked(&mut self,requests:&[HairResponseSystem])->Result<Vec<Vec<Vec<f64>>>,Box<dyn std::error::Error>> {
        let refinements=self.contact_residual_refinements.unwrap_or(self.residual_refinements);
        if refinements>3 {return Err("hair contact residual refinements must be in 0..3".into());}
        let mut groups=std::collections::BTreeMap::<usize,Vec<usize>>::new();
        for (index,request) in requests.iter().enumerate() {
            request.validate()?;
            if !request.loads.is_empty() {groups.entry(request.system.rhs.len()).or_default().push(index);}
        }
        let mut output=vec![Vec::new();requests.len()];
        for indices in groups.values() {
            let active=requests[indices[0]].system.active.clone();
            let waves=indices.iter().map(|&i|requests[i].loads.len()).max().unwrap();
            if self.batch_response_waves {
                let loads:Vec<Vec<&[f64]>>=(0..waves).map(|wave|indices.iter().map(|&i|requests[i].loads.get(wave).map(Vec::as_slice).unwrap_or(&requests[i].system.rhs)).collect()).collect();
                let rows:Vec<_>=indices.iter().zip(&loads[0]).map(|(&i,&rhs)|BandedSystem {matrix:&requests[i].system.matrix,rhs}).collect();
                let first_input=BandedSolveInput::new_with_underflow_tolerance(&rows,active.clone(),1e-40)?;
                let mut inputs=Vec::with_capacity(waves);
                for loads in loads.iter().skip(1) {inputs.push(first_input.with_rhs(loads,1e-40)?);}
                inputs.insert(0,first_input);
                let mut responses=self.dispatch_response_waves(&inputs,false)?;
                for _ in 0..refinements {
                    let residuals=loads.iter().zip(&responses).map(|(loads,values)|indices.iter().zip(loads).zip(values).map(|((&i,&rhs),value)|requests[i].system.load_residual(value,rhs)).collect::<Result<Vec<_>,_>>()).collect::<Result<Vec<_>,_>>()?;
                    let increments=residuals.iter().map(|residuals| {
                        let rhs:Vec<_>=residuals.iter().map(Vec::as_slice).collect();
                        inputs[0].with_rhs(&rhs,1e-40)
                    }).collect::<Result<Vec<_>,_>>()?;
                    let increments=self.dispatch_response_waves(&increments,true)?;
                    for (wave,increments) in responses.iter_mut().zip(increments) {for (value,increment) in wave.iter_mut().zip(increments) {for i in active.clone() {value[i]+=increment[i];}}}
                }
                for (wave,(loads,responses)) in loads.iter().zip(responses).enumerate() {
                    for ((&index,&load),response) in indices.iter().zip(loads).zip(responses) {
                        requests[index].system.validate_load_correction(&response,load)?;
                        if wave<requests[index].loads.len() {output[index].push(response);}
                    }
                }
                continue;
            }
            let mut factored_input=None;
            for wave in 0..waves {
                let loads:Vec<_>=indices.iter().map(|&i|requests[i].loads.get(wave).map(Vec::as_slice).unwrap_or(&requests[i].system.rhs)).collect();
                let rows:Vec<_>=indices.iter().zip(&loads).map(|(&i,&rhs)|BandedSystem {matrix:&requests[i].system.matrix,rhs}).collect();
                let input=BandedSolveInput::new_with_underflow_tolerance(&rows,active.clone(),1e-40)?;
                let mut responses=self.dispatch_input(&input,factored_input.as_ref())?;
                self.response_dispatches+=1;
                if factored_input.is_none() {factored_input=Some(input);}
                for _ in 0..refinements {
                    let residuals=indices.iter().zip(&loads).zip(&responses).map(|((&i,&rhs),values)|requests[i].system.load_residual(values,rhs)).collect::<Result<Vec<_>,_>>()?;
                    let rows:Vec<_>=indices.iter().zip(&residuals).map(|(&i,rhs)|BandedSystem {matrix:&requests[i].system.matrix,rhs}).collect();
                    let increment=BandedSolveInput::new_with_underflow_tolerance(&rows,active.clone(),1e-40)?;
                    let increments=self.dispatch_input(&increment,factored_input.as_ref())?;
                    self.response_dispatches+=1;
                    for (values,increment) in responses.iter_mut().zip(increments) {
                        for i in active.clone() {values[i]+=increment[i];}
                    }
                }
                for ((&index,&load),response) in indices.iter().zip(&loads).zip(responses) {
                    requests[index].system.validate_load_correction(&response,load)?;
                    if wave<requests[index].loads.len() {output[index].push(response);}
                }
            }
        }
        Ok(output)
    }
}
impl HairLinearSolver for GpuHairLinearSolver {
    fn contact_responses_enabled(&self)->bool {self.contact_response_batches}
    fn solve_responses(&mut self,systems:&[HairResponseSystem])->Result<Vec<Vec<Vec<f64>>>, &'static str> {
        let started=std::time::Instant::now();self.response_calls+=1;
        let result=self.solve_responses_checked(systems);self.elapsed_ms+=started.elapsed().as_secs_f64()*1000.;
        match result {
            Ok(responses)=>{self.last_error=None;Ok(responses)},
            Err(error)=>{
                #[cfg(test)]
                if let Some(path)=std::env::var_os("VOXY_HAIR_RESPONSE_FAILURE_EXPORT") {
                    let requests:Vec<_>=systems.iter().map(|request|serde_json::json!({
                        "matrix":request.system.matrix,"rhs":request.system.rhs,
                        "first":request.system.active.start,"end":request.system.active.end,
                        "loads":request.loads,
                    })).collect();
                    let capture=serde_json::json!({"scope":"Rejected original f64 response batch; no trajectory qualification",
                        "response_call":self.response_calls,"error":error.to_string(),"requests":requests});
                    match serde_json::to_vec(&capture).map_err(|e|e.to_string()).and_then(|bytes|std::fs::write(&path,bytes).map_err(|e|e.to_string())) {
                        Ok(())=>eprintln!("HAIR RESPONSE FAILURE EXPORT {:?} call={}",path,self.response_calls),
                        Err(error)=>eprintln!("HAIR RESPONSE FAILURE EXPORT ERROR {error}"),
                    }
                }
                self.last_error=Some(error.to_string());Err("checked GPU hair response solver failed")
            }
        }
    }
    fn solve(&mut self,systems:&[HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {
        let started=std::time::Instant::now();self.calls+=1;
        let result=self.solve_checked(systems);self.elapsed_ms+=started.elapsed().as_secs_f64()*1000.;
        match result {Ok(corrections)=>{self.last_error=None;Ok(corrections)},Err(error)=>{self.last_error=Some(error.to_string());Err("checked GPU hair linear solver failed")}}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn captured_system(linear:&serde_json::Value)->HairLinearSystem {
        HairLinearSystem {
            band_width:linear["band_width"].as_u64().unwrap() as usize,
            matrix:serde_json::from_value(linear["matrix"].clone()).unwrap(),
            rhs:serde_json::from_value(linear["rhs"].clone()).unwrap(),
            active:linear["active_start"].as_u64().unwrap() as usize..linear["active_end"].as_u64().unwrap() as usize,
        }
    }
    #[test]
    #[ignore = "requires actual GPU and exported full captured structural batch"]
    fn gpu_captured_full_structural_batch() {
        let path=std::env::var("VOXY_HAIR_STRUCTURAL_BATCH_FIXTURE").expect("captured structural batch");
        let rows:Vec<serde_json::Value>=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let systems:Vec<_>=rows.iter().map(captured_system).collect();
        assert_eq!(systems.len(),469,"full guide density must be retained");
        let native:Vec<_>=systems.iter().map(|s|s.solve_native().unwrap()).collect();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let info=adapter.get_info();
        assert_ne!(info.device_type,wgpu::DeviceType::Cpu);
        eprintln!("CAPTURED FULL STRUCTURAL ADAPTER {info:?}");
        let mut descriptor=wgpu::DeviceDescriptor::default();
        let one_storage=std::env::var_os("VOXY_HAIR_TRANSFER_ONE_STORAGE").is_some();
        if one_storage {descriptor.required_limits.max_storage_buffers_per_shader_stage=1;}
        let (device,queue)=pollster::block_on(adapter.request_device(&descriptor)).unwrap();
        eprintln!("CAPTURED STRUCTURAL TRANSFER storage_limit={}",device.limits().max_storage_buffers_per_shader_stage);
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.residual_refinements=1;
        let mut times=[Vec::new(),Vec::new()];let mut maximum=[0f64;2];
        for iteration in 0..8 {for mode in [iteration%2,1-iteration%2] {
            gpu.reference_audit=iteration==0;
            gpu.reuse_refinement_factors=mode==1;
            let before=gpu.elapsed_ms;
            let actual=gpu.solve(&systems).unwrap_or_else(|error|panic!("{error}: {:?}",gpu.last_error));
            assert_eq!(actual.len(),systems.len());
            if iteration>0 {times[mode].push(gpu.elapsed_ms-before);}
            for ((system,expected),actual) in systems.iter().zip(&native).zip(actual) {
                system.validate_correction(&actual).expect("original full-density force residual gate");
                for i in system.active.clone() {let kind=usize::from(i%6>=3);maximum[kind]=maximum[kind].max((expected[i]-actual[i]).abs());}
            }
        }}
        for samples in &mut times {samples.sort_by(f64::total_cmp);}
        eprintln!("CAPTURED FULL STRUCTURAL systems={} maximum_position_difference_m={} maximum_angle_difference_rad={} fresh_factor_median_ms={} reused_factor_median_ms={}",systems.len(),maximum[0],maximum[1],times[0][3],times[1][3]);
        assert!(maximum[0]<1e-6 && maximum[1]<5e-5);
        assert_eq!(gpu.calls,16);assert_eq!(gpu.reused_factor_dispatches,8);
        if one_storage {assert!(gpu.rhs_transfer_program.is_none(),"copy transport must not require the dual-storage shader");}
    }
    #[test]
    #[ignore = "requires actual GPU and captured rejected guide linear system"]
    fn gpu_captured_strain_admission_correction() {
        let path=std::env::var("VOXY_HAIR_REJECTED_GUIDE_FIXTURE").expect("rejected guide fixture");
        let capture:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let linear=&capture["candidate_linear_system"];
        let system=captured_system(linear);
        let native=system.solve_native().unwrap();
        system.validate_correction(&native).unwrap();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let info=adapter.get_info();
        assert_ne!(info.device_type,wgpu::DeviceType::Cpu,"hardware qualification requires a GPU");
        eprintln!("CAPTURED STRAIN ADAPTER {info:?}");
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.reference_audit=true;gpu.residual_refinements=1;
        let corrections=gpu.solve(std::slice::from_ref(&system)).unwrap_or_else(|error|panic!("{error}: {:?}",gpu.last_error));
        assert_eq!(corrections.len(),1);
        system.validate_correction(&corrections[0]).expect("original f64 force residual gate");
        let mut maximum=[0f64;2];
        for i in system.active.clone() {
            let kind=usize::from(i%6>=3);
            maximum[kind]=maximum[kind].max((native[i]-corrections[0][i]).abs());
        }
        eprintln!("CAPTURED STRAIN GPU maximum_position_difference_m={} maximum_angle_difference_rad={} elapsed_ms={}",maximum[0],maximum[1],gpu.elapsed_ms);
        assert!(maximum[0]<1e-6 && maximum[1]<5e-5,"captured correction exceeds position/angular comparison budgets");
        assert_eq!(gpu.calls,1);
    }
    #[test]
    #[ignore = "requires real GPU and a captured rejected response batch"]
    fn gpu_captured_rejected_response_batch() {
        let path=std::env::var("VOXY_HAIR_RESPONSE_FAILURE_FIXTURE").expect("response failure fixture");
        let capture:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let requests:Vec<_>=capture["requests"].as_array().unwrap().iter().map(|case|HairResponseSystem {
            system:HairLinearSystem {band_width:9,matrix:serde_json::from_value(case["matrix"].clone()).unwrap(),
                rhs:serde_json::from_value(case["rhs"].clone()).unwrap(),
                active:case["first"].as_u64().unwrap() as usize..case["end"].as_u64().unwrap() as usize},
            loads:serde_json::from_value(case["loads"].clone()).unwrap(),
        }).collect();
        assert!(!requests.is_empty());
        let native=requests.iter().map(HairResponseSystem::solve_native).collect::<Result<Vec<_>,_>>().unwrap();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("REJECTED RESPONSE REPLAY ADAPTER {:?} requests={} captured_error={}",adapter.get_info(),requests.len(),capture["error"]);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.residual_refinements=1;gpu.contact_residual_refinements=Some(1);gpu.contact_response_batches=true;
        gpu.batch_response_waves=true;gpu.compact_response_readback=true;gpu.gpu_response_transport=true;
        let actual=gpu.solve_responses(&requests).unwrap_or_else(|error|panic!("{error}: {:?}",gpu.last_error));
        let mut maximum=0f64;
        for ((request,expected),actual) in requests.iter().zip(native).zip(actual) {
            for ((load,expected),actual) in request.loads.iter().zip(expected).zip(actual) {
                request.system.validate_load_correction(&actual,load).unwrap();
                let scale=expected.iter().map(|value|value.abs()).fold(1e-30,f64::max);
                maximum=maximum.max(expected.iter().zip(actual).map(|(a,b)|(a-b).abs()/scale).fold(0.,f64::max));
            }
        }
        eprintln!("REJECTED RESPONSE REPLAY maximum_relative_native_difference={maximum:e}");
    }
    #[test]
    #[ignore = "requires real GPU and exported captured contact matrices"]
    fn gpu_captured_body_contact_responses() {
        let path=std::env::var("VOXY_HAIR_CONTACT_REPLAY_MATRICES").expect("captured matrix fixture path");
        let cases:Vec<serde_json::Value>=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(cases.len(),4);
        let requests:Vec<_>=cases.iter().map(|case|HairResponseSystem {
            system:HairLinearSystem {band_width:9,
                matrix:serde_json::from_value(case["matrix"].clone()).unwrap(),
                rhs:serde_json::from_value(case["rhs"].clone()).unwrap(),
                active:case["first"].as_u64().unwrap() as usize..case["end"].as_u64().unwrap() as usize},
            loads:serde_json::from_value(case["loads"].clone()).unwrap(),
        }).collect();
        let native=requests.iter().map(HairResponseSystem::solve_native).collect::<Result<Vec<_>,_>>().unwrap();
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("CAPTURED CONTACT ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.residual_refinements=1;gpu.contact_response_batches=true;
        gpu.batch_response_waves=true;gpu.compact_response_readback=true;gpu.gpu_response_transport=true;
        let actual=gpu.solve_responses(&requests).unwrap_or_else(|error|panic!("{error}: {:?}",gpu.last_error));
        assert_eq!(actual.len(),requests.len(),"GPU omitted a captured contact system");
        assert_eq!(native.len(),requests.len());
        for ((request,reference),actual) in requests.iter().zip(&native).zip(&actual) {
            assert_eq!(actual.len(),request.loads.len(),"GPU omitted a captured contact load");
            assert_eq!(reference.len(),request.loads.len());
            for ((load,reference),actual) in request.loads.iter().zip(reference).zip(actual) {
                assert_eq!(actual.len(),reference.len(),"GPU truncated a captured correction");
                assert_eq!(actual.len(),load.len());
                request.system.validate_load_correction(actual,load).unwrap();
                let scale=reference.iter().map(|value|value.abs()).fold(1e-30,f64::max);
                let error=reference.iter().zip(actual).map(|(a,b)|(a-b).abs()/scale).fold(0.,f64::max);
                eprintln!("CAPTURED CONTACT RESPONSE relative_error={error:e}");
                assert!(error<1e-8);
            }
        }
        if let Ok(path)=std::env::var("VOXY_HAIR_CONTACT_REPLAY_RESPONSES") {
            assert!(std::path::Path::new(&path).is_absolute());
            std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"native":native,"gpu":actual})).unwrap()).unwrap();
        }
    }
    #[test]
    #[ignore = "requires a real GPU for ragged compliance batch qualification"]
    fn gpu_shared_response_batches_preserve_mixed_shapes_and_load_order() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("SHARED RESPONSE ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.residual_refinements=1;
        let mut requests=Vec::new();
        for (index,(segments,count)) in [(20,4),(8,2),(20,1),(12,0)].into_iter().enumerate() {
            let curve=(0..=segments).map(|point|[index as f64*0.001,point as f64*0.01,0.0001*(point as f64*0.3).sin()]).collect();
            let mut system=physics::hair::HairRod::new(curve,Default::default()).unwrap().linear_system(1./240.).unwrap();
            for value in &mut system.matrix {*value*=(1./240.)*(1./240.);}
            system.rhs.fill(0.);
            let loads=(0..count).map(|load| {
                let mut force=vec![0.;system.rhs.len()];
                let point=1+(load*3+index)%segments;
                force[point*6]=(load+1) as f64*1e-7;force[point*6+2]=-0.3e-7;
                force
            }).collect();
            requests.push(HairResponseSystem {system,loads});
        }
        let expected=requests.iter().map(HairResponseSystem::solve_native).collect::<Result<Vec<_>,_>>().unwrap();
        let result=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
        let mut max_relative=0f64;
        for ((request,expected),actual) in requests.iter().zip(&expected).zip(&result) {
            assert_eq!(actual.len(),request.loads.len());
            for ((load,expected),actual) in request.loads.iter().zip(expected).zip(actual) {
                request.system.validate_load_correction(actual,load).unwrap();
                let scale=expected.iter().map(|v|v.abs()).fold(1e-30,f64::max);
                for (a,b) in actual.iter().zip(expected) {max_relative=max_relative.max((a-b).abs()/scale);}
            }
        }
        assert!(max_relative<1e-8,"shared response error {max_relative}");
        assert_eq!(gpu.response_dispatches,12);
        let legacy=result.clone();
        gpu.batch_response_waves=true;gpu.response_dispatches=0;
        let batched=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
        assert_eq!(legacy,batched,"ordered waves changed arithmetic");
        assert_eq!(gpu.response_dispatches,12);
        assert_eq!(gpu.response_submissions,4);
        gpu.compact_response_readback=true;
        let compact=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
        assert_eq!(legacy,compact,"compact readback changed arithmetic");
        let snapshot_resident=|gpu:&GpuHairLinearSolver| {
            let mut encoder=gpu.device.create_command_encoder(&Default::default());
            let dispatch=gpu.resident.as_ref().unwrap().encode_snapshot(&mut encoder).unwrap();
            gpu.queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
            gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            read.try_read().unwrap().unwrap()
        };
        let resident_before=snapshot_resident(&gpu);
        gpu.gpu_response_transport=true;
        let transported=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
        assert_eq!(legacy,transported,"GPU transport changed arithmetic");
        assert_eq!(resident_before,snapshot_resident(&gpu),"GPU transport altered resident factors/header/status or solutions");
        assert!(gpu.response_transfer_dispatches>0);

        let dispatches_before_rejection=gpu.response_dispatches;

        assert_eq!(gpu.calls,0);
        let mut bad=requests.clone();bad[0].loads[0][0]=1.;
        assert!(gpu.solve_responses(&bad).is_err());
        assert_eq!(gpu.response_dispatches,dispatches_before_rejection,"invalid batch must be rejected before GPU work");
        // Exceed the bounded snapshot chunk and retain every requested column.
        let mut large=vec![requests[0].clone()];
        large[0].loads=(0..17).map(|i|requests[0].loads[i%4].clone()).collect();
        gpu.batch_response_waves=false;
        let separate=gpu.solve_responses(&large).unwrap();
        gpu.batch_response_waves=true;
        let submissions=gpu.response_submissions;
        let grouped=gpu.solve_responses(&large).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
        assert_eq!(separate,grouped);
        assert_eq!(grouped[0].len(),17);
        assert_eq!(gpu.response_submissions-submissions,6,"three ordered chunks per refinement stage");

        for refinement in 1..=3 {
            gpu.contact_residual_refinements=Some(refinement);
            let values=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
            let mut error=0f64;
            for (expected,actual) in expected.iter().zip(&values) {for (expected,actual) in expected.iter().zip(actual) {
                let scale=expected.iter().map(|v|v.abs()).fold(1e-30,f64::max);
                for (a,b) in actual.iter().zip(expected) {error=error.max((a-b).abs()/scale);}
            }}
            assert!(error<1e-8);
            assert_eq!(gpu.residual_refinements,1,"contact-only precision changed structural policy");
            eprintln!("CONTACT PRECISION QUALIFIED refinements={refinement} max_relative_error={error}");
        }
        let dispatches=gpu.response_dispatches;gpu.contact_residual_refinements=Some(4);
        assert!(gpu.solve_responses(&requests).is_err());assert_eq!(gpu.response_dispatches,dispatches);
        eprintln!("SHARED RESPONSE QUALIFIED requests=4 loads=7 mixed_shapes=true dispatches=12 max_relative_error={max_relative}");
    }
    #[test]
    #[ignore = "requires a real GPU; transport benchmark is not render FPS"]
    fn gpu_transport_benchmark_full_guide_count() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("TRANSPORT BENCH ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        gpu.residual_refinements=1;gpu.batch_response_waves=true;gpu.compact_response_readback=true;
        let mut requests=Vec::new();
        for index in 0..469 {
            let curve=(0..=20).map(|point|[index as f64*0.001,point as f64*0.01,0.0001*(point as f64*0.3+index as f64*0.001).sin()]).collect();
            let mut system=physics::hair::HairRod::new(curve,Default::default()).unwrap().linear_system(1./240.).unwrap();
            for value in &mut system.matrix {*value*=(1./240.)*(1./240.);}
            system.rhs.fill(0.);
            let loads=(0..17).map(|load| {let mut rhs=vec![0.;system.rhs.len()];let point=1+(load*3+index)%20;rhs[point*6]=(load+1) as f64*1e-7;rhs[point*6+2]=-0.3e-7;rhs}).collect();
            requests.push(HairResponseSystem {system,loads});
        }
        let reference=gpu.solve_responses(&requests).unwrap();
        gpu.gpu_response_transport=true;
        assert_eq!(reference,gpu.solve_responses(&requests).unwrap());
        let mut timings=[Vec::new(),Vec::new()];
        for iteration in 0..4 {
            for mode in [iteration%2,1-iteration%2] {
                gpu.gpu_response_transport=mode==1;
                let started=std::time::Instant::now();
                let result=gpu.solve_responses(&requests).unwrap_or_else(|e|panic!("{e}: {:?}",gpu.last_error));
                timings[mode].push(started.elapsed().as_secs_f64()*1000.);
                assert_eq!(reference,result,"transport changed the 469-guide response batch");
            }
        }
        eprintln!("TRANSPORT BENCH guides=469 loads_per_guide=17 refinements=1 host_copy_ms={:?} shader_transport_ms={:?} bitwise_equal=true",timings[0],timings[1]);
    }
    #[test]
    #[ignore = "requires a real GPU for factor-reuse qualification"]
    fn gpu_factor_reuse_matches_fresh_factorization_on_rod_compliance() {
        let instance=voxy_render::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        eprintln!("FACTOR REUSE ADAPTER {:?}",adapter.get_info());
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut fresh=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        let mut cached=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
        fresh.reuse_refinement_factors=false;
        cached.reuse_refinement_factors=true;
        let dt=1./240.;
        let count=std::env::var("VOXY_HAIR_FACTOR_REUSE_SYSTEMS").ok().map(|v|v.parse::<usize>().unwrap()).unwrap_or(3);
        assert!((1..=1024).contains(&count));
        let mut systems:Vec<_>=(0..count).map(|rod| {
            let curve=(0..=20).map(|p|[rod as f64*0.001+p as f64*0.0001,p as f64*0.01,0.0002*(p as f64*0.2).sin()]).collect();
            let mut system=physics::hair::HairRod::new(curve,Default::default()).unwrap().linear_system(dt).unwrap();
            for v in &mut system.matrix {*v*=dt*dt;}
            system
        }).collect();
        for refinements in 1..=3 {
            fresh.residual_refinements=refinements;cached.residual_refinements=refinements;
            for batch in 0..4 {
                for (rod,system) in systems.iter_mut().enumerate() {
                    system.rhs.fill(0.);
                    let point=1+(batch*5+rod)%20;
                    system.rhs[point*6]=1e-6;
                    system.rhs[point*6+2]=-0.3e-6;
                }
                let a=fresh.solve(&systems).unwrap_or_else(|e|panic!("{e}: {:?}",fresh.last_error));
                let b=cached.solve(&systems).unwrap_or_else(|e|panic!("{e}: {:?}",cached.last_error));
                assert_eq!(a,b,"reusing factors must preserve every correction bit");
            }
        }
        assert_eq!(cached.reused_factor_dispatches,24);
        assert_eq!(fresh.reused_factor_dispatches,0);
        eprintln!("FACTOR REUSE QUALIFIED batches=12 systems={} reused_dispatches=24 corrections=bitwise fresh_ms={} cached_ms={}",count,fresh.elapsed_ms,cached.elapsed_ms);
    }
}
