//! Owner-side integration of named input and the existing swept character solver.
//! Authoring descriptors live in the scene; velocity/grounding are runtime-only.
//! Characters collide with active static boxes, not with each other. Physics owns
//! translations during ticks. A character may rotate; its scale and ancestor
//! rotation/scale must be identity. Static boxes support affine transforms.
mod angular_sweep;
mod foot_contact;
mod staged_tick;
mod support;
pub use foot_contact::{
    FootContactCandidate, FootContactInput, FootContactSettings, FootContactState,
    FootContactStatus,
};
pub use staged_tick::{AcceptedCharacterPose, CharacterTickError, CharacterTickPreview};
pub use support::{SupportAnchor, SupportContact, SupportProbe, SupportQueryBudget, SupportWorld};
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
/// One world displacement followed by a body-local angular arc in radians.
/// Angular magnitude preserves winding; up to four full turns per tick are admitted.
#[derive(Clone, Copy, Debug)]
pub struct CharacterMotion {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub angular_displacement: Vec3,
}
/// Accepted motion excludes preceding input/gravity and includes angular ground snap.
#[derive(Clone, Copy, Debug)]
pub struct AppliedCharacterMotion {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub angular_displacement: Vec3,
    pub angular_fraction: f64,
}

/// Aggregate trajectory storage/work admitted in one fixed tick.
pub const MAX_CHARACTER_TRAJECTORY_SPANS: usize = 4096;
/// World translation followed by a complete rotation path around a fixed pivot.
/// `basis` conjugates the path's initial parent-local frame into body-local axes.
#[derive(Clone, Copy, Debug)]
pub struct CharacterTrajectoryMotion<'a> {
    pub owner: NodeId,
    pub displacement: Vec3,
    pub rotation: &'a voxy_animation::RootRotationPath,
    pub basis: glam::DQuat,
    /// Body-local offset from its center to the stationary rotation pivot.
    /// Zero preserves fixed-center rotation. The center follows the exact curve.
    pub pivot: Vec3,
}
/// Simultaneous translation and rotation, expressed in a rigid source frame.
/// `basis` and `origin` map source coordinates into initial body-local coordinates.
#[derive(Clone, Copy, Debug)]
pub struct CharacterRigidTrajectoryMotion<'a> {
    pub owner: NodeId,
    pub trajectory: &'a voxy_animation::RootRigidPath,
    /// Nonzero signed uniform scale; reflections use a negative scale and proper basis.
    pub scale: f64,
    pub basis: glam::DQuat,
    pub origin: Vec3,
}
/// Owned-field fade trajectory admitted through the staged character tick.
#[derive(Clone, Copy, Debug)]
pub struct CharacterCertifiedFadeMotion<'a> {
    pub owner: NodeId,
    pub fade: &'a voxy_animation::RootRigidCertifiedFadeInterval,
    pub scale: f64,
    pub basis: glam::DQuat,
    pub origin: Vec3,
    pub coordinate_axis: usize,
    /// Caller-supplied world bound for rounded pose evaluation error.
    pub evaluation_radius: f64,
    /// Optional caller-proven errors on each world axis, after all frame mappings.
    /// None uses the whole-body radius on every axis. Bounds must not exceed it.
    pub evaluation_axes: Option<[f64; 3]>,
}
/// A STEP collision may have path_fraction=1 without completing the final event.
/// Use `complete`, `completed_spans` and `span_fraction` for exact admission.
#[derive(Clone, Copy, Debug)]
pub struct AppliedCharacterTrajectoryMotion {
    rigid_identity: Option<usize>,
    pub owner: NodeId,
    pub displacement: Vec3,
    pub rotation: glam::DQuat,
    pub completed_spans: usize,
    pub span_fraction: f64,
    pub path_fraction: f64,
    pub complete: bool,
    pub advancement_iterations: usize,
    pub trajectory_queries: usize,
    proposal_evaluation_error: Option<([f64; 3], f64)>,
}
impl AppliedCharacterTrajectoryMotion {
    /// World-axis and L1 discrepancy for the accepted sweep proposal against
    /// its canonical field. Covers the whole affine body before ground snap,
    /// relocation and f32 scene publication; not a uniform trajectory bound.
    pub fn proposal_evaluation_error_bounds(&self) -> Option<([f64; 3], f64)> {
        self.proposal_evaluation_error
    }

