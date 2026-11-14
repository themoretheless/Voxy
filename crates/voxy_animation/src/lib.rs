//! Validated skeletal animation sampling and skin-matrix generation.

mod anim_graph;
mod animator;
mod asset_checkpoint;
mod blend_phase;
mod blend_space2d;
mod checkpoint;
mod clip;
mod compressed;
mod contact_bake;
mod crowd;
mod events;
mod exact_quaternion;
mod foot_plant;
mod foot_support;
mod gait;
mod gpu_layout;
mod graph;
mod ik;
mod linear_rank;
mod loop_closure;
mod pose;
mod pose_archive;
mod precision;
mod property_playback;
mod property_track;
mod ragdoll;
mod retarget;
mod rigid_fade;
mod root_clock;
mod root_curve;
mod root_motion_receipt;
mod root_rigid;
mod root_rotation;
mod skeleton;
mod soa;
mod state_machine;

pub use anim_graph::{
    ActiveWindow, AnimGraph, DEFAULT_EVENT_BUDGET, FloatInput, GraphDesc, GraphEvent, GraphFrame,
    GraphNode, GraphPlayer, IkSolver, MAX_GRAPH_NODES, NodeId, ParamId, ParamValue, Vec3Input,
};
pub(crate) use animator::Transition;
pub use animator::{AnimationPhaseInterval, Animator, AnimatorFrame};
pub use asset_checkpoint::{
    AnimationAssetCheckpoint, AnimationAssetError, AnimationAssetLimits, PreparedAnimationAssets,
};
pub use blend_phase::{
    ClipPhase, FrozenSourceTick, PoseBlendPhases, PoseBlendSource, SourcePhaseInterval,
};
pub use blend_space2d::{
    BlendSample2D, BlendSpace2D, BlendSpace2DPlayer, BlendWeights2D,
    blend_space_delaunay_barycentrics_2d,
};
pub use checkpoint::{
    AnimatorCheckpoint, AnimatorCheckpointError, AnimatorFrameCheckpoint, PoseCheckpoint,
};
pub use clip::{
    AnimationClip, Interpolation, JointTangents, JointTrack, KeyframeCursor, Playback, QuatKey,
    TrackInterpolation, Vec3Key,
};
pub(crate) use clip::{Timed, lerp_vec3};
#[cfg(test)]
pub(crate) use clip::{sample_quat, sample_vec3};
pub use compressed::{CompressedClip, CompressionReport, CompressionSettings};
pub use contact_bake::{
    BakedFootContacts, ContactBakeSettings, MAX_CONTACT_BAKE_SAMPLES, bake_periodic_foot_contacts,
};
pub use crowd::{CrowdEvaluator, CrowdLod, CrowdOutcome, CrowdPalettes};
pub use events::{ClipEvent, ClipEventOccurrence, ClipEvents, EventTickError, EventWindow};
pub use foot_plant::FootPlant;
pub use foot_support::{FootSupportPoint, FootSupportPoints, MAX_FOOT_SUPPORT_POINTS};
pub use gait::{
    CoupledPelvisThighFrame, FlatGround, GaitController, GaitFeet, GaitFrame, GaitRegime,
    GaitSettings, GaitSpace, GroundProbe, StrideScaleReport, ThighCapsuleState, ankle_positions,
    extract_pelvis_thigh_frame, measure_authored_stride_speed, scale_clip_stride_to_speed,
};
pub use gpu_layout::{
    DualQuat, GPU_NO_PARENT, GpuBlendJob, GpuClipTable, GpuJointKey, GpuSkeletonLayout,
    cpu_reference_palette, nlerp, pack_affine_3x4, unpack_affine_3x4,
};
pub use graph::{BlendSample, BlendSegment, BlendSpace1D, BlendSpace1DPlayer, BoneMask};
pub use ik::fabrik::{FabrikResult, FabrikSettings, IkChain, fabrik_two_bone_chain_reach};
pub use ik::full_body::{Effector, FullBodyResult, FullBodyRig, FullBodySettings};
pub use ik::limits::{JointLimit, swing_twist_angles, swing_twist_cone_clamp_quat};
pub use ik::look_at::{LookAtChain, look_at_error, micro_saccade_gaze_jitter_rad};
pub use ik::{PlantedLegsResult, TwoBoneChain, TwoBoneResult, TwoBoneTarget};
pub use pose::Pose;
pub use pose_archive::{PoseSnapshotArchive, PoseSnapshotBuilder, PreparedPoseSnapshots};
pub use precision::{Pose64, Transform64, markley_hemisphere_average, so3_exp, so3_log};
pub use property_track::{
    MAX_PROPERTY_KEYS, MAX_PROPERTY_TRACKS, PreparedPropertyAssets, PropertyAssetCheckpoint,
    PropertyAssetError, PropertyAssetLimits, PropertyClip, PropertyKey, PropertySet, PropertyTrack,
    PropertyValue,
};
pub use ragdoll::{
    ContactManifold, MotorSettings, PlaneEnvironment, Ragdoll, RagdollBody, RagdollDesc,
    RagdollEnvironment, RagdollJoint, RagdollState, powered_blend, spring_bone_verlet_step,
};
pub use retarget::{
    BoneRole, HumanoidProfile, ModelRotationRetarget, RetargetBinding, RetargetBindingCheckpoint,
    RetargetJoint, RetargetRootSimilarity, Side,
};
pub use rigid_fade::{
    PreparedRootRigidFadeCheckpoint, RootRigidFadeCheckpoint, RootRigidFadeCheckpointLimits,
    RootRigidFadeCompileContext, RootRigidFadePlan,
};
pub use root_clock::{RootCyclePhase, enclose_root_cycle_phase};
pub use root_motion_receipt::{RootRigidMotionCheckpoint, RootRigidMotionReceipt};
pub use root_rigid::{
    RootAngularDerivativeBounds, RootMotionInterval, RootMotionPartition, RootMotionStep,
    RootRigidApproximation, RootRigidCertifiedFadeInterval, RootRigidCoordinateCertificate,
    RootRigidCurve, RootRigidEnclosure, RootRigidEnclosureCheckpoint, RootRigidErrorAccumulator,
    RootRigidEvaluatedPose, RootRigidFadeDomain, RootRigidFadeFieldInterval,
    RootRigidFadePointCertificate, RootRigidFieldInterval, RootRigidIntegrationDomain,
    RootRigidMappedField, RootRigidMappedPath, RootRigidPath, RootRigidPathCheckpoint,
    RootRigidPathCheckpointLimits, RootRigidPathProgram, RootRigidPathStateRecord,
    RootRigidPathStep, RootRigidSourceField, RootRigidSpan, RootRigidTimedTwistStep,
    RootRigidTransform, RootRigidTwist, RootRigidTwistEnclosure, RootRigidTwistStep,
    RootRigidVelocity, RootRigidWallInterval, RootRigidWallPartition, RootScrewEnclosurePath,
    RootSourceScrewPointCertificate, RootSpatialTwistBounds, RootTimeCutEnclosure,
    RootTwistErrorBounds, RootTwistRateBounds, RootUniformScaleCheckpoint,
    RootUniformScaleEnclosure,
};
pub use root_rotation::{
    MAX_ROOT_ROTATION_CACHE_KEYS, MAX_ROOT_ROTATION_KEYS, MAX_ROOT_ROTATION_SPANS,
    RootRotationCurve, RootRotationPath, RootRotationSpan,
};
pub use skeleton::{Joint, Skeleton, Transform};
pub(crate) use skeleton::{rigs_match, usable_linear};
pub use soa::{
    AnimationUpdateJob, AnimationUpdateQueue, CompletedAnimationUpdate, SOA_NO_PARENT, SoaPose,
    SoaSkeleton,
};
pub use state_machine::{
    Condition, FadeCurve, MachineStatus, PhaseSync, StateId, StateMachine, StateMachineDesc,
    StateTransition,
};

