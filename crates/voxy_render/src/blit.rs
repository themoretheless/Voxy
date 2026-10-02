//! Nearest full-screen composition for processed color outputs.
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayParameter {
    Identity,
    Exposure,
    WhiteNits,
}
/// Linear single-sample output for native processing and subsequent composition.
#[derive(Debug)]
pub struct ProcessedColorTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    srgb_view: Option<wgpu::TextureView>,
}
impl ProcessedColorTarget {
    /// Allocate linear RGBA8 UNORM (LDR) or RGBA16 float (HDR).
    /// Use `new_with_srgb_view` when the backend supports additional view formats.
    /// Contents must be initialized by the producer before sampling. HDR output
    /// requires a suitable display pass; use `TextureBlit::tone_mapped` for SDR output.
    /// # Errors
    /// Rejects zero dimensions or dimensions exceeding device limits.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        hdr: bool,
    ) -> Result<Self, crate::RendererError> {
        Self::create(device, width, height, hdr, false)
    }
    /// Allocate LDR output with an additional sRGB display view.
    /// Requires adapter `DownlevelFlags::VIEW_FORMATS`; use a separate sRGB render
    /// target on backends without this capability, including OpenGL.
    /// # Errors
    /// Rejects invalid dimensions; wgpu validation reports unsupported view formats.
    pub fn new_with_srgb_view(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Self, crate::RendererError> {
        Self::create(device, width, height, false, true)
    }
    fn create(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        hdr: bool,
        srgb: bool,
    ) -> Result<Self, crate::RendererError> {
        let max_dimension = device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > max_dimension || height > max_dimension {
            return Err(crate::RendererError::InvalidSurfaceSize {
                width,
                height,
                max_dimension,
            });
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("processed color output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if hdr {
                wgpu::TextureFormat::Rgba16Float
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: if srgb {
                &[wgpu::TextureFormat::Rgba8UnormSrgb]
            } else {
                &[]
            },
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let srgb_view = srgb.then(|| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                usage: Some(
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                ..Default::default()
            })
        });
        Ok(Self {
            texture,
            view,
            srgb_view,
        })
    }
    /// Compatible display view for LDR outputs. Render linear color through this
    /// view to apply sRGB encoding once. HDR outputs return `None`.
    #[must_use]
    pub const fn srgb_view(&self) -> Option<&wgpu::TextureView> {
        self.srgb_view.as_ref()
    }
    #[must_use]
    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    #[must_use]
    pub const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

#[derive(Debug)]
pub struct TextureBlit {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: Option<wgpu::Sampler>,
    parameter: Option<wgpu::Buffer>,
    parameter_kind: Option<DisplayParameter>,
    auto_exposure: Option<wgpu::Buffer>,
}
impl TextureBlit {
    /// Visualize normalized depth as grayscale using 24 comparison-sampling steps.
    /// This diagnostic pass works around GLSL raw-depth sampling limitations;
    /// it approximates [0,1] depth and is not a production postprocessing pass.
    #[must_use]
    pub fn depth(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        Self::create_pipeline(
            device,
            target_format,
            &[("TONE_MAPPING", 0.0), ("EXPOSURE", 1.0)],
            None,
            wgpu::TextureSampleType::Depth,
            include_str!("depth_blit.wgsl"),
        )
    }

    /// Create a pass for a single-sample color target. Source must be a float
    /// sampled, single-sample 2D texture view on this device, distinct from target.
    /// Color conversions follow source/target view formats; HDR tone mapping is
    /// not performed. wgpu validation reports invalid formats and resource usage.
    #[must_use]
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        Self::create(device, target_format, false, 1.0)
    }
    /// Apply exposure to linear floating output for scRGB composition. Alpha is
    /// preserved. Positive overflow saturates to the target float maximum; this
    /// pass does not tone-map or encode a transfer function. Finite input required.
    #[must_use]
    pub fn linear_exposed(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        exposure: f32,
    ) -> Option<Self> {
        let maximum = match format {
            wgpu::TextureFormat::Rgba16Float => 65504.0,
            wgpu::TextureFormat::Rgba32Float => f32::MAX,
            _ => return None,
        };
        if !exposure.is_finite() || exposure <= 0.0 {
            return None;
        }
        Some(Self::create_pipeline(
            device,
            format,
            &[
                ("TONE_MAPPING", 0.0),
                ("LINEAR_EXPOSURE", 1.0),
                ("OUTPUT_MAX", f64::from(maximum)),
            ],
            Some((DisplayParameter::Exposure, exposure)),
            wgpu::TextureSampleType::Float { filterable: false },
            include_str!("blit.wgsl"),
        ))
    }

    /// Map linear HDR color to SDR with exposure and the Reinhard curve.
    /// Use an sRGB target view for display encoding, or a linear target for linear
    /// output. Alpha is preserved; negative RGB is clamped to zero. Finite HDR
    /// multiplication overflow saturates toward white. Source must contain finite
    /// linear color.
    /// Returns `None` for nonfinite or nonpositive exposure, before GPU allocation.
    #[must_use]
    pub fn tone_mapped(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        exposure: f32,
    ) -> Option<Self> {
        if !exposure.is_finite() || exposure <= 0.0 {
            return None;
        }
        Some(Self::create(device, target_format, true, exposure))
    }
    /// Encode display-referred linear BT.709 light into BT.2020/PQ for HDR10.
    /// `sdr_white_nits` maps input RGB 1 to absolute luminance, in (0, 10000].
    /// Input must be finite; negative channels clamp to zero and PQ saturates at
    /// 10000 nits. Alpha is preserved (subject to target quantization).
    /// This pass performs no artistic tone mapping or HDR metadata submission.
    /// Present with `SurfaceColorSpace::Bt2100Pq`, never an sRGB view.
    /// Returns `None` for invalid white luminance or incompatible output formats.
    #[must_use]
    pub fn hdr10(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        sdr_white_nits: f32,
    ) -> Option<Self> {
        if !sdr_white_nits.is_finite()
            || sdr_white_nits <= 0.0
            || sdr_white_nits > 10000.0
            || !matches!(
                target_format,
                wgpu::TextureFormat::Rgb10a2Unorm
                    | wgpu::TextureFormat::Rgba16Float
                    | wgpu::TextureFormat::Rgba32Float
            )
        {
            return None;
        }
        Some(Self::create_pipeline(
            device,
            target_format,
            &[],
            Some((DisplayParameter::WhiteNits, sdr_white_nits)),
            wgpu::TextureSampleType::Float { filterable: false },
            include_str!("hdr10.wgsl"),
        ))
    }

    /// Reuse the compiled color pipeline with an independent exposure snapshot.
    /// Separate passes can be encoded before one submission without overwriting
    /// each other's uniforms. A foreign device is rejected before allocating a snapshot.
    /// Returns `None` for passes without explicit exposure control or invalid exposure.
    #[must_use]
    pub fn with_exposure(&self, device: &wgpu::Device, exposure: f32) -> Option<Self> {
        if &self.device != device
            || self.parameter_kind != Some(DisplayParameter::Exposure)
            || !exposure.is_finite()
            || exposure <= 0.0
        {
            return None;
        }
        Some(self.with_parameter(device, exposure))
    }

    /// Reuse a PQ pipeline with an independent absolute-white snapshot.
    /// Returns `None` for non-PQ passes or white luminance outside (0,10000].
    /// `device` must be the original pipeline's device.
    #[must_use]
    pub fn with_sdr_white_nits(&self, device: &wgpu::Device, nits: f32) -> Option<Self> {
        if &self.device != device
            || self.parameter_kind != Some(DisplayParameter::WhiteNits)
            || !nits.is_finite()
            || nits <= 0.0
            || nits > 10000.0
        {
            return None;
        }
        Some(self.with_parameter(device, nits))
    }

    /// Bind a GPU-computed exposure without readback or recompilation. Encode the
    /// exposure frame before this pass, on the pipeline's original device.
    /// Supports SDR tone mapping, exposed linear output and PQ encoding; white
    /// luminance remains independent from the computed exposure.
    #[must_use]
    pub fn with_auto_exposure(&self, frame: &crate::AutoExposureFrame) -> Option<Self> {
        if !frame.belongs_to(&self.device)
            || !matches!(
                self.parameter_kind,
                Some(DisplayParameter::Exposure | DisplayParameter::WhiteNits)
            )
        {
            return None;
        }
        Some(Self {
            device: self.device.clone(),
            pipeline: self.pipeline.clone(),
            layout: self.layout.clone(),
            sampler: None,
            parameter: self.parameter.clone(),
            parameter_kind: self.parameter_kind,
            auto_exposure: Some(frame.output().clone()),
        })
    }

    fn with_parameter(&self, device: &wgpu::Device, value: f32) -> Self {
        Self {
            device: self.device.clone(),
            pipeline: self.pipeline.clone(),
            layout: self.layout.clone(),
            sampler: None,
            parameter: Some(display_parameter_buffer(device, value)),
            parameter_kind: self.parameter_kind,
            auto_exposure: self.auto_exposure.clone(),
        }
    }

    fn create(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        tone_mapping: bool,
        exposure: f32,
    ) -> Self {
        Self::create_pipeline(
            device,
            target_format,
            &[("TONE_MAPPING", f64::from(u8::from(tone_mapping)))],
            Some((
                if tone_mapping {
                    DisplayParameter::Exposure
                } else {
                    DisplayParameter::Identity
                },
                exposure,
            )),
            wgpu::TextureSampleType::Float { filterable: false },
            include_str!("blit.wgsl"),
        )
    }
    fn create_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        constants: &[(&str, f64)],
        parameter_value: Option<(DisplayParameter, f32)>,
        sample_type: wgpu::TextureSampleType,
        source: &'static str,
    ) -> Self {
        let sampler = (sample_type == wgpu::TextureSampleType::Depth).then(|| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            })
        });
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }];
        if sampler.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            });
        }
        let parameter = parameter_value.map(|(_, value)| display_parameter_buffer(device, value));
        if parameter.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            });
        }
        let auto_exposure = parameter_value.map(|_| display_parameter_buffer(device, 1.0));
        if auto_exposure.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("processed color blit"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("processed color blit"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("processed color blit"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("processed color blit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            device: device.clone(),
            pipeline,
            layout,
            sampler,
            parameter,
            parameter_kind: parameter_value.map(|(kind, _)| kind),
            auto_exposure,
        }
    }
    /// Replace target contents with source color, scaling by nearest sampling.
    /// # Panics
    /// Panics before encoding if the device differs from the pipeline owner.
    /// Use encode_checked for recoverable device-owner validation.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
    ) {
        self.encode_checked(device, encoder, source, target)
            .expect("display pipeline device mismatch");
    }
    /// Encode display composition, rejecting a foreign pipeline device before allocation.
    /// Source, target and encoder must also belong to the supplied device.
    /// # Errors
    /// Returns DeviceMismatch when the supplied device differs from the pipeline owner.
    pub fn encode_checked(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
    ) -> Result<(), crate::SceneError> {
        if &self.device != device {
            return Err(crate::SceneError::DeviceMismatch);
        }
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(source),
        }];
        if let Some(sampler) = &self.sampler {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
        }
        if let Some(parameter) = &self.parameter {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: parameter.as_entire_binding(),
            });
        }
        if let Some(exposure) = &self.auto_exposure {
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: exposure.as_entire_binding(),
            });
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("processed color blit"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("processed color composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

fn display_parameter_buffer(device: &wgpu::Device, value: f32) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("display parameter snapshot"),
        contents: bytemuck::cast_slice(&[value, 0.0_f32, 0.0, 0.0]),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}
