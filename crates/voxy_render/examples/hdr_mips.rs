//! Numeric linear HDR pyramid proof, including odd and one-dimensional inputs.
use voxy_render::HdrMipPyramid;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        println!("HDR MIP GPU: {:?}", adapter.get_info());
        let mut checked = 0;
        for (width, height) in [(8_u32, 8_u32), (7, 5), (1, 7), (1, 1)] {
            let pyramid = HdrMipPyramid::new(&device, width, height)?;
            let mut reference: Vec<[half::f16; 4]> = (0..height)
                .flat_map(|y| {
                    (0..width).map(move |x| {
                        [f32::from(u16::try_from(x * 2 + y).unwrap()), 4.0, 16.0, 1.0]
                            .map(half::f16::from_f32)
                    })
                })
                .collect();
            let upload: Vec<u8> = reference
                .iter()
                .flatten()
                .flat_map(|value| value.to_bits().to_le_bytes())
                .collect();
            queue.write_texture(
                pyramid.texture().as_image_copy(),
                &upload,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 8),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            pyramid.encode(&mut encoder);
            let mut reads = Vec::new();
            let (mut w, mut h) = (width, height);
            for level in 0..pyramid.texture().mip_level_count() {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("HDR mip readback"),
                    size: u64::from(h) * 256,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: pyramid.texture(),
                        mip_level: level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(256),
                            rows_per_image: Some(h),
                        },
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
                reads.push((buffer, w, h, reference.clone()));
                let (dw, dh) = ((w / 2).max(1), (h / 2).max(1));
                let mut next = Vec::new();
                for y in 0..dh {
                    for x in 0..dw {
                        let (x0, x1, y0, y1) =
                            (x * w / dw, (x + 1) * w / dw, y * h / dh, (y + 1) * h / dh);
                        let mut sum = [0_f32; 4];
                        for sy in y0..y1 {
                            for sx in x0..x1 {
                                for c in 0..4 {
                                    sum[c] += reference[usize::try_from(sy * w + sx)?][c].to_f32();
                                }
                            }
                        }
                        let count = f32::from(u16::try_from((x1 - x0) * (y1 - y0))?);
                        next.push(sum.map(|v| half::f16::from_f32(v / count)));
                    }
                }
                reference = next;
                w = dw;
                h = dh;
            }
            queue.submit([encoder.finish()]);
            for (buffer, w, h, expected) in reads {
                let (tx, rx) = std::sync::mpsc::channel();
                buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                device.poll(wgpu::PollType::wait_indefinitely())?;
                rx.recv()??;
                let bytes = buffer.slice(..).get_mapped_range()?;
                for y in 0..h {
                    for x in 0..w {
                        for c in 0..4 {
                            let offset = usize::try_from(y * 256 + x * 8)? + c * 2;
                            let actual = u16::from_le_bytes(bytes[offset..offset + 2].try_into()?);
                            let expected = expected[usize::try_from(y * w + x)?][c].to_bits();
                            assert!(
                                half::f16::from_bits(actual).is_finite()
                                    && actual.abs_diff(expected) <= 1,
                                "{width}x{height} level {w}x{h} pixel {x},{y}/{c}: {actual} != {expected}"
                            );
                            checked += 1;
                        }
                    }
                }
                drop(bytes);
                buffer.unmap();
            }
        }
        assert!(HdrMipPyramid::new(&device, 0, 1).is_err());
        println!(
            "HDR MIP PASS: {checked} half-float channel references, odd edges, 1D/1x1, linear radiance above one, invalid size rejection"
        );
        Ok(())
    })
}
