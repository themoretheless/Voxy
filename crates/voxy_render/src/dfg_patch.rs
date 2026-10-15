//! Structural validation and immutable packing of experimental energy patches.
//! Format validation is not a scientific whole-cell accuracy certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GgxEnergyPatchError {
    InvalidBounds,
    InvalidAxis { axis: usize, index: usize },
    InvalidMoment { index: usize },
    EmptyAtlas,
    AtlasTooLarge,
    Overlap { first: usize, second: usize },
}
impl std::fmt::Display for GgxEnergyPatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid experimental GGX patch: {self:?}")
    }
}
impl std::error::Error for GgxEnergyPatchError {}
#[derive(Debug, Clone)]
pub struct GgxEnergyPatchData {
    bounds: [f32; 4],
    view_nodes: [f32; 6],
    roughness_nodes: [f32; 6],
    moments: [[f32; 4]; 36],
}
impl GgxEnergyPatchData {
    /// Validate storage, axes, node validity and energy mass. Does not certify
    /// interpolation accuracy. Bounds are [min_view,min_rough,max_view,max_rough].
    pub fn try_new(
        bounds: [f32; 4],
        view_nodes: [f32; 6],
        roughness_nodes: [f32; 6],
        moments: [[f32; 4]; 36],
    ) -> Result<Self, GgxEnergyPatchError> {
        if !bounds
            .iter()
            .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            || bounds[0] >= bounds[2]
            || bounds[1] >= bounds[3]
        {
            return Err(GgxEnergyPatchError::InvalidBounds);
        }
        for (axis, nodes) in [&view_nodes, &roughness_nodes].iter().enumerate() {
            for (index, &value) in nodes.iter().enumerate() {
                if !value.is_finite()
                    || !(0. ..=1.).contains(&value)
                    || (axis == 1 && (value * value) * (value * value) <= 0.)
                    || (index > 0 && value <= nodes[index - 1])
                {
                    return Err(GgxEnergyPatchError::InvalidAxis { axis, index });
                }
            }
        }
        if bounds[0] < view_nodes[0]
            || bounds[2] > view_nodes[5]
            || bounds[1] < roughness_nodes[0]
            || bounds[3] > roughness_nodes[5]
        {
            return Err(GgxEnergyPatchError::InvalidBounds);
        }
        for (index, m) in moments.iter().enumerate() {
            if m[3] != 1.
                || !m[..3]
                    .iter()
                    .all(|v| v.is_finite() && (0. ..=1.).contains(v))
                || (m[0] + m[1] + m[2] - 1.).abs() > 5e-6
            {
                return Err(GgxEnergyPatchError::InvalidMoment { index });
            }
        }
        Ok(Self {
            bounds,
            view_nodes,
            roughness_nodes,
            moments,
        })
    }
    #[must_use]
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }
}
// Balanced preorder AABB tree. Escape indices skip an entire rejected
// subtree, so GPU traversal requires neither a stack nor recursion.
fn spatial_index(order: &[&GgxEnergyPatchData]) -> Vec<([f32; 4], [f32; 4])> {
    fn emit(
        ids: &mut [usize],
        order: &[&GgxEnergyPatchData],
        nodes: &mut Vec<([f32; 4], [f32; 4])>,
    ) {
        let mut bounds = order[ids[0]].bounds;
        for &id in &ids[1..] {
            let b = order[id].bounds;
            bounds = [
                bounds[0].min(b[0]),
                bounds[1].min(b[1]),
                bounds[2].max(b[2]),
                bounds[3].max(b[3]),
            ];
        }
        let current = nodes.len();
        nodes.push((bounds, [0.; 4]));
        if ids.len() == 1 {
            nodes[current].1 = [(current + 1) as f32, ids[0] as f32, 1., 0.];
            return;
        }
        let axis = usize::from(bounds[3] - bounds[1] > bounds[2] - bounds[0]);
        ids.sort_by(|&a, &b| {
            let center = |id: usize| order[id].bounds[axis] + order[id].bounds[axis + 2];
            center(a).total_cmp(&center(b)).then(a.cmp(&b))
        });
        let middle = ids.len() / 2;
        let (left, right) = ids.split_at_mut(middle);
        emit(left, order, nodes);
        emit(right, order, nodes);
        nodes[current].1 = [nodes.len() as f32, -1., 0., 0.];
    }
    let mut nodes = Vec::with_capacity(2 * order.len() - 1);
    emit(&mut (0..order.len()).collect::<Vec<_>>(), order, &mut nodes);
    nodes
}
pub(super) fn atlas_height(patch_count: usize) -> Result<u32, GgxEnergyPatchError> {
    if patch_count == 0 {
        return Err(GgxEnergyPatchError::EmptyAtlas);
    }
    let count = u32::try_from(patch_count).map_err(|_| GgxEnergyPatchError::AtlasTooLarge)?;
    let start = count
        .checked_mul(8)
        .and_then(|h| h.checked_add(1))
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)?;
    let nodes = count
        .checked_mul(2)
        .and_then(|n| n.checked_sub(1))
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)?;
    start
        .checked_add(nodes.div_ceil(3))
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)
}
pub(super) fn atlas_allocation_bytes(patch_count: usize) -> Result<usize, GgxEnergyPatchError> {
    // RGBA32F texture (six pixels/row) plus retained 256-byte upload rows.
    (atlas_height(patch_count)? as usize)
        .checked_mul(6 * 16 + 256)
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)
}
/// Padded rows for a six-texel-wide RGBA32F texture. Sixteen texels per
/// upload row give the required 256-byte buffer-copy row alignment.
pub(super) fn pack_atlas(
    patches: &[GgxEnergyPatchData],
    max_dimension: u32,
) -> Result<(u32, Vec<[f32; 4]>), GgxEnergyPatchError> {
    if patches.is_empty() {
        return Err(GgxEnergyPatchError::EmptyAtlas);
    }
    let count = u32::try_from(patches.len()).map_err(|_| GgxEnergyPatchError::AtlasTooLarge)?;
    let index_start = count
        .checked_mul(8)
        .and_then(|h| h.checked_add(1))
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)?;
    let node_count = count
        .checked_mul(2)
        .and_then(|n| n.checked_sub(1))
        .ok_or(GgxEnergyPatchError::AtlasTooLarge)?;
    let height = atlas_height(patches.len())?;
    if max_dimension < 6 || height > max_dimension {
        return Err(GgxEnergyPatchError::AtlasTooLarge);
    }
    let mut order = patches.iter().collect::<Vec<_>>();
    order.sort_by(|a, b| {
        a.bounds
            .iter()
            .zip(b.bounds)
            .map(|(x, y)| x.total_cmp(&y))
            .find(|o| !o.is_eq())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for first in 0..order.len() {
        for second in first + 1..order.len() {
            let a = order[first].bounds;
            let b = order[second].bounds;
            if a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3] {
                return Err(GgxEnergyPatchError::Overlap { first, second });
            }
        }
    }
    let mut pixels = vec![[0.; 4]; height as usize * 16];
    pixels[0] = [4137., 2., count as f32, 8.];
    pixels[1] = [index_start as f32, node_count as f32, 1., 0.];
    for (i, p) in order.iter().enumerate() {
        let row = 1 + 8 * i;
        pixels[row * 16] = p.bounds;
        for x in 0..6 {
            pixels[(row + 1) * 16 + x] = [p.view_nodes[x], p.roughness_nodes[x], 0., 1.];
        }
        for y in 0..6 {
            for x in 0..6 {
                pixels[(row + 2 + y) * 16 + x] = p.moments[y * 6 + x];
            }
        }
    }
    let nodes = spatial_index(&order);
    assert_eq!(nodes.len(), node_count as usize);
    for (i, (bounds, links)) in nodes.into_iter().enumerate() {
        let row = index_start as usize + i / 3;
        let x = 2 * (i % 3);
        pixels[row * 16 + x] = bounds;
        pixels[row * 16 + x + 1] = links;
    }
    Ok((height, pixels))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn region(bounds: [f32; 4]) -> GgxEnergyPatchData {
        GgxEnergyPatchData::try_new(
            bounds,
            [0., 0.2, 0.4, 0.6, 0.8, 1.],
            [0.05, 0.2, 0.4, 0.6, 0.8, 1.],
            [[0.5, 0.25, 0.25, 1.]; 36],
        )
        .unwrap()
    }
    #[test]
    fn rejects_invalid_nodes_and_axes() {
        let p = region([0.2, 0.2, 0.8, 0.8]);
        let mut m = p.moments;
        m[4][3] = 0.;
        assert!(matches!(
            GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, p.roughness_nodes, m),
            Err(GgxEnergyPatchError::InvalidMoment { index: 4 })
        ));
        let mut xs = p.view_nodes;
        xs[2] = xs[1];
        assert!(GgxEnergyPatchData::try_new(p.bounds, xs, p.roughness_nodes, p.moments).is_err());
        let mut ys = p.roughness_nodes;
        ys[0] = f32::MIN_POSITIVE;
        assert!(GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, ys, p.moments).is_err());
        let mut m = p.moments;
        m[0][0] = f32::NAN;
        assert!(GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, p.roughness_nodes, m).is_err());
    }
    #[test]
    fn rejects_bounds_outside_support_and_energy_error() {
        let p = region([0.2, 0.2, 0.8, 0.8]);
        assert!(
            GgxEnergyPatchData::try_new(
                [0., 0., 1., 1.],
                p.view_nodes,
                p.roughness_nodes,
                p.moments
            )
            .is_err()
        );
        let mut m = p.moments;
        m[0][0] = 0.6;
        assert!(GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, p.roughness_nodes, m).is_err());
    }
    #[test]
    fn overlap_and_capacity_reject_before_upload() {
        let p = region([0.2, 0.2, 0.8, 0.8]);
        assert!(matches!(
            pack_atlas(&[p.clone(), p], 1024),
            Err(GgxEnergyPatchError::Overlap { .. })
        ));
        assert!(pack_atlas(&[], 1024).is_err());
        assert!(pack_atlas(&[region([0.2, 0.2, 0.8, 0.8])], 8).is_err());
    }
    #[test]
    fn canonical_rows_preserve_bits_and_touching_bounds() {
        let a = region([0.2, 0.2, 0.4, 0.8]);
        let b = region([0.4, 0.2, 0.8, 0.8]);
        let (height, pixels) = pack_atlas(&[b.clone(), a.clone()], 1024).unwrap();
        assert_eq!(height, 18);
        assert_eq!(pixels[16], a.bounds);
        assert_eq!(pixels[9 * 16], b.bounds);
        assert_eq!(
            pixels[3 * 16].map(f32::to_bits),
            a.moments[0].map(f32::to_bits)
        );
        assert_eq!(pixels.len(), height as usize * 16);
    }
    fn search(pixels: &[[f32; 4]], uv: [f32; 2]) -> (Option<usize>, usize) {
        let start = pixels[1][0] as usize;
        let count = pixels[1][1] as usize;
        let (mut index, mut visited, mut best) = (0, 0, None);
        while index < count {
            visited += 1;
            let row = start + index / 3;
            let x = 2 * (index % 3);
            let bounds = pixels[row * 16 + x];
            let links = pixels[row * 16 + x + 1];
            assert!(links[0] as usize > index && links[0] as usize <= count);
            let hit = uv[0] >= bounds[0]
                && uv[0] <= bounds[2]
                && uv[1] >= bounds[1]
                && uv[1] <= bounds[3];
            if !hit {
                index = links[0] as usize;
            } else if links[2] == 1. {
                let id = links[1] as usize;
                best = Some(best.map_or(id, |old: usize| old.min(id)));
                index = links[0] as usize;
            } else {
                index += 1;
            }
        }
        (best, visited)
    }
    #[test]
    fn index_keeps_closed_boundary_ties_and_holes() {
        let a = region([0.2, 0.2, 0.4, 0.8]);
        let b = region([0.4, 0.2, 0.8, 0.8]);
        let (_, pixels) = pack_atlas(&[b.clone(), a.clone()], 1024).unwrap();
        let (_, reversed) = pack_atlas(&[a, b], 1024).unwrap();
        assert_eq!(pixels, reversed);
        assert_eq!(search(&pixels, [0.4, 0.4]).0, Some(0));
        assert_eq!(search(&pixels, [0.8, 0.8]).0, Some(1));
        assert_eq!(search(&pixels, [0.1, 0.1]).0, None);
    }
    #[test]
    fn balanced_index_prunes_large_atlas() {
        let mut regions = Vec::new();
        for y in 0..32 {
            for x in 0..32 {
                let coordinate = |i| 0.2 + 0.6 * i as f32 / 32.;
                regions.push(region([
                    coordinate(x),
                    coordinate(y),
                    coordinate(x + 1),
                    coordinate(y + 1),
                ]));
            }
        }
        let (_, pixels) = pack_atlas(&regions, 16384).unwrap();
        for y in [0, 7, 31] {
            for x in [0, 9, 31] {
                let uv = [
                    0.2 + 0.6 * (x as f32 + 0.5) / 32.,
                    0.2 + 0.6 * (y as f32 + 0.5) / 32.,
                ];
                let (id, visited) = search(&pixels, uv);
                assert_eq!(id, Some(x * 32 + y));
                assert!(visited <= 21, "visited {visited} of 2047 nodes");
            }
        }
    }
}
