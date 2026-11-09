use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use voxy_core::{ByteReader, ByteWriter, CHUNK_VOLUME, ChunkPos, LocalIndex};
use voxy_world::{BlockRegistry, ChunkData, ChunkRevision, PalettedBlocks, ResourceKey};

const MAGIC: &[u8; 4] = b"VXCH";
const VERSION: u16 = 1;
const MAX_RECORD: usize = 8 * 1024 * 1024;
const MAX_KEY: usize = 256;
const MAX_META_ENTRIES: usize = 4096;
const MAX_META_VALUE: usize = 64 * 1024;
const MAX_META_TOTAL: usize = 4 * 1024 * 1024;

const CRC32_IEEE_TABLE: [u32; 256] = {
    let mut table = [0_u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

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
pub fn encode_chunk(chunk: &StoredChunk, registry: &BlockRegistry) -> Result<Vec<u8>, CodecError> {
    let (src_palette, slots) = chunk.data.blocks.palette_indices();
    let mut src_keys = Vec::with_capacity(src_palette.len());
    let mut unique_keys = BTreeSet::new();
    for &block in &src_palette {
        let key = &registry
            .get(block)
            .ok_or(CodecError::UnknownBlockId(block.get()))?
            .key;
        src_keys.push(key);
        unique_keys.insert(key.clone());
    }
    let palette: Vec<_> = unique_keys.into_iter().collect();
    if palette.is_empty() || palette.len() > usize::from(u16::MAX) + 1 {
        return Err(CodecError::PaletteTooLarge);
    }
    let lookup: BTreeMap<_, _> = palette
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), u16::try_from(index).unwrap_or(u16::MAX)))
        .collect();
    let remap: Vec<u16> = src_keys
        .into_iter()
        .map(|key| lookup[key.as_str()])
        .collect();
    let bits = bits_for(palette.len())?;
    let indexes: Vec<u16> = slots
        .into_iter()
        .map(|slot| remap[usize::from(slot)])
        .collect();
    validate_metadata(&chunk.data.block_data)?;

    let packed = pack(&indexes, bits);
    let mut out = Vec::with_capacity(64 + palette.len() * 16 + packed.len());
    let mut writer = ByteWriter::new(&mut out);
    writer.write_bytes(MAGIC);
    writer.write_u16_le(VERSION);
    writer.write_i64_le(chunk.pos.x);
    writer.write_i64_le(chunk.pos.y);
    writer.write_i64_le(chunk.pos.z);
    writer.write_u64_le(chunk.revision.get());
    writer.write_u16_le(u16::try_from(palette.len()).map_err(|_| CodecError::PaletteTooLarge)?);
    writer.write_bytes(&[bits, 0]);
    for key in palette {
        let bytes = key.as_str().as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(bytes.len()));
        }
        writer.write_u16_le(
            u16::try_from(bytes.len()).map_err(|_| CodecError::InvalidKeyLength(bytes.len()))?,
        );
        writer.write_bytes(bytes);
    }
    writer.write_u32_le(u32::try_from(packed.len()).map_err(|_| CodecError::RecordTooLarge)?);
    writer.write_bytes(&packed);
    writer.write_u16_le(
        u16::try_from(chunk.data.block_data.len())
            .map_err(|_| CodecError::TooManyMetadataEntries)?,
    );
    for (&index, value) in &chunk.data.block_data {
        writer.write_u16_le(index.get());
        writer.write_u32_le(
            u32::try_from(value.len()).map_err(|_| CodecError::MetadataValueTooLarge)?,
        );
        writer.write_bytes(value);
    }
    if writer
        .len()
        .checked_add(4)
        .is_none_or(|length| length > MAX_RECORD)
    {
        return Err(CodecError::RecordTooLarge);
    }
    let crc = checksum(writer.as_slice());
    writer.write_u32_le(crc);
    Ok(out)
}

