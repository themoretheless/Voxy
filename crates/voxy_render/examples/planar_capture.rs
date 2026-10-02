//! Numeric planar capture proof; no ray-query feature is required.
use glam::{Mat4, Vec3};
use voxy_render::{PlanarReflectionCapture, SceneCamera, SceneDraw, SceneMesh, SceneProjection};
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
        println!("PLANAR CAPTURE GPU: {:?}", adapter.get_info());
        let mut capture = PlanarReflectionCapture::new(&device, 8, 8)?;
        let original = capture.color().clone();
        assert!(!capture.resize(8, 8)?);
        assert_eq!(capture.color(), &original);
        assert!(capture.resize(0, 8).is_err());
        assert_eq!(capture.color(), &original);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (offset, color) in [(-0.25, [4., 0., 0., 1.]), (0.25, [0., 2., 0., 1.])] {
            let mesh = SceneMesh::quad(color);
            let base = u32::try_from(vertices.len())?;
            vertices.extend(mesh.vertices().iter().map(|vertex| {
                let mut vertex = *vertex;
                vertex.position[0] = vertex.position[0] * 0.5 + offset;
                vertex
            }));
            indices.extend(mesh.indices().iter().map(|index| index + base));
        }
        let geometry = capture
            .renderer()
            .upload_mesh(&device, &SceneMesh::new(vertices, indices)?)?;
        let texture = capture
            .renderer()
            .upload_texture(&device, &queue, 1, 1, &[255; 4])?;
        let camera = SceneCamera {
            eye: Vec3::new(0., 0., 3.),
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: SceneProjection::Orthographic {
                left: -1.,
                right: 1.,
                bottom: -1.,
                top: 1.,
                near: 0.1,
                far: 10.,
            },
        };
        let mut surface = PlanarReflectionCapture::new(&device, 8, 8)?;
        surface
            .reload_shader(voxy_render::PLANAR_REFLECTION_SURFACE_SHADER)
            .await?;
        let surface_mesh = SceneMesh::quad([1.; 4]);
        let surface_vertices = surface_mesh
            .vertices()
            .iter()
            .map(|vertex| {
                let mut vertex = *vertex;
                vertex.uv = [0., 0.]; // Projective sampling must ignore authored UVs.
                vertex
            })
            .collect();
        let surface_geometry = surface.renderer().upload_mesh(
            &device,
            &SceneMesh::new(surface_vertices, surface_mesh.indices().to_vec())?,
        )?;
        let surface_transform = surface
            .renderer()
            .create_transform(&device, Mat4::from_scale(Vec3::splat(2.)))?;
        let captured_material = capture
            .mip_material_binding(surface.renderer(), voxy_render::TextureSampling::default())?;
        assert_eq!(capture.color().mip_level_count(), 4);
        let filtered_material = capture.mip_material_binding(
            surface.renderer(),
            voxy_render::TextureSampling {
                min_filter: voxy_render::TextureFilter::Linear,
                mag_filter: voxy_render::TextureFilter::Linear,
                mipmap_filter: Some(voxy_render::TextureFilter::Linear),
                ..voxy_render::TextureSampling::default()
            },
        )?;
        let mut pbr_reference = Vec::new();
        let perspective = SceneCamera {
            projection: SceneProjection::Perspective {
                vertical_fov: 2.0 * (1.0_f32 / 3.0).atan(),
                aspect: 1.0,
                near: 0.1,
                far: 10.0,
            },
            ..camera
        };
        // Oblique view gives different capture W across the planar surface.
        let oblique = SceneCamera {
            eye: Vec3::new(0.1, 0.0, 3.0),
            ..perspective
        };
        for (camera, mirrored, clipped, pbr, fresnel) in [
            (camera, false, false, false, 0),
            (
                camera.reflected(Vec3::ZERO, Vec3::Z)?,
                true,
                false,
                false,
                0,
            ),
            (perspective, false, false, false, 0),
            (
                perspective.reflected(Vec3::ZERO, Vec3::Z)?,
                true,
                false,
                false,
                0,
            ),
            (
                perspective.reflected(Vec3::ZERO, Vec3::Z)?,
                true,
                true,
                false,
                0,
            ),
            (oblique, false, false, false, 0),
            (
                oblique.reflected(Vec3::ZERO, Vec3::Z)?,
                true,
                false,
                false,
                0,
            ),
            (
                oblique.reflected(Vec3::ZERO, Vec3::Z)?,
                true,
                true,
                false,
                0,
            ),
            (camera.reflected(Vec3::ZERO, Vec3::Z)?, true, true, false, 0),
            (camera.reflected(Vec3::ZERO, Vec3::Z)?, true, false, true, 0),
            (camera.reflected(Vec3::ZERO, Vec3::Z)?, true, true, true, 0),
            (camera, false, false, false, 1),
            (camera, false, false, false, 2),
            (camera, false, false, false, 3),
            (camera, false, false, false, 4),
            (camera, false, false, false, 5),
            (camera, false, false, false, 6),
            (camera, false, false, false, 7),
            (camera, false, false, false, 8),
            (camera, false, false, false, 9),
            (camera, false, false, false, 10),
            (camera, false, false, false, 11),
            (camera, false, false, false, 12),
            (camera, false, false, false, 13),
            (camera, false, false, false, 14),
            (camera, false, false, false, 15),
            (camera, false, false, false, 16),
            (camera, false, false, false, 17),
            (camera, false, false, false, 18),
        ] {
            if pbr {
                let shader = if clipped {
                    voxy_render::planar_reflection_pbr_clip_shader()
                } else {
                    voxy_render::TEXTURED_POINT_LIGHT_SHADER.to_owned()
                };
                capture.reload_shader(&shader).await?;
            } else if clipped {
                capture
                    .reload_shader(voxy_render::PLANAR_REFLECTION_CLIP_SHADER)
                    .await?;
            } else {
                capture
                    .reload_shader(voxy_render::DEFAULT_SCENE_SHADER)
                    .await?;
            }
            let surface_shader = if fresnel == 4 {
                voxy_render::PLANAR_REFLECTION_SURFACE_SHADER.replace("uv, 0.0", "uv, 3.0")
            } else if fresnel == 0 {
                voxy_render::PLANAR_REFLECTION_SURFACE_SHADER.to_owned()
            } else if fresnel >= 5 {
                voxy_render::PLANAR_REFLECTION_ROUGH_SHADER.to_owned()
            } else {
                voxy_render::PLANAR_REFLECTION_FRESNEL_SHADER.to_owned()
            };
            surface.reload_shader(&surface_shader).await?;
            let surface_eye = if fresnel == 2 {
                Vec3::new(3.0, 0.0, 0.1)
            } else {
                Vec3::new(0.0, 0.0, 3.0)
            };
            if (1..=3).contains(&fresnel) || fresnel >= 5 {
                surface_transform.update_scene_material(
                    &queue,
                    Mat4::from_scale(Vec3::splat(2.)),
                    [0.65, 0.65, 0.65, 1.0],
                    [0.; 4],
                )?;
                surface_transform.update_view_position(&queue, surface_eye)?;
                surface_transform.update_pbr_material(
                    &queue,
                    match fresnel {
                        5 | 16 => 0.0,
                        6 | 17 => 1.0,
                        7 | 18 => 0.5_f32.sqrt(),
                        _ => 0.35,
                    },
                    match fresnel {
                        3 | 16 | 17 => 1.0,
                        18 => 0.5,
                        _ => 0.0,
                    },
                )?;
            }
            let capture_projection = match fresnel {
                8 => Mat4::from_diagonal(glam::Vec4::new(1.0, 1.0, 1.0, -1.0)),
                9 => Mat4::from_translation(Vec3::new(4.0, 0.0, 0.0)),
                10 => Mat4::from_translation(Vec3::new(0.0, 0.0, -1.0)),
                11 => Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                12 => Mat4::from_diagonal(glam::Vec4::new(1.0, 1.0, 1.0, 0.0)),
                13 => Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
                14 => Mat4::from_translation(Vec3::new(0.0, 4.0, 0.0)),
                15 => Mat4::from_translation(Vec3::new(0.0, -4.0, 0.0)),
                _ => camera.view_projection()?,
            };
            surface_transform.update_planar_projection(
                &queue,
                Mat4::from_scale(Vec3::splat(2.)),
                capture_projection * Mat4::from_scale(Vec3::splat(2.)),
            )?;
            assert!(
                surface_transform
                    .update_planar_projection(
                        &queue,
                        Mat4::from_cols_array(&[f32::NAN; 16]),
                        Mat4::IDENTITY
                    )
                    .is_err()
            );
            let transform = capture.renderer().create_transform(
                &device,
                camera.view_projection()? * Mat4::from_scale(Vec3::splat(2.)),
            )?;
            if pbr {
                transform.update_scene_material(
                    &queue,
                    Mat4::from_scale(Vec3::splat(2.)),
                    [1.; 4],
                    [0., 0., 3., 35.],
                )?;
                transform.update_view_position(&queue, camera.eye)?;
                transform.update_pbr_material(&queue, 0.6, 0.2)?;
                if clipped {
                    transform.update_pbr_capture_plane(&queue, Vec3::new(0.25, 0., 0.), Vec3::X)?;
                    assert!(
                        transform
                            .update_pbr_capture_plane(&queue, Vec3::ZERO, Vec3::ZERO)
                            .is_err()
                    );
                }
            } else if clipped {
                transform.update_planar_clip(
                    &queue,
                    Mat4::from_scale(Vec3::splat(2.)),
                    [1.; 4],
                    Vec3::new(0.25, 0., 0.),
                    Vec3::X,
                )?;
                assert!(
                    transform
                        .update_planar_clip(&queue, Mat4::IDENTITY, [1.; 4], Vec3::ZERO, Vec3::ZERO)
                        .is_err()
                );
            }
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            capture.encode(
                &mut encoder,
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &geometry,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }],
            )?;
            surface.encode(
                &mut encoder,
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &surface_geometry,
                    texture: if fresnel >= 5 {
                        &filtered_material
                    } else {
                        &captured_material
                    },
                    transform: &surface_transform,
                    overlay: false,
                }],
            )?;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("planar readback"),
                size: 2048,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                surface.color().as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
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
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
            device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            let bytes = buffer.slice(..).get_mapped_range()?;
            for y in 0..8 {
                for x in 0..8 {
                    for channel in 0..3 {
                        let offset = y * 256 + x * 8 + channel * 2;
                        let value = half::f16::from_bits(u16::from_le_bytes(
                            bytes[offset..offset + 2].try_into()?,
                        ))
                        .to_f32();
                        if (8..=15).contains(&fresnel) {
                            assert_eq!(
                                value, 0.0,
                                "capture frustum case {fresnel} pixel {x},{y}/{channel}"
                            );
                            continue;
                        }
                        if pbr {
                            assert!(value.is_finite() && value >= 0.);
                            if !clipped {
                                pbr_reference.push(value);
                            } else {
                                let expected = if x < 5 {
                                    0.
                                } else {
                                    pbr_reference[(y * 8 + x) * 3 + channel]
                                };
                                assert!(
                                    (value - expected).abs() < 1e-6,
                                    "PBR clip pixel {x},{y}/{channel}: {value} != {expected}"
                                );
                            }
                            continue;
                        }
                        // Projective coordinates compensate the mirrored capture's X parity.
                        let red = x < 4;
                        let mut expected = if red && channel == 0 && !clipped {
                            4.0
                        } else if !red && channel == 1 && (!clipped || x >= 5) {
                            2.0
                        } else {
                            0.0
                        };
                        if fresnel == 4 {
                            expected = [2.0, 1.0, 0.0][channel];
                        } else if fresnel != 0 {
                            if fresnel == 6 || fresnel == 17 {
                                expected = [2.0, 1.0, 0.0][channel];
                            } else if fresnel == 7 || fresnel == 18 {
                                let u = (f32::from(u16::try_from(x)?) + 0.5) / 8.0;
                                let red_weight = |size: f32| {
                                    let p = u * size - 0.5;
                                    let i = p.floor();
                                    let a = if i < size * 0.5 { 1.0 } else { 0.0 };
                                    let b = if i + 1.0 < size * 0.5 { 1.0 } else { 0.0 };
                                    a + (b - a) * (p - i)
                                };
                                let weight = 0.5 * (red_weight(4.0) + red_weight(2.0));
                                expected = match channel {
                                    0 => 4.0 * weight,
                                    1 => 2.0 * (1.0 - weight),
                                    _ => 0.0,
                                };
                            }
                            let x = f32::from(u16::try_from(x)?) * 0.25 - 0.875;
                            let y = 0.875 - f32::from(u16::try_from(y)?) * 0.25;
                            let view = (surface_eye - Vec3::new(x, y, 0.)).normalize();
                            let nv = view.z.clamp(0., 1.);
                            let f0 = match fresnel {
                                3 | 16 | 17 => 0.65,
                                18 => 0.345,
                                _ => 0.04,
                            };
                            expected *= f0 + (1.0 - f0) * (1.0 - nv).powi(5);
                            assert!(
                                (value - expected).abs() <= expected.abs().max(0.01) * 0.002,
                                "Fresnel {fresnel}: {value} != {expected}"
                            );
                            continue;
                        }
                        assert!(
                            value.is_finite()
                                && value >= 0.0
                                && half::f16::from_f32(value)
                                    .to_bits()
                                    .abs_diff(half::f16::from_f32(expected).to_bits())
                                    <= 1,
                            "mirror={mirrored} pixel {x},{y} channel {channel}: {value} != {expected}"
                        );
                    }
                }
            }
            drop(bytes);
            buffer.unmap();
        }
        assert!(pbr_reference.iter().any(|value| *value > 0.1));
        let (foreign, _) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let foreign_capture = PlanarReflectionCapture::new(&foreign, 8, 8)?;
        assert!(matches!(
            capture.material_binding(
                foreign_capture.renderer(),
                voxy_render::TextureSampling::default()
            ),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        let foreign_geometry = foreign_capture
            .renderer()
            .upload_mesh(&foreign, &SceneMesh::quad([1.; 4]))?;
        // A second device uses its own queue even when it shares the physical adapter.
        let (foreign_texture_device, foreign_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let foreign_texture_capture = PlanarReflectionCapture::new(&foreign_texture_device, 8, 8)?;
        let foreign_texture = foreign_texture_capture.renderer().upload_texture(
            &foreign_texture_device,
            &foreign_queue,
            1,
            1,
            &[255; 4],
        )?;
        let transform = capture
            .renderer()
            .create_transform(&device, Mat4::IDENTITY)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for draw in [
            SceneDraw {
                geometry: &foreign_geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &geometry,
                texture: &foreign_texture,
                transform: &transform,
                overlay: false,
            },
        ] {
            assert!(matches!(
                capture.encode(&mut encoder, wgpu::Color::BLACK, &[draw]),
                Err(voxy_render::SceneError::DeviceMismatch)
            ));
        }
        assert!(matches!(
            capture.encode(
                &mut encoder,
                wgpu::Color::BLACK,
                &[SceneDraw {
                    geometry: &geometry,
                    texture: &captured_material,
                    transform: &transform,
                    overlay: false,
                }]
            ),
            Err(voxy_render::SceneError::InvalidTexture)
        ));
        queue.submit([encoder.finish()]);
        assert!(capture.resize(4, 4)?);
        assert_ne!(capture.color(), &original);
        assert_eq!(capture.color().size().width, 4);
        assert_eq!(captured_material.texture(), &original);
        let resized_material = capture
            .material_binding(surface.renderer(), voxy_render::TextureSampling::default())?;
        assert_eq!(resized_material.texture(), capture.color());
        println!(
            "PLANAR CAPTURE PASS: 1728 HDR + 192 PBR + 576 Fresnel + 192 last-mip + 1152 roughness/material + 1536 frustum rejection references; projection, clipping, retention and ownership"
        );
        Ok(())
    })
}
