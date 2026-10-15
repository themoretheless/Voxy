//! Full immutable polygon scene AA qualification on Metal, without a window.
#[path = "street_walk/aa_validation.rs"]
mod aa_validation;
#[path = "street_walk/materials.rs"]
mod materials;
#[path = "street_walk/original.rs"]
mod original;
#[path = "street_walk/scene.rs"]
mod scene;
use image::ImageEncoder;
use std::{io::Write, path::Path, time::Instant};
use voxy_render::{ArchiveAntialiasing, archive_scene::ArchiveSceneRenderer};
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let stride = (WIDTH * 8).div_ceil(256) * 256;
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("full polygon HDR readback"),
        size: u64::from(stride) * u64::from(HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(HEIGHT),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    let mut rgb = Vec::<f32>::with_capacity((WIDTH * HEIGHT * 3) as usize);
    let mut rgba = Vec::<u8>::with_capacity((WIDTH * HEIGHT * 4) as usize);
    let mut peak = 0_f32;
    let mut above = 0;
    for row in bytes.chunks_exact(stride as usize) {
        for p in row[..(WIDTH * 8) as usize].chunks_exact(8) {
            let colour: [f32; 3] = std::array::from_fn(|c| {
                half::f16::from_bits(u16::from_le_bytes([p[c * 2], p[c * 2 + 1]])).to_f32()
            });
            if !colour.iter().all(|v| v.is_finite()) {
                return Err("nonfinite HDR polygon output".into());
            }
            if colour.iter().any(|v| *v > 1.) {
                above += 1;
            }
            for x in colour {
                rgb.push(x);
                peak = peak.max(x);
                let x = x.max(0.);
                let x = ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0., 1.);
                let encoded = if x <= 0.0031308 {
                    12.92 * x
                } else {
                    1.055 * x.powf(1. / 2.4) - 0.055
                };
                rgba.push((encoded * 255.).round().clamp(0., 255.) as u8);
            }
            rgba.push(255);
        }
    }
    drop(bytes);
    read.unmap();
    let png = std::io::BufWriter::new(std::fs::File::create(path)?);
    image::codecs::png::PngEncoder::new_with_quality(
        png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(&rgba, WIDTH, HEIGHT, image::ExtendedColorType::Rgba8)?;
    let mut pfm = std::io::BufWriter::new(std::fs::File::create(path.with_extension("pfm"))?);
    write!(pfm, "PF\n{WIDTH} {HEIGHT}\n-1.0\n")?;
    for row in rgb.chunks_exact((WIDTH * 3) as usize).rev() {
        pfm.write_all(bytemuck::cast_slice(row))?;
    }
    pfm.flush()?;
    std::fs::write(
        path.with_extension("hdr.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"finite":true,"pixels":WIDTH*HEIGHT,"render_headroom":2.,"peak_relative_sdr_white":peak,"pixels_above_sdr_white":above,"surface_format":"Rgba16Float","png_is_sdr_preview":true}),
        )?,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("pass a capture root directory")?;
    let root = Path::new(&root);
    std::fs::create_dir_all(root)?;
    let mut scene = original::generate(0x564f5859, original::Ageing::default())?;
    let count = scene.collision.triangle_count();
    let rendered: usize = scene
        .geometry
        .iter()
        .map(|m| m.indices.len() / 3 * m.instances.len())
        .sum();
    assert_eq!(
        count, rendered,
        "qualification must render every collision triangle"
    );
    let mut route = aa_validation::AaValidation {
        directory: root.into(),
        eye: scene.eye,
        direction: scene.direction,
        frame_ms: Vec::new(),
        cycle_metalfx: false,
    };
    let (environment, sun, sun_colour) = materials::daylight()?;
    scene.sun = sun;
    scene.sun_colour = sun_colour;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    eprintln!(
        "POLYGON AA adapter={:?} triangles={rendered}",
        adapter.get_info()
    );
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("full polygon AA qualification"),
        required_features: wgpu::Features::TEXTURE_COMPRESSION_BC
            | wgpu::Features::FLOAT32_FILTERABLE,
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: 16384,
            max_storage_buffer_binding_size: adapter.limits().max_storage_buffer_binding_size,
            ..Default::default()
        },
        ..Default::default()
    }))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scale: u32 = std::env::var("VOXY_STREET_RENDER_SCALE").unwrap_or_else(|_| "1".into()).parse()?;
    let resolve = voxy_render::SupersampleResolve::new(&device, WIDTH, HEIGHT, scale, wgpu::TextureFormat::Rgba16Float)?;
    let (render_width, render_height) = resolve.render_size();
    let mut renderer = ArchiveSceneRenderer::new_with_environment(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba16Float,
        render_width,
        render_height,
        scene.images,
        scene.materials,
        scene.geometry,
        &scene.lights,
        scene.sun,
        scene.sun_colour,
        scene.bounds,
        Some(environment),
    )?;
    renderer.set_display_headroom(2.)?;
    renderer.set_weather_time(0.);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("full polygon AA output"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());
    for (name, mode) in [
        ("off", ArchiveAntialiasing::Off),
        ("temporal", ArchiveAntialiasing::Temporal),
        ("metalfx", ArchiveAntialiasing::MetalFx),
    ] {
        route.directory = root.join(name);
        std::fs::create_dir_all(&route.directory)?;
        route.frame_ms.clear();
        renderer.set_antialiasing(mode)?;
        eprintln!("POLYGON AA mode={mode:?}");
        for frame in 0..160 {
            if frame == 128 {
                renderer.reset_temporal_history();
            }
            let (eye, direction) = route.camera(frame);
            let start = Instant::now();
            let submission = renderer.draw_supersampled(&resolve, &view, eye, direction)?;
            device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
            route.frame_ms.push(start.elapsed().as_secs_f64() * 1000.);
            if aa_validation::AaValidation::capture(frame + 1) {
                capture(
                    &device,
                    &queue,
                    &output,
                    &route.directory.join(format!("frame-{:03}.png", frame + 1)),
                )?;
            }
        }
        let report = serde_json::json!({"scene":"Original Street","antialiasing":format!("{mode:?}"),"adapter":format!("{:?}",adapter.get_info()),"resolution":[WIDTH,HEIGHT],"render_scale":scale,"internal_resolution":[render_width,render_height],"format":"Rgba16Float","render_headroom":2.,"collision_triangles":count,"rendered_triangles":rendered,"source_simplified":false,"frames":160,"route":"static 1-32; rotation 33-64; translation 65-96; cut 97; explicit reset without moving camera 129","completed_frame_timings":true,"frame_ms":route.frame_ms,"windowless":true});
        std::fs::write(
            route.directory.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        eprintln!("POLYGON AA mode={mode:?} completed 160 frames");
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.to_string().into());
    }
    eprintln!("POLYGON AA all modes passed Metal validation");
    Ok(())
}
