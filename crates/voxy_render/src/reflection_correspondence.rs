//! Previous deformed triangle positions for current reflected ray hits.
use crate::{RaySceneError, SurfaceReflectionJob};
use wgpu::util::DeviceExt;

/// Keys use the current hit identity. Vertices are corresponding previous-frame
/// WORLD positions in unchanged vertex order/topology. Omit newly created or
/// topology-changed triangles. Remap stable scene IDs to current instance indices
/// before preparing this input; never reuse a table across incompatible epochs.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PreviousReflectionTriangle {
    pub identity: [u32; 4],
    pub vertices: [[f32; 4]; 3],
}
#[derive(Debug)]
pub struct ReflectionCorrespondencePipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct ReflectionCorrespondenceJob {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    positions: wgpu::Buffer,
    dimensions: [u32; 2],
}
impl ReflectionCorrespondencePipeline {
    /// # Errors
    /// Rejects insufficient compute/storage capacity.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 3
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
        {
            return Err(RaySceneError::Capacity);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reflected previous triangle correspondence"),
            source: wgpu::ShaderSource::Wgsl(include_str!("reflection_correspondence.wgsl").into()),
        });
        Ok(Self {
            device: device.clone(),
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("reflected correspondence pipeline"),
                layout: None,
                module: &shader,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            }),
        })
    }
    /// Prepare immutable previous geometry and explicit history-reset state.
    /// Encode after the matching reflection job and before readers of positions.
    /// Output is row-major previous WORLD XYZ with W=1 for correspondence, zero
    /// for miss/unknown identity/reset. It is not reflection screen-space motion:
    /// reprojection must also account for the reflecting surface and camera.
    /// # Errors
    /// Rejects foreign reflection jobs, duplicate identities, nonfinite vertices
    /// and buffer/dispatch limits before resource creation.
    pub fn prepare(
        &self,
        reflected: &SurfaceReflectionJob,
        triangles: &[PreviousReflectionTriangle],
        reset: bool,
    ) -> Result<ReflectionCorrespondenceJob, RaySceneError> {
        reflected.validate_device(&self.device)?;
        self.prepare_hits(
            reflected.hits(),
            [reflected.distance().width(), reflected.distance().height()],
            triangles,
            reset,
        )
    }
    /// Prepare from row-major ReflectionHit storage on this device. The caller
    /// must preserve hit/table scene identity and encode after the hit producer.
    /// wgpu validates raw-buffer device ownership during binding.
    /// # Errors
    /// Rejects incompatible buffer/extent, duplicate/nonfinite geometry and limits.
    pub fn prepare_hits(
        &self,
        hits: &wgpu::Buffer,
        dimensions: [u32; 2],
        triangles: &[PreviousReflectionTriangle],
        reset: bool,
    ) -> Result<ReflectionCorrespondenceJob, RaySceneError> {
        let [width, height] = dimensions;
        let count = u64::from(width) * u64::from(height);
        if count == 0
            || hits.size() != count.checked_mul(48).ok_or(RaySceneError::Capacity)?
            || !hits.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let mut sorted = triangles.to_vec();
        sorted.sort_unstable_by_key(|t| t.identity);
        if sorted
            .iter()
            .any(|t| t.vertices.iter().flatten().any(|v| !v.is_finite()))
            || sorted
                .windows(2)
                .any(|pair| pair[0].identity == pair[1].identity)
        {
            return Err(RaySceneError::InvalidGeometry);
        }

        let triangle_count = u32::try_from(sorted.len()).map_err(|_| RaySceneError::Capacity)?;
        let bytes = u64::from(triangle_count.max(1)) * 64;
        let output_bytes = count.checked_mul(16).ok_or(RaySceneError::Capacity)?;
        let limits = self.device.limits();
        if width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || hits.size() > limits.max_storage_buffer_binding_size
            || count > u64::from(u32::MAX)
            || [bytes, output_bytes]
                .iter()
                .any(|n| *n > limits.max_buffer_size || *n > limits.max_storage_buffer_binding_size)
        {
            return Err(RaySceneError::Capacity);
        }
        if sorted.is_empty() {
            sorted.push(bytemuck::Zeroable::zeroed());
        }
        let triangles = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("previous reflected triangles sorted by current identity"),
                contents: bytemuck::cast_slice(&sorted),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let options = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("reflected correspondence reset"),
                contents: bytemuck::cast_slice(&[triangle_count, u32::from(reset), width, height]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let positions = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("previous reflected world positions"),
            size: output_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reflection correspondence inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: hits.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: triangles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: positions.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: options.as_entire_binding(),
                },
            ],
        });
        Ok(ReflectionCorrespondenceJob {
            device: self.device.clone(),
            pipeline: self.pipeline.clone(),
            bindings,
            positions,
            dimensions,
        })
    }
}
impl ReflectionCorrespondenceJob {
    pub(crate) fn validate_device(&self, device: &wgpu::Device) -> Result<(), RaySceneError> {
        if self.device != *device {
            return Err(RaySceneError::DeviceMismatch);
        }
        Ok(())
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("previous reflected positions"),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.dimensions[0].div_ceil(8),
            self.dimensions[1].div_ceil(8),
            1,
        );
    }
    #[must_use]
    pub fn positions(&self) -> &wgpu::Buffer {
        &self.positions
    }
}
