//! Pose-graph building blocks: per-joint masks, layered and additive blending,
//! and phase-synchronized 1D blend spaces (walk/jog/run by speed).
//!
//! Nodes are pure functions of validated poses; no node owns time except
//! [`BlendSpace1DPlayer`], which keeps one normalized phase shared by every
//! sample so foot contacts stay aligned across clips of different length.

use std::sync::Arc;

use glam::{Quat, Vec3};

use crate::{AnimationClip, AnimationError, Pose, Skeleton, Transform, lerp_vec3, rigs_match};

/// Per-joint layer weight in `[0, 1]`, indexed like the skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct BoneMask {
    weights: Vec<f32>,
}

impl BoneMask {
    /// Uniform weight for every joint.
    ///
    /// # Errors
    /// Rejects a nonfinite weight.
    pub fn uniform(skeleton: &Skeleton, weight: f32) -> Result<Self, AnimationError> {
        let weight = checked_weight(weight)?;
        Ok(Self {
            weights: vec![weight; skeleton.joints().len()],
        })
    }

    /// `weight` on `root` and all its descendants, zero elsewhere
    /// (upper-body layers: aim, reload, wave over locomotion).
    ///
    /// # Errors
    /// Rejects an unknown root or nonfinite weight.
    pub fn subtree(skeleton: &Skeleton, root: u16, weight: f32) -> Result<Self, AnimationError> {
        let weight = checked_weight(weight)?;
        let joints = skeleton.joints();
        if usize::from(root) >= joints.len() {
            return Err(AnimationError::InvalidPose(usize::from(root)));
        }
        // Parents precede children, so one forward pass marks the subtree.
        let mut inside = vec![false; joints.len()];
        inside[usize::from(root)] = true;
        for (index, joint) in joints.iter().enumerate() {
            if let Some(parent) = joint.parent
                && inside[usize::from(parent)]
            {
                inside[index] = true;
            }
        }
        Ok(Self {
            weights: inside
                .into_iter()
                .map(|flag| if flag { weight } else { 0.0 })
                .collect(),
        })
    }

    #[must_use]
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    /// Scales every joint weight (layer fade in/out).
    ///
    /// # Errors
    /// Rejects a nonfinite factor.
    pub fn scaled(&self, factor: f32) -> Result<Self, AnimationError> {
        let factor = checked_weight(factor)?;
        Ok(Self {
            weights: self.weights.iter().map(|w| w * factor).collect(),
        })
    }
}

fn checked_weight(weight: f32) -> Result<f32, AnimationError> {
    if weight.is_finite() {
        Ok(weight.clamp(0.0, 1.0))
    } else {
        Err(AnimationError::InvalidBlendWeight)
    }
}

fn same_layout(a: &Pose, b: &Pose) -> Result<(), AnimationError> {
    if a.local.len() != b.local.len() {
        return Err(AnimationError::PoseCountMismatch);
    }
    if !rigs_match(&a.rig, &b.rig) {
        return Err(AnimationError::SkeletonMismatch);
    }
    Ok(())
}

fn lerp_transform(a: Transform, b: Transform, weight: f32) -> Transform {
    Transform {
        translation: lerp_vec3(a.translation, b.translation, weight),
        rotation: a.rotation.slerp(b.rotation, weight).normalize(),
        scale: lerp_vec3(a.scale, b.scale, weight),
    }
}

fn validated(index: usize, value: Transform) -> Result<Transform, AnimationError> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(AnimationError::InvalidPose(index))
    }
}

impl Pose {
    /// Overrides `base` with `overlay` per joint by mask weight.
    ///
    /// # Errors
    /// Rejects mismatched rigs/lengths or invalid transforms.
    pub fn blend_masked(
        base: &Self,
        overlay: &Self,
        mask: &BoneMask,
    ) -> Result<Self, AnimationError> {
        same_layout(base, overlay)?;
        if mask.weights.len() != base.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        let local = base
            .local
            .iter()
            .zip(&overlay.local)
            .zip(&mask.weights)
            .enumerate()
            .map(|(index, ((a, b), &w))| {
                if !a.is_valid() || !b.is_valid() {
                    return Err(AnimationError::InvalidPose(index));
                }
                validated(index, lerp_transform(*a, *b, w))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            rig: base.rig.clone(),
            local,
        })
    }

