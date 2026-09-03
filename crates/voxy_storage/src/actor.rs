use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use voxy_core::WorldEpoch;
use voxy_world::{BlockRegistry, CommitId};

use crate::{PersistCommit, WalCursor, WalError, WalRecovery, WalWriter};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StorageRequestId(u64);

impl StorageRequestId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub enum StorageResult {
    Journaled {
        cursor: WalCursor,
        sequence: u64,
    },
    FlushComplete {
        drained_through: Option<CommitId>,
        durable_through: Option<WalCursor>,
    },
    ShutdownComplete {
        drained_through: Option<CommitId>,
        durable_through: Option<WalCursor>,
    },
    Failed(StoreError),
}

#[derive(Debug)]
pub struct StorageReply {
    pub request: StorageRequestId,
    pub world_epoch: WorldEpoch,
    pub result: StorageResult,
}

#[derive(Debug)]
pub struct StoragePending {
    request: StorageRequestId,
    replies: Receiver<StorageReply>,
}

impl StoragePending {
    #[must_use]
    pub const fn request(&self) -> StorageRequestId {
        self.request
    }

    /// Waits for the exactly-one terminal reply reserved when the request was admitted.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ActorStopped`] if the actor terminates without replying.
    pub fn recv(self) -> Result<StorageReply, StoreError> {
        self.replies.recv().map_err(|_| StoreError::ActorStopped)
    }
}

#[derive(Debug)]
pub enum JournalSubmitError {
    Backpressure(PersistCommit),
    ActorStopped(PersistCommit),
}

#[derive(Debug)]
pub enum StoreError {
    Wal(WalError),
    ActorStopped,
    RequestIdOverflow,
    FenceUnavailable { required: u64, next_sequence: u64 },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage error: {self:?}")
    }
}

impl std::error::Error for StoreError {}

impl From<WalError> for StoreError {
    fn from(error: WalError) -> Self {
        Self::Wal(error)
    }
}

#[derive(Debug)]
pub struct ActorJoinError;

impl fmt::Display for ActorJoinError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("storage actor panicked")
    }
}

impl std::error::Error for ActorJoinError {}

#[derive(Debug)]
enum Request {
    Journal(PersistCommit),
    Flush(Option<CommitId>),
    Shutdown(Option<CommitId>),
}

#[derive(Debug)]
struct Envelope {
    id: StorageRequestId,
    epoch: WorldEpoch,
    request: Request,
    reply: SyncSender<StorageReply>,
}

#[derive(Debug)]
pub struct StorageActor {
    epoch: WorldEpoch,
    normal: SyncSender<Envelope>,
    control: mpsc::Sender<Envelope>,
    next_request: u64,
    thread: Option<JoinHandle<()>>,
}

impl StorageActor {
    /// Opens the WAL synchronously and starts its sole owning I/O thread.
    ///
    /// # Errors
    ///
    /// Returns recovery/open errors or an invalid zero normal-queue capacity.
    pub fn spawn(
        path: impl AsRef<Path>,
        registry: Arc<BlockRegistry>,
        epoch: WorldEpoch,
        normal_capacity: usize,
    ) -> Result<(Self, WalRecovery), StoreError> {
        if normal_capacity == 0 {
            return Err(StoreError::ActorStopped);
        }
        let (writer, recovery) = WalWriter::open(path, &registry)?;
        let (normal_tx, normal_rx) = mpsc::sync_channel(normal_capacity);
        let (control_tx, control_rx) = mpsc::channel();
        let actor_thread = thread::Builder::new()
            .name("voxy-storage".into())
            .spawn(move || run_actor(writer, &registry, &normal_rx, &control_rx))
            .map_err(|error| StoreError::Wal(WalError::Io(error)))?;
        Ok((
            Self {
                epoch,
                normal: normal_tx,
                control: control_tx,
                next_request: 0,
                thread: Some(actor_thread),
            },
            recovery,
        ))
    }

    /// Attempts to admit a journal operation without blocking on a full bounded queue.
    ///
    /// # Errors
    ///
    /// Returns the original immutable commit on backpressure or actor termination.
    pub fn try_journal(
        &mut self,
        commit: PersistCommit,
    ) -> Result<StoragePending, JournalSubmitError> {
        let (envelope, pending) = self.envelope(Request::Journal(commit)).map_err(|request| {
            let Request::Journal(commit) = request else {
                unreachable!();
            };
            JournalSubmitError::ActorStopped(commit)
        })?;
        match self.normal.try_send(envelope) {
            Ok(()) => Ok(pending),
            Err(TrySendError::Full(envelope)) => {
                let Request::Journal(commit) = envelope.request else {
                    unreachable!();
                };
                Err(JournalSubmitError::Backpressure(commit))
            }
            Err(TrySendError::Disconnected(envelope)) => {
                let Request::Journal(commit) = envelope.request else {
                    unreachable!();
                };
                Err(JournalSubmitError::ActorStopped(commit))
            }
        }
    }

