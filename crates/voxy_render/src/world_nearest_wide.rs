//! Lossless scheduling view of the existing binary source hierarchy.
//! No quantized bounds, source geometry, or intersection predicates live here.
//! Leaf words retain their original descriptors. Internal words point to the
//! derived eight-child frontier. The source revision remains the resource owner.
const LEAF: u32 = u32::MAX - 1;

#[derive(Debug)]
pub(crate) struct WideNode {
    pub(crate) count: u32,
    pub(crate) children: [[u32; 8]; 8],
}
#[derive(Debug)]
pub(crate) struct WideHierarchy {
    pub(crate) root: Option<[u32; 8]>,
    pub(crate) nodes: Vec<WideNode>,
    /// Upper bound for pending references under any child visitation order.
    pub(crate) pending_capacity: usize,
}

/// Standalone derived scheduling block. The original binary hierarchy and its
/// material-table offset remain intact when this block is appended later.
/// Header: magic, wide count, pending capacity, root present, eight root words.
/// Each node has four header words followed by eight original 8-word records.
impl WideHierarchy {
    pub(crate) fn packed_word_len(&self) -> Result<usize, &'static str> {
        let length = self
            .nodes
            .len()
            .checked_mul(68)
            .and_then(|n| n.checked_add(12))
            .ok_or("wide block length overflow")?;
        u32::try_from(length).map_err(|_| "wide block word offset overflow")?;
        Ok(length)
    }
    pub(crate) fn to_words(&self) -> Result<Vec<u32>, &'static str> {
        let count = u32::try_from(self.nodes.len()).map_err(|_| "wide node count overflow")?;
        let capacity =
            u32::try_from(self.pending_capacity).map_err(|_| "wide capacity overflow")?;
        let length = self.packed_word_len()?;
        let mut words = Vec::with_capacity(length);
        words.extend([0x38595657, count, capacity, u32::from(self.root.is_some())]);
        words.extend(self.root.unwrap_or([0; 8]));
        for node in &self.nodes {
            if !(2..=8).contains(&node.count) {
                return Err("invalid wide frontier count");
            }
            words.extend([node.count, 0, 0, 0]);
            for child in node.children {
                words.extend(child);
            }
        }
        Ok(words)
    }
}

/// Validate the complete source tree before conversion. Shared children, cycles,
/// unreachable nodes and invalid references must not silently discard geometry.
pub(crate) fn convert(source: &[[u32; 8]], root: u32) -> Result<WideHierarchy, &'static str> {
    if source.is_empty() {
        return if root == u32::MAX {
            Ok(WideHierarchy {
                root: None,
                nodes: Vec::new(),
                pending_capacity: 0,
            })
        } else {
            Err("nonempty root of empty source hierarchy")
        };
    }
    let mut seen = vec![false; source.len()];
    let mut pending = vec![(root, 0usize)];
    let mut reached = 0;
    let mut traversal = Vec::with_capacity(source.len());
    while let Some((index, depth)) = pending.pop() {
        let node = source
            .get(index as usize)
            .ok_or("invalid source node reference")?;
        if depth >= 63 {
            return Err("source hierarchy exceeds original depth bound");
        }
        if std::mem::replace(&mut seen[index as usize], true) {
            return Err("shared or cyclic source hierarchy");
        }
        reached += 1;
        traversal.push(index);
        if node[7] < LEAF {
            pending.extend([(node[7], depth + 1), (node[6], depth + 1)]);
        }
    }
    if reached != source.len() {
        return Err("unreachable source nodes");
    }
    // A binary subtree of height h always fits h pending references under
    // either child order. Retain this complete fallback while grouping nodes.
    let mut heights = vec![1u8; source.len()];
    for &index in traversal.iter().rev() {
        let node = source[index as usize];
        if node[7] < LEAF {
            heights[index as usize] = 1 + heights[node[6] as usize].max(heights[node[7] as usize]);
        }
    }
    let mut nodes = Vec::new();
    let (root, pending_capacity) = emit(source, root, &mut nodes, &heights, 64)?;
    Ok(WideHierarchy {
        root: Some(root),
        nodes,
        pending_capacity,
    })
}

