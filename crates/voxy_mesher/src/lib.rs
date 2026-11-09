//! Pure, deterministic chunk-neighborhood to quad-mesh transformations.

mod classify;
mod greedy;
mod mesh;
mod neighborhood;
mod types;

pub use classify::{FaceDecision, classify_directed_face};
pub use greedy::{build_mesh, build_naive_mesh};
pub use neighborhood::{HALO_OFFSETS, HaloOffset, MeshStamp, MeshingInput};
pub use types::{
    ChunkMesh, ChunkVisibilityGraph6, FaceDir, LocalAabb, MAX_DIRECTED_FACES, MeshError,
    PackedChunkMesh64, PackedQuad64, Quad, QuadDiagonal, RenderLayer,
    pack_octahedral_normal_snorm16, quad_diagonal_flip_for_ao, select_ao_quad_diagonal,
    unpack_octahedral_normal_snorm16,
};
pub use voxy_world::MaterialId;
