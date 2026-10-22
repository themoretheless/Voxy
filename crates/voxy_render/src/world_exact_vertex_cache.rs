//! GPU-prepared lossless source-affine values. Original source prefix is retained.
use crate::{ComputeMemoryBudget, ComputeStorage, RayScene, RaySceneError};
use std::sync::Arc;
#[path = "world_exact_vertex_cache_layout.rs"]
mod layout;
#[derive(Debug)]
pub(super) struct WorldExactVertexCache {
    pub owner: Arc<RayScene>,
    pub relative_layout: super::cached_relative::CacheLayout,
    pub buffer: ComputeStorage,
    pub header: u32,
    source: wgpu::Buffer,
    source_offset: u64,
    control: ComputeStorage,
    _parameters: ComputeStorage,
    pipelines: Vec<wgpu::ComputePipeline>,
    groups: Vec<wgpu::BindGroup>,
    workers: u32,
}
impl WorldExactVertexCache {
    pub fn new(
        device: &wgpu::Device,
        scene: &Arc<RayScene>,
        scratch: &ComputeStorage,
        workers: u32,
    ) -> Result<Self, RaySceneError> {
        let sources = scene
            .archive_candidate_source_bindings()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let wgpu::BindingResource::Buffer(source) = &sources[0] else {
            return Err(RaySceneError::InvalidGeometry);
        };
        let bytes = source.size.map_or(
            source.buffer.size() - source.offset,
            std::num::NonZeroU64::get,
        );
        let source_words = u32::try_from(bytes / 4).map_err(|_| RaySceneError::Capacity)?;
        let counts = scene
            .archive_source_vertex_counts()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let limits = device.limits();
        if workers == 0 || workers > limits.max_compute_workgroups_per_dimension {
            return Err(RaySceneError::Capacity);
        }
        let shape = layout::Layout::new(
            source_words,
            &counts,
            limits
                .max_storage_buffer_binding_size
                .min(limits.max_buffer_size),
        )
        .ok_or(RaySceneError::Capacity)?;
        let slots = u32::try_from(counts.len()).map_err(|_| RaySceneError::Capacity)?;
        let mut initial = vec![0u32; (shape.records - shape.header) as usize];
        initial[1] = slots;
        initial[2] = shape.vertices;
        initial[3] = shape.records;
        initial[4] = 12;
        let revision = scene.geometry_revision();
        initial[8] = revision as u32;
        initial[9] = (revision >> 32) as u32;
        initial[16..16 + shape.prefix.len()].copy_from_slice(&shape.prefix);
        let parameters = [
            shape.header,
            slots,
            shape.vertices,
            shape.records,
            workers,
            0,
            0,
            0,
        ];
        let [buffer, control, parameters] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "original vertices plus exact sparse world-coordinate cache",
                    size: u64::from(shape.words) * 4,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "immutable exact vertex cache control",
                    size: initial.len() as u64 * 4,
                    contents: Some(bytemuck::cast_slice(&initial)),
                    usage: wgpu::BufferUsages::COPY_SRC,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "exact vertex cache preparation parameters",
                    size: 32,
                    contents: Some(bytemuck::cast_slice(&parameters)),
                    usage: wgpu::BufferUsages::UNIFORM,
                },
            ])
            .map_err(|_| RaySceneError::Capacity)?;
        let shader_source = [
            include_str!("world_exact_integer.wgsl"),
            include_str!("world_exact_vertex_cache_prepare.wgsl"),
        ]
        .join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("full precision affine cache prepare and check"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let mut pipelines = Vec::new();
        let mut groups = Vec::new();
        for entry in [
            "cs_vertex_cache_prepare",
            "cs_vertex_cache_check",
            "cs_vertex_cache_ready",
        ] {
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
            let mut entries = vec![
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: parameters.as_entire_binding(),
                },
            ];
            if entry != "cs_vertex_cache_ready" {
                entries.extend([
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: sources[0].clone(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: sources[2].clone(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: scene
                            .archive_candidate_instance_layout()
                            .ok_or(RaySceneError::InvalidGeometry)?,
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: scratch.as_entire_binding(),
                    },
                ]);
            }
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(entry),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            }));
            pipelines.push(pipeline);
        }
        Ok(Self {
            owner: Arc::clone(scene),
            relative_layout: super::cached_relative::CacheLayout {
                header: shape.header,
                records: shape.records,
                vertices: shape.vertices,
                slots,
                words: shape.words,
                revision,
            },
            buffer,
            header: shape.header,
            source: source.buffer.clone(),
            source_offset: source.offset,
            control,
            _parameters: parameters,
            pipelines,
            groups,
            workers,
        })
    }
    pub fn source(&self) -> String {
        let original = include_str!("world_exact.wgsl");
        assert_eq!(original.matches("fn exact_axis(").count(), 1);
        format!(
            "const EX_VERTEX_CACHE_HEADER:u32={}u;\n{}\n{}",
            self.header,
            original.replace("fn exact_axis(", "fn exact_axis_original("),
            include_str!("world_exact_vertex_cache_read.wgsl")
        )
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        // This immutable prefix preserves every source bit. Reset ready/check totals
        // before preparing a replacement, including repeated explicit scene builds.
        encoder.copy_buffer_to_buffer(
            &self.source,
            self.source_offset,
            &self.buffer,
            0,
            u64::from(self.header) * 4,
        );
        encoder.copy_buffer_to_buffer(
            &self.control,
            0,
            &self.buffer,
            u64::from(self.header) * 4,
            self.control.size(),
        );
        for i in 0..3 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("prepare and certify lossless world vertex cache"),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipelines[i]);
            pass.set_bind_group(0, &self.groups[i], &[]);
            pass.dispatch_workgroups(if i == 2 { 1 } else { self.workers }, 1, 1);
        }
    }
}
