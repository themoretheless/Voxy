use super::*;

#[test]
#[ignore = "requires GPU; scene ABI deformation readback"]
fn gpu_scene_skinning_independent_instances_and_last_good_pose() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(gpu.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let skinner = SceneSkinner::new(&renderer).unwrap();
    let asset = crate::ModelAsset::parse(
        include_bytes!("../../../examples/assets/animated-triangle.glb"),
        &[],
        crate::ModelLimits::default(),
    )
    .unwrap();
    let crate::ModelGeometry::Skinned(mesh) = &asset.primitives[0].geometry else {
        panic!("skin required")
    };
    let source = skinner
        .upload_source(Arc::new(mesh.clone()), 0, 4096)
        .unwrap();
    assert_eq!(source.allocation_bytes(), mesh.vertices().len() as u64 * 64);
    let palette = asset
        .skin_matrices(&asset.sample_pose(None, 0.).unwrap())
        .unwrap();
    let color = [0.25, 0.5, 0.75, 1.];
    let first = skinner
        .create_instance(
            &renderer,
            source.clone(),
            &palette,
            color,
            source.allocation_bytes(),
            4096,
        )
        .unwrap();
    let live = source.allocation_bytes() + first.allocation_bytes();
    assert!(matches!(
        skinner.create_instance(&renderer, source.clone(), &palette, color, live, live),
        Err(SceneSkinError::BudgetExceeded { .. })
    ));
    let second = skinner
        .create_instance(&renderer, source.clone(), &palette, color, live, 4096)
        .unwrap();
    assert!(Arc::ptr_eq(&first.source, &second.source));
    assert_ne!(first.geometry.vertices, second.geometry.vertices);
    assert_ne!(first.palette, second.palette);
    let read = |instance: &SceneSkinInstance, pose: Option<&[Mat4]>| {
        let size = instance.geometry.vertices.size();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        if let Some(pose) = pose {
            skinner
                .encode_pose(&queue, &mut encoder, instance, pose)
                .unwrap();
        }
        encoder.copy_buffer_to_buffer(&instance.geometry.vertices, 0, &staging, 0, size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = staging.slice(..).get_mapped_range().unwrap();
        let vertices = bytemuck::cast_slice::<u8, SceneVertex>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        vertices
    };
    let mut results = Vec::new();
    for (instance, time) in [(&first, 0.25), (&second, 0.75), (&first, 0.25)] {
        let pose = asset.sample_pose(Some(0), time).unwrap();
        let palette = asset.skin_matrices(&pose).unwrap();
        let actual = read(instance, Some(&palette));
        let expected = mesh
            .posed_scene_mesh(&palette, Mat4::IDENTITY, color)
            .unwrap();
        for (a, b) in actual.iter().zip(expected.vertices()) {
            assert!(
                glam::Vec3::from_array(a.position)
                    .abs_diff_eq(glam::Vec3::from_array(b.position), 1e-5)
            );
            assert_eq!(a.uv, b.uv);
            assert_eq!(a.color, b.color);
        }
        results.push(actual);
    }
    assert_eq!(results[0], results[2]);
    assert_ne!(results[0], results[1]);
    // Both outputs use the actual scene draw pipeline, with the CPU reference
    // in the neighboring viewport. This exercises the unchanged scene ABI.
    let reference = renderer
        .upload_mesh(
            &device,
            &SceneMesh::new(results[0].clone(), mesh.indices().to_vec()).unwrap(),
        )
        .unwrap();
    let white = renderer
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for vertex in &results[0] {
        let p = glam::Vec3::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) * 0.5;
    let scale = 1.5 / (max.x - min.x).max(max.y - min.y);
    let transform = renderer
        .create_transform(
            &device,
            Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
                * Mat4::from_scale(glam::Vec3::splat(scale))
                * Mat4::from_translation(-center),
        )
        .unwrap();
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 32,
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
    let color_target = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let left = [SceneDraw {
        geometry: first.geometry(),
        texture: &white,
        transform: &transform,
        overlay: true,
    }];
    let right = [SceneDraw {
        geometry: &reference,
        texture: &white,
        transform: &transform,
        overlay: true,
    }];
    let mut encoder = device.create_command_encoder(&Default::default());
    let other = SceneSkinner {
        device: device.clone(),
        pipeline: skinner.pipeline.clone(),
        identity: Arc::new(()),
    };
    assert!(matches!(
        other.encode_pose(&queue, &mut encoder, &first, &palette),
        Err(SceneSkinError::ForeignSkinner)
    ));
    renderer
        .encode_views(
            &mut encoder,
            &color_target.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color::BLACK,
            &[
                SceneView {
                    viewport: [0, 0, 32, 32],
                    draws: &left,
                },
                SceneView {
                    viewport: [32, 0, 32, 32],
                    draws: &right,
                },
            ],
        )
        .unwrap();
    let pixels = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 32,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        color_target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &pixels,
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
    let (tx, rx) = std::sync::mpsc::channel();
    pixels
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = pixels.slice(..).get_mapped_range().unwrap();
    let mut colored = 0;
    for y in 0..32 {
        for x in 0..32 {
            let a = &mapped[y * 256 + x * 4..y * 256 + x * 4 + 4];
            let b = &mapped[y * 256 + (x + 32) * 4..y * 256 + (x + 32) * 4 + 4];
            assert_eq!(a, b);
            colored += usize::from(a[0] > 0);
        }
    }
    assert!(colored > 50);
    drop(mapped);
    pixels.unmap();
    let mut encoder = device.create_command_encoder(&Default::default());
    assert!(matches!(
        skinner.encode_pose(&queue, &mut encoder, &first, &[]),
        Err(SceneSkinError::Pose(_))
    ));
    let bad = vec![Mat4::from_cols_array(&[f32::NAN; 16]); palette.len()];
    assert!(matches!(
        skinner.encode_pose(&queue, &mut encoder, &first, &bad),
        Err(SceneSkinError::Pose(_))
    ));
    queue.submit([encoder.finish()]);
    assert_eq!(read(&first, None), results[0]);
    assert!(pollster::block_on(scope.pop()).is_none());
}
