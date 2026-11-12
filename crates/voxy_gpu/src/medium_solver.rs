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
        let bind_group_layout =
            voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(Some("gpu medium layout"), 5)
                .uniform_buffer(0, wgpu::ShaderStages::COMPUTE)
                .storage_buffer(1, wgpu::ShaderStages::COMPUTE, true)
                .storage_buffer(2, wgpu::ShaderStages::COMPUTE, true)
                .storage_buffer(3, wgpu::ShaderStages::COMPUTE, false)
                .storage_buffer(4, wgpu::ShaderStages::COMPUTE, false)
                .build(device);
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
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
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
                voxy_gpu_core::BindGroupBuilder::with_capacity(Some("medium bind group"), 5)
                    .buffer(0, &uniform)
                    .buffer(1, &velocity)
                    .buffer(2, &buffers[i])
                    .buffer(3, &buffers[1 - i])
                    .buffer(4, &settled)
                    .build(&self.device, &self.bind_group_layout)
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

/// Evaluates the normalized 3D Henyey–Greenstein anisotropic scattering phase function
/// $\Phi(\cos\theta; g) = \frac{1 - g^2}{4\pi (1 + g^2 - 2 g \cos\theta)^{3/2}}$
/// for cosine scattering angle `cos_theta` $\in [-1, 1]$ and asymmetry factor `g` $\in (-1, 1)$.
#[must_use]
pub fn henyey_greenstein_phase(cos_theta: f32, g: f32) -> f32 {
    if !cos_theta.is_finite() || !g.is_finite() {
        return 0.0;
    }
    let mu = cos_theta.clamp(-1.0, 1.0);
    let g_clamped = g.clamp(-0.99, 0.99);
    let g2 = g_clamped * g_clamped;
    let denom_base = (1.0 + g2 - 2.0 * g_clamped * mu).max(1e-5);
    (1.0 - g2) / (4.0 * std::f32::consts::PI * denom_base * denom_base.sqrt())
}

#[cfg(test)]
mod phase_tests {
    use super::*;

    #[test]
    fn henyey_greenstein_favors_forward_scattering_for_positive_g() {
        let iso = henyey_greenstein_phase(0.0, 0.0);
        assert!((iso - 1.0 / (4.0 * std::f32::consts::PI)).abs() < 1e-6);
        let fwd = henyey_greenstein_phase(1.0, 0.7);
        let back = henyey_greenstein_phase(-1.0, 0.7);
        assert!(fwd > back * 10.0);
    }
}
