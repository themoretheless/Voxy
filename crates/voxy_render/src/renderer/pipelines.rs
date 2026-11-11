//! Pipeline, bind group layout, and resource constructors for [`super::Renderer`].

use std::fmt;
use std::sync::Arc;
use voxy_gpu_core::{BindGroupBuilder, BindGroupLayoutBuilder};

use super::GpuQuad;
use crate::{MaterialLayer, MaterialPack, MaterialSet, SkinnedUploadError};

#[derive(Debug)]
pub(super) struct VoxelLayerPipelines {
    pub opaque: wgpu::RenderPipeline,
    pub cutout: wgpu::RenderPipeline,
    pub translucent: wgpu::RenderPipeline,
}

#[derive(Clone, Debug)]
pub(super) struct ChunkLayerDraws {
    pub origin: [i32; 4],
    pub opaque: std::ops::Range<u32>,
    pub cutout: std::ops::Range<u32>,
    pub translucent: std::ops::Range<u32>,
}

pub(super) fn encode_voxel_layers<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    pipelines: &'a VoxelLayerPipelines,
    camera_group: &'a wgpu::BindGroup,
    material_group: &'a wgpu::BindGroup,
    quad_buffer: &'a wgpu::Buffer,
    chunks: &[ChunkLayerDraws],
    view_projection: glam::Mat4,
    camera_eye: glam::Vec3,
    draw_translucent: bool,
) {
    let mut visible: Vec<&ChunkLayerDraws> = chunks
        .iter()
        .filter(|c| super::camera_math::chunk_intersects_frustum(view_projection, c.origin))
        .collect();
    if visible.is_empty() {
        return;
    }

    if !draw_translucent {
        if visible.iter().any(|c| !c.opaque.is_empty()) {
            pass.set_pipeline(&pipelines.opaque);
            pass.set_bind_group(0, camera_group, &[]);
            pass.set_bind_group(1, material_group, &[]);
            pass.set_vertex_buffer(0, quad_buffer.slice(..));
            for chunk in &visible {
                if !chunk.opaque.is_empty() {
                    pass.draw(0..6, chunk.opaque.clone());
                }
            }
        }
        if visible.iter().any(|c| !c.cutout.is_empty()) {
            pass.set_pipeline(&pipelines.cutout);
            pass.set_bind_group(0, camera_group, &[]);
            pass.set_bind_group(1, material_group, &[]);
            pass.set_vertex_buffer(0, quad_buffer.slice(..));
            for chunk in &visible {
                if !chunk.cutout.is_empty() {
                    pass.draw(0..6, chunk.cutout.clone());
                }
            }
        }
    } else {
        visible.retain(|c| !c.translucent.is_empty());
        if visible.is_empty() {
            return;
        }
        visible.sort_by(|a, b| {
            let center_a = glam::Vec3::new(
                a.origin[0] as f32 + 16.0,
                a.origin[1] as f32 + 16.0,
                a.origin[2] as f32 + 16.0,
            );
            let center_b = glam::Vec3::new(
                b.origin[0] as f32 + 16.0,
                b.origin[1] as f32 + 16.0,
                b.origin[2] as f32 + 16.0,
            );
            center_b
                .distance_squared(camera_eye)
                .total_cmp(&center_a.distance_squared(camera_eye))
        });
        pass.set_pipeline(&pipelines.translucent);
        pass.set_bind_group(0, camera_group, &[]);
        pass.set_bind_group(1, material_group, &[]);
        pass.set_vertex_buffer(0, quad_buffer.slice(..));
        for chunk in visible {
            pass.draw(0..6, chunk.translucent.clone());
        }
    }
}

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
    VoxelLayerPipelines,
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

    let make_voxel = |label, cull_mode, depth_write, blend, fs_entry| {
        let mut ds = depth_stencil.clone();
        if let Some(d) = &mut ds {
            d.depth_write_enabled = Some(depth_write);
        }
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&voxel_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_voxel"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(quad_buffer_layout.clone())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode,
                ..Default::default()
            },
            depth_stencil: ds,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(fs_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    };
    let voxel_pipelines = VoxelLayerPipelines {
        opaque: make_voxel(
            "Voxy photoreal voxel opaque pipeline",
            Some(wgpu::Face::Back),
            true,
            None,
            "fs_voxel_opaque",
        ),
        cutout: make_voxel(
            "Voxy photoreal voxel cutout pipeline",
            None,
            true,
            None,
            "fs_voxel_cutout",
        ),
        translucent: make_voxel(
            "Voxy photoreal voxel translucent pipeline",
            Some(wgpu::Face::Back),
            false,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "fs_main",
        ),
    };

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
        voxel_pipelines,
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
) -> VoxelLayerPipelines {
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
    let make = |label, cull_mode, depth_write, blend, fs_entry| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vertex_buffer.clone())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(fs_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    };
    VoxelLayerPipelines {
        opaque: make(
            "Voxy voxel opaque pipeline",
            Some(wgpu::Face::Back),
            true,
            None,
            "fs_opaque",
        ),
        cutout: make("Voxy voxel cutout pipeline", None, true, None, "fs_cutout"),
        translucent: make(
            "Voxy voxel translucent pipeline",
            Some(wgpu::Face::Back),
            false,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "fs_main",
        ),
    }
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

pub(super) fn write_or_grow_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    capacity: &mut usize,
    data: &[T],
    usage: wgpu::BufferUsages,
    label: &'static str,
) -> bool {
    if data.is_empty() {
        return false;
    }
    if data.len() <= *capacity {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        false
    } else {
        let cap = data.len().next_power_of_two();
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (cap * size_of::<T>()) as wgpu::BufferAddress,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        *capacity = cap;
        true
    }
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

#[derive(Debug)]
pub enum RendererError {
    InvalidPrefilterSamples(u32),
    /// Shader, pipeline, surface configuration or allocation failed before use.
    Initialization(wgpu::Error),
    InvalidSurfaceSize {
        width: u32,
        height: u32,
        max_dimension: u32,
    },
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    UnsupportedSurface,
    FrameIdExhausted,
    /// A native/temporal consumer failed after scene submission, before present.
    TemporalConsumer(String),
    SurfaceLost,
    DeviceLost(String),
    MeshTooLarge,
    TooManyChunks(usize),
    OriginOutOfRange,
    InvalidLightVolume(usize),
    LightDataTooLarge,
    Skinned(SkinnedUploadError),
    SkinnedLod(crate::SkinnedLodGpuError),
    Ray(crate::RaySceneError),
    NoSkinnedMesh,
    NoSkinnedLod,
    InvalidCamera,
    Scene(crate::SceneError),
}

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "renderer error: {self:?}")
    }
}

impl std::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            Self::Skinned(error) => Some(error),
            Self::SkinnedLod(error) => Some(error),
            Self::Ray(error) => Some(error),
            Self::Scene(error) => Some(error),
            _ => None,
        }
    }
}

pub(super) fn encode_motion_layers(
    encoder: &mut wgpu::CommandEncoder,
    motion: Option<&crate::RasterMotionPass>,
    base: Option<&crate::DepthMotionPass>,
) -> Result<(), RendererError> {
    if let (Some(motion), Some(base)) = (motion, base) {
        base.encode(encoder);
        motion
            .encode_over(encoder, base.output())
            .map_err(RendererError::Ray)?;
    }
    Ok(())
}
