use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use super::{SkinnedMesh, SkinnedUploadError, SkinnedVertex};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct ObjectUniform {
    model: [[f32; 4]; 4],
    material: [u32; 4],
}

#[derive(Debug)]
#[doc(hidden)]
pub struct GpuSkinnedMesh {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub index_count: u32,
    pub opaque_ranges: Vec<std::ops::Range<u32>>,
    pub fibre_ranges: Vec<std::ops::Range<u32>>,
    pub joint_count: usize,
    pub material_layer: u32,
    pub joint_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub object_buffer: wgpu::Buffer,
}

impl GpuSkinnedMesh {
    /// Admit the whole candidate before writing either GPU buffer. The caller
    /// publishes its CPU pose only after this succeeds.
    pub(crate) fn write_pose(
        &self,
        queue: &wgpu::Queue,
        mesh: &SkinnedMesh,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), SkinnedUploadError> {
        self.write_pose_timed(queue, mesh, joints, model, false)
            .map(|_| ())
    }

    pub(crate) fn write_pose_timed(
        &self,
        queue: &wgpu::Queue,
        mesh: &SkinnedMesh,
        joints: &[Mat4],
        model: Mat4,
        timed: bool,
    ) -> Result<Option<(crate::RenderPoseProfile, std::time::Duration)>, SkinnedUploadError> {
        if joints.len() != self.joint_count {
            return Err(SkinnedUploadError::JointCountMismatch);
        }
        let admission = if timed {
            let (result, profile) = mesh.validate_render_pose_profiled(joints, model);
            result?;
            Some(profile)
        } else {
            mesh.validate_render_pose(joints, model)?;
            None
        };
        let upload = timed.then(std::time::Instant::now);
        queue.write_buffer(&self.joint_buffer, 0, bytemuck::cast_slice(joints));
        queue.write_buffer(
            &self.object_buffer,
            0,
            bytemuck::bytes_of(&object_uniform(model, self.material_layer)),
        );
        Ok(admission.zip(upload.map(|start| start.elapsed())))
    }

    pub fn allocation_bytes(&self) -> u64 {
        self.vertex.size()
            + self.index.size()
            + self.joint_buffer.size()
            + self.object_buffer.size()
    }
}

pub(crate) fn object_uniform(model: Mat4, material_layer: u32) -> ObjectUniform {
    ObjectUniform {
        model: model.to_cols_array_2d(),
        material: [material_layer, 0, 0, 0],
    }
}

pub fn create_skin_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Voxy skin layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub(crate) fn create_skinned_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    skin_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxy skinned shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../skinned.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxy skinned pipeline layout"),
        bind_group_layouts: &[
            Some(camera_layout),
            Some(material_layout),
            Some(skin_layout),
        ],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
        3 => Uint16x4,
        4 => Unorm16x4
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Voxy skinned pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SkinnedVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
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
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[doc(hidden)]
pub fn upload_skinned(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    mesh: &SkinnedMesh,
    joints: &[Mat4],
    model: Mat4,
    material_layer: u32,
) -> Result<GpuSkinnedMesh, SkinnedUploadError> {
    mesh.validate_render_pose(joints, model)?;
    let index_count =
        u32::try_from(mesh.indices.len()).map_err(|_| SkinnedUploadError::TooManyIndices)?;
    let (opaque_ranges, fibre_ranges) = split_fibre_draws(mesh);
    let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Voxy skinned vertices"),
        contents: bytemuck::cast_slice(mesh.vertices()),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Voxy skinned indices"),
        contents: bytemuck::cast_slice(mesh.indices()),
        usage: wgpu::BufferUsages::INDEX,
    });
    let joint_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Voxy joint palette"),
        contents: bytemuck::cast_slice(joints),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let object_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Voxy skinned object"),
        contents: bytemuck::bytes_of(&object_uniform(model, material_layer)),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Voxy skin bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: object_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: joint_buffer.as_entire_binding(),
            },
        ],
    });
    Ok(GpuSkinnedMesh {
        vertex,
        index,
        index_count,
        opaque_ranges,
        fibre_ranges,
        joint_count: joints.len(),
        material_layer,
        joint_buffer,
        bind_group,
        object_buffer,
    })
}

/// Preserve triangle ordering while separating animal fibre materials from skin.
pub(super) fn split_fibre_draws(
    mesh: &SkinnedMesh,
) -> (Vec<std::ops::Range<u32>>, Vec<std::ops::Range<u32>>) {
    let mut opaque: Vec<std::ops::Range<u32>> = Vec::new();
    let mut fibres: Vec<std::ops::Range<u32>> = Vec::new();
    for (i, triangle) in mesh.indices().chunks_exact(3).enumerate() {
        let fibre = triangle
            .iter()
            .all(|&v| mesh.vertices()[v as usize].uv[1] < -13000.0);
        let ranges = if fibre { &mut fibres } else { &mut opaque };
        let start = i as u32 * 3;
        if let Some(last) = ranges.last_mut().filter(|last| last.end == start) {
            last.end += 3;
        } else {
            ranges.push(start..start + 3);
        }
    }
    (opaque, fibres)
}
