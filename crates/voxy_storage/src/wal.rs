use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use voxy_world::{BlockRegistry, CommitReceipt};

use crate::codec::checksum;
use crate::{CodecError, StoredChunk, decode_chunk, encode_chunk};

const MAGIC: &[u8; 4] = b"VWAL";
const VERSION: u16 = 1;
const MAX_FRAME: usize = 64 * 1024 * 1024;
const MAX_WAL: u64 = 1024 * 1024 * 1024;
const MAX_CHUNKS_PER_COMMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistCommit {
    pub sequence: u64,
    pub chunks: Vec<StoredChunk>,
}

impl From<&CommitReceipt> for PersistCommit {
    /// Captures the complete canonical post-image of every chunk touched by a world commit.
    fn from(receipt: &CommitReceipt) -> Self {
        Self {
            sequence: receipt.commit.get(),
            chunks: receipt
                .chunks
                .iter()
                .map(|delta| StoredChunk {
                    pos: delta.pos,
                    revision: delta.after_revision,
                    data: (*delta.after).clone(),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WalCursor {
    pub offset: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalRecovery {
    pub commits: Vec<PersistCommit>,
    pub valid_bytes: u64,
    pub discarded_tail: bool,
}

#[derive(Debug)]
pub struct WalWriter {
    file: File,
    next_sequence: u64,
    cursor: u64,
}

impl WalWriter {
    #[must_use]
    pub const fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Opens the sole writer, recovers committed frames and truncates an incomplete/corrupt tail.
    ///
    /// # Errors
    ///
    /// Returns I/O, codec, size or sequence errors. The returned recovery explicitly reports any
    /// discarded tail so callers can surface degraded-save diagnostics.
    pub fn open(
        path: impl AsRef<Path>,
        registry: &BlockRegistry,
    ) -> Result<(Self, WalRecovery), WalError> {
        let path = path.as_ref();
        let recovery = recover_wal(path, registry)?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        if recovery.discarded_tail {
            file.set_len(recovery.valid_bytes)?;
            file.sync_all()?;
        }
        file.seek(SeekFrom::Start(recovery.valid_bytes))?;
        let next_sequence = recovery.commits.last().map_or(Ok(0), |commit| {
            commit
                .sequence
                .checked_add(1)
                .ok_or(WalError::SequenceOverflow)
        })?;
        Ok((
            Self {
                file,
                next_sequence,
                cursor: recovery.valid_bytes,
            },
            recovery,
        ))
    }

    /// Appends one complete transaction frame and executes a durability barrier before ack.
    ///
    /// # Errors
    ///
    /// Rejects non-gap-free sequences, empty/oversized commits and codec/I/O failures.
    pub fn append(
        &mut self,
        commit: &PersistCommit,
        registry: &BlockRegistry,
    ) -> Result<WalCursor, WalError> {
        if commit.sequence != self.next_sequence {
            return Err(WalError::UnexpectedSequence {
                expected: self.next_sequence,
                actual: commit.sequence,
            });
        }
        let frame = encode_commit(commit, registry)?;
        self.file.write_all(&frame)?;
        self.file.sync_data()?;
        self.cursor = self
            .cursor
            .checked_add(u64::try_from(frame.len()).map_err(|_| WalError::FrameTooLarge)?)
            .ok_or(WalError::WalTooLarge)?;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(WalError::SequenceOverflow)?;
        Ok(WalCursor {
            offset: self.cursor,
        })
    }

    /// Flushes file metadata as a stronger checkpoint/shutdown barrier.
    ///
    /// # Errors
    ///
    /// Returns the platform I/O failure.
    pub fn sync_all(&self) -> Result<(), WalError> {
        self.file.sync_all().map_err(WalError::Io)
    }
}

/// Scans complete checksum-valid transaction frames and reports the first invalid tail.
///
/// # Errors
///
/// Rejects I/O, oversized WAL/frames, invalid committed payloads and non-gap-free sequences.
pub fn recover_wal(
    path: impl AsRef<Path>,
    registry: &BlockRegistry,
) -> Result<WalRecovery, WalError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(WalRecovery {
            commits: Vec::new(),
            valid_bytes: 0,
            discarded_tail: false,
        });
    }
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len > MAX_WAL {
        return Err(WalError::WalTooLarge);
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(file_len).map_err(|_| WalError::WalTooLarge)?);
    file.read_to_end(&mut bytes)?;
    let mut offset = 0_usize;
    let mut commits = Vec::new();
    while bytes.len().saturating_sub(offset) >= 12 {
        if bytes.get(offset..offset + 4) != Some(MAGIC.as_slice()) {
            break;
        }
        let length = u32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| WalError::Truncated)?,
        );
        let length = usize::try_from(length).map_err(|_| WalError::FrameTooLarge)?;
        if length > MAX_FRAME {
            return Err(WalError::FrameTooLarge);
        }
        let frame_end = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .and_then(|value| value.checked_add(4))
            .ok_or(WalError::WalTooLarge)?;
        let Some(frame) = bytes.get(offset..frame_end) else {
            break;
        };
        let expected = u32::from_le_bytes(
            frame[frame.len() - 4..]
                .try_into()
                .map_err(|_| WalError::Truncated)?,
        );
        if checksum(&frame[..frame.len() - 4]) != expected {
            break;
        }
        let commit = decode_commit(&frame[8..8 + length], registry)?;
        let expected_sequence = commits.last().map_or(0, |previous: &PersistCommit| {
            previous.sequence.saturating_add(1)
        });
        if commit.sequence != expected_sequence {
            return Err(WalError::UnexpectedSequence {
                expected: expected_sequence,
                actual: commit.sequence,
            });
        }
        commits.push(commit);
        offset = frame_end;
    }
    Ok(WalRecovery {
        commits,
        valid_bytes: u64::try_from(offset).map_err(|_| WalError::WalTooLarge)?,
        discarded_tail: offset != bytes.len(),
    })
}

fn encode_commit(commit: &PersistCommit, registry: &BlockRegistry) -> Result<Vec<u8>, WalError> {
    if commit.chunks.is_empty() || commit.chunks.len() > MAX_CHUNKS_PER_COMMIT {
        return Err(WalError::InvalidChunkCount(commit.chunks.len()));
    }
    let mut payload = Vec::new();
    put_u16(&mut payload, VERSION);
    put_u64(&mut payload, commit.sequence);
    put_u16(
        &mut payload,
        u16::try_from(commit.chunks.len())
            .map_err(|_| WalError::InvalidChunkCount(commit.chunks.len()))?,
    );
    for chunk in &commit.chunks {
        let bytes = encode_chunk(chunk, registry)?;
        put_u32(
            &mut payload,
            u32::try_from(bytes.len()).map_err(|_| WalError::FrameTooLarge)?,
        );
        payload.extend_from_slice(&bytes);
    }
    if payload.len() > MAX_FRAME {
        return Err(WalError::FrameTooLarge);
    }
    let mut frame = Vec::with_capacity(payload.len() + 12);
    frame.extend_from_slice(MAGIC);
    put_u32(
        &mut frame,
        u32::try_from(payload.len()).map_err(|_| WalError::FrameTooLarge)?,
    );
    frame.extend_from_slice(&payload);
    let crc = checksum(&frame);
    put_u32(&mut frame, crc);
    Ok(frame)
}

fn decode_commit(bytes: &[u8], registry: &BlockRegistry) -> Result<PersistCommit, WalError> {
    let mut reader = Reader::new(bytes);
    if reader.u16()? != VERSION {
        return Err(WalError::UnsupportedVersion);
    }
    let sequence = reader.u64()?;
    let count = usize::from(reader.u16()?);
    if count == 0 || count > MAX_CHUNKS_PER_COMMIT {
        return Err(WalError::InvalidChunkCount(count));
    }
    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        let length = usize::try_from(reader.u32()?).map_err(|_| WalError::FrameTooLarge)?;
        if length > MAX_FRAME {
            return Err(WalError::FrameTooLarge);
        }
        chunks.push(decode_chunk(reader.take(length)?, registry)?);
    }
    if !reader.done() {
        return Err(WalError::TrailingBytes);
    }
    Ok(PersistCommit { sequence, chunks })
}

fn put_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn put_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], WalError> {
        let end = self.offset.checked_add(length).ok_or(WalError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(WalError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, WalError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| WalError::Truncated)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, WalError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| WalError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, WalError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| WalError::Truncated)?,
        ))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[derive(Debug)]
