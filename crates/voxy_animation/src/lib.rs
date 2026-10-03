//! Validated skeletal animation sampling and skin-matrix generation.

use std::fmt;
use std::sync::Arc;

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

#[derive(Clone, Debug)]
pub struct Joint {
    pub name: Arc<str>,
    pub parent: Option<u16>,
    pub bind_local: Transform,
    pub inverse_bind: Mat4,
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
    name: Arc<str>,
    duration: f32,
    playback: Playback,
    tracks: Arc<[JointTrack]>,
    interpolation: Arc<[TrackInterpolation]>,
    tangents: Arc<[JointTangents]>,
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
        Ok(Self {
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

    /// Samples without runtime admission. Cubic curves can produce invalid TRS;
    /// use `try_sample` to reject those poses before publication.
    #[must_use]
    pub fn sample(&self, skeleton: &Skeleton, time: f32) -> Pose {
        let time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local(skeleton, time)
    }

    /// Samples a pose suitable for admission into a runtime or GPU palette.
    /// # Errors
    /// Rejects non-finite time, mismatched joint counts and invalid interpolated TRS.
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
        let pose = self.sample(skeleton, time);
        for (joint, transform) in pose.local.iter().enumerate() {
            if !transform.is_valid() {
                return Err(AnimationError::InvalidPose(joint));
            }
        }
        Ok(pose)
    }

    fn sample_local(&self, skeleton: &Skeleton, time: f32) -> Pose {
        let local = skeleton
            .joints
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
        Pose { local }
    }

    fn root_at_unwrapped(&self, skeleton: &Skeleton, time: f32) -> Vec3 {
        match self.playback {
            Playback::Clamp => {
                self.sample_local(skeleton, time.clamp(0.0, self.duration))
                    .local[0]
                    .translation
            }
            Playback::Loop => {
                let cycles = (time / self.duration).floor();
                let local_time = time.rem_euclid(self.duration);
                let start = self.sample_local(skeleton, 0.0).local[0].translation;
                let end = self.sample_local(skeleton, self.duration).local[0].translation;
                self.sample_local(skeleton, local_time).local[0].translation
                    + (end - start) * cycles
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
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
        Ok(Self { local })
    }
}

#[derive(Clone, Debug)]
struct Transition {
    source: Arc<AnimationClip>,
    source_time: f32,
    elapsed: f32,
    duration: f32,
}

#[derive(Clone, Debug)]
pub struct Animator {
    current: Arc<AnimationClip>,
    time: f32,
    speed: f32,
    transition: Option<Transition>,
}

#[derive(Clone, Debug)]
pub struct AnimatorFrame {
    pub pose: Pose,
    pub skin_matrices: Vec<Mat4>,
    pub root_motion: Vec3,
    pub transition_weight: f32,
}

impl Animator {
    #[must_use]
    pub fn new(initial: Arc<AnimationClip>) -> Self {
        Self {
            current: initial,
            time: 0.0,
            speed: 1.0,
            transition: None,
        }
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

    /// Starts a crossfade to a new clip. A zero duration switches immediately.
    ///
    /// # Errors
    ///
    /// Rejects non-finite, negative, or excessively long transitions.
    pub fn transition_to(
        &mut self,
        next: Arc<AnimationClip>,
        duration: f32,
    ) -> Result<(), AnimationError> {
        if !duration.is_finite() || !(0.0..=60.0).contains(&duration) {
            return Err(AnimationError::InvalidTransitionDuration);
        }
        if duration == 0.0 {
            self.current = next;
            self.time = 0.0;
            self.transition = None;
            return Ok(());
        }
        self.transition = Some(Transition {
            source: Arc::clone(&self.current),
            source_time: self.time,
            elapsed: 0.0,
            duration,
        });
        self.current = next;
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
        }
        let delta = dt * self.speed;
        let old_time = self.time;
        self.time += delta;
        let root_motion = self.current.root_at_unwrapped(skeleton, self.time)
            - self.current.root_at_unwrapped(skeleton, old_time);
        if !self.time.is_finite() || !root_motion.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let target = self.current.try_sample(skeleton, self.time)?;
        let (pose, weight, transition_complete) = if let Some(transition) = &mut self.transition {
            transition.source_time += delta;
            transition.elapsed = (transition.elapsed + dt).min(transition.duration);
            let weight = transition.elapsed / transition.duration;
            let source = transition
                .source
                .try_sample(skeleton, transition.source_time)?;
            (
                Pose::blend(&source, &target, weight)?,
                weight,
                transition.elapsed >= transition.duration,
            )
        } else {
            (target, 1.0, false)
        };
        if transition_complete {
            self.transition = None;
        }
        let skin_matrices = pose.skin_matrices(skeleton)?;
        Ok(AnimatorFrame {
            pose,
            skin_matrices,
            root_motion,
            transition_weight: weight,
        })
    }
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
            Interpolation::Linear => from.value.lerp(to.value, alpha),
            Interpolation::CubicSpline => {
                if alpha == 0.0 {
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
            local: vec![Transform::IDENTITY],
        };
        let b = Pose {
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
        assert_eq!(animator.time.to_bits(), 0.0f32.to_bits());
        assert!((animator.advance(&skeleton, 0.5).unwrap().root_motion.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn palette_overflow_preserves_clock_and_unfinished_transition() {
        let good = skeleton();
        let mut joints = good.joints().to_vec();
        joints[0].bind_local.scale = Vec3::splat(2.0);
        joints[0].inverse_bind = Mat4::from_scale(Vec3::splat(f32::MAX));
        let bad = Skeleton::new(joints).unwrap();
        let mut animator = Animator::new(root_clip(&good, 0.0));
        animator.transition_to(root_clip(&good, 2.0), 0.5).unwrap();
        animator.advance(&good, 0.25).unwrap();
        let before = animator.clone();
        assert!(matches!(
            animator.advance(&bad, 0.25),
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
        let actual = animator.advance(&good, 0.25).unwrap();
        let expected = control.advance(&good, 0.25).unwrap();
        assert_eq!(actual.pose, expected.pose);
        assert_eq!(actual.root_motion, expected.root_motion);
        assert!(animator.transition.is_none());
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
        assert_eq!(animator.time.to_bits(), 0.0f32.to_bits());
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
