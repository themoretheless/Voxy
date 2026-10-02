//! Independent nonblocking terrain jobs and cancellation on a real native GPU.
mod support;
use voxy_core::{CancelToken, ChunkPos};
use voxy_gpu::{GpuTerrainError, TerrainProgram};
use voxy_render::{ComputeError, GraphicsOptions};
use voxy_world::{ChunkGenerator, GenerationError, ProceduralTerrainGenerator, WorldSeed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Async terrain GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let (palette, water) = support::test_palette()?;
    let cpu = ProceduralTerrainGenerator::new(palette, water);
    let program = pollster::block_on(TerrainProgram::new(&device, palette, water))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let token = CancelToken::new();
    let positions = [
        ChunkPos { x: -7, y: -1, z: 4 },
        ChunkPos {
            x: i64::MIN / 32,
            y: 0,
            z: i64::MAX / 32,
        },
    ];
    let mut reads = Vec::new();
    for pos in positions {
        reads.push(
            program
                .create_job(&device, pos, WorldSeed(u64::MAX), &token)?
                .encode(&mut encoder)?,
        );
    }
    let cancelled = CancelToken::new();
    let cancelled_job =
        program.create_job(&device, ChunkPos::default(), WorldSeed(0), &cancelled)?;
    cancelled.cancel();
    assert!(matches!(
        cancelled_job.encode(&mut encoder),
        Err(GpuTerrainError::Generation(GenerationError::Cancelled))
    ));
    assert!(matches!(
        program.create_job(&device, ChunkPos::default(), WorldSeed(0), &cancelled),
        Err(GpuTerrainError::Generation(GenerationError::Cancelled))
    ));
    let late_cancel = CancelToken::new();
    let abandoned = program
        .create_job(&device, ChunkPos::default(), WorldSeed(42), &late_cancel)?
        .encode(&mut encoder)?;
    queue.submit([encoder.finish()]);
    let mut reads: Vec<_> = reads
        .into_iter()
        .map(voxy_gpu::TerrainDispatch::begin_read)
        .collect();
    let mut abandoned = abandoned.begin_read();
    late_cancel.cancel();
    assert!(matches!(
        abandoned.try_read(),
        Err(GpuTerrainError::Generation(GenerationError::Cancelled))
    ));
    assert!(matches!(
        abandoned.try_read(),
        Err(GpuTerrainError::Compute(ComputeError::Consumed))
    ));
    drop(abandoned);
    device.poll(wgpu::PollType::wait_indefinitely())?;
    for (read, pos) in reads.iter_mut().zip(positions) {
        let actual = read.try_read()?.ok_or("mapping pending")?;
        assert_eq!(
            actual.data,
            cpu.generate(pos, WorldSeed(u64::MAX), &token)?.data
        );
        assert!(matches!(
            read.try_read(),
            Err(GpuTerrainError::Compute(ComputeError::Consumed))
        ));
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "PASS: 65536 CPU/GPU block comparisons, independent jobs, cancellation before upload/encode/publication, one-shot results"
    );
    Ok(())
}
