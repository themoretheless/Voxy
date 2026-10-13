use super::*;
use crate::{
    ComputeDispatch, ComputeProgram, FIBER_SURFACE_SHADER, FiberSurfaceFrame, FiberSurfaceInput,
    FiberSurfaceTransfer, GraphicsOptions,
};

#[test]
fn fibre_scatter_preserves_draw_streams_and_neighbouring_geometry() {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).expect("GPU required");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let frame = FiberSurfaceFrame {
        position_arc: [0.2, 0.8, -0.1, 1.],
        u_red: [0., 1., 0., 0.055],
        v_green: [-1., 0., 0., 0.022],
        w_blue: [0., 0., 1., 0.009],
    };
    let offsets: Vec<_> = (0..60)
        .map(|i| [i as f32 * 0.0001, -0.001, 0.002, 0.8])
        .collect();
    let input = FiberSurfaceInput::new(&[frame], 60, &offsets, 0.0003, 0.6).unwrap();
    let count = input.vertex_count() as usize;
    let sentinel = SceneVertex {
        position: [0.3, 0.4, 0.5],
        uv: [0.6, 0.7],
        color: [0.1, 0.2, 0.3, 1.],
    };
    let mesh = SceneMesh::new(vec![sentinel; count + 6], vec![0, 1, 2])
        .unwrap()
        .with_normals(vec![[0., 0., 1.]; count + 6])
        .unwrap();
    let scene = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let geometry = scene
        .upload_mesh_with_usage(
            &device,
            &mesh,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        )
        .unwrap();
    let program = pollster::block_on(ComputeProgram::new(&device, FIBER_SURFACE_SHADER)).unwrap();
    let job = program.create_job(&device, input.bytes()).unwrap();
    let transfer = pollster::block_on(FiberSurfaceTransfer::new(
        &device,
        &geometry,
        job.buffer(),
        &input,
        3,
    ))
    .unwrap();
    // Reject an out-of-bounds partition before encoding a write.
    assert!(
        pollster::block_on(FiberSurfaceTransfer::new(
            &device,
            &geometry,
            job.buffer(),
            &input,
            7
        ))
        .is_err()
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder, [input.vertex_count().div_ceil(64), 1, 1])
        .unwrap();
    transfer.encode(&mut encoder);
    let vertex_bytes = ((count + 6) * 36) as u64;
    let normal_bytes = ((count + 6) * 12) as u64;
    let dispatch = ComputeDispatch::gather_buffers(
        &device,
        &mut encoder,
        &[
            (geometry.deformation_vertices(), 0, 0, vertex_bytes),
            (
                geometry.deformation_normals(),
                0,
                vertex_bytes,
                normal_bytes,
            ),
            (
                job.buffer(),
                0,
                vertex_bytes + normal_bytes,
                input.bytes().len() as u64,
            ),
        ],
        vertex_bytes + normal_bytes + input.bytes().len() as u64,
    )
    .unwrap();
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = pending.try_read().unwrap().unwrap();
    let vertices = &bytes[..vertex_bytes as usize];
    let normals = &bytes[vertex_bytes as usize..(vertex_bytes + normal_bytes) as usize];
    let generated = input
        .output(&bytes[(vertex_bytes + normal_bytes) as usize..])
        .unwrap();
    for i in 0..count + 6 {
        let vertex = &vertices[i * 36..(i + 1) * 36];
        let normal = &normals[i * 12..(i + 1) * 12];
        if (3..count + 3).contains(&i) {
            let source = &generated[(i - 3) * 48..(i - 2) * 48];
            assert_eq!(vertex, &source[..36], "vertex stream {i}");
            assert_eq!(normal, &source[36..], "normal stream {i}");
        } else {
            assert_eq!(
                vertex,
                bytemuck::bytes_of(&sentinel),
                "neighbour vertex {i}"
            );
            assert_eq!(
                normal,
                bytemuck::bytes_of(&[0f32, 0., 1.]),
                "neighbour normal {i}"
            );
        }
    }
}

#[test]
fn fibre_director_scatter_tracks_pose_and_preserves_neighbours() {
    pollster::block_on(async {
        let instance = GraphicsOptions::default().create_instance();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("GPU required");
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let frame = FiberSurfaceFrame {
            position_arc: [0., 0.8, 0., 0.5],
            u_red: [1., 0., 0., 0.1],
            v_green: [0., 1., 0., 0.2],
            w_blue: [0., 0., 1., 0.3],
        };
        let mut input =
            FiberSurfaceInput::new(&[frame], 2, &[[0., 0., 0., 1.]; 2], 0.0003, 0.6).unwrap();
        let count = input.vertex_count() as usize;
        let mesh = SceneMesh::new(
            vec![
                SceneVertex {
                    position: [0.; 3],
                    uv: [0.; 2],
                    color: [1.; 4]
                };
                count + 6
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let sentinel = [0., 1., 0., -1.];
        let geometry = renderer
            .upload_compute_tangent_mesh(&device, &mesh, &vec![sentinel; count + 6])
            .unwrap();
        let plain = renderer.upload_compute_mesh(&device, &mesh).unwrap();
        let program = ComputeProgram::new(&device, FIBER_SURFACE_SHADER)
            .await
            .unwrap();
        let job = program.create_job(&device, input.bytes()).unwrap();
        assert!(
            FiberSurfaceTransfer::new_with_tangents(&device, &plain, job.buffer(), &input, 3)
                .await
                .is_err()
        );
        assert!(
            FiberSurfaceTransfer::new_with_tangents(&device, &geometry, job.buffer(), &input, 7)
                .await
                .is_err()
        );
        let transfer =
            FiberSurfaceTransfer::new_with_tangents(&device, &geometry, job.buffer(), &input, 3)
                .await
                .unwrap();
        for angle in [0.37, -0.81] {
            // Nonuniform affine pose exercises normalization and w=0 translation exclusion.
            let pose = glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::new(0.7, 1.3, 0.9),
                glam::Quat::from_rotation_y(angle),
                glam::Vec3::new(3., -4., 2.),
            );
            let offset = input.transform_byte_offset();
            queue.write_buffer(job.buffer(), offset, input.replace_transform(pose).unwrap());
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
            transfer.encode(&mut encoder);
            let size = ((count + 6) * 16) as u64;
            let dispatch = ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[(geometry.deformation_tangents().unwrap(), 0, 0, size)],
                size,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            let values: &[[f32; 4]] = bytemuck::cast_slice(&bytes);
            let expected = pose.transform_vector3(glam::Vec3::Z).normalize();
            for (i, value) in values.iter().enumerate() {
                if (3..count + 3).contains(&i) {
                    assert!(
                        glam::Vec3::from_slice(value).distance(expected) < 3e-7,
                        "director {i}: {value:?}"
                    );
                    assert!((glam::Vec3::from_slice(value).length() - 1.).abs() < 3e-7);
                    assert_eq!(value[3], 1.);
                } else {
                    assert_eq!(*value, sentinel, "neighbour {i}");
                }
            }
        }
    });
}
