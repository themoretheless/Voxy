use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use voxy_core::ChunkPos;
use voxy_world::BlockRegistry;

use crate::codec::checksum;
use crate::{CodecError, StoredChunk, WalCursor, decode_chunk, encode_chunk};

const REGION_EDGE: i64 = 8;
const REGION_SLOTS: usize = 8 * 8 * 8;
const FILE_MAGIC: &[u8; 4] = b"VREG";
const FRAME_MAGIC: &[u8; 4] = b"RCP1";
const VERSION: u16 = 1;
const HEADER_LEN: usize = 36;
const MAX_FRAME: usize = 64 * 1024 * 1024;
const MAX_FILE: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegionPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RegionEntry {
    chunk: StoredChunk,
    last_commit: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionCheckpoint {
    pub through: WalCursor,
    pub revisions: Box<[(ChunkPos, voxy_world::ChunkRevision)]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionRecovery {
    pub valid_bytes: u64,
    pub discarded_tail: bool,
    pub checkpoint_through: Option<WalCursor>,
    pub chunk_count: usize,
}

#[derive(Debug)]
pub struct RegionStore {
    file: File,
    region: RegionPos,
    entries: BTreeMap<u16, RegionEntry>,
    cursor: u64,
    checkpoint_through: Option<WalCursor>,
}

impl RegionStore {
    /// Opens one region file, recovers complete checkpoint frames and truncates an invalid tail.
    ///
    /// # Errors
    ///
    /// Returns format, corruption, codec or I/O errors.
    pub fn open(
        path: impl AsRef<Path>,
        region: RegionPos,
        registry: &BlockRegistry,
    ) -> Result<(Self, RegionRecovery), RegionError> {
        let path = path.as_ref();
        let (entries, recovery) = recover(path, region, registry)?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        if recovery.valid_bytes == 0 {
            let header = encode_header(region);
            file.write_all(&header)?;
            file.sync_all()?;
        } else if recovery.discarded_tail {
            file.set_len(recovery.valid_bytes)?;
            file.sync_all()?;
        }
        let cursor = if recovery.valid_bytes == 0 {
            u64::try_from(HEADER_LEN).map_err(|_| RegionError::FileTooLarge)?
        } else {
            recovery.valid_bytes
        };
        file.seek(SeekFrom::Start(cursor))?;
        Ok((
            Self {
                file,
                region,
                entries,
                cursor,
                checkpoint_through: recovery.checkpoint_through,
            },
            RegionRecovery {
                valid_bytes: cursor,
                ..recovery
            },
        ))
    }

    #[must_use]
    pub fn load(&self, pos: ChunkPos) -> Option<StoredChunk> {
        let (region, slot) = split_region(pos);
        (region == self.region)
            .then(|| self.entries.get(&slot).map(|entry| entry.chunk.clone()))
            .flatten()
    }

    #[must_use]
    pub const fn checkpoint_through(&self) -> Option<WalCursor> {
        self.checkpoint_through
    }

    /// Atomically appends a complete multi-chunk checkpoint frame and durability barrier.
    ///
    /// # Errors
    ///
    /// Rejects wrong-region chunks, duplicates, revision rollback, oversized frames and I/O.
    pub fn checkpoint(
        &mut self,
        chunks: &[(StoredChunk, u64)],
        through: WalCursor,
        registry: &BlockRegistry,
    ) -> Result<RegionCheckpoint, RegionError> {
        if chunks.is_empty() || chunks.len() > REGION_SLOTS {
            return Err(RegionError::InvalidChunkCount(chunks.len()));
        }
        if self
            .checkpoint_through
            .is_some_and(|cursor| cursor > through)
        {
            return Err(RegionError::CheckpointRollback);
        }
        let mut slots = BTreeSet::new();
        let mut staged = Vec::with_capacity(chunks.len());
        for (chunk, last_commit) in chunks {
            let (region, slot) = split_region(chunk.pos);
            if region != self.region {
                return Err(RegionError::WrongRegion {
                    expected: self.region,
                    actual: region,
                });
            }
            if !slots.insert(slot) {
                return Err(RegionError::DuplicateSlot(slot));
            }
            let candidate = RegionEntry {
                chunk: chunk.clone(),
                last_commit: *last_commit,
            };
            if let Some(old) = self.entries.get(&slot)
                && (old.chunk.revision > chunk.revision
                    || (old.chunk.revision == chunk.revision && old != &candidate))
            {
                return Err(RegionError::RevisionConflict(chunk.pos));
            }
            staged.push((slot, chunk.clone(), *last_commit));
        }
        staged.sort_by_key(|(slot, _, _)| *slot);
        let frame = encode_frame(&staged, through, registry)?;
        let frame_len = u64::try_from(frame.len()).map_err(|_| RegionError::FrameTooLarge)?;
        if self.cursor.saturating_add(frame_len) > MAX_FILE {
            return Err(RegionError::FileTooLarge);
        }
        self.file.write_all(&frame)?;
        self.file.sync_data()?;
        self.cursor += frame_len;
        self.checkpoint_through = Some(through);
        for (slot, chunk, last_commit) in &staged {
            self.entries.insert(
                *slot,
                RegionEntry {
                    chunk: chunk.clone(),
                    last_commit: *last_commit,
                },
            );
        }
        Ok(RegionCheckpoint {
            through,
            revisions: staged
                .iter()
                .map(|(_, chunk, _)| (chunk.pos, chunk.revision))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }

    /// Executes the stronger metadata durability barrier used by clean shutdown.
    ///
    /// # Errors
    ///
    /// Returns the platform I/O failure.
    pub fn sync_all(&self) -> Result<(), RegionError> {
        self.file.sync_all().map_err(RegionError::Io)
    }
}

#[must_use]
pub fn split_region(pos: ChunkPos) -> (RegionPos, u16) {
    let region = RegionPos {
        x: pos.x.div_euclid(REGION_EDGE),
        y: pos.y.div_euclid(REGION_EDGE),
        z: pos.z.div_euclid(REGION_EDGE),
    };
    let lx = u16::try_from(pos.x.rem_euclid(REGION_EDGE)).unwrap_or_default();
    let ly = u16::try_from(pos.y.rem_euclid(REGION_EDGE)).unwrap_or_default();
    let lz = u16::try_from(pos.z.rem_euclid(REGION_EDGE)).unwrap_or_default();
    let slot = lx + 8 * (lz + 8 * ly);
    (region, slot)
}

fn recover(
    path: &Path,
    region: RegionPos,
    registry: &BlockRegistry,
) -> Result<(BTreeMap<u16, RegionEntry>, RegionRecovery), RegionError> {
    if !path.exists() {
        return Ok((
            BTreeMap::new(),
            RegionRecovery {
                valid_bytes: 0,
                discarded_tail: false,
                checkpoint_through: None,
                chunk_count: 0,
            },
        ));
    }
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length > MAX_FILE {
        return Err(RegionError::FileTooLarge);
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(length).map_err(|_| RegionError::FileTooLarge)?);
    file.read_to_end(&mut bytes)?;
    if bytes.len() < HEADER_LEN {
        return Err(RegionError::InvalidHeader);
    }
    decode_header(&bytes[..HEADER_LEN], region)?;
    let mut entries: BTreeMap<u16, RegionEntry> = BTreeMap::new();
    let mut offset = HEADER_LEN;
    let mut checkpoint_through = None;
    while bytes.len().saturating_sub(offset) >= 12 {
        if bytes.get(offset..offset + 4) != Some(FRAME_MAGIC.as_slice()) {
            break;
        }
        let payload_len = usize::try_from(u32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| RegionError::Truncated)?,
        ))
        .map_err(|_| RegionError::FrameTooLarge)?;
        if payload_len > MAX_FRAME {
            return Err(RegionError::FrameTooLarge);
        }
        let Some(end) = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(payload_len))
            .and_then(|value| value.checked_add(4))
        else {
            return Err(RegionError::FileTooLarge);
        };
        let Some(frame) = bytes.get(offset..end) else {
            break;
        };
        let expected = u32::from_le_bytes(
            frame[frame.len() - 4..]
                .try_into()
                .map_err(|_| RegionError::Truncated)?,
        );
        if checksum(&frame[..frame.len() - 4]) != expected {
            break;
        }
        let (through, decoded) = decode_frame(&frame[8..8 + payload_len], region, registry)?;
        if checkpoint_through.is_some_and(|old: WalCursor| old > through) {
            return Err(RegionError::CheckpointRollback);
        }
        for (slot, entry) in decoded {
            if let Some(old) = entries.get(&slot)
                && (old.chunk.revision > entry.chunk.revision
                    || (old.chunk.revision == entry.chunk.revision && old != &entry))
            {
                return Err(RegionError::RevisionConflict(entry.chunk.pos));
            }
            entries.insert(slot, entry);
        }
        checkpoint_through = Some(through);
        offset = end;
    }
    Ok((
        entries,
        RegionRecovery {
            valid_bytes: u64::try_from(offset).map_err(|_| RegionError::FileTooLarge)?,
            discarded_tail: offset != bytes.len(),
            checkpoint_through,
            chunk_count: 0,
        },
    ))
    .map(|(entries, mut recovery)| {
        recovery.chunk_count = entries.len();
        (entries, recovery)
    })
}

fn encode_header(region: RegionPos) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN);
    bytes.extend_from_slice(FILE_MAGIC);
    put_u16(&mut bytes, VERSION);
    put_u16(&mut bytes, 0);
    put_i64(&mut bytes, region.x);
    put_i64(&mut bytes, region.y);
    put_i64(&mut bytes, region.z);
    let crc = checksum(&bytes);
    put_u32(&mut bytes, crc);
    bytes
}

