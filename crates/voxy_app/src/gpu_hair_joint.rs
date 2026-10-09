//! Resident GPU equality backend under physics-owned active-set selection.
use super::*;
use voxy_render::{ResidentContactEqualityInput,JOINT_CONTACT_QR_SHADER,JOINT_CONTACT_EQUALITY_SHADER};
impl GpuHairLinearSolver {
    pub(super) fn solve_joint_checked(&mut self,columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)
        ->Result<Option<(Vec<f64>,Vec<f64>)>,Box<dyn std::error::Error>> {
        if !self.joint_contact_qr {return Ok(None);}
        if self.joint_programs.is_none() {
            let qr=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_QR_SHADER))?;
            let solve=pollster::block_on(ComputeProgram::new(&self.device,JOINT_CONTACT_EQUALITY_SHADER))?;
            self.joint_programs=Some((qr,solve));
        }
        let (qr,solve)=self.joint_programs.as_ref().unwrap();let mut error=None;
        let output=HairResponseSystem::solve_contact_coordinates_with_equality_accelerator(columns,bounds,tolerance,
            |columns,bounds,_| {
                self.joint_equality_dispatches+=1;
                let result=(||->Result<_,Box<dyn std::error::Error>> {
                    let input=ResidentContactEqualityInput::new(columns,bounds)?;
                    let mut job=qr.create_job(&self.device,input.bytes())?;
                    let mut encoder=self.device.create_command_encoder(&Default::default());
                    for _ in 0..input.columns() {job.encode_step(&mut encoder,[1,1,1])?;}
                    job.use_program(solve)?;job.encode_step(&mut encoder,[1,1,1])?;
                    let dispatch=job.encode_readback(&mut encoder)?;
                    self.queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
                    self.device.poll(wgpu::PollType::wait_indefinitely())?;
                    let bytes=read.try_read()?.ok_or("pending GPU joint contact readback")?;
                    let output=input.decode(&bytes)?;Ok((output.coordinates,output.reactions))
                })();
                match result {Ok(output)=>Some(output),Err(reason)=>{error=Some(reason);None}}
            });
        if let Some(error)=error {return Err(error);}
        if output.is_none() {self.last_error=Some("GPU joint active contacts did not converge".into());}
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
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
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut gpu=pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();gpu.joint_contact_qr=true;
        let (responses,reactions,accelerated)=HairResponseSystem::solve_joint_load_inequalities_accelerated(&requests,&bounds,tolerance,
            |columns,bounds,tolerance|gpu.solve_joint_coordinates(columns,bounds,tolerance)).unwrap();
        gpu.joint_contact_result(accelerated);
        assert!(accelerated,"GPU was not admitted: {:?}",gpu.last_error);
        HairResponseSystem::validate_joint_solution(&requests,&bounds,&responses,&reactions,tolerance).unwrap();
        assert!(gpu.joint_coordinate_calls>0&&gpu.joint_equality_dispatches>0);
        assert_eq!(gpu.joint_admitted,1);assert_eq!(gpu.joint_native_fallbacks,0);
        eprintln!("JOINT BACKEND ORIGINAL systems={systems} rows={rows} width={width} tolerance={tolerance:e} coordinate_calls={} equality_dispatches={} admitted={} fallbacks={}",gpu.joint_coordinate_calls,gpu.joint_equality_dispatches,gpu.joint_admitted,gpu.joint_native_fallbacks);
    }
}
