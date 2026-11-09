use std::collections::VecDeque;

use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, CancelToken, PADDED_EDGE, PADDED_VOLUME};

use crate::padded::{PaddedCells, fill_face_halo};
use crate::types::{
    EDGE, FaceNeighbor, LightVolume, LightingBudget, LightingError, LightingInput, MAX_LIGHT,
};

pub(crate) const STRIDE_Z: usize = PADDED_EDGE;
pub(crate) const STRIDE_Y: usize = PADDED_EDGE * PADDED_EDGE;

/// Compile-time boundary capability table for each cell in the `34^3` padded volume.
/// Eliminates integer divisions (`/ 34` and `% 34`) from `neighbor_candidates`.
static BOUNDARY_FLAGS: [u8; PADDED_VOLUME] = {
    let mut table = [0_u8; PADDED_VOLUME];
    let mut y = 0;
    while y < PADDED_EDGE {
        let mut z = 0;
        while z < PADDED_EDGE {
            let mut x = 0;
            while x < PADDED_EDGE {
                let mut flags = 0_u8;
                if y > 0 {
                    flags |= 1 << 0;
                }
                if y < PADDED_EDGE - 1 {
                    flags |= 1 << 1;
                }
                if x > 0 {
                    flags |= 1 << 2;
                }
                if x < PADDED_EDGE - 1 {
                    flags |= 1 << 3;
                }
                if z > 0 {
                    flags |= 1 << 4;
                }
                if z < PADDED_EDGE - 1 {
                    flags |= 1 << 5;
                }
                table[(y * PADDED_EDGE + z) * PADDED_EDGE + x] = flags;
                x += 1;
            }
            z += 1;
        }
        y += 1;
    }
    table
};

/// Builds a center-chunk light volume from canonical data and a one-cell face halo.
///
/// Direct skylight travels downward without attenuation; all lateral/upward skylight and emitted
/// block light attenuate by one per voxel. Opaque cells neither receive nor transmit light.
///
/// # Errors
///
/// Returns cancellation, invalid budget, unknown block, inconsistent stamp, or propagation budget
/// failures without publishing a partial volume.
pub fn build_light(
    input: &LightingInput,
    budget: LightingBudget,
    cancel: &CancelToken,
) -> Result<LightVolume, LightingError> {
    validate(input, budget)?;
    if cancel.is_cancelled() {
        return Err(LightingError::Cancelled);
    }
    let mut cells = PaddedCells::new();
    let registry = &input.registry;
    if let voxy_world::PalettedBlocks::Uniform(block) = &input.center.data.blocks {
        cells.fill_center_uniform(registry, *block)?;
    } else {
        cells.fill_center_paletted(registry, &input.center.data.blocks)?;
    }
    for face in FaceNeighbor::ALL {
        let Some(neighbor) = &input.neighbors[face.index()] else {
            continue;
        };
        fill_face_halo(&mut cells, registry, neighbor, face)?;
    }

    let mut sky = vec![0_u8; PADDED_VOLUME];
    if input.sky_from_above {
        seed_direct_sky(&cells.opaque, &mut sky);
    }
    propagate(&cells.opaque, &mut sky, budget, cancel, true)?;
    let mut block = cells.emission;
    if cells.has_emission {
        propagate(&cells.opaque, &mut block, budget, cancel, false)?;
    }

    let mut packed = vec![0_u8; CHUNK_VOLUME];
    for y in 0..EDGE {
        let py = (y + 1) * PADDED_EDGE;
        for z in 0..EDGE {
            let base_padded = (py + (z + 1)) * PADDED_EDGE + 1;
            let base_local = EDGE * (z + EDGE * y);
            for x in 0..EDGE {
                let padded = base_padded + x;
                packed[base_local + x] = (sky[padded] << 4) | block[padded];
            }
        }
    }
    Ok(LightVolume {
        packed: packed.into_boxed_slice(),
        stamp: input.stamp.clone(),
    })
}

pub(crate) fn validate(input: &LightingInput, budget: LightingBudget) -> Result<(), LightingError> {
    if budget.max_propagation_steps == 0 {
        return Err(LightingError::InvalidBudget);
    }
    if input.stamp.center != input.center.revision {
        return Err(LightingError::StampMismatch);
    }
    for face in FaceNeighbor::ALL {
        let actual = input.neighbors[face.index()]
            .as_ref()
            .map(|snapshot| snapshot.revision);
        if input.stamp.faces[face.index()] != actual {
            return Err(LightingError::StampMismatch);
        }
    }
    Ok(())
}

pub(crate) fn seed_direct_sky(opaque: &[bool], sky: &mut [u8]) {
    for z in 0..CHUNK_EDGE {
        for x in 0..CHUNK_EDGE {
            let mut level = MAX_LIGHT;
            for y in (0..CHUNK_EDGE).rev() {
                let index = padded_index([x, y, z]);
                if opaque[index] {
                    level = 0;
                } else {
                    sky[index] = level;
                }
            }
        }
    }
}

