//! GPU deformation into the existing scene vertex ABI. Immutable source data is
//! shared; palette and output streams belong to a single animated instance.
use super::*;
use crate::{SkinnedMesh, SkinnedUploadError};
use std::sync::Arc;

#[derive(Debug)]
pub enum SceneSkinError {
    Scene(SceneError),
    Pose(SkinnedUploadError),
    Bake(Box<dyn std::error::Error>),
    Unsupported,
    ForeignSkinner,
    BudgetExceeded {
        live: u64,
        additional: u64,
        budget: u64,
    },
}
impl std::fmt::Display for SceneSkinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene skinning error: {self:?}")
    }
}
impl std::error::Error for SceneSkinError {}

#[derive(Debug)]
pub struct SceneSkinner {
    device: wgpu::Device,
    memory_budget: crate::ComputeMemoryBudget,
    pipeline: wgpu::ComputePipeline,
    identity: Arc<()>,
}
#[derive(Debug)]
pub struct SceneSkinSource {
    device: wgpu::Device,
    mesh: Arc<SkinnedMesh>,
    buffer: crate::ComputeStorage,
}
#[derive(Debug)]
pub struct SceneSkinInstance {
    owner: Arc<()>,
    source: Arc<SceneSkinSource>,
    geometry: SceneGeometry,
    palette: Arc<crate::ComputeStorage>,
    binding: wgpu::BindGroup,
    parameters: Arc<crate::ComputeStorage>,
}

/// A pose preflight bound to immutable instance and palette borrows.
/// Private fields prevent bypassing validation or substituting another palette.
///
/// ```compile_fail
/// fn mutate_palette(skinner: &voxy_render::SceneSkinner,
///     instance: &voxy_render::SceneSkinInstance, queue: &wgpu::Queue,
///     encoder: &mut wgpu::CommandEncoder, joints: &mut Vec<glam::Mat4>) {
///     let prepared = skinner.prepare_pose(instance, joints).unwrap();
///     joints[0] = glam::Mat4::IDENTITY;
///     skinner.encode_prepared_pose(queue, encoder, &prepared).unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct SceneSkinPose<'a> {
    instance: &'a SceneSkinInstance,
    joints: &'a [Mat4],
}

/// One immutable index variant borrowing the owner's deformed GPU streams.
/// Keep the owner instance alive while updating/rendering this level.
#[derive(Debug)]
pub struct SceneSkinLodLevel {
    geometry: SceneGeometry,
}
impl SceneSkinLodLevel {
    pub fn geometry(&self) -> &SceneGeometry {
        &self.geometry
    }
    /// Additional logical bytes; shared streams are counted by the owner.
    pub fn index_allocation_bytes(&self) -> u64 {
        self.geometry.indices.size()
    }
}

