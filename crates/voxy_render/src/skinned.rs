use crate::pose_admission_profile::{Clock, RenderPoseProfile, cpu_us};
/// Joint budget of a skinned mesh; `voxy_model` checks it equals the animation rig budget.
pub const MAX_SKIN_JOINTS: usize = 256;
use std::sync::Arc;
mod checkpoint;

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
    coherent_weights: bool,
    /// At least one; bounds every immutable authored position component.
    max_position_component: f32,
    /// Vertices grouped by identical influences; present only for large meshes.
    groups: Option<Arc<InfluenceGroups>>,
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
        if joint_count == 0 || usize::from(joint_count) > MAX_SKIN_JOINTS {
            return Err(SkinnedMeshError::InvalidJointCount(joint_count));
        }
        let mut max_position_component = 1.0f32;
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
            max_position_component = max_position_component
                .max(glam::Vec3::from_array(vertex.position).abs().max_element());
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
        // Select once from immutable influences. Mixed companion surfaces benefit
        // even when only a quarter of neighboring vertices share the same skin.
        let coherent_weights = vertices
            .windows(2)
            .filter(|pair| pair[0].joints == pair[1].joints && pair[0].weights == pair[1].weights)
            .count()
            * 4
            >= vertices.len().saturating_sub(1);
        let groups = InfluenceGroups::build(&vertices).map(Arc::new);
        Ok(Self {
            coherent_weights,
            max_position_component,
            groups,
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
    #[doc(hidden)]
    pub fn validate_render_pose(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> Result<(), SkinnedUploadError> {
        self.validate_render_pose_impl::<false>(joints, model, &mut RenderPoseProfile::default())
    }

    /// Same admission and exact error ordering, with scheduled CPU versus wall clocks.
    #[doc(hidden)]
    pub fn validate_render_pose_profiled(
        &self,
        joints: &[Mat4],
        model: Mat4,
    ) -> (Result<(), SkinnedUploadError>, RenderPoseProfile) {
        let mut profile = RenderPoseProfile::default();
        let caller = cpu_us(false);
        let process = cpu_us(true);
        let start = std::time::Instant::now();
        let result = self.validate_render_pose_impl::<true>(joints, model, &mut profile);
        let caller_end = cpu_us(false);
        let process_end = cpu_us(true);
        profile.total_wall_us = start.elapsed().as_secs_f64() * 1e6;
        profile.cpu_clock_available = caller.is_some()
            && process.is_some()
            && caller_end.is_some()
            && process_end.is_some()
            && [
                profile.spawn_cpu_us,
                profile.main_work_cpu_us,
                profile.worker_cpu_sum_us,
                profile.join_cpu_us,
            ]
            .into_iter()
            .all(f64::is_finite);
        profile.caller_cpu_us = caller.zip(caller_end).map_or(0., |(a, b)| (b - a).max(0.));
        profile.process_cpu_us = process
            .zip(process_end)
            .map_or(0., |(a, b)| (b - a).max(0.));
        (result, profile)
    }

    fn validate_render_pose_impl<const PROFILE: bool>(
        &self,
        joints: &[Mat4],
        model: Mat4,
        profile: &mut RenderPoseProfile,
    ) -> Result<(), SkinnedUploadError> {
        validate_temporal_palette(self, joints, model)?;
        let vertices: &[SkinnedVertex] = &self.vertices;
        let workers = if vertices.len() >= PARALLEL_ADMISSION_MIN_VERTICES {
            std::thread::available_parallelism()
                .map_or(1, std::num::NonZeroUsize::get)
                .min(vertices.len() / PARALLEL_ADMISSION_MIN_CHUNK)
                .max(1)
        } else {
            1
        };
        if PROFILE {
            profile.workers = workers;
            profile.groups = self.groups.as_ref().map_or(0, |groups| groups.keys.len());
        }
        // One interval certificate covers every possible normalized blend within
        // each active joint family. An uncertain family retains the original path.
        if let Some(groups) = &self.groups {
            let clock = PROFILE.then(Clock::start);
            let certified = !groups.families.is_empty()
                && !groups.has_bad_normal
                && finite_temporal_positions(self.max_position_component, joints, model)
                && groups
                    .families
                    .iter()
                    .all(|&family| crate::pose_family_bound::certify(family, joints, model));
            if let Some(clock) = clock {
                let (wall, cpu) = clock.elapsed();
                profile.work(wall, cpu, false);
            }
            if certified {
                if PROFILE {
                    profile.workers = 0;
                }
                return Ok(());
            }
        }
        // Certified groups prove the pose admissible without reading vertices; an
        // uncertified pose takes the exact scan below, which yields the first error.
        if self.groups.as_ref().is_some_and(|groups| {
            groups.certify_profiled::<PROFILE>(joints, model, workers, profile)
        }) {
            return Ok(());
        }
        if PROFILE {
            profile.fallback = true;
        }
        if workers <= 1 {
            return admit_render_vertices(vertices, joints, model, self.coherent_weights);
        }
        // Each contiguous chunk reports its first error; the earliest failing chunk
        // therefore reproduces exactly the error of the sequential vertex-order scan.
        let chunk = vertices.len().div_ceil(workers);
        let coherent = self.coherent_weights;
        std::thread::scope(|scope| {
            let spawn = PROFILE.then(Clock::start);
            let mut chunks = vertices.chunks(chunk);
            let first = chunks.next().unwrap_or(&[]);
            let handles: Vec<_> = chunks
                .map(|part| {
                    let launch = PROFILE.then(std::time::Instant::now);
                    scope.spawn(move || {
                        let delay = launch.map_or(0., |s| s.elapsed().as_secs_f64() * 1e6);
                        let work = PROFILE.then(Clock::start);
                        let result = admit_render_vertices(part, joints, model, coherent);
                        (result, delay, work.map_or((0., 0.), Clock::elapsed))
                    })
                })
                .collect();
            if let Some(clock) = spawn {
                let (wall, cpu) = clock.elapsed();
                profile.spawn_wall_us += wall;
                profile.spawn_cpu_us += cpu;
            }
            let work = PROFILE.then(Clock::start);
            let mut result = admit_render_vertices(first, joints, model, coherent);
            if let Some(clock) = work {
                let (wall, cpu) = clock.elapsed();
                profile.work(wall, cpu, false);
            }
            let join = PROFILE.then(Clock::start);
            for handle in handles {
                let (part, delay, (wall, cpu)) = handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
                if PROFILE {
                    profile.work(wall, cpu, true);
                    profile.launch_delay_max_us = profile.launch_delay_max_us.max(delay);
                }
                if result.is_ok() {
                    result = part;
                }
            }
            if let Some(clock) = join {
                let (wall, cpu) = clock.elapsed();
                profile.join_wall_us += wall;
                profile.join_cpu_us += cpu;
            }
            result
        })
    }

    /// Shared admission for scene CPU/GPU consumers, including flat-shaded
    /// zero normal streams. Performs no writes or pose publication.
    /// # Errors
    /// Rejects invalid palettes, overflowing positions and collapsed normal maps.
    pub fn validate_scene_pose(&self, joints: &[Mat4]) -> Result<(), SkinnedUploadError> {
        self.validate_temporal_pose(joints, Mat4::IDENTITY)?;
        self.validate_scene_normals(joints)
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

    #[doc(hidden)]
    pub fn posed_normals(
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
        // Uncertain/extreme poses retain the original position-only scan.
        if self.vertices.len() >= 1024
            && finite_temporal_positions(self.max_position_component, joints, model)
        {
            return Ok(());
        }
        admit_temporal_vertices(&self.vertices, joints, model, self.coherent_weights)
    }

    #[doc(hidden)]
    pub fn posed_positions(
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
            coherent_weights: self.coherent_weights,
            max_position_component: self.max_position_component,
            groups: self.groups.clone(),
            vertices: self.vertices.clone(),
            indices,
            joint_count: self.joint_count,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("skinned mesh error: {self:?}")]
pub enum SkinnedMeshError {
    InvalidGeometry,
    InvalidJointCount(u16),
    NonFiniteVertex(usize),
    InvalidWeights(usize),
    InvalidJoint(usize),
    InvalidIndex,
}

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
fn split_fibre_draws(mesh: &SkinnedMesh) -> (Vec<std::ops::Range<u32>>, Vec<std::ops::Range<u32>>) {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("skinned upload error: {self:?}")]
pub enum SkinnedUploadError {
    /// Prepared temporal snapshot no longer matches this history generation.
    StaleHistory,
    NonAffineMatrix,
    SingularNormalMatrix,
    JointCountMismatch,
    NonFiniteMatrix,
    TooManyIndices,
}

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
    fn animal_fibre_ranges_preserve_all_opaque_triangles_and_original_order() {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for flags in [
            [false, false, false],
            [true, true, true],
            [true, true, true],
            [false, false, false],
            [true, false, true],
        ] {
            for (k, fibre) in flags.into_iter().enumerate() {
                let mut v = vertex([65535, 0, 0, 0], [0; 4]);
                v.position = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]][k];
                v.uv[1] = if fibre { -13860.125 } else { -10850.0 };
                indices.push(vertices.len() as u32);
                vertices.push(v);
            }
        }
        let mesh = SkinnedMesh::new(vertices, indices, 1).unwrap();
        let (opaque, fibres) = split_fibre_draws(&mesh);
        assert_eq!(opaque, vec![0..3, 9..15]);
        assert_eq!(fibres, vec![3..9]);
        let mut covered = opaque
            .into_iter()
            .chain(fibres)
            .flatten()
            .collect::<Vec<_>>();
        covered.sort_unstable();
        assert_eq!(covered, (0..15).collect::<Vec<_>>());
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

/// Below this size thread startup costs more than the sequential admission scan.
const PARALLEL_ADMISSION_MIN_VERTICES: usize = 131_072;
/// Smallest per-thread share once admission is split.
const PARALLEL_ADMISSION_MIN_CHUNK: usize = 32_768;

/// Scaled-determinant floor above which a group's normal transform provably
/// accepts every authored normal of admissible magnitude. With scaled entries at
/// most 1, the cofactor matrix is `det * L^-T` and `sigma_max(L) <= 3`, so the exact
/// transformed normal (input max component exactly 1) has a max-norm of at least
/// `det / (3 * sqrt 3)`, about 1.9e-4. Rounding of the cross products and the product
/// is below 1.5e-5 (entries bounded by 6), so the result can neither round to zero
/// nor overflow. The determinant passed the existing sign-uncertainty test.
const CERTIFIED_NORMAL_DETERMINANT: f32 = 1.0 / 1024.0;

/// Immutable grouping of vertices by identical (joints, weights). Every vertex in
/// a group shares one blended model-times-skin matrix and normal transform. A pose
/// is admitted by certifying each group from conservative per-group bounds, without
/// reading the vertices. Any uncertified group falls back to the exact per-vertex
/// scan, so results (including the first error) are unchanged.
#[derive(Debug)]
struct InfluenceGroups {
    /// Distinct influence sets in first-occurrence order.
    keys: Vec<([u16; 4], [u16; 4])>,
    families: Vec<[u16; 4]>,
    has_bad_normal: bool,
    /// Largest absolute position component in each group.
    max_position: Vec<f32>,
    /// Whether any nonzero authored normal of the group has an inadmissible magnitude.
    bad_normal: Vec<bool>,
}

impl InfluenceGroups {
    /// Groups large meshes whose influence sets repeat at least twice on average.
    fn build(vertices: &[SkinnedVertex]) -> Option<Self> {
        if vertices.len() < PARALLEL_ADMISSION_MIN_VERTICES {
            return None;
        }
        let mut lookup = std::collections::HashMap::with_capacity(vertices.len() / 4);
        let mut keys = Vec::new();
        let mut max_position: Vec<f32> = Vec::new();
        let mut bad_normal = Vec::new();
        for vertex in vertices {
            let key = (vertex.joints, vertex.weights);
            let group = *lookup.entry(key).or_insert_with(|| {
                keys.push(key);
                max_position.push(0.0);
                bad_normal.push(false);
                keys.len() - 1
            });
            if keys.len() * 2 > vertices.len() {
                return None;
            }
            let position = glam::Vec3::from_array(vertex.position).abs().max_element();
            max_position[group] = max_position[group].max(position);
            let magnitude = glam::Vec3::from_array(vertex.normal).abs().max_element();
            if magnitude != 0.0
                && (magnitude <= f32::MIN_POSITIVE || magnitude >= 1.0 / f32::MIN_POSITIVE)
            {
                bad_normal[group] = true;
            }
        }
        let mut unique = std::collections::HashSet::new();
        let mut families = Vec::new();
        for &(joints, weights) in &keys {
            let mut family = [u16::MAX; 4];
            for (slot, (joint, _)) in joints
                .into_iter()
                .zip(weights)
                .filter(|(_, weight)| *weight != 0)
                .enumerate()
            {
                family[slot] = joint;
            }
            family.sort_unstable();
            // Repeated IDs describe the same convex hull; keep sentinel padding.
            let mut canonical = [u16::MAX; 4];
            let mut count = 0;
            for joint in family {
                if joint != u16::MAX && (count == 0 || canonical[count - 1] != joint) {
                    canonical[count] = joint;
                    count += 1;
                }
            }
            if unique.insert(canonical) {
                families.push(canonical);
                // Bound immutable metadata and per-pose proof work for diverse
                // meshes; they keep the original group/vertex admission path.
                if families.len() > 1024 {
                    families.clear();
                    break;
                }
            }
        }
        if families.len().saturating_mul(32) > keys.len() {
            families.clear();
        }
        let has_bad_normal = bad_normal.iter().any(|&bad| bad);
        Some(Self {
            keys,
            families,
            has_bad_normal,
            max_position,
            bad_normal,
        })
    }

    /// Certifies a group for this pose: no vertex can fail position or normal admission.
    fn certified(&self, group: usize, joints: &[Mat4], model: Mat4) -> bool {
        if self.bad_normal[group] {
            return false;
        }
        let (group_joints, weights) = self.keys[group];
        let mut skin = Mat4::ZERO;
        for (joint, weight) in group_joints.into_iter().zip(weights) {
            skin += joints[usize::from(joint)] * (f32::from(weight) / 65535.0);
        }
        let world = model * skin;
        // `f32::max` ignores NaN, so finiteness is checked explicitly first.
        if !world.is_finite() {
            return false;
        }
        // |(world * p)_i| <= W (3P + 1); a margin of 4 absorbs every rounding step.
        let entries = world
            .to_cols_array()
            .into_iter()
            .map(f32::abs)
            .fold(0.0, f32::max);
        let bound = f64::from(entries) * (3.0 * f64::from(self.max_position[group]) + 1.0);
        if !entries.is_finite() || bound > f64::from(f32::MAX) * 0.25 {
            return false;
        }
        NormalTransform::new(world)
            .is_ok_and(|normal| normal.determinant >= CERTIFIED_NORMAL_DETERMINANT)
    }

    /// `true` when every group is certified, so the whole pose is admissible.
    #[cfg(test)]
    fn certify(&self, joints: &[Mat4], model: Mat4, workers: usize) -> bool {
        self.certify_profiled::<false>(joints, model, workers, &mut RenderPoseProfile::default())
    }

    fn certify_profiled<const PROFILE: bool>(
        &self,
        joints: &[Mat4],
        model: Mat4,
        workers: usize,
        profile: &mut RenderPoseProfile,
    ) -> bool {
        let groups = self.keys.len();
        if workers <= 1 {
            let clock = PROFILE.then(Clock::start);
            let result = (0..groups).all(|group| self.certified(group, joints, model));
            if let Some(clock) = clock {
                let (wall, cpu) = clock.elapsed();
                profile.work(wall, cpu, false);
            }
            return result;
        }
        let chunk = groups.div_ceil(workers).max(1);
        std::thread::scope(|scope| {
            let spawn = PROFILE.then(Clock::start);
            let handles: Vec<_> = (chunk..groups)
                .step_by(chunk)
                .map(|start| {
                    let end = (start + chunk).min(groups);
                    let launch = PROFILE.then(std::time::Instant::now);
                    scope.spawn(move || {
                        let delay = launch.map_or(0., |s| s.elapsed().as_secs_f64() * 1e6);
                        let clock = PROFILE.then(Clock::start);
                        let result = (start..end).all(|g| self.certified(g, joints, model));
                        (result, delay, clock.map_or((0., 0.), Clock::elapsed))
                    })
                })
                .collect();
            if let Some(clock) = spawn {
                let (wall, cpu) = clock.elapsed();
                profile.spawn_wall_us += wall;
                profile.spawn_cpu_us += cpu;
            }
            let clock = PROFILE.then(Clock::start);
            let mut all = (0..chunk.min(groups)).all(|g| self.certified(g, joints, model));
            if let Some(clock) = clock {
                let (wall, cpu) = clock.elapsed();
                profile.work(wall, cpu, false);
            }
            let join = PROFILE.then(Clock::start);
            for handle in handles {
                let (part, delay, (wall, cpu)) = handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
                if PROFILE {
                    profile.work(wall, cpu, true);
                    profile.launch_delay_max_us = profile.launch_delay_max_us.max(delay);
                }
                all &= part;
            }
            if let Some(clock) = join {
                let (wall, cpu) = clock.elapsed();
                profile.join_wall_us += wall;
                profile.join_cpu_us += cpu;
            }
            all
        })
    }
}

/// Sufficient bound only, after complete finite/affine palette admission.
/// Four weighted matrices and two four-term dot products have intermediate
/// magnitude below 2^18 * model_max * joint_max * position_max. The 2^20
/// admission margin also covers f64 bound rounding; see the CPU replay report.
fn finite_temporal_positions(position_max: f32, joints: &[Mat4], model: Mat4) -> bool {
    let model_max = model
        .to_cols_array()
        .into_iter()
        .map(f32::abs)
        .fold(1.0, f32::max);
    let joint_max = joints
        .iter()
        .flat_map(|matrix| matrix.to_cols_array())
        .map(f32::abs)
        .fold(1.0, f32::max);
    let bound = f64::from(model_max) * f64::from(joint_max) * f64::from(position_max);
    bound <= f64::from(f32::MAX) / 1_048_576.0
}

/// Position-only admission keeps temporal semantics: a singular normal transform
/// is permitted here. Reuse only bit-identical influence sets within this call.
fn admit_temporal_vertices(
    vertices: &[SkinnedVertex],
    joints: &[Mat4],
    model: Mat4,
    coherent: bool,
) -> Result<(), SkinnedUploadError> {
    if !coherent {
        for vertex in vertices {
            skinned_position(vertex, joints, model)?;
        }
        return Ok(());
    }
    let mut cached: Option<([u16; 4], [u16; 4], Mat4)> = None;
    for vertex in vertices {
        let world = if let Some((_, _, world)) = cached
            .as_ref()
            .filter(|(indices, weights, _)| *indices == vertex.joints && *weights == vertex.weights)
        {
            *world
        } else {
            let world = model * blended_skin(vertex, joints);
            cached = Some((vertex.joints, vertex.weights, world));
            world
        };
        skinned_position_in(vertex, world)?;
    }
    Ok(())
}

/// Sequential render-pose admission of a contiguous vertex range, returning the
/// first error in vertex order. The coherent path reuses the model-times-skin
/// matrix and normal transform across consecutive identical influences.
fn admit_render_vertices(
    vertices: &[SkinnedVertex],
    joints: &[Mat4],
    model: Mat4,
    coherent: bool,
) -> Result<(), SkinnedUploadError> {
    if coherent {
        let mut cached: Option<([u16; 4], [u16; 4], Mat4, NormalTransform)> = None;
        for vertex in vertices {
            let hit = cached.as_ref().filter(|(joints, weights, _, _)| {
                *joints == vertex.joints && *weights == vertex.weights
            });
            let (world, normal) = if let Some((_, _, world, normal)) = hit {
                (*world, *normal)
            } else {
                let world = model * blended_skin(vertex, joints);
                // Preserve position-before-normal error ordering.
                skinned_position_in(vertex, world)?;
                let normal = NormalTransform::new(world)?;
                cached = Some((vertex.joints, vertex.weights, world, normal));
                normal.apply(vertex)?;
                continue;
            };
            skinned_position_in(vertex, world)?;
            normal.apply(vertex)?;
        }
        return Ok(());
    }
    for vertex in vertices {
        // The blended palette is identical for the position and normal tests, and
        // so is its product with the model matrix: both tests read the same product.
        let world = model * blended_skin(vertex, joints);
        skinned_position_in(vertex, world)?;
        skinned_normal_in(vertex, world)?;
    }
    Ok(())
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
fn blended_skin(vertex: &SkinnedVertex, joints: &[Mat4]) -> Mat4 {
    let mut skin = Mat4::ZERO;
    for (joint, weight) in vertex.joints.into_iter().zip(vertex.weights) {
        skin += joints[usize::from(joint)] * (f32::from(weight) / 65535.0);
    }
    skin
}

fn skinned_position(
    vertex: &SkinnedVertex,
    joints: &[Mat4],
    model: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    skinned_position_with(vertex, blended_skin(vertex, joints), model)
}

fn skinned_position_with(
    vertex: &SkinnedVertex,
    skin: Mat4,
    model: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    skinned_position_in(vertex, model * skin)
}

/// Position under an already multiplied model-times-skin matrix.
fn skinned_position_in(
    vertex: &SkinnedVertex,
    world: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    let world = world * glam::Vec3::from_array(vertex.position).extend(1.0);
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
    skinned_normal_with(vertex, blended_skin(vertex, joints), model)
}

fn skinned_normal_with(
    vertex: &SkinnedVertex,
    skin: Mat4,
    model: Mat4,
) -> Result<[f32; 3], SkinnedUploadError> {
    skinned_normal_in(vertex, model * skin)
}

/// Normal under an already multiplied model-times-skin matrix.
fn skinned_normal_in(vertex: &SkinnedVertex, world: Mat4) -> Result<[f32; 3], SkinnedUploadError> {
    let mut linear = glam::Mat3::from_mat4(world);
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
#[derive(Clone, Copy)]
struct NormalTransform {
    cofactor: glam::Mat3,
    sign: f32,
    /// Absolute determinant of the scaled linear part (entries at most 1).
    determinant: f32,
}
impl NormalTransform {
    fn new(world: Mat4) -> Result<Self, SkinnedUploadError> {
        let mut linear = glam::Mat3::from_mat4(world);
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
        let cofactor = glam::Mat3::from_cols(
            linear.y_axis.cross(linear.z_axis),
            linear.z_axis.cross(linear.x_axis),
            linear.x_axis.cross(linear.y_axis),
        );
        Ok(Self {
            cofactor,
            sign: determinant.signum(),
            determinant: determinant.abs(),
        })
    }
    fn apply(self, vertex: &SkinnedVertex) -> Result<[f32; 3], SkinnedUploadError> {
        let original = glam::Vec3::from_array(vertex.normal);
        let magnitude = original.abs().max_element();
        if magnitude == 0.0 {
            return Ok(glam::Vec3::ZERO.to_array());
        }
        if magnitude <= f32::MIN_POSITIVE || magnitude >= 1.0 / f32::MIN_POSITIVE {
            return Err(SkinnedUploadError::SingularNormalMatrix);
        }

        let transformed = self.cofactor * (original / magnitude) * self.sign;
        let max = transformed.abs().max_element();
        if !transformed.is_finite() || max == 0.0 {
            return Err(SkinnedUploadError::SingularNormalMatrix);
        }
        Ok((transformed / max).normalize().to_array())
    }
}

#[cfg(test)]
mod coherent_pose_tests {
    use super::*;
    /// Same-process pairs avoid comparing identical controls on different CPU cores.
    #[test]
    #[ignore = "manual paired CPU kernel benchmark"]
    fn paired_cache_policy_cpu_bench() {
        use std::{hint::black_box, time::Instant};
        for (name, run) in [
            ("coherent", 64usize),
            ("alternating", 1),
            ("mixed_quarter", 0),
        ] {
            let vertices: Vec<_> = (0usize..100_000)
                .map(|i| {
                    let joint = (i
                        .checked_div(run)
                        .unwrap_or_else(|| i / 4 * 3 + (i % 4).saturating_sub(1))
                        % 16) as u16;
                    SkinnedVertex {
                        position: [(i % 71) as f32 * 0.01, 0.2, 0.3],
                        normal: [0.2, 0.8, 0.1],
                        uv: [0.; 2],
                        joints: [joint, (joint + 1) % 16, 0, 0],
                        weights: [45000, 20535, 0, 0],
                    }
                })
                .collect();
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 16).unwrap();
            let matches = mesh
                .vertices
                .windows(2)
                .filter(|p| p[0].joints == p[1].joints && p[0].weights == p[1].weights)
                .count();
            let old_policy = matches * 2 >= mesh.vertices.len() - 1;
            let joints: Vec<_> = (0..16)
                .map(|i| {
                    Mat4::from_rotation_translation(
                        glam::Quat::from_rotation_y(i as f32 * 0.01),
                        Vec3::Y * 0.1,
                    )
                })
                .collect();
            for round in 1..=5 {
                for (variant, policy) in [("before", old_policy), ("after", mesh.coherent_weights)]
                {
                    for _ in 0..3 {
                        admit_render_vertices(&mesh.vertices, &joints, Mat4::IDENTITY, policy)
                            .unwrap();
                    }
                    let mut samples = Vec::new();
                    for _ in 0..31 {
                        let start = Instant::now();
                        black_box(admit_render_vertices(
                            black_box(&mesh.vertices),
                            black_box(&joints),
                            black_box(Mat4::IDENTITY),
                            black_box(policy),
                        ))
                        .unwrap();
                        samples.push(start.elapsed().as_secs_f64() * 1e6);
                    }
                    samples.sort_by(f64::total_cmp);
                    println!("KERNEL_PAIR,{round},{variant},{name},{:.3}", samples[15]);
                }
            }
        }
    }

    #[test]
    fn cached_admission_matches_original_loop_and_normal_bits() {
        for run in [1usize, 8, 0] {
            let vertices: Vec<_> = (0usize..96)
                .map(|i| SkinnedVertex {
                    position: [i as f32 * 0.03, 0.2, -0.1],
                    normal: if i % 7 == 0 {
                        [0.; 3]
                    } else {
                        [0.1, 0.9, -0.3]
                    },
                    uv: [0.; 2],
                    joints: [
                        (i.checked_div(run)
                            .unwrap_or_else(|| i / 4 + (i % 4).saturating_sub(1))
                            % 2) as u16,
                        0,
                        0,
                        0,
                    ],
                    weights: [65535, 0, 0, 0],
                })
                .collect();
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 2).unwrap();
            assert_eq!(mesh.coherent_weights, run != 1);
            for scale in [
                Vec3::ONE,
                Vec3::new(-1., 0.01, 10.),
                Vec3::ZERO,
                Vec3::splat(f32::MAX),
            ] {
                for angle in [0.0, 0.3, 1.7] {
                    let joints = [Mat4::from_rotation_y(angle), Mat4::from_scale(scale)];
                    let model = Mat4::from_translation(Vec3::new(1., -2., 3.));
                    let reference = (|| {
                        validate_temporal_palette(&mesh, &joints, model)?;
                        for v in mesh.vertices.iter() {
                            let world = model * blended_skin(v, &joints);
                            skinned_position_in(v, world)?;
                            skinned_normal_in(v, world)?;
                        }
                        Ok::<_, SkinnedUploadError>(())
                    })();
                    assert_eq!(mesh.validate_render_pose(&joints, model), reference);
                    for v in mesh.vertices.iter() {
                        let world = model * blended_skin(v, &joints);
                        let original = skinned_normal_in(v, world).map(|n| n.map(f32::to_bits));
                        let prepared = NormalTransform::new(world)
                            .and_then(|n| n.apply(v))
                            .map(|n| n.map(f32::to_bits));
                        assert_eq!(prepared, original);
                    }
                }
            }
            let invalid = [Mat4::from_cols_array(&[f32::NAN; 16]); 2];
            assert_eq!(
                mesh.validate_render_pose(&invalid, Mat4::IDENTITY),
                Err(SkinnedUploadError::NonFiniteMatrix)
            );
            let shared = mesh.with_shared_indices(Arc::from([2, 1, 0])).unwrap();
            assert_eq!(shared.coherent_weights, mesh.coherent_weights);
        }
    }
    #[test]
    fn parallel_admission_matches_sequential_first_error() {
        let count = PARALLEL_ADMISSION_MIN_VERTICES * 2 + 17;
        for run in [1usize, 64] {
            let base: Vec<_> = (0..count)
                .map(|i| SkinnedVertex {
                    position: [(i % 97) as f32 * 0.01, 0.2, -0.1],
                    normal: [0.1, 0.9, -0.3],
                    uv: [0.; 2],
                    joints: [((i / run) % 3) as u16, 0, 0, 0],
                    weights: [65535, 0, 0, 0],
                })
                .collect();
            // Joint 2 collapses normals; place it late and a huge position early, so
            // a later chunk fails differently than the earliest failing vertex.
            let mut vertices = base.clone();
            for v in &mut vertices {
                v.joints[0] = v.joints[0].min(1);
            }
            let late = count - 5;
            vertices[late].joints = [2, 0, 0, 0];
            let early = count / 3;
            vertices[early].position = [f32::MAX, 0.0, 0.0];
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 3).unwrap();
            let joints = [
                Mat4::from_rotation_y(0.3),
                Mat4::from_translation(Vec3::Y),
                Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0)),
            ];
            let model = Mat4::from_scale(Vec3::splat(4.0));
            let sequential =
                admit_render_vertices(mesh.vertices(), &joints, model, mesh.coherent_weights);
            assert_eq!(sequential, Err(SkinnedUploadError::NonFiniteMatrix));
            assert_eq!(mesh.validate_render_pose(&joints, model), sequential);
            let valid = SkinnedMesh::new(
                base.iter()
                    .map(|v| SkinnedVertex {
                        joints: [v.joints[0].min(1), 0, 0, 0],
                        ..*v
                    })
                    .collect(),
                vec![0, 1, 2],
                3,
            )
            .unwrap();
            assert_eq!(valid.validate_render_pose(&joints, model), Ok(()));
            let mut only_late = base.clone();
            for v in &mut only_late {
                v.joints[0] = v.joints[0].min(1);
            }
            only_late[late].joints = [2, 0, 0, 0];
            let only_late = SkinnedMesh::new(only_late, vec![0, 1, 2], 3).unwrap();
            assert_eq!(
                only_late.validate_render_pose(&joints, model),
                Err(SkinnedUploadError::SingularNormalMatrix)
            );
        }
    }
    #[test]
    fn grouped_admission_matches_per_vertex_scan() {
        let count = PARALLEL_ADMISSION_MIN_VERTICES + 1001;
        // Scattered (non-adjacent) repeats of 5 influence sets, as on authored fur.
        let vertices: Vec<_> = (0..count)
            .map(|i| {
                let set = (i * 7919) % 5;
                SkinnedVertex {
                    position: [(i % 113) as f32 * 0.01, 0.2, -0.1],
                    normal: if i % 11 == 0 {
                        [0.; 3]
                    } else {
                        [0.1, 0.9, -0.3]
                    },
                    uv: [0.; 2],
                    joints: [set as u16, ((set + 1) % 4) as u16, 0, 0],
                    weights: [40000, 25535, 0, 0],
                }
            })
            .collect();
        let mesh = SkinnedMesh::new(vertices.clone(), vec![0, 1, 2], 5).unwrap();
        assert!(mesh.groups.is_some());
        assert!(!mesh.coherent_weights);
        let mut late_huge = vertices.clone();
        late_huge[count - 3].position = [f32::MAX, f32::MAX, 0.0];
        let late_huge = SkinnedMesh::new(late_huge, vec![0, 1, 2], 5).unwrap();
        let mut outcomes = std::collections::BTreeSet::new();
        for scale in [
            Vec3::ONE,
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(-1., 0.01, 10.),
            // Valid, but below the certified determinant: exercises the exact fallback.
            Vec3::new(1.0, 1e-4, 1.0),
            Vec3::splat(f32::MAX),
        ] {
            for angle in [0.0, 1.1] {
                let joints = [
                    Mat4::from_rotation_y(angle),
                    Mat4::from_scale(scale),
                    Mat4::from_translation(Vec3::X),
                    Mat4::from_rotation_x(angle),
                    Mat4::from_rotation_z(angle),
                ];
                for model in [Mat4::IDENTITY, Mat4::from_scale(Vec3::splat(3.0))] {
                    for mesh in [&mesh, &late_huge] {
                        let reference = mesh.vertices().iter().try_for_each(|v| {
                            let world = model * blended_skin(v, &joints);
                            skinned_position_in(v, world)?;
                            skinned_normal_in(v, world).map(|_| ())
                        });
                        let groups = mesh.groups.as_ref().unwrap();
                        for workers in [1, 3, 8] {
                            let certified = groups.certify(&joints, model, workers);
                            // Certification is sound: it never admits a failing pose.
                            assert!(!certified || reference.is_ok());
                            outcomes.insert((certified, reference.is_ok()));
                        }
                        assert_eq!(mesh.validate_render_pose(&joints, model), reference);
                    }
                }
            }
        }
        // Exercised: certified valid, uncertified valid (fallback), uncertified invalid.
        assert_eq!(
            outcomes.into_iter().collect::<Vec<_>>(),
            [(false, false), (false, true), (true, true)]
        );
        let shared = mesh.with_shared_indices(Arc::from([2, 1, 0])).unwrap();
        assert!(shared.groups.is_some());
        // Unique influences per vertex disable grouping.
        let unique: Vec<_> = (0..count)
            .map(|i| SkinnedVertex {
                weights: [(i % 60000) as u16 + 1, 65534 - (i % 60000) as u16, 0, 0],
                joints: [0, (i / 60000) as u16 + 1, 0, 0],
                ..vertices[0]
            })
            .collect();
        assert!(
            SkinnedMesh::new(unique, vec![0, 1, 2], 5)
                .unwrap()
                .groups
                .is_none()
        );
    }
    use glam::Vec3;
}

