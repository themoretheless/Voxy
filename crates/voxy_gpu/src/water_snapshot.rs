use crate::{
    PendingWaterTransfer, WaterComputeError, WaterNode, WaterNodeStatus, WaterTransferProgram,
    WaterTransferResult,
};
use physics_voxel::{WaterBudget, WaterError, WaterPlan, WaterStates};
use std::collections::{BTreeMap, BTreeSet};
use voxy_core::{ChunkPos, VoxelPos, split_voxel};
use voxy_world::{
    BlockRegistry, BlockStateId, ChunkSnapshot, EditSource, EditTxn, VoxelView, VoxelWrite,
};

#[derive(Debug)]
pub enum WaterSnapshotError {
    Water(WaterError),
    InvalidGraph,
    MissingActive(VoxelPos),
    /// Exact captured cell requiring loading or recovery before a retry.
    MissingSample(VoxelPos),
    InvalidResult,
    Compute(WaterComputeError),
}
impl From<WaterError> for WaterSnapshotError {
    fn from(value: WaterError) -> Self {
        Self::Water(value)
    }
}
impl std::fmt::Display for WaterSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compute(error) => write!(f, "water snapshot calculation failed: {error}"),
            error => write!(f, "GPU water snapshot error: {error:?}"),
        }
    }
}
impl std::error::Error for WaterSnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Water(error) => Some(error),
            Self::Compute(error) => Some(error),
            _ => None,
        }
    }
}

/// An immutable world snapshot retained until nonblocking device readback ends.
/// Native callers drive device polling; browser callers yield to the event loop.
#[derive(Debug)]
pub struct PendingWaterPlan {
    state: PendingPlanState,
    source: EditSource,
    budget: WaterBudget,
}
#[derive(Debug)]
enum PendingPlanState {
    Settled,
    Work(Box<(WaterWorldSnapshot, PendingWaterTransfer)>),
    Consumed,
}
impl PendingWaterPlan {
    /// Returns a revision-checked plan once mapping completes, without waiting.
    /// Commit against the original world; intervening edits must conflict.
    /// # Errors
    /// Returns solver/transaction errors or a consumed-result error. Both success
    /// and failure consume the pending plan; pending polls preserve it.
    pub fn try_plan(&mut self) -> Result<Option<WaterPlan>, WaterSnapshotError> {
        let outcome = match &mut self.state {
            PendingPlanState::Settled => Ok(Some(WaterPlan::Settled)),
            PendingPlanState::Consumed => {
                return Err(WaterSnapshotError::Compute(WaterComputeError::Compute(
                    voxy_render::ComputeError::Consumed,
                )));
            }
            PendingPlanState::Work(work) => {
                let (snapshot, pending) = work.as_mut();
                pending
                    .try_result()
                    .map_err(|error| world_compute_error(snapshot, self.budget, error))
                    .and_then(|result| {
                        result
                            .map(|result| snapshot.plan(&result, self.source, self.budget))
                            .transpose()
                    })
            }
        };
        if !matches!(outcome, Ok(None)) {
            self.state = PendingPlanState::Consumed;
        }
        outcome
    }
}
fn world_compute_error(
    snapshot: &WaterWorldSnapshot,
    budget: WaterBudget,
    error: WaterComputeError,
) -> WaterSnapshotError {
    match error {
        WaterComputeError::UnavailableNode(index) => usize::try_from(index)
            .ok()
            .and_then(|index| snapshot.positions.get(index).copied())
            .map_or(
                WaterSnapshotError::InvalidResult,
                WaterSnapshotError::MissingSample,
            ),
        WaterComputeError::UnknownNode(index) => snapshot
            .unknown_state(index)
            .map_or(WaterSnapshotError::InvalidResult, |block| {
                WaterError::UnknownState(block).into()
            }),
        WaterComputeError::CoordinateOverflow => WaterError::CoordinateOverflow.into(),
        WaterComputeError::SampleBudget => WaterError::SampleBudgetExceeded {
            limit: budget.max_samples,
        }
        .into(),
        error => WaterSnapshotError::Compute(error),
    }
}

