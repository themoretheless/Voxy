use std::{fmt, sync::Arc};

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    /// Authored direction, or zero to request flat shading from posed geometry
    /// in the built-in scene/material shaders.
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    /// UNORM16 weights. Non-zero influences must sum to 65535.
    pub weights: [u16; 4],
}

#[derive(Clone, Debug)]
pub struct SkinnedMesh {
    vertices: Arc<[SkinnedVertex]>,
    indices: Arc<[u32]>,
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
        mut vertices: Vec<SkinnedVertex>,
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
        // The shader reads every index, including zero-weight influences.
        for vertex in &mut vertices {
            for (joint, weight) in vertex.joints.iter_mut().zip(vertex.weights) {
                if weight == 0 {
                    *joint = 0;
                }
            }
        }
        Ok(Self {
            vertices: vertices.into(),
            indices: indices.into(),
            joint_count,
        })
    }

    /// Expand indexed geometry into paired world positions for temporal rasterization.
    /// Palettes must already include inverse-bind transforms, as for rendering.
    /// Retain the previous successfully presented pose; skipped frames must not advance it.
    /// CPU skinning follows the renderer's four UNORM16 influences and model order.
    /// # Errors
    /// Rejects mismatched palettes, nonfinite/nonaffine matrices or overflowing positions.
    pub fn previous_position_vertices(
        &self,
        palettes: [&[Mat4]; 2],
        models: [Mat4; 2],
    ) -> Result<Vec<crate::PreviousPositionVertex>, SkinnedUploadError> {
        for (palette, model) in palettes.into_iter().zip(models) {
            validate_temporal_palette(self, palette, model)?;
        }
        let positions = self
            .vertices
            .iter()
            .map(|vertex| {
                let mut pair = [[0.0; 3]; 2];
                for pose in 0..2 {
                    pair[pose] = skinned_position(vertex, palettes[pose], models[pose])?;
                }
                Ok(crate::PreviousPositionVertex {
                    current: pair[0],
                    previous: pair[1],
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .indices
            .iter()
            .map(|index| positions[*index as usize])
            .collect())
    }

    /// Bake a skeletal pose into indexed world-space scene geometry. Use the
    /// returned mesh with identity model for raster guides and ray-scene updates.
    /// Shares the exact position calculation used by temporal correspondence;
    /// scene shading retains authored normals transformed by the blended inverse transpose.
    /// Palettes already include inverse-bind transforms. CPU baking is explicit
    /// and does not advance pose history or update acceleration structures.
    /// # Errors
    /// Rejects invalid palettes/models, overflowing positions and invalid color.
    pub fn posed_scene_mesh(
        &self,
        joints: &[Mat4],
        model: Mat4,
        color: [f32; 4],
    ) -> Result<crate::SceneMesh, Box<dyn std::error::Error>> {
        validate_temporal_palette(self, joints, model)?;
        let vertices = self
            .vertices
            .iter()
            .map(|vertex| {
                Ok(crate::SceneVertex {
                    position: skinned_position(vertex, joints, model)?,
                    uv: vertex.uv,
                    color,
                })
            })
            .collect::<Result<Vec<_>, SkinnedUploadError>>()?;
        Ok(crate::SceneMesh::new(vertices, self.indices.to_vec())?
            .with_normals(self.posed_normals(joints, model)?)?)
    }

    /// Validate the complete world-space deformation before any legacy GPU writes.
    /// Positions alone can remain finite when a blended normal matrix collapses.
    pub(crate) fn validate_render_pose(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), SkinnedUploadError> {
        validate_temporal_palette(self, joints, model)?;
        for vertex in self.vertices.iter() {
            skinned_position(vertex, joints, model)?;
            skinned_normal(vertex, joints, model)?;
        }
        Ok(())
    }

    /// Scene shading requires an invertible local blended deformation. A
    /// scaled cofactor test avoids overflow and rejects numerically collapsed
    /// normals before publishing GPU writes.
    pub(crate) fn validate_scene_normals(&self, joints: &[Mat4]) -> Result<(), SkinnedUploadError> {
        for vertex in self.vertices.iter() {
            skinned_normal(vertex, joints, Mat4::IDENTITY)?;
        }
        Ok(())
    }

    pub(crate) fn posed_normals(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<Vec<[f32; 3]>, SkinnedUploadError> {
        validate_temporal_palette(self, joints, model)?;
        self.vertices
            .iter()
            .map(|vertex| skinned_normal(vertex, joints, model))
            .collect()
    }

    pub(crate) fn validate_temporal_pose(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), SkinnedUploadError> {
        validate_temporal_palette(self, joints, model)?;
        for vertex in self.vertices.iter() {
            skinned_position(vertex, joints, model)?;
        }
        Ok(())
    }

    pub(crate) fn posed_positions(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<Vec<[f32; 3]>, SkinnedUploadError> {
        validate_temporal_palette(self, joints, model)?;
        self.vertices
            .iter()
            .map(|vertex| skinned_position(vertex, joints, model))
            .collect()
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

    pub(crate) fn with_shared_indices(
        &self,
        indices: Arc<[u32]>,
    ) -> Result<Self, SkinnedMeshError> {
        if indices.is_empty() || !indices.len().is_multiple_of(3) {
            return Err(SkinnedMeshError::InvalidGeometry);
        }
        if indices
            .iter()
            .any(|&index| index as usize >= self.vertices.len())
        {
            return Err(SkinnedMeshError::InvalidIndex);
        }
        Ok(Self {
            vertices: self.vertices.clone(),
            indices,
            joint_count: self.joint_count,
        })
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
        if joints.len() != self.joint_count {
            return Err(SkinnedUploadError::JointCountMismatch);
        }
        mesh.validate_render_pose(joints, model)?;
        queue.write_buffer(&self.joint_buffer, 0, bytemuck::cast_slice(joints));
        queue.write_buffer(
            &self.object_buffer,
            0,
            bytemuck::bytes_of(&object_uniform(model, self.material_layer)),
        );
        Ok(())
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
    mesh.validate_render_pose(joints, model)?;
    let index_count =
        u32::try_from(mesh.indices.len()).map_err(|_| SkinnedUploadError::TooManyIndices)?;
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
    /// Prepared temporal snapshot no longer matches this history generation.
    StaleHistory,
    NonAffineMatrix,
    SingularNormalMatrix,
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
    fn posed_scene_matches_correspondence_and_preserves_indexing() {
        let mut a = vertex([32768, 32767, 0, 0], [0, 1, 0, 0]);
        a.position = [1.0, 2.0, 3.0];
        a.uv = [0.25, 0.75];
        let mut b = a;
        b.position = [-1.0, 0.0, 0.0];
        let mut c = a;
        c.position = [0.0, 1.0, 0.0];
        let mesh = SkinnedMesh::new(vec![a, b, c], vec![2, 0, 1], 2).unwrap();
        let joints = [Mat4::IDENTITY, Mat4::from_rotation_y(0.4)];
        let model = Mat4::from_translation(glam::Vec3::new(3.0, 4.0, 5.0));
        let posed = mesh
            .posed_scene_mesh(&joints, model, [0.8, 0.4, 0.2, 1.0])
            .unwrap();
        let pairs = mesh
            .previous_position_vertices([&joints, &joints], [model, model])
            .unwrap();
        assert_eq!(posed.indices(), mesh.indices());
        for (index, pair) in posed.indices().iter().zip(pairs) {
            let actual = &posed.vertices()[*index as usize];
            assert_eq!(actual.position, pair.current);
            assert_eq!(actual.uv, a.uv);
            assert_eq!(actual.color, [0.8, 0.4, 0.2, 1.0]);
        }
        assert!(mesh.posed_scene_mesh(&[], model, [1.0; 4]).is_err());
        assert!(
            mesh.posed_scene_mesh(&joints, model, [f32::NAN; 4])
                .is_err()
        );
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

    #[test]
    fn unused_joint_indices_are_safe_for_shader_reads() {
        let mesh = SkinnedMesh::new(
            vec![vertex([u16::MAX, 0, 0, 0], [0, 65535, 65535, 65535])],
            vec![0, 0, 0],
            1,
        )
        .unwrap();
        assert_eq!(mesh.vertices()[0].joints, [0; 4]);
    }
}

#[cfg(test)]
mod correspondence_tests {
    use super::*;
    #[test]
    fn weighted_poses_models_indices_and_invalid_palettes() {
        let vertex = SkinnedVertex {
            position: [1.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            joints: [0, 1, 0, 0],
            weights: [32768, 32767, 0, 0],
        };
        let mesh = SkinnedMesh::new(vec![vertex], vec![0; 3], 2).unwrap();
        let current = [Mat4::IDENTITY, Mat4::from_translation(glam::Vec3::Y * 2.0)];
        let previous = [Mat4::from_translation(glam::Vec3::X), Mat4::IDENTITY];
        let models = [
            Mat4::from_translation(glam::Vec3::Z),
            Mat4::from_scale(glam::Vec3::splat(2.0)),
        ];
        let output = mesh
            .previous_position_vertices([&current, &previous], models)
            .unwrap();
        let a = f32::from(32768_u16) / 65535.0;
        let b = f32::from(32767_u16) / 65535.0;
        for vertex in output {
            assert!(
                glam::Vec3::from_array(vertex.current)
                    .abs_diff_eq(glam::Vec3::new(1.0, 2.0 * b, 1.0), 1e-6)
            );
            assert!(
                glam::Vec3::from_array(vertex.previous)
                    .abs_diff_eq(glam::Vec3::new(2.0 * (1.0 + a), 0.0, 0.0), 1e-6)
            );
        }
        assert!(matches!(
            mesh.previous_position_vertices([&current[..1], &previous], models),
            Err(SkinnedUploadError::JointCountMismatch)
        ));
        let bad = [Mat4::from_cols_array(&[f32::NAN; 16]); 2];
        assert!(matches!(
            mesh.previous_position_vertices([&bad, &previous], models),
            Err(SkinnedUploadError::NonFiniteMatrix)
        ));
        let mut projective = current;
        projective[0].x_axis.w = 1.0;
        assert!(matches!(
            mesh.previous_position_vertices([&projective, &previous], models),
            Err(SkinnedUploadError::NonAffineMatrix)
        ));
    }
}

fn validate_temporal_palette(
    mesh: &SkinnedMesh,
    joints: &[Mat4],
    model: Mat4,
) -> Result<(), SkinnedUploadError> {
    validate_palette(mesh, joints, model)?;
    if model.row(3) != glam::Vec4::W || joints.iter().any(|matrix| matrix.row(3) != glam::Vec4::W) {
        return Err(SkinnedUploadError::NonAffineMatrix);
    }
    Ok(())
}
fn skinned_position(
    vertex: &SkinnedVertex,
    joints: &[Mat4],
    model: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    let mut skin = Mat4::ZERO;
    for (joint, weight) in vertex.joints.into_iter().zip(vertex.weights) {
        skin += joints[usize::from(joint)] * (f32::from(weight) / 65535.0);
    }
    let world = model * skin * glam::Vec3::from_array(vertex.position).extend(1.0);
    if !world.is_finite() {
        return Err(SkinnedUploadError::NonFiniteMatrix);
    }
    Ok(world.truncate().to_array())
}

fn skinned_normal(
    vertex: &SkinnedVertex,
    joints: &[Mat4],
    model: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    let mut skin = Mat4::ZERO;
    for (joint, weight) in vertex.joints.into_iter().zip(vertex.weights) {
        skin += joints[usize::from(joint)] * (f32::from(weight) / 65535.0);
    }
    let mut linear = glam::Mat3::from_mat4(model * skin);
    let scale = linear
        .to_cols_array()
        .into_iter()
        .map(f32::abs)
        .fold(0.0, f32::max);
    if !scale.is_finite() || scale <= f32::MIN_POSITIVE || scale >= 1.0 / f32::MIN_POSITIVE {
        return Err(SkinnedUploadError::SingularNormalMatrix);
    }
    linear *= 1.0 / scale;
    let determinant = linear.determinant();
    let a = linear.x_axis.abs();
    let b = linear.y_axis.abs();
    let c = linear.z_axis.abs();
    let terms = a.x * (b.y * c.z + b.z * c.y)
        + a.y * (b.z * c.x + b.x * c.z)
        + a.z * (b.x * c.y + b.y * c.x);
    // Reject an uncertain determinant sign, rather than imposing a fixed
    // minimum scale ratio on otherwise stable anisotropic transforms.
    let uncertainty = (16.0 * f32::EPSILON * terms).max(f32::MIN_POSITIVE);
    if !determinant.is_finite() || determinant.abs() <= uncertainty {
        return Err(SkinnedUploadError::SingularNormalMatrix);
    }
    let original = glam::Vec3::from_array(vertex.normal);
    let magnitude = original.abs().max_element();
    if magnitude == 0.0 {
        return Ok(glam::Vec3::ZERO.to_array());
    }
    if magnitude <= f32::MIN_POSITIVE || magnitude >= 1.0 / f32::MIN_POSITIVE {
        return Err(SkinnedUploadError::SingularNormalMatrix);
    }
    let cofactor = glam::Mat3::from_cols(
        linear.y_axis.cross(linear.z_axis),
        linear.z_axis.cross(linear.x_axis),
        linear.x_axis.cross(linear.y_axis),
    );
    let transformed = cofactor * (original / magnitude) * determinant.signum();
    let max = transformed.abs().max_element();
    if !transformed.is_finite() || max == 0.0 {
        return Err(SkinnedUploadError::SingularNormalMatrix);
    }
    Ok((transformed / max).normalize().to_array())
}

#[cfg(test)]
#[path = "skinned_normal_gpu_tests.rs"]
mod normal_tests;
