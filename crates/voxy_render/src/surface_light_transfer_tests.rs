use crate::SurfaceLightAdmission;
use wgpu::util::DeviceExt;

use crate::surface_light_transfer::*;
use crate::{
    ComputeDispatch, GraphicsOptions, SceneMesh, SceneRenderer, SceneVertex, SurfaceLightGeometry,
};
#[test]
fn gpu_material_transfer_preserves_geometry_and_rejected_light() {
    pollster::block_on(async {
        let instance = GraphicsOptions::default().create_instance();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("GPU required");
        eprintln!("MATERIAL TRANSFER {:?}", adapter.get_info());
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let vertices: Vec<_> = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [2., 2., 2.]]
            .into_iter()
            .map(|position| SceneVertex {
                position,
                uv: [0.25, 0.75],
                color: [0.9, 0.8, 0.7, 0.6],
            })
            .collect();
        let mesh = SceneMesh::new(vertices.clone(), vec![0, 1, 2])
            .unwrap()
            .with_normals(vec![[0., 0., 1.]; 4])
            .unwrap();
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let geometry = renderer
            .upload_mesh_with_usage(
                &device,
                &mesh,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            )
            .unwrap();
        let operator =
            SurfaceLightGeometry::new(&device, geometry.deformation_vertices(), 4, 9, &[[0, 1, 2]])
                .unwrap();
        let source = [
            [1., 0.2, 0.3, 1.],
            [0.1, 0.5, 0.9, 1.],
            [0.4, 0.3, 0.8, 1.],
            [0.2, 0.6, 0.7, 1.],
        ];
        let input = |label| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&source),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
        };
        let candidate = input("candidate");
        let incident = input("incident");
        let mut diffuse = source;
        for row in &mut diffuse {
            for c in &mut row[..3] {
                *c += 0.0001;
            }
        }
        queue.write_buffer(&candidate, 0, bytemuck::cast_slice(&diffuse));
        let admission = SurfaceLightAdmission::new(
            &device, &operator, &candidate, &incident, [0.; 3], 1e-3, 1e-6,
        )
        .unwrap();
        let mapping = [
            SurfaceLightVertexBinding {
                albedo: [0.72, 0.46, 0.34, 0.8],
                vertex: 1,
                light: 2,
                surface_fraction: 0.35,
                padding: 0,
            },
            SurfaceLightVertexBinding {
                albedo: [0.3, 0.2, 0.1, 1.],
                vertex: 2,
                light: 0,
                surface_fraction: 0.7,
                padding: 0,
            },
        ];
        let transfer = SurfaceLightTransfer::new(&device, &geometry, &admission, &mapping)
            .await
            .unwrap();
        let mut duplicate = mapping;
        duplicate[1].vertex = 1;
        assert!(
            SurfaceLightTransfer::new(&device, &geometry, &admission, &duplicate)
                .await
                .is_err()
        );
        let mut bad = mapping;
        bad[0].light = 4;
        assert!(
            SurfaceLightTransfer::new(&device, &geometry, &admission, &bad)
                .await
                .is_err()
        );
        let mut previous = Vec::new();
        for stage in 0..3 {
            if stage == 2 {
                let mut invalid = source;
                invalid[2][0] = -1.;
                queue.write_buffer(&incident, 0, bytemuck::cast_slice(&invalid));
            }
            let mut encoder = device.create_command_encoder(&Default::default());
            if stage > 0 {
                operator.encode(&mut encoder);
                admission.encode(&mut encoder);
            }
            transfer.encode(&mut encoder);
            let dispatch = ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[
                    (geometry.deformation_vertices(), 0, 0, 144),
                    (geometry.deformation_normals(), 0, 144, 48),
                    (admission.header_buffer(), 0, 192, 32),
                ],
                224,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            let header: &[u32] = bytemuck::cast_slice(&bytes[192..]);
            assert_eq!(header[1], u32::from(stage == 1));
            assert_eq!(header[2], u32::from(stage > 0));
            if stage == 2 {
                assert_eq!(&bytes[..192], previous.as_slice());
            }
            for (i, original) in vertices.iter().enumerate() {
                let actual: &[f32] = bytemuck::cast_slice(&bytes[i * 36..(i + 1) * 36]);
                assert_eq!(&actual[..3], &original.position);
                assert_eq!(&actual[3..5], &original.uv);
                assert_eq!(
                    &bytes[144 + i * 12..156 + i * 12],
                    bytemuck::cast_slice::<f32, u8>(&[0., 0., 1.])
                );
                if stage == 0 || i == 0 || i == 3 {
                    assert_eq!(&actual[5..], &original.color);
                } else {
                    let m = mapping.iter().find(|m| m.vertex as usize == i).unwrap();
                    for c in 0..3 {
                        assert!(
                            (actual[5 + c]
                                - m.albedo[c]
                                    * (m.surface_fraction * source[m.light as usize][c]
                                        + (1.0 - m.surface_fraction)
                                            * diffuse[m.light as usize][c]))
                                .abs()
                                < 1e-6
                        );
                    }
                    assert_eq!(actual[8], m.albedo[3]);
                }
            }
            previous = bytes[..192].to_vec();
        }
    });
}