use std::fmt;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::atomic::Ordering;

pub(crate) use glam::{Mat4, Quat, Vec3, Vec4};

pub const MAX_JOINTS: usize = 256;
/// Aggregate compiled translation keys per immutable clip, including its primary root.
pub const MAX_ROOT_TRANSLATION_CACHE_KEYS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TrackError {
    InvalidTangents,
    InvalidTime,
    InvalidValue,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AnimationError {
    InvalidRetargetBinding,
    InvalidIkChain,
    InvalidIkTarget,
    UnsupportedIkScale,
    DegenerateIkChain,
    UnreachableFootPlant,
    RootRigidBudget,
    RootMotionBudget,
    RootRotationBudget,
    RootRotationTransitionUnsupported,
    InvalidRootRotationCurve,
    InvalidRootMotionJoint(u16),
    SkeletonMismatch,
    InvalidSampleTime,
    InvalidJointCount(usize),
    InvalidJointName(usize),
    InvalidJointTransform(usize),
    InvalidParent { joint: usize, parent: u16 },
    InvalidClipHeader,
    UnsupportedLoopClosure,
    InvalidContactBake,
    TrackCountMismatch { expected: usize, actual: usize },
    InterpolationCountMismatch { expected: usize, actual: usize },
    TangentCountMismatch { expected: usize, actual: usize },
    InvalidTrack { joint: usize, reason: TrackError },
    PoseCountMismatch,
    InvalidPose(usize),
    InvalidBlendWeight,
    InvalidPlaybackSpeed,
    InvalidTransitionDuration,
    InvalidAnimationTimeStep,
    InvalidAnimationPhase,
    NumericalOverflow,
    GraphCycle,
    GraphNodeBudget,
    InvalidGraphNode(usize),
    InvalidGraphParam,
    InvalidStateMachine,
    EventBudget,
    InvalidJointLimit,
    InvalidRagdoll,
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "animation error: {self:?}")
    }
}