#[cfg(test)]
mod temporal_admission_tests {
    use super::*;
    fn mesh(run: usize, count: usize) -> SkinnedMesh {
        SkinnedMesh::new(
            (0..count)
                .map(|i| {
                    let joint = (i
                        .checked_div(run)
                        .unwrap_or_else(|| i / 4 * 3 + (i % 4).saturating_sub(1))
                        % 16) as u16;
                    SkinnedVertex {
                        position: [(i % 71) as f32 * 0.03, 0.2, 0.3],
                        normal: [0.2, 0.8, 0.1],
                        uv: [0.; 2],
                        joints: [joint, 0, 0, 0],
                        weights: [65535, 0, 0, 0],
                    }
                })
                .collect(),
            vec![0, 1, 2],
            16,
        )
        .unwrap()
    }
    #[test]
    fn temporal_cache_matches_original_positions_and_errors() {
        for run in [0, 1, 64] {
            let mesh = mesh(run, 2048);
            for scale in [1., 0., -1., f32::MAX] {
                let model = Mat4::from_scale(glam::Vec3::splat(scale));
                let joints = std::array::from_fn::<_, 16, _>(|i| {
                    Mat4::from_translation(glam::Vec3::X * i as f32 * 0.01)
                });
                assert_eq!(
                    admit_temporal_vertices(mesh.vertices(), &joints, model, false),
                    mesh.validate_temporal_pose(&joints, model)
                );
                let mut previous = None;
                for vertex in mesh.vertices() {
                    let world = if let Some((indices, weights, world)) = previous {
                        if vertex.joints == indices && vertex.weights == weights {
                            world
                        } else {
                            model * blended_skin(vertex, &joints)
                        }
                    } else {
                        model * blended_skin(vertex, &joints)
                    };
                    previous = Some((vertex.joints, vertex.weights, world));
                    assert_eq!(
                        skinned_position(vertex, &joints, model),
                        skinned_position_in(vertex, world)
                    );
                }
            }
            for joints in [vec![], vec![Mat4::from_cols_array(&[f32::NAN; 16]); 16]] {
                let expected =
                    validate_temporal_palette(&mesh, &joints, Mat4::IDENTITY).and_then(|()| {
                        admit_temporal_vertices(mesh.vertices(), &joints, Mat4::IDENTITY, false)
                    });
                assert_eq!(
                    mesh.validate_temporal_pose(&joints, Mat4::IDENTITY),
                    expected
                );
            }
        }
    }
    #[test]
    fn blended_weight_changes_and_cancellation_keep_temporal_contract() {
        let vertices = (0..48)
            .map(|i| {
                let weight = ((i / 3) % 2) * 32767;
                SkinnedVertex {
                    position: [2., -3., 4.],
                    normal: [0.; 3],
                    uv: [0.; 2],
                    joints: [0, 1, 0, 0],
                    weights: [weight, 65535 - weight, 0, 0],
                }
            })
            .collect();
        let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 2).unwrap();
        assert!(mesh.coherent_weights);
        for tick in 0..24 {
            let joints = [
                Mat4::from_rotation_y(tick as f32 * 0.1),
                Mat4::from_scale(glam::Vec3::splat(-1.)),
            ];
            for model in [
                Mat4::IDENTITY,
                Mat4::from_scale(glam::Vec3::splat(f32::MAX)),
            ] {
                assert_eq!(
                    mesh.validate_temporal_pose(&joints, model),
                    admit_temporal_vertices(mesh.vertices(), &joints, model, false)
                );
            }
        }
        // Temporal admission checks positions only, including a collapsed blend.
        assert!(
            mesh.validate_temporal_pose(&[Mat4::from_scale(glam::Vec3::ZERO); 2], Mat4::IDENTITY)
                .is_ok()
        );
    }
    #[test]
    #[ignore = "manual CPU changing-pose paired benchmark"]
    fn paired_temporal_cpu_bench() {
        use std::{hint::black_box, time::Instant};
        for (name, run) in [("coherent", 64), ("mixed_quarter", 0), ("alternating", 1)] {
            let mesh = mesh(run, 100_000);
            for round in 1..=5 {
                for (variant, coherent) in if round % 2 == 1 {
                    [("before", false), ("after", mesh.coherent_weights)]
                } else {
                    [("after", mesh.coherent_weights), ("before", false)]
                } {
                    let mut samples = Vec::new();
                    for sample in 0..54 {
                        let joints = std::array::from_fn::<_, 16, _>(|i| {
                            Mat4::from_rotation_translation(
                                glam::Quat::from_rotation_y(
                                    sample as f32 * 0.003 + i as f32 * 0.01,
                                ),
                                glam::Vec3::X * i as f32 * 0.01,
                            )
                        });
                        let model = Mat4::from_translation(glam::Vec3::Z * sample as f32 * 0.01);
                        let start = Instant::now();
                        validate_temporal_palette(&mesh, black_box(&joints), black_box(model))
                            .unwrap();
                        admit_temporal_vertices(
                            black_box(mesh.vertices()),
                            &joints,
                            model,
                            black_box(coherent),
                        )
                        .unwrap();
                        if sample >= 3 {
                            samples.push(start.elapsed().as_secs_f64() * 1e6);
                        }
                    }
                    samples.sort_by(f64::total_cmp);
                    println!("TEMPORAL_PAIR,{round},{variant},{name},{:.6}", samples[25]);
                }
            }
        }
    }
}

