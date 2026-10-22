//! CPU-produced, directly shader-bound shared buffers. No staging/blit upload.
use crate::{ComputeError, ComputeMemoryBudget, ComputeStorage, ManagedBufferDescriptor};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

const PENDING: u8 = 0;
const READY: u8 = 1;
const ENCODING: u8 = 2;
const FAILED: u8 = 3;

#[derive(Debug)]
struct Slot {
    buffer: ComputeStorage,
    state: Arc<AtomicU8>,
}

// Deferred mapping callbacks are dropped when their encoder/command buffer is
// discarded. Re-map that unused slot instead of permanently losing ring capacity.
struct ReclaimDiscarded {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    completed: bool,
}
impl Drop for ReclaimDiscarded {
    fn drop(&mut self) {
        if !self.completed {
            let state = self.state.clone();
            self.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Write, move |result| {
                    state.store(
                        if result.is_ok() { READY } else { FAILED },
                        Ordering::Release,
                    );
                });
        }
    }
}

/// Three independently mapped primary buffers for transient CPU-produced inputs.
///
/// A slot is leased to exactly one command encoder, then mapped back only after
/// that encoder is submitted and the GPU finishes using it. Exhaustion never
/// waits for an unsubmitted encoder: `encode` returns false for caller fallback.
/// Discarding an unsubmitted encoder asynchronously reclaims its unused slot.
/// The ring participates in the device's managed memory budget and retirement.
#[derive(Debug)]
pub struct SharedUploadRing {
    device: wgpu::Device,
    slots: Vec<Slot>,
}
impl SharedUploadRing {
    /// Returns None outside Apple Silicon Metal, without primary mapping support,
    /// or if extra ring memory cannot be admitted. The legacy path remains usable.
    pub fn new(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> Option<Self> {
        if !is_apple_metal(device)
            || !device
                .features()
                .contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
            || usage.intersects(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE)
        {
            return None;
        }
        let descriptors = [ManagedBufferDescriptor {
            label: "Apple shared transient input",
            size,
            contents: None,
            usage: usage | wgpu::BufferUsages::MAP_WRITE,
        }; 3];
        let buffers = ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .ok()?;
        let slots = buffers
            .into_iter()
            .map(|buffer| {
                let state = Arc::new(AtomicU8::new(PENDING));
                let ready = state.clone();
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Write, move |result| {
                        ready.store(
                            if result.is_ok() { READY } else { FAILED },
                            Ordering::Release,
                        );
                    });
                Slot { buffer, state }
            })
            .collect();
        Some(Self {
            device: device.clone(),
            slots,
        })
    }

    pub(crate) fn allocation_bytes(&self) -> u64 {
        self.slots.iter().map(|slot| slot.buffer.size()).sum()
    }

    /// The returned buffer may be consumed later in this same encoder only.
    /// Remapping is deferred until the complete command buffer finishes. Overflow
    /// records a unique staging snapshot before consumers, preserving queue order.
    pub(crate) fn write_or_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        legacy: &wgpu::Buffer,
        produce: impl Fn(wgpu::WriteOnly<'_, [u8]>),
    ) -> Result<(usize, wgpu::Buffer), ComputeError> {
        let mut selected = None;
        if self.encode(encoder, &produce, |index, buffer, _| {
            selected = Some((index, buffer.clone()))
        })? {
            return Ok(selected.unwrap());
        }
        self.stage_legacy(encoder, legacy, produce)?;
        Ok((3, legacy.clone()))
    }

    pub(crate) fn stage_legacy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        legacy: &wgpu::Buffer,
        produce: impl Fn(wgpu::WriteOnly<'_, [u8]>),
    ) -> Result<(), ComputeError> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transient input overflow staging"),
            size: legacy.size(),
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        {
            let mut mapped = staging
                .slice(..)
                .get_mapped_range_mut()
                .map_err(|e| ComputeError::Validation(e.to_string()))?;
            produce(mapped.slice(..));
        }
        staging.unmap();
        encoder.copy_buffer_to_buffer(&staging, 0, legacy, 0, legacy.size());
        Ok(())
    }

    pub(crate) fn buffers(&self) -> impl Iterator<Item = &wgpu::Buffer> {
        self.slots.iter().map(|slot| &*slot.buffer)
    }

    /// Produce the complete input directly into shared memory and encode its
    /// consumers. `consume` must use the buffer only in this encoder, and must
    /// not retain it for another submission. No API mapping may escape `produce`.
    /// Returns false without invoking either closure when all slots are busy.
    /// Producers must initialize every byte their consumers read on every call.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        produce: impl FnOnce(wgpu::WriteOnly<'_, [u8]>),
        consume: impl FnOnce(usize, &wgpu::Buffer, &mut wgpu::CommandEncoder),
    ) -> Result<bool, ComputeError> {
        // Poll only; never serialize the frame with a device-wide wait.
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|e| ComputeError::Validation(e.to_string()))?;
        let Some((index, slot)) = self.slots.iter().enumerate().find(|(_, slot)| {
            slot.state
                .compare_exchange(READY, ENCODING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        }) else {
            return Ok(false);
        };
        {
            let mut mapped = slot
                .buffer
                .slice(..)
                .get_mapped_range_mut()
                .map_err(|e| ComputeError::Validation(e.to_string()))?;
            produce(mapped.slice(..));
        }
        slot.buffer.unmap();
        consume(index, &slot.buffer, encoder);
        let mut reclaim = ReclaimDiscarded {
            buffer: (*slot.buffer).clone(),
            state: slot.state.clone(),
            completed: false,
        };
        slot.state.store(PENDING, Ordering::Release);
        encoder.map_buffer_on_submit(&slot.buffer, wgpu::MapMode::Write, .., move |result| {
            reclaim.completed = true;
            reclaim.state.store(
                if result.is_ok() { READY } else { FAILED },
                Ordering::Release,
            );
        });
        Ok(true)
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[allow(unsafe_code)]
fn is_apple_metal(device: &wgpu::Device) -> bool {
    // SAFETY: backend inspection only, with no retained native handle or mutation.
    unsafe { device.as_hal::<wgpu_hal::api::Metal>().is_some() }
}
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn is_apple_metal(_: &wgpu::Device) -> bool {
    false
}
