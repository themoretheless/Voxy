use voxy_render::{ComputeError, ComputeProgram, PendingComputeReadback};

/// Local adjacency graph; None amount is an impermeable registered block.
/// Missing indices are errors only when the ordered solver actually reads them.
#[derive(Clone, Copy, Debug)]
pub struct WaterNode {
    pub amount: Option<u8>,
    /// Below, -X, +X, -Z, +Z, in CPU liquid solver order.
    /// `Some(u32::MAX - 1)` denotes a coordinate-overflow edge; it errors
    /// before consuming a sample only when traversed by the solver.
    pub neighbors: [Option<u32>; 5],
}
/// Classification of a captured graph node before device execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaterNodeStatus {
    Loaded,
    Unavailable,
    Unknown,
}

#[derive(Debug)]
pub enum WaterComputeError {
    InvalidInput,
    MissingNeighbor,
    /// Captured unavailable cell reached by the solver; index identifies its position.
    UnavailableNode(u32),
    CoordinateOverflow,
    UnknownNode(u32),
    SampleBudget,
    WriteBudget,
    InvalidOutput,
    Compute(ComputeError),
    Cuda(voxy_cuda::CudaError),
}
impl std::fmt::Display for WaterComputeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cuda(error) => write!(f, "CUDA water error: {error}"),
            Self::Compute(error) => write!(f, "GPU water compute error: {error}"),
            error => write!(f, "GPU water error: {error:?}"),
        }
    }
}
impl std::error::Error for WaterComputeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cuda(error) => Some(error),
            Self::Compute(error) => Some(error),
            _ => None,
        }
    }
}

/// Read provenance for revision-checked world publication, in node order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaterTransferResult {
    pub amounts: Vec<Option<u8>>,
    pub sampled: Vec<bool>,
}

/// Submitted device work with nonblocking, single-consumption readback.
/// Native callers drive device polling; browser callers yield to the event loop.
#[derive(Debug)]
pub struct PendingWaterTransfer {
    pending: PendingComputeReadback,
    words: Vec<u32>,
    nodes: Vec<WaterNode>,
    submission: wgpu::SubmissionIndex,
}
impl PendingWaterTransfer {
    /// Returns `None` while mapping is pending, without waiting on the GPU.
    /// # Errors
    /// Reports mapping/solver/validation errors or a consumed readback.
    pub fn try_result(&mut self) -> Result<Option<WaterTransferResult>, WaterComputeError> {
        self.pending
            .try_read()
            .map_err(WaterComputeError::Compute)?
            .map(|bytes| decode(&bytes, &self.words, &self.nodes))
            .transpose()
    }
}

#[derive(Debug)]
pub struct WaterTransferProgram {
    device: wgpu::Device,
    program: ComputeProgram,
}
impl WaterTransferProgram {
    /// Creates an integer, ordered liquid transfer shader on this device.
    /// # Errors
    /// Rejects missing compute support or invalid pipeline creation.
    pub async fn new(device: &wgpu::Device) -> Result<Self, WaterComputeError> {
        Ok(Self {
            device: device.clone(),
            program: ComputeProgram::new(device, include_str!("water.wgsl"))
                .await
                .map_err(WaterComputeError::Compute)?,
        })
    }

