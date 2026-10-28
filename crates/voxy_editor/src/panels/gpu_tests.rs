//! Actual panel geometry/atlas rendered by the native scene renderer.
use super::*;
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn marker_controls_render_readable_glyphs_without_hit_overlap() {
    render_controls(Controls::Markers);
}
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn missing_model_notice_renders_three_readable_lines_without_control_overlap() {
    render_controls(Controls::Missing);
}
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn relink_path_entry_renders_readable_path_and_cancel_target() {
    render_controls(Controls::Relink);
}
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn load_progress_and_cancel_render_readable_glyphs() {
    render_controls(Controls::Loading);
}
#[test]
#[ignore = "requires a real GPU and local TrueType font"]
fn snapshot_failure_notice_renders_diagnostic_and_retry_hint_without_overlap() {
    render_controls(Controls::SnapshotError);
}
enum Controls {
    Markers,
    Missing,
    Relink,
    Loading,
    SnapshotError,
}
fn render_controls(scenario: Controls) {
    let with_notice = matches!(scenario, Controls::Missing | Controls::Relink);
    let editing = matches!(scenario, Controls::Relink);
    let loading = matches!(scenario, Controls::Loading);
    let with_error = matches!(scenario, Controls::SnapshotError);
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
    if with_notice {
        document.objects[0]
            .components
            .insert("editor.model.v1".into(), serde_json::json!("character"));
        panels.resource_notice = Some(ResourceNotice {
            lines: [
                "Model binding missing: character".into(),
                "Model unavailable; scene reference kept".into(),
                "Restore manifest binding or click Model to choose an asset".into(),
            ],
            retry: !editing,
            relink: true,
            editing,
        });
        if editing {
            panels.resource_notice.as_mut().unwrap().lines = [
                "Relink model: character".into(),
                "Path: models/new-character.glb|".into(),
                "Project-relative path; Enter applies, Escape cancels".into(),
            ];
        }
    }
    if with_error {
        let temp = crate::project_prefab_tests::Temp::new();
        let project =
            crate::project::create(&temp.0.join("project"), "Snapshot diagnostic").unwrap();
        let mut app = crate::App::from_project(&project).unwrap();
        app.report_snapshot_error("Cannot write snapshot: permission denied");
        panels.resource_notice = app.snapshot_failure_notice();
        app.stop_workers().unwrap();
    }
    if loading {
        panels.load_status = Some(crate::app::restore_barrier::load::LoadStatus {
            label: "Uploading models...",
            cancellable: true,
        });
    }
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
                    | Action::MorphDefaults(_)
                    | Action::MorphResume(_)
            )
        })
        .copied()
        .collect();
    assert_eq!(controls.len(), 7);
    for (rect, _) in &controls {
        for (other, action) in &panels.regions {
            if other != rect {
                assert!(
                    rect[0] >= other[0] + other[2]
                        || other[0] >= rect[0] + rect[2]
                        || rect[1] >= other[1] + other[3]
                        || other[1] >= rect[1] + rect[3],
                    "animation control {rect:?} overlaps {action:?} at {other:?}"
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
    if with_notice || with_error {
        for row in 0..3 {
            let mut glyph_pixels = 0;
            for y in 12 + row * 20..32 + row * 20 {
                for x in 166..828 {
                    let pixel = &pixels[(y * 1024 + x) * 4..][..4];
                    if pixel[0] > 150 && pixel[1] > 150 && pixel[2] > 150 {
                        glyph_pixels += 1;
                    }
                }
            }
            assert!(
                glyph_pixels > 100,
                "notice row {row} has no readable glyphs"
            );
        }
        for (rect, action) in &panels.regions {
            if matches!(
                action,
                Action::RelinkModel | Action::RelinkCancel | Action::RetryModel
            ) {
                continue;
            }
            assert!(
                rect[0] + rect[2] <= 158.
                    || rect[0] >= 836.
                    || rect[1] >= 76.
                    || rect[1] + rect[3] <= 8.,
                "notice overlaps {action:?}"
            );
        }
    }
    if loading {
        assert!(
            panels
                .regions
                .iter()
                .any(|(_, action)| *action == Action::CancelLoad)
        );
        assert!(
            !panels
                .regions
                .iter()
                .any(|(_, action)| matches!(action, Action::Save | Action::Load))
        );
        for row in [4, 5] {
            let mut glyphs = 0;
            for y in (400 + row * 24)..(424 + row * 24) {
                for x in 10..150 {
                    let pixel = &pixels[(y * 1024 + x) * 4..][..4];
                    if pixel[0] > 150 && pixel[1] > 150 && pixel[2] > 150 {
                        glyphs += 1;
                    }
                }
            }
            assert!(glyphs > 70, "load status row {row} has no readable glyphs");
        }
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        assert_eq!(panels.hit(Vec2::new(50.0, 532.0)), Some(Action::CancelLoad));
    }
    if with_notice {
        assert_eq!(panels.hit(Vec2::new(190., 90.)), None);
        if !editing {
            assert_eq!(panels.hit(Vec2::new(440., 90.)), None);
        }
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        assert_eq!(
            panels.hit(Vec2::new(190., 90.)),
            Some(if editing {
                Action::RelinkCancel
            } else {
                Action::RelinkModel
            })
        );
    }
    if with_notice && !editing {
        assert_eq!(panels.hit(Vec2::new(440., 90.)), Some(Action::RetryModel));
        let retry = panels
            .regions
            .iter()
            .find(|(_, action)| *action == Action::RetryModel)
            .unwrap()
            .0;
        let mut glyphs = 0;
        for y in retry[1] as usize..(retry[1] + retry[3]) as usize {
            for x in retry[0] as usize..(retry[0] + retry[2]) as usize {
                let pixel = &pixels[(y * 1024 + x) * 4..][..4];
                if pixel[0] > 150 && pixel[1] > 150 && pixel[2] > 150 {
                    glyphs += 1;
                }
            }
        }
        assert!(glyphs > 20, "retry button glyphs missing");
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
