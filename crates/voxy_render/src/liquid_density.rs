//! Transport and compute candidate only; original physics owns admission.
#[derive(Clone, Copy, Debug)]
pub struct LiquidDensityParticle {
    pub position: [f64; 3],
    pub mass: f64,
    pub rest_density: f64,
    pub carrier_droplet: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn density_shader_validates_and_unwritten_or_changed_inputs_reject() {
        let module=naga::front::wgsl::parse_str(LIQUID_DENSITY_SHADER).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::empty())
            .validate(&module).unwrap();
        let particles=[LiquidDensityParticle {position:[0.;3],mass:1.,rest_density:1000.,carrier_droplet:false}];
        let input=LiquidDensityInput::new(&particles,&[],1.,LiquidDensityKernel::MassPoly6).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words=input.words.clone();words[input.output..input.output+2].copy_from_slice(&pack(1.).unwrap());
        words[input.output+2]=0;
        assert_eq!(input.candidate(bytemuck::cast_slice(&words)).unwrap(),vec![1.]);
        for offset in 0..input.output {
            let mut corrupt=words.clone();corrupt[offset]^=1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        assert!(LiquidDensityInput::new(&particles,&[(0,1)],1.,LiquidDensityKernel::MassPoly6).is_err());
    }

    #[test]
    #[ignore="requires actual GPU; density candidate only, not a complete fluid step"]
    fn gpu_density_preserves_three_current_physical_formulations() {
        use physics::liquid::{Formulation,LiquidDensityOperator,Material,Particle};
        let instance=crate::GraphicsOptions::default().create_instance();
        let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let compute=pollster::block_on(LiquidDensityCompute::new(&device)).unwrap();
        let particles=(0..70).map(|i|Particle {position:[1e8+(i%7) as f64*0.01,
            (i/7) as f64*0.01,0.],velocity:[0.;3],mass:0.001*(1.+(i%3) as f64),material:i%2}).collect::<Vec<_>>();
        let properties=particles.iter().map(|p|if p.material==0 {Material::WATER} else {Material::OIL}).collect::<Vec<_>>();
        let flags=(0..70).map(|i|i%13==0).collect::<Vec<_>>();
        let pairs=(0..70).flat_map(|i|(i+1..70).map(move|j|(i,j))).collect::<Vec<_>>();
        let upload=particles.iter().zip(&properties).zip(&flags).map(|((p,m),&flag)|LiquidDensityParticle {
            position:p.position,mass:p.mass,rest_density:m.rest_density,carrier_droplet:flag}).collect::<Vec<_>>();
        for h in [0.2,1.] {
            for (formulation,kernel) in [(Formulation::MassDensity,LiquidDensityKernel::MassPoly6),
                (Formulation::RestVolume,LiquidDensityKernel::VolumePoly6),
                (Formulation::RestVolumeWendland,LiquidDensityKernel::VolumeWendland)] {
                let input=LiquidDensityInput::new(&upload,&pairs,h,kernel).unwrap();
                let budget=crate::ComputeMemoryBudget::for_device(&device);
                let before=budget.stats().allocated_bytes;
                let bytes=input.bytes().len() as u64;
                let mut encoder=device.create_command_encoder(&Default::default());
                let dispatch=compute.encode(&device,&mut encoder,input).unwrap();
                assert_eq!(budget.stats().allocated_bytes,before+bytes);
                let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
                assert_eq!(budget.stats().allocated_bytes,before+bytes);
                device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
                let candidate=read.try_candidate().unwrap().unwrap();
                assert_eq!(budget.stats().allocated_bytes,before);
                assert!(matches!(read.try_candidate(),Err(LiquidDensityReadError::Compute(crate::ComputeError::Consumed))));
                let owner=LiquidDensityOperator {particles:&particles,pairs:&pairs,properties:&properties,
                    formulation,smoothing_radius:h,carrier_droplets:Some(&flags)};
                owner.admit(&candidate,0.,2e-11).unwrap();
                let reference=owner.evaluate_native().unwrap();
                let error=candidate.iter().zip(reference).map(|(a,b)|(a-b).abs()/b).fold(0f64,f64::max);
                eprintln!("LIQUID GPU DENSITY particles=70 pairs={} formulation={formulation:?} h={h} maximum_relative_error={error:e}",pairs.len());
            }
        }
        use physics::liquid::{Config,Liquid};
        for formulation in [Formulation::MassDensity,Formulation::RestVolume,Formulation::RestVolumeWendland] {
            let mut native=Liquid::new(particles.clone(),vec![Material::WATER,Material::OIL],
                Config {smoothing_radius:0.2,..Config::default()}).unwrap();
            native.set_formulation(formulation);let mut accelerated=native.clone();
            let mut backend=|owner:&LiquidDensityOperator<'_>| {
                let upload=owner.particles.iter().zip(owner.properties).enumerate().map(|(i,(p,m))|LiquidDensityParticle {
                    position:p.position,mass:p.mass,rest_density:m.rest_density,
                    carrier_droplet:owner.carrier_droplets.is_some_and(|flags|flags[i])}).collect::<Vec<_>>();
                let kernel=match owner.formulation {Formulation::MassDensity=>LiquidDensityKernel::MassPoly6,
                    Formulation::RestVolume=>LiquidDensityKernel::VolumePoly6,
                    Formulation::RestVolumeWendland=>LiquidDensityKernel::VolumeWendland};
                let input=LiquidDensityInput::new(&upload,owner.pairs,owner.smoothing_radius,kernel).ok()?;
                let mut encoder=device.create_command_encoder(&Default::default());
                let dispatch=compute.encode(&device,&mut encoder,input).ok()?;
                let submission=queue.submit([encoder.finish()]);let mut read=dispatch.begin_read();
                device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).ok()?;
                read.try_candidate().ok()?
            };
            let mut admissions=0;
            for _ in 0..3 {
                native.step(1e-6,None).unwrap();
                let (_,counts)=accelerated.step_with_density_backend(1e-6,None,&mut backend,0.,2e-11).unwrap();
                assert!(counts.admitted>0);assert_eq!(counts.native_fallbacks,0);admissions+=counts.admitted;
                for (a,b) in native.particles().iter().zip(accelerated.particles()) {
                    assert_eq!(a.mass,b.mass);assert_eq!(a.material,b.material);
                    for axis in 0..3 {
                        assert!((a.position[axis]-b.position[axis]).abs()<1e-10);
                        assert!((a.velocity[axis]-b.velocity[axis]).abs()<1e-9);
                    }
                }
            }
            eprintln!("LIQUID GPU DENSITY STEP formulation={formulation:?} steps=3 admitted={admissions} native_fallbacks=0 scope=density_only_forces_native");
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidDensityKernel { MassPoly6, VolumePoly6, VolumeWendland }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidDensityError { Shape, Capacity, NumericRange, Output }
pub const LIQUID_DENSITY_SHADER: &str = include_str!("liquid_density.wgsl");

/// Device-owned density pipeline. Submission and device polling stay with the
/// application; this interface never waits for GPU completion.
#[derive(Debug)]
pub struct LiquidDensityCompute { program: crate::ComputeProgram }
impl LiquidDensityCompute {
    pub async fn new(device: &wgpu::Device) -> Result<Self, crate::ComputeError> {
        Ok(Self { program: crate::ComputeProgram::new(device, LIQUID_DENSITY_SHADER).await? })
    }

