use std::cell::RefCell;
use std::collections::VecDeque;

use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, CancelToken, PADDED_EDGE, PADDED_VOLUME};
use voxy_world::BlockStateId;

use crate::padded::{PaddedCells, fill_face_halo};
use crate::types::{
    EDGE, FaceNeighbor, LightVolume, LightingBudget, LightingError, LightingInput, MAX_LIGHT,
};

/// Reusable per-worker scratch buffers for [`build_light_with_scratch`], eliminating
/// `34^3` padded field (`sky`, `block`, `frontier`, `unlit`), palette, and BFS queue
/// heap allocations across chunk derivations while preserving exact BFS step budgets.
#[derive(Debug)]
pub struct LightingScratch {
    cells: PaddedCells,
    sky: Vec<u8>,
    frontier: Vec<u8>,
    unlit: Vec<u8>,
    queue: VecDeque<u16>,
    palette: Vec<BlockStateId>,
    slots: Vec<u16>,
    classes: Vec<(bool, u8)>,
}

impl Default for LightingScratch {
    fn default() -> Self {
        Self::new()
    }
}

impl LightingScratch {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: PaddedCells::new(),
            sky: vec![0_u8; PADDED_VOLUME],
            frontier: vec![0_u8; PADDED_VOLUME],
            unlit: vec![0_u8; PADDED_VOLUME],
            queue: VecDeque::with_capacity(4096),
            palette: Vec::new(),
            slots: Vec::new(),
            classes: Vec::new(),
        }
    }
}

thread_local! {
    static THREAD_LIGHT_SCRATCH: RefCell<LightingScratch> = RefCell::new(LightingScratch::new());
}

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
    THREAD_LIGHT_SCRATCH
        .with_borrow_mut(|scratch| build_light_with_scratch(input, budget, cancel, scratch))
}

