//! Numeric GPU receiver proof: shadow On/Off at matching camera/material/light.
use glam::{Mat4, Quat, Vec3};
use voxy_render::{SceneDraw, SceneMesh, SceneRenderer, ShadowMap, ShadowSettings};
fn read(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = buffer.slice(..).get_mapped_range()?.to_vec();
    buffer.unmap();
    Ok(bytes)
}
fn capture(texture: &wgpu::Texture, encoder: &mut wgpu::CommandEncoder, buffer: &wgpu::Buffer) {
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(8),
            },
        },
        wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
    );
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        println!("SHADOW VISIBILITY GPU: {:?}", adapter.get_info());
        let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba16Float);
        let map = ShadowMap::new(&device, 8, 8)?;
        let settings = ShadowSettings {
            light_from_world: Mat4::IDENTITY,
            bias: 0.0001,
            enabled: true,
            filter: voxy_render::ShadowFilter::Hard,
        };
        renderer
            .enable_shadowed_point_light(&device, &map, settings)
            .await?;
        let caster = renderer.upload_mesh(&device, &SceneMesh::quad([1.; 4]))?;
        let receiver = renderer.upload_mesh(&device, &SceneMesh::quad([0.5, 0.5, 0.5, 1.]))?;
        let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
        let world = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.),
            Quat::IDENTITY,
            Vec3::new(0., 0., 0.75),
        );
        let transform = renderer.create_transform(&device, world)?;
        transform.update_scene_material(&queue, world, [1.; 4], [0., 0., 2., 5.])?;
        transform.update_view_position(&queue, Vec3::new(0., 0., 2.))?;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow receiver radiance"),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = ShadowMap::new(&device, 8, 8)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2048,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let draws = [SceneDraw {
            geometry: &receiver,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        let light = Mat4::from_scale_rotation_translation(
            Vec3::new(1., 2., 1.),
            Quat::IDENTITY,
            Vec3::new(-0.5, 0., 0.25),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        map.encode(&mut encoder, &[map.prepare(&caster, light)?])?;
        renderer.encode(
            &mut encoder,
            &output.create_view(&Default::default()),
            depth.view(),
            wgpu::Color::BLACK,
            &draws,
        );
        capture(&output, &mut encoder, &buffer);
        queue.submit([encoder.finish()]);
        let on = read(&device, &buffer)?;
        assert!(
            renderer
                .update_shadow_settings(
                    &queue,
                    ShadowSettings {
                        bias: f32::NAN,
                        ..settings
                    }
                )
                .is_err()
        );
        renderer.update_shadow_settings(
            &queue,
            ShadowSettings {
                enabled: false,
                ..settings
            },
        )?;
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode(
            &mut encoder,
            &output.create_view(&Default::default()),
            depth.view(),
            wgpu::Color::BLACK,
            &draws,
        );
        capture(&output, &mut encoder, &buffer);
        queue.submit([encoder.finish()]);
        let off = read(&device, &buffer)?;
        // Explicit default parameters must match the legacy shader defaults.
        transform.update_pbr_material(&queue, 0.35, 0.5)?;
        for (roughness, metallic) in [
            (f32::NAN, 0.5),
            (0.5, f32::INFINITY),
            (-0.1, 0.5),
            (0.5, 1.1),
        ] {
            assert!(
                transform
                    .update_pbr_material(&queue, roughness, metallic)
                    .is_err()
            );
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode(
            &mut encoder,
            &output.create_view(&Default::default()),
            depth.view(),
            wgpu::Color::BLACK,
            &draws,
        );
        capture(&output, &mut encoder, &buffer);
        queue.submit([encoder.finish()]);
        assert_eq!(
            read(&device, &buffer)?,
            off,
            "explicit PBR defaults and rejected updates changed output"
        );
        transform.update_pbr_material(&queue, 1.0, 0.0)?;
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode(
            &mut encoder,
            &output.create_view(&Default::default()),
            depth.view(),
            wgpu::Color::BLACK,
            &draws,
        );
        capture(&output, &mut encoder, &buffer);
        queue.submit([encoder.finish()]);
        let changed = read(&device, &buffer)?;
        assert_ne!(
            changed, off,
            "different PBR material did not affect lighting"
        );
        // Updating transforms/light must not silently reset the object's PBR state.
        transform.update_scene_material(&queue, world, [1.; 4], [0., 0., 2., 5.])?;
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode(
            &mut encoder,
            &output.create_view(&Default::default()),
            depth.view(),
            wgpu::Color::BLACK,
            &draws,
        );
        capture(&output, &mut encoder, &buffer);
        queue.submit([encoder.finish()]);
        assert_eq!(
            read(&device, &buffer)?,
            changed,
            "world/light update reset PBR material"
        );
        transform.update_pbr_material(&queue, 0.35, 0.5)?;
        for y in 0..8 {
            for x in 0..8 {
                for channel in 0..3 {
                    let offset = y * 256 + x * 8 + channel * 2;
                    let a = half::f16::from_bits(u16::from_le_bytes(
                        on[offset..offset + 2].try_into().unwrap(),
                    ))
                    .to_f32();
                    let b = half::f16::from_bits(u16::from_le_bytes(
                        off[offset..offset + 2].try_into().unwrap(),
                    ))
                    .to_f32();
                    assert!(
                        a.is_finite() && b.is_finite() && a >= 0. && b > 0.,
                        "pixel {x},{y} channel {channel}: on={a}, off={b}"
                    );
                    if x < 4 {
                        assert!((a - 0.01).abs() < 1e-5, "ambient receiver {x},{y}: {a}");
                        assert!(b > a + 0.01);
                    } else {
                        assert!(
                            (a - b).abs() < 1e-6,
                            "unoccluded receiver changed {x},{y}: {a} vs {b}"
                        );
                    }
                }
            }
        }
        for (filter, radius) in [
            (voxy_render::ShadowFilter::Pcf3x3, 1_i32),
            (voxy_render::ShadowFilter::Pcf5x5, 2),
        ] {
            renderer.update_shadow_settings(&queue, ShadowSettings { filter, ..settings })?;
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(
                &mut encoder,
                &output.create_view(&Default::default()),
                depth.view(),
                wgpu::Color::BLACK,
                &draws,
            );
            capture(&output, &mut encoder, &buffer);
            queue.submit([encoder.finish()]);
            let filtered = read(&device, &buffer)?;
            for y in 0..8_i32 {
                for x in 0..8_i32 {
                    let mut lit = 0;
                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            let tx = x + dx;
                            let ty = y + dy;
                            if !(0..8).contains(&tx) || !(0..8).contains(&ty) || tx >= 4 {
                                lit += 1;
                            }
                        }
                    }
                    let width = 2 * radius + 1;
                    #[allow(clippy::cast_precision_loss)]
                    let visibility = lit as f32 / (width * width) as f32;
                    for channel in 0..3 {
                        let offset =
                            usize::try_from(y)? * 256 + usize::try_from(x)? * 8 + channel * 2;
                        let value = |data: &[u8]| {
                            half::f16::from_bits(u16::from_le_bytes(
                                data[offset..offset + 2].try_into().unwrap(),
                            ))
                            .to_f32()
                        };
                        let expected = 0.01 + (value(&off) - 0.01) * visibility;
                        assert!(
                            (value(&filtered) - expected).abs()
                                < (expected.abs() * 0.0015).max(0.0001),
                            "PCF radius {radius} pixel {x},{y}: {} != {expected}",
                            value(&filtered)
                        );
                    }
                }
            }
        }
        println!(
            "SHADOW VISIBILITY PASS: 64 receiver pixels/192 On-Off RGB pairs/384 PCF RGB references, occluded ambient-only, unoccluded unchanged, On/Off, 3x3/5x5 PCF kernels including borders, invalid-setting rejection, PBR default parity, material response and invalid-value preservation"
        );
        Ok(())
    })
}
