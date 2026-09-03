use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, ChunkPos, LocalIndex};
use voxy_world::{BlockRegistry, ChunkData, ChunkRevision, PalettedBlocks, ResourceKey};

const MAGIC: &[u8; 4] = b"VXCH";
const VERSION: u16 = 1;
const MAX_RECORD: usize = 8 * 1024 * 1024;
const MAX_KEY: usize = 256;
const MAX_META_ENTRIES: usize = 4096;
const MAX_META_VALUE: usize = 64 * 1024;
const MAX_META_TOTAL: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredChunk {
    pub pos: ChunkPos,
    pub revision: ChunkRevision,
    pub data: ChunkData,
}

/// Encodes canonical chunk data using stable resource keys instead of session-local block IDs.
///
/// # Errors
///
/// Rejects unknown IDs and data outside bounded format limits.
#[allow(clippy::too_many_lines)]
pub fn encode_chunk(chunk: &StoredChunk, registry: &BlockRegistry) -> Result<Vec<u8>, CodecError> {
    let dense = chunk.data.blocks.to_dense();
    let mut keys = BTreeSet::new();
    for &block in &dense {
        keys.insert(
            registry
                .get(block)
                .ok_or(CodecError::UnknownBlockId(block.get()))?
                .key
                .clone(),
        );
    }
    let palette: Vec<_> = keys.into_iter().collect();
    if palette.is_empty() || palette.len() > usize::from(u16::MAX) + 1 {
        return Err(CodecError::PaletteTooLarge);
    }
    let lookup: BTreeMap<_, _> = palette
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), u16::try_from(index).unwrap_or(u16::MAX)))
        .collect();
    let bits = bits_for(palette.len())?;
    let indexes = dense
        .into_iter()
        .map(|block| {
            let key = &registry
                .get(block)
                .ok_or(CodecError::UnknownBlockId(block.get()))?
                .key;
            Ok(lookup[key.as_str()])
        })
        .collect::<Result<Vec<_>, CodecError>>()?;
    validate_metadata(&chunk.data.block_data)?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    put_u16(&mut out, VERSION);
    put_i64(&mut out, chunk.pos.x);
    put_i64(&mut out, chunk.pos.y);
    put_i64(&mut out, chunk.pos.z);
    put_u64(&mut out, chunk.revision.get());
    put_u16(
        &mut out,
        u16::try_from(palette.len()).map_err(|_| CodecError::PaletteTooLarge)?,
    );
    out.extend_from_slice(&[bits, 0]);
    for key in palette {
        let bytes = key.as_str().as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(bytes.len()));
        }
        put_u16(
            &mut out,
            u16::try_from(bytes.len()).map_err(|_| CodecError::InvalidKeyLength(bytes.len()))?,
        );
        out.extend_from_slice(bytes);
    }
    let packed = pack(&indexes, bits);
    put_u32(
        &mut out,
        u32::try_from(packed.len()).map_err(|_| CodecError::RecordTooLarge)?,
    );
    out.extend_from_slice(&packed);
    put_u16(
        &mut out,
        u16::try_from(chunk.data.block_data.len())
            .map_err(|_| CodecError::TooManyMetadataEntries)?,
    );
    for (&index, value) in &chunk.data.block_data {
        put_u16(&mut out, index.get());
        put_u32(
            &mut out,
            u32::try_from(value.len()).map_err(|_| CodecError::MetadataValueTooLarge)?,
        );
        out.extend_from_slice(value);
    }
    if out
        .len()
        .checked_add(4)
        .is_none_or(|length| length > MAX_RECORD)
    {
        return Err(CodecError::RecordTooLarge);
    }
    let crc = checksum(&out);
    put_u32(&mut out, crc);
    Ok(out)
}

