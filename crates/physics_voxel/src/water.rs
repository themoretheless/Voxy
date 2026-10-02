use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use voxy_core::split_voxel;
use voxy_world::{
    BlockRegistry, BlockStateId, ChunkPos, CollisionShape, EditSource, EditTxn, RenderKind, Sample,
    UnavailableReason, VoxelPos, VoxelView, VoxelWrite,
};

const MAX_LEVEL: u8 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaterStates(pub [BlockStateId; MAX_LEVEL as usize]);

impl WaterStates {
    /// Validates eight unique translucent, non-colliding registered states ordered low to full.
    ///
    /// # Errors
    ///
    /// Rejects unknown, duplicate, opaque, or colliding block states.
    pub fn validate(self, registry: &BlockRegistry) -> Result<Self, WaterError> {
        let unique: BTreeSet<_> = self.0.into_iter().collect();
        if unique.len() != usize::from(MAX_LEVEL) {
            return Err(WaterError::InvalidStates);
        }
        for state in self.0 {
            let definition = registry.get(state).ok_or(WaterError::UnknownState(state))?;
            if definition.render != RenderKind::Translucent
                || definition.collision != CollisionShape::Empty
            {
                return Err(WaterError::InvalidStates);
            }
        }
        Ok(self)
    }

    pub(crate) fn level(self, state: BlockStateId) -> Option<u8> {
        self.0
            .iter()
            .position(|&candidate| candidate == state)
            .and_then(|index| u8::try_from(index + 1).ok())
    }

