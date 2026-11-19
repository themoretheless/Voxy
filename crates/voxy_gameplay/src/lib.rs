//! Owner-side integration of named input and the existing swept character solver.
//! Authoring descriptors live in the scene; velocity/grounding are runtime-only.
//! Characters collide with active static boxes, not with each other. Physics owns
//! translations during ticks. A character may rotate; its scale and ancestor
//! rotation/scale must be identity. Static boxes support affine transforms.

mod angular_sweep;
mod audio_assets;
mod audio_runtime;
mod audio_scene;
mod behaviors;
mod character_checkpoint;
mod character_physics;
mod character_world;
mod convex;
mod error;
mod follow_camera;
mod foot_contact;
pub mod foot_placement;
mod gameplay_checkpoint;
mod input_checkpoint;
mod joints;
mod kinematic;
mod liquid_body;
mod liquid_runtime;
mod liquid_source;
mod navigation;
mod point_loads;
mod rigid_frame;
mod staged_tick;
mod support;
mod ui_actions;
mod ui_runtime;
mod ui_scene;
mod ui_text;

pub use audio_assets::{
    AudioImportConfig, AudioImportSettings, decode_wav_observed, import_wav_asset,
    import_wav_cached, import_wav_with_inputs, synchronize_audio_assets,
};
pub use audio_runtime::{SceneAudioCheckpoint, SceneAudioCheckpointLimits, SceneAudioRuntime};
pub use audio_scene::{
    AudioBus, AudioListener, AudioListenerSnapshot, AudioSource, AudioSourceSnapshot,
    SceneAudioSnapshot, compute_doppler_pitch_shift, extract_scene_audio,
    inverse_distance_windowed_attenuation,
};
pub use behaviors::{
    AngularMotion, AngularMotionBatch, AngularMotionCheckpoint, GameplayFixedError,
    bind_authored_behaviors, gameplay_schedule, validate_behavior_descriptors,
};
pub use character_checkpoint::{CharacterCheckpointError, CharacterPhysicsCheckpoint};
pub use character_physics::{
    AppliedCharacterMotion, AppliedCharacterTrajectoryMotion, CharacterCertifiedFadeMotion,
    CharacterMotion, CharacterPhysics, CharacterRigidTrajectoryMotion, CharacterTrajectoryMotion,
    CharacterTranslationTrace, MAX_CHARACTER_TRAJECTORY_SPANS, rigid_source_coordinate_axis,
};
pub(crate) use character_world::{
    RuntimeBody, StaticBox, StaticWorld, affine_box, center_of, precise_center, static_world,
    validate_body, validate_extents, validate_static_collider,
};
pub use error::GameplayError;
pub use follow_camera::{FollowCamera, FollowSmoother, FollowSmootherCheckpoint};
pub use foot_contact::{
    FootContactCandidate, FootContactCheckpoint, FootContactCheckpointError, FootContactInput,
    FootContactSettings, FootContactState, FootContactStatus, FootPivotTransfer,
    elastic_support_rebound_velocity,
};
pub use foot_placement::{
    FootBinding, FootContactKey, FootOrientation, FootPivotKey, FootRuntime, FootSupportGeometry,
    ModelFootPlacement, SourceContactInterval,
};
pub use gameplay_checkpoint::{
    GameplayCheckpoint, GameplayCheckpointError, GameplayCheckpointLimits,
    PreparedGameplayCheckpoint,
};
pub use input_checkpoint::{
    InputCheckpointCodecError, decode_input_checkpoint, encode_input_checkpoint,
};
pub use joints::{
    PhysicsJoint, PhysicsJointKind, collect_physics_joints, solve_distance_spring_position,
    step_physics_joints, step_scene_physics_joints, validate_physics_joints,
};
pub use kinematic::{
    KinematicBody, PlatformMotion, step_kinematic_bodies, step_kinematic_bodies_with_dynamics,
};
pub use liquid_body::{LiquidBody, LiquidMassDistribution, LiquidMassPart};
pub use liquid_runtime::{
    SceneLiquidCheckpoint, SceneLiquidCheckpointLimits, SceneLiquidRuntime, SceneLiquidStep,
    SceneLiquidWireLimits, SceneSupportReactions, SceneSupportedLiquidStep,
};
pub use liquid_source::{LiquidPulse, LiquidSource};
pub use navigation::{
    NavigationFollower, NpcDestination, NpcRoute, SceneNavigationError, SceneNavigationGrid,
    SceneNavigationSettings, prepare_npc_routes, step_navigation_followers,
};
pub use point_loads::{RigidPointForce, RigidPointLoads};
pub use rigid_frame::{PreparedRigidPose, RigidBodyFrame, RigidPoseEdit, publish_rigid_poses};
pub use staged_tick::{AcceptedCharacterPose, CharacterTickError, CharacterTickPreview};
pub use support::{
    AngularBoxHit, BoxSweepHit, RayHit, RayQuery, SceneQueries, SupportAnchor,
    SupportAnchorCheckpoint, SupportContact, SupportProbe, SupportQueryBudget, SupportWorld,
    slope_traction_acceleration_scale,
};
mod triggers;
pub use triggers::{
    BoxTrigger, MAX_PAIRS as MAX_TRIGGER_PAIRS, TriggerEvent, TriggerPhase, TriggerTracker,
};
pub use ui_actions::{
    UiActionCheckpoint, UiActionCheckpointLimits, UiActionCheckpointRegistry, UiActionHandler,
    UiActionHandlers, UiActionRestoreContext, UiActionSetup, UiActionState,
};
pub use ui_runtime::{SceneUiCheckpoint, SceneUiRuntime, SceneUiSnapshotCheckpoint, UiActionEvent};
pub use ui_scene::{SceneUiSnapshot, UiElement, UiElementSnapshot, UiText, extract_scene_ui};
pub use ui_text::{PreparedUiText, decode_ui_font_observed, prepare_ui_text};

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxy_input::{Binding, Control, InputMap};
use voxy_scene::{ComponentRegistry, DocumentError, SceneGraph, SceneGraphError};