    /// Drives native mapping callbacks without waiting for device completion.
    /// Browser callers must yield to their event loop instead.
    /// # Errors
    /// Returns a device polling failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_native(&self) -> Result<(), WaterComputeError> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map(|_| ())
            .map_err(|failure| {
                WaterComputeError::Compute(ComputeError::Mapping(failure.to_string()))
            })
    }

    /// Executes a bounded graph tick. Active node indices must be sorted and
    /// unique in the caller's canonical voxel order. Native synchronous readback;
    /// queue must belong to the program's device. World publication is external.
    /// This first pass is sequential on the GPU to preserve CPU transfer ordering.
    /// Supports at most 131072 graph nodes and 16384 active nodes per tick.
    /// # Errors
    /// Rejects malformed graphs, missing required neighbors and read/write budgets.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<Vec<Option<u8>>, WaterComputeError> {
        Ok(self
            .step_detailed(
                queue,
                nodes,
                active,
                downward,
                horizontal,
                max_samples,
                max_writes,
            )?
            .amounts)
    }

    /// Executes the same transfer with exact lazy-read provenance for world
    /// revision checks. No transaction is published by this graph API.
    /// # Errors
    /// Returns the same validation/compute/budget errors as `step`.
    #[allow(clippy::too_many_arguments)]
    pub fn step_detailed(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<WaterTransferResult, WaterComputeError> {
        self.step_available(
            queue,
            nodes,
            active,
            None,
            downward,
            horizontal,
            max_samples,
            max_writes,
        )
    }

    /// Executes a graph with unavailable nodes represented explicitly.
    /// Unavailable nodes retain their graph identity for read-budget accounting;
    /// an error is raised only if the ordered shader reads one. Their amounts
    /// must be `None` and cannot be changed or treated as registered solids.
    /// # Errors
    /// Returns graph, availability-shape, compute and lazy-read errors.
    #[allow(clippy::too_many_arguments)]
    pub fn step_detailed_available(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        available: &[bool],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<WaterTransferResult, WaterComputeError> {
        let status: Vec<_> = available
            .iter()
            .map(|&loaded| {
                if loaded {
                    WaterNodeStatus::Loaded
                } else {
                    WaterNodeStatus::Unavailable
                }
            })
            .collect();
        self.step_detailed_status(
            queue,
            nodes,
            active,
            &status,
            downward,
            horizontal,
            max_samples,
            max_writes,
        )
    }

    /// Executes a graph with lazy unavailable and unknown-state reads.
    /// # Errors
    /// Reports `UnknownNode` with the graph index of the first unknown read,
    /// after checking its sample budget; rejects malformed status masks.
    #[allow(clippy::too_many_arguments)]
    pub fn step_detailed_status(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        status: &[WaterNodeStatus],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<WaterTransferResult, WaterComputeError> {
        self.step_available(
            queue,
            nodes,
            active,
            Some(status),
            downward,
            horizontal,
            max_samples,
            max_writes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn step_available(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        available: Option<&[WaterNodeStatus]>,
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<WaterTransferResult, WaterComputeError> {
        let mut pending = self.begin_available(
            queue,
            nodes,
            active,
            available,
            downward,
            horizontal,
            max_samples,
            max_writes,
        )?;
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(pending.submission.clone()),
                timeout: None,
            })
            .map_err(|e| WaterComputeError::Compute(ComputeError::Mapping(e.to_string())))?;
        pending
            .try_result()?
            .ok_or(WaterComputeError::InvalidOutput)
    }

    /// Submits a graph tick and begins mapping without blocking device polling.
    /// Browser callers must yield to their event loop before retrying readback;
    /// native callers must drive polling on this program's device.
    /// # Errors
    /// Rejects invalid graph/status/budget inputs before command submission.
    /// Solver and mapping errors are returned later by `try_result`.
    #[allow(clippy::too_many_arguments)]
    pub fn begin_step_detailed_status(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        status: &[WaterNodeStatus],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<PendingWaterTransfer, WaterComputeError> {
        self.begin_available(
            queue,
            nodes,
            active,
            Some(status),
            downward,
            horizontal,
            max_samples,
            max_writes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn begin_available(
        &self,
        queue: &wgpu::Queue,
        nodes: &[WaterNode],
        active: &[u32],
        available: Option<&[WaterNodeStatus]>,
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<PendingWaterTransfer, WaterComputeError> {
        let words = pack_graph(
            nodes,
            active,
            available,
            downward,
            horizontal,
            max_samples,
            max_writes,
        )?;
        let job = self
            .program
            .create_job(&self.device, bytemuck::cast_slice(&words))
            .map_err(WaterComputeError::Compute)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = job
            .encode(&mut encoder, [1, 1, 1])
            .map_err(WaterComputeError::Compute)?;
        let submission = queue.submit([encoder.finish()]);
        Ok(PendingWaterTransfer {
            pending: dispatch.begin_read(),
            words,
            nodes: nodes.to_vec(),
            submission,
        })
    }
}

fn pack_graph(
    nodes: &[WaterNode],
    active: &[u32],
    available: Option<&[WaterNodeStatus]>,
    downward: u8,
    horizontal: u8,
    max_samples: u32,
    max_writes: u32,
) -> Result<Vec<u32>, WaterComputeError> {
    if let Some(mask) = available
        && (mask.len() != nodes.len()
            || mask
                .iter()
                .zip(nodes)
                .any(|(&loaded, node)| loaded != WaterNodeStatus::Loaded && node.amount.is_some()))
    {
        return Err(WaterComputeError::InvalidInput);
    }
    let count = u32::try_from(nodes.len()).map_err(|_| WaterComputeError::InvalidInput)?;
    let active_count = u32::try_from(active.len()).map_err(|_| WaterComputeError::InvalidInput)?;
    if count == 0
        || count > 131_072
        || active_count > 16_384
        || count
            .checked_mul(8)
            .and_then(|n| n.checked_add(8))
            .and_then(|n| n.checked_add(active_count))
            .is_none()
        || !(1..=8).contains(&downward)
        || !(1..=8).contains(&horizontal)
        || max_samples == 0
        || max_writes == 0
        || active.iter().any(|&i| i >= count)
        || active.windows(2).any(|p| p[0] >= p[1])
        || nodes.iter().any(|node| {
            node.amount.is_some_and(|a| a > 8)
                || node
                    .neighbors
                    .iter()
                    .flatten()
                    .any(|&i| i >= count && i != u32::MAX - 1)
        })
    {
        return Err(WaterComputeError::InvalidInput);
    }
    let mut words = vec![
        count,
        active_count,
        u32::from(downward),
        u32::from(horizontal),
        max_samples,
        0,
        0,
        max_writes,
    ];
    for (index, node) in nodes.iter().enumerate() {
        let amount = match available.map_or(WaterNodeStatus::Loaded, |mask| mask[index]) {
            WaterNodeStatus::Loaded => node.amount.map_or(9, u32::from),
            WaterNodeStatus::Unavailable => 10,
            WaterNodeStatus::Unknown => 11,
        };
        words.extend([amount, amount, 0]);
        words.extend(node.neighbors.map(|i| i.unwrap_or(u32::MAX)));
    }
    words.extend(active);
    Ok(words)
}

/// Synchronous CUDA water graph solver; world publication remains revision checked.
#[derive(Debug)]
pub struct CudaWaterTransferProgram {
    compute: std::sync::Arc<voxy_cuda::CudaCompute>,
}
impl CudaWaterTransferProgram {
    #[must_use]
    pub fn new(compute: std::sync::Arc<voxy_cuda::CudaCompute>) -> Self {
        Self { compute }
    }

    /// Executes the same validated graph and decodes the same status/provenance
    /// as WGSL. No CPU solver fallback. Failed partial transfers are never returned
    /// as a successful `WaterTransferResult`.
    /// # Errors
    /// Returns graph, solver, CUDA driver or output validation failures.
    #[allow(clippy::too_many_arguments)]
    pub fn step_detailed_status(
        &self,
        nodes: &[WaterNode],
        active: &[u32],
        status: &[WaterNodeStatus],
        downward: u8,
        horizontal: u8,
        max_samples: u32,
        max_writes: u32,
    ) -> Result<WaterTransferResult, WaterComputeError> {
        let words = pack_graph(
            nodes,
            active,
            Some(status),
            downward,
            horizontal,
            max_samples,
            max_writes,
        )?;
        let output = self
            .compute
            .water_graph(&words)
            .map_err(WaterComputeError::Cuda)?;
        decode(bytemuck::cast_slice(&output), &words, nodes)
    }
}

fn decode(
    bytes: &[u8],
    expected_words: &[u32],
    nodes: &[WaterNode],
) -> Result<WaterTransferResult, WaterComputeError> {
    if bytes.len() != expected_words.len() * 4 {
        return Err(WaterComputeError::InvalidOutput);
    }
    let result: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    match result[5] {
        0 => {}
        1 => return Err(WaterComputeError::MissingNeighbor),
        2 => return Err(WaterComputeError::SampleBudget),
        3 => return Err(WaterComputeError::WriteBudget),
        4 => return Err(WaterComputeError::CoordinateOverflow),
        6 => {
            let index = usize::try_from(result[6]).map_err(|_| WaterComputeError::InvalidOutput)?;
            if index >= nodes.len() || expected_words[8 + index * 8] != 10 {
                return Err(WaterComputeError::InvalidOutput);
            }
            return Err(WaterComputeError::UnavailableNode(result[6]));
        }
        5 => {
            let index = usize::try_from(result[6]).map_err(|_| WaterComputeError::InvalidOutput)?;
            if index >= nodes.len() || expected_words[8 + index * 8] != 11 {
                return Err(WaterComputeError::InvalidOutput);
            }
            return Err(WaterComputeError::UnknownNode(result[6]));
        }
        _ => return Err(WaterComputeError::InvalidOutput),
    }
    // Structural fields are immutable across dispatch. In particular, an
    // unavailable node cannot become a solid or be marked read on success.
    if result[8..8 + nodes.len() * 8]
        .chunks_exact(8)
        .zip(expected_words[8..8 + nodes.len() * 8].chunks_exact(8))
        .any(|(row, initial)| {
            row[1] != initial[1]
                || row[3..] != initial[3..]
                || (initial[0] >= 10 && (row[0] != initial[0] || row[2] != 0))
        })
    {
        return Err(WaterComputeError::InvalidOutput);
    }
    let amounts: Vec<_> = result[8..8 + nodes.len() * 8]
        .chunks_exact(8)
        .map(|row| match row[0] {
            0..=8 => Ok(Some(
                u8::try_from(row[0]).map_err(|_| WaterComputeError::InvalidOutput)?,
            )),
            9..=11 => Ok(None),
            _ => Err(WaterComputeError::InvalidOutput),
        })
        .collect::<Result<_, _>>()?;
    if nodes
        .iter()
        .zip(&amounts)
        .any(|(node, amount)| node.amount.is_none() != amount.is_none())
    {
        return Err(WaterComputeError::InvalidOutput);
    }
    if nodes
        .iter()
        .map(|node| u64::from(node.amount.unwrap_or(0)))
        .sum::<u64>()
        != amounts
            .iter()
            .map(|a| u64::from(a.unwrap_or(0)))
            .sum::<u64>()
    {
        return Err(WaterComputeError::InvalidOutput);
    }
    let sampled: Vec<_> = result[8..8 + nodes.len() * 8]
        .chunks_exact(8)
        .map(|row| match row[2] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WaterComputeError::InvalidOutput),
        })
        .collect::<Result<_, _>>()?;
    if sampled.iter().filter(|&&read| read).count()
        != usize::try_from(result[6]).map_err(|_| WaterComputeError::InvalidOutput)?
        || nodes
            .iter()
            .zip(&amounts)
            .zip(&sampled)
            .any(|((node, amount), &read)| node.amount != *amount && !read)
    {
        return Err(WaterComputeError::InvalidOutput);
    }
    Ok(WaterTransferResult { amounts, sampled })
}

#[cfg(test)]
mod availability_tests {
    use super::*;

    #[test]
    fn unavailable_rows_must_stay_unread_and_structurally_unchanged() {
        let nodes = [WaterNode {
            amount: None,
            neighbors: [None; 5],
        }];
        let original = [
            1,
            0,
            8,
            4,
            1,
            0,
            0,
            1,
            10,
            10,
            0,
            u32::MAX,
            u32::MAX,
            u32::MAX,
            u32::MAX,
            u32::MAX,
        ];
        let mut unavailable = original;
        unavailable[5] = 6;
        unavailable[6] = 0;
        assert!(matches!(
            decode(bytemuck::cast_slice(&unavailable), &original, &nodes),
            Err(WaterComputeError::UnavailableNode(0))
        ));
        unavailable[6] = 1;
        assert!(matches!(
            decode(bytemuck::cast_slice(&unavailable), &original, &nodes),
            Err(WaterComputeError::InvalidOutput)
        ));
        let result = decode(bytemuck::cast_slice(&original), &original, &nodes).unwrap();
        assert_eq!(result.amounts, [None]);
        assert_eq!(result.sampled, [false]);
        for (index, value) in [(8, 9), (9, 9), (10, 1), (11, 0)] {
            let mut corrupted = original;
            corrupted[index] = value;
            assert!(matches!(
                decode(bytemuck::cast_slice(&corrupted), &original, &nodes),
                Err(WaterComputeError::InvalidOutput)
            ));
        }
    }
}
