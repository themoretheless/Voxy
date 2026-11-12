//! GPU Cellular Automata Voxel Lighting Pipeline.
//! Executes 3D stencil light propagation on 34x34x34 padded volume entirely on hardware.

use std::sync::Mutex;
use voxy_gpu_core::ComputeError;
use wgpu::util::DeviceExt;

pub use voxy_core::{CHUNK_VOLUME, PADDED_VOLUME};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightingConfig {
    preserve_direct_down: u32,
    sky_from_above: u32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Debug)]
pub enum GpuLightingError {
    InvalidInput,
    Compute(ComputeError),
    Wgpu(String),
}

impl std::fmt::Display for GpuLightingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compute(err) => write!(f, "GPU lighting compute error: {err}"),
            Self::Wgpu(msg) => write!(f, "GPU lighting wgpu error: {msg}"),
            Self::InvalidInput => write!(f, "GPU lighting invalid input dimensions"),
        }
    }
}

impl std::error::Error for GpuLightingError {}

#[derive(Debug)]
struct LightingScratch {
    opaque_words: Vec<u32>,
    emission_words: Vec<u32>,
    opaque_buffer: wgpu::Buffer,
    emission_buffer: wgpu::Buffer,
    buffer_a: wgpu::Buffer,
    sky_final_buffer: wgpu::Buffer,
    packed_output_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    bg_sky_open_ab: wgpu::BindGroup,
    bg_sky_open_ba: wgpu::BindGroup,
    bg_sky_closed_ab: wgpu::BindGroup,
    bg_sky_closed_ba: wgpu::BindGroup,
    bg_block_ab: wgpu::BindGroup,
    bg_block_ba: wgpu::BindGroup,
    bg_pack: wgpu::BindGroup,
}

/// GPU Compute Pipeline for Cellular Automata voxel light propagation.
#[derive(Debug)]
pub struct GpuLightingProgram {
    device: wgpu::Device,
    pipeline_seed_sky: wgpu::ComputePipeline,
    pipeline_step_light: wgpu::ComputePipeline,
    pipeline_pack: wgpu::ComputePipeline,
    scratch: Mutex<LightingScratch>,
}