#[cfg(test)]
mod temporal_bound_regressions {
    use super::*;
    #[test]
    fn conservative_temporal_bounds_preserve_position_only_oracle_and_reset_inputs() {
        let count = PARALLEL_ADMISSION_MIN_VERTICES + 17;
        let mut outcomes = std::collections::BTreeSet::new();
        for normal in [[0., 1., 0.], [0.; 3], [f32::MAX, 0., 0.]] {
            let vertices = (0..count)
                .map(|i| SkinnedVertex {
                    position: [(i % 71) as f32 * 0.03, -0.2, 0.3],
                    normal,
                    uv: [0.; 2],
                    joints: [(i % 2) as u16, 0, 0, 0],
                    weights: [65535, 0, 0, 0],
                })
                .collect();
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 2).unwrap();
            let shared = mesh.with_shared_indices(Arc::from([2, 1, 0])).unwrap();
            for joints in [
                [Mat4::IDENTITY, Mat4::from_rotation_y(0.3)],
                [Mat4::from_scale(glam::Vec3::ZERO); 2],
                [Mat4::from_scale(glam::Vec3::new(1., 0., 1.)); 2],
            ] {
                for model in [
                    Mat4::IDENTITY,
                    Mat4::from_scale(glam::Vec3::splat(-1.)),
                    Mat4::from_scale(glam::Vec3::splat(f32::MAX)),
                ] {
                    let reference = admit_temporal_vertices(
                        mesh.vertices(),
                        &joints,
                        model,
                        mesh.coherent_weights,
                    );
                    let certified =
                        finite_temporal_positions(mesh.max_position_component, &joints, model);
                    assert!(!certified || reference.is_ok());
                    outcomes.insert((certified, reference.is_ok()));
                    assert_eq!(mesh.validate_temporal_pose(&joints, model), reference);
                    assert_eq!(shared.validate_temporal_pose(&joints, model), reference);
                }
            }
            let large_translation = Mat4::from_translation(glam::Vec3::splat(f32::MAX / 8.));
            assert!(!finite_temporal_positions(
                mesh.max_position_component,
                &[Mat4::IDENTITY; 2],
                large_translation
            ));
            let reference = admit_temporal_vertices(
                mesh.vertices(),
                &[Mat4::IDENTITY; 2],
                large_translation,
                false,
            );
            assert!(reference.is_ok());
            assert_eq!(
                mesh.validate_temporal_pose(&[Mat4::IDENTITY; 2], large_translation),
                reference
            );
            outcomes.insert((false, true));
            for palette in [
                vec![],
                vec![Mat4::IDENTITY],
                vec![Mat4::from_cols_array(&[f32::NAN; 16]); 2],
            ] {
                let expected = validate_temporal_palette(&mesh, &palette, Mat4::IDENTITY);
                assert!(expected.is_err());
                assert_eq!(
                    mesh.validate_temporal_pose(&palette, Mat4::IDENTITY),
                    expected
                );
            }
            // Replacing the model/palette on each call must not retain a certificate.
            assert!(
                mesh.validate_temporal_pose(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
                    .is_ok()
            );
            assert!(
                mesh.validate_temporal_pose(
                    &[Mat4::IDENTITY; 2],
                    Mat4::from_scale(glam::Vec3::splat(f32::MAX))
                )
                .is_err()
            );
            assert!(
                mesh.validate_temporal_pose(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
                    .is_ok()
            );
        }
        assert_eq!(
            outcomes.into_iter().collect::<Vec<_>>(),
            vec![(false, false), (false, true), (true, true)]
        );
    }
}

#[cfg(test)]
mod temporal_bound_exponent_tests {
    use super::*;
    #[test]
    fn bounds_match_exact_scan_across_scale_exponents_and_blends() {
        let mut certified = 0;
        let mut uncertain = 0;
        for position_exponent in [-120, -1, 0, 80, 120] {
            let p = 2.0f32.powi(position_exponent);
            let vertices = (0..1025)
                .map(|i| SkinnedVertex {
                    position: [p, -p, p * 0.5],
                    normal: [0., 1., 0.],
                    uv: [0.; 2],
                    joints: [0, 1, 2, 0],
                    weights: [
                        [65535, 0, 0, 0],
                        [32768, 32767, 0, 0],
                        [1, 65534, 0, 0],
                        [21845, 21845, 21845, 0],
                    ][i % 4],
                })
                .collect();
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 3).unwrap();
            for joint_exponent in [-120, -1, 0, 80, 120] {
                let j = 2.0f32.powi(joint_exponent);
                let joints = [
                    Mat4::from_scale(glam::Vec3::splat(j)),
                    Mat4::from_scale(glam::Vec3::splat(-j)),
                    Mat4::from_translation(glam::Vec3::new(j, -j, j)),
                ];
                for model_exponent in [-120, -1, 0, 80, 120] {
                    let model = Mat4::from_scale(glam::Vec3::splat(2.0f32.powi(model_exponent)));
                    let reference = admit_temporal_vertices(mesh.vertices(), &joints, model, false);
                    if finite_temporal_positions(mesh.max_position_component, &joints, model) {
                        certified += 1;
                        assert!(reference.is_ok());
                    } else {
                        uncertain += 1;
                    }
                    assert_eq!(mesh.validate_temporal_pose(&joints, model), reference);
                }
            }
        }
        assert!(certified > 0 && uncertain > 0);
    }
}