fn decode_header(bytes: &[u8], expected_region: RegionPos) -> Result<(), RegionError> {
    if bytes.get(..4) != Some(FILE_MAGIC.as_slice()) {
        return Err(RegionError::InvalidHeader);
    }
    if u16::from_le_bytes(
        bytes[4..6]
            .try_into()
            .map_err(|_| RegionError::InvalidHeader)?,
    ) != VERSION
    {
        return Err(RegionError::UnsupportedVersion);
    }
    let region = RegionPos {
        x: i64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| RegionError::InvalidHeader)?,
        ),
        y: i64::from_le_bytes(
            bytes[16..24]
                .try_into()
                .map_err(|_| RegionError::InvalidHeader)?,
        ),
        z: i64::from_le_bytes(
            bytes[24..32]
                .try_into()
                .map_err(|_| RegionError::InvalidHeader)?,
        ),
    };
    let expected_crc = u32::from_le_bytes(
        bytes[32..36]
            .try_into()
            .map_err(|_| RegionError::InvalidHeader)?,
    );
    if checksum(&bytes[..32]) != expected_crc {
        return Err(RegionError::ChecksumMismatch);
    }
    if region != expected_region {
        return Err(RegionError::WrongRegion {
            expected: expected_region,
            actual: region,
        });
    }
    Ok(())
}

