//! Head close-up before and after one second of actual Cosserat hair simulation.
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
#[path = "../src/female_demo.rs"]
mod female_demo;
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
#[path = "../src/female_hair.rs"]
mod female_hair;
#[allow(dead_code)]
#[path = "../src/female_rig.rs"]
mod female_rig;
#[allow(dead_code)]
#[path = "../src/female_transmission.rs"]
mod female_transmission;
#[allow(dead_code)]
#[path = "../src/film_settings.rs"]
mod film_settings;
#[allow(dead_code)]
#[path = "../src/rig_skinning.rs"]
mod rig_skinning;
#[allow(dead_code)]
#[path = "../src/surface_film_preview.rs"]
mod surface_film_preview;
#[allow(dead_code)]
#[path = "../src/volume_regions.rs"]
mod volume_regions;
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut model = female_demo::FemaleDemo::new()?;
    model.animation_only = true;
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let camera = SceneCamera {
        eye: Vec3::new(0.20, 0.72, 0.70),
        target: Vec3::new(0., 0.64, 0.03),
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
    pollster::block_on(renderer.reload_shader(&device, female_eyes::MATERIAL_SHADER))?;
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
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hair inspection readback"),
        size: 2304 * 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut frames = Vec::new();
    for frame in 0..2 {
        if frame == 1 {
            for _ in 0..60 {
                model.advance(1. / 60.)?;
            }
        }
        let geometry = renderer.upload_mesh(&device, &model.mesh()?)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color {
                r: 0.08,
                g: 0.10,
                b: 0.14,
                a: 1.,
            },
            &[SceneDraw {
                geometry: &geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            }],
        );
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
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let mut pixels = Vec::new();
    for y in 0..768 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 2304..(y + 1) * 2304]);
        }
    }
    image::save_buffer(
        "/tmp/voxy-cosserat-hair.png",
        &pixels,
        1152,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("HAIR RENDER PASS: /tmp/voxy-cosserat-hair.png (rest / simulated one second)");
    Ok(())
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hair inspection target"),
        size: wgpu::Extent3d {
            width: 576,
            height: 768,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