    /// Submits a durability fence through the reserved control channel.
    ///
    /// # Errors
    ///
    /// Returns actor termination or request-ID exhaustion.
    pub fn flush(&mut self, drain_through: Option<CommitId>) -> Result<StoragePending, StoreError> {
        self.send_control(Request::Flush(drain_through))
    }

    /// Submits a fenced shutdown through the reserved control channel.
    ///
    /// # Errors
    ///
    /// Returns actor termination or request-ID exhaustion.
    pub fn shutdown(
        &mut self,
        drain_through: Option<CommitId>,
    ) -> Result<StoragePending, StoreError> {
        self.send_control(Request::Shutdown(drain_through))
    }

    /// Joins the actor after a terminal shutdown reply.
    ///
    /// # Errors
    ///
    /// Reports a worker panic.
    pub fn join(&mut self) -> Result<(), ActorJoinError> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        thread.join().map_err(|_| ActorJoinError)
    }

    fn send_control(&mut self, request: Request) -> Result<StoragePending, StoreError> {
        let (envelope, pending) = self
            .envelope(request)
            .map_err(|_| StoreError::RequestIdOverflow)?;
        self.control
            .send(envelope)
            .map_err(|_| StoreError::ActorStopped)?;
        Ok(pending)
    }

    fn envelope(&mut self, request: Request) -> Result<(Envelope, StoragePending), Request> {
        let Some(next_request) = self.next_request.checked_add(1) else {
            return Err(request);
        };
        let id = StorageRequestId(self.next_request);
        self.next_request = next_request;
        let (reply, replies) = mpsc::sync_channel(1);
        Ok((
            Envelope {
                id,
                epoch: self.epoch,
                request,
                reply,
            },
            StoragePending {
                request: id,
                replies,
            },
        ))
    }
}

