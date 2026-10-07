//! Shared managed-buffer and colour-texture admission. Retired resources stay charged until explicit cleanup.
use crate::ComputeError;
use std::sync::{Arc, Mutex, Weak};
use wgpu::util::DeviceExt;

/// A batch is admitted in full before any of its GPU buffers are created.
#[derive(Clone, Copy)]
pub(crate) struct ManagedBufferDescriptor<'a> {
    pub label: &'a str,
    pub size: u64,
    pub contents: Option<&'a [u8]>,
    pub usage: wgpu::BufferUsages,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComputeMemoryStats {
    pub allocated_bytes: u64,
    pub allocated_buffers: usize,
    pub retired_buffers: usize,
    pub allocated_textures: usize,
    pub retired_textures: usize,
}
#[derive(Debug, Default)]
struct State {
    stats: ComputeMemoryStats,
    retired: Vec<ManagedResource>,
    max_bytes: u64,
    require_retirement: bool,
    ever_allocated: bool,
}
#[derive(Debug)]
enum ManagedResource {
    Buffer(wgpu::Buffer),
    Texture { texture: wgpu::Texture, bytes: u64 },
}
impl ManagedResource {
    fn bytes(&self) -> u64 {
        match self {
            Self::Buffer(buffer) => buffer.size(),
            Self::Texture { bytes, .. } => *bytes,
        }
    }
    fn destroy(&self) {
        match self {
            Self::Buffer(buffer) => buffer.destroy(),
            Self::Texture { texture, .. } => texture.destroy(),
        }
    }
}
fn release_resources(state: &mut State, resources: &[ManagedResource]) {
    for resource in resources {
        state.stats.allocated_bytes -= resource.bytes();
        match resource {
            ManagedResource::Buffer(_) => state.stats.allocated_buffers -= 1,
            ManagedResource::Texture { .. } => state.stats.allocated_textures -= 1,
        }
    }
}
fn retire_resource(owner: &Arc<Inner>, resource: ManagedResource) {
    let mut state = owner.state.lock().expect("compute memory state poisoned");
    if state.require_retirement {
        state.retired.push(resource);
        drop(state);
        update_pin(owner);
    } else {
        release_resources(&mut state, std::slice::from_ref(&resource));
        drop(state);
        drop(resource);
    }
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
/// and scene geometry/transform uniforms/colour textures, not staging, CUDA allocations
/// or driver overhead.
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
            retired_buffers: state
                .retired
                .iter()
                .filter(|r| matches!(r, ManagedResource::Buffer(_)))
                .count(),
            retired_textures: state
                .retired
                .iter()
                .filter(|r| matches!(r, ManagedResource::Texture { .. }))
                .count(),
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
        let [storage] = self.allocate_buffers([ManagedBufferDescriptor {
            label,
            size: u64::try_from(data.len()).map_err(|_| ComputeError::InvalidBuffer)?,
            contents: Some(data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        }])?;
        Ok(storage)
    }

    pub(crate) fn allocate_buffers<const N: usize>(
        &self,
        descriptors: [ManagedBufferDescriptor<'_>; N],
    ) -> Result<[ComputeStorage; N], ComputeError> {
        let buffers = self.allocate_buffer_batch(&descriptors)?;
        Ok(buffers
            .try_into()
            .expect("batch preserves descriptor count"))
    }

    pub(crate) fn allocate_buffer_batch(
        &self,
        descriptors: &[ManagedBufferDescriptor<'_>],
    ) -> Result<Vec<ComputeStorage>, ComputeError> {
        let limits = self.0.device.limits();
        let mut bytes = 0_u64;
        for descriptor in descriptors {
            let size = descriptor.size;
            if size == 0
                || !size.is_multiple_of(4)
                || size > limits.max_buffer_size
                || (descriptor.usage.contains(wgpu::BufferUsages::STORAGE)
                    && size > limits.max_storage_buffer_binding_size)
                || descriptor
                    .contents
                    .is_some_and(|data| data.len() as u64 != size)
            {
                return Err(ComputeError::InvalidBuffer);
            }
            bytes = bytes.checked_add(size).ok_or(ComputeError::InvalidBuffer)?;
        }
        let mut state = self.0.state.lock().expect("compute memory state poisoned");
        if bytes > state.max_bytes || state.stats.allocated_bytes > state.max_bytes - bytes {
            return Err(ComputeError::MemoryBudget);
        }
        let buffers: Vec<_> = descriptors
            .iter()
            .map(|descriptor| {
                if let Some(contents) = descriptor.contents {
                    self.0
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(descriptor.label),
                            contents,
                            usage: descriptor.usage,
                        })
                } else {
                    self.0.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(descriptor.label),
                        size: descriptor.size,
                        usage: descriptor.usage,
                        mapped_at_creation: false,
                    })
                }
            })
            .collect();
        state.ever_allocated |= !descriptors.is_empty();
        state.stats.allocated_bytes += bytes;
        state.stats.allocated_buffers += descriptors.len();
        Ok(buffers
            .into_iter()
            .map(|buffer| ComputeStorage {
                owner: self.0.clone(),
                buffer: Some(buffer),
            })
            .collect())
    }
    /// Managed 2D colour allocation; all mip/layer payloads share device capacity.
    pub(crate) fn allocate_texture(
        &self,
        descriptor: &wgpu::TextureDescriptor<'_>,
    ) -> Result<Arc<ManagedTexture>, ComputeError> {
        let bytes = colour_texture_bytes(descriptor, &self.0.device.limits())?;
        let mut state = self.0.state.lock().expect("compute memory state poisoned");
        if bytes > state.max_bytes || state.stats.allocated_bytes > state.max_bytes - bytes {
            return Err(ComputeError::MemoryBudget);
        }
        let texture = self.0.device.create_texture(descriptor);
        state.ever_allocated = true;
        state.stats.allocated_bytes += bytes;
        state.stats.allocated_textures += 1;
        Ok(Arc::new(ManagedTexture {
            owner: self.0.clone(),
            texture: Some(texture),
            bytes,
        }))
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
        let resources = std::mem::take(
            &mut self
                .0
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired,
        );
        let (sender, receiver) = std::sync::mpsc::channel();
        for resource in &resources {
            resource.destroy();
        }
        if resources.is_empty() {
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
            resources: Some(resources),
            receiver,
        }
    }
    /// Destroy retired storage, then block until submitted GPU work completes.
    /// Discard unsubmitted encoders and externally retained bindings/resource clones
    /// referencing retired storage before calling this operation. Never implicit.
    /// # Errors
    /// Poll failures preserve storage charges for retry.
    /// # Panics
    /// Panics if the budget state or registry is poisoned.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn discard_retired(&self) -> Result<(), ComputeError> {
        let resources = std::mem::take(
            &mut self
                .0
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired,
        );
        if resources.is_empty() {
            return Ok(());
        }
        for resource in &resources {
            resource.destroy();
        }
        if let Err(error) = self.0.device.poll(wgpu::PollType::wait_indefinitely()) {
            self.0
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired
                .extend(resources);
            return Err(ComputeError::Mapping(error.to_string()));
        }
        {
            let mut state = self.0.state.lock().expect("compute memory state poisoned");
            release_resources(&mut state, &resources);
        }
        update_pin(&self.0);
        Ok(())
    }
}
/// Completion ticket for nonblocking native/browser retirement. Dropping an
/// unfinished ticket restores its resources to the charged retirement queue.
#[derive(Debug)]
pub struct PendingComputeRetirement {
    owner: Arc<Inner>,
    resources: Option<Vec<ManagedResource>>,
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
        if self.resources.is_none() {
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
        let resources = self.resources.take().expect("unfinished retirement");
        {
            let mut state = self
                .owner
                .state
                .lock()
                .expect("compute memory state poisoned");
            release_resources(&mut state, &resources);
        }
        update_pin(&self.owner);
        Ok(true)
    }
}
impl Drop for PendingComputeRetirement {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            self.owner
                .state
                .lock()
                .expect("compute memory state poisoned")
                .retired
                .extend(resources);
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
        retire_resource(&self.owner, ManagedResource::Buffer(buffer));
    }
}

