use crate::terrain::{GpuTerrainError, decode, prepare};
use std::sync::Mutex;
use voxy_core::{CancelToken, ChunkPos};
use voxy_cuda::{CudaCapabilities, CudaCompute, CudaError};
use voxy_world::{
    BlockStateId, ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor,
    ProceduralTerrainGenerator, TerrainPalette, WorldSeed,
};

#[derive(Debug)]
pub enum CudaTerrainError {
    Cuda(CudaError),
    Terrain(GpuTerrainError),
    Generation(GenerationError),
    Poisoned,
}
impl std::fmt::Display for CudaTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CUDA terrain error: {self:?}")
    }
}
impl std::error::Error for CudaTerrainError {}

/// Exact CUDA procedural terrain through the runtime's existing generator ABI.
/// Uses an explicit CUDA device ordinal, private buffers, cached compiled code
/// and synchronized readback. Default builds return Disabled during construction.
#[derive(Debug)]
pub struct CudaTerrainGenerator {
    compute: Mutex<CudaCompute>,
    capabilities: CudaCapabilities,
    cpu: ProceduralTerrainGenerator,
    palette: TerrainPalette,
    water: BlockStateId,
}
impl CudaTerrainGenerator {
    /// # Errors
    /// Reports disabled CUDA, invalid budgets and NVIDIA driver/device failures.
    pub fn new(
        ordinal: usize,
        palette: TerrainPalette,
        water: BlockStateId,
    ) -> Result<Self, CudaTerrainError> {
        let compute = CudaCompute::new(ordinal, voxy_cuda::TERRAIN_WORD_COUNT * 4)
            .map_err(CudaTerrainError::Cuda)?;
        let capabilities = compute.capabilities().map_err(CudaTerrainError::Cuda)?;
        Ok(Self {
            compute: Mutex::new(compute),
            capabilities,
            cpu: ProceduralTerrainGenerator::new(palette, water),
            palette,
            water,
        })
    }
    #[must_use]
    pub fn capabilities(&self) -> &CudaCapabilities {
        &self.capabilities
    }

    /// # Errors
    /// Reports cancellation/coordinate bounds and CUDA compiler/launch/readback
    /// errors without silently changing generator or compute backend.
    pub fn generate_detailed(
        &self,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<GeneratedChunk, CudaTerrainError> {
        let words = prepare(pos, seed, self.palette, self.water, cancel)
            .map_err(CudaTerrainError::Generation)?;
        let compute = self
            .compute
            .lock()
            .map_err(|_| CudaTerrainError::Poisoned)?;
        let output = compute
            .procedural_terrain(&words)
            .map_err(CudaTerrainError::Cuda)?;
        drop(compute);
        if cancel.is_cancelled() {
            return Err(CudaTerrainError::Generation(GenerationError::Cancelled));
        }
        decode(pos, &output, self.palette, self.water).map_err(CudaTerrainError::Terrain)
    }
}
impl ChunkGenerator for CudaTerrainGenerator {
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
                CudaTerrainError::Generation(error) => error,
                _ => GenerationError::BackendFailure,
            })
    }
}
