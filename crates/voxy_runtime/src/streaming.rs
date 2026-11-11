//! Bounded background chunk-rebuild worker pool feeding [`BarrierQueue`] and
//! durable [`voxy_storage`] persistence integration (`StorageActor` + WAL recovery).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use voxy_core::{ChunkPos, WorldEpoch};
use voxy_storage::{
    ActorJoinError, JournalSubmitError, PersistCommit, StorageActor, StoragePending, StorageResult,
    StoreError, WalRecovery,
};
use voxy_world::{BlockRegistry, ChunkSnapshot, CommitError, CommitId, CommitReceipt, World};

use crate::{
    BarrierQueue, BootstrapChunk, BootstrapError, Completion, CompletionKey, JobClass,
    rebuild_chunks_from_snapshots, rebuild_vertical_chunks_from_snapshots,
};

#[derive(Debug)]
pub struct RebuildBatchOutcome {
    pub derivation_epoch: u64,
    pub positions: Vec<ChunkPos>,
    pub duration: Duration,
    pub result: Result<Vec<BootstrapChunk>, BootstrapError>,
}

struct RebuildRequest {
    world_epoch: WorldEpoch,
    key: CompletionKey,
    derivation_epoch: u64,
    vertical_propagation: bool,
    registry: Arc<BlockRegistry>,
    snapshots: BTreeMap<ChunkPos, ChunkSnapshot>,
    positions: Vec<ChunkPos>,
}

#[derive(Debug, thiserror::Error)]
pub enum RebuildSubmitError {
    #[error("rebuild worker queue is full")]
    QueueFull,
    #[error("rebuild worker pool stopped")]
    PoolStopped,
}

/// Bounded persistent worker pool for off-thread chunk lighting + meshing that routes
/// completed batches through a [`BarrierQueue`] for epoch-checked, deterministic application.
#[derive(Debug)]
pub struct RebuildWorkerPool {
    sender: Option<SyncSender<RebuildRequest>>,
    receiver: Receiver<Option<Completion<RebuildBatchOutcome>>>,
    active_epoch: Arc<AtomicU64>,
    barrier: BarrierQueue<RebuildBatchOutcome>,
    next_ticket: u64,
    in_flight: usize,
    max_outstanding: usize,
    workers: Vec<JoinHandle<()>>,
}

impl RebuildWorkerPool {
    /// Spawns `worker_count` persistent worker threads (`1..=8`) sharing a bounded work queue.
    #[must_use]
    pub fn new(world_epoch: WorldEpoch, worker_count: usize, queue_capacity: usize) -> Self {
        let worker_count = worker_count.clamp(1, 8);
        let queue_capacity = queue_capacity.max(1);
        let (work_tx, work_rx) = mpsc::sync_channel::<RebuildRequest>(queue_capacity);
        let (done_tx, done_rx) = mpsc::channel();
        let active_epoch = Arc::new(AtomicU64::new(world_epoch.get()));
        let shared_rx = Arc::new(std::sync::Mutex::new(work_rx));

        let mut workers = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let rx = Arc::clone(&shared_rx);
            let tx = done_tx.clone();
            let active_epoch = Arc::clone(&active_epoch);
            if let Ok(handle) = thread::Builder::new()
                .name(format!("voxy-rebuild-{index}"))
                .spawn(move || {
                    loop {
                        let request = {
                            let Ok(guard) = rx.lock() else {
                                break;
                            };
                            match guard.recv() {
                                Ok(req) => req,
                                Err(_) => break,
                            }
                        };
                        if request.world_epoch.get() != active_epoch.load(Ordering::Acquire) {
                            if tx.send(None).is_err() {
                                break;
                            }
                            continue;
                        }
                        let started = Instant::now();
                        let result = if request.vertical_propagation {
                            rebuild_vertical_chunks_from_snapshots(
                                &request.registry,
                                &request.snapshots,
                                &request.positions,
                                request.derivation_epoch,
                            )
                        } else {
                            rebuild_chunks_from_snapshots(
                                &request.registry,
                                &request.snapshots,
                                &request.positions,
                                request.derivation_epoch,
                            )
                        };
                        let completion = Completion {
                            world_epoch: request.world_epoch,
                            key: request.key,
                            value: RebuildBatchOutcome {
                                derivation_epoch: request.derivation_epoch,
                                positions: request.positions,
                                duration: started.elapsed(),
                                result,
                            },
                        };
                        if tx.send(Some(completion)).is_err() {
                            break;
                        }
                    }
                })
            {
                workers.push(handle);
            }
        }