    /// Identity admission within the lifetime of the borrowed rigid request.
    /// The address is never dereferenced and is not a persistent asset ID.
    pub fn matches_rigid_trajectory(&self, path: &voxy_animation::RootRigidPath) -> bool {
        self.rigid_identity == Some(path as *const _ as usize)
    }
}

#[derive(Clone, Copy, Debug)]
struct RuntimeBody {
    state: CharacterState,
    published: Vec3,
    descriptor: CharacterBody,
    edges: [glam::DVec3; 3],
    rest_edges: [glam::DVec3; 3],
    orientation: glam::DQuat,
    published_rotation: Quat,
}
#[derive(Clone, Copy, Debug)]
struct StaticBox {
    half_extents: [f32; 3],
    shape: convex::AffineBox,
    axis_aligned: bool,
    owner: NodeId,
    min: [f64; 3],
    max: [f64; 3],
}
#[derive(Debug)]
struct StaticWorld(Vec<StaticBox>, SceneId);
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
    angular_iterations: usize,
    trajectory_queries: usize,
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
            angular_iterations: 256,
            trajectory_queries: 65_536,
        }
    }
    /// Bounds conservative angular advancement work per requested body.
    /// # Errors
    /// Rejects zero or more than 4096 iterations; exhaustion rejects the staged tick.
    pub fn with_angular_sweep_budget(mut self, iterations: usize) -> Result<Self, PhysicsError> {
        if !(1..=4096).contains(&iterations) {
            return Err(PhysicsError::Capacity);
        }
        self.angular_iterations = iterations;
        Ok(self)
    }
    /// Bounds total curved-trajectory queries over all requested bodies in a tick.
    /// One query admits one span, one whole-interval obstacle test, or one SAT
    /// obstacle distance test. Exhaustion rejects all staged bodies and input.
    /// # Errors
    /// Rejects zero or more than 1,048,576 queries.
    pub fn with_angular_trajectory_query_budget(
        mut self,
        queries: usize,
    ) -> Result<Self, PhysicsError> {
        if !(1..=1_048_576).contains(&queries) {
            return Err(PhysicsError::Capacity);
        }
        self.trajectory_queries = queries;
        Ok(self)
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
        validate_behavior_descriptors(scene).map_err(|_| PhysicsError::InvalidMotion)?;
        if scene.components::<CharacterBody>().count() > self.max_bodies
            || scene.components::<BoxCollider>().count() > self.max_colliders
        {
            return Err(PhysicsError::Capacity);
        }
        for (owner, body) in scene.components::<CharacterBody>() {
            validate_body(*body)?;
            translation(scene, owner)?;
            affine_box(scene, owner, body.half_extents)?;
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
            validate_static_collider(scene, owner, *collider)?;
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
                && overlaps(
                    &fresh(
                        translation(scene, owner)?,
                        *body,
                        affine_box(scene, owner, body.half_extents)?.edges,
                        scene.local(owner)?.rotation,
                    ),
                    &world,
                )
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
    /// Reads the published solver pose without reconstructing its orientation
    /// from narrowed scene values. Unstepped, inactive, detached or externally
    /// edited bodies return None until a new physical tick accepts them.
    pub fn accepted_pose(
        &self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<AcceptedCharacterPose>, PhysicsError> {
        self.validate_scene(scene)?;
        let local = scene.local(owner)?;
        let Some(runtime) = self.states.get(&owner) else {
            return Ok(None);
        };
        if !scene.active_in_hierarchy(owner)?
            || scene.component::<CharacterBody>(owner)? != Some(&runtime.descriptor)
            || local.rotation != runtime.published_rotation
            || translation(scene, owner)? != runtime.published
        {
            return Ok(None);
        }
        Ok(Some(AcceptedCharacterPose {
            owner,
            world_matrix: scene.world_matrix(owner)?,
            physical_center: precise_center(runtime.state.body),
            physical_rotation: runtime.orientation,
            velocity: glam::DVec3::from_array(runtime.state.velocity),
            grounded: runtime.state.grounded,
        }))
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
        if motions.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let requested_owners: HashSet<_> = motions.iter().map(|(owner, _)| *owner).collect();
        let motions: Vec<_> = motions
            .iter()
            .map(|&(owner, displacement)| CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            })
            .collect();
        self.fixed_step_with_rigid_motion(scene, input, dt, &motions)
            .map(|applied| {
                applied
                    .into_iter()
                    .filter(|motion| requested_owners.contains(&motion.owner))
                    .map(|motion| (motion.owner, motion.displacement))
                    .collect()
            })
    }

    /// Sweeps each requested translation, then its complete fixed-center rotation
    /// against active static boxes. Input/gravity runs first. Contact-limited arcs
    /// return their accepted fraction; winding is not discarded at equal endpoints.
    /// Active authored AngularMotion also produces receipts; a simultaneous nonzero
    /// explicit angular request is rejected rather than applied by two writers.
    /// # Errors
    /// Invalid requests and sweep-budget failure preserve all scene/body/input state.
    pub fn fixed_step_with_rigid_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[CharacterMotion],
    ) -> Result<Vec<AppliedCharacterMotion>, PhysicsError> {
        self.fixed_step_with_paths(scene, input, dt, motions, &[], &[], &[], None)
            .map(|(arcs, _)| arcs)
    }

    /// Admits each complete ordered LINEAR/STEP/CUBICSPLINE path in one staged
    /// tick. Contact stops at the first hit; later spans are not applied. Shared
    /// budgets never reset at key boundaries. Input/gravity precedes translation;
    /// grounding is refreshed once after the accepted rotation.
    /// # Errors
    /// Invalid/duplicate/inactive requests, conflicting AngularMotion, aggregate
    /// capacity and solver-budget failure preserve scene, all body state and input.
    pub fn fixed_step_with_trajectory_motion(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        paths: &[CharacterTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_with_motion_and_trajectories(scene, input, dt, &[], paths)
    }

    /// Admits translation-only owners and trajectory owners in the same atomic tick.
    /// A shared owner's translation must agree exactly in both lists; input is
    /// consumed once. Duplicate/conflicting requests preserve all runtime state.
    /// # Errors
    /// Uses the same capacity, coordinate, body, input and solver checks as the
    /// trajectory-only entry point. No request is applied before all admission passes.
    pub fn fixed_step_with_motion_and_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        if paths.len() > self.max_bodies || translations.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut owners =
            HashMap::with_capacity(self.max_bodies.min(translations.len() + paths.len()));
        let mut motions = Vec::with_capacity(self.max_bodies.min(translations.len() + paths.len()));
        for &(owner, displacement) in translations {
            if owners.insert(owner, displacement).is_some() {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            });
        }
        for path in paths {
            if let Some(displacement) = owners.get(&path.owner) {
                if *displacement != path.displacement {
                    return Err(PhysicsError::InvalidMotion);
                }
            } else {
                owners.insert(path.owner, path.displacement);
                if owners.len() > self.max_bodies {
                    return Err(PhysicsError::InvalidMotion);
                }
                motions.push(CharacterMotion {
                    owner: path.owner,
                    displacement: path.displacement,
                    angular_displacement: Vec3::ZERO,
                });
            }
        }
        self.fixed_step_with_paths(scene, input, dt, &motions, paths, &[], &[], None)
            .map(|(_, paths)| paths)
    }

    /// Advances complete composed trajectories through the same atomic character tick.
    /// # Errors
    /// Rejects duplicate owners, invalid frames, conflicting angular writers and
    /// exhausted collision budgets before publishing scene, input or body state.
    pub fn fixed_step_with_rigid_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        paths: &[CharacterRigidTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_with_motion_and_rigid_trajectories(scene, input, dt, &[], paths)
    }

    /// Shares one atomic tick between translation-only owners and composed paths.
    /// # Errors
    /// A composed owner cannot also receive an additive translation request.
    pub fn fixed_step_with_motion_and_rigid_trajectories(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        self.fixed_step_mixed(scene, input, dt, translations, paths, &[], None)
    }

    fn fixed_step_mixed(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        translations: &[(NodeId, Vec3)],
        paths: &[CharacterRigidTrajectoryMotion<'_>],
        certified: &[CharacterCertifiedFadeMotion<'_>],
        prepare: Option<&mut staged_tick::Prepare<'_>>,
    ) -> Result<Vec<AppliedCharacterTrajectoryMotion>, PhysicsError> {
        if paths.len() > self.max_bodies || translations.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut owners = HashSet::new();
        let mut motions = Vec::new();
        for &(owner, displacement) in translations {
            if !owners.insert(owner) {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner,
                displacement,
                angular_displacement: Vec3::ZERO,
            });
        }
        for path in paths {
            if !owners.insert(path.owner) || owners.len() > self.max_bodies {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push(CharacterMotion {
                owner: path.owner,
                displacement: Vec3::ZERO,
                angular_displacement: Vec3::ZERO,
            });
        }
        self.fixed_step_with_paths(scene, input, dt, &motions, &[], paths, certified, prepare)
            .map(|(_, paths)| paths)
    }

    fn fixed_step_with_paths(
        &mut self,
        scene: &mut SceneGraph,
        input: &mut InputMap,
        dt: f64,
        motions: &[CharacterMotion],
        paths: &[CharacterTrajectoryMotion<'_>],
        rigid_paths: &[CharacterRigidTrajectoryMotion<'_>],
        certified: &[CharacterCertifiedFadeMotion<'_>],
        prepare: Option<&mut staged_tick::Prepare<'_>>,
    ) -> Result<
        (
            Vec<AppliedCharacterMotion>,
            Vec<AppliedCharacterTrajectoryMotion>,
        ),
        PhysicsError,
    > {
        self.validate(scene)?;
        let mut requested_paths = HashMap::with_capacity(paths.len());
        let mut total_spans = 0_usize;
        for path in paths {
            total_spans = total_spans
                .checked_add(path.rotation.spans().len())
                .ok_or(PhysicsError::InvalidMotion)?;
            let travel = path.rotation.angular_travel_bound();
            if total_spans > MAX_CHARACTER_TRAJECTORY_SPANS
                || !travel.is_finite()
                || travel > f64::from(4. * std::f32::consts::TAU)
                || !path.basis.is_finite()
                || !path.basis.is_normalized()
                || !path.pivot.is_finite()
                || path.pivot.abs().max_element() > 1e6
                || requested_paths
                    .insert(
                        path.owner,
                        (path.rotation, path.basis.normalize(), path.pivot.as_dvec3()),
                    )
                    .is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut requested_rigid = HashMap::with_capacity(rigid_paths.len());
        for path in rigid_paths {
            total_spans = total_spans
                .checked_add(path.trajectory.spans().len())
                .ok_or(PhysicsError::InvalidMotion)?;
            let travel = path.trajectory.angular_travel_bound();
            if total_spans > MAX_CHARACTER_TRAJECTORY_SPANS
                || !travel.is_finite()
                || travel > f64::from(4. * std::f32::consts::TAU)
                || !path.basis.is_finite()
                || !path.basis.is_normalized()
                || !path.origin.is_finite()
                || path.origin.abs().max_element() > 1e6
                || !path.scale.is_finite()
                || path.scale == 0.
                || path.scale.abs() > 1e6
                || requested_paths.contains_key(&path.owner)
                || requested_rigid.insert(path.owner, *path).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut certified_by_owner = HashMap::with_capacity(certified.len());
        for request in certified {
            let path = requested_rigid
                .get(&request.owner)
                .ok_or(PhysicsError::InvalidMotion)?;
            if !std::ptr::eq(path.trajectory, &request.fade.approximation().path)
                || path.trajectory.duration() != dt
                || request.coordinate_axis >= 3
                || !request.evaluation_radius.is_finite()
                || request.evaluation_radius < 0.
                || request.evaluation_axes.is_some_and(|axes| {
                    axes.iter().any(|value| {
                        !value.is_finite() || *value < 0. || *value > request.evaluation_radius
                    })
                })
                || certified_by_owner.insert(request.owner, request).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut trajectory_queries = self.trajectory_queries;
        let mut applied_paths = Vec::with_capacity(paths.len() + rigid_paths.len());
        if motions.len() > self.max_bodies {
            return Err(PhysicsError::InvalidMotion);
        }
        let active: HashSet<_> = scene
            .active_components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect();
        let mut requested = HashMap::with_capacity(motions.len().min(active.len()));
        for &motion in motions {
            let owner = motion.owner;
            let displacement = motion.displacement;
            if !active.contains(&owner)
                || !displacement.is_finite()
                || displacement.abs().max_element() > 1e6
                || !motion.angular_displacement.is_finite()
                || motion.angular_displacement.as_dvec3().length()
                    > f64::from(4. * std::f32::consts::TAU)
                || requested.insert(owner, motion).is_some()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let mut applied = Vec::with_capacity(motions.len());
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
            return Err(PhysicsError::InvalidStep);
        }
        // Authored axes are parent-local; character ancestors have identity
        // rotation/scale, so this is a world axis. Convert to the API's body axis.
        // Preserve winding and admit exactly one angular source per owner.
        for (owner, _) in scene.active_components::<CharacterBody>() {
            let Some(motion) = scene.component::<AngularMotion>(owner)? else {
                continue;
            };
            let angle = motion.radians_per_second * dt;
            if angle.abs() > f64::from(4. * std::f32::consts::TAU) {
                return Err(PhysicsError::InvalidMotion);
            }
            if angle == 0. {
                continue;
            }
            let request = requested.entry(owner).or_insert(CharacterMotion {
                owner,
                displacement: Vec3::ZERO,
                angular_displacement: Vec3::ZERO,
            });
            if request.angular_displacement != Vec3::ZERO
                || requested_paths.contains_key(&owner)
                || requested_rigid.contains_key(&owner)
            {
                return Err(PhysicsError::InvalidMotion);
            }
            let local_rotation = scene.local(owner)?.rotation;
            let orientation = self
                .states
                .get(&owner)
                .filter(|state| {
                    state.published_rotation == local_rotation
                        && scene.component::<CharacterBody>(owner).ok().flatten()
                            == Some(&state.descriptor)
                        && translation(scene, owner).ok() == Some(state.published)
                })
                .map_or_else(
                    || {
                        glam::DQuat::from_array(local_rotation.to_array().map(f64::from))
                            .normalize()
                    },
                    |state| state.orientation,
                );
            request.angular_displacement = (orientation.conjugate()
                * Vec3::from_array(motion.axis).as_dvec3().normalize()
                * angle)
                .as_vec3();
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
            let edges = affine_box(scene, owner, descriptor.half_extents)?.edges;
            let authored_rotation = scene.local(owner)?.rotation;
            let runtime = next
                .entry(owner)
                .or_insert_with(|| fresh(center, *descriptor, edges, authored_rotation));
            if runtime.published != center
                || runtime.descriptor != *descriptor
                || runtime.published_rotation != authored_rotation
            {
                *runtime = fresh(center, *descriptor, edges, authored_rotation);
            }
            if overlaps(runtime, &world) {
                if !self.depenetration {
                    return Err(PhysicsError::InitialOverlap);
                }
                let mut recovered = precise_center(runtime.state.body);
                let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
                convex::recover_affine(&mut recovered, runtime.edges, &shapes)?;
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
            if !aligned_edges(runtime.edges)
                || world.0.iter().any(|obstacle| !obstacle.axis_aligned)
            {
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
            if let Some(request) = requested.get(&owner) {
                let start = precise_center(runtime.state.body);
                let mut position = start;
                let mut motion = request.displacement.as_dvec3();
                let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
                let mut carried_velocity = glam::DVec3::from_array(runtime.state.velocity);
                let grounded = convex::move_body_affine_carrying_velocity(
                    &mut position,
                    runtime.edges,
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
                let mut angular_fraction = 1.;
                let mut accepted_path = None;
                let angular = if let Some((path, basis, pivot)) = requested_paths.get(&owner) {
                    let hit = angular_sweep::sweep_path(
                        position,
                        runtime.rest_edges,
                        runtime.orientation,
                        path,
                        *basis,
                        *pivot,
                        &shapes,
                        self.angular_iterations,
                        &mut trajectory_queries,
                    )?;
                    position += hit.displacement;
                    angular_fraction = hit.path_fraction;
                    accepted_path = Some(AppliedCharacterTrajectoryMotion {
                        rigid_identity: None,
                        owner,
                        displacement: Vec3::ZERO,
                        rotation: hit.rotation,
                        completed_spans: hit.completed_spans,
                        span_fraction: hit.span_fraction,
                        path_fraction: hit.path_fraction,
                        complete: hit.complete,
                        advancement_iterations: hit.advancement_iterations,
                        trajectory_queries: hit.trajectory_queries,
                        proposal_evaluation_error: hit.pose_evaluation_error,
                    });
                    Some((hit.rotation, hit.normal))
                } else if let Some(path) = requested_rigid.get(&owner) {
                    let hit = if let Some(request) = certified_by_owner.get(&owner) {
                        angular_sweep::sweep_certified_rigid_fade_with_axis_errors(
                            position,
                            runtime.rest_edges,
                            runtime.orientation,
                            request.fade,
                            request.coordinate_axis,
                            path.basis,
                            path.origin.as_dvec3(),
                            path.scale,
                            request.evaluation_radius,
                            request.evaluation_axes,
                            &shapes,
                            self.angular_iterations,
                            &mut trajectory_queries,
                        )?
                    } else {
                        angular_sweep::sweep_rigid_path(
                            position,
                            runtime.rest_edges,
                            runtime.orientation,
                            path.trajectory,
                            path.basis,
                            path.origin.as_dvec3(),
                            path.scale,
                            &shapes,
                            self.angular_iterations,
                            &mut trajectory_queries,
                        )?
                    };
                    position += hit.displacement;
                    angular_fraction = hit.path_fraction;
                    accepted_path = Some(AppliedCharacterTrajectoryMotion {
                        rigid_identity: Some(path.trajectory as *const _ as usize),
                        owner,
                        displacement: Vec3::ZERO,
                        rotation: hit.rotation,
                        completed_spans: hit.completed_spans,
                        span_fraction: hit.span_fraction,
                        path_fraction: hit.path_fraction,
                        complete: hit.complete,
                        advancement_iterations: hit.advancement_iterations,
                        trajectory_queries: hit.trajectory_queries,
                        proposal_evaluation_error: hit.pose_evaluation_error,
                    });
                    Some((hit.rotation, hit.normal))
                } else if request.angular_displacement != Vec3::ZERO {
                    let angular = request.angular_displacement.as_dvec3();
                    let world_angular = convex::rotate_vector(runtime.orientation, angular);
                    let hit = angular_sweep::sweep(
                        position,
                        runtime.edges,
                        world_angular,
                        &shapes,
                        self.angular_iterations,
                    )?;
                    angular_fraction = hit.fraction;
                    Some((
                        glam::DQuat::from_scaled_axis(angular * angular_fraction),
                        hit.normal,
                    ))
                } else {
                    None
                };
                if let Some((rotation, normal)) = angular {
                    runtime.orientation = (runtime.orientation * rotation).normalize();
                    runtime.edges = runtime
                        .rest_edges
                        .map(|edge| convex::rotate_vector(runtime.orientation, edge));
                    runtime.published_rotation =
                        Quat::from_array(runtime.orientation.to_array().map(|value| value as f32))
                            .normalize();
                    if let Some(normal) = normal {
                        let into = carried_velocity.dot(normal);
                        if into < 0. {
                            carried_velocity -= normal * into;
                        }
                    }
                    // Refresh grounding with the accepted shape. Upward carried
                    // velocity still suppresses snap, preserving a pending jump.
                    let mut zero = glam::DVec3::ZERO;
                    runtime.state.grounded = convex::move_body_affine_carrying_velocity(
                        &mut position,
                        runtime.edges,
                        &mut zero,
                        0.,
                        &shapes,
                        Some(&mut carried_velocity),
                    );
                    runtime.state.velocity = carried_velocity.to_array();
                    relocate(runtime, position, *descriptor);
                }
                if let Some(mut receipt) = accepted_path {
                    receipt.displacement = (position - start).as_vec3();
                    applied_paths.push(receipt);
                }
                applied.push(AppliedCharacterMotion {
                    owner,
                    displacement: (position - start).as_vec3(),
                    angular_displacement: request.angular_displacement * angular_fraction as f32,
                    angular_fraction,
                });
            }
            let physical_center = precise_center(runtime.state.body);
            let extent = runtime
                .edges
                .iter()
                .map(|edge| edge.abs())
                .sum::<glam::DVec3>();
            if (physical_center.abs() + extent).max_element() > 1e6 {
                return Err(PhysicsError::CoordinateRange);
            }
            let position = center_of(runtime.state.body)?;
            let mut local = scene.local(owner)?;
            local.translation += position - center;
            local.rotation = runtime.published_rotation;
            local.matrix()?;
            let matrix = local.matrix()?;
            let composed = if let Some(parent) = scene.parent(owner)? {
                scene.world_matrix(parent)? * matrix
            } else {
                matrix
            };
            let published = composed.w_axis.truncate();
            let displayed_extent = [composed.x_axis, composed.y_axis, composed.z_axis]
                .into_iter()
                .zip(descriptor.half_extents)
                .map(|(axis, half)| axis.truncate().as_dvec3().abs() * f64::from(half))
                .sum::<glam::DVec3>();
            if !composed.is_finite()
                || (published.as_dvec3().abs() + displayed_extent).max_element() > 1e6
            {
                return Err(PhysicsError::CoordinateRange);
            }
            runtime.published = published;
            if requested_rigid.contains_key(&owner) {
                // Check both representations after snap/relocation. The rounded
                // scene matrix is the pose the preparation callback/render sees.
                let shapes: Vec<_> = world.0.iter().map(|obstacle| obstacle.shape).collect();
                angular_sweep::certify_published_pose(
                    physical_center,
                    runtime.edges,
                    &shapes,
                    &mut trajectory_queries,
                )?;
                let displayed_edges = [composed.x_axis, composed.y_axis, composed.z_axis]
                    .map(|axis| axis.truncate().as_dvec3());
                let displayed_edges = std::array::from_fn(|i| {
                    displayed_edges[i] * f64::from(descriptor.half_extents[i])
                });
                angular_sweep::certify_published_pose(
                    published.as_dvec3(),
                    displayed_edges,
                    &shapes,
                    &mut trajectory_queries,
                )?;
            }
            edits.push((owner, local));
        }
        if let Some(prepare) = prepare {
            let mut characters = Vec::with_capacity(edits.len());
            for &(owner, local) in &edits {
                let runtime = &next[&owner];
                let world_matrix = if let Some(parent) = scene.parent(owner)? {
                    scene.world_matrix(parent)? * local.matrix()?
                } else {
                    local.matrix()?
                };
                characters.push(AcceptedCharacterPose {
                    owner,
                    world_matrix,
                    physical_center: precise_center(runtime.state.body),
                    physical_rotation: runtime.orientation,
                    velocity: glam::DVec3::from_array(runtime.state.velocity),
                    grounded: runtime.state.grounded,
                });
            }
            characters.sort_by_key(|character| character.owner);
            let support = SupportWorld::from_static_world(&world)?;
            let preview = CharacterTickPreview {
                characters,
                support,
                motions: applied_paths.clone(),
            };
            let mut budget = SupportQueryBudget::new(trajectory_queries.min(65536))?;
            prepare(&preview, &mut budget)?;
        }
        scene.set_locals(&edits)?;
        self.states = next;
        input.finish_frame();
        Ok((applied, applied_paths))
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
    let grounded = convex::move_body_affine_carrying_velocity(
        &mut position,
        runtime.edges,
        &mut velocity,
        dt,
        &shapes,
        None,
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
    let fresh_state = fresh(
        position.as_vec3(),
        descriptor,
        runtime.edges,
        runtime.published_rotation,
    );
    let anchor = fresh_state.state.body.anchor;
    let relative = position - glam::DVec3::new(anchor.x as f64, anchor.y as f64, anchor.z as f64);
    let half = runtime
        .edges
        .iter()
        .map(|edge| edge.abs())
        .sum::<glam::DVec3>();
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
            half_extents: collider.half_extents,
            shape,
            axis_aligned,
            owner,
            min,
            max,
        });
    }
    Ok(StaticWorld(boxes, scene.identity()))
}
fn aligned_edges(edges: [glam::DVec3; 3]) -> bool {
    edges.iter().all(|edge| {
        edge.to_array()
            .into_iter()
            .filter(|value| *value != 0.)
            .count()
            == 1
    })
}
fn overlaps(body: &RuntimeBody, world: &StaticWorld) -> bool {
    let center = precise_center(body.state.body);
    world.0.iter().any(|obstacle| {
        obstacle
            .shape
            .penetration_affine(center, body.edges)
            .is_some()
    })
}
fn contact_epsilon(min: f64, max: f64) -> f64 {
    32.0 * f64::EPSILON * (1.0 + min.abs().max(max.abs()))
}
fn validate_static_collider(
    scene: &SceneGraph,
    owner: NodeId,
    collider: BoxCollider,
) -> Result<(), PhysicsError> {
    validate_extents(collider.half_extents)?;
    affine_box(scene, owner, collider.half_extents)?;
    if scene.component::<CharacterBody>(owner)?.is_some() {
        return Err(PhysicsError::InvalidBody);
    }
    let mut parent = scene.parent(owner)?;
    while let Some(id) = parent {
        if scene.component::<CharacterBody>(id)?.is_some() {
            return Err(PhysicsError::UnsupportedDynamicParent);
        }
        parent = scene.parent(id)?;
    }
    Ok(())
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
            || (id != owner
                && local.rotation != Quat::IDENTITY
                && local.rotation != -Quat::IDENTITY)
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
fn fresh(
    center: Vec3,
    descriptor: CharacterBody,
    edges: [glam::DVec3; 3],
    rotation: Quat,
) -> RuntimeBody {
    let orientation = glam::DQuat::from_array(rotation.to_array().map(f64::from)).normalize();
    let rest_edges = edges.map(|edge| convex::rotate_vector(orientation.conjugate(), edge));
    let half = edges.iter().map(|edge| edge.abs()).sum::<glam::DVec3>();
    let min = (center.as_dvec3() - half).to_array();
    let max = (center.as_dvec3() + half).to_array();
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
        edges,
        rest_edges,
        orientation,
        published_rotation: rotation,
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
