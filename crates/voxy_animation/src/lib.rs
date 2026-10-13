//! Validated skeletal animation sampling and skin-matrix generation.

mod contact_bake;
mod loop_closure;
mod precision;
pub use contact_bake::{
    BakedFootContacts, ContactBakeSettings, MAX_CONTACT_BAKE_SAMPLES, bake_periodic_foot_contacts,
};
pub use precision::{Pose64, Transform64};
mod events;
pub use events::{ClipEvent, ClipEventOccurrence, ClipEvents, EventTickError, EventWindow};
mod exact_quaternion;
mod root_clock;
mod root_curve;
pub use root_clock::{RootCyclePhase, enclose_root_cycle_phase};
mod foot_support;
mod ik;
pub use foot_support::{FootSupportPoint, FootSupportPoints, MAX_FOOT_SUPPORT_POINTS};
mod foot_plant;
pub use foot_plant::FootPlant;
mod retarget;
pub use retarget::{ModelRotationRetarget, RetargetBinding, RetargetJoint, RetargetRootSimilarity};
mod blend_phase;
pub use blend_phase::{
    ClipPhase, FrozenSourceTick, PoseBlendPhases, PoseBlendSource, SourcePhaseInterval,
};

pub use ik::{PlantedLegsResult, TwoBoneChain, TwoBoneResult, TwoBoneTarget};
mod rigid_fade;
pub use rigid_fade::RootRigidFadePlan;
mod root_rigid;
pub use root_rigid::{
    RootAngularDerivativeBounds, RootMotionInterval, RootMotionPartition, RootMotionStep,
    RootRigidApproximation, RootRigidCertifiedFadeInterval, RootRigidCoordinateCertificate,
    RootRigidCurve, RootRigidEnclosure, RootRigidErrorAccumulator, RootRigidEvaluatedPose,
    RootRigidFadeDomain, RootRigidFadeFieldInterval, RootRigidFadePointCertificate,
    RootRigidFieldInterval, RootRigidIntegrationDomain, RootRigidMappedField, RootRigidMappedPath,
    RootRigidPath, RootRigidSourceField, RootRigidSpan, RootRigidTransform, RootRigidTwist,
    RootRigidTwistEnclosure, RootRigidVelocity, RootRigidWallInterval, RootRigidWallPartition,
    RootScrewEnclosurePath, RootSourceScrewPointCertificate, RootSpatialTwistBounds,
    RootTimeCutEnclosure, RootTwistErrorBounds, RootTwistRateBounds, RootUniformScaleEnclosure,
};
mod root_rotation;
pub use root_rotation::{
    MAX_ROOT_ROTATION_CACHE_KEYS, MAX_ROOT_ROTATION_KEYS, MAX_ROOT_ROTATION_SPANS,
    RootRotationCurve, RootRotationPath, RootRotationSpan,
};
mod anim_graph;
mod blend_space2d;
mod state_machine;
pub use blend_space2d::{BlendSample2D, BlendSpace2D, BlendSpace2DPlayer, BlendWeights2D};
pub use state_machine::{
    Condition, FadeCurve, MachineStatus, PhaseSync, StateId, StateMachine, StateMachineDesc,
    StateTransition,
};
mod graph;
pub use anim_graph::{
    AnimGraph, FloatInput, GraphDesc, GraphFrame, GraphNode, GraphPlayer, MAX_GRAPH_NODES, NodeId,
    ParamId, ParamValue,
};
mod soa;
pub use graph::{BlendSample, BlendSegment, BlendSpace1D, BlendSpace1DPlayer, BoneMask};
pub use soa::{
    AnimationUpdateJob, AnimationUpdateQueue, CompletedAnimationUpdate, SOA_NO_PARENT, SoaPose,
    SoaSkeleton,
};

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use glam::{Mat4, Quat, Vec3, Vec4};

pub const MAX_JOINTS: usize = 256;
/// Aggregate compiled translation keys per immutable clip, including its primary root.
pub const MAX_ROOT_TRANSLATION_CACHE_KEYS: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    #[must_use]
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    fn is_valid(self) -> bool {
        self.translation.is_finite()
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self.scale.is_finite()
            && self.scale.abs().min_element() > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub name: Arc<str>,
    pub parent: Option<u16>,
    pub bind_local: Transform,
    pub inverse_bind: Mat4,
}

mod linear_rank;
fn usable_linear(matrix: glam::DMat4) -> bool {
    linear_rank::usable(matrix)
}

// Shared immutable layout is the fast path; exact structural equality also
// permits independently reconstructed copies without hashes or process IDs.
fn rigs_match(a: &Arc<[Joint]>, b: &Arc<[Joint]>) -> bool {
    Arc::ptr_eq(a, b) || a.as_ref() == b.as_ref()
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    joints: Arc<[Joint]>,
}

impl Skeleton {
    /// Creates a parent-before-child skeleton suitable for linear pose evaluation.
    /// Affine inverse binds are stored in canonical homogeneous form (w=1).
    ///
    /// # Errors
    ///
    /// Rejects empty/oversized skeletons, duplicate/empty names, invalid transforms, non-finite
    /// inverse bind matrices, non-affine/singular inverse binds, and parents that
    /// do not precede their child.
    pub fn new(mut joints: Vec<Joint>) -> Result<Self, AnimationError> {
        if joints.is_empty() || joints.len() > MAX_JOINTS {
            return Err(AnimationError::InvalidJointCount(joints.len()));
        }
        for index in 0..joints.len() {
            let joint = &joints[index];
            if joint.name.is_empty()
                || joints[..index]
                    .iter()
                    .any(|candidate| candidate.name == joint.name)
            {
                return Err(AnimationError::InvalidJointName(index));
            }
            let inverse = joint.inverse_bind;
            if !joint.bind_local.is_valid()
                || !inverse.is_finite()
                || inverse.x_axis.w != 0.
                || inverse.y_axis.w != 0.
                || inverse.z_axis.w != 0.
                || inverse.w_axis.w == 0.
            {
                return Err(AnimationError::InvalidJointTransform(index));
            }
            // An affine homogeneous map may have row (0,0,0,h). Canonicalize
            // by dividing every column by h, preserving the represented map.
            // Check the source row first so division cannot erase perspective.
            let h = inverse.w_axis.w;
            let inverse = Mat4::from_cols(
                inverse.x_axis / h,
                inverse.y_axis / h,
                inverse.z_axis / h,
                inverse.w_axis / h,
            );
            if !usable_linear(inverse.as_dmat4()) {
                return Err(AnimationError::InvalidJointTransform(index));
            }
            if let Some(parent) = joint.parent
                && usize::from(parent) >= index
            {
                return Err(AnimationError::InvalidParent {
                    joint: index,
                    parent,
                });
            }
            joints[index].inverse_bind = inverse;
        }
        let skeleton = Self {
            joints: joints.into(),
        };
        // Use the same evaluator as runtime: finite local transforms can still
        // overflow along a hierarchy or when multiplied by inverse binds.
        skeleton
            .bind_pose64()
            .skin_matrices(&skeleton)
            .map_err(|error| match error {
                AnimationError::InvalidPose(index) => AnimationError::InvalidJointTransform(index),
                other => other,
            })?;
        Ok(skeleton)
    }

