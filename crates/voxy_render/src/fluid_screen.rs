//! Screen-space optical reconstruction of liquid spheres and owned film prisms.
//! Simulation remains owned by the caller. Optical thickness is a ray integral,
//! not another liquid mass reservoir. Single-sample perspective and orthographic rendering.
use wgpu::util::DeviceExt;

/// World-space centre/radius and SI absorption coefficients with dielectric IOR.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FluidRenderParticle {
    pub position_radius: [f32; 4],
    pub absorption_ior: [f32; 4],
}

/// An owned film cell represented by its substrate triangle and normal thickness.
/// Positions and thickness use world units; absorption is per metre. This is a
/// triangular prism, not a spherical particle or a second simulation inventory.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FluidRenderFilmTriangle {
    pub(crate) a_thickness: [f32; 4],
    pub(crate) b: [f32; 4],
    pub(crate) c: [f32; 4],
    pub(crate) absorption_ior: [f32; 4],
}

impl FluidRenderFilmTriangle {
    /// Winding selects the outward extrusion normal. Dry cells should be omitted.
    /// # Errors
    /// Rejects nonfinite geometry, nonpositive thickness, degenerate or
    /// unrepresentable f32 normals, and invalid optical material.
    pub fn new(
        points: [[f32; 3]; 3],
        thickness: f32,
        absorption_ior: [f32; 4],
    ) -> Result<Self, &'static str> {
        let [a, b, c] = points.map(glam::Vec3::from_array);
        let normal = (b - a).cross(c - a);
        let norm = normal.length();
        if points
            .iter()
            .flatten()
            .chain(absorption_ior.iter())
            .any(|x| !x.is_finite())
            || !thickness.is_finite()
            || thickness <= 0.
            || !norm.is_finite()
            || norm == 0.
            || absorption_ior[..3].iter().any(|x| *x < 0.)
            || absorption_ior[3] < 1.
            || points
                .iter()
                .any(|p| !(glam::Vec3::from_array(*p) + normal / norm * thickness).is_finite())
        {
            return Err("invalid fluid film geometry, thickness or material");
        }
        Ok(Self {
            a_thickness: [a.x, a.y, a.z, thickness],
            b: [b.x, b.y, b.z, 0.],
            c: [c.x, c.y, c.z, 0.],
            absorption_ior,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FluidDepthFilter {
    None,
    #[default]
    Bilateral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidDiagnostic {
    Depth,
    Thickness,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    inverse_projection: [[f32; 4]; 4],
    inverse_view: [[f32; 4]; 4],
    // width, height, world units per metre, flags: filter bit 0 / orthographic bit 1
    viewport: [f32; 4],
    // near, far, filter max pixels, radius-relative range
    controls: [f32; 4],
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Owns particle and film storage, scene background and fluid reconstruction targets.
/// `encode` produces final colour; it does not supply temporal motion vectors or
/// write a combined scene/fluid depth for subsequent opaque draws.
#[derive(Debug)]
pub struct ScreenSpaceFluidRenderer {
    size: [u32; 2],
    allocation_bytes: u64,
    capacity: usize,
    count: u32,
    camera: wgpu::Buffer,
    particles: wgpu::Buffer,
    films: wgpu::Buffer,
    film_count: u32,
    particle_group: wgpu::BindGroup,
    filter_groups: [wgpu::BindGroup; 3],
    composite_group: wgpu::BindGroup,
    depth_pipeline: wgpu::RenderPipeline,
    thickness_pipeline: wgpu::RenderPipeline,
    film_depth_pipeline: wgpu::RenderPipeline,
    film_thickness_pipeline: wgpu::RenderPipeline,
    filter_pipelines: [wgpu::RenderPipeline; 2],
    composite_pipeline: wgpu::RenderPipeline,
    diagnostics: [wgpu::RenderPipeline; 2],
    background: Target,
    scene_depth: Target,
    particle_depth: Target,
    raw_depth: Target,
    ping_depth: Target,
    smooth_depth: Target,
    material: Target,
    thickness: Target,
    optical_depth: Target,
}

// Check enabled device limits, not merely what its adapter advertises.
// Geometry arrives through instance vertex buffers. Composition binds six
// textures, and the nearest-depth pass writes two 8-byte targets.
fn validate_device_limits(
    limits: &wgpu::Limits,
    width: u32,
    height: u32,
    capacity: usize,
) -> Result<(u64, u64), &'static str> {
    if limits.max_bind_groups < 1
        || limits.max_bindings_per_bind_group < 8
        || limits.max_sampled_textures_per_shader_stage < 6
        || limits.max_samplers_per_shader_stage < 1
        || limits.max_uniform_buffers_per_shader_stage < 1
        || u64::from(limits.max_uniform_buffer_binding_size) < size_of::<CameraUniform>() as u64
    {
        return Err("fluid renderer device binding limits are insufficient");
    }
    if limits.max_vertex_buffers < 1
        || limits.max_vertex_attributes < 4
        || limits.max_vertex_buffer_array_stride < size_of::<FluidRenderFilmTriangle>() as u32
        || limits.max_inter_stage_shader_variables < 4
    {
        return Err("fluid renderer device vertex limits are insufficient");
    }
    if limits.max_color_attachments < 2 || limits.max_color_attachment_bytes_per_sample < 16 {
        return Err("fluid renderer device attachment limits are insufficient");
    }
    let particles = capacity
        .checked_mul(size_of::<FluidRenderParticle>())
        .ok_or("fluid capacity overflow")? as u64;
    let films = capacity
        .checked_mul(size_of::<FluidRenderFilmTriangle>())
        .ok_or("fluid film capacity overflow")? as u64;
    if width == 0
        || height == 0
        || width > limits.max_texture_dimension_2d
        || height > limits.max_texture_dimension_2d
        || capacity == 0
        || capacity > u32::MAX as usize
        || particles > limits.max_buffer_size
        || films > limits.max_buffer_size
        || size_of::<CameraUniform>() as u64 > limits.max_buffer_size
    {
        return Err("invalid fluid target or buffer capacity for device limits");
    }
    Ok((particles, films))
}

fn validate_float_output(
    features: wgpu::Features,
    output: wgpu::TextureFormat,
) -> Result<(), &'static str> {
    if !matches!(
        output.sample_type(None, Some(features)),
        Some(wgpu::TextureSampleType::Float { .. })
    ) {
        return Err("fluid renderer output must be a float color format");
    }
    Ok(())
}

fn validate_formats(
    features: wgpu::Features,
    output: wgpu::TextureFormat,
    mut query: impl FnMut(wgpu::TextureFormat) -> wgpu::TextureFormatFeatures,
) -> Result<(), &'static str> {
    validate_float_output(features, output)?;
    let sampled = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    for (format, usage, blended) in [
        // SceneRenderer alpha-blends the background pass into this format.
        (output, sampled, true),
        (wgpu::TextureFormat::Depth32Float, sampled, false),
        (
            wgpu::TextureFormat::Rg32Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
            false,
        ),
        (
            wgpu::TextureFormat::R16Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
            true,
        ),
        (
            wgpu::TextureFormat::Rgba16Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
            true,
        ),
    ] {
        let mut support = query(format);
        if !features.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES) {
            let guaranteed = format.guaranteed_format_features(features);
            support.allowed_usages &= guaranteed.allowed_usages;
            support.flags &= guaranteed.flags;
        }
        if !support.allowed_usages.contains(usage) {
            return Err(
                "fluid renderer texture usages unsupported by adapter or enabled device features",
            );
        }
        if blended
            && !support
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::BLENDABLE)
        {
            return Err("fluid optical targets require blendable floating-point formats");
        }
    }
    Ok(())
}

