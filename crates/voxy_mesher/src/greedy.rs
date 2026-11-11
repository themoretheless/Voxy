//! Greedy 2D bitmask quad merging and naive oracle mesh builders.

use std::cell::RefCell;
use std::num::NonZeroU8;

use voxy_core::{CancelToken, ChunkPos, LocalIndex, LocalPos, PADDED_EDGE, PADDED_VOLUME};
use voxy_world::{BlockStateId, MaterialId, Occlusion, RenderKind, Sample};

use crate::classify::{FaceDecision, SlotClass, classify_directed_face, interior_face};
use crate::neighborhood::{HALO_OFFSETS, HaloOffset, MeshingInput, sample_relative};
use crate::types::{
    ChunkMesh, FaceDir, LocalAabb, MAX_DIRECTED_FACES, MeshError, Quad, QuadDiagonal, RenderLayer,
    quad_diagonal_flip_for_ao,
};

type FaceMask = [Option<(MaterialId, RenderLayer, [u8; 4])>; 1024];

#[inline]
const fn vertex_ao_level(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (side1 as u8) - (side2 as u8) - (corner as u8)
    }
}

#[inline]
const fn quad_diagonal_for_ao(ao: [u8; 4]) -> QuadDiagonal {
    if quad_diagonal_flip_for_ao(ao[0], ao[1], ao[2], ao[3]) {
        QuadDiagonal::Vu
    } else {
        QuadDiagonal::Uv
    }
}

#[inline]
fn sample_occluding(input: &MeshingInput, x: i16, y: i16, z: i16) -> bool {
    match sample_relative(input, x, y, z) {
        Sample::Loaded(id) => input
            .registry
            .get(id)
            .is_some_and(|def| def.occlusion == Occlusion::FullCube),
        Sample::Unloaded { .. } | Sample::Unavailable { .. } => false,
    }
}

#[inline]
fn face_ao_from_input(input: &MeshingInput, local: LocalPos, face: FaceDir) -> [u8; 4] {
    let (normal_axis, u_axis, v_axis) = face.axes();
    let mut base = [
        i16::from(local.x()),
        i16::from(local.y()),
        i16::from(local.z()),
    ];
    base[normal_axis] += if face.positive() { 1 } else { -1 };
    let occ = |du: i16, dv: i16| {
        let mut p = base;
        p[u_axis] += du;
        p[v_axis] += dv;
        sample_occluding(input, p[0], p[1], p[2])
    };
    let n_u0 = occ(-1, 0);
    let n_u1 = occ(1, 0);
    let n_v0 = occ(0, -1);
    let n_v1 = occ(0, 1);
    [
        vertex_ao_level(n_u0, n_v0, occ(-1, -1)),
        vertex_ao_level(n_u1, n_v0, occ(1, -1)),
        vertex_ao_level(n_u1, n_v1, occ(1, 1)),
        vertex_ao_level(n_u0, n_v1, occ(-1, 1)),
    ]
}

