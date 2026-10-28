//! Replay original-source fade certification; serialized bounds never execute as proofs.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct RootRigidFadeCheckpointLimits {
    pub max_bytes: usize,
    pub max_spans: usize,
    pub max_fields: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct RootRigidFadeCompileContext {
    pub common: RootRigidEnclosure,
    pub scale: RootUniformScaleEnclosure,
    pub origin_tolerance: f64,
    pub angular_tolerance: f64,
    pub max_spans: usize,
}
#[derive(Debug)]
pub struct PreparedRootRigidFadeCheckpoint {
    /// The plan shares this animator's immutable playback bindings and snapshots.
    pub initial: Animator,
    pub plan: RootRigidFadePlan,
    pub motion: RootRigidCertifiedFadeInterval,
    pub context: RootRigidFadeCompileContext,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Field {
    wall: [u64; 2],
    linear: [[u64; 2]; 3],
    angular: [[u64; 2]; 3],
}
fn fields(motion: &RootRigidCertifiedFadeInterval) -> Vec<Field> {
    motion
        .fields()
        .iter()
        .map(|f| Field {
            wall: f.wall_times().map(f64::to_bits),
            linear: f
                .velocity_enclosure()
                .linear_bounds()
                .map(|p| p.map(f64::to_bits)),
            angular: f
                .velocity_enclosure()
                .angular_bounds()
                .map(|p| p.map(f64::to_bits)),
        })
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidFadeCheckpoint {
    version: u32,
    initial: AnimatorCheckpoint,
    wall: u64,
    axes: [bool; 3],
    preparation_capacity: usize,
    integration_capacity: usize,
    common: RootRigidEnclosureCheckpoint,
    scale: RootUniformScaleCheckpoint,
    tolerances: [u64; 2],
    path_words: Vec<u64>,
    errors: [u64; 2],
    fields: Vec<Field>,
}
impl RootRigidFadePlan {
    /// Save an actual original-source/common-similarity certificate, verifying that
    /// the supplied compile context reproduces every path/error/field bit first.
    /// # Errors
    /// Rejects missing revisions, mismatched certificates and caller admission limits.
    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_original_sources(
        &self,
        motion: &RootRigidCertifiedFadeInterval,
        common: RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
        origin_tolerance: f64,
        angular_tolerance: f64,
        integration_capacity: usize,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        limits: RootRigidFadeCheckpointLimits,
    ) -> Result<RootRigidFadeCheckpoint, AnimatorCheckpointError> {
        let approximation = motion.approximation();
        let saved = RootRigidFadeCheckpoint {
            version: 1,
            initial: self.initial.checkpoint(&identify, limits.max_bytes)?,
            wall: self.wall_seconds.to_bits(),
            axes: self.extraction_axes,
            preparation_capacity: self.preparation_capacity,
            integration_capacity,
            common: common.checkpoint(),
            scale: scale.checkpoint(),
            tolerances: [origin_tolerance.to_bits(), angular_tolerance.to_bits()],
            path_words: approximation.path.checkpoint_words(),
            errors: [
                approximation.origin_error_bound.to_bits(),
                approximation.angular_error_bound.to_bits(),
            ],
            fields: fields(motion),
        };
        let skeleton = Skeleton {
            joints: self.initial.current.rig.clone(),
        };
        saved.replay(self.initial.clone(), &skeleton, limits)?;
        Ok(saved)
    }
}
impl RootRigidFadeCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and encoded byte budgets.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, AnimatorCheckpointError> {
        if self.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// The enclosing game must also bound its file read and verify resource identities.
    /// # Errors
    /// Rejects oversized text before parsing, unknown fields and versions.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, AnimatorCheckpointError> {
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(saved)
    }
    fn replay(
        &self,
        initial: Animator,
        skeleton: &Skeleton,
        limits: RootRigidFadeCheckpointLimits,
    ) -> Result<PreparedRootRigidFadeCheckpoint, AnimatorCheckpointError> {
        self.to_json(limits.max_bytes)?;
        if limits.max_spans > MAX_ROOT_ROTATION_SPANS
            || self.preparation_capacity == 0
            || self.integration_capacity == 0
            || self.preparation_capacity > limits.max_spans
            || self.integration_capacity > limits.max_spans
            || self.fields.len() > limits.max_fields
        {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let plan = initial
            .prepare_root_rigid_fade_wall(
                skeleton,
                f64::from_bits(self.wall),
                self.axes,
                self.preparation_capacity,
            )?
            .ok_or(AnimatorCheckpointError::State)?;
        let context = RootRigidFadeCompileContext {
            common: self.common.prepare()?,
            scale: self.scale.prepare()?,
            origin_tolerance: f64::from_bits(self.tolerances[0]),
            angular_tolerance: f64::from_bits(self.tolerances[1]),
            max_spans: self.integration_capacity,
        };
        let motion = plan.integrate_original_sources_common_similarity(
            context.common,
            context.scale,
            context.origin_tolerance,
            context.angular_tolerance,
            context.max_spans,
        )?;
        let approximation = motion.approximation();
        if motion.fields().len() > limits.max_fields {
            return Err(AnimatorCheckpointError::Capacity);
        }
        if approximation.path.checkpoint_words() != self.path_words
            || [
                approximation.origin_error_bound.to_bits(),
                approximation.angular_error_bound.to_bits(),
            ] != self.errors
            || fields(&motion) != self.fields
        {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(PreparedRootRigidFadeCheckpoint {
            initial,
            plan,
            motion,
            context,
        })
    }
    /// Restore an independent, internally shared animator/plan/certificate unit.
    /// # Errors
    /// Rejects incompatible resources, corrupt state/replayed results and budgets.
    pub fn prepare(
        &self,
        skeleton: &Skeleton,
        resolve: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        limits: RootRigidFadeCheckpointLimits,
    ) -> Result<PreparedRootRigidFadeCheckpoint, AnimatorCheckpointError> {
        self.to_json(limits.max_bytes)?;
        let initial = self.initial.prepare(skeleton, resolve, limits.max_bytes)?;
        self.replay(initial, skeleton, limits)
    }
    /// Link the plan to an already staged owner's animator, retaining its shared
    /// property-binding and frozen-snapshot identity. Exact pre-tick state must match.
    /// # Errors
    /// Rejects a different source state, replayed certificate or admission policy.
    pub fn prepare_from_animator(
        &self,
        initial: &Animator,
        skeleton: &Skeleton,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        limits: RootRigidFadeCheckpointLimits,
    ) -> Result<PreparedRootRigidFadeCheckpoint, AnimatorCheckpointError> {
        self.to_json(limits.max_bytes)?;
        if initial
            .checkpoint(identify, limits.max_bytes)?
            .to_json(limits.max_bytes)?
            != self.initial.to_json(limits.max_bytes)?
        {
            return Err(AnimatorCheckpointError::State);
        }
        self.replay(initial.clone(), skeleton, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(frozen: bool) -> (Skeleton, Arc<AnimationClip>, Animator) {
        let rig = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::X,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 0.,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 1.,
                    value: Quat::IDENTITY,
                },
            ],
            ..Default::default()
        };
        let clip = Arc::new(
            AnimationClip::new_with_tangents(
                "root",
                1.,
                Playback::Loop,
                vec![track],
                vec![TrackInterpolation {
                    rotation: Interpolation::CubicSpline,
                    ..Default::default()
                }],
                vec![JointTangents {
                    rotation: vec![[glam::Vec4::ZERO; 2]; 2],
                    ..Default::default()
                }],
                &rig,
            )
            .unwrap(),
        );
        let mut animator = Animator::new(clip.clone());
        animator.advance_wall(&rig, 0.2).unwrap();
        animator
            .transition_to_at_phase(clip.clone(), 0.125, 0.4)
            .unwrap();
        if frozen {
            animator
                .transition_to_at_phase(clip.clone(), 0.125, 0.8)
                .unwrap();
        }
        (rig, clip, animator)
    }
    fn limits() -> RootRigidFadeCheckpointLimits {
        RootRigidFadeCheckpointLimits {
            max_bytes: 2 * 1024 * 1024,
            max_spans: 256,
            max_fields: 256,
        }
    }
    #[test]
    fn fresh_source_replay_retains_fade_completion_frozen_source_and_every_proof_bit() {
        for frozen in [false, true] {
            let (rig, _, animator) = fixture(frozen);
            let wall = if frozen { 0.0625 } else { 0.25 };
            let plan = animator
                .prepare_root_rigid_fade_wall(&rig, wall, [true; 3], 256)
                .unwrap()
                .unwrap();
            let common = RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: glam::DVec3::new(10., 3., -2.),
                rotation: glam::DQuat::from_xyzw(0., 1., 0., 0.),
            })
            .unwrap();
            let scale = RootUniformScaleEnclosure::from_scale(-2.).unwrap();
            let original = plan
                .integrate_original_sources_common_similarity(common, scale, 0.01, 0.001, 256)
                .unwrap();
            let saved = plan
                .checkpoint_original_sources(
                    &original,
                    common,
                    scale,
                    0.01,
                    0.001,
                    256,
                    |_, _| Some("clip-v1".into()),
                    limits(),
                )
                .unwrap();
            let json = saved.to_json(limits().max_bytes).unwrap();
            let decoded = RootRigidFadeCheckpoint::from_json(&json, limits().max_bytes).unwrap();
            let (fresh_rig, fresh_clip, fresh_owner) = fixture(frozen);
            let prepared = decoded
                .prepare(
                    &fresh_rig,
                    |key| (key == "clip-v1").then(|| (fresh_clip.clone(), None)),
                    limits(),
                )
                .unwrap();
            assert!(prepared.plan.matches_animator(&prepared.initial));
            assert_eq!(
                prepared.motion.approximation().path.checkpoint_words(),
                original.approximation().path.checkpoint_words()
            );
            assert_eq!(fields(&prepared.motion), fields(&original));
            let linked = decoded
                .prepare_from_animator(
                    &fresh_owner,
                    &fresh_rig,
                    |_, _| Some("clip-v1".into()),
                    limits(),
                )
                .unwrap();
            assert!(linked.plan.matches_animator(&fresh_owner));
            for index in 0..original.approximation().path.spans().len() {
                for fraction in [0., 0.1, 0.5, 0.9, 1.] {
                    let a = original.accepted_wall_time(index, fraction, false).unwrap();
                    let b = prepared
                        .motion
                        .accepted_wall_time(index, fraction, false)
                        .unwrap();
                    assert_eq!(a.to_bits(), b.to_bits());
                    let a = plan.prepare_accepted_frame(&rig, a).unwrap().1;
                    let b = prepared
                        .plan
                        .prepare_accepted_frame(&fresh_rig, b)
                        .unwrap()
                        .1;
                    assert_eq!(a.pose, b.pose);
                    assert_eq!(a.skin_matrices, b.skin_matrices);
                }
            }
            assert!(decoded.prepare(&fresh_rig, |_| None, limits()).is_err());
            let mut changed = fresh_owner.clone();
            changed.set_speed(0.5).unwrap();
            assert!(
                decoded
                    .prepare_from_animator(
                        &changed,
                        &fresh_rig,
                        |_, _| Some("clip-v1".into()),
                        limits()
                    )
                    .is_err()
            );
            for target in [0, 1, 2] {
                let mut corrupt = decoded.clone();
                match target {
                    0 => corrupt.errors[0] ^= 1,
                    1 => corrupt.path_words[0] ^= 1,
                    _ => corrupt.fields[0].linear[0][0] ^= 1,
                }
                assert!(
                    corrupt
                        .prepare(&fresh_rig, |_| Some((fresh_clip.clone(), None)), limits())
                        .is_err()
                );
            }
            assert_eq!(decoded.to_json(limits().max_bytes).unwrap(), json);
        }
    }
}
