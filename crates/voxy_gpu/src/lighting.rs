//! GPU Cellular Automata Voxel Lighting Pipeline.
//! Executes 3D stencil light propagation on 34x34x34 padded volume entirely on hardware.

use voxy_render::ComputeError;
use wgpu::util::DeviceExt;

pub const PADDED_VOLUME: usize = 34 * 34 * 34;
pub const CHUNK_VOLUME: usize = 32 * 32 * 32;

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

/// GPU Compute Pipeline for Cellular Automata voxel light propagation.
#[derive(Debug)]
pub struct GpuLightingProgram {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_seed_sky: wgpu::ComputePipeline,
    pipeline_step_light: wgpu::ComputePipeline,
    pipeline_pack: wgpu::ComputePipeline,
}

impl GpuLightingProgram {
    /// Creates a new GPU lighting compute pipeline.
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuLightingError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cellular automata lighting shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lighting.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting storage layout"),
            entries: &[
                // binding 0: config
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: opaque
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: light_in
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: light_out
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 4: packed_output
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

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

        let pipeline_step_light = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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

        if let Some(err) = scope.pop().await {
            return Err(GpuLightingError::Wgpu(err.to_string()));
        }

        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline_seed_sky,
            pipeline_step_light,
            pipeline_pack,
        })
    }

    /// Propagates sky and block light through 3D cellular automata.
    /// Returns 32,768 packed light bytes for the inner 32x32x32 chunk: `(sky << 4) | block`.
    /// # Errors
    /// Returns an error if input arrays have invalid length or execution fails.
    #[allow(clippy::similar_names)]
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

        let opaque_words: Vec<u32> = opaque.iter().map(|&b| if b { 1 } else { 0 }).collect();
        let emission_words: Vec<u32> = emission.iter().map(|&e| u32::from(e)).collect();
        let has_emission = emission.iter().any(|&e| e > 0);

        let config_sky = LightingConfig {
            preserve_direct_down: 1,
            sky_from_above: if sky_from_above { 1 } else { 0 },
            _pad0: 0,
            _pad1: 0,
        };

        let config_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting config buffer"),
            contents: bytemuck::bytes_of(&config_sky),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let opaque_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting opaque buffer"),
            contents: bytemuck::cast_slice(&opaque_words),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Buffers A and B for ping-pong
        let buffer_a = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting buffer A"),
            size: (PADDED_VOLUME * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let buffer_b = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting buffer B"),
            size: (PADDED_VOLUME * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Buffer to store final sky light field before block light is processed
        let sky_final_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky final buffer"),
            size: (PADDED_VOLUME * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let packed_output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting packed output"),
            size: (CHUNK_VOLUME * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lighting staging readback"),
            size: (CHUNK_VOLUME * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group 1: A -> B
        let bind_group_ab = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting bind group A->B"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: opaque_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: packed_output_buffer.as_entire_binding(),
                },
            ],
        });

        // Bind group 2: B -> A
        let bind_group_ba = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting bind group B->A"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: opaque_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: packed_output_buffer.as_entire_binding(),
                },
            ],
        });

        // Final pack bind group: sky_final in binding 2, block in binding 3
        let bind_group_pack = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting bind group pack"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: opaque_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sky_final_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: packed_output_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cellular lighting encoder"),
        });

        // Step 1: Skylight seeding and 15 iterations of CA
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lighting seed sky pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_seed_sky);
            pass.set_bind_group(0, &bind_group_ab, &[]);
            pass.dispatch_workgroups(5, 5, 1); // 34x34 with (8, 8, 1) workgroups
        }

        // 15 ping-pong iterations: A->B, B->A, etc.
        // Even number (14 steps) ends on A, 15 steps ends on B.
        // Let's do 16 steps (8 pairs) so the result ends deterministically in Buffer A!
        for _ in 0..8 {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lighting step A->B"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline_step_light);
                pass.set_bind_group(0, &bind_group_ab, &[]);
                pass.dispatch_workgroups(9, 9, 9); // 34x34x34 with (4, 4, 4) workgroups
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lighting step B->A"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline_step_light);
                pass.set_bind_group(0, &bind_group_ba, &[]);
                pass.dispatch_workgroups(9, 9, 9);
            }
        }

        // Save sky light into sky_final_buffer
        encoder.copy_buffer_to_buffer(&buffer_a, 0, &sky_final_buffer, 0, (PADDED_VOLUME * 4) as u64);

        // Step 2: Block light propagation
        if has_emission {
            encoder.copy_buffer_to_buffer(
                &self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("emission upload"),
                    contents: bytemuck::cast_slice(&emission_words),
                    usage: wgpu::BufferUsages::COPY_SRC,
                }),
                0,
                &buffer_a,
                0,
                (PADDED_VOLUME * 4) as u64,
            );

            // Update config for block light: preserve_direct_down = 0
            let config_block = LightingConfig {
                preserve_direct_down: 0,
                sky_from_above: 0,
                _pad0: 0,
                _pad1: 0,
            };
            queue.write_buffer(&config_buffer, 0, bytemuck::bytes_of(&config_block));

            for _ in 0..8 {
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("block light step A->B"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.pipeline_step_light);
                    pass.set_bind_group(0, &bind_group_ab, &[]);
                    pass.dispatch_workgroups(9, 9, 9);
                }
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("block light step B->A"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.pipeline_step_light);
                    pass.set_bind_group(0, &bind_group_ba, &[]);
                    pass.dispatch_workgroups(9, 9, 9);
                }
            }
        } else {
            // Zero out buffer A
            encoder.clear_buffer(&buffer_a, 0, None);
        }

        // Step 3: Pack sky and block light into packed_output
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lighting pack pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_pack);
            pass.set_bind_group(0, &bind_group_pack, &[]);
            pass.dispatch_workgroups(8, 8, 8); // 32x32x32 with (4, 4, 4) workgroups
        }

        encoder.copy_buffer_to_buffer(
            &packed_output_buffer,
            0,
            &staging_buffer,
            0,
            (CHUNK_VOLUME * 4) as u64,
        );

        let submission = queue.submit([encoder.finish()]);

        let slice = staging_buffer.slice(..);
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
        let words: &[u32] = bytemuck::cast_slice(&mapped);
        let mut packed_bytes = Vec::with_capacity(CHUNK_VOLUME);
        for &word in words {
            packed_bytes.push(word as u8);
        }
        drop(mapped);
        staging_buffer.unmap();

        Ok(packed_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_lighting_skylight_unobstructed() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let program = pollster::block_on(GpuLightingProgram::new(&device)).unwrap();

        // Completely open sky (all false opaque, no emission)
        let opaque = vec![false; PADDED_VOLUME];
        let emission = vec![0u8; PADDED_VOLUME];

        let result = program.propagate(&queue, &opaque, &emission, true).unwrap();
        assert_eq!(result.len(), CHUNK_VOLUME);

        // Under open sky, every voxel should have full skylight (15 << 4 = 240) and zero block light
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
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let program = pollster::block_on(GpuLightingProgram::new(&device)).unwrap();

        let opaque = vec![false; PADDED_VOLUME];
        let mut emission = vec![0u8; PADDED_VOLUME];

        // Torch at padded (16, 16, 16)
        let center_idx = 16 + 16 * 34 + 16 * 1156;
        emission[center_idx] = 14;

        let result = program.propagate(&queue, &opaque, &emission, false).unwrap();
        assert_eq!(result.len(), CHUNK_VOLUME);

        // Center voxel (local 15, 15, 15)
        let local_center = 15 + 32 * (15 + 32 * 15);
        let center_block = result[local_center] & 0x0f;
        assert_eq!(center_block, 14);

        // Voxel 1 block away (local 16, 15, 15) should have 13
        let neighbor_local = 16 + 32 * (15 + 32 * 15);
        let neighbor_block = result[neighbor_local] & 0x0f;
        assert_eq!(neighbor_block, 13);
    }
}