fn fill_padded_occluding(
    input: &MeshingInput,
    classes: &[SlotClass],
    slots: &[u16],
    padded: &mut [u8],
) {
    padded.fill(0);
    for y in 0..32_usize {
        for z in 0..32_usize {
            let local_base = 32 * (z + 32 * y);
            let pad_base = ((y + 1) * PADDED_EDGE + (z + 1)) * PADDED_EDGE + 1;
            for x in 0..32_usize {
                padded[pad_base + x] =
                    u8::from(classes[usize::from(slots[local_base + x])].full_cube);
            }
        }
    }
    for (halo_idx, offset) in HALO_OFFSETS.iter().enumerate() {
        let Some(neighbor) = &input.neighbors[halo_idx] else {
            continue;
        };
        let (px_range, lx_base) = match offset.dx {
            -1 => (0..1_usize, 31_usize),
            0 => (1..33_usize, 0_usize),
            _ => (33..34_usize, 0_usize),
        };
        let (py_range, ly_base) = match offset.dy {
            -1 => (0..1_usize, 31_usize),
            0 => (1..33_usize, 0_usize),
            _ => (33..34_usize, 0_usize),
        };
        let (pz_range, lz_base) = match offset.dz {
            -1 => (0..1_usize, 31_usize),
            0 => (1..33_usize, 0_usize),
            _ => (33..34_usize, 0_usize),
        };
        if let voxy_world::PalettedBlocks::Uniform(block) = &neighbor.data.blocks {
            let occ = u8::from(
                input
                    .registry
                    .get(*block)
                    .is_some_and(|def| def.occlusion == Occlusion::FullCube),
            );
            if occ == 0 {
                continue;
            }
            for py in py_range {
                for pz in pz_range.clone() {
                    let row = (py * PADDED_EDGE + pz) * PADDED_EDGE;
                    padded[row + px_range.start..row + px_range.end].fill(occ);
                }
            }
            continue;
        }
        for (iy, py) in py_range.enumerate() {
            let ly = (ly_base + iy) as u8;
            for (iz, pz) in pz_range.clone().enumerate() {
                let lz = (lz_base + iz) as u8;
                let row = (py * PADDED_EDGE + pz) * PADDED_EDGE;
                for (ix, px) in px_range.clone().enumerate() {
                    let lx = (lx_base + ix) as u8;
                    let local = LocalPos::new(lx, ly, lz).expect("halo local in 0..32");
                    let id = neighbor.data.blocks.get(local.index());
                    padded[row + px] = u8::from(
                        input
                            .registry
                            .get(id)
                            .is_some_and(|def| def.occlusion == Occlusion::FullCube),
                    );
                }
            }
        }
    }
}

#[inline]
fn face_ao_from_padded(
    padded: &[u8],
    face: FaceDir,
    air_normal_pad: usize,
    u_pad: usize,
    v_pad: usize,
) -> [u8; 4] {
    let (normal_axis, u_axis, v_axis) = face.axes();
    let occ = |pu: usize, pv: usize| {
        let mut p = [0_usize; 3];
        p[normal_axis] = air_normal_pad;
        p[u_axis] = pu;
        p[v_axis] = pv;
        padded[(p[1] * PADDED_EDGE + p[2]) * PADDED_EDGE + p[0]] != 0
    };
    let n_u0 = occ(u_pad - 1, v_pad);
    let n_u1 = occ(u_pad + 1, v_pad);
    let n_v0 = occ(u_pad, v_pad - 1);
    let n_v1 = occ(u_pad, v_pad + 1);
    [
        vertex_ao_level(n_u0, n_v0, occ(u_pad - 1, v_pad - 1)),
        vertex_ao_level(n_u1, n_v0, occ(u_pad + 1, v_pad - 1)),
        vertex_ao_level(n_u1, n_v1, occ(u_pad + 1, v_pad + 1)),
        vertex_ao_level(n_u0, n_v1, occ(u_pad - 1, v_pad + 1)),
    ]
}

/// Builds an unmerged face mesh used as the correctness oracle.
///
/// # Errors
///
/// Returns cancellation or an output-bound failure.
pub fn build_naive_mesh(
    input: &MeshingInput,
    cancel: &CancelToken,
) -> Result<ChunkMesh, MeshError> {
    let mut quads = Vec::new();
    for index in LocalIndex::all() {
        if index.get() % 1024 == 0 && cancel.is_cancelled() {
            return Err(MeshError::Cancelled);
        }
        let local = index.position();
        let source = input.center.data.blocks.get(index);
        for face in FaceDir::ALL {
            let normal = face.normal();
            let neighbor = sample_relative(
                input,
                i16::from(local.x()) + normal[0],
                i16::from(local.y()) + normal[1],
                i16::from(local.z()) + normal[2],
            );
            if let FaceDecision::Emit { material, layer } =
                classify_directed_face(source, &neighbor, face, &input.registry)
            {
                if quads.len() == MAX_DIRECTED_FACES {
                    return Err(MeshError::OutputLimit);
                }
                let ao = face_ao_from_input(input, local, face);
                quads.push(make_quad(local, face, 1, 1, material, layer, ao));
            }
        }
    }
    Ok(finish_mesh(quads))
}

