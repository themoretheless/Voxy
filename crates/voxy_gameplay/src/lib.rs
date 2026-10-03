//! Owner-side integration of named input and the existing swept character solver.
//! Authoring descriptors live in the scene; velocity/grounding are runtime-only.
//! Characters collide with active static boxes, not with each other. Physics owns
//! translations during ticks. Rotation/scale of a physics owner or ancestor must
//! be identity; unsupported affine shapes are rejected rather than approximated.
mod audio_assets;
pub use audio_assets::{
    AudioImportConfig, AudioImportSettings, decode_wav_observed, import_wav_asset,
    import_wav_cached, import_wav_with_inputs, synchronize_audio_assets,
};
mod audio_runtime;
pub use audio_runtime::SceneAudioRuntime;
mod audio_scene;
pub use audio_scene::{
    AudioBus, AudioListener, AudioListenerSnapshot, AudioSource, AudioSourceSnapshot,
    SceneAudioSnapshot, extract_scene_audio,
};
mod ui_text;
pub use ui_text::{PreparedUiText, decode_ui_font_observed, prepare_ui_text};
mod ui_actions;
pub use ui_actions::{UiActionHandlers, UiActionSetup};
mod ui_runtime;
pub use ui_runtime::{SceneUiRuntime, UiActionEvent};
mod ui_scene;
pub use ui_scene::{SceneUiSnapshot, UiElement, UiElementSnapshot, UiText, extract_scene_ui};
mod behaviors;
mod convex;
pub use behaviors::{
    AngularMotion, AngularMotionBatch, GameplayFixedError, bind_authored_behaviors,
    gameplay_schedule, validate_behavior_descriptors,
};
use glam::{Quat, Vec3};
use physics::{
    AnchoredAabb, CharacterConfig, CharacterInput, CharacterState, CollisionWorld, Origin,
    SweepResult,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use voxy_input::{Binding, Control, InputMap};
use voxy_scene::{ComponentRegistry, DocumentError, NodeId, SceneGraph, SceneGraphError, SceneId};

/// Validated fixed-tick ownership order shared by the editor and headless game.
/// Synchronization observes behavior edits before physics owns translation writes.
/// Execution remains serial; the scoped runner enforces scene access only.
/// # Errors
/// Returns a validation error if the built-in phase specification is invalid.
pub fn character_schedule() -> Result<&'static voxy_scene::SchedulePlan, voxy_scene::ScheduleError>
{
    static PLAN: std::sync::OnceLock<Result<voxy_scene::SchedulePlan, voxy_scene::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        use voxy_scene::{SystemAccess, SystemSpec};
        voxy_scene::SchedulePlan::build(
            &[
                SystemSpec {
                    name: "character.synchronize".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![
                        SystemAccess {
                            resource: "scene".into(),
                            write: false,
                        },
                        SystemAccess {
                            resource: "character.physics".into(),
                            write: true,
                        },
                    ],
                },
                SystemSpec {
                    name: "character.step".into(),
                    phase: 1,
                    after: vec!["character.synchronize".into()],
                    access: vec![
                        SystemAccess {
                            resource: "scene".into(),
                            write: true,
                        },
                        SystemAccess {
                            resource: "character.physics".into(),
                            write: true,
                        },
                        SystemAccess {
                            resource: "player.input".into(),
                            write: true,
                        },
                    ],
                },
            ],
            2,
        )
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterBody {
    pub half_extents: [f32; 3],
    pub speed: f64,
    pub gravity: f64,
    pub jump_speed: f64,
}
impl Default for CharacterBody {
    fn default() -> Self {
        Self {
            half_extents: [0.05; 3],
            speed: 0.6,
            gravity: -2.4,
            jump_speed: 0.9,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxCollider {
    pub half_extents: [f32; 3],
}
impl Default for BoxCollider {
    fn default() -> Self {
        Self {
            half_extents: [0.05; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsError {
    Scene(SceneGraphError),
    Capacity,
    InvalidBody,
    UnsupportedTransform,
    UnsupportedDynamicParent,
    InitialOverlap,
    CoordinateRange,
    InvalidStep,
    InvalidMotion,
    SweepBudget,
    Solver,
    UnknownSystem,
    AccessDenied,
}
impl From<SceneGraphError> for PhysicsError {
    fn from(error: SceneGraphError) -> Self {
        Self::Scene(error)
    }
}
impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene physics error: {self:?}")
    }
}
impl std::error::Error for PhysicsError {}
/// Adds the durable descriptors; runtime velocity and grounding are never saved.
/// # Errors
/// Rejects duplicate schema names/types.
pub fn register_components(registry: &mut ComponentRegistry) -> Result<(), DocumentError> {
    registry.register::<CharacterBody>("game.character.v1")?;
    registry.register::<BoxCollider>("game.box.v1")?;
    registry.register::<AudioBus>("game.audio-bus.v1")?;
    registry.register::<AudioListener>("game.audio-listener.v1")?;
    registry.register::<AudioSource>("game.audio-source.v1")?;
    registry.register::<AngularMotion>("game.angular-motion.v1")?;
    registry.register::<UiElement>("game.ui-element.v1")?;
    Ok(())
}

/// Common authoring/play preflight for behavior, audio and screen UI descriptors.
/// Physics-specific shape/ownership checks remain with the physics owner.
/// # Errors
/// Rejects invalid descriptor values or extraction budgets without changing scene.
pub fn validate_game_descriptors(scene: &SceneGraph, capacity: usize) -> Result<(), String> {
    validate_behavior_descriptors(scene)?;
    extract_scene_audio(scene, capacity)?;
    extract_scene_ui(scene, [1.0, 1.0], capacity)?;
    Ok(())
}

pub const LEFT: Control = Control { device: 0, code: 1 };
pub const RIGHT: Control = Control { device: 0, code: 2 };
pub const FORWARD: Control = Control { device: 0, code: 3 };
pub const BACK: Control = Control { device: 0, code: 4 };
pub const JUMP: Control = Control { device: 0, code: 5 };
/// Platform adapters translate physical controls into these named actions.
/// Clear edges with `finish_frame` only after a successful fixed tick, never on a
/// render-only frame. This retains quick taps and consumes them once in catch-up.
/// # Errors
/// Propagates binding admission failures.
pub fn player_input() -> Result<InputMap, voxy_input::InputError> {
    let mut input = InputMap::new(131, 5);
    input.bind(
        "move_x",
        vec![
            Binding {
                control: LEFT,
                scale: -1.0,
            },
            Binding {
                control: RIGHT,
                scale: 1.0,
            },
        ],
    )?;
    input.bind(
        "move_z",
        vec![
            Binding {
                control: FORWARD,
                scale: -1.0,
            },
            Binding {
                control: BACK,
                scale: 1.0,
            },
        ],
    )?;
    input.bind(
        "jump",
        vec![Binding {
            control: JUMP,
            scale: 1.0,
        }],
    )?;
    Ok(input)
}
#[derive(Clone, Copy, Debug)]
struct RuntimeBody {
    state: CharacterState,
    published: Vec3,
    descriptor: CharacterBody,
}
#[derive(Clone, Copy, Debug)]
struct StaticBox {
    shape: convex::AffineBox,
    axis_aligned: bool,
    owner: NodeId,
    min: [f64; 3],
    max: [f64; 3],
}
#[derive(Debug)]
struct StaticWorld(Vec<StaticBox>);
impl CollisionWorld for StaticWorld {
    type Obstacle = NodeId;
    type Error = PhysicsError;
    #[allow(clippy::cast_precision_loss)]
    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<SweepResult<NodeId>, PhysicsError> {
        if self.0.len() > max_candidates {
            return Err(PhysicsError::SweepBudget);
        }
        let anchor = [
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ];
        let mut result = SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        };
        for obstacle in &self.0 {
            let min: [f64; 3] = std::array::from_fn(|i| obstacle.min[i] - anchor[i]);
            let max: [f64; 3] = std::array::from_fn(|i| obstacle.max[i] - anchor[i]);
            let mut query = body;
            // The primitive reports inclusive time-zero contact. A controller
            // must allow separating/tangent motion at a touching face. Correct
            // only f64 rebase roundoff, not authored f32 penetrations.
            for axis in 0..3 {
                let epsilon = contact_epsilon(obstacle.min[axis], obstacle.max[axis]);
                let gap = query.min[axis] - max[axis];
                if gap.abs() <= epsilon {
                    query.min[axis] -= gap;
                    query.max[axis] -= gap;
                }
                let gap = query.max[axis] - min[axis];
                if gap.abs() <= epsilon {
                    query.min[axis] -= gap;
                    query.max[axis] -= gap;
                }
            }
            if (0..3).any(|axis| {
                (query.min[axis] >= max[axis] && displacement[axis] >= 0.0)
                    || (query.max[axis] <= min[axis] && displacement[axis] <= 0.0)
            }) {
                continue;
            }
            if let Some((fraction, normal)) = physics::sweep_box(query, displacement, min, max)
                && (fraction < result.fraction || result.obstacle.is_none())
            {
                result = SweepResult {
                    fraction,
                    normal,
                    obstacle: Some(obstacle.owner),
                };
            }
        }
        Ok(result)
    }
}
#[derive(Debug)]
pub struct CharacterPhysics {
    scene: SceneId,
    states: HashMap<NodeId, RuntimeBody>,
    max_bodies: usize,
    max_colliders: usize,
    depenetration: bool,
}
impl CharacterPhysics {
    #[must_use]
    pub fn new(scene: &SceneGraph, max_bodies: usize, max_colliders: usize) -> Self {
        Self {
            scene: scene.identity(),
            states: HashMap::new(),
            max_bodies,
            max_colliders,
            depenetration: false,
        }
    }
    /// Enables bounded overlap recovery on runtime admission and external teleports.
    #[must_use]
    pub fn with_depenetration(mut self, enabled: bool) -> Self {
        self.depenetration = enabled;
        self
    }
    fn validate_scene(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        if scene.identity() != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        Ok(())
    }
    /// Validates authored geometry without starting simulation or changing data.
    /// # Errors
    /// Rejects admission limits, malformed descriptors, incompatible ancestors or
    /// dynamic/static shape combinations. Dynamic ancestors are unsupported.
    pub fn validate(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate_scene(scene)?;
        if scene.components::<CharacterBody>().count() > self.max_bodies
            || scene.components::<BoxCollider>().count() > self.max_colliders
        {
            return Err(PhysicsError::Capacity);
        }
        for (owner, body) in scene.components::<CharacterBody>() {
            validate_body(*body)?;
            translation(scene, owner)?;
            if scene.component::<BoxCollider>(owner)?.is_some() {
                return Err(PhysicsError::InvalidBody);
            }
            let mut parent = scene.parent(owner)?;
            while let Some(id) = parent {
                if scene.component::<CharacterBody>(id)?.is_some() {
                    return Err(PhysicsError::UnsupportedDynamicParent);
                }
                parent = scene.parent(id)?;
            }
        }
        for (owner, collider) in scene.components::<BoxCollider>() {
            validate_extents(collider.half_extents)?;
            affine_box(scene, owner, collider.half_extents)?;
            let mut parent = scene.parent(owner)?;
            while let Some(id) = parent {
                if scene.component::<CharacterBody>(id)?.is_some() {
                    return Err(PhysicsError::UnsupportedDynamicParent);
                }
                parent = scene.parent(id)?;
            }
        }
        Ok(())
    }
    /// Validates new runtime admission, including initial static penetrations.
    /// The swept controller does not perform depenetration; start/teleport poses
    /// must lie outside collider interiors. Exact touching is allowed.
    /// # Errors
    /// Returns descriptor/geometry errors or `InitialOverlap` without mutating data.
    pub fn validate_start(&self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate(scene)?;
        let world = static_world(scene)?;
        for (owner, body) in scene.active_components::<CharacterBody>() {
            if !self.depenetration
                && overlaps(fresh(translation(scene, owner)?, *body).state.body, &world)
            {
                return Err(PhysicsError::InitialOverlap);
            }
        }
        Ok(())
    }
    /// Dispatches a character phase using the scheduler's restricted scene grant.
    /// # Errors
    /// Rejects unknown phases and missing/read-only grants before scene mutation.
    pub fn run_scoped_system(
        &mut self,
        system: &str,
        mut access: voxy_scene::SceneSystemAccess<'_>,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        match system {
            "character.synchronize" => {
                access
                    .require_write("character.physics")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                self.synchronize(access.read().map_err(|_| PhysicsError::AccessDenied)?)
            }
            "character.step" => {
                access
                    .require_write("character.physics")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                access
                    .require_write("player.input")
                    .map_err(|_| PhysicsError::AccessDenied)?;
                self.fixed_step(
                    access.write().map_err(|_| PhysicsError::AccessDenied)?,
                    input,
                    dt,
                )
            }
            _ => Err(PhysicsError::UnknownSystem),
        }
    }
    /// Dispatches one phase of the built-in character plan through its domain owner.
    /// Unknown names fail before mutating physics, scene data or input.
    /// # Errors
    /// Returns `UnknownSystem` or the selected operation's original physics error.
    pub fn run_scheduled_system(
        &mut self,
        system: &str,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        match system {
            "character.synchronize" => self.synchronize(scene),
            "character.step" => self.fixed_step(scene, input, dt),
            _ => Err(PhysicsError::UnknownSystem),
        }
    }
    /// Releases deleted/detached body state at a structural barrier, including a
    /// zero-tick frame. Inactive owners retain velocity but do not advance.
    /// # Errors
    /// Foreign scenes leave all runtime state intact.
    pub fn synchronize(&mut self, scene: &SceneGraph) -> Result<(), PhysicsError> {
        self.validate_scene(scene)?;
        self.states.retain(|owner, _| {
            scene
                .component::<CharacterBody>(*owner)
                .ok()
                .flatten()
                .is_some()
        });
        Ok(())
    }
    #[must_use]
    pub fn body_count(&self) -> usize {
        self.states.len()
    }
    /// # Errors
    /// Rejects foreign/stale owners. An unstepped or detached body returns None.
    pub fn state(
        &self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<CharacterState>, PhysicsError> {
        self.validate_scene(scene)?;
        scene.local(owner)?;
        Ok(self.states.get(&owner).map(|body| body.state))
    }
    /// Advances all active bodies using named input. All bodies and scene edits
    /// publish together after every solver query succeeds. No character contacts
    /// with other characters are implied. External position/descriptor changes
    /// reset velocity and grounding on the next tick.
    /// # Errors
    /// Validation, solver, coordinate or publication failures preserve poses,
    /// runtime state and input edges. Scene coordinate magnitude is limited to 1e6.
    pub fn fixed_step(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
    ) -> Result<(), PhysicsError> {
        self.fixed_step_with_motion(scene, input, dt, &[])
            .map(|_| ())
    }

    /// Applies explicit world-space displacement once, after input/gravity in this tick.
    /// Each motion is swept against the same active static boxes; contacts slide and
    /// ground snap follows the existing controller policy. Returns actual motion only
    /// (excluding the preceding input/gravity movement). Requests are not retained.
    /// Animation owners should publish their clocks only after this operation succeeds.
    ///
    /// # Errors
    /// Duplicate, inactive, stale or non-character owners, nonfinite/excessive motions,
    /// and ordinary physics errors preserve all poses, body states and input edges.
    pub fn fixed_step_with_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[(NodeId, Vec3)],
    ) -> Result<Vec<(NodeId, Vec3)>, PhysicsError> {
        self.validate(scene)?;
        let active: HashSet<_> = scene
            .active_components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect();
        let mut requested = HashMap::with_capacity(motions.len().min(active.len()));
        for &(owner, displacement) in motions {
            if !active.contains(&owner)
                || !displacement.is_finite()
                || displacement.abs().max_element() > 1e6
                || requested.insert(owner, displacement).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut applied = Vec::with_capacity(motions.len());
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
            return Err(PhysicsError::InvalidStep);
        }
        let live: HashSet<_> = scene
            .components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect();
        let mut next = self.states.clone();
        next.retain(|owner, _| live.contains(owner));
        let world = static_world(scene)?;
        let move_x = f64::from(input.state("move_x").map_or(0.0, |state| state.value));
        let move_z = f64::from(input.state("move_z").map_or(0.0, |state| state.value));
        let length = move_x.hypot(move_z).max(1.0);
        let jump_pressed = input.state("jump").is_some_and(|state| state.pressed);
        let mut edits = Vec::new();
        for (owner, descriptor) in scene.active_components::<CharacterBody>() {
            let center = translation(scene, owner)?;
            let runtime = next
                .entry(owner)
                .or_insert_with(|| fresh(center, *descriptor));
            if runtime.published != center || runtime.descriptor != *descriptor {
                *runtime = fresh(center, *descriptor);
            }
            if overlaps(runtime.state.body, &world) {
                if !self.depenetration {
                    return Err(PhysicsError::InitialOverlap);
                }
                let mut recovered = precise_center(runtime.state.body);
                let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
                convex::recover(
                    &mut recovered,
                    Vec3::from_array(descriptor.half_extents).as_dvec3(),
                    &shapes,
                )?;
                relocate(runtime, recovered, *descriptor);
            }
            let config = CharacterConfig {
                gravity: descriptor.gravity,
                jump_speed: descriptor.jump_speed,
                terminal_fall_speed: 100.0,
                step_height: 0.02,
                ground_snap_distance: 0.005,
                max_slide_iterations: 4,
                max_candidates_per_sweep: self.max_colliders.max(1),
            };
            if world.0.iter().any(|obstacle| !obstacle.axis_aligned) {
                step_affine(
                    runtime,
                    *descriptor,
                    &world,
                    [
                        move_x / length * descriptor.speed,
                        move_z / length * descriptor.speed,
                    ],
                    jump_pressed,
                    dt,
                );
            } else {
                physics::step_character(
                    &world,
                    &mut runtime.state,
                    CharacterInput {
                        planar_velocity: [
                            move_x / length * descriptor.speed,
                            move_z / length * descriptor.speed,
                        ],
                        jump_pressed,
                    },
                    dt,
                    config,
                )
                .map_err(|_| PhysicsError::Solver)?;
            }
            if let Some(displacement) = requested.get(&owner) {
                let start = precise_center(runtime.state.body);
                let mut position = start;
                let mut motion = displacement.as_dvec3();
                let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
                let mut carried_velocity = glam::DVec3::from_array(runtime.state.velocity);
                let grounded = convex::move_body_carrying_velocity(
                    &mut position,
                    Vec3::from_array(descriptor.half_extents).as_dvec3(),
                    &mut motion,
                    1.0,
                    &shapes,
                    Some(&mut carried_velocity),
                );
                relocate(runtime, position, *descriptor);
                runtime.state.velocity = carried_velocity.to_array();
                runtime.state.grounded = grounded;
                if grounded && runtime.state.velocity[1] < 0. {
                    runtime.state.velocity[1] = 0.;
                }
                applied.push((owner, (position - start).as_vec3()));
            }
            let position = center_of(runtime.state.body)?;
            let mut local = scene.local(owner)?;
            local.translation += position - center;
            local.matrix()?;
            let matrix = local.matrix()?;
            let composed = if let Some(parent) = scene.parent(owner)? {
                scene.world_matrix(parent)? * matrix
            } else {
                matrix
            };
            let published = composed.w_axis.truncate();
            if !composed.is_finite() || published.abs().max_element() > 1e6 {
                return Err(PhysicsError::CoordinateRange);
            }
            runtime.published = published;
            edits.push((owner, local));
        }
        scene.set_locals(&edits)?;
        self.states = next;
        input.finish_frame();
        Ok(applied)
    }
}
fn step_affine(
    runtime: &mut RuntimeBody,
    descriptor: CharacterBody,
    world: &StaticWorld,
    planar: [f64; 2],
    jump_pressed: bool,
    dt: f64,
) {
    let mut position = precise_center(runtime.state.body);
    let mut velocity = glam::DVec3::from_array(runtime.state.velocity);
    velocity.x = planar[0];
    velocity.z = planar[1];
    if jump_pressed && runtime.state.grounded {
        velocity.y = descriptor.jump_speed;
    }
    velocity.y = (velocity.y + descriptor.gravity * dt).max(-100.);
    let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
    let grounded = convex::move_body(
        &mut position,
        Vec3::from_array(descriptor.half_extents).as_dvec3(),
        &mut velocity,
        dt,
        &shapes,
    );
    relocate(runtime, position, descriptor);
    runtime.state.velocity = velocity.to_array();
    runtime.state.grounded = grounded;
}
#[allow(clippy::cast_precision_loss)]
fn precise_center(body: AnchoredAabb) -> glam::DVec3 {
    glam::DVec3::new(
        body.anchor.x as f64,
        body.anchor.y as f64,
        body.anchor.z as f64,
    ) + (glam::DVec3::from_array(body.min) + glam::DVec3::from_array(body.max)) * 0.5
}
#[allow(clippy::cast_precision_loss)]
fn relocate(runtime: &mut RuntimeBody, position: glam::DVec3, descriptor: CharacterBody) {
    let fresh_state = fresh(position.as_vec3(), descriptor);
    let anchor = fresh_state.state.body.anchor;
    let relative = position - glam::DVec3::new(anchor.x as f64, anchor.y as f64, anchor.z as f64);
    let half = Vec3::from_array(descriptor.half_extents).as_dvec3();
    runtime.state.body = fresh_state.state.body;
    runtime.state.body.min = (relative - half).to_array();
    runtime.state.body.max = (relative + half).to_array();
}
fn static_world(scene: &SceneGraph) -> Result<StaticWorld, PhysicsError> {
    let mut boxes = Vec::new();
    for (owner, collider) in scene.active_components::<BoxCollider>() {
        let shape = affine_box(scene, owner, collider.half_extents)?;
        let extent = shape
            .edges
            .iter()
            .map(|edge| edge.abs())
            .sum::<glam::DVec3>();
        let min = (shape.center - extent).to_array();
        let max = (shape.center + extent).to_array();
        let axis_aligned = shape.edges.iter().all(|edge| {
            edge.to_array()
                .into_iter()
                .filter(|value| *value != 0.)
                .count()
                == 1
        });
        boxes.push(StaticBox {
            shape,
            axis_aligned,
            owner,
            min,
            max,
        });
    }
    Ok(StaticWorld(boxes))
}
#[allow(clippy::cast_precision_loss)]
fn overlaps(body: AnchoredAabb, world: &StaticWorld) -> bool {
    let anchor = [
        body.anchor.x as f64,
        body.anchor.y as f64,
        body.anchor.z as f64,
    ];
    let min = glam::DVec3::from_array(std::array::from_fn(|i| body.min[i] + anchor[i]));
    let max = glam::DVec3::from_array(std::array::from_fn(|i| body.max[i] + anchor[i]));
    world.0.iter().any(|obstacle| {
        obstacle
            .shape
            .penetration((min + max) * 0.5, (max - min) * 0.5)
            .is_some()
    })
}
fn contact_epsilon(min: f64, max: f64) -> f64 {
    32.0 * f64::EPSILON * (1.0 + min.abs().max(max.abs()))
}
fn validate_extents(extents: [f32; 3]) -> Result<(), PhysicsError> {
    if extents
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0 || *value > 1e4)
    {
        return Err(PhysicsError::InvalidBody);
    }
    Ok(())
}
fn validate_body(body: CharacterBody) -> Result<(), PhysicsError> {
    validate_extents(body.half_extents)?;
    if !body.speed.is_finite()
        || !(0.0..=1000.0).contains(&body.speed)
        || !body.gravity.is_finite()
        || !(-1000.0..=0.0).contains(&body.gravity)
        || !body.jump_speed.is_finite()
        || !(0.0..=1000.0).contains(&body.jump_speed)
    {
        return Err(PhysicsError::InvalidBody);
    }
    Ok(())
}
fn affine_box(
    scene: &SceneGraph,
    owner: NodeId,
    extents: [f32; 3],
) -> Result<convex::AffineBox, PhysicsError> {
    let matrix = scene.world_matrix(owner)?;
    let edges = [
        matrix.x_axis.truncate().as_dvec3() * f64::from(extents[0]),
        matrix.y_axis.truncate().as_dvec3() * f64::from(extents[1]),
        matrix.z_axis.truncate().as_dvec3() * f64::from(extents[2]),
    ];
    let center = matrix.w_axis.truncate().as_dvec3();
    let extent = edges.iter().map(|edge| edge.abs()).sum::<glam::DVec3>();
    if !matrix.is_finite()
        || !matrix.inverse().is_finite()
        || edges.iter().any(|edge| edge.length_squared() < 1e-20)
    {
        return Err(PhysicsError::UnsupportedTransform);
    }
    if (center.abs() + extent).max_element() > 1e6 {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(convex::AffineBox { center, edges })
}
fn translation(scene: &SceneGraph, owner: NodeId) -> Result<Vec3, PhysicsError> {
    let mut cursor = Some(owner);
    while let Some(id) = cursor {
        let local = scene.local(id)?;
        if local.scale != Vec3::ONE
            || (local.rotation != Quat::IDENTITY && local.rotation != -Quat::IDENTITY)
        {
            return Err(PhysicsError::UnsupportedTransform);
        }
        cursor = scene.parent(id)?;
    }
    let position = scene.world_matrix(owner)?.w_axis.truncate();
    if position.abs().max_element() > 1e6 {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(position)
}
fn bounds(center: Vec3, extents: [f32; 3]) -> ([f64; 3], [f64; 3]) {
    let center = center.to_array().map(f64::from);
    let extents = extents.map(f64::from);
    (
        std::array::from_fn(|i| center[i] - extents[i]),
        std::array::from_fn(|i| center[i] + extents[i]),
    )
}
fn fresh(center: Vec3, descriptor: CharacterBody) -> RuntimeBody {
    let (min, max) = bounds(center, descriptor.half_extents);
    RuntimeBody {
        state: CharacterState {
            body: AnchoredAabb {
                anchor: Origin::default(),
                min,
                max,
            },
            velocity: [0.0; 3],
            grounded: false,
        },
        published: center,
        descriptor,
    }
}
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn center_of(body: AnchoredAabb) -> Result<Vec3, PhysicsError> {
    let anchor = [
        body.anchor.x as f64,
        body.anchor.y as f64,
        body.anchor.z as f64,
    ];
    let position: [f64; 3] = std::array::from_fn(|i| anchor[i] + (body.min[i] + body.max[i]) * 0.5);
    if position
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(Vec3::from_array(position.map(|value| value as f32)))
}
