use crate::CudaError;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub(crate) struct AllocationBudget {
    limit: usize,
    used: AtomicUsize,
}
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
#[derive(Debug)]
pub(crate) struct Reservation {
    budget: Arc<AllocationBudget>,
    bytes: usize,
}
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
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Self, CudaError> {
        self.budget.reserve(bytes)
    }
}
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
}
