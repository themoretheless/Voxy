use std::fmt;

use voxy_world::{
    AnchoredAabb, BlockRegistry, SweepConfig, SweepError, SweepObstacle, VoxelView, sweep_aabb,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterState {
    pub body: AnchoredAabb,
    pub velocity: [f64; 3],
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterInput {
    /// Desired X/Z velocity for this simulation tick.
    pub planar_velocity: [f64; 2],
    pub jump_pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterConfig {
    pub gravity: f64,
    pub jump_speed: f64,
    pub terminal_fall_speed: f64,
    pub step_height: f64,
    pub ground_snap_distance: f64,
    pub max_slide_iterations: u8,
    pub max_candidate_voxels_per_sweep: usize,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            gravity: -24.0,
            jump_speed: 8.5,
            terminal_fall_speed: 48.0,
            step_height: 1.01,
            ground_snap_distance: 0.1,
            max_slide_iterations: 4,
            max_candidate_voxels_per_sweep: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterContact {
    pub normal: [i8; 3],
    pub obstacle: SweepObstacle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterStep {
    pub requested_displacement: [f64; 3],
    pub applied_displacement: [f64; 3],
    pub contacts: Vec<CharacterContact>,
    pub grounded: bool,
    pub stepped_up: bool,
}

/// Advances a character using a bounded iterative sweep-and-slide solver.
///
/// Horizontal input is a desired velocity rather than an acceleration, making input sampling
/// independent from frame rate. The caller supplies the fixed simulation `dt`; render
/// interpolation remains outside this authoritative step.
///
/// # Errors
///
/// Rejects invalid configuration/input/timestep, coordinate rebasing overflow, or voxel sweep
/// failures. State is only published after the full step succeeds.
pub fn step_character(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    state: &mut CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<CharacterStep, CharacterError> {
    validate(state, input, dt, config)?;
    let mut next = *state;
    let step_start = next.body;
    let may_step = next.grounded
        && squared_length([input.planar_velocity[0], 0.0, input.planar_velocity[1]]) > f64::EPSILON;
    next.velocity[0] = input.planar_velocity[0];
    next.velocity[2] = input.planar_velocity[1];
    if input.jump_pressed && next.grounded {
        next.velocity[1] = config.jump_speed;
        next.grounded = false;
    }
    next.velocity[1] = (next.velocity[1] + config.gravity * dt).max(-config.terminal_fall_speed);
    let requested = next.velocity.map(|velocity| velocity * dt);
    let mut remaining = requested;
    let mut applied = [0.0; 3];
    let mut contacts = Vec::new();
    let mut grounded = false;
    for _ in 0..config.max_slide_iterations {
        if squared_length(remaining) <= f64::EPSILON {
            break;
        }
        let hit = sweep_aabb(
            view,
            registry,
            next.body,
            remaining,
            SweepConfig {
                max_candidate_voxels: config.max_candidate_voxels_per_sweep,
            },
        )?;
        let movement = remaining.map(|value| value * hit.fraction);
        translate(&mut next.body, movement);
        add_assign(&mut applied, movement);
        let Some(obstacle) = hit.obstacle else {
            break;
        };
        contacts.push(CharacterContact {
            normal: hit.normal,
            obstacle,
        });
        if hit.normal[1] > 0 {
            grounded = true;
        }
        clip_against_plane(&mut next.velocity, hit.normal);
        remaining = remaining.map(|value| value * (1.0 - hit.fraction));
        clip_against_plane(&mut remaining, hit.normal);
    }
    let regular_horizontal = applied[0] * applied[0] + applied[2] * applied[2];
    let stepped = if may_step && contacts.iter().any(|contact| contact.normal[1] == 0) {
        try_step_up(view, registry, step_start, requested, config)?
    } else {
        None
    };
    let mut stepped_up = false;
    if let Some(candidate) = stepped {
        let candidate_horizontal = candidate.applied[0] * candidate.applied[0]
            + candidate.applied[2] * candidate.applied[2];
        if candidate_horizontal > regular_horizontal {
            next.body = candidate.body;
            applied = candidate.applied;
            contacts = candidate.contacts;
            grounded = true;
            next.velocity[1] = 0.0;
            stepped_up = true;
        }
    }
    next.grounded = grounded;
    rebase(&mut next.body)?;
    *state = next;
    Ok(CharacterStep {
        requested_displacement: requested,
        applied_displacement: applied,
        contacts,
        grounded,
        stepped_up,
    })
}

#[derive(Debug)]
struct StepCandidate {
    body: AnchoredAabb,
    applied: [f64; 3],
    contacts: Vec<CharacterContact>,
}

fn try_step_up(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    start: AnchoredAabb,
    requested: [f64; 3],
    config: CharacterConfig,
) -> Result<Option<StepCandidate>, CharacterError> {
    if config.step_height == 0.0 {
        return Ok(None);
    }
    let sweep_config = SweepConfig {
        max_candidate_voxels: config.max_candidate_voxels_per_sweep,
    };
    let lift = [0.0, config.step_height, 0.0];
    let lift_result = sweep_aabb(view, registry, start, lift, sweep_config)?;
    if lift_result.obstacle.is_some() {
        return Ok(None);
    }
    let mut body = start;
    translate(&mut body, lift);
    let horizontal = [requested[0], 0.0, requested[2]];
    let horizontal_result = sweep_aabb(view, registry, body, horizontal, sweep_config)?;
    if horizontal_result.fraction < 1.0 {
        return Ok(None);
    }
    translate(&mut body, horizontal);
    let drop = [
        0.0,
        -(config.step_height + config.ground_snap_distance),
        0.0,
    ];
    let drop_result = sweep_aabb(view, registry, body, drop, sweep_config)?;
    let Some(obstacle) = drop_result.obstacle else {
        return Ok(None);
    };
    if drop_result.normal != [0, 1, 0] {
        return Ok(None);
    }
    let vertical = lift[1] + drop[1] * drop_result.fraction;
    translate(&mut body, [0.0, drop[1] * drop_result.fraction, 0.0]);
    Ok(Some(StepCandidate {
        body,
        applied: [horizontal[0], vertical, horizontal[2]],
        contacts: vec![CharacterContact {
            normal: drop_result.normal,
            obstacle,
        }],
    }))
}

fn translate(body: &mut AnchoredAabb, displacement: [f64; 3]) {
    for (axis, value) in displacement.into_iter().enumerate() {
        body.min[axis] += value;
        body.max[axis] += value;
    }
}

fn add_assign(target: &mut [f64; 3], value: [f64; 3]) {
    for (target, value) in target.iter_mut().zip(value) {
        *target += value;
    }
}

fn clip_against_plane(vector: &mut [f64; 3], normal: [i8; 3]) {
    let dot = vector
        .iter()
        .zip(normal)
        .map(|(value, normal)| value * f64::from(normal))
        .sum::<f64>();
    if dot < 0.0 {
        for (value, normal) in vector.iter_mut().zip(normal) {
            *value -= f64::from(normal) * dot;
        }
    }
}

fn rebase(body: &mut AnchoredAabb) -> Result<(), CharacterError> {
    for axis in 0..3 {
        #[allow(clippy::cast_possible_truncation)]
        let shift = body.min[axis].floor() as i64;
        let anchor = match axis {
            0 => &mut body.anchor.x,
            1 => &mut body.anchor.y,
            _ => &mut body.anchor.z,
        };
        *anchor = anchor
            .checked_add(shift)
            .ok_or(CharacterError::CoordinateOverflow)?;
        #[allow(clippy::cast_precision_loss)]
        let shift = shift as f64;
        body.min[axis] -= shift;
        body.max[axis] -= shift;
    }
    Ok(())
}

fn squared_length(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum()
}

fn validate(
    state: &CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<(), CharacterError> {
    let finite_state = state.velocity.iter().all(|value| value.is_finite())
        && state.body.min.iter().all(|value| value.is_finite())
        && state.body.max.iter().all(|value| value.is_finite());
    let finite_input = input.planar_velocity.iter().all(|value| value.is_finite());
    if !finite_state || !finite_input {
        return Err(CharacterError::NonFiniteState);
    }
    if !dt.is_finite() || !(0.0..=0.25).contains(&dt) || dt == 0.0 {
        return Err(CharacterError::InvalidTimeStep);
    }
    if !config.gravity.is_finite()
        || config.gravity > 0.0
        || !config.jump_speed.is_finite()
        || config.jump_speed < 0.0
        || !config.terminal_fall_speed.is_finite()
        || config.terminal_fall_speed <= 0.0
        || !config.step_height.is_finite()
        || !(0.0..=2.0).contains(&config.step_height)
        || !config.ground_snap_distance.is_finite()
        || !(0.0..=1.0).contains(&config.ground_snap_distance)
        || !(1..=16).contains(&config.max_slide_iterations)
        || config.max_candidate_voxels_per_sweep == 0
    {
        return Err(CharacterError::InvalidConfig);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterError {
    NonFiniteState,
    InvalidTimeStep,
    InvalidConfig,
    CoordinateOverflow,
    Sweep(SweepError),
}

impl From<SweepError> for CharacterError {
    fn from(error: SweepError) -> Self {
        Self::Sweep(error)
    }
}

impl fmt::Display for CharacterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "character controller error: {self:?}")
    }
}

impl std::error::Error for CharacterError {}

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
