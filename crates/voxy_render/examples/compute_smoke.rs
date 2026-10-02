//! Native real-device compute and nonblocking readback evidence.
use voxy_render::{
    ComputeError, ComputeProgram, ComputeReadbackLimits, ComputeReadbackPool, GraphicsCapabilities,
    GraphicsOptions,
};

const SHADER: &str = r"
@group(0) @binding(0) var<storage, read_write> values: array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&values) {
        values[id.x] = values[id.x] * 3u + id.x;
    }
}";

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_COMPUTE_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
        Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
        Ok("metal") => voxy_render::GraphicsBackend::Metal,
        Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
        _ => return Err("VOXY_COMPUTE_BACKEND expects auto|vulkan|gl|metal|dx12".into()),
    };
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let require_nvidia = match arguments.as_slice() {
        [] => false,
        [flag] if flag == "--require-nvidia" => true,
        _ => return Err("expected optional --require-nvidia".into()),
    };
    let adapter = if require_nvidia {
        pollster::block_on(instance.enumerate_adapters(backend.backends()))
            .into_iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.vendor == 0x10de
                    && matches!(
                        info.device_type,
                        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                    )
            })
            .ok_or("requested compute backend has no physical NVIDIA adapter")?
    } else {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?
    };
    let capabilities = GraphicsCapabilities::discover(&adapter);
    if !capabilities.compute_shaders {
        return Err("adapter has no compute shaders".into());
    }
    println!("Compute on {:?}", capabilities.adapter);
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let pool = ComputeReadbackPool::configure(
        &device,
        ComputeReadbackLimits {
            max_bytes: 8192,
            max_buffers: 8,
        },
    )?;
    for invalid in [
        "broken WGSL".to_owned(),
        SHADER.replace("fn cs_main", "fn wrong_entry"),
        SHADER.replace("@binding(0)", "@binding(1)"),
    ] {
        assert!(matches!(
            pollster::block_on(ComputeProgram::new(&device, &invalid)),
            Err(ComputeError::Validation(_))
        ));
    }
    let program = pollster::block_on(ComputeProgram::new(&device, SHADER))?;
    assert!(matches!(
        program.create_job(&device, &[]),
        Err(ComputeError::InvalidBuffer)
    ));
    assert!(matches!(
        program.create_job(&device, &[1; 3]),
        Err(ComputeError::InvalidBuffer)
    ));
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert!(matches!(
        program
            .create_job(&device, &[0; 4])?
            .encode(&mut encoder, [0, 1, 1]),
        Err(ComputeError::InvalidDispatch)
    ));
    let over_limit = device
        .limits()
        .max_compute_workgroups_per_dimension
        .checked_add(1)
        .ok_or("compute workgroup limit cannot be incremented")?;
    for axis in 0..3 {
        let mut workgroups = [1; 3];
        workgroups[axis] = over_limit;
        assert!(matches!(
            program
                .create_job(&device, &[0; 4])?
                .encode(&mut encoder, workgroups),
            Err(ComputeError::InvalidDispatch)
        ));
    }
    // Nonmultiple of 64 verifies bounds guarding and the last partially filled workgroup.
    let input: Vec<u32> = (0..1025).map(|i| i * 7 + 2).collect();
    let second: Vec<u32> = (0..17).map(|i| i + 100).collect();
    let job = program.create_job(&device, bytemuck::cast_slice(&input))?;
    job.encode_step(&mut encoder, [17, 1, 1])?;
    let dispatch = job.encode_readback(&mut encoder)?;
    let other = program
        .create_job(&device, bytemuck::cast_slice(&second))?
        .encode(&mut encoder, [1, 1, 1])?;
    queue.submit([encoder.finish()]);
    let mut readback = dispatch.begin_read();
    let mut other_readback = other.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    verify(
        &readback.try_read()?.ok_or("readback still pending")?,
        &input,
    );
    verify(
        &other_readback
            .try_read()?
            .ok_or("second readback still pending")?,
        &second,
    );
    assert_eq!(readback.try_read(), Err(ComputeError::Consumed));
    // Cancel one read before callback completion and another after mapping,
    // without taking their data. Both resource lifecycles must be valid.
    for finish_before_drop in [false, true] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = program
            .create_job(&device, &[0; 4])?
            .encode(&mut encoder, [1, 1, 1])?;
        queue.submit([encoder.finish()]);
        let cancelled = dispatch.begin_read();
        if finish_before_drop {
            device.poll(wgpu::PollType::wait_indefinitely())?;
        }
        drop(cancelled);
        device.poll(wgpu::PollType::wait_indefinitely())?;
    }
    let resident = program.create_job(&device, bytemuck::cast_slice(&[2_u32, 9]))?;
    let mut snapshots = Vec::new();
    for _ in 0..3 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        resident.encode_step(&mut encoder, [1, 1, 1])?;
        let snapshot = resident.encode_snapshot(&mut encoder)?;
        queue.submit([encoder.finish()]);
        snapshots.push(snapshot.begin_read());
    }
    // Another step must not mutate the already-recorded independent snapshots.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    resident.encode_step(&mut encoder, [1, 1, 1])?;
    let final_result = resident.encode_readback(&mut encoder)?;
    queue.submit([encoder.finish()]);
    snapshots.push(final_result.begin_read());
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let mut expected = [2_u32, 9];
    for snapshot in &mut snapshots {
        expected = [expected[0] * 3, expected[1] * 3 + 1];
        let bytes = snapshot.try_read()?.ok_or("resident snapshot pending")?;
        assert_eq!(bytes.as_slice(), bytemuck::cast_slice::<u32, u8>(&expected));
    }
    println!(
        "RESIDENT COMPUTE PASS: four submissions, independent snapshots, job continued during mapping"
    );
    drop(readback);
    drop(other_readback);
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "DISPATCH LIMIT PASS: zero and over-limit X/Y/Z rejected; valid submissions remain clean"
    );
    reload_proof(&device, &queue)?;
    readback_pool_proof(&device, &queue, &pool)?;
    memory_budget_proof(&adapter)?;
    println!(
        "PASS: WGSL compute, 1042 exact results, partial workgroups, independent jobs, readback, cancellation and validation failures"
    );
    Ok(())
}

