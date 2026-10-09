//! Deterministic derived voxel lighting with revision-stamped inputs.

use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, CancelToken, LocalIndex};
use voxy_world::{BlockRegistry, BlockStateId, ChunkRevision, ChunkSnapshot, Occlusion};

const EDGE: usize = 32;
const PADDED_EDGE: usize = 34;
const PADDED_VOLUME: usize = PADDED_EDGE * PADDED_EDGE * PADDED_EDGE;
const STRIDE_Z: usize = PADDED_EDGE;
const STRIDE_Y: usize = PADDED_EDGE * PADDED_EDGE;
const MAX_LIGHT: u8 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceNeighbor {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl FaceNeighbor {
    const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    const fn index(self) -> usize {
        match self {
            Self::NegX => 0,
            Self::PosX => 1,
            Self::NegY => 2,
            Self::PosY => 3,
            Self::NegZ => 4,
            Self::PosZ => 5,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightStamp {
    pub center: ChunkRevision,
    pub faces: [Option<ChunkRevision>; 6],
    pub registry_epoch: u64,
    pub lighting_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct LightingInput {
    pub center: ChunkSnapshot,
    pub neighbors: [Option<ChunkSnapshot>; 6],
    pub registry: Arc<BlockRegistry>,
    /// Whether unobstructed sky enters the top halo at level 15.
    pub sky_from_above: bool,
    pub stamp: LightStamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightingBudget {
    pub max_propagation_steps: usize,
}

impl Default for LightingBudget {
    fn default() -> Self {
        Self {
            max_propagation_steps: 2_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightVolume {
    packed: Box<[u8]>,
    pub stamp: LightStamp,
}

impl LightVolume {
    #[must_use]
    pub fn packed(&self, index: LocalIndex) -> u8 {
        self.packed[usize::from(index.get())]
    }

    #[must_use]
    pub fn sky(&self, index: LocalIndex) -> u8 {
        self.packed(index) >> 4
    }

    #[must_use]
    pub fn block(&self, index: LocalIndex) -> u8 {
        self.packed(index) & 0x0f
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.packed
    }
}

/// Padded occlusion and emission of the center chunk plus its one-cell face halo.
struct PaddedCells {
    opaque: Vec<bool>,
    emission: Vec<u8>,
    has_emission: bool,
}

impl PaddedCells {
    fn new() -> Self {
        Self {
            opaque: vec![true; PADDED_VOLUME],
            emission: vec![0_u8; PADDED_VOLUME],
            has_emission: false,
        }
    }

    fn fill(
        &mut self,
        registry: &BlockRegistry,
        block: BlockStateId,
        index: usize,
    ) -> Result<(), LightingError> {
        let definition = registry
            .get(block)
            .ok_or(LightingError::UnknownBlock(block.get()))?;
        self.opaque[index] = definition.occlusion == Occlusion::FullCube;
        self.emission[index] = definition.emission;
        if definition.emission > 0 {
            self.has_emission = true;
        }
        Ok(())
    }

    /// Fills the center cells by classifying each distinct block once and expanding the
    /// chunk's palette indices instead of resolving every cell through the registry.
    fn fill_center_paletted(
        &mut self,
        registry: &BlockRegistry,
        blocks: &voxy_world::PalettedBlocks,
    ) -> Result<(), LightingError> {
        let (palette, slots) = blocks.palette_indices();
        let classes = palette
            .iter()
            .map(|&block| {
                registry
                    .get(block)
                    .map(|definition| {
                        (
                            definition.occlusion == Occlusion::FullCube,
                            definition.emission,
                        )
                    })
                    .ok_or(LightingError::UnknownBlock(block.get()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for y in 0..EDGE {
            for z in 0..EDGE {
                let base_local = EDGE * (z + EDGE * y);
                let base_padded = ((y + 1) * PADDED_EDGE + (z + 1)) * PADDED_EDGE + 1;
                for x in 0..EDGE {
                    let (blocked, emission) = classes[usize::from(slots[base_local + x])];
                    self.opaque[base_padded + x] = blocked;
                    self.emission[base_padded + x] = emission;
                }
            }
        }
        if classes.iter().any(|&(_, emission)| emission > 0) {
            self.has_emission = true;
        }
        Ok(())
    }

    /// Fills every center cell from one definition without expanding the chunk.
    fn fill_center_uniform(
        &mut self,
        registry: &BlockRegistry,
        block: BlockStateId,
    ) -> Result<(), LightingError> {
        let definition = registry
            .get(block)
            .ok_or(LightingError::UnknownBlock(block.get()))?;
        let blocked = definition.occlusion == Occlusion::FullCube;
        for y in 0..EDGE {
            for z in 0..EDGE {
                let start = ((y + 1) * PADDED_EDGE + (z + 1)) * PADDED_EDGE + 1;
                self.opaque[start..start + EDGE].fill(blocked);
                self.emission[start..start + EDGE].fill(definition.emission);
            }
        }
        if definition.emission > 0 {
            self.has_emission = true;
        }
        Ok(())
    }
}

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

/// Copies the neighbor's boundary slice into the matching halo face.
fn fill_face_halo(
    cells: &mut PaddedCells,
    registry: &BlockRegistry,
    neighbor: &ChunkSnapshot,
    face: FaceNeighbor,
) -> Result<(), LightingError> {
    let last = EDGE - 1;
    // (neighbor-local fixed coordinate, padded fixed coordinate) along the face axis.
    let (fixed_local, fixed_padded) = match face {
        FaceNeighbor::NegX | FaceNeighbor::NegY | FaceNeighbor::NegZ => (last, 0),
        FaceNeighbor::PosX | FaceNeighbor::PosY | FaceNeighbor::PosZ => (0, PADDED_EDGE - 1),
    };
    for a in 0..EDGE {
        for b in 0..EDGE {
            let (x, y, z) = match face {
                FaceNeighbor::NegX | FaceNeighbor::PosX => (fixed_local, a, b),
                FaceNeighbor::NegY | FaceNeighbor::PosY => (b, fixed_local, a),
                FaceNeighbor::NegZ | FaceNeighbor::PosZ => (b, a, fixed_local),
            };
            let (px, py, pz) = match face {
                FaceNeighbor::NegX | FaceNeighbor::PosX => (fixed_padded, a + 1, b + 1),
                FaceNeighbor::NegY | FaceNeighbor::PosY => (b + 1, fixed_padded, a + 1),
                FaceNeighbor::NegZ | FaceNeighbor::PosZ => (b + 1, a + 1, fixed_padded),
            };
            let Some(local) =
                LocalIndex::new(u16::try_from(x + EDGE * (z + EDGE * y)).unwrap_or(u16::MAX))
            else {
                continue;
            };
            let block = neighbor.data.blocks.get(local);
            cells.fill(registry, block, (py * PADDED_EDGE + pz) * PADDED_EDGE + px)?;
        }
    }
    Ok(())
}

fn validate(input: &LightingInput, budget: LightingBudget) -> Result<(), LightingError> {
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

fn seed_direct_sky(opaque: &[bool], sky: &mut [u8]) {
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
    let x = index % PADDED_EDGE;
    let yz = index / PADDED_EDGE;
    let z = yz % PADDED_EDGE;
    let y = yz / PADDED_EDGE;
    [
        ((y > 0).then(|| index - STRIDE_Y), down),
        ((y < PADDED_EDGE - 1).then(|| index + STRIDE_Y), attenuated),
        ((x > 0).then(|| index - 1), attenuated),
        ((x < PADDED_EDGE - 1).then(|| index + 1), attenuated),
        ((z > 0).then(|| index - STRIDE_Z), attenuated),
        ((z < PADDED_EDGE - 1).then(|| index + STRIDE_Z), attenuated),
    ]
}

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
    let mut queue = VecDeque::new();
    let mut steps = 0_usize;
    for (index, (&level, &flag)) in light.iter().zip(&frontier).enumerate() {
        if level <= 1 {
            continue;
        }
        if flag != 0 {
            queue.push_back(index);
        } else {
            steps += 1;
        }
    }
    if steps > budget.max_propagation_steps {
        return Err(LightingError::BudgetExceeded);
    }
    while let Some(index) = queue.pop_front() {
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
            if !opaque[neighbor] && candidate > light[neighbor] {
                light[neighbor] = candidate;
                if candidate > 1 {
                    queue.push_back(neighbor);
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

fn padded_index(pos: [i64; 3]) -> usize {
    let [x, y, z] = pos.map(|value| usize::try_from(value + 1).unwrap_or(0));
    (y * PADDED_EDGE + z) * PADDED_EDGE + x
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LightingError {
    InvalidBudget,
    StampMismatch,
    UnknownBlock(u32),
    BudgetExceeded,
    Cancelled,
}

impl fmt::Display for LightingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "lighting error: {self:?}")
    }
}

impl std::error::Error for LightingError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use voxy_world::{
        BlockDef, BlockStateId, ChunkData, ChunkPos, CollisionShape, MaterialId, PalettedBlocks,
        RenderKind, ResourceKey,
    };

    use voxy_core::LocalPos;

    use super::*;

    fn registry() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, 0, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, 0, 20),
                block("lamp", RenderKind::Opaque, Occlusion::None, 15, 5),
            ])
            .unwrap(),
        )
    }

    fn block(
        name: &str,
        render: RenderKind,
        occlusion: Occlusion,
        emission: u8,
        resistance: u16,
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
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission,
            blast_resistance: resistance,
        }
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

    fn input(blocks: PalettedBlocks) -> LightingInput {
        LightingInput {
            center: snapshot(ChunkPos { x: 0, y: 0, z: 0 }, blocks),
            neighbors: std::array::from_fn(|_| None),
            registry: registry(),
            sky_from_above: true,
            stamp: LightStamp {
                center: ChunkRevision::default(),
                faces: [None; 6],
                registry_epoch: 1,
                lighting_epoch: 1,
            },
        }
    }

    #[test]
    fn open_air_receives_full_direct_sky() {
        let light = build_light(
            &input(PalettedBlocks::uniform(BlockStateId::AIR)),
            LightingBudget::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert!(LocalIndex::all().all(|index| light.sky(index) == 15));
    }

    #[test]
    fn emission_propagates_and_opaque_cells_stop_it() {
        let registry = registry();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp_pos = LocalPos::new(16, 16, 16).unwrap();
        let stone_pos = LocalPos::new(17, 16, 16).unwrap();
        let blocks = PalettedBlocks::uniform(BlockStateId::AIR)
            .with_updates(&[(lamp_pos.index(), lamp), (stone_pos.index(), stone)])
            .unwrap();
        let light = build_light(
            &input(blocks),
            LightingBudget::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(light.block(lamp_pos.index()), 15);
        assert_eq!(light.block(stone_pos.index()), 0);
        assert_eq!(light.block(LocalPos::new(15, 16, 16).unwrap().index()), 14);
    }

    #[test]
    fn stamp_budget_and_cancellation_are_fail_closed() {
        let mut invalid = input(PalettedBlocks::uniform(BlockStateId::AIR));
        invalid.stamp.faces[0] = Some(ChunkRevision::default());
        assert_eq!(
            build_light(&invalid, LightingBudget::default(), &CancelToken::new()),
            Err(LightingError::StampMismatch)
        );
        let valid = input(PalettedBlocks::uniform(BlockStateId::AIR));
        assert_eq!(
            build_light(
                &valid,
                LightingBudget {
                    max_propagation_steps: 1
                },
                &CancelToken::new()
            ),
            Err(LightingError::BudgetExceeded)
        );
        let cancel = CancelToken::new();
        cancel.cancel();
        assert_eq!(
            build_light(&valid, LightingBudget::default(), &cancel),
            Err(LightingError::Cancelled)
        );
    }

    /// The previous implementation, kept verbatim as the oracle for exact equivalence:
    /// identical light bytes and identical propagation-budget accounting.
    mod reference {
        use super::super::*;
        use voxy_core::LocalPos;

        pub fn build_light(
            input: &LightingInput,
            budget: LightingBudget,
            cancel: &CancelToken,
        ) -> Result<LightVolume, LightingError> {
            validate(input, budget)?;
            if cancel.is_cancelled() {
                return Err(LightingError::Cancelled);
            }
            let mut opaque = vec![true; PADDED_VOLUME];
            let mut emission = vec![0_u8; PADDED_VOLUME];
            let mut has_emission = false;
            for y in -1..=CHUNK_EDGE {
                for z in -1..=CHUNK_EDGE {
                    for x in -1..=CHUNK_EDGE {
                        let Some(block) = sample(input, [x, y, z]) else {
                            continue;
                        };
                        let definition = input
                            .registry
                            .get(block)
                            .ok_or(LightingError::UnknownBlock(block.get()))?;
                        let index = padded_index([x, y, z]);
                        opaque[index] = definition.occlusion == Occlusion::FullCube;
                        emission[index] = definition.emission;
                        if definition.emission > 0 {
                            has_emission = true;
                        }
                    }
                }
            }
            let mut sky = vec![0_u8; PADDED_VOLUME];
            if input.sky_from_above {
                seed_direct_sky(&opaque, &mut sky);
            }
            propagate(&opaque, &mut sky, budget, cancel, true)?;
            let mut block = emission;
            if has_emission {
                propagate(&opaque, &mut block, budget, cancel, false)?;
            }
            let mut packed = vec![0_u8; CHUNK_VOLUME];
            for index in LocalIndex::all() {
                let pos = index.position();
                let padded =
                    padded_index([i64::from(pos.x()), i64::from(pos.y()), i64::from(pos.z())]);
                packed[usize::from(index.get())] = (sky[padded] << 4) | block[padded];
            }
            Ok(LightVolume {
                packed: packed.into_boxed_slice(),
                stamp: input.stamp.clone(),
            })
        }

        fn sample(input: &LightingInput, pos: [i64; 3]) -> Option<BlockStateId> {
            let outside = pos.map(|value| !(0..CHUNK_EDGE).contains(&value));
            let outside_count = outside.into_iter().filter(|outside| *outside).count();
            if outside_count > 1 {
                return None;
            }
            let (snapshot, local) = if outside_count == 0 {
                (&input.center, pos)
            } else if pos[0] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegX.index()].as_ref()?,
                    [CHUNK_EDGE - 1, pos[1], pos[2]],
                )
            } else if pos[0] >= CHUNK_EDGE {
                (
                    input.neighbors[FaceNeighbor::PosX.index()].as_ref()?,
                    [0, pos[1], pos[2]],
                )
            } else if pos[1] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegY.index()].as_ref()?,
                    [pos[0], CHUNK_EDGE - 1, pos[2]],
                )
            } else if pos[1] >= CHUNK_EDGE {
                (
                    input.neighbors[FaceNeighbor::PosY.index()].as_ref()?,
                    [pos[0], 0, pos[2]],
                )
            } else if pos[2] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegZ.index()].as_ref()?,
                    [pos[0], pos[1], CHUNK_EDGE - 1],
                )
            } else {
                (
                    input.neighbors[FaceNeighbor::PosZ.index()].as_ref()?,
                    [pos[0], pos[1], 0],
                )
            };
            let local = LocalPos::new(
                u8::try_from(local[0]).ok()?,
                u8::try_from(local[1]).ok()?,
                u8::try_from(local[2]).ok()?,
            )
            .ok()?;
            Some(snapshot.data.blocks.get(local.index()))
        }

        fn propagate(
            opaque: &[bool],
            light: &mut [u8],
            budget: LightingBudget,
            cancel: &CancelToken,
            preserve_direct_down: bool,
        ) -> Result<(), LightingError> {
            let mut queue: VecDeque<_> = light
                .iter()
                .enumerate()
                .filter_map(|(index, &level)| (level > 1).then_some(index))
                .collect();
            if queue.is_empty() {
                return Ok(());
            }
            let mut steps = 0_usize;
            while let Some(index) = queue.pop_front() {
                steps = steps.checked_add(1).ok_or(LightingError::BudgetExceeded)?;
                if steps > budget.max_propagation_steps {
                    return Err(LightingError::BudgetExceeded);
                }
                if steps.is_multiple_of(1024) && cancel.is_cancelled() {
                    return Err(LightingError::Cancelled);
                }
                let current_light = light[index];
                let current_attenuated = current_light.saturating_sub(1);
                let x = index % PADDED_EDGE;
                let yz = index / PADDED_EDGE;
                let z = yz % PADDED_EDGE;
                let y = yz / PADDED_EDGE;
                if y > 0 {
                    let neighbor_index = index - STRIDE_Y;
                    if !opaque[neighbor_index] {
                        let candidate = if preserve_direct_down && current_light == MAX_LIGHT {
                            MAX_LIGHT
                        } else {
                            current_attenuated
                        };
                        if candidate > light[neighbor_index] {
                            light[neighbor_index] = candidate;
                            if candidate > 1 {
                                queue.push_back(neighbor_index);
                            }
                        }
                    }
                }
                if current_attenuated > 0 {
                    let lateral = [
                        (y < PADDED_EDGE - 1, index + STRIDE_Y),
                        (x > 0, index.wrapping_sub(1)),
                        (x < PADDED_EDGE - 1, index + 1),
                        (z > 0, index.wrapping_sub(STRIDE_Z)),
                        (z < PADDED_EDGE - 1, index + STRIDE_Z),
                    ];
                    for (valid, neighbor_index) in lateral {
                        if valid
                            && !opaque[neighbor_index]
                            && current_attenuated > light[neighbor_index]
                        {
                            light[neighbor_index] = current_attenuated;
                            if current_attenuated > 1 {
                                queue.push_back(neighbor_index);
                            }
                        }
                    }
                }
            }
            Ok(())
        }
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

    /// Height-field terrain with overhangs, caves, lamps and glass for one chunk.
    fn terrain_chunk(seed: u64, registry: &BlockRegistry, pos: ChunkPos) -> ChunkSnapshot {
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let glass = registry
            .find(&ResourceKey::parse("voxy:glass").unwrap())
            .unwrap();
        let mut random = Lcg(seed
            ^ pos.x.cast_unsigned().wrapping_mul(31)
            ^ pos.z.cast_unsigned().wrapping_mul(977));
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for z in 0..32_usize {
            for x in 0..32_usize {
                let jitter = usize::try_from(random.next() % 7).unwrap();
                let height = 8 + ((x * 3 + z * 5 + jitter) % 20);
                for y in 0..32_usize {
                    let index = x + 32 * (z + 32 * y);
                    let block = if y < height {
                        if random.next().is_multiple_of(23) {
                            BlockStateId::AIR
                        } else if random.next().is_multiple_of(97) {
                            lamp
                        } else {
                            stone
                        }
                    } else if y == height + 3 && random.next().is_multiple_of(5) {
                        stone
                    } else if random.next().is_multiple_of(61) {
                        glass
                    } else {
                        BlockStateId::AIR
                    };
                    dense[index] = block;
                }
            }
        }
        snapshot(pos, PalettedBlocks::from_dense(dense).unwrap())
    }

    fn registry_with_glass() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, 0, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, 0, 20),
                block("lamp", RenderKind::Opaque, Occlusion::None, 15, 5),
                block("glass", RenderKind::Translucent, Occlusion::None, 0, 3),
            ])
            .unwrap(),
        )
    }

    fn scenario(seed: u64, with_neighbors: bool, sky_from_above: bool) -> LightingInput {
        let registry = registry_with_glass();
        let center = ChunkPos { x: 0, y: 0, z: 0 };
        let neighbors: [Option<ChunkSnapshot>; 6] = if with_neighbors {
            [
                ChunkPos { x: -1, ..center },
                ChunkPos { x: 1, ..center },
                ChunkPos { y: -1, ..center },
                ChunkPos { y: 1, ..center },
                ChunkPos { z: -1, ..center },
                ChunkPos { z: 1, ..center },
            ]
            .map(|pos| Some(terrain_chunk(seed + 1, &registry, pos)))
        } else {
            std::array::from_fn(|_| None)
        };
        let faces = neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
        LightingInput {
            center: terrain_chunk(seed, &registry, center),
            neighbors,
            registry,
            sky_from_above,
            stamp: LightStamp {
                center: ChunkRevision::default(),
                faces,
                registry_epoch: 1,
                lighting_epoch: 1,
            },
        }
    }

    fn minimal_budget(
        input: &LightingInput,
        build: fn(
            &LightingInput,
            LightingBudget,
            &CancelToken,
        ) -> Result<LightVolume, LightingError>,
    ) -> usize {
        let (mut low, mut high) = (1_usize, LightingBudget::default().max_propagation_steps);
        assert!(
            build(
                input,
                LightingBudget {
                    max_propagation_steps: high
                },
                &CancelToken::new()
            )
            .is_ok()
        );
        while low < high {
            let middle = low + (high - low) / 2;
            let budget = LightingBudget {
                max_propagation_steps: middle,
            };
            if build(input, budget, &CancelToken::new()).is_ok() {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    }

    #[test]
    fn matches_reference_bytes_and_budget_on_terrain() {
        let cancel = CancelToken::new();
        for (seed, with_neighbors, sky) in [
            (1, true, true),
            (2, false, true),
            (3, true, false),
            (4, false, false),
            (5, true, true),
        ] {
            let input = scenario(seed, with_neighbors, sky);
            let expected =
                reference::build_light(&input, LightingBudget::default(), &cancel).unwrap();
            let actual = build_light(&input, LightingBudget::default(), &cancel).unwrap();
            assert_eq!(actual, expected, "seed {seed}");
            let budget = minimal_budget(&input, reference::build_light);
            assert_eq!(minimal_budget(&input, build_light), budget, "seed {seed}");
        }
    }

    #[test]
    fn matches_reference_for_uniform_chunks() {
        let registry = registry_with_glass();
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let cancel = CancelToken::new();
        for block in [BlockStateId::AIR, stone, lamp] {
            for with_neighbors in [false, true] {
                let mut input = scenario(9, with_neighbors, true);
                input.center = snapshot(ChunkPos::default(), PalettedBlocks::uniform(block));
                let expected = reference::build_light(&input, LightingBudget::default(), &cancel);
                let actual = build_light(&input, LightingBudget::default(), &cancel);
                assert_eq!(actual, expected);
                let budget = minimal_budget(&input, reference::build_light);
                assert_eq!(minimal_budget(&input, build_light), budget);
            }
        }
    }
}