    #[must_use]
    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    #[must_use]
    pub fn bind_pose(&self) -> Pose {
        Pose {
            rig: self.joints.clone(),
            local: self.joints.iter().map(|joint| joint.bind_local).collect(),
        }
    }

    /// Creates a Structure-of-Arrays (SoA) skeleton layout for cache-friendly pose evaluation.
    #[must_use]
    pub fn to_soa(&self) -> SoaSkeleton {
        SoaSkeleton::from_skeleton(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3Key {
    pub time: f32,
    pub value: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuatKey {
    pub time: f32,
    pub value: Quat,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JointTrack {
    pub translations: Vec<Vec3Key>,
    pub rotations: Vec<QuatKey>,
    pub scales: Vec<Vec3Key>,
}

/// Sampling mode belongs to each property channel, not to the whole clip.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
    CubicSpline,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrackInterpolation {
    pub translation: Interpolation,
    pub rotation: Interpolation,
    pub scale: Interpolation,
}

/// Incoming and outgoing derivatives per key, in value units per second.
/// Quaternion derivatives are four-component vectors, not normalized rotations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JointTangents {
    pub translation: Vec<[Vec3; 2]>,
    pub rotation: Vec<[Vec4; 2]>,
    pub scale: Vec<[Vec3; 2]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Playback {
    Loop,
    Clamp,
}

#[derive(Clone, Debug)]
pub struct AnimationClip {
    rig: Arc<[Joint]>,
    root_curve: Arc<root_curve::RootCurve>,
    motion_curves: Arc<[OnceLock<Result<Arc<root_curve::RootCurve>, AnimationError>>]>,
    motion_cache_keys: Arc<AtomicUsize>,
    rigid_curves: Arc<[OnceLock<Result<RootRigidCurve, AnimationError>>]>,
    rotation_curves: Arc<[OnceLock<Result<RootRotationCurve, AnimationError>>]>,
    rotation_cache_keys: Arc<AtomicUsize>,
    name: Arc<str>,
    duration: f32,
    playback: Playback,
    tracks: Arc<[JointTrack]>,
    interpolation: Arc<[TrackInterpolation]>,
    tangents: Arc<[JointTangents]>,
    constant_transforms: Arc<[Option<Transform>]>,
    constant_translations: Arc<[Option<Vec3>]>,
}

impl AnimationClip {
    /// Validates and creates a clip whose track count matches the skeleton.
    ///
    /// # Errors
    ///
    /// Rejects invalid name/duration/count or keys that are non-finite, unordered, outside the
    /// clip, non-normalized, or contain invalid scale values.
    pub fn new(
        name: impl Into<Arc<str>>,
        duration: f32,
        playback: Playback,
        tracks: Vec<JointTrack>,
        skeleton: &Skeleton,
    ) -> Result<Self, AnimationError> {
        let interpolation = vec![TrackInterpolation::default(); tracks.len()];
        Self::new_with_interpolation(name, duration, playback, tracks, interpolation, skeleton)
    }

    /// Creates a clip with independently specified translation, rotation and scale modes.
    /// # Errors
    /// Applies the same key validation as `new`, and rejects mismatched mode counts.
    pub fn new_with_interpolation(
        name: impl Into<Arc<str>>,
        duration: f32,
        playback: Playback,
        tracks: Vec<JointTrack>,
        interpolation: Vec<TrackInterpolation>,
        skeleton: &Skeleton,
    ) -> Result<Self, AnimationError> {
        let tangents = vec![JointTangents::default(); tracks.len()];
        Self::new_with_tangents(
            name,
            duration,
            playback,
            tracks,
            interpolation,
            tangents,
            skeleton,
        )
    }

    /// Creates a clip with validated cubic derivative streams.
    /// # Errors
    /// Rejects missing, non-finite, unused or incorrectly sized derivative streams,
    /// or cubic channels with fewer than two keys, in addition to `new` validation.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_tangents(
        name: impl Into<Arc<str>>,
        duration: f32,
        playback: Playback,
        tracks: Vec<JointTrack>,
        interpolation: Vec<TrackInterpolation>,
        tangents: Vec<JointTangents>,
        skeleton: &Skeleton,
    ) -> Result<Self, AnimationError> {
        if tangents.len() != tracks.len() {
            return Err(AnimationError::TangentCountMismatch {
                expected: tracks.len(),
                actual: tangents.len(),
            });
        }
        if interpolation.len() != tracks.len() {
            return Err(AnimationError::InterpolationCountMismatch {
                expected: tracks.len(),
                actual: interpolation.len(),
            });
        }
        let name = name.into();
        if name.is_empty() || !duration.is_finite() || duration <= 0.0 {
            return Err(AnimationError::InvalidClipHeader);
        }
        if tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: tracks.len(),
            });
        }
        for (joint, track) in tracks.iter().enumerate() {
            let mode = interpolation[joint];
            let derivatives = &tangents[joint];
            for (mode, count, tangent_count, finite) in [
                (
                    mode.translation,
                    track.translations.len(),
                    derivatives.translation.len(),
                    derivatives
                        .translation
                        .iter()
                        .flatten()
                        .all(|v| v.is_finite()),
                ),
                (
                    mode.rotation,
                    track.rotations.len(),
                    derivatives.rotation.len(),
                    derivatives.rotation.iter().flatten().all(|v| v.is_finite()),
                ),
                (
                    mode.scale,
                    track.scales.len(),
                    derivatives.scale.len(),
                    derivatives.scale.iter().flatten().all(|v| v.is_finite()),
                ),
            ] {
                if !finite
                    || if mode == Interpolation::CubicSpline {
                        count < 2 || tangent_count != count
                    } else {
                        tangent_count != 0
                    }
                {
                    return Err(AnimationError::InvalidTrack {
                        joint,
                        reason: TrackError::InvalidTangents,
                    });
                }
            }
            validate_vec_keys(&track.translations, duration, false)
                .map_err(|reason| AnimationError::InvalidTrack { joint, reason })?;
            validate_quat_keys(&track.rotations, duration)
                .map_err(|reason| AnimationError::InvalidTrack { joint, reason })?;
            validate_vec_keys(&track.scales, duration, true)
                .map_err(|reason| AnimationError::InvalidTrack { joint, reason })?;
        }
        if tracks[0].translations.len() > MAX_ROOT_TRANSLATION_CACHE_KEYS {
            return Err(AnimationError::RootMotionBudget);
        }
        let root_curve = Arc::new(root_curve::RootCurve::new(
            &tracks[0].translations,
            interpolation[0].translation,
            &tangents[0].translation,
            duration,
            playback,
        ));
        let constant_transforms = skeleton
            .joints
            .iter()
            .zip(&tracks)
            .zip(&interpolation)
            .zip(&tangents)
            .map(|(((joint, track), mode), tangents)| {
                constant_transform(joint.bind_local, track, *mode, tangents)
            })
            .collect::<Vec<_>>()
            .into();
        let constant_translations = skeleton
            .joints
            .iter()
            .zip(&tracks)
            .zip(&interpolation)
            .zip(&tangents)
            .map(|(((joint, track), mode), tangents)| {
                constant_vec_channel(
                    &track.translations,
                    joint.bind_local.translation,
                    mode.translation,
                    &tangents.translation,
                )
            })
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            constant_translations,
            constant_transforms,
            rig: skeleton.joints.clone(),
            root_curve,
            motion_cache_keys: Arc::new(AtomicUsize::new(tracks[0].translations.len())),
            motion_curves: (0..tracks.len())
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into(),
            rigid_curves: (0..tracks.len())
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into(),
            rotation_curves: (0..tracks.len())
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into(),
            rotation_cache_keys: Arc::new(AtomicUsize::new(0)),
            name,
            duration,
            playback,
            tracks: tracks.into(),
            interpolation: interpolation.into(),
            tangents: tangents.into(),
        })
    }

    /// Exact authored equivalence for retaining playback across asset reloads.
    /// Derived motion caches and allocation identity do not affect the animation.
    #[must_use]
    pub fn has_same_authored_animation(&self, other: &Self) -> bool {
        rigs_match(&self.rig, &other.rig)
            && self.name == other.name
            && self.duration == other.duration
            && self.playback == other.playback
            && self.tracks == other.tracks
            && self.interpolation == other.interpolation
            && self.tangents == other.tangents
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }

    fn motion_curve(&self, joint: u16) -> Result<Arc<root_curve::RootCurve>, AnimationError> {
        let i = usize::from(joint);
        let slot = self
            .motion_curves
            .get(i)
            .ok_or(AnimationError::InvalidRootMotionJoint(joint))?;
        if joint == 0 {
            return Ok(self.root_curve.clone());
        }
        slot.get_or_init(|| {
            let keys = self.tracks[i].translations.len();
            self.motion_cache_keys
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                    used.checked_add(keys)
                        .filter(|total| *total <= MAX_ROOT_TRANSLATION_CACHE_KEYS)
                })
                .map_err(|_| AnimationError::RootMotionBudget)?;
            Ok(Arc::new(root_curve::RootCurve::new(
                &self.tracks[i].translations,
                self.interpolation[i].translation,
                &self.tangents[i].translation,
                self.duration,
                self.playback,
            )))
        })
        .clone()
    }

    /// Shared composed rotation/translation coefficients for one motion joint.
    /// Translation masks are interval inputs, not separate compiled copies.
    /// # Errors
    /// Invalid joints or exhausted per-clip channel-cache budgets reject selection.
    pub fn root_rigid_curve(&self, joint: u16) -> Result<RootRigidCurve, AnimationError> {
        let i = usize::from(joint);
        let slot = self
            .rigid_curves
            .get(i)
            .ok_or(AnimationError::InvalidRootMotionJoint(joint))?;
        slot.get_or_init(|| {
            RootRigidCurve::new(
                self.motion_curve(joint)?,
                self.root_rotation_curve(joint)?,
                self.tracks[i]
                    .translations
                    .first()
                    .map_or(self.rig[i].bind_local.translation, |key| key.value),
                self.rig[i].bind_local.translation,
                self.duration,
                self.playback,
            )
        })
        .clone()
    }

    /// Original-key absolute local translation/unit rotation over one continuous
    /// local key interval. Bind offsets and initial authored rotation are retained;
    /// extracted root factors and loop powers are not substituted for local poses.
    /// Scale channels are excluded. None denotes an unsupported source channel.
    /// # Errors
    /// Rejects unknown joints, invalid intervals, jumps and enclosure overflow.
    pub fn joint_rigid_pose_interval_enclosure(
        &self,
        joint: u16,
        times: [f64; 2],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        let curve = self.root_rigid_curve(joint)?;
        let index = usize::from(joint);
        let initial = self.tracks[index]
            .rotations
            .first()
            .map_or(self.rig[index].bind_local.rotation, |key| key.value);
        curve.absolute_source_pose_interval_enclosure(times, initial)
    }

    /// Uniform error of the actual f32 local translation sampler against the
    /// real interpolant of original stored keys at the same f32 local time.
    /// Held/default channels are exact. LINEAR covers one key interval; None
    /// requires partitioning at keys or qualification of a different mode.
    /// Clock conversion, loop wrapping and retargeting are separate obligations.
    /// # Errors
    /// Rejects unknown joints and invalid/out-of-clip local time intervals.
    pub fn joint_translation_sample_error_bounds(
        &self,
        joint: u16,
        times: [f32; 2],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let index = usize::from(joint);
        let track = self
            .tracks
            .get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(joint))?;
        if times.iter().any(|v| !v.is_finite())
            || times[0] < 0.
            || times[1] > self.duration
            || times[0] > times[1]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let keys = &track.translations;
        let Some(first) = keys.first() else {
            return Ok(Some([0.; 3]));
        };
        if times[1] <= first.time || times[0] >= keys[keys.len() - 1].time {
            return Ok(Some([0.; 3]));
        }
        let upper = keys.partition_point(|key| key.time <= times[0]);
        if upper == 0 || times[1] > keys[upper].time {
            return Ok(None);
        }
        let from = &keys[upper - 1];
        let to = &keys[upper];
        match self.interpolation[index].translation {
            Interpolation::Linear if from.value != to.value => {
                RootRigidEnclosure::source_linear_translation_sample_error(
                    from.value.to_array(),
                    to.value.to_array(),
                    [from.time, to.time],
                    times,
                )
                .map(Some)
            }
            Interpolation::Linear => Ok(Some([0.; 3])),
            Interpolation::Step if times[1] < to.time || from.value == to.value => {
                Ok(Some([0.; 3]))
            }
            _ => Ok(None),
        }
    }

    /// Local LINEAR translation error including narrowing f64 time to f32.
    /// Compares sampling at narrowed time with the real original-key curve at
    /// the supplied local f64 time. The real and rounded interval must fit one
    /// key segment. Clock accumulation, clamping/wrapping and blending remain
    /// separate; this does not certify try_sample_clock across loop seams.
    /// # Errors
    /// Rejects unknown joints and invalid/out-of-clip intervals.
    pub fn joint_translation_local_time_error_bounds(
        &self,
        joint: u16,
        times: [f64; 2],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let index = usize::from(joint);
        let track = self
            .tracks
            .get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(joint))?;
        if times.iter().any(|v| !v.is_finite())
            || times[0] < 0.
            || times[1] > f64::from(self.duration)
            || times[0] > times[1]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let rounded = times.map(|v| f64::from(v as f32));
        let hull = [times[0].min(rounded[0]), times[1].max(rounded[1])];
        let keys = &track.translations;
        let Some(first) = keys.first() else {
            return Ok(Some([0.; 3]));
        };
        if hull[1] <= f64::from(first.time) || hull[0] >= f64::from(keys[keys.len() - 1].time) {
            return Ok(Some([0.; 3]));
        }
        let upper = keys.partition_point(|key| f64::from(key.time) <= hull[0]);
        if upper == 0 || hull[1] > f64::from(keys[upper].time) {
            return Ok(None);
        }
        let from = &keys[upper - 1];
        let to = &keys[upper];
        match self.interpolation[index].translation {
            Interpolation::Linear if from.value != to.value => {
                RootRigidEnclosure::source_linear_translation_local_time_error(
                    from.value.to_array(),
                    to.value.to_array(),
                    [from.time, to.time],
                    times,
                )
                .map(Some)
            }
            Interpolation::Linear => Ok(Some([0.; 3])),
            Interpolation::Step if hull[1] < f64::from(to.time) || from.value == to.value => {
                Ok(Some([0.; 3]))
            }
            _ => Ok(None),
        }
    }

    /// Same-time error of held/default/STEP and stationary LINEAR quaternion
    /// sampling against the original real unit source orientation. Constant
    /// LINEAR qualifies both runtime normalizations and the raw first boundary.
    /// Moving LINEAR/CUBICSPLINE and key crossings return None; local clocks,
    /// wrapping and downstream retarget/scene arithmetic are separate.
    /// # Errors
    /// Rejects unknown joints, invalid local intervals and normalization overflow.
    pub fn joint_rotation_sample_error_bounds(
        &self,
        joint: u16,
        times: [f32; 2],
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        let index = usize::from(joint);
        let track = self
            .tracks
            .get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(joint))?;
        if times.iter().any(|v| !v.is_finite())
            || times[0] < 0.
            || times[1] > self.duration
            || times[0] > times[1]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let keys = &track.rotations;
        let q = if let Some(first) = keys.first() {
            let last = &keys[keys.len() - 1];
            if times[1] <= first.time {
                first.value
            } else if times[0] >= last.time {
                last.value
            } else {
                let upper = keys.partition_point(|key| key.time <= times[0]);
                if upper == 0 || times[1] >= keys[upper].time {
                    return Ok(None);
                }
                match self.interpolation[index].rotation {
                    Interpolation::Step => keys[upper - 1].value,
                    Interpolation::Linear => {
                        return RootRigidEnclosure::stationary_nlerp_sample_error_bounds([
                            keys[upper - 1].value.to_array(),
                            keys[upper].value.to_array(),
                        ]);
                    }
                    Interpolation::CubicSpline => return Ok(None),
                }
            }
        } else {
            self.rig[index].bind_local.rotation
        };
        RootRigidEnclosure::stored_rotation_source_error(q.to_array()).map(Some)
    }

    /// Compiles one selected quaternion channel for ordered root rotation extraction.
    /// Selected channels compile once per immutable clip, including concurrent
    /// callers. Clones share coefficients. This preserves curves and winding across
    /// keys/loops; it does not collapse the path to an endpoint quaternion.
    /// # Errors
    /// Rejects an unknown joint or more than 65,536 compiled rotation keys in
    /// total across this clip's selected channels. Cached data lasts with the clip.
    pub fn root_rotation_curve(&self, joint: u16) -> Result<RootRotationCurve, AnimationError> {
        let index = usize::from(joint);
        let Some(cache) = self.rotation_curves.get(index) else {
            return Err(AnimationError::InvalidRootMotionJoint(joint));
        };
        cache
            .get_or_init(|| {
                let count = self.tracks[index].rotations.len();
                self.rotation_cache_keys
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |keys| {
                        keys.checked_add(count)
                            .filter(|total| *total <= MAX_ROOT_ROTATION_CACHE_KEYS)
                    })
                    .map_err(|_| AnimationError::RootRotationBudget)?;
                RootRotationCurve::new(
                    &self.tracks[index].rotations,
                    self.interpolation[index].rotation,
                    &self.tangents[index].rotation,
                    self.rig[index].bind_local.rotation,
                    self.duration,
                    self.playback,
                )
            })
            .clone()
    }

    /// Exact rig compatibility, including ordered names, hierarchy and bind data.
    /// Shared immutable layouts use a constant-time pointer fast path.
    #[must_use]
    pub fn is_compatible_with(&self, skeleton: &Skeleton) -> bool {
        rigs_match(&self.rig, &skeleton.joints)
    }

    /// Whether a joint has no authored channels and therefore remains at bind TRS.
    /// Unknown joint indices return false. This is deliberately conservative:
    /// constant authored channels are still channels, not an implicit fixed basis.
    #[must_use]
    pub fn joint_uses_bind_pose(&self, index: usize) -> bool {
        self.tracks.get(index).is_some_and(|track| {
            track.translations.is_empty() && track.rotations.is_empty() && track.scales.is_empty()
        })
    }

    /// A transform proved constant over the entire authored clip, including held
    /// endpoints and loop seams. Missing channels use bind values. Authored values
    /// may differ from bind. None means moving, unproved or an unknown joint.
    /// Proofs are compiled once at clip admission; this lookup is constant-time.
    #[must_use]
    pub fn constant_joint_transform(&self, index: usize) -> Option<Transform> {
        self.constant_transforms.get(index).copied().flatten()
    }

    /// A whole-channel admission proof, independent of animated rotation/scale.
    /// Cubic translation with equal endpoints and nonzero tangents is not constant.
    #[must_use]
    pub fn constant_joint_translation(&self, index: usize) -> Option<Vec3> {
        self.constant_translations.get(index).copied().flatten()
    }

    /// Samples without runtime admission. Cubic curves can produce invalid TRS;
    /// use `try_sample` to reject those poses before publication. Bind defaults
    /// always come from the clip's source rig; the legacy skeleton argument does
    /// not retarget a clip. Returned poses retain the source rig binding.
    #[must_use]
    pub fn sample(&self, _skeleton: &Skeleton, time: f32) -> Pose {
        let time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local(time)
    }

    /// Samples a pose suitable for admission into a runtime or GPU palette.
    /// # Errors
    /// Rejects non-finite time, mismatched rig layouts and invalid interpolated TRS.
    pub fn try_sample(&self, skeleton: &Skeleton, time: f32) -> Result<Pose, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.try_sample_local(skeleton, local_time)
    }

    /// Samples a normalized authored phase without advancing a playback clock.
    /// Phase 1 includes the final authored pose even for a looping clip.
    /// This stateless preview does not emit animation events.
    /// # Errors
    /// Rejects phases outside [0,1], nonfinite phases and invalid poses/rigs.
    pub fn try_sample_phase(
        &self,
        skeleton: &Skeleton,
        phase: f64,
    ) -> Result<Pose, AnimationError> {
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.try_sample_local(skeleton, (phase * f64::from(self.duration)) as f32)
    }

    fn try_sample_local(&self, skeleton: &Skeleton, time: f32) -> Result<Pose, AnimationError> {
        if self.tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: self.tracks.len(),
            });
        }
        if !self.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let pose = self.sample_local(time);
        for (joint, transform) in pose.local.iter().enumerate() {
            if !transform.is_valid() {
                return Err(AnimationError::InvalidPose(joint));
            }
        }
        Ok(pose)
    }

    fn phase(&self, time: f64) -> f64 {
        let duration = f64::from(self.duration);
        match self.playback {
            Playback::Loop => time.rem_euclid(duration),
            Playback::Clamp => time.clamp(0.0, duration),
        }
    }

    fn try_sample_clock(&self, skeleton: &Skeleton, time: f64) -> Result<Pose, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        // Reduce once in the clock's precision. A phase just below the seam can
        // round to duration in f32; wrapping it again would publish the first
        // pose before the clock actually crosses the seam.
        self.try_sample_local(skeleton, self.phase(time) as f32)
    }

    fn sample_local(&self, time: f32) -> Pose {
        let local = self
            .rig
            .iter()
            .zip(self.tracks.iter())
            .zip(self.interpolation.iter())
            .zip(self.tangents.iter())
            .map(|(((joint, track), mode), tangents)| Transform {
                translation: sample_vec3(
                    &track.translations,
                    time,
                    joint.bind_local.translation,
                    mode.translation,
                    &tangents.translation,
                ),
                rotation: sample_quat(
                    &track.rotations,
                    time,
                    joint.bind_local.rotation,
                    mode.rotation,
                    &tangents.rotation,
                ),
                scale: sample_vec3(
                    &track.scales,
                    time,
                    joint.bind_local.scale,
                    mode.scale,
                    &tangents.scale,
                ),
            })
            .collect();
        Pose {
            rig: self.rig.clone(),
            local,
        }
    }

    /// Samples directly into a Structure-of-Arrays ([`SoaPose`]) layout without intermediate AoS allocation.
    #[must_use]
    pub fn sample_soa(&self, _skeleton: &Skeleton, time: f32) -> SoaPose {
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local_soa(local_time)
    }

    /// Samples and validates a [`SoaPose`] suitable for admission into a runtime or GPU palette.
    ///
    /// # Errors
    /// Rejects non-finite time, mismatched rig layouts, and invalid interpolated TRS.
    pub fn try_sample_soa(
        &self,
        skeleton: &Skeleton,
        time: f32,
    ) -> Result<SoaPose, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: self.tracks.len(),
            });
        }
        if !self.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        let mut pose = self.sample_local_soa(local_time);
        for index in 0..pose.len() {
            let transform = Transform {
                translation: pose.translations[index],
                rotation: pose.rotations[index],
                scale: pose.scales[index],
            };
            if !transform.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
        }
        pose.validated = true;
        Ok(pose)
    }

    fn sample_local_soa(&self, time: f32) -> SoaPose {
        let len = self.rig.len();
        let mut translations = Vec::with_capacity(len);
        let mut rotations = Vec::with_capacity(len);
        let mut scales = Vec::with_capacity(len);
        for index in 0..len {
            let joint = &self.rig[index];
            let track = &self.tracks[index];
            let mode = &self.interpolation[index];
            let tangents = &self.tangents[index];
            translations.push(sample_vec3(
                &track.translations,
                time,
                joint.bind_local.translation,
                mode.translation,
                &tangents.translation,
            ));
            rotations.push(sample_quat(
                &track.rotations,
                time,
                joint.bind_local.rotation,
                mode.rotation,
                &tangents.rotation,
            ));
            scales.push(sample_vec3(
                &track.scales,
                time,
                joint.bind_local.scale,
                mode.scale,
                &tangents.scale,
            ));
        }
        SoaPose {
            rig: self.rig.clone(),
            translations,
            rotations,
            scales,
            validated: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    rig: Arc<[Joint]>,
    local: Vec<Transform>,
}

impl Pose {
    #[must_use]
    pub fn local(&self) -> &[Transform] {
        &self.local
    }

    /// Converts this AoS [`Pose`] into a contiguous [`SoaPose`].
    #[must_use]
    pub fn to_soa(&self) -> SoaPose {
        SoaPose::from_pose(self)
    }

    /// Updates one local translation, preserving its rotation and scale.
    /// Invalid joints or nonfinite translations leave the pose unchanged.
    pub fn set_joint_translation(
        &mut self,
        index: usize,
        translation: Vec3,
    ) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform {
            translation,
            ..current
        };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Updates one local scale, preserving translation and rotation.
    /// Invalid joints, nonfinite or singular scales leave the pose unchanged.
    pub fn set_joint_scale(&mut self, index: usize, scale: Vec3) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform { scale, ..current };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Updates one local rotation, preserving its translation and scale.
    /// # Errors
    /// Rejects an unknown joint or a nonfinite/nonunit quaternion without changing the pose.
    pub fn set_joint_rotation(
        &mut self,
        index: usize,
        rotation: Quat,
    ) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform {
            rotation,
            ..current
        };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Produces `global_joint * inverse_bind` matrices into reusable buffers.
    ///
    /// # Errors
    /// Rejects a pose created for a different skeleton, invalid blend data,
    /// or overflowing global/palette matrices.
    pub fn skin_matrices_into(
        &self,
        skeleton: &Skeleton,
        global: &mut Vec<Mat4>,
        skin: &mut Vec<Mat4>,
    ) -> Result<(), AnimationError> {
        if self.local.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        global.clear();
        skin.clear();
        global.reserve(self.local.len());
        skin.reserve(self.local.len());
        for (index, (local, joint)) in self.local.iter().zip(skeleton.joints.iter()).enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let matrix = joint.parent.map_or_else(
                || local.matrix(),
                |parent| global[usize::from(parent)] * local.matrix(),
            );
            let palette = matrix * joint.inverse_bind;
            if !usable_linear(matrix.as_dmat4()) || !usable_linear(palette.as_dmat4()) {
                return Err(AnimationError::InvalidPose(index));
            }
            global.push(matrix);
            skin.push(palette);
        }
        Ok(())
    }

    /// Produces `global_joint * inverse_bind` matrices in skeleton order.
    ///
    /// # Errors
    ///
    /// Rejects a pose created for a different skeleton, invalid blend data,
    /// or overflowing global/palette matrices.
    pub fn skin_matrices(&self, skeleton: &Skeleton) -> Result<Vec<Mat4>, AnimationError> {
        let mut global = Vec::with_capacity(self.local.len());
        let mut skin = Vec::with_capacity(self.local.len());
        self.skin_matrices_into(skeleton, &mut global, &mut skin)?;
        Ok(skin)
    }

    /// Blends two local poses with clamped weight and shortest-path quaternion interpolation.
    ///
    /// # Errors
    ///
    /// Rejects different pose lengths, a non-finite weight, or invalid input/result TRS.
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Result<Self, AnimationError> {
        if a.local.len() != b.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&a.rig, &b.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = weight.clamp(0.0, 1.0);
        let local = a
            .local
            .iter()
            .zip(&b.local)
            .enumerate()
            .map(|(index, (a, b))| {
                if !a.is_valid() || !b.is_valid() {
                    return Err(AnimationError::InvalidPose(index));
                }
                let value = Transform {
                    translation: lerp_vec3(a.translation, b.translation, weight),
                    rotation: a.rotation.slerp(b.rotation, weight).normalize(),
                    scale: lerp_vec3(a.scale, b.scale, weight),
                };
                if value.is_valid() {
                    Ok(value)
                } else {
                    Err(AnimationError::InvalidPose(index))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            rig: a.rig.clone(),
            local,
        })
    }
}

#[derive(Clone, Debug)]
struct Transition {
    source: Arc<AnimationClip>,
    source_curve: Arc<root_curve::RootCurve>,
    source_time: f64,
    // Interruption captures the displayed blend once; staging shares this snapshot.
    source_pose: Option<Arc<Pose>>,
    elapsed: f64,
    duration: f64,
}

/// Normalized target-clip interval. Looping end may exceed one; no cycles are enumerated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationPhaseInterval {
    pub start: f64,
    pub end: f64,
    pub looping: bool,
}

