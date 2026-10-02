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
        println!("IBL GPU: {:?}", adapter.get_info());
        let environment = voxy_render::GgxEnvironmentPrefilter::new(&device, 8)?;
        let diffuse = voxy_render::DiffuseEnvironmentConvolution::new(&device, 8)?;
        let dfg = voxy_render::GgxDfgLut::with_samples(&device, 8, 256)?;
        let pixel: Vec<u8> = [4., 2., 8., 1.]
            .into_iter()
            .flat_map(|v| half::f16::from_f32(v).to_bits().to_le_bytes())
            .collect();
        for destination in [environment.input(), diffuse.input()] {
            for face in 0..6 {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: destination,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: face,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixel.repeat(64),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(64),
                        rows_per_image: Some(8),
                    },
                    wgpu::Extent3d {
                        width: 8,
                        height: 8,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba16Float);
        renderer
            .enable_environment_lighting(&device, &environment, &dfg)
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
            size: 2048,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        environment.encode(&mut encoder);
        dfg.encode(&mut encoder);
        diffuse.encode(&mut encoder);
        capture(dfg.output(), &mut encoder, &lut_buffer);
        queue.submit([encoder.finish()]);
        let lut = read(&device, &lut_buffer)?;
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
            .enable_environment_lighting(&environment, &dfg)
            .await?;
        let capture_revision = reflection.renderer().shader_revision();
        assert!(
            reflection
                .enable_environment_lighting(&foreign_environment, &dfg)
                .await
                .is_err()
        );
        assert!(
            reflection
                .enable_environment_lighting(&environment, &foreign_dfg)
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
                .enable_environment_lighting(&device, &environment, &dfg)
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
                    let sx = (nv * 8. - 0.5).clamp(0., 7.);
                    let ix = sx.floor() as usize;
                    let fx = sx - ix as f32;
                    let mut ab = [0.; 2];
                    for c in 0..2 {
                        ab[c] += (value(&lut, ix, 3, c) * (1. - fx)
                            + value(&lut, (ix + 1).min(7), 3, c) * fx)
                            * 0.5
                            + (value(&lut, ix, 4, c) * (1. - fx)
                                + value(&lut, (ix + 1).min(7), 4, c) * fx)
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
                (&foreign_device, &environment, &dfg),
                (&device, &foreign_environment, &dfg),
                (&device, &environment, &foreign_dfg),
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
            renderer
                .enable_full_environment_lighting(&device, &environment, &dfg, &diffuse)
                .await?;
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
            reflection
                .enable_full_environment_lighting(&environment, &dfg, &diffuse)
                .await?;
            let reflected_transform = reflection.renderer().create_transform(&device, world)?;
            reflected_transform.update_scene_material(&queue, world, tint, [0., 0., 2., 0.])?;
            reflected_transform.update_view_position(&queue, Vec3::new(0., 0., 2.))?;
            reflected_transform.update_pbr_material(&queue, 0.5, metallic)?;
            reflected_transform.update_pbr_capture_plane(&queue, Vec3::ZERO, Vec3::Z)?;
            for scale in [0.0, 1.0, 2.0] {
                reflection.set_environment_intensity(&queue, scale)?;
                reflection
                    .enable_full_environment_lighting(&environment, &dfg, &diffuse)
                    .await?;
                let mut encoder = device.create_command_encoder(&Default::default());
                reflection.encode(
                    &mut encoder,
                    wgpu::Color::BLACK,
                    &[SceneDraw {
                        geometry: &mesh,
                        texture: &texture,
                        transform: &reflected_transform,
                        overlay: false,
                    }],
                )?;
                capture(reflection.color(), &mut encoder, &buffer);
                queue.submit([encoder.finish()]);
                let reflected = read(&device, &buffer)?;
                for y in 0..8 {
                    for x in 0..8 {
                        for c in 0..3 {
                            let ambient = tint[c] * 0.02;
                            let expected = ambient + (value(&full, x, y, c) - ambient) * scale;
                            assert!(
                                (value(&reflected, x, y, c) - expected).abs() < 0.025,
                                "clipped IBL {scale}: {x}/{y}/{c}"
                            );
                        }
                    }
                }
            }
            reflection.set_environment_intensity(&queue, 1.0)?;
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
            for intensity in [0.0, 0.5, 2.0] {
                renderer.set_environment_intensity(&queue, intensity)?;
                for invalid in [-1.0, f32::NAN, f32::INFINITY] {
                    assert!(renderer.set_environment_intensity(&queue, invalid).is_err());
                }
                // Resource replacement preserves the chosen scale.
                renderer
                    .enable_full_environment_lighting(&device, &environment, &dfg, &diffuse)
                    .await?;
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
                let scaled = read(&device, &buffer)?;
                for y in 0..8 {
                    for x in 0..8 {
                        for c in 0..3 {
                            let ambient = tint[c] * 0.02;
                            let expected = ambient + (value(&full, x, y, c) - ambient) * intensity;
                            assert!(
                                (value(&scaled, x, y, c) - expected).abs() < 0.025,
                                "IBL intensity {intensity}: {x}/{y}/{c}"
                            );
                        }
                    }
                }
            }
            renderer.set_environment_intensity(&queue, 1.0)?;
            println!(
                "IBL PASS shadows={shadows} metallic={metallic} tint={tint:?}: 192 specular + 192 diffuse HDR references; foreign resources/invalid shader preserve output"
            );
        }
        Ok(())
    })
}
