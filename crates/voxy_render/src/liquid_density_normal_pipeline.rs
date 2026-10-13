//! GPU density -> supported color normals without an intermediate CPU read.
use crate::{
    ComputeError, ComputeProgram, LIQUID_DENSITY_SHADER, LIQUID_NORMALS_SHADER, LiquidDensityError,
    LiquidDensityInput, LiquidDensityKernel, LiquidDensityParticle, LiquidForcePipelineError,
    LiquidNormalParticle, LiquidNormalsInput,
};
#[derive(Clone, Copy, Debug)]
pub struct LiquidPressureParameters {
    pub rest_density: f64,
    pub sound_speed: f64,
    pub heat_capacity_ratio: Option<f64>,
}
#[derive(Debug)]
pub struct LiquidDensityNormalPipeline {
    density: ComputeProgram,
    normals: ComputeProgram,
}
#[derive(Debug)]
pub struct LiquidDensityNormalInput {
    pub(crate) density: LiquidDensityInput,
    pub(crate) normals: LiquidNormalsInput,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidDensityNormalCandidate {
    pub densities: Vec<f64>,
    pub normals: Vec<[f64; 3]>,
    pub pressures: Option<Vec<f64>>,
}
impl LiquidDensityNormalInput {
    pub fn new(
        particles: &[LiquidDensityParticle],
        materials: &[u32],
        pairs: &[(usize, usize)],
        h: f64,
        kernel: LiquidDensityKernel,
        support: &[(usize, f64)],
    ) -> Result<Self, LiquidDensityError> {
        let topology = crate::LiquidPairTopology::new(particles.len(), pairs)?;
        Self::with_topology(particles, materials, &topology, h, kernel, support, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_pressure(
        particles: &[LiquidDensityParticle],
        materials: &[u32],
        pairs: &[(usize, usize)],
        h: f64,
        kernel: LiquidDensityKernel,
        support: &[(usize, f64)],
        laws: &[LiquidPressureParameters],
    ) -> Result<Self, LiquidDensityError> {
        let topology = crate::LiquidPairTopology::new(particles.len(), pairs)?;
        Self::with_topology(
            particles,
            materials,
            &topology,
            h,
            kernel,
            support,
            Some(laws),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn with_topology(
        particles: &[LiquidDensityParticle],
        materials: &[u32],
        topology: &crate::LiquidPairTopology,
        h: f64,
        kernel: LiquidDensityKernel,
        support: &[(usize, f64)],
        laws: Option<&[LiquidPressureParameters]>,
    ) -> Result<Self, LiquidDensityError> {
        if particles.len() != materials.len()
            || laws.is_some_and(|laws| {
                particles.len() != laws.len()
                    || particles
                        .iter()
                        .zip(laws)
                        .any(|(p, l)| p.rest_density != l.rest_density)
            })
        {
            return Err(LiquidDensityError::Shape);
        }
        let density = LiquidDensityInput::with_topology(particles, topology, h, kernel)?;
        let normals = particles
            .iter()
            .zip(materials)
            .map(|(p, &material)| LiquidNormalParticle {
                position: p.position,
                mass: p.mass,
                density: 1.,
                material,
            })
            .collect::<Vec<_>>();
        let mut normals = LiquidNormalsInput::with_support(&normals, topology, h, support)?;
        if let Some(laws) = laws {
            normals = normals.with_pressure(laws)?;
        }
        Ok(Self { density, normals })
    }
    pub fn candidate(
        &self,
        bytes: &[u8],
    ) -> Result<LiquidDensityNormalCandidate, LiquidDensityError> {
        let split = self.density.bytes().len();
        if bytes.len() != split + self.normals.bytes().len() {
            return Err(LiquidDensityError::Output);
        }
        let (density_bytes, normal_bytes) = bytes.split_at(split);
        let (densities, normals) =
            self.normals
                .candidate_with_gpu_density(normal_bytes, &self.density, density_bytes)?;
        let pressures = self.normals.pressures_after_density_audit(normal_bytes)?;
        Ok(LiquidDensityNormalCandidate {
            densities,
            normals,
            pressures,
        })
    }
}
impl LiquidDensityNormalPipeline {
    pub async fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        Ok(Self {
            density: ComputeProgram::new(device, LIQUID_DENSITY_SHADER).await?,
            normals: ComputeProgram::new(device, LIQUID_NORMALS_SHADER).await?,
        })
    }
    /// Caller submits/polls; discard its encoder on error. Jobs stay charged until read completion.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input: LiquidDensityNormalInput,
    ) -> Result<LiquidDensityNormalDispatch, LiquidForcePipelineError> {
        let copies = (0..input.density.count())
            .map(|i| {
                let (from, length) = input
                    .density
                    .density_output_range(i)
                    .ok_or(LiquidDensityError::Shape)?;
                let (to, _) = input
                    .normals
                    .density_input_range(i)
                    .ok_or(LiquidDensityError::Shape)?;
                Ok((from, to, length))
            })
            .collect::<Result<Vec<_>, LiquidDensityError>>()?;
        let dispatch = crate::compute_chain::encode_two_stage(
            device,
            encoder,
            &self.density,
            input.density.bytes(),
            input.density.workgroups(),
            &self.normals,
            input.normals.bytes(),
            input.normals.workgroups(),
            &copies,
        )?;
        Ok(LiquidDensityNormalDispatch { input, dispatch })
    }
}
#[derive(Debug)]
pub struct LiquidDensityNormalDispatch {
    input: LiquidDensityNormalInput,
    dispatch: crate::compute_chain::ComputeChainDispatch,
}
impl LiquidDensityNormalDispatch {
    pub fn begin_read(self) -> PendingLiquidDensityNormal {
        PendingLiquidDensityNormal {
            input: self.input,
            read: self.dispatch.begin_read(),
        }
    }
}
#[derive(Debug)]
pub struct PendingLiquidDensityNormal {
    input: LiquidDensityNormalInput,
    read: crate::compute_chain::PendingComputeChain,
}
impl PendingLiquidDensityNormal {
    /// Complete transfer/status audit; original physics still admits the result.
    pub fn try_candidate(
        &mut self,
    ) -> Result<Option<LiquidDensityNormalCandidate>, LiquidForcePipelineError> {
        self.read
            .try_read()?
            .map(|bytes| self.input.candidate(&bytes).map_err(Into::into))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joint_input_rejects_unwritten_results_and_invalid_support() {
        let module = naga::front::wgsl::parse_str(LIQUID_NORMALS_SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let particles = [LiquidDensityParticle {
            position: [0.; 3],
            mass: 1.,
            rest_density: 1000.,
            carrier_droplet: false,
        }];
        let laws = [LiquidPressureParameters {
            rest_density: 1000.,
            sound_speed: 1.,
            heat_capacity_ratio: None,
        }];
        let input = LiquidDensityNormalInput::new_with_pressure(
            &particles,
            &[0],
            &[],
            0.2,
            LiquidDensityKernel::MassPoly6,
            &[(0, 0.1)],
            &laws,
        )
        .unwrap();
        let mut bytes = input.density.bytes().to_vec();
        bytes.extend(input.normals.bytes());
        assert!(input.candidate(&bytes).is_err());
        let mut upstream = bytemuck::cast_slice::<u8, u32>(input.density.bytes()).to_vec();
        let (from, _) = input.density.density_output_range(0).unwrap();
        let from = from as usize / 4;
        upstream[from] = 1f32.to_bits();
        upstream[from + 1] = 0;
        upstream[from + 2] = 0;
        let mut downstream = bytemuck::cast_slice::<u8, u32>(input.normals.bytes()).to_vec();
        let (to, _) = input.normals.density_input_range(0).unwrap();
        let to = to as usize / 4;
        downstream[to..to + 2].copy_from_slice(&upstream[from..from + 2]);
        let output = downstream[4] as usize;
        let density_output = downstream[5] as usize;
        downstream[output..].fill(0);
        let hi = 1.1f64 as f32;
        let lo = (1.1f64 - f64::from(hi)) as f32;
        downstream[density_output] = hi.to_bits();
        downstream[density_output + 1] = lo.to_bits();
        let mut snapshot = upstream.clone();
        snapshot.extend(&downstream);
        assert_eq!(
            input
                .candidate(bytemuck::cast_slice(&snapshot))
                .unwrap()
                .pressures,
            Some(vec![0.])
        );
        for gamma in [Some(1.), Some(f64::NAN)] {
            assert!(
                LiquidDensityNormalInput::new_with_pressure(
                    &particles,
                    &[0],
                    &[],
                    0.2,
                    LiquidDensityKernel::MassPoly6,
                    &[],
                    &[LiquidPressureParameters {
                        heat_capacity_ratio: gamma,
                        ..laws[0]
                    }]
                )
                .is_err()
            );
        }
        for index in 0..from {
            let mut corrupt = snapshot.clone();
            corrupt[index] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        for index in 0..output {
            let mut corrupt = snapshot.clone();
            corrupt[upstream.len() + index] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&corrupt)).is_err());
        }
        for support in [&[(1, 0.1)][..], &[(0, -0.1)][..], &[(0, f64::NAN)][..]] {
            assert!(
                LiquidDensityNormalInput::new(
                    &particles,
                    &[0],
                    &[],
                    0.2,
                    LiquidDensityKernel::MassPoly6,
                    support
                )
                .is_err()
            );
        }
    }
    #[test]
    #[ignore = "requires actual GPU; supported density and normal chain"]
    fn gpu_density_normal_chain_preserves_support_and_native_step() {
        use physics::liquid::{
            BoundarySample, Config, Formulation, Liquid, LiquidComputeBackend,
            LiquidDensityNormalOperator, LiquidDensityNormalResult, LiquidDensityOperator,
            Material, Particle, ReflectingBox,
        };
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let pipeline = pollster::block_on(LiquidDensityNormalPipeline::new(&device)).unwrap();
        let force_pipeline = pollster::block_on(crate::LiquidForcePipeline::new(&device)).unwrap();
        fn input(owner: &LiquidDensityNormalOperator<'_>) -> LiquidDensityNormalInput {
            let fluid = &owner.fluid;
            let particles = fluid
                .particles
                .iter()
                .zip(fluid.properties)
                .enumerate()
                .map(|(i, (p, m))| LiquidDensityParticle {
                    position: p.position,
                    mass: p.mass,
                    rest_density: m.rest_density,
                    carrier_droplet: fluid.carrier_droplets.is_some_and(|f| f[i]),
                })
                .collect::<Vec<_>>();
            let materials = fluid
                .particles
                .iter()
                .map(|p| u32::try_from(p.material).unwrap())
                .collect::<Vec<_>>();
            let kernel = match fluid.formulation {
                Formulation::MassDensity => LiquidDensityKernel::MassPoly6,
                Formulation::RestVolume => LiquidDensityKernel::VolumePoly6,
                Formulation::RestVolumeWendland => LiquidDensityKernel::VolumeWendland,
            };
            if let Some(laws) = owner.pressure_laws {
                let laws = laws
                    .iter()
                    .map(|law| LiquidPressureParameters {
                        rest_density: law.rest_density,
                        sound_speed: law.sound_speed,
                        heat_capacity_ratio: law.heat_capacity_ratio,
                    })
                    .collect::<Vec<_>>();
                LiquidDensityNormalInput::new_with_pressure(
                    &particles,
                    &materials,
                    fluid.pairs,
                    fluid.smoothing_radius,
                    kernel,
                    owner.support,
                    &laws,
                )
                .unwrap()
            } else {
                LiquidDensityNormalInput::new(
                    &particles,
                    &materials,
                    fluid.pairs,
                    fluid.smoothing_radius,
                    kernel,
                    owner.support,
                )
                .unwrap()
            }
        }
        struct Backend<'a> {
            pipeline: &'a LiquidDensityNormalPipeline,
            force_pipeline: &'a crate::LiquidForcePipeline,
            device: &'a wgpu::Device,
            queue: &'a wgpu::Queue,
            support_seen: bool,
        }
        impl LiquidComputeBackend for Backend<'_> {
            fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
                panic!("joint density callback must be used")
            }
            fn supports_force_pipeline(&self) -> bool {
                true
            }
            fn force_pipeline_candidate(
                &mut self,
                owner: &physics::liquid::LiquidForcePipelineOperator<'_>,
            ) -> Option<physics::liquid::LiquidForcePipelineResult> {
                let parameters = owner
                    .inputs
                    .iter()
                    .map(|p| crate::LiquidPairForceParameters {
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
                    })
                    .collect::<Vec<_>>();
                let masses = owner.particles.iter().map(|p| p.mass).collect::<Vec<_>>();
                let input = crate::LiquidForcePipelineInput::new(
                    &parameters,
                    &masses,
                    owner.pairs,
                    owner.gravity,
                )
                .ok()?;
                let mut encoder = self.device.create_command_encoder(&Default::default());
                let dispatch = self
                    .force_pipeline
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
                Some(physics::liquid::LiquidForcePipelineResult {
                    forces: result
                        .forces
                        .into_iter()
                        .map(|p| physics::liquid::LiquidPairForceResult {
                            force: p.force,
                            viscous_conductance: p.viscous_conductance,
                        })
                        .collect(),
                    accumulation: physics::liquid::LiquidAccumulationResult {
                        accelerations: result.accumulation.accelerations,
                        viscous_rates: result.accumulation.viscous_rates,
                    },
                })
            }
            fn supports_density_normal_pressure(&self) -> bool {
                true
            }
            fn supports_density_normals(&self) -> bool {
                true
            }
            fn density_normal_candidate(
                &mut self,
                owner: &LiquidDensityNormalOperator<'_>,
            ) -> Option<LiquidDensityNormalResult> {
                self.support_seen |= !owner.support.is_empty();
                let mut encoder = self.device.create_command_encoder(&Default::default());
                let dispatch = self
                    .pipeline
                    .encode(self.device, &mut encoder, input(owner))
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
                Some(LiquidDensityNormalResult {
                    densities: result.densities,
                    normals: result.normals,
                    pressures: result.pressures,
                })
            }
        }
        let particles = (0..70)
            .map(|i| Particle {
                position: [1e8 + (i % 7) as f64 * 0.01, (i / 7) as f64 * 0.01, 0.],
                velocity: [0.; 3],
                mass: 0.001 * (1. + (i % 3) as f64),
                material: i % 2,
            })
            .collect::<Vec<_>>();
        let properties = particles
            .iter()
            .map(|p| {
                if p.material == 0 {
                    Material::WATER
                } else {
                    Material::OIL
                }
            })
            .collect::<Vec<_>>();
        let pairs = (0..70)
            .flat_map(|i| (i + 1..70).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let support = (0..70)
            .flat_map(|i| [(i, 0.1 + i as f64 * 0.001), (i, 1000.2)])
            .collect::<Vec<_>>();
        let laws = properties
            .iter()
            .map(|p| physics::liquid::LiquidPressureLaw {
                rest_density: p.rest_density,
                sound_speed: p.sound_speed,
                heat_capacity_ratio: None,
            })
            .collect::<Vec<_>>();
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            let owner = LiquidDensityNormalOperator {
                fluid: LiquidDensityOperator {
                    particles: &particles,
                    pairs: &pairs,
                    properties: &properties,
                    formulation,
                    smoothing_radius: 0.2,
                    carrier_droplets: None,
                },
                support: &support,
                pressure_laws: Some(&laws),
            };
            let mut backend = Backend {
                pipeline: &pipeline,
                force_pipeline: &force_pipeline,
                device: &device,
                queue: &queue,
                support_seen: false,
            };
            let candidate = backend.density_normal_candidate(&owner).unwrap();
            owner.admit(&candidate, 1e-12, 2e-11).unwrap();
            assert!(
                candidate
                    .pressures
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|&x| x > 0.)
            );
            eprintln!(
                "LIQUID DENSITY NORMAL CHAIN particles=70 pairs=2415 support=140 formulation={formulation:?} submissions=1 intermediate_cpu_reads=0"
            );
            for reflected in [false, true] {
                if reflected && formulation == Formulation::MassDensity {
                    continue;
                }
                let particles = (0..3)
                    .map(|i| Particle {
                        position: [0.12 + i as f64 * 0.04, 0.15, 0.2],
                        velocity: [0.01 * i as f64, 0., 0.],
                        mass: 0.1,
                        material: 0,
                    })
                    .collect();
                let mut native = Liquid::new(
                    particles,
                    vec![Material {
                        rest_density: 1.,
                        sound_speed: 1.,
                        viscosity: 0.01,
                    }],
                    Config {
                        smoothing_radius: 1.,
                        gravity: [0.; 3],
                        ..Config::default()
                    },
                )
                .unwrap();
                native.set_formulation(formulation);
                if !reflected {
                    native
                        .configure_boundaries(vec![BoundarySample {
                            position: [-0.1, 0.15, 0.2],
                            volume: 0.1,
                        }])
                        .unwrap();
                }
                if reflected {
                    native
                        .set_reflecting_box(Some(ReflectingBox {
                            min: [0.; 3],
                            max: [3.; 3],
                        }))
                        .unwrap();
                }
                let mut actual = native.clone();
                let mut admitted = 0;
                for _ in 0..3 {
                    native.step(1e-5, None).unwrap();
                    let (_, stats) = actual
                        .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
                        .unwrap();
                    assert!(stats.density_normal_admitted > 0);
                    assert_eq!(stats.density_normal_native_fallbacks, 0);
                    admitted += stats.density_normal_admitted;
                    assert_eq!(
                        stats.density_normal_pressure_admitted,
                        stats.density_normal_admitted
                    );
                    assert!(stats.force_pipeline_admitted > 0);
                    assert_eq!(stats.force_pipeline_native_fallbacks, 0);
                    for (a, b) in actual.particles().iter().zip(native.particles()) {
                        assert_eq!(a.mass, b.mass);
                        assert_eq!(a.material, b.material);
                        for axis in 0..3 {
                            assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                            assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
                        }
                    }
                }
                assert!(backend.support_seen);
                eprintln!(
                    "LIQUID DENSITY NORMAL STEP formulation={formulation:?} reflected={reflected} steps=3 admitted={admitted} native_chain_fallbacks=0 five_gpu_stages=2_submissions wall_forces_and_integration=native"
                );
            }
        }
        use physics::liquid::{IdealGas, LiquidField, TransportMaterial};
        let gas = IdealGas {
            gas_constant: 1.,
            heat_capacity_ratio: 1.4,
        };
        let particles = vec![
            Particle {
                position: [-0.2, 0., 0.],
                velocity: [0.01, 0., 0.],
                mass: 1.,
                material: 0,
            },
            Particle {
                position: [0.2, 0., 0.],
                velocity: [-0.01, 0., 0.],
                mass: 1.,
                material: 0,
            },
        ];
        let mut native = Liquid::new(
            particles,
            vec![Material {
                rest_density: 1000.,
                sound_speed: 1.,
                viscosity: 0.,
            }],
            Config {
                gravity: [0.; 3],
                smoothing_radius: 1.,
                ..Config::default()
            },
        )
        .unwrap();
        native
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 300.,
                        concentration: 0.
                    };
                    2
                ],
                vec![TransportMaterial {
                    specific_heat: gas.specific_heat_cv().unwrap(),
                    conductivity: 0.,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        native.configure_gas_equations(vec![Some(gas)]).unwrap();
        let mut actual = native.clone();
        let mut backend = Backend {
            pipeline: &pipeline,
            force_pipeline: &force_pipeline,
            device: &device,
            queue: &queue,
            support_seen: false,
        };
        for _ in 0..3 {
            native.step(1e-5, None).unwrap();
            let (_, stats) = actual
                .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
                .unwrap();
            assert!(stats.density_normal_pressure_admitted > 0);
            assert_eq!(stats.density_normal_native_fallbacks, 0);
            assert_eq!(stats.force_pipeline_native_fallbacks, 0);
            for (a, b) in actual.particles().iter().zip(native.particles()) {
                assert_eq!(a.mass, b.mass);
                assert_eq!(a.material, b.material);
                for axis in 0..3 {
                    assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                    assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
                }
            }
            for (a, b) in actual
                .fields()
                .unwrap()
                .iter()
                .zip(native.fields().unwrap())
            {
                assert!((a.temperature - b.temperature).abs() < 1e-9);
            }
        }
        eprintln!(
            "LIQUID GAS PRESSURE GPU STEP steps=3 native_chain_fallbacks=0 pressure_work_and_transport=native"
        );
    }
}
