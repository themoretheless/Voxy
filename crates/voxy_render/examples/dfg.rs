//! GPU split-sum integration versus double-precision CPU reference.
use voxy_render::GgxDfgLut;
fn reference(nv: f64, r: f64, samples: u32) -> [f64; 4] {
    let a2 = r.powi(4);
    let vx = (1. - nv * nv).sqrt();
    let mut ab = [0.; 4];
    for i in 0..samples {
        let phi = std::f64::consts::TAU * f64::from(i) / f64::from(samples);
        let xi = f64::from(i.reverse_bits()) / 4294967296.;
        let nh = ((1. - xi) / (1. + (a2 - 1.) * xi)).sqrt();
        let vh = (vx * phi.cos() * (1. - nh * nh).sqrt() + nv * nh).max(0.);
        let nl = (2. * vh * nh - nv).max(0.);
        if nl > 0. && vh > 0. {
            let den =
                nl * (nv * nv * (1. - a2) + a2).sqrt() + nv * (nl * nl * (1. - a2) + a2).sqrt();
            let weight = 2. * nl * vh / (nh * den);
            let fc = (1. - vh).clamp(0., 1.).powi(5);
            ab[0] += (1. - fc) * weight / f64::from(samples);
            ab[1] += fc * weight / f64::from(samples);
        }
    }
    ab[3] = 1.;
    ab
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        println!("DFG GPU: {:?}", adapter.get_info());
        for (size, samples) in [(1, 1), (3, 64), (8, 256), (8, 4096)] {
            let lut = GgxDfgLut::with_samples(&device, size, samples)?;
            let original = lut.output().clone();
            for _ in 0..2 {
                let mut encoder = device.create_command_encoder(&Default::default());
                lut.encode(&mut encoder);
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: u64::from(size) * 256,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: lut.output(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
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
                queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                device.poll(wgpu::PollType::wait_indefinitely())?;
                rx.recv()??;
                let bytes = buffer.slice(..).get_mapped_range()?;
                for y in 0..size {
                    for x in 0..size {
                        let expected = reference(
                            (f64::from(x) + 0.5) / f64::from(size),
                            (f64::from(y) + 0.5) / f64::from(size),
                            samples,
                        );
                        for (c, expected) in expected.into_iter().enumerate() {
                            let offset = usize::try_from(y * 256 + x * 8)? + c * 2;
                            let value = half::f16::from_bits(u16::from_le_bytes(
                                bytes[offset..offset + 2].try_into()?,
                            ))
                            .to_f64();
                            assert!(
                                value.is_finite() && (value - expected).abs() < 0.002,
                                "{size}/{samples} ({x},{y}) channel {c}: {value} vs {expected}"
                            );
                        }
                    }
                }
                assert_eq!(lut.output(), &original);
            }
            println!(
                "DFG PASS size={size} samples={samples}: {} CPU channel references, retained output",
                size * size * 8
            );
        }
        for n in [0, 4097, u32::MAX] {
            assert!(
                matches!(GgxDfgLut::with_samples(&device,8,n), Err(voxy_render::RendererError::InvalidPrefilterSamples(v)) if v==n)
            );
        }
        assert!(GgxDfgLut::new(&device, 0).is_err());
        Ok(())
    })
}
