//! Point-light Lambertian/GGX radiance and opaque shadow traversal from GPU surfaces.
use crate::{
    HdrCompositionPipeline, PrimarySurfaceJob, RadianceComposition, RayScene, RaySceneError,
};
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, Debug)]
pub struct SurfacePointLight {
    pub position: [f32; 3],
    /// Linear RGB radiant intensity, in [0,65504].
    pub intensity: [f32; 3],
    pub bias: f32,
}
#[derive(Debug)]
pub struct SurfaceLightingJob {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
/// Reusable direct GGX point-light pipeline with immutable per-job inputs.
#[derive(Debug)]
pub struct GgxLightingPipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    composition: HdrCompositionPipeline,
}
/// Multiple independently shadowed point lights summed in wide HDR.
#[derive(Debug)]
pub struct GgxLightingBatch {
    jobs: Vec<SurfaceLightingJob>,
    sums: Vec<RadianceComposition>,
    output: wgpu::Texture,
}
impl GgxLightingBatch {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for job in &self.jobs {
            job.encode(encoder);
        }
        for sum in &self.sums {
            sum.encode(encoder);
        }
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
}
/// Matching primary surfaces, reflectance maps, camera and direct point light.
#[derive(Clone, Copy, Debug)]
pub struct GgxLightingInputs<'a> {
    pub primary: &'a PrimarySurfaceJob,
    pub diffuse: &'a wgpu::Texture,
    pub f0: &'a wgpu::Texture,
    pub camera: [f32; 3],
    pub light: SurfacePointLight,
}
impl GgxLightingPipeline {
    /// Compile once for this device; jobs retain independent light/camera uniforms.
    /// # Errors
    /// Rejects missing ray-query support and insufficient compute/resource limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        validate_capacity(device)?;
        Ok(Self {
            device: device.clone(),
            pipeline: lighting_pipeline(device, true),
            composition: HdrCompositionPipeline::new(device)?,
        })
    }
    /// Prepare direct lighting without recompiling the shader or pipeline.
    /// All resources must share this device and primary world coordinates.
    /// # Errors
    /// Preserves GGX camera, light and texture validation errors.
    pub fn create_job(
        &self,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        diffuse: &wgpu::Texture,
        f0: &wgpu::Texture,
        camera: [f32; 3],
        light: SurfacePointLight,
    ) -> Result<SurfaceLightingJob, RaySceneError> {
        validate_ggx(&self.device, primary, f0, camera, light)?;
        SurfaceLightingJob::create(
            &self.device,
            scene,
            primary,
            diffuse,
            light,
            Some((f0, camera)),
            Some((&self.pipeline, None)),
        )
    }
    /// Transfer a previous direct-light output for a later ordered GGX dispatch.
    /// Size/format changes and input aliasing allocate replacement storage.
    /// Old consumers must precede this dispatch on the same queue.
    /// # Errors
    /// Rejects foreign jobs and preserves light/material validation errors.
    pub fn create_job_reusing(
        &self,
        scene: &RayScene,
        inputs: GgxLightingInputs<'_>,
        previous: SurfaceLightingJob,
    ) -> Result<SurfaceLightingJob, RaySceneError> {
        if previous.device != self.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        validate_ggx(
            &self.device,
            inputs.primary,
            inputs.f0,
            inputs.camera,
            inputs.light,
        )?;
        SurfaceLightingJob::create(
            &self.device,
            scene,
            inputs.primary,
            inputs.diffuse,
            inputs.light,
            Some((inputs.f0, inputs.camera)),
            Some((&self.pipeline, Some(previous.output))),
        )
    }
    /// Prepare independently shadowed lights and their `RGBA32Float` HDR sum.
    /// The compiled lighting pipeline is reused; each light has its own output.
    /// This performs one lighting dispatch per light, then ordered HDR additions.
    /// # Errors
    /// Rejects an empty light list and preserves individual input/limit errors.
    pub fn create_lights(
        &self,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        diffuse: &wgpu::Texture,
        f0: &wgpu::Texture,
        camera: [f32; 3],
        lights: &[SurfacePointLight],
    ) -> Result<GgxLightingBatch, RaySceneError> {
        if lights.is_empty() {
            return Err(RaySceneError::InvalidGeometry);
        }
        let jobs = lights
            .iter()
            .map(|light| self.create_job(scene, primary, diffuse, f0, camera, *light))
            .collect::<Result<Vec<_>, _>>()?;
        let mut output = jobs[0].output().clone();
        let mut sums = Vec::with_capacity(jobs.len() - 1);
        for job in &jobs[1..] {
            let sum = self.composition.create_job(&output, job.output())?;
            output = sum.output().clone();
            sums.push(sum);
        }
        Ok(GgxLightingBatch { jobs, sums, output })
    }
}
impl SurfaceLightingJob {
    /// Shade primary GPU samples with a matching linear diffuse-reflectance texture.
    /// All inputs must share this device and world coordinates. Supports one point
    /// light and opaque shadows. Encode after primary reconstruction and scene build.
    /// Background/invalid samples and light distances <= max(2*bias,1e-10) write black.
    /// Output sums saturate at binary16 maximum; input diffuse alpha is ignored.
    /// # Errors
    /// Rejects unsupported ray queries, light values, map formats/dimensions/usages,
    /// and insufficient compute or resource limits.
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        diffuse: &wgpu::Texture,
        light: SurfacePointLight,
    ) -> Result<Self, RaySceneError> {
        Self::create(device, scene, primary, diffuse, light, None, None)
    }
    /// Add GGX direct specular lighting with a matching F0 map and world camera.
    /// Outputs `RGBA32Float`; primary roughness below 0.001 has no finite GGX lobe.
    /// # Errors
    /// Preserves light/map validation and rejects invalid camera values and F0 texture descriptors.
    /// F0 texels outside [0,1] contribute no specular light.
    pub fn with_ggx(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        diffuse: &wgpu::Texture,
        f0: &wgpu::Texture,
        camera: [f32; 3],
        light: SurfacePointLight,
    ) -> Result<Self, RaySceneError> {
        validate_ggx(device, primary, f0, camera, light)?;
        Self::create(
            device,
            scene,
            primary,
            diffuse,
            light,
            Some((f0, camera)),
            None,
        )
    }

    fn create(
        device: &wgpu::Device,
        scene: &RayScene,
        primary: &PrimarySurfaceJob,
        diffuse: &wgpu::Texture,
        light: SurfacePointLight,
        specular: Option<(&wgpu::Texture, [f32; 3])>,
        compiled: Option<(&wgpu::ComputePipeline, Option<wgpu::Texture>)>,
    ) -> Result<Self, RaySceneError> {
        primary.validate_device(device)?;
        scene.validate_device(device)?;
        validate_inputs(device, primary, diffuse, light)?;
        let data = [
            [
                light.position[0],
                light.position[1],
                light.position[2],
                light.bias,
            ],
            [
                light.intensity[0],
                light.intensity[1],
                light.intensity[2],
                0.0,
            ],
            specular.map_or([0.0; 4], |(_, camera)| {
                [camera[0], camera[1], camera[2], 1.0]
            }),
        ];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primary point light"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let [width, height] = primary.dimensions();
        let (compiled, reuse) =
            compiled.map_or((None, None), |(pipeline, output)| (Some(pipeline), output));
        let format = if specular.is_some() {
            wgpu::TextureFormat::Rgba32Float
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let output = reuse
            .filter(|output| {
                [output.width(), output.height()] == [width, height]
                    && output.format() == format
                    && output != diffuse
                    && specular.is_none_or(|(f0, _)| output != f0)
            })
            .unwrap_or_else(|| light_texture(device, width, height, specular.is_some()));
        let pipeline = compiled.map_or_else(
            || lighting_pipeline(device, specular.is_some()),
            wgpu::ComputePipeline::clone,
        );
        let diffuse_view = diffuse.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let f0_view = specular
            .map_or(diffuse, |(map, _)| map)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU primary light inputs"),
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
                    resource: wgpu::BindingResource::TextureView(&diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&f0_view),
                },
            ],
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
            bindings,
            output,
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
            label: Some("surface direct lighting"),
            timestamp_writes,
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

