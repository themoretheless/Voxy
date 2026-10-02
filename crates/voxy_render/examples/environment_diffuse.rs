//! HDR diffuse cube acceptance: constant-radiance conservation and six-face response.
use voxy_render::DiffuseEnvironmentConvolution;
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
        println!("DIFFUSE ENVIRONMENT GPU: {:?}", adapter.get_info());
        for invalid in [0, 4097, u32::MAX] {
            assert!(
                matches!(DiffuseEnvironmentConvolution::with_samples(&device, 8, invalid),
                Err(voxy_render::RendererError::InvalidPrefilterSamples(n)) if n == invalid)
            );
        }
        for (cube_size, samples) in [(8_u32, 1), (8, 64), (8, 256), (8, 4096), (1, 64), (3, 64)] {
            let filter = DiffuseEnvironmentConvolution::with_samples(&device, cube_size, samples)?;
            let original = filter.output().clone();
            let mut checked = 0;
            let mut changed = 0;
            for varied in [false, true] {
                for face in 0..6_u32 {
                    let red = f32::from(u16::try_from(face + 1)?);
                    let color = if varied {
                        [red, 0., 0., 1.]
                    } else {
                        [4., 2., 8., 1.]
                    };
                    let pixel: Vec<u8> = color
                        .into_iter()
                        .flat_map(|v| half::f16::from_f32(v).to_bits().to_le_bytes())
                        .collect();
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: filter.input(),
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: face,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &pixel.repeat(usize::try_from(cube_size * cube_size)?),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(cube_size * 8),
                            rows_per_image: Some(cube_size),
                        },
                        wgpu::Extent3d {
                            width: cube_size,
                            height: cube_size,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                filter.encode(&mut encoder);
                let mut reads = Vec::new();
                for mip in 0..filter.output().mip_level_count() {
                    for face in 0..6 {
                        let size = (cube_size >> mip).max(1);
                        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("DIFFUSE cube readback"),
                            size: u64::from(size) * 256,
                            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                            mapped_at_creation: false,
                        });
                        encoder.copy_texture_to_buffer(
                            wgpu::TexelCopyTextureInfo {
                                texture: filter.output(),
                                mip_level: mip,
                                origin: wgpu::Origin3d {
                                    x: 0,
                                    y: 0,
                                    z: face,
                                },
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::TexelCopyBufferInfo {
                                buffer: &buffer,
                                layout: wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(256),
                                    rows_per_image: Some(size),
                                },
                            },
                            wgpu::Extent3d {
                                width: size,
                                height: size,
                                depth_or_array_layers: 1,
                            },
                        );
                        reads.push((buffer, mip, face, size));
                    }
                }
                queue.submit([encoder.finish()]);
                assert_eq!(filter.output(), &original);
                for (buffer, mip, face, size) in reads {
                    let (tx, rx) = std::sync::mpsc::channel();
                    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                        let _ = tx.send(r);
                    });
                    device.poll(wgpu::PollType::wait_indefinitely())?;
                    rx.recv()??;
                    let bytes = buffer.slice(..).get_mapped_range()?;
                    for y in 0..size {
                        for x in 0..size {
                            for c in 0..4 {
                                let offset = usize::try_from(y * 256 + x * 8)? + c * 2;
                                let bits =
                                    u16::from_le_bytes(bytes[offset..offset + 2].try_into()?);
                                let value = half::f16::from_bits(bits).to_f32();
                                assert!(value.is_finite());
                                let face_value = f32::from(u16::try_from(face + 1)?);
                                if !varied {
                                    let expected = if varied {
                                        [face_value, 0., 0., 1.][c]
                                    } else {
                                        [4., 2., 8., 1.][c]
                                    };
                                    assert!(
                                        bits.abs_diff(half::f16::from_f32(expected).to_bits()) <= 1,
                                        "varied={varied} mip={mip} face={face} pixel {x},{y}/{c}: {value} != {expected}"
                                    );
                                    checked += 1;
                                } else if c == 0 {
                                    assert!((0.999..=6.001).contains(&value));
                                    if mip + 1 == filter.output().mip_level_count()
                                        && x == size / 2
                                        && y == size / 2
                                        && (value - face_value).abs() > 0.05
                                    {
                                        changed += 1;
                                    }
                                } else {
                                    assert_eq!(value, if c == 3 { 1. } else { 0. });
                                }
                            }
                        }
                    }
                    drop(bytes);
                    buffer.unmap();
                }
            }
            if samples > 1 {
                assert!(
                    changed >= 2,
                    "diffuse convolution must mix six-face radiance"
                );
            } else {
                assert_eq!(changed, 0);
            }
            assert!(DiffuseEnvironmentConvolution::new(&device, 0).is_err());
            println!(
                "DIFFUSE ENVIRONMENT PASS size={cube_size} samples={samples}: {checked} constant channel references, bounded diffuse radiance, {changed} changed diffuse face centers, retained output, invalid size rejection"
            );
        }
        Ok(())
    })
}