fn emit(
    source: &[[u32; 8]],
    index: u32,
    wide: &mut Vec<WideNode>,
    heights: &[u8],
    budget: usize,
) -> Result<([u32; 8], usize), &'static str> {
    if usize::from(heights[index as usize]) > budget {
        return Err("source subtree exceeds remaining scheduling budget");
    }
    let mut reference = source[index as usize];
    if reference[7] >= LEAF {
        return Ok((reference, 1));
    }
    let mut frontier = vec![reference[6], reference[7]];
    let mut depths = vec![1u8, 1u8];
    // Prefer shallow frontiers, but expand only while every child retains
    // enough budget for its COMPLETE binary subtree. This adapts packet width,
    // never truncates children or widens the existing GPU private stack.
    while frontier.len() < 8 {
        let mut selected: Option<usize> = None;
        for (position, &child) in frontier.iter().enumerate() {
            let node = source[child as usize];
            if node[7] >= LEAF {
                continue;
            }
            let mut child_height = heights[node[6] as usize].max(heights[node[7] as usize]);
            for (other_position, &other) in frontier.iter().enumerate() {
                if other_position != position {
                    child_height = child_height.max(heights[other as usize]);
                }
            }
            // Expanding one entry adds one sibling. All siblings can remain
            // pending while any one child's complete traversal executes.
            if frontier.len() + usize::from(child_height) > budget {
                continue;
            }
            if selected.is_none_or(|old| depths[position] < depths[old]) {
                selected = Some(position);
            }
        }
        let Some(position) = selected else { break };
        let node = source[frontier[position] as usize];
        let depth = depths[position] + 1;
        frontier.splice(position..=position, [node[6], node[7]]);
        depths.splice(position..=position, [depth, depth]);
    }
    let destination = u32::try_from(wide.len()).map_err(|_| "wide hierarchy index overflow")?;
    if destination >= LEAF {
        return Err("reserved wide hierarchy index");
    }
    let count = frontier.len();
    wide.push(WideNode {
        count: count as u32,
        children: [[0; 8]; 8],
    });
    let child_budget = budget
        .checked_sub(count - 1)
        .ok_or("wide child budget underflow")?;
    let mut child_capacity = 1;
    for (lane, child) in frontier.into_iter().enumerate() {
        let (words, capacity) = emit(source, child, wide, heights, child_budget)?;
        wide[destination as usize].children[lane] = words;
        child_capacity = child_capacity.max(capacity);
    }
    // All sibling references may remain pending while the deepest child runs.
    let capacity = child_capacity
        .checked_add(count - 1)
        .ok_or("wide pending capacity overflow")?;
    if capacity > budget {
        return Err("wide schedule exceeds certified pending budget");
    }
    reference[6] = destination;
    reference[7] = 0;
    Ok((reference, capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree(leaves: usize, chain: bool) -> Vec<[u32; 8]> {
        fn build(n: usize, chain: bool, nodes: &mut Vec<[u32; 8]>, slot: &mut u32) -> u32 {
            let i = nodes.len() as u32;
            // Include distinct signed-zero/subnormal/infinite raw bound words:
            // conversion preserves the source contract rather than recomputing it.
            nodes.push([0x80000000, 1, 0xff800000, 0, 0x7f7fffff, 0x7f800000, 0, 0]);
            if n == 1 {
                nodes[i as usize][6] = *slot;
                nodes[i as usize][7] = u32::MAX - (*slot % 2);
                *slot += 1;
            } else {
                let left = if chain { 1 } else { n / 2 };
                let a = build(left, chain, nodes, slot);
                let b = build(n - left, chain, nodes, slot);
                nodes[i as usize][6] = a;
                nodes[i as usize][7] = b;
            }
            i
        }
        let mut nodes = Vec::new();
        build(leaves, chain, &mut nodes, &mut 0);
        nodes
    }
    fn walk(
        h: &WideHierarchy,
        r: [u32; 8],
        pending: &mut Vec<[u32; 8]>,
        leaves: &mut Vec<[u32; 8]>,
        reverse: bool,
    ) -> usize {
        pending.push(r);
        let mut maximum = 1;
        while let Some(r) = pending.pop() {
            if r[7] >= LEAF {
                leaves.push(r);
            } else {
                let node = &h.nodes[r[6] as usize];
                let active = &node.children[..node.count as usize];
                if reverse {
                    pending.extend(active.iter().copied());
                } else {
                    pending.extend(active.iter().rev().copied());
                }
            }
            maximum = maximum.max(pending.len());
        }
        maximum
    }
    #[test]
    fn all_original_leaf_words_and_order_survive_both_leaf_tags() {
        for n in 1..=257 {
            for chain in [false, true] {
                if chain && n >= 64 {
                    continue;
                }
                let source = tree(n, chain);
                let expected: Vec<_> = source.iter().copied().filter(|x| x[7] >= LEAF).collect();
                let h = convert(&source, 0).unwrap();
                assert_eq!(&h.root.unwrap()[..6], &source[0][..6]);
                for reverse in [false, true] {
                    let mut actual = Vec::new();
                    let peak = walk(&h, h.root.unwrap(), &mut Vec::new(), &mut actual, reverse);
                    if reverse {
                        actual.reverse();
                    }
                    assert_eq!(actual, expected);
                    assert!(peak <= h.pending_capacity);
                }
                assert!(h.nodes.iter().all(|x| (2..=8).contains(&x.count)));
                assert!(h.nodes.len() <= (source.len() - 1) / 2);
            }
        }
    }
    #[test]
    fn serialized_records_keep_original_bounds_and_leaf_words() {
        let source = tree(31, false);
        let h = convert(&source, 0).unwrap();
        let words = h.to_words().unwrap();
        assert_eq!(
            &words[..4],
            &[
                0x38595657,
                h.nodes.len() as u32,
                h.pending_capacity as u32,
                1
            ]
        );
        assert_eq!(&words[4..12], &h.root.unwrap());
        assert_eq!(words.len(), 12 + 68 * h.nodes.len());
        assert_eq!(words.len(), h.packed_word_len().unwrap());
        for (index, node) in h.nodes.iter().enumerate() {
            let base = 12 + index * 68;
            assert_eq!(&words[base..base + 4], &[node.count, 0, 0, 0]);
            for lane in 0..8 {
                let begin = base + 4 + lane * 8;
                assert_eq!(&words[begin..begin + 8], &node.children[lane]);
            }
        }
        let empty = convert(&[], u32::MAX).unwrap().to_words().unwrap();
        assert_eq!(empty.len(), 12);
        assert_eq!(empty[3], 0);
    }

    #[test]
    fn each_retained_bound_still_owns_the_exact_original_subtree() {
        let mut source = tree(73, false);
        for i in (0..source.len()).rev() {
            let (lo, hi) = if source[i][7] >= LEAF {
                let lo = source[i][6] as f32 * 2.;
                (lo, lo + 1.)
            } else {
                (
                    f32::from_bits(source[source[i][6] as usize][0]),
                    f32::from_bits(source[source[i][7] as usize][3]),
                )
            };
            source[i][..6].copy_from_slice(&[
                lo.to_bits(),
                0x80000000,
                0,
                hi.to_bits(),
                1f32.to_bits(),
                1f32.to_bits(),
            ]);
        }
        let owners: std::collections::BTreeMap<[u32; 6], usize> = source
            .iter()
            .enumerate()
            .map(|(i, n)| (n[..6].try_into().unwrap(), i))
            .collect();
        assert_eq!(
            owners.len(),
            source.len(),
            "distinct source subtrees need distinct test bounds"
        );
        let h = convert(&source, 0).unwrap();
        let mut references = vec![h.root.unwrap()];
        for node in &h.nodes {
            references.extend_from_slice(&node.children[..node.count as usize]);
        }
        for reference in references {
            let key: [u32; 6] = reference[..6].try_into().unwrap();
            let mut stack = vec![owners[&key]];
            let mut expected = Vec::new();
            while let Some(index) = stack.pop() {
                let node = source[index];
                if node[7] >= LEAF {
                    expected.push(node);
                } else {
                    stack.extend([node[7] as usize, node[6] as usize]);
                }
            }
            let mut actual = Vec::new();
            walk(&h, reference, &mut Vec::new(), &mut actual, false);
            assert_eq!(
                actual, expected,
                "bounds must remain attached to their original leaf sequence"
            );
        }
    }

    #[test]
    fn large_balanced_tree_retains_all_leaves_within_existing_gpu_stack() {
        let source = tree(16384, false);
        let h = convert(&source, 0).unwrap();
        assert!(
            h.pending_capacity <= 64,
            "wide schedule needs {} pending references",
            h.pending_capacity
        );
        let mut actual = Vec::new();
        walk(&h, h.root.unwrap(), &mut Vec::new(), &mut actual, false);
        let expected: Vec<_> = source.iter().copied().filter(|n| n[7] >= LEAF).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn malformed_source_is_rejected_without_partial_result() {
        let mut nodes = tree(4, false);
        nodes[0][6] = 0;
        assert!(convert(&nodes, 0).is_err());
        let mut nodes = tree(4, false);
        nodes[0][7] = nodes[0][6];
        assert!(convert(&nodes, 0).is_err());
        let mut nodes = tree(4, false);
        nodes[0][6] = 999;
        assert!(convert(&nodes, 0).is_err());
        let mut nodes = tree(4, false);
        nodes.push(nodes[1]);
        assert!(convert(&nodes, 0).is_err());
        assert!(convert(&tree(64, true), 0).is_err());
        assert!(convert(&[], 0).is_err());
        assert!(convert(&[], u32::MAX).unwrap().root.is_none());
    }
}