impl WaterTransferProgram {
    /// Submits a world tick without blocking and retains its captured revisions.
    /// Empty batches produce a ready settled plan without device work.
    /// # Errors
    /// Rejects invalid states/budgets and graph inputs before submission; lazy
    /// reads and transaction failures are returned later by `try_plan`.
    #[allow(clippy::too_many_arguments)]
    pub fn begin_plan_world(
        &self,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        source: EditSource,
        budget: WaterBudget,
    ) -> Result<PendingWaterPlan, WaterSnapshotError> {
        let states = states.validate(registry)?;
        if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
            return Err(WaterError::InvalidBudget.into());
        }
        let state = if active.is_empty() {
            PendingPlanState::Settled
        } else {
            let snapshot = WaterWorldSnapshot::capture_active_available(
                view, registry, states, active, budget,
            )?;
            let indices = snapshot.active_indices(active)?;
            let pending = self
                .begin_step_detailed_status(
                    queue,
                    snapshot.nodes(),
                    &indices,
                    snapshot.node_status(),
                    8,
                    4,
                    u32::try_from(budget.max_samples).unwrap_or(u32::MAX),
                    u32::try_from(budget.max_writes).unwrap_or(u32::MAX),
                )
                .map_err(|error| world_compute_error(&snapshot, budget, error))?;
            PendingPlanState::Work(Box::new((snapshot, pending)))
        };
        Ok(PendingWaterPlan {
            state,
            source,
            budget,
        })
    }

    /// Plans one canonical world tick on this program's native GPU device.
    /// Captures immutable data/revisions, performs sparse ordered transfers and
    /// returns a transaction for the caller to commit. No CPU water solver runs.
    /// Empty active batches settle without uploading or dispatching work.
    /// # Errors
    /// Rejects invalid states/budgets, graph limits, lazy device reads, numerical
    /// coordinate overflow, malformed output and transaction budget violations.
    /// The queue must belong to this program's device. No world mutation occurs.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_world(
        &self,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        source: EditSource,
        budget: WaterBudget,
    ) -> Result<WaterPlan, WaterSnapshotError> {
        let states = states.validate(registry)?;
        if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
            return Err(WaterError::InvalidBudget.into());
        }
        if active.is_empty() {
            return Ok(WaterPlan::Settled);
        }
        let snapshot =
            WaterWorldSnapshot::capture_active_available(view, registry, states, active, budget)?;
        let indices = snapshot.active_indices(active)?;
        let result = self
            .step_detailed_status(
                queue,
                snapshot.nodes(),
                &indices,
                snapshot.node_status(),
                8,
                4,
                u32::try_from(budget.max_samples).unwrap_or(u32::MAX),
                u32::try_from(budget.max_writes).unwrap_or(u32::MAX),
            )
            .map_err(|error| world_compute_error(&snapshot, budget, error))?;
        snapshot.plan(&result, source, budget)
    }
}

impl crate::CudaWaterTransferProgram {
    /// Captures the same lazy world graph as WGSL, executes CUDA and returns a
    /// revision-checked plan. The caller commits it; no world writes occur here.
    /// This method synchronizes CUDA readback and has no CPU solver fallback.
    /// # Errors
    /// Rejects invalid budgets/states, missing/unknown samples, CUDA failures and
    /// invalid output before returning a transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_world(
        &self,
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        source: EditSource,
        budget: WaterBudget,
    ) -> Result<WaterPlan, WaterSnapshotError> {
        let states = states.validate(registry)?;
        if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
            return Err(WaterError::InvalidBudget.into());
        }
        if active.is_empty() {
            return Ok(WaterPlan::Settled);
        }
        let snapshot =
            WaterWorldSnapshot::capture_active_available(view, registry, states, active, budget)?;
        let indices = snapshot.active_indices(active)?;
        let result = self
            .step_detailed_status(
                snapshot.nodes(),
                &indices,
                snapshot.node_status(),
                8,
                4,
                u32::try_from(budget.max_samples).unwrap_or(u32::MAX),
                u32::try_from(budget.max_writes).unwrap_or(u32::MAX),
            )
            .map_err(|error| world_compute_error(&snapshot, budget, error))?;
        snapshot.plan(&result, source, budget)
    }
}

