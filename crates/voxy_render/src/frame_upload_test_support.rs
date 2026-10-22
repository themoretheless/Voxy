//! Physical-GPU helpers; timing waits are deliberately outside production code.
use std::time::Instant;
pub(crate) fn device() -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("frame input adapter={:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
        ..Default::default()
    }))
    .unwrap();
    (adapter, device, queue)
}
pub(crate) fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    bytes
}
pub(crate) fn output(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame input parity target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 16,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
pub(crate) fn capture(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    offset: u64,
) {
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(256),
                rows_per_image: Some(16),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 16,
            depth_or_array_layers: 1,
        },
    );
}
pub(crate) fn pairs(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &str,
    count: usize,
    mut frame: impl FnMut(bool, u32, &mut wgpu::CommandEncoder),
) {
    // AB/BA pairs in one process. The first complete pair warms both paths.
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    for pair in 0..10 {
        for shared in if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let all = Instant::now();
            let mut cpu = 0.;
            for index in 0..24 {
                let start = Instant::now();
                let mut encoder = device.create_command_encoder(&Default::default());
                frame(shared, index, &mut encoder);
                queue.submit([encoder.finish()]);
                cpu += start.elapsed().as_secs_f64() * 1e6;
                if index % 3 == 2 {
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                }
            }
            if pair != 0 {
                println!(
                    "{path},{count},{pair},{},{:.3},{:.3}",
                    if shared { "shared" } else { "legacy" },
                    cpu / 24.,
                    all.elapsed().as_secs_f64() * 1e6 / 24.
                );
            }
        }
    }
}

/// Interleave short AB/BA blocks; include generation, validation, encoding and
/// submission in CPU time. Drain each three frames identically for both paths.
pub(crate) fn generation_pairs_mode(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &str,
    count: usize,
    submit: bool,
    mut frame: impl FnMut(bool, u32, &mut wgpu::CommandEncoder),
) {
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    for pair in 0..10 {
        let mut cpu = [0.; 2];
        let mut drained = [0.; 2];
        for block in 0..32 {
            for mode in if (pair + block) % 2 == 0 {
                [0, 1]
            } else {
                [1, 0]
            } {
                let all = Instant::now();
                for index in 0..3 {
                    let started = Instant::now();
                    let mut encoder = device.create_command_encoder(&Default::default());
                    frame(mode == 1, block * 3 + index, &mut encoder);
                    if submit {
                        queue.submit([encoder.finish()]);
                    } else {
                        drop(encoder.finish());
                    }
                    cpu[mode] += started.elapsed().as_secs_f64() * 1e6;
                }
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                drained[mode] += all.elapsed().as_secs_f64() * 1e6;
            }
        }
        if pair != 0 {
            for mode in 0..2 {
                println!(
                    "GEN,{path},{count},{pair},{},{:.3},{:.3}",
                    if mode == 1 { "after" } else { "before" },
                    cpu[mode] / 96.,
                    drained[mode] / 96.
                );
            }
        }
    }
}
