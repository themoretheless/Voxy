//! Shared resident-compute admission. Retired resources stay charged until explicit cleanup.
use crate::ComputeError;
use std::sync::{Arc, Mutex, Weak};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComputeMemoryStats {
    pub allocated_bytes: u64,
    pub allocated_buffers: usize,
    pub retired_buffers: usize,
}
#[derive(Debug, Default)]
struct State {
    stats: ComputeMemoryStats,
    retired: Vec<wgpu::Buffer>,
    max_bytes: u64,
    require_retirement: bool,
    ever_allocated: bool,
}
#[derive(Debug)]
struct Inner {
    device: wgpu::Device,
    state: Mutex<State>,
}
#[derive(Debug)]
struct Entry {
    weak: Weak<Inner>,
    pinned: Option<Arc<Inner>>,
}
#[cfg(not(target_arch = "wasm32"))]
static REGISTRY: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
#[cfg(target_arch = "wasm32")]
thread_local! {
    static REGISTRY: std::cell::RefCell<Vec<Entry>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn registry<T>(f: impl FnOnce(&mut Vec<Entry>) -> T) -> T {
    #[cfg(not(target_arch = "wasm32"))]
    {
        f(&mut REGISTRY.lock().expect("compute memory registry poisoned"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        REGISTRY.with(|entries| f(&mut entries.borrow_mut()))
    }
}
fn update_pin(inner: &Arc<Inner>) {
    registry(|entries| {
        if let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.weak.ptr_eq(&Arc::downgrade(inner)))
        {
            let retired = !inner
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired
                .is_empty();
            entry.pinned = retired.then(|| inner.clone());
        }
    });
}
/// One shared resident-storage limit per device. This covers managed compute
/// buffers, not textures, meshes, staging, CUDA allocations or driver overhead.
#[derive(Clone, Debug)]
pub struct ComputeMemoryBudget(Arc<Inner>);
impl ComputeMemoryBudget {
    /// Gets the shared accounting owner. Without explicit configuration there
    /// is no practical admission cap; configure a limit and retirement cadence
    /// together before creating device owners.
    /// # Panics
    /// Panics if the registry is poisoned.
    #[must_use]
    pub fn for_device(device: &wgpu::Device) -> Self {
        Self::registered(device, None).expect("default compute budget is valid")
    }
    /// Configure before the first managed storage allocation. Existing unused
    /// accounting owners (including a renderer) share the configured limit.
    /// # Errors
    /// Rejects zero/unaligned limits, changing an existing configured limit,
    /// or enabling a limit after unbounded allocations have already been used.
    /// # Panics
    /// Panics if the registry is poisoned.
    pub fn configure(device: &wgpu::Device, max_bytes: u64) -> Result<Self, ComputeError> {
        if max_bytes == 0 || !max_bytes.is_multiple_of(4) {
            return Err(ComputeError::InvalidBuffer);
        }
        Self::registered(device, Some(max_bytes))
    }
    fn registered(device: &wgpu::Device, limit: Option<u64>) -> Result<Self, ComputeError> {
        registry(|entries| {
            entries.retain(|entry| entry.weak.strong_count() != 0);
            if let Some(inner) = entries
                .iter()
                .filter_map(|entry| entry.weak.upgrade())
                .find(|inner| inner.device == *device)
            {
                if let Some(limit) = limit {
                    let mut state = inner.state.lock().expect("compute memory state poisoned");
                    if !state.require_retirement && !state.ever_allocated {
                        state.max_bytes = limit;
                        state.require_retirement = true;
                    } else if !state.require_retirement || limit != state.max_bytes {
                        return Err(ComputeError::Validation(
                            "compute memory budget already configured or used".into(),
                        ));
                    }
                }
                return Ok(Self(inner));
            }
            let inner = Arc::new(Inner {
                device: device.clone(),
                state: Mutex::new(State {
                    max_bytes: limit.unwrap_or(u64::MAX - 3),
                    require_retirement: limit.is_some(),
                    ..State::default()
                }),
            });
            entries.push(Entry {
                weak: Arc::downgrade(&inner),
                pinned: None,
            });
            Ok(Self(inner))
        })
    }
    /// Current shared admission limit.
    /// # Panics
    /// Panics if the budget state is poisoned.
    #[must_use]
    pub fn max_bytes(&self) -> u64 {
        self.0
            .state
            .lock()
            .expect("compute memory state poisoned")
            .max_bytes
    }
    /// Configured budgets include live and retired storage until confirmation.
    /// Unconfigured accounting counts live managed owners only; it cannot measure
    /// driver-retained submissions or external buffer clones.
    /// # Panics
    /// Panics if the budget state is poisoned.
    #[must_use]
    pub fn stats(&self) -> ComputeMemoryStats {
        let state = self.0.state.lock().expect("compute memory state poisoned");
        ComputeMemoryStats {
            retired_buffers: state.retired.len(),
            ..state.stats
        }
    }
    /// Allocate initialized storage charged against this device's shared limit.
    /// The returned owner retains accounting after program/job destruction.
    /// # Errors
    /// Rejects invalid storage sizes and exhausted shared capacity before allocation.
    /// # Panics
    /// Panics if the budget state is poisoned.
    pub fn allocate_storage(
        &self,
        label: &str,
        data: &[u8],
    ) -> Result<ComputeStorage, ComputeError> {
        let size = u64::try_from(data.len()).map_err(|_| ComputeError::InvalidBuffer)?;
        let limits = self.0.device.limits();
        if size == 0
            || !size.is_multiple_of(4)
            || size > limits.max_buffer_size
            || size > limits.max_storage_buffer_binding_size
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut state = self.0.state.lock().expect("compute memory state poisoned");
        if size > state.max_bytes || state.stats.allocated_bytes > state.max_bytes - size {
            return Err(ComputeError::MemoryBudget);
        }
        let buffer = self
            .0
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });
        state.ever_allocated = true;
        state.stats.allocated_bytes += size;
        state.stats.allocated_buffers += 1;
        Ok(ComputeStorage {
            owner: self.0.clone(),
            buffer: Some(buffer),
        })
    }
    /// Schedule nonblocking retirement on this device's queue after submitting
    /// all commands that reference retired storage. Discard unsubmitted encoders
    /// and externally retained bindings/clones first. The queue must belong to
    /// this budget's device. Poll the returned ticket or the browser event loop;
    /// charges remain until its completion is observed. Cancellation retains them.
    /// # Panics
    /// Panics if the budget state is poisoned.
    #[must_use]
    pub fn begin_retirement(&self, queue: &wgpu::Queue) -> PendingComputeRetirement {
        let buffers = std::mem::take(
            &mut self
                .0
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired,
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        for buffer in &buffers {
            buffer.destroy();
        }
        if buffers.is_empty() {
            let _ = sender.send(());
        } else {
            // Capture only the Send notification: browser resources stay on
            // their owning event-loop thread in the ticket.
            queue.on_submitted_work_done(move || {
                let _ = sender.send(());
            });
        }
        PendingComputeRetirement {
            owner: self.0.clone(),
            buffers: Some(buffers),
            receiver,
        }
    }
    /// Destroy retired storage, then block until submitted GPU work completes.
    /// Discard unsubmitted encoders and externally retained bindings/buffer clones
    /// referencing retired storage before calling this operation. Never implicit.
    /// # Errors
    /// Poll failures preserve storage charges for retry.
    /// # Panics
    /// Panics if the budget state or registry is poisoned.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn discard_retired(&self) -> Result<(), ComputeError> {
        let buffers = std::mem::take(
            &mut self
                .0
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired,
        );
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
                .expect("compute memory state poisoned")
                .retired
                .extend(buffers);
            return Err(ComputeError::Mapping(error.to_string()));
        }
        {
            let mut state = self.0.state.lock().expect("compute memory state poisoned");
            state.stats.allocated_bytes -= buffers.iter().map(wgpu::Buffer::size).sum::<u64>();
            state.stats.allocated_buffers -= buffers.len();
        }
        update_pin(&self.0);
        Ok(())
    }
}
/// Completion ticket for nonblocking native/browser retirement. Dropping an
/// unfinished ticket restores its buffers to the charged retirement queue.
#[derive(Debug)]
pub struct PendingComputeRetirement {
    owner: Arc<Inner>,
    buffers: Option<Vec<wgpu::Buffer>>,
    receiver: std::sync::mpsc::Receiver<()>,
}
impl PendingComputeRetirement {
    /// Returns true after queue completion was observed and charges released.
    /// Repeated calls after completion also return true. A lost notification
    /// retains charges; dropping this ticket allows explicit cleanup/retry.
    /// # Errors
    /// Reports a disconnected completion callback without releasing charges.
    /// # Panics
    /// Panics if the budget state or registry is poisoned.
    pub fn try_finish(&mut self) -> Result<bool, ComputeError> {
        if self.buffers.is_none() {
            return Ok(true);
        }
        match self.receiver.try_recv() {
            Ok(()) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(false),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(ComputeError::Mapping(
                    "retirement callback disconnected".into(),
                ));
            }
        }
        let buffers = self.buffers.take().expect("unfinished retirement");
        {
            let mut state = self
                .owner
                .state
                .lock()
                .expect("compute memory state poisoned");
            state.stats.allocated_bytes -= buffers.iter().map(wgpu::Buffer::size).sum::<u64>();
            state.stats.allocated_buffers -= buffers.len();
        }
        update_pin(&self.owner);
        Ok(true)
    }
}
impl Drop for PendingComputeRetirement {
    fn drop(&mut self) {
        if let Some(buffers) = self.buffers.take() {
            self.owner
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired
                .extend(buffers);
            update_pin(&self.owner);
        }
    }
}

