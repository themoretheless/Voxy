//! Pipeline, bind group layout, and resource constructors for [`super::Renderer`].

use std::sync::Arc;
use voxy_gpu_core::{BindGroupBuilder, BindGroupLayoutBuilder};

use super::GpuQuad;
use crate::{MaterialLayer, MaterialPack, MaterialSet};

pub(super) fn create_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    create_camera_layout_for_stages(device, "Voxy camera layout", wgpu::ShaderStages::VERTEX)
}

pub(super) fn create_photoreal_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    create_camera_layout_for_stages(
        device,
        "Voxy photoreal camera layout",
        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
    )
}

fn create_camera_layout_for_stages(
    device: &wgpu::Device,
    label: &str,
    stages: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    BindGroupLayoutBuilder::with_capacity(Some(label), 3)
        .uniform_buffer(0, stages)
        .storage_buffer(1, stages, true)
        .storage_buffer(2, stages, true)
        .build(device)
}

pub(super) fn create_photoreal_pipelines(
    device: &wgpu::Device,
    hdr_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    skin_layout: &wgpu::BindGroupLayout,
) -> (
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxy photoreal HDR PBR shader"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("../skin_hair.wgsl"),
                include_str!("../voxel_photoreal.wgsl")
            )
            .into(),
        ),
    });
    let voxel_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy photoreal voxel pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(material_layout)],
        immediate_size: 0,
    });
    let skinned_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy photoreal skinned pipeline layout"),
        bind_group_layouts: &[
            Some(camera_layout),
            Some(material_layout),
            Some(skin_layout),
        ],
        immediate_size: 0,
    });

    let quad_attrs = wgpu::vertex_attr_array![
        0 => Uint8x4,
        1 => Uint8x4,
        2 => Uint16x2,
        3 => Uint8x4,
        4 => Uint16x2
    ];
    let quad_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuQuad>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &quad_attrs,
    };

    let indirect_attrs = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Uint32,
        2 => Uint32,
        3 => Uint32,
        4 => Float32x2
    ];
    let indirect_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: 32,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &indirect_attrs,
    };

    let skin_attrs = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
        3 => Uint16x4,
        4 => Unorm16x4
    ];
    let skin_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: size_of::<crate::SkinnedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &skin_attrs,
    };

    let depth_stencil = Some(wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    let voxel_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy photoreal voxel pipeline"),
        layout: Some(&voxel_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_voxel"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(quad_buffer_layout)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: depth_stencil.clone(),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: hdr_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let indirect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy photoreal indirect voxel pipeline"),
        layout: Some(&voxel_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_indirect"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(indirect_buffer_layout)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: depth_stencil.clone(),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: hdr_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let skinned_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy photoreal skinned pipeline"),
        layout: Some(&skinned_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_skinned"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(skin_buffer_layout.clone())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: depth_stencil.clone(),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: hdr_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let mut fibre_depth = depth_stencil;
    if let Some(depth) = &mut fibre_depth {
        depth.depth_write_enabled = Some(false);
    }
    let fibre_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy antialiased animal fibres"),
        layout: Some(&skinned_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_skinned"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(skin_buffer_layout)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: fibre_depth,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: hdr_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    (
        voxel_pipeline,
        indirect_pipeline,
        skinned_pipeline,
        fibre_pipeline,
    )
}

pub(super) fn create_camera_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    chunk_metadata: &wgpu::Buffer,
    light_data: &wgpu::Buffer,
) -> wgpu::BindGroup {
    BindGroupBuilder::new(Some("Voxy frame data bind group"), layout)
        .buffer(0, camera)
        .buffer(1, chunk_metadata)
        .buffer(2, light_data)
        .build(device)
}

pub(super) fn create_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    BindGroupLayoutBuilder::with_capacity(Some("Voxy material layout"), 2)
        .entry(wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        })
        .sampler(
            1,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::SamplerBindingType::Filtering,
        )
        .build(device)
}

pub(super) fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    materials: &MaterialSet,
) -> wgpu::BindGroup {
    BindGroupBuilder::new(Some("Voxy material bind group"), layout)
        .texture_view(0, &materials.view)
        .sampler(1, &materials.sampler)
        .build(device)
}

pub(super) fn create_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxy voxel shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../voxel.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy voxel pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(material_layout)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![
        0 => Uint8x4,
        1 => Uint8x4,
        2 => Uint16x2,
        3 => Uint8x4,
        4 => Uint16x2
    ];
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuQuad>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy voxel pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_buffer)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) fn create_indirect_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxy voxel indirect shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../voxel_indirect.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy voxel indirect pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(material_layout)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Uint32,
        2 => Uint32,
        3 => Uint32,
        4 => Float32x2
    ];
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: 32,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attributes,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy voxel indirect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_buffer)],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) fn create_depth(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Voxy depth texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

pub(super) fn default_material_pack() -> MaterialPack {
    let colors: [[u8; 4]; 5] = [
        [48, 65, 42, 255],
        [90, 105, 66, 255],
        [132, 145, 87, 255],
        [76, 91, 57, 255],
        [151, 161, 99, 120],
    ];
    let layers = colors
        .into_iter()
        .map(|color| {
            let mut bytes = Vec::with_capacity(16 * 16 * 4);
            for y in 0..16 {
                for x in 0..16 {
                    bytes.extend(color.map(|channel| {
                        if channel == 255 || (x / 4 + y / 4) % 2 == 0 {
                            channel
                        } else {
                            u8::try_from(u16::from(channel) * 82 / 100).unwrap_or(channel)
                        }
                    }));
                }
            }
            MaterialLayer {
                rgba8_srgb: Arc::from(bytes),
            }
        })
        .collect();
    MaterialPack::new(16, 16, layers).expect("built-in material pack is valid")
}