impl std::error::Error for AnimationError {}

/// Evaluates a quintic $C^2$ inertialization transition offset from initial error `x0`
/// and velocity `v0` over `duration` at elapsed time `t`.
#[must_use]
pub fn inertialize_scalar_transition(x0: f32, v0: f32, duration: f32, t: f32) -> f32 {
    if !x0.is_finite() || !v0.is_finite() || !duration.is_finite() || !t.is_finite() {
        return 0.0;
    }
    if duration <= 0.0 || t >= duration {
        return 0.0;
    }
    if t <= 0.0 {
        return x0;
    }
    let s = (t / duration).clamp(0.0, 1.0);
    let one_minus_s = 1.0 - s;
    let h0 = one_minus_s * one_minus_s * one_minus_s * (1.0 + 3.0 * s + 6.0 * s * s);
    let h1 = t * one_minus_s * one_minus_s * one_minus_s * (1.0 + 3.0 * s);
    x0 * h0 + v0 * h1
}

/// Computes normalized barycentric coordinates `[w_a, w_b, w_c]` for point `p` inside
/// 2D blend-space triangle `(a, b, c)`, clamping outside points to the triangle and returning
/// `None` if the triangle is degenerate.
#[must_use]
pub fn barycentric_blend_space_2d(
    p: [f32; 2],
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
) -> Option<[f32; 3]> {
    let v0 = [b[0] - a[0], b[1] - a[1]];
    let v1 = [c[0] - a[0], c[1] - a[1]];
    let v2 = [p[0] - a[0], p[1] - a[1]];
    let den = v0[0] * v1[1] - v1[0] * v0[1];
    if !den.is_finite() || den.abs() <= 1e-7 {
        return None;
    }
    let inv_den = 1.0 / den;
    let v = ((v2[0] * v1[1] - v1[0] * v2[1]) * inv_den).max(0.0);
    let w = ((v0[0] * v2[1] - v2[0] * v0[1]) * inv_den).max(0.0);
    let u = (1.0 - v - w).max(0.0);
    let sum = u + v + w;
    if !sum.is_finite() || sum <= 0.0 {
        return None;
    }
    Some([u / sum, v / sum, w / sum])
}

/// Clamps a signed swing/twist joint deflection angle `angle_rad` symmetrically to `[-max_cone_rad, max_cone_rad]`.
#[must_use]
pub fn clamp_swing_twist_angle(angle_rad: f32, max_cone_rad: f32) -> f32 {
    if !angle_rad.is_finite() || !max_cone_rad.is_finite() || max_cone_rad <= 0.0 {
        return 0.0;
    }
    let limit = max_cone_rad.min(std::f32::consts::PI);
    angle_rad.clamp(-limit, limit)
}

