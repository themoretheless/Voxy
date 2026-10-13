//! Ordered resident lighting from the same geometry streams used for drawing.
use crate::{
    SceneError, SceneGeometry, SurfaceLightAdmission, SurfaceLightGeometry, SurfaceLightPcg,
    SurfaceLightTransfer, SurfaceLightVertexBinding,
};
use wgpu::util::DeviceExt;
#[derive(Debug, Clone, Copy)]
pub struct SurfaceLightFrameSettings {
    pub key_direction: [f32; 3],
    pub key_weight: f32,
    pub fill_direction: [f32; 3],
    pub fill_weight: f32,
    pub ambient: f32,
    pub albedo: [f32; 4],
    pub surface_fraction: f32,
    pub radii: [f32; 3],
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    key: [f32; 4],
    fill: [f32; 4],
    ambient: f32,
    count: u32,
    padding: [u32; 2],
}
#[derive(Debug)]
pub struct SurfaceLightFrame {
    incident: wgpu::Buffer,
    incident_pipeline: wgpu::ComputePipeline,
    incident_group: wgpu::BindGroup,
    operator: SurfaceLightGeometry,
    solver: SurfaceLightPcg,
    admission: SurfaceLightAdmission,
    transfer: SurfaceLightTransfer,
    groups: u32,
    surface_fraction: f32,
    reference_normals: Option<wgpu::Buffer>,
}
impl SurfaceLightFrame {
    /// Light a fixed prefix of the resident mesh. Topology and settings are immutable.
    /// Encode after pose/normal updates and before drawing. Invalid geometry or
    /// normals reject publication and retain the last complete illumination pair.
    pub async fn new(
        device: &wgpu::Device,
        geometry: &SceneGeometry,
        count: usize,
        triangles: &[[u32; 3]],
        settings: SurfaceLightFrameSettings,
    ) -> Result<Self, SceneError> {
        Self::new_with_normal_source(
            device,
            geometry,
            count,
            triangles,
            settings,
            geometry.deformation_normals(),
        )
        .await
    }
    /// Bind the authored lighting-normal stream independently of geometric draw
    /// normals. This stream may be produced by an earlier GPU transport pass.
    pub async fn new_with_normal_source(
        device: &wgpu::Device,
        geometry: &SceneGeometry,
        count: usize,
        triangles: &[[u32; 3]],
        settings: SurfaceLightFrameSettings,
        normals: &wgpu::Buffer,
    ) -> Result<Self, SceneError> {
        if !geometry.belongs_to(device) {
            return Err(SceneError::DeviceMismatch);
        }
        let direction = |v: [f32; 3]| glam::Vec3::from(v).try_normalize().map(|v| v.to_array());
        let key = direction(settings.key_direction).ok_or(SceneError::InvalidGeometry)?;
        let fill = direction(settings.fill_direction).ok_or(SceneError::InvalidGeometry)?;
        let limits = device.limits();
        let bytes = (count as u64)
            .checked_mul(16)
            .ok_or(SceneError::InvalidGeometry)?;
        if count == 0
            || count > geometry.deformation_vertex_capacity()
            || count > u32::MAX as usize
            || normals.size() < count as u64 * 12
            || count.div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize
            || bytes > limits.max_buffer_size
            || bytes > limits.max_storage_buffer_binding_size
            || normals.size() > limits.max_storage_buffer_binding_size
            || !normals.usage().contains(wgpu::BufferUsages::STORAGE)
            || [settings.ambient, settings.key_weight, settings.fill_weight]
                .iter()
                .any(|x| !x.is_finite() || *x < 0.)
            || !(settings.ambient + settings.key_weight + settings.fill_weight).is_finite()
        {
            return Err(SceneError::InvalidGeometry);
        }
        let operator = SurfaceLightGeometry::new(
            device,
            geometry.deformation_vertices(),
            count,
            9,
            triangles,
        )?;
        let solver = SurfaceLightPcg::from_geometry(device, &operator, settings.radii)?;
        let incident = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("resident normal incident lighting"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let admission = SurfaceLightAdmission::new(
            device,
            &operator,
            solver.output(),
            &incident,
            settings.radii,
            1e-3,
            1e-6,
        )?;
        let mapping: Vec<_> = (0..count)
            .map(|i| SurfaceLightVertexBinding {
                albedo: settings.albedo,
                vertex: i as u32,
                light: i as u32,
                surface_fraction: settings.surface_fraction,
                padding: 0,
            })
            .collect();
        let transfer = SurfaceLightTransfer::new(device, geometry, &admission, &mapping)
            .await
            .map_err(|_| SceneError::InvalidGeometry)?;
        let params = Parameters {
            key: [key[0], key[1], key[2], settings.key_weight],
            fill: [fill[0], fill[1], fill[2], settings.fill_weight],
            ambient: settings.ambient,
            count: count as u32,
            padding: [0; 2],
        };
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("incident lighting settings"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resident normal lighting"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_frame.wgsl").into()),
        });
        let incident_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("resident normal lighting"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let resources = [&uniform, normals, &incident];
        let entries: Vec<_> = resources
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let incident_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resident normal lighting streams"),
            layout: &incident_pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        if scope.pop().await.is_some() {
            return Err(SceneError::InvalidGeometry);
        }
        Ok(Self {
            incident,
            incident_pipeline,
            incident_group,
            operator,
            solver,
            admission,
            transfer,
            groups: (count as u32).div_ceil(64),
            surface_fraction: settings.surface_fraction,
            reference_normals: None,
        })
    }
    /// Transitional CPU authored-normal input. Positions, light calculation,
    /// operator, solve and publication remain resident. No GPU light readback.
    pub async fn new_with_reference_normals(
        device: &wgpu::Device,
        geometry: &SceneGeometry,
        triangles: &[[u32; 3]],
        settings: SurfaceLightFrameSettings,
        normals: &[[f32; 3]],
    ) -> Result<Self, SceneError> {
        if normals.is_empty() || normals.iter().flatten().any(|n| !n.is_finite()) {
            return Err(SceneError::InvalidGeometry);
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("authored lighting normal reference"),
            contents: bytemuck::cast_slice(normals),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let mut frame = Self::new_with_normal_source(
            device,
            geometry,
            normals.len(),
            triangles,
            settings,
            &buffer,
        )
        .await?;
        frame.reference_normals = Some(buffer);
        Ok(frame)
    }
    pub fn upload_reference_normals(
        &self,
        queue: &wgpu::Queue,
        normals: &[[f32; 3]],
    ) -> Result<(), SceneError> {
        if normals.len() != self.admission.vertex_count()
            || normals.iter().flatten().any(|n| !n.is_finite())
        {
            return Err(SceneError::InvalidGeometry);
        }
        let buffer = self
            .reference_normals
            .as_ref()
            .ok_or(SceneError::InvalidGeometry)?;
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(normals));
        Ok(())
    }
    /// A fresh solve uses the current positions and normals without CPU readback.
    /// Iteration count is a budget; independent admission decides publication.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        iterations: u32,
    ) -> Result<(), SceneError> {
        if iterations == 0 || iterations > 8192 {
            return Err(SceneError::InvalidGeometry);
        }
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.incident_pipeline);
            pass.set_bind_group(0, &self.incident_group, &[]);
            pass.dispatch_workgroups(self.groups, 1, 1);
        }
        self.operator.encode(encoder);
        self.solver
            .reset_from_resident_source(encoder, &self.incident)?;
        self.solver.encode_timed(encoder, iterations, None)?;
        self.admission.encode(encoder);
        self.transfer.encode(encoder);
        Ok(())
    }
    #[must_use]
    pub fn admission(&self) -> &SurfaceLightAdmission {
        &self.admission
    }
    #[must_use]
    pub fn surface_fraction(&self) -> f32 {
        self.surface_fraction
    }
    #[must_use]
    pub fn incident_buffer(&self) -> &wgpu::Buffer {
        &self.incident
    }
}
