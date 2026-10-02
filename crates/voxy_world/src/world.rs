use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use voxy_core::{ChunkPos, LocalPos, TickId, VoxelPos, WorldEpoch, split_voxel};

use crate::{BlockRegistry, BlockStateId, ChunkData, ChunkRevision, ChunkSnapshot, GeneratedChunk};

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
            tick: TickId::new(0),
            next_commit: 0,
            limits,
        }
    }

    #[must_use]
    pub const fn epoch(&self) -> WorldEpoch {
        self.epoch
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

    /// Adds generated canonical data at revision zero.
    ///
    /// # Errors
    ///
    /// Rejects replacement of a loaded chunk and unknown block IDs.
    pub fn insert_generated(&mut self, generated: GeneratedChunk) -> Result<(), CommitError> {
        if self.chunks.contains_key(&generated.pos) {
            return Err(CommitError::ChunkAlreadyLoaded(generated.pos));
        }
        if self.unloaded_revisions.contains_key(&generated.pos) {
            return Err(CommitError::ChunkRequiresRestore(generated.pos));
        }
        self.validate_chunk(&generated.data)?;
        self.unavailable.remove(&generated.pos);
        self.chunks.insert(
            generated.pos,
            ChunkSlot {
                revision: ChunkRevision::default(),
                data: Arc::new(generated.data),
            },
        );
        Ok(())
    }

    /// Removes residency while returning the exact edited data for caller retention.
    /// The caller must retain or persist the snapshot before discarding it.
    /// # Errors
    /// Rejects missing chunks and revisions that cannot advance on restoration.
    pub fn unload_chunk(&mut self, pos: ChunkPos) -> Result<ChunkSnapshot, CommitError> {
        let slot = self
            .chunks
            .get(&pos)
            .ok_or(CommitError::ChunkNotLoaded(pos))?;
        slot.revision
            .checked_next()
            .ok_or(CommitError::RevisionOverflow(pos))?;
        let slot = self
            .chunks
            .remove(&pos)
            .ok_or(CommitError::ChunkNotLoaded(pos))?;
        self.unloaded_revisions.insert(pos, slot.revision);
        Ok(ChunkSnapshot {
            pos,
            revision: slot.revision,
            data: slot.data,
        })
    }

    /// Restores retained data with a fresh revision, invalidating pre-unload plans.
    /// # Errors
    /// Rejects loaded/unavailable chunks, stale snapshots, unknown blocks and overflow.
    pub fn restore_chunk(&mut self, snapshot: &ChunkSnapshot) -> Result<(), CommitError> {
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
        self.chunks.insert(
            snapshot.pos,
            ChunkSlot {
                revision,
                data: Arc::clone(&snapshot.data),
            },
        );
        self.unloaded_revisions.remove(&snapshot.pos);
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
    #[allow(clippy::too_many_lines)]
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
        writes.sort_by_key(|(chunk, local, _)| (*chunk, local.index()));
        for pair in writes.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 {
                return Err(CommitError::DuplicateWrite(pair[0].2.pos));
            }
        }

        let touched: BTreeSet<_> = writes.iter().map(|(chunk, _, _)| *chunk).collect();
        if touched.len() > self.limits.max_chunks_per_transaction {
            return Err(CommitError::TooManyChunks(touched.len()));
        }
        for &pos in &touched {
            if !self.chunks.contains_key(&pos) {
                return Err(CommitError::ChunkNotLoaded(pos));
            }
        }
        let commit = CommitId(self.next_commit);
        let next_commit = self
            .next_commit
            .checked_add(1)
            .ok_or(CommitError::CommitIdOverflow)?;

        let mut staged = Vec::with_capacity(touched.len());
        let mut inverse = Vec::with_capacity(writes.len());
        for pos in touched {
            let slot = self
                .chunks
                .get(&pos)
                .ok_or(CommitError::ChunkNotLoaded(pos))?;
            let after_revision = slot
                .revision
                .checked_next()
                .ok_or(CommitError::RevisionOverflow(pos))?;
            let chunk_writes: Vec<_> = writes
                .iter()
                .filter(|(chunk, _, _)| *chunk == pos)
                .map(|(_, local, write)| (*local, *write))
                .collect();
            for (local, write) in &chunk_writes {
                inverse.push(VoxelWrite {
                    pos: write.pos,
                    block: slot.data.blocks.get(local.index()),
                });
            }
            let blocks = slot
                .data
                .blocks
                .with_updates(
                    &chunk_writes
                        .iter()
                        .map(|(local, write)| (local.index(), write.block))
                        .collect::<Vec<_>>(),
                )
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
                dirty: dirty_bounds(&chunk_writes),
                invalidated_neighbor_meshes: invalidated_neighbors(pos, &chunk_writes),
            });
        }

        for delta in &staged {
            if let Some(slot) = self.chunks.get_mut(&delta.pos) {
                slot.revision = delta.after_revision;
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

fn dirty_bounds(writes: &[(LocalPos, VoxelWrite)]) -> DirtyBounds {
    let mut min = writes[0].0;
    let mut max = min;
    for &(local, _) in &writes[1..] {
        min = LocalPos::new(
            min.x().min(local.x()),
            min.y().min(local.y()),
            min.z().min(local.z()),
        )
        .expect("minimum remains local");
        max = LocalPos::new(
            max.x().max(local.x()),
            max.y().max(local.y()),
            max.z().max(local.z()),
        )
        .expect("maximum remains local");
    }
    DirtyBounds {
        min,
        max_inclusive: max,
    }
}

fn invalidated_neighbors(pos: ChunkPos, writes: &[(LocalPos, VoxelWrite)]) -> Box<[ChunkPos]> {
    let mut offsets = BTreeSet::new();
    for &(local, _) in writes {
        let xs: &[i64] = match local.x() {
            0 => &[-1, 0],
            31 => &[0, 1],
            _ => &[0],
        };
        let ys: &[i64] = match local.y() {
            0 => &[-1, 0],
            31 => &[0, 1],
            _ => &[0],
        };
        let zs: &[i64] = match local.z() {
            0 => &[-1, 0],
            31 => &[0, 1],
            _ => &[0],
        };
        for &x in xs {
            for &y in ys {
                for &z in zs {
                    if (x, y, z) != (0, 0, 0) {
                        offsets.insert((x, y, z));
                    }
                }
            }
        }
    }
    offsets
        .into_iter()
        .filter_map(|(x, y, z)| {
            Some(ChunkPos {
                x: pos.x.checked_add(x)?,
                y: pos.y.checked_add(y)?,
                z: pos.z.checked_add(z)?,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitError {
    EmptyTransaction,
    TooManyWrites(usize),
    TooManyChunks(usize),
    DuplicateWrite(VoxelPos),
    DuplicateExpectation(ChunkPos),
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

impl fmt::Display for CommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "world commit failed: {self:?}")
    }
}

impl std::error::Error for CommitError {}

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