fn verify(data: &[u8], input: &[u32]) {
    assert_eq!(data.len(), input.len() * 4);
    for (index, (bytes, value)) in data.chunks_exact(4).zip(input).enumerate() {
        let actual = u32::from_ne_bytes(bytes.try_into().unwrap());
        let index_u32 = u32::try_from(index).unwrap();
        assert_eq!(actual, value * 3 + index_u32, "compute element {index}");
    }
}

fn reload_proof(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = SHADER.replace("fn cs_main", "fn selected_kernel")
        + r"
@compute @workgroup_size(64)
fn alternate_kernel(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&values) {
        values[id.x] = values[id.x] * 101u + id.x;
    }
}";
    assert!(pollster::block_on(ComputeProgram::new(device, &source)).is_err());
    let mut program = pollster::block_on(ComputeProgram::with_entry_point(
        device,
        &source,
        "selected_kernel",
    ))?;
    let input = [2_u32, 5, 11];
    let old = program.create_job(device, bytemuck::cast_slice(&input))?;
    assert_eq!(old.shader_revision(), 0);
    assert_eq!(old.entry_point(), "selected_kernel");
    let alternate = pollster::block_on(ComputeProgram::with_entry_point(
        device,
        &source,
        "alternate_kernel",
    ))?;
    let alternate = alternate.create_job(device, bytemuck::cast_slice(&input))?;
    assert!(!pollster::block_on(program.reload_shader(&source))?);
    let changed = source.replace("* 3u", "* 5u");
    assert!(pollster::block_on(program.reload_shader(&changed))?);
    assert_eq!(program.shader_revision(), 1);
    assert!(!pollster::block_on(program.reload_shader(&changed))?);
    for invalid in [
        "invalid WGSL".to_owned(),
        changed.replace("fn selected_kernel", "fn missing"),
        changed.replace("@binding(0)", "@binding(1)"),
    ] {
        assert!(pollster::block_on(program.reload_shader(&invalid)).is_err());
        assert_eq!(program.shader_revision(), 1);
    }
    let new = program.create_job(device, bytemuck::cast_slice(&input))?;
    assert_eq!(old.shader_revision(), 0);
    assert_eq!(new.shader_revision(), 1);
    assert_eq!(new.entry_point(), "selected_kernel");
    assert_eq!(alternate.entry_point(), "alternate_kernel");
    assert!(pollster::block_on(
        program.reload_with_entry_point(&changed, "alternate_kernel")
    )?);
    assert_eq!(program.shader_revision(), 2);
    assert!(!pollster::block_on(
        program.reload_with_entry_point(&changed, "alternate_kernel")
    )?);
    assert!(
        pollster::block_on(program.reload_with_entry_point(&changed, "missing_kernel")).is_err()
    );
    assert_eq!(program.shader_revision(), 2);
    let switched = program.create_job(device, bytemuck::cast_slice(&input))?;
    assert_eq!(switched.entry_point(), "alternate_kernel");
    assert_eq!(switched.shader_revision(), 2);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let old = old.encode(&mut encoder, [1, 1, 1])?;
    let new = new.encode(&mut encoder, [1, 1, 1])?;
    let alternate = alternate.encode(&mut encoder, [1, 1, 1])?;
    let switched = switched.encode(&mut encoder, [1, 1, 1])?;
    queue.submit([encoder.finish()]);
    for (dispatch, multiplier) in [(old, 3_u32), (new, 5), (alternate, 101), (switched, 101)] {
        let mut read = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let bytes = read.try_read()?.ok_or("reload readback pending")?;
        for (index, (bytes, value)) in bytes.chunks_exact(4).zip(input).enumerate() {
            assert_eq!(
                u32::from_le_bytes(bytes.try_into()?),
                value * multiplier + u32::try_from(index)?
            );
        }
    }
    println!(
        "COMPUTE RELOAD PASS: old jobs retained, new shader active, invalid replacement rejected"
    );
    Ok(())
}

