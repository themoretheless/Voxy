//! Depth-rejected backward-UV reprojection of caller-owned presented radiance.
use crate::ComputeError;
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, Debug)]
pub struct TemporalResolveOptions {
    pub history_weight: f32,
    pub depth_tolerance: f32,
    pub reset_history: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct TemporalResolveInputs<'a> {
    pub current: &'a wgpu::Texture,
    /// Previous UV minus current UV in XY, top-left origin, normalized units.
    /// RGBA32 motion maps are supported for GL storage output; ZW are ignored.
    pub motion: &'a wgpu::Texture,
    pub history: &'a wgpu::Texture,
    /// Previous-camera NDC depth of the corresponding previous surface, at
    /// current-frame coverage. Invalid/background is 0 or 1. This must account
    /// for object deformation; current depth is not a substitute.
    pub expected_previous_depth: &'a wgpu::Texture,
    /// Previous presented NDC depth, with the same convention and camera.
    pub history_depth: &'a wgpu::Texture,
}
#[derive(Debug)]
pub struct TemporalResolve {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct TemporalResolveFrame {
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
}
impl TemporalResolve {
    /// # Errors
    /// Rejects insufficient compute, binding or texture limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        let limits = device.limits();
        if limits.max_compute_invocations_per_workgroup < 64
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_sampled_textures_per_shader_stage < 5
            || limits.max_storage_textures_per_shader_stage < 1
            || limits.max_uniform_buffers_per_shader_stage < 1
        {
            return Err(ComputeError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depth rejected temporal resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("temporal_resolve.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("temporal resolve"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
        })
    }
    /// Nearest-pixel history lookup with depth rejection and an explicit blend.
    /// Every input/encoder must belong to this device. Keep history only after
    /// successful presentation; reset on camera cuts, resize or resource loss.
    /// No variance clipping, denoising, per-pixel sample counts or history commit.
    /// # Errors
    /// Rejects incompatible textures, invalid settings and dispatch dimensions.
    pub fn prepare(
        &self,
        inputs: TemporalResolveInputs<'_>,
        options: TemporalResolveOptions,
    ) -> Result<TemporalResolveFrame, ComputeError> {
        self.prepare_inner(inputs, options, false, None)
    }
    /// Limit valid history to the current 3x3 linear RGB range before blending.
    /// Nonfinite neighbors are ignored; finite RGB is clamped nonnegative.
    /// Bounds never read outside the image.
    /// This suppresses stale color trails but can bias noisy radiance samples.
    /// Same device and presented-history requirements as `prepare` apply.
    /// # Errors
    /// Rejects the same resources/settings as `prepare`.
    pub fn prepare_clipped(
        &self,
        inputs: TemporalResolveInputs<'_>,
        options: TemporalResolveOptions,
    ) -> Result<TemporalResolveFrame, ComputeError> {
        self.prepare_inner(inputs, options, true, None)
    }
    /// Resolve into a retained caller-owned target without allocating a texture.
    /// Target must be a distinct single-mip RGBA32Float storage texture with the input size.
    /// All resources must belong to this device; wgpu validates ownership.
    /// # Errors
    /// Rejects incompatible output, feedback aliases or invalid inputs/settings.
    pub fn prepare_into(
        &self,
        inputs: TemporalResolveInputs<'_>,
        options: TemporalResolveOptions,
        output: &wgpu::Texture,
        clip_history: bool,
    ) -> Result<TemporalResolveFrame, ComputeError> {
        self.prepare_inner(inputs, options, clip_history, Some(output))
    }
    fn prepare_inner(
        &self,
        inputs: TemporalResolveInputs<'_>,
        options: TemporalResolveOptions,
        clip_history: bool,
        retained_output: Option<&wgpu::Texture>,
    ) -> Result<TemporalResolveFrame, ComputeError> {
        let size = validate_inputs(inputs)?;
        if !options.history_weight.is_finite()
            || !(0.0..=1.0).contains(&options.history_weight)
            || !options.depth_tolerance.is_finite()
            || !(0.0..=1.0).contains(&options.depth_tolerance)
        {
            return Err(ComputeError::InvalidDispatch);
        }
        if size.width.div_ceil(8) > self.device.limits().max_compute_workgroups_per_dimension
            || size.height.div_ceil(8) > self.device.limits().max_compute_workgroups_per_dimension
        {
            return Err(ComputeError::InvalidDispatch);
        }
        if let Some(output) = retained_output {
            if output.size() != size
                || output.format() != wgpu::TextureFormat::Rgba32Float
                || output.dimension() != wgpu::TextureDimension::D2
                || output.sample_count() != 1
                || output.mip_level_count() != 1
                || !output
                    .usage()
                    .contains(wgpu::TextureUsages::STORAGE_BINDING)
                || [
                    inputs.current,
                    inputs.motion,
                    inputs.history,
                    inputs.expected_previous_depth,
                    inputs.history_depth,
                ]
                .contains(&output)
            {
                return Err(ComputeError::InvalidBuffer);
            }
        }
        let output = retained_output.cloned().unwrap_or_else(|| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("reprojected HDR"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        });
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("temporal resolve settings"),
                contents: bytemuck::cast_slice(&[
                    options.history_weight,
                    options.depth_tolerance,
                    if options.reset_history { 1.0 } else { 0.0 },
                    if clip_history { 1.0 } else { 0.0 },
                ]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let views = [
            inputs.current,
            inputs.motion,
            inputs.history,
            inputs.expected_previous_depth,
            inputs.history_depth,
            &output,
        ]
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let mut entries = views
            .iter()
            .zip(0..6)
            .map(|(view, binding)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect::<Vec<_>>();
        entries.push(wgpu::BindGroupEntry {
            binding: 6,
            resource: uniform.as_entire_binding(),
        });
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temporal resolve inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        Ok(TemporalResolveFrame {
            pipeline: self.pipeline.clone(),
            bindings,
            output,
        })
    }
}
impl TemporalResolveFrame {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_with_timestamps(encoder, None);
    }
    /// Encode with optional timestamps on this actual compute pass.
    /// The query set must belong to the same device with timestamp support enabled;
    /// indices must be distinct, in range, and not reused before resolution.
    pub fn encode_with_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("temporal resolve"),
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

fn validate_inputs(inputs: TemporalResolveInputs<'_>) -> Result<wgpu::Extent3d, ComputeError> {
    let size = inputs.current.size();
    for (texture, formats) in [
        (
            inputs.current,
            &[
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Rgba32Float,
            ][..],
        ),
        (
            inputs.history,
            &[
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Rgba32Float,
            ][..],
        ),
        (
            inputs.motion,
            &[
                wgpu::TextureFormat::Rg16Float,
                wgpu::TextureFormat::Rg32Float,
                wgpu::TextureFormat::Rgba32Float,
            ][..],
        ),
        (
            inputs.expected_previous_depth,
            &[wgpu::TextureFormat::R16Float, wgpu::TextureFormat::R32Float][..],
        ),
        (
            inputs.history_depth,
            &[wgpu::TextureFormat::R16Float, wgpu::TextureFormat::R32Float][..],
        ),
    ] {
        if texture.size() != size
            || texture.dimension() != wgpu::TextureDimension::D2
            || texture.depth_or_array_layers() != 1
            || texture.sample_count() != 1
            || !texture
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            || !formats.contains(&texture.format())
        {
            return Err(ComputeError::InvalidBuffer);
        }
    }
    Ok(size)
}