impl GpuLightingProgram {
    /// Creates a new GPU lighting compute pipeline and reusable scratch buffers.
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuLightingError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cellular automata lighting shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lighting.wgsl").into()),
        });

        let bind_group_layout = voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(
            Some("lighting storage layout"),
            5,
        )
        .storage_buffer(0, wgpu::ShaderStages::COMPUTE, true)
        .storage_buffer(1, wgpu::ShaderStages::COMPUTE, true)
        .storage_buffer(2, wgpu::ShaderStages::COMPUTE, false)
        .storage_buffer(3, wgpu::ShaderStages::COMPUTE, false)
        .storage_buffer(4, wgpu::ShaderStages::COMPUTE, false)
        .build(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lighting pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline_seed_sky = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lighting seed sky"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_seed_sky"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_step_light =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("lighting step light"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_step_light"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let pipeline_pack = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lighting pack"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_pack"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Immutable per-stage configs prevent queue.write_buffer timeline hazards
        // between sky seeding and block-light propagation within a single submission.
        let make_config = |label, preserve_direct_down, sky_from_above| {
            let cfg = LightingConfig {
                preserve_direct_down,
                sky_from_above,
                _pad0: 0,
                _pad1: 0,
            };
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::bytes_of(&cfg),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let config_sky_open = make_config("lighting config sky open", 1, 1);
        let config_sky_closed = make_config("lighting config sky closed", 1, 0);
        let config_block = make_config("lighting config block", 0, 0);

        let padded_bytes = (PADDED_VOLUME * 4) as u64;
        let opaque_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting opaque buffer"),
            size: padded_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let emission_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting emission upload"),
            size: padded_bytes,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting buffer A"),
            size: padded_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting buffer B"),
            size: padded_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let sky_final_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky final buffer"),
            size: padded_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let packed_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting packed output"),
            size: CHUNK_VOLUME as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting staging readback"),
            size: CHUNK_VOLUME as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let make_group = |label, cfg: &wgpu::Buffer, b2: &wgpu::Buffer, b3: &wgpu::Buffer| {
            voxy_gpu_core::BindGroupBuilder::with_capacity(Some(label), 5)
                .buffer(0, cfg)
                .buffer(1, &opaque_buffer)
                .buffer(2, b2)
                .buffer(3, b3)
                .buffer(4, &packed_output_buffer)
                .build(device, &bind_group_layout)
        };
        let bg_sky_open_ab = make_group(
            "lighting sky open A->B",
            &config_sky_open,
            &buffer_a,
            &buffer_b,
        );
        let bg_sky_open_ba = make_group(
            "lighting sky open B->A",
            &config_sky_open,
            &buffer_b,
            &buffer_a,
        );
        let bg_sky_closed_ab = make_group(
            "lighting sky closed A->B",
            &config_sky_closed,
            &buffer_a,
            &buffer_b,
        );
        let bg_sky_closed_ba = make_group(
            "lighting sky closed B->A",
            &config_sky_closed,
            &buffer_b,
            &buffer_a,
        );
        let bg_block_ab = make_group("lighting block A->B", &config_block, &buffer_a, &buffer_b);
        let bg_block_ba = make_group("lighting block B->A", &config_block, &buffer_b, &buffer_a);
        let bg_pack = make_group("lighting pack", &config_block, &sky_final_buffer, &buffer_a);

        if let Some(err) = scope.pop().await {
            return Err(GpuLightingError::Wgpu(err.to_string()));
        }

        Ok(Self {
            device: device.clone(),
            pipeline_seed_sky,
            pipeline_step_light,
            pipeline_pack,
            scratch: Mutex::new(LightingScratch {
                opaque_words: vec![0; PADDED_VOLUME],
                emission_words: vec![0; PADDED_VOLUME],
                opaque_buffer,
                emission_buffer,
                buffer_a,
                sky_final_buffer,
                packed_output_buffer,
                staging_buffer,
                bg_sky_open_ab,
                bg_sky_open_ba,
                bg_sky_closed_ab,
                bg_sky_closed_ba,
                bg_block_ab,
                bg_block_ba,
                bg_pack,
            }),
        })
    }

    /// Propagates sky and block light through 3D cellular automata.
    /// Returns 32,768 packed light bytes for the inner 32x32x32 chunk: `(sky << 4) | block`.
    /// # Errors
    /// Returns an error if input arrays have invalid length or execution fails.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn propagate(
        &self,
        queue: &wgpu::Queue,
        opaque: &[bool],
        emission: &[u8],
        sky_from_above: bool,
    ) -> Result<Vec<u8>, GpuLightingError> {
        if opaque.len() != PADDED_VOLUME || emission.len() != PADDED_VOLUME {
            return Err(GpuLightingError::InvalidInput);
        }

        let mut scratch = self
            .scratch
            .lock()
            .map_err(|_| GpuLightingError::Wgpu("lighting scratch mutex poisoned".into()))?;

        for (dst, &b) in scratch.opaque_words.iter_mut().zip(opaque.iter()) {
            *dst = u32::from(b);
        }
        queue.write_buffer(
            &scratch.opaque_buffer,
            0,
            bytemuck::cast_slice(&scratch.opaque_words),
        );

        let mut has_emission = false;
        for (dst, &e) in scratch.emission_words.iter_mut().zip(emission.iter()) {
            *dst = u32::from(e);
            has_emission |= e > 0;
        }
        if has_emission {
            queue.write_buffer(
                &scratch.emission_buffer,
                0,
                bytemuck::cast_slice(&scratch.emission_words),
            );
        }

        let (sky_ab, sky_ba) = if sky_from_above {
            (&scratch.bg_sky_open_ab, &scratch.bg_sky_open_ba)
        } else {
            (&scratch.bg_sky_closed_ab, &scratch.bg_sky_closed_ba)
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("cellular lighting encoder"),
            });

        // Pass 1: Skylight seeding + 16 ping-pong stencil dispatches inside a single compute pass
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lighting sky pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_seed_sky);
            pass.set_bind_group(0, sky_ab, &[]);
            pass.dispatch_workgroups(5, 5, 1);

            pass.set_pipeline(&self.pipeline_step_light);
            for _ in 0..8 {
                pass.set_bind_group(0, sky_ab, &[]);
                pass.dispatch_workgroups(9, 9, 9);
                pass.set_bind_group(0, sky_ba, &[]);
                pass.dispatch_workgroups(9, 9, 9);
            }
        }

        let padded_bytes = (PADDED_VOLUME * 4) as u64;
        encoder.copy_buffer_to_buffer(
            &scratch.buffer_a,
            0,
            &scratch.sky_final_buffer,
            0,
            padded_bytes,
        );

        if has_emission {
            encoder.copy_buffer_to_buffer(
                &scratch.emission_buffer,
                0,
                &scratch.buffer_a,
                0,
                padded_bytes,
            );
        } else {
            encoder.clear_buffer(&scratch.buffer_a, 0, None);
        }

        // Pass 2: Block-light stencil dispatches (when needed) + byte packing
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lighting block and pack pass"),
                timestamp_writes: None,
            });
            if has_emission {
                pass.set_pipeline(&self.pipeline_step_light);
                for _ in 0..8 {
                    pass.set_bind_group(0, &scratch.bg_block_ab, &[]);
                    pass.dispatch_workgroups(9, 9, 9);
                    pass.set_bind_group(0, &scratch.bg_block_ba, &[]);
                    pass.dispatch_workgroups(9, 9, 9);
                }
            }
            pass.set_pipeline(&self.pipeline_pack);
            pass.set_bind_group(0, &scratch.bg_pack, &[]);
            pass.dispatch_workgroups(4, 8, 8);
        }

        encoder.copy_buffer_to_buffer(
            &scratch.packed_output_buffer,
            0,
            &scratch.staging_buffer,
            0,
            CHUNK_VOLUME as u64,
        );

        let submission = queue.submit([encoder.finish()]);

        let slice = scratch.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuLightingError::Wgpu(e.to_string()))?;

        rx.recv()
            .map_err(|_| GpuLightingError::Wgpu("readback channel dropped".into()))?
            .map_err(|e| GpuLightingError::Wgpu(e.to_string()))?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|e| GpuLightingError::Wgpu(e.to_string()))?;
        let packed_bytes = mapped.as_ref().to_vec();
        drop(mapped);
        scratch.staging_buffer.unmap();

        Ok(packed_bytes)
    }
}

