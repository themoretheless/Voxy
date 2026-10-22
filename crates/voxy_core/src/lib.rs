//! Dependency-free value types and deterministic primitives shared by Voxy.

mod cancel;
mod clock;
mod coordinates;
mod hash;
mod ids;

pub use cancel::CancelToken;
pub use clock::{ClockError, ClockPlan, FixedClock};
pub use coordinates::{
    AabbI64, BitMatrix32, CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, ChunkSphereIterator,
    CoordinateOverflow, Direction6, FaceMask, FloatingOrigin, InvalidLocalPos, LocalIndex,
    LocalPos, PADDED_EDGE, PADDED_VOLUME, RayGridIterator, RayVoxelStep, SpiralChunkIterator,
    SubVoxelCoord, SubVoxelPos, VoxelPos, join_voxel, split_voxel,
};
pub use hash::{
    DeterministicRng, FastHashBuilder, FastHasher, SeededFastHashBuilder, crc32c, crc32c_append,
};
pub use ids::{GenerationalId, NonMaxU32, NonMaxU64, SlotMap, TickId, WorldEpoch};
