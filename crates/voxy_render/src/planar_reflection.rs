//! Backward reflection UV for perfect planar mirrors using virtual world points.
use crate::RaySceneError;
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, Debug)]
pub struct PlanarReflectionCameras {
    /// Current, previous unjittered world-to-clip matrices, NDC depth in (0,1).
    pub cameras: [glam::Mat4; 2],
    /// Current, previous world planes: unit normal XYZ and signed offset W.
    pub planes: [[f32; 4]; 2],
}
#[derive(Debug)]
pub struct PlanarReflectionPipeline {
    motion_format: wgpu::TextureFormat,
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct PlanarReflectionJob {
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    motion: wgpu::Texture,
    expected_depth: wgpu::Texture,
    current_depth: wgpu::Texture,
}
impl PlanarReflectionPipeline {
    /// # Errors
    /// Rejects insufficient compute/storage limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_storage_textures_per_shader_stage < 3
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
        {
            return Err(RaySceneError::Capacity);
        }
        let motion_format = if device.adapter_info().backend == wgpu::Backend::Gl {
            wgpu::TextureFormat::Rgba32Float
        } else {
            wgpu::TextureFormat::Rg32Float
        };
        let source = if motion_format == wgpu::TextureFormat::Rgba32Float {
            include_str!("planar_reflection.wgsl").replace("rg32float", "rgba32float")
        } else {
            include_str!("planar_reflection.wgsl").to_owned()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("planar reflection virtual-point reprojection"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        Ok(Self {
            motion_format,
            device: device.clone(),
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("planar reflection reprojection"),
                layout: None,
                module: &shader,
                entry_point: Some("cs_main"),
                compilation_options: Default::default(),
                cache: None,
            }),
        })
    }
    /// Prepare directly from the production ray and previous-geometry jobs.
    /// Resource dimensions derive from the ray output; device ownership is checked.
    /// See `prepare` for plane, camera, history and encoding-order requirements.
    /// # Errors
    /// Rejects foreign jobs and propagates geometry/capacity errors.
    pub fn prepare_reflection(
        &self,
        reflected: &crate::SurfaceReflectionJob,
        previous: &crate::ReflectionCorrespondenceJob,
        options: PlanarReflectionCameras,
    ) -> Result<PlanarReflectionJob, RaySceneError> {
        reflected.validate_device(&self.device)?;
        previous.validate_device(&self.device)?;
        self.prepare(
            reflected.hits(),
            previous.positions(),
            [reflected.distance().width(), reflected.distance().height()],
            options,
        )
    }

    /// `hits` contains row-major ReflectionHit records; previous_positions has
    /// matching WORLD XYZ/W validity from ReflectionCorrespondenceJob. Encode
    /// after both producers. All resources and encoder must share this device.
    /// Works for zero-roughness opaque planar mirrors with matching unjittered
    /// primary coverage; curved/rough surfaces require a different reprojection.
    /// Motion/expected depth feed TemporalResolve; commit current virtual depth
    /// via TemporalHistory::encode_depth after resolving reflection radiance.
    /// This reflection history must be separate from primary/direct lighting.
    /// # Errors
    /// Rejects sizes/usages, invalid cameras/nonunit planes and dispatch limits.
    pub fn prepare(
        &self,
        hits: &wgpu::Buffer,
        previous_positions: &wgpu::Buffer,
        dimensions: [u32; 2],
        options: PlanarReflectionCameras,
    ) -> Result<PlanarReflectionJob, RaySceneError> {
        let [width, height] = dimensions;
        let count = u64::from(width) * u64::from(height);
        let limits = self.device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || width.div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(RaySceneError::Capacity);
        }
        for (buffer, stride) in [(hits, 48), (previous_positions, 16)] {
            if buffer.size() != count * stride
                || !buffer.usage().contains(wgpu::BufferUsages::STORAGE)
                || buffer.size() > limits.max_storage_buffer_binding_size
            {
                return Err(RaySceneError::InvalidGeometry);
            }
        }
        if options.cameras.iter().any(|m| {
            !m.is_finite() || !m.determinant().is_finite() || m.determinant().abs() <= f32::EPSILON
        }) || options.planes.iter().any(|p| {
            p.iter().any(|v| !v.is_finite())
                || (p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.0).abs() > 1.0e-4
        }) {
            return Err(RaySceneError::InvalidGeometry);
        }
        let mut words = options
            .cameras
            .iter()
            .flat_map(|m| m.to_cols_array())
            .collect::<Vec<_>>();
        // Normalize tolerated unit-normal roundoff, including signed offset.
        for p in options.planes {
            let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            words.extend(p.map(|v| v / length));
        }
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("planar reflection cameras/planes"),
                contents: bytemuck::cast_slice(&words),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let texture = |format| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("planar reflected temporal guide"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let motion = texture(self.motion_format);
        let expected_depth = texture(wgpu::TextureFormat::R32Float);
        let current_depth = texture(wgpu::TextureFormat::R32Float);
        let views =
            [&motion, &expected_depth, &current_depth].map(|t| t.create_view(&Default::default()));
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("planar reflection guide inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: hits.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: previous_positions.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
            ],
        });
        Ok(PlanarReflectionJob {
            pipeline: self.pipeline.clone(),
            bindings,
            motion,
            expected_depth,
            current_depth,
        })
    }
}
impl PlanarReflectionJob {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("planar reflection temporal reprojection"),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.motion.width().div_ceil(8),
            self.motion.height().div_ceil(8),
            1,
        );
    }
    #[must_use]
    /// Backward UV in XY: RGBA32Float on GL, RG32Float on other backends.
    /// Inspect the format when copying/readback; unused ZW are zero.
    pub fn motion(&self) -> &wgpu::Texture {
        &self.motion
    }
    #[must_use]
    pub fn expected_previous_depth(&self) -> &wgpu::Texture {
        &self.expected_depth
    }
    #[must_use]
    pub fn current_depth(&self) -> &wgpu::Texture {
        &self.current_depth
    }
}
