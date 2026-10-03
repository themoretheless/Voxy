use super::*;

#[test]
#[ignore = "requires GPU; full production legacy skin raster and rejected-pose retention"]
fn gpu_legacy_skin_raster_matches_cpu_and_retains_rejected_pose() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let camera_layout = create_camera_layout(&device);
    let material_layout = create_material_layout(&device);
    let skin_layout = create_skin_layout(&device);
    let pipeline = create_skinned_pipeline(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        &camera_layout,
        &material_layout,
        &skin_layout,
    );
    let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let dummy = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[0; 32],
        usage: wgpu::BufferUsages::STORAGE,
    });
    let camera_bind = create_camera_bind_group(&device, &camera_layout, &camera, &dummy, &dummy);
    let pack = MaterialPack::new(
        1,
        1,
        vec![MaterialLayer {
            rgba8_srgb: Arc::from([255_u8; 4]),
        }],
    )
    .unwrap();
    let materials = MaterialSet::upload(&device, &queue, &pack);
    let material_bind = create_material_bind_group(&device, &material_layout, &materials);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let (_depth, depth) = create_depth(&device, 128, 128);
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 128 * 128 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let render = |gpu: &GpuSkinnedMesh| {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, &material_bind, &[]);
            pass.set_bind_group(2, &gpu.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu.vertex.slice(..));
            pass.set_index_buffer(gpu.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..gpu.index_count, 0, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
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
        let pixels = mapped.to_vec();
        drop(mapped);
        staging.unmap();
        pixels
    };
    let normal = Vec3::new(0., 1., 1.).normalize();
    let base: Vec<_> = [[-0.7, -0.6, 0.4], [0.7, -0.6, 0.4], [0., 0.7, 0.4]]
        .into_iter()
        .map(|position| crate::SkinnedVertex {
            position,
            normal: normal.to_array(),
            uv: [0.3, 0.4],
            joints: [0, 1, 0, 0],
            weights: [32768, 32767, 0, 0],
        })
        .collect();
    let mesh = SkinnedMesh::new(base, vec![0, 1, 2], 2).unwrap();
    let joints = [
        Mat4::IDENTITY,
        Mat4::from_scale_rotation_translation(
            Vec3::new(1.3, 0.7, 1.1),
            glam::Quat::from_rotation_z(0.18),
            Vec3::new(0.1, 0., 0.),
        ),
    ];
    let mut model = Mat4::from_scale(Vec3::new(0.8, 0.6, 0.15));
    model.y_axis.x = 0.25;
    let gpu = upload_skinned(&device, &skin_layout, &mesh, &joints, model, 0).unwrap();
    let original = render(&gpu);
    let colored = original
        .chunks_exact(4)
        .filter(|p| p[..3].iter().any(|v| *v > 0))
        .count();
    assert!(colored > 1000);
    // Independent double-precision affine bake and inverse-transpose oracle.
    let blend = joints[0] * (32768.0 / 65535.) + joints[1] * (32767.0 / 65535.);
    let world = model * blend;
    let d = glam::DMat4::from_cols_array(&world.to_cols_array().map(f64::from));
    let correct = (d.inverse().transpose() * normal.as_dvec3().extend(0.))
        .truncate()
        .normalize()
        .as_vec3();
    let incorrect = world.transform_vector3(normal).normalize();
    let sun = Vec3::new(0.45, 0.82, 0.35).normalize();
    assert!(
        (correct.dot(sun) - incorrect.dot(sun)).abs() > 0.1,
        "fixture must distinguish illumination, not only normal direction"
    );
    let baked = |n: Vec3| {
        SkinnedMesh::new(
            mesh.vertices()
                .iter()
                .map(|v| crate::SkinnedVertex {
                    position: (d * Vec3::from_array(v.position).as_dvec3().extend(1.))
                        .truncate()
                        .as_vec3()
                        .to_array(),
                    normal: n.to_array(),
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                    ..*v
                })
                .collect(),
            vec![0, 1, 2],
            1,
        )
        .unwrap()
    };
    let cpu = upload_skinned(
        &device,
        &skin_layout,
        &baked(correct),
        &[Mat4::IDENTITY],
        Mat4::IDENTITY,
        0,
    )
    .unwrap();
    let expected = render(&cpu);
    let difference = original
        .iter()
        .zip(&expected)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        difference, 0,
        "full raster must match independently baked pose"
    );
    let wrong = upload_skinned(
        &device,
        &skin_layout,
        &baked(incorrect),
        &[Mat4::IDENTITY],
        Mat4::IDENTITY,
        0,
    )
    .unwrap();
    let wrong_pixels = render(&wrong);
    let wrong_difference = original
        .iter()
        .zip(&wrong_pixels)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        wrong_difference > 1000,
        "old transform must change lighting: changed={wrong_difference}, normal={correct:?}, old={incorrect:?}"
    );
    for (palette, transform) in [
        (joints, Mat4::from_scale(Vec3::new(0., 1., 1.))),
        ([Mat4::from_cols_array(&[f32::NAN; 16]); 2], model),
    ] {
        assert!(gpu.write_pose(&queue, &mesh, &palette, transform).is_err());
        assert_eq!(
            render(&gpu),
            original,
            "rejection must preserve actual pixels"
        );
    }
    let new_model = Mat4::from_translation(Vec3::new(0.08, 0., 0.)) * model;
    gpu.write_pose(&queue, &mesh, &joints, new_model).unwrap();
    assert_ne!(
        render(&gpu),
        original,
        "valid recovery must change the rendered pose"
    );
    // Independent per-pixel Lambert oracle for varying authored normals.
    // This catches missing fragment normalization, which constant-normal fixtures cannot.
    let smooth_normals = [
        Vec3::new(-0.8, 0., 0.6),
        Vec3::new(0.8, 0., 0.6),
        Vec3::new(0., 0.8, 0.6),
    ];
    let smooth = SkinnedMesh::new(
        mesh.vertices()
            .iter()
            .enumerate()
            .map(|(i, v)| crate::SkinnedVertex {
                normal: smooth_normals[i].to_array(),
                joints: [0; 4],
                weights: [65535, 0, 0, 0],
                ..*v
            })
            .collect(),
        vec![0, 1, 2],
        1,
    )
    .unwrap();
    let smooth_gpu = upload_skinned(
        &device,
        &skin_layout,
        &smooth,
        &[Mat4::IDENTITY],
        Mat4::IDENTITY,
        0,
    )
    .unwrap();
    let smooth_pixels = render(&smooth_gpu);
    let encode = |linear: f32| -> u8 {
        let srgb = if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1. / 2.4) - 0.055
        };
        (srgb * 255.).round() as u8
    };
    let mut checked = 0;
    let mut max_channel_error = 0_u8;
    let mut old_interpolation_differences = 0;
    for y in 0..128 {
        for x in 0..128 {
            let px = 2. * (x as f32 + 0.5) / 128. - 1.;
            let py = 1. - 2. * (y as f32 + 0.5) / 128.;
            let c = (py + 0.6) / 1.3;
            let b = (px + 0.7 - 0.7 * c) / 1.4;
            let a = 1. - b - c;
            if a.min(b).min(c) < 0.03 {
                continue;
            }
            let interpolated =
                smooth_normals[0] * a + smooth_normals[1] * b + smooth_normals[2] * c;
            let illumination = 0.18 + 0.82 * interpolated.normalize().dot(sun).max(0.);
            let scaled = illumination.clamp(0., 0.999) * 32.;
            // Exclude quantizer thresholds sensitive to f32 raster roundoff.
            if (scaled - scaled.round()).abs() < 1e-4 {
                continue;
            }
            let level = scaled.floor() / 31.;
            let expected = [
                encode(0.035 + (0.78 - 0.035) * level),
                encode(0.035 + (0.78 - 0.035) * level),
                encode(0.035 + (0.76 - 0.035) * level),
            ];
            let pixel = &smooth_pixels[(y * 128 + x) * 4..][..3];
            for (actual, expected) in pixel.iter().zip(expected) {
                max_channel_error = max_channel_error.max(actual.abs_diff(expected));
            }
            let old_level = ((0.18 + 0.82 * interpolated.dot(sun).max(0.)).clamp(0., 0.999) * 32.)
                .floor()
                / 31.;
            if old_level != level {
                old_interpolation_differences += 1;
            }
            checked += 1;
        }
    }
    assert!(checked > 2000);
    assert!(
        max_channel_error <= 1,
        "normalized fragment shading differs from CPU: max={max_channel_error}"
    );
    assert!(old_interpolation_differences > 1000);
    println!(
        "LEGACY_SMOOTH_NORMALS checked_pixels={} max_srgb_channel_error={} old_unnormalized_differing_pixels={}",
        checked, max_channel_error, old_interpolation_differences
    );
    // Absent authored normals must agree with the posed CCW face normal.
    let flat_mesh = |normal: Vec3| {
        SkinnedMesh::new(
            mesh.vertices()
                .iter()
                .map(|v| crate::SkinnedVertex {
                    normal: normal.to_array(),
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                    ..*v
                })
                .collect(),
            vec![0, 1, 2],
            1,
        )
        .unwrap()
    };
    for model in [
        Mat4::IDENTITY,
        Mat4::from_rotation_y(0.4) * Mat4::from_scale(Vec3::new(0.8, 0.7, 1.)),
    ] {
        let flat = upload_skinned(
            &device,
            &skin_layout,
            &flat_mesh(Vec3::ZERO),
            &[Mat4::IDENTITY],
            model,
            0,
        )
        .unwrap();
        let authored = upload_skinned(
            &device,
            &skin_layout,
            &flat_mesh(Vec3::Z),
            &[Mat4::IDENTITY],
            model,
            0,
        )
        .unwrap();
        let flat_pixels = render(&flat);
        let authored_pixels = render(&authored);
        let flat_difference = flat_pixels
            .iter()
            .zip(&authored_pixels)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            flat_difference, 0,
            "derived front-face normal must match authored CCW normal"
        );
        println!(
            "LEGACY_FLAT_NORMALS model={model:?} cpu_face_channel_difference={flat_difference}"
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "LEGACY_RASTER adapter={:?} colored={} cpu_pixel_difference={} old_normal_channel_difference={} rejected_frames=2 recovery=true",
        adapter.get_info(),
        colored,
        difference,
        wrong_difference
    );
}