/// Symmetric collision filtering. A pair interacts only when each mask
/// accepts at least one bit of the other participant's membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollisionLayers {
    pub membership: u32,
    pub mask: u32,
}

impl Default for CollisionLayers {
    fn default() -> Self {
        Self {
            membership: 1,
            mask: u32::MAX,
        }
    }
}

impl CollisionLayers {
    #[must_use]
    pub const fn interacts(self, other: Self) -> bool {
        self.mask & other.membership != 0 && other.mask & self.membership != 0
    }

    pub(crate) fn on(scene: &SceneGraph, owner: voxy_scene::NodeId) -> Self {
        scene
            .component::<Self>(owner)
            .ok()
            .flatten()
            .copied()
            .unwrap_or_default()
    }
}

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
    ContactWitness,
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

/// Registers the follow descriptor and remaps prefab-local target identities.
/// # Errors
/// Rejects duplicate schema names or component types.
pub fn register_follow_camera(registry: &mut ComponentRegistry) -> Result<(), DocumentError> {
    registry.register_with_reference_remap::<FollowCamera>(
        "game.follow-camera.v1",
        |camera| vec![camera.target.clone()],
        |camera, ids| {
            if let Some(target) = ids.get(&camera.target) {
                camera.target = target.clone();
            }
        },
    )
}

/// Registers the `PhysicsJoint` descriptor and remaps prefab-local target identities.
/// # Errors
/// Rejects duplicate schema names or component types.
pub fn register_physics_joint(registry: &mut ComponentRegistry) -> Result<(), DocumentError> {
    registry.register_with_reference_remap::<PhysicsJoint>(
        "game.physics-joint.v1",
        |joint| vec![joint.target.clone()],
        |joint, ids| {
            if let Some(target) = ids.get(&joint.target) {
                joint.target = target.clone();
            }
        },
    )
}

