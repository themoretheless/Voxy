use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use voxy_core::{ChunkPos, LocalPos, TickId, VoxelPos, WorldEpoch, split_voxel};

use crate::{
    BlockRegistry, BlockStateId, ChunkData, ChunkRevision, ChunkSnapshot, GeneratedChunk,
    PalettedBlocks,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnavailableReason {
    Corrupt,
    UnsupportedVersion,
    PermanentStorageFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sample<T> {
    Loaded(T),
    Unloaded {
        chunk: ChunkPos,
    },
    Unavailable {
        chunk: ChunkPos,
        cause: UnavailableReason,
    },
}

pub trait VoxelView {
    fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId>;
    fn chunk(&self, pos: ChunkPos) -> Option<ChunkSnapshot>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditSource {
    Player(u64),
    Simulation,
    Editor,
    Replay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoxelWrite {
    pub pos: VoxelPos,
    pub block: BlockStateId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditTxn {
    pub source: EditSource,
    pub expected: Vec<(ChunkPos, ChunkRevision)>,
    pub writes: Vec<VoxelWrite>,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommitId(u64);

impl CommitId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurabilityTicket(pub CommitId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirtyBounds {
    pub min: LocalPos,
    pub max_inclusive: LocalPos,
}

#[derive(Clone, Debug)]
pub struct ChunkDelta {
    pub pos: ChunkPos,
    pub before_revision: ChunkRevision,
    pub after_revision: ChunkRevision,
    pub before: Arc<ChunkData>,
    pub after: Arc<ChunkData>,
    pub dirty: DirtyBounds,
    pub invalidated_neighbor_meshes: Box<[ChunkPos]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InverseEdit {
    pub writes: Box<[VoxelWrite]>,
}

#[derive(Clone, Debug)]
pub struct CommitReceipt {
    pub tick: TickId,
    pub commit: CommitId,
    pub source: EditSource,
    pub chunks: Box<[ChunkDelta]>,
    pub inverse: InverseEdit,
    pub durability: DurabilityTicket,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldLimits {
    pub max_writes_per_transaction: usize,
    pub max_chunks_per_transaction: usize,
}

impl Default for WorldLimits {
    fn default() -> Self {
        Self {
            max_writes_per_transaction: 65_536,
            max_chunks_per_transaction: 256,
        }
    }
}

#[derive(Clone, Debug)]
struct ChunkSlot {
    regenerable: bool,
    revision: ChunkRevision,
    data: Arc<ChunkData>,
}

#[derive(Clone, Debug)]
pub struct World {
    epoch: WorldEpoch,
    registry: Arc<BlockRegistry>,
    chunks: BTreeMap<ChunkPos, ChunkSlot>,
    unavailable: BTreeMap<ChunkPos, UnavailableReason>,
    unloaded_revisions: BTreeMap<ChunkPos, ChunkRevision>,
    generated_revision: ChunkRevision,
    tick: TickId,
    next_commit: u64,
    limits: WorldLimits,
}

impl World {
    #[must_use]
    pub fn new(epoch: WorldEpoch, registry: Arc<BlockRegistry>, limits: WorldLimits) -> Self {
        Self {
            epoch,
            registry,
            chunks: BTreeMap::new(),
            unavailable: BTreeMap::new(),
            unloaded_revisions: BTreeMap::new(),
            generated_revision: ChunkRevision::default(),
            tick: TickId::new(0),
            next_commit: 0,
            limits,
        }
    }

    #[must_use]
    pub const fn epoch(&self) -> WorldEpoch {
        self.epoch
    }

    pub fn resident_chunk_positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    #[must_use]
    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    #[must_use]
    pub fn registry_handle(&self) -> Arc<BlockRegistry> {
        Arc::clone(&self.registry)
    }

    /// Advances the authoritative tick watermark.
    ///
    /// # Errors
    ///
    /// Rejects a tick older than the current watermark.
    pub fn set_tick(&mut self, tick: TickId) -> Result<(), CommitError> {
        if tick < self.tick {
            return Err(CommitError::TickWentBackwards {
                current: self.tick,
                requested: tick,
            });
        }
        self.tick = tick;
        Ok(())
    }

    /// Adds generated canonical data at the world's current generation revision.
    ///
    /// # Errors
    ///
    /// Rejects replacement of a loaded chunk and unknown block IDs.
    pub fn insert_generated(&mut self, generated: GeneratedChunk) -> Result<(), CommitError> {
        self.validate_generated(&generated)?;
        self.install_generated(generated, self.generated_revision);
        Ok(())
    }

    fn validate_generated(&self, generated: &GeneratedChunk) -> Result<(), CommitError> {
        if self.chunks.contains_key(&generated.pos) {
            return Err(CommitError::ChunkAlreadyLoaded(generated.pos));
        }
        if self.unloaded_revisions.contains_key(&generated.pos) {
            return Err(CommitError::ChunkRequiresRestore(generated.pos));
        }
        self.validate_chunk(&generated.data)
    }

    fn install_generated(&mut self, generated: GeneratedChunk, revision: ChunkRevision) {
        self.unavailable.remove(&generated.pos);
        self.unloaded_revisions.remove(&generated.pos);
        self.chunks.insert(
            generated.pos,
            ChunkSlot {
                regenerable: true,
                revision,
                data: Arc::new(generated.data),
            },
        );
    }

    /// Removes residency while returning the exact edited data for caller retention.
    /// The caller must retain or persist the snapshot before discarding it.
    /// # Errors
    /// Rejects missing chunks and revisions that cannot advance on restoration.
    pub fn unload_chunk(&mut self, pos: ChunkPos) -> Result<ChunkSnapshot, CommitError> {
        self.validate_unload(pos)?;
        Ok(self.remove_resident_chunk(pos))
    }

    fn validate_unload(&self, pos: ChunkPos) -> Result<(), CommitError> {
        let slot = self
            .chunks
            .get(&pos)
            .ok_or(CommitError::ChunkNotLoaded(pos))?;
        slot.revision
            .checked_next()
            .ok_or(CommitError::RevisionOverflow(pos))?;
        Ok(())
    }

    fn remove_resident_chunk(&mut self, pos: ChunkPos) -> ChunkSnapshot {
        let slot = self.chunks.remove(&pos).expect("validated resident chunk");
        self.unloaded_revisions.insert(pos, slot.revision);
        ChunkSnapshot {
            pos,
            revision: slot.revision,
            data: slot.data,
        }
    }

    /// Revision that an evicted snapshot must match to be restored.
    #[must_use]
    pub fn unloaded_chunk_revision(&self, pos: ChunkPos) -> Option<ChunkRevision> {
        self.unloaded_revisions.get(&pos).copied()
    }

    /// Restores retained data with a fresh revision, invalidating pre-unload plans.
    /// # Errors
    /// Rejects loaded/unavailable chunks, stale snapshots, unknown blocks and overflow.
    pub fn restore_chunk(&mut self, snapshot: &ChunkSnapshot) -> Result<(), CommitError> {
        let revision = self.validate_restore(snapshot)?;
        self.install_restored(snapshot, revision);
        Ok(())
    }

    fn validate_restore(&self, snapshot: &ChunkSnapshot) -> Result<ChunkRevision, CommitError> {
        if self.chunks.contains_key(&snapshot.pos) {
            return Err(CommitError::ChunkAlreadyLoaded(snapshot.pos));
        }
        if self.unavailable.contains_key(&snapshot.pos) {
            return Err(CommitError::InvalidChunk(snapshot.pos));
        }
        let actual = self
            .unloaded_revisions
            .get(&snapshot.pos)
            .copied()
            .ok_or(CommitError::ChunkNotLoaded(snapshot.pos))?;
        if snapshot.revision != actual {
            return Err(CommitError::RevisionConflict {
                pos: snapshot.pos,
                expected: snapshot.revision,
                actual,
            });
        }
        let revision = actual
            .checked_next()
            .ok_or(CommitError::RevisionOverflow(snapshot.pos))?;
        self.validate_chunk(&snapshot.data)?;
        Ok(revision)
    }

    fn install_restored(&mut self, snapshot: &ChunkSnapshot, revision: ChunkRevision) {
        self.chunks.insert(
            snapshot.pos,
            ChunkSlot {
                regenerable: false,
                revision,
                data: Arc::clone(&snapshot.data),
            },
        );
        self.unloaded_revisions.remove(&snapshot.pos);
    }

    /// Changes residency as a single batch and returns the evicted data in input order.
    /// Retain or persist the returned snapshots for later restoration.
    ///
    /// # Errors
    /// Rejects repeated positions and any invalid individual change before mutating the world.
    pub fn replace_resident_chunks(
        &mut self,
        evict: &[ChunkPos],
        restore: &[ChunkSnapshot],
        generate: Vec<GeneratedChunk>,
    ) -> Result<Vec<ChunkSnapshot>, CommitError> {
        self.replace_residency(evict, restore, generate, false)
    }

    /// Change residency while discarding untouched, reproducible generated data.
    /// The caller must use the same generator and seed for every regenerated chunk.
    /// Generated revisions advance globally, without retaining per-position markers.
    ///
    /// # Errors
    /// Rejects invalid inputs before mutating the world.
    pub fn replace_resident_generated_chunks(
        &mut self,
        evict: &[ChunkPos],
        restore: &[ChunkSnapshot],
        generate: Vec<GeneratedChunk>,
    ) -> Result<Vec<ChunkSnapshot>, CommitError> {
        self.replace_residency(evict, restore, generate, true)
    }

    fn replace_residency(
        &mut self,
        evict: &[ChunkPos],
        restore: &[ChunkSnapshot],
        generate: Vec<GeneratedChunk>,
        discard_generated: bool,
    ) -> Result<Vec<ChunkSnapshot>, CommitError> {
        let mut positions = BTreeSet::new();
        for pos in evict
            .iter()
            .copied()
            .chain(restore.iter().map(|chunk| chunk.pos))
            .chain(generate.iter().map(|chunk| chunk.pos))
        {
            if !positions.insert(pos) {
                return Err(CommitError::DuplicateResidencyChange(pos));
            }
        }
        for &pos in evict {
            self.validate_unload(pos)?;
        }
        let revisions = restore
            .iter()
            .map(|chunk| self.validate_restore(chunk))
            .collect::<Result<Vec<_>, _>>()?;
        for chunk in &generate {
            self.validate_generated(chunk)?;
        }
        // Generated chunks never modified since their generation share a
        // world-wide watermark. Advance it before forgetting any of them so
        // even a later direct insertion cannot reuse an evicted revision.
        let generated_revision = if discard_generated
            && let Some(pos) = evict
                .first()
                .copied()
                .or_else(|| generate.first().map(|chunk| chunk.pos))
        {
            self.generated_revision
                .checked_next()
                .ok_or(CommitError::RevisionOverflow(pos))?
        } else {
            self.generated_revision
        };
        let mut evicted = Vec::new();
        for &pos in evict {
            if discard_generated && self.chunks[&pos].regenerable {
                self.chunks.remove(&pos);
            } else {
                evicted.push(self.remove_resident_chunk(pos));
            }
        }
        self.generated_revision = generated_revision;
        for (chunk, revision) in restore.iter().zip(revisions) {
            self.install_restored(chunk, revision);
        }
        for chunk in generate {
            self.install_generated(chunk, generated_revision);
        }
        Ok(evicted)
    }

    /// Loads or overwrites a canonical chunk from durable storage recovery (`WAL` / `RegionStore`)
    /// at its exact persisted revision and advances the monotonic commit sequence watermark.
    ///
    /// # Errors
    ///
    /// Rejects unknown blocks, revision rollbacks, and commit sequence overflow.
    pub fn apply_persisted_chunk(
        &mut self,
        snapshot: ChunkSnapshot,
        commit_sequence: Option<u64>,
    ) -> Result<(), CommitError> {
        if self.unavailable.contains_key(&snapshot.pos) {
            return Err(CommitError::InvalidChunk(snapshot.pos));
        }
        if let Some(slot) = self.chunks.get(&snapshot.pos)
            && snapshot.revision < slot.revision
        {
            return Err(CommitError::RevisionConflict {
                pos: snapshot.pos,
                expected: snapshot.revision,
                actual: slot.revision,
            });
        }
        self.validate_chunk(&snapshot.data)?;
        if let Some(seq) = commit_sequence {
            let next = seq.checked_add(1).ok_or(CommitError::CommitIdOverflow)?;
            self.next_commit = self.next_commit.max(next);
        }
        self.unloaded_revisions.remove(&snapshot.pos);
        self.chunks.insert(
            snapshot.pos,
            ChunkSlot {
                regenerable: false,
                revision: snapshot.revision,
                data: snapshot.data,
            },
        );
        Ok(())
    }

    pub fn mark_unavailable(&mut self, pos: ChunkPos, cause: UnavailableReason) {
        self.chunks.remove(&pos);
        self.unavailable.insert(pos, cause);
    }

    /// Atomically validates and applies all writes in a transaction.
    ///
    /// # Errors
    ///
    /// The entire transaction is rejected on limits, duplicate targets, missing chunks,
    /// revision conflicts, unknown blocks, or counter overflow.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn commit(&mut self, mut txn: EditTxn) -> Result<CommitReceipt, CommitError> {
        if txn.writes.is_empty() {
            return Err(CommitError::EmptyTransaction);
        }
        if txn.writes.len() > self.limits.max_writes_per_transaction {
            return Err(CommitError::TooManyWrites(txn.writes.len()));
        }

        txn.expected.sort_by_key(|(pos, _)| *pos);
        for pair in txn.expected.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(CommitError::DuplicateExpectation(pair[0].0));
            }
        }
        for &(pos, expected) in &txn.expected {
            let actual = self
                .chunks
                .get(&pos)
                .ok_or(CommitError::ChunkNotLoaded(pos))?
                .revision;
            if actual != expected {
                return Err(CommitError::RevisionConflict {
                    pos,
                    expected,
                    actual,
                });
            }
        }

        let mut writes = Vec::with_capacity(txn.writes.len());
        for write in txn.writes {
            if self.registry.get(write.block).is_none() {
                return Err(CommitError::UnknownBlock(write.block));
            }
            let (chunk, local) = split_voxel(write.pos);
            writes.push((chunk, local, write));
        }
        writes.sort_unstable_by_key(|(chunk, local, _)| (*chunk, local.index()));
        for pair in writes.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 {
                return Err(CommitError::DuplicateWrite(pair[0].2.pos));
            }
        }

        // `writes` is sorted by `ChunkPos`, so contiguous runs correspond 1-to-1 with touched chunks.
        let mut touched_count = 0_usize;
        for run in writes.chunk_by(|a, b| a.0 == b.0) {
            touched_count += 1;
            if touched_count > self.limits.max_chunks_per_transaction {
                // Report exact total count of distinct chunks in the transaction.
                let total_chunks = writes.chunk_by(|a, b| a.0 == b.0).count();
                return Err(CommitError::TooManyChunks(total_chunks));
            }
            let pos = run[0].0;
            if !self.chunks.contains_key(&pos) {
                return Err(CommitError::ChunkNotLoaded(pos));
            }
        }
        let commit = CommitId(self.next_commit);
        let next_commit = self
            .next_commit
            .checked_add(1)
            .ok_or(CommitError::CommitIdOverflow)?;

        let mut staged = Vec::with_capacity(touched_count);
        let mut inverse = Vec::with_capacity(writes.len());
        let mut updates = Vec::new();
        for run_writes in writes.chunk_by(|a, b| a.0 == b.0) {
            let pos = run_writes[0].0;
            let slot = self
                .chunks
                .get(&pos)
                .ok_or(CommitError::ChunkNotLoaded(pos))?;
            let after_revision = slot
                .revision
                .checked_next()
                .ok_or(CommitError::RevisionOverflow(pos))?;
            updates.clear();
            updates.reserve(run_writes.len());
            for &(_, local, write) in run_writes {
                inverse.push(VoxelWrite {
                    pos: write.pos,
                    block: slot.data.blocks.get(local.index()),
                });
                updates.push((local.index(), write.block));
            }
            let blocks = slot
                .data
                .blocks
                .with_updates(&updates)
                .map_err(|_| CommitError::InvalidChunk(pos))?;
            let after = Arc::new(ChunkData {
                blocks,
                block_data: slot.data.block_data.clone(),
            });
            staged.push(ChunkDelta {
                pos,
                before_revision: slot.revision,
                after_revision,
                before: Arc::clone(&slot.data),
                after,
                dirty: dirty_bounds(run_writes),
                invalidated_neighbor_meshes: invalidated_neighbors(pos, run_writes),
            });
        }

        for delta in &staged {
            if let Some(slot) = self.chunks.get_mut(&delta.pos) {
                slot.revision = delta.after_revision;
                slot.regenerable = false;
                slot.data = Arc::clone(&delta.after);
            }
        }
        self.next_commit = next_commit;
        Ok(CommitReceipt {
            tick: self.tick,
            commit,
            source: txn.source,
            chunks: staged.into_boxed_slice(),
            inverse: InverseEdit {
                writes: inverse.into_boxed_slice(),
            },
            durability: DurabilityTicket(commit),
        })
    }

    fn validate_chunk(&self, data: &ChunkData) -> Result<(), CommitError> {
        // Packed palettes list exactly the blocks present, so checking them avoids
        // expanding the chunk; the dense scan only runs to report the first bad cell.
        let distinct: &[BlockStateId] = match &data.blocks {
            PalettedBlocks::Uniform(block) => std::slice::from_ref(block),
            PalettedBlocks::Packed { palette, .. } => palette,
            PalettedBlocks::Direct(blocks) => blocks,
        };
        if distinct
            .iter()
            .all(|block| self.registry.get(*block).is_some())
        {
            return Ok(());
        }
        for block in data.blocks.to_dense() {
            if self.registry.get(block).is_none() {
                return Err(CommitError::UnknownBlock(block));
            }
        }
        Ok(())
    }
}

impl VoxelView for World {
    fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
        let (chunk, local) = split_voxel(pos);
        if let Some(slot) = self.chunks.get(&chunk) {
            Sample::Loaded(slot.data.blocks.get(local.index()))
        } else if let Some(&cause) = self.unavailable.get(&chunk) {
            Sample::Unavailable { chunk, cause }
        } else {
            Sample::Unloaded { chunk }
        }
    }

    fn chunk(&self, pos: ChunkPos) -> Option<ChunkSnapshot> {
        self.chunks.get(&pos).map(|slot| ChunkSnapshot {
            pos,
            revision: slot.revision,
            data: Arc::clone(&slot.data),
        })
    }
}

fn dirty_bounds(writes: &[(ChunkPos, LocalPos, VoxelWrite)]) -> DirtyBounds {
    let mut min_x = writes[0].1.x();
    let mut min_y = writes[0].1.y();
    let mut min_z = writes[0].1.z();
    let mut max_x = min_x;
    let mut max_y = min_y;
    let mut max_z = min_z;
    for &(_, local, _) in &writes[1..] {
        min_x = min_x.min(local.x());
        min_y = min_y.min(local.y());
        min_z = min_z.min(local.z());
        max_x = max_x.max(local.x());
        max_y = max_y.max(local.y());
        max_z = max_z.max(local.z());
    }
    DirtyBounds {
        min: LocalPos::new(min_x, min_y, min_z).expect("minimum remains local"),
        max_inclusive: LocalPos::new(max_x, max_y, max_z).expect("maximum remains local"),
    }
}

fn invalidated_neighbors(
    pos: ChunkPos,
    writes: &[(ChunkPos, LocalPos, VoxelWrite)],
) -> Box<[ChunkPos]> {
    let mut mask = 0_u32;
    for &(_, local, _) in writes {
        mask |= local.affected_neighbor_chunks();
    }
    mask &= !(1_u32 << 13);
    if mask == 0 {
        return Box::new([]);
    }
    let mut out = Vec::with_capacity(mask.count_ones() as usize);
    for dx in -1_i64..=1 {
        for dy in -1_i64..=1 {
            for dz in -1_i64..=1 {
                let bit = (dx + 1) as u32 + 3 * ((dz + 1) as u32 + 3 * (dy + 1) as u32);
                if (mask & (1_u32 << bit)) != 0
                    && let (Some(x), Some(y), Some(z)) = (
                        pos.x.checked_add(dx),
                        pos.y.checked_add(dy),
                        pos.z.checked_add(dz),
                    )
                {
                    out.push(ChunkPos { x, y, z });
                }
            }
        }
    }
    out.into_boxed_slice()
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("world commit failed: {self:?}")]
pub enum CommitError {
    EmptyTransaction,
    TooManyWrites(usize),
    TooManyChunks(usize),
    DuplicateWrite(VoxelPos),
    DuplicateExpectation(ChunkPos),
    DuplicateResidencyChange(ChunkPos),
    ChunkNotLoaded(ChunkPos),
    ChunkAlreadyLoaded(ChunkPos),
    ChunkRequiresRestore(ChunkPos),
    UnknownBlock(BlockStateId),
    RevisionConflict {
        pos: ChunkPos,
        expected: ChunkRevision,
        actual: ChunkRevision,
    },
    RevisionOverflow(ChunkPos),
    CommitIdOverflow,
    InvalidChunk(ChunkPos),
    TickWentBackwards {
        current: TickId,
        requested: TickId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::test_registry;

    fn setup() -> (World, BlockStateId, ChunkPos) {
        let registry = Arc::new(test_registry());
        let stone = registry
            .find(&crate::ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let pos = ChunkPos { x: 0, y: 0, z: 0 };
        let mut world = World::new(
            WorldEpoch::new(1).unwrap(),
            registry,
            WorldLimits::default(),
        );
        world
            .insert_generated(GeneratedChunk {
                pos,
                data: ChunkData::uniform(BlockStateId::AIR),
            })
            .unwrap();
        (world, stone, pos)
    }

    #[test]
    fn residency_batch_keeps_world_unchanged_when_later_input_is_invalid() {
        let (mut world, _, resident) = setup();
        let returning = ChunkPos { x: 1, ..resident };
        let fresh = ChunkPos { x: 2, ..resident };
        let generated = |pos| GeneratedChunk {
            pos,
            data: ChunkData::uniform(BlockStateId::AIR),
        };
        world.insert_generated(generated(returning)).unwrap();
        let snapshot = world.unload_chunk(returning).unwrap();
        assert!(matches!(
            world.replace_resident_chunks(
                &[resident],
                std::slice::from_ref(&snapshot),
                vec![generated(resident)]
            ),
            Err(CommitError::DuplicateResidencyChange(pos)) if pos == resident
        ));
        let mut stale = snapshot.clone();
        stale.revision = stale.revision.checked_next().unwrap();
        assert!(matches!(
            world.replace_resident_chunks(&[resident], &[stale], vec![generated(fresh)]),
            Err(CommitError::RevisionConflict { pos, .. }) if pos == returning
        ));
        assert!(world.chunk(resident).is_some());
        assert!(world.chunk(returning).is_none());
        assert!(world.chunk(fresh).is_none());
        let evicted = world
            .replace_resident_chunks(&[resident], &[snapshot], vec![generated(fresh)])
            .unwrap();
        assert_eq!(evicted[0].pos, resident);
        assert!(world.chunk(resident).is_none());
        assert!(world.chunk(returning).is_some());
        assert!(world.chunk(fresh).is_some());
    }

    #[test]
    fn streaming_discards_pristine_payload_but_keeps_revision_and_edits() {
        let (mut world, stone, chunk) = setup();
        let voxel = VoxelPos { x: 4, y: 5, z: 6 };
        let generate = || GeneratedChunk {
            pos: chunk,
            data: ChunkData::uniform(BlockStateId::AIR),
        };
        for _ in 0..2 {
            let previous = world.chunk(chunk).unwrap().revision;
            assert!(
                world
                    .replace_resident_generated_chunks(&[chunk], &[], vec![])
                    .unwrap()
                    .is_empty()
            );
            assert!(world.unloaded_revisions.is_empty());
            world.insert_generated(generate()).unwrap();
            assert!(world.chunk(chunk).unwrap().revision > previous);
            assert!(matches!(
                world.commit(EditTxn {
                    source: EditSource::Simulation,
                    expected: vec![(chunk, previous)],
                    writes: vec![VoxelWrite {
                        pos: voxel,
                        block: stone
                    }],
                }),
                Err(CommitError::RevisionConflict { .. })
            ));
        }
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![],
                writes: vec![VoxelWrite {
                    pos: voxel,
                    block: stone,
                }],
            })
            .unwrap();
        let retained = world
            .replace_resident_generated_chunks(&[chunk], &[], vec![])
            .unwrap();
        assert_eq!(retained.len(), 1);
        assert!(
            matches!(world.replace_resident_generated_chunks(&[], &[], vec![generate()]), Err(CommitError::ChunkRequiresRestore(pos)) if pos == chunk)
        );
        world
            .replace_resident_generated_chunks(&[], &retained, vec![])
            .unwrap();
        assert_eq!(world.sample(voxel), Sample::Loaded(stone));
    }

    #[test]
    fn unload_preserves_edits_and_invalidates_old_plans_after_restore() {
        let (mut world, stone, chunk) = setup();
        let pos = VoxelPos { x: 4, y: 5, z: 6 };
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![],
                writes: vec![VoxelWrite { pos, block: stone }],
            })
            .unwrap();
        let snapshot = world.unload_chunk(chunk).unwrap();
        let old = snapshot.clone();
        assert_eq!(world.sample(pos), Sample::Unloaded { chunk });
        assert_eq!(
            world.insert_generated(GeneratedChunk {
                pos: chunk,
                data: ChunkData::uniform(BlockStateId::AIR)
            }),
            Err(CommitError::ChunkRequiresRestore(chunk))
        );
        world.restore_chunk(&snapshot).unwrap();
        assert_eq!(world.sample(pos), Sample::Loaded(stone));
        assert_eq!(
            world.chunk(chunk).unwrap().revision.get(),
            old.revision.get() + 1
        );
        assert!(matches!(
            world.commit(EditTxn {
                source: EditSource::Simulation,
                expected: vec![(chunk, old.revision)],
                writes: vec![VoxelWrite {
                    pos,
                    block: BlockStateId::AIR
                }]
            }),
            Err(CommitError::RevisionConflict { .. })
        ));
        let current = world.unload_chunk(chunk).unwrap();
        assert!(matches!(
            world.restore_chunk(&old),
            Err(CommitError::RevisionConflict { .. })
        ));
        world.restore_chunk(&current).unwrap();
        assert_eq!(world.sample(pos), Sample::Loaded(stone));
    }

    #[test]
    fn failed_restore_keeps_retained_data_and_unloaded_state() {
        let (mut world, _, chunk) = setup();
        let retained = world.unload_chunk(chunk).unwrap();
        let mut corrupt = retained.clone();
        corrupt.data = Arc::new(ChunkData::uniform(BlockStateId::from_test(u32::MAX)));
        assert!(matches!(
            world.restore_chunk(&corrupt),
            Err(CommitError::UnknownBlock(_))
        ));
        assert!(world.chunk(chunk).is_none());
        assert_eq!(
            world.unloaded_revisions.get(&chunk),
            Some(&retained.revision)
        );
        world.restore_chunk(&retained).unwrap();
        assert!(Arc::ptr_eq(
            &world.chunk(chunk).unwrap().data,
            &retained.data
        ));
        assert_eq!(
            world.restore_chunk(&retained),
            Err(CommitError::ChunkAlreadyLoaded(chunk))
        );
    }

    #[test]
    fn unload_revision_overflow_does_not_remove_resident_data() {
        let (mut world, _, chunk) = setup();
        world.chunks.get_mut(&chunk).unwrap().revision = ChunkRevision::from_raw(u64::MAX);
        assert!(
            matches!(world.unload_chunk(chunk), Err(CommitError::RevisionOverflow(pos)) if pos == chunk)
        );
        assert!(world.chunk(chunk).is_some());
        assert!(!world.unloaded_revisions.contains_key(&chunk));
    }

    #[test]
    fn transaction_is_atomic_and_builds_inverse() {
        let (mut world, stone, chunk) = setup();
        world.set_tick(TickId::new(12)).unwrap();
        let receipt = world
            .commit(EditTxn {
                source: EditSource::Player(7),
                expected: vec![(chunk, ChunkRevision::default())],
                writes: vec![
                    VoxelWrite {
                        pos: VoxelPos { x: 0, y: 0, z: 0 },
                        block: stone,
                    },
                    VoxelWrite {
                        pos: VoxelPos { x: 4, y: 5, z: 6 },
                        block: stone,
                    },
                ],
            })
            .unwrap();
        assert_eq!(receipt.tick, TickId::new(12));
        assert_eq!(receipt.chunks[0].after_revision.get(), 1);
        assert_eq!(receipt.inverse.writes.len(), 2);
        assert_eq!(receipt.chunks[0].invalidated_neighbor_meshes.len(), 7);
        assert_eq!(
            world.sample(VoxelPos { x: 4, y: 5, z: 6 }),
            Sample::Loaded(stone)
        );
    }

    #[test]
    fn duplicate_or_conflicting_transaction_changes_nothing() {
        let (mut world, stone, chunk) = setup();
        let before = world.chunk(chunk).unwrap();
        let target = VoxelPos { x: 1, y: 2, z: 3 };
        assert!(matches!(
            world.commit(EditTxn {
                source: EditSource::Editor,
                expected: Vec::new(),
                writes: vec![
                    VoxelWrite {
                        pos: target,
                        block: stone
                    },
                    VoxelWrite {
                        pos: target,
                        block: BlockStateId::AIR
                    },
                ],
            }),
            Err(CommitError::DuplicateWrite(_))
        ));
        let after_duplicate = world.chunk(chunk).unwrap();
        assert_eq!(after_duplicate.revision, before.revision);
        assert_eq!(after_duplicate.data, before.data);

        assert!(matches!(
            world.commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![(chunk, ChunkRevision::default().checked_next().unwrap())],
                writes: vec![VoxelWrite {
                    pos: target,
                    block: stone
                }],
            }),
            Err(CommitError::RevisionConflict { .. })
        ));
        assert_eq!(world.chunk(chunk).unwrap().revision, before.revision);
    }

    #[test]
    fn missing_and_unavailable_are_not_air() {
        let (mut world, _, _) = setup();
        let missing = VoxelPos { x: 32, y: 0, z: 0 };
        assert!(matches!(world.sample(missing), Sample::Unloaded { .. }));
        world.mark_unavailable(ChunkPos { x: 1, y: 0, z: 0 }, UnavailableReason::Corrupt);
        assert!(matches!(world.sample(missing), Sample::Unavailable { .. }));
    }
}
