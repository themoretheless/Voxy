//! Checked hybrid bridge. Matrix assembly/contact ownership stays in physics.
//! Blocking correction readback is a qualification path, not a real-time claim.
use physics::hair::{HairLinearSolver, HairLinearSystem};
use voxy_render::{BandedSolveInput,BandedSystem,ComputeJob,ComputeProgram,BANDED_SOLVE_SHADER};
#[derive(Debug)]
pub struct GpuHairLinearSolver {
    device:wgpu::Device,queue:wgpu::Queue,program:ComputeProgram,resident:Option<ComputeJob>,
    pub reference_audit:bool,
    pub max_linear_error:[f64;2],
    pub calls:usize,pub elapsed_ms:f64,pub last_error:Option<String>,
}
impl GpuHairLinearSolver {
    pub async fn new(device:&wgpu::Device,queue:&wgpu::Queue)->Result<Self,voxy_render::ComputeError> {
        Self::new_with_extra_division_refinement(device,queue,false).await
    }
    /// Qualify a second compensated division residual update. This changes
    /// arithmetic accuracy only; the physical matrices and residual gates stay fixed.
    pub async fn new_with_extra_division_refinement(device:&wgpu::Device,queue:&wgpu::Queue,extra:bool)->Result<Self,voxy_render::ComputeError> {
        let source=if extra {
            let declaration="const division_refinements:u32=1u;";
            if BANDED_SOLVE_SHADER.matches(declaration).count()!=1 {
                return Err(voxy_render::ComputeError::Validation("missing unique division refinement declaration".into()));
            }
            std::borrow::Cow::Owned(BANDED_SOLVE_SHADER.replace(declaration,"const division_refinements:u32=2u;"))
        } else {std::borrow::Cow::Borrowed(BANDED_SOLVE_SHADER)};
        let program=ComputeProgram::new(device,&source).await?;
        Ok(Self {device:device.clone(),queue:queue.clone(),program,resident:None,reference_audit:false,max_linear_error:[0.;2],calls:0,elapsed_ms:0.,last_error:None})
    }
    fn solve_checked(&mut self,systems:&[HairLinearSystem])->Result<Vec<Vec<f64>>,Box<dyn std::error::Error>> {
        let first=systems.first().ok_or("empty hair accelerator batch")?;
        if systems.iter().any(|s|s.band_width!=9 || s.active!=first.active) {return Err("inconsistent hair accelerator batch".into());}
        let rows:Vec<_>=systems.iter().map(|s|BandedSystem {matrix:&s.matrix,rhs:&s.rhs}).collect();
        let input=BandedSolveInput::new_with_underflow_tolerance(&rows,first.active.clone(),1e-40)?;
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
            for (index,(system,correction)) in systems.iter().zip(&corrections).enumerate() {
                let native=system.solve_native()?;
                for i in system.active.clone() {
                    let kind=usize::from(i%6>=3);let error=(native[i]-correction[i]).abs();
                    if error>errors[kind] {errors[kind]=error;worst[kind]=index;}
                }
            }
            for kind in 0..2 {self.max_linear_error[kind]=self.max_linear_error[kind].max(errors[kind]);}
            eprintln!("GPU HAIR LINEAR AUDIT call={} position_correction_error_m={} rotation_correction_error_rad={} worst_rods={:?}",self.calls,errors[0],errors[1],worst);
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