impl ScreenSpaceFluidRenderer {
    /// Logical texture texels and buffer bytes, excluding driver allocation padding
    /// and opaque pipeline/bind-group storage. No GPU resources are created.
    /// # Errors
    /// Empty targets/capacity, compressed/non-color formats or integer overflow.
    pub fn required_allocation_bytes(
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        capacity: usize,
    ) -> Result<u64, &'static str> {
        if width == 0
            || height == 0
            || capacity == 0
            || format.block_dimensions() != (1, 1)
            || !matches!(
                format.sample_type(None, None),
                Some(wgpu::TextureSampleType::Float { .. })
            )
        {
            return Err("invalid fluid allocation estimate");
        }
        let color = u64::from(
            format
                .block_copy_size(None)
                .ok_or("unsupported fluid color byte size")?,
        );
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or("fluid texture size overflow")?;
        let targets = pixels
            .checked_mul(color.checked_add(50).ok_or("fluid byte overflow")?)
            .ok_or("fluid texture byte overflow")?;
        let buffers = u64::try_from(capacity)
            .map_err(|_| "fluid capacity overflow")?
            .checked_mul(
                (size_of::<FluidRenderParticle>() + size_of::<FluidRenderFilmTriangle>()) as u64,
            )
            .ok_or("fluid buffer byte overflow")?;
        targets
            .checked_add(buffers)
            .and_then(|n| n.checked_add(size_of::<CameraUniform>() as u64))
            .ok_or("fluid total byte overflow")
    }

    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.allocation_bytes
    }

    /// Admit old and new resources together before creating a replacement.
    /// `other_live_bytes` must include any renderer retained during replacement.
    /// # Errors
    /// Budget overflow, unsupported adapter formats or device limits.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_adapter_budget(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        capacity: usize,
        other_live_bytes: u64,
        budget: u64,
    ) -> Result<Self, &'static str> {
        let required = Self::required_allocation_bytes(format, width, height, capacity)?;
        if other_live_bytes
            .checked_add(required)
            .is_none_or(|n| n > budget)
        {
            return Err("fluid GPU resource budget exceeded");
        }
        Self::new_with_adapter(device, adapter, format, width, height, capacity)
    }

    /// Checks actual format capabilities before creating GPU resources.
    /// Pass the adapter which created this device. Metadata mismatch is rejected;
    /// equal metadata alone is not proof of physical adapter identity.
    /// # Errors
    /// Rejects incompatible output/target formats, blending or device limits.
    pub fn new_with_adapter(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        capacity: usize,
    ) -> Result<Self, &'static str> {
        if device.adapter_info() != adapter.get_info() {
            return Err("fluid renderer adapter metadata differs from device");
        }
        validate_formats(device.features(), format, |f| {
            adapter.get_texture_format_features(f)
        })?;
        Self::new(device, format, width, height, capacity)
    }

    /// Allocates fixed particle and film-cell budgets (`capacity` each) and resolution.
    /// Recreate on resize/device recovery. Prefer `new_with_adapter` for format preflight.
    /// # Errors
    /// Rejects empty/excessive targets and particle buffers exceeding device limits.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        capacity: usize,
    ) -> Result<Self, &'static str> {
        validate_float_output(device.features(), format)?;
        let allocation_bytes = Self::required_allocation_bytes(format, width, height, capacity)?;
        let (bytes, film_bytes) =
            validate_device_limits(&device.limits(), width, height, capacity)?;
        let make_target = |label, format, extra| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | extra,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            Target { texture, view }
        };
        let sampled = wgpu::TextureUsages::TEXTURE_BINDING;
        let background = make_target("fluid scene colour", format, sampled);
        let scene_depth = make_target(
            "fluid scene occlusion",
            wgpu::TextureFormat::Depth32Float,
            sampled,
        );
        let particle_depth = make_target(
            "fluid nearest sphere z",
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::empty(),
        );
        let raw_depth = make_target(
            "fluid linear depth and radius",
            wgpu::TextureFormat::Rg32Float,
            sampled,
        );
        let ping_depth = make_target(
            "fluid depth filter ping",
            wgpu::TextureFormat::Rg32Float,
            sampled,
        );
        let smooth_depth = make_target(
            "fluid depth filter result",
            wgpu::TextureFormat::Rg32Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
        );
        let material = make_target(
            "fluid nearest optical material",
            wgpu::TextureFormat::Rgba16Float,
            sampled,
        );
        let thickness = make_target(
            "fluid optical path in metres",
            wgpu::TextureFormat::R16Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
        );
        let optical_depth = make_target(
            "fluid integrated RGB optical depth",
            wgpu::TextureFormat::Rgba16Float,
            sampled | wgpu::TextureUsages::COPY_SRC,
        );
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fluid camera"),
            contents: bytemuck::bytes_of(&<CameraUniform as bytemuck::Zeroable>::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let particles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physical liquid render samples"),
            size: bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let films = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physical liquid film cells"),
            size: film_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture_entry = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let uniform = |binding| {
            buffer_entry(
                binding,
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                wgpu::BufferBindingType::Uniform,
            )
        };
        let float_texture = |binding| {
            texture_entry(
                binding,
                wgpu::TextureSampleType::Float { filterable: false },
            )
        };
        let depth_sampler = crate::depth_sample::sampler(device);
        let depth_sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 7,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(crate::depth_sample::sampler_binding_type(device)),
            count: None,
        };
        let particle_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid particles layout"),
            entries: &[
                uniform(0),
                texture_entry(2, wgpu::TextureSampleType::Depth),
                depth_sampler_entry.clone(),
            ],
        });
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid depth filter layout"),
            entries: &[uniform(0), float_texture(1)],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid composite layout"),
            entries: &[
                uniform(0),
                float_texture(1),
                float_texture(2),
                float_texture(3),
                float_texture(4),
                texture_entry(5, wgpu::TextureSampleType::Depth),
                float_texture(6),
                depth_sampler_entry,
            ],
        });
        let camera_entry = wgpu::BindGroupEntry {
            binding: 0,
            resource: camera.as_entire_binding(),
        };
        let texture_binding = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let particle_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid physical particles"),
            layout: &particle_layout,
            entries: &[
                camera_entry.clone(),
                texture_binding(2, &scene_depth.view),
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
            ],
        });
        let filter_groups = [&raw_depth.view, &ping_depth.view, &smooth_depth.view].map(|view| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fluid filter input"),
                layout: &filter_layout,
                entries: &[camera_entry.clone(), texture_binding(1, view)],
            })
        });
        let composite_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid optical inputs"),
            layout: &composite_layout,
            entries: &[
                camera_entry,
                texture_binding(1, &smooth_depth.view),
                texture_binding(2, &thickness.view),
                texture_binding(3, &material.view),
                texture_binding(4, &background.view),
                texture_binding(5, &scene_depth.view),
                texture_binding(6, &optical_depth.view),
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen-space physical liquid"),
            source: wgpu::ShaderSource::Wgsl(crate::depth_sample::shader(
                device,
                include_str!("fluid_screen.wgsl"),
            )),
        });
        let filter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid filter"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fluid_filter.wgsl").into()),
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid composition"),
            source: wgpu::ShaderSource::Wgsl(crate::depth_sample::shader(
                device,
                concat!(
                    include_str!("dielectric_boundary.wgsl"),
                    "\n",
                    include_str!("fluid_composite.wgsl")
                ),
            )),
        });
        let particle_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let film_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
        let particle_vertex = wgpu::VertexBufferLayout {
            array_stride: size_of::<FluidRenderParticle>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &particle_attributes,
        };
        let film_vertex = wgpu::VertexBufferLayout {
            array_stride: size_of::<FluidRenderFilmTriangle>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &film_attributes,
        };
        let pipeline = |shader: &wgpu::ShaderModule,
                        label,
                        group_layout,
                        vs,
                        fs,
                        targets: &[Option<wgpu::ColorTargetState>],
                        depth| {
            let vertex_buffers = match vs {
                "vs_particle" => vec![Some(particle_vertex.clone())],
                "vs_film" => vec![Some(film_vertex.clone())],
                _ => vec![],
            };
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(group_layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets,
                }),
                primitive: Default::default(),
                depth_stencil: depth,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let color_target = |format, blend| {
            Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let depth_pipeline = pipeline(
            &shader,
            "fluid sphere depth",
            &particle_layout,
            "vs_particle",
            "fs_depth",
            &[
                color_target(wgpu::TextureFormat::Rg32Float, None),
                color_target(wgpu::TextureFormat::Rgba16Float, None),
            ],
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        };
        let thickness_pipeline = pipeline(
            &shader,
            "fluid ray thickness",
            &particle_layout,
            "vs_particle",
            "fs_thickness",
            &[
                color_target(wgpu::TextureFormat::R16Float, Some(additive)),
                color_target(wgpu::TextureFormat::Rgba16Float, Some(additive)),
            ],
            None,
        );
        let film_depth_pipeline = pipeline(
            &shader,
            "fluid film depth",
            &particle_layout,
            "vs_film",
            "fs_film_depth",
            &[
                color_target(wgpu::TextureFormat::Rg32Float, None),
                color_target(wgpu::TextureFormat::Rgba16Float, None),
            ],
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let film_thickness_pipeline = pipeline(
            &shader,
            "fluid film thickness",
            &particle_layout,
            "vs_film",
            "fs_film_thickness",
            &[
                color_target(wgpu::TextureFormat::R16Float, Some(additive)),
                color_target(wgpu::TextureFormat::Rgba16Float, Some(additive)),
            ],
            None,
        );
        let filter_pipelines = ["fs_filter_x", "fs_filter_y"].map(|fs| {
            pipeline(
                &filter_shader,
                "fluid depth filter",
                &filter_layout,
                "vs_fullscreen",
                fs,
                &[color_target(wgpu::TextureFormat::Rg32Float, None)],
                None,
            )
        });
        let composite_pipeline = pipeline(
            &composite_shader,
            "fluid dielectric composition",
            &composite_layout,
            "vs_fullscreen",
            "fs_composite",
            &[color_target(format, None)],
            None,
        );
        let diagnostics = ["fs_depth_debug", "fs_thickness_debug"].map(|fs| {
            pipeline(
                &composite_shader,
                "fluid diagnostic",
                &composite_layout,
                "vs_fullscreen",
                fs,
                &[color_target(format, None)],
                None,
            )
        });
        Ok(Self {
            size: [width, height],
            allocation_bytes,
            capacity,
            count: 0,
            camera,
            particles,
            films,
            film_count: 0,
            particle_group,
            filter_groups,
            composite_group,
            depth_pipeline,
            thickness_pipeline,
            film_depth_pipeline,
            film_thickness_pipeline,
            filter_pipelines,
            composite_pipeline,
            diagnostics,
            background,
            scene_depth,
            particle_depth,
            raw_depth,
            ping_depth,
            smooth_depth,
            material,
            thickness,
            optical_depth,
        })
    }

    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Updates a snapshot without changing caller simulation or particle data.
    /// All validation precedes GPU writes; a rejected update preserves the prior snapshot.
    /// # Errors
    /// Rejects invalid views, SI materials/particles and capacity overflow.
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        camera: crate::SceneCamera,
        particles: &[FluidRenderParticle],
        units_per_metre: f32,
        filter: FluidDepthFilter,
    ) -> Result<(), &'static str> {
        self.update_with_film(queue, camera, particles, &[], units_per_metre, filter)
    }

    /// Uploads particles and physical triangular film prisms as one snapshot.
    /// # Errors
    /// Rejects invalid cells, camera, materials or capacity before any GPU write.
    pub fn update_with_film(
        &mut self,
        queue: &wgpu::Queue,
        camera: crate::SceneCamera,
        particles: &[FluidRenderParticle],
        films: &[FluidRenderFilmTriangle],
        units_per_metre: f32,
        filter: FluidDepthFilter,
    ) -> Result<(), &'static str> {
        if films.len() > self.capacity
            || films.iter().any(|cell| {
                FluidRenderFilmTriangle::new(
                    [
                        cell.a_thickness[..3].try_into().unwrap(),
                        cell.b[..3].try_into().unwrap(),
                        cell.c[..3].try_into().unwrap(),
                    ],
                    cell.a_thickness[3],
                    cell.absorption_ior,
                )
                .is_err()
            })
        {
            return Err("invalid fluid film cells or capacity");
        }
        let (near, far) = match camera.projection {
            crate::SceneProjection::Perspective { near, far, .. }
            | crate::SceneProjection::Orthographic { near, far, .. } => (near, far),
        };
        let vp = camera
            .view_projection()
            .map_err(|_| "invalid fluid camera")?;
        if particles.len() > self.capacity
            || !units_per_metre.is_finite()
            || units_per_metre <= 0.0
            || particles.iter().any(|p| {
                p.position_radius
                    .iter()
                    .chain(p.absorption_ior.iter())
                    .any(|x| !x.is_finite())
                    || p.position_radius[3] <= 0.0
                    || p.absorption_ior[..3].iter().any(|x| *x < 0.0)
                    || p.absorption_ior[3] < 1.0
            })
        {
            return Err("invalid fluid particles, material or scene scale");
        }
        let view = glam::camera::rh::view::look_at_mat4(camera.eye, camera.target, camera.up);
        let projection = vp * view.inverse();
        let uniform = CameraUniform {
            view: view.to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
            inverse_projection: projection.inverse().to_cols_array_2d(),
            inverse_view: view.inverse().to_cols_array_2d(),
            viewport: [
                self.size[0] as f32,
                self.size[1] as f32,
                units_per_metre,
                match filter {
                    FluidDepthFilter::None => 0.0,
                    FluidDepthFilter::Bilateral => 1.0,
                } + if matches!(
                    camera.projection,
                    crate::SceneProjection::Orthographic { .. }
                ) {
                    2.0
                } else {
                    0.0
                },
            ],
            controls: [near, far, 8.0, 1.5],
        };
        queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&uniform));
        if !particles.is_empty() {
            queue.write_buffer(&self.particles, 0, bytemuck::cast_slice(particles));
        }
        if !films.is_empty() {
            queue.write_buffer(&self.films, 0, bytemuck::cast_slice(films));
        }
        self.film_count = films.len() as u32;
        self.count = particles.len() as u32;
        Ok(())
    }

    /// Render the opaque world, nearest liquid depth, thickness, two bilateral
    /// sweeps, final dielectric colour, then UI. Draws must use the update camera.
    pub fn encode(
        &self,
        renderer: &crate::SceneRenderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[crate::SceneDraw<'_>],
    ) {
        self.encode_region(renderer, encoder, output, clear, draws, None);
    }

    /// Compose this view into an existing full-window target without clearing neighbours.
    /// Local overlays use the caller's full-window depth attachment.
    /// # Errors
    /// Empty/overflowing/out-of-bounds regions or size mismatch; rejected before encoding.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_viewport(
        &self,
        renderer: &crate::SceneRenderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        target_size: [u32; 2],
        viewport: [u32; 4],
        clear: wgpu::Color,
        draws: &[crate::SceneDraw<'_>],
    ) -> Result<(), &'static str> {
        if viewport[2..] != self.size
            || viewport[2] == 0
            || viewport[3] == 0
            || viewport[0]
                .checked_add(viewport[2])
                .is_none_or(|v| v > target_size[0])
            || viewport[1]
                .checked_add(viewport[3])
                .is_none_or(|v| v > target_size[1])
        {
            return Err("invalid fluid composition viewport");
        }
        self.encode_region(renderer, encoder, output, clear, draws, Some(viewport));
        renderer.encode_overlays_viewport(encoder, output, depth, draws, viewport);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_region(
        &self,
        renderer: &crate::SceneRenderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[crate::SceneDraw<'_>],
        viewport: Option<[u32; 4]>,
    ) {
        let world: Vec<_> = draws
            .iter()
            .filter(|d| !d.overlay)
            .map(|d| crate::SceneDraw {
                geometry: d.geometry,
                texture: d.texture,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        renderer.encode(
            encoder,
            &self.background.view,
            &self.scene_depth.view,
            clear,
            &world,
        );
        {
            let attachments = [&self.raw_depth.view, &self.material.view].map(|view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fluid physical sphere and film surfaces"),
                color_attachments: &attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.particle_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.depth_pipeline);
            pass.set_bind_group(0, &self.particle_group, &[]);
            pass.set_vertex_buffer(0, self.particles.slice(..));
            pass.draw(0..6, 0..self.count);
            pass.set_pipeline(&self.film_depth_pipeline);
            pass.set_vertex_buffer(0, self.films.slice(..));
            pass.draw(0..6, 0..self.film_count);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fluid particle and film ray thickness"),
                color_attachments: &[&self.thickness.view, &self.optical_depth.view].map(|view| {
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.particle_group, &[]);
            pass.set_pipeline(&self.thickness_pipeline);
            pass.set_vertex_buffer(0, self.particles.slice(..));
            pass.draw(0..6, 0..self.count);
            pass.set_pipeline(&self.film_thickness_pipeline);
            pass.set_vertex_buffer(0, self.films.slice(..));
            pass.draw(0..6, 0..self.film_count);
        }
        for (source, target, axis) in [
            (0, &self.ping_depth.view, 0),
            (1, &self.smooth_depth.view, 1),
            (2, &self.ping_depth.view, 0),
            (1, &self.smooth_depth.view, 1),
        ] {
            self.draw_pass(
                encoder,
                target,
                &self.filter_pipelines[axis],
                &self.filter_groups[source],
                3,
                1,
            );
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fluid viewport composition"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: if viewport.is_some() {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            if let Some([x, y, w, h]) = viewport {
                pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0., 1.);
                pass.set_scissor_rect(x, y, w, h);
            }
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if viewport.is_none() {
            renderer.encode_overlays(encoder, output, &self.scene_depth.view, draws);
        }
    }

    fn draw_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        group: &wgpu::BindGroup,
        vertices: u32,
        instances: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fluid reconstruction pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..vertices, 0..instances);
    }

    /// Visualize the most recently encoded state without advancing the simulation.
    pub fn encode_diagnostic(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        diagnostic: FluidDiagnostic,
    ) {
        let index = match diagnostic {
            FluidDiagnostic::Depth => 0,
            FluidDiagnostic::Thickness => 1,
        };
        self.draw_pass(
            encoder,
            output,
            &self.diagnostics[index],
            &self.composite_group,
            3,
            1,
        );
    }

    /// Integrated dimensionless RGB absorption along the original camera ray.
    #[must_use]
    pub fn optical_depth_texture(&self) -> &wgpu::Texture {
        &self.optical_depth.texture
    }
    #[must_use]
    pub fn thickness_texture(&self) -> &wgpu::Texture {
        &self.thickness.texture
    }
    #[must_use]
    pub fn depth_radius_texture(&self) -> &wgpu::Texture {
        &self.smooth_depth.texture
    }
}

#[cfg(test)]
mod film_input_tests {
    use super::*;
    #[test]
    fn format_admission_rejects_missing_blending_and_nonfloat_outputs() {
        let features = wgpu::Features::empty();
        let supported = |f: wgpu::TextureFormat| f.guaranteed_format_features(features);
        assert!(validate_formats(features, wgpu::TextureFormat::Rgba8Unorm, supported).is_ok());
        assert!(validate_formats(features, wgpu::TextureFormat::Rgba8Uint, supported).is_err());
        assert!(validate_formats(features, wgpu::TextureFormat::Depth32Float, supported).is_err());
        assert_eq!(
            validate_formats(features, wgpu::TextureFormat::Rgba32Float, supported),
            Err("fluid optical targets require blendable floating-point formats")
        );
        let optional_attachment = |f: wgpu::TextureFormat| {
            let mut support = supported(f);
            if f == wgpu::TextureFormat::Rgba8Snorm {
                support.allowed_usages |= wgpu::TextureUsages::RENDER_ATTACHMENT;
            }
            support
        };
        assert!(
            validate_formats(
                features,
                wgpu::TextureFormat::Rgba8Snorm,
                optional_attachment
            )
            .is_err()
        );
        assert!(
            validate_formats(
                wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                wgpu::TextureFormat::Rgba8Snorm,
                optional_attachment
            )
            .is_ok()
        );
        let no_blend = |f: wgpu::TextureFormat| {
            let mut support = supported(f);
            if f == wgpu::TextureFormat::R16Float {
                support
                    .flags
                    .remove(wgpu::TextureFormatFeatureFlags::BLENDABLE);
            }
            support
        };
        assert_eq!(
            validate_formats(features, wgpu::TextureFormat::Rgba8Unorm, no_blend),
            Err("fluid optical targets require blendable floating-point formats")
        );
        let no_attachment = |f: wgpu::TextureFormat| {
            let mut support = supported(f);
            if f == wgpu::TextureFormat::Rg32Float {
                support
                    .allowed_usages
                    .remove(wgpu::TextureUsages::RENDER_ATTACHMENT);
            }
            support
        };
        assert!(
            validate_formats(features, wgpu::TextureFormat::Rgba8Unorm, no_attachment).is_err()
        );
    }
    #[test]
    fn device_limits_admit_vertex_instances_without_storage_and_reject_shortfalls() {
        let supported = wgpu::Limits::default();
        assert_eq!(
            validate_device_limits(&supported, 32, 32, 2).unwrap(),
            (64, 128)
        );
        assert_eq!(
            validate_device_limits(&wgpu::Limits::downlevel_webgl2_defaults(), 32, 32, 2).unwrap(),
            (64, 128)
        );
        let mut restricted = supported.clone();
        restricted.max_sampled_textures_per_shader_stage = 5;
        assert_eq!(
            validate_device_limits(&restricted, 32, 32, 2),
            Err("fluid renderer device binding limits are insufficient")
        );
        restricted = supported.clone();
        restricted.max_color_attachment_bytes_per_sample = 15;
        assert_eq!(
            validate_device_limits(&restricted, 32, 32, 2),
            Err("fluid renderer device attachment limits are insufficient")
        );
        restricted = supported.clone();
        restricted.max_buffer_size = 100;
        assert!(validate_device_limits(&restricted, 32, 32, 2).is_err());
        restricted = supported;
        restricted.max_storage_buffer_binding_size = 0;
        restricted.max_storage_buffers_per_shader_stage = 0;
        assert_eq!(
            validate_device_limits(&restricted, 32, 32, 2).unwrap(),
            (64, 128)
        );
        restricted.max_vertex_attributes = 3;
        assert!(validate_device_limits(&restricted, 32, 32, 2).is_err());
    }
    #[test]
    fn film_prism_admission_preserves_geometry_and_rejects_invalid_cells() {
        let points = [[0., 0., 0.], [0., 0., 1.], [1., 0., 0.]];
        let material = [0.1, 0.04, 0.02, 1.333];
        let cell = FluidRenderFilmTriangle::new(points, 0.003, material).unwrap();
        assert_eq!(cell.a_thickness, [0., 0., 0., 0.003]);
        assert_eq!(cell.b[..3], points[1]);
        assert_eq!(cell.c[..3], points[2]);
        assert_eq!(cell.absorption_ior, material);
        assert_eq!(std::mem::size_of::<FluidRenderFilmTriangle>(), 64);
        for thickness in [0., -0.001, f32::NAN, f32::INFINITY] {
            assert!(FluidRenderFilmTriangle::new(points, thickness, material).is_err());
        }
        assert!(FluidRenderFilmTriangle::new([points[0]; 3], 0.003, material).is_err());
        assert!(FluidRenderFilmTriangle::new(points, 0.003, [-1., 0., 0., 1.]).is_err());
        assert!(FluidRenderFilmTriangle::new(points, 0.003, [0., 0., 0., 0.9]).is_err());
        let mut bad = points;
        bad[0][0] = f32::NAN;
        assert!(FluidRenderFilmTriangle::new(bad, 0.003, material).is_err());
    }
}

#[cfg(test)]
mod film_gpu_tests {
    use super::*;

    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn refraction_pixel_scale_matches_projected_displacement() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = format!(
            "{}\n{}",
            concat!(
                include_str!("dielectric_boundary.wgsl"),
                "\n",
                include_str!("fluid_composite.wgsl")
            ),
            r"
@group(1) @binding(0) var<storage,read_write> result:array<vec2f>;
@compute @workgroup_size(1) fn check(@builtin(global_invocation_id) id:vec3u) {
    let depths=array<f32,5>(0.001,0.005,0.1,1.0,9.0);
    result[id.x]=pixels_per_unit(depths[id.x])*vec2f(0.02,0.01);
}"
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("actual fluid composite projection helper"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("check"),
            compilation_options: Default::default(),
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 40,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 40,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let result_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let mut checks = 0;
        for fov in [0.4, 0.8, 1.4] {
            for orthographic in [false, true] {
                let camera = crate::SceneCamera {
                    eye: glam::Vec3::ZERO,
                    target: -glam::Vec3::Z,
                    up: glam::Vec3::Y,
                    projection: if orthographic {
                        crate::SceneProjection::Orthographic {
                            left: -1.,
                            right: 3.,
                            bottom: -0.5,
                            top: 1.,
                            near: 0.0001,
                            far: 10.,
                        }
                    } else {
                        crate::SceneProjection::Perspective {
                            vertical_fov: fov,
                            aspect: 2.,
                            near: 0.0001,
                            far: 10.,
                        }
                    },
                };
                let projection = camera.view_projection().unwrap();
                let uniform = CameraUniform {
                    view: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    projection: projection.to_cols_array_2d(),
                    inverse_projection: projection.inverse().to_cols_array_2d(),
                    inverse_view: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    viewport: [128., 64., 1., if orthographic { 2. } else { 0. }],
                    controls: [0.0001, 10., 8., 1.5],
                };
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_compute_pass(&Default::default());
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &camera_group, &[]);
                    pass.set_bind_group(1, &result_group, &[]);
                    pass.dispatch_workgroups(5, 1, 1);
                }
                encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 40);
                queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                readback
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                rx.recv().unwrap().unwrap();
                let bytes = readback.slice(..).get_mapped_range().unwrap();
                let values: &[[f32; 2]] = bytemuck::cast_slice(&bytes);
                for (i, depth) in [0.001, 0.005, 0.1, 1., 9.].into_iter().enumerate() {
                    let point = glam::Vec3::new(0.3, -0.2, -depth);
                    let a = projection.project_point3(point);
                    let b = projection.project_point3(point + glam::Vec3::new(0.02, -0.01, 0.));
                    let expected = [(b.x - a.x) * 64., -(b.y - a.y) * 32.];
                    for axis in 0..2 {
                        assert!(
                            (values[i][axis] - expected[axis]).abs()
                                < 2e-4 + expected[axis].abs() * 1e-5,
                            "fov={fov} ortho={orthographic} depth={depth} actual={:?} expected={expected:?}",
                            values[i]
                        );
                        checks += 1;
                    }
                }
                drop(bytes);
                readback.unmap();
            }
        }
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "FLUID REFRACTION PROJECTION PASS scalar_checks={checks} fovs=3 depths=5 projections=2 asymmetric_ortho=true viewport=128x64"
        );
    }

    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn film_gpu_integrates_prism_thickness_and_near_clipping() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        eprintln!("FILM GPU {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_storage_buffers_per_shader_stage: 0,
                max_storage_buffer_binding_size: 0,
                ..wgpu::Limits::default()
            },
            ..wgpu::DeviceDescriptor::default()
        }))
        .unwrap();
        assert_eq!(device.limits().max_storage_buffers_per_shader_stage, 0);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        // The shared scene pass alpha-blends even though fluid composition replaces color.
        // Reject this before pipeline creation; the healthy render below checks recovery.
        assert!(
            ScreenSpaceFluidRenderer::new_with_adapter(
                &device,
                &adapter,
                wgpu::TextureFormat::Rgba32Float,
                32,
                32,
                2,
            )
            .is_err()
        );
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let renderer = crate::SceneRenderer::new(&device, format);
        let mut fluid =
            ScreenSpaceFluidRenderer::new_with_adapter(&device, &adapter, format, 32, 32, 2)
                .unwrap();
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("film analytic output"),
            size: wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("film thickness readback"),
            size: 256 * 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let optical_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("integrated absorption readback"),
            size: 256 * 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let color_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("absorption final color readback"),
            size: 256 * 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let cell = FluidRenderFilmTriangle::new(
            [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            0.04,
            [0.1, 0.04, 0.02, 1.0],
        )
        .unwrap();
        let opaque = crate::SceneMesh::new(
            [[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]]
                .map(|position| crate::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 2, 0, 2, 3],
        )
        .unwrap();
        let mut geometry = renderer.reserve_geometry(&device, 4, 6).unwrap();
        geometry.update(&queue, &opaque).unwrap();
        let texture = renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        for filter in [FluidDepthFilter::None, FluidDepthFilter::Bilateral] {
            for camera_case in 0..5 {
                let orthographic = camera_case == 1 || camera_case == 2 || camera_case == 4;
                let shifted = camera_case >= 2;
                for (count, near, occluder_z, expected_axial, with_particle) in [
                    (1, 0.1, None, 0.02, false),
                    (2, 0.1, None, 0.04, false),
                    (1, 1.98, None, 0.01, false),
                    (0, 0.1, None, 0., false),
                    (1, 0.1, Some(0.1), 0., false),
                    (1, 0.1, Some(0.02), 0.01, false),
                    (1, 0.1, Some(-0.1), 0.02, false),
                    (1, 0.1, None, 0.02, true),
                    (0, 0.1, None, 0., true),
                ] {
                    let camera = crate::SceneCamera {
                        eye: if shifted {
                            glam::Vec3::new(-0.1, 0.05, 2.)
                        } else {
                            glam::Vec3::new(0., 0., 2.)
                        },
                        target: if shifted {
                            glam::Vec3::new(-0.1, 0.05, 0.)
                        } else {
                            glam::Vec3::ZERO
                        },
                        up: if shifted {
                            glam::Vec3::X
                        } else {
                            glam::Vec3::Y
                        },
                        projection: if orthographic {
                            crate::SceneProjection::Orthographic {
                                left: if shifted { -0.6 } else { -0.8 },
                                right: if shifted { 1.0 } else { 0.8 },
                                bottom: if shifted { -0.9 } else { -0.8 },
                                top: if shifted { 0.7 } else { 0.8 },
                                near: if camera_case == 4 && near == 0.1 {
                                    0.
                                } else {
                                    near
                                },
                                far: 10.,
                            }
                        } else {
                            crate::SceneProjection::Perspective {
                                vertical_fov: 0.8,
                                aspect: 1.,
                                near,
                                far: 10.,
                            }
                        },
                    };
                    let mut cells = vec![cell; count];
                    if count == 2 {
                        cells[1].absorption_ior = [2., 3., 4., 1.47];
                    }

                    let particles = if with_particle {
                        vec![FluidRenderParticle {
                            position_radius: [0., 0., -0.3, 0.1],
                            absorption_ior: [1., 5., 10., 1.],
                        }]
                    } else {
                        vec![]
                    };
                    fluid
                        .update_with_film(&queue, camera, &particles, &cells, 2., filter)
                        .unwrap();
                    // Rejected uploads must not clear the accepted film snapshot.
                    assert!(
                        fluid
                            .update_with_film(
                                &queue,
                                camera,
                                &[],
                                &[],
                                f32::NAN,
                                FluidDepthFilter::None
                            )
                            .is_err()
                    );
                    let transform = renderer
                        .create_transform(
                            &device,
                            camera.view_projection().unwrap()
                                * glam::Mat4::from_translation(glam::Vec3::new(
                                    0.,
                                    0.,
                                    occluder_z.unwrap_or(0.),
                                )),
                        )
                        .unwrap();
                    let draws = if occluder_z.is_some() {
                        vec![crate::SceneDraw {
                            geometry: &geometry,
                            texture: &texture,
                            transform: &transform,
                            overlay: false,
                        }]
                    } else {
                        vec![]
                    };
                    let mut encoder = device.create_command_encoder(&Default::default());
                    fluid.encode(
                        &renderer,
                        &mut encoder,
                        &output.create_view(&Default::default()),
                        wgpu::Color::WHITE,
                        &draws,
                    );
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: fluid.thickness_texture(),
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(256),
                                rows_per_image: Some(32),
                            },
                        },
                        wgpu::Extent3d {
                            width: 32,
                            height: 32,
                            depth_or_array_layers: 1,
                        },
                    );
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: fluid.optical_depth_texture(),
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &optical_readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(256),
                                rows_per_image: Some(32),
                            },
                        },
                        wgpu::Extent3d {
                            width: 32,
                            height: 32,
                            depth_or_array_layers: 1,
                        },
                    );
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &output,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &color_readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(256),
                                rows_per_image: Some(32),
                            },
                        },
                        wgpu::Extent3d {
                            width: 32,
                            height: 32,
                            depth_or_array_layers: 1,
                        },
                    );
                    queue.submit([encoder.finish()]);
                    let (tx, rx) = std::sync::mpsc::channel();
                    readback
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            tx.send(result).unwrap();
                        });
                    let (optical_tx, optical_rx) = std::sync::mpsc::channel();
                    optical_readback
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            optical_tx.send(result).unwrap();
                        });
                    let (color_tx, color_rx) = std::sync::mpsc::channel();
                    color_readback
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            color_tx.send(result).unwrap();
                        });
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    rx.recv().unwrap().unwrap();
                    optical_rx.recv().unwrap().unwrap();
                    color_rx.recv().unwrap().unwrap();
                    let color_bytes = color_readback.slice(..).get_mapped_range().unwrap();
                    let optical_bytes = optical_readback.slice(..).get_mapped_range().unwrap();
                    let bytes = readback.slice(..).get_mapped_range().unwrap();
                    for (px, py) in [(16, 16), (24, 16), (0, 0)] {
                        let offset = py * 256 + px * 2;
                        let actual = half::f16::from_bits(u16::from_le_bytes([
                            bytes[offset],
                            bytes[offset + 1],
                        ]))
                        .to_f32();
                        let inverse = camera.view_projection().unwrap().inverse();
                        let x = ((px as f32 + 0.5) / 32.) * 2. - 1.;
                        let y = 1. - ((py as f32 + 0.5) / 32.) * 2.;
                        let start = inverse.project_point3(glam::Vec3::new(x, y, 0.));
                        let end = inverse.project_point3(glam::Vec3::new(x, y, 1.));
                        let direction = (end - start).normalize();
                        // Independent world-space triangle membership, not authored pixel labels.
                        let hit = start + direction * ((0.02 - start.z) / direction.z);
                        let inside = hit.y > -1. && hit.y < 1. && hit.x.abs() < (1. - hit.y) * 0.5;
                        let film_path = if inside {
                            expected_axial / direction.z.abs()
                        } else {
                            0.
                        };
                        let origin = if orthographic {
                            glam::Vec3::new(start.x, start.y, camera.eye.z)
                        } else {
                            camera.eye
                        };
                        let centre = glam::Vec3::new(0., 0., -0.3) - origin;
                        let b = direction.dot(centre);
                        let discriminant = b * b - centre.length_squared() + 0.1_f32.powi(2);
                        let sphere_path = if with_particle && discriminant > 0. {
                            discriminant.sqrt()
                        } else {
                            0.
                        };
                        // World-space chord 2*sqrt(discriminant), divided by 2 units/m.
                        let expected = film_path + sphere_path;
                        eprintln!(
                            "FILM camera_case={camera_case} filter={filter:?} orthographic={orthographic} count={count} near={near} occluder={occluder_z:?} pixel={px},{py} actual_m={actual} expected_m={expected}"
                        );
                        assert!((actual - expected).abs() <= expected.abs() * 0.003 + 0.000002);
                        for channel in 0..3 {
                            let optical_offset = py * 256 + px * 8 + channel * 2;
                            let tau = half::f16::from_bits(u16::from_le_bytes([
                                optical_bytes[optical_offset],
                                optical_bytes[optical_offset + 1],
                            ]))
                            .to_f32();
                            let coefficient_sum: f32 =
                                cells.iter().map(|c| c.absorption_ior[channel]).sum();
                            let expected_tau = if count > 0 {
                                film_path / count as f32 * coefficient_sum
                            } else {
                                0.
                            } + [1., 5., 10.][channel] * sphere_path;
                            eprintln!(
                                "ABSORPTION channel={channel} actual={tau} expected={expected_tau}"
                            );
                            assert!(
                                (tau - expected_tau).abs() <= expected_tau.abs() * 0.004 + 0.000002
                            );
                            if inside && count > 0 && occluder_z.is_none() && !with_particle {
                                // IOR=1 for the nearest planar layer eliminates interface
                                // reflection/refraction; white background reveals exp(-tau).
                                let linear = (-expected_tau).exp();
                                let srgb = if linear <= 0.0031308 {
                                    12.92 * linear
                                } else {
                                    1.055 * linear.powf(1. / 2.4) - 0.055
                                };
                                let actual_color =
                                    f32::from(color_bytes[py * 256 + px * 4 + channel]) / 255.;
                                assert!(
                                    (actual_color - srgb).abs() < 0.012,
                                    "absorption composition actual={actual_color} expected={srgb}"
                                );
                            }
                        }
                    }
                    drop(bytes);
                    readback.unmap();
                    drop(optical_bytes);
                    optical_readback.unmap();
                    drop(color_bytes);
                    color_readback.unmap();
                }
            }
        }
        assert!(pollster::block_on(scope.pop()).is_none());
    }
}

