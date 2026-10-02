//! Offscreen GPU evidence for the same overlay meshes used by game-check.
use super::*;
use voxy_gameplay::{UiElement, UiText};
use voxy_render::{GraphicsOptions, SceneDraw};
use voxy_scene::{SceneGraph, Transform};
use voxy_text::{FontLimits, TextFont};

#[test]
#[ignore = "requires a real GPU adapter and local TrueType font"]
#[allow(clippy::too_many_lines)]
fn gpu_overlay_preserves_glyph_alpha_and_clip() {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU required for UI pixel validation");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(renderer.reload_shader(&device, include_str!("../material.wgsl"))).unwrap();
    let bytes = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ]
    .iter()
    .find_map(|path| std::fs::read(path).ok())
    .expect("local TrueType font");
    let font = TextFont::parse(
        &bytes,
        FontLimits {
            max_font_bytes: 4 * 1024 * 1024,
            max_glyph_pixels: 16384,
            max_size: 128.0,
        },
    )
    .unwrap();
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            owner,
            UiElement {
                origin: [0.25, 0.25],
                size: [0.0625, 0.5],
                color: [0.0; 4],
                layer: 0,
                enabled: true,
                action: None,
                text: Some(UiText {
                    font: "font".into(),
                    content: "W".into(),
                    size: 40.0,
                    color: [0.0, 1.0, 0.0, 1.0],
                }),
            },
        )
        .unwrap();
    let snapshot = voxy_gameplay::extract_scene_ui(&scene, [128.0; 2], 128).unwrap();
    let text = voxy_gameplay::prepare_ui_text(&snapshot, |_| Some(&font)).unwrap();
    let meshes = build(&snapshot, &text).unwrap();
    assert_eq!(meshes.len(), 1);
    let mut cache = crate::gpu_model::ResidencyCache::with_budget(16 * 1024 * 1024);
    let published =
        crate::ui_live::upload_on(&renderer, &device, &queue, &mut cache, &meshes).unwrap();
    let reused =
        crate::ui_live::upload_on(&renderer, &device, &queue, &mut cache, &meshes).unwrap();
    assert!(Arc::ptr_eq(
        published[0].texture.as_ref().unwrap(),
        reused[0].texture.as_ref().unwrap()
    ));
    cache.geometry_live = published[0].geometry.allocation_bytes();
    cache.geometry_budget = cache.geometry_live;
    assert!(
        crate::ui_live::upload_on(&renderer, &device, &queue, &mut cache, &meshes)
            .unwrap_err()
            .to_string()
            .contains("peak budget")
    );
    assert!(published[0].geometry.index_count() > 0);
    cache.budget = cache.live_bytes();
    let additional_image = voxy_render::ImageAsset::from_rgba(
        1,
        1,
        vec![255, 0, 0, 255],
        voxy_render::ImageLimits::default(),
    )
    .unwrap();
    assert!(cache.preflight_images(&[additional_image]).is_err());

    let atlas = published[0].texture.as_ref().unwrap();
    let geometry = &published[0].geometry;
    let transform = renderer
        .create_transform(&device, glam::Mat4::IDENTITY)
        .unwrap();
    let output = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    renderer.encode(
        &mut encoder,
        &output.create_view(&wgpu::TextureViewDescriptor::default()),
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        },
        &[SceneDraw {
            geometry,
            texture: atlas,
            transform: &transform,
            overlay: true,
        }],
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("UI pixels"),
        size: 128 * 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512),
                rows_per_image: Some(128),
            },
        },
        wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range().unwrap();
    let mut covered = 0;
    let mut partial = 0;
    let mut transparent = 0;
    for y in 0..128 {
        for x in 0..128 {
            let pixel = &pixels[y * 512 + x * 4..y * 512 + x * 4 + 4];
            if !(32..40).contains(&x) || !(32..96).contains(&y) {
                assert_eq!(pixel, &[0, 0, 255, 255], "clip leaked at {x},{y}");
            } else if pixel[1] > 0 {
                covered += 1;
                if pixel[1] < 255 && pixel[2] > 0 {
                    partial += 1;
                }
            } else {
                transparent += 1;
            }
        }
    }
    assert!(covered > 10, "no visible glyph pixels");
    assert!(partial > 0, "glyph antialias alpha lost");
    assert!(transparent > 0, "transparent atlas covered the background");
    assert!(pollster::block_on(scope.pop()).is_none());
    println!("UI GPU PIXELS covered={covered} partial={partial} transparent={transparent}");
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI GPU acceptance target"),
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
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
