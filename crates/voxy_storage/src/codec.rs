use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use voxy_core::{ByteReader, ByteWriter, CHUNK_VOLUME, ChunkPos, LocalIndex};
use voxy_world::{
    BlockRegistry, BlockStateId, ChunkData, ChunkRevision, PalettedBlocks, ResourceKey,
};

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

#[derive(Default)]
struct EncodeScratch {
    src_palette: Vec<BlockStateId>,
    slots: Vec<u16>,
    remap: Vec<u16>,
    indexes: Vec<u16>,
}

#[derive(Default)]
struct DecodeScratch {
    slots: Vec<u16>,
    remapped_slots: Vec<u16>,
}

thread_local! {
    static ENCODE_SCRATCH: RefCell<EncodeScratch> = RefCell::new(EncodeScratch::default());
    static DECODE_SCRATCH: RefCell<DecodeScratch> = RefCell::new(DecodeScratch::default());
}

/// Encodes canonical chunk data using stable resource keys instead of session-local block IDs.
///
/// # Errors
///
/// Rejects unknown IDs and data outside bounded format limits.
pub fn encode_chunk(chunk: &StoredChunk, registry: &BlockRegistry) -> Result<Vec<u8>, CodecError> {
    ENCODE_SCRATCH.with_borrow_mut(|scratch| encode_chunk_with_scratch(chunk, registry, scratch))
}

fn encode_chunk_with_scratch(
    chunk: &StoredChunk,
    registry: &BlockRegistry,
    scratch: &mut EncodeScratch,
) -> Result<Vec<u8>, CodecError> {
    validate_metadata(&chunk.data.block_data)?;

    // Fast path for uniform chunks (single block across all 32,768 cells):
    if let PalettedBlocks::Uniform(block) = &chunk.data.blocks {
        let key = &registry
            .get(*block)
            .ok_or(CodecError::UnknownBlockId(block.get()))?
            .key;
        let key_bytes = key.as_str().as_bytes();
        if key_bytes.is_empty() || key_bytes.len() > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(key_bytes.len()));
        }
        let mut out = Vec::with_capacity(64 + key_bytes.len());
        let mut writer = ByteWriter::new(&mut out);
        writer.write_bytes(MAGIC);
        writer.write_u16_le(VERSION);
        writer.write_i64_le(chunk.pos.x);
        writer.write_i64_le(chunk.pos.y);
        writer.write_i64_le(chunk.pos.z);
        writer.write_u64_le(chunk.revision.get());
        writer.write_u16_le(1);
        writer.write_bytes(&[0, 0]);
        writer.write_u16_le(
            u16::try_from(key_bytes.len())
                .map_err(|_| CodecError::InvalidKeyLength(key_bytes.len()))?,
        );
        writer.write_bytes(key_bytes);
        writer.write_u32_le(0);
        return finish_chunk_metadata_and_crc(&mut writer, chunk).map(|()| out);
    }

    chunk
        .data
        .blocks
        .palette_indices_into(&mut scratch.src_palette, &mut scratch.slots);
    let mut src_keys: Vec<&str> = Vec::with_capacity(scratch.src_palette.len());
    for &block in &scratch.src_palette {
        let key = registry
            .get(block)
            .ok_or(CodecError::UnknownBlockId(block.get()))?
            .key
            .as_str();
        src_keys.push(key);
    }
    let mut palette: Vec<&str> = src_keys.clone();
    palette.sort_unstable();
    palette.dedup();
    if palette.is_empty() || palette.len() > usize::from(u16::MAX) + 1 {
        return Err(CodecError::PaletteTooLarge);
    }
    scratch.remap.clear();
    scratch.remap.reserve(src_keys.len());
    for &key in &src_keys {
        let idx = palette
            .binary_search(&key)
            .expect("palette contains every source key");
        scratch.remap.push(u16::try_from(idx).unwrap_or(u16::MAX));
    }
    let bits = bits_for(palette.len())?;
    scratch.indexes.clear();
    scratch.indexes.reserve(scratch.slots.len());
    for &slot in &scratch.slots {
        scratch.indexes.push(scratch.remap[usize::from(slot)]);
    }

    let packed_len = (scratch.indexes.len() * usize::from(bits)).div_ceil(8);
    let mut out = Vec::with_capacity(64 + palette.len() * 16 + packed_len);
    let mut writer = ByteWriter::new(&mut out);
    writer.write_bytes(MAGIC);
    writer.write_u16_le(VERSION);
    writer.write_i64_le(chunk.pos.x);
    writer.write_i64_le(chunk.pos.y);
    writer.write_i64_le(chunk.pos.z);
    writer.write_u64_le(chunk.revision.get());
    writer.write_u16_le(u16::try_from(palette.len()).map_err(|_| CodecError::PaletteTooLarge)?);
    writer.write_bytes(&[bits, 0]);
    for &key in &palette {
        let bytes = key.as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_KEY {
            return Err(CodecError::InvalidKeyLength(bytes.len()));
        }
        writer.write_u16_le(
            u16::try_from(bytes.len()).map_err(|_| CodecError::InvalidKeyLength(bytes.len()))?,
        );
        writer.write_bytes(bytes);
    }
    writer.write_u32_le(u32::try_from(packed_len).map_err(|_| CodecError::RecordTooLarge)?);
    let packed_start = out.len();
    out.resize(packed_start + packed_len, 0);
    pack_into(&scratch.indexes, bits, &mut out[packed_start..]);
    let mut writer = ByteWriter::new(&mut out);
    finish_chunk_metadata_and_crc(&mut writer, chunk)?;
    Ok(out)
}

