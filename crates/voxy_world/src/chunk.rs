use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, ChunkPos, LocalIndex};

use crate::BlockStateId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PalettedBlocks {
    Uniform(BlockStateId),
    Packed {
        bits_per_index: u8,
        palette: Box<[BlockStateId]>,
        words: Box<[u64]>,
    },
    Direct(Box<[BlockStateId]>),
}

impl PalettedBlocks {
    #[must_use]
    pub fn uniform(block: BlockStateId) -> Self {
        Self::Uniform(block)
    }

    /// Chooses a canonical representation for exactly one chunk of cells.
    ///
    /// # Errors
    ///
    /// Returns [`ChunkError::InvalidCellCount`] unless the input contains `32³` cells.
    pub fn from_dense(dense: Vec<BlockStateId>) -> Result<Self, ChunkError> {
        if dense.len() != CHUNK_VOLUME {
            return Err(ChunkError::InvalidCellCount(dense.len()));
        }
        let unique: BTreeSet<_> = dense.iter().copied().collect();
        if unique.len() == 1 {
            return Ok(Self::Uniform(dense[0]));
        }
        if unique.len() > 256 {
            return Ok(Self::Direct(dense.into_boxed_slice()));
        }

        let palette: Vec<_> = unique.into_iter().collect();
        let lookup: BTreeMap<_, _> = palette
            .iter()
            .copied()
            .enumerate()
            .map(|(index, block)| (block, index as u64))
            .collect();
        let bits =
            u8::try_from((usize::BITS - (palette.len() - 1).leading_zeros()).max(1)).unwrap_or(8);
        let word_count = (CHUNK_VOLUME * usize::from(bits)).div_ceil(64);
        let mut words = vec![0_u64; word_count];
        for (cell, block) in dense.into_iter().enumerate() {
            write_packed(&mut words, cell, bits, lookup[&block]);
        }
        Ok(Self::Packed {
            bits_per_index: bits,
            palette: palette.into_boxed_slice(),
            words: words.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn get(&self, index: LocalIndex) -> BlockStateId {
        match self {
            Self::Uniform(block) => *block,
            Self::Packed {
                bits_per_index,
                palette,
                words,
            } => {
                let palette_index = usize::try_from(read_packed(
                    words,
                    usize::from(index.get()),
                    *bits_per_index,
                ))
                .unwrap_or(0);
                palette[palette_index]
            }
            Self::Direct(blocks) => blocks[usize::from(index.get())],
        }
    }

    #[must_use]
    pub fn to_dense(&self) -> Vec<BlockStateId> {
        match self {
            Self::Uniform(block) => vec![*block; CHUNK_VOLUME],
            Self::Direct(blocks) => blocks.to_vec(),
            Self::Packed {
                bits_per_index,
                palette,
                words,
            } => {
                let bits = *bits_per_index;
                let mask = (1_u64 << bits) - 1;
                let mut dense = Vec::with_capacity(CHUNK_VOLUME);
                for index in 0..CHUNK_VOLUME {
                    let bit_index = index * usize::from(bits);
                    let word = bit_index / 64;
                    let shift = bit_index % 64;
                    let mut value = words[word] >> shift;
                    if shift + usize::from(bits) > 64 {
                        value |= words[word + 1] << (64 - shift);
                    }
                    let palette_index = (value & mask) as usize;
                    dense.push(palette[palette_index]);
                }
                dense
            }
        }
    }

    /// Returns a repacked copy with the supplied cells replaced.
    ///
    /// # Errors
    ///
    /// Propagates an internal cell-count invariant failure.
    pub fn with_updates(&self, updates: &[(LocalIndex, BlockStateId)]) -> Result<Self, ChunkError> {
        let mut dense = self.to_dense();
        for &(index, block) in updates {
            dense[usize::from(index.get())] = block;
        }
        Self::from_dense(dense)
    }

    #[must_use]
    pub fn encoded_bytes(&self) -> usize {
        match self {
            Self::Uniform(_) => size_of::<BlockStateId>(),
            Self::Packed { palette, words, .. } => {
                size_of::<BlockStateId>() * palette.len() + size_of::<u64>() * words.len()
            }
            Self::Direct(blocks) => size_of::<BlockStateId>() * blocks.len(),
        }
    }
}

fn write_packed(words: &mut [u64], index: usize, bits: u8, value: u64) {
    let bit = index * usize::from(bits);
    let word = bit / 64;
    let shift = bit % 64;
    words[word] |= value << shift;
    if shift + usize::from(bits) > 64 {
        words[word + 1] |= value >> (64 - shift);
    }
}

fn read_packed(words: &[u64], index: usize, bits: u8) -> u64 {
    let bit = index * usize::from(bits);
    let word = bit / 64;
    let shift = bit % 64;
    let mask = (1_u64 << bits) - 1;
    let mut value = words[word] >> shift;
    if shift + usize::from(bits) > 64 {
        value |= words[word + 1] << (64 - shift);
    }
    value & mask
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkData {
    pub blocks: PalettedBlocks,
    pub block_data: BTreeMap<LocalIndex, Arc<[u8]>>,
}

impl ChunkData {
    /// Bytes of block storage and attached block-data payloads.
    ///
    /// This is a logical payload charge, excluding allocator, map-node and `Arc`
    /// overhead. Shared payloads are charged per entry, so summing snapshots does
    /// not deduplicate shared allocations. Returns `None` on size overflow.
    #[must_use]
    pub fn payload_bytes(&self) -> Option<usize> {
        self.block_data
            .values()
            .try_fold(self.blocks.encoded_bytes(), |bytes, value| {
                bytes.checked_add(value.len())
            })
    }

    #[must_use]
    pub fn uniform(block: BlockStateId) -> Self {
        Self {
            blocks: PalettedBlocks::uniform(block),
            block_data: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChunkRevision(u64);

impl ChunkRevision {
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChunkSnapshot {
    pub pos: ChunkPos,
    pub revision: ChunkRevision,
    pub data: Arc<ChunkData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChunkError {
    InvalidCellCount(usize),
}

impl fmt::Display for ChunkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid chunk: {self:?}")
    }
}

impl std::error::Error for ChunkError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u32) -> BlockStateId {
        BlockStateId::from_test(value)
    }

    #[test]
    fn representations_match_dense_oracle() {
        for unique in [1, 2, 3, 16, 17, 255, 256, 257] {
            let dense: Vec<_> = (0..CHUNK_VOLUME)
                .map(|index| id(u32::try_from(index % unique).unwrap()))
                .collect();
            let blocks = PalettedBlocks::from_dense(dense.clone()).unwrap();
            assert_eq!(blocks.to_dense(), dense);
            match unique {
                1 => assert!(matches!(blocks, PalettedBlocks::Uniform(_))),
                2..=256 => assert!(matches!(blocks, PalettedBlocks::Packed { .. })),
                _ => assert!(matches!(blocks, PalettedBlocks::Direct(_))),
            }
        }
    }

    #[test]
    fn updates_repack_and_can_collapse_to_uniform() {
        let original = PalettedBlocks::uniform(id(1));
        let changed = original
            .with_updates(&[(LocalIndex::new(31).unwrap(), id(2))])
            .unwrap();
        assert!(matches!(changed, PalettedBlocks::Packed { .. }));
        let restored = changed
            .with_updates(&[(LocalIndex::new(31).unwrap(), id(1))])
            .unwrap();
        assert_eq!(restored, PalettedBlocks::Uniform(id(1)));
    }

    #[test]
    fn payload_charge_includes_metadata_for_each_representation() {
        for unique in [1, 3, 257] {
            let dense = (0..CHUNK_VOLUME)
                .map(|index| id(u32::try_from(index % unique).unwrap()))
                .collect();
            let blocks = PalettedBlocks::from_dense(dense).unwrap();
            let block_bytes = match unique {
                1 => 4,
                3 => 3 * 4 + CHUNK_VOLUME * 2 / 8,
                _ => CHUNK_VOLUME * 4,
            };
            let shared: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4, 5]);
            let chunk = ChunkData {
                blocks,
                block_data: BTreeMap::from([
                    (LocalIndex::new(0).unwrap(), Arc::clone(&shared)),
                    (LocalIndex::new(1).unwrap(), shared),
                    (LocalIndex::new(2).unwrap(), Arc::from([])),
                ]),
            };
            assert_eq!(chunk.payload_bytes(), Some(block_bytes + 10));
        }
    }
}