fn encode_frame(
    entries: &[(u16, StoredChunk, u64)],
    through: WalCursor,
    registry: &BlockRegistry,
) -> Result<Vec<u8>, RegionError> {
    let mut payload = Vec::new();
    put_u16(&mut payload, VERSION);
    put_u64(&mut payload, through.offset);
    put_u16(
        &mut payload,
        u16::try_from(entries.len()).map_err(|_| RegionError::InvalidChunkCount(entries.len()))?,
    );
    for (slot, chunk, last_commit) in entries {
        put_u16(&mut payload, *slot);
        put_u64(&mut payload, *last_commit);
        let chunk = encode_chunk(chunk, registry)?;
        put_u32(
            &mut payload,
            u32::try_from(chunk.len()).map_err(|_| RegionError::FrameTooLarge)?,
        );
        payload.extend_from_slice(&chunk);
    }
    if payload.len() > MAX_FRAME {
        return Err(RegionError::FrameTooLarge);
    }
    let mut frame = Vec::with_capacity(payload.len() + 12);
    frame.extend_from_slice(FRAME_MAGIC);
    put_u32(
        &mut frame,
        u32::try_from(payload.len()).map_err(|_| RegionError::FrameTooLarge)?,
    );
    frame.extend_from_slice(&payload);
    let crc = checksum(&frame);
    put_u32(&mut frame, crc);
    Ok(frame)
}

