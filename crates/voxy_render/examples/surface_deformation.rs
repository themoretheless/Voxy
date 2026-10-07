//! Actual prepared-body skin binding parity; not a full-game FPS benchmark.
#[path = "../../voxy_app/src/surface_normals.rs"]
mod surface_normals;
use voxy_render::{SceneRenderer, SurfaceDeformationWeight};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )?;
        let source_normals: Vec<_> = asset
            .normals
            .iter()
            .map(|n| {
                glam::Vec3::from_array(n.expect("prepared OBJ normal"))
                    .normalize()
                    .to_array()
            })
            .collect();
        let mesh = asset.mesh.with_normals(source_normals)?;
        let json: serde_json::Value = serde_json::from_str(include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined-skin.json"
        ))?;
        let nodes: Vec<[f64; 3]> = serde_json::from_value(json["positions"].clone())?;
        let triangles: Vec<[usize; 3]> = serde_json::from_value(json["triangles"].clone())?;
        let mut bindings = Vec::new();
        let mut rows = vec![Vec::new(); mesh.vertices().len()];
        for b in json["bindings"].as_array().unwrap() {
            let vertex = b["vertex"].as_u64().unwrap() as usize;
            let triangle = b["triangle"].as_u64().unwrap() as usize;
            let weights: [f64; 3] = serde_json::from_value(b["weights"].clone())?;
            bindings.push(physics::skin::SurfaceBinding {
                vertex,
                triangle,
                weights,
            });
            for (node, weight) in triangles[triangle].into_iter().zip(weights) {
                rows[vertex].push(SurfaceDeformationWeight {
                    control: node as u32,
                    weight: weight as f32,
                });
            }
        }
        let embedding = physics::skin::SkinEmbedding::new(
            mesh.vertices().len(),
            nodes.len(),
            triangles,
            bindings,
            true,
        )?;
        let mut offsets = vec![0];
        let mut weights = Vec::new();
        for row in rows {
            weights.extend(row);
            offsets.push(weights.len() as u32);
        }
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        assert!(
            renderer
                .prepare_surface_deformation(
                    &device,
                    &mesh,
                    &offsets[..offsets.len() - 1],
                    &weights,
                    nodes.len() as u32
                )
                .is_err()
        );
        let mut invalid_weights = weights.clone();
        invalid_weights[0].control = nodes.len() as u32;
        assert!(
            renderer
                .prepare_surface_deformation(
                    &device,
                    &mesh,
                    &offsets,
                    &invalid_weights,
                    nodes.len() as u32
                )
                .is_err()
        );
        let mut deformation = renderer.prepare_surface_deformation(
            &device,
            &mesh,
            &offsets,
            &weights,
            nodes.len() as u32,
        )?;
        let invalid_controls = vec![[f32::NAN, 0., 0., 0.]; nodes.len()];
        assert!(
            deformation
                .update_controls(&queue, &invalid_controls)
                .is_err()
        );
        let overflowing_controls = vec![[f32::MAX; 4]; nodes.len()];
        assert!(
            deformation
                .update_controls(&queue, &overflowing_controls)
                .is_err()
        );
        let mut uninitialized = device.create_command_encoder(&Default::default());
        assert!(deformation.encode(&mut uninitialized).is_err());
        let prepared_normals =
            surface_normals::PreparedNormals::new(mesh.vertices(), mesh.indices());
        let authored: Vec<glam::Vec3> = mesh
            .authored_normals()
            .expect("prepared body authored normals")
            .iter()
            .map(|n| glam::Vec3::from_array(*n))
            .collect();
        let size = mesh.vertices().len() as u64 * 36;
        let normal_size = mesh.vertices().len() as u64 * 12;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("body displacement parity"),
            size: size + normal_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let zero = vec![[0.; 3]; mesh.vertices().len()];
        let mut reports = Vec::new();
        for phase in [0., 0.25, 0.5, 0.] {
            let controls: Vec<[f32; 4]> = nodes
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let a = phase * (i as f64 * 0.113).sin();
                    [(a * 0.02) as f32, (a * 0.03) as f32, (-a * 0.01) as f32, 0.]
                })
                .collect();
            let posed: Vec<[f64; 3]> = nodes
                .iter()
                .zip(&controls)
                .map(|(p, d)| std::array::from_fn(|k| p[k] + f64::from(d[k])))
                .collect();
            let cpu = embedding.deform(&posed, &nodes, &zero)?;
            deformation.update_controls(&queue, &controls)?;
            let mut encoder = device.create_command_encoder(&Default::default());
            deformation.encode(&mut encoder)?;
            encoder.copy_buffer_to_buffer(deformation.vertex_buffer(), 0, &readback, 0, size);
            encoder.copy_buffer_to_buffer(
                deformation.normal_buffer(),
                0,
                &readback,
                size,
                normal_size,
            );
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            let data = readback.slice(..).get_mapped_range()?;
            let gpu: &[voxy_render::SceneVertex] = bytemuck::cast_slice(&data[..size as usize]);
            let gpu_normals: &[[f32; 3]] = bytemuck::cast_slice(&data[size as usize..]);
            let expected_normals = prepared_normals.transport(gpu, &authored);
            let normal_error = expected_normals
                .iter()
                .zip(gpu_normals)
                .map(|(a, b)| a.distance(glam::Vec3::from_array(*b)))
                .fold(0_f32, f32::max);
            assert!(
                normal_error.is_finite() && normal_error < 0.001,
                "GPU normal transport error {normal_error}"
            );
            let mut error = 0_f64;
            for ((rest, got), delta) in mesh.vertices().iter().zip(gpu).zip(cpu) {
                for k in 0..3 {
                    error = error.max(
                        (f64::from(got.position[k]) - (f64::from(rest.position[k]) + delta[k]))
                            .abs(),
                    );
                }
                assert_eq!(rest.uv, got.uv);
                assert_eq!(rest.color, got.color);
                if phase == 0. {
                    assert_eq!(rest.position, got.position);
                }
            }
            assert!(error < 1e-6, "GPU/CPU displacement error {error}");
            reports.push(serde_json::json!({"phase":phase,"maximum_position_error_m":error,"maximum_normal_vector_error":normal_error}));
            drop(data);
            readback.unmap();
        }
        let mut stage_times = Vec::new();
        for frame in 0..130 {
            let phase = (frame as f64 * 0.1).sin() * 0.5;
            let controls: Vec<[f32; 4]> = nodes
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let a = phase * (i as f64 * 0.113).sin();
                    [(a * 0.02) as f32, (a * 0.03) as f32, (-a * 0.01) as f32, 0.]
                })
                .collect();
            let start = std::time::Instant::now();
            deformation.update_controls(&queue, &controls)?;
            let mut encoder = device.create_command_encoder(&Default::default());
            deformation.encode(&mut encoder)?;
            queue.submit([encoder.finish()]);
            device.poll(wgpu::PollType::wait_indefinitely())?;
            if frame >= 10 {
                stage_times.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        stage_times.sort_by(f64::total_cmp);
        let stage_mean = stage_times.iter().sum::<f64>() / stage_times.len() as f64;
        let stage_p95 = stage_times[(stage_times.len() as f64 * 0.95).ceil() as usize - 1];
        let report = serde_json::json!({"adapter":format!("{:?}",adapter.get_info()),"vertices":mesh.vertices().len(),"controls":nodes.len(),"bindings":weights.len(),"reports":reports,"control_upload_bytes_per_frame":nodes.len()*16,"full_character_integrated":false,"normals_updated":true,"target_120fps_achieved":false,"deformation_and_normal_stage":{"samples":stage_times.len(),"mean_wall_ms":stage_mean,"p95_wall_ms":stage_p95,"max_wall_ms":stage_times.last(),"includes":"control upload, CPU encoding, submission, GPU work and blocking wait; no scene draw, lighting, simulation or presentation"},"scope":"Real prepared body and production SkinEmbedding CPU reference; synthetic finite displacements, not a full physics or frame-rate gate."});
        println!("SURFACE DEFORMATION GPU PASS {report}");
        if let Some(path) = std::env::args().nth(1) {
            std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
        }
        Ok(())
    })
}
