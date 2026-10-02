//! Opt-in ray queries on a window presentation device.
use std::sync::Arc;
use voxy_render::{SceneMesh, SceneSurface, SurfaceOutput};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
#[derive(Default)]
struct Probe {
    done: bool,
    host: Option<SceneSurface>,
    window: Option<Arc<Window>>,
    attempts: u32,
    failure: Option<String>,
}
impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), String> {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes().with_title("Voxy surface ray probe"),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let instance = voxy_render::GraphicsOptions {
                backend: voxy_render::GraphicsBackend::Metal,
                ..Default::default()
            }
            .create_instance_with_display(event_loop.owned_display_handle());
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| e.to_string())?;
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                }))
                .map_err(|e| e.to_string())?;
            drop(surface);
            let host = pollster::block_on(SceneSurface::new_with_adapter_and_output_experimental(
                window.clone(),
                320,
                240,
                &instance,
                adapter,
                wgpu::Features::EXPERIMENTAL_RAY_QUERY,
                SurfaceOutput::Sdr,
                experimental_features(),
            ))
            .map_err(|e| e.to_string())?;
            self.host = Some(host);
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error);
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if event == WindowEvent::RedrawRequested
            && let Some(host) = self.host.as_mut()
        {
            let result = (0..3).try_fold(true, |ready, version| {
                if ready {
                    verify_raster_frame(host, version)
                } else {
                    Ok(false)
                }
            });
            self.attempts += 1;
            match result {
                Ok(true) => {
                    self.done = true;
                    event_loop.exit();
                }
                Ok(false) if self.attempts < 120 => {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                Ok(false) => {
                    self.failure = Some("surface remained unavailable for 120 redraws".into());
                    event_loop.exit();
                }
                Err(error) => {
                    self.failure = Some(error);
                    event_loop.exit();
                }
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !std::env::args().any(|a| a == "--experimental") {
        return Err("pass --experimental to acknowledge experimental ray queries".into());
    }
    let mut probe = Probe::default();
    EventLoop::new()?.run_app(&mut probe)?;
    if let Some(error) = probe.failure {
        return Err(error.into());
    }
    if !probe.done {
        return Err("surface probe did not run".into());
    }
    Ok(())
}
#[allow(unsafe_code)]
fn experimental_features() -> wgpu::ExperimentalFeatures {
    // SAFETY: this isolated optional probe opts into experimental ray queries
    // with controlled geometry; backend implementation bugs can cause UB.
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}
#[allow(clippy::too_many_lines)] // One complete GPU submission and its verification.
fn verify_surface_ray(host: &mut SceneSurface) -> Result<bool, String> {
    let ray = voxy_render::RayScene::from_scene_mesh(host.device(), &SceneMesh::quad([1.0; 4]), 1)
        .map_err(|error| error.to_string())?;
    let segments = [
        voxy_render::RaySegment::new([0.0, 0.0, 1.0], [0.0, 0.0, -1.0], 0.001),
        voxy_render::RaySegment::new([2.0, 0.0, 1.0], [2.0, 0.0, -1.0], 0.001),
    ]
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())?;
    let job = voxy_render::RayVisibilityJob::new(host.device(), &ray, &segments)
        .map_err(|error| error.to_string())?;
    let samples = [[0.0, 0.0, 1.0], [2.0, 0.0, 1.0]]
        .into_iter()
        .map(|position| {
            voxy_render::PointLightSample::new(
                position,
                [0.0, 0.0, -1.0],
                [0.8, 0.4, 0.2],
                [position[0], 0.0, -1.0],
                [100.0; 3],
                0.001,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let lighting = voxy_render::DirectLightingJob::new(host.device(), &ray, [2, 1], &samples)
        .map_err(|error| error.to_string())?;
    let lighting_view = lighting
        .output()
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mirror_material = voxy_render::ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.0)
        .map_err(|error| error.to_string())?;
    let mirror_samples = [[0.0, 0.0, 1.0], [2.0, 0.0, 1.0]]
        .into_iter()
        .map(|position| {
            voxy_render::MirrorSurfaceSample::new(
                &mirror_material,
                position,
                [0.0, 0.0, -1.0],
                [position[0], 0.0, -1.0],
                0.001,
                10.0,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let reflection = voxy_render::SpecularDistancePipeline::new(host.device())
        .map_err(|error| error.to_string())?
        .create_mirror_job(&ray, [2, 1], &mirror_samples, &[[4.0, 2.0, 1.0, 1.0]; 2])
        .map_err(|error| error.to_string())?;
    let reflection_view = reflection
        .incident_radiance()
        .create_view(&wgpu::TextureViewDescriptor::default());
    let shader = host
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ray visibility display"),
            source: wgpu::ShaderSource::Wgsl(
                r"
@group(0) @binding(0) var radiance: texture_2d<f32>;
@group(0) @binding(1) var reflection: texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let p = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    return vec4f(p[i],0,1);
}
@fragment fn fs(@builtin(position) p: vec4f) -> @location(0) vec4f {
    let i = select(0u, 1u, p.x >= 160.0);
    let pixel = vec2i(i32(i), 0);
    let hdr = select(textureLoad(radiance, pixel, 0).rgb, textureLoad(reflection, pixel, 0).rgb, p.y >= 120.0);
    return vec4f(hdr / (vec3f(1) + hdr),1);
}"
                .into(),
            ),
        });
    let pipeline = host
        .device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ray visibility presentation"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: host.color_format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
    let group = host.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ray visibility display inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&lighting_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&reflection_view),
            },
        ],
    });
    let device = host.device().clone();
    let mut snapshot = None;
    let mut light_snapshot = None;
    let mut reflection_snapshot = None;
    let light_copy = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR light verification"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let outcome = host
        .render_custom::<_, Box<dyn std::error::Error>>(|encoder, view| {
            ray.build(encoder);
            job.encode(encoder);
            lighting.encode(encoder);
            reflection.encode(encoder);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: lighting.output(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &light_copy,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 2,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            light_snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                &device,
                encoder,
                &light_copy,
                0,
                16,
            )?);
            snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                &device,
                encoder,
                job.output(),
                0,
                8,
            )?);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: reflection.incident_radiance(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &light_copy,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 2,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            reflection_snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                &device,
                encoder,
                &light_copy,
                0,
                16,
            )?);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present ray visibility"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    if outcome != voxy_render::RenderOutcome::Presented {
        return Ok(false);
    }
    let dispatch = snapshot.ok_or("ray frame did not encode")?;
    let mut reflection_read = reflection_snapshot
        .ok_or("reflection not encoded")?
        .begin_read();
    let mut read = dispatch.begin_read();
    let mut light_read = light_snapshot
        .ok_or("lighting copy was not encoded")?
        .begin_read();
    host.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| error.to_string())?;
    let bytes = read
        .try_read()
        .map_err(|error| error.to_string())?
        .ok_or("surface ray read pending")?;
    if bytes.as_slice() != bytemuck::cast_slice::<u32, u8>(&[0, 1]) {
        return Err("surface ray visibility differs".into());
    }
    let light_bytes = light_read
        .try_read()
        .map_err(|error| error.to_string())?
        .ok_or("HDR lighting read pending")?;
    let values: Vec<_> = light_bytes
        .chunks_exact(2)
        .map(|b| half::f16::from_bits(u16::from_le_bytes([b[0], b[1]])).to_f32())
        .collect();
    let expected = [
        0.0,
        0.0,
        0.0,
        1.0,
        20.0 / std::f32::consts::PI,
        10.0 / std::f32::consts::PI,
        5.0 / std::f32::consts::PI,
        1.0,
    ];
    for (actual, expected) in values.iter().zip(expected) {
        if (actual - expected).abs() > 0.005 {
            return Err(format!("HDR point light mismatch: {values:?}"));
        }
    }
    let reflected = reflection_read
        .try_read()
        .map_err(|error| error.to_string())?
        .ok_or("reflection read pending")?;
    let values: Vec<_> = reflected
        .chunks_exact(2)
        .map(|b| half::f16::from_bits(u16::from_le_bytes([b[0], b[1]])).to_f32())
        .collect();
    for (actual, expected) in values.iter().zip([3.2, 0.8, 0.2, 1.0, 0.0, 0.0, 0.0, 1.0]) {
        if (actual - expected).abs() > 0.005 {
            return Err(format!("mirror HDR mismatch: {values:?}"));
        }
    }
    println!(
        "SURFACE RAY PASS: same submission: indexed BLAS, visibility, HDR point-light and material-weighted mirror compute, tone-mapped display, present; exact shadow and hit/miss radiance"
    );
    Ok(true)
}