#[cfg(test)]
mod family_bound_tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn interval_family_implies_existing_group_certificate_for_all_unorm16_pairs() {
        let mut accepted = 0;
        let mut uncertain = 0;
        for (joints, model) in [
            ([Mat4::IDENTITY, Mat4::from_rotation_y(0.4)], Mat4::IDENTITY),
            (
                [Mat4::from_rotation_x(-0.2), Mat4::from_rotation_x(0.3)],
                Mat4::from_scale(Vec3::splat(-2.)),
            ),
            (
                [Mat4::IDENTITY, Mat4::from_rotation_y(std::f32::consts::PI)],
                Mat4::IDENTITY,
            ),
            (
                [Mat4::IDENTITY, Mat4::from_scale(Vec3::new(1., 0., 1.))],
                Mat4::IDENTITY,
            ),
            (
                [Mat4::IDENTITY; 2],
                Mat4::from_scale(Vec3::new(1., 1e-4, 1.)),
            ),
            ([Mat4::from_scale(Vec3::splat(1e-15)); 2], Mat4::IDENTITY),
            ([Mat4::from_scale(Vec3::splat(1e15)); 2], Mat4::IDENTITY),
        ] {
            let certificate =
                crate::pose_family_bound::certify([0, 1, u16::MAX, u16::MAX], &joints, model);
            if certificate {
                accepted += 1;
            } else {
                uncertain += 1;
            }
            for weight in 0..=u16::MAX {
                let vertex = SkinnedVertex {
                    position: [0.5, -0.25, 1.],
                    normal: [0.1, 0.9, -0.3],
                    uv: [0.; 2],
                    joints: [0, 1, 0, 0],
                    weights: [weight, u16::MAX - weight, 0, 0],
                };
                let world = model * blended_skin(&vertex, &joints);
                if certificate {
                    let normal = NormalTransform::new(world).unwrap();
                    assert!(normal.determinant >= CERTIFIED_NORMAL_DETERMINANT);
                    skinned_position_in(&vertex, world).unwrap();
                    skinned_normal_in(&vertex, world).unwrap();
                }
            }
        }
        assert!(accepted >= 2 && uncertain >= 2);
    }

    #[test]
    fn family_fast_path_and_uncertain_fallback_preserve_exact_errors_and_pose_changes() {
        let count = PARALLEL_ADMISSION_MIN_VERTICES + 17;
        for normal in [[0., 1., 0.], [0.; 3], [f32::MAX, 0., 0.]] {
            let vertices = (0..count)
                .map(|i| {
                    let w = (i % 65536) as u16;
                    SkinnedVertex {
                        position: [(i % 71) as f32 * 0.01, -0.5, 0.25],
                        normal,
                        uv: [0.; 2],
                        joints: [0, 1, 0, 0],
                        weights: [w, u16::MAX - w, 0, 0],
                    }
                })
                .collect();
            let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 2).unwrap();
            let shared = mesh.with_shared_indices(Arc::from([2, 1, 0])).unwrap();
            for joints in [
                [Mat4::IDENTITY, Mat4::from_rotation_y(0.2)],
                [Mat4::IDENTITY, Mat4::from_rotation_y(std::f32::consts::PI)],
                [Mat4::from_scale(Vec3::ZERO); 2],
                [Mat4::IDENTITY; 2],
            ] {
                for model in [
                    Mat4::IDENTITY,
                    Mat4::from_scale(Vec3::splat(-2.)),
                    Mat4::from_scale(Vec3::splat(f32::MAX)),
                ] {
                    let expected = mesh.vertices().iter().try_for_each(|v| {
                        let world = model * blended_skin(v, &joints);
                        skinned_position_in(v, world)?;
                        skinned_normal_in(v, world).map(|_| ())
                    });
                    for view in [&mesh, &shared] {
                        assert_eq!(view.validate_render_pose(&joints, model), expected);
                        let (result, profile) = view.validate_render_pose_profiled(&joints, model);
                        assert_eq!(result, expected);
                        assert!(profile.total_wall_us >= 0. && profile.caller_cpu_us >= 0.);
                    }
                }
            }
        }
    }

    #[test]
    fn four_influence_extreme_scales_and_cancellation_never_false_certify() {
        let family = [0, 1, 2, 3];
        let mut accepted = 0;
        let mut uncertain = 0;
        let mut seed = 42_u64;
        for exponent in [-120, -40, -1, 0, 20, 32, 80, 120] {
            for iteration in 0..32 {
                let mut random = || {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((seed >> 32) as u32 as f64 / u32::MAX as f64) as f32
                };
                let scale = 2.0_f32.powi(exponent);
                let joints = std::array::from_fn::<_, 4, _>(|j| {
                    Mat4::from_scale_rotation_translation(
                        Vec3::splat(scale),
                        glam::Quat::from_rotation_y(
                            (random() - 0.5) * if iteration % 2 == 0 { 0.2 } else { 6.0 },
                        ),
                        Vec3::new(j as f32, 0., 0.),
                    )
                });
                let model = Mat4::from_scale_rotation_translation(
                    Vec3::new(if iteration % 3 == 0 { -1. } else { 1. }, 1., 1.),
                    glam::Quat::from_rotation_x(0.1),
                    Vec3::splat(100.),
                );
                let certificate = crate::pose_family_bound::certify(family, &joints, model);
                if certificate {
                    accepted += 1;
                } else {
                    uncertain += 1;
                }
                for _ in 0..64 {
                    let mut cuts = [
                        (random() * 65535.) as u16,
                        (random() * 65535.) as u16,
                        (random() * 65535.) as u16,
                    ];
                    cuts.sort_unstable();
                    let weights = [
                        cuts[0],
                        cuts[1] - cuts[0],
                        cuts[2] - cuts[1],
                        u16::MAX - cuts[2],
                    ];
                    let v = SkinnedVertex {
                        position: [1., -0.5, 0.25],
                        normal: [0.2, -0.8, 0.5],
                        uv: [0.; 2],
                        joints: family,
                        weights,
                    };
                    if certificate {
                        let world = model * blended_skin(&v, &joints);
                        let normal = NormalTransform::new(world).unwrap();
                        assert!(normal.determinant >= CERTIFIED_NORMAL_DETERMINANT);
                        skinned_normal_in(&v, world).unwrap();
                    }
                }
            }
        }
        assert!(accepted > 0 && uncertain > 0);
    }
}

#[cfg(test)]
mod disabled_family_bound_tests {
    use super::*;
    #[test]
    fn diverse_joint_sets_disable_bound_without_vacuous_admission() {
        let vertices = (0..PARALLEL_ADMISSION_MIN_VERTICES + 11)
            .map(|i| SkinnedVertex {
                position: [0.1, 0.2, 0.3],
                normal: [0., 1., 0.],
                uv: [0.; 2],
                joints: [
                    (i % 32) as u16,
                    ((i / 32) % 32 + 32) as u16,
                    ((i / 1024) % 32 + 64) as u16,
                    0,
                ],
                weights: [20000, 20000, 25535, 0],
            })
            .collect();
        let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 96).unwrap();
        assert!(mesh.groups.as_ref().unwrap().families.is_empty());
        assert_eq!(
            mesh.validate_render_pose(&[Mat4::IDENTITY; 96], Mat4::IDENTITY),
            Ok(())
        );
        let (result, profile) = mesh.validate_render_pose_profiled(
            &[Mat4::from_scale(glam::Vec3::ZERO); 96],
            Mat4::IDENTITY,
        );
        assert_eq!(result, Err(SkinnedUploadError::SingularNormalMatrix));
        assert!(profile.fallback);
    }
}
