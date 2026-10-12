//! Spatial hash grid over segment cells and candidate pair enumeration.
//!
//! Split out of `contact.rs` (plan step B4) with unchanged bodies.

use super::*;

pub(in crate::hair) fn cell_key(cell: [i32; 3]) -> (u64, u32) {
    (
        (u64::from(cell[0] as u32) << 32) | u64::from(cell[1] as u32),
        cell[2] as u32,
    )
}

pub(in crate::hair) type SegmentId = (usize, usize);
pub(super) type ContactGrid = HashMap<(u64, u32), Vec<(SegmentId, [i32; 3])>>;

// Preserve lexicographic pair order while sorting compact integer keys.
pub(super) fn sort_contact_pairs(pairs: &mut [(SegmentId, SegmentId)]) {
    if pairs.len() < 128 {
        pairs.sort_unstable();
        return;
    }
    let (mut rod_max, mut segment_max) = (0usize, 0usize);
    for &(a, b) in pairs.iter() {
        rod_max = rod_max.max(a.0).max(b.0);
        segment_max = segment_max.max(a.1).max(b.1);
    }
    let rod_bits = (usize::BITS - rod_max.leading_zeros()).max(1);
    let segment_bits = (usize::BITS - segment_max.leading_zeros()).max(1);
    if 2 * (rod_bits + segment_bits) > 64 {
        pairs.sort_unstable();
        return;
    }
    let mut keys: Vec<u64> = pairs
        .iter()
        .map(|&(a, b)| {
            (((((a.0 as u64) << segment_bits) | a.1 as u64) << rod_bits | b.0 as u64)
                << segment_bits)
                | b.1 as u64
        })
        .collect();
    keys.sort_unstable();
    let rod_mask = (1u64 << rod_bits) - 1;
    let segment_mask = (1u64 << segment_bits) - 1;
    for (pair, mut key) in pairs.iter_mut().zip(keys) {
        let ib = (key & segment_mask) as usize;
        key >>= segment_bits;
        let rb = (key & rod_mask) as usize;
        key >>= rod_bits;
        let ia = (key & segment_mask) as usize;
        key >>= segment_bits;
        *pair = ((key as usize, ia), (rb, ib));
    }
}

pub(in crate::hair) fn unique_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    unique_cell_pairs_filtered(grid, |_| true)
}
pub(super) fn unique_cell_pairs_filtered(
    grid: &ContactGrid,
    mut keep: impl FnMut((SegmentId, SegmentId)) -> bool,
) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::with_capacity(grid.len().saturating_mul(2));
    for (&key, entries) in grid {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key {
                    continue;
                }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 {
                    pair = (pair.1, pair.0);
                }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 {
                    continue;
                }
                if !keep(pair) {
                    continue;
                }
                pairs.push(pair);
            }
        }
    }
    sort_contact_pairs(&mut pairs);
    pairs
}

#[cfg(test)]
pub(super) fn candidate_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::new();
    for (&key, entries) in grid {
        let cell = [
            (key.0 >> 32) as u32 as i32,
            key.0 as u32 as i32,
            key.1 as i32,
        ];
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 {
                    continue;
                }
                // Reject duplicate cells axis by axis before packing a new key.
                // The first shared integer cell remains the sole pair owner.
                if (0..3).any(|axis| entries[a].1[axis].max(entries[b].1[axis]) != cell[axis]) {
                    continue;
                }
                if pair.0 > pair.1 {
                    pair = (pair.1, pair.0);
                }
                pairs.push(pair);
            }
        }
    }
    pairs.sort_unstable();
    pairs
}