    /// Applies `additive - reference` on top of `base` (breathing, hit
    /// reactions, lean). Translation adds, rotation pre-multiplies the delta,
    /// scale multiplies the ratio; each is faded by the mask weight.
    ///
    /// # Errors
    /// Rejects mismatched rigs/lengths or invalid transforms.
    pub fn apply_additive(
        base: &Self,
        additive: &Self,
        reference: &Self,
        mask: &BoneMask,
    ) -> Result<Self, AnimationError> {
        same_layout(base, additive)?;
        same_layout(base, reference)?;
        if mask.weights.len() != base.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        let mut local = Vec::with_capacity(base.local.len());
        for index in 0..base.local.len() {
            let (b, a, r, w) = (
                base.local[index],
                additive.local[index],
                reference.local[index],
                mask.weights[index],
            );
            if !b.is_valid() || !a.is_valid() || !r.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let delta_rotation = Quat::IDENTITY
                .slerp((a.rotation * r.rotation.inverse()).normalize(), w)
                .normalize();
            let ratio = a.scale / r.scale;
            let value = Transform {
                translation: b.translation + (a.translation - r.translation) * w,
                rotation: (delta_rotation * b.rotation).normalize(),
                scale: b.scale * lerp_vec3(Vec3::ONE, ratio, w),
            };
            local.push(validated(index, value)?);
        }
        Ok(Self {
            rig: base.rig.clone(),
            local,
        })
    }
}

/// One clip placed at a parameter value (for locomotion: speed in m/s).
#[derive(Clone, Debug)]
pub struct BlendSample {
    pub parameter: f32,
    pub clip: Arc<AnimationClip>,
}

/// Sorted 1D blend space. Samples share normalized phase; the cycle length
/// interpolates between neighbours so stride frequency changes smoothly.
#[derive(Clone, Debug)]
pub struct BlendSpace1D {
    samples: Vec<BlendSample>,
}

/// Neighbour pair and weight for a parameter value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendSegment {
    pub lower: usize,
    pub upper: usize,
    pub weight: f32,
}

impl BlendSpace1D {
    /// # Errors
    /// Rejects an empty set, nonfinite or duplicate parameters, and clips
    /// incompatible with `skeleton`.
    pub fn new(mut samples: Vec<BlendSample>, skeleton: &Skeleton) -> Result<Self, AnimationError> {
        if samples.is_empty() {
            return Err(AnimationError::InvalidClipHeader);
        }
        if samples
            .iter()
            .any(|s| !s.parameter.is_finite() || !s.clip.is_compatible_with(skeleton))
        {
            return Err(AnimationError::SkeletonMismatch);
        }
        samples.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
        if samples.windows(2).any(|w| w[0].parameter == w[1].parameter) {
            return Err(AnimationError::InvalidClipHeader);
        }
        Ok(Self { samples })
    }

    #[must_use]
    pub fn samples(&self) -> &[BlendSample] {
        &self.samples
    }

    /// Clamped neighbour lookup.
    ///
    /// # Errors
    /// Rejects a nonfinite parameter.
    pub fn segment(&self, parameter: f32) -> Result<BlendSegment, AnimationError> {
        if !parameter.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let last = self.samples.len() - 1;
        if parameter <= self.samples[0].parameter {
            return Ok(BlendSegment {
                lower: 0,
                upper: 0,
                weight: 0.0,
            });
        }
        if parameter >= self.samples[last].parameter {
            return Ok(BlendSegment {
                lower: last,
                upper: last,
                weight: 0.0,
            });
        }
        let upper = self.samples.partition_point(|s| s.parameter <= parameter);
        let lower = upper - 1;
        let (p0, p1) = (self.samples[lower].parameter, self.samples[upper].parameter);
        Ok(BlendSegment {
            lower,
            upper,
            weight: (parameter - p0) / (p1 - p0),
        })
    }