    /// Record computation and an independent snapshot in the caller's encoder.
    /// Submit that encoder before starting the read. Inputs are retained until
    /// decoding so immutable data and the entire output remain auditable.
    pub fn encode(&self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder,
        input: LiquidDensityInput) -> Result<LiquidDensityDispatch, crate::ComputeError> {
        let job = self.program.create_job(device, input.bytes())?;
        job.encode_step(encoder, input.workgroups())?;
        let snapshot = job.encode_snapshot(encoder)?;
        Ok(LiquidDensityDispatch { input, snapshot, job })
    }
}

#[derive(Debug)]
pub struct LiquidDensityDispatch {
    input: LiquidDensityInput,
    snapshot: crate::ComputeDispatch,
    job: crate::ComputeJob,
}
impl LiquidDensityDispatch {
    /// Must be called after command submission, following the shared compute ABI.
    pub fn begin_read(self) -> PendingLiquidDensity {
        PendingLiquidDensity { input: self.input, read: self.snapshot.begin_read(), job: Some(self.job) }
    }
}

#[derive(Debug)]
pub enum LiquidDensityReadError {
    Compute(crate::ComputeError),
    Candidate(LiquidDensityError),
}
pub struct PendingLiquidDensity {
    input: LiquidDensityInput,
    read: crate::PendingComputeReadback,
    // Keep storage charged while the encoded computation/snapshot is in flight.
    job: Option<crate::ComputeJob>,
}
impl PendingLiquidDensity {
    /// None means mapping is still pending. A returned candidate still requires
    /// admission against the current physical inputs before state publication.
    pub fn try_candidate(&mut self) -> Result<Option<Vec<f64>>, LiquidDensityReadError> {
        match self.read.try_read() {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) => {
                self.job.take();
                self.input.candidate(&bytes).map(Some).map_err(LiquidDensityReadError::Candidate)
            }
            Err(error) => {
                self.job.take();
                Err(LiquidDensityReadError::Compute(error))
            }
        }
    }
}

