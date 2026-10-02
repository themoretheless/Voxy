use std::collections::{BTreeMap, BTreeSet};

use voxy_core::split_voxel;
use voxy_world::{
    BlockRegistry, BlockStateId, EditSource, EditTxn, Sample, VoxelPos, VoxelView, VoxelWrite,
};

use crate::{LiquidBudget, LiquidError, LiquidPlan, LiquidStates};

/// One immiscible liquid. Density is a positive relative integer; only ordering matters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiquidMaterial {
    pub states: LiquidStates,
    pub density: u32,
}

/// Plans vertical density sorting of immiscible liquids.
/// A denser upper cell exchanges its complete contents with a lighter lower cell.
/// Fill levels travel with their material, preserving each liquid's volume.
/// Each cell participates in at most one exchange per tick. Equal densities stay put.
/// Run this phase and commit it before planning ordinary liquid flow.
///
/// # Errors
/// Rejects zero densities, overlapping or invalid state sets, invalid/exceeded budgets,
/// coordinate overflow, unknown blocks, unavailable chunks, and inconsistent views.
#[allow(clippy::too_many_lines)]
pub fn step_liquid_layers(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    materials: &[LiquidMaterial],
    active: &[VoxelPos],
    source: EditSource,
    budget: LiquidBudget,
) -> Result<LiquidPlan, LiquidError> {
    let mut densities = BTreeMap::new();
    for material in materials {
        material.states.validate(registry)?;
        if material.density == 0 {
            return Err(LiquidError::InvalidMaterials);
        }
        for state in material.states.0 {
            if densities.insert(state, material.density).is_some() {
                return Err(LiquidError::InvalidMaterials);
            }
        }
    }
    if budget.max_active == 0 || budget.max_samples == 0 || budget.max_writes == 0 {
        return Err(LiquidError::InvalidBudget);
    }
    let active: BTreeSet<_> = active.iter().copied().collect();
    if active.len() > budget.max_active {
        return Err(LiquidError::ActiveBudgetExceeded {
            required: active.len(),
            limit: budget.max_active,
        });
    }
    let mut sampled = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut writes = Vec::new();
    for pos in active {
        if used.contains(&pos) {
            continue;
        }
        let above = read(view, registry, pos, &mut sampled, budget.max_samples)?;
        let Some(upper_density) = densities.get(&above) else {
            continue;
        };
        let below = VoxelPos {
            y: pos
                .y
                .checked_sub(1)
                .ok_or(LiquidError::CoordinateOverflow)?,
            ..pos
        };
        if used.contains(&below) {
            continue;
        }
        let lower = read(view, registry, below, &mut sampled, budget.max_samples)?;
        if densities
            .get(&lower)
            .is_some_and(|density| upper_density > density)
        {
            used.insert(pos);
            used.insert(below);
            writes.push(VoxelWrite { pos, block: lower });
            writes.push(VoxelWrite {
                pos: below,
                block: above,
            });
        }
    }
    if writes.len() > budget.max_writes {
        return Err(LiquidError::WriteBudgetExceeded {
            required: writes.len(),
            limit: budget.max_writes,
        });
    }
    if writes.is_empty() {
        return Ok(LiquidPlan::Settled);
    }
    writes.sort_by_key(|write| write.pos);
    let chunks: BTreeSet<_> = sampled.keys().map(|pos| split_voxel(*pos).0).collect();
    let expected = chunks
        .into_iter()
        .map(|pos| {
            view.chunk(pos)
                .map(|chunk| (pos, chunk.revision))
                .ok_or(LiquidError::InconsistentView(pos))
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
            next_active.insert(VoxelPos {
                x: write
                    .pos
                    .x
                    .checked_add(dx)
                    .ok_or(LiquidError::CoordinateOverflow)?,
                y: write
                    .pos
                    .y
                    .checked_add(dy)
                    .ok_or(LiquidError::CoordinateOverflow)?,
                z: write
                    .pos
                    .z
                    .checked_add(dz)
                    .ok_or(LiquidError::CoordinateOverflow)?,
            });
        }
    }
    Ok(LiquidPlan::Transaction {
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

fn read(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    pos: VoxelPos,
    cache: &mut BTreeMap<VoxelPos, BlockStateId>,
    limit: usize,
) -> Result<BlockStateId, LiquidError> {
    if let Some(&state) = cache.get(&pos) {
        return Ok(state);
    }
    if cache.len() >= limit {
        return Err(LiquidError::SampleBudgetExceeded { limit });
    }
    let state = match view.sample(pos) {
        Sample::Loaded(state) => {
            if registry.get(state).is_none() {
                return Err(LiquidError::UnknownState(state));
            }
            state
        }
        Sample::Unloaded { chunk } => return Err(LiquidError::Unloaded { at: pos, chunk }),
        Sample::Unavailable { chunk, cause } => {
            return Err(LiquidError::Unavailable {
                at: pos,
                chunk,
                cause,
            });
        }
    };
    cache.insert(pos, state);
    Ok(state)
}
