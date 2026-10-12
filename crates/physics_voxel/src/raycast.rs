use std::fmt;

use voxy_core::VoxelPos;

use voxy_world::{BlockStateId, Sample, UnavailableReason, VoxelView};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayOrigin {
    /// Integer voxel containing the origin.
    pub voxel: VoxelPos,
    /// Position inside that voxel. Every component must be in `[0, 1)`.
    pub offset: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycastConfig {
    pub max_distance: f64,
    pub max_steps: u32,
}

impl Default for RaycastConfig {
    fn default() -> Self {
        Self {
            max_distance: 8.0,
            max_steps: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelHit {
    pub pos: VoxelPos,
    pub block: BlockStateId,
    /// Outward normal of the face through which the ray entered.
    pub normal: [i8; 3],
    pub distance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RaycastResult {
    Hit(VoxelHit),
    Miss,
    StepBudgetExhausted,
    Unloaded {
        at: VoxelPos,
        chunk: voxy_core::ChunkPos,
    },
    Unavailable {
        at: VoxelPos,
        chunk: voxy_core::ChunkPos,
        cause: UnavailableReason,
    },
}

/// Traverses the integer voxel grid without converting global coordinates to floating point.
///
/// Equal boundary times use the canonical axis order X, then Y, then Z. Starting inside a
/// non-air voxel returns a zero-distance hit with a zero normal.
///
/// # Errors
///
/// Rejects non-finite/zero directions, invalid local offsets or limits, and coordinate overflow.
pub fn raycast(
    view: &impl VoxelView,
    origin: RayOrigin,
    direction: [f64; 3],
    config: RaycastConfig,
) -> Result<RaycastResult, RaycastError> {
    validate(origin, direction, config)?;
    let length = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    let direction = direction.map(|value| value / length);
    let step = direction.map(|value| if value < 0.0 { -1_i64 } else { 1_i64 });
    let delta = direction.map(|value| {
        if value == 0.0 {
            f64::INFINITY
        } else {
            1.0 / value.abs()
        }
    });
    let mut next = std::array::from_fn(|axis| {
        if direction[axis] > 0.0 {
            (1.0 - origin.offset[axis]) * delta[axis]
        } else if direction[axis] < 0.0 {
            origin.offset[axis] * delta[axis]
        } else {
            f64::INFINITY
        }
    });
    let mut voxel = origin.voxel;
    let mut distance = 0.0;
    let mut normal = [0_i8; 3];
    let mut cursor = crate::chunk_cursor::ChunkCursor::new(view);

    for _ in 0..config.max_steps {
        match cursor.sample(voxel) {
            Sample::Loaded(block) if block != BlockStateId::AIR => {
                return Ok(RaycastResult::Hit(VoxelHit {
                    pos: voxel,
                    block,
                    normal,
                    distance,
                }));
            }
            Sample::Loaded(_) => {}
            Sample::Unloaded { chunk } => {
                return Ok(RaycastResult::Unloaded { at: voxel, chunk });
            }
            Sample::Unavailable { chunk, cause } => {
                return Ok(RaycastResult::Unavailable {
                    at: voxel,
                    chunk,
                    cause,
                });
            }
        }

        let axis = smallest_axis(next);
        distance = next[axis];
        if distance > config.max_distance {
            return Ok(RaycastResult::Miss);
        }
        next[axis] += delta[axis];
        let coordinate = match axis {
            0 => &mut voxel.x,
            1 => &mut voxel.y,
            _ => &mut voxel.z,
        };
        *coordinate = coordinate
            .checked_add(step[axis])
            .ok_or(RaycastError::CoordinateOverflow)?;
        normal = [0; 3];
        normal[axis] = if step[axis] > 0 { -1 } else { 1 };
    }
    Ok(RaycastResult::StepBudgetExhausted)
}

fn smallest_axis(values: [f64; 3]) -> usize {
    if values[0] <= values[1] && values[0] <= values[2] {
        0
    } else if values[1] <= values[2] {
        1
    } else {
        2
    }
}

fn validate(
    origin: RayOrigin,
    direction: [f64; 3],
    config: RaycastConfig,
) -> Result<(), RaycastError> {
    if origin
        .offset
        .iter()
        .any(|value| !value.is_finite() || !(0.0..1.0).contains(value))
    {
        return Err(RaycastError::InvalidOriginOffset);
    }
    if direction.iter().any(|value| !value.is_finite())
        || direction.iter().all(|&value| value == 0.0)
    {
        return Err(RaycastError::InvalidDirection);
    }
    if !config.max_distance.is_finite() || config.max_distance < 0.0 || config.max_steps == 0 {
        return Err(RaycastError::InvalidLimits);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaycastError {
    InvalidOriginOffset,
    InvalidDirection,
    InvalidLimits,
    CoordinateOverflow,
}

impl fmt::Display for RaycastError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "raycast error: {self:?}")
    }
}

impl std::error::Error for RaycastError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use voxy_core::{ChunkPos, split_voxel};

    use super::*;
    use voxy_world::ChunkSnapshot;

    #[derive(Default)]
    struct TestView {
        voxels: BTreeMap<VoxelPos, BlockStateId>,
        missing: Option<UnavailableReason>,
    }

    impl VoxelView for TestView {
        fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
            if let Some(&block) = self.voxels.get(&pos) {
                Sample::Loaded(block)
            } else if let Some(cause) = self.missing {
                Sample::Unavailable {
                    chunk: split_voxel(pos).0,
                    cause,
                }
            } else {
                Sample::Loaded(BlockStateId::AIR)
            }
        }

        fn chunk(&self, _pos: ChunkPos) -> Option<ChunkSnapshot> {
            None
        }
    }

    fn origin() -> RayOrigin {
        RayOrigin {
            voxel: VoxelPos { x: 0, y: 0, z: 0 },
            offset: [0.5; 3],
        }
    }

    #[test]
    fn hits_block_with_distance_and_entry_normal() {
        let mut view = TestView::default();
        view.voxels.insert(
            VoxelPos { x: 3, y: 0, z: 0 },
            crate::test_support::test_registry()
                .find(&voxy_world::ResourceKey::parse("voxy:stone").unwrap())
                .unwrap(),
        );
        let result = raycast(&view, origin(), [2.0, 0.0, 0.0], RaycastConfig::default()).unwrap();
        let RaycastResult::Hit(hit) = result else {
            panic!("expected hit");
        };
        assert_eq!(hit.pos, VoxelPos { x: 3, y: 0, z: 0 });
        assert_eq!(hit.normal, [-1, 0, 0]);
        assert!((hit.distance - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ties_step_x_before_y() {
        let mut view = TestView::default();
        view.voxels.insert(
            VoxelPos { x: 1, y: 0, z: 0 },
            crate::test_support::test_registry()
                .find(&voxy_world::ResourceKey::parse("voxy:stone").unwrap())
                .unwrap(),
        );
        assert!(matches!(
            raycast(&view, origin(), [1.0, 1.0, 0.0], RaycastConfig::default()).unwrap(),
            RaycastResult::Hit(VoxelHit {
                normal: [-1, 0, 0],
                ..
            })
        ));
    }

    #[test]
    fn distinguishes_unavailable_and_exhausted_budget() {
        let unavailable = TestView {
            missing: Some(UnavailableReason::Corrupt),
            ..TestView::default()
        };
        assert!(matches!(
            raycast(
                &unavailable,
                origin(),
                [1.0, 0.0, 0.0],
                RaycastConfig::default()
            )
            .unwrap(),
            RaycastResult::Unavailable {
                cause: UnavailableReason::Corrupt,
                ..
            }
        ));
        assert_eq!(
            raycast(
                &TestView::default(),
                origin(),
                [1.0, 0.0, 0.0],
                RaycastConfig {
                    max_distance: 8.0,
                    max_steps: 1
                }
            )
            .unwrap(),
            RaycastResult::StepBudgetExhausted
        );
    }

    #[test]
    fn rejects_invalid_inputs_and_detects_far_world_overflow() {
        assert_eq!(
            raycast(
                &TestView::default(),
                RayOrigin {
                    offset: [1.0, 0.0, 0.0],
                    ..origin()
                },
                [1.0, 0.0, 0.0],
                RaycastConfig::default()
            ),
            Err(RaycastError::InvalidOriginOffset)
        );
        let far = RayOrigin {
            voxel: VoxelPos {
                x: i64::MAX,
                y: 0,
                z: 0,
            },
            offset: [0.5; 3],
        };
        assert_eq!(
            raycast(
                &TestView::default(),
                far,
                [1.0, 0.0, 0.0],
                RaycastConfig::default()
            ),
            Err(RaycastError::CoordinateOverflow)
        );
    }
}