fn admit(live: u64, additional: u64, budget: u64) -> Result<(), SceneSkinError> {
    if additional > budget.saturating_sub(live) || live > budget {
        return Err(SceneSkinError::BudgetExceeded {
            live,
            additional,
            budget,
        });
    }
    Ok(())
}
impl SceneSkinner {
    /// Creates the shared compute pipeline. Unsupported backends must choose
    /// explicit CPU baking instead. This does not alter scene shader layouts.
    pub fn new(renderer: &SceneRenderer) -> Result<Self, SceneSkinError> {
        let device = &renderer.device;
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 4
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_compute_workgroup_size_x < 64
        {
            return Err(SceneSkinError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene skeletal deformation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("skinning.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("scene skeletal deformation"),
            layout: None,
            module: &shader,
            entry_point: Some("deform"),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            memory_budget: renderer.compute_memory_budget().clone(),
            pipeline,
            identity: Arc::new(()),
        })
    }

    /// Upload shared immutable skin attributes after logical-byte and device-budget admission.
    pub fn upload_source(
        &self,
        mesh: Arc<SkinnedMesh>,
        live: u64,
        budget: u64,
    ) -> Result<Arc<SceneSkinSource>, SceneSkinError> {
        let bytes = mesh.vertices().len() as u64 * 64;
        admit(live, bytes, budget)?;
        if bytes > u64::from(self.device.limits().max_storage_buffer_binding_size)
            || mesh.vertices().len().div_ceil(64)
                > self.device.limits().max_compute_workgroups_per_dimension as usize
        {
            return Err(SceneSkinError::Unsupported);
        }
        let mut packed = Vec::with_capacity(mesh.vertices().len() * 16);
        for v in mesh.vertices() {
            packed.extend(v.position);
            packed.extend(v.normal);
            packed.extend(v.uv);
            packed.extend(v.joints.map(f32::from));
            packed.extend(v.weights.map(|w| f32::from(w) / 65535.0));
        }
        let [buffer] = self
            .memory_budget
            .allocate_buffers([crate::compute_memory::ManagedBufferDescriptor {
                label: "shared scene skin source",
                size: bytes,
                contents: Some(bytemuck::cast_slice(&packed)),
                usage: wgpu::BufferUsages::STORAGE,
            }])
            .map_err(|error| SceneSkinError::Scene(super::scene_memory_error(error)))?;
        Ok(Arc::new(SceneSkinSource {
            device: self.device.clone(),
            mesh,
            buffer,
        }))
    }

    /// Creates an independently posed instance. The initial CPU preview ensures
    /// valid geometry before the first encoded compute pass. Source bytes are
    /// accounted separately; `live` must include every retained owner.
    /// Geometry, palette and parameters are admitted atomically against the
    /// shared device budget, preserving existing instances on rejection.
    pub fn create_instance(
        &self,
        renderer: &SceneRenderer,
        source: Arc<SceneSkinSource>,
        joints: &[Mat4],
        color: [f32; 4],
        live: u64,
        budget: u64,
    ) -> Result<SceneSkinInstance, SceneSkinError> {
        if self.device != renderer.device || self.device != source.device {
            return Err(SceneSkinError::Scene(SceneError::DeviceMismatch));
        }
        source
            .mesh
            .validate_temporal_pose(joints, Mat4::IDENTITY)
            .map_err(SceneSkinError::Pose)?;
        source
            .mesh
            .validate_scene_normals(joints)
            .map_err(SceneSkinError::Pose)?;
        let mesh = source
            .mesh
            .posed_scene_mesh(joints, Mat4::IDENTITY, color)
            .map_err(SceneSkinError::Bake)?
            .with_material_coordinates(source.mesh.vertices().iter().map(|v| v.position).collect())
            .map_err(SceneSkinError::Scene)?;
        let bytes = SceneRenderer::mesh_allocation_bytes(&mesh) + joints.len() as u64 * 64 + 32;
        admit(live, bytes, budget)?;
        let mut params = Vec::from(bytemuck::cast_slice::<f32, u8>(&color));
        params.extend((mesh.vertices().len() as u32).to_le_bytes());
        params.extend([0; 12]);
        let descriptors = [
            crate::compute_memory::ManagedBufferDescriptor {
                label: "scene instance palette",
                size: joints.len() as u64 * 64,
                contents: Some(bytemuck::cast_slice(joints)),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
            crate::compute_memory::ManagedBufferDescriptor {
                label: "scene instance skin parameters",
                size: params.len() as u64,
                contents: Some(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            },
        ];
        let mut batch = renderer
            .upload_mesh_buffer_batch(
                &self.device,
                &mesh,
                &[mesh.indices()],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                &descriptors,
            )
            .map_err(SceneSkinError::Scene)?;
        let geometry = batch.geometries.remove(0);
        let [palette, parameters]: [_; 2] = batch
            .additional_buffers
            .try_into()
            .expect("validated palette and parameter descriptors");
        let buffers: [&wgpu::Buffer; 5] = [
            &source.buffer,
            &palette,
            &geometry.vertices,
            &geometry.normals,
            &parameters,
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene instance skin"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        Ok(SceneSkinInstance {
            owner: Arc::clone(&self.identity),
            source,
            geometry,
            palette,
            binding,
            parameters,
        })
    }

    /// Validate the complete pose before any writes, then encode deformation
    /// before scene/shadow draws in the same command encoder. Submit previous
    /// commands before updating this instance again. Instance transforms remain
    /// in SceneTransform; palettes already contain the imported node hierarchy.
    pub fn encode_pose(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        instance: &SceneSkinInstance,
        joints: &[Mat4],
    ) -> Result<(), SceneSkinError> {
        let pose = self.prepare_pose(instance, joints)?;
        self.encode_prepared_pose(queue, encoder, &pose)
    }

    /// Encodes a previously preflighted immutable palette without repeating
    /// CPU vertex validation. Rejects tokens prepared by another skinner.
    pub fn encode_prepared_pose(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pose: &SceneSkinPose<'_>,
    ) -> Result<(), SceneSkinError> {
        self.encode_prepared_pose_profiled(queue, encoder, pose, None)
    }

    /// Optional compute-pass boundary timestamps. The caller owns the timestamp
    /// query set and resolves it after encoding. Requires TIMESTAMP_QUERY; query
    /// device, indices and type must satisfy wgpu validation requirements.
    /// Unsupported timestamp features are rejected before palette writes.
    pub fn encode_prepared_pose_profiled(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pose: &SceneSkinPose<'_>,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) -> Result<(), SceneSkinError> {
        if timestamp_writes.is_some()
            && !self
                .device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return Err(SceneSkinError::Unsupported);
        }
        let instance = pose.instance;
        if !Arc::ptr_eq(&self.identity, &instance.owner) {
            return Err(SceneSkinError::ForeignSkinner);
        }
        queue.write_buffer(&instance.palette, 0, bytemuck::cast_slice(pose.joints));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("scene skin pose"),
            timestamp_writes,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &instance.binding, &[]);
        pass.dispatch_workgroups(
            instance.source.mesh.vertices().len().div_ceil(64) as u32,
            1,
            1,
        );
        Ok(())
    }
    /// Uploads only a certified source's chosen index variant. Vertex, normal,
    /// material-coordinate and material-parameter streams remain shared with
    /// the owner's base geometry. Pose-specific error certification and camera
    /// selection belong to the caller; this operation makes no pose-quality claim.
    /// Failed source validation/admission leaves the owner untouched.
    pub fn create_lod_level(
        &self,
        instance: &SceneSkinInstance,
        source: &crate::SkinnedLodMesh,
        level: usize,
        live: u64,
        budget: u64,
    ) -> Result<SceneSkinLodLevel, SceneSkinError> {
        if !Arc::ptr_eq(&self.identity, &instance.owner) {
            return Err(SceneSkinError::ForeignSkinner);
        }
        let mesh = source.mesh();
        if mesh.joint_count() != instance.source.mesh.joint_count()
            || mesh.vertices() != instance.source.mesh.vertices()
            || mesh.indices() != instance.source.mesh.indices()
        {
            return Err(SceneSkinError::Scene(SceneError::InvalidGeometry));
        }
        let indices = source
            .indices(level)
            .ok_or(SceneSkinError::Scene(SceneError::InvalidGeometry))?;
        geometry_sizes(&self.device, mesh.vertices().len(), indices.len())
            .map_err(SceneSkinError::Scene)?;
        let bytes = indices.len() as u64 * 4;
        admit(live, bytes, budget)?;
        let indices_buffer =
            super::managed_scene_indices(&self.device, indices).map_err(SceneSkinError::Scene)?;
        let base = &instance.geometry;
        Ok(SceneSkinLodLevel {
            geometry: SceneGeometry {
                device: self.device.clone(),
                vertices: base.vertices.clone(),
                normals: base.normals.clone(),
                normal_cache: NormalCache::default(),
                material_coordinates: base.material_coordinates.clone(),
                coordinate_cache: Vec::new(),
                material_parameters: base.material_parameters.clone(),
                indices: indices_buffer,
                index_count: indices.len() as u32,
                vertex_capacity: base.vertex_capacity,
                index_capacity: indices.len(),
                depth_mode: base.depth_mode,
            },
        })
    }

