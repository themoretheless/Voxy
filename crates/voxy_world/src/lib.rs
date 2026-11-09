//! Canonical, renderer-independent voxel world state.

mod block;
mod chunk;
mod generator;
mod terrain;
mod world;
pub use terrain::{
    Biome, ProceduralTerrainGenerator, TerrainColumn, bilinear_climate_blend,
    orographic_precipitation_multiplier, stream_power_fluvial_incision_rate,
};

pub use block::{
    BlockDef, BlockRegistry, BlockStateId, CollisionShape, InterfaceGroupId, MaterialId, Occlusion,
    RegistryError, RenderKind, ResourceKey,
};
pub use chunk::{
    ChunkData, ChunkError, ChunkRevision, ChunkSnapshot, PalettedBlocks, SparseChunkOctreeMask,
    chunk_shannon_entropy_bits_per_voxel, optimal_palette_bit_width,
};
pub use generator::{
    ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor, SimpleTerrainGenerator,
    TerrainPalette, WorldSeed,
};
pub use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, LocalIndex, LocalPos, VoxelPos};
pub use world::{
    ChunkDelta, CommitError, CommitId, CommitReceipt, DirtyBounds, DurabilityTicket, EditSource,
    EditTxn, InverseEdit, Sample, UnavailableReason, VoxelView, VoxelWrite, World, WorldLimits,
};

mod vegetation;
pub use vegetation::{
    VegetationGenerator, VegetationPalette, can_plant_on_slope, self_thinning_canopy_competition,
};
