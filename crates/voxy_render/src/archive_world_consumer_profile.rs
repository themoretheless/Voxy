//! Prepared lossless consumer schedules. One workspace belongs to the serial frame.
use crate::{ComputeMemoryBudget, ComputeStorage, RaySceneError};
use std::sync::Arc;
#[path = "archive_consumer_sorted_layout.rs"]
mod sorted_layout;

#[derive(Debug, Clone)]
pub(crate) enum ConsumerWorkspace {
    Profile(Arc<[ComputeStorage; 4]>),
    Sorted(Arc<[ComputeStorage; 3]>),
}
impl ConsumerWorkspace {
    pub(crate) fn is_sorted(&self) -> bool {
        matches!(self, Self::Sorted(_))
    }
    pub(super) fn source(&self) -> &'static str {
        match self {
            Self::Profile(_) => include_str!("archive_world_consumer_profile.wgsl"),
            Self::Sorted(_) => include_str!("archive_world_consumer_sorted.wgsl"),
        }
    }
    fn entry_points(&self) -> &'static [&'static str] {
        if self.is_sorted() {
            &[
                "cs_consumer_sorted_project",
                "cs_consumer_sorted_prefix",
                "cs_consumer_sorted_scatter",
                "cs_consumer_sorted_traverse",
                "cs_consumer_sorted_colour",
            ]
        } else {
            &[
                "cs_consumer_profile_project",
                "cs_consumer_profile_traverse",
                "cs_consumer_profile_colour",
            ]
        }
    }
    fn bindings(&self) -> Vec<(u32, &ComputeStorage)> {
        match self {
            Self::Profile(b) => [23, 24, 25, 26].into_iter().zip(b.iter()).collect(),
            Self::Sorted(b) => [23, 26, 27].into_iter().zip(b.iter()).collect(),
        }
    }
}
#[derive(Debug)]
pub(super) struct ConsumerStages {
    workspace: ConsumerWorkspace,
    pipelines: Arc<Vec<wgpu::ComputePipeline>>,
    groups: [wgpu::BindGroup; 3],
}
impl ConsumerStages {
    pub(super) fn buffers(
        device: &wgpu::Device,
        capacity: u32,
    ) -> Result<ConsumerWorkspace, RaySceneError> {
        let sizes = workspace_sizes(capacity, device.limits().max_storage_buffer_binding_size)?;
        if device.limits().max_storage_buffers_per_shader_stage < 7 {
            return Err(RaySceneError::Unsupported);
        }
        let buffers = ComputeMemoryBudget::for_device(device)
            .allocate_buffers(sizes.map(|size| voxy_gpu_core::ManagedBufferDescriptor {
                label: "diagnostic exact consumer workspace",
                size,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            }))
            .map_err(|_| RaySceneError::Capacity)?;
        Ok(ConsumerWorkspace::Profile(Arc::new(buffers)))
    }
    pub(super) fn sorted_buffers(
        device: &wgpu::Device,
        capacity: u32,
    ) -> Result<ConsumerWorkspace, RaySceneError> {
        let sizes = sorted_layout::sizes(capacity, device.limits().max_storage_buffer_binding_size)
            .ok_or(RaySceneError::Capacity)?;
        if device.limits().max_storage_buffers_per_shader_stage < 6 {
            return Err(RaySceneError::Unsupported);
        }
        let buffers = ComputeMemoryBudget::for_device(device)
            .allocate_buffers(sizes.map(|size| voxy_gpu_core::ManagedBufferDescriptor {
                label: "exact projected-span consumer workspace",
                size,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }))
            .map_err(|_| RaySceneError::Capacity)?;
        Ok(ConsumerWorkspace::Sorted(Arc::new(buffers)))
    }
    pub(super) fn is_sorted(&self) -> bool {
        self.workspace.is_sorted()
    }
    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        workspace: &ConsumerWorkspace,
        guides: [&wgpu::TextureView; 2],
        tile: &wgpu::Buffer,
        input: &wgpu::Buffer,
        audit: &ComputeStorage,
        colours: &ComputeStorage,
    ) -> wgpu::BindGroup {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(guides[0]),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(guides[1]),
            },
            wgpu::BindGroupEntry {
                binding: 12,
                resource: tile.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 17,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 20,
                resource: audit.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 22,
                resource: colours.as_entire_binding(),
            },
        ];
        entries.extend(
            workspace
                .bindings()
                .into_iter()
                .map(|(binding, b)| wgpu::BindGroupEntry {
                    binding,
                    resource: b.as_entire_binding(),
                }),
        );
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("consumer schedule region owner"),
            layout,
            entries: &entries,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        constants: &[(&str, f64)],
        workspace: ConsumerWorkspace,
        camera: &ComputeStorage,
        guides: [&wgpu::TextureView; 2],
        tile: &wgpu::Buffer,
        input: &wgpu::Buffer,
        audit: &ComputeStorage,
        colours: &ComputeStorage,
        textures: [&wgpu::TextureView; 4],
    ) -> Self {
        let buffer = |binding, uniform| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if uniform {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage { read_only: false }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let image = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: if binding == 3 || binding == 4 {
                    wgpu::TextureSampleType::Depth
                } else {
                    wgpu::TextureSampleType::Float { filterable: false }
                },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("consumer schedule camera"),
            entries: &[buffer(0, true)],
        });
        let mut data = vec![
            image(6),
            image(7),
            buffer(12, true),
            buffer(17, false),
            buffer(20, false),
            buffer(22, false),
        ];
        data.extend(
            workspace
                .bindings()
                .iter()
                .map(|&(binding, _)| buffer(binding, false)),
        );
        let data_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("consumer schedule exact records"),
            entries: &data,
        });
        let history_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("consumer schedule immutable history"),
            entries: &[image(0), image(3), image(4), image(7)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("consumer schedule layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&data_layout),
                Some(&history_layout),
            ],
            immediate_size: 0,
        });
        let entries = workspace.entry_points();
        let pipelines = Arc::new(
            entries
                .iter()
                .map(|entry| {
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some(entry),
                        layout: Some(&layout),
                        module: shader,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants,
                            ..Default::default()
                        },
                        cache: None,
                    })
                })
                .collect(),
        );
        let groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                }],
            }),
            Self::group(
                device,
                &data_layout,
                &workspace,
                guides,
                tile,
                input,
                audit,
                colours,
            ),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &history_layout,
                entries: &[0, 3, 4, 7]
                    .into_iter()
                    .zip(textures)
                    .map(|(binding, view)| wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::TextureView(view),
                    })
                    .collect::<Vec<_>>(),
            }),
        ];
        Self {
            workspace,
            pipelines,
            groups,
        }
    }
    pub(super) fn for_region(
        &self,
        device: &wgpu::Device,
        guides: [&wgpu::TextureView; 2],
        tile: &wgpu::Buffer,
        input: &wgpu::Buffer,
        audit: &ComputeStorage,
        colours: &ComputeStorage,
    ) -> Self {
        let group = Self::group(
            device,
            &self.pipelines[0].get_bind_group_layout(1),
            &self.workspace,
            guides,
            tile,
            input,
            audit,
            colours,
        );
        Self {
            workspace: self.workspace.clone(),
            pipelines: self.pipelines.clone(),
            groups: [self.groups[0].clone(), group, self.groups[2].clone()],
        }
    }
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        extent: [u32; 2],
        profile: Option<(&wgpu::QuerySet, u32)>,
    ) {
        if let ConsumerWorkspace::Sorted(b) = &self.workspace {
            encoder.clear_buffer(&b[2], 0, Some(sorted_layout::HEADER_WORDS * 4));
        }
        for (stage, pipeline) in self.pipelines.iter().enumerate() {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(self.workspace.entry_points()[stage]),
                timestamp_writes: profile.map(|(set, base)| wgpu::ComputePassTimestampWrites {
                    query_set: set,
                    beginning_of_pass_write_index: Some(base + stage as u32 * 2),
                    end_of_pass_write_index: Some(base + stage as u32 * 2 + 1),
                }),
            });
            pass.set_pipeline(pipeline);
            for (i, group) in self.groups.iter().enumerate() {
                pass.set_bind_group(i as u32, group, &[]);
            }
            if self.is_sorted() && stage == 1 {
                pass.dispatch_workgroups(1, 1, 1);
            } else if self.is_sorted() && stage == 3 {
                pass.dispatch_workgroups((extent[0] * extent[1]).div_ceil(64), 1, 1);
            } else {
                pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
            }
        }
    }
}
fn workspace_sizes(capacity: u32, binding_limit: u64) -> Result<[u64; 4], RaySceneError> {
    let sizes = [64u64, 64, 48, 32].map(|stride| u64::from(capacity) * stride);
    if capacity == 0 || sizes.iter().any(|&s| s > binding_limit) {
        return Err(RaySceneError::Capacity);
    }
    Ok(sizes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_retains_full_records_within_each_binding() {
        assert_eq!(workspace_sizes(1, 128).unwrap(), [64, 64, 48, 32]);
        let s = workspace_sizes(1_658_880, 128 * 1024 * 1024).unwrap();
        assert_eq!(s.iter().sum::<u64>(), 1_658_880 * 208);
        assert!(workspace_sizes(0, 128).is_err());
        assert!(workspace_sizes(2_097_153, 128 * 1024 * 1024).is_err());
    }
}
