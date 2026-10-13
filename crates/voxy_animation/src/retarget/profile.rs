//! Humanoid role profiles: an authored map from semantic bone roles to joint
//! names on each rig, from which a one-to-one [`RetargetBinding`] follows.
//! Roles are explicit per the retarget policy (no name heuristics decide a
//! mapping); the profile only removes the per-pair boilerplate and derives
//! the root translation basis and leg-length scale from the two bind poses.

use std::sync::Arc;

use glam::{DVec3, Quat, Vec3};

use super::{RetargetBinding, RetargetJoint};
use crate::{AnimationClip, AnimationError, JointTrack, QuatKey, Skeleton, Vec3Key};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// Semantic humanoid bone roles. `Custom` carries any extra one-to-one pair.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BoneRole {
    Hips,
    Spine,
    Chest,
    Neck,
    Head,
    Shoulder(Side),
    UpperArm(Side),
    Forearm(Side),
    Hand(Side),
    Thigh(Side),
    Shin(Side),
    Foot(Side),
    Toe(Side),
    /// Both rigs must use the same key for a custom role.
    Custom(Arc<str>),
}

/// Explicit role → joint-name map validated against one skeleton.
#[derive(Clone, Debug)]
pub struct HumanoidProfile {
    rig: Arc<[crate::Joint]>,
    roles: Vec<(BoneRole, u16)>,
}

impl HumanoidProfile {
    /// Validates every named joint and requires `Hips`, both thighs and both
    /// shins (the leg-ratio reference) to be present.
    /// # Errors
    /// Rejects unknown names, repeated roles or joints, and a missing leg chain.
    pub fn new(skeleton: &Skeleton, roles: &[(BoneRole, &str)]) -> Result<Self, AnimationError> {
        let mut resolved: Vec<(BoneRole, u16)> = Vec::with_capacity(roles.len());
        for (role, name) in roles {
            let index = skeleton
                .joints()
                .iter()
                .position(|joint| joint.name.as_ref() == *name)
                .ok_or(AnimationError::InvalidRetargetBinding)?;
            if resolved
                .iter()
                .any(|(r, i)| r == role || usize::from(*i) == index)
            {
                return Err(AnimationError::InvalidRetargetBinding);
            }
            resolved.push((role.clone(), index as u16));
        }
        let profile = Self {
            rig: skeleton.joints.clone(),
            roles: resolved,
        };
        for required in [
            BoneRole::Hips,
            BoneRole::Thigh(Side::Left),
            BoneRole::Shin(Side::Left),
            BoneRole::Foot(Side::Left),
            BoneRole::Thigh(Side::Right),
            BoneRole::Shin(Side::Right),
            BoneRole::Foot(Side::Right),
        ] {
            profile
                .joint(&required)
                .ok_or(AnimationError::InvalidRetargetBinding)?;
        }
        Ok(profile)
    }

    #[must_use]
    pub fn joint(&self, role: &BoneRole) -> Option<u16> {
        self.roles.iter().find(|(r, _)| r == role).map(|(_, i)| *i)
    }

    #[must_use]
    pub fn roles(&self) -> &[(BoneRole, u16)] {
        &self.roles
    }

    fn name(&self, role: &BoneRole) -> Option<Arc<str>> {
        self.joint(role)
            .map(|i| self.rig[usize::from(i)].name.clone())
    }