/// Estimates the total number of 3D mip-pyramid voxel texels (`32^3 + 16^3 + 8^3 + 4^3 + 2^3 + 1 = 37,449`
/// per chunk) that must be updated when `dirty_chunk_count` lighting volumes are invalidated.
#[must_use]
pub const fn estimate_dirty_light_mip_voxels(dirty_chunk_count: u32) -> u64 {
    const MIP_CHAIN_VOXELS_PER_CHUNK: u64 = 32768 + 4096 + 512 + 64 + 8 + 1;
    (dirty_chunk_count as u64).saturating_mul(MIP_CHAIN_VOXELS_PER_CHUNK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_lighting_skylight_unobstructed() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        else {
            return;
        };

        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let program = pollster::block_on(GpuLightingProgram::new(&device)).unwrap();

        let opaque = vec![false; PADDED_VOLUME];
        let emission = vec![0u8; PADDED_VOLUME];

        let result = program.propagate(&queue, &opaque, &emission, true).unwrap();
        assert_eq!(result.len(), CHUNK_VOLUME);

        for &byte in &result {
            let sky = byte >> 4;
            let block = byte & 0x0f;
            assert_eq!(sky, 15);
            assert_eq!(block, 0);
        }
    }

    #[test]
    fn gpu_lighting_block_emission_spreads() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        else {
            return;
        };

        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let program = pollster::block_on(GpuLightingProgram::new(&device)).unwrap();

        let opaque = vec![false; PADDED_VOLUME];
        let mut emission = vec![0u8; PADDED_VOLUME];

        let center_idx = 16 + 16 * 34 + 16 * 1156;
        emission[center_idx] = 14;

        let result = program
            .propagate(&queue, &opaque, &emission, false)
            .unwrap();
        assert_eq!(result.len(), CHUNK_VOLUME);

        let local_center = 15 + 32 * (15 + 32 * 15);
        let center_block = result[local_center] & 0x0f;
        assert_eq!(center_block, 14);

        let neighbor_local = 16 + 32 * (15 + 32 * 15);
        let neighbor_block = result[neighbor_local] & 0x0f;
        assert_eq!(neighbor_block, 13);
        assert_eq!(
            estimate_dirty_light_mip_voxels(2),
            2 * (32768 + 4096 + 512 + 64 + 8 + 1)
        );
    }

    #[test]
    fn gpu_lighting_skylight_preserved_when_block_emission_present() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        else {
            return;
        };

        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let program = pollster::block_on(GpuLightingProgram::new(&device)).unwrap();

        let opaque = vec![false; PADDED_VOLUME];
        let mut emission = vec![0u8; PADDED_VOLUME];
        let center_idx = 16 + 16 * 34 + 16 * 1156;
        emission[center_idx] = 14;

        let result = program.propagate(&queue, &opaque, &emission, true).unwrap();
        assert_eq!(result.len(), CHUNK_VOLUME);

        let local_center = 15 + 32 * (15 + 32 * 15);
        assert_eq!(result[local_center] >> 4, 15);
        assert_eq!(result[local_center] & 0x0f, 14);

        let far_corner = 0;
        assert_eq!(result[far_corner] >> 4, 15);
        assert_eq!(result[far_corner] & 0x0f, 0);
    }
}
