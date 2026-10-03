//! Validated skeletal animation sampling and skin-matrix generation.

mod root_curve;
mod root_rotation;
pub use root_rotation::{
    MAX_ROOT_ROTATION_CACHE_KEYS, MAX_ROOT_ROTATION_KEYS, MAX_ROOT_ROTATION_SPANS,
    RootRotationCurve, RootRotationPath, RootRotationSpan,
};

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use glam::{Mat4, Quat, Vec3, Vec4};

pub const MAX_JOINTS: usize = 256;

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
    ///
    /// # Errors
    ///
    /// Rejects empty/oversized skeletons, duplicate/empty names, invalid transforms, non-finite
    /// inverse bind matrices, and parents that do not precede their child.
    pub fn new(joints: Vec<Joint>) -> Result<Self, AnimationError> {
        if joints.is_empty() || joints.len() > MAX_JOINTS {
            return Err(AnimationError::InvalidJointCount(joints.len()));
        }
        for (index, joint) in joints.iter().enumerate() {
            if joint.name.is_empty()
                || joints[..index]
                    .iter()
                    .any(|candidate| candidate.name == joint.name)
            {
                return Err(AnimationError::InvalidJointName(index));
            }
            if !joint.bind_local.is_valid() || !joint.inverse_bind.is_finite() {
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
        }
        Ok(Self {
            joints: joints.into(),
        })
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
    rotation_curves: Arc<[OnceLock<Result<RootRotationCurve, AnimationError>>]>,
    rotation_cache_keys: Arc<AtomicUsize>,
    name: Arc<str>,
    duration: f32,
    playback: Playback,
    tracks: Arc<[JointTrack]>,
    interpolation: Arc<[TrackInterpolation]>,
    tangents: Arc<[JointTangents]>,
    constant_transforms: Arc<[Option<Transform>]>,
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
        Ok(Self {
            constant_transforms,
            rig: skeleton.joints.clone(),
            root_curve,
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

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }

    fn motion_curve(&self, joint: u16) -> Arc<root_curve::RootCurve> {
        if joint == 0 {
            return self.root_curve.clone();
        }
        let i = usize::from(joint);
        Arc::new(root_curve::RootCurve::new(
            &self.tracks[i].translations,
            self.interpolation[i].translation,
            &self.tangents[i].translation,
            self.duration,
            self.playback,
        ))
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
        if self.tracks.len() != skeleton.joints.len() {
            return Err(AnimationError::TrackCountMismatch {
                expected: skeleton.joints.len(),
                actual: self.tracks.len(),
            });
        }
        if !self.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let pose = self.sample(skeleton, time);
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
        // Reduce before conversion so accumulated elapsed time never loses phase bits.
        self.try_sample(skeleton, self.phase(time) as f32)
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

    /// Produces `global_joint * inverse_bind` matrices in skeleton order.
    ///
    /// # Errors
    ///
    /// Rejects a pose created for a different skeleton, invalid blend data,
    /// or overflowing global/palette matrices.
    pub fn skin_matrices(&self, skeleton: &Skeleton) -> Result<Vec<Mat4>, AnimationError> {
        if self.local.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let mut global = Vec::with_capacity(self.local.len());
        let mut skin = Vec::with_capacity(self.local.len());
        for (index, (local, joint)) in self.local.iter().zip(skeleton.joints.iter()).enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let matrix = joint.parent.map_or_else(
                || local.matrix(),
                |parent| global[usize::from(parent)] * local.matrix(),
            );
            let palette = matrix * joint.inverse_bind;
            if !matrix.is_finite() || !palette.is_finite() {
                return Err(AnimationError::InvalidPose(index));
            }
            global.push(matrix);
            skin.push(palette);
        }
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
                    translation: a.translation.lerp(b.translation, weight),
                    rotation: a.rotation.slerp(b.rotation, weight).normalize(),
                    scale: a.scale.lerp(b.scale, weight),
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
        let joint = skeleton.joints.get(index)
            .ok_or(AnimationError::InvalidRootMotionJoint(self.root_motion_joint))?;
        if !rigs_match(&self.pose.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let local = self.pose.local.get_mut(index).ok_or(AnimationError::InvalidPose(index))?;
        if !local.is_valid() { return Err(AnimationError::InvalidPose(index)); }
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
        let curve = self.current.motion_curve(joint);
        let source = self
            .transition
            .as_ref()
            .map(|t| t.source.motion_curve(joint));
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
        if !duration.is_finite() || !(0.0..=60.0).contains(&duration) {
            return Err(AnimationError::InvalidTransitionDuration);
        }
        if !rigs_match(&self.current.rig, &next.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let next_curve = next.motion_curve(self.motion_joint);
        if duration == 0.0 {
            self.motion_curve = next_curve;
            self.current = next;
            self.time = 0.0;
            self.transition = None;
            return Ok(());
        }
        let source_pose = if let Some(transition) = &self.transition {
            let skeleton = Skeleton {
                joints: self.current.rig.clone(),
            };
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
        self.time = 0.0;
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
        // Clip references are shared; only the small clock/transition state is
        // staged. Failed pose or root-motion evaluation never publishes it.
        let mut candidate = self.clone();
        let frame = candidate.advance_candidate(skeleton, dt)?;
        *self = candidate;
        Ok(frame)
    }

    fn advance_candidate(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
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
        let dt = f64::from(dt);
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
            if weight >= 1.0 {
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
fn constant_transform(
    bind: Transform,
    track: &JointTrack,
    mode: TrackInterpolation,
    tangents: &JointTangents,
) -> Option<Transform> {
    fn vec_channel(
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
        translation: vec_channel(
            &track.translations,
            bind.translation,
            mode.translation,
            &tangents.translation,
        )?,
        rotation,
        scale: vec_channel(&track.scales, bind.scale, mode.scale, &tangents.scale)?,
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

fn sample_vec3(
    keys: &[Vec3Key],
    time: f32,
    fallback: Vec3,
    mode: Interpolation,
    tangents: &[[Vec3; 2]],
) -> Vec3 {
    let Some(first) = keys.first() else {
        return fallback;
    };
    if time <= first.time {
        return first.value;
    }
    let last = &keys[keys.len() - 1];
    if time >= last.time {
        return last.value;
    }
    sample_segment(keys, time).map_or_else(
        || first.value,
        |(index, from, to, alpha)| match mode {
            Interpolation::Step => from.value,
            Interpolation::Linear => {
                if from.value == to.value {
                    from.value
                } else {
                    from.value.lerp(to.value, alpha)
                }
            }
            Interpolation::CubicSpline => {
                if alpha == 0.0
                    || (from.value == to.value
                        && tangents[index][1] == Vec3::ZERO
                        && tangents[index + 1][0] == Vec3::ZERO)
                {
                    from.value
                } else {
                    hermite(
                        from.value,
                        to.value,
                        tangents[index][1],
                        tangents[index + 1][0],
                        alpha,
                        to.time - from.time,
                    )
                }
            }
        },
    )
}

fn sample_quat(
    keys: &[QuatKey],
    time: f32,
    fallback: Quat,
    mode: Interpolation,
    tangents: &[[Vec4; 2]],
) -> Quat {
    let Some(first) = keys.first() else {
        return fallback;
    };
    if time <= first.time {
        return first.value;
    }
    let last = &keys[keys.len() - 1];
    if time >= last.time {
        return last.value;
    }
    sample_segment(keys, time).map_or_else(
        || first.value,
        |(index, from, to, alpha)| match mode {
            Interpolation::Step => from.value,
            Interpolation::Linear => from.value.slerp(to.value, alpha).normalize(),
            Interpolation::CubicSpline => {
                if alpha == 0.0 {
                    return from.value;
                }
                let value = hermite(
                    Vec4::from_array(from.value.to_array()),
                    Vec4::from_array(to.value.to_array()),
                    tangents[index][1],
                    tangents[index + 1][0],
                    alpha,
                    to.time - from.time,
                );
                let scale = value.abs().max_element();
                if !value.is_finite() || scale == 0.0 {
                    // The existing pose/palette admission rejects this invalid
                    // sample before committing clocks or GPU resources.
                    Quat::from_array([f32::NAN; 4])
                } else {
                    Quat::from_array((value / scale).normalize().to_array())
                }
            }
        },
    )
}

fn hermite<T>(from: T, to: T, outgoing: T, incoming: T, t: f32, duration: f32) -> T
where
    T: Copy + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    let t2 = t * t;
    let t3 = t2 * t;
    from * (2.0 * t3 - 3.0 * t2 + 1.0)
        + outgoing * (duration * (t3 - 2.0 * t2 + t))
        + to * (-2.0 * t3 + 3.0 * t2)
        + incoming * (duration * (t3 - t2))
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

fn sample_segment<T: Timed>(keys: &[T], time: f32) -> Option<(usize, &T, &T, f32)> {
    let upper = keys.partition_point(|key| key.time() <= time);
    if upper == 0 || upper == keys.len() {
        return None;
    }
    let from = &keys[upper - 1];
    let to = &keys[upper];
    let alpha = (time - from.time()) / (to.time() - from.time());
    Some((upper - 1, from, to, alpha))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackError {
    InvalidTangents,
    InvalidTime,
    InvalidValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnimationError {
    RootRotationBudget,
    InvalidRootRotationCurve,
    InvalidRootMotionJoint(u16),
    SkeletonMismatch,
    InvalidSampleTime,
    InvalidJointCount(usize),
    InvalidJointName(usize),
    InvalidJointTransform(usize),
    InvalidParent { joint: usize, parent: u16 },
    InvalidClipHeader,
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
    NumericalOverflow,
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "animation error: {self:?}")
    }
}

impl std::error::Error for AnimationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_binding_accepts_exact_reconstruction_and_rejects_same_count_foreign_layouts() {
        let source = skeleton();
        let clip = AnimationClip::new(
            "bound",
            2.,
            Playback::Clamp,
            vec![JointTrack::default(); 2],
            &source,
        )
        .unwrap();
        let reconstructed = Skeleton::new(source.joints().to_vec()).unwrap();
        assert!(!Arc::ptr_eq(&source.joints, &reconstructed.joints));
        let pose = clip.try_sample(&reconstructed, 0.5).unwrap();
        assert_eq!(
            pose.skin_matrices(&reconstructed).unwrap(),
            source.bind_pose().skin_matrices(&source).unwrap()
        );
        assert!(Pose::blend(&pose, &reconstructed.bind_pose(), 0.5).is_ok());
        for change in 0..5 {
            let mut joints = source.joints().to_vec();
            match change {
                0 => joints[1].name = Arc::from("other bone"),
                1 => joints[1].parent = None,
                2 => joints[1].bind_local.translation = Vec3::Z,
                3 => joints[1].inverse_bind = Mat4::from_translation(-Vec3::Z),
                _ => {
                    joints[0].name = Arc::from("hand");
                    joints[1].name = Arc::from("root");
                }
            }
            let foreign = Skeleton::new(joints).unwrap();
            assert!(matches!(
                clip.try_sample(&foreign, 0.5),
                Err(AnimationError::SkeletonMismatch)
            ));
            assert!(matches!(
                pose.skin_matrices(&foreign),
                Err(AnimationError::SkeletonMismatch)
            ));
            for weight in [0., 0.5, 1.] {
                assert!(matches!(
                    Pose::blend(&pose, &foreign.bind_pose(), weight),
                    Err(AnimationError::SkeletonMismatch)
                ));
            }
            // The legacy sampler cannot substitute foreign bind defaults or relabel a clip.
            let unchecked = clip.sample(&foreign, 0.5);
            assert_eq!(unchecked, source.bind_pose());
            assert!(matches!(
                unchecked.skin_matrices(&foreign),
                Err(AnimationError::SkeletonMismatch)
            ));
        }
    }

    #[test]
    fn foreign_transition_and_advance_preserve_active_crossfade() {
        let source = skeleton();
        let make = |rig: &Skeleton, name: &str| {
            Arc::new(
                AnimationClip::new(
                    name,
                    2.,
                    Playback::Loop,
                    vec![JointTrack::default(); 2],
                    rig,
                )
                .unwrap(),
            )
        };
        let first = make(&source, "first");
        let reconstructed = Skeleton::new(source.joints().to_vec()).unwrap();
        let second = make(&reconstructed, "second");
        let mut animator = Animator::new(first);
        animator.advance(&source, 0.2).unwrap();
        animator.transition_to(second, 0.5).unwrap();
        animator.advance(&source, 0.1).unwrap();
        let mut control = animator.clone();
        let mut joints = source.joints().to_vec();
        joints[1].parent = None;
        let foreign = Skeleton::new(joints).unwrap();
        for duration in [0., 0.5] {
            assert!(matches!(
                animator.transition_to(make(&foreign, "foreign"), duration),
                Err(AnimationError::SkeletonMismatch)
            ));
            assert!(Arc::ptr_eq(&animator.current, &control.current));
            assert_eq!(animator.time, control.time);
            assert_eq!(
                animator.transition.as_ref().unwrap().elapsed,
                control.transition.as_ref().unwrap().elapsed
            );
            assert_eq!(
                animator.transition.as_ref().unwrap().source_time,
                control.transition.as_ref().unwrap().source_time
            );
        }
        assert!(matches!(
            animator.advance(&foreign, 0.1),
            Err(AnimationError::SkeletonMismatch)
        ));
        let actual = animator.advance(&source, 0.1).unwrap();
        let expected = control.advance(&source, 0.1).unwrap();
        assert_eq!(actual.pose, expected.pose);
        assert_eq!(actual.skin_matrices, expected.skin_matrices);
        assert_eq!(actual.transition_weight, expected.transition_weight);
        assert_eq!(actual.root_motion, expected.root_motion);
    }

    #[test]
    fn interrupted_crossfade_starts_from_displayed_pose_and_releases_snapshot() {
        let rig = skeleton();
        let constant = |name: &str, x: f32| {
            let mut tracks = vec![JointTrack::default(); 2];
            tracks[0].translations = vec![Vec3Key {
                time: 0.,
                value: Vec3::X * x,
            }];
            Arc::new(AnimationClip::new(name, 2., Playback::Clamp, tracks, &rig).unwrap())
        };
        let mut animator = Animator::new(constant("a", 0.));
        animator.transition_to(constant("b", 10.), 1.).unwrap();
        let displayed = animator.advance(&rig, 0.25).unwrap();
        assert_eq!(displayed.pose.local()[0].translation.x, 2.5);
        animator.transition_to(constant("c", 20.), 1.).unwrap();
        let snapshot = Arc::downgrade(
            animator
                .transition
                .as_ref()
                .unwrap()
                .source_pose
                .as_ref()
                .unwrap(),
        );
        assert_eq!(animator.advance(&rig, 0.).unwrap().pose, displayed.pose);
        assert_eq!(
            animator.advance(&rig, 0.2).unwrap().pose.local()[0]
                .translation
                .x,
            6.
        );
        let next_displayed = animator.advance(&rig, 0.).unwrap().pose;
        animator.transition_to(constant("d", -10.), 0.5).unwrap();
        assert!(snapshot.upgrade().is_none());
        assert_eq!(animator.advance(&rig, 0.).unwrap().pose, next_displayed);
        let snapshot = Arc::downgrade(
            animator
                .transition
                .as_ref()
                .unwrap()
                .source_pose
                .as_ref()
                .unwrap(),
        );
        assert_eq!(
            animator.advance(&rig, 0.5).unwrap().pose.local()[0]
                .translation
                .x,
            -10.
        );
        assert!(animator.transition.is_none());
        assert!(snapshot.upgrade().is_none());
    }

    #[test]
    fn interrupted_snapshot_is_shared_and_failed_advance_keeps_it() {
        let rig = skeleton();
        let mut animator = Animator::new(root_clip(&rig, 0.));
        animator.transition_to(root_clip(&rig, 2.), 1.).unwrap();
        animator.advance(&rig, 0.25).unwrap();
        let displayed = animator.advance(&rig, 0.).unwrap().pose;
        animator.transition_to(root_clip(&rig, 4.), 1.).unwrap();
        assert_eq!(animator.advance(&rig, 0.).unwrap().pose, displayed);
        let mut control = animator.clone();
        let left = animator
            .transition
            .as_ref()
            .unwrap()
            .source_pose
            .as_ref()
            .unwrap();
        let right = control
            .transition
            .as_ref()
            .unwrap()
            .source_pose
            .as_ref()
            .unwrap();
        assert!(Arc::ptr_eq(left, right));
        assert!(animator.advance(&rig, f32::NAN).is_err());
        assert_eq!(animator.time, control.time);
        let actual = animator.advance(&rig, 0.1).unwrap();
        let expected = control.advance(&rig, 0.1).unwrap();
        assert_eq!(actual.pose, expected.pose);
        assert_eq!(actual.skin_matrices, expected.skin_matrices);
    }

    #[test]
    fn interrupted_fade_preserves_rotated_scaled_hierarchical_palette() {
        let rig = skeleton();
        let clip = |name: &str, angle: f32, scale: f32| {
            let mut tracks = vec![JointTrack::default(); 2];
            for track in &mut tracks {
                track.rotations = vec![QuatKey {
                    time: 0.,
                    value: Quat::from_rotation_y(angle),
                }];
                track.scales = vec![Vec3Key {
                    time: 0.,
                    value: Vec3::splat(scale),
                }];
            }
            Arc::new(AnimationClip::new(name, 1., Playback::Clamp, tracks, &rig).unwrap())
        };
        let mut animator = Animator::new(clip("a", -0.6, 0.8));
        animator.transition_to(clip("b", 1.2, 1.7), 1.).unwrap();
        let before = animator.advance(&rig, 0.3).unwrap();
        animator.transition_to(clip("c", -1.1, 1.1), 0.4).unwrap();
        let after = animator.advance(&rig, 0.).unwrap();
        for (a, b) in before.pose.local().iter().zip(after.pose.local()) {
            assert!(a.translation.abs_diff_eq(b.translation, 1e-6));
            assert!(a.scale.abs_diff_eq(b.scale, 1e-6));
            assert!(a.rotation.angle_between(b.rotation) < 1e-5);
        }
        for (a, b) in before.skin_matrices.iter().zip(after.skin_matrices.iter()) {
            assert!(a.abs_diff_eq(*b, 2e-6));
        }
        let frame = animator.advance(&rig, 0.4).unwrap();
        assert!(frame.skin_matrices.iter().all(|m| m.is_finite()));
        assert!(animator.transition.is_none());
    }

    #[test]
    fn root_motion_crossfade_blends_velocities_and_splits_completion_tail() {
        let rig = skeleton();
        let mut animator = Animator::new(root_clip(&rig, 2.));
        animator.advance(&rig, 0.8).unwrap();
        animator.transition_to(root_clip(&rig, 6.), 1.).unwrap();
        let frame = animator.advance(&rig, 0.25).unwrap();
        assert!((frame.root_motion.x - 0.625).abs() < 1e-5);
        let mut single = animator.clone();
        let mut split = animator.clone();
        let whole = single.advance(&rig, 1.).unwrap().root_motion;
        let pieces = split.advance(&rig, 0.5).unwrap().root_motion
            + split.advance(&rig, 0.5).unwrap().root_motion;
        assert!(whole.abs_diff_eq(pieces, 1e-5));
        assert!((whole.x - 4.875).abs() < 1e-5);
        assert!(single.transition.is_none());
        single.set_speed(0.).unwrap();
        assert_eq!(single.advance(&rig, 0.1).unwrap().root_motion, Vec3::ZERO);
    }

    #[test]
    fn interrupted_frozen_source_has_no_root_displacement_and_pause_is_zero() {
        let rig = skeleton();
        let mut animator = Animator::new(root_clip(&rig, 2.));
        animator.transition_to(root_clip(&rig, 6.), 1.).unwrap();
        animator.advance(&rig, 0.2).unwrap();
        animator.transition_to(root_clip(&rig, 10.), 1.).unwrap();
        assert!((animator.advance(&rig, 0.2).unwrap().root_motion.x - 0.2).abs() < 1e-5);
        animator.set_speed(0.).unwrap();
        assert_eq!(animator.advance(&rig, 0.1).unwrap().root_motion, Vec3::ZERO);
    }

    #[test]
    fn completed_fade_does_not_admit_invalid_zero_weight_source() {
        let rig = skeleton();
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[0].rotations = vec![
            QuatKey {
                time: 0.,
                value: Quat::IDENTITY,
            },
            QuatKey {
                time: 1.,
                value: -Quat::IDENTITY,
            },
        ];
        let mut modes = vec![TrackInterpolation::default(); 2];
        modes[0].rotation = Interpolation::CubicSpline;
        let mut tangents = vec![JointTangents::default(); 2];
        tangents[0].rotation = vec![[Vec4::ZERO; 2]; 2];
        let source = Arc::new(
            AnimationClip::new_with_tangents(
                "singular midpoint",
                1.,
                Playback::Clamp,
                tracks,
                modes,
                tangents,
                &rig,
            )
            .unwrap(),
        );
        assert!(source.try_sample(&rig, 0.5).is_err());
        let mut animator = Animator::new(source);
        let target = root_clip(&rig, 2.);
        animator.transition_to(target.clone(), 0.5).unwrap();
        animator.advance(&rig, 0.25).unwrap();
        let frame = animator.advance(&rig, 0.25).unwrap();
        assert_eq!(frame.pose, target.try_sample(&rig, 0.5).unwrap());
        assert_eq!(frame.transition_weight, 1.);
        assert!(animator.transition.is_none());
    }

    #[test]
    fn animator_root_integral_uses_step_event_weights_and_substeps() {
        let rig = skeleton();
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[0].translations = vec![
            Vec3Key {
                time: 0.,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 0.25,
                value: Vec3::X * 2.,
            },
            Vec3Key {
                time: 0.75,
                value: Vec3::X * 5.,
            },
        ];
        let mut modes = vec![TrackInterpolation::default(); 2];
        modes[0].translation = Interpolation::Step;
        let target = Arc::new(
            AnimationClip::new_with_interpolation(
                "step motion",
                1.,
                Playback::Loop,
                tracks,
                modes,
                &rig,
            )
            .unwrap(),
        );
        let mut whole = Animator::new(root_clip(&rig, 0.));
        whole.transition_to(target, 1.).unwrap();
        let mut split = whole.clone();
        let full = whole.advance(&rig, 1.).unwrap().root_motion;
        let mut sum = Vec3::ZERO;
        for _ in 0..4 {
            sum += split.advance(&rig, 0.25).unwrap().root_motion;
        }
        assert!(full.abs_diff_eq(Vec3::X * 2.75, 1e-6));
        assert!(sum.abs_diff_eq(full, 1e-6));
    }

    #[test]
    fn animator_root_integral_uses_nonzero_cubic_derivatives_in_seconds() {
        let rig = skeleton();
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[0].translations = vec![
            Vec3Key {
                time: 0.,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 2.,
                value: Vec3::X * 2.,
            },
        ];
        let mut modes = vec![TrackInterpolation::default(); 2];
        modes[0].translation = Interpolation::CubicSpline;
        let mut tangents = vec![JointTangents::default(); 2];
        tangents[0].translation = vec![[Vec3::ZERO, Vec3::X * 4.], [Vec3::X * (-2.), Vec3::ZERO]];
        let target = Arc::new(
            AnimationClip::new_with_tangents(
                "cubic motion",
                2.,
                Playback::Clamp,
                tracks,
                modes,
                tangents,
                &rig,
            )
            .unwrap(),
        );
        let mut animator = Animator::new(root_clip(&rig, 0.));
        animator.transition_to(target, 2.).unwrap();
        let first = animator.advance(&rig, 1.).unwrap().root_motion;
        let second = animator.advance(&rig, 1.).unwrap().root_motion;
        assert!(first.abs_diff_eq(Vec3::X * 0.5, 1e-6));
        assert!((first + second).abs_diff_eq(Vec3::ZERO, 1e-6));
    }

    #[test]
    fn selected_motion_joint_updates_both_fade_curves_and_preserves_invalid_selection() {
        let rig = skeleton();
        let clip = |name: &str, speed: f32| {
            let mut tracks = vec![JointTrack::default(); 2];
            tracks[0].translations = vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::X * 100.,
                },
            ];
            tracks[1].translations = vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::Y,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::Y * (1. + speed),
                },
            ];
            Arc::new(AnimationClip::new(name, 1., Playback::Loop, tracks, &rig).unwrap())
        };
        let mut animator = Animator::new(clip("a", 2.));
        animator.transition_to(clip("b", 6.), 1.).unwrap();
        animator.advance(&rig, 0.25).unwrap();
        animator.set_root_motion_joint(1).unwrap();
        let mut control = animator.clone();
        assert!(matches!(
            animator.set_root_motion_joint(2),
            Err(AnimationError::InvalidRootMotionJoint(2))
        ));
        assert_eq!(animator.root_motion_joint(), 1);
        assert!(Arc::ptr_eq(&animator.motion_curve, &control.motion_curve));
        let actual = animator.advance(&rig, 0.25).unwrap();
        let expected = control.advance(&rig, 0.25).unwrap();
        assert_eq!(actual.root_motion_joint, 1);
        assert!(actual.root_motion.abs_diff_eq(Vec3::Y * 0.875, 1e-6));
        assert_eq!(actual.root_motion, expected.root_motion);
        assert_eq!(actual.pose, expected.pose);
        animator.transition_to(clip("c", 10.), 0.).unwrap();
        assert_eq!(
            animator.advance(&rig, 0.25).unwrap().root_motion,
            Vec3::Y * 2.5
        );
    }

    fn skeleton() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                name: Arc::from("root"),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: Arc::from("hand"),
                parent: Some(0),
                bind_local: Transform {
                    translation: Vec3::Y,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::from_translation(-Vec3::Y),
            },
        ])
        .unwrap()
    }

    #[test]
    fn constant_joint_proofs_cover_authored_nonbind_values_and_unused_cubic_tangents() {
        let rig = skeleton();
        let expected = Transform {
            translation: Vec3::new(7., 8., 9.),
            rotation: Quat::from_rotation_y(0.4),
            scale: Vec3::new(-2., 3., 4.),
        };
        for mode in [
            Interpolation::Linear,
            Interpolation::Step,
            Interpolation::CubicSpline,
        ] {
            let track = JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: expected.translation,
                    },
                    Vec3Key {
                        time: 1.,
                        value: expected.translation,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 0.,
                        value: expected.rotation,
                    },
                    QuatKey {
                        time: 1.,
                        value: expected.rotation,
                    },
                ],
                scales: vec![
                    Vec3Key {
                        time: 0.,
                        value: expected.scale,
                    },
                    Vec3Key {
                        time: 1.,
                        value: expected.scale,
                    },
                ],
            };
            let mut tangents = JointTangents::default();
            if mode == Interpolation::CubicSpline {
                tangents.translation = vec![
                    [Vec3::splat(f32::MAX), Vec3::ZERO],
                    [Vec3::ZERO, Vec3::splat(f32::MAX)],
                ];
                tangents.rotation = vec![
                    [Vec4::splat(f32::MAX), Vec4::ZERO],
                    [Vec4::ZERO, Vec4::splat(f32::MAX)],
                ];
                tangents.scale = tangents.translation.clone();
            }
            let clip = AnimationClip::new_with_tangents(
                "constant",
                1.,
                Playback::Loop,
                vec![track, JointTrack::default()],
                vec![
                    TrackInterpolation {
                        translation: mode,
                        rotation: mode,
                        scale: mode,
                    },
                    TrackInterpolation::default(),
                ],
                vec![tangents, JointTangents::default()],
                &rig,
            )
            .unwrap();
            assert_eq!(clip.constant_joint_transform(0), Some(expected));
            assert_eq!(
                clip.constant_joint_transform(1),
                Some(rig.joints()[1].bind_local)
            );
            assert_eq!(clip.constant_joint_transform(2), None);
            assert!(!clip.joint_uses_bind_pose(0));
            for time in [-1., 0., 0.1, 0.20000002, 0.25, 0.5, 0.75, 1., 2.] {
                let sample = clip.try_sample(&rig, time).unwrap();
                assert_eq!(sample.local()[0].translation, expected.translation);
                assert_eq!(sample.local()[0].scale, expected.scale);
                assert!(
                    clip.try_sample(&rig, time).unwrap().local()[0]
                        .matrix()
                        .abs_diff_eq(expected.matrix(), 2e-6)
                );
            }
        }
    }

    #[test]
    fn constant_joint_proofs_reject_hidden_cubic_motion_and_accept_linear_antipodes() {
        let rig = skeleton();
        let q = Quat::from_rotation_y(0.4);
        for mode in [Interpolation::Linear, Interpolation::Step] {
            let track = JointTrack {
                rotations: vec![
                    QuatKey { time: 0., value: q },
                    QuatKey {
                        time: 1.,
                        value: -q,
                    },
                ],
                ..Default::default()
            };
            let clip = AnimationClip::new_with_interpolation(
                "antipodes",
                1.,
                Playback::Clamp,
                vec![track, JointTrack::default()],
                vec![
                    TrackInterpolation {
                        rotation: mode,
                        ..Default::default()
                    },
                    TrackInterpolation::default(),
                ],
                &rig,
            )
            .unwrap();
            assert_eq!(clip.constant_joint_transform(0).unwrap().rotation, q);
        }
        for channel in 0..4 {
            let mut track = JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 1.,
                        value: Vec3::ZERO,
                    },
                ],
                rotations: vec![
                    QuatKey { time: 0., value: q },
                    QuatKey { time: 1., value: q },
                ],
                scales: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::ONE,
                    },
                    Vec3Key {
                        time: 1.,
                        value: Vec3::ONE,
                    },
                ],
            };
            let mut tangents = JointTangents {
                translation: vec![[Vec3::ZERO; 2]; 2],
                rotation: vec![[Vec4::ZERO; 2]; 2],
                scale: vec![[Vec3::ZERO; 2]; 2],
            };
            match channel {
                0 => tangents.translation[0][1] = Vec3::X,
                1 => tangents.rotation[0][1] = Vec4::X,
                2 => tangents.scale[0][1] = Vec3::X,
                _ => track.rotations[1].value = -q,
            }
            let clip = AnimationClip::new_with_tangents(
                "hidden motion",
                1.,
                Playback::Clamp,
                vec![track, JointTrack::default()],
                vec![
                    TrackInterpolation {
                        translation: Interpolation::CubicSpline,
                        rotation: Interpolation::CubicSpline,
                        scale: Interpolation::CubicSpline,
                    },
                    TrackInterpolation::default(),
                ],
                vec![tangents, JointTangents::default()],
                &rig,
            )
            .unwrap();
            assert!(clip.constant_joint_transform(0).is_none());
            if channel == 0 {
                assert!(clip.try_sample(&rig, 0.5).unwrap().local()[0].translation.x > 0.1);
            }
            if channel == 3 {
                assert!(clip.try_sample(&rig, 0.5).is_err());
            }
        }
    }

    #[test]
    fn mirrored_bind_transforms_keep_signed_scale_through_hierarchy_and_palette() {
        for bits in 0_u32..8 {
            let scale = Vec3::new(
                if bits & 1 != 0 { -2.0 } else { 2.0 },
                if bits & 2 != 0 { -3.0 } else { 3.0 },
                if bits & 4 != 0 { -4.0 } else { 4.0 },
            );
            let root = Transform {
                scale,
                rotation: Quat::from_rotation_y(0.4),
                translation: Vec3::new(1.0, 2.0, 3.0),
            };
            let child = Transform {
                translation: Vec3::Y,
                ..Transform::IDENTITY
            };
            let skeleton = Skeleton::new(vec![
                Joint {
                    name: "root".into(),
                    parent: None,
                    bind_local: root,
                    inverse_bind: Mat4::IDENTITY,
                },
                Joint {
                    name: "child".into(),
                    parent: Some(0),
                    bind_local: child,
                    inverse_bind: Mat4::IDENTITY,
                },
            ])
            .unwrap();
            let palette = skeleton.bind_pose().skin_matrices(&skeleton).unwrap();
            assert!(palette[0].abs_diff_eq(root.matrix(), 1e-6));
            assert!(palette[1].abs_diff_eq(root.matrix() * child.matrix(), 1e-6));
            assert_eq!(
                palette[0].determinant().is_sign_negative(),
                bits.count_ones() % 2 == 1
            );
        }
    }

    #[test]
    fn signed_scale_channels_and_blends_reject_only_singular_samples() {
        let skeleton = skeleton();
        let tracks = vec![
            JointTrack {
                scales: vec![
                    Vec3Key {
                        time: 0.0,
                        value: Vec3::ONE,
                    },
                    Vec3Key {
                        time: 2.0,
                        value: Vec3::new(-1.0, 1.0, 1.0),
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ];
        let linear =
            AnimationClip::new("mirror", 2.0, Playback::Clamp, tracks.clone(), &skeleton).unwrap();
        assert_eq!(
            linear.try_sample(&skeleton, 1.5).unwrap().local()[0]
                .scale
                .x,
            -0.5
        );
        assert!(linear.try_sample(&skeleton, 1.0).is_err());
        let a = linear.try_sample(&skeleton, 0.0).unwrap();
        let b = linear.try_sample(&skeleton, 2.0).unwrap();
        assert!(Pose::blend(&a, &b, 0.5).is_err());
        assert_eq!(Pose::blend(&a, &b, 1.0).unwrap(), b);
        let mut animator = Animator::new(Arc::new(linear));
        animator.advance(&skeleton, 0.5).unwrap();
        let before = animator.time;
        assert!(animator.advance(&skeleton, 0.5).is_err());
        assert_eq!(animator.time, before);
        let step = AnimationClip::new_with_interpolation(
            "step mirror",
            2.0,
            Playback::Clamp,
            tracks.clone(),
            vec![
                TrackInterpolation {
                    scale: Interpolation::Step,
                    ..TrackInterpolation::default()
                },
                TrackInterpolation::default(),
            ],
            &skeleton,
        )
        .unwrap();
        assert_eq!(
            step.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
            Vec3::ONE
        );
        assert_eq!(
            step.try_sample(&skeleton, 2.0).unwrap().local()[0].scale,
            Vec3::new(-1.0, 1.0, 1.0)
        );
        let cubic = AnimationClip::new_with_tangents(
            "cubic mirror",
            2.0,
            Playback::Clamp,
            tracks,
            vec![
                TrackInterpolation {
                    scale: Interpolation::CubicSpline,
                    ..TrackInterpolation::default()
                },
                TrackInterpolation::default(),
            ],
            vec![
                JointTangents {
                    scale: vec![[Vec3::ZERO; 2]; 2],
                    ..JointTangents::default()
                },
                JointTangents::default(),
            ],
            &skeleton,
        )
        .unwrap();
        assert!(cubic.try_sample(&skeleton, 1.0).is_err());
        assert!(cubic.try_sample(&skeleton, 1.5).unwrap().local()[0].scale.x < 0.0);
        let mirrored = AnimationClip::new(
            "negative endpoints",
            2.0,
            Playback::Clamp,
            vec![
                JointTrack {
                    scales: vec![
                        Vec3Key {
                            time: 0.0,
                            value: -Vec3::ONE,
                        },
                        Vec3Key {
                            time: 2.0,
                            value: -Vec3::ONE * 2.0,
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            &skeleton,
        )
        .unwrap();
        assert_eq!(
            mirrored.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
            Vec3::splat(-1.5)
        );
    }

    #[test]
    fn hierarchy_and_inverse_bind_produce_skin_matrices() {
        let skeleton = skeleton();
        let matrices = skeleton.bind_pose().skin_matrices(&skeleton).unwrap();
        assert_eq!(matrices.len(), 2);
        assert!(
            matrices
                .iter()
                .all(|matrix| matrix.abs_diff_eq(Mat4::IDENTITY, 1.0e-6))
        );
    }

    #[test]
    fn cubic_derivatives_use_segment_seconds_and_normalize_rotation() {
        let skeleton = skeleton();
        let end = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let tracks = vec![
            JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 1.0,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 3.0,
                        value: Vec3::X * 2.0,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 1.0,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 3.0,
                        value: end,
                    },
                ],
                scales: vec![
                    Vec3Key {
                        time: 1.0,
                        value: Vec3::ONE,
                    },
                    Vec3Key {
                        time: 3.0,
                        value: Vec3::splat(3.0),
                    },
                ],
            },
            JointTrack::default(),
        ];
        let mode = TrackInterpolation {
            translation: Interpolation::CubicSpline,
            rotation: Interpolation::CubicSpline,
            scale: Interpolation::Linear,
        };
        let tangents = vec![
            JointTangents {
                translation: vec![[Vec3::ZERO, Vec3::X * 4.0], [Vec3::X * -2.0, Vec3::ZERO]],
                rotation: vec![[Vec4::ZERO, Vec4::Z * 2.0], [Vec4::ZERO; 2]],
                scale: vec![],
            },
            JointTangents::default(),
        ];
        let clip = AnimationClip::new_with_tangents(
            "cubic",
            4.0,
            Playback::Clamp,
            tracks.clone(),
            vec![mode, TrackInterpolation::default()],
            tangents.clone(),
            &skeleton,
        )
        .unwrap();
        assert_eq!(
            clip.try_sample(&skeleton, 0.0).unwrap().local()[0].translation,
            Vec3::ZERO
        );
        assert!(
            clip.try_sample(&skeleton, 1.5).unwrap().local()[0]
                .translation
                .abs_diff_eq(Vec3::X * 1.625, 1e-6)
        );
        let midpoint = clip.try_sample(&skeleton, 2.0).unwrap();
        assert!(
            midpoint.local()[0]
                .translation
                .abs_diff_eq(Vec3::X * 2.5, 1e-6)
        );
        assert_eq!(midpoint.local()[0].scale, Vec3::splat(2.0));
        let expected = (Vec4::from_array(Quat::IDENTITY.to_array()) * 0.5
            + Vec4::from_array(end.to_array()) * 0.5
            + Vec4::Z * 0.5)
            .normalize();
        assert!(
            Vec4::from_array(midpoint.local()[0].rotation.to_array()).abs_diff_eq(expected, 1e-6)
        );
        assert!(midpoint.local()[0].rotation.is_normalized());
        assert_eq!(
            clip.try_sample(&skeleton, 3.0).unwrap().local()[0].rotation,
            end
        );
        assert_eq!(
            clip.try_sample(&skeleton, 10.0).unwrap().local()[0].translation,
            Vec3::X * 2.0
        );
        assert!(clip.try_sample(&skeleton, f32::NAN).is_err());
        assert!(
            AnimationClip::new_with_interpolation(
                "missing",
                4.0,
                Playback::Clamp,
                tracks.clone(),
                vec![mode, TrackInterpolation::default()],
                &skeleton
            )
            .is_err()
        );
        let mut broken = tangents.clone();
        broken[0].rotation[0][1].x = f32::INFINITY;
        assert!(
            AnimationClip::new_with_tangents(
                "bad",
                4.0,
                Playback::Clamp,
                tracks.clone(),
                vec![mode, TrackInterpolation::default()],
                broken,
                &skeleton
            )
            .is_err()
        );
        let mut short = tracks;
        short[0].translations.truncate(1);
        assert!(
            AnimationClip::new_with_tangents(
                "short",
                4.0,
                Playback::Clamp,
                short,
                vec![mode, TrackInterpolation::default()],
                tangents,
                &skeleton
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_cubic_pose_keeps_animator_clock_and_transition() {
        let skeleton = skeleton();
        let cubic = AnimationClip::new_with_tangents(
            "zero quaternion",
            2.0,
            Playback::Clamp,
            vec![
                JointTrack {
                    rotations: vec![
                        QuatKey {
                            time: 0.0,
                            value: Quat::IDENTITY,
                        },
                        QuatKey {
                            time: 2.0,
                            value: -Quat::IDENTITY,
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            vec![
                TrackInterpolation {
                    rotation: Interpolation::CubicSpline,
                    ..TrackInterpolation::default()
                },
                TrackInterpolation::default(),
            ],
            vec![
                JointTangents {
                    rotation: vec![[Vec4::ZERO; 2]; 2],
                    ..JointTangents::default()
                },
                JointTangents::default(),
            ],
            &skeleton,
        )
        .unwrap();
        assert!(matches!(
            cubic.try_sample(&skeleton, 1.0),
            Err(AnimationError::InvalidPose(0))
        ));
        let valid = Arc::new(
            AnimationClip::new(
                "bind",
                2.0,
                Playback::Clamp,
                vec![JointTrack::default(); 2],
                &skeleton,
            )
            .unwrap(),
        );
        let mut animator = Animator::new(valid);
        animator.transition_to(Arc::new(cubic), 2.0).unwrap();
        let before = animator.clone();
        assert!(animator.advance(&skeleton, 1.0).is_err());
        assert_eq!(animator.time, before.time);
        assert_eq!(
            animator.transition.as_ref().unwrap().elapsed,
            before.transition.as_ref().unwrap().elapsed
        );
        assert_eq!(
            animator.transition.as_ref().unwrap().source_time,
            before.transition.as_ref().unwrap().source_time
        );
        assert!(animator.advance(&skeleton, 0.25).is_ok());
    }

    #[test]
    fn cubic_scale_overshoot_is_sampled_and_invalid_zero_scale_is_rejected() {
        let skeleton = skeleton();
        let clip = AnimationClip::new_with_tangents(
            "scale",
            2.0,
            Playback::Clamp,
            vec![
                JointTrack {
                    scales: vec![
                        Vec3Key {
                            time: 0.0,
                            value: Vec3::ONE,
                        },
                        Vec3Key {
                            time: 1.0,
                            value: Vec3::ONE,
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            vec![
                TrackInterpolation {
                    scale: Interpolation::CubicSpline,
                    ..TrackInterpolation::default()
                },
                TrackInterpolation::default(),
            ],
            vec![
                JointTangents {
                    scale: vec![
                        [Vec3::ZERO, Vec3::splat(-4.0)],
                        [Vec3::splat(4.0), Vec3::ZERO],
                    ],
                    ..JointTangents::default()
                },
                JointTangents::default(),
            ],
            &skeleton,
        )
        .unwrap();
        assert!(
            clip.try_sample(&skeleton, 0.25).unwrap().local()[0]
                .scale
                .abs_diff_eq(Vec3::splat(0.25), 1e-6)
        );
        assert!(matches!(
            clip.try_sample(&skeleton, 0.5),
            Err(AnimationError::InvalidPose(0))
        ));
        assert_eq!(
            clip.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
            Vec3::ONE
        );
    }

    #[test]
    fn step_channels_hold_values_at_boundaries_without_changing_linear_channels() {
        let skeleton = skeleton();
        let rotation = Quat::from_rotation_z(1.2);
        let track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.2,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 0.5,
                    value: Vec3::X,
                },
                Vec3Key {
                    time: 1.0,
                    value: Vec3::Y,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 0.2,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 0.5,
                    value: rotation,
                },
            ],
            scales: vec![
                Vec3Key {
                    time: 0.0,
                    value: Vec3::ONE,
                },
                Vec3Key {
                    time: 1.0,
                    value: Vec3::splat(3.0),
                },
            ],
        };
        let tracks = vec![track, JointTrack::default()];
        let modes = vec![
            TrackInterpolation {
                translation: Interpolation::Step,
                rotation: Interpolation::Step,
                scale: Interpolation::Linear,
            },
            TrackInterpolation::default(),
        ];
        let clip = AnimationClip::new_with_interpolation(
            "mixed",
            2.0,
            Playback::Clamp,
            tracks.clone(),
            modes.clone(),
            &skeleton,
        )
        .unwrap();
        for (time, expected) in [
            (-1.0, Vec3::ZERO),
            (0.2, Vec3::ZERO),
            (0.5_f32.next_down(), Vec3::ZERO),
            (0.5, Vec3::X),
            (0.5_f32.next_up(), Vec3::X),
            (1.0_f32.next_down(), Vec3::X),
            (1.0, Vec3::Y),
            (3.0, Vec3::Y),
        ] {
            let pose = clip.sample(&skeleton, time);
            assert_eq!(pose.local()[0].translation, expected, "time={time}");
            assert_eq!(pose.local()[1], skeleton.joints()[1].bind_local);
        }
        assert_eq!(
            clip.sample(&skeleton, 0.5_f32.next_down()).local()[0].rotation,
            Quat::IDENTITY
        );
        assert_eq!(clip.sample(&skeleton, 0.5).local()[0].rotation, rotation);
        assert_eq!(
            clip.sample(&skeleton, 0.5).local()[0].scale,
            Vec3::splat(2.0)
        );
        let looped = AnimationClip::new_with_interpolation(
            "mixed",
            2.0,
            Playback::Loop,
            tracks.clone(),
            modes,
            &skeleton,
        )
        .unwrap();
        assert_eq!(
            looped.sample(&skeleton, 2.0).local()[0].translation,
            Vec3::ZERO
        );
        assert_eq!(
            looped.sample(&skeleton, -1.5).local()[0].translation,
            Vec3::X
        );
        assert!(matches!(
            AnimationClip::new_with_interpolation(
                "bad",
                2.0,
                Playback::Loop,
                tracks,
                vec![],
                &skeleton
            ),
            Err(AnimationError::InterpolationCountMismatch {
                expected: 2,
                actual: 0
            })
        ));
    }

    #[test]
    fn clip_interpolates_loops_and_uses_bind_fallbacks() {
        let skeleton = skeleton();
        let clip = AnimationClip::new(
            "wave",
            2.0,
            Playback::Loop,
            vec![
                JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.0,
                            value: Vec3::ZERO,
                        },
                        Vec3Key {
                            time: 2.0,
                            value: Vec3::new(2.0, 0.0, 0.0),
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            &skeleton,
        )
        .unwrap();
        let pose = clip.sample(&skeleton, 1.0);
        assert!(pose.local()[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
        assert!(pose.local()[1].translation.abs_diff_eq(Vec3::Y, 1.0e-6));
        let looped = clip.sample(&skeleton, 3.0);
        assert!(looped.local()[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
    }

    #[test]
    fn joint_override_rejects_invalid_rotation_without_mutation() {
        let mut pose = Pose {
            rig: Arc::from([skeleton().joints()[0].clone()]),
            local: vec![Transform::IDENTITY],
        };
        for invalid in [
            Quat::from_xyzw(0., 0., 0., 0.),
            Quat::from_xyzw(f32::NAN, 0., 0., 1.),
        ] {
            assert!(pose.set_joint_rotation(0, invalid).is_err());
            assert_eq!(pose.local[0], Transform::IDENTITY);
        }
        assert!(pose.set_joint_rotation(1, Quat::IDENTITY).is_err());
        let rotation = Quat::from_rotation_x(0.7);
        pose.set_joint_rotation(0, rotation).unwrap();
        assert_eq!(pose.local[0].rotation, rotation);
        assert_eq!(pose.local[0].translation, Vec3::ZERO);
        assert_eq!(pose.local[0].scale, Vec3::ONE);
    }
    #[test]
    fn pose_blend_uses_shortest_rotation_path() {
        let a = Pose {
            rig: Arc::from([skeleton().joints()[0].clone()]),
            local: vec![Transform::IDENTITY],
        };
        let b = Pose {
            rig: a.rig.clone(),
            local: vec![Transform {
                translation: Vec3::new(2.0, 0.0, 0.0),
                rotation: Quat::from_rotation_y(std::f32::consts::PI),
                scale: Vec3::splat(2.0),
            }],
        };
        let pose = Pose::blend(&a, &b, 0.5).unwrap();
        assert!(pose.local[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
        assert!(pose.local[0].scale.abs_diff_eq(Vec3::splat(1.5), 1.0e-6));
        assert!(pose.local[0].rotation.is_normalized());
    }

    #[test]
    fn malformed_hierarchy_and_tracks_are_rejected() {
        let invalid = Skeleton::new(vec![Joint {
            name: Arc::from("cycle"),
            parent: Some(0),
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }]);
        assert!(matches!(invalid, Err(AnimationError::InvalidParent { .. })));

        let skeleton = skeleton();
        let clip = AnimationClip::new(
            "bad",
            1.0,
            Playback::Clamp,
            vec![
                JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.8,
                            value: Vec3::ZERO,
                        },
                        Vec3Key {
                            time: 0.2,
                            value: Vec3::ONE,
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            &skeleton,
        );
        assert!(matches!(clip, Err(AnimationError::InvalidTrack { .. })));
    }

    fn root_clip(skeleton: &Skeleton, end: f32) -> Arc<AnimationClip> {
        Arc::new(
            AnimationClip::new(
                "walk",
                1.0,
                Playback::Loop,
                vec![
                    JointTrack {
                        translations: vec![
                            Vec3Key {
                                time: 0.0,
                                value: Vec3::ZERO,
                            },
                            Vec3Key {
                                time: 1.0,
                                value: Vec3::new(end, 0.0, 0.0),
                            },
                        ],
                        ..JointTrack::default()
                    },
                    JointTrack::default(),
                ],
                skeleton,
            )
            .unwrap(),
        )
    }

    #[test]
    fn extracted_root_rotation_returns_to_bind_and_rebuilds_the_hierarchical_palette() {
        let mut joints = skeleton().joints().to_vec();
        joints[1].bind_local.rotation = Quat::from_rotation_x(0.23);
        joints[1].bind_local.scale = Vec3::new(-1., 2., 1.);
        joints[1].inverse_bind = joints[1].bind_local.matrix().inverse();
        let rig = Skeleton::new(joints).unwrap();
        let mut pose = rig.bind_pose();
        pose.local[0].translation = Vec3::X * 7.;
        pose.local[0].rotation = Quat::from_rotation_y(0.4);
        pose.local[1] = Transform { translation: Vec3::new(3., 4., 5.),
            rotation: Quat::from_rotation_z(0.7), scale: Vec3::new(-2., 3., 4.) };
        let original = AnimatorFrame { skin_matrices: pose.skin_matrices(&rig).unwrap(), pose,
            root_motion: Vec3::new(0.1, 0.2, 0.3), root_motion_joint: 1, transition_weight: 0.5 };
        let output = original.clone().without_root_rotation(&rig).unwrap();
        assert_eq!(output.pose.local()[0], original.pose.local()[0]);
        assert_eq!(output.pose.local()[1].translation, original.pose.local()[1].translation);
        assert_eq!(output.pose.local()[1].scale, original.pose.local()[1].scale);
        assert_eq!(output.pose.local()[1].rotation, rig.joints()[1].bind_local.rotation);
        assert_eq!(output.root_motion, original.root_motion);
        assert_eq!(output.transition_weight, original.transition_weight);
        let expected_global = original.pose.local()[0].matrix()
            * Mat4::from_scale_rotation_translation(Vec3::new(-2., 3., 4.),
                Quat::from_rotation_x(0.23), Vec3::new(3., 4., 5.));
        assert!(output.skin_matrices[1].abs_diff_eq(expected_global * rig.joints()[1].inverse_bind, 1e-6));
        assert!((output.skin_matrices[1].transform_point3(Vec3::X)
            - original.skin_matrices[1].transform_point3(Vec3::X)).length() > 0.1);
        let (first, displacement) = output.clone().into_in_place_translation(&rig, [true, false, true]).unwrap();
        let (second, other_displacement) = original.clone().into_in_place_translation(&rig, [true, false, true]).unwrap();
        let second = second.without_root_rotation(&rig).unwrap();
        assert_eq!(first.pose, second.pose);
        assert_eq!(first.skin_matrices, second.skin_matrices);
        assert_eq!(displacement, other_displacement);
        assert_eq!(displacement, Vec3::new(0.1, 0., 0.3));
        let mut foreign = rig.joints().to_vec();
        foreign[1].name = Arc::from("different");
        assert_eq!(original.clone().without_root_rotation(&Skeleton::new(foreign).unwrap()).unwrap_err(), AnimationError::SkeletonMismatch);
        let mut invalid = original.clone();
        invalid.root_motion_joint = u16::MAX;
        assert_eq!(invalid.without_root_rotation(&rig).unwrap_err(), AnimationError::InvalidRootMotionJoint(u16::MAX));
        let mut invalid = original.clone();
        invalid.pose.local[1].rotation = Quat::from_array([f32::NAN; 4]);
        assert_eq!(invalid.without_root_rotation(&rig).unwrap_err(), AnimationError::InvalidPose(1));
        let mut invalid = original.clone();
        invalid.pose.local[0].translation = Vec3::splat(f32::INFINITY);
        assert_eq!(invalid.without_root_rotation(&rig).unwrap_err(), AnimationError::InvalidPose(0));
        assert!(original.pose.local[1].rotation.abs_diff_eq(Quat::from_rotation_z(0.7), 1e-6));
    }
    #[test]
    fn extraction_masks_selected_child_axes_and_preserves_other_channels() {
        let rig = skeleton();
        let mut pose = rig.bind_pose();
        pose.local[0].translation = Vec3::X * 7.0;
        pose.local[1] = Transform {
            translation: Vec3::new(3.0, 4.0, 5.0),
            rotation: Quat::from_rotation_z(0.4),
            scale: Vec3::new(-2.0, 3.0, 4.0),
        };
        let original = pose.clone();
        let frame = AnimatorFrame {
            skin_matrices: pose.skin_matrices(&rig).unwrap(),
            pose,
            root_motion: Vec3::new(0.1, 0.2, 0.3),
            root_motion_joint: 1,
            transition_weight: 0.5,
        };
        let (frame, motion) = frame
            .into_in_place_translation(&rig, [false, true, false])
            .unwrap();
        assert_eq!(motion, Vec3::Y * 0.2);
        assert_eq!(frame.root_motion, Vec3::new(0.1, 0.0, 0.3));
        assert_eq!(frame.pose.local()[0], original.local()[0]);
        assert_eq!(frame.pose.local()[1].translation, Vec3::new(3.0, 1.0, 5.0));
        assert_eq!(frame.pose.local()[1].rotation, original.local()[1].rotation);
        assert_eq!(frame.pose.local()[1].scale, original.local()[1].scale);
        assert_eq!(frame.transition_weight, 0.5);
        assert_eq!(frame.skin_matrices, frame.pose.skin_matrices(&rig).unwrap());
        let mut invalid = frame;
        invalid.root_motion = Vec3::splat(f32::INFINITY);
        assert!(invalid.into_in_place_translation(&rig, [true; 3]).is_err());
    }

    #[test]
    fn extracted_translation_is_removed_from_pose_palette_and_consumed_once() {
        let rig = skeleton();
        let mut animator = Animator::new(root_clip(&rig, 2.0));
        let sampled = animator.advance(&rig, 0.75).unwrap();
        let original = sampled.clone();
        let (in_place, motion) = sampled
            .into_in_place_translation(&rig, [true, false, true])
            .unwrap();
        assert_eq!(motion, Vec3::X * 1.5);
        assert_eq!(in_place.root_motion, Vec3::ZERO);
        assert_eq!(
            in_place.pose.local()[0].translation.x,
            rig.joints()[0].bind_local.translation.x
        );
        assert_eq!(in_place.pose.local()[1], original.pose.local()[1]);
        assert_eq!(
            in_place.pose.local()[0].rotation,
            original.pose.local()[0].rotation
        );
        assert_eq!(
            in_place.skin_matrices,
            in_place.pose.skin_matrices(&rig).unwrap()
        );
        assert_ne!(in_place.skin_matrices, original.skin_matrices);
        let (again, twice) = in_place
            .clone()
            .into_in_place_translation(&rig, [true, false, true])
            .unwrap();
        assert_eq!(twice, Vec3::ZERO);
        assert_eq!(again.pose, in_place.pose);
        let (wrapped, motion) = animator
            .advance(&rig, 0.5)
            .unwrap()
            .into_in_place_translation(&rig, [true, false, true])
            .unwrap();
        assert_eq!(motion, Vec3::X);
        assert_eq!(
            wrapped.pose.local()[0].translation.x,
            rig.joints()[0].bind_local.translation.x
        );
        let (unchanged, zero) = original
            .clone()
            .into_in_place_translation(&rig, [false; 3])
            .unwrap();
        assert_eq!(zero, Vec3::ZERO);
        assert_eq!(unchanged.pose, original.pose);
        assert_eq!(unchanged.root_motion, original.root_motion);
        let mut joints = rig.joints().to_vec();
        joints[0].name = "foreign".into();
        let foreign = Skeleton::new(joints).unwrap();
        assert!(matches!(
            original.into_in_place_translation(&foreign, [true; 3]),
            Err(AnimationError::SkeletonMismatch)
        ));
    }

    #[test]
    fn bounded_clocks_preserve_small_ticks_after_long_elapsed_time_and_many_loops() {
        let rig = skeleton();
        let mut animator = Animator::new(root_clip(&rig, 2.));
        // At this elapsed time f32 addition loses a 60 Hz tick completely.
        let long = 1_048_576_f32;
        assert_eq!(long + 1. / 60., long);
        animator.time = f64::from(long) + 0.875;
        let dt = 1. / 60.;
        let frame = animator.advance(&rig, dt).unwrap();
        assert!(frame.root_motion.abs_diff_eq(Vec3::X * (2. * dt), 1e-7));
        assert!((animator.time - (0.875 + f64::from(dt))).abs() < 1e-12);
        animator.set_speed(8.).unwrap();
        let frame = animator.advance(&rig, 1.).unwrap();
        assert_eq!(frame.root_motion, Vec3::X * 16.);
        assert!((0. ..1.).contains(&animator.time));
        animator.set_speed(1.).unwrap();
        let initial = animator.time;
        let mut displacement = 0_f64;
        for _ in 0..100_000 {
            let frame = animator.advance(&rig, dt).unwrap();
            displacement += f64::from(frame.root_motion.x);
            assert!((0. ..1.).contains(&animator.time));
        }
        assert!((displacement - 200_000. * f64::from(dt)).abs() < 1e-6);
        let expected = (initial + 100_000. * f64::from(dt)).rem_euclid(1.);
        assert!((animator.time - expected).abs() < 1e-12);
    }

    #[test]
    fn bounded_crossfade_clocks_match_local_phase_and_preserve_failed_tick() {
        let rig = skeleton();
        let mut control = Animator::new(root_clip(&rig, 2.));
        control.advance(&rig, 0.875).unwrap();
        control.transition_to(root_clip(&rig, 4.), 0.5).unwrap();
        control.advance(&rig, 0.125).unwrap();
        let mut long = control.clone();
        long.time += 1_048_576.;
        long.transition.as_mut().unwrap().source_time += 1_048_576.;
        let before = long.clone();
        assert!(long.advance(&rig, f32::NAN).is_err());
        assert_eq!(long.time, before.time);
        assert_eq!(
            long.transition.as_ref().unwrap().source_time,
            before.transition.as_ref().unwrap().source_time
        );
        for dt in [1. / 60., 0.25, 0.5] {
            let actual = long.advance(&rig, dt).unwrap();
            let expected = control.advance(&rig, dt).unwrap();
            assert_eq!(actual.pose, expected.pose);
            assert!(actual.root_motion.abs_diff_eq(expected.root_motion, 1e-7));
            assert_eq!(actual.transition_weight, expected.transition_weight);
        }
        assert!(long.transition.is_none());
        // Clamp remains at its endpoint rather than wrapping and replaying motion.
        let mut clip = (*root_clip(&rig, 2.)).clone();
        clip.playback = Playback::Clamp;
        clip.root_curve = Arc::new(root_curve::RootCurve::new(
            &clip.tracks[0].translations,
            clip.interpolation[0].translation,
            &clip.tangents[0].translation,
            clip.duration,
            Playback::Clamp,
        ));
        let mut clamped = Animator::new(Arc::new(clip));
        clamped.time = 1_048_576.;
        assert_eq!(
            clamped.advance(&rig, 1. / 60.).unwrap().root_motion,
            Vec3::ZERO
        );
        assert_eq!(clamped.time, 1.);
    }

    #[test]
    fn animator_preserves_root_motion_across_loop_boundary() {
        let skeleton = skeleton();
        let mut animator = Animator::new(root_clip(&skeleton, 2.0));
        let first = animator.advance(&skeleton, 0.75).unwrap();
        let wrapped = animator.advance(&skeleton, 0.5).unwrap();
        assert!((first.root_motion.x - 1.5).abs() < 1.0e-6);
        assert!((wrapped.root_motion.x - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn crossfade_is_bounded_and_reaches_target() {
        let skeleton = skeleton();
        let mut animator = Animator::new(root_clip(&skeleton, 0.0));
        animator
            .transition_to(root_clip(&skeleton, 2.0), 0.5)
            .unwrap();
        let half = animator.advance(&skeleton, 0.25).unwrap();
        assert!((half.transition_weight - 0.5).abs() < f32::EPSILON);
        let complete = animator.advance(&skeleton, 0.25).unwrap();
        assert!((complete.transition_weight - 1.0).abs() < f32::EPSILON);
        assert!(animator.transition.is_none());
    }

    #[test]
    fn incompatible_clip_rejects_without_advancing() {
        let skeleton = skeleton();
        let mut animator = Animator::new(root_clip(&skeleton, 2.0));
        let other = Skeleton::new(vec![skeleton.joints()[0].clone()]).unwrap();
        assert!(matches!(
            animator.advance(&other, 0.5),
            Err(AnimationError::TrackCountMismatch { .. })
        ));
        assert_eq!(animator.time.to_bits(), 0.0f64.to_bits());
        assert!((animator.advance(&skeleton, 0.5).unwrap().root_motion.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn palette_overflow_preserves_clock_and_unfinished_transition() {
        let good = skeleton();
        let mut joints = good.joints().to_vec();
        joints[0].inverse_bind = Mat4::from_scale(Vec3::splat(f32::MAX));
        let bad = Skeleton::new(joints).unwrap();
        let mut tracks = vec![JointTrack::default(); bad.joints().len()];
        tracks[0].scales = vec![
            Vec3Key {
                time: 0.,
                value: Vec3::splat(0.5),
            },
            Vec3Key {
                time: 1.,
                value: Vec3::splat(3.),
            },
        ];
        let target = Arc::new(
            AnimationClip::new("overflow scale", 1., Playback::Clamp, tracks, &bad).unwrap(),
        );
        let mut animator = Animator::new(root_clip(&bad, 0.0));
        animator.advance(&bad, 0.25).unwrap();
        animator.transition_to(target, 0.5).unwrap();
        animator.advance(&bad, 0.1).unwrap();
        let before = animator.clone();
        assert!(matches!(
            animator.advance(&bad, 0.4),
            Err(AnimationError::InvalidPose(_))
        ));
        assert_eq!(animator.time.to_bits(), before.time.to_bits());
        let transition = animator.transition.as_ref().unwrap();
        let prior = before.transition.as_ref().unwrap();
        assert_eq!(
            transition.source_time.to_bits(),
            prior.source_time.to_bits()
        );
        assert_eq!(transition.elapsed.to_bits(), prior.elapsed.to_bits());
        let mut control = before;
        let actual = animator.advance(&bad, 0.05).unwrap();
        let expected = control.advance(&bad, 0.05).unwrap();
        assert_eq!(actual.pose, expected.pose);
        assert_eq!(actual.root_motion, expected.root_motion);
        assert!(animator.transition.is_some());
    }

    #[test]
    fn root_motion_overflow_rejects_without_advancing() {
        let skeleton = skeleton();
        let mut animator = Animator::new(root_clip(&skeleton, f32::MAX));
        animator.set_speed(8.0).unwrap();
        assert!(matches!(
            animator.advance(&skeleton, 1.0),
            Err(AnimationError::NumericalOverflow)
        ));
        assert_eq!(animator.time.to_bits(), 0.0f64.to_bits());
        animator.set_speed(0.0).unwrap();
        assert!(
            animator
                .advance(&skeleton, 1.0)
                .unwrap()
                .root_motion
                .is_finite()
        );
    }

    #[test]
    fn finite_local_scales_cannot_publish_overflowing_global_matrices() {
        let mut joints = skeleton().joints().to_vec();
        for joint in &mut joints {
            joint.bind_local.scale = Vec3::splat(1e20);
        }
        let skeleton = Skeleton::new(joints).unwrap();
        assert!(matches!(
            skeleton.bind_pose().skin_matrices(&skeleton),
            Err(AnimationError::InvalidPose(1))
        ));
    }
}
