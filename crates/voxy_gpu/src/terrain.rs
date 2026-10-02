#[cfg(not(target_arch = "wasm32"))]
use crate::TerrainProgram;
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use voxy_core::{CHUNK_VOLUME, CancelToken, ChunkPos, LocalPos, join_voxel};
use voxy_render::ComputeError;
#[cfg(not(target_arch = "wasm32"))]
use voxy_render::{GraphicsCapabilities, GraphicsOptions};
use voxy_world::{
    BlockStateId, ChunkData, GeneratedChunk, GenerationError, PalettedBlocks, TerrainPalette,
    WorldSeed,
};

#[cfg(not(target_arch = "wasm32"))]
use voxy_world::{ChunkGenerator, GeneratorDescriptor, ProceduralTerrainGenerator};

const HEADER_WORDS: usize = 12;
const COLUMN_WORDS: usize = 26;
const COLUMN_COUNT: usize = 1024;
const BLOCK_OFFSET: usize = HEADER_WORDS + COLUMN_WORDS * COLUMN_COUNT;
const TOTAL_WORDS: usize = BLOCK_OFFSET + CHUNK_VOLUME;

#[derive(Debug)]
pub enum GpuTerrainError {
    Adapter(wgpu::RequestAdapterError),
    Device(wgpu::RequestDeviceError),
    Compute(ComputeError),
    Poll(wgpu::PollError),
    Generation(GenerationError),
    Poisoned,
    InvalidOutput,
}
impl std::fmt::Display for GpuTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU terrain error: {self:?}")
    }
}
impl std::error::Error for GpuTerrainError {}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    program: TerrainProgram,
}