fn decode_frame(
    bytes: &[u8],
    region: RegionPos,
    registry: &BlockRegistry,
) -> Result<(WalCursor, Vec<(u16, RegionEntry)>), RegionError> {
    let mut reader = Reader::new(bytes);
    if reader.u16()? != VERSION {
        return Err(RegionError::UnsupportedVersion);
    }
    let through = WalCursor {
        offset: reader.u64()?,
    };
    let count = usize::from(reader.u16()?);
    if count == 0 || count > REGION_SLOTS {
        return Err(RegionError::InvalidChunkCount(count));
    }
    let mut slots = BTreeSet::new();
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let slot = reader.u16()?;
        if usize::from(slot) >= REGION_SLOTS {
            return Err(RegionError::InvalidSlot(slot));
        }
        if !slots.insert(slot) {
            return Err(RegionError::DuplicateSlot(slot));
        }
        let last_commit = reader.u64()?;
        let length = usize::try_from(reader.u32()?).map_err(|_| RegionError::FrameTooLarge)?;
        if length > MAX_FRAME {
            return Err(RegionError::FrameTooLarge);
        }
        let chunk = decode_chunk(reader.take(length)?, registry)?;
        let (actual_region, actual_slot) = split_region(chunk.pos);
        if actual_region != region {
            return Err(RegionError::WrongRegion {
                expected: region,
                actual: actual_region,
            });
        }
        if actual_slot != slot {
            return Err(RegionError::SlotMismatch {
                encoded: slot,
                actual: actual_slot,
            });
        }
        entries.push((slot, RegionEntry { chunk, last_commit }));
    }
    if !reader.done() {
        return Err(RegionError::TrailingBytes);
    }
    Ok((through, entries))
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
fn put_i64(output: &mut Vec<u8>, value: i64) {
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
    fn take(&mut self, length: usize) -> Result<&'a [u8], RegionError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(RegionError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(RegionError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, RegionError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| RegionError::Truncated)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, RegionError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| RegionError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, RegionError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| RegionError::Truncated)?,
        ))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[derive(Debug)]
