//! Native GPU parity gate for screened surface irradiance diffusion.
use voxy_render::{SurfaceLightDiffusion, SurfaceLightEdge};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = std::env::args().nth(1) {
        return pollster::block_on(body_gate(&path));
    }
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let edges = vec![
            SurfaceLightEdge {
                neighbor: 1,
                weight: 1.,
            },
            SurfaceLightEdge {
                neighbor: 2,
                weight: 1.,
            },
            SurfaceLightEdge {
                neighbor: 0,
                weight: 1.,
            },
            SurfaceLightEdge {
                neighbor: 2,
                weight: 1.,
            },
            SurfaceLightEdge {
                neighbor: 0,
                weight: 1.,
            },
            SurfaceLightEdge {
                neighbor: 1,
                weight: 1.,
            },
        ];
        assert!(
            SurfaceLightDiffusion::new(&device, &[1.; 3], &[0, 2, 4, 6], &edges[..5], [1.; 3])
                .is_err()
        );
        let mut asymmetric = edges.clone();
        asymmetric[0].weight = 2.;
        assert!(
            SurfaceLightDiffusion::new(&device, &[1.; 3], &[0, 2, 4, 6], &asymmetric, [1.; 3])
                .is_err()
        );
        let mut diffusion =
            SurfaceLightDiffusion::new(&device, &[1.; 3], &[0, 2, 4, 6], &edges, [1., 1., 0.])?;
        let mut encoder = device.create_command_encoder(&Default::default());
        assert!(diffusion.encode(&mut encoder, 64).is_err());
        diffusion.update_source(
            &queue,
            &[[1., 0.7, 0.3, 1.], [0., 0.7, 0.4, 1.], [0., 0.7, 0.9, 1.]],
        )?;
        assert!(
            diffusion
                .update_source(&queue, &[[f32::NAN; 4]; 3])
                .is_err()
        );
        diffusion.encode(&mut encoder, 63)?;
        diffusion.encode(&mut encoder, 1)?; // Continue across both ping-pong orientations.
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("diffusion parity readback"),
            size: 48,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(diffusion.output(), 0, &readback, 0, 48);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let data = readback.slice(..).get_mapped_range()?;
        let values: &[[f32; 4]] = bytemuck::cast_slice(&data);
        for (i, expected) in [0.5, 0.25, 0.25].into_iter().enumerate() {
            assert!((values[i][0] - expected).abs() < 1e-6);
            assert!((values[i][1] - 0.7).abs() < 1e-6);
            assert_eq!(values[i][2], [0.3, 0.4, 0.9][i]);
        }
        assert!((values.iter().map(|v| v[0]).sum::<f32>() - 1.).abs() < 1e-6);
        println!(
            "SURFACE LIGHT GPU PASS {:?} rgba={values:?} bytes={} analytic_error_below=1e-6 native_render_integrated=false",
            adapter.get_info(),
            diffusion.allocation_bytes()
        );
        drop(data);
        readback.unmap();
        Ok(())
    })
}