/// Exact procedural-terrain v1 on a native compute device. Implements the
/// existing generator interface for worker-thread use; synchronization blocks
/// the worker, so this synchronous adapter must not run on a browser event loop.
/// The GPU owns hash/noise/biome/block computation; the host only prepares exact
/// lattice addresses and validates/palettizes returned block IDs.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct GpuTerrainGenerator {
    state: Mutex<State>,
    cpu: ProceduralTerrainGenerator,
    capabilities: GraphicsCapabilities,
}
#[cfg(not(target_arch = "wasm32"))]
impl GpuTerrainGenerator {
    /// Creates a dedicated device using the requested API and adapter policy.
    /// # Errors
    /// Reports unsupported adapters, device creation and shader validation.
    pub async fn new(
        options: GraphicsOptions,
        palette: TerrainPalette,
        water: BlockStateId,
    ) -> Result<Self, GpuTerrainError> {
        let instance = options.create_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                force_fallback_adapter: options.force_fallback_adapter,
                compatible_surface: None,
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(GpuTerrainError::Adapter)?;
        let capabilities = GraphicsCapabilities::discover(&adapter);
        if !capabilities.compute_shaders {
            return Err(GpuTerrainError::Compute(ComputeError::Unsupported));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(GpuTerrainError::Device)?;
        let program = TerrainProgram::new(&device, palette, water).await?;
        Ok(Self {
            state: Mutex::new(State {
                device,
                queue,
                program,
            }),
            cpu: ProceduralTerrainGenerator::new(palette, water),
            capabilities,
        })
    }

    #[must_use]
    pub fn capabilities(&self) -> &GraphicsCapabilities {
        &self.capabilities
    }

    /// Generates one chunk with exact CPU-compatible output and detailed errors.
    /// Cancellation is checked before allocation and again before publication.
    /// # Errors
    /// Reports cancellation, coordinate overflow, device/readback failures and
    /// malformed output. There is no implicit switch to CPU generation.
    pub fn generate_detailed(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GpuTerrainError> {
        let state = self.state.lock().map_err(|_| GpuTerrainError::Poisoned)?;
        let job = state.program.create_job(&state.device, pos, seed, cancel)?;
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = job.encode(&mut encoder)?;
        let submission = state.queue.submit([encoder.finish()]);
        let mut read = dispatch.begin_read();
        state
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(30)),
            })
            .map_err(GpuTerrainError::Poll)?;
        read.try_read()?.ok_or(GpuTerrainError::InvalidOutput)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl ChunkGenerator for GpuTerrainGenerator {
    fn descriptor(&self) -> GeneratorDescriptor {
        self.cpu.descriptor()
    }
    fn generate(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, GenerationError> {
        self.generate_detailed(pos, seed, cancel)
            .map_err(|error| match error {
                GpuTerrainError::Generation(error) => error,
                _ => GenerationError::BackendFailure,
            })
    }
}

fn split(value: u64) -> [u32; 2] {
    let bytes = value.to_le_bytes();
    [
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    ]
}
pub(crate) fn prepare(
    pos: ChunkPos,
    seed: WorldSeed,
    palette: TerrainPalette,
    water: BlockStateId,
    cancel: &CancelToken,
) -> Result<Vec<u32>, GenerationError> {
    if cancel.is_cancelled() {
        return Err(GenerationError::Cancelled);
    }
    let origin = join_voxel(
        pos,
        LocalPos::new(0, 0, 0).map_err(|_| GenerationError::CoordinateOverflow)?,
    )
    .map_err(|_| GenerationError::CoordinateOverflow)?;
    // Validate the whole chunk extent before addressing, including extreme y.
    join_voxel(
        pos,
        LocalPos::new(31, 31, 31).map_err(|_| GenerationError::CoordinateOverflow)?,
    )
    .map_err(|_| GenerationError::CoordinateOverflow)?;
    let mut words = vec![0; TOTAL_WORDS];
    words[..2].copy_from_slice(&split(seed.0));
    // Heights are 0..31. Clamping an entire below/above chunk is semantically
    // exact and avoids i64 truncation in the material pass.
    let y =
        i32::try_from(origin.y.clamp(-64, 32)).map_err(|_| GenerationError::CoordinateOverflow)?;
    words[2] = u32::from_ne_bytes(y.to_ne_bytes());
    for (slot, block) in words[4..9].iter_mut().zip([
        palette.air,
        palette.surface,
        palette.soil,
        palette.stone,
        water,
    ]) {
        *slot = block.get();
    }
    for z in 0..32_u8 {
        if cancel.is_cancelled() {
            return Err(GenerationError::Cancelled);
        }
        for x in 0..32_u8 {
            let column = usize::from(x) + 32 * usize::from(z);
            let wx = origin.x + i64::from(x);
            let wz = origin.z + i64::from(z);
            for (scale, period) in [128, 192, 32, 8].into_iter().enumerate() {
                let offset = HEADER_WORDS + column * COLUMN_WORDS + scale * 6;
                let cx = u64::from_ne_bytes(wx.div_euclid(period).to_ne_bytes());
                let cz = u64::from_ne_bytes(wz.div_euclid(period).to_ne_bytes());
                words[offset..offset + 2].copy_from_slice(&split(cx));
                words[offset + 2..offset + 4].copy_from_slice(&split(cz));
                words[offset + 4] = u32::try_from(wx.rem_euclid(period))
                    .map_err(|_| GenerationError::CoordinateOverflow)?;
                words[offset + 5] = u32::try_from(wz.rem_euclid(period))
                    .map_err(|_| GenerationError::CoordinateOverflow)?;
            }
        }
    }
    Ok(words)
}

pub(crate) fn decode(
    pos: ChunkPos,
    words: &[u32],
    palette: TerrainPalette,
    water: BlockStateId,
) -> Result<GeneratedChunk, GpuTerrainError> {
    if words.len() != TOTAL_WORDS {
        return Err(GpuTerrainError::InvalidOutput);
    }
    let allowed = [
        palette.air,
        palette.surface,
        palette.soil,
        palette.stone,
        water,
    ];
    let dense = words[BLOCK_OFFSET..]
        .iter()
        .map(|id| {
            allowed
                .iter()
                .find(|block| block.get() == *id)
                .copied()
                .ok_or(GpuTerrainError::InvalidOutput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GeneratedChunk {
        pos,
        data: ChunkData {
            blocks: PalettedBlocks::from_dense(dense)
                .map_err(|_| GpuTerrainError::InvalidOutput)?,
            block_data: BTreeMap::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_world::{
        BlockDef, BlockRegistry, CollisionShape, MaterialId, Occlusion, RenderKind, ResourceKey,
    };

    fn palette() -> (TerrainPalette, BlockStateId) {
        let definitions = ["air", "surface", "soil", "stone", "water"]
            .into_iter()
            .enumerate()
            .map(|(index, key)| BlockDef {
                key: ResourceKey::parse(format!("voxy:{key}")).unwrap(),
                render: if index == 0 {
                    RenderKind::Invisible
                } else {
                    RenderKind::Opaque
                },
                occlusion: if index == 0 {
                    Occlusion::None
                } else {
                    Occlusion::FullCube
                },
                collision: if index == 0 {
                    CollisionShape::Empty
                } else {
                    CollisionShape::FullCube
                },
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            })
            .collect();
        let registry = BlockRegistry::new(definitions).unwrap();
        let find = |name| {
            registry
                .find(&ResourceKey::parse(format!("voxy:{name}")).unwrap())
                .unwrap()
        };
        (
            TerrainPalette {
                air: find("air"),
                surface: find("surface"),
                soil: find("soil"),
                stone: find("stone"),
            },
            find("water"),
        )
    }

    #[test]
    fn request_bounds_and_cancel_precede_allocation() {
        let (palette, water) = palette();
        let cancelled = CancelToken::new();
        cancelled.cancel();
        assert_eq!(
            prepare(
                ChunkPos::default(),
                WorldSeed(0),
                palette,
                water,
                &cancelled
            ),
            Err(GenerationError::Cancelled)
        );
        assert_eq!(
            prepare(
                ChunkPos {
                    x: i64::MAX,
                    y: 0,
                    z: 0
                },
                WorldSeed(0),
                palette,
                water,
                &CancelToken::new()
            ),
            Err(GenerationError::CoordinateOverflow)
        );
        let words = prepare(
            ChunkPos {
                x: i64::MIN / 32,
                y: i64::MAX / 32,
                z: i64::MAX / 32,
            },
            WorldSeed(u64::MAX),
            palette,
            water,
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(words.len(), voxy_cuda::TERRAIN_WORD_COUNT);
        assert_eq!(words[..2], [u32::MAX; 2]);
        assert_eq!(words[2], 32);
        assert!(decode(ChunkPos::default(), &words[..12], palette, water).is_err());
    }

    #[test]
    #[ignore = "requires a native clang++ compiler; verifies CUDA source arithmetic, not NVIDIA execution"]
    fn cuda_source_host_parity() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let directory = root
            .join("target")
            .join(format!("cuda-terrain-host-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let binary = directory.join("terrain-host");
        let compile = std::process::Command::new("clang++")
            .args(["-std=c++17", "-Wall", "-Wextra", "-Werror", "-O2"])
            .arg(root.join("tools/cuda/terrain_host.cpp"))
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let (palette, water) = palette();
        let cpu = ProceduralTerrainGenerator::new(palette, water);
        let token = CancelToken::new();
        let input_path = directory.join("input.bin");
        let output_path = directory.join("output.bin");
        let mut chunks = 0;
        for seed in [0, 42, 43, 1 << 63, u64::MAX] {
            for (x, z) in [
                (-7, 4),
                (-6, -6),
                (-1, 1),
                (0, 0),
                (5, 6),
                (i64::MIN / 32, i64::MAX / 32),
            ] {
                for y in [-2, -1, 0, 1, i64::MIN / 32, i64::MAX / 32] {
                    let pos = ChunkPos { x, y, z };
                    let words = prepare(pos, WorldSeed(seed), palette, water, &token).unwrap();
                    let input: Vec<u8> = words.iter().flat_map(|word| word.to_ne_bytes()).collect();
                    std::fs::write(&input_path, input).unwrap();
                    let result = std::process::Command::new(&binary)
                        .arg(&input_path)
                        .arg(&output_path)
                        .output()
                        .unwrap();
                    assert!(result.status.success());
                    let bytes = std::fs::read(&output_path).unwrap();
                    assert_eq!(bytes.len(), TOTAL_WORDS * 4);
                    let output: Vec<u32> = bytes
                        .chunks_exact(4)
                        .map(|bytes| u32::from_ne_bytes(bytes.try_into().unwrap()))
                        .collect();
                    let actual = decode(pos, &output, palette, water).unwrap();
                    assert_eq!(
                        actual.data,
                        cpu.generate(pos, WorldSeed(seed), &token).unwrap().data,
                        "{pos:?} {seed}"
                    );
                    chunks += 1;
                }
            }
        }
        println!(
            "CUDA source host arithmetic parity: {chunks} chunks, {} exact block comparisons; NVIDIA execution remains unverified",
            chunks * CHUNK_VOLUME
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