    pub(crate) fn state(self, level: u8) -> BlockStateId {
        if level == 0 {
            BlockStateId::AIR
        } else {
            self.0[usize::from(level - 1)]
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaterBudget {
    pub max_active: usize,
    pub max_samples: usize,
    pub max_writes: usize,
}

impl Default for WaterBudget {
    fn default() -> Self {
        Self {
            max_active: 16_384,
            max_samples: 131_072,
            max_writes: 65_536,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaterPlan {
    Settled,
    Transaction {
        edit: EditTxn,
        next_active: Box<[VoxelPos]>,
    },
}

/// Eight registered fill levels of one liquid. Different liquids must use disjoint states.
pub type LiquidStates = WaterStates;
pub type LiquidBudget = WaterBudget;
pub type LiquidPlan = WaterPlan;
pub type LiquidError = WaterError;

/// Discrete transfer limits per active cell per tick, in eighths of a voxel.
/// These are gameplay parameters, not physical viscosity measurements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiquidFlow {
    pub max_downward: u8,
    pub max_horizontal: u8,
}

impl LiquidFlow {
    pub const WATER: Self = Self {
        max_downward: 8,
        max_horizontal: 4,
    };
    pub const OIL: Self = Self {
        max_downward: 2,
        max_horizontal: 2,
    };
    pub const LAVA: Self = Self {
        max_downward: 1,
        max_horizontal: 1,
    };
}

/// Plans one deterministic, volume-conserving voxel-water tick.
///
/// Full cells contain eight units. Each active cell first transfers as much as possible downward,
/// then distributes remaining pressure one unit at a time in canonical `-X,+X,-Z,+Z` order.
/// All reads use one consistent world snapshot and the result is one revision-checked transaction.
///
/// # Errors
///
/// Rejects invalid states/budgets, excessive active work, coordinate overflow, missing chunks,
/// unavailable data, unknown blocks, sample overflow, or write-budget overflow.
#[allow(clippy::too_many_lines)]
pub fn step_water(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    states: WaterStates,
    active: &[VoxelPos],
    source: EditSource,
    budget: WaterBudget,
) -> Result<WaterPlan, WaterError> {
    step_liquid(
        view,
        registry,
        states,
        active,
        source,
        budget,
        LiquidFlow::WATER,
    )
}

/// Plans a volume-conserving tick for one liquid with configurable flow limits.
/// Other registered liquids are impermeable boundaries; they are not replaced or mixed.
/// Commit each liquid's transaction before planning the next liquid against the updated world.
///
/// # Errors
/// Returns the same errors as `step_water`, plus `InvalidFlow` for limits outside 1..=8.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn step_liquid(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    states: LiquidStates,
    active: &[VoxelPos],
    source: EditSource,
    budget: LiquidBudget,
    flow: LiquidFlow,
) -> Result<LiquidPlan, LiquidError> {
    if !(1..=MAX_LEVEL).contains(&flow.max_downward)
        || !(1..=MAX_LEVEL).contains(&flow.max_horizontal)
    {
        return Err(WaterError::InvalidFlow);
    }
    let states = states.validate(registry)?;
    if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
        return Err(WaterError::InvalidBudget);
    }
    let active: BTreeSet<_> = active.iter().copied().collect();
    if active.len() > budget.max_active {
        return Err(WaterError::ActiveBudgetExceeded {
            required: active.len(),
            limit: budget.max_active,
        });
    }
    let mut sample_count = 0_usize;
    let mut initial = BTreeMap::new();
    let mut amounts = BTreeMap::new();
    let mut touched = BTreeSet::new();
    for position in active {
        let sampled = get_amount(
            view,
            registry,
            states,
            position,
            &mut initial,
            &mut amounts,
            &mut sample_count,
            budget.max_samples,
        )?;
        let Some(mut amount) = sampled else {
            continue;
        };
        if amount == 0 {
            continue;
        }
        let below = offset(position, 0, -1, 0)?;
        let below_amount = get_amount(
            view,
            registry,
            states,
            below,
            &mut initial,
            &mut amounts,
            &mut sample_count,
            budget.max_samples,
        )?;
        if let Some(below_amount) = below_amount {
            let transfer = amount.min(MAX_LEVEL - below_amount).min(flow.max_downward);
            if transfer > 0 {
                amount -= transfer;
                amounts.insert(position, Some(amount));
                amounts.insert(below, Some(below_amount + transfer));
                touched.insert(position);
                touched.insert(below);
            }
        }
        if amount == 0 {
            continue;
        }
        let mut horizontal_transferred = 0;
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let neighbor = offset(position, dx, 0, dz)?;
            let Some(neighbor_amount) = get_amount(
                view,
                registry,
                states,
                neighbor,
                &mut initial,
                &mut amounts,
                &mut sample_count,
                budget.max_samples,
            )?
            else {
                continue;
            };
            if horizontal_transferred < flow.max_horizontal
                && amount > 1
                && neighbor_amount + 1 < amount
            {
                horizontal_transferred += 1;
                amount -= 1;
                amounts.insert(position, Some(amount));
                amounts.insert(neighbor, Some(neighbor_amount + 1));
                touched.insert(position);
                touched.insert(neighbor);
            }
        }
    }
    let mut writes = touched
        .iter()
        .filter_map(|position| {
            let before = initial.get(position).copied().flatten()?;
            let after = amounts.get(position).copied().flatten()?;
            (before != after).then_some(VoxelWrite {
                pos: *position,
                block: states.state(after),
            })
        })
        .collect::<Vec<_>>();
    writes.sort_by_key(|write| write.pos);
    if writes.is_empty() {
        return Ok(WaterPlan::Settled);
    }
    if writes.len() > budget.max_writes {
        return Err(WaterError::WriteBudgetExceeded {
            required: writes.len(),
            limit: budget.max_writes,
        });
    }
    let chunks: BTreeSet<ChunkPos> = initial
        .keys()
        .map(|position| split_voxel(*position).0)
        .collect();
    let expected = chunks
        .into_iter()
        .map(|pos| {
            view.chunk(pos)
                .map(|snapshot| (pos, snapshot.revision))
                .ok_or(WaterError::InconsistentView(pos))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut next_active = BTreeSet::new();
    for write in &writes {
        next_active.insert(write.pos);
        for (dx, dy, dz) in [
            (0, 1, 0),
            (0, -1, 0),
            (-1, 0, 0),
            (1, 0, 0),
            (0, 0, -1),
            (0, 0, 1),
        ] {
            next_active.insert(offset(write.pos, dx, dy, dz)?);
        }
    }
    Ok(WaterPlan::Transaction {
        edit: EditTxn {
            source,
            expected,
            writes,
        },
        next_active: next_active
            .into_iter()
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    })
}

#[allow(clippy::too_many_arguments)]
fn get_amount(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    states: WaterStates,
    position: VoxelPos,
    initial: &mut BTreeMap<VoxelPos, Option<u8>>,
    amounts: &mut BTreeMap<VoxelPos, Option<u8>>,
    samples: &mut usize,
    limit: usize,
) -> Result<Option<u8>, WaterError> {
    if let Some(&amount) = amounts.get(&position) {
        return Ok(amount);
    }
    let amount = sample_amount(view, registry, states, position, samples, limit)?;
    initial.insert(position, amount);
    amounts.insert(position, amount);
    Ok(amount)
}

fn sample_amount(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    states: WaterStates,
    position: VoxelPos,
    samples: &mut usize,
    limit: usize,
) -> Result<Option<u8>, WaterError> {
    *samples = samples.checked_add(1).ok_or(WaterError::SampleOverflow)?;
    if *samples > limit {
        return Err(WaterError::SampleBudgetExceeded { limit });
    }
    match view.sample(position) {
        Sample::Loaded(BlockStateId::AIR) => Ok(Some(0)),
        Sample::Loaded(block) => {
            if let Some(level) = states.level(block) {
                Ok(Some(level))
            } else if registry.get(block).is_some() {
                Ok(None)
            } else {
                Err(WaterError::UnknownState(block))
            }
        }
        Sample::Unloaded { chunk } => Err(WaterError::Unloaded {
            at: position,
            chunk,
        }),
        Sample::Unavailable { chunk, cause } => Err(WaterError::Unavailable {
            at: position,
            chunk,
            cause,
        }),
    }
}

fn offset(position: VoxelPos, dx: i64, dy: i64, dz: i64) -> Result<VoxelPos, WaterError> {
    Ok(VoxelPos {
        x: position
            .x
            .checked_add(dx)
            .ok_or(WaterError::CoordinateOverflow)?,
        y: position
            .y
            .checked_add(dy)
            .ok_or(WaterError::CoordinateOverflow)?,
        z: position
            .z
            .checked_add(dz)
            .ok_or(WaterError::CoordinateOverflow)?,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaterError {
    InvalidStates,
    UnknownState(BlockStateId),
    InvalidBudget,
    InvalidFlow,
    InvalidMaterials,
    ActiveBudgetExceeded {
        required: usize,
        limit: usize,
    },
    SampleBudgetExceeded {
        limit: usize,
    },
    WriteBudgetExceeded {
        required: usize,
        limit: usize,
    },
    SampleOverflow,
    CoordinateOverflow,
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

impl fmt::Display for WaterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "water simulation error: {self:?}")
    }
}

impl std::error::Error for WaterError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use voxy_core::WorldEpoch;
    use voxy_world::{
        BlockDef, ChunkData, GeneratedChunk, InterfaceGroupId, MaterialId, Occlusion,
        PalettedBlocks, ResourceKey, World, WorldLimits,
    };

    use super::*;

    fn fixture() -> (World, Arc<BlockRegistry>, WaterStates, BlockStateId) {
        let mut definitions = vec![BlockDef {
            key: ResourceKey::parse("voxy:air").unwrap(),
            render: RenderKind::Invisible,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 0,
        }];
        definitions.push(BlockDef {
            key: ResourceKey::parse("voxy:stone").unwrap(),
            render: RenderKind::Opaque,
            occlusion: Occlusion::FullCube,
            collision: CollisionShape::FullCube,
            face_materials: [MaterialId(1); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: 20,
        });
        for level in 1..=8 {
            definitions.push(BlockDef {
                key: ResourceKey::parse(format!("voxy:water_{level}")).unwrap(),
                render: RenderKind::Translucent,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(2); 6],
                translucent_interface_group: Some(InterfaceGroupId(1)),
                emission: 0,
                blast_resistance: 0,
            });
        }
        for level in 1..=8 {
            definitions.push(BlockDef {
                key: ResourceKey::parse(format!("voxy:oil_{level}")).unwrap(),
                render: RenderKind::Translucent,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(3); 6],
                translucent_interface_group: Some(InterfaceGroupId(2)),
                emission: 0,
                blast_resistance: 0,
            });
        }
        let registry = Arc::new(BlockRegistry::new(definitions).unwrap());
        let states = WaterStates(std::array::from_fn(|index| {
            registry
                .find(&ResourceKey::parse(format!("voxy:water_{}", index + 1)).unwrap())
                .unwrap()
        }));
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
                pos: ChunkPos::default(),
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(BlockStateId::AIR),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        (world, registry, states, stone)
    }

    #[test]
    fn full_cell_falls_and_conserves_all_eight_units() {
        let (mut world, registry, states, _) = fixture();
        let source = VoxelPos { x: 5, y: 5, z: 5 };
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: source,
                    block: states.0[7],
                }],
            })
            .unwrap();
        let WaterPlan::Transaction { edit, .. } = step_water(
            &world,
            &registry,
            states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap() else {
            panic!("water should fall");
        };
        assert_eq!(edit.writes.len(), 2);
        world.commit(edit).unwrap();
        assert_eq!(world.sample(source), Sample::Loaded(BlockStateId::AIR));
        assert_eq!(
            world.sample(VoxelPos { y: 4, ..source }),
            Sample::Loaded(states.0[7])
        );
    }

