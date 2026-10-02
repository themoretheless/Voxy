//! Canonical, renderer-independent voxel world state.

mod block;
mod chunk;
mod generator;
mod terrain;
mod world;
pub use terrain::{Biome, ProceduralTerrainGenerator, TerrainColumn};

pub use block::{
    BlockDef, BlockRegistry, BlockStateId, CollisionShape, InterfaceGroupId, MaterialId, Occlusion,
    RegistryError, RenderKind, ResourceKey,
};
pub use chunk::{ChunkData, ChunkError, ChunkRevision, ChunkSnapshot, PalettedBlocks};
pub use generator::{
    ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor, SimpleTerrainGenerator,
    TerrainPalette, WorldSeed,
};
pub use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, LocalIndex, LocalPos, VoxelPos};
pub use world::{
    ChunkDelta, CommitError, CommitId, CommitReceipt, DirtyBounds, DurabilityTicket, EditSource,
    EditTxn, InverseEdit, Sample, UnavailableReason, VoxelView, VoxelWrite, World, WorldLimits,
};
