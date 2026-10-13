//! GPU transport of one participating-medium species (plan step B3). The
//! kernel mirrors the CPU reference in `physics::medium`; the reference
//! decides the substep count and remains the admission authority.

use physics::medium::MediumSpeciesProperties;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Config {
    shape: [u32; 3],
    cells: u32,
    spacing: [f32; 3],
    h: f32,
    up: [f32; 3],
    settling: f32,
    diffusivity: f32,
    volume: f32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum GpuMediumError {
    #[error("medium transport input is empty, mismatched or nonfinite")]
    InvalidInput,
    #[error("GPU medium wgpu error: {0}")]
    Wgpu(String),
}

/// Grid geometry and species transport parameters for one job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediumTransportJob {
    pub shape: [usize; 3],
    pub spacing: [f64; 3],
    pub up: [f64; 3],
    pub dt: f64,
    pub substeps: usize,
}

/// Result of one species transport job.
#[derive(Clone, Debug, PartialEq)]
pub struct MediumTransportResult {
    pub mass_kg: Vec<f32>,
    /// Mass that left through the bottom face by settling.
    pub settled_kg: f32,
}

#[derive(Debug)]
pub struct GpuMediumProgram {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl GpuMediumProgram {
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuMediumError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu medium shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("medium_solver.wgsl").into()),
        });
        let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu medium layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1, true),
                storage(2, true),
                storage(3, false),
                storage(4, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu medium pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gpu medium pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_medium_substep"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        if let Some(err) = scope.pop().await {
            return Err(GpuMediumError::Wgpu(err.to_string()));
        }
        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline,
        })
    }

    /// Runs `job.substeps` substeps for one species and reads the mass back.
    /// `gas_velocity` is per cell; `mass_kg` is the species mass per cell.
    /// # Errors
    /// Rejects mismatched sizes, nonfinite input or GPU failures.
    pub fn transport(
        &self,
        queue: &wgpu::Queue,
        job: MediumTransportJob,
        species: &MediumSpeciesProperties,
        gas_velocity: &[[f32; 3]],
        mass_kg: &[f32],
    ) -> Result<MediumTransportResult, GpuMediumError> {
        let cells = job.shape.iter().product::<usize>();
        if cells == 0
            || cells > u32::MAX as usize
            || gas_velocity.len() != cells
            || mass_kg.len() != cells
            || job.substeps == 0
            || !job.dt.is_finite()
            || job.dt < 0.
            || job.spacing.iter().any(|s| !s.is_finite() || *s <= 0.)
            || job.up.iter().any(|u| !u.is_finite())
            || gas_velocity.iter().flatten().any(|v| !v.is_finite())
            || mass_kg.iter().any(|m| !m.is_finite() || *m < 0.)
        {
            return Err(GpuMediumError::InvalidInput);
        }
        let config = Config {
            shape: job.shape.map(|n| n as u32),
            cells: cells as u32,
            spacing: job.spacing.map(|s| s as f32),
            h: (job.dt / job.substeps as f64) as f32,
            up: job.up.map(|u| u as f32),
            settling: species.settling_velocity_m_s as f32,
            diffusivity: species.diffusivity_m2_s as f32,
            volume: job.spacing.iter().product::<f64>() as f32,
            _pad0: 0,
            _pad1: 0,
        };
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("medium config"),
                contents: bytemuck::bytes_of(&config),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let velocity: Vec<[f32; 4]> = gas_velocity
            .iter()
            .map(|v| [v[0], v[1], v[2], 0.])
            .collect();
        let velocity = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("medium velocity"),
                contents: bytemuck::cast_slice(&velocity),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let bytes = (cells * 4) as u64;
        let make = |label: &str, contents: &[f32]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(contents),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                })
        };
        let buffers = [
            make("medium mass a", mass_kg),
            make("medium mass b", mass_kg),
        ];
        let settled = make("medium settled", &vec![0.; cells]);
        let bind_groups: Vec<wgpu::BindGroup> = (0..2)
            .map(|i| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("medium bind group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: velocity.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: buffers[i].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: buffers[1 - i].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: settled.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
        let staging_mass = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("medium staging mass"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_settled = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("medium staging settled"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("medium encoder"),
            });
        for step in 0..job.substeps {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("medium substep"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_groups[step % 2], &[]);
            pass.dispatch_workgroups((cells as u32).div_ceil(64), 1, 1);
        }
        let final_buffer = &buffers[job.substeps % 2];
        encoder.copy_buffer_to_buffer(final_buffer, 0, &staging_mass, 0, bytes);
        encoder.copy_buffer_to_buffer(&settled, 0, &staging_settled, 0, bytes);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();
        staging_mass
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        staging_settled
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx2.send(r);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuMediumError::Wgpu(e.to_string()))?;
        for receiver in [rx, rx2] {
            receiver
                .recv()
                .map_err(|_| GpuMediumError::Wgpu("staging channel dropped".into()))?
                .map_err(|e| GpuMediumError::Wgpu(e.to_string()))?;
        }
        let mass = {
            let mapped = staging_mass
                .slice(..)
                .get_mapped_range()
                .map_err(|e| GpuMediumError::Wgpu(e.to_string()))?;
            let out: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
            out
        };
        staging_mass.unmap();
        let settled_total = {
            let mapped = staging_settled
                .slice(..)
                .get_mapped_range()
                .map_err(|e| GpuMediumError::Wgpu(e.to_string()))?;
            let out: &[f32] = bytemuck::cast_slice(&mapped);
            out.iter().sum::<f32>()
        };
        staging_settled.unmap();
        Ok(MediumTransportResult {
            mass_kg: mass,
            settled_kg: settled_total,
        })
    }
}
