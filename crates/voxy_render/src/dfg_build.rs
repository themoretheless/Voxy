//! Bounded, nonblocking experimental atlas construction and resource handoff.
//! Oracle samples remain required; sampled admission is not a whole-cell bound.
use super::{
    GGX_ENERGY_GPU_BATCH_LIMIT, GgxDfgLut, GgxEnergyBatchKey, GgxEnergyBatchStage,
    GgxEnergyGpuPrecompute, GgxEnergyPatchError, GgxEnergyRefinementAssessment,
    GgxEnergyRefinementDecision, GgxEnergyRefinementError, GgxEnergyRefinementObservation,
    GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy, PendingGgxEnergyBatch,
};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use voxy_gpu_core::ComputeError;
#[derive(Debug)]
pub enum GgxEnergyBuildError {
    Compute(ComputeError),
    Refinement(GgxEnergyRefinementError),
    Patch(GgxEnergyPatchError),
    InvalidBudget,
    StaleGeneration,
    Consumed,
    SampledRejected,
    InvalidGeneration,
    OracleCountMismatch { expected: usize, actual: usize },
}
impl std::fmt::Display for GgxEnergyBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "experimental GGX atlas build: {self:?}")
    }
}
impl std::error::Error for GgxEnergyBuildError {}
impl From<ComputeError> for GgxEnergyBuildError {
    fn from(e: ComputeError) -> Self {
        Self::Compute(e)
    }
}
impl From<GgxEnergyRefinementError> for GgxEnergyBuildError {
    fn from(e: GgxEnergyRefinementError) -> Self {
        Self::Refinement(e)
    }
}
impl From<GgxEnergyPatchError> for GgxEnergyBuildError {
    fn from(e: GgxEnergyPatchError) -> Self {
        Self::Patch(e)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct GgxEnergyOracleSample {
    pub uv: [f32; 2],
    pub reference: [f64; 3],
    pub uncertainty: [f64; 3],
}
#[derive(Debug, Clone, Copy)]
pub struct GgxEnergyBuildBudget {
    pub max_nodes: usize,
    pub max_probes: usize,
    pub max_candidate_atlas_bytes: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgxEnergyBuildPhase {
    Nodes,
    Probes,
    Upload,
    Complete,
    Failed,
}
#[derive(Debug, Clone, Copy)]
pub struct GgxEnergyBuildProgress {
    pub phase: GgxEnergyBuildPhase,
    pub refinement: u32,
    pub completed_nodes: usize,
    pub total_nodes: usize,
    pub completed_probes: usize,
    pub total_probes: usize,
    pub waiting_for_capacity: bool,
}
struct Upload {
    atlas: GgxDfgLut,
    completion: Receiver<()>,
    report: GgxEnergyRefinementAssessment,
}
impl std::fmt::Debug for Upload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upload").finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub struct GgxEnergyAtlasBuild {
    device: wgpu::Device,
    queue: wgpu::Queue,
    generation: u64,
    regions: Vec<[f32; 4]>,
    oracle: Vec<GgxEnergyOracleSample>,
    policy: GgxEnergyRefinementPolicy,
    budget: GgxEnergyBuildBudget,
    plan: GgxEnergyRefinementPlan,
    phase: GgxEnergyBuildPhase,
    offset: usize,
    completed_nodes: usize,
    moments: Vec<[f32; 4]>,
    observations: Vec<GgxEnergyRefinementObservation>,
    pending: Option<(GgxEnergyBatchKey, PendingGgxEnergyBatch)>,
    upload: Option<Upload>,
    waiting: bool,
    history: Vec<(u32, GgxEnergyRefinementAssessment)>,
}
/// Upload-complete, sampled experimental candidate. Only the builder creates
/// this token; it is not a scientific whole-domain accuracy certificate.
#[derive(Debug)]
pub struct ReadyGgxEnergyAtlas {
    generation: u64,
    atlas: GgxDfgLut,
    report: GgxEnergyRefinementAssessment,
}
impl ReadyGgxEnergyAtlas {
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
    #[must_use]
    pub fn atlas(&self) -> &GgxDfgLut {
        &self.atlas
    }
    #[must_use]
    pub fn assessment(&self) -> &GgxEnergyRefinementAssessment {
        &self.report
    }
}
/// Transactional resource ownership. Scene bindings retain their old views
/// until explicitly rebuilt; this slot does not mutate a scene implicitly.
#[derive(Debug)]
pub struct GgxEnergyAtlasSlot {
    current: GgxDfgLut,
    requested: u64,
    installed: u64,
}
impl GgxEnergyAtlasSlot {
    pub fn new(current: GgxDfgLut, generation: u64) -> Self {
        Self {
            current,
            requested: generation,
            installed: generation,
        }
    }
    pub fn request_generation(&mut self, generation: u64) -> Result<(), GgxEnergyBuildError> {
        if generation <= self.requested {
            return Err(GgxEnergyBuildError::InvalidGeneration);
        }
        self.requested = generation;
        Ok(())
    }
    #[must_use]
    pub fn requested_generation(&self) -> u64 {
        self.requested
    }
    #[must_use]
    pub fn installed_generation(&self) -> u64 {
        self.installed
    }
    #[must_use]
    pub fn current(&self) -> &GgxDfgLut {
        &self.current
    }
    /// All checks precede replacement. Caller may retire or retain the old owner.
    pub fn try_install(
        &mut self,
        ready: ReadyGgxEnergyAtlas,
    ) -> Result<GgxDfgLut, GgxEnergyBuildError> {
        if ready.generation != self.requested {
            return Err(GgxEnergyBuildError::StaleGeneration);
        }
        if !self.current.belongs_to(&ready.atlas.device) {
            return Err(ComputeError::DeviceMismatch.into());
        }
        self.installed = ready.generation;
        Ok(std::mem::replace(&mut self.current, ready.atlas))
    }
}
impl GgxEnergyAtlasBuild {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        regions: Vec<[f32; 4]>,
        oracle: Vec<GgxEnergyOracleSample>,
        policy: GgxEnergyRefinementPolicy,
        budget: GgxEnergyBuildBudget,
    ) -> Result<Self, GgxEnergyBuildError> {
        if budget.max_nodes == 0
            || budget.max_probes == 0
            || budget.max_candidate_atlas_bytes == 0
            || budget.max_nodes > u32::MAX as usize
            || budget.max_probes > u32::MAX as usize
        {
            return Err(GgxEnergyBuildError::InvalidBudget);
        }
        if regions
            .len()
            .checked_mul(9)
            .is_none_or(|count| count > budget.max_probes)
        {
            return Err(ComputeError::WorkBudget.into());
        }
        let plan = GgxEnergyRefinementPlan::try_new(&regions, 1)?;
        if plan.nodes().len() > budget.max_nodes {
            return Err(ComputeError::WorkBudget.into());
        }
        if oracle.len() != plan.probes().len() {
            return Err(GgxEnergyBuildError::OracleCountMismatch {
                expected: plan.probes().len(),
                actual: oracle.len(),
            });
        }
        for (i, (p, o)) in plan.probes().iter().zip(&oracle).enumerate() {
            if p.uv.map(f32::to_bits) != o.uv.map(f32::to_bits) {
                return Err(GgxEnergyRefinementError::ObservationMismatch(i).into());
            }
            super::refinement::validate_reference(o.reference, o.uncertainty, i)?;
        }
        let moments = vec![[0.; 4]; plan.nodes().len()];
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            generation,
            regions,
            oracle,
            policy,
            budget,
            plan,
            phase: GgxEnergyBuildPhase::Nodes,
            offset: 0,
            completed_nodes: 0,
            moments,
            observations: Vec::new(),
            pending: None,
            upload: None,
            waiting: false,
            history: Vec::new(),
        })
    }
    #[must_use]
    pub fn progress(&self) -> GgxEnergyBuildProgress {
        GgxEnergyBuildProgress {
            phase: self.phase,
            refinement: self.plan.refinement(),
            completed_nodes: self.completed_nodes,
            total_nodes: self.plan.nodes().len(),
            completed_probes: self.observations.len(),
            total_probes: self.plan.probes().len(),
            waiting_for_capacity: self.waiting,
        }
    }
    #[must_use]
    pub fn assessments(&self) -> &[(u32, GgxEnergyRefinementAssessment)] {
        &self.history
    }
    /// One bounded GPU submission per call, existing pooled readbacks, and Poll
    /// rather than Wait. CPU transition work is bounded by the explicit plan budget.
    /// Capacity exhaustion is retryable; every other error terminates this build.
    pub fn poll(
        &mut self,
        gpu: &GgxEnergyGpuPrecompute,
        current_generation: u64,
    ) -> Result<Option<ReadyGgxEnergyAtlas>, GgxEnergyBuildError> {
        if matches!(
            self.phase,
            GgxEnergyBuildPhase::Complete | GgxEnergyBuildPhase::Failed
        ) {
            return Err(GgxEnergyBuildError::Consumed);
        }
        let result = if current_generation != self.generation {
            Err(GgxEnergyBuildError::StaleGeneration)
        } else if !gpu.belongs_to(&self.device) {
            Err(ComputeError::DeviceMismatch.into())
        } else {
            self.poll_inner(gpu)
        };
        if result.is_err() {
            self.phase = GgxEnergyBuildPhase::Failed;
            self.pending.take();
            self.upload.take();
        }
        result
    }
    fn poll_inner(
        &mut self,
        gpu: &GgxEnergyGpuPrecompute,
    ) -> Result<Option<ReadyGgxEnergyAtlas>, GgxEnergyBuildError> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|e| ComputeError::Mapping(e.to_string()))?;
        self.waiting = false;
        if self.phase == GgxEnergyBuildPhase::Upload {
            match self
                .upload
                .as_ref()
                .expect("upload phase")
                .completion
                .try_recv()
            {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => {
                    return Err(
                        ComputeError::Mapping("GGX upload completion disconnected".into()).into(),
                    );
                }
                Ok(()) => {
                    let upload = self.upload.take().expect("completed upload");
                    self.phase = GgxEnergyBuildPhase::Complete;
                    return Ok(Some(ReadyGgxEnergyAtlas {
                        generation: self.generation,
                        atlas: upload.atlas,
                        report: upload.report,
                    }));
                }
            }
        }
        if let Some((key, pending)) = &mut self.pending {
            let Some(output) = pending.try_read(&self.device, *key)? else {
                return Ok(None);
            };
            let count = output.coordinates.len();
            if output.key != *key {
                return Err(ComputeError::InvalidBuffer.into());
            }
            match self.phase {
                GgxEnergyBuildPhase::Nodes => {
                    if output.stage != GgxEnergyBatchStage::IntegrateNodes
                        || output.coordinates != self.plan.nodes()[self.offset..self.offset + count]
                    {
                        return Err(ComputeError::InvalidBuffer.into());
                    }
                    self.moments[self.offset..self.offset + count].copy_from_slice(&output.moments);
                }
                GgxEnergyBuildPhase::Probes => {
                    if output.stage != GgxEnergyBatchStage::InterpolateProbes {
                        return Err(ComputeError::InvalidBuffer.into());
                    }
                    for (i, (uv, m)) in output
                        .coordinates
                        .into_iter()
                        .zip(output.moments)
                        .enumerate()
                    {
                        let oracle = self.oracle[self.offset + i];
                        if uv.map(f32::to_bits) != oracle.uv.map(f32::to_bits) {
                            return Err(ComputeError::InvalidBuffer.into());
                        }
                        self.observations.push(GgxEnergyRefinementObservation {
                            uv,
                            gpu: m,
                            reference: oracle.reference,
                            uncertainty: oracle.uncertainty,
                        });
                    }
                }
                _ => unreachable!("only compute phases own pending work"),
            }
            if self.phase == GgxEnergyBuildPhase::Nodes {
                self.completed_nodes = self.offset + count;
            }
            self.offset += count;
            self.pending.take();
        }
        if self.phase == GgxEnergyBuildPhase::Nodes && self.offset == self.plan.nodes().len() {
            self.phase = GgxEnergyBuildPhase::Probes;
            self.offset = 0;
        }
        if self.phase == GgxEnergyBuildPhase::Probes && self.offset == self.plan.probes().len() {
            let report = self.policy.assess(&self.plan, &self.observations)?;
            self.history.push((self.plan.refinement(), report.clone()));
            match report.decision() {
                GgxEnergyRefinementDecision::BudgetExhausted => {
                    return Err(GgxEnergyBuildError::SampledRejected);
                }
                GgxEnergyRefinementDecision::Refine { next } => {
                    let plan = GgxEnergyRefinementPlan::try_new(&self.regions, next)?;
                    if plan.nodes().len() > self.budget.max_nodes
                        || plan.probes().len() > self.budget.max_probes
                    {
                        return Err(ComputeError::WorkBudget.into());
                    }
                    self.moments = vec![[0.; 4]; plan.nodes().len()];
                    self.plan = plan;
                    self.phase = GgxEnergyBuildPhase::Nodes;
                    self.offset = 0;
                    self.observations.clear();
                    self.completed_nodes = 0;
                }
                GgxEnergyRefinementDecision::SampledPass => {
                    let patches = self.plan.experimental_patch_data(&self.moments)?;
                    if super::patch::atlas_allocation_bytes(patches.len())?
                        > self.budget.max_candidate_atlas_bytes
                    {
                        return Err(ComputeError::MemoryBudget.into());
                    }
                    let atlas =
                        GgxDfgLut::with_experimental_energy_patches(&self.device, &patches)?;
                    let mut encoder = self.device.create_command_encoder(&Default::default());
                    atlas.encode(&mut encoder);
                    self.queue.submit([encoder.finish()]);
                    let (sender, completion) = mpsc::channel();
                    self.queue.on_submitted_work_done(move || {
                        let _ = sender.send(());
                    });
                    self.upload = Some(Upload {
                        atlas,
                        completion,
                        report,
                    });
                    self.phase = GgxEnergyBuildPhase::Upload;
                    return Ok(None);
                }
            }
        }
        let key = GgxEnergyBatchKey {
            generation: self.generation,
            refinement: self.plan.refinement(),
            offset: self.offset as u32,
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let encoded = match self.phase {
            GgxEnergyBuildPhase::Nodes => {
                let end = (self.offset + GGX_ENERGY_GPU_BATCH_LIMIT).min(self.plan.nodes().len());
                gpu.encode_nodes(&mut encoder, key, &self.plan.nodes()[self.offset..end])
            }
            GgxEnergyBuildPhase::Probes => {
                let end = (self.offset + GGX_ENERGY_GPU_BATCH_LIMIT).min(self.plan.probes().len());
                gpu.encode_probes(
                    &mut encoder,
                    key,
                    &self.plan.probes()[self.offset..end],
                    &self.moments,
                )
            }
            _ => unreachable!("compute or upload phase"),
        };
        let encoded = match encoded {
            Ok(value) => value,
            Err(ComputeError::MemoryBudget | ComputeError::ReadbackBudget) => {
                self.waiting = true;
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        self.queue.submit([encoder.finish()]);
        self.pending = Some((key, encoded.submitted()));
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    include!("dfg_build_fixture.rs");
    fn regions() -> Vec<[f32; 4]> {
        GgxEnergyRefinementPlan::regions_from_seeds(&[[1., 0.005]]).unwrap()
    }
    fn budget() -> GgxEnergyBuildBudget {
        GgxEnergyBuildBudget {
            max_nodes: 4096,
            max_probes: 512,
            max_candidate_atlas_bytes: 2 * 1024 * 1024,
        }
    }
    fn policy() -> GgxEnergyRefinementPolicy {
        GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap()
    }
    #[test]
    fn budgets_and_oracle_fail_before_gpu_work() {
        let (d, q) = wgpu::Device::noop(&Default::default());
        let mut small = budget();
        small.max_probes = 8;
        assert!(matches!(
            GgxEnergyAtlasBuild::new(&d, &q, 1, regions(), ORACLE.to_vec(), policy(), small),
            Err(GgxEnergyBuildError::Compute(ComputeError::WorkBudget))
        ));
        let mut oracle = ORACLE.to_vec();
        oracle[0].uncertainty[0] = 1.;
        assert!(matches!(
            GgxEnergyAtlasBuild::new(&d, &q, 1, regions(), oracle, policy(), budget()),
            Err(GgxEnergyBuildError::Refinement(
                GgxEnergyRefinementError::InvalidOracle(0)
            ))
        ));
        assert!(matches!(
            GgxEnergyAtlasBuild::new(&d, &q, 1, regions(), Vec::new(), policy(), budget()),
            Err(GgxEnergyBuildError::OracleCountMismatch { .. })
        ));
    }
    // Owner transaction only; physical completion is covered by the GPU test.
    fn ready(d: &wgpu::Device, generation: u64) -> ReadyGgxEnergyAtlas {
        let plan = GgxEnergyRefinementPlan::try_new(&regions(), 1).unwrap();
        let observations = ORACLE
            .iter()
            .map(|o| GgxEnergyRefinementObservation {
                uv: o.uv,
                gpu: [
                    o.reference[0] as f32,
                    o.reference[1] as f32,
                    o.reference[2] as f32,
                    1.,
                ],
                reference: o.reference,
                uncertainty: o.uncertainty,
            })
            .collect::<Vec<_>>();
        let report = policy().assess(&plan, &observations).unwrap();
        assert_eq!(report.decision(), GgxEnergyRefinementDecision::SampledPass);
        let patches = plan
            .experimental_patch_data(&vec![[0.5, 0.25, 0.25, 1.]; plan.nodes().len()])
            .unwrap();
        ReadyGgxEnergyAtlas {
            generation,
            atlas: GgxDfgLut::with_experimental_energy_patches(d, &patches).unwrap(),
            report,
        }
    }
    #[test]
    fn stale_ready_does_not_replace_current_resource() {
        let (d, _) = wgpu::Device::noop(&Default::default());
        let current = GgxDfgLut::new(&d, 2).unwrap();
        let original = current.output().clone();
        let mut slot = GgxEnergyAtlasSlot::new(current, 0);
        slot.request_generation(2).unwrap();
        assert!(matches!(
            slot.try_install(ready(&d, 1)),
            Err(GgxEnergyBuildError::StaleGeneration)
        ));
        assert_eq!(slot.current().output(), &original);
        assert_eq!(slot.installed_generation(), 0);
        assert!(slot.request_generation(2).is_err());
        assert_eq!(slot.requested_generation(), 2);
    }
    #[test]
    fn successful_install_returns_old_owner() {
        let (d, _) = wgpu::Device::noop(&Default::default());
        let current = GgxDfgLut::new(&d, 2).unwrap();
        let original = current.output().clone();
        let mut slot = GgxEnergyAtlasSlot::new(current, 0);
        slot.request_generation(1).unwrap();
        let old = slot.try_install(ready(&d, 1)).unwrap();
        assert_eq!(old.output(), &original);
        assert_ne!(slot.current().output(), &original);
        assert_eq!(slot.installed_generation(), 1);
    }
    #[test]
    #[ignore = "requires physical GPU; runtime atlas build and completed-generation handoff"]
    fn runtime_builder_preserves_old_owner_until_completed_upload() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu = pollster::block_on(GgxEnergyGpuPrecompute::new(&device)).unwrap();
        let current = GgxDfgLut::new(&device, 4).unwrap();
        let original = current.output().clone();
        let mut slot = GgxEnergyAtlasSlot::new(current, 0);
        slot.request_generation(1).unwrap();
        let mut builder = GgxEnergyAtlasBuild::new(
            &device,
            &queue,
            1,
            regions(),
            ORACLE.to_vec(),
            policy(),
            budget(),
        )
        .unwrap();
        let mut saw_upload = false;
        let started = std::time::Instant::now();
        let ready = loop {
            match builder.poll(&gpu, slot.requested_generation()).unwrap() {
                Some(ready) => break ready,
                None => {
                    saw_upload |= builder.progress().phase == GgxEnergyBuildPhase::Upload;
                    assert_eq!(slot.current().output(), &original);
                    assert_eq!(slot.installed_generation(), 0);
                    assert!(started.elapsed().as_secs() < 30);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        };
        assert!(saw_upload);
        assert_eq!(
            ready.assessment().decision(),
            GgxEnergyRefinementDecision::SampledPass
        );
        assert!(matches!(
            builder.poll(&gpu, 1),
            Err(GgxEnergyBuildError::Consumed)
        ));
        let old = slot.try_install(ready).unwrap();
        assert_eq!(old.output(), &original);
        let installed = slot.current().output().clone();
        slot.request_generation(2).unwrap();
        let mut obsolete = GgxEnergyAtlasBuild::new(
            &device,
            &queue,
            2,
            regions(),
            ORACLE.to_vec(),
            policy(),
            budget(),
        )
        .unwrap();
        assert!(obsolete.poll(&gpu, 2).unwrap().is_none());
        slot.request_generation(3).unwrap();
        assert!(matches!(
            obsolete.poll(&gpu, 3),
            Err(GgxEnergyBuildError::StaleGeneration)
        ));
        assert_eq!(obsolete.progress().phase, GgxEnergyBuildPhase::Failed);
        assert_eq!(obsolete.progress().completed_nodes, 0);
        assert_eq!(slot.current().output(), &installed);
        assert_eq!(slot.installed_generation(), 1);
    }
}