#[cfg(test)]
mod shader_portability_tests {
    #[test]
    fn optical_shaders_translate_to_gles_without_storage_buffers() {
        use naga::{ShaderStage, back::glsl, proc::BoundsCheckPolicies, valid};
        let modules = [
            ("geometry", include_str!("fluid_screen.wgsl")),
            ("filter", include_str!("fluid_filter.wgsl")),
            (
                "composite",
                concat!(
                    include_str!("dielectric_boundary.wgsl"),
                    "\n",
                    include_str!("fluid_composite.wgsl")
                ),
            ),
        ];
        let mut translated = 0;
        for (name, source) in modules {
            let source = crate::depth_sample::shader_for_backend(wgpu::Backend::Gl, source);
            let module = naga::front::wgsl::parse_str(&source).unwrap();
            assert!(
                module
                    .global_variables
                    .iter()
                    .all(|(_, v)| !matches!(v.space, naga::AddressSpace::Storage { .. }))
            );
            let info =
                valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
                    .validate(&module)
                    .unwrap();
            let options = glsl::Options {
                version: glsl::Version::new_gles(300),
                ..Default::default()
            };
            for entry in &module.entry_points {
                assert!(matches!(
                    entry.stage,
                    ShaderStage::Vertex | ShaderStage::Fragment
                ));
                let pipeline = glsl::PipelineOptions {
                    shader_stage: entry.stage,
                    entry_point: entry.name.clone(),
                    multiview: None,
                };
                let mut output = String::new();
                glsl::Writer::new(
                    &mut output,
                    &module,
                    &info,
                    &options,
                    &pipeline,
                    BoundsCheckPolicies::default(),
                )
                .unwrap()
                .write()
                .unwrap();
                assert!(output.starts_with("#version 300 es"));
                assert!(!output.contains(" buffer "));
                eprintln!(
                    "GLES300 {name}/{} translated_bytes={}",
                    entry.name,
                    output.len()
                );
                translated += 1;
            }
        }
        assert_eq!(translated, 14);
    }
}