/// Immutable graph and the chunk revisions that supplied its loaded blocks.
/// Strict capture requires loaded known states. Availability-aware capture
/// retains unloaded and unknown nodes for lazy device-side errors.
#[derive(Debug)]
pub struct WaterWorldSnapshot {
    positions: Vec<VoxelPos>,
    indices: BTreeMap<VoxelPos, u32>,
    nodes: Vec<WaterNode>,
    available: Vec<bool>,
    status: Vec<WaterNodeStatus>,
    unknown: BTreeMap<u32, BlockStateId>,
    chunks: BTreeMap<ChunkPos, ChunkSnapshot>,
    states: WaterStates,
}
impl WaterWorldSnapshot {
    /// Captures immutable chunk data before device work. No live `sample` calls
    /// are mixed with later revision queries.
    /// # Errors
    /// Rejects invalid water states, graph ordering/size, missing chunks and
    /// unknown block states. This stage requires a fully loaded graph.
    pub fn capture(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        positions: &[VoxelPos],
    ) -> Result<Self, WaterSnapshotError> {
        Self::capture_graph(view, registry, states, positions, false)
    }

    /// Captures a graph while retaining unavailable chunks as lazy device reads.
    /// # Errors
    /// Rejects malformed graphs and invalid water states.
    /// Missing chunks and unknown blocks are retained in `node_status` for lazy reads.
    pub fn capture_available(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        positions: &[VoxelPos],
    ) -> Result<Self, WaterSnapshotError> {
        Self::capture_graph(view, registry, states, positions, true)
    }

