//! Native regression: yielding and interleaved rendering preserve complete SH.
use glam::{Mat4, Vec3};
use voxy_render::archive_scene::{
    ArchiveInstance, ArchiveMaterial, ArchiveMaterialSource, ArchiveMeshSource,
    ArchiveSceneRenderer, ArchiveVertex,
};
fn renderer(device: &wgpu::Device, queue: &wgpu::Queue) -> ArchiveSceneRenderer {
    let material = ArchiveMaterialSource {
        parameters: ArchiveMaterial::metallic_roughness([0.6, 0.2, 0.05, 1.], 0., 0.5, 0., -1.),
        images: [None; 4],
        double_sided: true,
    };
    let vertices = [[-2., -2., -1.], [2., -2., -1.], [0., 2., -1.]].map(|position| ArchiveVertex {
        position,
        normal: [0., 0., 1.],
        tangent: [1., 0., 0., 1.],
        uv: [0., 0.],
    });
    let mesh = ArchiveMeshSource {
        vertices: vertices.to_vec(),
        indices: vec![0, 1, 2],
        instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
        material: 0,
        center: Vec3::new(0., 0., -1.),
    };
    let mut result = ArchiveSceneRenderer::new_with_images(
        device,
        queue,
        wgpu::TextureFormat::Rgba16Float,
        16,
        16,
        vec![],
        vec![material],
        vec![mesh],
        &[],
        Vec3::new(-0.5, 1., 0.3).normalize(),
        [1., 0.9, 0.8, 100000.],
        (Vec3::splat(-2.), Vec3::splat(2.)),
    )
    .unwrap();
    result
        .set_antialiasing(voxy_render::ArchiveAntialiasing::Temporal)
        .unwrap();
    result
}
fn read(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Buffer, size: u64) -> Vec<u8> {
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact probe coefficients"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, &target, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    target
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = target.slice(..).get_mapped_range().unwrap();
    let result = data.to_vec();
    drop(data);
    target.unmap();
    result
}
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let (min, max, spacing) = (Vec3::splat(-0.2), Vec3::splat(0.2), 1.);
    let mut reference = renderer(&device, &queue);
    let count = reference.bake_irradiance_probes(min, max, spacing).unwrap();
    assert_eq!(count, 8);
    assert_eq!(
        reference.antialiasing(),
        voxy_render::ArchiveAntialiasing::Temporal
    );
    let expected = read(
        &device,
        &queue,
        reference.irradiance_probe_coefficients_buffer(),
        count as u64 * 64,
    );
    let mut stepped = renderer(&device, &queue);
    let mut bake = stepped
        .begin_irradiance_probe_bake(min, max, spacing)
        .unwrap();
    assert!(matches!(
        reference.advance_irradiance_probe_bake(&mut bake),
        Err(voxy_render::ComputeError::DeviceMismatch)
    ));
    assert_eq!(bake.progress(), (0, 8));
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("interleaved editor frame"),
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut faces = 0;
    loop {
        faces += 1;
        if stepped.advance_irradiance_probe_bake(&mut bake).unwrap() {
            break;
        }
        let submission = stepped.draw(&view, Vec3::new(0., 0., 1.), Vec3::NEG_Z);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
    }
    assert_eq!(faces, 48);
    assert_eq!(bake.progress(), (8, 8));
    assert!(stepped.advance_irradiance_probe_bake(&mut bake).unwrap());
    assert_eq!(
        stepped.antialiasing(),
        voxy_render::ArchiveAntialiasing::Temporal
    );
    let actual = read(
        &device,
        &queue,
        stepped.irradiance_probe_coefficients_buffer(),
        count as u64 * 64,
    );
    assert_eq!(
        actual, expected,
        "yielding/interleaved draws changed full-precision coefficients"
    );
    assert!(
        actual
            .chunks_exact(4)
            .all(|v| f32::from_le_bytes(v.try_into().unwrap()).is_finite())
    );
    assert!(actual.iter().any(|&v| v != 0));
    assert!(
        actual.chunks_exact(64).any(|p| p != &actual[..64]),
        "fixture must exercise different probe radiance"
    );
    println!(
        "GPU_PROGRESSIVE_PROBE_BAKE probes=8 faces=48 interleaved_editor_frames=47 coefficients=128 every_float32_bit_exact=true foreign_renderer_rejected=true temporal_aa_restored=true completion_idempotent=true full_resolution_retained=true adapter={:?}",
        adapter.get_info().name
    );
}
