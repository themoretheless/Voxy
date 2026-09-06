use std::fmt;

use voxy_core::{ChunkPos, VoxelPos};

use voxy_world::{
    BlockRegistry, BlockStateId, CollisionShape, Sample, UnavailableReason, VoxelView,
};

const MAX_LOCAL_MAGNITUDE: f64 = 1_048_576.0;

/// An AABB expressed near an integer anchor, preserving precision in far worlds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredAabb {
    pub anchor: VoxelPos,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SweepConfig {
    pub max_candidate_voxels: usize,
}

impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            max_candidate_voxels: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepObstacle {
    Block {
        pos: VoxelPos,
        block: BlockStateId,
    },
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepResult {
    /// Safe fraction of the requested displacement in `[0, 1]`.
    pub fraction: f64,
    pub normal: [i8; 3],
    pub obstacle: Option<SweepObstacle>,
}

/// Sweeps an AABB against full-cube voxel collision shapes.
///
/// Missing and permanently unavailable voxels are conservative solid boundaries. Equal contact
/// times prefer X, then Y, then Z, followed by lexicographic voxel order.
///
/// # Errors
///
/// Rejects malformed/non-finite bounds or displacement, an empty candidate budget, excessive
/// broad-phase volume, coordinate overflow, and loaded blocks absent from the registry.
pub fn sweep_aabb(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    config: SweepConfig,
) -> Result<SweepResult, SweepError> {
    validate(aabb, displacement, config)?;
    let broad_min = std::array::from_fn(|axis| aabb.min[axis] + displacement[axis].min(0.0));
    let broad_max = std::array::from_fn(|axis| aabb.max[axis] + displacement[axis].max(0.0));
    let min_voxel = broad_min.map(floor_to_i64);
    // AABB maxima are exclusive. Moving one representable value inward avoids including a voxel
    // touched only at the broad-phase maximum.
    let max_voxel = broad_max.map(|value| floor_to_i64(next_down(value)));
    let candidate_count = candidate_count(min_voxel, max_voxel)?;
    if candidate_count > config.max_candidate_voxels {
        return Err(SweepError::CandidateBudgetExceeded {
            required: candidate_count,
            limit: config.max_candidate_voxels,
        });
    }

    let mut best: Option<Contact> = None;
    for x in min_voxel[0]..=max_voxel[0] {
        for y in min_voxel[1]..=max_voxel[1] {
            for z in min_voxel[2]..=max_voxel[2] {
                let pos = VoxelPos {
                    x: aabb
                        .anchor
                        .x
                        .checked_add(x)
                        .ok_or(SweepError::CoordinateOverflow)?,
                    y: aabb
                        .anchor
                        .y
                        .checked_add(y)
                        .ok_or(SweepError::CoordinateOverflow)?,
                    z: aabb
                        .anchor
                        .z
                        .checked_add(z)
                        .ok_or(SweepError::CoordinateOverflow)?,
                };
                let obstacle = match view.sample(pos) {
                    Sample::Loaded(block) => {
                        let definition =
                            registry.get(block).ok_or(SweepError::UnknownBlock(block))?;
                        if definition.collision == CollisionShape::Empty {
                            continue;
                        }
                        SweepObstacle::Block { pos, block }
                    }
                    Sample::Unloaded { chunk } => SweepObstacle::Unloaded { at: pos, chunk },
                    Sample::Unavailable { chunk, cause } => SweepObstacle::Unavailable {
                        at: pos,
                        chunk,
                        cause,
                    },
                };
                if let Some((fraction, normal)) = contact(aabb, displacement, [x, y, z]) {
                    let candidate = Contact {
                        fraction,
                        normal,
                        obstacle,
                    };
                    if best
                        .as_ref()
                        .is_none_or(|current| candidate.precedes(current))
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    Ok(best.map_or(
        SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        },
        |contact| SweepResult {
            fraction: contact.fraction,
            normal: contact.normal,
            obstacle: Some(contact.obstacle),
        },
    ))
}

#[derive(Clone, Copy, Debug)]
struct Contact {
    fraction: f64,
    normal: [i8; 3],
    obstacle: SweepObstacle,
}

impl Contact {
    fn precedes(&self, other: &Self) -> bool {
        self.fraction.total_cmp(&other.fraction).is_lt()
            || (self.fraction.total_cmp(&other.fraction).is_eq()
                && obstacle_position(self.obstacle) < obstacle_position(other.obstacle))
    }
}

fn obstacle_position(obstacle: SweepObstacle) -> VoxelPos {
    match obstacle {
        SweepObstacle::Block { pos, .. } => pos,
        SweepObstacle::Unloaded { at, .. } | SweepObstacle::Unavailable { at, .. } => at,
    }
}

fn contact(aabb: AnchoredAabb, displacement: [f64; 3], voxel: [i64; 3]) -> Option<(f64, [i8; 3])> {
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut normal = [0_i8; 3];
    for (axis, &axis_displacement) in displacement.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let voxel_min = voxel[axis] as f64;
        let voxel_max = voxel_min + 1.0;
        let velocity = axis_displacement;
        if velocity == 0.0 {
            if aabb.max[axis] <= voxel_min || aabb.min[axis] >= voxel_max {
                return None;
            }
            continue;
        }
        let first = (voxel_min - aabb.max[axis]) / velocity;
        let second = (voxel_max - aabb.min[axis]) / velocity;
        let axis_enter = first.min(second);
        let axis_exit = first.max(second);
        if axis_enter > enter {
            enter = axis_enter;
            normal = [0; 3];
            normal[axis] = if velocity > 0.0 { -1 } else { 1 };
        }
        exit = exit.min(axis_exit);
        if enter > exit {
            return None;
        }
    }
    if exit < 0.0 || enter > 1.0 {
        None
    } else if enter < 0.0 {
        Some((0.0, [0; 3]))
    } else {
        Some((enter, normal))
    }
}

fn candidate_count(min: [i64; 3], max: [i64; 3]) -> Result<usize, SweepError> {
    min.into_iter()
        .zip(max)
        .try_fold(1_usize, |volume, (min, max)| {
            let length = max
                .checked_sub(min)
                .and_then(|value| value.checked_add(1))
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(SweepError::CandidateCountOverflow)?;
            volume
                .checked_mul(length)
                .ok_or(SweepError::CandidateCountOverflow)
        })
}

fn floor_to_i64(value: f64) -> i64 {
    // Validation bounds the value far inside i64's exactly representable range.
    #[allow(clippy::cast_possible_truncation)]
    {
        value.floor() as i64
    }
}

fn next_down(value: f64) -> f64 {
    if value == f64::NEG_INFINITY {
        value
    } else if value == 0.0 {
        -f64::from_bits(1)
    } else {
        let bits = value.to_bits();
        f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
    }
}

fn validate(
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    config: SweepConfig,
) -> Result<(), SweepError> {
    if config.max_candidate_voxels == 0 {
        return Err(SweepError::InvalidCandidateBudget);
    }
    for (axis, &axis_displacement) in displacement.iter().enumerate() {
        if !aabb.min[axis].is_finite()
            || !aabb.max[axis].is_finite()
            || aabb.min[axis] >= aabb.max[axis]
            || aabb.min[axis].abs() > MAX_LOCAL_MAGNITUDE
            || aabb.max[axis].abs() > MAX_LOCAL_MAGNITUDE
        {
            return Err(SweepError::InvalidAabb);
        }
        if !axis_displacement.is_finite() || axis_displacement.abs() > MAX_LOCAL_MAGNITUDE {
            return Err(SweepError::InvalidDisplacement);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepError {
    InvalidAabb,
    InvalidDisplacement,
    InvalidCandidateBudget,
    CandidateCountOverflow,
    CandidateBudgetExceeded { required: usize, limit: usize },
    CoordinateOverflow,
    UnknownBlock(BlockStateId),
}

impl fmt::Display for SweepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "voxel sweep error: {self:?}")
    }
}

impl std::error::Error for SweepError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use voxy_core::split_voxel;

    use super::*;
    use voxy_world::{
        BlockDef, ChunkSnapshot, MaterialId, Occlusion, RegistryError, RenderKind, ResourceKey,
    };

    #[derive(Default)]
    struct TestView {
        blocks: BTreeMap<VoxelPos, BlockStateId>,
        unloaded: bool,
        unavailable: Option<UnavailableReason>,
    }

    impl VoxelView for TestView {
        fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
            if let Some(&block) = self.blocks.get(&pos) {
                Sample::Loaded(block)
            } else if let Some(cause) = self.unavailable {
                Sample::Unavailable {
                    chunk: split_voxel(pos).0,
                    cause,
                }
            } else if self.unloaded {
                Sample::Unloaded {
                    chunk: split_voxel(pos).0,
                }
            } else {
                Sample::Loaded(BlockStateId::AIR)
            }
        }

        fn chunk(&self, _pos: ChunkPos) -> Option<ChunkSnapshot> {
            None
        }
    }

    fn registry() -> Result<Arc<BlockRegistry>, RegistryError> {
        BlockRegistry::new(vec![
            BlockDef {
                key: ResourceKey::parse("voxy:air")?,
                render: RenderKind::Invisible,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            },
            BlockDef {
                key: ResourceKey::parse("voxy:stone")?,
                render: RenderKind::Opaque,
                occlusion: Occlusion::FullCube,
                collision: CollisionShape::FullCube,
                face_materials: [MaterialId(1); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 20,
            },
        ])
        .map(Arc::new)
    }

    fn body() -> AnchoredAabb {
        AnchoredAabb {
            anchor: VoxelPos { x: 0, y: 0, z: 0 },
            min: [0.1, 0.1, 0.1],
            max: [0.9, 0.9, 0.9],
        }
    }

    #[test]
    fn sweep_prevents_tunneling_and_returns_surface_normal() {
        let mut view = TestView::default();
        view.blocks.insert(
            VoxelPos { x: 5, y: 0, z: 0 },
            crate::test_support::test_registry()
                .find(&voxy_world::ResourceKey::parse("voxy:stone").unwrap())
                .unwrap(),
        );
        let hit = sweep_aabb(
            &view,
            &registry().unwrap(),
            body(),
            [10.0, 0.0, 0.0],
            SweepConfig::default(),
        )
        .unwrap();
        assert!((hit.fraction - 0.41).abs() < 1.0e-12);
        assert_eq!(hit.normal, [-1, 0, 0]);
        assert!(matches!(hit.obstacle, Some(SweepObstacle::Block { .. })));
    }

    #[test]
    fn empty_path_moves_fully_and_far_anchor_stays_precise() {
        let result = sweep_aabb(
            &TestView::default(),
            &registry().unwrap(),
            AnchoredAabb {
                anchor: VoxelPos {
                    x: 9_000_000_000_000,
                    y: 0,
                    z: 0,
                },
                ..body()
            },
            [3.0, 0.0, 0.0],
            SweepConfig::default(),
        )
        .unwrap();
        assert!((result.fraction - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.obstacle, None);
    }

    #[test]
    fn unloaded_and_unavailable_space_are_solid_boundaries() {
        let unloaded = sweep_aabb(
            &TestView {
                unloaded: true,
                ..TestView::default()
            },
            &registry().unwrap(),
            body(),
            [1.0, 0.0, 0.0],
            SweepConfig::default(),
        )
        .unwrap();
        assert!(unloaded.fraction.abs() < f64::EPSILON);
        assert!(matches!(
            unloaded.obstacle,
            Some(SweepObstacle::Unloaded { .. })
        ));

        let unavailable = sweep_aabb(
            &TestView {
                unavailable: Some(UnavailableReason::Corrupt),
                ..TestView::default()
            },
            &registry().unwrap(),
            body(),
            [1.0, 0.0, 0.0],
            SweepConfig::default(),
        )
        .unwrap();
        assert!(matches!(
            unavailable.obstacle,
            Some(SweepObstacle::Unavailable {
                cause: UnavailableReason::Corrupt,
                ..
            })
        ));
    }

    #[test]
    fn candidate_budget_bounds_work() {
        assert!(matches!(
            sweep_aabb(
                &TestView::default(),
                &registry().unwrap(),
                body(),
                [100.0, 0.0, 0.0],
                SweepConfig {
                    max_candidate_voxels: 2
                }
            ),
            Err(SweepError::CandidateBudgetExceeded { .. })
        ));
    }
}