#[derive(Debug)]
pub struct LiquidDensityInput { words: Vec<u32>, output: usize, count: usize }
fn pack(value:f64)->Result<[u32;2],LiquidDensityError> {
    let hi=value as f32;let lo=(value-f64::from(hi)) as f32;
    if !hi.is_finite() || !lo.is_finite() || (value!=0. && hi==0. && lo==0.) {
        return Err(LiquidDensityError::NumericRange);
    }
    Ok([hi.to_bits(),lo.to_bits()])
}
impl LiquidDensityInput {
    /// CSR adjacency preserves each particle's original pair accumulation order.
    pub fn new(particles:&[LiquidDensityParticle],pairs:&[(usize,usize)],h:f64,kernel:LiquidDensityKernel)
        ->Result<Self,LiquidDensityError> {
        let topology=crate::LiquidPairTopology::new(particles.len(),pairs)?;
        Self::with_topology(particles,&topology,h,kernel)
    }
    pub(crate) fn with_topology(particles:&[LiquidDensityParticle],topology:&crate::LiquidPairTopology,h:f64,kernel:LiquidDensityKernel)->Result<Self,LiquidDensityError> {
        if particles.len()!=topology.particle_count() {return Err(LiquidDensityError::Shape);}
        let pairs=topology.pairs();
        let n=particles.len();
        if n==0 || !h.is_finite() || h<=0. {return Err(LiquidDensityError::Shape);}
        let output=8usize.checked_add(n.checked_mul(12).ok_or(LiquidDensityError::Capacity)?)
            .and_then(|v|v.checked_add(pairs.len().checked_mul(4)?)).and_then(|v|v.checked_add(1))
            .ok_or(LiquidDensityError::Capacity)?;
        let total=output.checked_add(n.checked_mul(3).ok_or(LiquidDensityError::Capacity)?)
            .filter(|&v|v<=16*1024*1024).ok_or(LiquidDensityError::Capacity)?;
        let offsets=8+11*n+2*pairs.len();let adjacency=offsets+n+1;
        let coefficient=match kernel {LiquidDensityKernel::VolumeWendland=>21./(2.*std::f64::consts::PI*h.powi(3)),
            _=>315./(64.*std::f64::consts::PI*h.powi(9))*h.powi(6)};
        let coefficient=pack(coefficient)?;
        let mode=match kernel {LiquidDensityKernel::MassPoly6=>0,LiquidDensityKernel::VolumePoly6=>1,LiquidDensityKernel::VolumeWendland=>2};
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|LiquidDensityError::Capacity)?;
        words.extend([n as u32,mode,offsets as u32,adjacency as u32,output as u32,0,coefficient[0],coefficient[1]]);
        let origin=particles[0].position;
        for particle in particles {
            if !particle.mass.is_finite() || particle.mass<=0. || !particle.rest_density.is_finite() || particle.rest_density<=0. {
                return Err(LiquidDensityError::Shape);
            }
            for (value,base) in particle.position.into_iter().zip(origin) {
                if !value.is_finite() {return Err(LiquidDensityError::Shape);}
                words.extend(pack((value-base)/h)?);
            }
            words.extend(pack(particle.mass)?);words.extend(pack(particle.rest_density)?);
            words.push(u32::from(particle.carrier_droplet));
        }
        for &(i,j) in pairs {words.extend([i as u32,j as u32]);}
        topology.append_csr(&mut words);
        assert_eq!(words.len(),output);
        words.resize(total,u32::MAX);
        Ok(Self {words,output,count:n})
    }
    pub(crate) fn count(&self)->usize {self.count}
    pub(crate) fn density_output_range(&self,index:usize)->Option<(u64,u64)> {
        (index<self.count).then(||(((self.output+3*index)*4) as u64,8))
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn workgroups(&self)->[u32;3] {[(self.count as u32).div_ceil(64),1,1]}
    /// Complete ABI/status/finite audit; physics must still admit these densities.
    pub fn candidate(&self,bytes:&[u8])->Result<Vec<f64>,LiquidDensityError> {
        if bytes.len()!=self.words.len()*4 || bytes[..self.output*4]!=self.bytes()[..self.output*4] {
            return Err(LiquidDensityError::Output);
        }
        let word=|i:usize|u32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap());
        (0..self.count).map(|i| {
            let offset=self.output+3*i;let hi=f32::from_bits(word(offset));let lo=f32::from_bits(word(offset+1));
            let value=f64::from(hi)+f64::from(lo);
            if word(offset+2)!=0 || !hi.is_finite() || !lo.is_finite() || value<=0. {
                Err(LiquidDensityError::Output)
            } else {Ok(value)}
        }).collect()
    }
}
