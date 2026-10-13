//! Explicit hardware audit: reuse one GPU operator across accepted physical poses.
use voxy_render::{SurfaceLightEdge, SurfaceLightGeometry, SurfaceLightPcg};
use wgpu::util::DeviceExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: surface_light_geometry REPORT INPUT...".into());
    }
    let inputs: Vec<serde_json::Value> = args[1..]
        .iter()
        .map(|p| Ok::<_, Box<dyn std::error::Error>>(serde_json::from_slice(&std::fs::read(p)?)?))
        .collect::<Result<_, _>>()?;
    let points: Vec<[f32; 3]> = serde_json::from_value(inputs[0]["points"].clone())?;
    let triangles: Vec<[u32; 3]> = serde_json::from_value(inputs[0]["triangles"].clone())?;
    let radii: [f64; 3] = serde_json::from_value(inputs[0]["radii"].clone())?;
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance.request_adapter(&Default::default()).await?;
    let features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let packed = |p: &[[f32; 3]]| {
        p.iter()
            .map(|p| [p[0], p[1], p[2], 0., 0., 0., 0., 0., 0.])
            .collect::<Vec<_>>()
    };
    let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("resident interleaved body positions"),
        contents: bytemuck::cast_slice(&packed(&points)),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    assert!(SurfaceLightGeometry::new(&device, &positions, usize::MAX, 9, &triangles).is_err());
    assert!(SurfaceLightGeometry::new(&device, &positions, points.len(), 2, &triangles).is_err());
    assert!(SurfaceLightGeometry::new(&device, &positions, points.len(), 9, &[[0, 0, 1]]).is_err());
    assert!(
        SurfaceLightGeometry::new(
            &device,
            &positions,
            points.len(),
            9,
            &[[0, 1, points.len() as u32]]
        )
        .is_err()
    );
    let geometry = SurfaceLightGeometry::new(&device, &positions, points.len(), 9, &triangles)?;
    let mut solver = SurfaceLightPcg::from_geometry(&device, &geometry, radii.map(|x| x as f32))?;
    let queries = (!features.is_empty()).then(|| {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("resident assembly and solve times"),
            ty: wgpu::QueryType::Timestamp,
            count: 4,
        })
    });
    let timestamps = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("assembly timestamps"),
        size: 32,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let n = points.len();
    let light_bytes = n as u64 * 16;
    let mass_bytes = n as u64 * 4;
    let edge_bytes = geometry.edge_buffer().size();
    let mass_start = light_bytes;
    let edge_start = mass_start + mass_bytes;
    let status_start = edge_start + edge_bytes;
    let time_start = (status_start + 4).next_multiple_of(8);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("operator and light audit"),
        size: time_start + 32,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut reports = Vec::new();
    let mut last_points = points;
    for (sample, input) in inputs.iter().enumerate() {
        let points: Vec<[f32; 3]> = serde_json::from_value(input["points"].clone())?;
        let current_triangles: Vec<[u32; 3]> = serde_json::from_value(input["triangles"].clone())?;
        assert_eq!(current_triangles, triangles);
        assert_eq!(points.len(), n);
        let current_radii: [f64; 3] = serde_json::from_value(input["radii"].clone())?;
        assert_eq!(current_radii, radii);
        let mass: Vec<f64> = serde_json::from_value(input["mass"].clone())?;
        let edges: Vec<(usize, usize, f64)> = serde_json::from_value(input["edges"].clone())?;
        let source: Vec<[f64; 3]> = serde_json::from_value(input["source"].clone())?;
        let reference: Vec<[f64; 3]> = serde_json::from_value(input["reference"].clone())?;
        assert_eq!(mass.len(), n);
        assert_eq!(source.len(), n);
        assert_eq!(reference.len(), n);
        let mut expected_rows = vec![std::collections::BTreeMap::new(); n];
        for &(a, b, w) in &edges {
            expected_rows[a].insert(b, w);
            expected_rows[b].insert(a, w);
        }
        let mut expected_offsets = vec![0usize];
        for row in &expected_rows {
            expected_offsets.push(expected_offsets.last().unwrap() + row.len());
        }
        let expected_edges: Vec<_> = expected_rows
            .into_iter()
            .flat_map(|row| row.into_iter())
            .collect();
        assert_eq!(expected_edges.len() as u64 * 8, edge_bytes);
        queue.write_buffer(&positions, 0, bytemuck::cast_slice(&packed(&points)));
        let source_gpu: Vec<_> = source
            .iter()
            .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32, 1.])
            .collect();
        solver.update_source(&queue, &source_gpu)?;
        let start = std::time::Instant::now();
        let mut encoder = device.create_command_encoder(&Default::default());
        geometry.encode_timed(&mut encoder, queries.as_ref().map(|q| (q, 0, 1)));
        solver.encode_timed(&mut encoder, 40, queries.as_ref().map(|q| (q, 2, 3)))?;
        encoder.copy_buffer_to_buffer(solver.output(), 0, &readback, 0, light_bytes);
        encoder.copy_buffer_to_buffer(geometry.mass_buffer(), 0, &readback, mass_start, mass_bytes);
        encoder.copy_buffer_to_buffer(geometry.edge_buffer(), 0, &readback, edge_start, edge_bytes);
        encoder.copy_buffer_to_buffer(geometry.status_buffer(), 0, &readback, status_start, 4);
        if let Some(q) = &queries {
            encoder.resolve_query_set(q, 0..4, &timestamps, 0);
            encoder.copy_buffer_to_buffer(&timestamps, 0, &readback, time_start, 32);
        }
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let data = readback.slice(..).get_mapped_range()?;
        let values: &[[f32; 4]] = bytemuck::cast_slice(&data[..light_bytes as usize]);
        let gpu_mass: &[f32] =
            bytemuck::cast_slice(&data[mass_start as usize..edge_start as usize]);
        let gpu_edges: &[SurfaceLightEdge] =
            bytemuck::cast_slice(&data[edge_start as usize..status_start as usize]);
        let status =
            u32::from_le_bytes(data[status_start as usize..status_start as usize + 4].try_into()?);
        let mut mass_error = 0f64;
        let mut edge_error = 0f64;
        for (&a, &b) in gpu_mass.iter().zip(&mass) {
            mass_error = mass_error.max((f64::from(a) - b).abs() / b.abs().max(1e-30));
        }
        for (gpu, &(neighbor, w)) in gpu_edges.iter().zip(&expected_edges) {
            assert_eq!(gpu.neighbor as usize, neighbor);
            edge_error = edge_error.max((f64::from(gpu.weight) - w).abs() / w.abs().max(1e-30));
        }
        for a in 0..n {
            for edge in &gpu_edges[expected_offsets[a]..expected_offsets[a + 1]] {
                let b = edge.neighbor as usize;
                let reverse = &gpu_edges[expected_offsets[b]..expected_offsets[b + 1]];
                let index = reverse
                    .binary_search_by_key(&(a as u32), |e| e.neighbor)
                    .unwrap();
                assert_eq!(
                    edge.weight.to_bits(),
                    reverse[index].weight.to_bits(),
                    "assembled directed edges must remain bitwise symmetric"
                );
            }
        }
        assert!(
            gpu_mass.iter().all(|v| v.is_finite())
                && gpu_edges.iter().all(|e| e.weight.is_finite())
        );
        let mut error = 0f64;
        let mut rhs_norm = 0f64;
        let mut minimum = f32::INFINITY;
        let mut residual = vec![[0f64; 3]; n];
        for i in 0..n {
            for c in 0..3 {
                assert!(values[i][c].is_finite());
                minimum = minimum.min(values[i][c]);
                error = error.max((f64::from(values[i][c]) - reference[i][c]).abs());
                residual[i][c] = mass[i] * (f64::from(values[i][c]) - source[i][c]);
                rhs_norm += (mass[i] * source[i][c]).powi(2);
            }
        }
        for &(a, b, w) in &edges {
            for c in 0..3 {
                let flux =
                    radii[c].powi(2) * w * (f64::from(values[a][c]) - f64::from(values[b][c]));
                residual[a][c] += flux;
                residual[b][c] -= flux;
            }
        }
        let residual_ratio = residual.iter().flatten().map(|x| x * x).sum::<f64>() / rhs_norm;
        let times: &[u64] =
            bytemuck::cast_slice(&data[time_start as usize..time_start as usize + 32]);
        let elapsed = |a: usize, b: usize| {
            queries
                .as_ref()
                .and_then(|_| times[b].checked_sub(times[a]))
                .filter(|t| *t > 0)
                .map(|t| t as f64 * f64::from(queue.get_timestamp_period()) / 1e6)
        };
        let qualified = status == 0
            && mass_error < 1e-4
            && edge_error < 1e-4
            && error < 1e-3
            && residual_ratio < 1e-6
            && minimum >= 0.;
        let report = serde_json::json!({"sample":sample,"time_seconds":input.get("time_seconds"),"iterations":40,
            "geometry_status":status,"bitwise_symmetric":true,"mass_max_relative_error":mass_error,"edge_max_relative_error":edge_error,
            "irradiance_max_absolute_error":error,"squared_relative_equation_residual":residual_ratio,
            "minimum_irradiance":minimum,"assembly_gpu_ms":elapsed(0,1),"solve_gpu_ms":elapsed(2,3),
            "encode_wait_readback_and_audit_wall_ms":start.elapsed().as_secs_f64()*1000.,"qualified":qualified});
        println!("RESIDENT GEOMETRY LIGHT {report}");
        reports.push(report);
        drop(data);
        readback.unmap();
        last_points = points;
    }
    // Invalid geometry must be explicitly signaled, then clear on valid refresh.
    let mut invalid_reports = Vec::new();
    for scenario in ["collapsed_triangle", "nonfinite_position", "valid_recovery"] {
        let mut points = last_points.clone();
        if scenario == "collapsed_triangle" {
            points[triangles[0][1] as usize] = points[triangles[0][0] as usize];
        }
        if scenario == "nonfinite_position" {
            points[triangles[0][0] as usize][0] = f32::NAN;
        }
        queue.write_buffer(&positions, 0, bytemuck::cast_slice(&packed(&points)));
        let mut encoder = device.create_command_encoder(&Default::default());
        geometry.encode(&mut encoder);
        encoder.copy_buffer_to_buffer(geometry.status_buffer(), 0, &readback, 0, 4);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let data = readback.slice(..).get_mapped_range()?;
        let status = u32::from_le_bytes(data[..4].try_into()?);
        assert_eq!(status == 0, scenario == "valid_recovery");
        invalid_reports.push(serde_json::json!({"scenario":scenario,"status":status}));
        drop(data);
        readback.unmap();
    }
    assert!(scope.pop().await.is_none());
    let qualified = reports.iter().all(|r| r["qualified"] == true);
    let report = serde_json::json!({"adapter":format!("{:?}",adapter.get_info()),"vertices":n,"triangles":triangles.len(),
        "resident_operator_reused":true,"coefficient_readback_before_solve":false,"samples":reports,
        "invalid_geometry_checks":invalid_reports,"qualified":qualified,"native_renderer_integrated":false,
        "scope":"Dynamic resident assembly and solve on supplied accepted poses. Diagnostic tail readback only. Incident light still uploaded. No native frame/FPS qualification."});
    std::fs::write(&args[0], serde_json::to_vec_pretty(&report)?)?;
    if !qualified {
        return Err("Resident GPU operator/light parity gate failed".into());
    }
    Ok(())
}
