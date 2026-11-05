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

/// Standard 20-byte WebGPU `DrawIndexedIndirect` command payload (`[u32; 5]`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    /// Serializes the indirect draw command into 5 little-endian `u32` words (20 bytes).
    #[must_use]
    pub const fn to_words(self) -> [u32; 5] {
        [
            self.index_count,
            self.instance_count,
            self.first_index,
            self.base_vertex as u32,
            self.first_instance,
        ]
    }
}

/// Hierarchical Z-buffer (Hi-Z) conservative depth pyramid for two-phase occlusion culling.
///
/// Stores the farthest depth (`max(z)` for standard `[0, 1]` depth) across a power-of-two
///mip chain so screen-space bounding boxes can be tested in $O(1)$ at the mip level whose
/// texel footprint covers the projected box.
#[derive(Clone, Debug, PartialEq)]
pub struct HiZPyramid {
    base_width: u32,
    base_height: u32,
    mips: Vec<Vec<f32>>,
}

impl HiZPyramid {
    /// Builds a conservative `max`-depth mip pyramid from base level `depth` (`width * height`).
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::InvalidBuffer`] if dimensions are zero or `depth.len()` does not match.
    pub fn from_depth_buffer(
        width: u32,
        height: u32,
        depth: &[f32],
    ) -> Result<Self, ComputeError> {
        if width == 0 || height == 0 || depth.len() != (width as usize) * (height as usize) {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut mips = Vec::new();
        mips.push(depth.to_vec());
        let mut w = width;
        let mut h = height;
        while w > 1 || h > 1 {
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);
            let prev = mips.last().unwrap();
            let mut next = vec![0.0_f32; (next_w * next_h) as usize];
            for ny in 0..next_h {
                for nx in 0..next_w {
                    let x0 = (nx * 2).min(w - 1);
                    let y0 = (ny * 2).min(h - 1);
                    let x1 = (nx * 2 + 1).min(w - 1);
                    let y1 = (ny * 2 + 1).min(h - 1);
                    let d00 = prev[(y0 * w + x0) as usize];
                    let d10 = prev[(y0 * w + x1) as usize];
                    let d01 = prev[(y1 * w + x0) as usize];
                    let d11 = prev[(y1 * w + x1) as usize];
                    next[(ny * next_w + nx) as usize] = d00.max(d10).max(d01).max(d11);
                }
            }
            mips.push(next);
            w = next_w;
            h = next_h;
        }
        Ok(Self {
            base_width: width,
            base_height: height,
            mips,
        })
    }

    #[must_use]
    pub fn mip_count(&self) -> usize {
        self.mips.len()
    }

    /// Returns `true` when an axis-aligned normalized screen-space box `min_uv..=max_uv` (`[0, 1]^2`)
    /// with nearest depth `min_depth` is conservatively occluded (`min_depth > occluder_max_z`).
    #[must_use]
    pub fn is_occluded(&self, min_uv: [f32; 2], max_uv: [f32; 2], min_depth: f32) -> bool {
        let u0 = min_uv[0].clamp(0.0, 1.0);
        let v0 = min_uv[1].clamp(0.0, 1.0);
        let u1 = max_uv[0].clamp(0.0, 1.0);
        let v1 = max_uv[1].clamp(0.0, 1.0);
        if u0 > u1 || v0 > v1 {
            return false;
        }
        let footprint_x = ((u1 - u0) * self.base_width as f32).max(1.0);
        let footprint_y = ((v1 - v0) * self.base_height as f32).max(1.0);
        let max_footprint = footprint_x.max(footprint_y);
        let level = (max_footprint.log2().ceil().max(0.0) as usize)
            .min(self.mips.len().saturating_sub(1));
        let mip_w = (self.base_width >> level).max(1);
        let mip_h = (self.base_height >> level).max(1);
        let x0 = ((u0 * mip_w as f32).floor() as u32).min(mip_w - 1);
        let y0 = ((v0 * mip_h as f32).floor() as u32).min(mip_h - 1);
        let x1 = ((u1 * mip_w as f32).floor() as u32).min(mip_w - 1);
        let y1 = ((v1 * mip_h as f32).floor() as u32).min(mip_h - 1);
        let mip = &self.mips[level];
        let mut max_occluder = 0.0_f32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                max_occluder = max_occluder.max(mip[(y * mip_w + x) as usize]);
            }
        }
        min_depth > max_occluder
    }
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

/// Computes a validated 3D compute workgroup dispatch grid `[gx, gy, gz]` for `threads_3d`
/// elements using `workgroup_size_3d` threads per workgroup, rejecting zero sizes or any axis
/// exceeding `max_workgroups_per_dim`.
///
/// # Errors
///
/// Returns [`ComputeError::InvalidBuffer`] if any thread or workgroup dimension is `0`, or
/// [`ComputeError::MemoryBudget`] if any workgroup count exceeds `max_workgroups_per_dim`.
pub const fn checked_dispatch_3d(
    threads_3d: [u32; 3],
    workgroup_size_3d: [u32; 3],
    max_workgroups_per_dim: u32,
) -> Result<[u32; 3], ComputeError> {
    let mut out = [0_u32; 3];
    let mut i = 0;
    while i < 3 {
        if threads_3d[i] == 0 || workgroup_size_3d[i] == 0 {
            return Err(ComputeError::InvalidBuffer);
        }
        let groups = threads_3d[i].div_ceil(workgroup_size_3d[i]);
        if groups == 0 || groups > max_workgroups_per_dim {
            return Err(ComputeError::MemoryBudget);
        }
        out[i] = groups;
        i += 1;
    }
    Ok(out)
}

/// Greedy interval-graph coloring planner (`TransientBufferLifetimes`) that assigns transient
/// render/compute buffers with non-overlapping `[first_pass, last_pass]` lifetimes to shared
/// physical memory slots to reduce peak VRAM consumption.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransientBufferLifetimes {
    intervals: Vec<(u32, u32)>,
}

impl TransientBufferLifetimes {
    /// Creates an empty transient lifetime table.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// Registers a transient resource live from `first_pass` through `last_pass` (inclusive).
    #[must_use]
    pub fn with_buffer(mut self, first_pass: u32, last_pass: u32) -> Self {
        let lo = if first_pass <= last_pass {
            first_pass
        } else {
            last_pass
        };
        let hi = if first_pass <= last_pass {
            last_pass
        } else {
            first_pass
        };
        self.intervals.push((lo, hi));
        self
    }

    /// Assigns each registered transient buffer a `0..slot_count` physical slot index such that
    /// no two buffers with overlapping pass intervals share a slot. Returns `(slot_for_buffer, slot_count)`.
    #[must_use]
    pub fn assign_aliased_slots(&self) -> (Vec<usize>, usize) {
        let n = self.intervals.len();
        if n == 0 {
            return (Vec::new(), 0);
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&idx| self.intervals[idx]);
        let mut slot_last_pass: Vec<u32> = Vec::new();
        let mut assigned = vec![0_usize; n];
        for idx in order {
            let (first, last) = self.intervals[idx];
            if let Some((slot, end)) = slot_last_pass
                .iter_mut()
                .enumerate()
                .find(|(_, end)| **end < first)
            {
                *end = last;
                assigned[idx] = slot;
            } else {
                let slot = slot_last_pass.len();
                slot_last_pass.push(last);
                assigned[idx] = slot;
            }
        }
        (assigned, slot_last_pass.len())
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

    #[test]
    fn hiz_pyramid_culls_occluded_boxes_conservatively() {
        let mut depth = vec![0.4_f32; 8 * 8];
        depth[0] = 0.9; // One far pixel in top-left quadrant
        let hiz = HiZPyramid::from_depth_buffer(8, 8, &depth).unwrap();
        assert_eq!(hiz.mip_count(), 4);
        // Bottom-right quadrant has max depth 0.4 -> box at depth 0.5 is occluded.
        assert!(hiz.is_occluded([0.6, 0.6], [0.9, 0.9], 0.5));
        // Top-left quadrant contains 0.9 -> box at depth 0.5 is NOT occluded.
        assert!(!hiz.is_occluded([0.0, 0.0], [0.4, 0.4], 0.5));
        let args = DrawIndexedIndirectArgs {
            index_count: 36,
            instance_count: 1,
            first_index: 0,
            base_vertex: -4,
            first_instance: 2,
        };
        assert_eq!(args.to_words()[0], 36);
        assert_eq!(args.to_words()[3] as i32, -4);

        assert_eq!(
            checked_dispatch_3d([65, 32, 8], [64, 8, 8], 65535),
            Ok([2, 4, 1])
        );
        assert_eq!(
            checked_dispatch_3d([1000, 1, 1], [8, 1, 1], 64),
            Err(ComputeError::MemoryBudget)
        );

        let lifetimes = TransientBufferLifetimes::new()
            .with_buffer(0, 2)
            .with_buffer(1, 4)
            .with_buffer(3, 5);
        let (slots, count) = lifetimes.assign_aliased_slots();
        assert_eq!(count, 2);
        assert_eq!(slots[0], slots[2]);
        assert_ne!(slots[0], slots[1]);
    }
}