async fn body_gate(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let input: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let mass: Vec<f64> = serde_json::from_value(input["mass"].clone())?;
    let edges: Vec<(usize, usize, f64)> = serde_json::from_value(input["edges"].clone())?;
    let source: Vec<[f64; 3]> = serde_json::from_value(input["source"].clone())?;
    let reference: Vec<[f64; 3]> = serde_json::from_value(input["reference"].clone())?;
    let radii: [f64; 3] = serde_json::from_value(input["radii"].clone())?;
    assert_eq!(mass.len(), source.len());
    assert_eq!(mass.len(), reference.len());
    let mut rows = vec![Vec::new(); mass.len()];
    for &(a, b, w) in &edges {
        rows[a].push(SurfaceLightEdge {
            neighbor: b as u32,
            weight: w as f32,
        });
        rows[b].push(SurfaceLightEdge {
            neighbor: a as u32,
            weight: w as f32,
        });
    }
    let mut offsets = vec![0u32];
    let mut csr = Vec::new();
    for row in rows {
        csr.extend(row);
        offsets.push(csr.len() as u32);
    }
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance.request_adapter(&Default::default()).await?;
    let timestamp_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: timestamp_features,
            ..Default::default()
        })
        .await?;
    let queries = if timestamp_features.is_empty() {
        None
    } else {
        Some(device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("diffusion GPU time"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        }))
    };
    let timestamp_resolve = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("diffusion time resolve"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut diffusion = SurfaceLightDiffusion::new_accelerated(
        &device,
        &mass.iter().map(|m| *m as f32).collect::<Vec<_>>(),
        &offsets,
        &csr,
        radii.map(|r| r as f32),
    )?;
    let source_gpu: Vec<[f32; 4]> = source
        .iter()
        .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32, 1.])
        .collect();
    let size = (mass.len() * 16) as u64;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("full body diffusion parity"),
        size: size + 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut reports = Vec::new();
    for iterations in [16, 32, 64, 128, 256, 512] {
        diffusion.update_source(&queue, &source_gpu)?;
        let start = std::time::Instant::now();
        let mut encoder = device.create_command_encoder(&Default::default());
        diffusion.encode_timed(
            &mut encoder,
            iterations,
            queries.as_ref().map(|q| (q, 0, 1)),
        )?;
        if let Some(q) = &queries {
            encoder.resolve_query_set(q, 0..2, &timestamp_resolve, 0);
            encoder.copy_buffer_to_buffer(&timestamp_resolve, 0, &readback, size, 16);
        }
        encoder.copy_buffer_to_buffer(diffusion.output(), 0, &readback, 0, size);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let data = readback.slice(..).get_mapped_range()?;
        let values: &[[f32; 4]] = bytemuck::cast_slice(&data[..size as usize]);
        let gpu_ms = queries.as_ref().and_then(|_| {
            let times: &[u64] = bytemuck::cast_slice(&data[size as usize..]);
            times[1].checked_sub(times[0]).filter(|ticks| *ticks > 0).map(|ticks| ticks as f64 * f64::from(queue.get_timestamp_period()) / 1e6)
        });
        let mut error = 0f64;
        let mut rhs_norm = 0f64;
        let mut residual = vec![[0f64; 3]; mass.len()];
        for i in 0..mass.len() {
            for c in 0..3 {
                assert!(values[i][c].is_finite());
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
        let residual_ratio = residual.iter().flatten().map(|r| r * r).sum::<f64>() / rhs_norm;
        let wall_ms = start.elapsed().as_secs_f64() * 1000.;
        let minimum = values
            .iter()
            .flat_map(|v| &v[..3])
            .copied()
            .fold(f32::INFINITY, f32::min);
        let report = serde_json::json!({"gpu_compute_ms":gpu_ms, "method":"Chebyshev", "minimum_irradiance":minimum, "iterations":iterations,"max_absolute_error":error,"squared_l2_equation_residual_ratio":residual_ratio,"encode_submit_wait_readback_and_audit_wall_ms":wall_ms,"qualified":error<1e-3 && residual_ratio<1e-6 && minimum>=0.});
        println!("FULL BODY GPU DIFFUSION {report}");
        let qualified = report["qualified"] == true;
        reports.push(report);
        drop(data);
        readback.unmap();
        if qualified {
            break;
        }
    }
    let report = serde_json::json!({"adapter":format!("{:?}",adapter.get_info()),"vertices":mass.len(),"undirected_edges":edges.len(),"reports":reports,"native_character_integrated":false,"target_120fps_achieved":false,"scope":"Static original body against CPU f64 reference; wall times include CPU encoding, readback and equation audit, not GPU timestamps."});
    if let Some(path) = std::env::args().nth(2) {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    println!(
        "FULL BODY GATE qualification={}",
        report["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["qualified"] == true)
    );
    if !report["reports"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["qualified"] == true)
    {
        return Err("Full-body GPU diffusion did not meet the CPU parity gate".into());
    }
    Ok(())
}