    /// Interpolated cycle length in seconds at `parameter`.
    ///
    /// # Errors
    /// Rejects a nonfinite parameter.
    pub fn cycle_duration(&self, parameter: f32) -> Result<f32, AnimationError> {
        let s = self.segment(parameter)?;
        let d0 = self.samples[s.lower].clip.duration;
        let d1 = self.samples[s.upper].clip.duration;
        Ok(d0 + (d1 - d0) * s.weight)
    }

    /// Samples both neighbours at the shared normalized `phase` and blends.
    ///
    /// # Errors
    /// Rejects nonfinite input, phase outside `[0, 1]` and invalid poses.
    pub fn evaluate(
        &self,
        skeleton: &Skeleton,
        parameter: f32,
        phase: f64,
    ) -> Result<Pose, AnimationError> {
        let s = self.segment(parameter)?;
        let a = self.samples[s.lower]
            .clip
            .try_sample_phase(skeleton, phase)?;
        if s.lower == s.upper || s.weight == 0.0 {
            return Ok(a);
        }
        let b = self.samples[s.upper]
            .clip
            .try_sample_phase(skeleton, phase)?;
        Pose::blend(&a, &b, s.weight)
    }
}

/// Owns the shared phase of a [`BlendSpace1D`]; the parameter may change
/// every tick without popping because phase, not clip time, is preserved.
#[derive(Clone, Debug)]
pub struct BlendSpace1DPlayer {
    space: Arc<BlendSpace1D>,
    phase: f64,
    parameter: f32,
}

impl BlendSpace1DPlayer {
    #[must_use]
    pub fn new(space: Arc<BlendSpace1D>, parameter: f32) -> Self {
        Self {
            space,
            phase: 0.0,
            parameter,
        }
    }