fn lighting_pipeline(device: &wgpu::Device, wide: bool) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GPU primary shadow lighting"),
        source: wgpu::ShaderSource::Wgsl(
            if wide {
                include_str!("surface_lighting.wgsl")
                    .replace("rgba16float", "rgba32float")
                    .replace("65504.0", "3.402823466e38")
            } else {
                include_str!("surface_lighting.wgsl").to_owned()
            }
            .into(),
        ),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("GPU primary shadow lighting"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn validate_ggx(
    device: &wgpu::Device,
    primary: &PrimarySurfaceJob,
    f0: &wgpu::Texture,
    camera: [f32; 3],
    light: SurfacePointLight,
) -> Result<(), RaySceneError> {
    validate_inputs(device, primary, f0, light)?;
    if camera.iter().any(|v| !v.is_finite()) {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(())
}

fn light_texture(device: &wgpu::Device, width: u32, height: u32, wide: bool) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("GPU primary direct HDR light"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: if wide {
            wgpu::TextureFormat::Rgba32Float
        } else {
            wgpu::TextureFormat::Rgba16Float
        },
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
fn validate_inputs(
    device: &wgpu::Device,
    primary: &PrimarySurfaceJob,
    diffuse: &wgpu::Texture,
    light: SurfacePointLight,
) -> Result<(), RaySceneError> {
    if light.position.iter().any(|v| !v.is_finite())
        || !light.bias.is_finite()
        || light.bias <= 0.0
        || light
            .intensity
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=65504.0).contains(v))
        || [diffuse.width(), diffuse.height()] != primary.dimensions()
        || diffuse.format() != wgpu::TextureFormat::Rgba16Float
        || diffuse.dimension() != wgpu::TextureDimension::D2
        || diffuse.depth_or_array_layers() != 1
        || diffuse.sample_count() != 1
        || !diffuse
            .usage()
            .contains(wgpu::TextureUsages::TEXTURE_BINDING)
    {
        return Err(RaySceneError::InvalidGeometry);
    }
    validate_capacity(device)
}
fn validate_capacity(device: &wgpu::Device) -> Result<(), RaySceneError> {
    if !device
        .features()
        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
    {
        return Err(RaySceneError::Unsupported);
    }
    let limits = device.limits();
    if limits.max_compute_workgroup_size_x < 8
        || limits.max_compute_workgroup_size_y < 8
        || limits.max_compute_invocations_per_workgroup < 64
        || limits.max_storage_buffers_per_shader_stage < 1
        || limits.max_storage_textures_per_shader_stage < 1
        || limits.max_sampled_textures_per_shader_stage < 2
        || limits.max_uniform_buffers_per_shader_stage < 1
        || limits.max_uniform_buffer_binding_size < 48
        || limits.max_bindings_per_bind_group < 6
    {
        return Err(RaySceneError::Capacity);
    }
    Ok(())
}
