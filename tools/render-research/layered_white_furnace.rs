//! Validation of the shared layered GGX candidate, not production pass qualification.
use std::f64::consts::PI;
use voxy_gpu_core::{ComputeProgram, GraphicsOptions};
fn unit_energy(c: f64) -> f64 {
    1.0 - c * (1.0 + 1.0 / c).ln()
}
fn fresnel_bias(view: f64) -> f64 {
    let cells = 128;
    let mut sum = 0.;
    let azimuths = (0..cells)
        .map(|i| (2. * PI * (i as f64 + 0.5) / cells as f64).cos())
        .collect::<Vec<_>>();
    for i in 0..cells {
        let light = (i as f64 + 0.5) / cells as f64;
        for &azimuth in &azimuths {
            let vl =
                view * light + (1. - view * view).sqrt() * (1. - light * light).sqrt() * azimuth;
            let vh = ((1. + vl) * 0.5).sqrt();
            sum += (1. - vh).powi(5) * light / (view + light) / (cells * cells) as f64;
        }
    }
    sum
}
fn main() {
    const N: usize = 512;
    const STRIDE: usize = 18;
    const HEADER: usize = 4;
    let source = include_str!("../../crates/voxy_render/src/surface_lighting.wgsl");
    let structures = source
        .lines()
        .filter(|l| l.starts_with("struct Surface ") || l.starts_with("struct Light "))
        .collect::<Vec<_>>()
        .join("\n");
    let functions =
        &source[source.find("fn smith_lambda(").unwrap()..source.find("@compute").unwrap()];
    let closure = include_str!("../../crates/voxy_render/src/material_energy.wgsl");
    let kernel = r#"
@compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=data[0] {return;}
    if id.x==0u {
        let tiny=bitcast<f32>(data[2]);data[2]=bitcast<u32>(normalized_energy_product(tiny,tiny,tiny));
        let invalid=GgxEnergyMoments(0.0,1.0,0.0,1.0,0.0,1.0);
        data[3]=layered_ggx_energy(vec3<f32>(0.0),vec3<f32>(1.0),vec3<f32>(1.0),invalid).valid;
    }
    let b=4u+18u*id.x;
    let c=bitcast<f32>(data[b]);
    let e=GgxEnergyMoments(bitcast<f32>(data[b+1u]),bitcast<f32>(data[b+2u]),bitcast<f32>(data[b+3u]),bitcast<f32>(data[b+4u]),bitcast<f32>(data[b+5u]),bitcast<f32>(data[b+6u]));
    let f0=vec3<f32>(bitcast<f32>(data[b+7u]),bitcast<f32>(data[b+8u]),bitcast<f32>(data[b+9u]));
    let substrate=vec3<f32>(bitcast<f32>(data[b+10u]),bitcast<f32>(data[b+11u]),bitcast<f32>(data[b+12u]));
    let view=vec3<f32>(0.0,0.0,1.0);let direction=vec3<f32>(sqrt(max(0.0,1.0-c*c)),0.0,c);
    let surface=Surface(vec4<f32>(0.0,0.0,0.0,1.0),vec4<f32>(0.0,0.0,1.0,1.0));
    light.camera=vec4<f32>(view,1.0);
    let forward=layered_ggx_energy(specular_brdf(surface,direction,f0),f0,substrate,e);
    light.camera=vec4<f32>(direction,1.0);
    let swapped=GgxEnergyMoments(e.light_a,e.light_loss,e.view_a,e.view_loss,e.average_a,e.average_loss);
    let backward=layered_ggx_energy(specular_brdf(surface,view,f0),f0,substrate,swapped);
    let weighted=forward.value*c*6.283185307179586;
    let difference=abs(forward.value-backward.value);
    for(var channel=0u;channel<3u;channel++){data[b+13u+channel]=bitcast<u32>(weighted[channel]);}
    data[b+16u]=bitcast<u32>(max(difference.x,max(difference.y,difference.z)));
    data[b+17u]=forward.valid & backward.valid;
}
"#;
    let shader = format!(
        "{structures}\n@group(0) @binding(0) var<storage,read_write> data:array<u32>;\nvar<private> light:Light;\n{functions}\n{closure}\n{kernel}"
    );
    let mut cases = Vec::new();
    for f0 in [0., 0.04, 0.5, 1.] {
        for albedo in [0., 0.5, 1.] {
            cases.push(([f0; 3], [albedo; 3]));
        }
    }
    cases.push(([0.95, 0.64, 0.54], [0.; 3]));
    cases.push(([0.04; 3], [0.8, 0.1, 0.02]));
    let view_b = fresnel_bias(1.);
    let view_a = unit_energy(1.) - view_b;
    let view_loss = 1. - unit_energy(1.);
    let moments = (0..N)
        .map(|i| {
            let c = (i as f64 + 0.5) / N as f64;
            let b = fresnel_bias(c);
            (c, unit_energy(c) - b, 1. - unit_energy(c))
        })
        .collect::<Vec<_>>();
    let average_a = moments
        .iter()
        .map(|&(c, a, _)| 2. * c * a / N as f64)
        .sum::<f64>();
    let average_loss = moments
        .iter()
        .map(|&(c, _, loss)| 2. * c * loss / N as f64)
        .sum::<f64>();
    let mut words = vec![0u32; HEADER + STRIDE * N * cases.len()];
    words[0] = (N * cases.len()) as u32;
    words[1] = N as u32;
    words[2] = 1e-20f32.to_bits();
    for (case, &(f0, substrate)) in cases.iter().enumerate() {
        for (i, &(c, a, loss)) in moments.iter().enumerate() {
            let row = [
                c,
                view_a,
                view_loss,
                a,
                loss,
                average_a,
                average_loss,
                f0[0],
                f0[1],
                f0[2],
                substrate[0],
                substrate[1],
                substrate[2],
            ];
            let base = HEADER + STRIDE * (case * N + i);
            for (i, value) in row.into_iter().enumerate() {
                words[base + i] = (value as f32).to_bits();
            }
        }
    }
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(ComputeProgram::new(&device, &shader)).unwrap();
    let bytes = words
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    let job = program.create_job(&device, &bytes).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder, [(N * cases.len()).div_ceil(64) as u32, 1, 1])
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
    assert_eq!(bytes.len(), words.len() * 4);
    let output = bytes
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(&output[..2], &words[..2]);
    assert_eq!(output[3], 0, "invalid moments were accepted");
    let tiny = f32::from_bits(output[2]) as f64;
    assert!(
        (tiny / (1e-20 / PI) - 1.).abs() < 1e-5,
        "normalized small product lost precision"
    );
    for (case, &(f0, substrate)) in cases.iter().enumerate() {
        let mut energy = [0.; 3];
        let mut reciprocity = 0f64;
        for i in 0..N {
            let b = HEADER + STRIDE * (case * N + i);
            assert_eq!(output[b + 17], 1, "invalid material case={case} sample={i}");
            for channel in 0..3 {
                energy[channel] += f32::from_bits(output[b + 13 + channel]) as f64 / N as f64;
            }
            reciprocity = reciprocity.max(f32::from_bits(output[b + 16]) as f64);
        }
        let expected = std::array::from_fn::<_, 3, _>(|channel| {
            let f = f0[channel];
            let favg = f + (1. - f) / 21.;
            let ess = 1. - average_loss;
            let fm = favg * favg * ess / ((1. - favg) + favg * ess);
            let coat = f * view_a + view_b + fm * view_loss;
            coat + substrate[channel] * (1. - coat)
        });
        for channel in 0..3 {
            assert!(
                energy[channel].is_finite()
                    && energy[channel] >= 0.
                    && energy[channel] <= 1. + 2e-5
            );
            assert!(
                (energy[channel] - expected[channel]).abs() < 2e-5,
                "energy mismatch case={case} channel={channel} GPU={energy:?} expected={expected:?}"
            );
        }
        assert!(reciprocity < 2e-6, "reciprocity case={case}: {reciprocity}");
        println!(
            "{{\"case\":{case},\"f0\":{f0:?},\"substrate\":{substrate:?},\"energy\":{energy:?},\"expected\":{expected:?},\"max_reciprocity_error\":{reciprocity}}}"
        );
    }
    println!(
        "{{\"adapter\":{:?},\"backend\":\"{:?}\",\"cases\":{},\"directions_per_case\":{N},\"tiny_normalized_product\":{tiny},\"scope\":\"maximum-roughness shared closure prototype; production integration and Smith random-walk comparison pending\"}}",
        info.name,
        info.backend,
        cases.len()
    );
}