/// Retains one shared colour allocation across material and HDR mip views.
#[derive(Debug)]
pub(crate) struct ManagedTexture {
    owner: Arc<Inner>,
    texture: Option<wgpu::Texture>,
    bytes: u64,
}
impl ManagedTexture {
    pub(crate) fn allocation_bytes(&self) -> u64 {
        self.bytes
    }
}
impl std::ops::Deref for ManagedTexture {
    type Target = wgpu::Texture;
    fn deref(&self) -> &Self::Target {
        self.texture.as_ref().expect("live managed texture")
    }
}
impl Drop for ManagedTexture {
    fn drop(&mut self) {
        retire_resource(
            &self.owner,
            ManagedResource::Texture {
                texture: self.texture.take().expect("live managed texture"),
                bytes: self.bytes,
            },
        );
    }
}
fn colour_texture_bytes(
    descriptor: &wgpu::TextureDescriptor<'_>,
    limits: &wgpu::Limits,
) -> Result<u64, ComputeError> {
    let texel_bytes = match descriptor.format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb => 4_u64,
        wgpu::TextureFormat::Rgba16Float => 8,
        _ => return Err(ComputeError::InvalidBuffer),
    };
    let size = descriptor.size;
    if descriptor.dimension != wgpu::TextureDimension::D2
        || descriptor.sample_count != 1
        || size.width == 0
        || size.height == 0
        || size.depth_or_array_layers == 0
        || size.width > limits.max_texture_dimension_2d
        || size.height > limits.max_texture_dimension_2d
        || size.depth_or_array_layers > limits.max_texture_array_layers
        || descriptor.mip_level_count == 0
        || descriptor.mip_level_count > size.width.max(size.height).ilog2() + 1
    {
        return Err(ComputeError::InvalidBuffer);
    }
    let mut bytes = 0_u64;
    for level in 0..descriptor.mip_level_count {
        let level_bytes = u64::from((size.width >> level).max(1))
            .checked_mul(u64::from((size.height >> level).max(1)))
            .and_then(|n| n.checked_mul(u64::from(size.depth_or_array_layers)))
            .and_then(|n| n.checked_mul(texel_bytes))
            .ok_or(ComputeError::InvalidBuffer)?;
        bytes = bytes
            .checked_add(level_bytes)
            .ok_or(ComputeError::InvalidBuffer)?;
    }
    Ok(bytes)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    fn texture_descriptor(
        width: u32,
        height: u32,
        levels: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("budget texture fixture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }
    #[test]
    fn texture_mip_payload_and_invalid_admission_are_exact() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let limits = device.limits();
        let rgba = texture_descriptor(4, 2, 3, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(colour_texture_bytes(&rgba, &limits).unwrap(), 44);
        let hdr = texture_descriptor(3, 2, 2, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(colour_texture_bytes(&hdr, &limits).unwrap(), 56);
        let budget = ComputeMemoryBudget::configure(&device, 40).unwrap();
        assert!(matches!(
            budget.allocate_texture(&rgba),
            Err(ComputeError::MemoryBudget)
        ));
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
        for invalid in [
            texture_descriptor(0, 2, 1, wgpu::TextureFormat::Rgba8Unorm),
            texture_descriptor(4, 2, 4, wgpu::TextureFormat::Rgba8Unorm),
            texture_descriptor(4, 2, 0, wgpu::TextureFormat::Rgba8Unorm),
            texture_descriptor(4, 2, 1, wgpu::TextureFormat::Depth32Float),
        ] {
            assert!(matches!(
                budget.allocate_texture(&invalid),
                Err(ComputeError::InvalidBuffer)
            ));
            assert_eq!(budget.stats(), ComputeMemoryStats::default());
        }
    }
    #[test]
    fn shared_texture_and_buffer_retirement_cancellation_keeps_all_charges() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&device, 92).unwrap();
        let mut descriptor = texture_descriptor(4, 2, 3, wgpu::TextureFormat::Rgba8Unorm);
        descriptor.size.depth_or_array_layers = 2;
        let texture = budget.allocate_texture(&descriptor).unwrap();
        let alias = texture.clone();
        let buffer = budget.allocate_storage("mixed resource", &[0; 4]).unwrap();
        assert_eq!(budget.stats().allocated_bytes, 92);
        assert_eq!(budget.stats().allocated_textures, 1);
        drop(texture);
        assert_eq!(budget.stats().retired_textures, 0);
        drop(alias);
        drop(buffer);
        assert_eq!(budget.stats().retired_textures, 1);
        assert_eq!(budget.stats().retired_buffers, 1);
        let pending = budget.begin_retirement(&queue);
        assert_eq!(budget.stats().allocated_bytes, 92);
        assert!(matches!(
            budget.allocate_storage("pending fence", &[0; 4]),
            Err(ComputeError::MemoryBudget)
        ));
        drop(pending);
        assert_eq!(budget.stats().retired_textures, 1);
        assert_eq!(budget.stats().retired_buffers, 1);
        let mut pending = budget.begin_retirement(&queue);
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(sender);
        pending.receiver = receiver;
        assert!(matches!(
            pending.try_finish(),
            Err(ComputeError::Mapping(_))
        ));
        assert_eq!(budget.stats().allocated_bytes, 92);
        drop(pending);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
    }
    #[test]
    fn unconfigured_texture_owners_keep_ordinary_lifetime() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::for_device(&device);
        let texture = budget
            .allocate_texture(&texture_descriptor(
                1,
                1,
                1,
                wgpu::TextureFormat::Rgba8Unorm,
            ))
            .unwrap();
        assert_eq!(budget.stats().allocated_bytes, 4);
        let alias = texture.clone();
        drop(texture);
        assert_eq!(budget.stats().allocated_bytes, 4);
        drop(alias);
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
    }
    #[test]
    fn batch_validation_and_admission_leave_no_partial_retirement() {
        let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let budget = ComputeMemoryBudget::configure(&device, 8).unwrap();
        let descriptor = |size, contents| ManagedBufferDescriptor {
            label: "batch test",
            size,
            contents,
            usage: wgpu::BufferUsages::STORAGE,
        };
        assert!(matches!(
            budget.allocate_buffers([descriptor(4, Some(&[0; 4])), descriptor(4, Some(&[0; 3])),]),
            Err(ComputeError::InvalidBuffer)
        ));
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
        assert!(matches!(
            budget.allocate_buffers([descriptor(4, Some(&[0; 4])), descriptor(8, None),]),
            Err(ComputeError::MemoryBudget)
        ));
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
        let buffers = budget
            .allocate_buffers([descriptor(4, Some(&[0; 4])), descriptor(4, None)])
            .unwrap();
        assert_eq!(budget.stats().allocated_bytes, 8);
        drop(buffers);
        assert_eq!(budget.stats().retired_buffers, 2);
        budget.discard_retired().unwrap();
        assert_eq!(budget.stats(), ComputeMemoryStats::default());
    }
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
