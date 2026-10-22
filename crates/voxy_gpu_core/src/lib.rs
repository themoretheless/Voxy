//! Device-level GPU utilities shared by the renderer and the compute backends:
//! the compute error type, budgeted storage allocation, non-blocking readback
//! pools, the portable compute program and the backend options. No scene or pass code lives here,
//! so voxy_gpu can depend on it without depending on voxy_render.

mod backend;
mod compute;
mod compute_memory;
mod compute_readback;
mod gravity;
mod shared_upload;

pub use backend::*;
pub use compute::*;
pub use compute_memory::*;
pub use compute_readback::*;
pub use gravity::*;
pub use shared_upload::{SharedUploadCopy, SharedUploadRing, SharedUploadSnapshot};

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

/// Deterministic aligned bump/ring suballocator for per-frame uniform and storage
/// slices within a persistent GPU buffer capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RingSuballocator {
    capacity: u64,
    cursor: u64,
}

impl RingSuballocator {
    /// Creates a suballocator over `[0, capacity)`.
    #[must_use]
    pub const fn new(capacity: u64) -> Self {
        Self {
            capacity,
            cursor: 0,
        }
    }

    #[must_use]
    pub const fn capacity(&self) -> u64 {
        self.capacity
    }

    #[must_use]
    pub const fn used_bytes(&self) -> u64 {
        self.cursor
    }

    #[must_use]
    pub const fn remaining_bytes(&self) -> u64 {
        self.capacity.saturating_sub(self.cursor)
    }

    /// Resets the bump cursor at frame boundary after previous frame submissions retire.
    pub const fn reset(&mut self) {
        self.cursor = 0;
    }

    /// Allocates `size` bytes aligned up to `alignment` (must be a power of two).
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::InvalidBuffer`] if `size == 0` or `alignment` is not a power of two,
    /// or [`ComputeError::MemoryBudget`] if the aligned range exceeds `capacity`.
    pub const fn allocate(&mut self, size: u64, alignment: u64) -> Result<u64, ComputeError> {
        if size == 0 || !alignment.is_power_of_two() {
            return Err(ComputeError::InvalidBuffer);
        }
        let mask = alignment - 1;
        let Some(sum) = self.cursor.checked_add(mask) else {
            return Err(ComputeError::MemoryBudget);
        };
        let aligned = sum & !mask;
        let Some(end) = aligned.checked_add(size) else {
            return Err(ComputeError::MemoryBudget);
        };
        if end > self.capacity {
            return Err(ComputeError::MemoryBudget);
        }
        self.cursor = end;
        Ok(aligned)
    }
}

#[cfg(test)]
mod ring_suballocator_tests {
    use super::*;

    #[test]
    fn ring_suballocator_aligns_offsets_and_enforces_capacity() {
        let mut ring = RingSuballocator::new(1024);
        assert_eq!(ring.allocate(100, 256), Ok(0));
        assert_eq!(ring.allocate(128, 256), Ok(256));
        assert_eq!(ring.allocate(512, 256), Ok(512));
        assert_eq!(ring.remaining_bytes(), 0);
        assert_eq!(ring.allocate(16, 16), Err(ComputeError::MemoryBudget));
        ring.reset();
        assert_eq!(ring.used_bytes(), 0);
        assert_eq!(ring.allocate(64, 64), Ok(0));
    }
}
