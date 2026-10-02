//! GPU mirror reflection with uniform or per-pixel linear material F0.
use crate::{PrimarySurfaceJob, RayScene, RaySceneError, ReconstructionMaterial};
use wgpu::util::DeviceExt;
/// One row-major reflected opaque triangle correspondence record (48 bytes).
/// Validity is explicit: barycentrics_valid.w = 1 for a hit, 0 for miss/skipped
/// samples. World XYZ and ray distance are in position_distance. Identity is
/// [instance index, instance custom data, geometry index, primitive index].
/// XY barycentrics describe vertices 1/2; vertex 0 has weight 1-X-Y.
/// Instance indices are frame-local; retain stable scene identity separately
/// when matching previous geometry. This record does not itself supply motion.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReflectionHit {
    pub position_distance: [f32; 4],
    pub identity: [u32; 4],
    pub barycentrics_valid: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceReflectionOptions {
    pub material: ReconstructionMaterial,
    pub camera: [f32; 3],
    pub bias: f32,
    pub maximum_distance: f32,
}
#[derive(Debug)]
pub struct SurfaceReflectionJob {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    distance: wgpu::Texture,
    radiance: wgpu::Texture,
    hits: wgpu::Buffer,
}
/// Reusable compiled GPU GGX pipeline. Each job owns immutable seed/camera
/// uniforms and output textures, so multiple jobs can coexist in one submission.
#[derive(Debug)]
pub struct GgxReflectionPipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
/// Matching primary surface, emissive triangles, F0 map and seeded ray settings.
#[derive(Clone, Copy, Debug)]
pub struct GgxReflectionInputs<'a> {
    pub primary: &'a PrimarySurfaceJob,
    pub emission: &'a [[f32; 4]],
    pub options: SurfaceReflectionOptions,
    pub material_f0: &'a wgpu::Texture,
    pub seed: u32,
}
struct ReflectionMode<'a> {
    seed: Option<u32>,
    pipeline: Option<&'a wgpu::ComputePipeline>,
    reuse: Option<(wgpu::Texture, wgpu::Texture)>,
    reuse_hits: Option<wgpu::Buffer>,
}
impl GgxReflectionPipeline {
    /// # Errors
    /// Rejects missing ray-query support and compute/resource capacity.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        validate_capacity(device, 1)?;
        Ok(Self {
            device: device.clone(),
            pipeline: reflection_pipeline(device, true),
        })
    }
    /// Prepare a seeded job without recompiling its shader/pipeline.
    /// `options.material` is ignored in favor of the matching F0/roughness maps.
    /// # Errors
    /// Preserves primary reflection input and capacity validation.
    pub fn create_job(
        &self,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        emission: &[[f32; 4]],
        mut options: SurfaceReflectionOptions,
        material_f0: &wgpu::Texture,
        seed: u32,
    ) -> Result<SurfaceReflectionJob, RaySceneError> {
        validate_material_map(primary, material_f0)?;
        options.material = ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        SurfaceReflectionJob::create(
            &self.device,
            scene,
            primary,
            emission,
            options,
            Some(material_f0),
            ReflectionMode {
                seed: Some(seed),
                pipeline: Some(&self.pipeline),
                reuse: None,
                reuse_hits: None,
            },
        )
    }
    /// Transfer a previous radiance/distance pair into an ordered GGX dispatch.
    /// Resize or format changes replace both outputs; old readers must run first.
    /// # Errors
    /// Rejects foreign jobs and preserves reflection validation errors.
    pub fn create_job_reusing(
        &self,
        scene: &RayScene,
        mut inputs: GgxReflectionInputs<'_>,
        previous: SurfaceReflectionJob,
    ) -> Result<SurfaceReflectionJob, RaySceneError> {
        if previous.device != self.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        validate_material_map(inputs.primary, inputs.material_f0)?;
        inputs.options.material = ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        SurfaceReflectionJob::create(
            &self.device,
            scene,
            inputs.primary,
            inputs.emission,
            inputs.options,
            Some(inputs.material_f0),
            ReflectionMode {
                seed: Some(inputs.seed),
                pipeline: Some(&self.pipeline),
                reuse: Some((previous.distance, previous.radiance)),
                reuse_hits: Some(previous.hits),
            },
        )
    }
}
impl SurfaceReflectionJob {
    /// Reflect valid GPU primary samples and shade nearest emissive opaque triangles.
    /// Currently supports one zero-roughness material and a perspective camera.
    /// All inputs must share this device and world coordinates. Encode after primary
    /// reconstruction and acceleration builds. Invalid/background samples write black
    /// and zero hit distance. Nonzero-roughness pixels are skipped, not approximated.
    /// # Errors
    /// Rejects unsupported ray queries, rough material, camera/range/emission or limits.
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        emission: &[[f32; 4]],
        options: SurfaceReflectionOptions,
    ) -> Result<Self, RaySceneError> {
        Self::create(
            device,
            scene,
            primary,
            emission,
            options,
            None,
            ReflectionMode {
                seed: None,
                pipeline: None,
                reuse: None,
                reuse_hits: None,
            },
        )
    }
    /// Read each primary pixel's linear F0 from a matching material texture.
    /// The texture must come from the same depth/camera/surfaces as `primary`.
    /// Rough pixels remain unsupported. Input alpha zero marks invalid material.
    /// Map RGB must be finite reflectance in [0,1]; invalid pixels write black.
    /// `options.material` only validates the zero-roughness mode in this overload.
    /// # Errors
    /// Preserves `new` validation, and rejects invalid map formats/sizes/usages.
    pub fn with_material_map(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        emission: &[[f32; 4]],
        options: SurfaceReflectionOptions,
        material_f0: &wgpu::Texture,
    ) -> Result<Self, RaySceneError> {
        validate_material_map(primary, material_f0)?;
        Self::create(
            device,
            scene,
            primary,
            emission,
            options,
            Some(material_f0),
            ReflectionMode {
                seed: None,
                pipeline: None,
                reuse: None,
                reuse_hits: None,
            },
        )
    }
    /// GPU NDF-sampled GGX from primary normal/roughness and per-pixel F0.
    /// Outputs `RGBA32Float`. `seed` selects deterministic per-pixel random values.
    /// `options.material` is ignored: the map provides F0 and primary provides roughness.
    /// Roughness below 0.001 uses the delta limit; null samples write black/zero distance.
    /// # Errors
    /// Rejects invalid camera, map, emission, capabilities and capacity.
    pub fn with_ggx_material_map(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        emission: &[[f32; 4]],
        mut options: SurfaceReflectionOptions,
        material_f0: &wgpu::Texture,
        seed: u32,
    ) -> Result<Self, RaySceneError> {
        validate_material_map(primary, material_f0)?;
        options.material = ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        Self::create(
            device,
            scene,
            primary,
            emission,
            options,
            Some(material_f0),
            ReflectionMode {
                seed: Some(seed),
                pipeline: None,
                reuse: None,
                reuse_hits: None,
            },
        )
    }
    fn create(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        emission: &[[f32; 4]],
        options: SurfaceReflectionOptions,
        material_f0: Option<&wgpu::Texture>,
        mode: ReflectionMode<'_>,
    ) -> Result<Self, RaySceneError> {
        primary.validate_device(device)?;
        scene.validate_device(device)?;
        let ReflectionMode {
            seed: ggx_seed,
            pipeline: compiled,
            reuse,
            reuse_hits,
        } = mode;
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RaySceneError::Unsupported);
        }
        let f0 = validate_options(scene, emission, options)?;
        validate_capacity(device, emission.len())?;
        let camera = reflection_camera(device, f0, options, material_f0.is_some(), ggx_seed);
        let emission = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU reflected emission"),
            contents: bytemuck::cast_slice(emission),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let [width, height] = primary.dimensions();
        let hit_bytes = u64::from(width) * u64::from(height) * 48;
        let limits = device.limits();
        if hit_bytes == 0
            || hit_bytes > limits.max_buffer_size
            || hit_bytes > limits.max_storage_buffer_binding_size
        {
            return Err(RaySceneError::Capacity);
        }
        let hits = reuse_hits
            .filter(|buffer| buffer.size() == hit_bytes)
            .unwrap_or_else(|| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("reflected triangle correspondence"),
                    size: hit_bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            });

        let (distance, radiance) = reflection_outputs(
            device,
            [width, height],
            ggx_seed.is_some(),
            reuse,
            material_f0,
        );
        let pipeline = compiled.map_or_else(
            || reflection_pipeline(device, ggx_seed.is_some()),
            Clone::clone,
        );
        let distance_view = distance.create_view(&wgpu::TextureViewDescriptor::default());
        let radiance_view = radiance.create_view(&wgpu::TextureViewDescriptor::default());
        let fallback_map = material_f0
            .is_none()
            .then(|| texture(device, 1, 1, wgpu::TextureFormat::Rgba16Float));
        let material_view = material_f0
            .or(fallback_map.as_ref())
            .expect("material map or fallback")
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU primary mirror inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: primary.output().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: emission.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&distance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&radiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&material_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: hits.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
            bindings,
            distance,
            radiance,
            hits,
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_with_timestamps(encoder, None);
    }
    /// Optional timestamps on the actual compute pass. Queries must belong to the
    /// same device with enabled timestamp support and distinct valid indices.
    /// Resolve queries before reusing their indices.
    pub fn encode_with_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface reflections"),
            timestamp_writes,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.distance.width().div_ceil(8),
            self.distance.height().div_ceil(8),
            1,
        );
    }
    pub(crate) fn validate_device(&self, device: &wgpu::Device) -> Result<(), RaySceneError> {
        if self.device != *device { return Err(RaySceneError::DeviceMismatch); }
        Ok(())
    }
    /// Row-major ReflectionHit records written by this job's GPU dispatch.
    /// Encode readers after this job; match identity against the same scene epoch.
    #[must_use]
    pub fn hits(&self) -> &wgpu::Buffer {
        &self.hits
    }
    #[must_use]
    pub fn distance(&self) -> &wgpu::Texture {
        &self.distance
    }
    #[must_use]
    pub fn radiance(&self) -> &wgpu::Texture {
        &self.radiance
    }
}
fn texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("GPU primary reflected output"),
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
}

