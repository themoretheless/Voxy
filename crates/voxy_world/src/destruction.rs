use std::collections::BTreeSet;
use std::fmt;

use voxy_core::{ChunkPos, VoxelPos, split_voxel};

use crate::{BlockStateId, EditSource, EditTxn, Sample, UnavailableReason, VoxelView, VoxelWrite};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Explosion {
    pub center: VoxelPos,
    pub radius: u8,
    pub power: u32,
    pub attenuation_per_squared_voxel: u32,
    pub max_candidates: usize,
    pub max_writes: usize,
}

impl Default for Explosion {
    fn default() -> Self {
        Self {
            center: VoxelPos { x: 0, y: 0, z: 0 },
            radius: 4,
            power: 100,
            attenuation_per_squared_voxel: 4,
            max_candidates: 1_000_000,
            max_writes: 65_536,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DestructionPlan {
    NoOp,
    Transaction(EditTxn),
}

/// Plans a deterministic spherical destruction transaction against one consistent world view.
///
/// A block is removed when `power - distance_squared * attenuation` is strictly greater than its
/// blast resistance. The planner aborts before producing writes if any required voxel is missing.
/// Its expected revisions make the later [`crate::World::commit`] fail atomically after a race.
///
/// # Errors
///
/// Rejects invalid/bounded-work settings, coordinate arithmetic overflow, unavailable chunks,
/// unknown block IDs, an inconsistent view, or a write budget overflow.
#[allow(clippy::too_many_lines)]
pub fn plan_explosion(
    view: &impl VoxelView,
    registry: &crate::BlockRegistry,
    source: EditSource,
    explosion: Explosion,
) -> Result<DestructionPlan, DestructionError> {
    if explosion.radius > 64 || explosion.max_candidates == 0 || explosion.max_writes == 0 {
        return Err(DestructionError::InvalidLimits);
    }
    let radius = i64::from(explosion.radius);
    let diameter = usize::from(explosion.radius)
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(DestructionError::CandidateCountOverflow)?;
    let cube = diameter
        .checked_pow(3)
        .ok_or(DestructionError::CandidateCountOverflow)?;
    if cube > explosion.max_candidates {
        return Err(DestructionError::CandidateBudgetExceeded {
            required: cube,
            limit: explosion.max_candidates,
        });
    }
    let radius_squared = radius * radius;
    let mut writes = Vec::new();
    let mut touched = BTreeSet::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                let distance_squared = dx * dx + dy * dy + dz * dz;
                if distance_squared > radius_squared {
                    continue;
                }
                let attenuation = u32::try_from(distance_squared)
                    .ok()
                    .and_then(|distance| {
                        distance.checked_mul(explosion.attenuation_per_squared_voxel)
                    })
                    .ok_or(DestructionError::EnergyOverflow)?;
                let energy = explosion.power.saturating_sub(attenuation);
                if energy == 0 {
                    continue;
                }
                let pos = VoxelPos {
                    x: explosion
                        .center
                        .x
                        .checked_add(dx)
                        .ok_or(DestructionError::CoordinateOverflow)?,
                    y: explosion
                        .center
                        .y
                        .checked_add(dy)
                        .ok_or(DestructionError::CoordinateOverflow)?,
                    z: explosion
                        .center
                        .z
                        .checked_add(dz)
                        .ok_or(DestructionError::CoordinateOverflow)?,
                };
                match view.sample(pos) {
                    Sample::Loaded(BlockStateId::AIR) => {}
                    Sample::Loaded(block) => {
                        let definition = registry
                            .get(block)
                            .ok_or(DestructionError::UnknownBlock(block))?;
                        if energy > u32::from(definition.blast_resistance) {
                            if writes.len() == explosion.max_writes {
                                return Err(DestructionError::WriteBudgetExceeded {
                                    limit: explosion.max_writes,
                                });
                            }
                            writes.push(VoxelWrite {
                                pos,
                                block: BlockStateId::AIR,
                            });
                            touched.insert(split_voxel(pos).0);
                        }
                    }
                    Sample::Unloaded { chunk } => {
                        return Err(DestructionError::Unloaded { at: pos, chunk });
                    }
                    Sample::Unavailable { chunk, cause } => {
                        return Err(DestructionError::Unavailable {
                            at: pos,
                            chunk,
                            cause,
                        });
                    }
                }
            }
        }
    }
    if writes.is_empty() {
        return Ok(DestructionPlan::NoOp);
    }
    let expected = touched
        .into_iter()
        .map(|pos| {
            view.chunk(pos)
                .map(|snapshot| (pos, snapshot.revision))
                .ok_or(DestructionError::InconsistentView(pos))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DestructionPlan::Transaction(EditTxn {
        source,
        expected,
        writes,
    }))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DestructionError {
    InvalidLimits,
    CandidateCountOverflow,
    CandidateBudgetExceeded {
        required: usize,
        limit: usize,
    },
    WriteBudgetExceeded {
        limit: usize,
    },
    EnergyOverflow,
    CoordinateOverflow,
    UnknownBlock(BlockStateId),
    InconsistentView(ChunkPos),
    Unloaded {
        at: VoxelPos,
        chunk: ChunkPos,
    },
    Unavailable {
        at: VoxelPos,
        chunk: ChunkPos,
        cause: UnavailableReason,
    },
}

impl fmt::Display for DestructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "destruction planning error: {self:?}")
    }
}