    /// Sum of the left thigh→shin→foot bind distances in model space.
    fn leg_length(&self) -> Result<f64, AnimationError> {
        let skeleton = Skeleton {
            joints: self.rig.clone(),
        };
        let bind = skeleton.bind_pose64().global_matrices(&skeleton)?;
        let at = |role: BoneRole| -> Result<DVec3, AnimationError> {
            let joint = self
                .joint(&role)
                .ok_or(AnimationError::InvalidRetargetBinding)?;
            Ok(bind[usize::from(joint)].w_axis.truncate())
        };
        let (a, b, c) = (
            at(BoneRole::Thigh(Side::Left))?,
            at(BoneRole::Shin(Side::Left))?,
            at(BoneRole::Foot(Side::Left))?,
        );
        let length = a.distance(b) + b.distance(c);
        if !length.is_finite() || length <= 0. {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok(length)
    }

    /// Composed bind rotation of the hips' ancestors, which converts hip-local
    /// translation deltas into the model frame. Scaled ancestors are rejected.
    fn hips_ancestor_basis(&self) -> Result<Quat, AnimationError> {
        let hips = self
            .joint(&BoneRole::Hips)
            .ok_or(AnimationError::InvalidRetargetBinding)?;
        let mut basis = Quat::IDENTITY;
        let mut parent = self.rig[usize::from(hips)].parent;
        while let Some(index) = parent {
            let joint = &self.rig[usize::from(index)];
            if joint.bind_local.scale != Vec3::ONE {
                return Err(AnimationError::InvalidRetargetBinding);
            }
            basis = (joint.bind_local.rotation * basis).normalize();
            parent = joint.parent;
        }
        Ok(basis)
    }
}

impl RetargetBinding {
    /// Pairs every role present in both profiles. Rotation bases are identity
    /// (both rigs must share bone axis conventions, as the explicit demo
    /// mappings do); the hips carry translation scaled by the target/source
    /// leg-length ratio through the source hips' ancestor basis, every other
    /// joint keeps its target bind translation.
    /// # Errors
    /// Rejects profiles for other rigs, no shared roles, or invalid bind geometry.
    pub fn from_profiles(
        source: &HumanoidProfile,
        target: &HumanoidProfile,
        source_skeleton: &Skeleton,
        target_skeleton: &Skeleton,
    ) -> Result<Self, AnimationError> {
        if !crate::rigs_match(&source.rig, &source_skeleton.joints)
            || !crate::rigs_match(&target.rig, &target_skeleton.joints)
        {
            return Err(AnimationError::SkeletonMismatch);
        }
        let leg_ratio = (target.leg_length()? / source.leg_length()?) as f32;
        if !leg_ratio.is_finite() || leg_ratio <= 0. {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let translation_basis = source.hips_ancestor_basis()?;
        let mut mapping = Vec::new();
        for (role, _) in &source.roles {
            let (Some(from), Some(to)) = (source.name(role), target.name(role)) else {
                continue;
            };
            mapping.push(RetargetJoint {
                source: from,
                target: to,
                rotation_basis: Quat::IDENTITY,
                translation_basis,
                translation_scale: if *role == BoneRole::Hips {
                    leg_ratio
                } else {
                    0.
                },
            });
        }
        if mapping.is_empty() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Self::new(source_skeleton, target_skeleton, &mapping)
    }

    /// Bakes a source clip onto the target rig at `keys_per_second`, sampling
    /// through [`RetargetBinding::apply_pose`]. Every target joint receives
    /// full translation/rotation/scale keys so the result is self-contained.
    /// # Errors
    /// Rejects a nonpositive rate, a foreign clip or sampling failures.
    pub fn retarget_clip(
        &self,
        clip: &AnimationClip,
        keys_per_second: f32,
    ) -> Result<AnimationClip, AnimationError> {
        if !keys_per_second.is_finite() || keys_per_second <= 0. {
            return Err(AnimationError::InvalidClipHeader);
        }
        let source = Skeleton {
            joints: self.source.clone(),
        };
        let target = Skeleton {
            joints: self.target.clone(),
        };
        if !clip.is_compatible_with(&source) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let duration = clip.duration;
        let samples = ((duration * keys_per_second).ceil() as usize).max(1) + 1;
        let mut tracks = vec![JointTrack::default(); self.target.len()];
        for s in 0..samples {
            let phase = s as f64 / (samples - 1) as f64;
            let time = (phase * f64::from(duration)) as f32;
            let pose = self.apply_pose(&clip.try_sample_phase(&source, phase)?)?;
            for (track, local) in tracks.iter_mut().zip(pose.local()) {
                track.translations.push(Vec3Key {
                    time,
                    value: local.translation,
                });
                track.rotations.push(QuatKey {
                    time,
                    value: local.rotation,
                });
                track.scales.push(Vec3Key {
                    time,
                    value: local.scale,
                });
            }
        }
        AnimationClip::new(clip.name.clone(), duration, clip.playback, tracks, &target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, Playback, Transform};
    use glam::Mat4;

    /// Minimal humanoid with an optional armature root above the hips.
    fn rig(scale: f32, armature: Option<Quat>) -> Skeleton {
        let mut joints = Vec::new();
        let mut offset = 0u16;
        if let Some(rotation) = armature {
            joints.push(Joint {
                name: Arc::from("Armature"),
                parent: None,
                bind_local: Transform {
                    rotation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            });
            offset = 1;
        }
        let mut push = |name: &str, parent: Option<u16>, t: Vec3| {
            joints.push(Joint {
                name: Arc::from(name),
                parent,
                bind_local: Transform {
                    translation: t * scale,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            });
        };
        let armature_parent = armature.map(|_| 0);
        push("hips", armature_parent, Vec3::new(0., 1., 0.));
        let hips = offset;
        push("spine", Some(hips), Vec3::new(0., 0.3, 0.));
        push("thigh.L", Some(hips), Vec3::new(-0.1, 0., 0.));
        push("shin.L", Some(hips + 2), Vec3::new(0., -0.5, 0.));
        push("foot.L", Some(hips + 3), Vec3::new(0., -0.5, 0.));
        push("thigh.R", Some(hips), Vec3::new(0.1, 0., 0.));
        push("shin.R", Some(hips + 5), Vec3::new(0., -0.5, 0.));
        push("foot.R", Some(hips + 6), Vec3::new(0., -0.5, 0.));
        Skeleton::new(joints).unwrap()
    }

    fn profile(skeleton: &Skeleton) -> HumanoidProfile {
        HumanoidProfile::new(
            skeleton,
            &[
                (BoneRole::Hips, "hips"),
                (BoneRole::Spine, "spine"),
                (BoneRole::Thigh(Side::Left), "thigh.L"),
                (BoneRole::Shin(Side::Left), "shin.L"),
                (BoneRole::Foot(Side::Left), "foot.L"),
                (BoneRole::Thigh(Side::Right), "thigh.R"),
                (BoneRole::Shin(Side::Right), "shin.R"),
                (BoneRole::Foot(Side::Right), "foot.R"),
            ],
        )
        .unwrap()
    }

    #[test]
    fn identical_rigs_give_an_identity_transport_and_scaled_rigs_scale_hip_motion() {
        let source = rig(1., None);
        let same = rig(1., None);
        let binding =
            RetargetBinding::from_profiles(&profile(&source), &profile(&same), &source, &same)
                .unwrap();
        let mut pose = source.bind_pose();
        pose.set_joint_rotation(1, Quat::from_rotation_x(0.4))
            .unwrap();
        pose.set_joint_translation(0, Vec3::new(0.2, 1.1, -0.3))
            .unwrap();
        let out = binding.apply_pose(&pose).unwrap();
        for (a, b) in out.local().iter().zip(pose.local()) {
            assert!((a.translation - b.translation).length() < 1e-6);
            assert!(a.rotation.angle_between(b.rotation) < 1e-6);
        }
        // A rig twice as large moves its hips twice as far.
        let big = rig(2., None);
        let binding =
            RetargetBinding::from_profiles(&profile(&source), &profile(&big), &source, &big)
                .unwrap();
        let out = binding.apply_pose(&pose).unwrap();
        let expected = big.joints()[0].bind_local.translation + Vec3::new(0.2, 0.1, -0.3) * 2.;
        assert!(
            (out.local()[0].translation - expected).length() < 1e-6,
            "{:?}",
            out.local()[0]
        );
        assert!(
            out.local()[1]
                .rotation
                .angle_between(Quat::from_rotation_x(0.4))
                < 1e-6
        );
    }

    #[test]
    fn hips_ancestor_rotation_converts_the_translation_delta() {
        let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let source = rig(1., Some(rotation));
        let target = rig(1., None);
        let source_profile = profile(&source);
        let binding =
            RetargetBinding::from_profiles(&source_profile, &profile(&target), &source, &target)
                .unwrap();
        let mut pose = source.bind_pose();
        // Hips (joint 1 under the armature) move +x in hip-local space.
        pose.set_joint_translation(1, Vec3::new(0.5, 1., 0.))
            .unwrap();
        let out = binding.apply_pose(&pose).unwrap();
        let delta = out.local()[0].translation - target.joints()[0].bind_local.translation;
        assert!(
            (delta - rotation * Vec3::new(0.5, 0., 0.)).length() < 1e-6,
            "{delta:?}"
        );
    }

    #[test]
    fn baked_clip_samples_like_live_transport() {
        let source = rig(1., None);
        let target = rig(1.5, None);
        let binding =
            RetargetBinding::from_profiles(&profile(&source), &profile(&target), &source, &target)
                .unwrap();
        let mut tracks = vec![JointTrack::default(); source.joints().len()];
        for i in 0..=8 {
            let t = i as f32 / 8.;
            tracks[2].rotations.push(QuatKey {
                time: t,
                value: Quat::from_rotation_x((t * std::f32::consts::TAU).sin() * 0.6),
            });
            tracks[0].translations.push(Vec3Key {
                time: t,
                value: Vec3::new(0., 1., 0.4 * t),
            });
        }
        let clip = AnimationClip::new("walk", 1., Playback::Loop, tracks, &source).unwrap();
        let baked = binding.retarget_clip(&clip, 60.).unwrap();
        assert!(baked.is_compatible_with(&target));
        assert_eq!(baked.name(), "walk");
        for k in 0..30 {
            let phase = f64::from(k) / 30.;
            let live = binding
                .apply_pose(&clip.try_sample_phase(&source, phase).unwrap())
                .unwrap();
            let sampled = baked.try_sample_phase(&target, phase).unwrap();
            for (a, b) in sampled.local().iter().zip(live.local()) {
                assert!((a.translation - b.translation).length() < 1e-4);
                assert!(a.rotation.angle_between(b.rotation) < 1e-3);
            }
        }
        assert!(binding.retarget_clip(&clip, 0.).is_err());
    }

    #[test]
    fn profiles_reject_unknown_names_duplicates_and_missing_legs() {
        let skeleton = rig(1., None);
        assert!(HumanoidProfile::new(&skeleton, &[(BoneRole::Hips, "nope")]).is_err());
        assert!(
            HumanoidProfile::new(
                &skeleton,
                &[(BoneRole::Hips, "hips"), (BoneRole::Spine, "hips")]
            )
            .is_err()
        );
        assert!(HumanoidProfile::new(&skeleton, &[(BoneRole::Hips, "hips")]).is_err());
        let p = profile(&skeleton);
        assert_eq!(p.joint(&BoneRole::Hips), Some(0));
        assert_eq!(p.joint(&BoneRole::Head), None);
        let other = rig(1., Some(Quat::IDENTITY));
        assert!(RetargetBinding::from_profiles(&p, &p, &other, &skeleton).is_err());
    }
}
