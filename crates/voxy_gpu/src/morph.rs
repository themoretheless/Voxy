//! GPU Compute Vertex Morph / Blendshape Deformer.
//! Computes ellipsoidal C1-compact support blendshapes and head-space
//! coordinate transformations in parallel on GPU.

use glam::Mat4;
use wgpu::util::DeviceExt;

/// Compact GPU representation of a morph control (48 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMorphControl {
    pub center: [f32; 3],
    pub amount: f32,
    pub radius: [f32; 3],
    pub mode: u32,
    pub axis: u32,
    pub flags: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MorphUniform {
    vertex_count: u32,
    control_count: u32,
    _pad0: u32,
    _pad1: u32,
    head_inverse: [[f32; 4]; 4],
    head_transform: [[f32; 4]; 4],
}

#[derive(Debug)]
pub enum GpuMorphError {
    InvalidInput,
    Wgpu(String),
}

impl std::fmt::Display for GpuMorphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "Invalid morph input dimensions"),
            Self::Wgpu(msg) => write!(f, "GPU morph wgpu error: {msg}"),
        }
    }
}

impl std::error::Error for GpuMorphError {}

#[derive(Debug)]
pub struct GpuMorphProgram {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl GpuMorphProgram {
    /// Compiles WGSL morph compute shader and prepares execution pipeline.
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuMorphError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu morph shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("morph.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu morph layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu morph pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gpu morph pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_morph"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        if let Some(err) = scope.pop().await {
            return Err(GpuMorphError::Wgpu(err.to_string()));
        }

        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline,
        })
    }

    /// Evaluates morph blendshapes across vertices in parallel on GPU.
    /// # Errors
    /// Returns an error if buffer allocation or GPU mapping fails.
    pub fn deform(
        &self,
        queue: &wgpu::Queue,
        vertices: &[[f32; 3]],
        controls: &[GpuMorphControl],
        head_transform: Mat4,
    ) -> Result<Vec<[f32; 3]>, GpuMorphError> {
        if vertices.is_empty() {
            return Ok(Vec::new());
        }

        let vertex_count = vertices.len() as u32;
        let control_count = controls.len() as u32;

        let in_vec4: Vec<[f32; 4]> = vertices.iter().map(|&[x, y, z]| [x, y, z, 1.0]).collect();

        let uniform = MorphUniform {
            vertex_count,
            control_count,
            _pad0: 0,
            _pad1: 0,
            head_inverse: head_transform.inverse().to_cols_array_2d(),
            head_transform: head_transform.to_cols_array_2d(),
        };

        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("morph uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let in_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("morph in vertices"),
            contents: bytemuck::cast_slice(&in_vec4),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let dummy_ctrl = GpuMorphControl {
            center: [0.0; 3],
            amount: 0.0,
            radius: [1.0; 3],
            mode: 0,
            axis: 0,
            flags: 0,
            _pad0: 0,
            _pad1: 0,
        };
        let ctrl_data = if controls.is_empty() {
            std::slice::from_ref(&dummy_ctrl)
        } else {
            controls
        };

        let ctrl_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("morph controls"),
            contents: bytemuck::cast_slice(ctrl_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let out_bytes = u64::from(vertex_count) * 16;
        let out_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morph out vertices"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morph staging"),
            size: out_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("morph bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: in_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ctrl_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("morph encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("morph compute pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = vertex_count.div_ceil(64);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&out_buffer, 0, &staging, 0, out_bytes);
        let submission = queue.submit([encoder.finish()]);

        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuMorphError::Wgpu(e.to_string()))?;

        rx.recv()
            .map_err(|_| GpuMorphError::Wgpu("Staging mapping channel dropped".into()))?
            .map_err(|e| GpuMorphError::Wgpu(e.to_string()))?;

        let mapped = staging
            .slice(..)
            .get_mapped_range()
            .map_err(|e| GpuMorphError::Wgpu(e.to_string()))?;
        let output_vec4: &[[f32; 4]] = bytemuck::cast_slice(&mapped);
        let result: Vec<[f32; 3]> = output_vec4.iter().map(|&[x, y, z, _]| [x, y, z]).collect();
        drop(mapped);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_morph_deform_on_device() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let morph_program = pollster::block_on(GpuMorphProgram::new(&device))
            .expect("Morph program compilation failed");

        let vertices = vec![
            [0.0, 0.70, 0.10], // Facial vertex (y > 0.575)
            [0.0, 0.30, 0.00], // Body vertex (y < 0.575, should not deform)
        ];

        let controls = vec![GpuMorphControl {
            center: [0.0, 0.70, 0.10],
            amount: 5.0,
            radius: [0.05, 0.05, 0.05],
            mode: 1, // Translate
            axis: 1, // Y
            flags: 0,
            _pad0: 0,
            _pad1: 0,
        }];

        let deformed = morph_program
            .deform(&queue, &vertices, &controls, Mat4::IDENTITY)
            .expect("GPU morph execution failed");

        assert_eq!(deformed.len(), 2);
        // Face vertex shifted upwards along Y
        assert!(deformed[0][1] > vertices[0][1]);
        // Body vertex remained untouched
        assert!((deformed[1][1] - vertices[1][1]).abs() < 1e-5);
    }
}