fn reflection_pipeline(device: &wgpu::Device, ggx: bool) -> wgpu::ComputePipeline {
    let shader = reflection_shader(device, ggx);
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("primary reflection pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn reflection_shader(device: &wgpu::Device, ggx: bool) -> wgpu::ShaderModule {
    let source = if ggx {
        include_str!("surface_reflection.wgsl").replace("rgba16float", "rgba32float")
    } else {
        include_str!("surface_reflection.wgsl").to_owned()
    };
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("primary reflection shader"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn validate_options(
    scene: &RayScene,
    emission: &[[f32; 4]],
    options: SurfaceReflectionOptions,
) -> Result<[f32; 4], RaySceneError> {
    let f0 = options
        .material
        .mirror_throughput([0.0, 0.0, 1.0], [0.0, 0.0, 1.0])
        .map_err(|_| RaySceneError::InvalidGeometry)?;
    if options.camera.iter().any(|v| !v.is_finite())
        || !options.bias.is_finite()
        || options.bias <= 0.0
        || !options.maximum_distance.is_finite()
        || options.maximum_distance <= options.bias
        || usize::try_from(scene.triangle_count()).ok() != Some(emission.len())
        || emission.iter().any(|v| {
            v[..3]
                .iter()
                .any(|c| !c.is_finite() || !(0.0..=65504.0).contains(c))
        })
    {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(f0)
}

fn validate_material_map(
    primary: &PrimarySurfaceJob,
    map: &wgpu::Texture,
) -> Result<(), RaySceneError> {
    if [map.width(), map.height()] != primary.dimensions()
        || map.dimension() != wgpu::TextureDimension::D2
        || map.depth_or_array_layers() != 1
        || map.sample_count() != 1
        || map.format() != wgpu::TextureFormat::Rgba16Float
        || !map.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING)
    {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(())
}

fn validate_capacity(device: &wgpu::Device, count: usize) -> Result<(), RaySceneError> {
    let limits = device.limits();
    let bytes = u64::try_from(count).map_err(|_| RaySceneError::Capacity)? * 16;
    if bytes > limits.max_buffer_size
        || bytes > limits.max_storage_buffer_binding_size
        || limits.max_compute_workgroup_size_x < 8
        || limits.max_compute_workgroup_size_y < 8
        || limits.max_compute_invocations_per_workgroup < 64
        || limits.max_storage_buffers_per_shader_stage < 3
        || limits.max_storage_textures_per_shader_stage < 2
        || limits.max_sampled_textures_per_shader_stage < 1
    {
        return Err(RaySceneError::Capacity);
    }
    Ok(())
}

fn reflection_outputs(
    device: &wgpu::Device,
    dimensions: [u32; 2],
    ggx: bool,
    reuse: Option<(wgpu::Texture, wgpu::Texture)>,
    material_f0: Option<&wgpu::Texture>,
) -> (wgpu::Texture, wgpu::Texture) {
    let [width, height] = dimensions;
    let format = if ggx {
        wgpu::TextureFormat::Rgba32Float
    } else {
        wgpu::TextureFormat::Rgba16Float
    };
    reuse
        .filter(|(distance, radiance)| {
            [distance.width(), distance.height()] == [width, height]
                && [radiance.width(), radiance.height()] == [width, height]
                && radiance.format() == format
                && material_f0.is_none_or(|map| map != distance && map != radiance)
        })
        .unwrap_or_else(|| {
            (
                texture(device, width, height, wgpu::TextureFormat::R32Float),
                texture(device, width, height, format),
            )
        })
}

fn reflection_camera(
    device: &wgpu::Device,
    f0: [f32; 4],
    options: SurfaceReflectionOptions,
    has_material: bool,
    ggx_seed: Option<u32>,
) -> wgpu::Buffer {
    let uniform = [
        f0,
        [
            options.camera[0],
            options.camera[1],
            options.camera[2],
            options.bias,
        ],
        [
            options.maximum_distance,
            f32::from(has_material),
            f32::from(ggx_seed.is_some()),
            f32::from_bits(ggx_seed.unwrap_or(0)),
        ],
    ];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("GPU mirror camera and F0"),
        contents: bytemuck::cast_slice(&uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}