#[cfg(test)]
mod allocation_tests {
    use super::*;
    #[test]
    fn footprint_counts_nine_targets_both_geometry_buffers_and_uniform() {
        let expected = 64 * 32 * 54
            + 8 * (size_of::<FluidRenderParticle>() + size_of::<FluidRenderFilmTriangle>()) as u64
            + size_of::<CameraUniform>() as u64;
        assert_eq!(
            ScreenSpaceFluidRenderer::required_allocation_bytes(
                wgpu::TextureFormat::Rgba8Unorm,
                64,
                32,
                8
            )
            .unwrap(),
            expected
        );
        assert_eq!(
            ScreenSpaceFluidRenderer::required_allocation_bytes(
                wgpu::TextureFormat::Rgba16Float,
                64,
                32,
                8
            )
            .unwrap(),
            expected + 64 * 32 * 4
        );
        assert!(
            ScreenSpaceFluidRenderer::required_allocation_bytes(
                wgpu::TextureFormat::Rgba8Unorm,
                0,
                32,
                8
            )
            .is_err()
        );
        assert!(
            ScreenSpaceFluidRenderer::required_allocation_bytes(
                wgpu::TextureFormat::Bc1RgbaUnorm,
                64,
                32,
                8
            )
            .is_err()
        );
        assert!(
            ScreenSpaceFluidRenderer::required_allocation_bytes(
                wgpu::TextureFormat::Rgba8Unorm,
                u32::MAX,
                u32::MAX,
                usize::MAX
            )
            .is_err()
        );
    }
    #[test]
    #[ignore = "requires a physical GPU adapter"]
    fn gpu_budget_boundary_and_rejected_resize_preserve_previous_resources() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        println!("FLUID RESIDENCY GPU {:?}", adapter.get_info());
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let required =
            ScreenSpaceFluidRenderer::required_allocation_bytes(format, 64, 32, 8).unwrap();
        assert!(
            ScreenSpaceFluidRenderer::new_with_adapter_budget(
                &device,
                &adapter,
                format,
                64,
                32,
                8,
                0,
                required - 1
            )
            .is_err()
        );
        let old = ScreenSpaceFluidRenderer::new_with_adapter_budget(
            &device, &adapter, format, 64, 32, 8, 0, required,
        )
        .unwrap();
        let texture_bytes = [
            &old.background,
            &old.scene_depth,
            &old.particle_depth,
            &old.raw_depth,
            &old.ping_depth,
            &old.smooth_depth,
            &old.material,
            &old.thickness,
            &old.optical_depth,
        ]
        .iter()
        .map(|t| {
            u64::from(t.texture.width())
                * u64::from(t.texture.height())
                * u64::from(t.texture.format().block_copy_size(None).unwrap())
        })
        .sum::<u64>();
        let actual = texture_bytes + old.camera.size() + old.particles.size() + old.films.size();
        assert_eq!(actual, required);
        assert_eq!(old.allocation_bytes(), actual);
        let resized =
            ScreenSpaceFluidRenderer::required_allocation_bytes(format, 128, 64, 8).unwrap();
        assert!(
            ScreenSpaceFluidRenderer::new_with_adapter_budget(
                &device,
                &adapter,
                format,
                128,
                64,
                8,
                old.allocation_bytes(),
                required + resized - 1
            )
            .is_err()
        );
        assert_eq!(old.size(), [64, 32]);
        assert_eq!(old.allocation_bytes(), required);
        assert!(
            ScreenSpaceFluidRenderer::new_with_adapter_budget(
                &device,
                &adapter,
                format,
                128,
                64,
                8,
                u64::MAX,
                u64::MAX
            )
            .is_err()
        );
        assert!(pollster::block_on(scope.pop()).is_none());
        println!("logical_live_bytes={actual} rejected_resize_bytes={resized}");
    }
}