#[test]
fn resident_normal_lighting_updates_and_rejects_invalid_normals() {
    pollster::block_on(async {
        use crate::{SurfaceLightFrame, SurfaceLightFrameSettings};
        let instance = GraphicsOptions::default().create_instance();
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("GPU required");
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        eprintln!("RESIDENT LIGHT FRAME {:?}", adapter.get_info());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let authored = vec![
            SceneVertex {
                position: [0., 0., 0.],
                uv: [0.2, 0.3],
                color: [0.7, 0.6, 0.5, 1.],
            },
            SceneVertex {
                position: [1., 0., 0.],
                uv: [0.2, 0.3],
                color: [0.7, 0.6, 0.5, 1.],
            },
            SceneVertex {
                position: [0., 1., 0.],
                uv: [0.2, 0.3],
                color: [0.7, 0.6, 0.5, 1.],
            },
        ];
        let mesh = SceneMesh::new(authored.clone(), vec![0, 1, 2])
            .unwrap()
            .with_normals(vec![[0., 0., 1.]; 3])
            .unwrap();
        let mut geometry = renderer
            .upload_mesh_with_usage(
                &device,
                &mesh,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            )
            .unwrap();
        let settings = SurfaceLightFrameSettings {
            key_direction: [0., 0., 2.],
            key_weight: 0.6,
            fill_direction: [0., 3., 0.],
            fill_weight: 0.2,
            ambient: 0.2,
            albedo: [0.72, 0.46, 0.34, 1.],
            surface_fraction: 0.35,
            radii: [0.; 3],
        };
        let mut frame = SurfaceLightFrame::new(&device, &geometry, 3, &[[0, 1, 2]], settings)
            .await
            .unwrap();
        let mut invalid = settings;
        invalid.key_weight = f32::NAN;
        assert!(
            SurfaceLightFrame::new(&device, &geometry, 3, &[[0, 1, 2]], invalid)
                .await
                .is_err()
        );
        let mut last_colors = Vec::new();
        let mut generation = 0;
        for (stage, normal) in [[0., 0., 2.], [0., 3., 0.], [0., 0., 0.], [1., 0., 0.]]
            .into_iter()
            .enumerate()
        {
            let mut posed = authored.clone();
            posed[1].position[2] = stage as f32 * 0.1;
            let mesh = SceneMesh::new(posed.clone(), vec![0, 1, 2])
                .unwrap()
                .with_normals(vec![normal; 3])
                .unwrap();
            geometry.update(&queue, &mesh).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            assert!(frame.encode(&mut encoder, 0).is_err());
            frame.encode(&mut encoder, 1).unwrap();
            let dispatch = ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[
                    (geometry.deformation_vertices(), 0, 0, 108),
                    (frame.incident_buffer(), 0, 108, 48),
                    (frame.admission().header_buffer(), 0, 156, 32),
                ],
                188,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            let header: &[u32] = bytemuck::cast_slice(&bytes[156..]);
            let valid = stage != 2;
            if valid {
                generation += 1;
            }
            assert_eq!(&header[1..4], &[u32::from(valid), 1, generation]);
            let incident: &[[f32; 4]] = bytemuck::cast_slice(&bytes[108..156]);
            let unit = glam::Vec3::from(normal).normalize_or_zero();
            let light = 0.2 + 0.6 * unit.z.max(0.) + 0.2 * unit.y.max(0.);
            let mut colors = Vec::new();
            for (i, v) in posed.iter().enumerate() {
                let actual: &[f32] = bytemuck::cast_slice(&bytes[i * 36..(i + 1) * 36]);
                assert_eq!(&actual[..3], &v.position);
                assert_eq!(&actual[3..5], &v.uv);
                colors.extend_from_slice(&bytes[i * 36 + 20..(i + 1) * 36]);
                if valid {
                    for c in 0..3 {
                        assert!((incident[i][c] - light).abs() < 1e-6);
                        assert!((actual[5 + c] - settings.albedo[c] * light).abs() < 1e-6);
                    }
                }
            }
            if !valid {
                assert_eq!(
                    colors, last_colors,
                    "rejection must restore accepted colors after mesh upload"
                );
            }
            last_colors = colors;
        }
    });
}