/// Builds a center-chunk light volume reusing caller-supplied [`LightingScratch`] buffers.
///
/// Produces bit-exact output and step-budget accounting identical to [`build_light`].
///
/// # Errors
///
/// Returns cancellation, invalid budget, unknown block, inconsistent stamp, or propagation budget
/// failures without publishing a partial volume.
pub fn build_light_with_scratch(
    input: &LightingInput,
    budget: LightingBudget,
    cancel: &CancelToken,
    scratch: &mut LightingScratch,
) -> Result<LightVolume, LightingError> {
    validate(input, budget)?;
    if cancel.is_cancelled() {
        return Err(LightingError::Cancelled);
    }
    scratch.cells.reset();
    let registry = &input.registry;
    if let voxy_world::PalettedBlocks::Uniform(block) = &input.center.data.blocks {
        scratch.cells.fill_center_uniform(registry, *block)?;
    } else {
        scratch.cells.fill_center_paletted(
            registry,
            &input.center.data.blocks,
            &mut scratch.palette,
            &mut scratch.slots,
            &mut scratch.classes,
        )?;
    }
    for face in FaceNeighbor::ALL {
        let Some(neighbor) = &input.neighbors[face.index()] else {
            continue;
        };
        fill_face_halo(&mut scratch.cells, registry, neighbor, face)?;
    }

    scratch.sky.fill(0);
    if input.sky_from_above {
        let check_top_halo = input.neighbors[FaceNeighbor::PosY.index()].is_some();
        seed_direct_sky(&scratch.cells.opaque, &mut scratch.sky, check_top_halo);
    }
    propagate(
        &scratch.cells.opaque,
        &mut scratch.sky,
        &mut scratch.frontier,
        &mut scratch.unlit,
        &mut scratch.queue,
        budget,
        cancel,
        true,
    )?;
    if scratch.cells.has_emission {
        propagate(
            &scratch.cells.opaque,
            &mut scratch.cells.emission,
            &mut scratch.frontier,
            &mut scratch.unlit,
            &mut scratch.queue,
            budget,
            cancel,
            false,
        )?;
    }

    let mut packed = vec![0_u8; CHUNK_VOLUME];
    for y in 0..EDGE {
        let py = (y + 1) * PADDED_EDGE;
        for z in 0..EDGE {
            let base_padded = (py + (z + 1)) * PADDED_EDGE + 1;
            let base_local = EDGE * (z + EDGE * y);
            let sky_row = &scratch.sky[base_padded..base_padded + EDGE];
            let block_row = &scratch.cells.emission[base_padded..base_padded + EDGE];
            let out_row = &mut packed[base_local..base_local + EDGE];
            for x in 0..EDGE {
                out_row[x] = (sky_row[x] << 4) | block_row[x];
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

pub(crate) fn seed_direct_sky(opaque: &[bool], sky: &mut [u8], check_top_halo: bool) {
    for z in 0..CHUNK_EDGE {
        for x in 0..CHUNK_EDGE {
            let mut level = if check_top_halo && opaque[padded_index([x, CHUNK_EDGE, z])] {
                0
            } else {
                MAX_LIGHT
            };
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

/// Propagates previously solved face-neighbor [`LightVolume`] boundary slices across chunk seams
/// by seeding the 6 face halos of `scratch` and running bounded BFS relaxation.
///
/// Call this after single-chunk [`build_light_with_scratch`] passes when neighbor light volumes
/// are available (e.g. vertical skylight shafts below `y == 0` or cross-chunk lamps up to 15 voxels
/// away from a chunk seam).
///
/// # Errors
///
/// Returns cancellation, invalid budget, unknown block, inconsistent stamp, or propagation budget
/// failures without publishing a partial volume.
pub fn build_light_with_neighbor_volumes(
    input: &LightingInput,
    neighbor_lights: &[Option<&LightVolume>; 6],
    budget: LightingBudget,
    cancel: &CancelToken,
    scratch: &mut LightingScratch,
) -> Result<LightVolume, LightingError> {
    if neighbor_lights.iter().all(Option::is_none) {
        return build_light_with_scratch(input, budget, cancel, scratch);
    }
    validate(input, budget)?;
    if cancel.is_cancelled() {
        return Err(LightingError::Cancelled);
    }
    scratch.cells.reset();
    let registry = &input.registry;
    if let voxy_world::PalettedBlocks::Uniform(block) = &input.center.data.blocks {
        scratch.cells.fill_center_uniform(registry, *block)?;
    } else {
        scratch.cells.fill_center_paletted(
            registry,
            &input.center.data.blocks,
            &mut scratch.palette,
            &mut scratch.slots,
            &mut scratch.classes,
        )?;
    }
    for face in FaceNeighbor::ALL {
        if let Some(neighbor) = &input.neighbors[face.index()] {
            fill_face_halo(&mut scratch.cells, registry, neighbor, face)?;
        }
    }

    scratch.sky.fill(0);
    if input.sky_from_above {
        let check_top_halo = input.neighbors[FaceNeighbor::PosY.index()].is_some();
        seed_direct_sky(&scratch.cells.opaque, &mut scratch.sky, check_top_halo);
    }
    let mut has_halo_block_light = false;
    for face in FaceNeighbor::ALL {
        let Some(vol) = neighbor_lights[face.index()] else {
            continue;
        };
        let (fixed_local, fixed_padded) = match face {
            FaceNeighbor::NegX | FaceNeighbor::NegY | FaceNeighbor::NegZ => (EDGE - 1, 0),
            FaceNeighbor::PosX | FaceNeighbor::PosY | FaceNeighbor::PosZ => (0, PADDED_EDGE - 1),
        };
        for a in 0..EDGE {
            for b in 0..EDGE {
                let (nx, ny, nz, px, py, pz) = match face {
                    FaceNeighbor::NegX | FaceNeighbor::PosX => {
                        (fixed_local, a, b, fixed_padded, a + 1, b + 1)
                    }
                    FaceNeighbor::NegY | FaceNeighbor::PosY => {
                        (b, fixed_local, a, b + 1, fixed_padded, a + 1)
                    }
                    FaceNeighbor::NegZ | FaceNeighbor::PosZ => {
                        (b, a, fixed_local, b + 1, a + 1, fixed_padded)
                    }
                };
                let padded_idx = (py * PADDED_EDGE + pz) * PADDED_EDGE + px;
                // Even if neighbor wasn't provided in `input.neighbors`, allow light halo if non-opaque:
                if input.neighbors[face.index()].is_none() {
                    scratch.cells.opaque[padded_idx] = false;
                }
                if scratch.cells.opaque[padded_idx] {
                    continue;
                }
                let local_byte = vol.bytes()[nx + EDGE * (nz + EDGE * ny)];
                let n_sky = local_byte >> 4;
                let n_block = local_byte & 0x0f;
                if n_sky > scratch.sky[padded_idx] {
                    scratch.sky[padded_idx] = n_sky;
                }
                if n_block > scratch.cells.emission[padded_idx] {
                    scratch.cells.emission[padded_idx] = n_block;
                    if n_block > 1 {
                        has_halo_block_light = true;
                    }
                }
            }
        }
    }

    propagate(
        &scratch.cells.opaque,
        &mut scratch.sky,
        &mut scratch.frontier,
        &mut scratch.unlit,
        &mut scratch.queue,
        budget,
        cancel,
        true,
    )?;
    if scratch.cells.has_emission || has_halo_block_light {
        propagate(
            &scratch.cells.opaque,
            &mut scratch.cells.emission,
            &mut scratch.frontier,
            &mut scratch.unlit,
            &mut scratch.queue,
            budget,
            cancel,
            false,
        )?;
    }

    let mut packed = vec![0_u8; CHUNK_VOLUME];
    for y in 0..EDGE {
        let py = (y + 1) * PADDED_EDGE;
        for z in 0..EDGE {
            let base_padded = (py + (z + 1)) * PADDED_EDGE + 1;
            let base_local = EDGE * (z + EDGE * y);
            let sky_row = &scratch.sky[base_padded..base_padded + EDGE];
            let block_row = &scratch.cells.emission[base_padded..base_padded + EDGE];
            let out_row = &mut packed[base_local..base_local + EDGE];
            for x in 0..EDGE {
                out_row[x] = (sky_row[x] << 4) | block_row[x];
            }
        }
    }
    Ok(LightVolume {
        packed: packed.into_boxed_slice(),
        stamp: input.stamp.clone(),
    })
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
    frontier: &mut [u8],
    unlit: &mut [u8],
    queue: &mut VecDeque<u16>,
    budget: LightingBudget,
    cancel: &CancelToken,
    preserve_direct_down: bool,
) -> Result<(), LightingError> {
    // Only cells that can still raise a neighbor enter the queue. A lit cell whose
    // neighbors are already at least as bright would be popped and discarded, so it
    // is charged to the step budget directly instead; the queue order and therefore
    // the number of effective steps are unchanged.
    frontier_flags_into(opaque, light, preserve_direct_down, frontier, unlit);
    queue.clear();
    let mut steps = 0_usize;
    for (index, (&level, &flag)) in light.iter().zip(frontier.iter()).enumerate() {
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
fn frontier_flags_into(
    opaque: &[bool],
    light: &[u8],
    preserve_direct_down: bool,
    frontier: &mut [u8],
    unlit: &mut [u8],
) {
    if light.iter().all(|&level| level == 0 || level == MAX_LIGHT) {
        binary_frontier_into(opaque, light, frontier, unlit);
        return;
    }
    for (index, flag) in frontier.iter_mut().enumerate() {
        *flag = u8::from(
            light[index] > 1 && can_raise_neighbor(opaque, light, index, preserve_direct_down),
        );
    }
}

/// Frontier flags when every lit cell holds `MAX_LIGHT`, as after direct sky seeding.
///
/// Any candidate a lit cell offers exceeds an unlit level, so a lit cell is a frontier
/// cell exactly when some face neighbor is non-opaque and unlit. The neighbor test is
/// expressed as shifted slice ORs so it vectorizes instead of decoding coordinates per cell.
fn binary_frontier_into(opaque: &[bool], light: &[u8], reach: &mut [u8], unlit: &mut [u8]) {
    for ((out, &blocked), &level) in unlit.iter_mut().zip(opaque).zip(light) {
        *out = u8::from(!blocked && level == 0);
    }
    reach.fill(0);
    or_shifted(reach, unlit, STRIDE_Y);
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