/// Builds a deterministic greedy quad mesh using the same classifier as the oracle.
///
/// # Errors
///
/// Returns cancellation or an output-bound failure.
pub fn build_mesh(input: &MeshingInput, cancel: &CancelToken) -> Result<ChunkMesh, MeshError> {
    if cancel.is_cancelled() {
        return Err(MeshError::Cancelled);
    }
    if let voxy_world::PalettedBlocks::Uniform(block) = &input.center.data.blocks
        && input
            .registry
            .get(*block)
            .is_some_and(|def| def.render == RenderKind::Invisible)
    {
        return Ok(finish_mesh(Vec::new()));
    }

    // Classify each distinct block once; interior slices then only compare slots.
    // Meshing runs as pure jobs on worker threads; each thread keeps its expansion
    // buffers so a chunk does not allocate 64 KiB of slots per build.
    SCRATCH.with_borrow_mut(|(palette, slots, padded_occ)| {
        input
            .center
            .data
            .blocks
            .palette_indices_into(palette, slots);
        if padded_occ.len() != PADDED_VOLUME {
            padded_occ.resize(PADDED_VOLUME, 0);
        }
        mesh_from_slots(input, cancel, palette, slots, padded_occ)
    })
}

thread_local! {
    static SCRATCH: RefCell<(Vec<BlockStateId>, Vec<u16>, Vec<u8>)> =
        const { RefCell::new((Vec::new(), Vec::new(), Vec::new())) };
}

fn mesh_from_slots(
    input: &MeshingInput,
    cancel: &CancelToken,
    palette: &[BlockStateId],
    slots: &[u16],
    padded_occ: &mut [u8],
) -> Result<ChunkMesh, MeshError> {
    let classes: Vec<SlotClass> = palette
        .iter()
        .map(|&block| SlotClass::classify(&input.registry, block))
        .collect();
    fill_padded_occluding(input, &classes, slots, padded_occ);
    let rows = uniform_rows(slots);
    // One distinct block that never faces itself (occluding, grouped or invisible)
    // cannot produce interior faces at all.
    let skip_interior = palette.len() == 1 && interior_face(&classes, 0, 0, 0).is_none();
    let mut quads = Vec::new();
    let mut mask = [None; 32 * 32];

    for face in FaceDir::ALL {
        for slice in 0_u8..32 {
            if cancel.is_cancelled() {
                return Err(MeshError::Cancelled);
            }
            let is_boundary = match face {
                FaceDir::NegX | FaceDir::NegY | FaceDir::NegZ => slice == 0,
                FaceDir::PosX | FaceDir::PosY | FaceDir::PosZ => slice == 31,
            };
            if !is_boundary && skip_interior {
                continue;
            }
            let mask_has_any = if is_boundary {
                boundary_mask(
                    input, palette, &classes, slots, padded_occ, face, slice, &mut mask,
                )
            } else {
                interior_mask(&classes, slots, &rows, padded_occ, face, slice, &mut mask)
            };
            if mask_has_any {
                merge_mask(&mut mask, face, slice, &mut quads)?;
            }
        }
    }
    Ok(finish_mesh(quads))
}

fn populate_mask_ao(padded_occ: &[u8], face: FaceDir, slice: u8, mask: &mut FaceMask) {
    let air_normal_pad = if face.positive() {
        usize::from(slice) + 2
    } else {
        usize::from(slice)
    };
    for v in 0..32_usize {
        let row_base = v * 32;
        for u in 0..32_usize {
            if let Some((mat, layer, ao)) = &mut mask[row_base + u] {
                *ao = face_ao_from_padded(padded_occ, face, air_normal_pad, u + 1, v + 1);
                let _ = (*mat, *layer);
            }
        }
    }
}

