//! Prototype GPU moment audit. Conservation is not a convergence proof.
use voxy_gpu_core::{ComputeProgram, GraphicsOptions};
fn main() {
    const STRIDE: usize = 8;
    let mut queries = Vec::new();
    let refined_samples = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>().expect("sample budget must be an integer"))
        .unwrap_or(65536);
    assert!((4096..=1048576).contains(&refined_samples));
    let budgets = if std::env::args().any(|arg| arg == "--refined-only") {
        vec![refined_samples]
    } else {
        vec![4096u32, refined_samples]
    };
    for &samples in &budgets {
        for roughness in [0f32, 0.005, 0.05, 0.25, 0.6, 1.] {
            for nv in [0.05f32, 0.25, 0.7, 1.] {
                queries.push((nv, roughness, samples));
            }
        }
    }
    if std::env::args().any(|arg| arg == "--wide") {
        for &samples in &budgets {
            for (nv, r) in [
                (0.001f32, 1f32),
                (0.005, 0.6),
                (0.01, 0.1),
                (0.001, 0.005),
                (0.005, 0.05),
                (1., 0.0001),
                (0.25, 0.001),
                (0.999, 0.01),
                (0.5, 0.4),
                (0.9, 0.8),
            ] {
                queries.push((nv, r, samples));
            }
        }
    }
    if std::env::args().any(|s| s == "--lut-origin-32") {
        queries.push((0.015625, 0.015625, refined_samples));
    }
    if std::env::args().any(|s| s == "--grazing-grid") {
        let mut values = (0..64).map(|i| i as f32 / 63.).collect::<Vec<_>>();
        for value in [
            0.0001f32, 0.001, 0.01, 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.8, 0.9, 0.999,
        ] {
            if !values.iter().any(|v| v.to_bits() == value.to_bits()) {
                values.push(value);
            }
        }
        for r in values {
            queries.push((0., r, refined_samples));
        }
    } else if std::env::args().any(|s| s == "--grazing") {
        for r in [0f32, 0.0001, 0.001, 0.01, 0.1, 0.4, 0.8, 1.] {
            queries.push((0., r, refined_samples));
        }
    }
    if let Some(path) =
        std::env::args().find_map(|arg| arg.strip_prefix("--query-file=").map(str::to_owned))
    {
        queries.clear();
        let input = std::fs::read_to_string(path).expect("read moment queries");
        let mut identities = std::collections::HashSet::new();
        for line in input.lines().filter(|line| !line.trim().is_empty()) {
            let values = line
                .split_whitespace()
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 2);
            let (nv, r) = (values[0], values[1]);
            assert!((0.0..=1.0).contains(&nv) && (0.0..=1.0).contains(&r));
            assert!(
                identities.insert((nv.to_bits(), r.to_bits())),
                "duplicate moment query"
            );
            queries.push((nv, r, refined_samples));
        }
        assert!(!queries.is_empty());
    }
    queries.push((-1., 1., 4096));
    queries.push((1., 1., 0));
    let mut words = vec![0u32; 1 + STRIDE * queries.len()];
    words[0] = queries.len() as u32;
    for (i, &(nv, r, samples)) in queries.iter().enumerate() {
        let b = 1 + STRIDE * i;
        words[b] = nv.to_bits();
        words[b + 1] = r.to_bits();
        words[b + 2] = samples;
    }
    let kernel = r#"
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
@compute @workgroup_size(32) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=data[0]{return;}let b=1u+8u*id.x;
 let m=integrate_ggx_energy(bitcast<f32>(data[b]),bitcast<f32>(data[b+1u]),data[b+2u]);
 for(var c=0u;c<3u;c++){data[b+3u+c]=bitcast<u32>(m.value[c]);}data[b+6u]=m.valid;
 let nv=bitcast<f32>(data[b]);let r=bitcast<f32>(data[b+1u]);
 if m.valid==1u && r>0.0 && nv>0.0 {
  let view=vec3<f32>(sqrt(max(0.0,(1.0-nv)*(1.0+nv))),0.0,nv);
  let alpha=r*r;let lv=ggx_lambda_direction(view,alpha*alpha);
  data[b+7u]=bitcast<u32>(ggx_null_mass(nv,alpha,lv,512u));
 }
}
"#;
    let shader = format!(
        "{}\n{kernel}",
        include_str!("../../crates/voxy_render/src/material_energy_integrate.wgsl")
    );
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(ComputeProgram::new(&device, &shader)).unwrap();
    let batch_size = std::env::args()
        .find_map(|arg| arg.strip_prefix("--batch-size=").map(str::to_owned))
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(64);
    assert!((1..=256).contains(&batch_size));
    let mut output = words.clone();
    for start in (0..queries.len()).step_by(batch_size) {
        let count = (queries.len() - start).min(batch_size);
        let mut packet = Vec::with_capacity(1 + count * STRIDE);
        packet.push(count as u32);
        packet.extend_from_slice(&words[1 + start * STRIDE..1 + (start + count) * STRIDE]);
        let bytes = packet
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let job = program.create_job(&device, &bytes).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, [count.div_ceil(32) as u32, 1, 1])
            .unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let bytes = read.try_read().unwrap().unwrap();
        assert_eq!(bytes.len(), packet.len() * 4);
        let result = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(result[0], count as u32);
        output[1 + start * STRIDE..1 + (start + count) * STRIDE].copy_from_slice(&result[1..]);
    }
    assert_eq!(output[0], words[0]);
    let mut accuracy_failures = 0usize;
    for (i, &(nv, r, samples)) in queries.iter().enumerate() {
        let b = 1 + STRIDE * i;
        assert_eq!(&output[b..b + 3], &words[b..b + 3]);
        let m = std::array::from_fn::<_, 3, _>(|c| f32::from_bits(output[b + 3 + c]) as f64);
        let valid = output[b + 6];
        if nv < 0. || samples == 0 {
            assert_eq!(valid, 0);
            continue;
        }
        assert_eq!(
            valid, 1,
            "invalid moments nv={nv} roughness={r} samples={samples} m={m:?}"
        );
        assert!((m.iter().sum::<f64>() - 1.).abs() < 5e-6);
        assert!(m[0] + m[2] <= 1. + 1e-7);
        let analytic = if nv == 0. {
            Some(1.)
        } else if r == 1. {
            Some(1. - f64::from(nv) * (1. + 1. / f64::from(nv)).ln())
        } else if r == 0. {
            Some(1.)
        } else {
            None
        };
        if let Some(expected) = analytic {
            if (m[0] + m[1] - expected).abs() >= 0.00005 {
                accuracy_failures += 1;
                eprintln!(
                    "analytic mismatch nv={nv} roughness={r} samples={samples}: {m:?} expected={expected}"
                );
            }
        }
        if nv == 1. {
            // At normal incidence VNDF=NDF. Slopes q>=1 reflect below the
            // horizon; their exact probability is alpha^2/(1+alpha^2).
            // Nonzero loss and A+B+loss=1 do not establish its accuracy.
            let alpha_squared = f64::from(r).powi(4);
            let null_mass = alpha_squared / (1. + alpha_squared);
            if m[2] < null_mass * (1. - 1e-6) {
                accuracy_failures += 1;
                eprintln!(
                    "GGX loss below exact null-mass bound: r={r} samples={samples} loss={} bound={null_mass}",
                    m[2]
                );
            }
        }
        println!(
            "{{\"nv\":{nv},\"roughness\":{r},\"samples\":{samples},\"A\":{},\"B\":{},\"unit_loss\":{},\"null_mass_audit\":{},\"unit_energy\":{},\"analytic_unit_energy\":{}}}",
            m[0],
            m[1],
            m[2],
            f32::from_bits(output[b + 7]),
            m[0] + m[1],
            analytic.map_or_else(|| "null".to_owned(), |v| v.to_string())
        );
    }
    println!(
        "{{\"adapter\":{:?},\"backend\":\"{:?}\",\"valid_queries\":{},\"invalid_queries_rejected\":2,\"scope\":\"candidate moment producer; intermediate-roughness accuracy and production resource integration not proven\"}}",
        info.name,
        info.backend,
        queries.len() - 2
    );
    assert_eq!(
        accuracy_failures, 0,
        "GPU moment accuracy gate failed; do not integrate into production materials"
    );
}
