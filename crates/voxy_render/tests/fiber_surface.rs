use glam::{Quat, Vec3};
use voxy_render::{
    ComputeProgram, FIBER_SURFACE_SHADER, FiberSurfaceFrame, FiberSurfaceInput, GraphicsOptions,
};

/// Full authored groom dimensions, including terminal frames and all followers.
/// This qualifies geometry extraction, not simulation throughput or rendered FPS.
#[test]
fn full_density_gpu_fibres_preserve_rotated_local_frames() {
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("GPU required for fibre extraction qualification");
    eprintln!("FIBRE QUALIFICATION adapter={:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let mut frames = Vec::new();
    let mut offsets = Vec::new();
    for guide in 0..469 {
        for point in 0..21 {
            let arc = point as f32 / 20.;
            let q = Quat::from_rotation_y(guide as f32 * 0.037)
                * Quat::from_rotation_x(arc * 2.4)
                * Quat::from_rotation_z(arc * 4.1);
            let [u, v, w] = [q * Vec3::X, q * Vec3::Y, q * Vec3::Z];
            frames.push(FiberSurfaceFrame {
                position_arc: [guide as f32 * 0.001, 0.8 - arc * 0.3, arc * arc * 0.07, arc],
                u_red: [u.x, u.y, u.z, 0.055],
                v_green: [v.x, v.y, v.z, 0.022],
                w_blue: [w.x, w.y, w.z, 0.009],
            });
            for follower in 0..60 {
                let angle = follower as f32 * 2.399963;
                offsets.push([
                    angle.cos() * 0.002,
                    angle.sin() * 0.002,
                    arc * (follower as f32 - 30.) * 0.00001,
                    0.8 + ((guide + follower) % 7) as f32 * 0.04,
                ]);
            }
        }
    }
    let input = FiberSurfaceInput::new(&frames, 60, &offsets, 0.00030, 0.6).unwrap();
    assert_eq!(input.vertex_count(), 2_363_760);
    // Full-output qualification needs more than the production pool's 64 MiB.
    // Runtime extraction draws resident buffers and does not need this readback.
    let _readback_pool = voxy_render::ComputeReadbackPool::configure(
        &device,
        voxy_render::ComputeReadbackLimits {
            max_bytes: input.bytes().len() as u64,
            max_buffers: 1,
        },
    )
    .unwrap();
    let program = pollster::block_on(ComputeProgram::new(&device, FIBER_SURFACE_SHADER)).unwrap();
    let job = program.create_job(&device, input.bytes()).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    let dispatch = job
        .encode(&mut encoder, [input.vertex_count().div_ceil(64), 1, 1])
        .unwrap();
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = pending.try_read().unwrap().expect("completed GPU readback");
    let output: &[f32] = bytemuck::cast_slice(input.output(&bytes).unwrap());
    let mut max_error = 0f32;
    for (vertex, actual) in output.chunks_exact(12).enumerate() {
        let frame = frames[vertex / (60 * 4)];
        let offset = offsets[vertex / 4];
        let [u, v, w] = [
            Vec3::from_slice(&frame.u_red),
            Vec3::from_slice(&frame.v_green),
            Vec3::from_slice(&frame.w_blue),
        ];
        let normal = [u, v, -u, -v][vertex % 4];
        let arc = frame.position_arc[3];
        let reference = Vec3::from_slice(&frame.position_arc)
            + u * offset[0]
            + v * offset[1]
            + w * offset[2]
            + normal * (0.00030 * (1. - 0.6 * arc));
        let error = Vec3::from_slice(actual).distance(reference);
        assert!(
            error.is_finite() && error < 2e-7,
            "vertex={vertex} error={error}"
        );
        max_error = max_error.max(error);
        assert_eq!(&actual[3..5], &[-7., arc]);
        for (channel, base) in [0.055, 0.022, 0.009].into_iter().enumerate() {
            assert!((actual[5 + channel] - base * offset[3]).abs() < 1e-7);
        }
        assert_eq!(actual[8], 1.);
        assert!(Vec3::from_slice(&actual[9..]).distance(normal) < 1e-7);
    }
    eprintln!(
        "FIBRE QUALIFICATION vertices={} max_position_error_m={max_error}",
        input.vertex_count()
    );
}
