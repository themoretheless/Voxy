//! Actual Metal comparison against the original eight-corner evaluation order.
#[test]
#[ignore = "exclusive Metal GPU; VOXY_PROBE_SAMPLING_PROOF must be a fresh directory"]
#[cfg(target_os = "macos")]
fn original_probe_interpolation_words_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let output = std::path::PathBuf::from(
        std::env::var_os("VOXY_PROBE_SAMPLING_PROOF").ok_or("fresh proof directory required")?,
    );
    std::fs::create_dir(&output)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    const COUNT: u32 = 131072;
    let mut state = 0x7167_924bu32;
    let mut next = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state
    };
    let mut coefficients = Vec::<[f32; 4]>::new();
    for _ in 0..60 {
        for k in 0..4 {
            coefficients.push(std::array::from_fn(|c| {
                if c == 3 {
                    1.
                } else {
                    let v = ((next() >> 8) as f32 / 16777216.) * 4096.;
                    if k == 0 { v } else { v - 2048. }
                }
            }));
        }
    }
    let mut samples = Vec::<[f32; 8]>::new();
    for _ in 0..COUNT {
        let p = [6., 5., 7.].map(|extent| ((next() >> 8) as f32 / 16777216.) * extent - 1.);
        let v = glam::Vec3::new(
            (next() as i32) as f32,
            (next() as i32) as f32,
            (next() as i32) as f32,
        )
        .normalize();
        samples.push([p[0], p[1], p[2], 0., v.x, v.y, v.z, 0.]);
    }
    let grid = crate::probe_gi::ProbeGrid {
        origin: [0., 0., 0., 1.],
        spacing: [1., 1., 1., 0.],
        dims: [4, 3, 5, 0],
    };
    let source = format!(
        "{}\n{}\n{}",
        include_str!("probe_sampling.wgsl").replace("@group(2)", "@group(0)"),
        REFERENCE,
        r"
struct Sample { p:vec4<f32>, n:vec4<f32> }
@group(0) @binding(0) var<storage,read> samples:array<Sample>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=arrayLength(&samples){return;}let s=samples[id.x];
 result[id.x*2u]=bitcast<vec4<u32>>(probe_irradiance(s.p.xyz,s.n.xyz));
 result[id.x*2u+1u]=bitcast<vec4<u32>>(reference_probe_irradiance(s.p.xyz,s.n.xyz));
}"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("original probe interpolation order"),
        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("original probe interpolation order"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let upload = |label, bytes: &[u8], usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytes,
            usage,
        })
    };
    let inputs = upload(
        "actual probe interpolation inputs",
        bytemuck::cast_slice(&samples),
        wgpu::BufferUsages::STORAGE,
    );
    let sh = upload(
        "probe coefficients, never expected results",
        bytemuck::cast_slice(&coefficients),
        wgpu::BufferUsages::STORAGE,
    );
    let uniform = upload(
        "original grid",
        bytemuck::bytes_of(&grid),
        wgpu::BufferUsages::UNIFORM,
    );
    let size = u64::from(COUNT) * 32;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let entries = [(0, &inputs), (1, &result), (5, &sh), (6, &uniform)].map(|(binding, buffer)| {
        wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        }
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(COUNT.div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&result, 0, &staging, 0, size);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let (send, receive) = std::sync::mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).unwrap();
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    })?;
    receive.recv()??;
    let bytes = staging.slice(..).get_mapped_range()?;
    let mut different = 0;
    let mut first = Vec::new();
    for (i, pair) in bytes.chunks_exact(32).enumerate() {
        for c in 0..4 {
            let word = |offset| u32::from_le_bytes(pair[offset..offset + 4].try_into().unwrap());
            let a = word(c * 4);
            let b = word(16 + c * 4);
            assert!(f32::from_bits(a).is_finite() && f32::from_bits(b).is_finite());
            if a != b {
                different += 1;
                if first.len() < 16 {
                    first.push(serde_json::json!({"sample":i,"channel":c,"production_u32":a,"original_u32":b}));
                }
            }
        }
    }
    std::fs::write(output.join("actual-output-pairs.u32.bin"), &*bytes)?;
    std::fs::write(output.join("production-and-reference.wgsl"), &source)?;
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"samples":COUNT,"compared_native_words":COUNT*4,"different_native_words":different,"first_differences":first,"all_words_equal":different==0,"expected_results_uploaded":false,"input_blake3":blake3::hash(bytemuck::cast_slice(&samples)).to_hex().to_string(),"coefficient_blake3":blake3::hash(bytemuck::cast_slice(&coefficients)).to_hex().to_string(),"performance_qualified":false}),
        )?,
    )?;
    assert_eq!(
        different, 0,
        "probe interpolation changed the original rounding sequence"
    );
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
const REFERENCE: &str = r"
fn reference_probe_irradiance(p:vec3<f32>,n:vec3<f32>)->vec4<f32>{
 if probe_grid.origin.w<0.5{return vec4<f32>(0.);}
 let dims=vec3<f32>(probe_grid.dims.xyz);
 let g=(p+n*0.4-probe_grid.origin.xyz)/probe_grid.spacing.xyz;
 let inside=clamp(min(g+0.5,dims-0.5-g),vec3<f32>(0.),vec3<f32>(1.));
 let fade=inside.x*inside.y*inside.z;if fade<=0.{return vec4<f32>(0.);}
 let c=clamp(g,vec3<f32>(0.),dims-1.);let base=vec3<u32>(min(floor(c),dims-2.));let f=c-vec3<f32>(base);
 var sum=vec3<f32>(0.);
 for(var i=0u;i<8u;i++){
  let o=vec3<u32>(i&1u,(i>>1u)&1u,(i>>2u)&1u);let q=base+o;
  let w=select(1.-f.x,f.x,o.x==1u)*select(1.-f.y,f.y,o.y==1u)*select(1.-f.z,f.z,o.z==1u);
  sum+=probe_eval(q.x+probe_grid.dims.x*(q.y+probe_grid.dims.y*q.z),n)*w;
 }
 return vec4<f32>(sum,fade);
}";
