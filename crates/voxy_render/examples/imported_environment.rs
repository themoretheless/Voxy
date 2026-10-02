//! Specular IBL scene integration acceptance.
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
        println!("IMPORTED HDR GPU: {:?}", adapter.get_info());
        let mut source = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 4 +X 8\n".to_vec();
        source.extend_from_slice(&[64_u8, 32, 128, 132].repeat(32));
        let panorama =
            voxy_render::HdrImageAsset::decode(&source, voxy_render::ImageLimits::default())?;
        let imported = voxy_render::ImportedEnvironment::from_panorama(
            &device,
            &queue,
            &panorama,
            8,
            voxy_render::ImageLimits::default(),
        )?;
        let overrange = voxy_render::HdrImageAsset::from_rgb(
            1,
            1,
            vec![70000., 0., 0.],
            voxy_render::ImageLimits::default(),
        )?;
        assert!(matches!(
            voxy_render::ImportedEnvironment::from_panorama(
                &device,
                &queue,
                &overrange,
                1,
                voxy_render::ImageLimits::default()
            ),
            Err(voxy_render::ImportedEnvironmentError::HalfFloatRange)
        ));
        assert!(
            voxy_render::ImportedEnvironment::from_panorama(
                &device,
                &queue,
                &panorama,
                8,
                voxy_render::ImageLimits {
                    pixel_bytes: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let environment = imported.specular();
        let dfg = imported.dfg();
        let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba16Float);
        renderer
            .enable_environment_lighting(&device, environment, dfg)
            .await?;
        let mesh = renderer.upload_mesh(&device, &SceneMesh::quad([1.; 4]))?;
        let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
        let world = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.),
            Quat::IDENTITY,
            Vec3::new(0., 0., 0.5),
        );
        let transform = renderer.create_transform(&device, world)?;
        transform.update_scene_material(&queue, world, [1.; 4], [0., 0., 2., 0.])?;
        transform.update_view_position(&queue, Vec3::new(0., 0., 2.))?;
        transform.update_pbr_material(&queue, 0.5, 1.)?;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
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
        let shadow_map = ShadowMap::new(&device, 8, 8)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2048,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let lut_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 32768,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        imported.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            dfg.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &lut_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(512),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let lut = read(&device, &lut_buffer)?;
        let lut_value = |bytes: &[u8], x: usize, y: usize, c: usize| {
            half::f16::from_bits(u16::from_le_bytes(
                bytes[y * 512 + x * 8 + c * 2..y * 512 + x * 8 + c * 2 + 2]
                    .try_into()
                    .unwrap(),
            ))
            .to_f32()
        };
        let value = |bytes: &[u8], x: usize, y: usize, c: usize| {
            half::f16::from_bits(u16::from_le_bytes(
                bytes[y * 256 + x * 8 + c * 2..y * 256 + x * 8 + c * 2 + 2]
                    .try_into()
                    .unwrap(),
            ))
            .to_f32()
        };
        let (foreign_device, _) = adapter.request_device(&Default::default()).await?;
        let foreign_environment = voxy_render::GgxEnvironmentPrefilter::new(&foreign_device, 1)?;
        let foreign_dfg = voxy_render::GgxDfgLut::new(&foreign_device, 1)?;
        let mut reflection = voxy_render::PlanarReflectionCapture::new(&device, 8, 8)?;
        reflection
            .enable_environment_lighting(environment, dfg)
            .await?;
        let capture_revision = reflection.renderer().shader_revision();
        assert!(
            reflection
                .enable_environment_lighting(&foreign_environment, dfg)
                .await
                .is_err()
        );
        assert!(
            reflection
                .enable_environment_lighting(environment, &foreign_dfg)
                .await
                .is_err()
        );
        assert_eq!(reflection.renderer().shader_revision(), capture_revision);
        let mut encoder = device.create_command_encoder(&Default::default());
        reflection.encode(&mut encoder, wgpu::Color::BLACK, &[])?;
        queue.submit([encoder.finish()]);
        for (shadows, metallic, tint) in [
            (false, 1., [1.; 4]),
            (true, 1., [1.; 4]),
            (true, 1., [0.8, 0.2, 0.1, 1.]),
            (true, 0., [0.8, 0.2, 0.1, 1.]),
        ] {
            renderer
                .enable_environment_lighting(&device, environment, dfg)
                .await?;
            transform.update_scene_material(&queue, world, tint, [0., 0., 2., 0.])?;
            transform.update_pbr_material(&queue, 0.5, metallic)?;
            if shadows {
                renderer
                    .enable_shadowed_point_light(
                        &device,
                        &shadow_map,
                        ShadowSettings {
                            light_from_world: Mat4::IDENTITY,
                            bias: 0.,
                            enabled: false,
                            filter: voxy_render::ShadowFilter::Hard,
                        },
                    )
                    .await?;
            }
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(
                &mut encoder,
                &output.create_view(&Default::default()),
                depth.view(),
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &mesh,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }],
            );
            capture(&output, &mut encoder, &buffer);
            queue.submit([encoder.finish()]);
            let pixels = read(&device, &buffer)?;
            for y in 0..8 {
                for x in 0..8 {
                    let wx = (x as f32 + 0.5) / 4. - 1.;
                    let wy = 1. - (y as f32 + 0.5) / 4.;
                    let nv = 1.5 / (wx * wx + wy * wy + 2.25).sqrt();
                    let sx = (nv * 64. - 0.5).clamp(0., 63.);
                    let ix = sx.floor() as usize;
                    let fx = sx - ix as f32;
                    let mut ab = [0.; 2];
                    for c in 0..2 {
                        ab[c] += (lut_value(&lut, ix, 31, c) * (1. - fx)
                            + lut_value(&lut, (ix + 1).min(63), 31, c) * fx)
                            * 0.5
                            + (lut_value(&lut, ix, 32, c) * (1. - fx)
                                + lut_value(&lut, (ix + 1).min(63), 32, c) * fx)
                                * 0.5;
                    }

                    for (c, radiance) in [4., 2., 8.].into_iter().enumerate() {
                        let f0 = 0.04 * (1. - metallic) + tint[c] * metallic;
                        let expected = radiance * (f0 * ab[0] + ab[1]) + tint[c] * 0.02;
                        let actual = value(&pixels, x, y, c);
                        assert!(
                            (actual - expected).abs() < 0.008,
                            "({x},{y}) {c}: {actual} vs {expected}"
                        );
                    }
                }
            }
            let revision = renderer.shader_revision();
            for (supplied, cube, table) in [
                (&foreign_device, environment, dfg),
                (&device, &foreign_environment, dfg),
                (&device, environment, &foreign_dfg),
            ] {
                assert!(
                    renderer
                        .enable_environment_lighting(supplied, cube, table)
                        .await
                        .is_err()
                );
                assert_eq!(renderer.shader_revision(), revision);
            }
            assert!(
                renderer
                    .reload_shader(&device, "invalid shader")
                    .await
                    .is_err()
            );
            assert_eq!(renderer.shader_revision(), revision);
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(
                &mut encoder,
                &output.create_view(&Default::default()),
                depth.view(),
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &mesh,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }],
            );
            capture(&output, &mut encoder, &buffer);
            queue.submit([encoder.finish()]);
            let after_rejection = read(&device, &buffer)?;
            assert_eq!(
                after_rejection, pixels,
                "rejected resource or shader changed rendered IBL"
            );
            imported.attach(&mut renderer).await?;
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(
                &mut encoder,
                &output.create_view(&Default::default()),
                depth.view(),
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &mesh,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }],
            );
            capture(&output, &mut encoder, &buffer);
            queue.submit([encoder.finish()]);
            let full = read(&device, &buffer)?;
            for y in 0..8 {
                for x in 0..8 {
                    let wx = (x as f32 + 0.5) / 4. - 1.;
                    let wy = 1. - (y as f32 + 0.5) / 4.;
                    let nv = 1.5 / (wx * wx + wy * wy + 2.25).sqrt();
                    for (c, radiance) in [4., 2., 8.].into_iter().enumerate() {
                        let f0 = 0.04 * (1. - metallic) + tint[c] * metallic;
                        let fresnel = f0 + (1. - f0) * (1. - nv).powi(5);
                        let expected = value(&pixels, x, y, c)
                            + radiance * tint[c] * (1. - metallic) * (1. - fresnel);
                        assert!(
                            (value(&full, x, y, c) - expected).abs() < 0.012,
                            "diffuse {x}/{y}/{c}"
                        );
                    }
                }
            }
            println!(
                "IMPORTED HDR PASS shadows={shadows} metallic={metallic} tint={tint:?}: 192 specular + 192 diffuse HDR references; foreign resources/invalid shader preserve output"
            );
        }
        Ok(())
    })
}
