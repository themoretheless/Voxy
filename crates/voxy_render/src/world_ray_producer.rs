//! Live archive-frame guides -> exact world ray input, entirely on GPU.
use crate::{ComputeMemoryBudget, ComputeStorage, ExactWorldNearest, RaySceneError};
use std::sync::Arc;
#[doc(hidden)]
pub const WORLD_RAY_PRODUCER_WGSL: &str = include_str!("world_ray_producer.wgsl");
#[derive(Clone, Copy, Debug)]
pub enum WorldRayKind {
    Diffuse,
    Reflection,
}
/// Immutable pixel region; prepare again after resize or source-owner replacement.
/// The retained camera/guide bindings follow subsequent frames of the same renderer.
#[derive(Debug)]
pub struct ArchiveWorldRayProducer {
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 2],
    _camera: Arc<ComputeStorage>,
    _tile: ComputeStorage,
    _input: Arc<ComputeStorage>,
    _guides: [Arc<voxy_gpu_core::ManagedTexture>; 2],
    extent: [u32; 2],
    count: u32,
}
impl ArchiveWorldRayProducer {
    pub(crate) fn new(
        device: &wgpu::Device,
        camera: Arc<ComputeStorage>,
        guides: [Arc<voxy_gpu_core::ManagedTexture>; 2],
        job: &ExactWorldNearest,
        kind: WorldRayKind,
        offset: [u32; 2],
        extent: [u32; 2],
    ) -> Result<Self, RaySceneError> {
        let count = extent[0]
            .checked_mul(extent[1])
            .ok_or(RaySceneError::Capacity)?;
        if count == 0
            || count > job.capacity()
            || guides.iter().any(|t| {
                t.format() != wgpu::TextureFormat::Rgba32Float
                    || t.width() != guides[0].width()
                    || t.height() != guides[0].height()
            })
            || offset[0]
                .checked_add(extent[0])
                .is_none_or(|end| end > guides[0].width())
            || offset[1]
                .checked_add(extent[1])
                .is_none_or(|end| end > guides[0].height())
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let input = job.producer_input(device)?;
        let words = [offset[0], offset[1], extent[0], extent[1]];
        let [tile] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "exact world ray pixel region",
                size: 16,
                contents: Some(bytemuck::cast_slice(&words)),
                usage: wgpu::BufferUsages::UNIFORM,
            }])
            .map_err(|_| RaySceneError::Capacity)?;
        let buffer_entry = |binding, storage| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if storage {
                    wgpu::BufferBindingType::Storage { read_only: false }
                } else {
                    wgpu::BufferBindingType::Uniform
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("original archive camera for world rays"),
            entries: &[buffer_entry(0, false)],
        });
        let input_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("original archive guides for world rays"),
            entries: &[
                texture_entry(6),
                texture_entry(7),
                buffer_entry(12, false),
                buffer_entry(17, true),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&input_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("live original archive ray producer"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    crate::archive_scene::archive_shader_source(),
                    WORLD_RAY_PRODUCER_WGSL
                )
                .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GI/reflection guides to exact world rays"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("cs_world_ray_produce"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[(
                    "ARCHIVE_TRACE_SSR",
                    if matches!(kind, WorldRayKind::Reflection) {
                        1.
                    } else {
                        0.
                    },
                )],
                ..Default::default()
            },
            cache: None,
        });
        let groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                }],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &input_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(
                            &guides[0].create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(
                            &guides[1].create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: tile.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 17,
                        resource: input.as_entire_binding(),
                    },
                ],
            }),
        ];
        Ok(Self {
            pipeline,
            groups,
            _camera: camera.clone(),
            _tile: tile,
            _input: input,
            _guides: guides,
            extent,
            count,
        })
    }
    /// Reuse the immutable camera, guides, job and compiled kernel for another
    /// region. Only the small region uniform and its bindings are new.
    pub fn for_region(
        &self,
        device: &wgpu::Device,
        job: &ExactWorldNearest,
        offset: [u32; 2],
        extent: [u32; 2],
    ) -> Result<Self, RaySceneError> {
        let count = extent[0]
            .checked_mul(extent[1])
            .ok_or(RaySceneError::Capacity)?;
        if job.input() != &**self._input
            || count == 0
            || count > job.capacity()
            || offset[0]
                .checked_add(extent[0])
                .is_none_or(|v| v > self._guides[0].width())
            || offset[1]
                .checked_add(extent[1])
                .is_none_or(|v| v > self._guides[0].height())
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        job.producer_input(device)?;
        let [tile] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "shared-kernel world ray pixel region",
                size: 16,
                contents: Some(bytemuck::cast_slice(&[
                    offset[0], offset[1], extent[0], extent[1],
                ])),
                usage: wgpu::BufferUsages::UNIFORM,
            }])
            .map_err(|_| RaySceneError::Capacity)?;
        let a = self._guides[0].create_view(&Default::default());
        let b = self._guides[1].create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared world producer region"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&a),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&b),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: tile.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 17,
                    resource: self._input.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            pipeline: self.pipeline.clone(),
            groups: [self.groups[0].clone(), group],
            _camera: self._camera.clone(),
            _tile: tile,
            _input: self._input.clone(),
            _guides: self._guides.clone(),
            extent,
            count,
        })
    }
    /// Produce and query the same prepared input buffer. Reject a mismatched job
    /// before appending any GPU commands, including when device/scene are shared.
    pub fn trace(
        &self,
        job: &ExactWorldNearest,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RaySceneError> {
        if job.input() != &**self._input {
            return Err(RaySceneError::InvalidGeometry);
        }
        self.encode(encoder);
        job.encode(encoder, self.count)
    }
    pub(crate) fn region_uniform(&self) -> &wgpu::Buffer {
        &self._tile
    }
    pub const fn count(&self) -> u32 {
        self.count
    }
    /// Order after archive guide rasterization, then encode the exact query and consumer.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_timed(encoder, None);
    }
    pub(crate) fn encode_timed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("live archive world ray producer"),
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[0], &[]);
        pass.set_bind_group(1, &self.groups[1], &[]);
        pass.dispatch_workgroups(self.extent[0].div_ceil(8), self.extent[1].div_ceil(8), 1);
    }
}