#[derive(Clone, Debug)]
pub struct Animator {
    current: Arc<AnimationClip>,
    motion_joint: u16,
    motion_curve: Arc<root_curve::RootCurve>,
    time: f64,
    speed: f32,
    transition: Option<Transition>,
}

#[derive(Clone, Debug)]
pub struct AnimatorFrame {
    pub pose: Pose,
    pub skin_matrices: Vec<Mat4>,
    /// Translation displacement in the selected joint's parent-local frame.
    /// It excludes animated ancestor motion and has not been applied to the pose.
    pub root_motion: Vec3,
    pub root_motion_joint: u16,
    pub transition_weight: f32,
}

impl AnimatorFrame {
    /// Removes the selected joint's displayed rotation after its ordered motion
    /// has been extracted separately. Bind rotation, translation, signed scale,
    /// other joints and translation motion remain authoritative. Rebuilds the
    /// skin palette before returning; it neither extracts nor applies a path.
    /// # Errors
    /// Rejects a foreign rig, unknown motion joint, invalid pose or palette.
    pub fn without_root_rotation(mut self, skeleton: &Skeleton) -> Result<Self, AnimationError> {
        let index = usize::from(self.root_motion_joint);
        let joint = skeleton
            .joints
            .get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(
                self.root_motion_joint,
            ))?;
        if !rigs_match(&self.pose.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let local = self
            .pose
            .local
            .get_mut(index)
            .ok_or(AnimationError::InvalidPose(index))?;
        if !local.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        local.rotation = joint.bind_local.rotation;
        self.skin_matrices = self.pose.skin_matrices(skeleton)?;
        Ok(self)
    }

