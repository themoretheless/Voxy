//! Keyframed animation clips, track validation, cursor-hinted sampling, and error bounds.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use glam::{Quat, Vec3, Vec4};

use crate::{
    AnimationError, Joint, MAX_ROOT_ROTATION_CACHE_KEYS, MAX_ROOT_TRANSLATION_CACHE_KEYS, Pose,
    RootRigidCurve, RootRigidEnclosure, RootRotationCurve, Skeleton, SoaPose, TrackError,
    Transform, precision, rigs_match, root_curve,
};

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

/// Per-joint keyframe interval cursor (`upper` index in `1..keys.len()`) accelerating
/// sequential monotonic forward/loop playback from $O(\log K)$ binary search to $O(1)$
/// while falling back to `partition_point` on seeks or loop wraps.
#[derive(Clone, Debug, Default)]
pub struct KeyframeCursor {
    joints: Vec<[usize; 3]>,
}

impl KeyframeCursor {
    /// Creates a cursor sized for `joint_count` joints.
    #[must_use]
    pub fn with_joint_count(joint_count: usize) -> Self {
        Self {
            joints: vec![[1; 3]; joint_count],
        }
    }

    fn ensure_len(&mut self, joint_count: usize) {
        if self.joints.len() != joint_count {
            self.joints.resize(joint_count, [1; 3]);
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub(crate) rig: Arc<[Joint]>,
    pub(crate) root_curve: Arc<root_curve::RootCurve>,
    pub(crate) motion_curves: Arc<[OnceLock<Result<Arc<root_curve::RootCurve>, AnimationError>>]>,
    pub(crate) motion_cache_keys: Arc<AtomicUsize>,
    pub(crate) rigid_curves: Arc<[OnceLock<Result<RootRigidCurve, AnimationError>>]>,
    pub(crate) rotation_curves: Arc<[OnceLock<Result<RootRotationCurve, AnimationError>>]>,
    pub(crate) rotation_cache_keys: Arc<AtomicUsize>,
    pub(crate) name: Arc<str>,
    pub(crate) duration: f32,
    pub(crate) playback: Playback,
    pub(crate) tracks: Arc<[JointTrack]>,
    pub(crate) interpolation: Arc<[TrackInterpolation]>,
    pub(crate) tangents: Arc<[JointTangents]>,
    pub(crate) constant_transforms: Arc<[Option<Transform>]>,
    pub(crate) constant_translations: Arc<[Option<Vec3>]>,
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
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
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

    pub(crate) fn motion_curve(
        &self,
        joint: u16,
    ) -> Result<Arc<root_curve::RootCurve>, AnimationError> {
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
        let mut out = Pose {
            rig: self.rig.clone(),
            local: Vec::with_capacity(self.rig.len()),
        };
        self.sample_into(_skeleton, time, &mut out);
        out
    }

    /// Samples into an existing [`Pose`] without heap allocation when `out` already
    /// holds `self.rig.len()` joint transforms.
    pub fn sample_into(&self, _skeleton: &Skeleton, time: f32, out: &mut Pose) {
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local_into(local_time, None, out);
    }

    /// Samples into an existing [`Pose`] using a [`KeyframeCursor`] hint to accelerate
    /// monotonic forward playback in $O(1)$ per animated channel.
    pub fn sample_with_cursor_into(
        &self,
        _skeleton: &Skeleton,
        time: f32,
        cursor: &mut KeyframeCursor,
        out: &mut Pose,
    ) {
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local_into(local_time, Some(cursor), out);
    }

    /// Samples a pose suitable for admission into a runtime or GPU palette.
    /// # Errors
    /// Rejects non-finite time, mismatched rig layouts and invalid interpolated TRS.
    pub fn try_sample(&self, skeleton: &Skeleton, time: f32) -> Result<Pose, AnimationError> {
        let mut out = Pose {
            rig: self.rig.clone(),
            local: Vec::with_capacity(self.rig.len()),
        };
        self.try_sample_into(skeleton, time, &mut out)?;
        Ok(out)
    }

    /// Samples and validates directly into `out`, reusing `out`'s allocation when sized.
    ///
    /// # Errors
    /// Rejects non-finite time, mismatched rig layouts, and invalid interpolated TRS.
    pub fn try_sample_into(
        &self,
        skeleton: &Skeleton,
        time: f32,
        out: &mut Pose,
    ) -> Result<(), AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        let local_time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.try_sample_local_into(skeleton, local_time, None, out)
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

    pub(crate) fn try_sample_local(
        &self,
        skeleton: &Skeleton,
        time: f32,
    ) -> Result<Pose, AnimationError> {
        let mut out = Pose {
            rig: self.rig.clone(),
            local: Vec::with_capacity(self.rig.len()),
        };
        self.try_sample_local_into(skeleton, time, None, &mut out)?;
        Ok(out)
    }

    pub(crate) fn try_sample_local_into(
        &self,
        skeleton: &Skeleton,
        time: f32,
        cursor: Option<&mut KeyframeCursor>,
        out: &mut Pose,
    ) -> Result<(), AnimationError> {
        if self.tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: self.tracks.len(),
            });
        }
        if !self.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        self.sample_local_into(time, cursor, out);
        for (joint, transform) in out.local.iter().enumerate() {
            if !transform.is_valid() {
                return Err(AnimationError::InvalidPose(joint));
            }
        }
        Ok(())
    }

    pub(crate) fn phase(&self, time: f64) -> f64 {
        let duration = f64::from(self.duration);
        match self.playback {
            Playback::Loop => time.rem_euclid(duration),
            Playback::Clamp => time.clamp(0.0, duration),
        }
    }

    pub(crate) fn try_sample_clock(
        &self,
        skeleton: &Skeleton,
        time: f64,
    ) -> Result<Pose, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        // Reduce once in the clock's precision. A phase just below the seam can
        // round to duration in f32; wrapping it again would publish the first
        // pose before the clock actually crosses the seam.
        self.try_sample_local(skeleton, self.phase(time) as f32)
    }

    pub(crate) fn sample_local_slice(&self, time: f32, out: &mut [Transform]) {
        debug_assert_eq!(out.len(), self.rig.len());
        for (index, slot) in out.iter_mut().enumerate() {
            if let Some(constant) = self.constant_transforms[index] {
                *slot = constant;
                continue;
            }
            let joint = &self.rig[index];
            let track = &self.tracks[index];
            let mode = &self.interpolation[index];
            let tangents = &self.tangents[index];
            *slot = Transform {
                translation: sample_vec3_hinted(
                    &track.translations,
                    time,
                    joint.bind_local.translation,
                    mode.translation,
                    &tangents.translation,
                    None,
                ),
                rotation: sample_quat_hinted(
                    &track.rotations,
                    time,
                    joint.bind_local.rotation,
                    mode.rotation,
                    &tangents.rotation,
                    None,
                ),
                scale: sample_vec3_hinted(
                    &track.scales,
                    time,
                    joint.bind_local.scale,
                    mode.scale,
                    &tangents.scale,
                    None,
                ),
            };
        }
    }

    fn sample_local_into(
        &self,
        time: f32,
        mut cursor: Option<&mut KeyframeCursor>,
        out: &mut Pose,
    ) {
        if !Arc::ptr_eq(&out.rig, &self.rig) {
            out.rig = self.rig.clone();
        }
        let len = self.rig.len();
        out.local.clear();
        out.local.reserve(len);
        if let Some(c) = cursor.as_deref_mut() {
            c.ensure_len(len);
        }
        for index in 0..len {
            if let Some(constant) = self.constant_transforms[index] {
                out.local.push(constant);
                continue;
            }
            let joint = &self.rig[index];
            let track = &self.tracks[index];
            let mode = &self.interpolation[index];
            let tangents = &self.tangents[index];
            let mut hints = cursor.as_deref_mut().map(|c| &mut c.joints[index]);
            let translation = sample_vec3_hinted(
                &track.translations,
                time,
                joint.bind_local.translation,
                mode.translation,
                &tangents.translation,
                hints.as_deref_mut().map(|h| &mut h[0]),
            );
            let rotation = sample_quat_hinted(
                &track.rotations,
                time,
                joint.bind_local.rotation,
                mode.rotation,
                &tangents.rotation,
                hints.as_deref_mut().map(|h| &mut h[1]),
            );
            let scale = sample_vec3_hinted(
                &track.scales,
                time,
                joint.bind_local.scale,
                mode.scale,
                &tangents.scale,
                hints.map(|h| &mut h[2]),
            );
            out.local.push(Transform {
                translation,
                rotation,
                scale,
            });
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
            if let Some(constant) = self.constant_transforms[index] {
                translations.push(constant.translation);
                rotations.push(constant.rotation);
                scales.push(constant.scale);
                continue;
            }
            let joint = &self.rig[index];
            let track = &self.tracks[index];
            let mode = &self.interpolation[index];
            let tangents = &self.tangents[index];
            translations.push(sample_vec3_hinted(
                &track.translations,
                time,
                joint.bind_local.translation,
                mode.translation,
                &tangents.translation,
                None,
            ));
            rotations.push(sample_quat_hinted(
                &track.rotations,
                time,
                joint.bind_local.rotation,
                mode.rotation,
                &tangents.rotation,
                None,
            ));
            scales.push(sample_vec3_hinted(
                &track.scales,
                time,
                joint.bind_local.scale,
                mode.scale,
                &tangents.scale,
                None,
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
#[inline]
pub(crate) fn lerp_vec3(from: Vec3, to: Vec3, weight: f32) -> Vec3 {
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

#[inline]
fn find_upper<T: Timed>(keys: &[T], time: f32, hint: Option<&mut usize>) -> usize {
    if let Some(cursor) = hint {
        let idx = (*cursor).clamp(1, keys.len() - 1);
        if keys[idx - 1].time() <= time {
            if time < keys[idx].time() {
                return idx;
            }
            if idx + 1 < keys.len() && time < keys[idx + 1].time() {
                *cursor = idx + 1;
                return idx + 1;
            }
        }
        let found = keys.partition_point(|key| key.time() <= time);
        *cursor = found;
        found
    } else {
        keys.partition_point(|key| key.time() <= time)
    }
}

fn sample_vec3_hinted(
    keys: &[Vec3Key],
    time: f32,
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
    hint: Option<&mut usize>,
) -> Vec3 {
    if mode == Interpolation::Linear
        && keys.len() > 1
        && time > keys[0].time
        && time < keys[keys.len() - 1].time
    {
        let upper = find_upper(keys, time, hint);
        let from = &keys[upper - 1];
        let to = &keys[upper];
        let alpha = (time - from.time) / (to.time - from.time);
        return lerp_vec3(from.value, to.value, alpha);
    }
    precision::sample_vector(keys, f64::from(time), fallback, mode, tangents).as_vec3()
}

fn sample_quat_hinted(
    keys: &[QuatKey],
    time: f32,
    fallback: Quat,
    mode: Interpolation,
    tangents: &[[Vec4; 2]],
    hint: Option<&mut usize>,
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
        let upper = find_upper(keys, time, hint);
        return keys[upper - 1].value;
    }
    if mode == Interpolation::Linear {
        let upper = find_upper(keys, time, hint);
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

#[cfg(test)]
pub(crate) fn sample_vec3(
    keys: &[Vec3Key],
    time: f32,
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
) -> Vec3 {
    sample_vec3_hinted(keys, time, fallback, mode, tangents, None)
}

#[cfg(test)]
pub(crate) fn sample_quat(
    keys: &[QuatKey],
    time: f32,
    fallback: Quat,
    mode: Interpolation,
    tangents: &[[Vec4; 2]],
) -> Quat {
    sample_quat_hinted(keys, time, fallback, mode, tangents, None)
}

pub(crate) trait Timed {
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
