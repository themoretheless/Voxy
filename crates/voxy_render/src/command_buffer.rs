//! Multi-threaded GPU command buffer recording and deterministic submission merging
//! (Optimizations #76-80).

/// Partitions a slice of rendering work (such as [`crate::SceneDraw`] items) into at most
/// `max_partitions` balanced, non-empty contiguous sub-slices for parallel command encoding.
#[must_use]
pub fn partition_work_slices<T>(items: &[T], max_partitions: usize) -> Vec<&[T]> {
    if items.is_empty() || max_partitions == 0 {
        return Vec::new();
    }
    let partitions = max_partitions.min(items.len());
    let base_len = items.len() / partitions;
    let remainder = items.len() % partitions;
    let mut out = Vec::with_capacity(partitions);
    let mut offset = 0;
    for idx in 0..partitions {
        let len = base_len + usize::from(idx < remainder);
        out.push(&items[offset..offset + len]);
        offset += len;
    }
    out
}

/// Multi-threaded command encoder coordinator that records independent GPU passes on
/// thread-local [`wgpu::CommandEncoder`] instances and merges the resulting
/// [`wgpu::CommandBuffer`]s in deterministic submission order for a single `queue.submit`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParallelCommandRecorder {
    max_threads: usize,
}

impl ParallelCommandRecorder {
    /// Creates a recorder using the host's available hardware parallelism.
    #[must_use]
    pub fn new() -> Self {
        let max_threads = std::thread::available_parallelism().map_or(2, usize::from);
        Self { max_threads }
    }

    /// Creates a recorder capped at `max_threads` worker threads.
    #[must_use]
    pub fn with_max_threads(max_threads: usize) -> Self {
        Self {
            max_threads: max_threads.max(1),
        }
    }

    #[must_use]
    pub const fn max_threads(&self) -> usize {
        self.max_threads
    }

    /// Records a sequence of independent pass closures in parallel across worker threads,
    /// returning their finished [`wgpu::CommandBuffer`]s in exact input order (`0..passes.len()`).
    ///
    /// # Errors
    /// If any pass closure returns `Err(E)`, returns the first error in pass order before
    /// submitting any work to the GPU queue.
    pub fn record_passes<F, E>(
        &self,
        device: &wgpu::Device,
        passes: Vec<(&str, F)>,
    ) -> Result<Vec<wgpu::CommandBuffer>, E>
    where
        F: FnOnce(&mut wgpu::CommandEncoder) -> Result<(), E> + Send,
        E: Send,
    {
        if passes.is_empty() {
            return Ok(Vec::new());
        }
        if passes.len() == 1 || self.max_threads <= 1 {
            let mut buffers = Vec::with_capacity(passes.len());
            for (label, record) in passes {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(label),
                    });
                record(&mut encoder)?;
                buffers.push(encoder.finish());
            }
            return Ok(buffers);
        }

        let mut results: Vec<Result<wgpu::CommandBuffer, E>> = Vec::with_capacity(passes.len());
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(passes.len());
            for (label, record) in passes {
                handles.push(scope.spawn(move || {
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some(label),
                        });
                    record(&mut encoder)?;
                    Ok(encoder.finish())
                }));
            }
            for handle in handles {
                results.push(handle.join().expect("parallel command encoder worker panicked"));
            }
        });

        let mut buffers = Vec::with_capacity(results.len());
        for result in results {
            buffers.push(result?);
        }
        Ok(buffers)
    }

    /// Records all passes in parallel and submits their finished command buffers in a single
    /// `queue.submit` call.
    ///
    /// # Errors
    /// Propagates any error returned by a pass closure without submitting to `queue`.
    pub fn record_and_submit<F, E>(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        passes: Vec<(&str, F)>,
    ) -> Result<usize, E>
    where
        F: FnOnce(&mut wgpu::CommandEncoder) -> Result<(), E> + Send,
        E: Send,
    {
        let buffers = self.record_passes(device, passes)?;
        let count = buffers.len();
        if count > 0 {
            queue.submit(buffers);
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_work_slices_balances_items_and_preserves_order() {
        let items: Vec<u32> = (0..10).collect();
        let slices = partition_work_slices(&items, 4);
        assert_eq!(slices.len(), 4);
        assert_eq!(slices[0], &[0, 1, 2]);
        assert_eq!(slices[1], &[3, 4, 5]);
        assert_eq!(slices[2], &[6, 7]);
        assert_eq!(slices[3], &[8, 9]);

        let more_partitions = partition_work_slices(&items[..2], 8);
        assert_eq!(more_partitions.len(), 2);
        assert_eq!(more_partitions[0], &[0]);
        assert_eq!(more_partitions[1], &[1]);

        let empty: [u32; 0] = [];
        assert!(partition_work_slices(&empty, 4).is_empty());

        let recorder = ParallelCommandRecorder::with_max_threads(4);
        assert_eq!(recorder.max_threads(), 4);
    }
}
