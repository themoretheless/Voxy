//! Canonical, renderer-independent voxel world state.

mod block;
mod chunk;
mod collision;
mod destruction;
mod generator;
mod raycast;
mod world;

pub use block::{
    BlockDef, BlockRegistry, BlockStateId, CollisionShape, InterfaceGroupId, MaterialId, Occlusion,
    RegistryError, RenderKind, ResourceKey,
};
pub use chunk::{ChunkData, ChunkError, ChunkRevision, ChunkSnapshot, PalettedBlocks};
pub use collision::{
    AnchoredAabb, SweepConfig, SweepError, SweepObstacle, SweepResult, sweep_aabb,
};
pub use destruction::{DestructionError, DestructionPlan, Explosion, plan_explosion};
pub use generator::{
    ChunkGenerator, GeneratedChunk, GenerationError, GeneratorDescriptor, SimpleTerrainGenerator,
    TerrainPalette, WorldSeed,
};
pub use raycast::{RayOrigin, RaycastConfig, RaycastError, RaycastResult, VoxelHit, raycast};
pub use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, LocalIndex, LocalPos, VoxelPos};
pub use world::{
    ChunkDelta, CommitError, CommitId, CommitReceipt, DirtyBounds, DurabilityTicket, EditSource,
    EditTxn, InverseEdit, Sample, UnavailableReason, VoxelView, VoxelWrite, World, WorldLimits,
};
