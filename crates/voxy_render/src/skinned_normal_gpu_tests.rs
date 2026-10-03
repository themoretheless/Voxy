use super::*;

#[test]
#[ignore = "requires GPU; production vertex-shader normal transport readback"]
fn gpu_legacy_normal_transport_matches_inverse_transpose() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let original = glam::Vec3::new(0.3, 0.7, 0.6).normalize();
    let mut cases = Vec::new();
    for sign in [-1.0, 1.0] {
        for scale in [
            glam::Vec3::new(0.2, 3.0, 1.4),
            glam::Vec3::new(1e-10, 2e-10, 3e-10),
            glam::Vec3::new(1e10, 2e10, 3e10),
        ] {
            let mut shear = Mat4::IDENTITY;
            shear.y_axis.x = 0.45;
            let world = Mat4::from_rotation_y(0.7)
                * shear
                * Mat4::from_scale(scale * glam::Vec3::new(sign, 1.0, 1.0));
            cases.push((world, original));
            cases.push((world, glam::Vec3::ZERO));
        }
    }
    let inputs: Vec<[f32; 20]> = cases
        .iter()
        .map(|(world, normal)| {
            let mut row = [0.; 20];
            row[..16].copy_from_slice(&world.to_cols_array());
            row[16..19].copy_from_slice(&normal.to_array());
            row
        })
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = cases.len() as u64 * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    // Compile the actual production function, keeping all vertex/fragment source.
    let source = format!(
        "{}\n{}",
        include_str!("skinned.wgsl"),
        r#"
struct Probe { world: mat4x4<f32>, normal: vec4<f32> }
@group(0) @binding(1) var<storage, read> probes: array<Probe>;
@group(0) @binding(2) var<storage, read_write> results: array<vec4<f32>>;
@compute @workgroup_size(1)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    results[id.x] = vec4<f32>(transported_normal(probes[id.x].world, probes[id.x].normal.xyz), 0.);
}
"#
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(cases.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    let actual = bytemuck::cast_slice::<u8, [f32; 4]>(&mapped);
    let mut max_error = 0.0_f32;
    let mut old_error = 0.0_f32;
    for ((world, normal), actual) in cases.iter().zip(actual) {
        let expected = if *normal == glam::Vec3::ZERO {
            glam::Vec3::ZERO
        } else {
            // Independent double-precision inverse-transpose oracle.
            let matrix = glam::DMat3::from_cols(
                world.x_axis.truncate().as_dvec3(),
                world.y_axis.truncate().as_dvec3(),
                world.z_axis.truncate().as_dvec3(),
            );
            (matrix.inverse().transpose() * normal.as_dvec3())
                .normalize()
                .as_vec3()
        };
        let actual = glam::Vec3::new(actual[0], actual[1], actual[2]);
        let error = actual.distance(expected);
        assert!(error < 2e-6, "world={world:?} error={error}");
        max_error = max_error.max(error);
        if *normal != glam::Vec3::ZERO {
            old_error = old_error.max(
                world
                    .transform_vector3(*normal)
                    .normalize()
                    .distance(expected),
            );
        }
    }
    assert!(
        old_error > 0.5,
        "fixture must distinguish the old forward transform"
    );
    drop(mapped);
    readback.unmap();
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "LEGACY_NORMAL_PROBE adapter={:?} cases={} max_error={} old_forward_error={}",
        adapter.get_info(),
        cases.len(),
        max_error,
        old_error
    );
}