/// Resident storage ownership. Configured budgets retain dropped buffers until
/// explicit retirement; unconfigured owners use ordinary wgpu lifetime handling.
#[derive(Debug)]
pub struct ComputeStorage {
    owner: Arc<Inner>,
    buffer: Option<wgpu::Buffer>,
}
impl std::ops::Deref for ComputeStorage {
    type Target = wgpu::Buffer;
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().expect("live compute storage")
    }
}
impl Drop for ComputeStorage {
    fn drop(&mut self) {
        let buffer = self.buffer.take().expect("live compute storage");
        let mut state = self
            .owner
            .state
            .lock()
            .expect("compute memory state poisoned");
        if state.require_retirement {
            state.retired.push(buffer);
            drop(state);
            update_pin(&self.owner);
        } else {
            // Preserve ordinary wgpu lifetime management until the application
            // opts into a bounded budget plus an explicit retirement cadence.
            state.stats.allocated_bytes -= buffer.size();
            state.stats.allocated_buffers -= 1;
            drop(state);
            drop(buffer);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn shared_limit_retains_retired_charge_after_last_owner() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&device, 8).unwrap();
        let shared = ComputeMemoryBudget::for_device(&device);
        let a = budget.allocate_storage("a", &[0; 4]).unwrap();
        let b = shared.allocate_storage("b", &[0; 4]).unwrap();
        assert!(matches!(
            shared.allocate_storage("c", &[0; 4]),
            Err(ComputeError::MemoryBudget)
        ));
        drop(a);
        drop(b);
        drop(budget);
        drop(shared);
        let retained = ComputeMemoryBudget::for_device(&device);
        assert_eq!(retained.stats().allocated_bytes, 8);
        assert_eq!(retained.stats().retired_buffers, 2);
        retained.discard_retired().unwrap();
        assert_eq!(retained.stats().allocated_bytes, 0);
        let storage = retained.allocate_storage("after cleanup", &[0; 8]).unwrap();
        drop(storage);
        retained.discard_retired().unwrap();
    }
    #[test]
    fn cancelled_async_retirement_preserves_charges() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&device, 4).unwrap();
        drop(budget.allocate_storage("retire", &[0; 4]).unwrap());
        let pending = budget.begin_retirement(&queue);
        assert_eq!(budget.stats().allocated_bytes, 4);
        assert!(matches!(
            budget.allocate_storage("too early", &[0; 4]),
            Err(ComputeError::MemoryBudget)
        ));
        drop(pending);
        assert_eq!(budget.stats().retired_buffers, 1);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats().allocated_bytes, 0);
        let mut empty = budget.begin_retirement(&queue);
        assert!(empty.try_finish().unwrap());
        assert!(empty.try_finish().unwrap());
    }
    #[test]
    fn disconnected_notification_is_an_error_and_keeps_charges() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&device, 4).unwrap();
        drop(budget.allocate_storage("retire", &[0; 4]).unwrap());
        let mut pending = budget.begin_retirement(&queue);
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(sender);
        pending.receiver = receiver;
        assert!(matches!(
            pending.try_finish(),
            Err(ComputeError::Mapping(_))
        ));
        assert_eq!(budget.stats().allocated_bytes, 4);
        drop(pending);
        assert_eq!(budget.stats().retired_buffers, 1);
        budget.discard_retired().unwrap();
    }
    #[test]
    fn unconfigured_owners_preserve_ordinary_buffer_lifetime() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::for_device(&device);
        let storage = budget.allocate_storage("ordinary", &[0; 4]).unwrap();
        assert_eq!(budget.stats().allocated_bytes, 4);
        drop(storage);
        assert_eq!(budget.stats().allocated_bytes, 0);
        assert_eq!(budget.stats().retired_buffers, 0);
    }
    #[test]
    fn existing_unused_owner_can_be_configured_but_used_owner_cannot() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let owner = ComputeMemoryBudget::for_device(&device);
        let configured = ComputeMemoryBudget::configure(&device, 4).unwrap();
        assert_eq!(owner.max_bytes(), 4);
        drop(owner.allocate_storage("configured", &[0; 4]).unwrap());
        configured.discard_retired().unwrap();
        let (other, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let used = ComputeMemoryBudget::for_device(&other);
        drop(used.allocate_storage("unbounded", &[0; 4]).unwrap());
        assert!(ComputeMemoryBudget::configure(&other, 4).is_err());
    }
    #[test]
    fn device_isolation_and_invalid_admission() {
        let (a, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let (b, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&a, 4).unwrap();
        assert!(ComputeMemoryBudget::configure(&a, 8).is_err());
        assert!(ComputeMemoryBudget::configure(&b, 3).is_err());
        assert!(matches!(
            budget.allocate_storage("bad", &[0; 3]),
            Err(ComputeError::InvalidBuffer)
        ));
        assert_eq!(
            ComputeMemoryBudget::for_device(&b).stats().allocated_bytes,
            0
        );
        assert_eq!(budget.stats().allocated_bytes, 0);
    }
}