        let max_outstanding = queue_capacity.saturating_add(workers.len());
        Self {
            sender: Some(work_tx),
            receiver: done_rx,
            active_epoch,
            barrier: BarrierQueue::new(world_epoch),
            next_ticket: 0,
            in_flight: 0,
            max_outstanding,
            workers,
        }
    }

    #[must_use]
    pub const fn world_epoch(&self) -> WorldEpoch {
        self.barrier.world_epoch()
    }

    #[must_use]
    pub const fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Advances the active [`WorldEpoch`], discarding unpublished old-world results.
    /// Workers skip queued requests from older epochs. An already running
    /// rebuild may finish, and the barrier discards its result.
    pub fn advance_epoch(&mut self) -> Option<WorldEpoch> {
        let epoch = self.barrier.advance_epoch()?;
        self.active_epoch.store(epoch.get(), Ordering::Release);
        Some(epoch)
    }

    /// Submits an extracted neighborhood batch (`extract_chunk_neighborhood`) without cloning `World`.
    ///
    /// # Errors
    ///
    /// Returns [`RebuildSubmitError::QueueFull`] when queued, running and
    /// unpublished batches exhaust the shared capacity, or
    /// [`RebuildSubmitError::PoolStopped`] if all workers exited.
    pub fn try_submit(
        &mut self,
        class: JobClass,
        derivation_epoch: u64,
        vertical_propagation: bool,
        registry: Arc<BlockRegistry>,
        snapshots: BTreeMap<ChunkPos, ChunkSnapshot>,
        positions: Vec<ChunkPos>,
    ) -> Result<CompletionKey, RebuildSubmitError> {
        let Some(sender) = &self.sender else {
            return Err(RebuildSubmitError::PoolStopped);
        };
        if self.in_flight.saturating_add(self.barrier.len()) >= self.max_outstanding {
            return Err(RebuildSubmitError::QueueFull);
        }
        let anchor = positions.first().copied().unwrap_or_default();
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        let key = CompletionKey {
            class,
            chunk: anchor,
            ticket,
        };
        let request = RebuildRequest {
            world_epoch: self.barrier.world_epoch(),
            key,
            derivation_epoch,
            vertical_propagation,
            registry,
            snapshots,
            positions,
        };
        match sender.try_send(request) {
            Ok(()) => {
                self.in_flight += 1;
                Ok(key)
            }
            Err(TrySendError::Full(_)) => Err(RebuildSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(RebuildSubmitError::PoolStopped),
        }
    }

    /// Pulls finished worker completions into the internal [`BarrierQueue`] and drains up to
    /// `max_batches` in canonical `(JobClass, ChunkPos, ticket)` order.
    pub fn drain_ready(&mut self, max_batches: usize) -> Vec<(CompletionKey, RebuildBatchOutcome)> {
        while let Ok(completion) = self.receiver.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            if let Some(completion) = completion {
                let _ = self.barrier.push(completion);
            }
        }
        self.barrier.drain_barrier(max_batches)
    }
}

