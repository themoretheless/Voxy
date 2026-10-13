//! Force-to-accumulation GPU chain: one submission, one final staging read.
use crate::{
    ComputeError, ComputeProgram, LIQUID_ACCUMULATION_SHADER, LIQUID_PAIR_FORCES_SHADER,
    LiquidAccumulationCandidate, LiquidAccumulationInput, LiquidDensityError,
    LiquidPairForceCandidate, LiquidPairForceParameters, LiquidPairForcesInput,
};
#[derive(Debug)]
pub enum LiquidForcePipelineError {
    Input(LiquidDensityError),
    Compute(ComputeError),
}
impl From<ComputeError> for LiquidForcePipelineError {
    fn from(e: ComputeError) -> Self {
        Self::Compute(e)
    }
}
impl From<LiquidDensityError> for LiquidForcePipelineError {
    fn from(e: LiquidDensityError) -> Self {
        Self::Input(e)
    }
}
#[derive(Debug)]
pub struct LiquidForcePipeline {
    forces: ComputeProgram,
    accumulation: ComputeProgram,
}
#[derive(Debug)]
pub struct LiquidForcePipelineInput {
    forces: LiquidPairForcesInput,
    accumulation: LiquidAccumulationInput,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidForcePipelineCandidate {
    pub forces: Vec<LiquidPairForceCandidate>,
    pub accumulation: LiquidAccumulationCandidate,
}
impl LiquidForcePipelineInput {
    pub fn new(
        parameters: &[LiquidPairForceParameters],
        masses: &[f64],
        pairs: &[(usize, usize)],
        gravity: [f64; 3],
    ) -> Result<Self, LiquidDensityError> {
        if pairs.len() != parameters.len()
            || pairs.iter().zip(parameters).any(|(&(i, j), p)| {
                masses.get(i) != Some(&p.mass[0]) || masses.get(j) != Some(&p.mass[1])
            })
        {
            return Err(LiquidDensityError::Shape);
        }
        let forces = LiquidPairForcesInput::new(parameters)?;
        let zero = LiquidPairForceCandidate {
            force: [0.; 3],
            viscous_conductance: 0.,
        };
        let accumulation =
            LiquidAccumulationInput::new(masses, pairs, &vec![zero; pairs.len()], gravity)?;
        Ok(Self {
            forces,
            accumulation,
        })
    }
    pub fn candidate(
        &self,
        bytes: &[u8],
    ) -> Result<LiquidForcePipelineCandidate, LiquidDensityError> {
        let split = self.forces.bytes().len();
        if bytes.len() != split + self.accumulation.bytes().len() {
            return Err(LiquidDensityError::Output);
        }
        let (force_bytes, accumulation_bytes) = bytes.split_at(split);
        let source = self.forces.audit(force_bytes)?;
        let accumulation = self
            .accumulation
            .candidate_with_audited_forces(accumulation_bytes, &source)?;
        let forces = source.into_values();
        Ok(LiquidForcePipelineCandidate {
            forces,
            accumulation,
        })
    }
}
impl LiquidForcePipeline {
    pub async fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        Ok(Self {
            forces: ComputeProgram::new(device, LIQUID_PAIR_FORCES_SHADER).await?,
            accumulation: ComputeProgram::new(device, LIQUID_ACCUMULATION_SHADER).await?,
        })
    }
    /// Caller owns submission/polling. Discard the encoder if encoding fails.
    /// Both jobs remain charged while their GPU work and final read are pending.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input: LiquidForcePipelineInput,
    ) -> Result<LiquidForcePipelineDispatch, LiquidForcePipelineError> {
        let copies = (0..input.forces.count())
            .map(|index| {
                let (from, length) = input
                    .forces
                    .force_output_range(index)
                    .ok_or(LiquidDensityError::Shape)?;
                let (to, _) = input
                    .accumulation
                    .force_input_range(index)
                    .ok_or(LiquidDensityError::Shape)?;
                Ok((from, to, length))
            })
            .collect::<Result<Vec<_>, LiquidDensityError>>()?;
        let dispatch = crate::compute_chain::encode_two_stage(
            device,
            encoder,
            &self.forces,
            input.forces.bytes(),
            input.forces.workgroups(),
            &self.accumulation,
            input.accumulation.bytes(),
            input.accumulation.workgroups(),
            &copies,
        )?;
        Ok(LiquidForcePipelineDispatch { input, dispatch })
    }
}
#[derive(Debug)]
pub struct LiquidForcePipelineDispatch {
    input: LiquidForcePipelineInput,
    dispatch: crate::compute_chain::ComputeChainDispatch,
}
impl LiquidForcePipelineDispatch {
    /// Call after submission. The chain itself performs no blocking device wait.
    pub fn begin_read(self) -> PendingLiquidForcePipeline {
        PendingLiquidForcePipeline {
            input: self.input,
            read: self.dispatch.begin_read(),
        }
    }
}
#[derive(Debug)]
pub struct PendingLiquidForcePipeline {
    input: LiquidForcePipelineInput,
    read: crate::compute_chain::PendingComputeChain,
}
impl PendingLiquidForcePipeline {
    /// Whole-chain transport audit only. Physics must admit both force and
    /// acceleration results against its current original inputs before publication.
    pub fn try_candidate(
        &mut self,
    ) -> Result<Option<LiquidForcePipelineCandidate>, LiquidForcePipelineError> {
        match self.read.try_read() {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) => Ok(Some(self.input.candidate(&bytes)?)),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LiquidDensityKernel;
    fn parameters() -> LiquidPairForceParameters {
        LiquidPairForceParameters {
            delta: [0.03, -0.01, 0.02],
            relative_velocity: [0.2, -0.1, 0.3],
            normal_difference: [0.1, 0.2, -0.3],
            mass: [1., 2.],
            density: [1050., 900.],
            pressure: [120., 80.],
            reference_density: [1000., 850.],
            smoothing_radius: 0.1,
            viscosity: 0.03,
            surface_strength: 0.07,
            interface_coefficient: 0.2,
            kernel: LiquidDensityKernel::MassPoly6,
            gas_pressure: false,
            viscous_heating: false,
        }
    }
    #[test]
    fn chain_audits_upstream_status_static_inputs_and_exact_transferred_words() {
        let input =
            LiquidForcePipelineInput::new(&[parameters()], &[1., 2.], &[(0, 1)], [0.; 3]).unwrap();
        let mut forces = bytemuck::cast_slice::<u8, u32>(input.forces.bytes()).to_vec();
        let (from, _) = input.forces.force_output_range(0).unwrap();
        let from = from as usize / 4;
        for (i, x) in [1f32, -2., 3., 0.5].into_iter().enumerate() {
            forces[from + 2 * i] = x.to_bits();
            forces[from + 2 * i + 1] = 0;
        }
        forces[from + 8] = 0;
        let mut accumulation = bytemuck::cast_slice::<u8, u32>(input.accumulation.bytes()).to_vec();
        let (to, _) = input.accumulation.force_input_range(0).unwrap();
        let to = to as usize / 4;
        accumulation[to..to + 8].copy_from_slice(&forces[from..from + 8]);
        let output = accumulation[4] as usize;
        accumulation[output..].fill(0);
        let mut words = forces.clone();
        words.extend(&accumulation);
        assert_eq!(
            input
                .candidate(bytemuck::cast_slice(&words))
                .unwrap()
                .forces[0]
                .force,
            [1., -2., 3.]
        );
        for offset in 0..from {
            let mut corrupt = words.clone();
            corrupt[offset] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        for offset in 0..output {
            let mut corrupt = words.clone();
            corrupt[forces.len() + offset] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        let mut corrupt = words.clone();
        corrupt[from + 8] = 1;
        assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        assert!(
            input
                .candidate(&bytemuck::cast_slice::<u32, u8>(&words)[..16])
                .is_err()
        );
        assert!(
            LiquidForcePipelineInput::new(&[parameters()], &[2., 2.], &[(0, 1)], [0.; 3]).is_err()
        );
    }
    #[test]
    #[ignore = "requires actual GPU; force and accumulation chain with one final readback"]
    fn gpu_force_chain_preserves_physical_owners_and_resource_lifetime() {
        use physics::liquid::{
            Formulation, LiquidAccumulationOperator, LiquidAccumulationResult,
            LiquidPairForceInput, LiquidPairForceResult, Particle,
        };
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let pipeline = pollster::block_on(LiquidForcePipeline::new(&device)).unwrap();
        let particles = (0..70)
            .map(|i| Particle {
                position: [
                    i as f64 * 0.001,
                    (i % 3) as f64 * 0.002,
                    (i % 5) as f64 * 0.003,
                ],
                velocity: [0.01 * i as f64, 0.02, 0.03],
                mass: 0.002 * (1. + (i % 3) as f64),
                material: i % 2,
            })
            .collect::<Vec<_>>();
        let masses = particles.iter().map(|p| p.mass).collect::<Vec<_>>();
        let pairs = (0..70)
            .flat_map(|i| (i + 1..70).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        fn upload(p: &LiquidPairForceInput) -> LiquidPairForceParameters {
            LiquidPairForceParameters {
                delta: p.delta,
                relative_velocity: p.relative_velocity,
                normal_difference: p.normal_difference,
                mass: p.mass,
                density: p.density,
                pressure: p.pressure,
                reference_density: p.reference_density,
                smoothing_radius: p.smoothing_radius,
                viscosity: p.viscosity,
                surface_strength: p.surface_strength,
                interface_coefficient: p.interface_coefficient,
                kernel: match p.formulation {
                    Formulation::MassDensity => LiquidDensityKernel::MassPoly6,
                    Formulation::RestVolume => LiquidDensityKernel::VolumePoly6,
                    Formulation::RestVolumeWendland => LiquidDensityKernel::VolumeWendland,
                },
                gas_pressure: p.gas_pressure,
                viscous_heating: p.viscous_heating,
            }
        }
        let budget = crate::ComputeMemoryBudget::for_device(&device);
        let pool = crate::ComputeReadbackPool::for_device(&device);
        for (formulation, kernel) in [
            (Formulation::MassDensity, LiquidDensityKernel::MassPoly6),
            (Formulation::RestVolume, LiquidDensityKernel::VolumePoly6),
            (
                Formulation::RestVolumeWendland,
                LiquidDensityKernel::VolumeWendland,
            ),
        ] {
            let owners = pairs
                .iter()
                .map(|&(i, j)| {
                    let a = particles[i];
                    let b = particles[j];
                    LiquidPairForceInput {
                        delta: std::array::from_fn(|axis| a.position[axis] - b.position[axis]),
                        relative_velocity: std::array::from_fn(|axis| {
                            b.velocity[axis] - a.velocity[axis]
                        }),
                        normal_difference: [0.1, 0.2, -0.3],
                        mass: [a.mass, b.mass],
                        density: [1050. + i as f64, 900. + j as f64],
                        pressure: [120., 80.],
                        reference_density: [1000., 850.],
                        smoothing_radius: 0.1,
                        viscosity: 0.03,
                        surface_strength: 0.07,
                        interface_coefficient: 0.2,
                        formulation,
                        gas_pressure: false,
                        viscous_heating: false,
                    }
                })
                .collect::<Vec<_>>();
            let parameters = owners
                .iter()
                .map(|p| LiquidPairForceParameters {
                    delta: p.delta,
                    relative_velocity: p.relative_velocity,
                    normal_difference: p.normal_difference,
                    mass: p.mass,
                    density: p.density,
                    pressure: p.pressure,
                    reference_density: p.reference_density,
                    smoothing_radius: p.smoothing_radius,
                    viscosity: p.viscosity,
                    surface_strength: p.surface_strength,
                    interface_coefficient: p.interface_coefficient,
                    kernel,
                    gas_pressure: p.gas_pressure,
                    viscous_heating: p.viscous_heating,
                })
                .collect::<Vec<_>>();
            let gravity = [0.1, -9.81, 0.2];
            let input =
                LiquidForcePipelineInput::new(&parameters, &masses, &pairs, gravity).unwrap();
            let storage_bytes =
                (input.forces.bytes().len() + input.accumulation.bytes().len()) as u64;
            let before = budget.stats().allocated_bytes;
            let before_pool = pool.stats();
            let mut encoder = device.create_command_encoder(&Default::default());
            let dispatch = pipeline.encode(&device, &mut encoder, input).unwrap();
            assert_eq!(budget.stats().allocated_bytes, before + storage_bytes);
            let after_pool = pool.stats();
            assert_eq!(
                after_pool.creations + after_pool.reuses,
                before_pool.creations + before_pool.reuses + 1
            );
            let submission = queue.submit([encoder.finish()]);
            let mut read = dispatch.begin_read();
            assert_eq!(budget.stats().allocated_bytes, before + storage_bytes);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let candidate = read.try_candidate().unwrap().unwrap();
            assert_eq!(budget.stats().allocated_bytes, before);
            assert!(matches!(
                read.try_candidate(),
                Err(LiquidForcePipelineError::Compute(ComputeError::Consumed))
            ));
            let forces = candidate
                .forces
                .iter()
                .map(|p| LiquidPairForceResult {
                    force: p.force,
                    viscous_conductance: p.viscous_conductance,
                })
                .collect::<Vec<_>>();
            for (owner, &result) in owners.iter().zip(&forces) {
                owner.admit(result, 1e-12, 2e-11).unwrap();
            }
            let owner = LiquidAccumulationOperator {
                particles: &particles,
                pairs: &pairs,
                forces: &forces,
                gravity,
            };
            owner
                .admit(
                    &LiquidAccumulationResult {
                        accelerations: candidate.accumulation.accelerations,
                        viscous_rates: candidate.accumulation.viscous_rates,
                    },
                    1e-12,
                    2e-11,
                )
                .unwrap();
            eprintln!(
                "LIQUID FORCE CHAIN particles=70 pairs={} formulation={formulation:?} submissions=1 staging_leases=1 intermediate_cpu_reads=0",
                pairs.len()
            );
        }
        use physics::liquid::{
            Config, Liquid, LiquidComputeBackend, LiquidDensityOperator,
            LiquidForcePipelineOperator, LiquidForcePipelineResult, Material,
        };
        struct Backend<'a> {
            pipeline: &'a LiquidForcePipeline,
            device: &'a wgpu::Device,
            queue: &'a wgpu::Queue,
        }
        impl LiquidComputeBackend for Backend<'_> {
            fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
                None
            }
            fn supports_force_pipeline(&self) -> bool {
                true
            }
            fn force_pipeline_candidate(
                &mut self,
                owner: &LiquidForcePipelineOperator<'_>,
            ) -> Option<LiquidForcePipelineResult> {
                let parameters = owner.inputs.iter().map(upload).collect::<Vec<_>>();
                let masses = owner.particles.iter().map(|p| p.mass).collect::<Vec<_>>();
                let input =
                    LiquidForcePipelineInput::new(&parameters, &masses, owner.pairs, owner.gravity)
                        .ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                let dispatch = self
                    .pipeline
                    .encode(self.device, &mut encoder, input)
                    .ok()?;
                let submission = self.queue.submit([encoder.finish()]);
                let mut read = dispatch.begin_read();
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .ok()?;
                let result = read.try_candidate().ok()??;
                Some(LiquidForcePipelineResult {
                    forces: result
                        .forces
                        .into_iter()
                        .map(|p| LiquidPairForceResult {
                            force: p.force,
                            viscous_conductance: p.viscous_conductance,
                        })
                        .collect(),
                    accumulation: LiquidAccumulationResult {
                        accelerations: result.accumulation.accelerations,
                        viscous_rates: result.accumulation.viscous_rates,
                    },
                })
            }
        }
        let mut backend = Backend {
            pipeline: &pipeline,
            device: &device,
            queue: &queue,
        };
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            let particles = (0..8)
                .map(|i| Particle {
                    position: [
                        (i % 2) as f64 * 0.02,
                        ((i / 2) % 2) as f64 * 0.02,
                        (i / 4) as f64 * 0.02,
                    ],
                    velocity: [0.01 * i as f64, -0.02 * i as f64, 0.03],
                    mass: 0.02 * (1. + (i % 3) as f64),
                    material: i % 2,
                })
                .collect();
            let mut native = Liquid::new(
                particles,
                vec![Material::WATER, Material::OIL],
                Config {
                    gravity: [0.; 3],
                    smoothing_radius: 0.05,
                    particle_radius: 0.005,
                    ..Config::default()
                },
            )
            .unwrap();
            native.set_formulation(formulation);
            native.set_surface_strength(0, 0.07).unwrap();
            native.set_surface_strength(1, 0.03).unwrap();
            native.set_interface_penalty(0, 1, 0.02).unwrap();
            assert!(
                native
                    .pair_force_inputs()
                    .unwrap()
                    .iter()
                    .any(|p| p.pressure.iter().any(|&v| v > 0.))
            );
            let mut actual = native.clone();
            let mut admitted = 0;
            for _ in 0..3 {
                native.step(1e-6, None).unwrap();
                let (_, stats) = actual
                    .step_with_compute_backend(1e-6, None, &mut backend, 1e-12, 2e-11)
                    .unwrap();
                assert!(stats.force_pipeline_admitted > 0);
                assert_eq!(stats.force_pipeline_native_fallbacks, 0);
                admitted += stats.force_pipeline_admitted;
                for (a, b) in actual.particles().iter().zip(native.particles()) {
                    assert_eq!(a.mass, b.mass);
                    assert_eq!(a.material, b.material);
                    for axis in 0..3 {
                        assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                        assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
                    }
                }
            }
            eprintln!(
                "LIQUID FORCE CHAIN STEP formulation={formulation:?} steps=3 admitted={admitted} native_chain_fallbacks=0 intermediate_cpu_reads=0 scope=force_to_accumulation_only"
            );
        }
    }
}
