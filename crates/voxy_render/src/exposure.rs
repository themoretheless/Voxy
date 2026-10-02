//! GPU log-average luminance and exposure adaptation without CPU readback.
use crate::ComputeError;
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug)]
pub struct ExposureSettings {
    pub middle_gray: f32,
    /// Positive normal-f32 lower bound; subnormal values are rejected.
    pub minimum: f32,
    pub maximum: f32,
    /// Exponential adaptation rates per second, when exposure increases/decreases.
    pub increase_rate: f32,
    pub decrease_rate: f32,
}
impl Default for ExposureSettings {
    fn default() -> Self {
        Self {
            middle_gray: 0.18,
            minimum: 1.0 / 64.0,
            maximum: 64.0,
            increase_rate: 1.0,
            decrease_rate: 3.0,
        }
    }
}
#[derive(Debug)]
pub struct AutoExposure {
    device: wgpu::Device,
    meter: wgpu::ComputePipeline,
    adapt: wgpu::ComputePipeline,
}
/// Owns a candidate exposure and metering commands. Keep the last successfully
/// presented frame as history; preparing/encoding a skipped frame never commits it.
#[derive(Debug)]
pub struct AutoExposureFrame {
    meter: wgpu::ComputePipeline,
    adapt: wgpu::ComputePipeline,
    meter_bindings: wgpu::BindGroup,
    adapt_bindings: wgpu::BindGroup,
    output: wgpu::Buffer,
    groups: [u32; 2],
    device: wgpu::Device,
}
impl AutoExposure {
    /// # Errors
    /// Rejects devices without the required compute/storage/uniform limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        let limits = device.limits();
        if limits.max_compute_invocations_per_workgroup < 64
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_compute_workgroup_storage_size < 512
            || limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_uniform_buffers_per_shader_stage < 2
            || limits.max_sampled_textures_per_shader_stage < 2
            || limits.max_bindings_per_bind_group < 6
            || limits.max_bind_groups < 1
            || limits.max_uniform_buffer_binding_size < 48
            || limits.max_storage_buffer_binding_size < 16
            || limits.max_buffer_size < 48
        {
            return Err(ComputeError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDR exposure metering"),
            source: wgpu::ShaderSource::Wgsl(include_str!("exposure.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Ok(Self {
            device: device.clone(),
            meter: pipeline("meter"),
            adapt: pipeline("adapt"),
        })
    }
    /// Meter finite linear BT.709 RGB. Alpha is ignored; black/invalid pixels are
    /// excluded. Positive luminance clamps to 1e-6..1e6 before log averaging.
    /// All-black frames retain the previous exposure (one without history).
    /// The source must belong to this device. `previous` must be the last presented
    /// frame from this device; reset with None.
    /// # Errors
    /// Rejects incompatible textures, invalid settings/time or capacity overflow.
    pub fn prepare(
        &self,
        source: &wgpu::Texture,
        previous: Option<&AutoExposureFrame>,
        settings: ExposureSettings,
        seconds: f32,
    ) -> Result<AutoExposureFrame, ComputeError> {
        self.prepare_sources(source, None, previous, settings, seconds)
    }

    /// Meter the union of two equally sized linear eye textures into one shared
    /// exposure. Both eyes must belong to this device; formats may differ.
    /// Commit only after successful stereo presentation and reset on tracking loss.
    /// # Errors
    /// Rejects invalid/mismatched eyes, settings, device or aggregate capacity.
    pub fn prepare_stereo(
        &self,
        eyes: [&wgpu::Texture; 2],
        previous: Option<&AutoExposureFrame>,
        settings: ExposureSettings,
        seconds: f32,
    ) -> Result<AutoExposureFrame, ComputeError> {
        self.prepare_sources(eyes[0], Some(eyes[1]), previous, settings, seconds)
    }

    fn prepare_sources(
        &self,
        source: &wgpu::Texture,
        secondary: Option<&wgpu::Texture>,
        previous: Option<&AutoExposureFrame>,
        settings: ExposureSettings,
        seconds: f32,
    ) -> Result<AutoExposureFrame, ComputeError> {
        let (groups, tiles) = self.validate(source, previous, settings, seconds)?;
        self.validate_secondary(source, secondary, previous, settings, seconds)?;
        let bytes = u64::from(tiles) * 8;
        let partials = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exposure tile sums"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let settings_buffer = self.settings_buffer(
            [source.width(), source.height(), groups[0], tiles],
            settings,
            seconds,
            secondary.is_some(),
            previous.is_some(),
        );
        let fallback = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("initial exposure"),
                contents: bytemuck::cast_slice(&[1.0_f32, 0.0, 0.0, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU exposure uniform"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let second_view = secondary
            .unwrap_or(source)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let meter_bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("exposure meter inputs"),
            layout: &self.meter.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&second_view),
                },
            ],
        });
        let adapt_bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("exposure adaptation inputs"),
            layout: &self.adapt.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: settings_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: previous
                        .map_or(&fallback, |frame| &frame.output)
                        .as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(AutoExposureFrame {
            meter: self.meter.clone(),
            adapt: self.adapt.clone(),
            meter_bindings,
            adapt_bindings,
            output,
            groups,
            device: self.device.clone(),
        })
    }
    fn validate_secondary(
        &self,
        source: &wgpu::Texture,
        secondary: Option<&wgpu::Texture>,
        previous: Option<&AutoExposureFrame>,
        settings: ExposureSettings,
        seconds: f32,
    ) -> Result<(), ComputeError> {
        if let Some(eye) = secondary {
            self.validate(eye, previous, settings, seconds)?;
            if source.size() != eye.size() {
                return Err(ComputeError::InvalidBuffer);
            }
            if source
                .width()
                .checked_mul(source.height())
                .and_then(|n| n.checked_mul(2))
                .is_none()
            {
                return Err(ComputeError::InvalidDispatch);
            }
        }
        Ok(())
    }
    fn settings_buffer(
        &self,
        dimensions: [u32; 4],
        settings: ExposureSettings,
        seconds: f32,
        stereo: bool,
        history: bool,
    ) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("exposure settings"),
                contents: bytemuck::cast_slice(&[
                    dimensions[0],
                    dimensions[1],
                    dimensions[2],
                    dimensions[3],
                    settings.middle_gray.to_bits(),
                    settings.minimum.to_bits(),
                    settings.maximum.to_bits(),
                    seconds.to_bits(),
                    settings.increase_rate.to_bits(),
                    settings.decrease_rate.to_bits(),
                    (if stereo { 2.0_f32 } else { 1.0_f32 }).to_bits(),
                    f32::from(u8::from(history)).to_bits(),
                ]),
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }
    fn validate(
        &self,
        source: &wgpu::Texture,
        previous: Option<&AutoExposureFrame>,
        settings: ExposureSettings,
        seconds: f32,
    ) -> Result<([u32; 2], u32), ComputeError> {
        if previous.is_some_and(|frame| frame.device != self.device) {
            return Err(ComputeError::DeviceMismatch);
        }
        if ![
            settings.middle_gray,
            settings.minimum,
            settings.maximum,
            settings.increase_rate,
            settings.decrease_rate,
            seconds,
        ]
        .into_iter()
        .all(f32::is_finite)
            || settings.middle_gray <= 0.0
            || settings.minimum < f32::MIN_POSITIVE
            || settings.maximum < settings.minimum
            || settings.increase_rate < 0.0
            || settings.decrease_rate < 0.0
            || seconds < 0.0
        {
            return Err(ComputeError::InvalidDispatch);
        }
        if !matches!(
            source.format(),
            wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
        ) || source.dimension() != wgpu::TextureDimension::D2
            || source.depth_or_array_layers() != 1
            || source.sample_count() != 1
            || !source
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let groups = [source.width().div_ceil(8), source.height().div_ceil(8)];
        let tiles = groups[0]
            .checked_mul(groups[1])
            .ok_or(ComputeError::InvalidDispatch)?;
        let bytes = u64::from(tiles) * 8;
        let limits = self.device.limits();
        if source.width().checked_mul(source.height()).is_none()
            || groups
                .into_iter()
                .any(|n| n > limits.max_compute_workgroups_per_dimension)
            || bytes > limits.max_buffer_size
            || bytes > limits.max_storage_buffer_binding_size
        {
            return Err(ComputeError::InvalidDispatch);
        }
        Ok((groups, tiles))
    }
}
impl AutoExposureFrame {
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    /// Encode after the HDR producer and before tone mapping. Both passes must
    /// run in order; resource transitions are handled by wgpu.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (pipeline, bindings, groups) in [
            (&self.meter, &self.meter_bindings, self.groups),
            (&self.adapt, &self.adapt_bindings, [1, 1]),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bindings, &[]);
            pass.dispatch_workgroups(groups[0], groups[1], 1);
        }
    }
    /// The exposure is the first float of a 16-byte GPU uniform. Undefined until encoded.
    #[must_use]
    pub fn output(&self) -> &wgpu::Buffer {
        &self.output
    }
}