fn readback_pool_proof(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pool: &ComputeReadbackPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let a = pollster::block_on(ComputeProgram::new(device, SHADER))?;
    let b = pollster::block_on(ComputeProgram::new(device, SHADER))?;
    let data = vec![7_u8; 8192];
    let job = a.create_job(device, &data)?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = job.encode_snapshot(&mut encoder)?;
    assert!(matches!(
        b.create_job(device, &[0; 4])?
            .encode(&mut encoder, [1, 1, 1]),
        Err(ComputeError::ReadbackBudget)
    ));
    queue.submit([encoder.finish()]);
    let cancelled = dispatch.begin_read();
    let creations = pool.stats().creations;
    drop(cancelled);
    assert_eq!(pool.stats().allocated_bytes, 8192);
    device.poll(wgpu::PollType::wait_indefinitely())?;
    assert_eq!(pool.stats().cached_buffers, 1);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = job.encode_snapshot(&mut encoder)?;
    assert_eq!(pool.stats().creations, creations);
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    assert_eq!(pending.try_read()?.ok_or("pooled readback pending")?, data);
    assert_eq!(pool.stats().cached_buffers, 1);
    assert!(pool.stats().reuses > 0);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let abandoned = job.encode_snapshot(&mut encoder)?;
    drop(encoder);
    drop(abandoned);
    assert_eq!(pool.stats().quarantined_buffers, 1);
    assert_eq!(pool.stats().allocated_bytes, 8192);
    pool.discard_quarantine()?;
    assert_eq!(pool.stats().quarantined_buffers, 0);
    assert_eq!(pool.stats().allocated_bytes, 0);
    println!(
        "READBACK POOL PASS: shared 8192-byte admission, cancellation completion, exact reuse, unsubmitted quarantine and explicit cleanup"
    );
    Ok(())
}

fn memory_budget_proof(adapter: &wgpu::Adapter) -> Result<(), Box<dyn std::error::Error>> {
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let budget = voxy_render::ComputeMemoryBudget::configure(&device, 8)?;
    let first_program = pollster::block_on(ComputeProgram::new(&device, SHADER))?;
    let second_program = pollster::block_on(ComputeProgram::new(&device, SHADER))?;
    let first = first_program.create_job(&device, bytemuck::cast_slice(&[3_u32]))?;
    let second = second_program.create_job(&device, bytemuck::cast_slice(&[5_u32]))?;
    assert!(matches!(
        second_program.create_job(&device, &[0; 4]),
        Err(ComputeError::MemoryBudget)
    ));
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = first.encode(&mut encoder, [1, 1, 1])?;
    assert_eq!(budget.stats().allocated_bytes, 8);
    assert_eq!(budget.stats().retired_buffers, 1);
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let result = pending
        .try_read()?
        .ok_or("storage-budget readback pending")?;
    assert_eq!(bytemuck::cast_slice::<u8, u32>(&result), &[9]);
    assert!(matches!(
        first_program.create_job(&device, &[0; 4]),
        Err(ComputeError::MemoryBudget)
    ));
    let mut retirement = budget.begin_retirement(&queue);
    assert_eq!(budget.stats().allocated_bytes, 8);
    device.poll(wgpu::PollType::wait_indefinitely())?;
    assert!(retirement.try_finish()?);
    assert!(retirement.try_finish()?);
    assert_eq!(budget.stats().allocated_bytes, 4);
    let replacement = first_program.create_job(&device, &[0; 4])?;
    drop(replacement);
    drop(second);
    budget.discard_retired()?;
    assert_eq!(budget.stats().allocated_bytes, 0);
    println!(
        "COMPUTE MEMORY PASS: shared 8-byte admission, exact GPU result, retired charges and explicit reclamation"
    );
    Ok(())
}
