//! Presented skeletal poses driving raster, BLAS replacement, ray HDR and motion.
use glam::{Mat4, Vec3};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use voxy_render::{SceneMesh, SceneSurface, SceneVertex, SkinnedMotionHistory};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// Independent probe modes and observed lifecycle conditions.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct Demo {
    window: Option<Arc<Window>>,
    host: Option<SceneSurface>,
    history: Option<SkinnedMotionHistory>,
    scene: Option<voxy_render::RayScene>,
    frames: u32,
    draw_attempts: u32,
    retry_at: Option<Instant>,
    last_skipped: Option<voxy_render::RenderOutcome>,
    prepare_ms: Vec<f64>,
    submit_ms: Vec<f64>,
    reflected: Option<voxy_render::SurfaceReflectionJob>,
    direct: Option<voxy_render::SurfaceLightingJob>,
    combined: Option<voxy_render::RadianceComposition>,
    motion: Option<voxy_render::RasterMotionPass>,
    expected_depth: Option<voxy_render::PreviousDepthPass>,
    primary: Option<voxy_render::PrimarySurfaceJob>,
    attachments: Option<voxy_render::RasterRayAttachments>,
    reused_primary: u32,
    resized_primary: u32,
    started: Option<Instant>,
    frame_loop: voxy_runtime::FrameLoop,
    paused: bool,
    suspended: bool,
    occluded: bool,
    phase: f32,
    exposure: f32,
    roughness: f32,
    metallic: f32,
    smoke: bool,
    profile: bool,
    gpu_profile: bool,
    backend: voxy_render::GraphicsBackend,
    require_nvidia: bool,
    timestamps: Option<(wgpu::QuerySet, wgpu::Buffer)>,
    test_started: Option<Instant>,
    initial_size: Option<[u32; 2]>,
    configured_size: Option<[u32; 2]>,
    resized: bool,
    moving_samples: u32,
    depth_changes: u32,
    reflection_samples: u32,
    failure: Option<String>,
    overlay: Option<Overlay>,
    texture: Option<voxy_render::SceneTexture>,
    resolver: Option<voxy_render::TemporalResolve>,
    lighting: Option<voxy_render::GgxRayLightingPipeline>,
    display: Option<(u32, voxy_render::TextureBlit)>,
    temporal: Option<voxy_render::TemporalHistory>,
    previous_camera: Option<Mat4>,
}
struct Overlay {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}
impl Overlay {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray demo controls"),
            contents: bytemuck::cast_slice(&[1.0_f32, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ray demo 2D controls"), source: wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var<uniform> controls: vec4<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) color: vec3<f32> }
@vertex fn vs_main(@builtin(vertex_index) i:u32, @builtin(instance_index) item:u32)->Vertex {
    let corners = array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(1.,1.),vec2(0.,0.),vec2(1.,1.),vec2(0.,1.));
    var bounds=vec4<f32>(-.95,.82,.95,.96);
    var color=vec3<f32>(.03,.03,.05);
    if item==1u { bounds=vec4<f32>(-.92,.86,-.92+1.5*controls.x/8.,.92); color=vec3<f32>(.1,.5,1.); }
    if item==2u { bounds=vec4<f32>(.8,.86,.92,.92); color=select(vec3<f32>(.1,.8,.2),vec3<f32>(1.,.3,.05),controls.y>0.); }
    var out:Vertex; out.position=vec4<f32>(mix(bounds.xy,bounds.zw,corners[i]),0.,1.); out.color=color; return out;
}
@fragment fn fs_main(v:Vertex)->@location(0) vec4<f32> { return vec4<f32>(v.color,1.); }
".into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ray demo overlay"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray demo controls"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            bindings,
            uniform,
        }
    }
    fn encode(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ray demo 2D status and exposure"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..6, 0..3);
    }
}
fn mesh() -> Result<voxy_render::SkinnedMesh, Box<dyn std::error::Error>> {
    let vertices = [
        [-1.0, -1.0, 0.5],
        [1.0, -1.0, 0.5],
        [1.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|position| voxy_render::SkinnedVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        uv: [(position[0] + 1.0) * 0.5, (position[1] + 1.0) * 0.5],
        joints: [0, 1, 0, 0],
        weights: if position[1] < 0.0 {
            [65535, 0, 0, 0]
        } else {
            [0, 65535, 0, 0]
        },
    });
    Ok(voxy_render::SkinnedMesh::new(
        vertices.to_vec(),
        vec![0, 1, 2, 0, 2, 3],
        2,
    )?)
}
fn ray_mesh(surface: &SceneMesh) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    let mut vertices = surface.vertices().to_vec();
    let mut indices = surface.indices().to_vec();
    let base = u32::try_from(vertices.len())?;
    for position in [[-10.0, -10.0, 3.0], [10.0, -10.0, 3.0], [0.0, 10.0, 3.0]] {
        vertices.push(SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        });
    }
    indices.extend([base, base + 1, base + 2]);
    Ok(SceneMesh::new(vertices, indices)?)
}
// Experimental ray queries require an explicit opt-in from the executable.
#[allow(unsafe_code)]
fn experimental() -> wgpu::ExperimentalFeatures {
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}
#[allow(clippy::cast_precision_loss)]
fn center_depth(
    current_camera: Mat4,
    previous_camera: Mat4,
    current: &[voxy_render::PreviousPositionVertex],
    previous: &[voxy_render::PreviousPositionVertex],
    size: [u32; 2],
) -> Result<f32, Box<dyn std::error::Error>> {
    let point = glam::Vec2::new((size[0] / 2) as f32 + 0.5, (size[1] / 2) as f32 + 0.5)
        / glam::Vec2::new(size[0] as f32, size[1] as f32)
        * 2.0
        - glam::Vec2::ONE;
    let point = glam::Vec2::new(point.x, -point.y);
    for (triangle, old) in current.chunks_exact(3).zip(previous.chunks_exact(3)) {
        let clips = triangle
            .iter()
            .map(|vertex| current_camera * Vec3::from_array(vertex.current).extend(1.0))
            .collect::<Vec<_>>();
        let xy = clips
            .iter()
            .map(|clip| clip.truncate().truncate() / clip.w)
            .collect::<Vec<_>>();
        let e0 = xy[1] - xy[0];
        let e1 = xy[2] - xy[0];
        let p = point - xy[0];
        let determinant = e0.perp_dot(e1);
        let b = p.perp_dot(e1) / determinant;
        let c = e0.perp_dot(p) / determinant;
        let a = 1.0 - b - c;
        if a >= 0.0 && b >= 0.0 && c >= 0.0 {
            let weights = [a / clips[0].w, b / clips[1].w, c / clips[2].w];
            let denominator = weights.iter().sum::<f32>();
            let position = old
                .iter()
                .zip(weights)
                .map(|(v, w)| Vec3::from_array(v.previous) * (w / denominator))
                .sum::<Vec3>();
            let clip = previous_camera * position.extend(1.0);
            return Ok(clip.z / clip.w);
        }
    }
    Err("center had no animated triangle coverage".into())
}
impl Demo {
    fn reset_presentation_history(&mut self) {
        if let Some(history) = &mut self.history {
            history.reset();
        }
        self.temporal = None;
        self.previous_camera = None;
        self.frame_loop.reset();
        self.started = Some(Instant::now());
        self.retry_at = None;
    }

