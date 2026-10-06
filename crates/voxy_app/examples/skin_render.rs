#[allow(dead_code)]
#[path = "../src/body_parameters.rs"]
mod body_parameters;
#[allow(dead_code)]
#[path = "../src/face_parameters.rs"]
mod face_parameters;
#[allow(dead_code)]
#[path = "../src/female_complexion.rs"]
mod female_complexion;
#[allow(dead_code)]
#[path = "../src/female_eyes.rs"]
mod female_eyes;
#[allow(dead_code)]
#[path = "../src/female_face.rs"]
mod female_face;
#[allow(dead_code)]
#[path = "../src/female_features.rs"]
mod female_features;
#[allow(dead_code)]
#[path = "../src/film_settings.rs"]
mod film_settings;
#[path = "../src/rig_skinning.rs"]
mod rig_skinning;
// Renders the supplied full-body physical skin and its displacement map offscreen.
// Run with VOXY_SKIN_ONLY=1 to isolate skin from optional hair dynamics.
#[allow(dead_code)]
#[path = "../src/female_demo.rs"]
mod female_demo;
#[allow(dead_code)]
#[path = "../src/female_hair.rs"]
mod female_hair;
#[allow(dead_code)]
#[path = "../src/female_rig.rs"]
mod female_rig;
#[allow(dead_code)]
#[path = "../src/female_transmission.rs"]
mod female_transmission;
#[allow(dead_code)]
#[path = "../src/surface_film_preview.rs"]
mod surface_film_preview;
#[allow(dead_code)]
#[path = "../src/volume_regions.rs"]
mod volume_regions;
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut model = female_demo::FemaleDemo::new()?;
    let sequence = std::env::args().any(|a| a == "--sequence");
    let frame_step = std::env::args().any(|a| a == "--frame-step");
    let sequence_directory = "/tmp/voxy-full-character-frames";
    if sequence {
        std::fs::create_dir_all(sequence_directory)?;
    }
    model.show_complexion = !std::env::args().any(|a| a == "--bare");
    model.animation_only = false;
    let probe = std::env::args().any(|a| a == "--probe");
    model.probe_enabled = probe;
    if probe {
        model.pressing = false;
    }
    let msaa = !std::env::args().any(|a| a == "--no-msaa");
    let face = std::env::args().any(|a| a == "--face");
    let strain = std::env::args().any(|a| a == "--strain");
    let side = std::env::args().any(|a| a == "--side");
    model.preview_camera_eye = Some(if face {
        Vec3::new(0., 0.715, 0.57)
    } else if side {
        Vec3::new(2.4, 0.1, 0.4)
    } else {
        Vec3::new(0.4, 0.1, 2.4)
    });
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Model smoke on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    if msaa {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Depth32Float,
        ] {
            if !adapter
                .get_texture_format_features(format)
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            {
                return Err("four-sample character rendering unsupported".into());
            }
        }
    }
    let mut renderer = if msaa {
        SceneRenderer::new_msaa4(&device, wgpu::TextureFormat::Rgba8Unorm)
    } else {
        SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm)
    };
    pollster::block_on(renderer.reload_shader(&device, female_eyes::MATERIAL_SHADER))?;
    let mesh = model.mesh()?;
    let mut geometry = renderer.upload_mesh(&device, &mesh)?;
    let texture = renderer.upload_texture_with_sampling(
        &device,
        &queue,
        female_complexion::WIDTH,
        female_complexion::SIZE,
        female_complexion::atlas(),
        voxy_render::TextureSampling {
            min_filter: voxy_render::TextureFilter::Linear,
            mag_filter: voxy_render::TextureFilter::Linear,
            ..Default::default()
        },
    )?;
    let camera = SceneCamera {
        eye: if probe {
            Vec3::new(0.20, 0.18, 0.65)
        } else if face {
            Vec3::new(0., 0.715, 0.57)
        } else if side {
            Vec3::new(2.4, 0.1, 0.4)
        } else {
            Vec3::new(0.4, 0.1, 2.4)
        },
        target: if probe {
            Vec3::new(0., 0.16, 0.1)
        } else if face {
            Vec3::new(0., 0.70, 0.12)
        } else {
            Vec3::ZERO
        },
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45f32.to_radians(),
            aspect: 0.75,
            near: 0.01,
            far: 10.,
        },
    };
    let transform = renderer.create_transform(&device, camera.view_projection()?)?;
    transform.update_view_position(&queue, camera.eye)?;
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
    );
    let multisample = msaa.then(|| {
        (
            target(
                &device,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                4,
            ),
            target(
                &device,
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                4,
            ),
        )
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("model readback"),
        size: 768 * 2304,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut frames = Vec::new();
    let mut mesh_ms = Vec::new();
    let mut step_ms = Vec::new();
    let mut solver_ms = [Vec::new(), Vec::new()];
    for frame in 0..if sequence { 120 } else { 2 } {
        for _ in 0..if sequence {
            if frame_step { 1 } else { 6 }
        } else if probe {
            40
        } else {
            30
        } {
            let started = std::time::Instant::now();
            model.advance(if sequence && frame_step {
                0.05
            } else {
                1.0 / 120.0
            })?;
            step_ms.push(started.elapsed().as_secs_f64() * 1000.);
            for (samples, elapsed) in solver_ms.iter_mut().zip(model.solver_ms) {
                samples.push(elapsed);
            }
        }
        // The smoke verifier requires eight physical steps; the first sequence
        // sample contains six. Every subsequent sample validates the full state.
        if !sequence || model.steps >= 8 {
            model.verify()?;
        }
        model.show_skin = !sequence && frame == 1 && !strain;
        model.show_strain = !sequence && frame == 1 && strain;
        let started = std::time::Instant::now();
        geometry.update(&queue, &model.mesh()?)?;
        mesh_ms.push(started.elapsed().as_secs_f64() * 1000.);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let output = color.create_view(&wgpu::TextureViewDescriptor::default());
        let clear = wgpu::Color {
            r: 0.08,
            g: 0.10,
            b: 0.14,
            a: 1.,
        };
        let draws = [SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        if let Some((ms_color, ms_depth)) = &multisample {
            renderer.encode_msaa4(
                &mut encoder,
                &ms_color.create_view(&wgpu::TextureViewDescriptor::default()),
                &ms_depth.create_view(&wgpu::TextureViewDescriptor::default()),
                &output,
                clear,
                &draws,
            )?;
        } else {
            renderer.encode(
                &mut encoder,
                &output,
                &depth.create_view(&wgpu::TextureViewDescriptor::default()),
                clear,
                &draws,
            );
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(2304),
                    rows_per_image: Some(768),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        assert!(
            pixels
                .chunks_exact(4)
                .filter(|p| p[0] > 70 || p[2] > 150)
                .count()
                > 1000,
            "body missing from render"
        );
        if sequence {
            image::save_buffer(
                format!("{sequence_directory}/{frame:04}.png"),
                &pixels,
                576,
                768,
                image::ColorType::Rgba8,
            )?;
        } else {
            frames.push(pixels.to_vec());
        }
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    for (name, samples) in ["Skin", "Hair"].into_iter().zip(&solver_ms) {
        println!(
            "{name} solver CPU: mean {:.2} ms",
            samples.iter().sum::<f64>() / samples.len() as f64
        );
    }
    mesh_ms.sort_by(f64::total_cmp);
    let mean_step = step_ms.iter().sum::<f64>() / step_ms.len() as f64;
    step_ms.sort_by(f64::total_cmp);
    println!(
        "Physical step CPU: {} samples, mean {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        step_ms.len(),
        mean_step,
        step_ms[step_ms.len() * 95 / 100],
        step_ms.last().unwrap()
    );
    println!(
        "Mesh update CPU: median {:.2} ms, max {:.2} ms",
        mesh_ms[mesh_ms.len() / 2],
        mesh_ms.last().unwrap()
    );
    if sequence {
        println!(
            "PASS: full physical character rendered across {} steps and the six-second rig loop",
            model.steps
        );
        return Ok(());
    }
    let mut pixels = Vec::new();
    for y in 0..768 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 2304..(y + 1) * 2304]);
        }
    }
    image::save_buffer(
        if probe {
            "/tmp/voxy-skin-probe-preview.png"
        } else if side {
            "/tmp/voxy-skin-side-preview.png"
        } else {
            if strain {
                "/tmp/voxy-skin-strain-preview.png"
            } else {
                "/tmp/voxy-skin-preview.png"
            }
        },
        &pixels,
        1152,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("PASS: full-body physical skin and selected diagnostic rendered");
    Ok(())
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model target"),
        size: wgpu::Extent3d {
            width: 576,
            height: 768,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