/// Adds authoring descriptors; controller velocity/grounding use separate runtime checkpoints.
/// # Errors
/// Rejects duplicate schema names/types.
pub fn register_components(registry: &mut ComponentRegistry) -> Result<(), DocumentError> {
    register_follow_camera(registry)?;
    register_physics_joint(registry)?;
    registry.register::<RigidPointLoads>("game.rigid-point-loads.v1")?;
    registry.register::<LiquidBody>("game.liquid-body.v1")?;
    registry.register::<LiquidMassDistribution>("game.liquid-mass.v1")?;
    registry.register::<LiquidSource>("game.liquid-source.v1")?;
    registry.register::<KinematicBody>("game.kinematic-body.v1")?;
    registry.register::<CharacterBody>("game.character.v1")?;
    registry.register::<NpcRoute>("game.npc-route.v1")?;
    registry.register::<NpcDestination>("game.npc-destination.v1")?;
    registry.register::<BoxCollider>("game.box.v1")?;
    registry.register::<CollisionLayers>("game.collision-layers.v1")?;
    registry.register::<BoxTrigger>("game.box-trigger.v1")?;
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
pub fn validate_game_descriptors(scene: &SceneGraph, capacity: usize) -> Result<(), GameplayError> {
    if scene.components::<RigidPointLoads>().next().is_some() {
        return Err("rigid point loads require a point-load contact runtime".into());
    }
    for (_, source) in scene.components::<LiquidSource>() {
        source
            .prepare([0.; 3], 0)
            .map_err(|e| format!("invalid game.liquid-source.v1: {e:?}"))?;
    }
    if scene.components::<LiquidSource>().next().is_some()
        || scene.components::<LiquidBody>().next().is_some()
        || scene
            .components::<LiquidMassDistribution>()
            .next()
            .is_some()
    {
        return Err("game.liquid-source.v1 requires an attached scene liquid runtime".into());
    }
    validate_common_game_descriptors(scene, capacity)
}

/// Preflight for hosts that attach and tick the admitted liquid owner.
/// # Errors
/// Invalid shared descriptors, foreign runtime or changed liquid source bindings.
pub fn validate_game_descriptors_with_liquid_runtime(
    scene: &SceneGraph,
    capacity: usize,
    runtime: &SceneLiquidRuntime,
) -> Result<(), GameplayError> {
    runtime.validate_bindings(scene)?;
    validate_common_game_descriptors(scene, capacity)
}

fn validate_common_game_descriptors(
    scene: &SceneGraph,
    capacity: usize,
) -> Result<(), GameplayError> {
    if scene
        .components::<FollowCamera>()
        .any(|(_, camera)| !camera.valid())
    {
        return Err("invalid game.follow-camera.v1 descriptor".into());
    }
    if scene
        .components::<PhysicsJoint>()
        .any(|(_, joint)| !joint.valid())
    {
        return Err("invalid game.physics-joint.v1 descriptor".into());
    }
    validate_behavior_descriptors(scene)?;
    kinematic::validate_kinematic_bodies(scene)?;
    triggers::validate_triggers(scene, capacity)
        .map_err(|error| format!("invalid game.box-trigger.v1 descriptor: {error}"))?;
    extract_scene_audio(scene, capacity)?;
    extract_scene_ui(scene, [1.0, 1.0], capacity)?;
    Ok(())
}

pub const LEFT: Control = Control { device: 0, code: 1 };
pub const RIGHT: Control = Control { device: 0, code: 2 };
pub const FORWARD: Control = Control { device: 0, code: 3 };
pub const BACK: Control = Control { device: 0, code: 4 };
pub const JUMP: Control = Control { device: 0, code: 5 };

pub const GAMEPAD_X: Control = Control { device: 1, code: 1 };
pub const GAMEPAD_Z: Control = Control { device: 1, code: 2 };
pub const GAMEPAD_JUMP: Control = Control { device: 1, code: 3 };

/// Platform adapters translate physical controls into these named actions.
/// Clear edges with `finish_frame` only after a successful fixed tick, never on a
/// render-only frame. This retains quick taps and consumes them once in catch-up.
/// # Errors
/// Propagates binding admission failures.
pub fn player_input() -> Result<InputMap, voxy_input::InputError> {
    build_player_input(false)
}

/// Adds independent signed movement axes and a jump button to the standard actions.
/// # Errors
/// Propagates binding admission failures.
pub fn player_input_with_gamepad() -> Result<InputMap, voxy_input::InputError> {
    build_player_input(true)
}

fn build_player_input(gamepad: bool) -> Result<InputMap, voxy_input::InputError> {
    let mut input = InputMap::new(131, if gamepad { 8 } else { 5 });
    let mut move_x = vec![
        Binding {
            control: LEFT,
            scale: -1.0,
        },
        Binding {
            control: RIGHT,
            scale: 1.0,
        },
    ];
    let mut move_z = vec![
        Binding {
            control: FORWARD,
            scale: -1.0,
        },
        Binding {
            control: BACK,
            scale: 1.0,
        },
    ];
    let mut jump_bindings = vec![Binding {
        control: JUMP,
        scale: 1.0,
    }];
    if gamepad {
        move_x.push(Binding {
            control: GAMEPAD_X,
            scale: 1.0,
        });
        move_z.push(Binding {
            control: GAMEPAD_Z,
            scale: 1.0,
        });
        jump_bindings.push(Binding {
            control: GAMEPAD_JUMP,
            scale: 1.0,
        });
    }
    input.bind("move_x", move_x)?;
    input.bind("move_z", move_z)?;
    input.bind("jump", jump_bindings)?;
    Ok(input)
}

/// Evaluates a normalized logistic utility response curve in `[0.0, 1.0]` for AI decision scoring.
#[must_use]
pub fn utility_response_curve(input: f32, slope: f32, midpoint: f32) -> f32 {
    if !input.is_finite() || !slope.is_finite() || !midpoint.is_finite() {
        return 0.0;
    }
    let x = input.clamp(0.0, 1.0);
    let m = midpoint.clamp(0.0, 1.0);
    let k = slope.clamp(-50.0, 50.0);
    (1.0 / (1.0 + (-k * (x - m)).exp())).clamp(0.0, 1.0)
}

/// Computes an AI sensory perception visibility factor in `[0.0, 1.0]` based on distance and vision cone angle.
#[must_use]
pub fn perception_cone_visibility(
    eye: [f32; 3],
    forward: [f32; 3],
    target: [f32; 3],
    max_range: f32,
    half_fov_cos: f32,
) -> f32 {
    if !max_range.is_finite() || max_range <= 0.0 || !half_fov_cos.is_finite() {
        return 0.0;
    }
    let eye_v = Vec3::from_array(eye);
    let Some(fwd_v) = Vec3::from_array(forward).try_normalize() else {
        return 0.0;
    };
    let delta = Vec3::from_array(target) - eye_v;
    let dist = delta.length();
    if !dist.is_finite() || dist > max_range {
        return 0.0;
    }
    if dist <= 1e-5 {
        return 1.0;
    }
    let dir = delta / dist;
    let cos_theta = fwd_v.dot(dir);
    let min_cos = half_fov_cos.clamp(-1.0, 0.9999);
    if cos_theta < min_cos {
        return 0.0;
    }
    let angular = ((cos_theta - min_cos) / (1.0 - min_cos)).clamp(0.0, 1.0);
    let radial = (1.0 - dist / max_range).clamp(0.0, 1.0);
    (angular * radial).clamp(0.0, 1.0)
}

#[cfg(test)]
mod n8_gameplay_ai_tests {
    use super::*;

    #[test]
    fn utility_curve_and_perception_cone_score_targets_monotonically() {
        let low = utility_response_curve(0.1, 10.0, 0.5);
        let mid = utility_response_curve(0.5, 10.0, 0.5);
        let high = utility_response_curve(0.9, 10.0, 0.5);
        assert!(low < mid && mid < high);
        assert!((mid - 0.5).abs() < 1e-6);

        let vis_center = perception_cone_visibility(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 5.0],
            10.0,
            0.5,
        );
        let vis_behind = perception_cone_visibility(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -5.0],
            10.0,
            0.5,
        );
        assert!((vis_center - 0.5).abs() < 1e-5);
        assert_eq!(vis_behind, 0.0);
    }
}