fn run_actor(
    mut writer: WalWriter,
    registry: &BlockRegistry,
    normal: &Receiver<Envelope>,
    control: &Receiver<Envelope>,
) {
    let mut durable_cursor = None;
    loop {
        let envelope = match control.try_recv() {
            Ok(envelope) => envelope,
            Err(TryRecvError::Empty) => match normal.recv_timeout(Duration::from_millis(10)) {
                Ok(envelope) => envelope,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            Err(TryRecvError::Disconnected) => match normal.recv() {
                Ok(envelope) => envelope,
                Err(_) => break,
            },
        };
        let stop = matches!(envelope.request, Request::Shutdown(_));
        let result = match envelope.request {
            Request::Journal(commit) => {
                journal(&mut writer, registry, &commit, &mut durable_cursor)
            }
            Request::Flush(fence) => fence_result(
                &mut writer,
                registry,
                normal,
                fence,
                &mut durable_cursor,
                false,
            ),
            Request::Shutdown(fence) => fence_result(
                &mut writer,
                registry,
                normal,
                fence,
                &mut durable_cursor,
                true,
            ),
        };
        let _ = envelope.reply.send(StorageReply {
            request: envelope.id,
            world_epoch: envelope.epoch,
            result,
        });
        if stop {
            break;
        }
    }
}

fn journal(
    writer: &mut WalWriter,
    registry: &BlockRegistry,
    commit: &PersistCommit,
    durable_cursor: &mut Option<WalCursor>,
) -> StorageResult {
    let sequence = commit.sequence;
    match writer.append(commit, registry) {
        Ok(cursor) => {
            *durable_cursor = Some(cursor);
            StorageResult::Journaled { cursor, sequence }
        }
        Err(error) => StorageResult::Failed(error.into()),
    }
}

fn fence_result(
    writer: &mut WalWriter,
    registry: &BlockRegistry,
    normal: &Receiver<Envelope>,
    fence: Option<CommitId>,
    durable_cursor: &mut Option<WalCursor>,
    shutdown: bool,
) -> StorageResult {
    if let Some(required) = fence {
        while writer.next_sequence() <= required.get() {
            let Ok(envelope) = normal.recv() else {
                return StorageResult::Failed(StoreError::FenceUnavailable {
                    required: required.get(),
                    next_sequence: writer.next_sequence(),
                });
            };
            let result = match envelope.request {
                Request::Journal(commit) => journal(writer, registry, &commit, durable_cursor),
                Request::Flush(_) | Request::Shutdown(_) => {
                    StorageResult::Failed(StoreError::ActorStopped)
                }
            };
            let failed = matches!(result, StorageResult::Failed(_));
            let _ = envelope.reply.send(StorageReply {
                request: envelope.id,
                world_epoch: envelope.epoch,
                result,
            });
            if failed {
                return StorageResult::Failed(StoreError::FenceUnavailable {
                    required: required.get(),
                    next_sequence: writer.next_sequence(),
                });
            }
        }
    }
    if let Err(error) = writer.sync_all() {
        return StorageResult::Failed(error.into());
    }
    if shutdown {
        StorageResult::ShutdownComplete {
            drained_through: fence,
            durable_through: *durable_cursor,
        }
    } else {
        StorageResult::FlushComplete {
            drained_through: fence,
            durable_through: *durable_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use voxy_core::VoxelPos;
    use voxy_world::{
        BlockStateId, EditSource, EditTxn, GeneratedChunk, VoxelWrite, World, WorldLimits,
    };

    use super::*;

    fn temp_path() -> std::path::PathBuf {
        static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "voxy-actor-{}-{nonce}-{sequence}.wal",
            std::process::id()
        ))
    }

    #[test]
    fn shutdown_fence_drains_all_admitted_commits_before_replying() {
        let path = temp_path();
        let (registry, initial) = crate::codec::tests::sample_chunk();
        let registry = Arc::new(registry);
        let epoch = WorldEpoch::new(1).unwrap();
        let mut world = World::new(epoch, Arc::clone(&registry), WorldLimits::default());
        world
            .insert_generated(GeneratedChunk {
                pos: initial.pos,
                data: initial.data,
            })
            .unwrap();
        let origin = VoxelPos {
            x: initial.pos.x * 32,
            y: initial.pos.y * 32,
            z: initial.pos.z * 32,
        };
        let first = world
            .commit(EditTxn {
                source: EditSource::Player(1),
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: origin,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        let second = world
            .commit(EditTxn {
                source: EditSource::Player(1),
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: VoxelPos {
                        x: origin.x + 1,
                        ..origin
                    },
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();

        let (mut actor, recovery) =
            StorageActor::spawn(&path, Arc::clone(&registry), epoch, 2).unwrap();
        assert!(recovery.commits.is_empty());
        let first_pending = actor.try_journal((&first).into()).unwrap();
        let second_pending = actor.try_journal((&second).into()).unwrap();
        let shutdown = actor.shutdown(Some(second.commit)).unwrap();

        assert!(matches!(
            first_pending.recv().unwrap().result,
            StorageResult::Journaled { sequence: 0, .. }
        ));
        assert!(matches!(
            second_pending.recv().unwrap().result,
            StorageResult::Journaled { sequence: 1, .. }
        ));
        assert!(matches!(
            shutdown.recv().unwrap().result,
            StorageResult::ShutdownComplete {
                drained_through: Some(value),
                durable_through: Some(_),
            } if value == second.commit
        ));
        actor.join().unwrap();

        let recovered = crate::recover_wal(&path, &registry).unwrap();
        assert_eq!(recovered.commits.len(), 2);
        assert_eq!(recovered.commits[1], PersistCommit::from(&second));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounded_journal_admission_returns_original_commit_on_pressure() {
        let (normal, _normal_rx) = mpsc::sync_channel(1);
        let (control, _control_rx) = mpsc::channel();
        let epoch = WorldEpoch::new(2).unwrap();
        let mut actor = StorageActor {
            epoch,
            normal,
            control,
            next_request: 0,
            thread: None,
        };
        let (_, chunk) = crate::codec::tests::sample_chunk();
        let first = PersistCommit {
            sequence: 0,
            chunks: vec![chunk.clone()],
        };
        let second = PersistCommit {
            sequence: 1,
            chunks: vec![chunk],
        };
        let _pending = actor.try_journal(first).unwrap();
        let JournalSubmitError::Backpressure(returned) = actor.try_journal(second).unwrap_err()
        else {
            panic!("expected bounded backpressure");
        };
        assert_eq!(returned.sequence, 1);
    }
}