    #[test]
    fn blocked_full_cell_spreads_in_canonical_order_without_losing_volume() {
        let (mut world, registry, states, stone) = fixture();
        let source = VoxelPos { x: 8, y: 8, z: 8 };
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: Vec::new(),
                writes: vec![
                    VoxelWrite {
                        pos: source,
                        block: states.0[7],
                    },
                    VoxelWrite {
                        pos: VoxelPos { y: 7, ..source },
                        block: stone,
                    },
                ],
            })
            .unwrap();
        let first = step_water(
            &world,
            &registry,
            states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap();
        let second = step_water(
            &world,
            &registry,
            states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap();
        assert_eq!(first, second);
        let WaterPlan::Transaction { edit, .. } = first else {
            panic!("water should spread");
        };
        assert_eq!(edit.writes.len(), 5);
        let total = edit
            .writes
            .iter()
            .filter_map(|write| states.level(write.block))
            .map(u16::from)
            .sum::<u16>();
        assert_eq!(total, 8);
    }
    #[test]
    fn adjacent_active_cells_use_transferred_volume() {
        let (mut world, registry, states, stone) = fixture();
        let left = VoxelPos { x: 8, y: 8, z: 8 };
        let right = VoxelPos { x: 9, ..left };
        let mut writes = vec![
            VoxelWrite {
                pos: left,
                block: states.0[7],
            },
            VoxelWrite {
                pos: right,
                block: states.0[1],
            },
        ];
        for cell in [left, right] {
            for (dx, dy, dz) in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
                let pos = offset(cell, dx, dy, dz).unwrap();
                if pos != left && pos != right {
                    writes.push(VoxelWrite { pos, block: stone });
                }
            }
        }
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: vec![],
                writes,
            })
            .unwrap();
        for _ in 0..10 {
            let plan = step_water(
                &world,
                &registry,
                states,
                &[right, left, right],
                EditSource::Simulation,
                WaterBudget::default(),
            )
            .unwrap();
            if let WaterPlan::Transaction { edit, .. } = plan {
                world.commit(edit).unwrap();
            }
            let total: u8 = [left, right]
                .into_iter()
                .map(|pos| match world.sample(pos) {
                    Sample::Loaded(block) => states.level(block).unwrap_or(0),
                    _ => panic!("missing cell"),
                })
                .sum();
            assert_eq!(total, 10);
        }
    }
    #[test]
    fn liquid_presets_limit_fall_and_preserve_volume() {
        for (flow, expected_fall) in [
            (LiquidFlow::WATER, 8),
            (LiquidFlow::OIL, 2),
            (LiquidFlow::LAVA, 1),
        ] {
            let (mut world, registry, states, stone) = fixture();
            let pos = VoxelPos { x: 8, y: 8, z: 8 };
            let mut writes = vec![VoxelWrite {
                pos,
                block: states.0[7],
            }];
            for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                writes.push(VoxelWrite {
                    pos: offset(pos, dx, 0, dz).unwrap(),
                    block: stone,
                });
            }
            world
                .commit(EditTxn {
                    source: EditSource::Simulation,
                    expected: vec![],
                    writes,
                })
                .unwrap();
            let LiquidPlan::Transaction { edit, .. } = step_liquid(
                &world,
                &registry,
                states,
                &[pos],
                EditSource::Simulation,
                LiquidBudget::default(),
                flow,
            )
            .unwrap() else {
                panic!("must flow")
            };
            world.commit(edit).unwrap();
            let Sample::Loaded(below) = world.sample(offset(pos, 0, -1, 0).unwrap()) else {
                panic!("missing cell")
            };
            let Sample::Loaded(above) = world.sample(pos) else {
                panic!("missing cell")
            };
            assert_eq!(states.level(below), Some(expected_fall));
            assert_eq!(
                states.level(below).unwrap() + states.level(above).unwrap_or(0),
                8
            );
        }
    }

    #[test]
    fn invalid_flow_is_rejected() {
        let (world, registry, states, _) = fixture();
        for flow in [
            LiquidFlow {
                max_downward: 0,
                max_horizontal: 1,
            },
            LiquidFlow {
                max_downward: 1,
                max_horizontal: 9,
            },
        ] {
            assert_eq!(
                step_liquid(
                    &world,
                    &registry,
                    states,
                    &[],
                    EditSource::Simulation,
                    LiquidBudget::default(),
                    flow
                ),
                Err(LiquidError::InvalidFlow)
            );
        }
    }
    #[test]
    fn other_liquid_is_not_overwritten() {
        let (mut world, registry, states, stone) = fixture();
        let pos = VoxelPos { x: 8, y: 8, z: 8 };
        let oil = registry
            .find(&ResourceKey::parse("voxy:oil_8").unwrap())
            .unwrap();
        let mut writes = vec![
            VoxelWrite {
                pos,
                block: states.0[7],
            },
            VoxelWrite {
                pos: offset(pos, 0, -1, 0).unwrap(),
                block: oil,
            },
        ];
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            writes.push(VoxelWrite {
                pos: offset(pos, dx, 0, dz).unwrap(),
                block: stone,
            });
        }
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: vec![],
                writes,
            })
            .unwrap();
        assert_eq!(
            step_liquid(
                &world,
                &registry,
                states,
                &[pos],
                EditSource::Simulation,
                LiquidBudget::default(),
                LiquidFlow::WATER
            )
            .unwrap(),
            LiquidPlan::Settled
        );
        assert_eq!(
            world.sample(offset(pos, 0, -1, 0).unwrap()),
            Sample::Loaded(oil)
        );
    }
    #[test]
    fn density_layers_preserve_partial_volumes_and_stay_stable() {
        use crate::{LiquidMaterial, step_liquid_layers};
        let (mut world, registry, water, _) = fixture();
        let oil = WaterStates(std::array::from_fn(|i| {
            registry
                .find(&ResourceKey::parse(format!("voxy:oil_{}", i + 1)).unwrap())
                .unwrap()
        }));
        let materials = [
            LiquidMaterial {
                states: water,
                density: 1000,
            },
            LiquidMaterial {
                states: oil,
                density: 800,
            },
        ];
        let top = VoxelPos { x: 8, y: 8, z: 8 };
        let bottom = VoxelPos { y: 7, ..top };
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: vec![],
                writes: vec![
                    VoxelWrite {
                        pos: top,
                        block: water.0[4],
                    },
                    VoxelWrite {
                        pos: bottom,
                        block: oil.0[2],
                    },
                ],
            })
            .unwrap();
        let LiquidPlan::Transaction { edit, .. } = step_liquid_layers(
            &world,
            &registry,
            &materials,
            &[top, bottom, top],
            EditSource::Simulation,
            LiquidBudget::default(),
        )
        .unwrap() else {
            panic!("must separate")
        };
        assert_eq!(
            step_liquid_layers(
                &world,
                &registry,
                &materials,
                &[top],
                EditSource::Simulation,
                LiquidBudget {
                    max_writes: 1,
                    ..LiquidBudget::default()
                }
            ),
            Err(LiquidError::WriteBudgetExceeded {
                required: 2,
                limit: 1
            })
        );
        assert_eq!(world.sample(top), Sample::Loaded(water.0[4]));
        assert_eq!(
            step_liquid_layers(
                &world,
                &registry,
                &materials,
                &[top],
                EditSource::Simulation,
                LiquidBudget {
                    max_samples: 1,
                    ..LiquidBudget::default()
                }
            ),
            Err(LiquidError::SampleBudgetExceeded { limit: 1 })
        );
        world.commit(edit).unwrap();
        assert_eq!(world.sample(top), Sample::Loaded(oil.0[2]));
        assert_eq!(world.sample(bottom), Sample::Loaded(water.0[4]));
        assert_eq!(
            step_liquid_layers(
                &world,
                &registry,
                &materials,
                &[top, bottom],
                EditSource::Simulation,
                LiquidBudget::default()
            )
            .unwrap(),
            LiquidPlan::Settled
        );
        let overlapping = [materials[0], materials[0]];
        assert_eq!(
            step_liquid_layers(
                &world,
                &registry,
                &overlapping,
                &[top],
                EditSource::Simulation,
                LiquidBudget::default()
            ),
            Err(LiquidError::InvalidMaterials)
        );
        let equal = [
            materials[0],
            LiquidMaterial {
                density: 1000,
                ..materials[1]
            },
        ];
        assert_eq!(
            step_liquid_layers(
                &world,
                &registry,
                &equal,
                &[top],
                EditSource::Simulation,
                LiquidBudget::default()
            )
            .unwrap(),
            LiquidPlan::Settled
        );
    }
}
