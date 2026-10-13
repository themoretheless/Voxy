//! Shared resident compute chains; numerical ABIs and admission remain typed owners.
use crate::{ComputeDispatch, ComputeError, ComputeJob, ComputeProgram, PendingComputeReadback};
#[derive(Debug)]
pub(crate) struct ComputeChainDispatch {
    snapshot: ComputeDispatch,
    jobs: Vec<ComputeJob>,
    recycle: bool,
    sizes: Vec<usize>,
    device: wgpu::Device,
    reuses: usize,
}
pub(crate) struct PendingComputeChain {
    read: PendingComputeReadback,
    jobs: Option<Vec<ComputeJob>>,
    recycle: bool,
    sizes: Vec<usize>,
    device: wgpu::Device,
    reuses: usize,
    ready: Option<ComputeChainWorkspace>,
}
impl std::fmt::Debug for PendingComputeChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingComputeChain")
            .field("jobs_retained", &self.jobs.is_some())
            .finish_non_exhaustive()
    }
}
/// Exclusively owned completed jobs. Only a successfully completed read can
/// return these to a caller; an in-flight or abandoned dispatch never can.
#[derive(Debug)]
pub(crate) struct ComputeChainWorkspace {
    jobs: Vec<ComputeJob>,
    sizes: Vec<usize>,
    device: wgpu::Device,
    pub reuses: usize,
}
/// Ordered kernels sharing a single storage allocation in this stage.
/// Transfers address the preceding stage's storage. ABI compatibility is owned
/// by the typed pipeline, rather than inferred by this transport layer.
pub(crate) struct ResidentStage<'a> {
    pub bytes: &'a [u8],
    pub passes: &'a [(&'a ComputeProgram, [u32; 3])],
    pub copies: &'a [(u64, u64, u64)],
}
fn validate_transfers(
    source: usize,
    destination: usize,
    copies: &[(u64, u64, u64)],
) -> Result<(), ComputeError> {
    let mut end = 0;
    for &(from, to, length) in copies {
        if length == 0
            || (from | to | length) % 4 != 0
            || to < end
            || from.checked_add(length).is_none_or(|v| v > source as u64)
            || to
                .checked_add(length)
                .is_none_or(|v| v > destination as u64)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        end = to + length;
    }
    Ok(())
}
/// Records all stages and one gathered snapshot. Caller discards its encoder on
/// error and owns submission/polling. No stage performs host reads or submits.
pub(crate) fn encode_chain(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stages: &[ResidentStage<'_>],
) -> Result<ComputeChainDispatch, ComputeError> {
    encode_chain_inner(device, encoder, stages, None, None, false)
}
pub(crate) fn encode_chain_reusing(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    stages: &[ResidentStage<'_>],
    workspace: Option<ComputeChainWorkspace>,
) -> Result<ComputeChainDispatch, ComputeError> {
    encode_chain_inner(device, encoder, stages, Some(queue), workspace, true)
}
fn encode_chain_inner(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stages: &[ResidentStage<'_>],
    queue: Option<&wgpu::Queue>,
    workspace: Option<ComputeChainWorkspace>,
    recycle: bool,
) -> Result<ComputeChainDispatch, ComputeError> {
    if stages.is_empty()
        || !stages[0].copies.is_empty()
        || stages.iter().any(|s| s.passes.is_empty())
    {
        return Err(ComputeError::InvalidBuffer);
    }
    // Validate the entire transfer plan before recording any commands.
    for pair in stages.windows(2) {
        validate_transfers(pair[0].bytes.len(), pair[1].bytes.len(), pair[1].copies)?;
    }
    let sizes = stages.iter().map(|s| s.bytes.len()).collect::<Vec<_>>();
    let previous_reuses = workspace.as_ref().map_or(0, |w| w.reuses);
    let matching = workspace
        .as_ref()
        .is_some_and(|w| w.device == *device && w.sizes == sizes);
    let (mut jobs, reuses) = if matching {
        let workspace = workspace.unwrap();
        let mut jobs = workspace.jobs;
        // Reset the first kernel as previous multi-kernel stages may have ended
        // with another program. Validate all program/device bindings before writes.
        for (job, stage) in jobs.iter_mut().zip(stages) {
            job.use_program(stage.passes[0].0)?;
        }
        let queue = queue.ok_or(ComputeError::InvalidBuffer)?;
        for (job, stage) in jobs.iter().zip(stages) {
            queue.write_buffer(job.buffer(), 0, stage.bytes);
        }
        (jobs, previous_reuses + 1)
    } else {
        drop(workspace);
        (
            stages
                .iter()
                .map(|s| s.passes[0].0.create_job(device, s.bytes))
                .collect::<Result<Vec<_>, _>>()?,
            previous_reuses,
        )
    };
    for (index, stage) in stages.iter().enumerate() {
        if index != 0 {
            for &(from, to, length) in stage.copies {
                encoder.copy_buffer_to_buffer(
                    jobs[index - 1].buffer(),
                    from,
                    jobs[index].buffer(),
                    to,
                    length,
                );
            }
        }
        for (pass, &(program, groups)) in stage.passes.iter().enumerate() {
            if pass != 0 {
                jobs[index].use_program(program)?;
            }
            jobs[index].encode_step(encoder, groups)?;
        }
    }
    let mut total = 0u64;
    let mut ranges = Vec::with_capacity(jobs.len());
    for (job, stage) in jobs.iter().zip(stages) {
        let length = stage.bytes.len() as u64;
        ranges.push((job.buffer(), 0, total, length));
        total = total
            .checked_add(length)
            .ok_or(ComputeError::InvalidBuffer)?;
    }
    let snapshot = ComputeDispatch::gather_buffers(device, encoder, &ranges, total)?;
    Ok(ComputeChainDispatch {
        snapshot,
        jobs,
        recycle,
        sizes,
        device: device.clone(),
        reuses,
    })
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_two_stage(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    first: &ComputeProgram,
    first_bytes: &[u8],
    first_groups: [u32; 3],
    second: &ComputeProgram,
    second_bytes: &[u8],
    second_groups: [u32; 3],
    copies: &[(u64, u64, u64)],
) -> Result<ComputeChainDispatch, ComputeError> {
    encode_chain(
        device,
        encoder,
        &[
            ResidentStage {
                bytes: first_bytes,
                passes: &[(first, first_groups)],
                copies: &[],
            },
            ResidentStage {
                bytes: second_bytes,
                passes: &[(second, second_groups)],
                copies,
            },
        ],
    )
}
impl ComputeChainDispatch {
    pub(crate) fn begin_read(self) -> PendingComputeChain {
        PendingComputeChain {
            read: self.snapshot.begin_read(),
            jobs: Some(self.jobs),
            recycle: self.recycle,
            sizes: self.sizes,
            device: self.device,
            reuses: self.reuses,
            ready: None,
        }
    }
}
impl PendingComputeChain {
    pub(crate) fn try_read(&mut self) -> Result<Option<Vec<u8>>, ComputeError> {
        match self.read.try_read() {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) => {
                if self.recycle {
                    self.ready = Some(ComputeChainWorkspace {
                        jobs: self.jobs.take().unwrap(),
                        sizes: self.sizes.clone(),
                        device: self.device.clone(),
                        reuses: self.reuses,
                    });
                } else {
                    self.jobs.take();
                }
                Ok(Some(bytes))
            }
            Err(error) => {
                self.jobs.take();
                self.ready.take();
                Err(error)
            }
        }
    }
    pub(crate) fn take_workspace(&mut self) -> Option<ComputeChainWorkspace> {
        self.ready.take()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_plan_rejects_overlap_alignment_overflow_and_bounds() {
        for copies in [
            vec![(0, 0, 0)],
            vec![(1, 0, 4)],
            vec![(0, 2, 4)],
            vec![(0, 0, 3)],
            vec![(0, 0, 8), (0, 4, 4)],
            vec![(0, 8, 4), (0, 0, 4)],
            vec![(12, 0, 8)],
            vec![(0, 12, 8)],
            vec![(u64::MAX - 3, 0, 8)],
            vec![(0, u64::MAX - 3, 8)],
        ] {
            assert!(validate_transfers(16, 16, &copies).is_err(), "{copies:?}");
        }
        validate_transfers(16, 16, &[(8, 0, 8), (0, 8, 8)]).unwrap();
    }
    #[test]
    #[ignore = "requires actual GPU; four resident jobs, five kernels, one final read"]
    fn gpu_multi_stage_chain_retains_jobs_and_transfers_in_order() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let source = "@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(1) fn cs_main() { data[1]=data[0]+1u; }";
        let second = "@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(1) fn cs_main() { data[1]=data[1]*3u; }";
        let program = pollster::block_on(ComputeProgram::new(&device, source)).unwrap();
        let multiply = pollster::block_on(ComputeProgram::new(&device, second)).unwrap();
        let initial = [7u32, u32::MAX];
        let empty = [u32::MAX; 2];
        let bytes = bytemuck::cast_slice(&empty);
        let pass = [(&program, [1, 1, 1])];
        let two_passes = [(&program, [1, 1, 1]), (&multiply, [1, 1, 1])];
        let copy = [(4, 0, 4)];
        let budget = crate::ComputeMemoryBudget::for_device(&device);
        let pool = crate::ComputeReadbackPool::for_device(&device);
        let before = budget.stats().allocated_bytes;
        let before_pool = pool.stats();
        let mut encoder = device.create_command_encoder(&Default::default());
        let dispatch = encode_chain(
            &device,
            &mut encoder,
            &[
                ResidentStage {
                    bytes: bytemuck::cast_slice(&initial),
                    passes: &pass,
                    copies: &[],
                },
                ResidentStage {
                    bytes,
                    passes: &pass,
                    copies: &copy,
                },
                ResidentStage {
                    bytes,
                    passes: &two_passes,
                    copies: &copy,
                },
                ResidentStage {
                    bytes,
                    passes: &pass,
                    copies: &copy,
                },
            ],
        )
        .unwrap();
        assert_eq!(budget.stats().allocated_bytes, before + 32);
        let after_pool = pool.stats();
        assert_eq!(
            after_pool.creations + after_pool.reuses,
            before_pool.creations + before_pool.reuses + 1
        );
        let submission = queue.submit([encoder.finish()]);
        let mut read = dispatch.begin_read();
        assert_eq!(budget.stats().allocated_bytes, before + 32);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let result = read.try_read().unwrap().unwrap();
        let words = result
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(words, [7, 8, 8, 9, 9, 30, 30, 31]);
        assert_eq!(budget.stats().allocated_bytes, before);
        assert!(matches!(read.try_read(), Err(ComputeError::Consumed)));
        eprintln!(
            "RESIDENT CHAIN jobs=4 kernels=5 submissions=1 staging_leases=1 intermediate_cpu_reads=0 storage_released=true"
        );
    }
    #[test]
    #[ignore = "requires actual GPU; completed storage reuse with changing bytes and buffer shape"]
    fn gpu_chain_reuses_only_completed_jobs_and_resizes_without_stale_inputs() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program=pollster::block_on(ComputeProgram::new(&device,
            "@group(0) @binding(0) var<storage,read_write> data:array<u32>; @compute @workgroup_size(1) fn cs_main() {data[1]=data[0]+1u;}")).unwrap();
        let budget = crate::ComputeMemoryBudget::for_device(&device);
        let baseline = budget.stats().allocated_bytes;
        let mut workspace = None;
        for (iteration, size) in [2, 2, 3, 3].into_iter().enumerate() {
            let mut first = vec![u32::MAX; size];
            first[0] = 7 + iteration as u32;
            let second = vec![u32::MAX; size];
            let mut encoder = device.create_command_encoder(&Default::default());
            let dispatch = encode_chain_reusing(
                &device,
                &queue,
                &mut encoder,
                &[
                    ResidentStage {
                        bytes: bytemuck::cast_slice(&first),
                        passes: &[(&program, [1, 1, 1])],
                        copies: &[],
                    },
                    ResidentStage {
                        bytes: bytemuck::cast_slice(&second),
                        passes: &[(&program, [1, 1, 1])],
                        copies: &[(4, 0, 4)],
                    },
                ],
                workspace.take(),
            )
            .unwrap();
            let submission = queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            assert!(pending.take_workspace().is_none());
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let output = pending.try_read().unwrap().unwrap();
            let values = output
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .collect::<Vec<_>>();
            assert_eq!(values[0], 7 + iteration as u32);
            assert_eq!(values[1], 8 + iteration as u32);
            assert_eq!(values[size], 8 + iteration as u32);
            assert_eq!(values[size + 1], 9 + iteration as u32);
            if size == 3 {
                assert_eq!(values[2], u32::MAX);
                assert_eq!(values[5], u32::MAX);
            }
            workspace = pending.take_workspace();
            assert!(workspace.is_some());
            assert!(pending.take_workspace().is_none());
            assert_eq!(workspace.as_ref().unwrap().reuses, [0, 1, 1, 2][iteration]);
            assert_eq!(budget.stats().allocated_bytes, baseline + (size * 8) as u64);
        }
        drop(workspace);
        assert_eq!(budget.stats().allocated_bytes, baseline);
        eprintln!(
            "RESIDENT REUSE submissions=4 reuses=2 resized_once=true changing_inputs=true storage_released=true"
        );
    }
}
