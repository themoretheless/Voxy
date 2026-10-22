//! Derived rejection-only source hierarchy with optional bounded primitive groups.
use crate::archive_scene::ArchiveMeshSource;

// CPU scheduling candidate; not selected by the production query encoder.
#[path = "world_nearest_wide.rs"]
pub(super) mod wide;
struct Item {
    slot: u32,
    bounds: [f32; 6],
    center: [f64; 3],
}
fn build(items: &mut [Item], nodes: &mut Vec<[u32; 8]>, depth: u32) -> Result<u32, &'static str> {
    build_with_partition(items, nodes, depth, false)
}
// SAH changes scheduling only: every original leaf and its outward bounds remain.
// Twelve bins on each axis bound construction cost. Degenerate/nonfinite costs
// and deep partitions use the balanced builder, retaining the GPU stack bound.
fn sah_partition(items: &mut [Item]) -> Option<usize> {
    const BINS: usize = 12;
    let empty = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let merge = |a: &mut [f64; 6], b: [f64; 6]| {
        for k in 0..3 {
            a[k] = a[k].min(b[k]);
            a[k + 3] = a[k + 3].max(b[k + 3]);
        }
    };
    let area = |b: [f64; 6]| {
        let d: [f64; 3] = std::array::from_fn(|k| (b[k + 3] - b[k]).max(0.));
        d[0] * d[1] + d[1] * d[2] + d[2] * d[0]
    };
    let mut best = None;
    let mut best_cost = f64::INFINITY;
    for axis in 0..3 {
        let lo = items
            .iter()
            .map(|v| v.center[axis])
            .fold(f64::INFINITY, f64::min);
        let hi = items
            .iter()
            .map(|v| v.center[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        if !lo.is_finite() || !hi.is_finite() || hi <= lo {
            continue;
        }
        let bucket = |x: f64| (((x - lo) / (hi - lo) * BINS as f64) as usize).min(BINS - 1);
        let mut counts = [0usize; BINS];
        let mut bounds = [empty; BINS];
        for item in items.iter() {
            let bin = bucket(item.center[axis]);
            counts[bin] += 1;
            merge(&mut bounds[bin], item.bounds.map(f64::from));
        }
        let mut prefix = [empty; BINS];
        let mut prefix_count = [0usize; BINS];
        let mut b = empty;
        let mut count = 0;
        for i in 0..BINS {
            if counts[i] != 0 {
                merge(&mut b, bounds[i]);
            }
            count += counts[i];
            prefix[i] = b;
            prefix_count[i] = count;
        }
        b = empty;
        count = 0;
        for i in (1..BINS).rev() {
            if counts[i] != 0 {
                merge(&mut b, bounds[i]);
            }
            count += counts[i];
            if count == 0 || prefix_count[i - 1] == 0 {
                continue;
            }
            let cost = area(prefix[i - 1]) * prefix_count[i - 1] as f64 + area(b) * count as f64;
            if cost.is_finite() && cost < best_cost {
                best_cost = cost;
                best = Some((axis, lo, hi, i));
            }
        }
    }
    let (axis, lo, hi, split) = best?;
    let mut mid = 0;
    for i in 0..items.len() {
        let bin = (((items[i].center[axis] - lo) / (hi - lo) * BINS as f64) as usize).min(BINS - 1);
        if bin < split {
            items.swap(mid, i);
            mid += 1;
        }
    }
    (mid > 0 && mid < items.len()).then_some(mid)
}
fn build_with_partition(
    items: &mut [Item],
    nodes: &mut Vec<[u32; 8]>,
    depth: u32,
    sah: bool,
) -> Result<u32, &'static str> {
    if depth >= 63 || items.is_empty() {
        return Err("invalid source hierarchy depth");
    }
    let index = u32::try_from(nodes.len()).map_err(|_| "node count overflow")?;
    nodes.push([0; 8]);
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for item in items.iter() {
        for axis in 0..3 {
            bounds[axis] = bounds[axis].min(item.bounds[axis]);
            bounds[axis + 3] = bounds[axis + 3].max(item.bounds[axis + 3]);
        }
    }
    let mut node = [0u32; 8];
    for (out, value) in node.iter_mut().zip(bounds) {
        *out = value.to_bits();
    }
    if items.len() == 1 {
        node[6] = items[0].slot;
        node[7] = u32::MAX;
    } else {
        let mut axis = 0;
        let mut extent = -1.;
        for candidate in 0..3 {
            let lo = items
                .iter()
                .map(|v| v.center[candidate])
                .fold(f64::INFINITY, f64::min);
            let hi = items
                .iter()
                .map(|v| v.center[candidate])
                .fold(f64::NEG_INFINITY, f64::max);
            if hi - lo > extent {
                axis = candidate;
                extent = hi - lo;
            }
        }
        let selected = if sah && depth < 24 {
            sah_partition(items)
        } else {
            None
        };
        let mid = selected.unwrap_or(items.len() / 2);
        if selected.is_none() {
            items.select_nth_unstable_by(mid, |a, b| {
                a.center[axis]
                    .total_cmp(&b.center[axis])
                    .then(a.slot.cmp(&b.slot))
            });
        }
        let (left, right) = items.split_at_mut(mid);
        node[6] = build_with_partition(left, nodes, depth + 1, sah)?;
        node[7] = build_with_partition(right, nodes, depth + 1, sah)?;
    }
    nodes[index as usize] = node;
    Ok(index)
}
// Shared outward affine box construction for both leaf formats. Bounds are
// derived acceleration data; original vertex and matrix words remain the authority.
fn affine_bounds(
    local: [[f32; 2]; 3],
    world: &[[f32; 4]; 4],
) -> Result<([f32; 6], [f64; 3]), &'static str> {
    let mut bounds = [0f32; 6];
    let mut center = [0f64; 3];
    for axis in 0..3 {
        let mut lo = f64::from(world[3][axis]);
        let mut hi = lo;
        for k in 0..3 {
            // Binary32 products are exact in binary64, including subnormal inputs.
            let a = f64::from(world[k][axis]) * f64::from(local[k][0]);
            let b = f64::from(world[k][axis]) * f64::from(local[k][1]);
            lo = (lo + a.min(b)).next_down();
            hi = (hi + a.max(b)).next_up();
        }
        if !lo.is_finite() || !hi.is_finite() {
            return Err("nonfinite source affine interval");
        }
        bounds[axis] = (lo as f32).next_down();
        bounds[axis + 3] = (hi as f32).next_up();
        center[axis] = lo * 0.5 + hi * 0.5;
    }
    Ok((bounds, center))
}
/// Header, same-owner active masks, optional range descriptors, then outward nodes.
pub(super) fn pack(meshes: &[ArchiveMeshSource], active: &[u32]) -> Result<Vec<u32>, &'static str> {
    let mut items = Vec::with_capacity(active.len());
    for mesh in meshes {
        let mut local = [[0f32; 2]; 3];
        for axis in 0..3 {
            local[axis][0] = mesh
                .vertices
                .iter()
                .map(|p| p.position[axis])
                .fold(f32::INFINITY, f32::min);
            local[axis][1] = mesh
                .vertices
                .iter()
                .map(|p| p.position[axis])
                .fold(f32::NEG_INFINITY, f32::max);
        }
        if local.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite source box");
        }
        for instance in &mesh.instances {
            let (bounds, center) = affine_bounds(local, &instance.world)?;
            items.push(Item {
                slot: u32::try_from(items.len()).map_err(|_| "instance count overflow")?,
                bounds,
                center,
            });
        }
    }
    if items.len() != active.len() {
        return Err("source hierarchy owner mismatch");
    }
    let mut nodes = Vec::new();
    let root = if items.is_empty() {
        u32::MAX
    } else {
        build(&mut items, &mut nodes, 0)?
    };
    let offset = 4usize
        .checked_add(active.len())
        .ok_or("hierarchy offset overflow")?;
    let mut words = vec![
        u32::try_from(active.len()).map_err(|_| "instance count overflow")?,
        u32::try_from(nodes.len()).map_err(|_| "node count overflow")?,
        u32::try_from(offset).map_err(|_| "hierarchy offset overflow")?,
        root,
    ];
    words.extend_from_slice(active);
    words.extend_from_slice(bytemuck::cast_slice(&nodes));
    Ok(words)
}

