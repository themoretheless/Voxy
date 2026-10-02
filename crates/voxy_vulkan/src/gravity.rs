//! Resident CUDA solver and exportable wgpu storage owned as one resource.
#![allow(unsafe_code)]
use crate::{VulkanExportBuffer, device_uuid};
use voxy_cuda::{
    CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityJob, CudaGravityParameters,
};
type Error = Box<dyn std::error::Error + Send + Sync>;

/// GPU-to-GPU render publication; no per-frame body upload/readback.
#[derive(Debug)]
pub struct CudaGravityGraphics {
    compute: CudaCompute,
    job: CudaGravityJob,
    storage: VulkanExportBuffer,
    count: u32,
    words: usize,
}
impl CudaGravityGraphics {
    /// Creates the f64 solver and initialized render storage on matching devices.
    /// # Safety
    /// Exclude concurrent host queue access during Vulkan allocation initialization.
    /// # Errors
    /// Rejects invalid bodies/budgets, mismatched UUIDs, absent export support,
    /// NVRTC/compiler and driver/resource failures without selecting another GPU.
    pub unsafe fn new(
        device: &wgpu::Device,
        compute: CudaCompute,
        bodies: &[CudaGravityBody],
        parameters: CudaGravityParameters,
        budget: CudaGravityBudget,
    ) -> Result<Self, Error> {
        let count = u32::try_from(bodies.len())?;
        let words = render_words(count)?;
        let cuda = compute.capabilities()?;
        if cuda.uuid == [0; 16] || cuda.uuid != device_uuid(device)? {
            return Err("CUDA/Vulkan physical device UUID mismatch".into());
        }
        let job = compute.create_gravity_job(bodies, parameters, budget)?;
        let bytes = u64::try_from(words)?
            .checked_mul(4)
            .ok_or("render size overflow")?;
        // SAFETY: Caller excludes concurrent queue access; exact render allocation size.
        let storage = unsafe { VulkanExportBuffer::new(device, bytes)? };
        if storage.allocation_bytes() > u64::try_from(cuda.allocation_budget_bytes)? {
            return Err(voxy_cuda::CudaError::BufferLimit.into());
        }
        Ok(Self {
            compute,
            job,
            storage,
            count,
            words,
        })
    }
    #[must_use]
    pub fn body_count(&self) -> u32 {
        self.count
    }
    /// Render-only storage retained by graphics bind groups.
    /// # Errors
    /// Rejects access while CUDA owns the allocation after a failed handoff.
    pub fn buffer(&self) -> Result<&wgpu::Buffer, Error> {
        self.storage.buffer()
    }
    /// Advances the solver and publishes its committed bodies. Zero steps only
    /// publishes the current state, including the initial state.
    /// # Safety
    /// Flush pending graphics writes/submissions and exclude concurrent host queue
    /// access. Do not use retained buffer/binding clones until this call completes.
    /// # Errors
    /// Reports step budget, solver/conversion and CUDA/Vulkan failures. Failed
    /// conversion retains previous render contents; failed synchronization prevents
    /// graphics reacquisition. CPU receives status only, never body data.
    pub unsafe fn publish(&mut self, steps: u32) -> Result<(), Error> {
        // Wrong ownership must be rejected before mutating the private solver.
        self.storage.buffer()?;
        if steps > 0 {
            self.job.step(steps)?;
        }
        let allocation = usize::try_from(self.storage.allocation_bytes())?;
        // SAFETY: Caller supplies exclusive host queue access and flushed graphics work.
        let fd = unsafe { self.storage.release()? };
        // SAFETY: Constructor matches device UUID, exact allocation and exclusive ownership.
        let mut imported = unsafe {
            self.compute
                .import_external_u32(fd, allocation, 0, self.words)?
        };
        let result = self.job.write_render_view(&mut imported);
        imported.release()?;
        self.compute.synchronize()?;
        // SAFETY: Mapping has dropped, CUDA completed, caller excludes concurrent queue use.
        let acquired = unsafe { self.storage.acquire() };
        result?;
        acquired
    }
}
pub(crate) fn render_words(count: u32) -> Result<usize, Error> {
    if count == 0 {
        return Err("empty gravity render view".into());
    }
    Ok(usize::try_from(
        count
            .checked_mul(8)
            .and_then(|words| words.checked_add(8))
            .ok_or("render view address overflow")?,
    )?)
}
#[cfg(test)]
mod tests {
    use super::render_words;
    #[test]
    fn render_capacity_matches_shader_address_space() {
        assert_eq!(render_words(1).unwrap(), 16);
        assert_eq!(render_words(257).unwrap(), 2064);
        assert!(render_words(0).is_err());
        assert!(render_words(u32::MAX).is_err());
        assert!(render_words(u32::MAX / 8).is_err());
    }
}
