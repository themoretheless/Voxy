//! Independent exhaustive GPU power-of-two input scaling gate.
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let count = (1u32 << 24) + 190 * 4 * 2;
    let size = u64::from(count) * 4;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("all signed subnormal scaling outputs"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scaling proof readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let source = format!(
        "{}\n{}",
        include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),
        r"
@group(0) @binding(0) var<storage,read_write> results:array<u32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x+id.y*65536u;
 if index>=arrayLength(&results){return;}
 var bits=0u;
 if index<16777216u {bits=(index&0x007fffffu)|select(0u,0x80000000u,index>=8388608u);}
 else {
  let k=index-16777216u;let mantissas=array<u32,4>(0u,1u,0x003fffffu,0x007fffffu);
  bits=(((k/8u)+1u)<<23u)|mantissas[(k/2u)%4u]|select(0u,0x80000000u,(k&1u)!=0u);
 }
 results[index]=bitcast<u32>(projection_scale_up64(bitcast<f32>(bits)));
}
"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("exact subnormal normalization gate"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(1024, count.div_ceil(65536), 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = read.slice(..).get_mapped_range().unwrap();
    let mut failures = 0;
    for (i, row) in bytes.chunks_exact(4).enumerate() {
        let i = i as u32;
        let bits = if i < 1 << 24 {
            (i & 0x007fffff) | if i >= 1 << 23 { 0x80000000 } else { 0 }
        } else {
            let k = i - (1 << 24);
            let fraction = [0, 1, 0x003fffff, 0x007fffff][((k / 2) % 4) as usize];
            (((k / 8) + 1) << 23) | fraction | if k & 1 != 0 { 0x80000000 } else { 0 }
        };
        // f64 represents every original f32 exactly; scaling by this power
        // stays exact and finite throughout the helper's entire tested domain.
        let expected = (f64::from(f32::from_bits(bits)) * 18446744073709551616f64) as f32;
        let actual = u32::from_le_bytes(row.try_into().unwrap());
        if actual != expected.to_bits() {
            if failures < 10 {
                eprintln!(
                    "SCALE_MISMATCH input={bits:08x} actual={actual:08x} expected={:08x}",
                    expected.to_bits()
                );
            }
            failures += 1;
        }
    }
    drop(bytes);
    read.unmap();
    println!(
        "GPU_PROJECTION_SCALE_UP64 cases={count} signed_subnormals_and_zero_exhaustive=true normal_exponents_1_through_190=true independent_f64_oracle=true failures={failures} mathematical_scale_exact=true frame_rate_proof=false"
    );
    assert_eq!(failures, 0);
}
