//! Conservative current-frame spatial filtering for sampled rough reflections.
use crate::{PrimarySurfaceJob, RaySceneError, SurfaceReflectionJob};
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, Debug)]
pub struct ReflectionSpatialOptions {
    pub normal_cosine: f32,
    pub plane_tolerance: f32,
    pub relative_distance_tolerance: f32,
    pub minimum_roughness: f32,
}
impl Default for ReflectionSpatialOptions {
    fn default() -> Self {
        Self {
            normal_cosine: 0.95,
            plane_tolerance: 0.02,
            relative_distance_tolerance: 0.1,
            minimum_roughness: 0.05,
        }
    }
}
#[derive(Debug)]
pub struct ReflectionSpatialPipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct ReflectionSpatialJob {
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
impl ReflectionSpatialPipeline {
    /// # Errors
    /// Rejects insufficient compute/storage capacity.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_storage_textures_per_shader_stage < 1
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
        {
            return Err(RaySceneError::Capacity);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rough reflection spatial filter"),
            source: wgpu::ShaderSource::Wgsl(include_str!("reflection_spatial.wgsl").into()),
        });
        Ok(Self {
            device: device.clone(),
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("reflection spatial filter"),
                layout: None,
                module: &shader,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            }),
        })
    }
    /// Encode after the matching primary reconstruction and reflection query.
    /// A 3x3 kernel rejects incompatible local tangent planes, normals, roughness,
    /// reflected geometry identity and hit distances when both samples hit. Perfect mirrors
    /// bypass filtering. Misses on valid rough primary surfaces contribute zero
    /// radiance, including a missed centre, to reduce stochastic hit/miss speckles.
    /// This is a biased spatial estimator, not temporal motion,
    /// multi-frame denoising or proof of image quality on arbitrary curved geometry.
    /// # Errors
    /// Rejects foreign producers and incompatible resources/settings.
    pub fn prepare(
        &self,
        primary: &PrimarySurfaceJob,
        reflected: &SurfaceReflectionJob,
        options: ReflectionSpatialOptions,
    ) -> Result<ReflectionSpatialJob, RaySceneError> {
        primary.validate_device(&self.device)?;
        reflected.validate_device(&self.device)?;
        if primary.dimensions() != [reflected.radiance().width(), reflected.radiance().height()] {
            return Err(RaySceneError::InvalidGeometry);
        }
        self.prepare_raw(
            primary.output(),
            reflected.hits(),
            reflected.radiance(),
            options,
        )
    }
    /// Same-device, row-major primary 32-byte and ReflectionHit 48-byte buffers.
    /// Producers must complete before this job; wgpu validates raw device ownership.
    /// # Errors
    /// Rejects format, storage capacities, dimensions and nonfinite settings.
    pub fn prepare_raw(
        &self,
        primary: &wgpu::Buffer,
        hits: &wgpu::Buffer,
        radiance: &wgpu::Texture,
        options: ReflectionSpatialOptions,
    ) -> Result<ReflectionSpatialJob, RaySceneError> {
        let size = radiance.size();
        let count = u64::from(size.width) * u64::from(size.height);
        let limits = self.device.limits();
        if size.width == 0
            || size.height == 0
            || size.depth_or_array_layers != 1
            || radiance.dimension() != wgpu::TextureDimension::D2
            || radiance.sample_count() != 1
            || radiance.mip_level_count() != 1
            || !matches!(
                radiance.format(),
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
            || !radiance
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        if count > u64::from(u32::MAX)
            || size.width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || size.height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(RaySceneError::Capacity);
        }
        for (buffer, stride) in [(primary, 32), (hits, 48)] {
            if buffer.size() != count * stride
                || !buffer.usage().contains(wgpu::BufferUsages::STORAGE)
                || buffer.size() > limits.max_storage_buffer_binding_size
            {
                return Err(RaySceneError::InvalidGeometry);
            }
        }
        let settings = [
            options.normal_cosine,
            options.plane_tolerance,
            options.relative_distance_tolerance,
            options.minimum_roughness,
        ];
        if settings.iter().any(|v| !v.is_finite())
            || !(0.0..=1.0).contains(&settings[0])
            || settings[1] < 0.0
            || !(0.0..=1.0).contains(&settings[2])
            || !(0.0..=1.0).contains(&settings[3])
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spatial reflection thresholds"),
                contents: bytemuck::cast_slice(&settings),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let output = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("spatial rough reflection"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let input = radiance.create_view(&Default::default());
        let view = output.create_view(&Default::default());
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spatial reflection inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: primary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hits.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        Ok(ReflectionSpatialJob {
            pipeline: self.pipeline.clone(),
            bindings,
            output,
        })
    }
}
impl ReflectionSpatialJob {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("spatial rough reflection"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.output.width().div_ceil(8),
            self.output.height().div_ceil(8),
            1,
        );
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
}