// Versioned leaf tag: MAX is an entire instance; MAX-1 indexes a descriptor
// (stable instance, original first primitive, original primitive count). Derived
// descriptors precede nodes; the node offset and material-table formula stay valid.
pub(super) fn pack_groups(
    meshes: &[ArchiveMeshSource],
    active: &[u32],
) -> Result<Vec<u32>, &'static str> {
    let mut items = Vec::new();
    let mut descriptors = Vec::<u32>::new();
    let mut slot = 0usize;
    for mesh in meshes {
        if mesh.indices.len() % 3 != 0 {
            return Err("incomplete source primitive");
        }
        let primitives = mesh.indices.len() / 3;
        for instance in &mesh.instances {
            for first in (0..primitives).step_by(8) {
                let count = (primitives - first).min(8);
                let mut local = [[f32::INFINITY, f32::NEG_INFINITY]; 3];
                for &index in &mesh.indices[first * 3..(first + count) * 3] {
                    let vertex = mesh
                        .vertices
                        .get(index as usize)
                        .ok_or("invalid source vertex")?;
                    for axis in 0..3 {
                        let v = vertex.position[axis];
                        if !v.is_finite() {
                            return Err("nonfinite source vertex");
                        }
                        local[axis][0] = local[axis][0].min(v);
                        local[axis][1] = local[axis][1].max(v);
                    }
                }
                let (bounds, center) = affine_bounds(local, &instance.world)?;
                let descriptor = u32::try_from(descriptors.len() / 3)
                    .map_err(|_| "source group count overflow")?;
                descriptors.extend([
                    u32::try_from(slot).map_err(|_| "instance count overflow")?,
                    u32::try_from(first).map_err(|_| "primitive count overflow")?,
                    count as u32,
                ]);
                items.push(Item {
                    slot: descriptor,
                    bounds,
                    center,
                });
            }
            slot += 1;
        }
    }
    if slot != active.len() {
        return Err("source group owner mismatch");
    }
    let mut nodes = Vec::new();
    let root = if items.is_empty() {
        u32::MAX
    } else {
        build_with_partition(&mut items, &mut nodes, 0, true)?
    };
    if nodes.len() >= u32::MAX as usize - 1 {
        return Err("reserved source node tag");
    }
    for node in &mut nodes {
        if node[7] == u32::MAX {
            node[7] = u32::MAX - 1;
        }
    }
    let offset = 4usize
        .checked_add(active.len())
        .and_then(|n| n.checked_add(descriptors.len()))
        .ok_or("group offset overflow")?;
    let mut words = vec![
        u32::try_from(active.len()).map_err(|_| "instance count overflow")?,
        u32::try_from(nodes.len()).map_err(|_| "node count overflow")?,
        u32::try_from(offset).map_err(|_| "node offset overflow")?,
        root,
    ];
    words.extend_from_slice(active);
    words.extend(descriptors);
    words.extend_from_slice(bytemuck::cast_slice(&nodes));
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balanced_tree_keeps_every_source_slot_and_encloses_children() {
        let mut items: Vec<_> = (0..257)
            .map(|slot| {
                let x = slot as f32;
                Item {
                    slot,
                    bounds: [x, -x, -0., x + 0.5, -x + 0.5, 0.],
                    center: [f64::from(x), -f64::from(x), 0.],
                }
            })
            .collect();
        let mut nodes = Vec::new();
        let root = build(&mut items, &mut nodes, 0).unwrap();
        assert_eq!(nodes.len(), 513);
        let mut stack = vec![(root, 0)];
        let mut leaves = std::collections::BTreeSet::new();
        let mut visited = std::collections::BTreeSet::new();
        while let Some((index, depth)) = stack.pop() {
            assert!(depth < 64);
            assert!(visited.insert(index));
            let node = nodes[index as usize];
            if node[7] == u32::MAX {
                assert!(leaves.insert(node[6]));
                continue;
            }
            for child in [node[6], node[7]] {
                let bounds = nodes[child as usize];
                for axis in 0..3 {
                    assert!(f32::from_bits(node[axis]) <= f32::from_bits(bounds[axis]));
                    assert!(f32::from_bits(node[axis + 3]) >= f32::from_bits(bounds[axis + 3]));
                }
                stack.push((child, depth + 1));
            }
        }
        assert_eq!(visited.len(), nodes.len());
        assert_eq!(leaves, (0..257).collect());
    }
    #[test]
    fn primitive_groups_keep_every_original_id_under_wide_affine_transforms() {
        use crate::archive_scene::{ArchiveInstance, ArchiveVertex};
        let mut seed = 0x564f5859u32;
        let mut raw = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let exponent = 1 + (seed >> 24) % 254;
            (seed & 0x807fffff) | (exponent << 23)
        };
        let mut meshes = Vec::new();
        for _ in 0..32 {
            let vertices = (0..51)
                .map(|_| ArchiveVertex {
                    position: std::array::from_fn(|_| f32::from_bits(raw())),
                    normal: [0.; 3],
                    tangent: [0.; 4],
                    uv: [0.; 2],
                })
                .collect();
            let instances = (0..2)
                .map(|_| {
                    let mut world: [[f32; 4]; 4] =
                        std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(raw())));
                    for column in &mut world[..3] {
                        column[3] = 0.;
                    }
                    world[3][3] = 1.;
                    ArchiveInstance {
                        world,
                        normal: [[0.; 4]; 3],
                    }
                })
                .collect();
            meshes.push(ArchiveMeshSource {
                vertices,
                indices: (0..51).collect(),
                instances,
                material: 0,
                center: glam::Vec3::ZERO,
            });
        }
        // Include singular/reflected transforms and subnormal source words.
        meshes[0].instances[0].world = [[0.; 4]; 4];
        meshes[0].instances[0].world[3][3] = 1.;
        meshes[0].vertices[0].position = [f32::from_bits(1), -f32::from_bits(1), -0.];
        let words = pack_groups(&meshes, &vec![255; 64]).unwrap();
        let begin = 4 + 64;
        let end = words[2] as usize;
        assert_eq!((end - begin) / 3, 64 * 3);
        let mut covered = std::collections::BTreeSet::new();
        for descriptor in words[begin..end].chunks_exact(3) {
            assert!(descriptor[2] > 0 && descriptor[2] <= 8);
            for primitive in descriptor[1]..descriptor[1] + descriptor[2] {
                assert!(covered.insert((descriptor[0], primitive)));
            }
        }
        assert_eq!(
            covered,
            (0..64)
                .flat_map(|slot| (0..17).map(move |p| (slot, p)))
                .collect()
        );
        if let Some(path) = std::env::var_os("VOXY_SOURCE_GROUP_BOUND_DUMP") {
            let source:Vec<_>=meshes.iter().map(|m|serde_json::json!({"positions":m.vertices.iter().map(|v|v.position.map(f32::to_bits)).collect::<Vec<_>>(),"indices":m.indices,"instances":m.instances.iter().map(|v|v.world.map(|r|r.map(f32::to_bits))).collect::<Vec<_>>()})).collect();
            std::fs::write(
                path,
                serde_json::to_vec(&serde_json::json!({"meshes":source,"hierarchy":words}))
                    .unwrap(),
            )
            .unwrap();
        }
    }
    #[test]
    fn sah_skewed_clusters_keep_all_leaves_and_reduce_total_internal_area() {
        fn items() -> Vec<Item> {
            (0..129)
                .map(|slot| {
                    let x = if slot == 128 {
                        10000.
                    } else {
                        slot as f32 * 0.01
                    };
                    Item {
                        slot,
                        bounds: [x, 0., 0., x + 0.005, 1., 1.],
                        center: [f64::from(x) + 0.0025, 0.5, 0.5],
                    }
                })
                .collect()
        }
        let mut median = Vec::new();
        build(&mut items(), &mut median, 0).unwrap();
        let mut sah = Vec::new();
        let root = build_with_partition(&mut items(), &mut sah, 0, true).unwrap();
        assert_eq!(sah.len(), median.len());
        let mut leaves = std::collections::BTreeSet::new();
        let mut visited = std::collections::BTreeSet::new();
        let mut stack = vec![(root, 0)];
        while let Some((id, depth)) = stack.pop() {
            assert!(depth < 63 && visited.insert(id));
            let n = sah[id as usize];
            if n[7] == u32::MAX {
                assert!(leaves.insert(n[6]));
                continue;
            }
            for child in [n[6], n[7]] {
                let b = sah[child as usize];
                for axis in 0..3 {
                    assert!(f32::from_bits(n[axis]) <= f32::from_bits(b[axis]));
                    assert!(f32::from_bits(n[axis + 3]) >= f32::from_bits(b[axis + 3]));
                }
                stack.push((child, depth + 1));
            }
        }
        assert_eq!(visited.len(), sah.len());
        assert_eq!(leaves, (0..129).collect());
        let cost = |nodes: &Vec<[u32; 8]>| {
            nodes
                .iter()
                .filter(|n| n[7] != u32::MAX)
                .map(|n| {
                    let d: [f64; 3] = std::array::from_fn(|k| {
                        f64::from(f32::from_bits(n[k + 3])) - f64::from(f32::from_bits(n[k]))
                    });
                    d[0] * d[1] + d[1] * d[2] + d[2] * d[0]
                })
                .sum::<f64>()
        };
        assert!(cost(&sah) < cost(&median));
        let mut degenerate = items();
        for v in &mut degenerate {
            v.center = [0.; 3];
        }
        assert!(sah_partition(&mut degenerate).is_none());
        // Deep nodes retain median partitioning, keeping a finite stack bound.
        build_with_partition(&mut items(), &mut Vec::new(), 24, true).unwrap();
    }
    #[test]
    fn invalid_depth_is_an_error_instead_of_a_truncated_tree() {
        let mut items = [Item {
            slot: 0,
            bounds: [0.; 6],
            center: [0.; 3],
        }];
        assert!(build(&mut items, &mut Vec::new(), 63).is_err());
    }
}
