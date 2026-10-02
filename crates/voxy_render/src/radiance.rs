//! Combine linear primary lighting and BSDF-weighted reflected radiance before RR.
use crate::RaySceneError;

#[derive(Debug)]
pub struct RadianceComposition {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
/// Reusable wide HDR addition pipeline for independent lighting contributions.
#[derive(Debug)]
pub struct HdrCompositionPipeline {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
impl HdrCompositionPipeline {
    /// # Errors
    /// Rejects insufficient compute and texture binding limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_sampled_textures_per_shader_stage < 2
            || limits.max_storage_textures_per_shader_stage < 1
        {
            return Err(RaySceneError::Capacity);
        }
        Ok(Self {
            device: device.clone(),
            pipeline: composition_pipeline(device, wgpu::TextureFormat::Rgba32Float),
        })
    }
    /// Create independent output/bindings without recompiling the pipeline.
    /// # Errors
    /// Preserves HDR input texture and dispatch capacity validation errors.
    pub fn create_job(
        &self,
        primary: &wgpu::Texture,
        reflected: &wgpu::Texture,
    ) -> Result<RadianceComposition, RaySceneError> {
        RadianceComposition::with_format(
            &self.device,
            primary,
            reflected,
            wgpu::TextureFormat::Rgba32Float,
            Some(&self.pipeline),
            None,
        )
    }
    /// Transfer a previous output into a later, ordered wide HDR composition.
    /// Resize, format changes or aliasing an input allocate replacement storage.
    /// Old output consumers must precede this job on the same queue.
    /// # Errors
    /// Rejects foreign jobs and preserves input/dispatch validation errors.
    pub fn create_job_reusing(
        &self,
        previous: RadianceComposition,
        primary: &wgpu::Texture,
        reflected: &wgpu::Texture,
    ) -> Result<RadianceComposition, RaySceneError> {
        if previous.device != self.device {
            return Err(RaySceneError::DeviceMismatch);
        }
        RadianceComposition::with_format(
            &self.device,
            primary,
            reflected,
            wgpu::TextureFormat::Rgba32Float,
            Some(&self.pipeline),
            Some(previous.output),
        )
    }
}
impl RadianceComposition {
    /// Combine two complete lighting contributions, without exposure or tone mapping.
    /// Inputs must share this device, pixel coordinates and primary surface; RGB must
    /// be finite/nonnegative. Reflection must already include its primary BSDF weight.
    /// RGB sums saturate at binary16 maximum 65504; output alpha is one. Input alpha
    /// is ignored. The output is suitable as an HDR color resource, not a proof that
    /// all lighting terms or temporal RR inputs have been provided.
    /// # Errors
    /// Rejects non-RGBA16, mismatched, layered/multisampled or unsampleable textures,
    /// and devices with insufficient compute/storage limits.
    pub fn new(
        device: &wgpu::Device,
        primary: &wgpu::Texture,
        reflected: &wgpu::Texture,
    ) -> Result<Self, RaySceneError> {
        Self::with_format(
            device,
            primary,
            reflected,
            wgpu::TextureFormat::Rgba16Float,
            None,
            None,
        )
    }
    /// Compose RGBA16/32 contributions into wide HDR `RGBA32Float`.
    /// Finite nonnegative RGB sums saturate at `f32::MAX`; alpha is one.
    /// # Errors
    /// Rejects incompatible inputs and insufficient compute/resource limits.
    pub fn new_hdr(
        device: &wgpu::Device,
        primary: &wgpu::Texture,
        reflected: &wgpu::Texture,
    ) -> Result<Self, RaySceneError> {
        Self::with_format(
            device,
            primary,
            reflected,
            wgpu::TextureFormat::Rgba32Float,
            None,
            None,
        )
    }
    fn with_format(
        device: &wgpu::Device,
        primary: &wgpu::Texture,
        reflected: &wgpu::Texture,
        format: wgpu::TextureFormat,
        compiled: Option<&wgpu::ComputePipeline>,
        reuse: Option<wgpu::Texture>,
    ) -> Result<Self, RaySceneError> {
        let limits = device.limits();
        if limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_sampled_textures_per_shader_stage < 2
            || limits.max_storage_textures_per_shader_stage < 1
            || primary.width().div_ceil(8) > limits.max_compute_workgroups_per_dimension
            || primary.height().div_ceil(8) > limits.max_compute_workgroups_per_dimension
        {
            return Err(RaySceneError::Capacity);
        }
        if primary.size() != reflected.size()
            || [primary, reflected].into_iter().any(|texture| {
                !(texture.format() == wgpu::TextureFormat::Rgba16Float
                    || (format == wgpu::TextureFormat::Rgba32Float && texture.format() == format))
                    || texture.dimension() != wgpu::TextureDimension::D2
                    || texture.depth_or_array_layers() != 1
                    || texture.sample_count() != 1
                    || !texture
                        .usage()
                        .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            })
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        let pipeline = compiled.map_or_else(
            || composition_pipeline(device, format),
            wgpu::ComputePipeline::clone,
        );
        let output = reuse
            .filter(|output| {
                output.size() == primary.size()
                    && output.format() == format
                    && output != primary
                    && output != reflected
            })
            .unwrap_or_else(|| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("RR HDR noisy color"),
                    size: primary.size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
            });
        let views = [primary, reflected, &output]
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("primary and reflected lighting"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
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
    /// Encode after both lighting producers on the same ordered GPU stream.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_with_timestamps(encoder, None);
    }
    /// Optional timing for the actual HDR composition pass. Queries must belong
    /// to this device, have distinct valid indices, and be resolved before reuse.
    pub fn encode_with_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("HDR composition"),
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

fn composition_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HDR lighting composition"),
        source: wgpu::ShaderSource::Wgsl(
            if format == wgpu::TextureFormat::Rgba32Float {
                include_str!("radiance.wgsl")
                    .replace("rgba16float", "rgba32float")
                    .replace("65504.0", "3.402823466e38")
            } else {
                include_str!("radiance.wgsl").to_owned()
            }
            .into(),
        ),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("HDR lighting composition"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}
