use super::*;

#[test]
#[ignore = "requires a real GPU adapter; run explicitly for multi-view acceptance"]
fn gpu_disjoint_views_preserve_both_regions_and_clear_gap() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter required");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = renderer
        .upload_mesh(&device, &SceneMesh::quad([1.; 4]))
        .unwrap();
    let red = renderer
        .upload_texture(&device, &queue, 1, 1, &[255, 0, 0, 255])
        .unwrap();
    let blue = renderer
        .upload_texture(&device, &queue, 1, 1, &[0, 0, 255, 255])
        .unwrap();
    let left_transform = renderer
        .create_transform(&device, Mat4::from_scale(glam::Vec3::splat(4.)))
        .unwrap();
    let right_transform = renderer
        .create_transform(&device, Mat4::from_scale(glam::Vec3::splat(2.)))
        .unwrap();
    let left = [SceneDraw {
        geometry: &geometry,
        texture: &red,
        transform: &left_transform,
        overlay: true,
    }];
    let right = [SceneDraw {
        geometry: &geometry,
        texture: &blue,
        transform: &right_transform,
        overlay: false,
    }];
    let green = renderer
        .upload_texture(&device, &queue, 1, 1, &[0, 255, 0, 255])
        .unwrap();
    let ui_transform = renderer
        .create_transform(
            &device,
            Mat4::from_translation(glam::Vec3::new(0., 0.75, 0.))
                * Mat4::from_scale(glam::Vec3::new(2., 0.5, 1.)),
        )
        .unwrap();
    let ui = [SceneDraw {
        geometry: &geometry,
        texture: &green,
        transform: &ui_transform,
        overlay: true,
    }];
    // A color permutation makes using a stale/default shader observable in pixels.
    let custom = DEFAULT_SCENE_SHADER.replace(
        "return textureSample(image, image_sampler, in.uv) * in.color;",
        "let color = textureSample(image, image_sampler, in.uv) * in.color; return color.bgra;",
    );
    assert_ne!(custom, DEFAULT_SCENE_SHADER);
    assert!(pollster::block_on(renderer.reload_shader(&device, &custom)).unwrap());
    let revision = renderer.shader_revision();
    let (foreign, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    assert!(pollster::block_on(renderer.enable_msaa4(&foreign)).is_err());
    assert!(pollster::block_on(renderer.enable_msaa4(&device)).unwrap());
    assert!(!pollster::block_on(renderer.enable_msaa4(&device)).unwrap());
    assert_eq!(renderer.shader_revision(), revision);
    // Both module parsing and later pipeline ABI admission must preserve the
    // accepted custom shader, including its already enabled MSAA variants.
    let missing_entry = custom.replace("fn vs_main(", "fn absent_vertex_entry(");
    assert_ne!(missing_entry, custom);
    for rejected in ["this is not WGSL", missing_entry.as_str()] {
        let error = pollster::block_on(renderer.reload_shader(&device, rejected)).unwrap_err();
        assert!(!error.to_string().is_empty());
        assert_eq!(renderer.shader_revision(), revision);
        assert!(!pollster::block_on(renderer.reload_shader(&device, &custom)).unwrap());
        assert!(!pollster::block_on(renderer.enable_msaa4(&device)).unwrap());
        eprintln!("REJECTED SCENE SHADER: {error}");
    }
    eprintln!("SHADER ROLLBACK GPU {:?}", adapter.get_info());

    let target = |format, usage, sample_count| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("multi-view pixels"),
            size: wgpu::Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
    );
    let msaa_color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let msaa_depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let edge_mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-1., -1., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., -1., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [-1., 1., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap();
    let edge_geometry = renderer.upload_mesh(&device, &edge_mesh).unwrap();
    let edge_transform = renderer.create_transform(&device, Mat4::IDENTITY).unwrap();
    let left_edge = [SceneDraw {
        geometry: &edge_geometry,
        texture: &red,
        transform: &edge_transform,
        overlay: false,
    }];
    let right_edge = [SceneDraw {
        geometry: &edge_geometry,
        texture: &blue,
        transform: &edge_transform,
        overlay: false,
    }];
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("multi-view readback"),
        size: 256 * 32,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for (msaa, with_overlay, edges) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (true, true, false),
        (false, false, true),
        (true, false, true),
    ] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let views = [
            SceneView {
                viewport: [0, 0, 16, 32],
                draws: if edges { &left_edge } else { &left },
            },
            SceneView {
                viewport: [32, 0, 32, 32],
                draws: if edges { &right_edge } else { &right },
            },
        ];
        let overlays = if with_overlay { &ui[..] } else { &[] };
        if msaa {
            assert_eq!(
                renderer.encode_views_msaa4(
                    &mut encoder,
                    &color_view,
                    &depth_view,
                    &color_view,
                    wgpu::Color::BLACK,
                    &views
                ),
                Err(SceneError::InvalidGeometry)
            );
            renderer
                .encode_view_frame_msaa4(
                    &mut encoder,
                    (
                        &msaa_color.create_view(&wgpu::TextureViewDescriptor::default()),
                        &msaa_depth.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                    &color_view,
                    &depth_view,
                    wgpu::Color::BLACK,
                    &views,
                    overlays,
                )
                .unwrap();
        } else {
            renderer
                .encode_view_frame(
                    &mut encoder,
                    &color_view,
                    &depth_view,
                    wgpu::Color::BLACK,
                    &views,
                    overlays,
                )
                .unwrap();
        }
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(32),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
        );
        let submitted = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        {
            let mapped = readback.slice(..).get_mapped_range().unwrap();
            let mut partial = [0_usize; 2];
            let mut full = [0_usize; 2];
            for y in 0..32 {
                for x in 0..64 {
                    let offset = y * 256 + x * 4;
                    let pixel = &mapped[offset..offset + 3];
                    if edges && !(16..32).contains(&x) {
                        let (view, channel) = if x < 16 { (0, 2) } else { (1, 0) };
                        assert_eq!(pixel[1], 0);
                        assert_eq!(pixel[2 - channel], 0);
                        if pixel[channel] == 255 {
                            full[view] += 1;
                        } else if pixel[channel] != 0 {
                            partial[view] += 1;
                        }
                    } else {
                        let expected = if with_overlay && y < 8 {
                            [0, 255, 0]
                        } else if edges {
                            [0, 0, 0]
                        } else if x < 16 {
                            [0, 0, 255]
                        } else if x < 32 {
                            [0, 0, 0]
                        } else {
                            [255, 0, 0]
                        };
                        assert_eq!(pixel, &expected, "pixel ({x}, {y}), msaa={msaa}");
                    }
                }
            }
            if edges {
                assert!(full.iter().all(|&count| count > 0));
                if msaa {
                    assert!(
                        partial.iter().all(|&count| count > 0),
                        "both view edges need partial coverage: {partial:?}"
                    );
                } else {
                    assert_eq!(partial, [0, 0]);
                }
            }
        }
        readback.unmap();
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires a real GPU adapter; run explicitly for multi-view X-ray acceptance"]
fn gpu_disjoint_views_xray_keeps_nearest_internals_and_world_depth() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new_msaa4(&device, wgpu::TextureFormat::Rgba8Unorm);
    let world = renderer
        .upload_mesh(&device, &SceneMesh::quad([1.; 4]))
        .unwrap();
    let mut internals = renderer
        .upload_mesh(&device, &SceneMesh::quad([1.; 4]))
        .unwrap();
    internals.set_depth_mode(SceneDepthMode::Xray);
    let tex = |rgba: &[u8]| {
        renderer
            .upload_texture(&device, &queue, 1, 1, rgba)
            .unwrap()
    };
    let white = tex(&[255; 4]);
    let red = tex(&[255, 0, 0, 255]);
    let blue = tex(&[0, 0, 255, 255]);
    let green = tex(&[0, 255, 0, 255]);
    let transform = |z| {
        renderer
            .create_transform(
                &device,
                Mat4::from_translation(glam::Vec3::new(0., 0., z))
                    * Mat4::from_scale(glam::Vec3::new(2., 2., 1.)),
            )
            .unwrap()
    };
    let wall = transform(0.1);
    let near = transform(0.4);
    let far = transform(0.6);
    let ui_transform = renderer
        .create_transform(
            &device,
            Mat4::from_translation(glam::Vec3::new(0., 0.75, 0.))
                * Mat4::from_scale(glam::Vec3::new(2., 0.5, 1.)),
        )
        .unwrap();
    let w = SceneDraw {
        geometry: &world,
        texture: &white,
        transform: &wall,
        overlay: false,
    };
    let n = SceneDraw {
        geometry: &internals,
        texture: &blue,
        transform: &near,
        overlay: false,
    };
    let f = SceneDraw {
        geometry: &internals,
        texture: &red,
        transform: &far,
        overlay: false,
    };
    let left = [w, n, f];
    let right = [
        SceneDraw {
            geometry: &world,
            texture: &white,
            transform: &wall,
            overlay: false,
        },
        SceneDraw {
            geometry: &internals,
            texture: &red,
            transform: &far,
            overlay: false,
        },
        SceneDraw {
            geometry: &internals,
            texture: &blue,
            transform: &near,
            overlay: false,
        },
    ];
    let ui = [SceneDraw {
        geometry: &world,
        texture: &green,
        transform: &ui_transform,
        overlay: true,
    }];
    let views = [
        SceneView {
            viewport: [0, 0, 32, 32],
            draws: &left,
        },
        SceneView {
            viewport: [32, 0, 32, 32],
            draws: &right,
        },
    ];
    for (samples, legacy) in [(1, false), (4, false), (1, true)] {
        let target = |format, count| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("X-ray views test"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | if count == 1 {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    },
                view_formats: &[],
            })
        };
        let output = target(wgpu::TextureFormat::Rgba8Unorm, 1);
        let color = target(wgpu::TextureFormat::Rgba8Unorm, samples);
        let depth = target(wgpu::TextureFormat::Depth32Float, samples);
        let xray = target(wgpu::TextureFormat::Depth32Float, samples);
        let overlay_depth = target(wgpu::TextureFormat::Depth32Float, 1);
        let cv = color.create_view(&Default::default());
        let dv = depth.create_view(&Default::default());
        let xv = xray.create_view(&Default::default());
        let ov = output.create_view(&Default::default());
        let ui_depth = overlay_depth.create_view(&Default::default());
        let resolve = (samples == 4).then_some(&ov);
        let mut encoder = device.create_command_encoder(&Default::default());
        for invalid in [None, Some(&dv)] {
            assert_eq!(
                renderer.encode_view_frame_targets(
                    &mut encoder,
                    SceneViewTargets {
                        color: &cv,
                        depth: &dv,
                        xray_depth: invalid,
                        resolve,
                        overlay_depth: &ui_depth,
                    },
                    wgpu::Color::BLACK,
                    &views,
                    &ui
                ),
                Err(SceneError::InvalidGeometry)
            );
        }
        if legacy {
            let draws = [
                SceneDraw {
                    geometry: &world,
                    texture: &white,
                    transform: &wall,
                    overlay: false,
                },
                SceneDraw {
                    geometry: &internals,
                    texture: &blue,
                    transform: &near,
                    overlay: false,
                },
                SceneDraw {
                    geometry: &internals,
                    texture: &red,
                    transform: &far,
                    overlay: false,
                },
                SceneDraw {
                    geometry: &world,
                    texture: &green,
                    transform: &ui_transform,
                    overlay: true,
                },
            ];
            renderer.encode_with_xray_depth(
                &mut encoder,
                &cv,
                &dv,
                &xv,
                wgpu::Color::BLACK,
                &draws,
            );
        } else {
            renderer
                .encode_view_frame_targets(
                    &mut encoder,
                    SceneViewTargets {
                        color: &cv,
                        depth: &dv,
                        xray_depth: Some(&xv),
                        resolve,
                        overlay_depth: &ui_depth,
                    },
                    wgpu::Color::BLACK,
                    &views,
                    &ui,
                )
                .unwrap();
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("X-ray view color/depth"),
            size: 256 * 32 * 2,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let color_output = if samples == 4 { &output } else { &color };
        encoder.copy_texture_to_buffer(
            color_output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(32),
                },
            },
            color_output.size(),
        );
        if samples == 1 {
            encoder.copy_texture_to_buffer(
                depth.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 256 * 32,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(32),
                    },
                },
                depth.size(),
            );
        }
        let submitted = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        {
            let pixels = buffer.slice(..).get_mapped_range().unwrap();
            for y in 0..32 {
                for x in 0..64 {
                    let at = y * 256 + x * 4;
                    let expected = if y < 8 { [0, 255, 0] } else { [0, 0, 255] };
                    assert_eq!(
                        &pixels[at..at + 3],
                        &expected,
                        "({x},{y}), samples={samples}"
                    );
                    if samples == 1 {
                        let at = at + 256 * 32;
                        let depth = f32::from_le_bytes(pixels[at..at + 4].try_into().unwrap());
                        assert!(
                            (depth - 0.1).abs() < 1e-6,
                            "world depth changed at ({x},{y}): {depth}"
                        );
                    }
                }
            }
        }
        buffer.unmap();
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