impl Drop for RebuildWorkerPool {
    fn drop(&mut self) {
        // Zero is never a live world epoch. Pending requests can be dropped
        // without running their rebuild while shutdown joins active workers.
        self.active_epoch.store(0, Ordering::Release);
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("storage error: {0}")]
    Store(#[from] StoreError),
    #[error("world commit replay error: {0:?}")]
    World(CommitError),
    #[error("storage actor join error: {0}")]
    Join(#[from] ActorJoinError),
}

impl From<CommitError> for PersistenceError {
    fn from(err: CommitError) -> Self {
        Self::World(err)
    }
}

/// Runtime-owned bridge between authoritative [`World`] commits and [`voxy_storage::StorageActor`].
#[derive(Debug)]
pub struct WorldPersistence {
    actor: StorageActor,
}

impl WorldPersistence {
    /// Opens the WAL at `wal_path`, replays all recovered durable commits onto `world`,
    /// and starts the background [`StorageActor`].
    ///
    /// # Errors
    ///
    /// Reports WAL recovery, block validation, or actor spawn failures.
    pub fn open_and_recover(
        wal_path: impl AsRef<Path>,
        world: &mut World,
        queue_capacity: usize,
    ) -> Result<(Self, WalRecovery), PersistenceError> {
        let (actor, recovery) = StorageActor::spawn(
            wal_path,
            world.registry_handle(),
            world.epoch(),
            queue_capacity.max(1),
        )?;
        apply_wal_recovery(world, &recovery)?;
        Ok((Self { actor }, recovery))
    }

    /// Submits a committed [`CommitReceipt`] to the background WAL journal without blocking I/O.
    ///
    /// # Errors
    ///
    /// Returns backpressure or actor-stopped errors with the preserved [`PersistCommit`].
    pub fn journal_receipt(
        &mut self,
        receipt: &CommitReceipt,
    ) -> Result<StoragePending, JournalSubmitError> {
        self.actor.try_journal(PersistCommit::from(receipt))
    }

    /// Flushes durable WAL commits up to `through` and waits for the durability barrier.
    ///
    /// # Errors
    ///
    /// Reports storage or actor failures.
    pub fn flush_blocking(&mut self, through: Option<CommitId>) -> Result<(), PersistenceError> {
        let reply = self.actor.flush(through)?.recv()?;
        match reply.result {
            StorageResult::FlushComplete { .. } => Ok(()),
            StorageResult::Failed(err) => Err(PersistenceError::Store(err)),
            _ => Err(PersistenceError::Store(StoreError::ActorStopped)),
        }
    }

    /// Cleanly drains, syncs, and joins the background storage actor thread.
    ///
    /// # Errors
    ///
    /// Reports flush or thread join errors.
    pub fn shutdown(mut self, through: Option<CommitId>) -> Result<(), PersistenceError> {
        let reply = self.actor.shutdown(through)?.recv()?;
        self.actor.join()?;
        match reply.result {
            StorageResult::ShutdownComplete { .. } => Ok(()),
            StorageResult::Failed(err) => Err(PersistenceError::Store(err)),
            _ => Err(PersistenceError::Store(StoreError::ActorStopped)),
        }
    }
}

/// Applies recovered [`WalRecovery`] frames onto `world`, restoring chunk revisions and commit IDs.
///
/// # Errors
///
/// Rejects unknown blocks, revision rollbacks, or sequence overflow.
pub fn apply_wal_recovery(world: &mut World, recovery: &WalRecovery) -> Result<(), CommitError> {
    for commit in &recovery.commits {
        for stored in &commit.chunks {
            world.apply_persisted_chunk(
                ChunkSnapshot {
                    pos: stored.pos,
                    revision: stored.revision,
                    data: Arc::new(stored.data.clone()),
                },
                Some(commit.sequence),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use voxy_core::VoxelPos;
    use voxy_world::{
        BlockStateId, EditSource, EditTxn, ResourceKey, Sample, VoxelView, VoxelWrite,
    };

    use super::*;
    use crate::{build_procedural_scene, build_vertical_procedural_scene};

    fn unique_wal_path(tag: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "voxy_runtime_{tag}_{}_{id}.wal",
            std::process::id()
        ))
    }

    #[test]
    fn worker_pool_rebuilds_extracted_neighborhood_through_barrier_queue() {
        let scene = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
        let epoch = scene.world.epoch();
        let mut pool = RebuildWorkerPool::new(epoch, 2, 8);
        let target = vec![ChunkPos { x: 0, y: 0, z: 0 }];
        let snapshots = crate::extract_chunk_neighborhood(&scene.world, &target);
        assert!(snapshots.len() <= 27);
        pool.try_submit(
            JobClass::Visible,
            2,
            false,
            scene.world.registry_handle(),
            snapshots,
            target.clone(),
        )
        .unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut drained = Vec::new();
        while drained.is_empty() && Instant::now() < deadline {
            drained = pool.drain_ready(4);
            if drained.is_empty() {
                thread::sleep(Duration::from_millis(2));
            }
        }
        assert_eq!(drained.len(), 1);
        let chunks = drained.pop().unwrap().1.result.unwrap();
        let expected = crate::rebuild_bootstrap_chunks(&scene.world, &target, 2).unwrap();
        assert_eq!(chunks, expected);
    }

    #[test]
    fn unpublished_rebuilds_apply_backpressure_until_drained_or_invalidated() {
        let scene = build_procedural_scene(19, 0).unwrap();
        let mut pool = RebuildWorkerPool::new(scene.world.epoch(), 1, 1);
        let submit = |pool: &mut RebuildWorkerPool, epoch| {
            pool.try_submit(
                JobClass::Visible,
                epoch,
                false,
                scene.world.registry_handle(),
                BTreeMap::new(),
                Vec::new(),
            )
        };
        let collect = |pool: &mut RebuildWorkerPool| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while pool.in_flight() != 0 && Instant::now() < deadline {
                assert!(pool.drain_ready(0).is_empty());
                thread::yield_now();
            }
            assert_eq!(pool.in_flight(), 0);
        };
        for epoch in [1, 2] {
            submit(&mut pool, epoch).unwrap();
            collect(&mut pool);
        }
        assert!(matches!(
            submit(&mut pool, 3),
            Err(RebuildSubmitError::QueueFull)
        ));
        let first = pool.drain_ready(1);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1.derivation_epoch, 1);
        assert!(first[0].1.result.is_ok());
        submit(&mut pool, 3).unwrap();
        collect(&mut pool);
        assert!(matches!(
            submit(&mut pool, 4),
            Err(RebuildSubmitError::QueueFull)
        ));

        pool.advance_epoch().unwrap();
        assert!(pool.drain_ready(4).is_empty());
        submit(&mut pool, 4).unwrap();
        collect(&mut pool);
        let current = pool.drain_ready(4);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].1.derivation_epoch, 4);
        assert!(current[0].1.result.is_ok());

