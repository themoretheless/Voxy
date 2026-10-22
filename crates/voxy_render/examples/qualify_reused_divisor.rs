//! GPU differential and exact-oracle payloads for repeated-divisor arithmetic.
use wgpu::util::DeviceExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("new output directory"));
    std::fs::create_dir(&root)?;
    let mut cases = Vec::<[u32; 8]>::new();
    let mut seed = 0x64697669736f7273u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as u32
    };
    for i in 0..524288 {
        let mut a = random();
        let mut b = random();
        if i % 2 == 0 {
            a = (a & 0x807fffff) | ((68 + random() % 123) << 23);
            b = (b & 0x807fffff) | ((68 + random() % 123) << 23);
        }
        let x = f32::from_bits(a);
        let y = f32::from_bits(b);
        let al = if i % 3 == 0 {
            0.
        } else {
            x * 2f32.powi(-26) * (random() as i32 as f32 / 2147483648.)
        };
        let bl = if i % 3 == 0 {
            0.
        } else {
            y * 2f32.powi(-26) * (random() as i32 as f32 / 2147483648.)
        };
        cases.push([a, b, 0, 0, al.to_bits(), bl.to_bits(), 0, 0]);
        if i % 16 == 0 && y.is_finite() && y != 0. {
            let z = (1.0 / f64::from(y)) as f32;
            for delta in [-2i32, -1, 1, 2] {
                cases.push([
                    a,
                    b,
                    z.to_bits().wrapping_add_signed(delta),
                    1,
                    al.to_bits(),
                    bl.to_bits(),
                    0,
                    0,
                ]);
            }
        }
    }
    let edges = [
        0u32, 0x80000000, 1, 0x007fffff, 0x00800000, 0x00800001, 0x217fffff, 0x21800000,
        0x21ffffff, 0x22000000, 0x22000001, 0x3effffff, 0x3f000000, 0x3f7fffff, 0x3f800000,
        0x3f800001, 0x3fffffff, 0x40000000, 0x40000001, 0x5f7fffff, 0x5f800000, 0x7f7fffff,
        0x7f800000, 0x7fc00001, 0xff800000, 0xffc00001,
    ];
    for a in edges {
        for b in edges {
            for sign in [0, 0x80000000] {
                cases.push([a, b ^ sign, 0, 0, 0, 0, 0, 0]);
            }
        }
    }
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("REUSED DIVISOR GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("exact original float words and corrupted reciprocal cases"),
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = cases.len() as u64 * 64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("certified reciprocal and Markstein quotient differential"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("../src/archive_projection_pair.wgsl"),
                include_str!("qualify_reused_divisor.wgsl")
            )
            .into(),
        ),
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
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |x| tx.send(x).unwrap());
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    rx.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    std::fs::write(root.join("inputs.bin"), bytemuck::cast_slice(&cases))?;
    std::fs::write(root.join("outputs.bin"), &bytes)?;
    println!(
        "{}",
        serde_json::json!({"cases":cases.len(),"input_stride":32,"output_stride":64,"production_connected":false,"full_scene_qualified":false,"fps_qualified":false})
    );
    Ok(())
}
