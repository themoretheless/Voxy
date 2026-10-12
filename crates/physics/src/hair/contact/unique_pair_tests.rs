use super::*;
#[test]
#[ignore = "paired broad-phase benchmark"]
fn owner_filter_profile() {
    let mut grid = ContactGrid::new();
    for rod in 0..469usize {
        for segment in 0..20usize {
            let min = [
                (rod % 19) as i32 - 9,
                segment as i32 - 10,
                (rod / 19) as i32 - 12,
            ];
            for x in min[0]..=min[0] + 1 {
                for y in min[1]..=min[1] + 2 {
                    for z in min[2]..=min[2] + 1 {
                        grid.entry(cell_key([x, y, z]))
                            .or_default()
                            .push(((rod, segment), min));
                    }
                }
            }
        }
    }
    let baseline = || {
        let mut pairs = Vec::new();
        for (&key, entries) in &grid {
            for a in 0..entries.len() {
                for b in a + 1..entries.len() {
                    let owner =
                        std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
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
                    pairs.push(pair);
                }
            }
        }
        pairs.sort_unstable();
        pairs
    };
    let reference = baseline();
    assert_eq!(unique_cell_pairs(&grid), reference);
    assert_eq!(candidate_cell_pairs(&grid), reference);
    let mut times = [Vec::new(), Vec::new()];
    for repeat in 0..12 {
        for mode in [repeat % 2, 1 - repeat % 2] {
            let started = std::time::Instant::now();
            let result = if mode == 0 {
                baseline()
            } else {
                unique_cell_pairs(&grid)
            };
            times[mode].push(started.elapsed().as_secs_f64() * 1000.);
            assert_eq!(result, reference);
            std::hint::black_box(result);
        }
    }
    for samples in &mut times {
        samples.sort_by(f64::total_cmp);
    }
    eprintln!(
        "PACKED SORT pairs={} baseline_median_ms={:.3} candidate_median_ms={:.3}",
        reference.len(),
        times[0][6],
        times[1][6]
    );
}
#[test]
fn packed_sort_preserves_lexicographic_order_and_large_id_fallback() {
    for size in [0, 1, 127, 128, 4096] {
        for large in [false, true] {
            let mut pairs: Vec<_> = (0..size)
                .map(|i| {
                    let rod = if large { usize::MAX - i } else { i * 37 % 469 };
                    ((rod, i * 13 % 20), (i * 71 % 469, i * 7 % 20))
                })
                .collect();
            let mut reference = pairs.clone();
            reference.sort_unstable();
            sort_contact_pairs(&mut pairs);
            assert_eq!(pairs, reference);
        }
    }
    let mut zeros = vec![((0, 0), (0, 0)); 128];
    sort_contact_pairs(&mut zeros);
    assert!(zeros.iter().all(|p| *p == ((0, 0), (0, 0))));
}
#[test]
fn canonical_cells_preserve_hashset_candidates_exactly() {
    let mut grid = ContactGrid::new();
    for id in 0..48 {
        let min = [
            (id % 5) as i32 - 3,
            (id % 7) as i32 - 4,
            (id % 3) as i32 - 2,
        ];
        let max = [min[0] + 3, min[1] + 2, min[2] + 1];
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    grid.entry(cell_key([x, y, z]))
                        .or_default()
                        .push(((id / 8, id % 8), min));
                }
            }
        }
    }
    let mut reference = std::collections::HashSet::new();
    for entries in grid.values() {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 {
                    pair = (pair.1, pair.0);
                }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 {
                    continue;
                }
                reference.insert(pair);
            }
        }
    }
    let mut reference: Vec<_> = reference.into_iter().collect();
    reference.sort_unstable();
    assert!(!reference.is_empty());
    assert_eq!(unique_cell_pairs(&grid), reference);
    assert_eq!(candidate_cell_pairs(&grid), reference);
}
