//! Greedy 2D bitmask quad merging and naive oracle mesh builders.

use std::cell::RefCell;
use std::num::NonZeroU8;

use voxy_core::{CancelToken, ChunkPos, LocalIndex, LocalPos};
use voxy_world::{BlockStateId, MaterialId, RenderKind, Sample};

use crate::classify::{FaceDecision, SlotClass, classify_directed_face, interior_face};
use crate::neighborhood::{HaloOffset, MeshingInput, sample_relative};
use crate::types::{
    ChunkMesh, FaceDir, LocalAabb, MAX_DIRECTED_FACES, MeshError, Quad, QuadDiagonal, RenderLayer,
};

type FaceMask = [Option<(MaterialId, RenderLayer)>; 1024];

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
                quads.push(make_quad(local, face, 1, 1, material, layer));
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
    SCRATCH.with_borrow_mut(|(palette, slots)| {
        input
            .center
            .data
            .blocks
            .palette_indices_into(palette, slots);
        mesh_from_slots(input, cancel, palette, slots)
    })
}

thread_local! {
    static SCRATCH: RefCell<(Vec<BlockStateId>, Vec<u16>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

fn mesh_from_slots(
    input: &MeshingInput,
    cancel: &CancelToken,
    palette: &[BlockStateId],
    slots: &[u16],
) -> Result<ChunkMesh, MeshError> {
    let classes: Vec<SlotClass> = palette
        .iter()
        .map(|&block| SlotClass::classify(&input.registry, block))
        .collect();
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
                boundary_mask(input, palette, &classes, slots, face, slice, &mut mask)
            } else {
                interior_mask(&classes, slots, &rows, face, slice, &mut mask)
            };
            if mask_has_any {
                merge_mask(&mut mask, face, slice, &mut quads)?;
            }
        }
    }
    Ok(finish_mesh(quads))
}

/// Fills the mask of a chunk-boundary slice by sampling the halo through the oracle classifier.
fn boundary_mask(
    input: &MeshingInput,
    palette: &[BlockStateId],
    classes: &[SlotClass],
    slots: &[u16],
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
                        Some((source_class.materials[mat_idx], layer));
                    mask_has_any = true;
                }
            }
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
                mask[usize::from(v) * 32 + usize::from(u)] = Some((material, layer));
                mask_has_any = true;
            }
        }
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
    out: &mut [Option<(MaterialId, RenderLayer)>],
) -> bool {
    if let (Some(source), Some(neighbor)) = (rows[row], rows[neighbor_row]) {
        return match interior_face(classes, source, neighbor, mat_idx) {
            Some(entry) => {
                out.fill(Some(entry));
                true
            }
            None => false,
        };
    }
    let mut any = false;
    for (u, cell) in out.iter_mut().enumerate() {
        if let Some(entry) =
            interior_face(classes, slots[base + u], slots[neighbor_base + u], mat_idx)
        {
            *cell = Some(entry);
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
    face: FaceDir,
    slice: u8,
    mask: &mut FaceMask,
) -> bool {
    let mat_idx = face.material_index();
    let slice = usize::from(slice);
    let neighbor_slice = if face.positive() {
        slice + 1
    } else {
        slice - 1
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
                            slots[base + slice],
                            slots[base + neighbor_slice],
                            mat_idx,
                        ),
                    };
                    if let Some(entry) = decision {
                        mask[v * 32 + u] = Some(entry);
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
                    (slice * 32 + v, 32 * (v + 32 * slice)),
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
                    (v * 32 + slice, 32 * (slice + 32 * v)),
                    (v * 32 + neighbor_slice, 32 * (neighbor_slice + 32 * v)),
                    &mut mask[v * 32..v * 32 + 32],
                );
            }
        }
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
                ao: [3; 4],
                diagonal: QuadDiagonal::Uv,
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
        ao: [3; 4],
        diagonal: QuadDiagonal::Uv,
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
