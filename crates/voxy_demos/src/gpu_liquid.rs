//! Application adapter; physics owns admission and transactional publication.
use physics::liquid::{
    Formulation, LiquidAccumulationResult, LiquidComputeBackend, LiquidDensityNormalResult,
    LiquidDensityOperator, LiquidForcePipelineResult, LiquidPairForceResult,
    LiquidPipelineOperator, LiquidPipelineResult,
};
use voxy_render::{
    LiquidDensityKernel, LiquidDensityParticle, LiquidPairForceParameters, LiquidPipeline,
    LiquidPipelineInput, LiquidPressureParameters,
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuLiquidStats {
    pub submissions: usize,
    pub decoded: usize,
    pub failures: usize,
    pub empty_pair_skips: usize,
    pub workspace_reuses: usize,
    pub workspace_creations: usize,
    pub topology_creations: usize,
    pub topology_reuses: usize,
}
/// Cumulative host intervals, not GPU timestamp measurements. Successful phases
/// before a later failure remain counted; end-to-end step timing is external.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuLiquidTimings {
    pub preparation_ns: u128,
    pub encode_submit_ns: u128,
    pub wait_ns: u128,
    pub decode_ns: u128,
}
#[derive(Debug)]
pub enum GpuLiquidError {
    Input(voxy_render::LiquidDensityError),
    Pipeline(voxy_render::LiquidForcePipelineError),
    Poll(String),
    ReadNotReady,
}
impl From<voxy_render::LiquidDensityError> for GpuLiquidError {
    fn from(e: voxy_render::LiquidDensityError) -> Self {
        Self::Input(e)
    }
}
impl From<voxy_render::LiquidForcePipelineError> for GpuLiquidError {
    fn from(e: voxy_render::LiquidForcePipelineError) -> Self {
        Self::Pipeline(e)
    }
}
/// Synchronous candidate adapter for the current synchronous physics step API.
/// It submits one chain per substep. Decoded counts are transport success only;
/// use the returned physics stats for admissions and native fallbacks.
#[derive(Debug)]
pub struct GpuLiquidBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: LiquidPipeline,
    timings: GpuLiquidTimings,
    stats: GpuLiquidStats,
    last_error: Option<GpuLiquidError>,
    workspace: voxy_render::LiquidPipelineWorkspace,
    topology: Option<voxy_render::LiquidPairTopology>,
}
impl GpuLiquidBackend {
    /// Device and queue must belong to the same renderer.
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, voxy_render::ComputeError> {
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            pipeline: LiquidPipeline::new(device).await?,
            timings: Default::default(),
            stats: Default::default(),
            last_error: None,
            workspace: Default::default(),
            topology: None,
        })
    }
    pub fn stats(&self) -> GpuLiquidStats {
        self.stats
    }
    pub fn timings(&self) -> GpuLiquidTimings {
        self.timings
    }
    pub fn last_error(&self) -> Option<&GpuLiquidError> {
        self.last_error.as_ref()
    }
    /// Release cached storage; configured budgets still require explicit retirement.
    pub fn clear_workspace(&mut self) {
        self.workspace = Default::default();
    }
    fn compute(
        &mut self,
        owner: &LiquidPipelineOperator<'_>,
    ) -> Result<LiquidPipelineResult, GpuLiquidError> {
        let preparation = std::time::Instant::now();
        let fluid = &owner.fields.fluid;
        if fluid
            .carrier_droplets
            .is_some_and(|f| f.len() != fluid.particles.len())
            || fluid.properties.len() != fluid.particles.len()
            || owner
                .inputs
                .iter()
                .any(|p| p.formulation != fluid.formulation)
        {
            return Err(voxy_render::LiquidDensityError::Shape.into());
        }
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
            .map(|p| {
                u32::try_from(p.material).map_err(|_| voxy_render::LiquidDensityError::Capacity)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let kernel = match fluid.formulation {
            Formulation::MassDensity => LiquidDensityKernel::MassPoly6,
            Formulation::RestVolume => LiquidDensityKernel::VolumePoly6,
            Formulation::RestVolumeWendland => LiquidDensityKernel::VolumeWendland,
        };
        let laws = owner
            .fields
            .pressure_laws
            .ok_or(voxy_render::LiquidDensityError::Shape)?
            .iter()
            .map(|p| LiquidPressureParameters {
                rest_density: p.rest_density,
                sound_speed: p.sound_speed,
                heat_capacity_ratio: p.heat_capacity_ratio,
            })
            .collect::<Vec<_>>();
        let parameters = owner
            .inputs
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
        if self
            .topology
            .as_ref()
            .is_some_and(|t| t.matches(particles.len(), fluid.pairs))
        {
            self.stats.topology_reuses += 1;
        } else {
            self.topology = Some(voxy_render::LiquidPairTopology::new(
                particles.len(),
                fluid.pairs,
            )?);
            self.stats.topology_creations += 1;
        }
        let input = LiquidPipelineInput::with_topology(
            &particles,
            &materials,
            self.topology.as_ref().unwrap(),
            fluid.smoothing_radius,
            kernel,
            owner.fields.support,
            &laws,
            &parameters,
            owner.gravity,
        )?;
        self.timings.preparation_ns += preparation.elapsed().as_nanos();
        let encode = std::time::Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("liquid arithmetic chain"),
            });
        let workspace = std::mem::take(&mut self.workspace);
        let previous_reuses = workspace.reuses();
        let dispatch = self.pipeline.encode_reusing(
            &self.device,
            &self.queue,
            &mut encoder,
            input,
            workspace,
        )?;
        let submission = self.queue.submit([encoder.finish()]);
        self.stats.submissions += 1;
        let mut pending = dispatch.begin_read();
        self.timings.encode_submit_ns += encode.elapsed().as_nanos();
        let wait = std::time::Instant::now();
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(5)),
            })
            .map_err(|e| GpuLiquidError::Poll(e.to_string()))?;
        self.timings.wait_ns += wait.elapsed().as_nanos();
        let decode = std::time::Instant::now();
        let c = pending
            .try_candidate()?
            .ok_or(GpuLiquidError::ReadNotReady)?;
        self.workspace = pending
            .take_workspace()
            .ok_or(GpuLiquidError::ReadNotReady)?;
        if self.workspace.reuses() > previous_reuses {
            self.stats.workspace_reuses += 1;
        } else {
            self.stats.workspace_creations += 1;
        }
        let result = LiquidPipelineResult {
            fields: LiquidDensityNormalResult {
                densities: c.fields.densities,
                normals: c.fields.normals,
                pressures: c.fields.pressures,
            },
            forces: LiquidForcePipelineResult {
                forces: c
                    .forces
                    .into_iter()
                    .map(|p| LiquidPairForceResult {
                        force: p.force,
                        viscous_conductance: p.viscous_conductance,
                    })
                    .collect(),
                accumulation: LiquidAccumulationResult {
                    accelerations: c.accumulation.accelerations,
                    viscous_rates: c.accumulation.viscous_rates,
                },
            },
        };
        self.timings.decode_ns += decode.elapsed().as_nanos();
        Ok(result)
    }
}
impl LiquidComputeBackend for GpuLiquidBackend {
    fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
        None
    }
    fn profile_pipeline_admission(&self) -> bool {
        true
    }
    fn supports_pipeline(&self) -> bool {
        true
    }
    fn pipeline_candidate(
        &mut self,
        owner: &LiquidPipelineOperator<'_>,
    ) -> Option<LiquidPipelineResult> {
        self.last_error = None;
        // No pair force job is needed; retain native handling for isolated particles.
        if owner.fields.fluid.pairs.is_empty() {
            self.stats.empty_pair_skips += 1;
            return None;
        }
        match self.compute(owner) {
            Ok(candidate) => {
                self.stats.decoded += 1;
                Some(candidate)
            }
            Err(error) => {
                self.stats.failures += 1;
                self.last_error = Some(error);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires actual GPU; application adapter and deliberate memory-budget rejection"]
    fn gpu_application_liquid_backend_steps_and_recovers_from_budget_rejection() {
        use physics::liquid::{Config, Liquid, Material, Particle};
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let budget = voxy_render::ComputeMemoryBudget::configure(&device, 1024 * 1024).unwrap();
        let mut backend = pollster::block_on(GpuLiquidBackend::new(&device, &queue)).unwrap();
        let particles = (0..8)
            .map(|i| Particle {
                position: [i as f64 * 0.01, 0., 0.],
                velocity: [0.01 * i as f64, 0., 0.],
                mass: 0.1,
                material: 0,
            })
            .collect();
        let initial = Liquid::new(
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
        let mut native = initial.clone();
        let mut actual = initial.clone();
        for _ in 0..3 {
            native.step(1e-5, None).unwrap();
            let (_, stats) = actual
                .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
                .unwrap();
            assert!(stats.pipeline_admitted > 0);
            assert_eq!(stats.pipeline_native_fallbacks, 0);
            assert!(backend.last_error().is_none());
            budget.discard_retired().unwrap();
            for (a, b) in actual.particles().iter().zip(native.particles()) {
                for axis in 0..3 {
                    assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                    assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
                }
            }
        }
        assert_eq!(backend.stats().decoded, backend.stats().submissions);
        assert_eq!(backend.stats().failures, 0);
        assert_eq!(backend.stats().workspace_creations, 1);
        assert_eq!(backend.stats().workspace_reuses, 2);
        assert_eq!(backend.stats().topology_creations, 1);
        assert_eq!(backend.stats().topology_reuses, 2);
        // Reject new storage before submission, then verify native whole fallback.
        backend.clear_workspace();
        budget.discard_retired().unwrap();
        let blocker = budget
            .allocate_storage(
                "deliberate capacity pressure",
                &vec![0u8; budget.max_bytes() as usize],
            )
            .unwrap();
        let mut fallback = initial.clone();
        let mut expected = initial.clone();
        expected.step(1e-5, None).unwrap();
        let before = backend.stats();
        let (_, stats) = fallback
            .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
            .unwrap();
        assert_eq!(fallback, expected);
        assert!(stats.pipeline_native_fallbacks > 0);
        assert!(backend.last_error().is_some());
        assert_eq!(backend.stats().submissions, before.submissions);
        assert!(backend.stats().failures > before.failures);
        drop(blocker);
        budget.discard_retired().unwrap();
        let (_, stats) = fallback
            .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
            .unwrap();
        assert!(stats.pipeline_admitted > 0);
        assert!(backend.last_error().is_none());
        // A different particle/pair shape must rebuild the topology and storage.
        let before = backend.stats();
        let mut changed = Liquid::new(
            initial.particles()[..3].to_vec(),
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
        let mut expected = changed.clone();
        expected.step(1e-5, None).unwrap();
        let (_, stats) = changed
            .step_with_compute_backend(1e-5, None, &mut backend, 1e-12, 2e-11)
            .unwrap();
        assert!(stats.pipeline_admitted > 0);
        assert_eq!(stats.pipeline_native_fallbacks, 0);
        assert_eq!(
            backend.stats().topology_creations,
            before.topology_creations + 1
        );
        for (a, b) in changed.particles().iter().zip(expected.particles()) {
            for axis in 0..3 {
                assert!((a.position[axis] - b.position[axis]).abs() < 1e-10);
                assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-9);
            }
        }
        eprintln!(
            "APPLICATION LIQUID BACKEND steps=3 memory_rejection=native_exact recovery=admitted stats={:?}",
            backend.stats()
        );
    }
}
