//! Half-resolution screen-space GI and GTAO compute pass reusing prepass geometric normals.
use crate::{ComputeError, ComputeMemoryBudget, ComputeStorage};
use std::sync::Arc;

pub(super) struct FastGiInputs<'a> {
    pub device: &'a wgpu::Device,
    pub shader: &'a wgpu::ShaderModule,
    pub camera: &'a ComputeStorage,
    pub background: &'a wgpu::Texture,
    pub depth: &'a wgpu::Texture,
    pub history: &'a wgpu::Texture,
    pub bounds: &'a wgpu::Texture,
    pub constants: &'a [(&'static str, f64)],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub(super) struct ArchiveFastGi {
    _prepass_normal: Arc<voxy_gpu_core::ManagedTexture>,
    prepass_normal_view: wgpu::TextureView,
    _half_gi: Arc<voxy_gpu_core::ManagedTexture>,
    half_gi_view: wgpu::TextureView,
    _half_ao: Arc<voxy_gpu_core::ManagedTexture>,
    half_ao_view: wgpu::TextureView,
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 3],
    half_size: (u32, u32),
}

fn allocate_texture(
    device: &wgpu::Device,
    label: &str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, ComputeError> {
    ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn build_bind_groups(
    inputs: &FastGiInputs<'_>,
    pipeline: &wgpu::ComputePipeline,
    normal_view: &wgpu::TextureView,
    gi_view: &wgpu::TextureView,
    ao_view: &wgpu::TextureView,
) -> [wgpu::BindGroup; 3] {
    let device = inputs.device;
    let group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fast GI/AO camera"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: inputs.camera.as_entire_binding(),
        }],
    });
    let group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fast GI/AO targets"),
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 17,
                resource: wgpu::BindingResource::TextureView(normal_view),
            },
            wgpu::BindGroupEntry {
                binding: 18,
                resource: wgpu::BindingResource::TextureView(gi_view),
            },
            wgpu::BindGroupEntry {
                binding: 19,
                resource: wgpu::BindingResource::TextureView(ao_view),
            },
        ],
    });
    let bg_view = inputs.background.create_view(&Default::default());
    let depth_view = inputs.depth.create_view(&Default::default());
    let hist_view = inputs.history.create_view(&Default::default());
    let bounds_view = inputs.bounds.create_view(&Default::default());
    let group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fast GI/AO inputs"),
        layout: &pipeline.get_bind_group_layout(2),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&bg_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&depth_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&hist_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&bounds_view),
            },
        ],
    });
    [group0, group1, group2]
}

impl ArchiveFastGi {
    pub(super) fn new(inputs: &FastGiInputs<'_>) -> Result<Self, ComputeError> {
        let half_size = (inputs.width.div_ceil(2), inputs.height.div_ceil(2));
        let prepass_normal = allocate_texture(
            inputs.device,
            "prepass geometric normal",
            (inputs.width, inputs.height),
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        )?;
        let half_gi = allocate_texture(
            inputs.device,
            "half-res screen GI",
            half_size,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        )?;
        let half_ao = allocate_texture(
            inputs.device,
            "half-res screen GTAO",
            half_size,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        )?;
        let prepass_normal_view = prepass_normal.create_view(&Default::default());
        let half_gi_view = half_gi.create_view(&Default::default());
        let half_ao_view = half_ao.create_view(&Default::default());
        let pipeline = inputs
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("half-res screen GI and GTAO"),
                layout: None,
                module: inputs.shader,
                entry_point: Some("cs_fast_gi_ao"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: inputs.constants,
                    ..Default::default()
                },
                cache: None,
            });
        let groups = build_bind_groups(
            inputs,
            &pipeline,
            &prepass_normal_view,
            &half_gi_view,
            &half_ao_view,
        );
        Ok(Self {
            _prepass_normal: prepass_normal,
            prepass_normal_view,
            _half_gi: half_gi,
            half_gi_view,
            _half_ao: half_ao,
            half_ao_view,
            pipeline,
            groups,
            half_size,
        })
    }

    pub(super) const fn prepass_normal_view(&self) -> &wgpu::TextureView {
        &self.prepass_normal_view
    }

    pub(super) const fn gi_view(&self) -> &wgpu::TextureView {
        &self.half_gi_view
    }

    pub(super) const fn ao_view(&self) -> &wgpu::TextureView {
        &self.half_ao_view
    }

    pub(super) fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("half-res screen GI and GTAO"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        for (i, group) in self.groups.iter().enumerate() {
            pass.set_bind_group(i as u32, group, &[]);
        }
        pass.dispatch_workgroups(
            self.half_size.0.div_ceil(8),
            self.half_size.1.div_ceil(8),
            1,
        );
    }
}
