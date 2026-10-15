//! Actual GPU readback of low bits and HDR values in scanned-material transport.
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::FLOAT32_FILTERABLE,
        ..Default::default()
    }))
    .unwrap();
    let expected = vec![
        1.0_f32 / 65535.,
        2. / 65535.,
        0.12345679,
        1.,
        1.000001,
        2.500001,
        0.8765432,
        1.,
    ];
    let source = voxy_render::archive_scene::ArchiveImageSource::Float {
        width: 2,
        height: 1,
        pixels: expected.clone(),
    };
    let texture = source.upload(&device, &queue).unwrap();
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba32Float);
    assert_eq!(texture.mip_level_count(), 2);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("float material readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    for (actual, expected) in mapped[..32].chunks_exact(4).zip(expected) {
        assert_eq!(
            u32::from_le_bytes(actual.try_into().unwrap()),
            expected.to_bits()
        );
    }
    println!(
        "GPU_FLOAT_MATERIAL adapter={} original_float32_bits_exact=true complete_mip_chain=true",
        adapter.get_info().name
    );
}
