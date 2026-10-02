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
    pipeline: wgpu::ComputePipeline,
    identity: Arc<()>,
}
#[derive(Debug)]
pub struct SceneSkinSource {
    device: wgpu::Device,
    mesh: Arc<SkinnedMesh>,
    buffer: wgpu::Buffer,
}
#[derive(Debug)]
pub struct SceneSkinInstance {
    owner: Arc<()>,
    source: Arc<SceneSkinSource>,
    geometry: SceneGeometry,
    palette: wgpu::Buffer,
    binding: wgpu::BindGroup,
    bytes: u64,
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
            pipeline,
            identity: Arc::new(()),
        })
    }

    /// Upload shared immutable skin attributes after logical-byte admission.
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
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shared scene skin source"),
                contents: bytemuck::cast_slice(&packed),
                usage: wgpu::BufferUsages::STORAGE,
            });
        Ok(Arc::new(SceneSkinSource {
            device: self.device.clone(),
            mesh,
            buffer,
        }))
    }

    /// Creates an independently posed instance. The initial CPU preview ensures
    /// valid geometry before the first encoded compute pass. Source bytes are
    /// accounted separately; `live` must include every retained owner.
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
        let mesh = source
            .mesh
            .posed_scene_mesh(joints, Mat4::IDENTITY, color)
            .map_err(SceneSkinError::Bake)?
            .with_material_coordinates(source.mesh.vertices().iter().map(|v| v.position).collect())
            .map_err(SceneSkinError::Scene)?;
        let bytes = SceneRenderer::mesh_allocation_bytes(&mesh) + joints.len() as u64 * 64 + 32;
        admit(live, bytes, budget)?;
        let geometry = renderer
            .upload_mesh_with_usage(
                &self.device,
                &mesh,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            )
            .map_err(SceneSkinError::Scene)?;
        let palette = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene instance palette"),
                contents: bytemuck::cast_slice(joints),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let mut params = Vec::from(bytemuck::cast_slice::<f32, u8>(&color));
        params.extend((mesh.vertices().len() as u32).to_le_bytes());
        params.extend([0; 12]);
        let parameters = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene instance skin parameters"),
                contents: &params,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let buffers = [
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
            bytes,
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
        queue.write_buffer(&instance.palette, 0, bytemuck::cast_slice(joints));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("scene skin pose"),
            timestamp_writes: None,
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
        self.bytes
    }
}

#[cfg(test)]
mod tests;
