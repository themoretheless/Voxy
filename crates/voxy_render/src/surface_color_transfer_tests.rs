use crate::{
    ComputeDispatch, GraphicsOptions, SceneMesh, SceneRenderer, SceneVertex, SurfaceColorBinding,
    SurfaceColorTransfer,
};
#[test]
fn inherited_colors_follow_current_sources_without_touching_geometry() {
    pollster::block_on(async {
        let adapter = GraphicsOptions::default()
            .create_instance()
            .request_adapter(&Default::default())
            .await
            .expect("GPU required");
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        eprintln!("INHERITED SURFACE COLOR {:?}", adapter.get_info());
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let authored: Vec<_> = (0..6)
            .map(|i| SceneVertex {
                position: [i as f32, 0.2, 0.3],
                uv: [0.4, 0.5],
                color: [
                    0.1 + i as f32 * 0.1,
                    0.2 + i as f32 * 0.05,
                    0.3,
                    0.5 + i as f32 * 0.05,
                ],
            })
            .collect();
        let mesh = SceneMesh::new(authored.clone(), vec![0, 1, 2])
            .unwrap()
            .with_normals(vec![[0., 0., 1.]; 6])
            .unwrap();
        let mut geometry = renderer
            .upload_mesh_with_usage(
                &device,
                &mesh,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            )
            .unwrap();
        let mapping = [
            SurfaceColorBinding {
                sources: [0, 1, 2],
                target: 4,
                weights: [0.25, 0.5, 0.25],
                padding: 0.,
            },
            SurfaceColorBinding {
                sources: [2, 0, 1],
                target: 3,
                weights: [0.125, 0.125, 0.75],
                padding: 0.,
            },
        ];
        let transfer = SurfaceColorTransfer::new(&device, &geometry, 3, &mapping)
            .await
            .unwrap();
        for invalid in [
            SurfaceColorBinding {
                target: 1,
                ..mapping[0]
            },
            SurfaceColorBinding {
                sources: [0, 1, 3],
                ..mapping[0]
            },
            SurfaceColorBinding {
                weights: [f32::NAN, 0., 1.],
                ..mapping[0]
            },
            SurfaceColorBinding {
                weights: [0.2, 0.2, 0.2],
                ..mapping[0]
            },
        ] {
            assert!(
                SurfaceColorTransfer::new(&device, &geometry, 3, &[invalid])
                    .await
                    .is_err()
            );
        }
        assert!(
            SurfaceColorTransfer::new(&device, &geometry, 3, &[mapping[0], mapping[0]])
                .await
                .is_err()
        );
        for sample in 0..3 {
            let mut vertices = authored.clone();
            for (i, v) in vertices[..3].iter_mut().enumerate() {
                for c in 0..4 {
                    v.color[c] += (sample * (i + c + 1)) as f32 * 0.01;
                }
            }
            let mesh = SceneMesh::new(vertices.clone(), vec![0, 1, 2])
                .unwrap()
                .with_normals(vec![[0., 0., 1.]; 6])
                .unwrap();
            geometry.update(&queue, &mesh).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            transfer.encode(&mut encoder);
            let dispatch = ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[
                    (geometry.deformation_vertices(), 0, 0, 216),
                    (geometry.deformation_normals(), 0, 216, 72),
                ],
                288,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            for (i, v) in vertices.iter().enumerate() {
                let actual: &[f32] = bytemuck::cast_slice(&bytes[i * 36..(i + 1) * 36]);
                assert_eq!(&actual[..3], &v.position);
                assert_eq!(&actual[3..5], &v.uv);
                assert_eq!(
                    &bytes[216 + i * 12..228 + i * 12],
                    bytemuck::cast_slice::<f32, u8>(&[0., 0., 1.])
                );
                if let Some(m) = mapping.iter().find(|m| m.target as usize == i) {
                    for c in 0..4 {
                        let expected = vertices[m.sources[0] as usize].color[c] * m.weights[0]
                            + vertices[m.sources[1] as usize].color[c] * m.weights[1]
                            + vertices[m.sources[2] as usize].color[c] * m.weights[2];
                        assert!((actual[5 + c] - expected).abs() < 2e-7);
                    }
                } else {
                    assert_eq!(&actual[5..], &v.color);
                }
            }
        }
    });
}