    /// Preflights a palette without writing GPU buffers or encoding commands.
    /// Use before publishing a model containing several animated primitives.
    pub fn validate_pose(
        &self,
        instance: &SceneSkinInstance,
        joints: &[Mat4],
    ) -> Result<(), SceneSkinError> {
        self.prepare_pose(instance, joints).map(|_| ())
    }

    /// Preflights without queue writes. Borrowed inputs cannot be modified while
    /// the prepared pose is in use; callers may stage all primitives first.
    pub fn prepare_pose<'a>(
        &self,
        instance: &'a SceneSkinInstance,
        joints: &'a [Mat4],
    ) -> Result<SceneSkinPose<'a>, SceneSkinError> {
        if self.device != instance.source.device {
            return Err(SceneSkinError::Scene(SceneError::DeviceMismatch));
        }
        if !Arc::ptr_eq(&self.identity, &instance.owner) {
            return Err(SceneSkinError::ForeignSkinner);
        }
        instance
            .source
            .mesh
            .validate_temporal_pose(joints, Mat4::IDENTITY)
            .map_err(SceneSkinError::Pose)?;
        instance
            .source
            .mesh
            .validate_scene_normals(joints)
            .map_err(SceneSkinError::Pose)?;
        Ok(SceneSkinPose { instance, joints })
    }
}
impl SceneSkinSource {
    pub fn allocation_bytes(&self) -> u64 {
        self.buffer.size()
    }
}
impl SceneSkinInstance {
    pub fn geometry(&self) -> &SceneGeometry {
        &self.geometry
    }
    pub fn allocation_bytes(&self) -> u64 {
        self.geometry.allocation_bytes() + self.palette.size() + self.parameters.size()
    }
}

#[cfg(test)]
mod tests;