/// Decodes a checksummed chunk after validating every allocation-driving length.
///
/// # Errors
///
/// Rejects corruption, truncation, unknown content and non-canonical encoding.
#[allow(clippy::too_many_lines)]
pub fn decode_chunk(bytes: &[u8], registry: &BlockRegistry) -> Result<StoredChunk, CodecError> {
    if bytes.len() < 4 {
        return Err(CodecError::Truncated);
    }
    if bytes.len() > MAX_RECORD {
        return Err(CodecError::RecordTooLarge);
    }
    let payload_len = bytes.len() - 4;
    let expected = u32::from_le_bytes(
        bytes[payload_len..]
            .try_into()
            .map_err(|_| CodecError::Truncated)?,
    );
    if checksum(&bytes[..payload_len]) != expected {
        return Err(CodecError::ChecksumMismatch);
    }
    let mut reader = Reader::new(&bytes[..payload_len]);
    if reader.take(4)? != MAGIC {
        return Err(CodecError::BadMagic);
    }
    if reader.u16()? != VERSION {
        return Err(CodecError::UnsupportedVersion);
    }
    let pos = ChunkPos {
        x: reader.i64()?,
        y: reader.i64()?,
        z: reader.i64()?,
    };
    let revision = ChunkRevision::from_raw(reader.u64()?);
    let palette_len = usize::from(reader.u16()?);
    if palette_len == 0 {
        return Err(CodecError::EmptyPalette);
    }
    let bits = reader.u8()?;
    if reader.u8()? != 0 || bits != bits_for(palette_len)? {
        return Err(CodecError::NonCanonicalEncoding);
    }
    let mut palette = Vec::with_capacity(palette_len);
    let mut previous: Option<ResourceKey> = None;
    for _ in 0..palette_len {
        let length = usize::from(reader.u16()?);
        if length == 0 || length > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(length));
        }
        let text = std::str::from_utf8(reader.take(length)?).map_err(|_| CodecError::InvalidKey)?;
        let key = ResourceKey::parse(text).map_err(|_| CodecError::InvalidKey)?;
        if previous.as_ref().is_some_and(|value| value >= &key) {
            return Err(CodecError::NonCanonicalEncoding);
        }
        let block = registry
            .find(&key)
            .ok_or_else(|| CodecError::UnknownResourceKey(key.clone()))?;
        previous = Some(key);
        palette.push(block);
    }
    let packed_len = usize::try_from(reader.u32()?).map_err(|_| CodecError::RecordTooLarge)?;
    let canonical_len = (CHUNK_VOLUME * usize::from(bits)).div_ceil(8);
    if packed_len != canonical_len {
        return Err(CodecError::InvalidPackedLength);
    }
    let packed = reader.take(packed_len)?;
    let mut dense = Vec::with_capacity(CHUNK_VOLUME);
    for index in 0..CHUNK_VOLUME {
        dense.push(
            *palette
                .get(usize::from(unpack(packed, index, bits)))
                .ok_or(CodecError::InvalidPaletteIndex)?,
        );
    }
    let count = usize::from(reader.u16()?);
    if count > MAX_META_ENTRIES {
        return Err(CodecError::TooManyMetadataEntries);
    }
    let mut total = 0_usize;
    let mut block_data = BTreeMap::new();
    for _ in 0..count {
        let index = LocalIndex::new(reader.u16()?).ok_or(CodecError::InvalidMetadataIndex)?;
        let length =
            usize::try_from(reader.u32()?).map_err(|_| CodecError::MetadataValueTooLarge)?;
        if length > MAX_META_VALUE {
            return Err(CodecError::MetadataValueTooLarge);
        }
        total = total
            .checked_add(length)
            .ok_or(CodecError::MetadataTooLarge)?;
        if total > MAX_META_TOTAL {
            return Err(CodecError::MetadataTooLarge);
        }
        if block_data
            .insert(index, Arc::from(reader.take(length)?))
            .is_some()
        {
            return Err(CodecError::DuplicateMetadataIndex);
        }
    }
    if !reader.done() {
        return Err(CodecError::TrailingBytes);
    }
    Ok(StoredChunk {
        pos,
        revision,
        data: ChunkData {
            blocks: PalettedBlocks::from_dense(dense)
                .map_err(|_| CodecError::InvalidPackedLength)?,
            block_data,
        },
    })
}

fn validate_metadata(data: &BTreeMap<LocalIndex, Arc<[u8]>>) -> Result<(), CodecError> {
    if data.len() > MAX_META_ENTRIES {
        return Err(CodecError::TooManyMetadataEntries);
    }
    let mut total = 0_usize;
    for value in data.values() {
        if value.len() > MAX_META_VALUE {
            return Err(CodecError::MetadataValueTooLarge);
        }
        total = total
            .checked_add(value.len())
            .ok_or(CodecError::MetadataTooLarge)?;
    }
    if total > MAX_META_TOTAL {
        return Err(CodecError::MetadataTooLarge);
    }
    Ok(())
}

fn bits_for(count: usize) -> Result<u8, CodecError> {
    if count == 0 || count > usize::from(u16::MAX) + 1 {
        return Err(CodecError::PaletteTooLarge);
    }
    Ok(u8::try_from((usize::BITS - (count - 1).leading_zeros()).max(1)).unwrap_or(16))
}