/// Evaluates one step of Géry Casiez's adaptive $1\text{€}$ (One-Euro) low-pass filter
/// for jitter-free motion-capture / procedural target tracking at low speeds and low latency at high speeds.
#[must_use]
pub fn one_euro_filter_step(
    prev_filtered: f32,
    prev_raw: f32,
    current_raw: f32,
    dt_s: f32,
    min_cutoff_hz: f32,
    beta: f32,
    d_cutoff_hz: f32,
) -> f32 {
    if !prev_filtered.is_finite()
        || !prev_raw.is_finite()
        || !current_raw.is_finite()
        || !dt_s.is_finite()
        || dt_s <= 0.0
    {
        return current_raw;
    }
    let alpha = |cutoff_hz: f32| -> f32 {
        let tau = 1.0 / (std::f32::consts::TAU * cutoff_hz.max(1e-4));
        1.0 / (1.0 + tau / dt_s)
    };
    let dx = (current_raw - prev_raw) / dt_s;
    let a_d = alpha(d_cutoff_hz);
    let edx = a_d * dx;
    let cutoff = min_cutoff_hz.max(1e-4) + beta.max(0.0) * edx.abs();
    let a = alpha(cutoff);
    prev_filtered + a * (current_raw - prev_filtered)
}

/// Evaluates the $C^2$ quintic inertialization decay offset polynomial
/// $x(t) = (x_0 + v_0 t)(1 - s)^3 (1 + 3s + 6s^2) - v_0 t (1 - s)^3 \cdot 6s^2$ where $s = \text{clamp}(t / T, 0, 1)$.
#[must_use]
pub fn inertialization_quintic_decay_offset(
    initial_offset: f32,
    initial_vel: f32,
    elapsed_s: f32,
    blend_duration_s: f32,
) -> f32 {
    inertialize_scalar_transition(initial_offset, initial_vel, blend_duration_s, elapsed_s)
}

/// Evaluates the asymmetric human bilateral eyelid blink curve in `[0, 1]` over normalized phase
/// `phase_01` $\in [0, 1]$, where fast downward lid closure occupies `close_speed_ratio` (typically $\approx 0.30$)
/// of the blink cycle and slower exponential-like opening occupies the remainder.
#[must_use]
pub fn bilateral_eyelid_blink_curve(phase_01: f32, close_speed_ratio: f32) -> f32 {
    if !phase_01.is_finite() || !close_speed_ratio.is_finite() || !(0.0..=1.0).contains(&phase_01) {
        return 0.0;
    }
    let split = close_speed_ratio.clamp(0.05, 0.95);
    if phase_01 <= split {
        let u = phase_01 / split;
        u * u * (3.0 - 2.0 * u)
    } else {
        let u = (phase_01 - split) / (1.0 - split);
        let one_minus_u = (1.0 - u).clamp(0.0, 1.0);
        one_minus_u * one_minus_u
    }
}

