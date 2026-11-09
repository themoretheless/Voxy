//! Versioned, bounded chunk codec and crash-recoverable write-ahead log.

mod actor;
mod codec;
mod region;
mod wal;

pub use actor::{
    ActorJoinError, JournalSubmitError, StorageActor, StoragePending, StorageReply,
    StorageRequestId, StorageResult, StoreError, token_bucket_io_admission,
};
pub use codec::{
    CodecError, StoredChunk, decode_chunk, encode_chunk, zigzag_delta_decode_i32,
    zigzag_delta_encode_i32,
};
pub use region::{
    RegionCheckpoint, RegionError, RegionPos, RegionRecovery, RegionStore,
    region_sector_fragmentation_ratio, split_region,
};
pub use voxy_core::WorldEpoch;
pub use wal::{
    PersistCommit, WalCursor, WalError, WalRecovery, WalWriter, XorDeltaChunkPatch, recover_wal,
    wal_checkpoint_urgency_score,
};
