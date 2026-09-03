use std::fmt;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    /// UNORM16 weights. Non-zero influences must sum to 65535.
    pub weights: [u16; 4],
}

#[derive(Clone, Debug)]
pub struct SkinnedMesh {
    vertices: Vec<SkinnedVertex>,
    indices: Vec<u32>,
    joint_count: u16,
}

impl SkinnedMesh {
    /// Creates a validated indexed triangle mesh.
    ///
    /// # Errors
    ///
    /// Rejects empty/non-triangular geometry, non-finite attributes, invalid indices, zero or
    /// non-normalized weights, and joint indices outside the supplied palette size.
    pub fn new(
        vertices: Vec<SkinnedVertex>,
        indices: Vec<u32>,
        joint_count: u16,
    ) -> Result<Self, SkinnedMeshError> {
        if vertices.is_empty() || indices.is_empty() || !indices.len().is_multiple_of(3) {
            return Err(SkinnedMeshError::InvalidGeometry);
        }
        if joint_count == 0 || usize::from(joint_count) > voxy_animation::MAX_JOINTS {
            return Err(SkinnedMeshError::InvalidJointCount(joint_count));
        }
        for (index, vertex) in vertices.iter().enumerate() {
            if vertex
                .position
                .iter()
                .chain(&vertex.normal)
                .chain(&vertex.uv)
                .any(|value| !value.is_finite())
            {
                return Err(SkinnedMeshError::NonFiniteVertex(index));
            }
            let weight_sum: u32 = vertex.weights.into_iter().map(u32::from).sum();
            if weight_sum != u32::from(u16::MAX) {
                return Err(SkinnedMeshError::InvalidWeights(index));
            }
            if vertex
                .joints
                .into_iter()
                .zip(vertex.weights)
                .any(|(joint, weight)| weight != 0 && joint >= joint_count)
            {
                return Err(SkinnedMeshError::InvalidJoint(index));
            }
        }
        if indices
            .iter()
            .any(|&index| usize::try_from(index).map_or(true, |index| index >= vertices.len()))
        {
            return Err(SkinnedMeshError::InvalidIndex);
        }
        Ok(Self {
            vertices,
            indices,
            joint_count,
        })
    }

    #[must_use]
    pub fn vertices(&self) -> &[SkinnedVertex] {
        &self.vertices
    }

    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    #[must_use]
    pub const fn joint_count(&self) -> u16 {
        self.joint_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkinnedMeshError {
    InvalidGeometry,
    InvalidJointCount(u16),
    NonFiniteVertex(usize),
    InvalidWeights(usize),
    InvalidJoint(usize),
    InvalidIndex,
}

impl fmt::Display for SkinnedMeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "skinned mesh error: {self:?}")
    }
}

impl std::error::Error for SkinnedMeshError {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct ObjectUniform {
    model: [[f32; 4]; 4],
    material: [u32; 4],
}

#[derive(Debug)]
pub(crate) struct GpuSkinnedMesh {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub index_count: u32,
    pub joint_count: usize,
    pub material_layer: u32,
    pub joint_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub object_buffer: wgpu::Buffer,
}

pub(crate) fn object_uniform(model: Mat4, material_layer: u32) -> ObjectUniform {
    ObjectUniform {
        model: model.to_cols_array_2d(),
        material: [material_layer, 0, 0, 0],
    }
}

pub(crate) fn create_skin_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
        source: wgpu::ShaderSource::Wgsl(include_str!("skinned.wgsl").into()),
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

pub(crate) fn upload_skinned(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    mesh: &SkinnedMesh,
    joints: &[Mat4],
    model: Mat4,
    material_layer: u32,
) -> Result<GpuSkinnedMesh, SkinnedUploadError> {
    validate_palette(mesh, joints, model)?;
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
        index_count: u32::try_from(mesh.indices.len())
            .map_err(|_| SkinnedUploadError::TooManyIndices)?,
        joint_count: joints.len(),
        material_layer,
        joint_buffer,
        bind_group,
        object_buffer,
    })
}

pub(crate) fn validate_palette(
    mesh: &SkinnedMesh,
    joints: &[Mat4],
    model: Mat4,
) -> Result<(), SkinnedUploadError> {
    if joints.len() != usize::from(mesh.joint_count()) {
        return Err(SkinnedUploadError::JointCountMismatch);
    }
    if !model.is_finite() || joints.iter().any(|matrix| !matrix.is_finite()) {
        return Err(SkinnedUploadError::NonFiniteMatrix);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkinnedUploadError {
    JointCountMismatch,
    NonFiniteMatrix,
    TooManyIndices,
}

impl fmt::Display for SkinnedUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "skinned upload error: {self:?}")
    }
}

impl std::error::Error for SkinnedUploadError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(weights: [u16; 4], joints: [u16; 4]) -> SkinnedVertex {
        SkinnedVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            joints,
            weights,
        }
    }

    #[test]
    fn mesh_rejects_bad_weights_joints_and_indices() {
        assert_eq!(
            SkinnedMesh::new(vec![vertex([1, 0, 0, 0], [0; 4])], vec![0, 0, 0], 1).unwrap_err(),
            SkinnedMeshError::InvalidWeights(0)
        );
        assert_eq!(
            SkinnedMesh::new(
                vec![vertex([u16::MAX, 0, 0, 0], [1, 0, 0, 0])],
                vec![0, 0, 0],
                1,
            )
            .unwrap_err(),
            SkinnedMeshError::InvalidJoint(0)
        );
        assert_eq!(
            SkinnedMesh::new(vec![vertex([u16::MAX, 0, 0, 0], [0; 4])], vec![0, 0, 1], 1,)
                .unwrap_err(),
            SkinnedMeshError::InvalidIndex
        );
    }
}
