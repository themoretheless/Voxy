//! GPU crowd pose evaluation: resampled clip tables and blend jobs in, one
//! skin palette per instance out. The reference is
//! [`voxy_animation::cpu_reference_palette`].

use glam::Mat4;
use voxy_animation::{GpuBlendJob, GpuClipTable, GpuSkeletonLayout, MAX_JOINTS};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Config {
    job_count: u32,
    joint_count: u32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum GpuPoseEvalError {
    #[error("pose eval input is empty, mismatched or exceeds the joint budget")]
    InvalidInput,
    #[error("GPU pose eval wgpu error: {0}")]
    Wgpu(String),
}

#[derive(Debug)]
pub struct GpuPoseEvalProgram {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

/// Uploaded clip tables shared by many dispatches.
#[derive(Debug)]
pub struct GpuClipSet {
    headers: wgpu::Buffer,
    keys: wgpu::Buffer,
    joint_count: u32,
    clip_count: u32,
}

impl GpuPoseEvalProgram {
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuPoseEvalError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu pose eval shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pose_eval.wgsl").into()),
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
            label: Some("gpu pose eval layout"),
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
                storage(3, true),
                storage(4, true),
                storage(5, true),
                storage(6, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu pose eval pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gpu pose eval pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_pose_eval"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        if let Some(err) = scope.pop().await {
            return Err(GpuPoseEvalError::Wgpu(err.to_string()));
        }
        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline,
        })
    }

    /// Uploads clip tables; all tables must share one joint count.
    ///
    /// # Errors
    /// Rejects an empty set, mixed joint counts or more than `MAX_JOINTS` joints.
    pub fn upload_clips(&self, tables: &[GpuClipTable]) -> Result<GpuClipSet, GpuPoseEvalError> {
        let Some(first) = tables.first() else {
            return Err(GpuPoseEvalError::InvalidInput);
        };
        let joint_count = first.joint_count;
        if joint_count == 0
            || joint_count as usize > MAX_JOINTS
            || tables
                .iter()
                .any(|t| t.joint_count != joint_count || t.sample_count < 2)
        {
            return Err(GpuPoseEvalError::InvalidInput);
        }
        let mut headers: Vec<u32> = Vec::with_capacity(tables.len() * 8);
        let mut keys: Vec<f32> = Vec::new();
        for table in tables {
            // Offsets count vec4 slots: three per key.
            let key_offset = (keys.len() / 4) as u32;
            headers.extend(table.header(key_offset));
            keys.extend(table.to_floats());
        }
        let headers = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval clip headers"),
                contents: bytemuck::cast_slice(&headers),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let keys = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval clip keys"),
                contents: bytemuck::cast_slice(&keys),
                usage: wgpu::BufferUsages::STORAGE,
            });
        Ok(GpuClipSet {
            headers,
            keys,
            joint_count,
            clip_count: tables.len() as u32,
        })
    }

    /// Evaluates all jobs and reads back `instance_count` palettes
    /// (instance-major). Instances without a job keep identity.
    ///
    /// # Errors
    /// Rejects mismatched layouts, out-of-range jobs, or GPU failures.
    pub fn evaluate(
        &self,
        queue: &wgpu::Queue,
        clips: &GpuClipSet,
        layout: &GpuSkeletonLayout,
        jobs: &[GpuBlendJob],
        instance_count: u32,
    ) -> Result<Vec<Mat4>, GpuPoseEvalError> {
        let joint_count = clips.joint_count;
        if layout.parents.len() as u32 != joint_count
            || layout.inverse_binds.len() as u32 != joint_count
            || jobs.is_empty()
            || instance_count == 0
            || jobs.iter().any(|j| {
                j.instance >= instance_count
                    || j.clip_a >= clips.clip_count
                    || j.clip_b >= clips.clip_count
                    || !j.phase.is_finite()
                    || !j.weight.is_finite()
            })
        {
            return Err(GpuPoseEvalError::InvalidInput);
        }
        let config = Config {
            job_count: jobs.len() as u32,
            joint_count,
            _pad0: 0,
            _pad1: 0,
        };
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval config"),
                contents: bytemuck::bytes_of(&config),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let job_words: Vec<u32> = jobs.iter().flat_map(|j| j.to_words()).collect();
        let jobs_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval jobs"),
                contents: bytemuck::cast_slice(&job_words),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let parents = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval parents"),
                contents: bytemuck::cast_slice(&layout.parents),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let binds: Vec<[[f32; 4]; 4]> = layout
            .inverse_binds
            .iter()
            .map(|m| m.to_cols_array_2d())
            .collect();
        let inverse_binds = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval inverse binds"),
                contents: bytemuck::cast_slice(&binds),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let identity: Vec<[[f32; 4]; 4]> =
            vec![Mat4::IDENTITY.to_cols_array_2d(); (instance_count * joint_count) as usize];
        let out_bytes = (identity.len() * 64) as u64;
        let palettes = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pose eval palettes"),
                contents: bytemuck::cast_slice(&identity),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pose eval staging"),
            size: out_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pose eval bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: clips.headers.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: clips.keys.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: jobs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: parents.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: inverse_binds.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: palettes.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pose eval encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pose eval pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((jobs.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&palettes, 0, &staging, 0, out_bytes);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuPoseEvalError::Wgpu(e.to_string()))?;
        rx.recv()
            .map_err(|_| GpuPoseEvalError::Wgpu("staging mapping channel dropped".into()))?
            .map_err(|e| GpuPoseEvalError::Wgpu(e.to_string()))?;
        let mapped = staging
            .slice(..)
            .get_mapped_range()
            .map_err(|e| GpuPoseEvalError::Wgpu(e.to_string()))?;
        let cols: &[[[f32; 4]; 4]] = bytemuck::cast_slice(&mapped);
        let result: Vec<Mat4> = cols.iter().map(Mat4::from_cols_array_2d).collect();
        drop(mapped);
        staging.unmap();
        Ok(result)
    }
}
