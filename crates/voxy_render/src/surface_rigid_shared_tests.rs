use super::*;
use crate::{ComputeMemoryBudget, SceneMesh, SceneRenderer};

pub(crate) fn device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("shared palette adapter: {:?}", adapter.get_info());
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
        ..Default::default()
    }))
    .unwrap()
}
fn skin(device: &wgpu::Device, joints: u32) -> SurfaceRigidSkinning {
    let mesh = SceneMesh::quad([1.; 4]);
    let renderer = SceneRenderer::new(device, wgpu::TextureFormat::Rgba8Unorm);
    let deformation = renderer
        .prepare_surface_deformation(device, &mesh, &vec![0; mesh.vertices().len() + 1], &[], 1)
        .unwrap();
    let geometry = deformation.geometry();
    let [rest] = ComputeMemoryBudget::for_device(device)
        .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
            label: "shared skin test rest",
            size: mesh.vertices().len() as u64 * 36,
            contents: Some(bytemuck::cast_slice(mesh.vertices())),
            usage: wgpu::BufferUsages::STORAGE,
        }])
        .unwrap();
    let normals = vec![[0., 0., 1.]; mesh.vertices().len()];
    let weights = vec![
        SurfaceRigidSkinWeight {
            joints: [0, joints - 1, 0, 0],
            weights: [0.25, 0.75, 0., 0.]
        };
        normals.len()
    ];
    SurfaceRigidSkinning::new(
        device, &rest, &normals, geometry, &weights, joints, None, 2.,
    )
    .unwrap()
}
fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    data
}
#[test]
#[ignore = "requires physical Apple Silicon Metal GPU"]
fn gpu_shared_rigid_matches_legacy_and_rejects_invalid_snapshots() {
    let (device, queue) = device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shared = skin(&device, 2);
    assert!(shared.shared.is_some());
    let mut legacy = skin(&device, 2);
    legacy.shared = None;
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let size = shared.posed.size();
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size * 12,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut unsubmitted = Vec::new();
    for frame in 0..6 {
        let matrices = [
            Mat4::from_rotation_y(frame as f32 * 0.13),
            Mat4::from_rotation_translation(
                Quat::from_rotation_z(0.17),
                glam::Vec3::new(0.2, frame as f32, 0.3),
            ),
        ];
        let offset = glam::Vec3::new(frame as f32 * 10., -0.5, 0.75);
        for (kind, skin) in [(0, &legacy), (1, &shared)] {
            skin.upload_with_offset(&queue, &matrices, offset).unwrap();
            let mut invalid = matrices;
            invalid[1] = Mat4::from_cols_array(&[f32::NAN; 16]);
            assert!(
                skin.upload_with_offset(&queue, &invalid, glam::Vec3::ZERO)
                    .is_err()
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            skin.encode(&mut encoder).unwrap();
            encoder.copy_buffer_to_buffer(&skin.posed, 0, &output, (frame * 2 + kind) * size, size);
            if kind == 0 {
                queue.submit([encoder.finish()]);
            } else {
                unsubmitted.push(encoder.finish());
            }
        }
    }
    // Three shared slots plus three staged overflow snapshots precede submission.
    queue.submit(unsubmitted);
    let data = read(&device, &output);
    for frame in data.chunks_exact((size * 2) as usize) {
        assert_eq!(&frame[..size as usize], &frame[size as usize..]);
    }
    // Re-encoding without upload and ring reuse must preserve the snapshot too.
    let mut encoder = device.create_command_encoder(&Default::default());
    shared.encode(&mut encoder).unwrap();
    encoder.copy_buffer_to_buffer(&shared.posed, 0, &output, 0, size);
    queue.submit([encoder.finish()]);
    assert_eq!(
        &read(&device, &output)[..size as usize],
        &data[data.len() - size as usize..]
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "manual same-process paired production benchmark on Apple Silicon"]
fn paired_rigid_palette_benchmark() {
    let (device, queue) = device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    println!("joints,pair,path,cpu_us_per_frame,drained_us_per_frame");
    for joints in [32, 1024, 8192] {
        let shared = skin(&device, joints);
        assert!(shared.shared.is_some());
        let mut legacy = skin(&device, joints);
        legacy.shared = None;
        let mut matrices: Vec<_> = (0..joints)
            .map(|i| {
                Mat4::from_rotation_translation(
                    Quat::from_rotation_y(i as f32 * 0.001),
                    glam::Vec3::new(0.1, 0.2, 0.3),
                )
            })
            .collect();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        for pair in 0..10 {
            for use_shared in if pair % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let skin = if use_shared { &shared } else { &legacy };
                let start = std::time::Instant::now();
                let mut cpu = 0.;
                for frame in 0..48 {
                    matrices[0].w_axis.x = frame as f32 * 0.001;
                    let step = std::time::Instant::now();
                    skin.upload_with_offset(&queue, &matrices, glam::Vec3::X)
                        .unwrap();
                    let mut encoder = device.create_command_encoder(&Default::default());
                    skin.encode(&mut encoder).unwrap();
                    queue.submit([encoder.finish()]);
                    cpu += step.elapsed().as_secs_f64() * 1e6;
                    if frame % 3 == 2 {
                        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    }
                }
                let total = start.elapsed().as_secs_f64() * 1e6;
                if pair != 0 {
                    println!(
                        "{joints},{pair},{},{:.3},{:.3}",
                        if use_shared { "shared" } else { "legacy" },
                        cpu / 48.,
                        total / 48.
                    );
                }
            }
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