pub enum WalError {
    Io(std::io::Error),
    Codec(CodecError),
    UnsupportedVersion,
    InvalidChunkCount(usize),
    UnexpectedSequence { expected: u64, actual: u64 },
    SequenceOverflow,
    FrameTooLarge,
    WalTooLarge,
    Truncated,
    TrailingBytes,
}

impl From<std::io::Error> for WalError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CodecError> for WalError {
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}

impl fmt::Display for WalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "WAL error: {self:?}")
    }
}

impl std::error::Error for WalError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use voxy_core::{VoxelPos, WorldEpoch};
    use voxy_world::{
        BlockStateId, EditSource, EditTxn, GeneratedChunk, VoxelWrite, World, WorldLimits,
    };

    fn temp_path() -> std::path::PathBuf {
        static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "voxy-wal-{}-{nonce}-{sequence}.bin",
            std::process::id()
        ))
    }

    #[test]
    fn durable_append_recovers_gap_free_commits() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (mut writer, initial) = WalWriter::open(&path, &registry).unwrap();
        assert!(initial.commits.is_empty());
        let first = PersistCommit {
            sequence: 0,
            chunks: vec![chunk.clone()],
        };
        let first_cursor = writer.append(&first, &registry).unwrap();
        let second = PersistCommit {
            sequence: 1,
            chunks: vec![chunk],
        };
        let second_cursor = writer.append(&second, &registry).unwrap();
        assert!(second_cursor.offset > first_cursor.offset);
        drop(writer);
        let recovered = recover_wal(&path, &registry).unwrap();
        assert_eq!(recovered.commits, vec![first, second]);
        assert!(!recovered.discarded_tail);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn incomplete_tail_is_reported_and_truncated_on_open() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (mut writer, _) = WalWriter::open(&path, &registry).unwrap();
        writer
            .append(
                &PersistCommit {
                    sequence: 0,
                    chunks: vec![chunk],
                },
                &registry,
            )
            .unwrap();
        let valid = writer.cursor;
        drop(writer);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"VWAL\x20\x00").unwrap();
        file.sync_data().unwrap();
        drop(file);
        let recovery = recover_wal(&path, &registry).unwrap();
        assert!(recovery.discarded_tail);
        assert_eq!(recovery.valid_bytes, valid);
        let (_writer, reopened) = WalWriter::open(&path, &registry).unwrap();
        assert!(reopened.discarded_tail);
        assert_eq!(fs::metadata(&path).unwrap().len(), valid);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn writer_rejects_sequence_gaps_before_io() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (mut writer, _) = WalWriter::open(&path, &registry).unwrap();
        assert!(matches!(
            writer.append(
                &PersistCommit {
                    sequence: 2,
                    chunks: vec![chunk],
                },
                &registry,
            ),
            Err(WalError::UnexpectedSequence {
                expected: 0,
                actual: 2
            })
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn world_receipt_becomes_a_complete_persistable_post_image() {
        let (registry, initial) = crate::codec::tests::sample_chunk();
        let registry = Arc::new(registry);
        let mut world = World::new(
            WorldEpoch::new(9).unwrap(),
            Arc::clone(&registry),
            WorldLimits::default(),
        );
        world
            .insert_generated(GeneratedChunk {
                pos: initial.pos,
                data: initial.data,
            })
            .unwrap();
        let receipt = world
            .commit(EditTxn {
                source: EditSource::Player(7),
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: VoxelPos {
                        x: initial.pos.x * 32,
                        y: initial.pos.y * 32,
                        z: initial.pos.z * 32,
                    },
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();

        let persisted = PersistCommit::from(&receipt);
        assert_eq!(persisted.sequence, receipt.commit.get());
        assert_eq!(persisted.chunks.len(), 1);
        assert_eq!(persisted.chunks[0].pos, initial.pos);
        assert_eq!(
            persisted.chunks[0].revision,
            receipt.chunks[0].after_revision
        );
        assert_eq!(persisted.chunks[0].data, *receipt.chunks[0].after);
        let encoded = encode_chunk(&persisted.chunks[0], &registry).unwrap();
        assert_eq!(
            decode_chunk(&encoded, &registry).unwrap(),
            persisted.chunks[0]
        );
    }
}
