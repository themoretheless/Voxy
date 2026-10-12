//! Device-level GPU utilities shared by the renderer and the compute backends:
//! the compute error type, budgeted storage allocation, non-blocking readback
//! pools, the portable compute program and the backend options. No scene or pass code lives here,
//! so voxy_gpu can depend on it without depending on voxy_render.

mod backend;
mod compute;
mod compute_memory;
mod compute_readback;

pub use backend::*;
pub use compute::*;
pub use compute_memory::*;
pub use compute_readback::*;

/// Failure classes of portable compute: capability, budget, validation, lifecycle.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("compute error: {self:?}")]
pub enum ComputeError {
    Unsupported,
    InvalidBuffer,
    ReadbackBudget,
    MemoryBudget,
    WorkBudget,
    InvalidDispatch,
    DeviceMismatch,
    Validation(String),
    Mapping(String),
    Consumed,
}

/// Hardware indirect voxel mesh buffers directly produced by GPU compute mesher.
#[derive(Debug)]
pub struct GpuIndirectMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub indirect_buffer: wgpu::Buffer,
}
