//! Deterministic derived voxel lighting with revision-stamped inputs.

use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use voxy_core::{CHUNK_EDGE, CHUNK_VOLUME, CancelToken, LocalIndex, LocalPos};
use voxy_world::{BlockRegistry, ChunkRevision, ChunkSnapshot, Occlusion};

const PADDED_EDGE: usize = 34;
const PADDED_VOLUME: usize = PADDED_EDGE * PADDED_EDGE * PADDED_EDGE;
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
    let mut opaque = vec![true; PADDED_VOLUME];
    let mut emission = vec![0_u8; PADDED_VOLUME];
    let mut has_emission = false;
    let center_dense = input.center.data.blocks.to_dense();

    for y in -1..=CHUNK_EDGE {
        let py = usize::try_from(y + 1).unwrap_or(0);
        let in_y = (0..CHUNK_EDGE).contains(&y);
        for z in -1..=CHUNK_EDGE {
            let pz = usize::try_from(z + 1).unwrap_or(0);
            let in_z = (0..CHUNK_EDGE).contains(&z);
            let base_padded = (py * PADDED_EDGE + pz) * PADDED_EDGE;

            for x in -1..=CHUNK_EDGE {
                let block = if in_y && in_z && (0..CHUNK_EDGE).contains(&x) {
                    center_dense[(x as usize) + 32 * ((z as usize) + 32 * (y as usize))]
                } else {
                    let Some(b) = sample(input, [x, y, z]) else {
                        continue;
                    };
                    b
                };
                let definition = input
                    .registry
                    .get(block)
                    .ok_or(LightingError::UnknownBlock(block.get()))?;
                let px = usize::try_from(x + 1).unwrap_or(0);
                let index = base_padded + px;
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
    for y in 0..32_usize {
        let py = (y + 1) * PADDED_EDGE;
        for z in 0..32_usize {
            let base_padded = (py + (z + 1)) * PADDED_EDGE + 1;
            let base_local = 32 * (z + 32 * y);
            for x in 0..32_usize {
                let padded = base_padded + x;
                let local_idx = base_local + x;
                packed[local_idx] = (sky[padded] << 4) | block[padded];
            }
        }
    }
    Ok(LightVolume {
        packed: packed.into_boxed_slice(),
        stamp: input.stamp.clone(),
    })
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

fn sample(input: &LightingInput, pos: [i64; 3]) -> Option<voxy_world::BlockStateId> {
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
    const STRIDE_Z: usize = PADDED_EDGE;
    const STRIDE_Y: usize = PADDED_EDGE * PADDED_EDGE;

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

        // Down: y > 0
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
            // Up: y < PADDED_EDGE - 1
            if y < PADDED_EDGE - 1 {
                let neighbor_index = index + STRIDE_Y;
                if !opaque[neighbor_index] && current_attenuated > light[neighbor_index] {
                    light[neighbor_index] = current_attenuated;
                    if current_attenuated > 1 {
                        queue.push_back(neighbor_index);
                    }
                }
            }
            // Left: x > 0
            if x > 0 {
                let neighbor_index = index - 1;
                if !opaque[neighbor_index] && current_attenuated > light[neighbor_index] {
                    light[neighbor_index] = current_attenuated;
                    if current_attenuated > 1 {
                        queue.push_back(neighbor_index);
                    }
                }
            }
            // Right: x < PADDED_EDGE - 1
            if x < PADDED_EDGE - 1 {
                let neighbor_index = index + 1;
                if !opaque[neighbor_index] && current_attenuated > light[neighbor_index] {
                    light[neighbor_index] = current_attenuated;
                    if current_attenuated > 1 {
                        queue.push_back(neighbor_index);
                    }
                }
            }
            // Back: z > 0
            if z > 0 {
                let neighbor_index = index - STRIDE_Z;
                if !opaque[neighbor_index] && current_attenuated > light[neighbor_index] {
                    light[neighbor_index] = current_attenuated;
                    if current_attenuated > 1 {
                        queue.push_back(neighbor_index);
                    }
                }
            }
            // Forward: z < PADDED_EDGE - 1
            if z < PADDED_EDGE - 1 {
                let neighbor_index = index + STRIDE_Z;
                if !opaque[neighbor_index] && current_attenuated > light[neighbor_index] {
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
}
