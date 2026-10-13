//! Actual-device audit of conditional resident illumination publication.
use voxy_render::{SurfaceLightAdmission, SurfaceLightGeometry, SurfaceLightPcg};
use wgpu::util::DeviceExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
fn read(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let data = {
        let view = buffer.slice(..).get_mapped_range()?;
        bytemuck::cast_slice::<u8, u64>(&view).to_vec()
    };
    buffer.unmap();
    Ok(data)
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: surface_light_admission REPORT INPUT...".into());
    }
    let inputs: Vec<serde_json::Value> = args[1..]
        .iter()
        .map(|p| Ok::<_, Box<dyn std::error::Error>>(serde_json::from_slice(&std::fs::read(p)?)?))
        .collect::<Result<_, _>>()?;
    let triangles: Vec<[u32; 3]> = serde_json::from_value(inputs[0]["triangles"].clone())?;
    let mut points: Vec<[f32; 3]> = serde_json::from_value(inputs[0]["points"].clone())?;
    let radii: [f32; 3] = serde_json::from_value(inputs[0]["radii"].clone())?;
    let n = points.len();
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
        label: Some("admission body stream"),
        contents: bytemuck::cast_slice(&packed(&points)),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let geometry = SurfaceLightGeometry::new(&device, &positions, n, 9, &triangles)?;
    let mut solver = SurfaceLightPcg::from_geometry(&device, &geometry, radii)?;
    let bytes = n as u64 * 16;
    let incident = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("immutable incident light"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let short = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("short source validation fixture"),
        size: 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let no_copy = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("source copy usage validation fixture"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let mut rejected_encoder = device.create_command_encoder(&Default::default());
    assert!(
        solver
            .reset_from_resident_source(&mut rejected_encoder, &short)
            .is_err()
    );
    assert!(
        solver
            .reset_from_resident_source(&mut rejected_encoder, &no_copy)
            .is_err()
    );
    assert!(
        SurfaceLightAdmission::new(&device, &geometry, &short, &incident, radii, 1e-3, 1e-6)
            .is_err()
    );
    assert!(
        SurfaceLightAdmission::new(
            &device,
            &geometry,
            solver.output(),
            &incident,
            radii,
            0.,
            1e-6
        )
        .is_err()
    );
    assert!(
        SurfaceLightAdmission::new(
            &device,
            &geometry,
            solver.output(),
            &incident,
            radii,
            f32::NAN,
            1e-6
        )
        .is_err()
    );
    let admission = SurfaceLightAdmission::new(
        &device,
        &geometry,
        solver.output(),
        &incident,
        radii,
        1e-3,
        1e-6,
    )?;
    let queries = (!features.is_empty()).then(|| {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("assembly solve admission times"),
            ty: wgpu::QueryType::Timestamp,
            count: 6,
        })
    });
    let timestamps = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("publication timestamps"),
        size: 48,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let published_bytes = bytes * 2;
    let header_start = bytes + published_bytes;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("publication audit"),
        size: header_start + 32 + 48,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut accepted = vec![0u8; published_bytes as usize];
    let mut generation = 0u32;
    let mut reports = Vec::new();
    let mut last_source = Vec::new();
    let mut last_budget = 0;
    for (sample, input) in inputs.iter().enumerate() {
        let current_triangles: Vec<[u32; 3]> = serde_json::from_value(input["triangles"].clone())?;
        assert_eq!(current_triangles, triangles);
        let current_radii: [f32; 3] = serde_json::from_value(input["radii"].clone())?;
        assert_eq!(current_radii, radii);
        points = serde_json::from_value(input["points"].clone())?;
        assert_eq!(points.len(), n);
        let source: Vec<[f64; 3]> = serde_json::from_value(input["source"].clone())?;
        let reference: Vec<[f64; 3]> = serde_json::from_value(input["reference"].clone())?;
        assert_eq!(source.len(), n);
        assert_eq!(reference.len(), n);
        let source: Vec<_> = source
            .iter()
            .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32, 1.])
            .collect();
        queue.write_buffer(&positions, 0, bytemuck::cast_slice(&packed(&points)));
        queue.write_buffer(&incident, 0, bytemuck::cast_slice::<[f32; 4], u8>(&source));
        let mut admitted = false;
        for iterations in [1, 40, 64, 128, 256] {
            let mut encoder = device.create_command_encoder(&Default::default());
            geometry.encode_timed(&mut encoder, queries.as_ref().map(|q| (q, 0, 1)));
            solver.reset_from_resident_source(&mut encoder, &incident)?;
            solver.encode_timed(
                &mut encoder,
                iterations,
                queries.as_ref().map(|q| (q, 2, 3)),
            )?;
            admission.encode_timed(&mut encoder, queries.as_ref().map(|q| (q, 4, 5)));
            encoder.copy_buffer_to_buffer(solver.output(), 0, &readback, 0, bytes);
            encoder.copy_buffer_to_buffer(
                admission.accepted_buffer(),
                0,
                &readback,
                bytes,
                published_bytes,
            );
            encoder.copy_buffer_to_buffer(
                admission.header_buffer(),
                0,
                &readback,
                header_start,
                32,
            );
            if let Some(q) = &queries {
                encoder.resolve_query_set(q, 0..6, &timestamps, 0);
                encoder.copy_buffer_to_buffer(&timestamps, 0, &readback, header_start + 32, 48);
            }
            queue.submit([encoder.finish()]);
            let words = read(&device, &readback)?;
            let data: &[u8] = bytemuck::cast_slice(&words);
            let header: &[u32] =
                bytemuck::cast_slice(&data[(header_start) as usize..(header_start + 32) as usize]);
            let candidate: &[[f32; 4]] = bytemuck::cast_slice(&data[..bytes as usize]);
            let mut error = 0f64;
            for i in 0..n {
                for c in 0..3 {
                    error = error.max((f64::from(candidate[i][c]) - reference[i][c]).abs());
                }
            }
            assert_eq!(header[0], 0);
            let current = header[1] == 1;
            let published = &data[bytes as usize..(header_start) as usize];
            if current {
                assert!(
                    error <= f64::from(f32::from_bits(header[4])),
                    "admitted max-error estimate must cover the independent CPU error"
                );
                assert!(
                    candidate
                        .iter()
                        .flatten()
                        .all(|x| x.is_finite() && *x >= 0.)
                );
                assert!(
                    error < 1e-3,
                    "admitted candidate failed independent CPU error gate {error}"
                );
                assert_eq!(&published[..bytes as usize], &data[..bytes as usize]);
                assert_eq!(
                    &published[bytes as usize..],
                    bytemuck::cast_slice::<[f32; 4], u8>(&source)
                );
                generation += 1;
                assert_eq!(header[3], generation);
                assert_eq!(header[2], 1);
                accepted = published.to_vec();
                admitted = true;
                last_budget = iterations;
            } else {
                assert_eq!(
                    published,
                    accepted.as_slice(),
                    "rejection must preserve every accepted RGBA bit"
                );
                assert_eq!(header[3], generation);
                assert_eq!(header[2], u32::from(generation > 0));
            }
            if iterations == 1 && n > 1000 {
                assert!(!current, "underconverged full-body solve must be rejected");
            }
            let times: &[u64] = bytemuck::cast_slice(&data[(header_start + 32) as usize..]);
            let elapsed = |a: usize, b: usize| {
                queries
                    .as_ref()
                    .and_then(|_| times[b].checked_sub(times[a]))
                    .filter(|t| *t > 0)
                    .map(|t| t as f64 * f64::from(queue.get_timestamp_period()) / 1e6)
            };
            let report = serde_json::json!({"sample":sample,"time_seconds":input.get("time_seconds"),"iterations":iterations,
                "admitted":current,"has_accepted":header[2],"generation":header[3],"max_error_estimate":f32::from_bits(header[4]),
                "squared_relative_residual":f32::from_bits(header[5]),"cpu_max_error":error,
                "assembly_gpu_ms":elapsed(0,1),"solve_gpu_ms":elapsed(2,3),"admission_gpu_ms":elapsed(4,5)});
            println!("SURFACE LIGHT PUBLICATION {report}");
            reports.push(report);
            if iterations == 256 && !current {
                let path = std::path::Path::new(&args[0])
                    .with_extension(format!("sample-{sample}.candidate.f32"));
                std::fs::write(path, &data[..bytes as usize])?;
            }
            if admitted {
                break;
            }
        }
        assert!(admitted, "no candidate met publication gate");
        last_source = source;
    }
    let mut failures = Vec::new();
    for scenario in [
        "collapsed_geometry",
        "nonfinite_geometry",
        "nonfinite_incident",
        "negative_incident",
        "negative_alpha",
        "recovery",
    ] {
        let mut p = points.clone();
        let mut source = last_source.clone();
        if scenario == "collapsed_geometry" {
            p[triangles[0][1] as usize] = p[triangles[0][0] as usize];
        }
        if scenario == "nonfinite_geometry" {
            p[triangles[0][0] as usize][0] = f32::NAN;
        }
        if scenario == "nonfinite_incident" {
            source[triangles[0][0] as usize][0] = f32::NAN;
        }
        if scenario == "negative_incident" {
            for s in &mut source {
                s[0] = -1.;
            }
        }
        if scenario == "negative_alpha" {
            source[triangles[0][0] as usize][3] = -1.;
        }
        queue.write_buffer(&positions, 0, bytemuck::cast_slice(&packed(&p)));
        queue.write_buffer(&incident, 0, bytemuck::cast_slice::<[f32; 4], u8>(&source));
        let mut encoder = device.create_command_encoder(&Default::default());
        geometry.encode(&mut encoder);
        solver.reset_from_resident_source(&mut encoder, &incident)?;
        solver.encode_timed(&mut encoder, last_budget, None)?;
        admission.encode(&mut encoder);
        encoder.copy_buffer_to_buffer(
            admission.accepted_buffer(),
            0,
            &readback,
            bytes,
            published_bytes,
        );
        encoder.copy_buffer_to_buffer(admission.header_buffer(), 0, &readback, header_start, 32);
        queue.submit([encoder.finish()]);
        let words = read(&device, &readback)?;
        let data: &[u8] = bytemuck::cast_slice(&words);
        let header: &[u32] =
            bytemuck::cast_slice(&data[(header_start) as usize..(header_start + 32) as usize]);
        let recovery = scenario == "recovery";
        assert_eq!(header[1] == 1, recovery);
        assert_eq!(header[2], 1);
        if recovery {
            generation += 1;
        }
        assert_eq!(header[3], generation);
        assert_eq!(
            &data[bytes as usize..(header_start) as usize],
            accepted.as_slice(),
            "failed attempts must retain accepted light; deterministic recovery must reproduce it"
        );
        failures.push(serde_json::json!({"scenario":scenario,"geometry_status":header[0],"admitted":header[1],"generation":header[3],"accepted_bits_preserved":true}));
    }
    assert!(scope.pop().await.is_none());
    let report = serde_json::json!({"adapter":format!("{:?}",adapter.get_info()),"vertices":n,"samples":reports,
        "rejection_and_recovery":failures,"gpu_only_admission":true,"native_renderer_integrated":false,
        "scope":"Independent GPU residual and max-error estimate admission, bitwise retained accepted light. CPU tail audit only. Not an interval proof, whole-model rollback or frame/FPS qualification."});
    std::fs::write(&args[0], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