impl std::error::Error for DestructionError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use voxy_core::WorldEpoch;

    use super::*;
    use crate::{
        ChunkData, GeneratedChunk, PalettedBlocks, ResourceKey, World, WorldLimits, block,
    };

    fn stone_world() -> (World, Arc<crate::BlockRegistry>, BlockStateId) {
        let registry = Arc::new(block::test_registry());
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let mut world = World::new(
            WorldEpoch::new(1).unwrap(),
            Arc::clone(&registry),
            WorldLimits::default(),
        );
        world
            .insert_generated(GeneratedChunk {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(stone),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        (world, registry, stone)
    }

    #[test]
    fn plans_and_commits_one_atomic_blast_with_inverse() {
        let (mut world, registry, stone) = stone_world();
        let explosion = Explosion {
            center: VoxelPos { x: 8, y: 8, z: 8 },
            radius: 1,
            power: 100,
            attenuation_per_squared_voxel: 10,
            ..Explosion::default()
        };
        let DestructionPlan::Transaction(txn) =
            plan_explosion(&world, &registry, EditSource::Simulation, explosion).unwrap()
        else {
            panic!("blast should affect stone");
        };
        assert_eq!(txn.writes.len(), 7);
        assert_eq!(txn.expected.len(), 1);
        let receipt = world.commit(txn).unwrap();
        assert_eq!(receipt.inverse.writes.len(), 7);
        assert!(
            receipt
                .inverse
                .writes
                .iter()
                .all(|write| write.block == stone)
        );
        assert_eq!(
            world.sample(explosion.center),
            Sample::Loaded(BlockStateId::AIR)
        );
    }

    #[test]
    fn resistance_and_budgets_are_enforced_before_commit() {
        let (world, registry, _) = stone_world();
        let weak = Explosion {
            center: VoxelPos { x: 8, y: 8, z: 8 },
            radius: 0,
            power: 20,
            ..Explosion::default()
        };
        assert_eq!(
            plan_explosion(&world, &registry, EditSource::Simulation, weak).unwrap(),
            DestructionPlan::NoOp
        );
        let too_many = Explosion {
            radius: 4,
            max_candidates: 10,
            ..weak
        };
        assert!(matches!(
            plan_explosion(&world, &registry, EditSource::Simulation, too_many),
            Err(DestructionError::CandidateBudgetExceeded { .. })
        ));
    }

    #[test]
    fn blast_aborts_when_affected_domain_crosses_unloaded_space() {
        let (world, registry, _) = stone_world();
        let explosion = Explosion {
            center: VoxelPos { x: 31, y: 8, z: 8 },
            radius: 1,
            power: 100,
            ..Explosion::default()
        };
        assert!(matches!(
            plan_explosion(&world, &registry, EditSource::Simulation, explosion),
            Err(DestructionError::Unloaded { .. })
        ));
    }
}