/// Decodes a checksummed chunk after validating every allocation-driving length.
///
/// # Errors
///
/// Rejects corruption, truncation, unknown content and non-canonical encoding.
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
    let mut reader = ByteReader::new(&bytes[..payload_len]);
    if reader.read_bytes(4).map_err(|_| CodecError::Truncated)? != MAGIC {
        return Err(CodecError::BadMagic);
    }
    if reader.read_u16_le().map_err(|_| CodecError::Truncated)? != VERSION {
        return Err(CodecError::UnsupportedVersion);
    }
    let pos = ChunkPos {
        x: reader.read_i64_le().map_err(|_| CodecError::Truncated)?,
        y: reader.read_i64_le().map_err(|_| CodecError::Truncated)?,
        z: reader.read_i64_le().map_err(|_| CodecError::Truncated)?,
    };
    let revision =
        ChunkRevision::from_raw(reader.read_u64_le().map_err(|_| CodecError::Truncated)?);
    let palette_len = usize::from(reader.read_u16_le().map_err(|_| CodecError::Truncated)?);
    if palette_len == 0 {
        return Err(CodecError::EmptyPalette);
    }
    let bits = reader.read_u8().map_err(|_| CodecError::Truncated)?;
    if reader.read_u8().map_err(|_| CodecError::Truncated)? != 0 || bits != bits_for(palette_len)? {
        return Err(CodecError::NonCanonicalEncoding);
    }
    let mut palette = Vec::with_capacity(palette_len);
    let mut previous: Option<ResourceKey> = None;
    for _ in 0..palette_len {
        let length = usize::from(reader.read_u16_le().map_err(|_| CodecError::Truncated)?);
        if length == 0 || length > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(length));
        }
        let raw = reader
            .read_bytes(length)
            .map_err(|_| CodecError::Truncated)?;
        let text = std::str::from_utf8(raw).map_err(|_| CodecError::InvalidKey)?;
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
    let packed_len = usize::try_from(reader.read_u32_le().map_err(|_| CodecError::Truncated)?)
        .map_err(|_| CodecError::RecordTooLarge)?;
    let canonical_len = (CHUNK_VOLUME * usize::from(bits)).div_ceil(8);
    if packed_len != canonical_len {
        return Err(CodecError::InvalidPackedLength);
    }
    let packed = reader
        .read_bytes(packed_len)
        .map_err(|_| CodecError::Truncated)?;
    let mut dense = Vec::with_capacity(CHUNK_VOLUME);
    for index in 0..CHUNK_VOLUME {
        dense.push(
            *palette
                .get(usize::from(unpack(packed, index, bits)))
                .ok_or(CodecError::InvalidPaletteIndex)?,
        );
    }
    let count = usize::from(reader.read_u16_le().map_err(|_| CodecError::Truncated)?);
    if count > MAX_META_ENTRIES {
        return Err(CodecError::TooManyMetadataEntries);
    }
    let mut total = 0_usize;
    let mut block_data = BTreeMap::new();
    for _ in 0..count {
        let index = LocalIndex::new(reader.read_u16_le().map_err(|_| CodecError::Truncated)?)
            .ok_or(CodecError::InvalidMetadataIndex)?;
        let length = usize::try_from(reader.read_u32_le().map_err(|_| CodecError::Truncated)?)
            .map_err(|_| CodecError::MetadataValueTooLarge)?;
        if length > MAX_META_VALUE {
            return Err(CodecError::MetadataValueTooLarge);
        }
        total = total
            .checked_add(length)
            .ok_or(CodecError::MetadataTooLarge)?;
        if total > MAX_META_TOTAL {
            return Err(CodecError::MetadataTooLarge);
        }
        let meta_bytes = reader
            .read_bytes(length)
            .map_err(|_| CodecError::Truncated)?;
        if block_data.insert(index, Arc::from(meta_bytes)).is_some() {
            return Err(CodecError::DuplicateMetadataIndex);
        }
    }
    if !reader.is_empty() {
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
    let bit_width = usize::from(bits);
    let mut out = vec![0_u8; (indexes.len() * bit_width).div_ceil(8)];
    let mask = if bits >= 16 {
        u32::from(u16::MAX)
    } else {
        (1_u32 << bits) - 1
    };
    let mut bit_pos = 0_usize;
    for &value in indexes {
        let mut val = (u32::from(value) & mask) << (bit_pos & 7);
        let mut byte_idx = bit_pos >> 3;
        let mut remaining = (bit_pos & 7) + bit_width;
        while remaining > 0 {
            out[byte_idx] |= (val & 0xff) as u8;
            val >>= 8;
            byte_idx += 1;
            remaining = remaining.saturating_sub(8);
        }
        bit_pos += bit_width;
    }
    out
}

#[inline]
fn unpack(bytes: &[u8], index: usize, bits: u8) -> u16 {
    let bit_width = usize::from(bits);
    let bit_pos = index * bit_width;
    let byte_idx = bit_pos >> 3;
    let bit_offset = bit_pos & 7;
    let b0 = u32::from(bytes[byte_idx]);
    let b1 = bytes.get(byte_idx + 1).copied().map_or(0, u32::from);
    let b2 = bytes.get(byte_idx + 2).copied().map_or(0, u32::from);
    let combined = b0 | (b1 << 8) | (b2 << 16);
    let mask = if bits >= 16 {
        u32::from(u16::MAX)
    } else {
        (1_u32 << bits) - 1
    };
    ((combined >> bit_offset) & mask) as u16
}

#[inline]
pub(crate) fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        let table_idx = ((crc as u8) ^ byte) as usize;
        crc = (crc >> 8) ^ CRC32_IEEE_TABLE[table_idx];
    }
    !crc
}

/// Delta-encodes a slice of `i32` values (using wrapping differences) and maps each signed delta
/// to an unsigned `u32` via ZigZag encoding `(d << 1) ^ (d >> 31)` so small deltas near zero
/// become small positive integers ideal for variable-length / bit-packed storage.
#[must_use]
pub fn zigzag_delta_encode_i32(values: &[i32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(values.len());
    let mut prev = 0_i32;
    for &v in values {
        let delta = v.wrapping_sub(prev);
        prev = v;
        let zz = ((delta << 1) ^ (delta >> 31)) as u32;
        out.push(zz);
    }
    out
}

/// Decodes a ZigZag delta-encoded `u32` stream back into the original `i32` sequence.
#[must_use]
pub fn zigzag_delta_decode_i32(encoded: &[u32]) -> Vec<i32> {
    let mut out = Vec::with_capacity(encoded.len());
    let mut acc = 0_i32;
    for &zz in encoded {
        let delta = ((zz >> 1) as i32) ^ -((zz & 1) as i32);
        acc = acc.wrapping_add(delta);
        out.push(acc);
    }
    out
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("chunk codec error: {self:?}")]
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
        payload.extend_from_slice(&crc.to_le_bytes());
        assert_eq!(
            decode_chunk(&payload, &registry),
            Err(CodecError::TrailingBytes)
        );
        let original = vec![100_i32, 102, 99, 99, -50, 0, i32::MAX, i32::MIN];
        let encoded = zigzag_delta_encode_i32(&original);
        assert_eq!(zigzag_delta_decode_i32(&encoded), original);
    }
}
