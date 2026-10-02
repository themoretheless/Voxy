//! Synchronous CUDA publication into a shared D3D12/wgpu buffer.
#![allow(unsafe_code)]

use crate::D3d12ExportBuffer;
use voxy_cuda::{
    CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityJob, CudaGravityParameters,
};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Owns the resident f64 solver, shared graphics storage and ordering endpoints.
/// Host synchronization transfers status only; body data remains on the GPU.
#[derive(Debug)]
pub struct CudaGravityD3d12Graphics {
    compute: CudaCompute,
    job: CudaGravityJob,
    storage: D3d12ExportBuffer,
    device: wgpu::Device,
    queue: wgpu::Queue,
    count: u32,
    words: usize,
    graphics_ready: bool,
}

impl CudaGravityD3d12Graphics {
    /// Creates solver and shared storage on the same physical single-node GPU.
    /// # Safety
    /// Queue must belong to device; exclude concurrent native/wgpu queue access.
    /// # Errors
    /// Reports invalid inputs/budgets, adapter mismatch and CUDA/D3D12 failures.
    pub unsafe fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        compute: CudaCompute,
        bodies: &[CudaGravityBody],
        parameters: CudaGravityParameters,
        budget: CudaGravityBudget,
    ) -> Result<Self, Error> {
        let count = u32::try_from(bodies.len())?;
        let words = super::gravity::render_words(count)?;
        let bytes = u64::try_from(words)?
            .checked_mul(4)
            .ok_or("render size overflow")?;
        let cuda = compute.capabilities()?;
        if bytes > u64::try_from(cuda.allocation_budget_bytes)? {
            return Err(voxy_cuda::CudaError::BufferLimit.into());
        }
        let storage = unsafe { D3d12ExportBuffer::new(device, queue, bytes) }?;
        storage.validate_cuda_device(&compute)?;
        if storage.allocation_bytes() > u64::try_from(cuda.allocation_budget_bytes)? {
            return Err(voxy_cuda::CudaError::BufferLimit.into());
        }
        let job = compute.create_gravity_job(bodies, parameters, budget)?;
        Ok(Self {
            compute,
            job,
            storage,
            device: device.clone(),
            queue: queue.clone(),
            count,
            words,
            graphics_ready: true,
        })
    }

    #[must_use]
    pub fn body_count(&self) -> u32 {
        self.count
    }

    /// Render-only storage; retained bindings obey publish's exclusion contract.
    /// # Errors
    /// Rejects graphics use after an incomplete queue/CUDA synchronization.
    pub fn buffer(&self) -> Result<&wgpu::Buffer, Error> {
        if !self.graphics_ready {
            return Err("D3D12 buffer has incomplete external handoff".into());
        }
        Ok(self.storage.buffer())
    }

    /// Advances CUDA physics and publishes the committed f32 render ABI in place.
    /// Zero steps publishes the initial/current state. Graphics queue completion
    /// precedes CUDA access; CUDA completion precedes exposing graphics storage.
    /// # Safety
    /// Flush pending graphics commands before calling. Exclude concurrent queue
    /// submissions and all retained graphics bindings until this returns. No
    /// external native code may alter the resource state or queue ordering.
    /// # Errors
    /// Reports solver/conversion, graphics completion or CUDA failures. On an
    /// unconfirmed synchronization failure, graphics access remains disabled.
    pub unsafe fn publish(&mut self, steps: u32) -> Result<(), Error> {
        self.buffer()?;
        if steps > 0 {
            self.job.step(steps)?;
        }
        self.graphics_ready = false;
        // The ordered fence also flushes pending wgpu queue writes. Completed
        // D3D12 buffer command lists decay to COMMON before external CUDA use.
        let fence = self.queue.submit([]);
        self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(fence),
            timeout: None,
        })?;
        // SAFETY: Constructor checks LUID/node identity and graphics completion.
        let mut mapping = unsafe { self.storage.import_cuda(&self.compute, 0, self.words) }?;
        let result = self.job.write_render_view(&mut mapping);
        mapping.release()?;
        // Drain CUDA even after import/conversion errors. CUDA writes do not
        // modify the D3D12 resource state; next wgpu use starts from COMMON.
        self.compute.synchronize()?;
        self.graphics_ready = true;
        result?;
        Ok(())
    }
}
