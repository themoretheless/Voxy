//! Exact GPU/CPU procedural world parity, including signed i64 extremes.
mod support;
use voxy_core::{CancelToken, ChunkPos};
use voxy_gpu::{CudaTerrainGenerator, GpuTerrainGenerator};
use voxy_render::{GraphicsBackend, GraphicsOptions};
use voxy_world::{ChunkGenerator, GenerationError, ProceduralTerrainGenerator, WorldSeed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (selection, ordinal) = selection()?;
    let backend = match selection.as_deref() {
        None | Some("auto" | "cuda") => GraphicsBackend::Auto,
        Some("metal") => GraphicsBackend::Metal,
        Some("vulkan") => GraphicsBackend::Vulkan,
        Some("dx12") => GraphicsBackend::DirectX12,
        Some("gl") => GraphicsBackend::OpenGl,
        Some(value) => return Err(format!("unsupported compute backend {value}").into()),
    };
    let (palette, water) = support::test_palette()?;
    let gpu: Box<dyn ChunkGenerator> = if selection.as_deref() == Some("cuda") {
        let generator = CudaTerrainGenerator::new(ordinal, palette, water)?;
        println!("Terrain CUDA: {:?}", generator.capabilities());
        Box::new(generator)
    } else {
        let generator = pollster::block_on(GpuTerrainGenerator::new(
            GraphicsOptions {
                backend,
                ..GraphicsOptions::default()
            },
            palette,
            water,
        ))?;
        println!("Terrain GPU: {:?}", generator.capabilities().adapter);
        Box::new(generator)
    };
    let cpu = ProceduralTerrainGenerator::new(palette, water);
    assert_eq!(gpu.descriptor(), cpu.descriptor());
    let token = CancelToken::new();
    let mut chunks = 0;
    for seed in [0, 42, 43, 1 << 63, u64::MAX] {
        for (x, z) in [
            -7,
            -6,
            -5,
            -4,
            -1,
            0,
            1,
            3,
            4,
            5,
            6,
            7,
            i64::MIN / 32,
            i64::MAX / 32,
        ]
        .into_iter()
        .map(|x| (x, x))
        .chain([
            (-7, 4),
            (0, -1),
            (5, 6),
            (i64::MAX / 32, i64::MIN / 32),
            (i64::MIN / 32, 0),
            (0, i64::MAX / 32),
        ]) {
            for y in [-2, -1, 0, 1, 2] {
                let pos = ChunkPos { x, y, z };
                let actual = gpu.generate(pos, WorldSeed(seed), &token)?;
                let expected = cpu.generate(pos, WorldSeed(seed), &token)?;
                assert_eq!(actual.pos, expected.pos);
                assert_eq!(actual.data, expected.data, "chunk {pos:?}, seed {seed}");
                chunks += 1;
            }
        }
    }
    for y in [i64::MIN / 32, i64::MAX / 32] {
        let pos = ChunkPos { x: 0, y, z: 0 };
        assert_eq!(
            gpu.generate(pos, WorldSeed(42), &token)?.data,
            cpu.generate(pos, WorldSeed(42), &token)?.data
        );
        chunks += 1;
    }
    let cancelled = CancelToken::new();
    cancelled.cancel();
    assert!(matches!(
        gpu.generate(ChunkPos::default(), WorldSeed(42), &cancelled),
        Err(GenerationError::Cancelled)
    ));
    assert!(matches!(
        gpu.generate(
            ChunkPos {
                x: i64::MAX,
                y: 0,
                z: 0
            },
            WorldSeed(42),
            &token
        ),
        Err(GenerationError::CoordinateOverflow)
    ));
    println!(
        "PASS: {chunks} chunks, {} exact block comparisons, descriptor/seed/i64 bounds/cancellation parity",
        chunks * voxy_core::CHUNK_VOLUME
    );
    Ok(())
}

fn selection() -> Result<(Option<String>, usize), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let selection = args.next();
    let ordinal = if selection.as_deref() == Some("cuda") {
        args.next().map_or(Ok(0), |value| value.parse::<usize>())?
    } else {
        0
    };
    if args.next().is_some() {
        return Err("terrain_smoke [auto|metal|vulkan|dx12|gl|cuda [ordinal]]".into());
    }
    Ok((selection, ordinal))
}