/// Fills the mask of a chunk-boundary slice by sampling the halo through the oracle classifier.
fn boundary_mask(
    input: &MeshingInput,
    palette: &[BlockStateId],
    classes: &[SlotClass],
    slots: &[u16],
    padded_occ: &[u8],
    face: FaceDir,
    slice: u8,
    mask: &mut FaceMask,
) -> bool {
    let (normal_axis, u_axis, v_axis) = face.axes();
    let normal = face.normal();
    let offset = HaloOffset {
        dx: i8::try_from(normal[0]).unwrap_or(0),
        dy: i8::try_from(normal[1]).unwrap_or(0),
        dz: i8::try_from(normal[2]).unwrap_or(0),
    };
    let chunk = ChunkPos {
        x: input.center.pos.x.saturating_add(i64::from(offset.dx)),
        y: input.center.pos.y.saturating_add(i64::from(offset.dy)),
        z: input.center.pos.z.saturating_add(i64::from(offset.dz)),
    };
    let neighbor = offset
        .index()
        .and_then(|index| input.neighbors[index].as_ref());
    let mirrored = if face.positive() { 0 } else { 31 };
    let mat_idx = face.material_index();

    // Fast path when neighbor chunk is unloaded: every visible block emits its face.
    if neighbor.is_none() {
        let mut mask_has_any = false;
        for v in 0_u8..32 {
            for u in 0_u8..32 {
                let mut cell = [0_u8; 3];
                cell[normal_axis] = slice;
                cell[u_axis] = u;
                cell[v_axis] = v;
                let cell_index =
                    usize::from(cell[0]) + 32 * (usize::from(cell[2]) + 32 * usize::from(cell[1]));
                let source_class = &classes[usize::from(slots[cell_index])];
                if let Some(layer) = source_class.layer {
                    mask[usize::from(v) * 32 + usize::from(u)] =
                        Some((source_class.materials[mat_idx], layer, [3; 4]));
                    mask_has_any = true;
                }
            }
        }
        if mask_has_any {
            populate_mask_ao(padded_occ, face, slice, mask);
        }
        return mask_has_any;
    }

    let neighbor_snapshot = neighbor.expect("checked Some above");
    // Fast path when neighbor chunk is uniform and full-cube occluding: all boundary faces are culled.
    if let voxy_world::PalettedBlocks::Uniform(n_block) = &neighbor_snapshot.data.blocks {
        let n_class = SlotClass::classify(&input.registry, *n_block);
        if n_class.known && n_class.full_cube {
            return false;
        }
    }

    let mut mask_has_any = false;
    for v in 0_u8..32 {
        for u in 0_u8..32 {
            let mut cell = [0_u8; 3];
            cell[normal_axis] = slice;
            cell[u_axis] = u;
            cell[v_axis] = v;
            let cell_index =
                usize::from(cell[0]) + 32 * (usize::from(cell[2]) + 32 * usize::from(cell[1]));
            let slot = usize::from(slots[cell_index]);
            if classes[slot].layer.is_none() {
                continue;
            }
            let source = palette[slot];
            let mut local = cell;
            local[normal_axis] = mirrored;
            let sample = match LocalPos::new(local[0], local[1], local[2]) {
                Ok(local) => Sample::Loaded(neighbor_snapshot.data.blocks.get(local.index())),
                Err(_) => Sample::Unloaded { chunk },
            };
            if let FaceDecision::Emit { material, layer } =
                classify_directed_face(source, &sample, face, &input.registry)
            {
                mask[usize::from(v) * 32 + usize::from(u)] = Some((material, layer, [3; 4]));
                mask_has_any = true;
            }
        }
    }
    if mask_has_any {
        populate_mask_ao(padded_occ, face, slice, mask);
    }
    mask_has_any
}

/// Slot shared by all 32 cells of an x-row, keyed by `y * 32 + z`, when the row is uniform.
/// Allocated on the stack (`[Option<u16>; 1024]`, 4 KiB) to avoid heap allocation per chunk.
fn uniform_rows(slots: &[u16]) -> [Option<u16>; 1024] {
    let mut out = [None; 1024];
    for (row_idx, row) in slots.chunks_exact(32).enumerate() {
        let first = row[0];
        if row[1..].iter().all(|&slot| slot == first) {
            out[row_idx] = Some(first);
        }
    }
    out
}