fn finish_chunk_metadata_and_crc(
    writer: &mut ByteWriter<'_>,
    chunk: &StoredChunk,
) -> Result<(), CodecError> {
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
    Ok(())
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
    let blocks = decode_paletted_blocks(packed, bits, &palette)?;
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
        data: ChunkData { blocks, block_data },
    })
}

fn decode_paletted_blocks(
    packed: &[u8],
    bits: u8,
    palette: &[BlockStateId],
) -> Result<PalettedBlocks, CodecError> {
    if palette.len() == 1 {
        if packed.iter().any(|&b| b != 0) {
            return Err(CodecError::InvalidPaletteIndex);
        }
        return Ok(PalettedBlocks::uniform(palette[0]));
    }
    if palette.len() <= 256 {
        let fast = DECODE_SCRATCH.with_borrow_mut(|scratch| {
            unpack_slots_into(packed, bits, palette.len(), &mut scratch.slots)?;
            // `palette` is sorted by ResourceKey on the wire, whereas `PalettedBlocks`
            // canonicalizes by `BlockStateId` order. Check if already sorted unique:
            if palette.windows(2).all(|pair| pair[0] < pair[1]) {
                return Ok::<Option<PalettedBlocks>, CodecError>(
                    PalettedBlocks::from_canonical_palette_slots(palette.to_vec(), &scratch.slots),
                );
            }
            let mut order: Vec<usize> = (0..palette.len()).collect();
            order.sort_unstable_by_key(|&i| palette[i]);
            let sorted_palette: Vec<BlockStateId> = order.iter().map(|&i| palette[i]).collect();
            if sorted_palette.windows(2).all(|pair| pair[0] < pair[1]) {
                let mut inv = [0_u16; 256];
                for (new_slot, &old_slot) in order.iter().enumerate() {
                    inv[old_slot] = new_slot as u16;
                }
                scratch.remapped_slots.clear();
                scratch.remapped_slots.reserve(CHUNK_VOLUME);
                for &old_slot in &scratch.slots {
                    scratch.remapped_slots.push(inv[usize::from(old_slot)]);
                }
                return Ok(PalettedBlocks::from_canonical_palette_slots(
                    sorted_palette,
                    &scratch.remapped_slots,
                ));
            }
            Ok(None)
        })?;
        if let Some(blocks) = fast {
            return Ok(blocks);
        }
    }
    let mut dense = Vec::with_capacity(CHUNK_VOLUME);
    unpack_palette_into_dense(packed, bits, palette, &mut dense)?;
    PalettedBlocks::from_dense(dense).map_err(|_| CodecError::InvalidPackedLength)
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
    if count == 1 {
        return Ok(0);
    }
    Ok(u8::try_from(usize::BITS - (count - 1).leading_zeros()).unwrap_or(16))
}

