use std::fmt;
use std::num::NonZeroU8;
use std::sync::Arc;

use voxy_core::{CHUNK_VOLUME, CancelToken, ChunkPos, LocalIndex, LocalPos};
use voxy_world::{
    BlockRegistry, BlockStateId, ChunkRevision, ChunkSnapshot, MaterialId, Occlusion, RenderKind,
    Sample,
};

pub const MAX_DIRECTED_FACES: usize = 6 * CHUNK_VOLUME;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FaceDir {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl FaceDir {
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    const fn material_index(self) -> usize {
        match self {
            Self::NegX => 0,
            Self::PosX => 1,
            Self::NegY => 2,
            Self::PosY => 3,
            Self::NegZ => 4,
            Self::PosZ => 5,
        }
    }

    const fn normal(self) -> [i16; 3] {
        match self {
            Self::NegX => [-1, 0, 0],
            Self::PosX => [1, 0, 0],
            Self::NegY => [0, -1, 0],
            Self::PosY => [0, 1, 0],
            Self::NegZ => [0, 0, -1],
            Self::PosZ => [0, 0, 1],
        }
    }

    const fn axes(self) -> (usize, usize, usize) {
        match self {
            Self::NegX | Self::PosX => (0, 2, 1),
            Self::NegY | Self::PosY => (1, 0, 2),
            Self::NegZ | Self::PosZ => (2, 0, 1),
        }
    }

    const fn positive(self) -> bool {
        matches!(self, Self::PosX | Self::PosY | Self::PosZ)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderLayer {
    Opaque,
    Cutout,
    Translucent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuadDiagonal {
    Uv,
    Vu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quad {
    pub origin: [u8; 3],
    pub extent_u: NonZeroU8,
    pub extent_v: NonZeroU8,
    pub face: FaceDir,
    pub material: MaterialId,
    pub layer: RenderLayer,
    pub ao: [u8; 4],
    pub diagonal: QuadDiagonal,
}

impl Quad {
    #[must_use]
    pub fn area(self) -> u16 {
        u16::from(self.extent_u.get()) * u16::from(self.extent_v.get())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalAabb {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl LocalAabb {
    pub const EMPTY: Self = Self {
        min: [0; 3],
        max: [0; 3],
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkMesh {
    pub opaque: Box<[Quad]>,
    pub cutout: Box<[Quad]>,
    pub translucent: Box<[Quad]>,
    pub bounds: LocalAabb,
}

impl ChunkMesh {
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.opaque.len() + self.cutout.len() + self.translucent.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HaloOffset {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

pub const HALO_OFFSETS: [HaloOffset; 26] = [
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: 1,
    },
];

impl HaloOffset {
    /// Position in [`HALO_OFFSETS`]: `dx` varies fastest, then `dz`, then `dy`, with the
    /// center excluded. Pinned by `halo_order_is_unique_and_canonical`.
    #[must_use]
    pub fn index(self) -> Option<usize> {
        let component = |value: i8| usize::try_from(value + 1).ok().filter(|v| *v < 3);
        let raw = component(self.dx)? + 3 * component(self.dz)? + 9 * component(self.dy)?;
        match raw.cmp(&13) {
            std::cmp::Ordering::Less => Some(raw),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(raw - 1),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshStamp {
    pub center: ChunkRevision,
    pub halo: [Option<ChunkRevision>; 26],
    pub registry_epoch: u64,
    pub mesher_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct MeshingInput {
    pub center: ChunkSnapshot,
    pub neighbors: [Option<ChunkSnapshot>; 26],
    pub registry: Arc<BlockRegistry>,
    pub stamp: MeshStamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceDecision {
    Skip,
    Emit {
        material: MaterialId,
        layer: RenderLayer,
    },
}

#[must_use]
pub fn classify_directed_face(
    source: BlockStateId,
    neighbor: &Sample<BlockStateId>,
    face: FaceDir,
    registry: &BlockRegistry,
) -> FaceDecision {
    let Some(source_def) = registry.get(source) else {
        return FaceDecision::Skip;
    };
    let layer = match source_def.render {
        RenderKind::Invisible => return FaceDecision::Skip,
        RenderKind::Opaque => RenderLayer::Opaque,
        RenderKind::Cutout => RenderLayer::Cutout,
        RenderKind::Translucent => RenderLayer::Translucent,
    };
    if let Sample::Loaded(neighbor_id) = *neighbor
        && let Some(neighbor_def) = registry.get(neighbor_id)
    {
        if neighbor_def.occlusion == Occlusion::FullCube {
            return FaceDecision::Skip;
        }
        if source_def.translucent_interface_group.is_some()
            && source_def.translucent_interface_group == neighbor_def.translucent_interface_group
        {
            return FaceDecision::Skip;
        }
    }
    FaceDecision::Emit {
        material: source_def.face_materials[face.material_index()],
        layer,
    }
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
                quads.push(make_quad(local, face, 1, 1, material, layer));
            }
        }
    }
    Ok(finish_mesh(quads))
}

/// Face-classification data for one distinct block of a chunk.
#[derive(Clone, Copy, Debug)]
struct SlotClass {
    /// `None` for unregistered or invisible blocks, which emit no faces.
    layer: Option<RenderLayer>,
    /// Registered at all; unknown neighbors never occlude.
    known: bool,
    full_cube: bool,
    group: Option<voxy_world::InterfaceGroupId>,
    materials: [MaterialId; 6],
}

impl SlotClass {
    fn classify(registry: &BlockRegistry, block: BlockStateId) -> Self {
        let Some(definition) = registry.get(block) else {
            return Self {
                layer: None,
                known: false,
                full_cube: false,
                group: None,
                materials: [MaterialId(0); 6],
            };
        };
        let layer = match definition.render {
            RenderKind::Invisible => None,
            RenderKind::Opaque => Some(RenderLayer::Opaque),
            RenderKind::Cutout => Some(RenderLayer::Cutout),
            RenderKind::Translucent => Some(RenderLayer::Translucent),
        };
        Self {
            layer,
            known: true,
            full_cube: definition.occlusion == Occlusion::FullCube,
            group: definition.translucent_interface_group,
            materials: definition.face_materials,
        }
    }
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
    let (palette, slots) = input.center.data.blocks.palette_indices();
    let classes: Vec<SlotClass> = palette
        .iter()
        .map(|&block| SlotClass::classify(&input.registry, block))
        .collect();
    let rows = uniform_rows(&slots);
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
                boundary_mask(input, &palette, &slots, face, slice, &mut mask)
            } else {
                interior_mask(&classes, &slots, &rows, face, slice, &mut mask)
            };
            if mask_has_any {
                merge_mask(&mut mask, face, slice, &mut quads)?;
            }
        }
    }
    Ok(finish_mesh(quads))
}

type FaceMask = [Option<(MaterialId, RenderLayer)>; 1024];

/// Fills the mask of a chunk-boundary slice by sampling the halo through the oracle classifier.
///
/// A boundary slice faces exactly one halo chunk, resolved once per slice; every cell then
/// reads the mirrored boundary cell of that neighbor or reports it unloaded, exactly as
/// [`sample_relative`] would.
fn boundary_mask(
    input: &MeshingInput,
    palette: &[BlockStateId],
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
    let mut mask_has_any = false;
    for v in 0_u8..32 {
        for u in 0_u8..32 {
            let mut cell = [0_u8; 3];
            cell[normal_axis] = slice;
            cell[u_axis] = u;
            cell[v_axis] = v;
            let cell_index =
                usize::from(cell[0]) + 32 * (usize::from(cell[2]) + 32 * usize::from(cell[1]));
            let source = palette[usize::from(slots[cell_index])];
            if source == BlockStateId::AIR {
                continue;
            }
            let mut local = cell;
            local[normal_axis] = mirrored;
            let sample = match (neighbor, LocalPos::new(local[0], local[1], local[2])) {
                (Some(snapshot), Ok(local)) => {
                    Sample::Loaded(snapshot.data.blocks.get(local.index()))
                }
                _ => Sample::Unloaded { chunk },
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
///
/// Terrain rows are mostly uniform (air above, rock below), so interior slices decide
/// whole rows at once and only resolve mixed rows cell by cell.
fn uniform_rows(slots: &[u16]) -> Vec<Option<u16>> {
    slots
        .chunks_exact(32)
        .map(|row| row.iter().all(|slot| *slot == row[0]).then_some(row[0]))
        .collect()
}

/// Face decision between two cells inside the chunk, mirroring [`classify_directed_face`].
fn interior_face(
    classes: &[SlotClass],
    source_slot: u16,
    neighbor_slot: u16,
    mat_idx: usize,
) -> Option<(MaterialId, RenderLayer)> {
    let source = &classes[usize::from(source_slot)];
    let layer = source.layer?;
    if neighbor_slot == source_slot {
        if source.full_cube || source.group.is_some() {
            return None;
        }
    } else {
        let neighbor = &classes[usize::from(neighbor_slot)];
        if neighbor.known
            && (neighbor.full_cube || (source.group.is_some() && source.group == neighbor.group))
        {
            return None;
        }
    }
    Some((source.materials[mat_idx], layer))
}

/// Fills one mask row from an x-row and its neighbor x-row (both contiguous in `slots`).
fn fill_row(
    classes: &[SlotClass],
    slots: &[u16],
    rows: &[Option<u16>],
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
    rows: &[Option<u16>],
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
            // u runs along z, v along y; both cells sit in the same x-row.
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
            // u runs along x, v along z; the slice is y.
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
            // u runs along x, v along y; the slice is z.
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
    for v in 0_usize..32 {
        for u in 0_usize..32 {
            let Some(key) = mask[v * 32 + u] else {
                continue;
            };
            let width = (u..32)
                .take_while(|&candidate| mask[v * 32 + candidate] == Some(key))
                .count();
            let height = (v..32)
                .take_while(|&row| {
                    (u..u + width).all(|column| mask[row * 32 + column] == Some(key))
                })
                .count();
            for row in v..v + height {
                for column in u..u + width {
                    mask[row * 32 + column] = None;
                }
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

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn sample_relative(input: &MeshingInput, x: i16, y: i16, z: i16) -> Sample<BlockStateId> {
    if (0..32).contains(&x) && (0..32).contains(&y) && (0..32).contains(&z) {
        let local = LocalPos::new(x as u8, y as u8, z as u8).expect("range checked");
        return Sample::Loaded(input.center.data.blocks.get(local.index()));
    }
    let offset = HaloOffset {
        dx: x.div_euclid(32) as i8,
        dy: y.div_euclid(32) as i8,
        dz: z.div_euclid(32) as i8,
    };
    let chunk = ChunkPos {
        x: input.center.pos.x.saturating_add(i64::from(offset.dx)),
        y: input.center.pos.y.saturating_add(i64::from(offset.dy)),
        z: input.center.pos.z.saturating_add(i64::from(offset.dz)),
    };
    let Some(index) = offset.index() else {
        return Sample::Unloaded { chunk };
    };
    let Some(neighbor) = &input.neighbors[index] else {
        return Sample::Unloaded { chunk };
    };
    let local = LocalPos::new(
        x.rem_euclid(32) as u8,
        y.rem_euclid(32) as u8,
        z.rem_euclid(32) as u8,
    )
    .expect("remainder is local");
    Sample::Loaded(neighbor.data.blocks.get(local.index()))
}

fn finish_mesh(quads: Vec<Quad>) -> ChunkMesh {
    let bounds = if quads.is_empty() {
        LocalAabb::EMPTY
    } else {
        let mut min = [u8::MAX; 3];
        let mut max = [0_u8; 3];
        for quad in &quads {
            let (_, u_axis, v_axis) = quad.face.axes();
            let mut quad_max = quad.origin;
            quad_max[u_axis] += quad.extent_u.get();
            quad_max[v_axis] += quad.extent_v.get();
            for axis in 0..3 {
                min[axis] = min[axis].min(quad.origin[axis]);
                max[axis] = max[axis].max(quad_max[axis]);
            }
        }
        LocalAabb { min, max }
    };
    let mut opaque = Vec::new();
    let mut cutout = Vec::new();
    let mut translucent = Vec::new();
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
        bounds,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshError {
    Cancelled,
    OutputLimit,
    Invariant,
}

impl fmt::Display for MeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "meshing failed: {self:?}")
    }
}

impl std::error::Error for MeshError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use voxy_world::{
        BlockDef, ChunkData, CollisionShape, InterfaceGroupId, PalettedBlocks, ResourceKey,
    };

    fn block(
        name: &str,
        render: RenderKind,
        occlusion: Occlusion,
        group: Option<u16>,
        material: u16,
    ) -> BlockDef {
        BlockDef {
            key: ResourceKey::parse(format!("voxy:{name}")).unwrap(),
            render,
            occlusion,
            collision: if occlusion == Occlusion::FullCube {
                CollisionShape::FullCube
            } else {
                CollisionShape::Empty
            },
            face_materials: [MaterialId(material); 6],
            translucent_interface_group: group.map(InterfaceGroupId),
            emission: 0,
            blast_resistance: if render == RenderKind::Invisible {
                0
            } else {
                20
            },
        }
    }

    fn registry() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, None, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, None, 1),
                block(
                    "glass",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(1),
                    2,
                ),
                block(
                    "water",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(2),
                    3,
                ),
            ])
            .unwrap(),
        )
    }

    fn state(registry: &BlockRegistry, name: &str) -> BlockStateId {
        registry
            .find(&ResourceKey::parse(format!("voxy:{name}")).unwrap())
            .unwrap()
    }

    fn snapshot(pos: ChunkPos, blocks: PalettedBlocks) -> ChunkSnapshot {
        ChunkSnapshot {
            pos,
            revision: ChunkRevision::default(),
            data: Arc::new(ChunkData {
                blocks,
                block_data: BTreeMap::new(),
            }),
        }
    }

    fn input(registry: Arc<BlockRegistry>, blocks: PalettedBlocks) -> MeshingInput {
        MeshingInput {
            center: snapshot(ChunkPos { x: 0, y: 0, z: 0 }, blocks),
            neighbors: std::array::from_fn(|_| None),
            registry,
            stamp: MeshStamp {
                center: ChunkRevision::default(),
                halo: [None; 26],
                registry_epoch: 0,
                mesher_epoch: 0,
            },
        }
    }

    fn face_areas(mesh: &ChunkMesh) -> [u32; 6] {
        let mut areas = [0_u32; 6];
        for quad in mesh
            .opaque
            .iter()
            .chain(mesh.cutout.iter())
            .chain(mesh.translucent.iter())
        {
            let index = FaceDir::ALL
                .iter()
                .position(|face| *face == quad.face)
                .unwrap();
            areas[index] += u32::from(quad.area());
        }
        areas
    }

    #[test]
    fn halo_order_is_unique_and_canonical() {
        for (index, offset) in HALO_OFFSETS.into_iter().enumerate() {
            assert_eq!(offset.index(), Some(index));
            assert_ne!((offset.dx, offset.dy, offset.dz), (0, 0, 0));
        }
    }

    #[test]
    fn greedy_matches_naive_coverage_and_merges_full_chunk() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let input = input(registry, PalettedBlocks::uniform(stone));
        let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
        let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(face_areas(&naive), face_areas(&greedy));
        assert_eq!(naive.quad_count(), 6 * 32 * 32);
        assert_eq!(greedy.quad_count(), 6);
        assert_eq!(face_areas(&greedy), [1024; 6]);
    }

    #[test]
    fn one_block_and_empty_chunk_have_canonical_geometry() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        dense[usize::from(LocalPos::new(3, 4, 5).unwrap().index().get())] = stone;
        let one = input(
            Arc::clone(&registry),
            PalettedBlocks::from_dense(dense).unwrap(),
        );
        let one_mesh = build_mesh(&one, &CancelToken::new()).unwrap();
        assert_eq!(one_mesh.quad_count(), 6);
        assert_eq!(
            one_mesh.bounds,
            LocalAabb {
                min: [3, 4, 5],
                max: [4, 5, 6]
            }
        );
        let empty = input(registry, PalettedBlocks::uniform(BlockStateId::AIR));
        let mesh = build_mesh(&empty, &CancelToken::new()).unwrap();
        assert_eq!(mesh.quad_count(), 0);
        assert_eq!(mesh.bounds, LocalAabb::EMPTY);
    }

    #[test]
    fn classifier_obeys_occlusion_and_translucent_groups() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let glass = state(&registry, "glass");
        let water = state(&registry, "water");
        assert_eq!(
            classify_directed_face(glass, &Sample::Loaded(glass), FaceDir::PosX, &registry),
            FaceDecision::Skip
        );
        assert!(matches!(
            classify_directed_face(glass, &Sample::Loaded(water), FaceDir::PosX, &registry),
            FaceDecision::Emit { .. }
        ));
        assert_eq!(
            classify_directed_face(glass, &Sample::Loaded(stone), FaceDir::PosX, &registry),
            FaceDecision::Skip
        );
        assert!(matches!(
            classify_directed_face(
                stone,
                &Sample::Unloaded {
                    chunk: ChunkPos { x: 1, y: 0, z: 0 }
                },
                FaceDir::PosX,
                &registry
            ),
            FaceDecision::Emit { .. }
        ));
    }

    #[test]
    fn loaded_halo_suppresses_shared_boundary() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let mut input = input(Arc::clone(&registry), PalettedBlocks::uniform(stone));
        let offset = HaloOffset {
            dx: 1,
            dy: 0,
            dz: 0,
        };
        input.neighbors[offset.index().unwrap()] = Some(snapshot(
            ChunkPos { x: 1, y: 0, z: 0 },
            PalettedBlocks::uniform(stone),
        ));
        let mesh = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(mesh.quad_count(), 5);
        assert_eq!(face_areas(&mesh), [1024, 0, 1024, 1024, 1024, 1024]);
    }

    #[test]
    fn cancellation_stops_both_meshers() {
        let token = CancelToken::new();
        token.cancel();
        let input = input(registry(), PalettedBlocks::uniform(BlockStateId::AIR));
        assert_eq!(build_naive_mesh(&input, &token), Err(MeshError::Cancelled));
        assert_eq!(build_mesh(&input, &token), Err(MeshError::Cancelled));
    }

    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// Terrain-like chunk: rock below a jittered height, pockets, lamps (opaque but not
    /// occluding), glass and water layers above, and a uniform-rock variant.
    fn terrain_blocks(seed: u64, registry: &BlockRegistry, rock_only: bool) -> PalettedBlocks {
        let stone = state(registry, "stone");
        let glass = state(registry, "glass");
        let water = state(registry, "water");
        let lamp = state(registry, "lamp");
        if rock_only {
            return PalettedBlocks::uniform(stone);
        }
        let mut random = Lcg(seed);
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for z in 0..32_usize {
            for x in 0..32_usize {
                let jitter = usize::try_from(random.next() % 5).unwrap();
                let height = 6 + ((x * 2 + z * 3 + jitter) % 18);
                for y in 0..32_usize {
                    let index = x + 32 * (z + 32 * y);
                    dense[index] = if y < height {
                        match random.next() % 29 {
                            0 => BlockStateId::AIR,
                            1 => lamp,
                            _ => stone,
                        }
                    } else if y < height + 2 && random.next().is_multiple_of(3) {
                        water
                    } else if random.next().is_multiple_of(41) {
                        glass
                    } else {
                        BlockStateId::AIR
                    };
                }
            }
        }
        PalettedBlocks::from_dense(dense).unwrap()
    }

    #[test]
    fn random_terrain_with_halos_matches_naive() {
        let registry = Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, None, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, None, 1),
                block(
                    "glass",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(1),
                    2,
                ),
                block(
                    "water",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(2),
                    3,
                ),
                block("lamp", RenderKind::Opaque, Occlusion::None, None, 4),
            ])
            .unwrap(),
        );
        for (seed, rock_only, with_halo) in [
            (1, false, true),
            (2, false, false),
            (3, true, true),
            (4, true, false),
            (5, false, true),
        ] {
            let mut input = input(
                Arc::clone(&registry),
                terrain_blocks(seed, &registry, rock_only),
            );
            if with_halo {
                for (index, offset) in HALO_OFFSETS.into_iter().enumerate() {
                    if (index + usize::try_from(seed).unwrap()) % 3 == 0 {
                        continue;
                    }
                    input.neighbors[index] = Some(snapshot(
                        ChunkPos {
                            x: i64::from(offset.dx),
                            y: i64::from(offset.dy),
                            z: i64::from(offset.dz),
                        },
                        terrain_blocks(seed + 10 + index as u64, &registry, index % 4 == 0),
                    ));
                }
            }
            let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
            let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
            assert_eq!(face_areas(&naive), face_areas(&greedy), "seed {seed}");
            assert_eq!(naive.bounds, greedy.bounds, "seed {seed}");
            assert_eq!(covered_cells(&naive), covered_cells(&greedy), "seed {seed}");
        }
    }

    /// Every unit face with its material and layer, independent of quad merging.
    fn covered_cells(mesh: &ChunkMesh) -> BTreeMap<([u8; 3], FaceDir), (MaterialId, RenderLayer)> {
        let mut cells = BTreeMap::new();
        for quad in mesh
            .opaque
            .iter()
            .chain(mesh.cutout.iter())
            .chain(mesh.translucent.iter())
        {
            let (_, u_axis, v_axis) = quad.face.axes();
            for du in 0..quad.extent_u.get() {
                for dv in 0..quad.extent_v.get() {
                    let mut cell = quad.origin;
                    cell[u_axis] += du;
                    cell[v_axis] += dv;
                    let previous = cells.insert((cell, quad.face), (quad.material, quad.layer));
                    assert!(previous.is_none(), "overlapping quads at {cell:?}");
                }
            }
        }
        cells
    }

    #[test]
    fn complex_terrain_greedy_matches_naive() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let glass = state(&registry, "glass");
        let water = state(&registry, "water");

        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for y in 0..16 {
            for z in 0..32 {
                for x in 0..32 {
                    let idx = x + 32 * (z + 32 * y);
                    if y < 8 {
                        dense[idx] = stone;
                    } else if (x + z) % 3 == 0 {
                        dense[idx] = glass;
                    } else if (x * z) % 7 == 0 {
                        dense[idx] = water;
                    }
                }
            }
        }
        let input = input(
            Arc::clone(&registry),
            PalettedBlocks::from_dense(dense).unwrap(),
        );
        let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
        let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(face_areas(&naive), face_areas(&greedy));
        assert!(greedy.quad_count() < naive.quad_count());
    }
}