/// Fills one mask row from an x-row and its neighbor x-row (both contiguous in `slots`).
#[inline]
fn fill_row(
    classes: &[SlotClass],
    slots: &[u16],
    rows: &[Option<u16>; 1024],
    mat_idx: usize,
    (row, base): (usize, usize),
    (neighbor_row, neighbor_base): (usize, usize),
    out: &mut [Option<(MaterialId, RenderLayer, [u8; 4])>],
) -> bool {
    if let (Some(source), Some(neighbor)) = (rows[row], rows[neighbor_row]) {
        return match interior_face(classes, source, neighbor, mat_idx) {
            Some((mat, layer)) => {
                out.fill(Some((mat, layer, [3; 4])));
                true
            }
            None => false,
        };
    }
    let mut any = false;
    for (u, cell) in out.iter_mut().enumerate() {
        if let Some((mat, layer)) =
            interior_face(classes, slots[base + u], slots[neighbor_base + u], mat_idx)
        {
            *cell = Some((mat, layer, [3; 4]));
            any = true;
        }
    }
    any
}

/// Fills the mask of an interior slice from per-slot classes; both cells are in the chunk.
fn interior_mask(
    classes: &[SlotClass],
    slots: &[u16],
    rows: &[Option<u16>; 1024],
    padded_occ: &[u8],
    face: FaceDir,
    slice: u8,
    mask: &mut FaceMask,
) -> bool {
    let mat_idx = face.material_index();
    let slice_idx = usize::from(slice);
    let neighbor_slice = if face.positive() {
        slice_idx + 1
    } else {
        slice_idx - 1
    };
    let mut mask_has_any = false;
    match face {
        FaceDir::NegX | FaceDir::PosX => {
            for v in 0..32 {
                for u in 0..32 {
                    let base = 32 * (u + 32 * v);
                    let decision = match rows[v * 32 + u] {
                        Some(slot) => interior_face(classes, slot, slot, mat_idx),
                        None => interior_face(
                            classes,
                            slots[base + slice_idx],
                            slots[base + neighbor_slice],
                            mat_idx,
                        ),
                    };
                    if let Some((mat, layer)) = decision {
                        mask[v * 32 + u] = Some((mat, layer, [3; 4]));
                        mask_has_any = true;
                    }
                }
            }
        }
        FaceDir::NegY | FaceDir::PosY => {
            for v in 0..32 {
                mask_has_any |= fill_row(
                    classes,
                    slots,
                    rows,
                    mat_idx,
                    (slice_idx * 32 + v, 32 * (v + 32 * slice_idx)),
                    (neighbor_slice * 32 + v, 32 * (v + 32 * neighbor_slice)),
                    &mut mask[v * 32..v * 32 + 32],
                );
            }
        }
        FaceDir::NegZ | FaceDir::PosZ => {
            for v in 0..32 {
                mask_has_any |= fill_row(
                    classes,
                    slots,
                    rows,
                    mat_idx,
                    (v * 32 + slice_idx, 32 * (slice_idx + 32 * v)),
                    (v * 32 + neighbor_slice, 32 * (neighbor_slice + 32 * v)),
                    &mut mask[v * 32..v * 32 + 32],
                );
            }
        }
    }
    if mask_has_any {
        populate_mask_ao(padded_occ, face, slice, mask);
    }
    mask_has_any
}