    /// Consumes selected translation axes into a separate parent-local motion request.
    /// The selected joint's corresponding pose coordinates return to bind translation;
    /// rotations, scale, other axes and other joints are retained. The palette is
    /// rebuilt before returning, so skinning cannot retain the extracted translation.
    /// Extracted axes are removed from `root_motion` to prevent repeated consumption.
    /// This does not transform the request into world space or apply character physics.
    ///
    /// # Errors
    /// Rejects a foreign rig, unknown motion joint, invalid displacement or palette.
    pub fn into_in_place_translation(
        mut self,
        skeleton: &Skeleton,
        axes: [bool; 3],
    ) -> Result<(Self, Vec3), AnimationError> {
        let index = usize::from(self.root_motion_joint);
        let joint = skeleton
            .joints
            .get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(
                self.root_motion_joint,
            ))?;
        if !self.root_motion.is_finite() {
            return Err(AnimationError::InvalidPose(index));
        }
        if !rigs_match(&self.pose.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let local = self
            .pose
            .local
            .get_mut(index)
            .ok_or(AnimationError::InvalidPose(index))?;
        let mut extracted = Vec3::ZERO;
        for axis in 0..3 {
            if axes[axis] {
                local.translation[axis] = joint.bind_local.translation[axis];
                extracted[axis] = self.root_motion[axis];
                self.root_motion[axis] = 0.0;
            }
        }
        self.skin_matrices = self.pose.skin_matrices(skeleton)?;
        Ok((self, extracted))
    }
}

