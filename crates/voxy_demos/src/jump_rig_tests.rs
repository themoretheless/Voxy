use super::*;

#[test]
fn jump_legs_plant_feet_bend_knees_and_keep_bone_lengths() {
    let asset = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(asset.mesh.vertices()).unwrap();
    for time in [0., 0.20, 0.38, 0.48, 0.56, 0.81, 1.06, 1.18, 1.26, 1.65, 5.] {
        let motion = crate::jump_motion::sample(time);
        let palette = rig.jump_palette(time);
        for (hip, knee, foot, sign) in [(7, 8, 9, 1.), (13, 14, 15, -1.)] {
            let rest = [
                Vec3::new(sign * 0.09, -0.10, 0.),
                Vec3::new(sign * 0.10, -0.44, 0.),
                Vec3::new(sign * 0.10, -0.74, 0.),
            ];
            let posed: [Vec3; 3] = std::array::from_fn(|i| {
                palette[[hip, knee, foot][i]].transform_point3(rest[i])
                    + Vec3::Y * motion.height as f32
            });
            for i in 0..2 {
                assert!(
                    (posed[i].distance(posed[i + 1]) - rest[i].distance(rest[i + 1])).abs() < 2e-6,
                    "leg length changed at {time}"
                );
            }
            if motion.flight.is_none() {
                assert!(
                    posed[2].distance(rest[2]) < 2e-6,
                    "grounded foot slid at {time}: {:?}",
                    posed[2]
                );
                assert!(
                    palette[foot].transform_vector3(Vec3::Y).distance(Vec3::Y) < 2e-6,
                    "grounded sole tilted at {time}"
                );
            } else if motion.height > 1e-6 {
                assert!(posed[2].y > rest[2].y, "airborne foot still on floor");
            }
            if time == 0.38 || time == 1.26 {
                let bend = (posed[1] - posed[0])
                    .normalize()
                    .dot((posed[2] - posed[1]).normalize())
                    .clamp(-1., 1.)
                    .acos();
                assert!(bend > 0.9, "squat/landing knee remained straight");
                assert!(posed[1].z > 0.15, "knee must flex forward");
            }
        }
    }
    let mut crouched = asset.mesh.vertices().to_vec();
    rig.deform_jump(&mut crouched, 0.38);
    let bob = crate::jump_motion::sample(0.38).height as f32;
    let mut pinned = 0;
    let mut knee_motion = 0f32;
    for ((rest, posed), weights) in asset
        .mesh
        .vertices()
        .iter()
        .zip(&crouched)
        .zip(&rig.weights)
    {
        let p = Vec3::from_array(rest.position);
        let q = Vec3::from_array(posed.position) + Vec3::Y * bob;
        if p.y < -0.755
            && weights
                .iter()
                .any(|(i, w)| (*i == 9 || *i == 15) && *w > 0.9999)
        {
            assert!(p.distance(q) < 2e-6, "rendered sole moved while planted");
            pinned += 1;
        }
        if (-0.51..-0.37).contains(&p.y) {
            knee_motion = knee_motion.max(p.distance(q));
        }
    }
    assert!(pinned > 100, "no actual sole vertices checked");
    assert!(knee_motion > 0.12, "actual model knees did not articulate");
    eprintln!("JUMP RIG actual_planted_vertices={pinned} maximum_knee_motion_m={knee_motion}");
}

#[test]
#[ignore = "requires an actual GPU; complete body DQ skinning and displacement readback"]
fn gpu_jump_matches_actual_body_and_rigid_trailing_hair() {
    let asset = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(asset.mesh.vertices()).unwrap();
    let body = asset.mesh.vertices().len();
    let mut vertices = asset.mesh.vertices().to_vec();
    let mut indices = asset.mesh.indices().to_vec();
    for p in [[-0.03, 0.80, -0.04], [0.03, 0.80, -0.04], [0., 0.84, -0.04]] {
        vertices.push(SceneVertex {
            position: p,
            uv: [-7., 0.],
            color: [0.2, 0.1, 0.05, 1.],
        });
    }
    indices.extend([body as u32, body as u32 + 1, body as u32 + 2]);
    let mesh = voxy_render::SceneMesh::new(vertices, indices)
        .unwrap()
        .with_prepared_upload_streams();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    eprintln!("JUMP GPU ADAPTER {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let offsets: Vec<u32> = (0..=mesh.vertices().len() as u32).collect();
    let weights = vec![
        voxy_render::SurfaceDeformationWeight {
            control: 0,
            weight: 1.
        };
        mesh.vertices().len()
    ];
    let mut job = renderer
        .prepare_surface_deformation(&device, &mesh, &offsets, &weights, 1)
        .unwrap();
    job.enable_rigid_skinning(
        &device,
        mesh.authored_normals().unwrap(),
        &rig.gpu_jump_weights(),
        rig.skeleton.joints().len() as u32,
        Some(3),
    )
    .unwrap();
    job.set_deforming_normal_prefix(body as u32).unwrap();
    let mut maximum_error = 0f32;
    for time in [0., 0.38, 0.81, 1.26, 5., 0.38] {
        let palette = rig.jump_palette(time);
        let bob = crate::jump_motion::sample(time).height as f32;
        job.upload_rigid_pose(&queue, &palette).unwrap();
        let mut invalid = palette.clone();
        invalid[0] *= Mat4::from_scale(Vec3::splat(2.));
        assert!(
            job.upload_rigid_pose(&queue, &invalid).is_err(),
            "nonrigid palette accepted"
        );
        job.update_controls(&queue, &[[0., bob, 0., 0.]]).unwrap();
        let size = mesh.vertices().len() as u64 * 36;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode(&mut encoder).unwrap();
        encoder.copy_buffer_to_buffer(job.vertex_buffer(), 0, &staging, 0, size);
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
        let raw = staging.slice(..).get_mapped_range().unwrap();
        let actual: Vec<SceneVertex> = raw
            .chunks_exact(36)
            .map(|b| SceneVertex {
                position: std::array::from_fn(|k| {
                    f32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap())
                }),
                uv: std::array::from_fn(|k| {
                    f32::from_le_bytes(b[12 + k * 4..16 + k * 4].try_into().unwrap())
                }),
                color: std::array::from_fn(|k| {
                    f32::from_le_bytes(b[20 + k * 4..24 + k * 4].try_into().unwrap())
                }),
            })
            .collect();
        drop(raw);
        staging.unmap();
        let mut expected = asset.mesh.vertices().to_vec();
        rig.deform_jump(&mut expected, time);
        for (i, vertex) in actual.iter().enumerate() {
            let p = if i < body {
                Vec3::from_array(expected[i].position)
            } else {
                palette[3].transform_point3(Vec3::from_array(mesh.vertices()[i].position))
            };
            let error = Vec3::from_array(vertex.position).distance(p + Vec3::Y * bob);
            maximum_error = maximum_error.max(error);
            assert!(
                error < 2e-6,
                "GPU jump differs at time={time} vertex={i}: {error}"
            );
            assert_eq!(vertex.uv, mesh.vertices()[i].uv);
            assert_eq!(vertex.color, mesh.vertices()[i].color);
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    eprintln!("JUMP GPU body_vertices={body} samples=6 maximum_position_error_m={maximum_error}");
}
