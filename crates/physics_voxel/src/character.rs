use crate::{AnchoredAabb, SweepConfig, SweepError, SweepObstacle, sweep_aabb};
pub use physics::{CharacterConfig, CharacterInput};
use voxy_world::{BlockRegistry, VoxelView};
pub type CharacterError = physics::CharacterError<SweepError>;
pub type CharacterContact = physics::CharacterContact<SweepObstacle>;
pub type CharacterStep = physics::CharacterStep<SweepObstacle>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterState {
    pub body: AnchoredAabb,
    pub velocity: [f64; 3],
    pub grounded: bool,
}

/// Collision-query adapter; no world mutation is performed.
#[derive(Debug)]
pub struct VoxelCollisionWorld<'a, V> {
    pub view: &'a V,
    pub registry: &'a BlockRegistry,
}

impl<V: VoxelView> physics::CollisionWorld for VoxelCollisionWorld<'_, V> {
    type Obstacle = SweepObstacle;
    type Error = SweepError;

    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::SweepResult<Self::Obstacle>, Self::Error> {
        let hit = sweep_aabb(
            self.view,
            self.registry,
            from_body(body),
            displacement,
            SweepConfig {
                max_candidate_voxels: max_candidates,
            },
        )?;
        Ok(physics::SweepResult {
            fraction: hit.fraction,
            normal: hit.normal,
            obstacle: hit.obstacle,
        })
    }
}

fn from_body(body: physics::AnchoredAabb) -> AnchoredAabb {
    AnchoredAabb {
        anchor: voxy_core::VoxelPos {
            x: body.anchor.x,
            y: body.anchor.y,
            z: body.anchor.z,
        },
        min: body.min,
        max: body.max,
    }
}

