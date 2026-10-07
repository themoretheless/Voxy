//! Verify asynchronous diagnostic readback against a nonuniform HDR texture.
use voxy_render::HdrPixelProbe;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR probe fixture"),
            size: wgpu::Extent3d {
                width: 3,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let pixels: Vec<u16> = (0..6)
            .flat_map(|index| {
                [index as f32 + 2.0, 0.25, 16.0, 1.0].map(|v| half::f16::from_f32(v).to_bits())
            })
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(24),
                rows_per_image: Some(2),
            },
            texture.size(),
        );
        let mut probe = HdrPixelProbe::new(&device);
        probe.begin_read(); // No copy yet: must remain usable.
        assert!(probe.take_result().is_none());
        let mut encoder = device.create_command_encoder(&Default::default());
        assert!(probe.encode(&mut encoder, &texture, 3, 0).is_err());
        assert!(probe.encode(&mut encoder, &texture, 0, 2).is_err());
        for (format, usage) in [
            (wgpu::TextureFormat::R8Unorm, wgpu::TextureUsages::COPY_SRC),
            (
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureUsages::TEXTURE_BINDING,
            ),
        ] {
            let invalid = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("invalid probe source"),
                size: texture.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            });
            assert!(probe.encode(&mut encoder, &invalid, 0, 0).is_err());
        }
        probe.encode(&mut encoder, &texture, 1, 1)?;
        probe.encode(&mut encoder, &texture, 0, 0)?; // One-shot: first copy wins.
        queue.submit([encoder.finish()]);
        probe.begin_read();
        probe.begin_read(); // Must not request a second mapping.
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let value = probe.take_result().expect("mapping completed")?;
        assert_eq!(value, [6.0, 0.25, 16.0, 1.0]);
        assert!(probe.take_result().is_none());
        let full = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RGBA32 temporal probe fixture"),
            size: texture.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let reference = [70000.125_f32, 0.123456, 16.25, 1.0];
        let pixels: Vec<f32> = (0..6)
            .flat_map(|index| if index == 4 { reference } else { [0.0; 4] })
            .collect();
        queue.write_texture(
            full.as_image_copy(),
            bytemuck::cast_slice(&pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(48),
                rows_per_image: Some(2),
            },
            full.size(),
        );
        let mut full_probe = HdrPixelProbe::new(&device);
        let mut encoder = device.create_command_encoder(&Default::default());
        full_probe.encode(&mut encoder, &full, 1, 1)?;
        queue.submit([encoder.finish()]);
        full_probe.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        assert_eq!(
            full_probe
                .take_result()
                .expect("RGBA32 mapping completed")?,
            reference
        );
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            let source = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("presentation pixel fixture"),
                size: texture.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let bgra = matches!(
                format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            );
            let rgba = [17u8, 83, 201, 129];
            let storage = if bgra {
                [rgba[2], rgba[1], rgba[0], rgba[3]]
            } else {
                rgba
            };
            let mut pixels = vec![0u8; 24];
            pixels[16..20].copy_from_slice(&storage);
            queue.write_texture(
                source.as_image_copy(),
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(12),
                    rows_per_image: Some(2),
                },
                source.size(),
            );
            let mut probe = HdrPixelProbe::new(&device);
            let mut encoder = device.create_command_encoder(&Default::default());
            probe.encode(&mut encoder, &source, 1, 1)?;
            queue.submit([encoder.finish()]);
            probe.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely())?;
            assert_eq!(
                probe.take_result().expect("8-bit mapping completed")?,
                rgba.map(|v| f32::from(v) / 255.)
            );
            assert!(probe.take_result().is_none());
            println!("PRESENTATION PIXEL PASS format={format:?}");
        }
        println!(
            "HDR probe PASS {:?}: nonzero origin, HDR range, rejection and one-shot lifecycle",
            adapter.get_info()
        );
        Ok(())
    })
}