fn merge_mask(
    mask: &mut FaceMask,
    face: FaceDir,
    slice: u8,
    output: &mut Vec<Quad>,
) -> Result<(), MeshError> {
    let (normal_axis, u_axis, v_axis) = face.axes();
    let mut active = [0_u32; 32];
    for (v, row_bits) in active.iter_mut().enumerate() {
        let mut bits = 0_u32;
        let row_base = v * 32;
        for u in 0_usize..32 {
            if mask[row_base + u].is_some() {
                bits |= 1_u32 << u;
            }
        }
        *row_bits = bits;
    }

    for v in 0_usize..32 {
        while active[v] != 0 {
            let u = active[v].trailing_zeros() as usize;
            let v_row = v * 32;
            let Some(key) = mask[v_row + u] else {
                active[v] &= !(1_u32 << u);
                continue;
            };
            let mut width = 1_usize;
            while u + width < 32
                && (active[v] & (1_u32 << (u + width))) != 0
                && mask[v_row + u + width] == Some(key)
            {
                width += 1;
            }
            let span_mask = if width == 32 {
                u32::MAX
            } else {
                ((1_u32 << width) - 1) << u
            };
            let mut height = 1_usize;
            'find_height: while v + height < 32 {
                if (active[v + height] & span_mask) != span_mask {
                    break;
                }
                let row_base = (v + height) * 32 + u;
                for col in 0..width {
                    if mask[row_base + col] != Some(key) {
                        break 'find_height;
                    }
                }
                height += 1;
            }
            for row in 0..height {
                active[v + row] &= !span_mask;
                let row_base = (v + row) * 32 + u;
                mask[row_base..row_base + width].fill(None);
            }
            let mut origin = [0_u8; 3];
            origin[normal_axis] = slice + u8::from(face.positive());
            origin[u_axis] = u8::try_from(u).unwrap_or(0);
            origin[v_axis] = u8::try_from(v).unwrap_or(0);
            if output.len() == MAX_DIRECTED_FACES {
                return Err(MeshError::OutputLimit);
            }
            output.push(Quad {
                origin,
                extent_u: NonZeroU8::new(u8::try_from(width).unwrap_or(32)).unwrap(),
                extent_v: NonZeroU8::new(u8::try_from(height).unwrap_or(32)).unwrap(),
                face,
                material: key.0,
                layer: key.1,
                ao: key.2,
                diagonal: quad_diagonal_for_ao(key.2),
            });
        }
    }
    Ok(())
}

#[inline]
fn make_quad(
    local: LocalPos,
    face: FaceDir,
    extent_u: u8,
    extent_v: u8,
    material: MaterialId,
    layer: RenderLayer,
    ao: [u8; 4],
) -> Quad {
    let mut origin = [local.x(), local.y(), local.z()];
    let (normal_axis, _, _) = face.axes();
    origin[normal_axis] += u8::from(face.positive());
    Quad {
        origin,
        extent_u: NonZeroU8::new(extent_u).unwrap(),
        extent_v: NonZeroU8::new(extent_v).unwrap(),
        face,
        material,
        layer,
        ao,
        diagonal: quad_diagonal_for_ao(ao),
    }
}

fn finish_mesh(quads: Vec<Quad>) -> ChunkMesh {
    if quads.is_empty() {
        return ChunkMesh {
            opaque: Box::new([]),
            cutout: Box::new([]),
            translucent: Box::new([]),
            bounds: LocalAabb::EMPTY,
        };
    }
    let mut min = [u8::MAX; 3];
    let mut max = [0_u8; 3];
    let mut opaque_count = 0_usize;
    let mut cutout_count = 0_usize;
    let mut translucent_count = 0_usize;
    for quad in &quads {
        let (_, u_axis, v_axis) = quad.face.axes();
        let mut quad_max = quad.origin;
        quad_max[u_axis] += quad.extent_u.get();
        quad_max[v_axis] += quad.extent_v.get();
        for axis in 0..3 {
            min[axis] = min[axis].min(quad.origin[axis]);
            max[axis] = max[axis].max(quad_max[axis]);
        }
        match quad.layer {
            RenderLayer::Opaque => opaque_count += 1,
            RenderLayer::Cutout => cutout_count += 1,
            RenderLayer::Translucent => translucent_count += 1,
        }
    }
    let mut opaque = Vec::with_capacity(opaque_count);
    let mut cutout = Vec::with_capacity(cutout_count);
    let mut translucent = Vec::with_capacity(translucent_count);
    for quad in quads {
        match quad.layer {
            RenderLayer::Opaque => opaque.push(quad),
            RenderLayer::Cutout => cutout.push(quad),
            RenderLayer::Translucent => translucent.push(quad),
        }
    }
    ChunkMesh {
        opaque: opaque.into_boxed_slice(),
        cutout: cutout.into_boxed_slice(),
        translucent: translucent.into_boxed_slice(),
        bounds: LocalAabb { min, max },
    }
}
