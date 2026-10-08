use voxy_render::{ComputeDispatch, ComputeError, PendingComputeReadback};

/// Explicit f32 hardware representation; CPU f64 solvers are not implicitly
/// replaced or silently downgraded. Mass must be positive and data finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityBody {
    pub mass: f32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct GravityParameters {
    pub constant: f32,
    pub softening: f32,
    pub uniform_acceleration: [f32; 3],
    pub dt: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct GravityBudget {
    pub max_bodies: u32,
    pub max_steps_per_encode: u32,
}
impl Default for GravityBudget {
    fn default() -> Self {
        Self {
            max_bodies: 4096,
            max_steps_per_encode: 256,
        }
    }
}
#[derive(Debug)]
pub enum GravityComputeError {
    InvalidInput,
    Budget,
    SingularPair,
    NumericalOverflow,
    Compute(ComputeError),
}
impl std::fmt::Display for GravityComputeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU gravity error: {self:?}")
    }
}
impl std::error::Error for GravityComputeError {}

#[derive(Debug)]
pub struct GravityProgram {
    device: wgpu::Device,
    readback_pool: voxy_render::ComputeReadbackPool,
    memory_budget: voxy_render::ComputeMemoryBudget,
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::ComputePipeline; 3],
    budget: GravityBudget,
}
impl GravityProgram {
    /// Creates three ordered velocity-Verlet passes on the caller's device.
    /// # Errors
    /// Rejects invalid budgets, missing compute support and shader ABI failures.
    pub async fn new(
        device: &wgpu::Device,
        budget: GravityBudget,
    ) -> Result<Self, GravityComputeError> {
        if budget.max_bodies == 0
            || budget.max_bodies > u32::MAX / 24
            || budget.max_steps_per_encode == 0
        {
            return Err(GravityComputeError::Budget);
        }
        if device.limits().max_compute_workgroups_per_dimension == 0
            || device.limits().max_storage_buffers_per_shader_stage == 0
        {
            return Err(GravityComputeError::Compute(ComputeError::Unsupported));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resident gravity Verlet"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gravity.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gravity storage"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gravity layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = ["predict", "correct", "commit"].map(|entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        if let Some(error) = scope.pop().await {
            return Err(GravityComputeError::Compute(ComputeError::Validation(
                error.to_string(),
            )));
        }
        Ok(Self {
            device: device.clone(),
            readback_pool: voxy_render::ComputeReadbackPool::for_device(device),
            memory_budget: voxy_render::ComputeMemoryBudget::for_device(device),
            layout,
            pipelines,
            budget,
        })
    }

    /// Uploads initial bodies once; subsequent steps stay on the same GPU storage.
    /// # Errors
    /// Rejects empty/invalid inputs, unrepresentable softening and memory/dispatch
    /// budgets before allocating. Unequal device handles are rejected; all
    /// resources are allocated on the retained owner even when separate wgpu
    /// instances reuse device IDs. Submit and poll on the original device.
    pub fn create_job(
        &self,
        device: &wgpu::Device,
        bodies: &[GravityBody],
        parameters: GravityParameters,
    ) -> Result<GravityJob, GravityComputeError> {
        if device != &self.device {
            return Err(GravityComputeError::Compute(ComputeError::DeviceMismatch));
        }
        let device = &self.device;
        let count = u32::try_from(bodies.len()).map_err(|_| GravityComputeError::Budget)?;
        if count == 0 || count > self.budget.max_bodies {
            return Err(GravityComputeError::Budget);
        }
        let eps2 = parameters.softening * parameters.softening;
        if !parameters.constant.is_finite()
            || parameters.constant < 0.0
            || !parameters.softening.is_finite()
            || parameters.softening < 0.0
            || !eps2.is_finite()
            || (parameters.softening > 0.0 && eps2 == 0.0)
            || !parameters.dt.is_finite()
            || parameters.dt <= 0.0
            || parameters
                .uniform_acceleration
                .iter()
                .any(|value| !value.is_finite())
            || bodies.iter().any(|body| {
                !body.mass.is_finite()
                    || body.mass <= 0.0
                    || body
                        .position
                        .iter()
                        .chain(&body.velocity)
                        .any(|value| !value.is_finite())
            })
        {
            return Err(GravityComputeError::InvalidInput);
        }
        let size = 32 + u64::from(count) * 96;
        let limits = device.limits();
        if size > limits.max_buffer_size
            || size > limits.max_storage_buffer_binding_size
            || count.div_ceil(64) > limits.max_compute_workgroups_per_dimension
        {
            return Err(GravityComputeError::Budget);
        }
        let mut words =
            Vec::with_capacity(usize::try_from(size / 4).map_err(|_| GravityComputeError::Budget)?);
        words.extend([
            parameters.constant.to_bits(),
            eps2.to_bits(),
            parameters.dt.to_bits(),
            count,
        ]);
        words.extend(parameters.uniform_acceleration.map(f32::to_bits));
        words.push(0);
        for body in bodies {
            words.extend(body.position.map(f32::to_bits));
            words.push(body.mass.to_bits());
            words.extend(body.velocity.map(f32::to_bits));
            words.push(0);
        }
        words.resize(words.len() + bodies.len() * 16, 0);
        let storage = self
            .memory_budget
            .allocate_storage("resident gravity state", bytemuck::cast_slice(&words))
            .map_err(GravityComputeError::Compute)?;
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gravity job"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage.as_entire_binding(),
            }],
        });
        Ok(GravityJob {
            _readback_pool: self.readback_pool.clone(),
            storage,
            binding,
            pipelines: self.pipelines.clone(),
            count,
            max_steps: self.budget.max_steps_per_encode,
        })
    }
}
#[derive(Debug)]
pub struct GravityJob {
    // Keep the shared staging cache alive even after the program is dropped.
    _readback_pool: voxy_render::ComputeReadbackPool,
    storage: voxy_render::ComputeStorage,
    binding: wgpu::BindGroup,
    pipelines: [wgpu::ComputePipeline; 3],
    count: u32,
    max_steps: u32,
}
impl GravityJob {
    /// Number of committed bodies in the resident buffer.
    #[must_use]
    pub fn body_count(&self) -> u32 {
        self.count
    }

