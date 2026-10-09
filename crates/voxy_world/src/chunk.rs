use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, ChunkPos, LocalIndex};

use crate::BlockStateId;

/// Largest palette that stays packed; more distinct blocks switch to direct storage.
const MAX_PALETTE: usize = 256;

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
    /// The packed palette is the sorted set of blocks actually present, so equal cell
    /// contents always produce equal representations.
    ///
    /// # Errors
    ///
    /// Returns [`ChunkError::InvalidCellCount`] unless the input contains `32³` cells.
    pub fn from_dense(dense: Vec<BlockStateId>) -> Result<Self, ChunkError> {
        if dense.len() != CHUNK_VOLUME {
            return Err(ChunkError::InvalidCellCount(dense.len()));
        }
        let Some(palette) = sorted_palette(&dense) else {
            return Ok(Self::Direct(dense.into_boxed_slice()));
        };
        if palette.len() == 1 {
            return Ok(Self::Uniform(dense[0]));
        }
        let bits = bits_for_palette(palette.len());
        let mut words = vec![0_u64; packed_word_count(bits)];
        let mut cached = (dense[0], palette_slot(&palette, dense[0]));
        for (cell, block) in dense.into_iter().enumerate() {
            if block != cached.0 {
                cached = (block, palette_slot(&palette, block));
            }
            write_packed(&mut words, cell, bits, cached.1);
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
                let mut dense = Vec::with_capacity(CHUNK_VOLUME);
                for_each_packed(words, *bits_per_index, |slot| dense.push(palette[slot]));
                dense
            }
        }
    }

    /// Expands the chunk into its distinct blocks plus one palette index per cell.
    ///
    /// Consumers that classify blocks per distinct block (meshing, lighting) index a
    /// table built from the returned palette instead of resolving every cell. Packed and
    /// uniform chunks expand in one pass; direct chunks derive the sorted palette first.
    #[must_use]
    pub fn palette_indices(&self) -> (Vec<BlockStateId>, Vec<u16>) {
        match self {
            Self::Uniform(block) => (vec![*block], vec![0; CHUNK_VOLUME]),
            Self::Packed {
                bits_per_index,
                palette,
                words,
            } => {
                let mut indices = Vec::with_capacity(CHUNK_VOLUME);
                for_each_packed(words, *bits_per_index, |slot| {
                    indices.push(u16::try_from(slot).unwrap_or(0));
                });
                (palette.to_vec(), indices)
            }
            Self::Direct(blocks) => {
                let mut palette = blocks.to_vec();
                palette.sort_unstable();
                palette.dedup();
                let indices = blocks
                    .iter()
                    .map(|block| {
                        palette
                            .binary_search(block)
                            .map_or(0, |slot| u16::try_from(slot).unwrap_or(0))
                    })
                    .collect();
                (palette, indices)
            }
        }
    }

    /// Returns a repacked copy with the supplied cells replaced.
    ///
    /// Later updates to the same cell win. The result is the canonical representation
    /// of the updated cells, exactly as [`Self::from_dense`] would choose it.
    ///
    /// # Errors
    ///
    /// Propagates an internal cell-count invariant failure.
    pub fn with_updates(&self, updates: &[(LocalIndex, BlockStateId)]) -> Result<Self, ChunkError> {
        if let Some(packed) = self.with_updates_in_palette(updates) {
            return Ok(packed);
        }
        let mut dense = self.to_dense();
        for &(index, block) in updates {
            dense[usize::from(index.get())] = block;
        }
        Self::from_dense(dense)
    }

    /// Rewrites packed indices in place when every update stays inside the current
    /// palette and no palette entry disappears, so the canonical palette is unchanged.
    fn with_updates_in_palette(&self, updates: &[(LocalIndex, BlockStateId)]) -> Option<Self> {
        let Self::Packed {
            bits_per_index,
            palette,
            words,
        } = self
        else {
            return None;
        };
        let bits = *bits_per_index;
        let mut resolved = Vec::with_capacity(updates.len());
        for &(index, block) in updates {
            let slot = palette.binary_search(&block).ok()?;
            resolved.push((usize::from(index.get()), slot));
        }
        let mut words = words.clone();
        let mut overwritten = [false; MAX_PALETTE];
        for &(cell, slot) in &resolved {
            let previous = usize::try_from(read_packed(&words, cell, bits)).ok()?;
            overwritten[previous] = true;
            set_packed(&mut words, cell, bits, slot_value(slot));
        }
        // Only the final value of each updated cell is guaranteed present; every other
        // overwritten slot must still occur somewhere in the untouched cells.
        for &(cell, _) in &resolved {
            let kept = usize::try_from(read_packed(&words, cell, bits)).ok()?;
            overwritten[kept] = false;
        }
        let mut missing = overwritten[..palette.len()]
            .iter()
            .filter(|flag| **flag)
            .count();
        if missing > 0 {
            scan_packed(&words, bits, |slot| {
                if overwritten[slot] {
                    overwritten[slot] = false;
                    missing -= 1;
                }
                missing > 0
            });
            if missing > 0 {
                return None;
            }
        }
        Some(Self::Packed {
            bits_per_index: bits,
            palette: palette.clone(),
            words,
        })
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

/// Sorted distinct blocks of a chunk, or `None` once more than [`MAX_PALETTE`] occur.
fn sorted_palette(dense: &[BlockStateId]) -> Option<Vec<BlockStateId>> {
    let mut palette: Vec<BlockStateId> = Vec::with_capacity(16);
    let mut previous = None;
    for &block in dense {
        if previous == Some(block) {
            continue;
        }
        previous = Some(block);
        if let Err(slot) = palette.binary_search(&block) {
            if palette.len() == MAX_PALETTE {
                return None;
            }
            palette.insert(slot, block);
        }
    }
    Some(palette)
}

fn palette_slot(palette: &[BlockStateId], block: BlockStateId) -> u64 {
    palette.binary_search(&block).map_or(0, slot_value)
}

fn slot_value(slot: usize) -> u64 {
    u64::try_from(slot).unwrap_or(0)
}

fn bits_for_palette(len: usize) -> u8 {
    u8::try_from((usize::BITS - (len - 1).leading_zeros()).max(1)).unwrap_or(8)
}

fn packed_word_count(bits: u8) -> usize {
    (CHUNK_VOLUME * usize::from(bits)).div_ceil(64)
}

/// Visits every cell's palette slot in index order with one bounds check per word.
fn for_each_packed(words: &[u64], bits: u8, mut visit: impl FnMut(usize)) {
    scan_packed(words, bits, |slot| {
        visit(slot);
        true
    });
}

/// Visits palette slots in index order until the visitor returns `false`.
fn scan_packed(words: &[u64], bits: u8, mut visit: impl FnMut(usize) -> bool) {
    let bits = usize::from(bits);
    let mask = (1_u64 << bits) - 1;
    let mut word = 0;
    let mut shift = 0;
    let mut current = words.first().copied().unwrap_or(0);
    for _ in 0..CHUNK_VOLUME {
        let mut value = current >> shift;
        if shift + bits > 64 {
            value |= words[word + 1] << (64 - shift);
        }
        if !visit(usize::try_from(value & mask).unwrap_or(0)) {
            return;
        }
        shift += bits;
        if shift >= 64 {
            shift -= 64;
            word += 1;
            current = words.get(word).copied().unwrap_or(0);
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

fn set_packed(words: &mut [u64], index: usize, bits: u8, value: u64) {
    let bit = index * usize::from(bits);
    let word = bit / 64;
    let shift = bit % 64;
    let mask = (1_u64 << bits) - 1;
    words[word] = (words[word] & !(mask << shift)) | (value << shift);
    if shift + usize::from(bits) > 64 {
        let spill = 64 - shift;
        words[word + 1] = (words[word + 1] & !(mask >> spill)) | (value >> spill);
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
    use std::collections::BTreeSet;

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

    /// Deterministic generator so the oracle comparison is reproducible.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn below(&mut self, bound: usize) -> usize {
            usize::try_from(self.next()).unwrap() % bound
        }
    }

    fn apply_dense(
        blocks: &PalettedBlocks,
        updates: &[(LocalIndex, BlockStateId)],
    ) -> PalettedBlocks {
        let mut dense = blocks.to_dense();
        for &(index, block) in updates {
            dense[usize::from(index.get())] = block;
        }
        PalettedBlocks::from_dense(dense).unwrap()
    }

    fn assert_canonical(blocks: &PalettedBlocks) {
        if let PalettedBlocks::Packed {
            bits_per_index,
            palette,
            words,
        } = blocks
        {
            assert!(palette.windows(2).all(|pair| pair[0] < pair[1]));
            let present: BTreeSet<_> = blocks.to_dense().into_iter().collect();
            assert_eq!(present.into_iter().collect::<Vec<_>>(), palette.to_vec());
            assert_eq!(*bits_per_index, bits_for_palette(palette.len()));
            assert_eq!(words.len(), packed_word_count(*bits_per_index));
        }
    }

    fn local(cell: usize) -> LocalIndex {
        LocalIndex::new(u16::try_from(cell).unwrap()).unwrap()
    }

    #[test]
    fn from_dense_matches_sorted_unique_palette_for_random_cells() {
        let mut random = Lcg(7);
        for unique in [2, 3, 5, 7, 16, 100, 256, 300] {
            let dense: Vec<_> = (0..CHUNK_VOLUME)
                .map(|_| id(u32::try_from(random.below(unique)).unwrap()))
                .collect();
            let blocks = PalettedBlocks::from_dense(dense.clone()).unwrap();
            assert_eq!(blocks.to_dense(), dense);
            assert_canonical(&blocks);
            for cell in [0, 1, 63, 64, 1000, CHUNK_VOLUME - 1] {
                assert_eq!(blocks.get(local(cell)), dense[cell]);
            }
        }
    }

    #[test]
    fn in_palette_updates_match_dense_oracle() {
        let mut random = Lcg(11);
        for unique in [2, 3, 4, 7, 8, 9, 31, 33, 255, 256] {
            let dense: Vec<_> = (0..CHUNK_VOLUME)
                .map(|cell| id(u32::try_from(cell % unique).unwrap()))
                .collect();
            let blocks = PalettedBlocks::from_dense(dense).unwrap();
            for round in 0..8 {
                let count = 1 + random.below(64) * round;
                let updates: Vec<_> = (0..count)
                    .map(|_| {
                        let block = id(u32::try_from(random.below(unique + 1)).unwrap());
                        (local(random.below(CHUNK_VOLUME)), block)
                    })
                    .collect();
                let fast = blocks.with_updates(&updates).unwrap();
                assert_eq!(fast, apply_dense(&blocks, &updates));
                assert_canonical(&fast);
            }
        }
    }

    #[test]
    fn removing_last_occurrence_shrinks_palette() {
        let mut dense = vec![id(1); CHUNK_VOLUME];
        dense[5] = id(2);
        dense[6] = id(3);
        let blocks = PalettedBlocks::from_dense(dense).unwrap();
        let overwrite = [(local(6), id(1))];
        let shrunk = blocks.with_updates(&overwrite).unwrap();
        match &shrunk {
            PalettedBlocks::Packed { palette, .. } => {
                assert_eq!(palette.to_vec(), vec![id(1), id(2)]);
            }
            other => panic!("expected packed, got {other:?}"),
        }
        assert_eq!(shrunk, apply_dense(&blocks, &overwrite));
        let moved = blocks
            .with_updates(&[(local(6), id(1)), (local(7), id(3))])
            .unwrap();
        assert_eq!(
            moved,
            blocks
                .with_updates(&[(local(7), id(3)), (local(6), id(1))])
                .unwrap()
        );
        assert_canonical(&moved);
    }

    #[test]
    fn duplicate_updates_apply_in_order() {
        let dense: Vec<_> = (0..CHUNK_VOLUME)
            .map(|cell| id(u32::try_from(cell % 3).unwrap()))
            .collect();
        let blocks = PalettedBlocks::from_dense(dense).unwrap();
        let cell = local(4_000);
        let updates = [(cell, id(2)), (cell, id(0)), (cell, id(1))];
        let result = blocks.with_updates(&updates).unwrap();
        assert_eq!(result.get(cell), id(1));
        assert_eq!(result, apply_dense(&blocks, &updates));
    }

    #[test]
    fn transient_duplicate_write_does_not_keep_a_vanished_palette_entry() {
        // Block 2 occurs only at cell 1; cell 2 is briefly written with 2 and then
        // overwritten, so the final palette must drop block 2.
        let mut dense = vec![id(1); CHUNK_VOLUME];
        dense[1] = id(2);
        dense[2] = id(1);
        dense[3] = id(3);
        let blocks = PalettedBlocks::from_dense(dense).unwrap();
        let updates = [(local(2), id(2)), (local(2), id(3)), (local(1), id(1))];
        let result = blocks.with_updates(&updates).unwrap();
        match &result {
            PalettedBlocks::Packed { palette, .. } => {
                assert_eq!(palette.to_vec(), vec![id(1), id(3)]);
            }
            other => panic!("expected packed, got {other:?}"),
        }
        assert_eq!(result, apply_dense(&blocks, &updates));
        assert_canonical(&result);
    }

    #[test]
    fn straddling_fields_round_trip_through_set_packed() {
        for bits in 1..=8_u8 {
            let mut words = vec![0_u64; packed_word_count(bits)];
            let mask = (1_u64 << bits) - 1;
            let value = |cell: usize, factor: u64| (u64::try_from(cell).unwrap() * factor) & mask;
            for cell in 0..CHUNK_VOLUME {
                set_packed(&mut words, cell, bits, value(cell, 7));
            }
            for cell in 0..CHUNK_VOLUME {
                assert_eq!(read_packed(&words, cell, bits), value(cell, 7));
            }
            for cell in (0..CHUNK_VOLUME).step_by(13) {
                set_packed(&mut words, cell, bits, value(cell, 3));
            }
            let mut visited = Vec::with_capacity(CHUNK_VOLUME);
            for_each_packed(&words, bits, |slot| visited.push(slot_value(slot)));
            for (cell, seen) in visited.iter().enumerate() {
                let expected = if cell % 13 == 0 {
                    value(cell, 3)
                } else {
                    value(cell, 7)
                };
                assert_eq!(
                    read_packed(&words, cell, bits),
                    expected,
                    "bits {bits} cell {cell}"
                );
                assert_eq!(*seen, expected, "bits {bits} cell {cell}");
            }
        }
    }
}
