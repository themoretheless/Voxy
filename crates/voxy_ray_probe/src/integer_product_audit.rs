//! Isolated arithmetic audit; operands only are uploaded, never CPU outcomes.
use wgpu::util::DeviceExt;
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::var_os("VOXY_RAY_INTEGER_INPUT").ok_or("missing integer input")?;
    let target = std::env::var_os("VOXY_RAY_INTEGER_OUTPUT").ok_or("missing integer output")?;
    let bytes = std::fs::read(source)?;
    if bytes.is_empty() || bytes.len() % 2048 != 0 {
        return Err("integer input ABI mismatch".into());
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    if words.iter().any(|&v| v > 65535) {
        return Err("non-radix16 digit".into());
    }
    let ratio_seed = std::env::var("VOXY_RAY_INTEGER_RATIO_SEED").as_deref() == Ok("1");
    let ratio_audit = std::env::var("VOXY_RAY_INTEGER_RATIO").as_deref() == Ok("1");
    let float_product = std::env::var("VOXY_RAY_INTEGER_FLOAT_PRODUCT").as_deref() == Ok("1");
    let position_admission =
        std::env::var("VOXY_RAY_INTEGER_POSITION_ADMISSION").as_deref() == Ok("1");
    if u32::from(ratio_seed)
        + u32::from(ratio_audit)
        + u32::from(float_product)
        + u32::from(position_admission)
        > 1
    {
        return Err("select one arithmetic audit mode".into());
    }
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("full width integer product audit"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                include_str!("../../voxy_render/src/world_exact_integer.wgsl"),
                if ratio_audit || ratio_seed {
                    include_str!("../../voxy_render/src/world_exact_ratio.wgsl")
                } else {
                    ""
                },
                include_str!("integer_product_audit.wgsl"),
                if ratio_audit {
                    include_str!("ratio_rounding_audit.wgsl")
                } else {
                    ""
                },
                if float_product {
                    include_str!("float_product_audit.wgsl")
                } else {
                    ""
                },
                if position_admission {
                    include_str!("../../voxy_render/src/world_exact_history_position.wgsl")
                } else {
                    ""
                },
                if position_admission {
                    include_str!("position_admission_audit.wgsl")
                } else {
                    ""
                },
                if ratio_seed {
                    include_str!("../../voxy_render/src/world_exact_ratio_seed.wgsl")
                } else {
                    ""
                },
                if ratio_seed {
                    include_str!("ratio_seed_audit.wgsl")
                } else {
                    ""
                }
            )
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some(if ratio_seed {
            "main_ratio_seed_audit"
        } else if ratio_audit {
            "main_ratio_audit"
        } else if float_product {
            "main_float_product_audit"
        } else if position_admission {
            "main_position_admission_audit"
        } else {
            "main_integer_audit"
        }),
        compilation_options: Default::default(),
        cache: None,
    });
    let scratch = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16 * 512 * 128 * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut output = Vec::new();
    for batch in words.chunks(16 * 512) {
        let count = batch.len() / 512;
        let size = count as u64 * 132 * 4;
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(batch),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &scratch,
            0,
            bytemuck::cast_slice(&vec![0xdeadbeefu32; 16 * 512 * 128]),
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: result.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: scratch.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(count as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&result, 0, &read, 0, size);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        {
            let data = read.slice(..).get_mapped_range()?;
            output.extend_from_slice(&data);
        }
        read.unmap();
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target)?;
    file.write_all(&output)?;
    file.sync_all()?;
    println!(
        "INTEGER PRODUCT AUDIT cases={} scratch_poisoned=true gold_uploaded=false",
        words.len() / 512
    );
    Ok(())
}
