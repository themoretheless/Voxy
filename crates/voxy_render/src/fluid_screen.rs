//! Screen-space optical reconstruction of physical spherical liquid samples.
//! Simulation remains owned by the caller. Optical thickness is a ray integral,
//! not another liquid mass reservoir. Single-sample perspective rendering only.
use wgpu::util::DeviceExt;

/// World-space centre/radius and SI absorption coefficients with dielectric IOR.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FluidRenderParticle {
    pub position_radius: [f32; 4],
    pub absorption_ior: [f32; 4],
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
    // width, height, world units per metre, filter mode
    viewport: [f32; 4],
    // near, far, filter max pixels, radius-relative range
    controls: [f32; 4],
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Owns reusable particle storage, scene background and fluid reconstruction targets.
/// `encode` produces final colour; it does not supply temporal motion vectors or
/// write a combined scene/fluid depth for subsequent opaque draws.
#[derive(Debug)]
pub struct ScreenSpaceFluidRenderer {
    size: [u32; 2],
    capacity: usize,
    count: u32,
    camera: wgpu::Buffer,
    particles: wgpu::Buffer,
    particle_group: wgpu::BindGroup,
    filter_groups: [wgpu::BindGroup; 3],
    composite_group: wgpu::BindGroup,
    depth_pipeline: wgpu::RenderPipeline,
    thickness_pipeline: wgpu::RenderPipeline,
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
}

impl ScreenSpaceFluidRenderer {
    /// Allocates a fixed particle budget and resolution. Recreate on resize/device recovery.
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
        let bytes = capacity
            .checked_mul(size_of::<FluidRenderParticle>())
            .ok_or("fluid capacity overflow")?;
        if width == 0
            || height == 0
            || width > device.limits().max_texture_dimension_2d
            || height > device.limits().max_texture_dimension_2d
            || capacity == 0
            || capacity > u32::MAX as usize
            || bytes as u64 > u64::from(device.limits().max_storage_buffer_binding_size)
        {
            return Err("invalid fluid target or particle capacity");
        }
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
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fluid camera"),
            contents: bytemuck::bytes_of(&<CameraUniform as bytemuck::Zeroable>::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let particles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physical liquid render samples"),
            size: bytes as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
        let particle_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid particles layout"),
            entries: &[
                uniform(0),
                buffer_entry(
                    1,
                    wgpu::ShaderStages::VERTEX,
                    wgpu::BufferBindingType::Storage { read_only: true },
                ),
                texture_entry(2, wgpu::TextureSampleType::Depth),
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
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles.as_entire_binding(),
                },
                texture_binding(2, &scene_depth.view),
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
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen-space physical liquid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fluid_screen.wgsl").into()),
        });
        let filter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid filter"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fluid_filter.wgsl").into()),
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid composition"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fluid_composite.wgsl").into()),
        });
        let pipeline = |shader: &wgpu::ShaderModule,
                        label,
                        group_layout,
                        vs,
                        fs,
                        targets: &[Option<wgpu::ColorTargetState>],
                        depth| {
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
                    buffers: &[],
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
            &[color_target(wgpu::TextureFormat::R16Float, Some(additive))],
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
            capacity,
            count: 0,
            camera,
            particles,
            particle_group,
            filter_groups,
            composite_group,
            depth_pipeline,
            thickness_pipeline,
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
        })
    }

    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Updates a snapshot without changing caller simulation or particle data.
    /// All validation precedes GPU writes; a rejected update preserves the prior snapshot.
    /// # Errors
    /// Rejects non-perspective views, invalid SI materials/particles and capacity overflow.
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        camera: crate::SceneCamera,
        particles: &[FluidRenderParticle],
        units_per_metre: f32,
        filter: FluidDepthFilter,
    ) -> Result<(), &'static str> {
        let crate::SceneProjection::Perspective { near, far, .. } = camera.projection else {
            return Err("fluid renderer requires perspective camera");
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
                },
            ],
            controls: [near, far, 8.0, 1.5],
        };
        queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&uniform));
        if !particles.is_empty() {
            queue.write_buffer(&self.particles, 0, bytemuck::cast_slice(particles));
        }
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
                label: Some("fluid physical sphere surface"),
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
            pass.draw(0..6, 0..self.count);
        }
        self.draw_pass(
            encoder,
            &self.thickness.view,
            &self.thickness_pipeline,
            &self.particle_group,
            6,
            self.count,
        );
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
        self.draw_pass(
            encoder,
            output,
            &self.composite_pipeline,
            &self.composite_group,
            3,
            1,
        );
        renderer.encode_overlays(encoder, output, &self.scene_depth.view, draws);
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

    #[must_use]
    pub fn thickness_texture(&self) -> &wgpu::Texture {
        &self.thickness.texture
    }
    #[must_use]
    pub fn depth_radius_texture(&self) -> &wgpu::Texture {
        &self.smooth_depth.texture
    }
}