pub enum RegionError {
    Io(std::io::Error),
    Codec(CodecError),
    InvalidHeader,
    UnsupportedVersion,
    ChecksumMismatch,
    WrongRegion {
        expected: RegionPos,
        actual: RegionPos,
    },
    InvalidChunkCount(usize),
    InvalidSlot(u16),
    DuplicateSlot(u16),
    SlotMismatch {
        encoded: u16,
        actual: u16,
    },
    RevisionConflict(ChunkPos),
    CheckpointRollback,
    FrameTooLarge,
    FileTooLarge,
    Truncated,
    TrailingBytes,
}
impl From<std::io::Error> for RegionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<CodecError> for RegionError {
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}
impl fmt::Display for RegionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "region error: {self:?}")
    }
}
impl std::error::Error for RegionError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path() -> std::path::PathBuf {
        static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "voxy-region-{}-{nonce}-{sequence}.vxr",
            std::process::id()
        ))
    }

    #[test]
    fn euclidean_region_mapping_is_stable_across_negative_edges() {
        assert_eq!(
            split_region(ChunkPos { x: -1, y: -8, z: 8 }),
            (RegionPos { x: -1, y: -1, z: 1 }, 7)
        );
        assert_eq!(
            split_region(ChunkPos {
                x: -8,
                y: -9,
                z: -9
            }),
            (
                RegionPos {
                    x: -1,
                    y: -2,
                    z: -2
                },
                504
            )
        );
        assert_eq!(
            split_region(ChunkPos { x: 7, y: 7, z: 7 }),
            (RegionPos { x: 0, y: 0, z: 0 }, 511)
        );
    }

    #[test]
    fn checkpoint_round_trip_preserves_latest_chunk_and_cursor() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (region, _) = split_region(chunk.pos);
        let (mut store, initial) = RegionStore::open(&path, region, &registry).unwrap();
        assert_eq!(initial.valid_bytes, u64::try_from(HEADER_LEN).unwrap());
        let checkpoint = store
            .checkpoint(&[(chunk.clone(), 41)], WalCursor { offset: 900 }, &registry)
            .unwrap();
        assert_eq!(
            checkpoint.revisions.as_ref(),
            &[(chunk.pos, chunk.revision)]
        );
        store.sync_all().unwrap();
        drop(store);

        let (reopened, recovery) = RegionStore::open(&path, region, &registry).unwrap();
        assert_eq!(recovery.chunk_count, 1);
        assert_eq!(recovery.checkpoint_through, Some(WalCursor { offset: 900 }));
        assert!(!recovery.discarded_tail);
        assert_eq!(reopened.load(chunk.pos), Some(chunk));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn incomplete_checkpoint_frame_is_discarded_atomically() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (region, _) = split_region(chunk.pos);
        let (mut store, _) = RegionStore::open(&path, region, &registry).unwrap();
        store
            .checkpoint(&[(chunk.clone(), 2)], WalCursor { offset: 123 }, &registry)
            .unwrap();
        let valid = store.cursor;
        drop(store);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"RCP1\x20\x00").unwrap();
        file.sync_data().unwrap();
        drop(file);

        let (reopened, recovery) = RegionStore::open(&path, region, &registry).unwrap();
        assert!(recovery.discarded_tail);
        assert_eq!(recovery.valid_bytes, valid);
        assert_eq!(fs::metadata(&path).unwrap().len(), valid);
        assert_eq!(reopened.load(chunk.pos), Some(chunk));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn header_prevents_opening_file_as_the_wrong_region() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (region, _) = split_region(chunk.pos);
        let (store, _) = RegionStore::open(&path, region, &registry).unwrap();
        drop(store);
        let wrong = RegionPos {
            x: region.x + 1,
            ..region
        };
        assert!(matches!(
            RegionStore::open(&path, wrong, &registry),
            Err(RegionError::WrongRegion { .. })
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn equal_revision_cannot_be_rebound_to_another_commit() {
        let path = temp_path();
        let (registry, chunk) = crate::codec::tests::sample_chunk();
        let (region, _) = split_region(chunk.pos);
        let (mut store, _) = RegionStore::open(&path, region, &registry).unwrap();
        store
            .checkpoint(&[(chunk.clone(), 10)], WalCursor { offset: 100 }, &registry)
            .unwrap();
        assert!(matches!(
            store.checkpoint(
                &[(chunk.clone(), 11)],
                WalCursor { offset: 101 },
                &registry,
            ),
            Err(RegionError::RevisionConflict(pos)) if pos == chunk.pos
        ));
        drop(store);
        let (reopened, _) = RegionStore::open(&path, region, &registry).unwrap();
        assert_eq!(reopened.load(chunk.pos), Some(chunk));
        fs::remove_file(path).unwrap();
    }
}
