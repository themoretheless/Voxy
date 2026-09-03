//! Dependency-free value types and deterministic primitives shared by Voxy.

mod cancel;
mod clock;
mod coordinates;
mod ids;

pub use cancel::CancelToken;
pub use clock::{ClockError, ClockPlan, FixedClock};
pub use coordinates::{
    CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, CoordinateOverflow, InvalidLocalPos, LocalIndex, LocalPos,
    VoxelPos, join_voxel, split_voxel,
};
pub use ids::{TickId, WorldEpoch};
