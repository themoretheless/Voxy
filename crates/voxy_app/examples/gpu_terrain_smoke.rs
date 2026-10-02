//! Native world/meshing/lighting integration for an explicit accelerated generator.
use voxy_render::{GraphicsBackend, GraphicsOptions};
use voxy_world::GenerationError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selection = std::env::args().nth(1).unwrap_or_else(|| "auto".into());
    let backend = match selection.as_str() {
        "auto" | "cuda" => GraphicsBackend::Auto,
        "metal" => GraphicsBackend::Metal,
        "vulkan" => GraphicsBackend::Vulkan,
        "dx12" => GraphicsBackend::DirectX12,
        value => return Err(format!("unknown terrain backend: {value}").into()),
    };
    let accelerated_scene = voxy_runtime::build_generated_scene(42, 1, |palette, water| {
        if selection == "cuda" {
            let generator =
                voxy_gpu::CudaTerrainGenerator::new(0, palette, water).map_err(|error| {
                    eprintln!("{error}");
                    GenerationError::BackendFailure
                })?;
            println!("Terrain CUDA: {:?}", generator.capabilities());
            Ok(Box::new(generator))
        } else {
            let generator = pollster::block_on(voxy_gpu::GpuTerrainGenerator::new(
                GraphicsOptions {
                    backend,
                    ..GraphicsOptions::default()
                },
                palette,
                water,
            ))
            .map_err(|error| {
                eprintln!("{error}");
                GenerationError::BackendFailure
            })?;
            println!("Terrain GPU: {:?}", generator.capabilities().adapter);
            Ok(Box::new(generator))
        }
    })?;
    let cpu_scene = voxy_runtime::build_procedural_scene(42, 1)?;
    assert_eq!(accelerated_scene.anchor, cpu_scene.anchor);
    assert_eq!(accelerated_scene.chunks, cpu_scene.chunks);
    println!(
        "PASS: {} rendered chunks and their generated halos, exact CPU mesh/light parity",
        accelerated_scene.chunks.len()
    );
    Ok(())
}
