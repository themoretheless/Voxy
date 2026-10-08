//! Checked hybrid bridge. Matrix assembly/contact ownership stays in physics.
//! Blocking correction readback is a qualification path, not a real-time claim.
use physics::hair::{HairLinearSolver, HairLinearSystem};
use voxy_render::{BandedSolveInput,BandedSystem,ComputeJob,ComputeProgram,BANDED_SOLVE_SHADER};
#[derive(Debug)]
pub struct GpuHairLinearSolver {
    device:wgpu::Device,queue:wgpu::Queue,program:ComputeProgram,resident:Option<ComputeJob>,
    pub reference_audit:bool,
    pub max_linear_error:[f64;2],
    pub max_packing_error:[f64;2],
    /// Correction errors from equilibration, input packing and GPU arithmetic.
    pub max_stage_error:[[f64;2];3],
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
        Ok(Self {device:device.clone(),queue:queue.clone(),program,resident:None,reference_audit:false,max_linear_error:[0.;2],max_packing_error:[0.;2],max_stage_error:[[0.;2];3],calls:0,elapsed_ms:0.,last_error:None})
    }
    fn solve_checked(&mut self,systems:&[HairLinearSystem])->Result<Vec<Vec<f64>>,Box<dyn std::error::Error>> {
        let first=systems.first().ok_or("empty hair accelerator batch")?;
        if systems.iter().any(|s|s.band_width!=9 || s.active!=first.active) {return Err("inconsistent hair accelerator batch".into());}
        let rows:Vec<_>=systems.iter().map(|s|BandedSystem {matrix:&s.matrix,rhs:&s.rhs}).collect();
        let input=BandedSolveInput::new_with_underflow_tolerance(&rows,first.active.clone(),1e-40)?;
        let packing_error=input.packing_error();
        for kind in 0..2 {self.max_packing_error[kind]=self.max_packing_error[kind].max(packing_error[kind]);}
        if self.resident.as_ref().is_none_or(|job|job.buffer().size()!=input.bytes().len() as u64) {
            self.resident=Some(self.program.create_job(&self.device,input.bytes())?);
        } else {self.queue.write_buffer(self.resident.as_ref().unwrap().buffer(),0,input.bytes());}
        let job=self.resident.as_ref().unwrap();
        let mut encoder=self.device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder,input.dispatch())?;
        let dispatch=job.encode_snapshot(&mut encoder)?;self.queue.submit([encoder.finish()]);
        let mut read=dispatch.begin_read();self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes=read.try_read()?.ok_or("hair accelerator readback pending")?;
        let corrections=input.decode_checked(&bytes,1e-8)?;
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
            eprintln!("GPU HAIR LINEAR AUDIT call={} position_correction_error_m={} rotation_correction_error_rad={} worst_rods={:?} packing_errors={:?} stage_errors={:?}",self.calls,errors[0],errors[1],worst,packing_error,stages);
        }
        Ok(corrections)
    }
}
impl HairLinearSolver for GpuHairLinearSolver {
    fn solve(&mut self,systems:&[HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {
        let started=std::time::Instant::now();self.calls+=1;
        let result=self.solve_checked(systems);self.elapsed_ms+=started.elapsed().as_secs_f64()*1000.;
        match result {Ok(corrections)=>{self.last_error=None;Ok(corrections)},Err(error)=>{self.last_error=Some(error.to_string());Err("checked GPU hair linear solver failed")}}
    }
}
