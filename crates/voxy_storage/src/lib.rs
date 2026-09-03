//! Versioned, bounded chunk codec and crash-recoverable write-ahead log.

mod actor;
mod codec;
mod region;
mod wal;

pub use actor::{
    ActorJoinError, JournalSubmitError, StorageActor, StoragePending, StorageReply,
    StorageRequestId, StorageResult, StoreError,
};
pub use codec::{CodecError, StoredChunk, decode_chunk, encode_chunk};
pub use region::{
    RegionCheckpoint, RegionError, RegionPos, RegionRecovery, RegionStore, split_region,
};
pub use voxy_core::WorldEpoch;
pub use wal::{PersistCommit, WalCursor, WalError, WalRecovery, WalWriter, recover_wal};