fn pack(indexes: &[u16], bits: u8) -> Vec<u8> {
    let mut out = vec![0_u8; (indexes.len() * usize::from(bits)).div_ceil(8)];
    for (index, &value) in indexes.iter().enumerate() {
        let start = index * usize::from(bits);
        for offset in 0..bits {
            if value & (1_u16 << offset) != 0 {
                let target = start + usize::from(offset);
                out[target / 8] |= 1_u8 << (target % 8);
            }
        }
    }
    out
}

fn unpack(bytes: &[u8], index: usize, bits: u8) -> u16 {
    let start = index * usize::from(bits);
    let mut value = 0_u16;
    for offset in 0..bits {
        let source = start + usize::from(offset);
        if bytes[source / 8] & (1_u8 << (source % 8)) != 0 {
            value |= 1_u16 << offset;
        }
    }
    value
}

pub(crate) fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], CodecError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(CodecError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(CodecError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| CodecError::Truncated)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| CodecError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| CodecError::Truncated)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| CodecError::Truncated)?,
        ))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodecError {
    UnknownBlockId(u32),
    UnknownResourceKey(ResourceKey),
    PaletteTooLarge,
    EmptyPalette,
    InvalidKeyLength(usize),
    InvalidKey,
    TooManyMetadataEntries,
    MetadataValueTooLarge,
    MetadataTooLarge,
    RecordTooLarge,
    BadMagic,
    UnsupportedVersion,
    ChecksumMismatch,
    Truncated,
    NonCanonicalEncoding,
    InvalidPackedLength,
    InvalidPaletteIndex,
    InvalidMetadataIndex,
    DuplicateMetadataIndex,
    TrailingBytes,
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "chunk codec error: {self:?}")
    }
}
impl std::error::Error for CodecError {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use voxy_world::{BlockDef, CollisionShape, MaterialId, Occlusion, RenderKind};

    fn registry() -> BlockRegistry {
        BlockRegistry::new(vec![
            definition("air", RenderKind::Invisible, Occlusion::None, 0),
            definition("stone", RenderKind::Opaque, Occlusion::FullCube, 20),
            definition("dirt", RenderKind::Opaque, Occlusion::FullCube, 8),
        ])
        .unwrap()
    }

    fn definition(
        name: &str,
        render: RenderKind,
        occlusion: Occlusion,
        resistance: u16,
    ) -> BlockDef {
        BlockDef {
            key: ResourceKey::parse(format!("voxy:{name}")).unwrap(),
            render,
            occlusion,
            collision: if occlusion == Occlusion::FullCube {
                CollisionShape::FullCube
            } else {
                CollisionShape::Empty
            },
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: resistance,
        }
    }

    pub(crate) fn sample_chunk() -> (BlockRegistry, StoredChunk) {
        let registry = registry();
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let dirt = registry
            .find(&ResourceKey::parse("voxy:dirt").unwrap())
            .unwrap();
        let blocks = PalettedBlocks::uniform(stone)
            .with_updates(&[(LocalIndex::new(17).unwrap(), dirt)])
            .unwrap();
        let mut block_data = BTreeMap::new();
        block_data.insert(LocalIndex::new(17).unwrap(), Arc::from([1_u8, 2, 3]));
        (
            registry,
            StoredChunk {
                pos: ChunkPos { x: -7, y: 2, z: 11 },
                revision: ChunkRevision::from_raw(42),
                data: ChunkData { blocks, block_data },
            },
        )
    }

    #[test]
    fn chunk_round_trip_preserves_canonical_data() {
        let (registry, chunk) = sample_chunk();
        let bytes = encode_chunk(&chunk, &registry).unwrap();
        assert_eq!(decode_chunk(&bytes, &registry).unwrap(), chunk);
        assert_eq!(encode_chunk(&chunk, &registry).unwrap(), bytes);
    }

    #[test]
    fn checksum_detects_payload_corruption_before_decode() {
        let (registry, chunk) = sample_chunk();
        let mut bytes = encode_chunk(&chunk, &registry).unwrap();
        bytes[24] ^= 0x40;
        assert_eq!(
            decode_chunk(&bytes, &registry),
            Err(CodecError::ChecksumMismatch)
        );
    }

    #[test]
    fn truncation_and_trailing_data_are_rejected() {
        let (registry, chunk) = sample_chunk();
        let bytes = encode_chunk(&chunk, &registry).unwrap();
        assert_eq!(
            decode_chunk(&bytes[..3], &registry),
            Err(CodecError::Truncated)
        );
        let mut payload = bytes[..bytes.len() - 4].to_vec();
        payload.push(0);
        let crc = checksum(&payload);
        put_u32(&mut payload, crc);
        assert_eq!(
            decode_chunk(&payload, &registry),
            Err(CodecError::TrailingBytes)
        );
    }
}