/// Advances the shared controller against voxel collision geometry.
///
/// # Errors
/// Returns input validation, coordinate overflow, or voxel query errors without changing state.
pub fn step_character(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    state: &mut CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<CharacterStep, CharacterError> {
    let mut general = physics::CharacterState {
        body: physics::AnchoredAabb {
            anchor: physics::Origin {
                x: state.body.anchor.x,
                y: state.body.anchor.y,
                z: state.body.anchor.z,
            },
            min: state.body.min,
            max: state.body.max,
        },
        velocity: state.velocity,
        grounded: state.grounded,
    };
    let report = physics::step_character(
        &VoxelCollisionWorld { view, registry },
        &mut general,
        input,
        dt,
        config,
    )?;
    *state = CharacterState {
        body: from_body(general.body),
        velocity: general.velocity,
        grounded: general.grounded,
    };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use voxy_world::{
        BlockDef, BlockStateId, ChunkPos, ChunkSnapshot, CollisionShape, MaterialId, Occlusion,
        RenderKind, ResourceKey, Sample, VoxelPos,
    };

    use super::*;

    struct Geometry {
        solid: BTreeSet<VoxelPos>,
        stone: BlockStateId,
    }

    impl VoxelView for Geometry {
        fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
            Sample::Loaded(if self.solid.contains(&pos) {
                self.stone
            } else {
                BlockStateId::AIR
            })
        }

        fn chunk(&self, _pos: ChunkPos) -> Option<ChunkSnapshot> {
            None
        }
    }

    fn registry() -> BlockRegistry {
        BlockRegistry::new(vec![
            BlockDef {
                key: ResourceKey::parse("voxy:air").unwrap(),
                render: RenderKind::Invisible,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            },
            BlockDef {
                key: ResourceKey::parse("voxy:stone").unwrap(),
                render: RenderKind::Opaque,
                occlusion: Occlusion::FullCube,
                collision: CollisionShape::FullCube,
                face_materials: [MaterialId(1); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 20,
            },
        ])
        .unwrap()
    }

    fn geometry(solid: BTreeSet<VoxelPos>) -> Geometry {
        let blocks = registry();
        let stone = blocks
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        Geometry { solid, stone }
    }

    fn state() -> CharacterState {
        CharacterState {
            body: AnchoredAabb {
                anchor: VoxelPos { x: 0, y: 0, z: 0 },
                min: [0.1, 1.0, 0.1],
                max: [0.9, 2.8, 0.9],
            },
            velocity: [0.0; 3],
            grounded: false,
        }
    }

    #[test]
    fn gravity_lands_and_sets_grounded() {
        let floor = (-2..=2)
            .flat_map(|x| (-2..=2).map(move |z| VoxelPos { x, y: 0, z }))
            .collect();
        let mut state = state();
        let report = step_character(
            &geometry(floor),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [0.0; 2],
                jump_pressed: false,
            },
            1.0 / 60.0,
            CharacterConfig::default(),
        )
        .unwrap();
        assert!(report.grounded);
        assert!(state.grounded);
        assert!(state.velocity[1].abs() < f64::EPSILON);
    }

    #[test]
    fn wall_contact_slides_along_free_axis() {
        let wall = (0..=3)
            .flat_map(|y| (-2..=3).map(move |z| VoxelPos { x: 2, y, z }))
            .collect();
        let mut state = state();
        let config = CharacterConfig {
            gravity: 0.0,
            ..CharacterConfig::default()
        };
        let report = step_character(
            &geometry(wall),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [2.0, 1.0],
                jump_pressed: false,
            },
            1.0,
            config,
        );
        assert_eq!(report, Err(CharacterError::InvalidTimeStep));

        let report = step_character(
            &geometry(
                (0..=3)
                    .flat_map(|y| (-2..=3).map(move |z| VoxelPos { x: 2, y, z }))
                    .collect(),
            ),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [8.0, 4.0],
                jump_pressed: false,
            },
            0.25,
            config,
        )
        .unwrap();
        assert!(
            report
                .contacts
                .iter()
                .any(|contact| contact.normal == [-1, 0, 0])
        );
        assert!(report.applied_displacement[2] > 0.9);
        assert!(state.velocity[0].abs() < f64::EPSILON);
    }

    #[test]
    fn grounded_character_steps_up_full_voxel_when_headroom_is_clear() {
        let mut solid = (-2..=4)
            .flat_map(|x| (-2..=2).map(move |z| VoxelPos { x, y: 0, z }))
            .collect::<BTreeSet<_>>();
        solid.insert(VoxelPos { x: 2, y: 1, z: 0 });
        let mut state = state();
        state.grounded = true;
        let report = step_character(
            &geometry(solid),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [8.0, 0.0],
                jump_pressed: false,
            },
            0.25,
            CharacterConfig {
                gravity: 0.0,
                ..CharacterConfig::default()
            },
        )
        .unwrap();
        assert!(report.stepped_up);
        assert!(report.grounded);
        assert!(report.applied_displacement[0] > 1.9);
        assert!((report.applied_displacement[1] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn step_up_is_rejected_without_headroom() {
        let mut solid = (-2..=4)
            .flat_map(|x| (-2..=2).map(move |z| VoxelPos { x, y: 0, z }))
            .collect::<BTreeSet<_>>();
        solid.insert(VoxelPos { x: 2, y: 1, z: 0 });
        solid.insert(VoxelPos { x: 0, y: 3, z: 0 });
        let mut state = state();
        state.grounded = true;
        let report = step_character(
            &geometry(solid),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [8.0, 0.0],
                jump_pressed: false,
            },
            0.25,
            CharacterConfig {
                gravity: 0.0,
                ..CharacterConfig::default()
            },
        )
        .unwrap();
        assert!(!report.stepped_up);
        assert!(report.applied_displacement[0] < 1.2);
    }

    #[test]
    fn jump_only_uses_grounded_edge_and_far_world_rebases() {
        let mut state = CharacterState {
            body: AnchoredAabb {
                anchor: VoxelPos {
                    x: 9_000_000_000_000,
                    y: 10,
                    z: 0,
                },
                min: [0.9, 0.2, 0.1],
                max: [1.7, 2.0, 0.9],
            },
            velocity: [0.0; 3],
            grounded: true,
        };
        step_character(
            &geometry(BTreeSet::new()),
            &registry(),
            &mut state,
            CharacterInput {
                planar_velocity: [10.0, 0.0],
                jump_pressed: true,
            },
            0.1,
            CharacterConfig::default(),
        )
        .unwrap();
        assert!(state.velocity[1] > 0.0);
        assert!(state.body.anchor.x > 9_000_000_000_000);
        assert!((0.0..1.0).contains(&state.body.min[0]));
    }
}
