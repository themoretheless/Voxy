//! Dependency-free value types and deterministic primitives shared by Voxy.

mod cancel;
mod clock;
mod codec;
mod coordinates;
mod hash;
mod ids;

pub use cancel::CancelToken;
pub use clock::{ClockError, ClockPlan, FixedClock};
pub use codec::{BinaryCheckpoint, ByteReader, ByteWriter, CodecError};
pub use coordinates::{
    AabbI64, BitMatrix32, CHUNK_EDGE, CHUNK_VOLUME, ChunkPos, ChunkSphereIterator,
    CoordinateOverflow, Direction6, FaceMask, FloatingOrigin, InvalidLocalPos, LocalIndex,
    LocalPos, PADDED_EDGE, PADDED_VOLUME, QuantizedNormal16, RayGridIterator, RayVoxelStep,
    SpiralChunkIterator, SubVoxelCoord, SubVoxelPos, VoxelPos, f16_to_f32, f32_to_f16, join_voxel,
    linear_to_srgb, morton_decode_3d_u10, morton_encode_3d_u10, padded_index, split_voxel,
    srgb_to_linear, voxel_ray_dda_first_step,
};
pub use hash::{
    DeterministicRng, FastHashBuilder, FastHasher, SeededFastHashBuilder, crc32c, crc32c_append,
};
pub use ids::{GenerationalId, NonMaxU32, NonMaxU64, SlotMap, TickId, WorldEpoch};