/// Word-packed (`u64` little-endian bit accumulator) encoder producing 100% byte-identical
/// `"VXCH"` v1 streams without per-index byte-slicing loops.
fn pack_into(indexes: &[u16], bits: u8, out: &mut [u8]) {
    let bit_width = u32::from(bits);
    let mask = if bits >= 16 {
        u64::from(u16::MAX)
    } else {
        (1_u64 << bit_width) - 1
    };
    let mut acc = 0_u64;
    let mut acc_bits = 0_u32;
    let mut out_offset = 0_usize;
    for &value in indexes {
        let v = u64::from(value) & mask;
        acc |= v << acc_bits;
        acc_bits += bit_width;
        if acc_bits >= 64 {
            out[out_offset..out_offset + 8].copy_from_slice(&acc.to_le_bytes());
            out_offset += 8;
            acc_bits -= 64;
            acc = if acc_bits == 0 {
                0
            } else {
                v >> (bit_width - acc_bits)
            };
        }
    }
    while acc_bits > 0 {
        out[out_offset] = acc as u8;
        out_offset += 1;
        acc >>= 8;
        acc_bits = acc_bits.saturating_sub(8);
    }
}

/// Bulk `u64` word-level bit-stream unpacker resolving `CHUNK_VOLUME` palette slots into `slots`.
fn unpack_slots_into(
    bytes: &[u8],
    bits: u8,
    palette_len: usize,
    slots: &mut Vec<u16>,
) -> Result<(), CodecError> {
    slots.clear();
    slots.reserve(CHUNK_VOLUME);
    let bit_width = u32::from(bits);
    let mask = if bits >= 16 {
        u64::from(u16::MAX)
    } else {
        (1_u64 << bit_width) - 1
    };
    let mut byte_cursor = 0_usize;
    let mut acc = 0_u64;
    let mut acc_bits = 0_u32;
    for _ in 0..CHUNK_VOLUME {
        if acc_bits < bit_width {
            let rem = bytes.len() - byte_cursor;
            let next_word = if rem >= 8 {
                let word = u64::from_le_bytes(
                    bytes[byte_cursor..byte_cursor + 8]
                        .try_into()
                        .expect("8 bytes checked"),
                );
                byte_cursor += 8;
                word
            } else {
                let mut tail = [0_u8; 8];
                tail[..rem].copy_from_slice(&bytes[byte_cursor..]);
                byte_cursor = bytes.len();
                u64::from_le_bytes(tail)
            };
            if acc_bits == 0 {
                acc = next_word;
                acc_bits = 64;
            } else {
                let raw = (acc | (next_word << acc_bits)) & mask;
                let consumed_from_next = bit_width - acc_bits;
                acc = next_word >> consumed_from_next;
                acc_bits = 64 - consumed_from_next;
                let slot = raw as usize;
                if slot >= palette_len {
                    return Err(CodecError::InvalidPaletteIndex);
                }
                slots.push(raw as u16);
                continue;
            }
        }
        let raw = acc & mask;
        acc >>= bit_width;
        acc_bits -= bit_width;
        if (raw as usize) >= palette_len {
            return Err(CodecError::InvalidPaletteIndex);
        }
        slots.push(raw as u16);
    }
    Ok(())
}

