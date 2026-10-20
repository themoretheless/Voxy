//! Exact forward-Z bounds with 8x8 or per-pixel leaves and a min/max hierarchy.
use crate::{ComputeError, ComputeMemoryBudget};
use std::sync::Arc;
#[derive(Debug)]
pub struct ArchiveDepthBounds {
    pub texture: Arc<voxy_gpu_core::ManagedTexture>,
    pipeline: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    width: u32,
    height: u32,
    pixel_leaves: bool,
    reduction: Option<wgpu::ComputePipeline>,
    levels: Vec<(wgpu::BindGroup, u32, u32)>,
}
impl ArchiveDepthBounds {
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Self, ComputeError> {
        Self::create(device, depth, width, height, false, false)
    }
    pub fn new_hierarchy(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Self, ComputeError> {
        Self::create(device, depth, width, height, true, false)
    }
    /// Full-resolution exact min/max hierarchy. Uses the same owner and reduction
    /// as tile bounds; every pixel remains represented. Experimental traversal only.
    #[doc(hidden)]
    pub fn new_pixel_hierarchy(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Self, ComputeError> {
        Self::create(device, depth, width, height, true, true)
    }
    fn create(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        width: u32,
        height: u32,
        hierarchy: bool,
        pixel_leaves: bool,
    ) -> Result<Self, ComputeError> {
        if width == 0
            || height == 0
            || depth.width() != width
            || depth.height() != height
            || depth.format() != wgpu::TextureFormat::Depth32Float
            || depth.sample_count() != 1
            || !depth.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let leaf = if pixel_leaves { 1 } else { 8 };
        let (mut width, mut height) = (width.div_ceil(leaf), height.div_ceil(leaf));
        if hierarchy {
            width = width.next_power_of_two();
            height = height.next_power_of_two();
        }
        let mip_level_count = if hierarchy {
            width.max(height).ilog2() + 1
        } else {
            1
        };
        let texture =
            ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                label: Some("exact screen-ray depth bounds"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rg32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("exact screen-ray depth bounds"),
            source: wgpu::ShaderSource::Wgsl(include_str!("archive_depth_bounds.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("exact screen-ray depth bounds"),
            layout: None,
            module: &shader,
            entry_point: Some(if pixel_leaves {
                "pixel_bounds"
            } else {
                "bounds"
            }),
            compilation_options: Default::default(),
            cache: None,
        });
        let mip_view = |mip| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &depth.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mip_view(0)),
                },
            ],
        });
        let mut reduction = None;
        let mut levels = Vec::new();
        if mip_level_count > 1 {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("exact min/max depth mip reduction"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("archive_depth_minmax_reduce.wgsl").into(),
                ),
            });
            let reduce = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("exact min/max depth mip reduction"),
                layout: None,
                module: &shader,
                entry_point: Some("reduce"),
                compilation_options: Default::default(),
                cache: None,
            });
            for mip in 1..mip_level_count {
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("adjacent non-overlapping depth mips"),
                    layout: &reduce.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&mip_view(mip - 1)),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&mip_view(mip)),
                        },
                    ],
                });
                levels.push((bind, (width >> mip).max(1), (height >> mip).max(1)));
            }
            reduction = Some(reduce);
        }
        Ok(Self {
            texture,
            pipeline,
            bind,
            width,
            height,
            pixel_leaves,
            reduction,
            levels,
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("screen-ray full-depth bounds"),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.dispatch_workgroups(
                if self.pixel_leaves {
                    self.width.div_ceil(8)
                } else {
                    self.width
                },
                if self.pixel_leaves {
                    self.height.div_ceil(8)
                } else {
                    self.height
                },
                1,
            );
        }
        if let Some(pipeline) = &self.reduction {
            for (bind, width, height) in &self.levels {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("screen-ray exact min/max mip"),
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind, &[]);
                pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
            }
        }
    }
}
