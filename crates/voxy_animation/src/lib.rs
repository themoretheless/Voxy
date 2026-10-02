//! Validated skeletal animation sampling and skin-matrix generation.

use std::fmt;
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

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
            && self.scale.min_element() > 0.0
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

    #[must_use]
    pub fn sample(&self, skeleton: &Skeleton, time: f32) -> Pose {
        let time = match self.playback {
            Playback::Loop => time.rem_euclid(self.duration),
            Playback::Clamp => time.clamp(0.0, self.duration),
        };
        self.sample_local(skeleton, time)
    }

    fn sample_local(&self, skeleton: &Skeleton, time: f32) -> Pose {
        let local = skeleton
            .joints
            .iter()
            .zip(self.tracks.iter())
            .map(|(joint, track)| Transform {
                translation: sample_vec3(&track.translations, time, joint.bind_local.translation),
                rotation: sample_quat(&track.rotations, time, joint.bind_local.rotation),
                scale: sample_vec3(&track.scales, time, joint.bind_local.scale),
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
    /// Rejects different pose lengths or a non-finite weight.
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Result<Self, AnimationError> {
        if a.local.len() != b.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = weight.clamp(0.0, 1.0);
        Ok(Self {
            local: a
                .local
                .iter()
                .zip(&b.local)
                .map(|(a, b)| Transform {
                    translation: a.translation.lerp(b.translation, weight),
                    rotation: a.rotation.slerp(b.rotation, weight).normalize(),
                    scale: a.scale.lerp(b.scale, weight),
                })
                .collect(),
        })
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
        let target = self.current.sample(skeleton, self.time);
        let (pose, weight, transition_complete) = if let Some(transition) = &mut self.transition {
            transition.source_time += delta;
            transition.elapsed = (transition.elapsed + dt).min(transition.duration);
            let weight = transition.elapsed / transition.duration;
            let source = transition.source.sample(skeleton, transition.source_time);
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
        if !key.value.is_finite() || (scale && key.value.min_element() <= 0.0) {
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

fn sample_vec3(keys: &[Vec3Key], time: f32, fallback: Vec3) -> Vec3 {
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
        |(from, to, alpha)| from.value.lerp(to.value, alpha),
    )
}

fn sample_quat(keys: &[QuatKey], time: f32, fallback: Quat) -> Quat {
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
        |(from, to, alpha)| from.value.slerp(to.value, alpha).normalize(),
    )
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

fn sample_segment<T: Timed>(keys: &[T], time: f32) -> Option<(&T, &T, f32)> {
    let upper = keys.partition_point(|key| key.time() <= time);
    if upper == 0 || upper == keys.len() {
        return None;
    }
    let from = &keys[upper - 1];
    let to = &keys[upper];
    let alpha = (time - from.time()) / (to.time() - from.time());
    Some((from, to, alpha))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackError {
    InvalidTime,
    InvalidValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnimationError {
    InvalidJointCount(usize),
    InvalidJointName(usize),
    InvalidJointTransform(usize),
    InvalidParent { joint: usize, parent: u16 },
    InvalidClipHeader,
    TrackCountMismatch { expected: usize, actual: usize },
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