    #[must_use]
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// Sets the target parameter and advances the looping phase by `dt`.
    ///
    /// # Errors
    /// Rejects nonfinite/negative `dt` or a nonfinite parameter.
    pub fn advance(&mut self, parameter: f32, dt: f32) -> Result<(), AnimationError> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let duration = self.space.cycle_duration(parameter)?;
        self.parameter = parameter;
        self.phase = (self.phase + f64::from(dt) / f64::from(duration)).rem_euclid(1.0);
        Ok(())
    }

    /// # Errors
    /// Propagates sampling errors.
    pub fn pose(&self, skeleton: &Skeleton) -> Result<Pose, AnimationError> {
        self.space.evaluate(skeleton, self.parameter, self.phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, JointTrack, Playback, Vec3Key};
    use glam::Mat4;

    fn rig() -> Skeleton {
        let joint = |name: &str, parent| Joint {
            name: Arc::from(name),
            parent,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        };
        Skeleton::new(vec![
            joint("pelvis", None),
            joint("leg", Some(0)),
            joint("spine", Some(0)),
            joint("arm", Some(2)),
        ])
        .unwrap()
    }

    fn clip(rig: &Skeleton, duration: f32, x: f32) -> Arc<AnimationClip> {
        let mut tracks = vec![JointTrack::default(); 4];
        for track in &mut tracks {
            track.translations = vec![
                Vec3Key {
                    time: 0.0,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: duration,
                    value: Vec3::new(x, 0.0, 0.0),
                },
            ];
        }
        Arc::new(AnimationClip::new("c", duration, Playback::Loop, tracks, rig).unwrap())
    }

    #[test]
    fn subtree_mask_covers_descendants_only() {
        let rig = rig();
        let mask = BoneMask::subtree(&rig, 2, 0.8).unwrap();
        assert_eq!(mask.weights(), &[0.0, 0.0, 0.8, 0.8]);
        assert!(BoneMask::subtree(&rig, 9, 1.0).is_err());
        assert!(BoneMask::uniform(&rig, f32::NAN).is_err());
    }

    #[test]
    fn masked_blend_overrides_upper_body_only() {
        let rig = rig();
        let base = clip(&rig, 1.0, 1.0).try_sample_phase(&rig, 1.0).unwrap();
        let over = clip(&rig, 1.0, 5.0).try_sample_phase(&rig, 1.0).unwrap();
        let mask = BoneMask::subtree(&rig, 2, 1.0).unwrap();
        let out = Pose::blend_masked(&base, &over, &mask).unwrap();
        let xs: Vec<f32> = out.local().iter().map(|t| t.translation.x).collect();
        assert_eq!(xs, vec![1.0, 1.0, 5.0, 5.0]);
    }

    #[test]
    fn additive_adds_delta_from_reference() {
        let rig = rig();
        let base = clip(&rig, 1.0, 2.0).try_sample_phase(&rig, 1.0).unwrap();
        let reference = rig.bind_pose();
        let mut additive = rig.bind_pose();
        additive
            .set_joint_rotation(1, Quat::from_rotation_z(0.5))
            .unwrap();
        additive.set_joint_translation(3, Vec3::Y).unwrap();
        let half = BoneMask::uniform(&rig, 0.5).unwrap();
        let out = Pose::apply_additive(&base, &additive, &reference, &half).unwrap();
        assert!((out.local()[3].translation - Vec3::new(2.0, 0.5, 0.0)).length() < 1e-6);
        let angle = out.local()[1].rotation.angle_between(Quat::IDENTITY);
        assert!((angle - 0.25).abs() < 1e-5);
        let none = BoneMask::uniform(&rig, 0.0).unwrap();
        assert_eq!(
            Pose::apply_additive(&base, &additive, &reference, &none)
                .unwrap()
                .local(),
            base.local()
        );
    }

    #[test]
    fn blend_space_interpolates_pose_and_cycle_length() {
        let rig = rig();
        let space = BlendSpace1D::new(
            vec![
                BlendSample {
                    parameter: 4.0,
                    clip: clip(&rig, 0.6, 4.0),
                },
                BlendSample {
                    parameter: 1.5,
                    clip: clip(&rig, 1.2, 2.0),
                },
            ],
            &rig,
        )
        .unwrap();
        assert_eq!(space.samples()[0].parameter, 1.5);
        assert!((space.cycle_duration(2.75).unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(space.cycle_duration(-1.0).unwrap(), 1.2);
        assert_eq!(space.cycle_duration(9.0).unwrap(), 0.6);
        let pose = space.evaluate(&rig, 2.75, 1.0).unwrap();
        assert!((pose.local()[0].translation.x - 3.0).abs() < 1e-5);
        assert!(space.evaluate(&rig, f32::NAN, 0.5).is_err());
        let dup = vec![
            BlendSample {
                parameter: 1.0,
                clip: clip(&rig, 1.0, 1.0),
            },
            BlendSample {
                parameter: 1.0,
                clip: clip(&rig, 1.0, 1.0),
            },
        ];
        assert!(BlendSpace1D::new(dup, &rig).is_err());
    }

    #[test]
    fn player_keeps_phase_continuous_across_speed_change() {
        let rig = rig();
        let space = Arc::new(
            BlendSpace1D::new(
                vec![
                    BlendSample {
                        parameter: 1.5,
                        clip: clip(&rig, 1.0, 1.0),
                    },
                    BlendSample {
                        parameter: 4.0,
                        clip: clip(&rig, 0.5, 1.0),
                    },
                ],
                &rig,
            )
            .unwrap(),
        );
        let mut player = BlendSpace1DPlayer::new(space, 1.5);
        player.advance(1.5, 0.25).unwrap();
        assert!((player.phase() - 0.25).abs() < 1e-9);
        player.advance(4.0, 0.25).unwrap();
        assert!((player.phase() - 0.75).abs() < 1e-9);
        player.advance(4.0, 0.25).unwrap();
        assert!((player.phase() - 0.25).abs() < 1e-9);
        assert!(player.advance(4.0, -1.0).is_err());
        player.pose(&rig).unwrap();
    }
}
