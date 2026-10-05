//! Actual panel geometry/atlas rendered by the native scene renderer.
use super::*;
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn marker_controls_render_readable_glyphs_without_hit_overlap() {
    let mut document = SceneDocument::from_json(r#"{"version":1,"objects":[{"id":"rig","parent":null,"name":"Animated rig","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}}]}"#).unwrap();
    document.objects[0].components.insert(
        "editor.model-animation.v1".into(),
        serde_json::to_value(crate::ModelAnimation {
            events: vec![
                crate::ModelAnimationEvent {
                    name: "step".into(),
                    phase: 0.25,
                },
                crate::ModelAnimationEvent {
                    name: "land".into(),
                    phase: 0.75,
                },
            ],
            ..Default::default()
        })
        .unwrap(),
    );
    let mut panels = Panels::new().unwrap();
    let mesh = panels
        .build(
            &document,
            0,
            Vec2::new(1024., 640.),
            false,
            None,
            0,
            false,
            InspectorMode::Components(0),
            "Texture: none",
            "Animation markers",
            None,
        )
        .unwrap();
    let controls: Vec<_> = panels
        .regions
        .iter()
        .filter(|(_, action)| {
            matches!(
                action,
                Action::MarkerAdd(_)
                    | Action::MarkerDelete(..)
                    | Action::MarkerPreview(..)
                    | Action::PreviewSeek
            )
        })
        .copied()
        .collect();
    assert_eq!(controls.len(), 5);
    for (rect, _) in &controls {
        for (other, action) in &panels.regions {
            if matches!(action, Action::Field(_)) {
                assert!(
                    rect[0] >= other[0] + other[2]
                        || other[0] >= rect[0] + rect[2]
                        || rect[1] >= other[1] + other[3]
                        || other[1] >= rect[1] + rect[3],
                    "overlapping control"
                );
            }
        }
    }
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("real GPU required");
    println!("MARKER PANEL GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer.upload_mesh(&device, &mesh).unwrap();
    let texture = renderer
        .upload_texture(&device, &queue, 512, 128, &panels.rgba)
        .unwrap();
    let transform = renderer
        .create_transform(&device, glam::Mat4::IDENTITY)
        .unwrap();
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("marker panel pixels"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 640,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("marker readback"),
        size: 1024 * 640 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color::BLACK,
        &[voxy_render::SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &transform,
            overlay: true,
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
                bytes_per_row: Some(4096),
                rows_per_image: Some(640),
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = readback.slice(..).get_mapped_range().unwrap();
    for (rect, _) in controls {
        let mut glyph_pixels = 0;
        for y in rect[1] as usize..(rect[1] + rect[3]) as usize {
            for x in rect[0] as usize..(rect[0] + rect[2]) as usize {
                let pixel = &pixels[(y * 1024 + x) * 4..][..4];
                if pixel[0] > 150 && pixel[1] > 150 && pixel[2] > 150 {
                    glyph_pixels += 1;
                }
            }
        }
        assert!(glyph_pixels > 20, "button glyphs missing: {glyph_pixels}");
    }
    if let Ok(path) = std::env::var("VOXY_MARKER_PANEL_PPM") {
        let mut ppm = b"P6\n1024 640\n255\n".to_vec();
        for pixel in pixels.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(path, ppm).unwrap();
    }
    drop(pixels);
    readback.unmap();
    assert!(pollster::block_on(scope.pop()).is_none());
}