    fn capture_graph(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        positions: &[VoxelPos],
        allow_missing: bool,
    ) -> Result<Self, WaterSnapshotError> {
        let states = states.validate(registry)?;
        if positions.is_empty()
            || positions.len() > 131_072
            || positions.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(WaterSnapshotError::InvalidGraph);
        }
        let indices: BTreeMap<_, _> = positions
            .iter()
            .copied()
            .enumerate()
            .map(|(i, pos)| {
                u32::try_from(i)
                    .map(|i| (pos, i))
                    .map_err(|_| WaterSnapshotError::InvalidGraph)
            })
            .collect::<Result<_, _>>()?;
        let mut chunks = BTreeMap::new();
        let mut missing = BTreeSet::new();
        for &pos in positions {
            let chunk = split_voxel(pos).0;
            if missing.contains(&chunk) {
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = chunks.entry(chunk) {
                match view.chunk(chunk) {
                    Some(snapshot) if snapshot.pos == chunk => {
                        entry.insert(snapshot);
                    }
                    None if allow_missing => {
                        missing.insert(chunk);
                    }
                    _ => return Err(WaterError::InconsistentView(chunk).into()),
                }
            }
        }
        let mut nodes = Vec::with_capacity(positions.len());
        let mut available = Vec::with_capacity(positions.len());
        let mut classifications = Vec::with_capacity(positions.len());
        let mut unknown = BTreeMap::new();
        for &pos in positions {
            let (chunk, local) = split_voxel(pos);
            let snapshot = chunks.get(&chunk);
            let mut node_status = if snapshot.is_some() {
                WaterNodeStatus::Loaded
            } else {
                WaterNodeStatus::Unavailable
            };
            let amount = if let Some(snapshot) = snapshot {
                let block = snapshot.data.blocks.get(local.index());
                if block == BlockStateId::AIR {
                    Some(0)
                } else if let Some(i) = states.0.iter().position(|&water| water == block) {
                    Some(u8::try_from(i + 1).map_err(|_| WaterSnapshotError::InvalidGraph)?)
                } else if registry.get(block).is_some() {
                    None
                } else if allow_missing {
                    node_status = WaterNodeStatus::Unknown;
                    unknown.insert(
                        *indices.get(&pos).ok_or(WaterSnapshotError::InvalidGraph)?,
                        block,
                    );
                    None
                } else {
                    return Err(WaterError::UnknownState(block).into());
                }
            } else {
                None
            };
            let neighbors =
                [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)].map(|delta| {
                    match neighbor(pos, delta) {
                        Ok(p) => indices.get(&p).copied(),
                        Err(_) => Some(u32::MAX - 1),
                    }
                });
            available.push(node_status == WaterNodeStatus::Loaded);
            classifications.push(node_status);
            nodes.push(WaterNode { amount, neighbors });
        }
        Ok(Self {
            positions: positions.to_vec(),
            indices,
            nodes,
            available,
            status: classifications,
            unknown,
            chunks,
            states,
        })
    }
    /// Builds the sparse loaded graph for one canonical active batch.
    /// Every potential source and its five transfer destinations are retained;
    /// no unrelated world cells are uploaded. Empty batches need no GPU work.
    /// # Errors
    /// Rejects empty batches, invalid budgets, active limits, coordinate overflow,
    /// unavailable chunks and unknown states, as well as capture errors.
    pub fn capture_active(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        budget: WaterBudget,
    ) -> Result<Self, WaterSnapshotError> {
        Self::capture_active_graph(view, registry, states, active, budget, false)
    }

    /// Captures a sparse active graph with lazy unavailable chunk nodes.
    /// # Errors
    /// Returns the validation errors of `capture_active` except that missing
    /// chunks and unknown blocks are retained in `node_status` for lazy reads.
    /// Overflowing edges are retained as lazy coordinate-error sentinels.
    pub fn capture_active_available(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        budget: WaterBudget,
    ) -> Result<Self, WaterSnapshotError> {
        Self::capture_active_graph(view, registry, states, active, budget, true)
    }

    fn capture_active_graph(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        states: WaterStates,
        active: &[VoxelPos],
        budget: WaterBudget,
        allow_missing: bool,
    ) -> Result<Self, WaterSnapshotError> {
        if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
            return Err(WaterError::InvalidBudget.into());
        }
        let active: BTreeSet<_> = active.iter().copied().collect();
        if active.len() > budget.max_active {
            return Err(WaterError::ActiveBudgetExceeded {
                required: active.len(),
                limit: budget.max_active,
            }
            .into());
        }
        let mut positions = active.clone();
        for pos in active {
            for delta in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
                match neighbor(pos, delta) {
                    Ok(p) => {
                        positions.insert(p);
                    }
                    Err(_) if allow_missing => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Self::capture_graph(
            view,
            registry,
            states,
            &positions.into_iter().collect::<Vec<_>>(),
            allow_missing,
        )
    }

    #[must_use]
    pub fn nodes(&self) -> &[WaterNode] {
        &self.nodes
    }

    /// Compatibility mask treating both unknown and unavailable nodes as unreadable.
    /// Use `node_status` to preserve typed unknown-state errors.
    #[must_use]
    pub fn availability(&self) -> &[bool] {
        &self.available
    }

    /// Full node classification for `step_detailed_status`.
    #[must_use]
    pub fn node_status(&self) -> &[WaterNodeStatus] {
        &self.status
    }

    /// Original unknown block ID for translating a device `UnknownNode` error.
    #[must_use]
    pub fn unknown_state(&self, index: u32) -> Option<BlockStateId> {
        self.unknown.get(&index).copied()
    }

    /// Deduplicates and sorts active positions in canonical voxel order.
    /// # Errors
    /// Rejects an active position absent from the captured graph.
    pub fn active_indices(&self, active: &[VoxelPos]) -> Result<Vec<u32>, WaterSnapshotError> {
        active
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|p| {
                self.indices
                    .get(&p)
                    .copied()
                    .ok_or(WaterSnapshotError::MissingActive(p))
            })
            .collect()
    }

    /// Builds a world transaction from the result of this snapshot's GPU graph.
    /// Expected revisions come exclusively from captured chunks actually read by
    /// the device. Commit on the original world; a stale result must conflict.
    /// # Errors
    /// Rejects invalid result shape, unread changes, classification/volume changes,
    /// budgets and coordinate overflow. No world mutation occurs here.
    pub fn plan(
        &self,
        result: &WaterTransferResult,
        source: EditSource,
        budget: WaterBudget,
    ) -> Result<WaterPlan, WaterSnapshotError> {
        if result.amounts.len() != self.nodes.len() || result.sampled.len() != self.nodes.len() {
            return Err(WaterSnapshotError::InvalidResult);
        }
        if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
            return Err(WaterError::InvalidBudget.into());
        }
        if result.sampled.iter().filter(|&&read| read).count() > budget.max_samples {
            return Err(WaterError::SampleBudgetExceeded {
                limit: budget.max_samples,
            }
            .into());
        }
        if self
            .available
            .iter()
            .zip(&result.sampled)
            .any(|(&available, &read)| !available && read)
        {
            return Err(WaterSnapshotError::InvalidResult);
        }
        let mut before_volume = 0_u64;
        let mut after_volume = 0_u64;
        let mut writes = Vec::new();
        let mut sampled_chunks = BTreeSet::new();
        for (((&pos, node), &after), &read) in self
            .positions
            .iter()
            .zip(&self.nodes)
            .zip(&result.amounts)
            .zip(&result.sampled)
        {
            if after.is_some_and(|a| a > 8)
                || node.amount.is_none() != after.is_none()
                || (node.amount != after && !read)
            {
                return Err(WaterSnapshotError::InvalidResult);
            }
            before_volume += u64::from(node.amount.unwrap_or(0));
            after_volume += u64::from(after.unwrap_or(0));
            if read {
                sampled_chunks.insert(split_voxel(pos).0);
            }
            if node.amount != after {
                let level = after.ok_or(WaterSnapshotError::InvalidResult)?;
                let block = if level == 0 {
                    BlockStateId::AIR
                } else {
                    self.states.0[usize::from(level - 1)]
                };
                writes.push(VoxelWrite { pos, block });
            }
        }
        if before_volume != after_volume {
            return Err(WaterSnapshotError::InvalidResult);
        }
        if writes.is_empty() {
            return Ok(WaterPlan::Settled);
        }
        if writes.len() > budget.max_writes {
            return Err(WaterError::WriteBudgetExceeded {
                required: writes.len(),
                limit: budget.max_writes,
            }
            .into());
        }
        let expected = sampled_chunks
            .into_iter()
            .map(|pos| {
                self.chunks
                    .get(&pos)
                    .map(|chunk| (pos, chunk.revision))
                    .ok_or(WaterSnapshotError::InvalidResult)
            })
            .collect::<Result<_, _>>()?;
        let mut next_active = BTreeSet::new();
        for write in &writes {
            next_active.insert(write.pos);
            for delta in [
                (0, 1, 0),
                (0, -1, 0),
                (-1, 0, 0),
                (1, 0, 0),
                (0, 0, -1),
                (0, 0, 1),
            ] {
                next_active.insert(neighbor(write.pos, delta)?);
            }
        }
        Ok(WaterPlan::Transaction {
            edit: EditTxn {
                source,
                expected,
                writes,
            },
            next_active: next_active
                .into_iter()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }
}
fn neighbor(pos: VoxelPos, (dx, dy, dz): (i64, i64, i64)) -> Result<VoxelPos, WaterError> {
    Ok(VoxelPos {
        x: pos
            .x
            .checked_add(dx)
            .ok_or(WaterError::CoordinateOverflow)?,
        y: pos
            .y
            .checked_add(dy)
            .ok_or(WaterError::CoordinateOverflow)?,
        z: pos
            .z
            .checked_add(dz)
            .ok_or(WaterError::CoordinateOverflow)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_world::{CommitError, ResourceKey, Sample};

    #[test]
    fn captured_revision_rejects_stale_gpu_transaction_atomically() {
        let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let mut levels = [BlockStateId::AIR; 8];
        for (i, level) in levels.iter_mut().enumerate() {
            *level = scene
                .world
                .registry()
                .find(&ResourceKey::parse(format!("voxy:water_{}", i + 1)).unwrap())
                .unwrap();
        }
        let positions = [
            VoxelPos {
                x: 16,
                y: 17,
                z: 16,
            },
            VoxelPos {
                x: 16,
                y: 18,
                z: 16,
            },
        ];
        let capture = |world: &voxy_world::World| {
            WaterWorldSnapshot::capture(world, world.registry(), WaterStates(levels), &positions)
                .unwrap()
        };
        let sparse = WaterWorldSnapshot::capture_active(
            &scene.world,
            scene.world.registry(),
            WaterStates(levels),
            &[positions[1], positions[1]],
            WaterBudget::default(),
        )
        .unwrap();
        assert_eq!(sparse.nodes().len(), 6);
        assert_eq!(
            sparse
                .active_indices(&[positions[1], positions[1]])
                .unwrap()
                .len(),
            1
        );
        assert!(matches!(
            WaterWorldSnapshot::capture_active(
                &scene.world,
                scene.world.registry(),
                WaterStates(levels),
                &positions,
                WaterBudget {
                    max_active: 1,
                    ..WaterBudget::default()
                },
            ),
            Err(WaterSnapshotError::Water(
                WaterError::ActiveBudgetExceeded {
                    required: 2,
                    limit: 1
                }
            ))
        ));
        let edge = VoxelPos {
            x: i64::MAX,
            y: i64::MIN,
            z: i64::MAX,
        };
        let edge_graph = WaterWorldSnapshot::capture_active_available(
            &scene.world,
            scene.world.registry(),
            WaterStates(levels),
            &[edge],
            WaterBudget::default(),
        )
        .unwrap();
        assert_eq!(edge_graph.nodes().len(), 3);
        let edge_index = edge_graph.active_indices(&[edge]).unwrap()[0] as usize;
        assert_eq!(
            edge_graph.nodes()[edge_index].neighbors[0],
            Some(u32::MAX - 1)
        );
        assert_eq!(
            edge_graph.nodes()[edge_index].neighbors[2],
            Some(u32::MAX - 1)
        );
        assert_eq!(
            edge_graph.nodes()[edge_index].neighbors[4],
            Some(u32::MAX - 1)
        );
        let snapshot = capture(&scene.world);
        assert_eq!(snapshot.nodes()[0].amount, Some(0));
        assert_eq!(snapshot.nodes()[1].amount, Some(8));
        let result = WaterTransferResult {
            amounts: vec![Some(8), Some(0)],
            sampled: vec![true, true],
        };
        let transaction = |snapshot: &WaterWorldSnapshot| {
            let WaterPlan::Transaction { edit, .. } = snapshot
                .plan(&result, EditSource::Simulation, WaterBudget::default())
                .unwrap()
            else {
                panic!("expected transfer")
            };
            edit
        };
        let stale = transaction(&snapshot);
        scene
            .world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: vec![],
                writes: vec![VoxelWrite {
                    pos: VoxelPos { x: 0, y: 20, z: 0 },
                    block: levels[0],
                }],
            })
            .unwrap();
        assert!(matches!(
            scene.world.commit(stale),
            Err(CommitError::RevisionConflict { .. })
        ));
        assert_eq!(
            scene.world.sample(positions[0]),
            Sample::Loaded(BlockStateId::AIR)
        );
        assert_eq!(scene.world.sample(positions[1]), Sample::Loaded(levels[7]));
        let fresh = capture(&scene.world);
        let unread = WaterTransferResult {
            sampled: vec![false, true],
            ..result.clone()
        };
        assert!(matches!(
            fresh.plan(&unread, EditSource::Simulation, WaterBudget::default()),
            Err(WaterSnapshotError::InvalidResult)
        ));
        scene.world.commit(transaction(&fresh)).unwrap();
        assert_eq!(scene.world.sample(positions[0]), Sample::Loaded(levels[7]));
        assert_eq!(
            scene.world.sample(positions[1]),
            Sample::Loaded(BlockStateId::AIR)
        );
    }
}