        submit(&mut pool, 5).unwrap();
        pool.advance_epoch().unwrap();
        collect(&mut pool);
        assert!(pool.drain_ready(4).is_empty());
    }

    #[test]
    fn worker_acknowledges_an_old_request_without_producing_a_mesh() {
        let scene = build_procedural_scene(19, 0).unwrap();
        let old_epoch = scene.world.epoch();
        let mut pool = RebuildWorkerPool::new(old_epoch, 1, 1);
        pool.advance_epoch().unwrap();

        // Emulate an old request dequeued after the epoch changed.
        let pos = ChunkPos { x: -1, y: 0, z: 0 };
        pool.sender
            .as_ref()
            .unwrap()
            .send(RebuildRequest {
                world_epoch: old_epoch,
                key: CompletionKey {
                    class: JobClass::Visible,
                    chunk: pos,
                    ticket: 0,
                },
                derivation_epoch: 1,
                vertical_propagation: false,
                registry: scene.world.registry_handle(),
                snapshots: BTreeMap::new(),
                positions: vec![pos],
            })
            .unwrap();
        assert!(
            pool.receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .is_none()
        );

        pool.try_submit(
            JobClass::Visible,
            2,
            false,
            scene.world.registry_handle(),
            BTreeMap::new(),
            Vec::new(),
        )
        .unwrap();
        let current = pool
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(current.world_epoch, pool.world_epoch());
        assert!(current.value.result.is_ok());
    }

    #[test]
    fn vertical_scene_meshes_multiple_y_layers_and_propagates_sky_downward() {
        let scene = build_vertical_procedural_scene(0x56_4f_58_59, 0, -1, 1, false).unwrap();
        assert_eq!(scene.chunks.len(), 3);
        let y_layers: Vec<i64> = scene.chunks.iter().map(|c| c.pos.y).collect();
        assert_eq!(y_layers, vec![-1, 0, 1]);
        let mid = scene.chunks.iter().find(|c| c.pos.y == 0).unwrap();
        assert!(!mid.mesh.opaque.is_empty());
        // Top of y=0 chunk under open y=1 air receives full sky light from y=1 volume.
        assert!(mid.light.bytes().iter().any(|&b| (b >> 4) == 15));
    }

    #[test]
    fn world_persistence_journals_and_recovers_commits_across_restart() {
        let wal_path = unique_wal_path("persist_roundtrip");
        let _ = std::fs::remove_file(&wal_path);

        let mut scene = build_procedural_scene(19, 0).unwrap();
        let stone = scene
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let edit_pos = VoxelPos { x: 8, y: 25, z: 8 };
        {
            let (mut persistence, initial_recovery) =
                WorldPersistence::open_and_recover(&wal_path, &mut scene.world, 8).unwrap();
            assert!(initial_recovery.commits.is_empty());
            let receipt = scene
                .world
                .commit(EditTxn {
                    source: EditSource::Player(1),
                    expected: Vec::new(),
                    writes: vec![VoxelWrite {
                        pos: edit_pos,
                        block: stone,
                    }],
                })
                .unwrap();
            let pending = persistence.journal_receipt(&receipt).unwrap();
            let reply = pending.recv().unwrap();
            assert!(matches!(
                reply.result,
                StorageResult::Journaled { sequence: 0, .. }
            ));
            persistence.shutdown(Some(receipt.commit)).unwrap();
        }

        let mut fresh = build_procedural_scene(19, 0).unwrap();
        assert_eq!(
            fresh.world.sample(edit_pos),
            Sample::Loaded(BlockStateId::AIR)
        );
        let (persistence, recovery) =
            WorldPersistence::open_and_recover(&wal_path, &mut fresh.world, 8).unwrap();
        assert_eq!(recovery.commits.len(), 1);
        assert_eq!(fresh.world.sample(edit_pos), Sample::Loaded(stone));

        // Subsequent commits continue gap-free at sequence 1 without conflict.
        let receipt2 = fresh
            .world
            .commit(EditTxn {
                source: EditSource::Player(1),
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: edit_pos,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert_eq!(receipt2.commit.get(), 1);
        persistence.shutdown(None).unwrap();
        let _ = std::fs::remove_file(&wal_path);
    }
}