/// Evaluates asymmetric exponential moving average (EMA) lip-sync viseme coarticulation weights
/// across 8 facial viseme channels, applying `attack_rate` when a channel opens toward `target_weights[i]`
/// and `release_rate` when relaxing.
#[must_use]
pub fn coarticulation_ema_viseme_weights(
    current_weights: &[f32; 8],
    target_weights: &[f32; 8],
    attack_rate: f32,
    release_rate: f32,
    dt_s: f32,
) -> [f32; 8] {
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return *current_weights;
    }
    let a_atk = 1.0 - (-attack_rate.max(0.0) * dt_s).exp();
    let a_rel = 1.0 - (-release_rate.max(0.0) * dt_s).exp();
    std::array::from_fn(|i| {
        let cur = current_weights[i].clamp(0.0, 1.0);
        let tgt = target_weights[i].clamp(0.0, 1.0);
        let alpha = if tgt >= cur { a_atk } else { a_rel };
        (cur + alpha * (tgt - cur)).clamp(0.0, 1.0)
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rotation_runtime_tests;

#[cfg(test)]
mod authored_phase_preview_tests;

#[cfg(test)]
mod inverse_bind_validation_tests;

#[cfg(test)]
mod n8_animation_math_tests {
    use super::*;

    #[test]
    fn inertialize_barycentric_and_swing_twist_clamp_behave_smoothly() {
        assert!((inertialize_scalar_transition(2.0, -1.0, 0.5, 0.0) - 2.0).abs() < 1e-6);
        assert_eq!(
            inertialization_quintic_decay_offset(2.0, -1.0, 0.5, 0.5),
            0.0
        );
        let mid = inertialize_scalar_transition(2.0, 0.0, 1.0, 0.5);
        assert!((mid - 1.0).abs() < 1e-5);

        let filtered = one_euro_filter_step(0.0, 0.0, 1.0, 1.0 / 60.0, 1.0, 0.05, 1.0);
        assert!(filtered > 0.0 && filtered < 1.0);

        let blink_peak = bilateral_eyelid_blink_curve(0.3, 0.3);
        assert!((blink_peak - 1.0).abs() < 1e-6);
        assert!(bilateral_eyelid_blink_curve(0.15, 0.3) > 0.0);
        assert!(bilateral_eyelid_blink_curve(0.7, 0.3) < 1.0);

        let vis = coarticulation_ema_viseme_weights(&[0.0; 8], &[1.0; 8], 20.0, 8.0, 0.05);
        assert!(vis[0] > 0.5 && vis[0] < 1.0);

        let weights =
            barycentric_blend_space_2d([0.25, 0.25], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]).unwrap();
        assert!((weights[0] - 0.5).abs() < 1e-6);
        assert!((weights[1] - 0.25).abs() < 1e-6);
        assert!((weights[2] - 0.25).abs() < 1e-6);
        assert!(
            barycentric_blend_space_2d([0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [2.0, 0.0]).is_none()
        );

        assert!((clamp_swing_twist_angle(1.2, 0.5) - 0.5).abs() < 1e-6);
        assert!((clamp_swing_twist_angle(-1.2, 0.5) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn zero_alloc_sample_into_blend_into_and_advance_into_match_owned_evaluators() {
        let rig = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: glam::Mat4::IDENTITY,
        }])
        .unwrap();
        let clip = std::sync::Arc::new(
            AnimationClip::new(
                "walk",
                1.0,
                Playback::Loop,
                vec![JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.0,
                            value: glam::Vec3::ZERO,
                        },
                        Vec3Key {
                            time: 0.5,
                            value: glam::Vec3::X,
                        },
                        Vec3Key {
                            time: 1.0,
                            value: glam::Vec3::new(2.0, 0.0, 0.0),
                        },
                    ],
                    rotations: vec![],
                    scales: vec![],
                }],
                &rig,
            )
            .unwrap(),
        );

        let mut cursor = KeyframeCursor::with_joint_count(1);
        let mut pose = rig.bind_pose();
        let ptr_before = pose.local().as_ptr();
        for &t in &[0.1, 0.25, 0.6, 0.85, 0.15] {
            clip.sample_with_cursor_into(&rig, t, &mut cursor, &mut pose);
            assert_eq!(pose.local(), clip.sample(&rig, t).local());
            assert_eq!(pose.local().as_ptr(), ptr_before);
        }

        let a = clip.sample(&rig, 0.2);
        let b = clip.sample(&rig, 0.8);
        Pose::blend_into(&a, &b, 0.35, &mut pose).unwrap();
        assert_eq!(pose, Pose::blend(&a, &b, 0.35).unwrap());
        assert_eq!(pose.local().as_ptr(), ptr_before);

        let mut anim_a = Animator::new(clip.clone());
        let mut anim_b = Animator::new(clip);
        let mut frame = AnimatorFrame::reusable(&rig);
        let skin_ptr = frame.skin_matrices.as_ptr();
        for _ in 0..8 {
            let expected = anim_a.advance(&rig, 0.125).unwrap();
            anim_b.advance_into(&rig, 0.125, &mut frame).unwrap();
            assert_eq!(frame.pose, expected.pose);
            assert_eq!(frame.skin_matrices, expected.skin_matrices);
            assert_eq!(frame.root_motion, expected.root_motion);
            assert_eq!(frame.skin_matrices.as_ptr(), skin_ptr);
        }
    }
}