impl Animator {
    /// Creates playback after admitting its initial pose and skin palette.
    /// This checks the entry frame; subsequent phases remain transactionally
    /// checked by `advance_wall`.
    /// # Errors
    /// Rejects invalid initial interpolated TRS or overflowing hierarchy/palette.
    pub fn try_new(initial: Arc<AnimationClip>) -> Result<Self, AnimationError> {
        let skeleton = Skeleton {
            joints: initial.rig.clone(),
        };
        initial
            .try_sample_clock(&skeleton, 0.)?
            .skin_matrices(&skeleton)?;
        Ok(Self::new(initial))
    }

    #[must_use]
    pub fn new(initial: Arc<AnimationClip>) -> Self {
        Self {
            motion_joint: 0,
            motion_curve: initial.root_curve.clone(),
            current: initial,
            time: 0.0,
            speed: 1.0,
            transition: None,
        }
    }

    /// Current target clip phase in [0,1], using the same bounded f64 clock as pose sampling.
    #[must_use]
    pub fn normalized_phase(&self) -> f64 {
        self.current.phase(self.time) / f64::from(self.current.duration)
    }

    /// Reads the stored root extraction factor at the current bounded clip
    /// phase. An active blend has no unique authored phase anchor.
    pub fn root_rigid_phase_factor(
        &self,
        axes: [bool; 3],
    ) -> Result<RootRigidTransform, AnimationError> {
        if self.transition.is_some() {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        self.current
            .root_rigid_curve(self.motion_joint)?
            .sample(self.current.phase(self.time), axes)
    }

    /// Original-source extraction factor at the stored current clip phase.
    /// Encloses source normalization/evaluation; an active blend has no unique
    /// phase anchor. None identifies an unsupported source rotation channel.
    pub fn root_rigid_source_phase_factor_enclosure(
        &self,
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if self.transition.is_some() {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        self.current
            .root_rigid_curve(self.motion_joint)?
            .source_phase_enclosure(self.current.phase(self.time), axes)
    }

    /// Predicts the interval from the current clock without advancing it.
    /// # Errors
    /// Rejects the same invalid timestep range as `advance`.
    pub fn phase_interval(&self, dt: f32) -> Result<AnimationPhaseInterval, AnimationError> {
        self.phase_interval_wall(f64::from(dt))
    }

    /// Exact stored f64 wall time for physics-accepted contact intervals.
    pub fn phase_interval_wall(&self, dt: f64) -> Result<AnimationPhaseInterval, AnimationError> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let duration = f64::from(self.current.duration);
        let start = self.current.phase(self.time);
        let end = start + dt * f64::from(self.speed);
        let looping = self.current.playback == Playback::Loop;
        Ok(AnimationPhaseInterval {
            start: start / duration,
            end: if looping {
                end / duration
            } else {
                end.min(duration) / duration
            },
            looping,
        })
    }

    /// Select the joint whose local translation drives displacement extraction.
    /// Curves are compiled only on selection/switch, not on every frame.
    /// This does not remove motion from the pose or convert it to model/world space.
    /// # Errors
    /// An unknown joint preserves the selection, clocks and active transition.
    pub fn set_root_motion_joint(&mut self, joint: u16) -> Result<(), AnimationError> {
        if usize::from(joint) >= self.current.rig.len() {
            return Err(AnimationError::InvalidRootMotionJoint(joint));
        }
        if joint == self.motion_joint {
            return Ok(());
        }
        let curve = self.current.motion_curve(joint)?;
        let source = self
            .transition
            .as_ref()
            .map(|t| t.source.motion_curve(joint))
            .transpose()?;
        self.motion_joint = joint;
        self.motion_curve = curve;
        if let (Some(transition), Some(curve)) = (&mut self.transition, source) {
            transition.source_curve = curve;
        }
        Ok(())
    }

    #[must_use]
    pub const fn root_motion_joint(&self) -> u16 {
        self.motion_joint
    }

    /// Changes playback speed. Zero pauses the animator.
    ///
    /// # Errors
    ///
    /// Rejects non-finite, negative, or excessive speed.
    pub fn set_speed(&mut self, speed: f32) -> Result<(), AnimationError> {
        if !speed.is_finite() || !(0.0..=8.0).contains(&speed) {
            return Err(AnimationError::InvalidPlaybackSpeed);
        }
        self.speed = speed;
        Ok(())
    }

    /// Produces the selected joint's complete rotation path and displayed frame
    /// from the same bounded clock interval. Both succeed before the clock publishes.
    /// The caller still owns coordinate conversion, in-place extraction and physics.
    /// # Errors
    /// Rejects moving crossfades until velocity-blended angular trajectories are
    /// available. Zero elapsed time or zero playback speed yields identity motion
    /// while retaining normal pose blending. All failures preserve the clock.
    pub fn advance_with_root_rotation(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
        max_spans: usize,
    ) -> Result<(AnimatorFrame, RootRotationPath), AnimationError> {
        if self.transition.is_some() && dt != 0. && self.speed != 0. {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        let mut candidate = self.clone();
        let start = self.current.phase(self.time);
        let frame = candidate.advance_candidate(skeleton, dt)?;
        let end = start + f64::from(dt) * f64::from(self.speed);
        let path = self
            .current
            .root_rotation_curve(self.motion_joint)?
            .path(start, end, max_spans)?;
        *self = candidate;
        Ok((frame, path))
    }

    /// Stages a displayed frame and complete simultaneous root-motion path from
    /// one phase interval. The legacy translation delta remains a compatibility
    /// view: consumers must not apply it alongside this composed path.
    /// # Errors
    /// Rig, pose, trajectory-budget and active crossfade errors preserve the clock.
    pub fn advance_with_root_rigid_motion(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<(AnimatorFrame, RootRigidPath), AnimationError> {
        self.advance_with_root_rigid_motion_wall(skeleton, f64::from(dt), axes, max_spans)
    }

    /// Stages a frame and rigid path from the same f64 simulation wall interval.
    /// Pose or path failure preserves all playback state.
    /// # Errors
    /// Rejects invalid steps, incompatible rigs, moving fades or path budget failures.
    pub fn advance_with_root_rigid_motion_wall(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<(AnimatorFrame, RootRigidPath), AnimationError> {
        if self.transition.is_some() && dt != 0. && self.speed != 0. {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        let mut candidate = self.clone();
        let start = self.current.phase(self.time);
        let frame = candidate.advance_candidate_wall(skeleton, dt)?;
        let end = start + dt * f64::from(self.speed);
        let path = self
            .current
            .root_rigid_curve(self.motion_joint)?
            .path(start, end, axes, max_spans)?;
        *self = candidate;
        Ok((frame, path))
    }

    /// Starts a crossfade to a new clip. Interrupting an active fade starts from
    /// its current blended pose, keeping pose continuity. The captured source
    /// is held for the new fade; ordinary fade sources continue playing.
    /// A zero duration switches immediately.
    ///
    /// # Errors
    ///
    /// Rejects incompatible rigs or non-finite, negative, or excessively long transitions.
    pub fn transition_to(
        &mut self,
        next: Arc<AnimationClip>,
        duration: f32,
    ) -> Result<(), AnimationError> {
        self.transition_to_at_phase(next, duration, 0.)
    }

    /// Starts a transition at a normalized target phase, for live clip revisions.
    /// Phase transfer changes the sampling origin, not extracted root displacement.
    /// Every admission error preserves the previous playback state.
    pub fn transition_to_at_phase(
        &mut self,
        next: Arc<AnimationClip>,
        duration: f32,
        phase: f64,
    ) -> Result<(), AnimationError> {
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err(AnimationError::InvalidAnimationPhase);
        }
        let next_time = phase * f64::from(next.duration);
        if !duration.is_finite() || !(0.0..=60.0).contains(&duration) {
            return Err(AnimationError::InvalidTransitionDuration);
        }
        if !rigs_match(&self.current.rig, &next.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        // A clip may have valid keys but an invalid interpolated pose at the
        // requested phase. Admit the displayed palette before publishing it.
        let skeleton = Skeleton {
            joints: self.current.rig.clone(),
        };
        next.try_sample_clock(&skeleton, next_time)?
            .skin_matrices(&skeleton)?;
        let next_curve = next.motion_curve(self.motion_joint)?;
        if duration == 0.0 {
            self.motion_curve = next_curve;
            self.current = next;
            self.time = next_time;
            self.transition = None;
            return Ok(());
        }
        let source_pose = if let Some(transition) = &self.transition {
            let target = self.current.try_sample_clock(&skeleton, self.time)?;
            let sampled;
            let source = if let Some(pose) = &transition.source_pose {
                pose.as_ref()
            } else {
                sampled = transition
                    .source
                    .try_sample_clock(&skeleton, transition.source_time)?;
                &sampled
            };
            let pose = Pose::blend(
                source,
                &target,
                (transition.elapsed / transition.duration) as f32,
            )?;
            // A snapshot must pass the same palette admission as a displayed frame.
            pose.skin_matrices(&skeleton)?;
            Some(Arc::new(pose))
        } else {
            None
        };
        self.transition = Some(Transition {
            source_curve: self.motion_curve.clone(),
            source_pose,
            source: Arc::clone(&self.current),
            source_time: self.time,
            elapsed: 0.0,
            duration: f64::from(duration),
        });
        self.current = next;
        self.motion_curve = next_curve;
        self.time = next_time;
        Ok(())
    }

    /// Advances animation time and returns the blended pose, palette and loop-safe root delta.
    ///
    /// # Errors
    ///
    /// Rejects invalid timestep, clip/skeleton mismatch or numerical overflow.
    /// Every error preserves the prior clock and transition state.
    pub fn advance(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
    ) -> Result<AnimatorFrame, AnimationError> {
        self.advance_wall(skeleton, f64::from(dt))
    }

    /// Advances by the fixed simulation's stored f64 wall step without narrowing.
    /// Every error preserves the prior clock and transition state.
    /// # Errors
    /// Rejects invalid wall steps, incompatible rigs or numerical overflow.
    pub fn advance_wall(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
    ) -> Result<AnimatorFrame, AnimationError> {
        // Clip references are shared; only the small clock/transition state is
        // staged. Failed pose or root-motion evaluation never publishes it.
        let mut candidate = self.clone();
        let frame = candidate.advance_candidate_wall(skeleton, dt)?;
        *self = candidate;
        Ok(frame)
    }

    fn advance_candidate(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
    ) -> Result<AnimatorFrame, AnimationError> {
        self.advance_candidate_wall(skeleton, f64::from(dt))
    }

    fn advance_candidate_wall(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
    ) -> Result<AnimatorFrame, AnimationError> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        for clip in std::iter::once(&self.current)
            .chain(self.transition.iter().map(|transition| &transition.source))
        {
            if clip.tracks.len() != skeleton.joints.len() {
                return Err(AnimationError::TrackCountMismatch {
                    expected: skeleton.joints.len(),
                    actual: clip.tracks.len(),
                });
            }
            if !rigs_match(&clip.rig, &skeleton.joints) {
                return Err(AnimationError::SkeletonMismatch);
            }
        }
        let speed = f64::from(self.speed);
        self.time = self.current.phase(self.time);
        if let Some(transition) = &mut self.transition {
            transition.source_time = transition.source.phase(transition.source_time);
        }
        let delta = dt * speed;
        let old_time = self.time;
        self.time += delta;
        let mut root_motion = if self.transition.is_none() {
            self.motion_curve
                .integral(old_time, self.time, 1., 1.)
                .as_vec3()
        } else {
            Vec3::ZERO
        };
        if !self.time.is_finite() || !root_motion.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let target = self.current.try_sample_clock(skeleton, self.time)?;
        let (pose, weight, transition_complete) = if let Some(transition) = &mut self.transition {
            let fade_dt = dt.min((transition.duration - transition.elapsed).max(0.0));
            let fade_delta = fade_dt * speed;
            let start_weight = transition.elapsed / transition.duration;
            let end_weight = (transition.elapsed + fade_dt) / transition.duration;
            let target_end = old_time + fade_delta;
            let target_fade =
                self.motion_curve
                    .integral(old_time, target_end, start_weight, end_weight);
            let source_fade = if transition.source_pose.is_some() {
                glam::DVec3::ZERO
            } else {
                transition.source_curve.integral(
                    transition.source_time,
                    transition.source_time + fade_delta,
                    1. - start_weight,
                    1. - end_weight,
                )
            };
            let tail = self.motion_curve.integral(target_end, self.time, 1., 1.);
            root_motion = (source_fade + target_fade + tail).as_vec3();
            if !root_motion.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            transition.source_time += fade_delta;
            transition.elapsed = (transition.elapsed + dt).min(transition.duration);
            let weight = (transition.elapsed / transition.duration) as f32;
            if transition.elapsed >= transition.duration {
                (target, 1.0, true)
            } else {
                let sampled;
                let source = if let Some(pose) = &transition.source_pose {
                    pose.as_ref()
                } else {
                    sampled = transition
                        .source
                        .try_sample_clock(skeleton, transition.source_time)?;
                    &sampled
                };
                (
                    Pose::blend(source, &target, weight)?,
                    weight,
                    transition.elapsed >= transition.duration,
                )
            }
        } else {
            (target, 1.0, false)
        };
        if transition_complete {
            self.transition = None;
        }
        let skin_matrices = pose.skin_matrices(skeleton)?;
        // Bound storage only after integrating this complete tick, including all loops.
        self.time = self.current.phase(self.time);
        if let Some(transition) = &mut self.transition {
            transition.source_time = transition.source.phase(transition.source_time);
        }
        Ok(AnimatorFrame {
            pose,
            skin_matrices,
            root_motion,
            root_motion_joint: self.motion_joint,
            transition_weight: weight,
        })
    }
}

// Constant Hermite vector channels require zero derivatives on every used
// segment. First incoming and last outgoing tangents are never sampled.
fn constant_vec_channel(
    keys: &[Vec3Key],
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
) -> Option<Vec3> {
    let Some(first) = keys.first() else {
        return Some(fallback);
    };
    if keys.iter().any(|key| key.value != first.value) {
        return None;
    }
    if mode == Interpolation::CubicSpline
        && tangents
            .windows(2)
            .any(|pair| pair[0][1] != Vec3::ZERO || pair[1][0] != Vec3::ZERO)
    {
        return None;
    }
    Some(first.value)
}

fn constant_transform(
    bind: Transform,
    track: &JointTrack,
    mode: TrackInterpolation,
    tangents: &JointTangents,
) -> Option<Transform> {
    let rotation = if let Some(first) = track.rotations.first() {
        let constant = track.rotations.iter().all(|key| {
            key.value == first.value
                || (mode.rotation != Interpolation::CubicSpline && key.value == -first.value)
        });
        if !constant
            || (mode.rotation == Interpolation::CubicSpline
                && tangents
                    .rotation
                    .windows(2)
                    .any(|pair| pair[0][1] != Vec4::ZERO || pair[1][0] != Vec4::ZERO))
        {
            return None;
        }
        first.value
    } else {
        bind.rotation
    };
    Some(Transform {
        translation: constant_vec_channel(
            &track.translations,
            bind.translation,
            mode.translation,
            &tangents.translation,
        )?,
        rotation,
        scale: constant_vec_channel(&track.scales, bind.scale, mode.scale, &tangents.scale)?,
    })
}

fn validate_vec_keys(keys: &[Vec3Key], duration: f32, scale: bool) -> Result<(), TrackError> {
    let mut previous = None;
    for key in keys {
        if !key.time.is_finite()
            || !(0.0..=duration).contains(&key.time)
            || previous.is_some_and(|previous| key.time <= previous)
        {
            return Err(TrackError::InvalidTime);
        }
        if !key.value.is_finite() || (scale && key.value.abs().min_element() <= 0.0) {
            return Err(TrackError::InvalidValue);
        }
        previous = Some(key.time);
    }
    Ok(())
}

fn validate_quat_keys(keys: &[QuatKey], duration: f32) -> Result<(), TrackError> {
    let mut previous = None;
    for key in keys {
        if !key.time.is_finite()
            || !(0.0..=duration).contains(&key.time)
            || previous.is_some_and(|previous| key.time <= previous)
        {
            return Err(TrackError::InvalidTime);
        }
        if !key.value.is_finite() || !key.value.is_normalized() {
            return Err(TrackError::InvalidValue);
        }
        previous = Some(key.time);
    }
    Ok(())
}

// Keep render interpolation compatible; use wide arithmetic only on overflow.
fn lerp_vec3(from: Vec3, to: Vec3, weight: f32) -> Vec3 {
    if weight == 0. || from == to {
        return from;
    }
    if weight == 1. {
        return to;
    }
    let value = from.lerp(to, weight);
    if value.is_finite() {
        value
    } else {
        precision::linear_vector(from, to, f64::from(weight)).as_vec3()
    }
}

fn sample_vec3(
    keys: &[Vec3Key],
    time: f32,
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
) -> Vec3 {
    if mode == Interpolation::Linear
        && keys.len() > 1
        && time > keys[0].time
        && time < keys[keys.len() - 1].time
    {
        let upper = keys.partition_point(|key| key.time <= time);
        let from = &keys[upper - 1];
        let to = &keys[upper];
        let alpha = (time - from.time) / (to.time - from.time);
        return lerp_vec3(from.value, to.value, alpha);
    }
    precision::sample_vector(keys, f64::from(time), fallback, mode, tangents).as_vec3()
}

fn sample_quat(
    keys: &[QuatKey],
    time: f32,
    fallback: Quat,
    mode: Interpolation,
    tangents: &[[Vec4; 2]],
) -> Quat {
    // Preserve exact authored f32 endpoint/step values for the render API.
    let Some(first) = keys.first() else {
        return fallback;
    };
    if time <= first.time {
        return first.value;
    }
    let last = keys.last().unwrap();
    if time >= last.time {
        return last.value;
    }
    if mode == Interpolation::Step {
        let upper = keys.partition_point(|key| key.time <= time);
        return keys[upper - 1].value;
    }
    if mode == Interpolation::Linear {
        let upper = keys.partition_point(|key| key.time <= time);
        let from = &keys[upper - 1];
        let to = &keys[upper];
        let alpha = (time - from.time) / (to.time - from.time);
        return from.value.slerp(to.value, alpha).normalize();
    }
    Quat::from_array(
        precision::sample_rotation(keys, f64::from(time), fallback, mode, tangents)
            .to_array()
            .map(|v| v as f32),
    )
    .normalize()
}

trait Timed {
    fn time(&self) -> f32;
}

impl Timed for Vec3Key {
    fn time(&self) -> f32 {
        self.time
    }
}

impl Timed for QuatKey {
    fn time(&self) -> f32 {
        self.time
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackError {
    InvalidTangents,
    InvalidTime,
    InvalidValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "animation error: {self:?}")
    }
}

impl std::error::Error for AnimationError {}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rotation_runtime_tests;

#[cfg(test)]
mod authored_phase_preview_tests;

#[cfg(test)]
mod inverse_bind_validation_tests;