#[allow(clippy::too_many_lines)] // Complete raster-to-ray presentation verification.
fn verify_raster_frame(host: &mut SceneSurface, version: u32) -> Result<bool, String> {
    use voxy_render::{
        ReconstructionGuideMesh, ReconstructionGuidePass, ReconstructionMaterial,
        SurfacePointLight, SurfaceReflectionOptions,
    };
    let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
        let device = host.device().clone();
        let mips = std::env::var("VOXY_RAY_TEXTURE_MIPS").as_deref() == Ok("1");
        let srgb = std::env::var("VOXY_RAY_TEXTURE_SRGB").as_deref() == Ok("1");
        let guides = host.create_ray_guides(64, 64)?;
        let pass = ReconstructionGuidePass::for_guides(&device, &guides);
        let camera = glam::camera::rh::proj::directx::perspective(1.0, 1.0, 0.1, 10.0)
            * glam::camera::rh::view::look_at_mat4(
                glam::Vec3::new(0.0, 0.0, 3.0),
                glam::Vec3::ZERO,
                glam::Vec3::Y,
            );
        let material = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("guide texture two colors"),
            size: wgpu::Extent3d {
                width: if mips { 256 } else { 2 },
                height: if mips { 256 } else { 1 },
                depth_or_array_layers: 1,
            },
            mip_level_count: if mips { 9 } else { 1 },
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if srgb {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if mips {
            for level in 0..9 {
                let size = 256u32 >> level;
                let mut bytes = Vec::new();
                for y in 0..size {
                    for x in 0..size {
                        let gray = if level == 0 {
                            if (x + y) % 2 == 0 { 0 } else { 255 }
                        } else {
                            128
                        };
                        bytes.extend_from_slice(&if level == 0 {
                            [gray, gray, gray, 255]
                        } else {
                            [64, 128, 192, 255]
                        });
                    }
                }
                host.queue().write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &material,
                        mip_level: level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size * 4),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        } else {
            host.queue().write_texture(
                material.as_image_copy(),
                &[128, 255, 64, 255, 255, 64, 255, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(1),
                },
                material.size(),
            );
        }
        let linear = mips || std::env::var("VOXY_RAY_TEXTURE_LINEAR").as_deref() == Ok("1");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: if linear {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            mag_filter: if linear {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            mipmap_filter: if mips {
                wgpu::MipmapFilterMode::Linear
            } else {
                wgpu::MipmapFilterMode::Nearest
            },
            anisotropy_clamp: if mips { 4 } else { 1 },
            ..Default::default()
        });
        let placeholder = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("textured primary distance placeholder"),
            size: guides.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let inputs = pass.textured_inputs(
            &device,
            camera,
            [0.0, 0.0, 3.0],
            &placeholder,
            &material,
            &sampler,
        )?;
        let model = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(2.0, 0.5, 1.5),
            glam::Quat::from_rotation_x(0.7),
            glam::Vec3::new(0.3, -0.2, 0.8),
        );
        let inverse = model.inverse();
        let scene_mesh = SceneMesh::new(
            [[-1.0, -1.0, 0.5], [3.0, -1.0, 0.5], [-1.0, 3.0, 0.5]]
                .into_iter()
                .map(|position| voxy_render::SceneVertex {
                    position: inverse
                        .transform_point3(glam::Vec3::from_array(position))
                        .to_array(),
                    uv: [(position[0] + 1.0) / 4.0, (position[1] + 1.0) / 4.0],
                    color: [0.8, 0.4, 0.2, 1.0],
                })
                .collect(),
            vec![0, 1, 2],
        )?;
        ReconstructionGuideMesh::from_scene_mesh(&device, &scene_mesh, model, 0.5, 0.0)?;
        for invalid in [
            glam::Mat4::from_scale(glam::Vec3::new(1.0, 0.0, 1.0)),
            camera,
        ] {
            if ReconstructionGuideMesh::from_scene_mesh(&device, &scene_mesh, invalid, 0.5, 0.0)
                .is_ok()
            {
                return Err("invalid model transform was accepted".into());
            }
        }
        let make_mesh = |positions: &[[f32;3]]| -> Result<ReconstructionGuideMesh, Box<dyn std::error::Error>> {
            let vertices = positions.iter().map(|position| voxy_render::SceneVertex { position: inverse.transform_point3(glam::Vec3::from_array(*position)).to_array(), uv: [(position[0]+1.0)/4.0, (position[1]+1.0)/4.0], color: [0.8,0.4,0.2,1.0] }).collect();
            let indices = (0..u32::try_from(positions.len())?).collect();
            Ok(ReconstructionGuideMesh::from_scene_mesh(&device, &SceneMesh::new(vertices, indices)?, model, 0.5, 0.0)?)
        };
        let left_mesh = make_mesh(&[
            [-1.0, -1.0, 0.5],
            [0.0, -1.0, 0.5],
            [0.0, 2.0, 0.5],
            [-1.0, -1.0, 0.5],
            [0.0, 2.0, 0.5],
            [-1.0, 3.0, 0.5],
        ])?;
        let right_mesh = make_mesh(&[[0.0, -1.0, 0.5], [3.0, -1.0, 0.5], [0.0, 2.0, 0.5]])?;
        let renderer = host.create_scene_renderer();
        let mut white = renderer.upload_texture(&device, host.queue(), 1, 1, &[255; 4])?;
        if version == 0 {
            let instance = voxy_render::GraphicsOptions {
                backend: voxy_render::GraphicsBackend::Metal,
                ..Default::default()
            }
            .create_instance();
            let adapter = pollster::block_on(
                instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
            )?;
            let (foreign_device, foreign_queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::EXPERIMENTAL_RAY_QUERY,
                    required_limits: adapter.limits(),
                    experimental_features: experimental_features(),
                    ..Default::default()
                }))?;
            let clone = device.clone();
            if device != clone || device == foreign_device {
                return Err("native device identity is not context-aware or clone-stable".into());
            }
            #[allow(clippy::mutable_key_type)]
            // Device identity is immutable despite internal GPU state.
            let mut devices = std::collections::HashSet::new();
            devices.insert(device.clone());
            devices.insert(clone);
            devices.insert(foreign_device.clone());
            if devices.len() != 2 {
                return Err("native device equality/hash contract failed".into());
            }
            let foreign_scene =
                voxy_render::RayScene::from_scene_mesh(&foreign_device, &scene_mesh, 1)?;
            let mismatch =
                voxy_render::RayVisibilityPipeline::new(&device)?.create_job(&foreign_scene, &[]);
            if !matches!(mismatch, Err(voxy_render::RaySceneError::DeviceMismatch)) {
                return Err("foreign BLAS/TLAS was not rejected before ray allocation".into());
            }
            let foreign_guides =
                voxy_render::RayReconstructionGuides::new(&foreign_device, &adapter, 64, 64)?;
            let foreign_depth = foreign_device.create_texture(&wgpu::TextureDescriptor {
                label: Some("foreign primary depth"),
                size: foreign_guides.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let foreign_primary = voxy_render::PrimarySurfaceJob::new(
                &foreign_device,
                &foreign_depth,
                foreign_guides.normal_roughness(),
                camera,
                1.0,
            )?;
            if !matches!(
                voxy_render::PrimaryMotionPass::new(
                    &device,
                    &foreign_primary,
                    camera,
                    camera,
                    false
                ),
                Err(voxy_render::RaySceneError::DeviceMismatch)
            ) {
                return Err("foreign primary buffer accepted by motion consumer".into());
            }
            let foreign_renderer = voxy_render::SceneRenderer::new(
                &foreign_device,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            );
            let foreign_material = foreign_renderer.upload_texture(
                &foreign_device,
                &foreign_queue,
                1,
                1,
                &[255; 4],
            )?;
            if pass
                .scene_material_inputs(
                    &device,
                    camera,
                    [0.0, 0.0, 3.0],
                    &placeholder,
                    &foreign_material,
                )
                .is_ok()
                || pass
                    .scene_material_inputs(
                        &foreign_device,
                        camera,
                        [0.0, 0.0, 3.0],
                        &placeholder,
                        &white,
                    )
                    .is_ok()
            {
                return Err("foreign material/device accepted by guide pass".into());
            }
        }
        let right_inputs =
            pass.scene_material_inputs(&device, camera, [0.0, 0.0, 3.0], &placeholder, &white)?;
        if host
            .replace_scene_texture(
                &renderer,
                &mut white,
                1,
                1,
                &[255; 3],
                voxy_render::TextureSampling::default(),
            )
            .is_ok()
        {
            return Err("invalid material replacement accepted".into());
        }
        if version > 0 {
            host.replace_scene_texture(
                &renderer,
                &mut white,
                1,
                1,
                &[128, 255, 64, 255],
                voxy_render::TextureSampling::default(),
            )?;
        }
        let right_inputs = if version == 1 {
            pass.scene_material_inputs(&device, camera, [0.0, 0.0, 3.0], &placeholder, &white)?
        } else {
            right_inputs
        };
        let scene = voxy_render::RayScene::new(
            &device,
            &[[-10.0, -10.0, 3.0], [10.0, -10.0, 3.0], [0.0, 10.0, 3.0]],
        )?;
        let lighting = voxy_render::RasterRayFrame::new(
            &device,
            host.adapter(),
            &scene,
            voxy_render::RasterRayOptions {
                dimensions: [64, 64],
                view_projection: camera,
                clear_depth: 1.0,
                light: SurfacePointLight {
                    position: [0.0, 0.0, 2.0],
                    intensity: [10.0; 3],
                    bias: 0.001,
                },
                reflection: SurfaceReflectionOptions {
                    material: ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)?,
                    camera: [0.0, 0.0, 3.0],
                    bias: 0.001,
                    maximum_distance: 10.0,
                },
            },
            &[[4.0, 1.0, 0.5, 1.0]],
        )?;
        let motion = lighting.object_motion(
            camera,
            &[
                [
                    glam::Mat4::IDENTITY,
                    glam::Mat4::from_translation(glam::Vec3::new(-0.1, 0.0, 0.0)),
                ],
                [
                    glam::Mat4::IDENTITY,
                    glam::Mat4::from_translation(glam::Vec3::new(0.1, 0.0, 0.0)),
                ],
            ],
            version == 2,
        )?;
        let deformation = std::env::var("VOXY_RAY_DEFORMATION").as_deref() == Ok("1");
        let paired = [
            [-1.0, -1.0, 0.5],
            [0.0, -1.0, 0.5],
            [0.0, 2.0, 0.5],
            [-1.0, -1.0, 0.5],
            [0.0, 2.0, 0.5],
            [-1.0, 3.0, 0.5],
            [0.0, -1.0, 0.5],
            [3.0, -1.0, 0.5],
            [0.0, 2.0, 0.5],
        ]
        .map(|position| {
            let current = model
                .transform_point3(inverse.transform_point3(glam::Vec3::from_array(position)))
                .to_array();
            voxy_render::PreviousPositionVertex {
                current,
                // A shear varies displacement continuously over the surface.
                previous: [current[0] + 0.05 * current[1], current[1], current[2]],
            }
        });
        let deformed_motion = lighting.deformation_motion(camera, &paired, version == 2)?;
        let mut motion_snapshot = None;
        let blit = voxy_render::TextureBlit::tone_mapped(&device, host.color_format(), 1.0)
            .ok_or("tone mapping unavailable")?;
        let color = lighting
            .output()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let pixel_copy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster HDR full frame copy"),
            size: 32768,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut snapshot = None;
        let outcome = host.render_custom::<_, Box<dyn std::error::Error>>(|encoder, target| {
            scene.build(encoder);
            lighting.encode(
                encoder,
                &[(&left_mesh, &inputs), (&right_mesh, &right_inputs)],
            )?;
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: lighting.output(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &pixel_copy,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(512),
                        rows_per_image: Some(64),
                    },
                },
                wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
            );
            snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                &device,
                encoder,
                &pixel_copy,
                0,
                32768,
            )?);
            lighting.encode_object_ids(encoder, &[(&left_mesh, 0), (&right_mesh, 1)])?;
            let motion_texture = if deformation {
                deformed_motion.encode(encoder);
                deformed_motion.output()
            } else {
                motion.encode(encoder);
                motion.output()
            };
            encoder.copy_texture_to_buffer(
                motion_texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &pixel_copy,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                motion_texture.size(),
            );
            motion_snapshot = Some(voxy_render::ComputeDispatch::copy_buffer(
                &device,
                encoder,
                &pixel_copy,
                0,
                16384,
            )?);
            blit.encode(&device, encoder, &color, target);
            Ok(())
        })?;
        if outcome != voxy_render::RenderOutcome::Presented {
            return Ok(false);
        }
        let mut motion_read = motion_snapshot
            .ok_or("raster motion was not encoded")?
            .begin_read();
        let mut read = snapshot
            .ok_or("raster HDR copy was not encoded")?
            .begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes = read.try_read()?.ok_or("raster HDR read pending")?;
        let motion_bytes = motion_read
            .try_read()?
            .ok_or("raster motion read pending")?;
        let mut covered = 0;
        let mut background = 0;
        for (index, pixel) in bytes.chunks_exact(8).enumerate() {
            let values: Vec<_> = pixel
                .chunks_exact(2)
                .map(|b| half::f16::from_bits(u16::from_le_bytes([b[0], b[1]])).to_f32())
                .collect();
            let column = f32::from(u16::try_from(index % 64)?);
            let row = f32::from(u16::try_from(index / 64)?);
            let x = ((column + 0.5) / 32.0 - 1.0) * 2.5 * 0.5_f32.tan();
            let y = (1.0 - (row + 0.5) / 32.0) * 2.5 * 0.5_f32.tan();
            let mut expected = [0.0, 0.0, 0.0, 1.0];
            if x >= -1.0 && y >= -1.0 && x + y <= 2.0 {
                covered += 1;
                let radius2 = x * x + y * y;
                let irradiance = 15.0 / (std::f32::consts::PI * (2.25 + radius2).powf(1.5));
                let fresnel = (1.0 - 2.5 / (6.25 + radius2).sqrt()).powi(5);
                for channel in 0..3 {
                    let decode = |value: f32| {
                        if !srgb {
                            value
                        } else if value <= 0.04045 {
                            value / 12.92
                        } else {
                            ((value + 0.055) / 1.055).powf(2.4)
                        }
                    };
                    let left = [128.0 / 255.0, 1.0, 64.0 / 255.0].map(decode);
                    let right = [1.0, 64.0 / 255.0, 1.0].map(decode);
                    let blend = if linear {
                        (x * 0.5).clamp(0.0, 1.0)
                    } else if x < 1.0 {
                        0.0
                    } else {
                        1.0
                    };
                    let tint = std::array::from_fn::<_, 3, _>(|i| {
                        left[i] * (1.0 - blend) + right[i] * blend
                    });
                    let tint = if mips {
                        [64.0 / 255.0, 128.0 / 255.0, 192.0 / 255.0].map(decode)
                    } else {
                        tint
                    };
                    let diffuse = [0.4, 0.2, 0.1][channel]
                        * if x >= 0.0 {
                            if version == 1 {
                                [128.0_f32 / 255.0, 1.0, 64.0 / 255.0].map(|value| {
                                    if value <= 0.04045 {
                                        value / 12.92
                                    } else {
                                        ((value + 0.055) / 1.055).powf(2.4)
                                    }
                                })[channel]
                            } else {
                                1.0
                            }
                        } else {
                            tint[channel]
                        };
                    let f0 = 0.02 + diffuse;
                    let emission = [4.0, 1.0, 0.5][channel];
                    expected[channel] =
                        diffuse * irradiance + emission * (f0 + (1.0 - f0) * fresnel);
                }
            } else {
                background += 1;
            }
            let moving = x >= -1.0 && y >= -1.0 && x + y <= 2.0 && version != 2;
            let expected_motion = if moving {
                (if deformation {
                    0.05 * y
                } else if x < 0.0 {
                    -0.1
                } else {
                    0.1
                }) / (5.0 * 0.5_f32.tan())
            } else {
                0.0
            };
            let offset = index * 4;
            for channel in 0..2 {
                let offset = offset + channel * 2;
                let value = half::f16::from_bits(u16::from_le_bytes([
                    motion_bytes[offset],
                    motion_bytes[offset + 1],
                ]))
                .to_f32();
                let expected = if channel == 0 { expected_motion } else { 0.0 };
                if !value.is_finite() || (value - expected).abs() > 0.0001 {
                    return Err(format!(
                        "raster motion mismatch ({column},{row}): {value} != {expected}"
                    )
                    .into());
                }
            }
            for (actual, expected) in values.iter().zip(expected) {
                if !actual.is_finite() || (actual - expected).abs() > 0.01 {
                    return Err(format!("raster HDR pixel ({column},{row}) mismatch: {values:?}, expected {expected}").into());
                }
            }
        }
        if covered == 0 || background == 0 || covered + background != 4096 {
            return Err("full frame verification omitted pixels".into());
        }
        println!(
            "RASTER HDR PIXELS PASS: {covered} shaded, {background} background; sRGB={srgb}; material version={version}"
        );
        println!(
            "RASTER RAY WINDOW PASS: guides -> primary -> lighting/reflection -> HDR -> presentation"
        );
        verify_surface_ray(host).map_err(Into::into)
    })();
    result.map_err(|error| error.to_string())
}
