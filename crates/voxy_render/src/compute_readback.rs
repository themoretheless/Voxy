//! Device-wide bounded staging storage. Reuse requires successful mapping completion.
use crate::ComputeError;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU8, Ordering},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComputeReadbackLimits {
    pub max_bytes: u64,
    pub max_buffers: usize,
}
impl Default for ComputeReadbackLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_buffers: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComputeReadbackStats {
    pub allocated_bytes: u64,
    pub allocated_buffers: usize,
    pub cached_buffers: usize,
    pub quarantined_buffers: usize,
    pub creations: u64,
    pub reuses: u64,
}

#[derive(Debug, Default)]
struct State {
    stats: ComputeReadbackStats,
    free: Vec<wgpu::Buffer>,
    quarantine: Vec<wgpu::Buffer>,
}
#[derive(Debug)]
struct Inner {
    device: wgpu::Device,
    limits: ComputeReadbackLimits,
    state: Mutex<State>,
}

#[derive(Debug)]
struct RegistryEntry {
    pool: Weak<Inner>,
    // Unconfirmed GPU uses must outlive the last program/consumer owner.
    pinned: Option<Arc<Inner>>,
}
fn pin_quarantine(inner: &Arc<Inner>, pin: bool) {
    let update = |entries: &mut Vec<RegistryEntry>| {
        if let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.pool.ptr_eq(&Arc::downgrade(inner)))
        {
            let has_quarantine = !inner
                .state
                .lock()
                .expect("readback pool poisoned")
                .quarantine
                .is_empty();
            if pin && has_quarantine {
                entry.pinned = Some(inner.clone());
            } else if !has_quarantine {
                entry.pinned = None;
            }
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    update(&mut POOLS.lock().expect("readback registry poisoned"));
    #[cfg(target_arch = "wasm32")]
    POOLS.with(|entries| update(&mut entries.borrow_mut()));
}

#[cfg(not(target_arch = "wasm32"))]
static POOLS: Mutex<Vec<RegistryEntry>> = Mutex::new(Vec::new());
#[cfg(target_arch = "wasm32")]
thread_local! {
    static POOLS: std::cell::RefCell<Vec<RegistryEntry>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Shared by all portable compute readbacks on one device. Browser ownership is
/// on the device's event-loop thread. Limits cover staging only, not total VRAM.
#[derive(Clone, Debug)]
pub struct ComputeReadbackPool(Arc<Inner>);
impl ComputeReadbackPool {
    /// Gets the existing device pool, or creates it with default limits.
    /// # Panics
    /// Panics if the device pool registry is poisoned.
    #[must_use]
    pub fn for_device(device: &wgpu::Device) -> Self {
        Self::registered(device, None).expect("default readback limits are valid")
    }

    /// Configure before creating compute programs. Existing pools with different
    /// limits are rejected, so programs cannot silently acquire separate budgets.
    /// # Errors
    /// Rejects empty/unaligned limits and reconfiguration of a live device pool.
    pub fn configure(
        device: &wgpu::Device,
        limits: ComputeReadbackLimits,
    ) -> Result<Self, ComputeError> {
        if limits.max_bytes == 0 || !limits.max_bytes.is_multiple_of(4) || limits.max_buffers == 0 {
            return Err(ComputeError::InvalidBuffer);
        }
        Self::registered(device, Some(limits))
    }
    fn registered(
        device: &wgpu::Device,
        limits: Option<ComputeReadbackLimits>,
    ) -> Result<Self, ComputeError> {
        let register = |pools: &mut Vec<RegistryEntry>| {
            pools.retain(|entry| entry.pool.strong_count() != 0);
            for inner in pools.iter().filter_map(|entry| entry.pool.upgrade()) {
                if inner.device == *device {
                    if limits.is_some_and(|limits| limits != inner.limits) {
                        return Err(ComputeError::Validation(
                            "readback pool already configured for device".into(),
                        ));
                    }
                    return Ok(Self(inner));
                }
            }
            let inner = Arc::new(Inner {
                device: device.clone(),
                limits: limits.unwrap_or_default(),
                state: Mutex::new(State::default()),
            });
            pools.push(RegistryEntry {
                pool: Arc::downgrade(&inner),
                pinned: None,
            });
            Ok(Self(inner))
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            register(&mut POOLS.lock().expect("readback registry poisoned"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            POOLS.with(|pools| register(&mut pools.borrow_mut()))
        }
    }
    #[must_use]
    pub fn limits(&self) -> ComputeReadbackLimits {
        self.0.limits
    }
    /// Capacity available to new leases, including evictable cached buffers.
    /// This is a scheduling snapshot, not a reservation: concurrent consumers
    /// must still handle admission failure. Quarantined storage stays charged.
    #[must_use]
    pub fn available_capacity(&self) -> ComputeReadbackLimits {
        let state = self.0.state.lock().expect("readback pool poisoned");
        let cached_bytes: u64 = state.free.iter().map(wgpu::Buffer::size).sum();
        ComputeReadbackLimits {
            max_bytes: self.0.limits.max_bytes - (state.stats.allocated_bytes - cached_bytes),
            max_buffers: self.0.limits.max_buffers - (state.stats.allocated_buffers - state.free.len()),
        }
    }
    /// Current staging allocation and reuse counters.
    /// # Panics
    /// Panics if the pool state is poisoned.
    #[must_use]
    pub fn stats(&self) -> ComputeReadbackStats {
        let state = self.0.state.lock().expect("readback pool poisoned");
        ComputeReadbackStats {
            cached_buffers: state.free.len(),
            quarantined_buffers: state.quarantine.len(),
            ..state.stats
        }
    }
    /// Retire abandoned copies on native devices. Destroy their buffers first
    /// (any retained unsubmitted encoder using them must be discarded), then
    /// wait for submitted GPU work before releasing their budget charges.
    /// This explicit cleanup blocks; normal readback never calls it.
    /// # Errors
    /// Poll failures retain the quarantined storage and its charge.
    /// # Panics
    /// Panics if the pool state or registry is poisoned.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn discard_quarantine(&self) -> Result<(), ComputeError> {
        let buffers = {
            let mut state = self.0.state.lock().expect("readback pool poisoned");
            std::mem::take(&mut state.quarantine)
        };
        if buffers.is_empty() {
            return Ok(());
        }
        for buffer in &buffers {
            buffer.destroy();
        }
        if let Err(error) = self.0.device.poll(wgpu::PollType::wait_indefinitely()) {
            self.0
                .state
                .lock()
                .expect("readback pool poisoned")
                .quarantine
                .extend(buffers);
            return Err(ComputeError::Mapping(error.to_string()));
        }
        {
            let mut state = self.0.state.lock().expect("readback pool poisoned");
            state.stats.allocated_bytes -= buffers.iter().map(wgpu::Buffer::size).sum::<u64>();
            state.stats.allocated_buffers -= buffers.len();
        }
        pin_quarantine(&self.0, false);
        Ok(())
    }

    pub(crate) fn acquire(&self, size: u64) -> Result<Arc<ReadbackLease>, ComputeError> {
        if size == 0 || !size.is_multiple_of(4) || size > self.0.device.limits().max_buffer_size {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut state = self.0.state.lock().expect("readback pool poisoned");
        let buffer = if let Some(index) = state.free.iter().position(|buffer| buffer.size() == size)
        {
            state.stats.reuses += 1;
            state.free.swap_remove(index)
        } else {
            if size > self.0.limits.max_bytes {
                return Err(ComputeError::ReadbackBudget);
            }
            while state.stats.allocated_bytes > self.0.limits.max_bytes - size
                || state.stats.allocated_buffers >= self.0.limits.max_buffers
            {
                let Some(buffer) = state.free.pop() else {
                    return Err(ComputeError::ReadbackBudget);
                };
                state.stats.allocated_bytes -= buffer.size();
                state.stats.allocated_buffers -= 1;
                buffer.destroy();
            }
            // Admission and allocation are serialized across all program owners.
            let mut usage = wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ;
            if self.0.device.features().contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS) {
                usage |= wgpu::BufferUsages::STORAGE;
            }
            let buffer = self.0.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pooled compute readback"),
                size,
                usage,
                mapped_at_creation: false,
            });
            state.stats.allocated_bytes += size;
            state.stats.allocated_buffers += 1;
            state.stats.creations += 1;
            buffer
        };
        Ok(Arc::new(ReadbackLease {
            owner: self.0.clone(),
            buffer: Some(buffer),
            state: AtomicU8::new(0),
        }))
    }
}

/// 0 encoded/unconfirmed, 1 mapping, 2 successfully mapped, 3 unmapped,
/// 4 mapping/range failure. The callback holds a lease until completion.
#[derive(Debug)]
pub(crate) struct ReadbackLease {
    owner: Arc<Inner>,
    buffer: Option<wgpu::Buffer>,
    state: AtomicU8,
}
impl ReadbackLease {
    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.as_ref().expect("live lease")
    }
    pub(crate) fn mapping_started(&self) {
        self.state.store(1, Ordering::Release);
    }
    pub(crate) fn mapping_finished(&self, success: bool) {
        self.state
            .store(if success { 2 } else { 4 }, Ordering::Release);
    }
    pub(crate) fn unmapped(&self) {
        self.state.store(3, Ordering::Release);
    }
    pub(crate) fn failed(&self) {
        self.state.store(4, Ordering::Release);
    }
}
impl Drop for ReadbackLease {
    fn drop(&mut self) {
        let buffer = self.buffer.take().expect("live lease");
        let status = self.state.load(Ordering::Acquire);
        if status == 2 {
            buffer.unmap();
        }
        let mut state = self.owner.state.lock().expect("readback pool poisoned");
        if status == 2 || status == 3 {
            state.free.push(buffer);
        } else {
            // A dropped encoder/readback or mapping error does not prove that
            // all GPU uses completed. Preserve both storage and its budget charge.
            state.quarantine.push(buffer);
            drop(state);
            pin_quarantine(&self.owner, true);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn available_capacity_excludes_live_and_quarantined_but_includes_cache() {
        let (device, _) = wgpu::Device::noop(&Default::default());
        let pool = ComputeReadbackPool::configure(&device, ComputeReadbackLimits {
            max_bytes: 16, max_buffers: 2,
        }).unwrap();
        let first = pool.acquire(4).unwrap();
        let second = pool.acquire(8).unwrap();
        assert_eq!(pool.available_capacity(), ComputeReadbackLimits { max_bytes: 4, max_buffers: 0 });
        first.unmapped(); // Simulate a successfully completed map/unmap.
        drop(first);
        assert_eq!(pool.available_capacity(), ComputeReadbackLimits { max_bytes: 8, max_buffers: 1 });
        drop(second); // Unconfirmed storage remains charged.
        assert_eq!(pool.available_capacity(), ComputeReadbackLimits { max_bytes: 8, max_buffers: 1 });
        pool.discard_quarantine().unwrap();
        assert_eq!(pool.available_capacity(), pool.limits());
    }
    #[test]
    fn shared_admission_and_unconfirmed_cancellation() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let pool = ComputeReadbackPool::configure(
            &device,
            ComputeReadbackLimits {
                max_bytes: 8,
                max_buffers: 2,
            },
        )
        .unwrap();
        let shared = ComputeReadbackPool::for_device(&device);
        let first = pool.acquire(4).unwrap();
        let second = shared.acquire(4).unwrap();
        assert!(matches!(
            shared.acquire(4),
            Err(ComputeError::ReadbackBudget)
        ));
        drop(first);
        drop(second);
        assert_eq!(pool.stats().quarantined_buffers, 2);
        assert_eq!(pool.stats().allocated_bytes, 8);
        assert!(matches!(pool.acquire(4), Err(ComputeError::ReadbackBudget)));
        drop(pool);
        drop(shared);
        let retained = ComputeReadbackPool::for_device(&device);
        assert_eq!(retained.stats().allocated_bytes, 8);
        retained.discard_quarantine().unwrap();
        assert_eq!(retained.stats().allocated_bytes, 0);
    }
    #[test]
    fn buffer_count_is_independent_of_byte_capacity() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let pool = ComputeReadbackPool::configure(
            &device,
            ComputeReadbackLimits {
                max_bytes: 1024,
                max_buffers: 1,
            },
        )
        .unwrap();
        let lease = pool.acquire(4).unwrap();
        assert!(matches!(pool.acquire(4), Err(ComputeError::ReadbackBudget)));
        assert_eq!(pool.stats().allocated_bytes, 4);
        drop(lease);
        pool.discard_quarantine().unwrap();
        let lease = pool.acquire(8).unwrap();
        assert_eq!(pool.stats().allocated_bytes, 8);
        drop(lease);
        pool.discard_quarantine().unwrap();
    }
    #[test]
    fn device_identity_and_configuration_are_isolated() {
        let (a, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let (b, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let limits = ComputeReadbackLimits {
            max_bytes: 4,
            max_buffers: 1,
        };
        let pool = ComputeReadbackPool::configure(&a, limits).unwrap();
        let lease = pool.acquire(4).unwrap();
        assert_eq!(
            ComputeReadbackPool::for_device(&b).stats().allocated_bytes,
            0
        );
        assert!(ComputeReadbackPool::configure(&a, ComputeReadbackLimits::default()).is_err());
        assert!(
            ComputeReadbackPool::configure(
                &b,
                ComputeReadbackLimits {
                    max_bytes: 3,
                    max_buffers: 1
                }
            )
            .is_err()
        );
        drop(lease);
        pool.discard_quarantine().unwrap();
    }
}
