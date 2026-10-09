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
    #[must_use]
    pub fn index(self) -> Option<usize> {
        HALO_OFFSETS.iter().position(|candidate| *candidate == self)
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

/// Builds a deterministic greedy quad mesh using the same classifier as the oracle.
///
/// # Errors
///
/// Returns cancellation or an output-bound failure.
pub fn build_mesh(input: &MeshingInput, cancel: &CancelToken) -> Result<ChunkMesh, MeshError> {
    if cancel.is_cancelled() {
        return Err(MeshError::Cancelled);
    }
    if let voxy_world::PalettedBlocks::Uniform(block) = &input.center.data.blocks {
        if let Some(def) = input.registry.get(*block) {
            if def.render == RenderKind::Invisible {
                return Ok(finish_mesh(Vec::new()));
            }
        }
    }

    let center_dense = input.center.data.blocks.to_dense();
    let mut quads = Vec::new();
    let mut mask = [None; 32 * 32];

    for face in FaceDir::ALL {
        let (normal_axis, u_axis, v_axis) = face.axes();
        let normal = face.normal();
        let mat_idx = face.material_index();

        for slice in 0_u8..32 {
            if cancel.is_cancelled() {
                return Err(MeshError::Cancelled);
            }

            let mut mask_has_any = false;
            let is_boundary = match face {
                FaceDir::NegX | FaceDir::NegY | FaceDir::NegZ => slice == 0,
                FaceDir::PosX | FaceDir::PosY | FaceDir::PosZ => slice == 31,
            };

            if is_boundary {
                for v in 0_u8..32 {
                    for u in 0_u8..32 {
                        let mut cell = [0_u8; 3];
                        cell[normal_axis] = slice;
                        cell[u_axis] = u;
                        cell[v_axis] = v;
                        let cell_index = usize::from(cell[0])
                            + 32 * (usize::from(cell[2]) + 32 * usize::from(cell[1]));
                        let source = center_dense[cell_index];
                        if source == BlockStateId::AIR {
                            continue;
                        }
                        let nx = i16::from(cell[0]) + normal[0];
                        let ny = i16::from(cell[1]) + normal[1];
                        let nz = i16::from(cell[2]) + normal[2];
                        let neighbor = sample_relative(input, nx, ny, nz);
                        if let FaceDecision::Emit { material, layer } =
                            classify_directed_face(source, &neighbor, face, &input.registry)
                        {
                            mask[usize::from(v) * 32 + usize::from(u)] = Some((material, layer));
                            mask_has_any = true;
                        }
                    }
                }
            } else {
                let neighbor_offset: isize = match face {
                    FaceDir::PosX => 1,
                    FaceDir::NegX => -1,
                    FaceDir::PosY => 1024,
                    FaceDir::NegY => -1024,
                    FaceDir::PosZ => 32,
                    FaceDir::NegZ => -32,
                };

                let mut last_source = BlockStateId::AIR;
                let mut last_source_def = None;
                let mut last_layer = RenderLayer::Opaque;
                let mut last_neighbor = BlockStateId::AIR;
                let mut last_neighbor_def = None;

                for v in 0_u8..32 {
                    let v_idx = usize::from(v);
                    let v_base = match face {
                        FaceDir::NegX | FaceDir::PosX => usize::from(slice) + 1024 * v_idx,
                        FaceDir::NegY | FaceDir::PosY => 32 * v_idx + 1024 * usize::from(slice),
                        FaceDir::NegZ | FaceDir::PosZ => 32 * usize::from(slice) + 1024 * v_idx,
                    };
                    let u_stride = match face {
                        FaceDir::NegX | FaceDir::PosX => 32,
                        FaceDir::NegY | FaceDir::PosY | FaceDir::NegZ | FaceDir::PosZ => 1,
                    };

                    for u in 0_u8..32 {
                        let u_idx = usize::from(u);
                        let cell_index = v_base + u_idx * u_stride;
                        let source = center_dense[cell_index];
                        if source == BlockStateId::AIR {
                            continue;
                        }
                        let (source_def, layer) = if source == last_source {
                            if let Some(def) = last_source_def {
                                (def, last_layer)
                            } else {
                                continue;
                            }
                        } else {
                            last_source = source;
                            let Some(def) = input.registry.get(source) else {
                                last_source_def = None;
                                continue;
                            };
                            let l = match def.render {
                                RenderKind::Invisible => {
                                    last_source_def = None;
                                    continue;
                                }
                                RenderKind::Opaque => RenderLayer::Opaque,
                                RenderKind::Cutout => RenderLayer::Cutout,
                                RenderKind::Translucent => RenderLayer::Translucent,
                            };
                            last_source_def = Some(def);
                            last_layer = l;
                            (def, l)
                        };

                        let n_index = (cell_index as isize + neighbor_offset) as usize;
                        let neighbor_id = center_dense[n_index];

                        if neighbor_id == source {
                            if source_def.occlusion == Occlusion::FullCube
                                || source_def.translucent_interface_group.is_some()
                            {
                                continue;
                            }
                        } else if neighbor_id != BlockStateId::AIR {
                            let neighbor_def = if neighbor_id == last_neighbor {
                                last_neighbor_def
                            } else {
                                last_neighbor = neighbor_id;
                                last_neighbor_def = input.registry.get(neighbor_id);
                                last_neighbor_def
                            };
                            if let Some(neighbor_def) = neighbor_def {
                                if neighbor_def.occlusion == Occlusion::FullCube {
                                    continue;
                                }
                                if source_def.translucent_interface_group.is_some()
                                    && source_def.translucent_interface_group
                                        == neighbor_def.translucent_interface_group
                                {
                                    continue;
                                }
                            }
                        }

                        mask[v_idx * 32 + u_idx] = Some((source_def.face_materials[mat_idx], layer));
                        mask_has_any = true;
                    }
                }
            }

            if mask_has_any {
                merge_mask(&mut mask, face, slice, &mut quads)?;
            }
        }
    }
    Ok(finish_mesh(quads))
}

fn merge_mask(
    mask: &mut [Option<(MaterialId, RenderLayer)>; 1024],
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
            let v_row = v * 32;
            let mut width = 1_usize;
            while u + width < 32 && mask[v_row + u + width] == Some(key) {
                width += 1;
            }
            let mut height = 1_usize;
            'find_height: while v + height < 32 {
                let row_base = (v + height) * 32 + u;
                for col in 0..width {
                    if mask[row_base + col] != Some(key) {
                        break 'find_height;
                    }
                }
                height += 1;
            }
            for row in 0..height {
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