/// Face neighbors of a padded cell in relaxation order with the light each would receive.
///
/// Direct skylight keeps level 15 travelling down; everything else attenuates by one.
#[inline]
fn neighbor_candidates(
    index: usize,
    level: u8,
    preserve_direct_down: bool,
) -> [(Option<usize>, u8); 6] {
    let attenuated = level.saturating_sub(1);
    let down = if preserve_direct_down && level == MAX_LIGHT {
        MAX_LIGHT
    } else {
        attenuated
    };
    let flags = BOUNDARY_FLAGS[index];
    [
        (((flags & (1 << 0)) != 0).then(|| index - STRIDE_Y), down),
        (
            ((flags & (1 << 1)) != 0).then(|| index + STRIDE_Y),
            attenuated,
        ),
        (((flags & (1 << 2)) != 0).then(|| index - 1), attenuated),
        (((flags & (1 << 3)) != 0).then(|| index + 1), attenuated),
        (
            ((flags & (1 << 4)) != 0).then(|| index - STRIDE_Z),
            attenuated,
        ),
        (
            ((flags & (1 << 5)) != 0).then(|| index + STRIDE_Z),
            attenuated,
        ),
    ]
}

#[inline]
fn can_raise_neighbor(
    opaque: &[bool],
    light: &[u8],
    index: usize,
    preserve_direct_down: bool,
) -> bool {
    neighbor_candidates(index, light[index], preserve_direct_down)
        .into_iter()
        .any(|(neighbor, candidate)| {
            neighbor.is_some_and(|neighbor| !opaque[neighbor] && candidate > light[neighbor])
        })
}

fn propagate(
    opaque: &[bool],
    light: &mut [u8],
    budget: LightingBudget,
    cancel: &CancelToken,
    preserve_direct_down: bool,
) -> Result<(), LightingError> {
    // Only cells that can still raise a neighbor enter the queue. A lit cell whose
    // neighbors are already at least as bright would be popped and discarded, so it
    // is charged to the step budget directly instead; the queue order and therefore
    // the number of effective steps are unchanged.
    let frontier = frontier_flags(opaque, light, preserve_direct_down);
    let mut queue: VecDeque<u16> = VecDeque::new();
    let mut steps = 0_usize;
    for (index, (&level, &flag)) in light.iter().zip(&frontier).enumerate() {
        if level <= 1 {
            continue;
        }
        if flag != 0 {
            queue.push_back(index as u16);
        } else {
            steps += 1;
        }
    }
    if steps > budget.max_propagation_steps {
        return Err(LightingError::BudgetExceeded);
    }
    while let Some(index_u16) = queue.pop_front() {
        let index = usize::from(index_u16);
        steps = steps.checked_add(1).ok_or(LightingError::BudgetExceeded)?;
        if steps > budget.max_propagation_steps {
            return Err(LightingError::BudgetExceeded);
        }
        if steps.is_multiple_of(1024) && cancel.is_cancelled() {
            return Err(LightingError::Cancelled);
        }
        for (neighbor, candidate) in neighbor_candidates(index, light[index], preserve_direct_down)
        {
            let Some(neighbor) = neighbor else {
                continue;
            };
            // The level test fails for most already-lit neighbors, so it goes first and
            // the opaque flag is only read when a raise is actually possible.
            if candidate > light[neighbor] && !opaque[neighbor] {
                light[neighbor] = candidate;
                if candidate > 1 {
                    queue.push_back(neighbor as u16);
                }
            }
        }
    }
    Ok(())
}

/// One flag per padded cell: nonzero when the cell can raise a face neighbor now.
fn frontier_flags(opaque: &[bool], light: &[u8], preserve_direct_down: bool) -> Vec<u8> {
    if light.iter().all(|&level| level == 0 || level == MAX_LIGHT) {
        return binary_frontier(opaque, light);
    }
    (0..PADDED_VOLUME)
        .map(|index| {
            u8::from(
                light[index] > 1 && can_raise_neighbor(opaque, light, index, preserve_direct_down),
            )
        })
        .collect()
}

/// Frontier flags when every lit cell holds `MAX_LIGHT`, as after direct sky seeding.
///
/// Any candidate a lit cell offers exceeds an unlit level, so a lit cell is a frontier
/// cell exactly when some face neighbor is non-opaque and unlit. The neighbor test is
/// expressed as shifted slice ORs so it vectorizes instead of decoding coordinates per cell.
fn binary_frontier(opaque: &[bool], light: &[u8]) -> Vec<u8> {
    let unlit: Vec<u8> = opaque
        .iter()
        .zip(light)
        .map(|(&blocked, &level)| u8::from(!blocked && level == 0))
        .collect();
    let mut reach = vec![0_u8; PADDED_VOLUME];
    or_shifted(&mut reach, &unlit, STRIDE_Y);
    for slab in 0..PADDED_EDGE {
        let range = slab * STRIDE_Y..(slab + 1) * STRIDE_Y;
        or_shifted(&mut reach[range.clone()], &unlit[range], STRIDE_Z);
    }
    for row in 0..PADDED_EDGE * PADDED_EDGE {
        let range = row * PADDED_EDGE..(row + 1) * PADDED_EDGE;
        or_shifted(&mut reach[range.clone()], &unlit[range], 1);
    }
    for (flag, &level) in reach.iter_mut().zip(light) {
        *flag &= u8::from(level == MAX_LIGHT);
    }
    reach
}

/// Marks each cell whose neighbor at `+shift` or `-shift` inside the block is set.
fn or_shifted(target: &mut [u8], source: &[u8], shift: usize) {
    let len = target.len();
    for (flag, &neighbor) in target[..len - shift].iter_mut().zip(&source[shift..]) {
        *flag |= neighbor;
    }
    for (flag, &neighbor) in target[shift..].iter_mut().zip(&source[..len - shift]) {
        *flag |= neighbor;
    }
}

#[inline]
pub(crate) fn padded_index(pos: [i64; 3]) -> usize {
    let [x, y, z] = pos.map(|value| usize::try_from(value + 1).unwrap_or(0));
    voxy_core::padded_index(x, y, z)
}