    /// Resident storage for subsequent GPU passes. Committed Body records start
    /// at byte 32, each with position/mass vec4 followed by velocity/padding vec4.
    /// Callers must order reads after encoded steps and use this job's device.
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.storage
    }

    /// Records steps without host transfers. Pass boundaries enforce snapshot
    /// dependencies; failed jobs keep their prior input state and a sticky error.
    /// # Errors
    /// Rejects zero/excessive step counts before recording any command.
    pub fn encode_steps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        steps: u32,
    ) -> Result<(), GravityComputeError> {
        if steps == 0 || steps > self.max_steps {
            return Err(GravityComputeError::Budget);
        }
        for _ in 0..steps {
            for pipeline in &self.pipelines {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.binding, &[]);
                pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
            }
        }
        Ok(())
    }
    /// Copies only committed bodies and error metadata, after all prior passes.
    /// The resident state remains usable; submit before starting mapping.
    /// # Errors
    /// Reports readback buffer validation failures.
    pub fn encode_readback(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<GravityReadback, GravityComputeError> {
        let dispatch = ComputeDispatch::copy_buffer(
            device,
            encoder,
            &self.storage,
            0,
            32 + u64::from(self.count) * 32,
        )
        .map_err(GravityComputeError::Compute)?;
        Ok(GravityReadback {
            dispatch,
            count: self.count,
        })
    }
}
#[derive(Debug)]
pub struct GravityReadback {
    dispatch: ComputeDispatch,
    count: u32,
}
impl GravityReadback {
    #[must_use]
    pub fn begin_read(self) -> PendingGravity {
        PendingGravity {
            read: self.dispatch.begin_read(),
            count: self.count,
        }
    }
}
#[derive(Debug)]
pub struct PendingGravity {
    read: PendingComputeReadback,
    count: u32,
}
impl PendingGravity {
    /// Returns one validated snapshot. Singular/overflow failures publish no bodies.
    /// # Errors
    /// Reports GPU physics errors, malformed output, mapping errors or Consumed.
    pub fn try_read(&mut self) -> Result<Option<Vec<GravityBody>>, GravityComputeError> {
        let Some(bytes) = self.read.try_read().map_err(GravityComputeError::Compute)? else {
            return Ok(None);
        };
        if bytes.len()
            != 32 + usize::try_from(self.count).map_err(|_| GravityComputeError::Budget)? * 32
        {
            return Err(GravityComputeError::InvalidInput);
        }
        let words: Vec<_> = bytes
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        match words[7] {
            0 => {}
            1 => return Err(GravityComputeError::SingularPair),
            _ => return Err(GravityComputeError::NumericalOverflow),
        }
        let bodies = words[8..]
            .chunks_exact(8)
            .map(|body| GravityBody {
                position: [body[0], body[1], body[2]].map(f32::from_bits),
                mass: f32::from_bits(body[3]),
                velocity: [body[4], body[5], body[6]].map(f32::from_bits),
            })
            .collect::<Vec<_>>();
        if bodies.iter().any(|body| {
            !body.mass.is_finite()
                || body.mass <= 0.0
                || body
                    .position
                    .iter()
                    .chain(&body.velocity)
                    .any(|v| !v.is_finite())
        }) {
            return Err(GravityComputeError::NumericalOverflow);
        }
        Ok(Some(bodies))
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;

    fn fixture() -> ([GravityBody; 1], GravityParameters) {
        (
            [GravityBody {
                mass: 1.0,
                position: [0.0; 3],
                velocity: [0.0; 3],
            }],
            GravityParameters {
                constant: 0.0,
                softening: 0.0,
                uniform_acceleration: [0.0, -9.0, 0.0],
                dt: 1.0 / 60.0,
            },
        )
    }

    #[test]
    fn equal_ids_from_separate_instances_allocate_on_the_program_owner() {
        let (owner, _owner_queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let other = owner.clone();
        assert_eq!(owner, other);
        let program =
            pollster::block_on(GravityProgram::new(&owner, GravityBudget::default())).unwrap();
        let (bodies, parameters) = fixture();
        let job = program.create_job(&other, &bodies, parameters).unwrap();
        assert!(
            program
                .create_job(&owner.clone(), &bodies, parameters)
                .is_ok()
        );
        assert!(job.buffer().size() > 0);
    }

    #[test]
    fn unequal_device_handles_are_rejected_before_upload() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::NOOP,
            backend_options: wgpu::BackendOptions {
                noop: wgpu::NoopBackendOptions::enabled(),
                ..Default::default()
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (owner, _owner_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let (other, _other_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let program =
            pollster::block_on(GravityProgram::new(&owner, GravityBudget::default())).unwrap();
        let (bodies, parameters) = fixture();
        assert!(matches!(
            program.create_job(&other, &bodies, parameters),
            Err(GravityComputeError::Compute(ComputeError::DeviceMismatch))
        ));
    }
}
