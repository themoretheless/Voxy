//! Nonblocking terrain jobs for native command queues and browser event loops.
use crate::terrain::{GpuTerrainError, decode, prepare};
use voxy_core::{CancelToken, ChunkPos};
use voxy_render::{
    ComputeDispatch, ComputeError, ComputeJob, ComputeProgram, PendingComputeReadback,
};
use voxy_world::{BlockStateId, GeneratedChunk, GenerationError, TerrainPalette, WorldSeed};

#[derive(Debug)]
pub struct TerrainProgram {
    program: ComputeProgram,
    palette: TerrainPalette,
    water: BlockStateId,
}
impl TerrainProgram {
    /// Creates the fixed CPU-compatible procedural v1 pipeline on a caller's
    /// device. WebGL devices return Unsupported; no backend is substituted.
    /// # Errors
    /// Returns unsupported compute limits or pipeline validation errors.
    pub async fn new(
        device: &wgpu::Device,
        palette: TerrainPalette,
        water: BlockStateId,
    ) -> Result<Self, GpuTerrainError> {
        let program = ComputeProgram::new(device, include_str!("terrain.wgsl"))
            .await
            .map_err(GpuTerrainError::Compute)?;
        Ok(Self {
            program,
            palette,
            water,
        })
    }
    /// Prepares independent storage for one bounded chunk without submitting it.
    /// # Errors
    /// Rejects cancellation/coordinate overflow before upload and device limits
    /// before allocation. Use the same device that created the program.
    pub fn create_job(
        &self,
        device: &wgpu::Device,
        pos: ChunkPos,
        seed: WorldSeed,
        cancel: &CancelToken,
    ) -> Result<TerrainJob, GpuTerrainError> {
        let words = prepare(pos, seed, self.palette, self.water, cancel)
            .map_err(GpuTerrainError::Generation)?;
        let job = self
            .program
            .create_job(device, bytemuck::cast_slice(&words))
            .map_err(GpuTerrainError::Compute)?;
        Ok(TerrainJob {
            job,
            metadata: Metadata {
                pos,
                palette: self.palette,
                water: self.water,
                cancel: cancel.clone(),
            },
        })
    }
}
#[derive(Debug)]
struct Metadata {
    pos: ChunkPos,
    palette: TerrainPalette,
    water: BlockStateId,
    cancel: CancelToken,
}
#[derive(Debug)]
pub struct TerrainJob {
    job: ComputeJob,
    metadata: Metadata,
}
impl TerrainJob {
    /// Records generation and ordered readback copy. Submit the encoder before
    /// starting readback. Cancellation after upload rejects before recording.
    /// # Errors
    /// Reports cancellation or dispatch errors.
    pub fn encode(
        self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<TerrainDispatch, GpuTerrainError> {
        if self.metadata.cancel.is_cancelled() {
            return Err(GpuTerrainError::Generation(GenerationError::Cancelled));
        }
        let dispatch = self
            .job
            .encode(encoder, [16, 1, 1])
            .map_err(GpuTerrainError::Compute)?;
        Ok(TerrainDispatch {
            dispatch,
            metadata: self.metadata,
        })
    }
}
#[derive(Debug)]
pub struct TerrainDispatch {
    dispatch: ComputeDispatch,
    metadata: Metadata,
}
impl TerrainDispatch {
    /// Native callers poll the device; browser callers yield to their event loop.
    #[must_use]
    pub fn begin_read(self) -> PendingTerrain {
        PendingTerrain {
            read: self.dispatch.begin_read(),
            metadata: self.metadata,
            consumed: false,
        }
    }
}
#[derive(Debug)]
pub struct PendingTerrain {
    read: PendingComputeReadback,
    metadata: Metadata,
    consumed: bool,
}
impl PendingTerrain {
    /// Takes a validated generated chunk once its mapping callback completes.
    /// Dropping this pending job cancels readback and releases private storage.
    /// # Errors
    /// Reports mapping/format errors, cancellation before publication or Consumed
    /// after any terminal result. None means the callback has not completed yet.
    pub fn try_read(&mut self) -> Result<Option<GeneratedChunk>, GpuTerrainError> {
        if self.consumed {
            return Err(GpuTerrainError::Compute(ComputeError::Consumed));
        }
        if self.metadata.cancel.is_cancelled() {
            self.consumed = true;
            return Err(GpuTerrainError::Generation(GenerationError::Cancelled));
        }
        let bytes = match self.read.try_read() {
            Ok(None) => return Ok(None),
            Ok(Some(bytes)) => bytes,
            Err(error) => {
                self.consumed = true;
                return Err(GpuTerrainError::Compute(error));
            }
        };
        self.consumed = true;
        if bytes.len() != voxy_cuda::TERRAIN_WORD_COUNT * 4 {
            return Err(GpuTerrainError::InvalidOutput);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|bytes| u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .collect::<Vec<_>>();
        decode(
            self.metadata.pos,
            &words,
            self.metadata.palette,
            self.metadata.water,
        )
        .map(Some)
    }
}