#[test]
fn legacy_render_admission_rejects_collapsed_blends_and_keeps_history() {
    let vertex = SkinnedVertex {
        position: [0.2, 0.3, 0.4],
        normal: [0.3, 0.7, 0.6],
        uv: [0.; 2],
        joints: [0, 1, 0, 0],
        weights: [32768, 32767, 0, 0],
    };
    let mesh = SkinnedMesh::new(vec![vertex], vec![0; 3], 2).unwrap();
    let good = [Mat4::IDENTITY; 2];
    let mut bad = good;
    bad[1] = Mat4::from_scale(glam::Vec3::new(-32768.0 / 32767.0, 1., 1.));
    // Both bones are invertible; their blended deformation collapses X.
    assert!(bad.iter().all(|m| m.determinant() != 0.));
    assert_eq!(
        mesh.validate_render_pose(&bad, Mat4::IDENTITY),
        Err(SkinnedUploadError::SingularNormalMatrix)
    );
    assert_eq!(
        mesh.validate_render_pose(&good, Mat4::from_scale(glam::Vec3::new(1., 0., 1.))),
        Err(SkinnedUploadError::SingularNormalMatrix)
    );
    let mut projective = Mat4::IDENTITY;
    projective.x_axis.w = 0.2;
    assert_eq!(
        mesh.validate_render_pose(&good, projective),
        Err(SkinnedUploadError::NonAffineMatrix)
    );
    let huge = Mat4::from_scale(glam::Vec3::splat(1e30));
    assert!(mesh.validate_render_pose(&[huge; 2], huge).is_err());
    let mut history = crate::SkinnedMotionHistory::new(mesh.clone());
    history.presented(&good, Mat4::IDENTITY).unwrap();
    assert!(
        history
            .mesh()
            .validate_render_pose(&bad, Mat4::IDENTITY)
            .is_err()
    );
    let next = history
        .prepare(&good, Mat4::from_translation(glam::Vec3::X))
        .unwrap();
    assert!(next.history_valid);
    assert_eq!(next.vertices[0].previous, vertex.position);
    let (device, _) = wgpu::Device::noop(&Default::default());
    let layout = create_skin_layout(&device);
    assert!(matches!(
        upload_skinned(&device, &layout, &mesh, &bad, Mat4::IDENTITY, 0),
        Err(SkinnedUploadError::SingularNormalMatrix)
    ));
    assert!(upload_skinned(&device, &layout, &mesh, &good, Mat4::IDENTITY, 0).is_ok());
    // Reflection is valid; determinant zero, rather than sign, is the rejection.
    assert!(
        mesh.validate_render_pose(&good, Mat4::from_scale(glam::Vec3::new(-2., 3., 4.)))
            .is_ok()
    );
}

#[test]
#[ignore = "release-mode CPU admission profiling; no FPS inference"]
fn profile_legacy_render_admission_reference_rigs() {
    assert!(!cfg!(debug_assertions), "run this profile with --release");
    for (label, bytes) in [
        (
            "Fox",
            include_bytes!("../examples/assets/fox/Fox.glb").as_slice(),
        ),
        (
            "RiggedFigure",
            include_bytes!("../examples/assets/rigged-figure/RiggedFigure.glb").as_slice(),
        ),
    ] {
        let gltf = gltf::Gltf::from_slice(bytes).unwrap();
        let asset = crate::ModelAsset::parse(
            bytes,
            &[gltf.blob.as_deref().unwrap()],
            crate::ModelLimits::default(),
        )
        .unwrap();
        let crate::ModelGeometry::Skinned(mesh) = &asset.primitives[0].geometry else {
            panic!("skinned fixture required")
        };
        let model = Mat4::from_scale(glam::Vec3::new(0.8, 1.2, 0.7));
        let mut timings = Vec::new();
        for frame in 0..240 {
            let clip = frame % asset.animations.len();
            let pose = asset.sample_pose(Some(clip), frame as f32 * 0.013).unwrap();
            let palette = asset.skin_matrices(&pose).unwrap();
            let start = std::time::Instant::now();
            mesh.validate_render_pose(std::hint::black_box(&palette), std::hint::black_box(model))
                .unwrap();
            if frame >= 32 {
                timings.push(start.elapsed().as_secs_f64() * 1e6);
            }
        }
        timings.sort_by(f64::total_cmp);
        println!(
            "LEGACY_ADMISSION_CPU asset={} vertices={} samples={} median_us={} p95_us={}",
            label,
            mesh.vertices().len(),
            timings.len(),
            timings[timings.len() / 2],
            timings[(timings.len() - 1) * 95 / 100]
        );
    }
}