    fn initialize(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_inner_size(winit::dpi::LogicalSize::new(640, 480))
                    .with_title(
                        "Voxy animated ray | Space pause | R reset | G roughness | M metallic | +/- exposure | Esc exit",
                    ),
            )?,
        );
        let instance = voxy_render::GraphicsOptions {
            backend: self.backend,
            ..Default::default()
        }
        .create_instance_with_display(event_loop.owned_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = if self.require_nvidia {
            pollster::block_on(instance.enumerate_adapters(self.backend.backends()))
                .into_iter()
                .find(|adapter| {
                    let info = adapter.get_info();
                    info.vendor == 0x10de
                        && matches!(
                            info.device_type,
                            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                        )
                        && adapter.is_surface_supported(&surface)
                        && adapter
                            .features()
                            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
                })
                .ok_or(
                    "no physical NVIDIA ray-query adapter supports the selected window/backend",
                )?
        } else {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            }))?
        };
        if !adapter
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(
                "selected surface adapter does not support experimental ray queries".into(),
            );
        }
        drop(surface);
        let size = window.inner_size();
        let host = pollster::block_on(SceneSurface::new_with_adapter_and_output_experimental(
            window.clone(),
            size.width,
            size.height,
            &instance,
            adapter,
            wgpu::Features::EXPERIMENTAL_RAY_QUERY
                | if self.gpu_profile {
                    wgpu::Features::TIMESTAMP_QUERY
                } else {
                    wgpu::Features::empty()
                },
            voxy_render::SurfaceOutput::Sdr,
            experimental(),
        ))?;
        if self.gpu_profile {
            self.timestamps = Some((
                host.device().create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("animated GPU stages"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 330 * 8,
                }),
                host.device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("animated GPU samples"),
                    size: 330 * 256,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
            ));
        }
        println!("ANIMATED RAY GPU: {:?}", host.adapter().get_info());
        let history = SkinnedMotionHistory::new(mesh()?);
        let initial =
            history.prepare_frame(&[Mat4::IDENTITY; 2], Mat4::IDENTITY, [0.8, 0.4, 0.2, 1.0])?;
        self.scene = Some(voxy_render::RayScene::from_scene_mesh(
            host.device(),
            &ray_mesh(initial.scene())?,
            1,
        )?);
        self.history = Some(history);
        let renderer = host.create_scene_renderer();
        self.texture = Some(renderer.upload_texture(
            host.device(),
            host.queue(),
            2,
            2,
            &[
                255, 255, 255, 255, 128, 255, 128, 255, 128, 128, 255, 255, 255, 255, 255, 255,
            ],
        )?);
        self.overlay = Some(Overlay::new(host.device(), host.color_format()));
        self.resolver = Some(voxy_render::TemporalResolve::new(host.device())?);
        self.lighting = Some(voxy_render::GgxRayLightingPipeline::new(host.device())?);
        self.display = Some((
            self.exposure.to_bits(),
            voxy_render::TextureBlit::tone_mapped(
                host.device(),
                host.color_format(),
                self.exposure,
            )
            .ok_or("tone mapping unavailable")?,
        ));
        self.temporal = None;
        self.previous_camera = None;
        self.reflected = None;
        self.direct = None;
        self.combined = None;
        self.motion = None;
        self.expected_depth = None;
        self.primary = None;
        self.attachments = None;
        self.host = Some(host);
        self.frame_loop.reset();
        self.started = Some(Instant::now());
        self.test_started = self.started;
        self.initial_size = Some([size.width, size.height]);
        self.configured_size = self.initial_size;
        window.focus_window();
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.suspended {
            return Ok(());
        }
        let window = self.window.as_ref().ok_or("missing window")?;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if self.profile && self.initial_size != Some([size.width, size.height]) {
            return Err("profile window dimensions changed".into());
        }
        if self.configured_size != Some([size.width, size.height]) {
            if let Some(host) = &mut self.host {
                host.resize(size.width, size.height)?;
            }
            if let Some(history) = &mut self.history {
                history.reset();
            }
            self.temporal = None;
            self.previous_camera = None;
            self.configured_size = Some([size.width, size.height]);
        }
        if (self.smoke || self.profile)
            && self
                .test_started
                .is_some_and(|start| start.elapsed().as_secs() > 60)
        {
            return Err(format!(
                "animated ray smoke timed out after {} presentations and {} draw attempts",
                self.frames, self.draw_attempts
            )
            .into());
        }
        if self.occluded {
            self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            return Ok(());
        }
        let prepare_started = Instant::now();
        self.draw_attempts += 1;
        let now = Instant::now();
        let elapsed = self
            .started
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64());
        self.frame_loop.set_paused(self.paused);
        let work = self.frame_loop.advance(if self.smoke || self.profile {
            1.0 / 60.0
        } else {
            elapsed
        });
        self.phase += work.delta_seconds as f32;
        let joints = [
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(
                0.35 * self.phase.sin(),
                0.0,
                0.1 * (self.phase * 0.5).sin(),
            )),
        ];
        let history = self.history.as_mut().ok_or("missing pose history")?;
        let candidate = history.prepare_frame(&joints, Mat4::IDENTITY, [0.8, 0.4, 0.2, 1.0])?;
        let scene = self.scene.as_mut().ok_or("missing rays")?;
        scene.replace_scene_mesh(&ray_mesh(candidate.scene())?)?;
        let host = self.host.as_mut().ok_or("missing host")?;
        let device = host.device().clone();
        let camera =
            glam::camera::rh::proj::directx::perspective(
                1.0,
                size.width as f32 / size.height as f32,
                0.1,
                10.0,
            ) * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let previous_storage = self
            .primary
            .as_ref()
            .map(|p| (p.dimensions(), p.output().clone()));
        let previous_attachments = self.attachments.as_ref().map(|a| {
            (
                a.depth().size(),
                a.depth().clone(),
                a.guides().normal_roughness().clone(),
                a.guides().diffuse_albedo().clone(),
                a.guides().material_f0().clone(),
            )
        });
        let previous_reflected = self
            .reflected
            .as_ref()
            .map(|p| (p.radiance().clone(), p.distance().clone()));
        let previous_direct = self.direct.as_ref().map(|p| p.output().clone());
        let previous_hdr = self.combined.as_ref().map(|p| p.output().clone());
        let frame = voxy_render::RasterRayFrame::with_ggx_resources(
            &device,
            host.adapter(),
            scene,
            voxy_render::RasterRayOptions {
                dimensions: [size.width, size.height],
                view_projection: camera,
                clear_depth: 1.0,
                light: voxy_render::SurfacePointLight {
                    position: [0.0, 0.0, 2.0],
                    intensity: [100.0; 3],
                    bias: 0.001,
                },
                reflection: voxy_render::SurfaceReflectionOptions {
                    material: voxy_render::ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)?,
                    camera: [0.0, 0.0, 3.0],
                    bias: 0.001,
                    maximum_distance: 10.0,
                },
            },
            &[[0.0; 4], [0.0; 4], [4.0, 1.0, 0.5, 1.0]],
            voxy_render::GgxRasterRayResources {
                pipeline: self.lighting.as_ref().ok_or("missing cached lighting")?,
                previous_attachments: self.attachments.take(),
                state: voxy_render::GgxRayLightingState {
                    seed: self.frames,
                    previous_primary: self.primary.take(),
                    previous_combined: self.combined.take(),
                    previous_direct: self.direct.take(),
                    previous_reflected: self.reflected.take(),
                },
            },
        )?;
        if self.smoke {
            if let Some((radiance, distance)) = previous_reflected {
                for (old, new) in [
                    (&radiance, frame.reflected_radiance()),
                    (&distance, frame.reflection_distance()),
                ] {
                    if (old == new) != (old.size() == new.size()) {
                        return Err("reflection reuse or resize mismatch".into());
                    }
                }
            }
            if let Some(old) = previous_direct
                && (old == *frame.direct_radiance())
                    != (old.size() == frame.direct_radiance().size())
            {
                return Err("direct light reuse or resize mismatch".into());
            }
            if let Some(old) = previous_hdr
                && (old == *frame.output()) != (old.size() == frame.output().size())
            {
                return Err("HDR storage reuse or resize mismatch".into());
            }
            if let Some((dimensions, depth, normals, diffuse, f0)) = previous_attachments {
                let same_size = [dimensions.width, dimensions.height] == [size.width, size.height];
                for (old, new) in [
                    (&depth, frame.depth()),
                    (&normals, frame.guides().normal_roughness()),
                    (&diffuse, frame.guides().diffuse_albedo()),
                    (&f0, frame.guides().material_f0()),
                ] {
                    if (old == new) != same_size {
                        return Err("raster attachment reuse/resize mismatch".into());
                    }
                }
            }
            if let Some((dimensions, storage)) = previous_storage {
                if dimensions == [size.width, size.height] {
                    if frame.primary().output() != &storage {
                        return Err("primary storage was not reused".into());
                    }
                    self.reused_primary += 1;
                } else {
                    if frame.primary().output() == &storage {
                        return Err("resize retained incompatible primary storage".into());
                    }
                    self.resized_primary += 1;
                }
            }
        }
        let texture = self.texture.as_ref().ok_or("missing scene texture")?;
        let material = frame.material_inputs(texture)?;
        let geometry = voxy_render::ReconstructionGuideMesh::from_scene_mesh(
            &device,
            candidate.scene(),
            Mat4::IDENTITY,
            self.metallic,
            self.roughness,
        )?;
        let previous_camera = self.previous_camera.unwrap_or(camera);
        let prior_motion = self.motion.as_ref().map(|p| p.output().clone());
        let prior_depth = self.expected_depth.as_ref().map(|p| p.output().clone());
        let motion = if let Some(previous) = self.motion.take() {
            previous.next_frame_reusing(
                &device,
                frame.depth(),
                [camera, previous_camera],
                &candidate.motion().vertices,
                !candidate.motion().history_valid,
            )?
        } else {
            frame.deformation_motion(
                previous_camera,
                &candidate.motion().vertices,
                !candidate.motion().history_valid,
            )?
        };
        let expected_depth = if let Some(previous) = self.expected_depth.take() {
            previous.next_frame_reusing(
                &device,
                frame.depth(),
                [camera, previous_camera],
                &candidate.motion().vertices,
            )?
        } else {
            frame.previous_depth(previous_camera, &candidate.motion().vertices)?
        };
        if self.smoke {
            for (old, new) in [
                (prior_motion, motion.output()),
                (prior_depth, expected_depth.output()),
            ] {
                if let Some(old) = old
                    && (old == *new) != (old.size() == new.size())
                {
                    return Err("depth/motion reuse or resize mismatch".into());
                }
            }
        }
        let current_vertices = candidate
            .motion()
            .vertices
            .iter()
            .map(|vertex| voxy_render::PreviousPositionVertex {
                current: vertex.current,
                previous: vertex.current,
            })
            .collect::<Vec<_>>();
        let current_depth = expected_depth.next_frame(
            &device,
            frame.depth(),
            [camera, camera],
            &current_vertices,
        )?;
        if self.temporal.is_none() {
            self.temporal = Some(voxy_render::TemporalHistory::new(
                &device,
                frame.output().width(),
                frame.output().height(),
            )?);
        }
        let temporal = self.temporal.as_mut().ok_or("missing temporal history")?;
        temporal.resize(frame.output().width(), frame.output().height())?;
        let reset_temporal = !temporal.valid() || !candidate.motion().history_valid;
        let resolved = temporal.prepare_resolve(
            self.resolver.as_ref().ok_or("missing temporal resolver")?,
            frame.output(),
            motion.output(),
            expected_depth.output(),
            voxy_render::TemporalResolveOptions {
                history_weight: 0.9,
                depth_tolerance: 0.001,
                reset_history: !candidate.motion().history_valid,
            },
            true,
        )?;
        let display = self
            .display
            .as_mut()
            .ok_or("missing retained tone mapper")?;
        if display.0 != self.exposure.to_bits() {
            display.1 = display
                .1
                .with_exposure(&device, self.exposure)
                .ok_or("tone mapping exposure unavailable")?;
            display.0 = self.exposure.to_bits();
        }
        let blit = &display.1;
        let color = resolved
            .output()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let copy = self.smoke.then(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("animated ray proof"),
                size: 1792,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        let overlay = self.overlay.as_ref().ok_or("missing 2D overlay")?;
        host.queue().write_buffer(
            &overlay.uniform,
            0,
            bytemuck::cast_slice(&[self.exposure, if self.paused { 1.0 } else { 0.0 }, 0.0, 0.0]),
        );
        let mut snapshot = None;
        let prepare_ms = prepare_started.elapsed().as_secs_f64() * 1000.0;
        let submit_started = Instant::now();
        let outcome = host.render_custom::<_, Box<dyn std::error::Error>>(|encoder, target| {
            scene.build(encoder);
            let timing = |offset| {
                self.timestamps
                    .as_ref()
                    .map(|(queries, _)| wgpu::ComputePassTimestampWrites {
                        query_set: queries,
                        beginning_of_pass_write_index: Some(self.frames * 8 + offset),
                        end_of_pass_write_index: Some(self.frames * 8 + offset + 1),
                    })
            };
            frame.encode_with_lighting_timestamps(
                encoder,
                &[(&geometry, &material)],
                [timing(4), timing(6), timing(2)],
            )?;
            motion.encode(encoder);
            expected_depth.encode(encoder);
            current_depth.encode(encoder);
            temporal.encode_depth_attachment(encoder, frame.depth())?;
            resolved.encode_with_timestamps(
                encoder,
                self.timestamps
                    .as_ref()
                    .map(|(queries, _)| wgpu::ComputePassTimestampWrites {
                        query_set: queries,
                        beginning_of_pass_write_index: Some(self.frames * 8),
                        end_of_pass_write_index: Some(self.frames * 8 + 1),
                    }),
            );
            blit.encode_checked(&device, encoder, &color, target)?;
            overlay.encode(encoder, target);
            if let Some((queries, samples)) = &self.timestamps {
                encoder.resolve_query_set(
                    queries,
                    self.frames * 8..self.frames * 8 + 8,
                    samples,
                    u64::from(self.frames) * 256,
                );
            }
            if let Some(copy) = &copy {
                for (offset, source) in [
                    (0, resolved.output()),
                    (256, motion.output()),
                    (512, expected_depth.output()),
                    (768, current_depth.output()),
                    (1024, frame.reflected_radiance()),
                    (1280, frame.direct_radiance()),
                    (1536, temporal.output_depth()),
                ] {
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: source,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: size.width / 2,
                                y: size.height / 2,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: copy,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset,
                                bytes_per_row: Some(256),
                                rows_per_image: Some(1),
                            },
                        },
                        wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                    &device, encoder, copy, 0, 1792,
                )?);
            }
            Ok(())
        })?;
        let submit_ms = submit_started.elapsed().as_secs_f64() * 1000.0;
        if self.smoke
            && outcome != voxy_render::RenderOutcome::Presented
            && self.last_skipped != Some(outcome)
        {
            eprintln!(
                "ANIMATED RAY SKIPPED: attempt={} presented={} outcome={outcome:?} prepare_ms={prepare_ms:.3} submit_ms={submit_ms:.3}",
                self.draw_attempts, self.frames
            );
        }
        self.last_skipped = (outcome != voxy_render::RenderOutcome::Presented).then_some(outcome);
        self.retry_at = (outcome != voxy_render::RenderOutcome::Presented)
            .then(|| Instant::now() + Duration::from_millis(100));
        if outcome == voxy_render::RenderOutcome::Presented {
            if self.smoke || (self.profile && self.frames >= 30) {
                self.prepare_ms.push(prepare_ms);
                self.submit_ms.push(submit_ms);
            }
            history.presented_frame(&candidate)?;
            self.frames += 1;
            self.resized |=
                self.frames > 30 && self.initial_size != Some([size.width, size.height]);
            if let Some(snapshot) = snapshot {
                let mut read = snapshot.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely())?;
                let bytes = read.try_read()?.ok_or("animated readback pending")?;
                let value = |offset| {
                    half::f16::from_bits(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
                        .to_f32()
                };
                let scalar =
                    |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
                let direct = [scalar(1280), scalar(1284), scalar(1288)];
                if !direct.iter().all(|x| x.is_finite() && *x >= 0.0)
                    || !direct.iter().any(|x| *x > 0.0)
                {
                    return Err(format!("invalid direct GGX radiance {direct:?}").into());
                }
                let reflected = [scalar(1024), scalar(1028), scalar(1032)];
                if !reflected.iter().all(|x| x.is_finite() && *x >= 0.0) {
                    return Err(format!("invalid GGX reflection {reflected:?}").into());
                }
                if reflected.iter().any(|x| *x > 0.0) {
                    self.reflection_samples += 1;
                }
                let hdr = [scalar(0), scalar(4), scalar(8)];
                if reset_temporal {
                    for channel in 0..3 {
                        let expected = direct[channel] + reflected[channel];
                        if (hdr[channel] - expected).abs() > expected.abs().max(1.0) * 0.00001 {
                            return Err(format!(
                                "reset retained material history: {} != {expected}",
                                hdr[channel]
                            )
                            .into());
                        }
                    }
                }
                if self.smoke && self.frames == 91 && !reset_temporal {
                    return Err("material switch failed to reset history".into());
                }
                if (scalar(1536) - scalar(768)).abs() > 0.000_001 {
                    return Err("retained history depth differs from current surface".into());
                }
                if (scalar(512) - scalar(768)).abs() > 0.000_001 {
                    self.depth_changes += 1;
                }
                for (actual, projection, vertices) in [
                    (
                        scalar(512),
                        previous_camera,
                        candidate.motion().vertices.as_slice(),
                    ),
                    (scalar(768), camera, current_vertices.as_slice()),
                ] {
                    let expected = center_depth(
                        camera,
                        projection,
                        &candidate.motion().vertices,
                        vertices,
                        [size.width, size.height],
                    )?;
                    if !actual.is_finite() || (actual - expected).abs() > 0.000_001 {
                        return Err(format!("temporal depth {actual} != {expected}").into());
                    }
                }
                if !hdr.iter().all(|x| x.is_finite() && *x >= 0.0) || !hdr.iter().any(|x| *x > 1.0)
                {
                    return Err(format!("invalid animated HDR {hdr:?}").into());
                }
                let movement = [value(256), value(258)];
                if candidate.motion().history_valid && movement.iter().any(|x| x.abs() > 0.000_001)
                {
                    self.moving_samples += 1;
                }
                if !movement.iter().all(|x| x.is_finite()) {
                    return Err("nonfinite motion".into());
                }
                if !candidate.motion().history_valid && movement.iter().any(|x| x.abs() > 0.0001) {
                    return Err("reset motion was nonzero".into());
                }
            }
            self.temporal
                .as_mut()
                .ok_or("missing presented history")?
                .presented();
            self.previous_camera = Some(camera);
            window.set_title(&format!(
                "Voxy ray | frame {} | {} | exposure {:.1} | roughness {:.2} metallic {:.1} | Space / R / G / M / +/-",
                self.frames,
                if self.paused { "paused" } else { "animated" },
                self.exposure,
                self.roughness,
                self.metallic
            ));
            if self.smoke && self.frames == 30 {
                let _ = window.request_inner_size(winit::dpi::LogicalSize::new(480, 360));
            }
            if self.smoke && self.frames == 60 {
                history.reset();
            }
            if self.smoke && self.frames == 90 {
                self.roughness = if self.roughness < 0.6 { 1.0 } else { 0.1 };
                self.metallic = 1.0 - self.metallic;
                self.temporal = None;
                println!(
                    "ANIMATED MATERIAL SWITCH: roughness={} metallic={}",
                    self.roughness, self.metallic
                );
            }
            if self.smoke && self.frames == 105 {
                self.exposure = 2.0;
            }
        }
        self.motion = Some(motion);
        self.expected_depth = Some(expected_depth);
        let (attachments, lighting) = frame.into_all_resources();
        self.reflected = Some(lighting.reflected);
        self.direct = Some(lighting.direct);
        self.combined = Some(lighting.combined);
        self.attachments = Some(attachments);
        self.primary = Some(lighting.primary);
        Ok(())
    }
}
impl ApplicationHandler for Demo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        self.occluded = false;
        event_loop.set_control_flow(ControlFlow::Wait);
        self.reset_presentation_history();
        if self.host.is_some() {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }
        if let Err(error) = self.initialize(event_loop) {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = true;
        self.reset_presentation_history();
        if let Some(host) = &mut self.host
            && let Err(error) = host.resize(0, 0)
        {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
        self.configured_size = Some([0, 0]);
        event_loop.set_control_flow(ControlFlow::Wait);
    }
    #[allow(clippy::too_many_lines)] // Window lifecycle and demo controls are dispatched together.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Occluded(occluded) => {
                    if self.occluded != occluded {
                        self.occluded = occluded;
                        self.reset_presentation_history();
                    }
                    if occluded {
                        let deadline = Instant::now() + Duration::from_millis(100);
                        self.retry_at = Some(deadline);
                        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                    } else {
                        event_loop.set_control_flow(ControlFlow::Wait);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    // Exercise draw-time size reconciliation even when a resize
                    // notification has not yet been handled by the application.
                    if self.smoke && (30..60).contains(&self.frames) {
                        return Ok(());
                    }
                    if let Some(host) = &mut self.host {
                        host.resize(size.width, size.height)?;
                    }
                    if let Some(history) = &mut self.history {
                        history.reset();
                    }
                    self.configured_size = Some([size.width, size.height]);
                    self.temporal = None;
                    self.previous_camera = None;
                    self.started = Some(Instant::now());
                    self.frame_loop.reset();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed && !event.repeat =>
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.paused = !self.paused;
                            self.frame_loop.set_paused(self.paused);
                            self.started = Some(Instant::now());
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            if let Some(history) = &mut self.history {
                                history.reset();
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyG) => {
                            self.roughness = if self.roughness < 0.2 {
                                0.35
                            } else if self.roughness < 0.6 {
                                1.0
                            } else {
                                0.1
                            };
                            self.temporal = None;
                            println!(
                                "MATERIAL: roughness={} metallic={} history reset",
                                self.roughness, self.metallic
                            );
                        }
                        PhysicalKey::Code(KeyCode::KeyM) => {
                            self.metallic = if self.metallic < 0.5 {
                                0.5
                            } else if self.metallic < 1.0 {
                                1.0
                            } else {
                                0.0
                            };
                            self.temporal = None;
                            println!(
                                "MATERIAL: roughness={} metallic={} history reset",
                                self.roughness, self.metallic
                            );
                        }
                        PhysicalKey::Code(KeyCode::Equal) => {
                            self.exposure = (self.exposure + 0.25).min(8.0);
                        }
                        PhysicalKey::Code(KeyCode::Minus) => {
                            self.exposure = (self.exposure - 0.25).max(0.25);
                        }
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.draw()?;
                    if self.profile && self.frames >= 330 {
                        if self.prepare_ms.len() != 300 || self.submit_ms.len() != 300 {
                            return Err("profile sample count mismatch".into());
                        }
                        println!(
                            "ANIMATED PROFILE: warmup=30 measured=300 dimensions={:?} no readback; CPU-side wall clock, not GPU timestamps",
                            self.initial_size
                        );
                        report_cpu_times("prepare_no_readback", &self.prepare_ms);
                        report_cpu_times("encode_submit_present_no_readback", &self.submit_ms);
                        event_loop.exit();
                    } else if self.smoke && self.frames >= 120 {
                        if !self.resized
                            || self.moving_samples == 0
                            || self.depth_changes == 0
                            || self.reflection_samples == 0
                            || self.reused_primary == 0
                            || self.resized_primary == 0
                        {
                            return Err("resize, motion, depth difference or GGX reflection was not observed".into());
                        }
                        println!(
                            "ANIMATED RAY PASS: 120 presentations, retained reflection/direct-light/HDR/depth/motion/raster attachments/primary storage and resize replacement, skeletal BLAS, positive finite direct GGX light and reflections, HDR reprojection, analytic previous/current depths and reset motion, live material switch and unblended history reset"
                        );
                        report_cpu_times("prepare", &self.prepare_ms);
                        report_cpu_times("encode_submit_present", &self.submit_ms);
                        event_loop.exit();
                    } else if let Some(deadline) = self.retry_at {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                    } else if let Some(window) = &self.window {
                        event_loop.set_control_flow(ControlFlow::Wait);
                        window.request_redraw();
                    }
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.suspended {
            return;
        }
        if let Some(deadline) = self.retry_at {
            if Instant::now() >= deadline {
                self.retry_at = None;
                event_loop.set_control_flow(ControlFlow::Wait);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        }
    }
}
#[allow(clippy::too_many_lines)] // CLI, event-loop completion and profile readback orchestration.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut smoke = false;
    let mut profile = false;
    let mut gpu_profile = false;
    let mut opt_in = false;
    let mut roughness = 0.35_f32;
    let mut metallic = 0.5_f32;
    let mut backend = voxy_render::GraphicsBackend::Auto;
    let mut backend_set = false;
    let mut require_nvidia = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--experimental" => opt_in = true,
            "--require-nvidia" if !require_nvidia => require_nvidia = true,
            "--backend" if !backend_set => {
                backend = match arguments.next().as_deref() {
                    Some("auto") => voxy_render::GraphicsBackend::Auto,
                    Some("metal") => voxy_render::GraphicsBackend::Metal,
                    Some("vulkan") => voxy_render::GraphicsBackend::Vulkan,
                    Some("dx12") => voxy_render::GraphicsBackend::DirectX12,
                    _ => return Err("--backend requires auto|metal|vulkan|dx12".into()),
                };
                backend_set = true;
            }
            "--smoke" if !smoke => smoke = true,
            "--profile" if !profile => profile = true,
            "--gpu-profile" if !profile => {
                profile = true;
                gpu_profile = true;
            }
            _ if argument.starts_with("--roughness=") || argument.starts_with("--metallic=") => {
                let (name, value) = argument.split_once('=').ok_or("missing material value")?;
                let value: f32 = value.parse()?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!("{name} must be finite and in [0,1]").into());
                }
                if name == "--roughness" {
                    roughness = value;
                } else {
                    metallic = value;
                }
            }
            _ => return Err(format!("unknown argument {argument}").into()),
        }
    }
    if smoke && profile {
        return Err("--smoke and --profile are mutually exclusive".into());
    }
    if !opt_in {
        return Err("animated_ray requires --experimental".into());
    }
    let mut app = Demo {
        exposure: 1.0,
        roughness,
        metallic,
        smoke,
        profile,
        gpu_profile,
        backend,
        require_nvidia,
        ..Default::default()
    };
    println!("ANIMATED MATERIAL: roughness={roughness} metallic={metallic}");
    EventLoop::new()?.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if smoke && app.frames < 120 {
        return Err("smoke exited before 120 frames".into());
    }
    if profile && app.frames < 330 {
        return Err("profile exited before 300 measured frames".into());
    }
    if gpu_profile {
        let host = app.host.as_ref().ok_or("missing profile device")?;
        let (_, samples) = app.timestamps.as_ref().ok_or("missing GPU samples")?;
        let mut encoder = host
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let snapshot = voxy_render::ComputeDispatch::copy_buffer(
            host.device(),
            &mut encoder,
            samples,
            0,
            330 * 256,
        )?;
        host.queue().submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        host.device().poll(wgpu::PollType::wait_indefinitely())?;
        let bytes = read.try_read()?.ok_or("GPU timestamps pending")?;
        let period = f64::from(host.queue().get_timestamp_period()) / 1_000_000.0;
        // Compare each pass's own start/end, not independent passes that may overlap.
        let mut reports = Vec::new();
        for (stage, start, end) in [
            ("temporal", 0, 1),
            ("hdr_composition", 2, 3),
            ("direct_ggx", 4, 5),
            ("reflection_ggx", 6, 7),
        ] {
            let mut values = Vec::new();
            for frame in 30..330 {
                let timestamp = |index: usize| {
                    u64::from_le_bytes(
                        bytes[frame * 256 + index * 8..frame * 256 + index * 8 + 8]
                            .try_into()
                            .unwrap(),
                    )
                };
                let delta = timestamp(end)
                    .checked_sub(timestamp(start))
                    .ok_or("nonmonotonic GPU timestamps")?;
                #[allow(clippy::cast_precision_loss)]
                // Timestamp conversion to milliseconds is approximate.
                let milliseconds = delta as f64 * period;
                values.push(milliseconds);
            }
            if values.iter().all(|value| *value == 0.0) {
                return Err("GPU timestamp queries returned no elapsed time".into());
            }
            values.sort_by(f64::total_cmp);
            reports.push((stage, values));
        }
        for (stage, values) in reports {
            let unresolved = values.iter().filter(|value| **value == 0.0).count();
            if unresolved != 0 {
                println!(
                    "ANIMATED GPU TIMING UNAVAILABLE: stage={stage} zero_elapsed_samples={unresolved}/300; no percentiles published"
                );
                continue;
            }
            println!(
                "ANIMATED GPU MS: stage={stage} samples=300 p50={:.3} p95={:.3} max={:.3}; timestamp queries, one final readback",
                values[149], values[284], values[299]
            );
        }
    }
    Ok(())
}

fn report_cpu_times(stage: &str, samples: &[f64]) {
    if samples.is_empty() {
        return;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile =
        |percent: usize| sorted[(sorted.len() * percent).div_ceil(100).saturating_sub(1)];
    let Ok(count) = u32::try_from(sorted.len()) else {
        return;
    };
    let mean = sorted.iter().sum::<f64>() / f64::from(count);
    println!(
        "ANIMATED CPU MS: stage={stage} samples={} mean={mean:.3} p50={:.3} p95={:.3} p99={:.3} max={:.3}; excludes explicit readback wait; not GPU time",
        sorted.len(),
        percentile(50),
        percentile(95),
        percentile(99),
        sorted[sorted.len() - 1]
    );
}