/// Bulk `u64` word-level bit-stream unpacker resolving `CHUNK_VOLUME` palette slots into `dense`.
fn unpack_palette_into_dense(
    bytes: &[u8],
    bits: u8,
    palette: &[BlockStateId],
    dense: &mut Vec<BlockStateId>,
) -> Result<(), CodecError> {
    let bit_width = u32::from(bits);
    let mask = if bits >= 16 {
        u64::from(u16::MAX)
    } else {
        (1_u64 << bit_width) - 1
    };
    let palette_len = palette.len();
    let mut byte_cursor = 0_usize;
    let mut acc = 0_u64;
    let mut acc_bits = 0_u32;
    for _ in 0..CHUNK_VOLUME {
        if acc_bits < bit_width {
            let rem = bytes.len() - byte_cursor;
            let next_word = if rem >= 8 {
                let word = u64::from_le_bytes(
                    bytes[byte_cursor..byte_cursor + 8]
                        .try_into()
                        .expect("8 bytes checked"),
                );
                byte_cursor += 8;
                word
            } else {
                let mut tail = [0_u8; 8];
                tail[..rem].copy_from_slice(&bytes[byte_cursor..]);
                byte_cursor = bytes.len();
                u64::from_le_bytes(tail)
            };
            if acc_bits == 0 {
                acc = next_word;
                acc_bits = 64;
            } else {
                let raw = (acc | (next_word << acc_bits)) & mask;
                let consumed_from_next = bit_width - acc_bits;
                acc = next_word >> consumed_from_next;
                acc_bits = 64 - consumed_from_next;
                let slot = raw as usize;
                if slot >= palette_len {
                    return Err(CodecError::InvalidPaletteIndex);
                }
                dense.push(palette[slot]);
                continue;
            }
        }
        let slot = (acc & mask) as usize;
        acc >>= bit_width;
        acc_bits -= bit_width;
        if slot >= palette_len {
            return Err(CodecError::InvalidPaletteIndex);
        }
        dense.push(palette[slot]);
    }
    Ok(())
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

        let air = registry
            .find(&ResourceKey::parse("voxy:air").unwrap())
            .unwrap();
        let uniform_chunk = StoredChunk {
            pos: ChunkPos { x: 0, y: 1, z: -2 },
            revision: ChunkRevision::from_raw(7),
            data: ChunkData {
                blocks: PalettedBlocks::uniform(air),
                block_data: BTreeMap::new(),
            },
        };
        let uniform_bytes = encode_chunk(&uniform_chunk, &registry).unwrap();
        // Uniform single-palette chunks write 0 packed index bytes (~60 bytes total, not 4 KiB!)
        assert!(
            uniform_bytes.len() < 80,
            "uniform len = {}",
            uniform_bytes.len()
        );
        assert_eq!(
            decode_chunk(&uniform_bytes, &registry).unwrap(),
            uniform_chunk
        );
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

    #[test]
    fn word_packed_codec_matches_reference_byte_layout_across_all_bit_widths() {
        fn ref_pack(indexes: &[u16], bits: u8) -> Vec<u8> {
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

        let mut defs = Vec::with_capacity(256);
        defs.push(definition("air", RenderKind::Invisible, Occlusion::None, 0));
        for idx in 1..256 {
            defs.push(definition(
                &format!("b{idx:03}"),
                RenderKind::Opaque,
                Occlusion::FullCube,
                10,
            ));
        }
        let big_registry = BlockRegistry::new(defs).unwrap();
        let reg_ids: Vec<BlockStateId> = (0..256)
            .map(|idx| {
                let name = if idx == 0 {
                    "voxy:air".to_owned()
                } else {
                    format!("voxy:b{idx:03}")
                };
                big_registry
                    .find(&ResourceKey::parse(name).unwrap())
                    .unwrap()
            })
            .collect();

        for bits in 1_u8..=16 {
            let modulus = if bits == 16 {
                usize::from(u16::MAX) + 1
            } else {
                1_usize << bits
            };
            let indexes: Vec<u16> = (0..CHUNK_VOLUME)
                .map(|i| ((i * 7919 + 17) % modulus) as u16)
                .collect();
            let expected = ref_pack(&indexes, bits);
            let mut actual = vec![0_u8; expected.len()];
            pack_into(&indexes, bits, &mut actual);
            assert_eq!(actual, expected, "pack_into mismatch for bits={bits}");

            if modulus <= reg_ids.len() {
                let palette = &reg_ids[..modulus];
                let mut dense = Vec::with_capacity(CHUNK_VOLUME);
                unpack_palette_into_dense(&actual, bits, palette, &mut dense).unwrap();
                assert_eq!(dense.len(), CHUNK_VOLUME);
                for (cell, &state) in dense.iter().enumerate() {
                    assert_eq!(
                        state,
                        palette[usize::from(indexes[cell])],
                        "unpack mismatch at cell={cell} bits={bits}"
                    );
                }
            } else {
                // For >256 palette slots (e.g. bits 9..=16), test with a 65536-entry slice mapped cyclically:
                let palette: Vec<BlockStateId> =
                    (0..modulus).map(|i| reg_ids[i % reg_ids.len()]).collect();
                let mut dense = Vec::with_capacity(CHUNK_VOLUME);
                unpack_palette_into_dense(&actual, bits, &palette, &mut dense).unwrap();
                assert_eq!(dense.len(), CHUNK_VOLUME);
                for (cell, &state) in dense.iter().enumerate() {
                    assert_eq!(
                        state,
                        palette[usize::from(indexes[cell])],
                        "unpack mismatch at cell={cell} bits={bits}"
                    );
                }
            }
        }
    }
}
