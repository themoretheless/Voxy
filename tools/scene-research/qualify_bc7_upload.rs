//! Physical GPU compressed-mip transfer qualification; no rendered-scene claim.
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let image = voxy_render::Bc7ImageAsset::decode_dds(
        std::fs::read(path).unwrap().into(),
        512 * 1024 * 1024,
        16384,
    )
    .unwrap();
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    assert!(
        adapter
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    );
    let descriptor = wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let texture = image.upload(&device, &queue).unwrap();
    let last = image.mip_ranges.len() - 1;
    let expected = image.mip_bytes(last).unwrap();
    assert_eq!(expected.len(), 16);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("BC7 original block readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: last as u32,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
    );
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
    assert_eq!(&mapped[..16], expected);
    drop(mapped);
    readback.unmap();
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "BC7 UPLOAD adapter={} format={:?} dimensions={}x{} mip_levels={} last_mip_block_bit_exact=true resident_payload_bytes={}",
        adapter.get_info().name,
        image.format,
        image.width,
        image.height,
        image.mip_ranges.len(),
        texture.allocation_bytes()
    );
}
