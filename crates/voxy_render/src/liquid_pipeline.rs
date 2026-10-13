//! Density, supported normals/pressure, pair preparation/forces and accumulation.
//! One caller submission and final read; original physics owns admission.
use crate::*;
#[derive(Debug)]
pub struct LiquidPipeline {density:ComputeProgram,normals:ComputeProgram,prepare:ComputeProgram,forces:ComputeProgram,accumulation:ComputeProgram,prepare_accumulation:ComputeProgram}
#[derive(Debug)]
pub struct LiquidPipelineInput {fields:LiquidDensityNormalInput,forces:LiquidPreparedPairForcesInput,accumulation:crate::liquid_accumulation::LiquidPreparedAccumulationInput}
#[derive(Clone,Debug,PartialEq)]
pub struct LiquidPipelineCandidate {pub fields:LiquidDensityNormalCandidate,pub forces:Vec<LiquidPairForceCandidate>,pub accumulation:LiquidAccumulationCandidate}
impl LiquidPipelineInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(particles:&[LiquidDensityParticle],materials:&[u32],pairs:&[(usize,usize)],h:f64,kernel:LiquidDensityKernel,
        support:&[(usize,f64)],laws:&[LiquidPressureParameters],parameters:&[LiquidPairForceParameters],gravity:[f64;3])
        ->Result<Self,LiquidDensityError> {
        let topology=LiquidPairTopology::new(particles.len(),pairs)?;
        Self::with_topology(particles,materials,&topology,h,kernel,support,laws,parameters,gravity)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn with_topology(particles:&[LiquidDensityParticle],materials:&[u32],topology:&LiquidPairTopology,h:f64,kernel:LiquidDensityKernel,
        support:&[(usize,f64)],laws:&[LiquidPressureParameters],parameters:&[LiquidPairForceParameters],gravity:[f64;3])->Result<Self,LiquidDensityError> {
        let pairs=topology.pairs();
        if topology.particle_count()!=particles.len() {return Err(LiquidDensityError::Shape);}
        if pairs.len()!=parameters.len() {return Err(LiquidDensityError::Shape);}
        for (&(i,j),p) in pairs.iter().zip(parameters) {
            let a=particles.get(i).ok_or(LiquidDensityError::Shape)?;let b=particles.get(j).ok_or(LiquidDensityError::Shape)?;
            if p.mass!=[a.mass,b.mass] || p.reference_density!=[a.rest_density,b.rest_density]
                || p.delta!=std::array::from_fn(|axis|a.position[axis]-b.position[axis])
                || p.smoothing_radius!=h || p.kernel!=kernel {return Err(LiquidDensityError::Shape);}
        }
        let fields=LiquidDensityNormalInput::with_topology(particles,materials,topology,h,kernel,support,Some(laws))?;
        let forces=LiquidPreparedPairForcesInput::with_topology(parameters,topology)?;
        let masses=particles.iter().map(|p|p.mass).collect::<Vec<_>>();
        let zero=LiquidPairForceCandidate {force:[0.;3],viscous_conductance:0.};
        let accumulation=LiquidAccumulationInput::with_topology(&masses,topology,&vec![zero;pairs.len()],gravity)?;
        let accumulation=crate::liquid_accumulation::LiquidPreparedAccumulationInput::new(accumulation)?;
        Ok(Self {fields,forces,accumulation})
    }
    pub fn candidate(&self,bytes:&[u8])->Result<LiquidPipelineCandidate,LiquidDensityError> {
        let density_len=self.fields.density.bytes().len();let normal_len=self.fields.normals.bytes().len();
        let field_len=density_len+normal_len;let force_len=self.forces.bytes().len();
        if bytes.len()!=field_len+force_len+self.accumulation.bytes().len() {return Err(LiquidDensityError::Output);}
        let fields=self.fields.candidate(&bytes[..field_len])?;
        let (from,length)=self.fields.normals.pressure_fields_range()?;
        let upstream=&bytes[density_len+from as usize..density_len+(from+length) as usize];
        let force_bytes=&bytes[field_len..field_len+force_len];
        let source=self.forces.audit(force_bytes,upstream)?;
        let accumulation=self.accumulation.candidate(&bytes[field_len+force_len..],&source)?;
        let forces=source.into_values();
        Ok(LiquidPipelineCandidate {fields,forces,accumulation})
    }
}
#[derive(Debug,Default)]
pub struct LiquidPipelineWorkspace {chain:Option<crate::compute_chain::ComputeChainWorkspace>}
impl LiquidPipelineWorkspace {pub fn reuses(&self)->usize {self.chain.as_ref().map_or(0,|w|w.reuses)}}
impl LiquidPipeline {
    pub async fn new(device:&wgpu::Device)->Result<Self,ComputeError> {
        Ok(Self {density:ComputeProgram::new(device,LIQUID_DENSITY_SHADER).await?,normals:ComputeProgram::new(device,LIQUID_NORMALS_SHADER).await?,
            prepare:ComputeProgram::with_entry_point(device,LIQUID_PAIR_FORCES_SHADER,"cs_prepare").await?,
            forces:ComputeProgram::new(device,LIQUID_PAIR_FORCES_SHADER).await?,accumulation:ComputeProgram::new(device,LIQUID_ACCUMULATION_SHADER).await?,
            prepare_accumulation:ComputeProgram::with_entry_point(device,LIQUID_ACCUMULATION_SHADER,"cs_prepare_forces").await?})
    }
    /// Discard caller's encoder on error. Submit before begin_read; polling is external.
    pub fn encode(&self,device:&wgpu::Device,encoder:&mut wgpu::CommandEncoder,input:LiquidPipelineInput)
        ->Result<LiquidPipelineDispatch,LiquidForcePipelineError> {
        self.encode_inner(device,None,encoder,input,None)
    }
    /// Exclusively consumes completed storage. A successful typed read returns it.
    pub fn encode_reusing(&self,device:&wgpu::Device,queue:&wgpu::Queue,encoder:&mut wgpu::CommandEncoder,
        input:LiquidPipelineInput,workspace:LiquidPipelineWorkspace)->Result<LiquidPipelineDispatch,LiquidForcePipelineError> {
        self.encode_inner(device,Some(queue),encoder,input,Some(workspace))
    }
    fn encode_inner(&self,device:&wgpu::Device,queue:Option<&wgpu::Queue>,encoder:&mut wgpu::CommandEncoder,
        input:LiquidPipelineInput,workspace:Option<LiquidPipelineWorkspace>)->Result<LiquidPipelineDispatch,LiquidForcePipelineError> {
        let density=&input.fields.density;let normals=&input.fields.normals;
        let densities=(0..density.count()).map(|i| {
            let (from,length)=density.density_output_range(i).ok_or(LiquidDensityError::Shape)?;
            let (to,_)=normals.density_input_range(i).ok_or(LiquidDensityError::Shape)?;Ok((from,to,length))
        }).collect::<Result<Vec<_>,LiquidDensityError>>()?;
        let (from,length)=normals.pressure_fields_range()?;let (to,expected)=input.forces.particle_fields_range();
        if length!=expected {return Err(LiquidDensityError::Shape.into());}
        let (force_from,force_length)=input.forces.output_block_range();let (force_to,expected)=input.accumulation.fields_range();
        if force_length!=expected {return Err(LiquidDensityError::Shape.into());}
        use crate::compute_chain::ResidentStage;
        let stages=[
            ResidentStage {bytes:density.bytes(),passes:&[(&self.density,density.workgroups())],copies:&[]},
            ResidentStage {bytes:normals.bytes(),passes:&[(&self.normals,normals.workgroups())],copies:&densities},
            ResidentStage {bytes:input.forces.bytes(),passes:&[(&self.prepare,input.forces.workgroups()),(&self.forces,input.forces.workgroups())],copies:&[(from,to,length)]},
            ResidentStage {bytes:input.accumulation.bytes(),passes:&[(&self.prepare_accumulation,input.accumulation.prepare_workgroups()),(&self.accumulation,input.accumulation.workgroups())],copies:&[(force_from,force_to,force_length)]},
        ];
        let dispatch=if let Some(queue)=queue {crate::compute_chain::encode_chain_reusing(device,queue,encoder,&stages,workspace.and_then(|w|w.chain))?}
            else {crate::compute_chain::encode_chain(device,encoder,&stages)?};
        Ok(LiquidPipelineDispatch {input,dispatch})
    }
}
#[derive(Debug)]
pub struct LiquidPipelineDispatch {input:LiquidPipelineInput,dispatch:crate::compute_chain::ComputeChainDispatch}
impl LiquidPipelineDispatch {
    pub fn begin_read(self)->PendingLiquidPipeline {PendingLiquidPipeline {input:self.input,read:self.dispatch.begin_read(),validated:false}}
}
#[derive(Debug)]
pub struct PendingLiquidPipeline {input:LiquidPipelineInput,read:crate::compute_chain::PendingComputeChain,validated:bool}
impl PendingLiquidPipeline {
    pub fn try_candidate(&mut self)->Result<Option<LiquidPipelineCandidate>,LiquidForcePipelineError> {
        match self.read.try_read()? {
            None=>Ok(None),
            Some(bytes)=>match self.input.candidate(&bytes) {
                Ok(candidate)=>{self.validated=true;Ok(Some(candidate))},
                Err(error)=>{self.read.take_workspace();Err(error.into())},
            }
        }
    }
    pub fn take_workspace(&mut self)->Option<LiquidPipelineWorkspace> {
        if !self.validated {return None;}
        self.read.take_workspace().map(|chain|LiquidPipelineWorkspace {chain:Some(chain)})
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_chain_rejects_cross_stage_geometry_and_material_mismatch() {
        let particles=[LiquidDensityParticle {position:[0.;3],mass:0.002,rest_density:1000.,carrier_droplet:false},
            LiquidDensityParticle {position:[0.01,0.,0.],mass:0.003,rest_density:850.,carrier_droplet:false}];
        let parameters=LiquidPairForceParameters {delta:[-0.01,0.,0.],relative_velocity:[0.;3],normal_difference:[0.;3],
            mass:[0.002,0.003],density:[1.;2],pressure:[0.;2],reference_density:[1000.,850.],smoothing_radius:0.1,
            viscosity:0.03,surface_strength:0.07,interface_coefficient:0.2,kernel:LiquidDensityKernel::MassPoly6,gas_pressure:false,viscous_heating:false};
        let laws=[LiquidPressureParameters {rest_density:1000.,sound_speed:10.,heat_capacity_ratio:None},
            LiquidPressureParameters {rest_density:850.,sound_speed:10.,heat_capacity_ratio:None}];
        let build=|p|LiquidPipelineInput::new(&particles,&[0,1],&[(0,1)],0.1,LiquidDensityKernel::MassPoly6,&[],&laws,&[p],[0.,-9.81,0.]);
        let input=build(parameters).unwrap();
        let unwritten=[input.fields.density.bytes(),input.fields.normals.bytes(),input.forces.bytes(),input.accumulation.bytes()].concat();
        assert!(input.candidate(&unwritten).is_err());
        for mutation in 0..5 {
            let mut bad=parameters;
            match mutation {0=>bad.mass[0]*=2.,1=>bad.reference_density[1]+=1.,2=>bad.delta[0]+=1.,
                3=>bad.smoothing_radius*=2.,_=>bad.kernel=LiquidDensityKernel::VolumePoly6}
            assert!(build(bad).is_err(),"mutation={mutation}");
        }
        let mut bad_laws=laws;bad_laws[0].rest_density+=1.;
        assert!(LiquidPipelineInput::new(&particles,&[0,1],&[(0,1)],0.1,LiquidDensityKernel::MassPoly6,&[],&bad_laws,&[parameters],[0.;3]).is_err());
    }
    #[test]
    #[ignore="requires actual GPU; complete arithmetic chain, original physical owners"]
    fn gpu_liquid_chain_admits_real_density_pressure_force_and_accumulation() {
        use physics::liquid::{Particle,Material,Formulation,LiquidDensityOperator,LiquidDensityNormalOperator,LiquidDensityNormalResult,
            LiquidPressureLaw,LiquidPairForceInput,LiquidPairForceResult,LiquidAccumulationOperator,LiquidAccumulationResult};
        let instance=GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let pipeline=pollster::block_on(LiquidPipeline::new(&device)).unwrap();
        let particles=(0..70).map(|i|Particle {position:[i as f64*0.001,(i%3) as f64*0.002,(i%5) as f64*0.003],
            velocity:[0.01*i as f64,0.02,0.03],mass:0.002*(1.+(i%3) as f64),material:i%2}).collect::<Vec<_>>();
        let pairs=(0..70).flat_map(|i|(i+1..70).map(move|j|(i,j))).collect::<Vec<_>>();
        let support=(0..70).flat_map(|i|[(i,1000.+i as f64),(i,0.1)]).collect::<Vec<_>>();
        let h=0.1;let gravity=[0.1,-9.81,0.2];
        let properties=particles.iter().map(|p|Material {rest_density:if p.material==0 {1000.} else {850.},sound_speed:10.,viscosity:0.03}).collect::<Vec<_>>();
        let laws=properties.iter().map(|p|LiquidPressureLaw {rest_density:p.rest_density,sound_speed:p.sound_speed,heat_capacity_ratio:None}).collect::<Vec<_>>();
        let gpu_particles=particles.iter().zip(&properties).map(|(p,m)|LiquidDensityParticle {position:p.position,mass:p.mass,rest_density:m.rest_density,carrier_droplet:false}).collect::<Vec<_>>();
        let materials=particles.iter().map(|p|p.material as u32).collect::<Vec<_>>();
        let gpu_laws=laws.iter().map(|p|LiquidPressureParameters {rest_density:p.rest_density,sound_speed:p.sound_speed,heat_capacity_ratio:p.heat_capacity_ratio}).collect::<Vec<_>>();
        let budget=ComputeMemoryBudget::for_device(&device);let pool=ComputeReadbackPool::for_device(&device);
        for (formulation,kernel) in [(Formulation::MassDensity,LiquidDensityKernel::MassPoly6),(Formulation::RestVolume,LiquidDensityKernel::VolumePoly6),
            (Formulation::RestVolumeWendland,LiquidDensityKernel::VolumeWendland)] {
            let parameters=pairs.iter().map(|&(i,j)|LiquidPairForceParameters {
                delta:std::array::from_fn(|axis|particles[i].position[axis]-particles[j].position[axis]),
                relative_velocity:std::array::from_fn(|axis|particles[j].velocity[axis]-particles[i].velocity[axis]),normal_difference:[0.;3],
                mass:[particles[i].mass,particles[j].mass],density:[1.;2],pressure:[0.;2],reference_density:[properties[i].rest_density,properties[j].rest_density],
                smoothing_radius:h,viscosity:0.03,surface_strength:0.07,interface_coefficient:0.2,kernel,gas_pressure:false,viscous_heating:false,
            }).collect::<Vec<_>>();
            let input=LiquidPipelineInput::new(&gpu_particles,&materials,&pairs,h,kernel,&support,&gpu_laws,&parameters,gravity).unwrap();
            let mut bad=parameters.clone();bad[0].delta[0]+=1.;
            assert!(LiquidPipelineInput::new(&gpu_particles,&materials,&pairs,h,kernel,&support,&gpu_laws,&bad,gravity).is_err());
            let bytes=input.fields.density.bytes().len()+input.fields.normals.bytes().len()+input.forces.bytes().len()+input.accumulation.bytes().len();
            let before=budget.stats().allocated_bytes;let before_pool=pool.stats();
            let mut encoder=device.create_command_encoder(&Default::default());
            let dispatch=pipeline.encode(&device,&mut encoder,input).unwrap();
            assert_eq!(budget.stats().allocated_bytes,before+bytes as u64);
            let after_pool=pool.stats();assert_eq!(after_pool.creations+after_pool.reuses,before_pool.creations+before_pool.reuses+1);
            let submission=queue.submit([encoder.finish()]);let mut pending=dispatch.begin_read();
            assert_eq!(budget.stats().allocated_bytes,before+bytes as u64);
            device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
            // Retain the full snapshot for immutable-prefix and transfer corruption checks.
            let snapshot=pending.read.try_read().unwrap().unwrap();
            let candidate=pending.input.candidate(&snapshot).unwrap();
            assert_eq!(budget.stats().allocated_bytes,before);
            assert!(matches!(pending.try_candidate(),Err(LiquidForcePipelineError::Compute(ComputeError::Consumed))));
            let owner=LiquidDensityNormalOperator {fluid:LiquidDensityOperator {particles:&particles,pairs:&pairs,properties:&properties,
                formulation,smoothing_radius:h,carrier_droplets:None},support:&support,pressure_laws:Some(&laws)};
            owner.admit(&LiquidDensityNormalResult {densities:candidate.fields.densities.clone(),normals:candidate.fields.normals.clone(),pressures:candidate.fields.pressures.clone()},1e-12,2e-11).unwrap();
            assert!(candidate.fields.pressures.as_ref().unwrap().iter().all(|&p|p>0.));
            let pressure=candidate.fields.pressures.as_ref().unwrap();
            for ((&(i,j),p),result) in pairs.iter().zip(&parameters).zip(&candidate.forces) {
                let owner=LiquidPairForceInput {delta:p.delta,relative_velocity:p.relative_velocity,
                    normal_difference:std::array::from_fn(|axis|candidate.fields.normals[i][axis]-candidate.fields.normals[j][axis]),
                    mass:p.mass,density:[candidate.fields.densities[i],candidate.fields.densities[j]],pressure:[pressure[i],pressure[j]],
                    reference_density:p.reference_density,smoothing_radius:h,viscosity:p.viscosity,surface_strength:p.surface_strength,
                    interface_coefficient:p.interface_coefficient,formulation,gas_pressure:false,viscous_heating:false};
                owner.admit(LiquidPairForceResult {force:result.force,viscous_conductance:result.viscous_conductance},1e-12,2e-11).unwrap();
            }
            let forces=candidate.forces.iter().map(|p|LiquidPairForceResult {force:p.force,viscous_conductance:p.viscous_conductance}).collect::<Vec<_>>();
            LiquidAccumulationOperator {particles:&particles,pairs:&pairs,forces:&forces,gravity}.admit(
                &LiquidAccumulationResult {accelerations:candidate.accumulation.accelerations,viscous_rates:candidate.accumulation.viscous_rates},1e-12,2e-11).unwrap();
            let density_len=pending.input.fields.density.bytes().len();let normal_len=pending.input.fields.normals.bytes().len();
            let force_len=pending.input.forces.bytes().len();let (from,_)=pending.input.fields.normals.pressure_fields_range().unwrap();
            let (copied,_)=pending.input.forces.particle_fields_range();
            let (copied_forces,_)=pending.input.accumulation.fields_range();
            for offset in [0,density_len,density_len+from as usize,density_len+normal_len+14*4,
                density_len+normal_len+copied as usize,density_len+normal_len+force_len-4,density_len+normal_len+force_len,
                density_len+normal_len+force_len+copied_forces as usize,density_len+normal_len+force_len+copied_forces as usize+32,snapshot.len()-4] {
                let mut corrupt=snapshot.clone();corrupt[offset]^=1;
                assert!(pending.input.candidate(&corrupt).is_err(),"offset={offset}");
            }
            eprintln!("LIQUID COMPLETE ARITHMETIC CHAIN particles=70 pairs={} support={} formulation={formulation:?} jobs=4 kernels=6 submissions=1 staging_leases=1 intermediate_cpu_reads=0 original_physical_admission=true integration=not_exercised",pairs.len(),support.len());
        }
    }
    #[test]
    #[ignore="requires actual GPU; complete chain through transactional liquid stepping"]
    fn gpu_complete_chain_steps_walls_reflections_and_gas_without_stage_fallback() {
        use physics::liquid::{Liquid,Config,Particle,Material,Formulation,BoundarySample,ReflectingBox,
            LiquidComputeBackend,LiquidDensityOperator,LiquidPipelineOperator,LiquidPipelineResult,LiquidDensityNormalResult,
            LiquidForcePipelineResult,LiquidPairForceResult,LiquidAccumulationResult,IdealGas,LiquidField,TransportMaterial};
        let instance=GraphicsOptions::default().create_instance();let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type,wgpu::DeviceType::Cpu);
        let (device,queue)=pollster::block_on(adapter.request_device(&Default::default())).unwrap();let pipeline=pollster::block_on(LiquidPipeline::new(&device)).unwrap();
        struct Backend<'a> {pipeline:&'a LiquidPipeline,device:&'a wgpu::Device,queue:&'a wgpu::Queue,submissions:usize}
        impl LiquidComputeBackend for Backend<'_> {
            fn candidate(&mut self,_:&LiquidDensityOperator<'_>)->Option<Vec<f64>> {panic!("whole-chain capability must bypass individual callbacks")}
            fn supports_pipeline(&self)->bool {true}
            fn pipeline_candidate(&mut self,owner:&LiquidPipelineOperator<'_>)->Option<LiquidPipelineResult> {
                let fluid=&owner.fields.fluid;
                let particles=fluid.particles.iter().zip(fluid.properties).enumerate().map(|(i,(p,m))|LiquidDensityParticle {position:p.position,mass:p.mass,
                    rest_density:m.rest_density,carrier_droplet:fluid.carrier_droplets.is_some_and(|f|f[i])}).collect::<Vec<_>>();
                let materials=fluid.particles.iter().map(|p|u32::try_from(p.material).unwrap()).collect::<Vec<_>>();
                let kernel=match fluid.formulation {Formulation::MassDensity=>LiquidDensityKernel::MassPoly6,Formulation::RestVolume=>LiquidDensityKernel::VolumePoly6,
                    Formulation::RestVolumeWendland=>LiquidDensityKernel::VolumeWendland};
                let laws=owner.fields.pressure_laws.unwrap().iter().map(|law|LiquidPressureParameters {rest_density:law.rest_density,sound_speed:law.sound_speed,
                    heat_capacity_ratio:law.heat_capacity_ratio}).collect::<Vec<_>>();
                let parameters=owner.inputs.iter().map(|p|LiquidPairForceParameters {delta:p.delta,relative_velocity:p.relative_velocity,normal_difference:p.normal_difference,
                    mass:p.mass,density:p.density,pressure:p.pressure,reference_density:p.reference_density,smoothing_radius:p.smoothing_radius,viscosity:p.viscosity,
                    surface_strength:p.surface_strength,interface_coefficient:p.interface_coefficient,kernel,gas_pressure:p.gas_pressure,viscous_heating:p.viscous_heating}).collect::<Vec<_>>();
                let input=LiquidPipelineInput::new(&particles,&materials,fluid.pairs,fluid.smoothing_radius,kernel,owner.fields.support,&laws,&parameters,owner.gravity).unwrap();
                let mut encoder=self.device.create_command_encoder(&Default::default());let dispatch=self.pipeline.encode(self.device,&mut encoder,input).unwrap();
                let submission=self.queue.submit([encoder.finish()]);self.submissions+=1;let mut pending=dispatch.begin_read();
                self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
                let c=pending.try_candidate().unwrap().unwrap();
                Some(LiquidPipelineResult {fields:LiquidDensityNormalResult {densities:c.fields.densities,normals:c.fields.normals,pressures:c.fields.pressures},
                    forces:LiquidForcePipelineResult {forces:c.forces.iter().map(|p|LiquidPairForceResult {force:p.force,viscous_conductance:p.viscous_conductance}).collect(),
                    accumulation:LiquidAccumulationResult {accelerations:c.accumulation.accelerations,viscous_rates:c.accumulation.viscous_rates}}})
            }
        }
        fn compare(a:&Liquid,b:&Liquid) {
            for (a,b) in a.particles().iter().zip(b.particles()) {
                assert_eq!(a.mass,b.mass);assert_eq!(a.material,b.material);
                for axis in 0..3 {assert!((a.position[axis]-b.position[axis]).abs()<1e-10);assert!((a.velocity[axis]-b.velocity[axis]).abs()<1e-9);}
            }
        }
        let mut backend=Backend {pipeline:&pipeline,device:&device,queue:&queue,submissions:0};
        for formulation in [Formulation::MassDensity,Formulation::RestVolume,Formulation::RestVolumeWendland] {
            for reflected in [false,true] {
                if reflected && formulation==Formulation::MassDensity {continue;}
                let particles=(0..3).map(|i|Particle {position:[0.12+i as f64*0.04,0.15,0.2],velocity:[0.01*i as f64,0.,0.],mass:0.1,material:0}).collect();
                let mut native=Liquid::new(particles,vec![Material {rest_density:1.,sound_speed:1.,viscosity:0.01}],Config {smoothing_radius:1.,gravity:[0.;3],..Config::default()}).unwrap();
                native.set_formulation(formulation);
                if reflected {native.set_reflecting_box(Some(ReflectingBox {min:[0.;3],max:[3.;3]})).unwrap();}
                else {native.configure_boundaries(vec![BoundarySample {position:[-0.1,0.15,0.2],volume:0.1}]).unwrap();}
                let mut actual=native.clone();
                for _ in 0..3 {
                    native.step(1e-5,None).unwrap();let before=backend.submissions;
                    let (step,stats)=actual.step_with_compute_backend(1e-5,None,&mut backend,1e-12,2e-11).unwrap();
                    assert!(stats.pipeline_admitted>0);assert_eq!(stats.pipeline_native_fallbacks,0);assert_eq!(stats.pipeline_calls,step.substeps);
                    assert_eq!(backend.submissions-before,stats.pipeline_calls);assert_eq!(stats.density_normal_calls,0);assert_eq!(stats.force_pipeline_calls,0);compare(&actual,&native);
                }
                eprintln!("COMPLETE GPU LIQUID STEP formulation={formulation:?} reflected={reflected} steps=3 submissions_per_substep=1 whole_chain_fallbacks=0 integration_and_wall_forces=native");
            }
        }
        let gas=IdealGas {gas_constant:1.,heat_capacity_ratio:1.4};
        let particles=vec![Particle {position:[-0.2,0.,0.],velocity:[0.01,0.,0.],mass:1.,material:0},Particle {position:[0.2,0.,0.],velocity:[-0.01,0.,0.],mass:1.,material:0}];
        let mut native=Liquid::new(particles,vec![Material {rest_density:1000.,sound_speed:1.,viscosity:0.}],Config {gravity:[0.;3],smoothing_radius:1.,..Config::default()}).unwrap();
        native.configure_transport(vec![LiquidField {temperature:300.,concentration:0.};2],vec![TransportMaterial {specific_heat:gas.specific_heat_cv().unwrap(),conductivity:0.,..TransportMaterial::default()}]).unwrap();
        native.configure_gas_equations(vec![Some(gas)]).unwrap();let mut actual=native.clone();
        for _ in 0..3 {
            native.step(1e-5,None).unwrap();let before=backend.submissions;
            let (_,stats)=actual.step_with_compute_backend(1e-5,None,&mut backend,1e-12,2e-11).unwrap();
            assert!(stats.pipeline_admitted>0);assert_eq!(stats.pipeline_native_fallbacks,0);assert_eq!(backend.submissions-before,stats.pipeline_calls);compare(&actual,&native);
            for (a,b) in actual.fields().unwrap().iter().zip(native.fields().unwrap()) {assert!((a.temperature-b.temperature).abs()<1e-9);}
        }
        eprintln!("COMPLETE GPU GAS STEP steps=3 submissions_per_substep=1 whole_chain_fallbacks=0 pressure_work_transport_integration=native");
    }

}
