#[cfg(any(feature = "cuda", test))]
use crate::CudaError;
#[cfg(any(feature = "cuda", test))]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[cfg(any(feature = "cuda", test))]
#[derive(Debug)]
pub(crate) struct AllocationBudget {
    limit: usize,
    used: AtomicUsize,
}
#[cfg(any(feature = "cuda", test))]
impl AllocationBudget {
    pub(crate) fn used_bytes(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
    pub(crate) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
        })
    }
    pub(crate) fn reserve(self: &Arc<Self>, bytes: usize) -> Result<Reservation, CudaError> {
        if bytes == 0 {
            return Err(CudaError::BufferLimit);
        }
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|&total| total <= self.limit)
            })
            .map_err(|_| CudaError::BufferLimit)?;
        Ok(Reservation {
            budget: Arc::clone(self),
            bytes,
        })
    }
}
#[cfg(any(feature = "cuda", test))]
#[derive(Debug)]
pub(crate) struct Reservation {
    budget: Arc<AllocationBudget>,
    bytes: usize,
}
#[cfg(any(feature = "cuda", test))]
impl Reservation {
    #[cfg(feature = "cuda")]
    pub(crate) fn release<T>(
        self,
        storage: T,
        context: &Arc<cudarc::driver::CudaContext>,
    ) -> Result<(), CudaError> {
        if let Err(error) = context
            .bind_to_thread()
            .and_then(|()| context.synchronize())
        {
            std::mem::forget(storage);
            std::mem::forget(self);
            return Err(CudaError::Driver(error));
        }
        drop(storage);
        if let Err(error) = context.synchronize().and_then(|()| context.check_err()) {
            std::mem::forget(self);
            return Err(CudaError::Driver(error));
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Self, CudaError> {
        self.budget.reserve(bytes)
    }
}
#[cfg(any(feature = "cuda", test))]
impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservations_share_capacity_and_release_on_failure() {
        let budget = AllocationBudget::new(12);
        let first = budget.reserve(8).unwrap();
        assert!(matches!(budget.reserve(5), Err(CudaError::BufferLimit)));
        let second = first.reserve(4).unwrap();
        assert!(matches!(budget.reserve(1), Err(CudaError::BufferLimit)));
        drop(first);
        let replacement = budget.reserve(8).unwrap();
        drop(second);
        drop(replacement);
        assert_eq!(budget.used_bytes(), 0);
        assert!(budget.reserve(12).is_ok());
        assert!(matches!(
            budget.reserve(usize::MAX),
            Err(CudaError::BufferLimit)
        ));
    }
    #[test]
    fn retained_reservation_keeps_budget_after_owner_drop() {
        let budget = AllocationBudget::new(12);
        let first = budget.reserve(8).unwrap();
        drop(budget);
        let second = first.reserve(4).unwrap();
        assert!(matches!(second.reserve(1), Err(CudaError::BufferLimit)));
        drop(first);
        assert!(second.reserve(8).is_ok());
    }
    #[test]
    fn concurrent_reservations_cannot_exceed_capacity() {
        let budget = AllocationBudget::new(8);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let budget = Arc::clone(&budget);
                    let barrier = Arc::clone(&barrier);
                    scope.spawn(move || {
                        let reservation = budget.reserve(8);
                        barrier.wait();
                        barrier.wait();
                        reservation.is_ok()
                    })
                })
                .collect();
            barrier.wait();
            assert_eq!(budget.used_bytes(), 8);
            barrier.wait();
            assert_eq!(
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap())
                    .filter(|&accepted| accepted)
                    .count(),
                1
            );
        });
        assert_eq!(budget.used_bytes(), 0);
    }
    #[test]
    fn cuda_sm_occupancy_warps_respects_register_and_shared_memory_caps() {
        let (warps, occ) = cuda_sm_occupancy_warps(256, 32, 8192, 64, 65536, 65536).unwrap();
        assert_eq!(warps, 64);
        assert!((occ - 1.0).abs() < 1e-6);
        let (reg_limited, occ2) = cuda_sm_occupancy_warps(256, 64, 4096, 64, 65536, 65536).unwrap();
        assert_eq!(reg_limited, 32);
        assert!((occ2 - 0.5).abs() < 1e-6);
        assert_eq!(cuda_sm_occupancy_warps(0, 32, 0, 64, 65536, 65536), None);
    }
}

/// Computes active warps per Streaming Multiprocessor `(active_warps, occupancy_01)` given
/// block configuration (`threads_per_block`, `regs_per_thread`, `shared_mem_per_block`) and SM hardware caps.
#[must_use]
pub fn cuda_sm_occupancy_warps(
    threads_per_block: u32,
    regs_per_thread: u32,
    shared_mem_per_block: u32,
    max_warps_per_sm: u32,
    max_regs_per_sm: u32,
    max_shared_per_sm: u32,
) -> Option<(u32, f32)> {
    if threads_per_block == 0
        || threads_per_block > 1024
        || max_warps_per_sm == 0
        || max_regs_per_sm == 0
    {
        return None;
    }
    let warps_per_block = threads_per_block.div_ceil(32);
    if warps_per_block > max_warps_per_sm {
        return Some((0, 0.0));
    }
    let mut max_blocks = max_warps_per_sm / warps_per_block;
    if regs_per_thread > 0 {
        let regs_per_block = warps_per_block
            .saturating_mul(32)
            .saturating_mul(regs_per_thread);
        if regs_per_block > max_regs_per_sm {
            return Some((0, 0.0));
        }
        max_blocks = max_blocks.min(max_regs_per_sm / regs_per_block);
    }
    if shared_mem_per_block > 0 {
        if shared_mem_per_block > max_shared_per_sm {
            return Some((0, 0.0));
        }
        max_blocks = max_blocks.min(max_shared_per_sm / shared_mem_per_block);
    }
    let active_warps = (max_blocks * warps_per_block).min(max_warps_per_sm);
    let occupancy = (active_warps as f32 / max_warps_per_sm as f32).clamp(0.0, 1.0);
    Some((active_warps, occupancy))
}
