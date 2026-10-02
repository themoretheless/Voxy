//! Reconstruct primary world-space surfaces from the matching depth and normal guides.
use crate::RaySceneError;
use wgpu::util::DeviceExt;

/// Row-major GPU surface array: position.xyz/valid, world normal.xyz/roughness.
#[derive(Debug)]
pub struct PrimarySurfaceJob {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Buffer,
    dimensions: [u32; 2],
}
/// Device-owned compiled depth-to-world reconstruction pipeline.
/// Jobs keep independent camera uniforms, bindings and surface buffers.
#[derive(Debug)]
pub struct PrimarySurfacePipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
impl PrimarySurfacePipeline {
    /// # Errors
    /// Rejects insufficient compute and binding limits before compilation.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_sampled_textures_per_shader_stage < 2
            || limits.max_storage_buffers_per_shader_stage < 1
        {
            return Err(RaySceneError::Capacity);
        }
        Ok(Self {
            device: device.clone(),
            pipeline: reconstruction_pipeline(device),
        })
    }
    /// Prepare a reconstruction without recompiling the shader/pipeline.
    /// Textures must belong to this device and describe the same raster surfaces.
    /// # Errors
    /// Preserves camera, guide and surface-buffer capacity validation errors.
    pub fn prepare(
        &self,
        depth: &wgpu::Texture,
        normal_roughness: &wgpu::Texture,
        view_projection: glam::Mat4,
        clear_depth: f32,
    ) -> Result<PrimarySurfaceJob, RaySceneError> {
        PrimarySurfaceJob::create(
            &self.device,
            depth,
            normal_roughness,
            view_projection,
            clear_depth,
            Some(&self.pipeline),
            None,
        )
    }
    /// Transfer a previous job's surface buffer into a new reconstruction.
    /// Equal dimensions reuse storage; resize allocates replacement storage.
    /// Order all old buffer consumers before the new encode on the same queue.
    /// Cloned raw buffer handles must obey that order too; consumption does not
    /// imply exclusive ownership of the underlying GPU resource.
    /// # Errors
    /// Rejects foreign previous jobs and preserves normal preparation validation.
    pub fn prepare_reusing(
        &self,
        previous: PrimarySurfaceJob,
        depth: &wgpu::Texture,
        normal_roughness: &wgpu::Texture,
        view_projection: glam::Mat4,
        clear_depth: f32,
    ) -> Result<PrimarySurfaceJob, RaySceneError> {
        previous.validate_device(&self.device)?;
        let reuse =
            (previous.dimensions == [depth.width(), depth.height()]).then_some(previous.output);
        PrimarySurfaceJob::create(
            &self.device,
            depth,
            normal_roughness,
            view_projection,
            clear_depth,
            Some(&self.pipeline),
            reuse,
        )
    }
}
impl PrimarySurfaceJob {
    pub(crate) fn validate_device(&self, device: &wgpu::Device) -> Result<(), RaySceneError> {
        if device != &self.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        Ok(())
    }
    /// Unproject pixel centers with the exact jittered view-projection used for depth.
    /// Wgpu depth is [0,1]; Y is flipped from texture coordinates to clip coordinates.
    /// Supply the depth clear value for either conventional or reversed depth.
    /// Background/invalid normal samples write zero valid flag and zero data.
    /// Input textures must share this device and describe the same primary surfaces.
    /// # Errors
    /// Rejects invalid matrices, formats/dimensions, missing sampling usage or limits.
    pub fn new(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        normal_roughness: &wgpu::Texture,
        view_projection: glam::Mat4,
        clear_depth: f32,
    ) -> Result<Self, RaySceneError> {
        Self::create(
            device,
            depth,
            normal_roughness,
            view_projection,
            clear_depth,
            None,
            None,
        )
    }
    fn create(
        device: &wgpu::Device,
        depth: &wgpu::Texture,
        normal_roughness: &wgpu::Texture,
        view_projection: glam::Mat4,
        clear_depth: f32,
        compiled: Option<&wgpu::ComputePipeline>,
        reuse: Option<wgpu::Buffer>,
    ) -> Result<Self, RaySceneError> {
        let determinant = view_projection.determinant();
        if !view_projection.is_finite() || !determinant.is_finite() || determinant == 0.0 {
            return Err(RaySceneError::InvalidGeometry);
        }
        let inverse = view_projection.inverse();
        if !view_projection.is_finite()
            || !inverse.is_finite()
            || !clear_depth.is_finite()
            || !(0.0..=1.0).contains(&clear_depth)
            || depth.format() != wgpu::TextureFormat::Depth32Float
            || !matches!(
                normal_roughness.format(),
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
            || depth.size() != normal_roughness.size()
            || [depth, normal_roughness].into_iter().any(|t| {
                t.dimension() != wgpu::TextureDimension::D2
                    || t.depth_or_array_layers() != 1
                    || t.sample_count() != 1
                    || !t.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
            })
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let limits = device.limits();
        let bytes = u64::from(depth.width()) * u64::from(depth.height()) * 32;
        if limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_sampled_textures_per_shader_stage < 2
            || limits.max_storage_buffers_per_shader_stage < 1
            || bytes > limits.max_buffer_size
            || bytes > limits.max_storage_buffer_binding_size
            || depth.width().div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || depth.height().div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(RaySceneError::Capacity);
        }
        let mut camera = inverse.to_cols_array().to_vec();
        camera.extend([clear_depth, 0.0, 0.0, 0.0]);
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primary inverse camera"),
            contents: bytemuck::cast_slice(&camera),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output = reuse.unwrap_or_else(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("primary world surfaces"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        let pipeline = compiled.map_or_else(|| reconstruction_pipeline(device), Clone::clone);
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_view = normal_roughness.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_sampler = crate::depth_sample::sampler(device);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("primary surface inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
            bindings,
            output,
            dimensions: [depth.width(), depth.height()],
        })
    }
    /// Encode after matching depth and normal producers on the same GPU stream.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.dimensions[0].div_ceil(8),
            self.dimensions[1].div_ceil(8),
            1,
        );
    }
    #[must_use]
    pub const fn dimensions(&self) -> [u32; 2] {
        self.dimensions
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Buffer {
        &self.output
    }
}

fn reconstruction_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = crate::depth_sample::module(
        device,
        "depth to world surface",
        include_str!("primary_surface.wgsl"),
    );
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("depth to world surface"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}
