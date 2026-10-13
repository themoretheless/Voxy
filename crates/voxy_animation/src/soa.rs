//! Structure-of-Arrays (SoA) skeletal pose evaluation, prefetching, and lock-free update queue
//! (Optimizations #72-75).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use glam::{Mat4, Quat, Vec3};

use crate::{
    AnimationClip, AnimationError, Joint, Pose, Skeleton, Transform, lerp_vec3, rigs_match,
    usable_linear,
};

/// Sentinel parent index for root joints in [`SoaSkeleton`].
pub const SOA_NO_PARENT: u16 = u16::MAX;

/// Structure-of-Arrays (SoA) skeleton layout separating hierarchy topology, bind pose
/// channels, and inverse-bind matrices into dense contiguous allocations.
///
/// Unlike [`Skeleton`], which interleaves `Arc<str>` joint names with transforms and
/// `Mat4` inverse binds (128 bytes per joint), `SoaSkeleton` keeps `parents` (`2` bytes/joint)
/// and `inverse_binds` (`64` bytes/joint) tightly packed for L1 cache locality during
/// hierarchical transform propagation and skin matrix generation.
#[derive(Clone, Debug)]
pub struct SoaSkeleton {
    rig: Arc<[Joint]>,
    parents: Arc<[u16]>,
    bind_translations: Arc<[Vec3]>,
    bind_rotations: Arc<[Quat]>,
    bind_scales: Arc<[Vec3]>,
    inverse_binds: Arc<[Mat4]>,
}

impl SoaSkeleton {
    /// Builds a contiguous SoA view from a validated [`Skeleton`].
    #[must_use]
    pub fn from_skeleton(skeleton: &Skeleton) -> Self {
        let joints = skeleton.joints();
        let len = joints.len();
        let mut parents = Vec::with_capacity(len);
        let mut bind_translations = Vec::with_capacity(len);
        let mut bind_rotations = Vec::with_capacity(len);
        let mut bind_scales = Vec::with_capacity(len);
        let mut inverse_binds = Vec::with_capacity(len);
        for joint in joints {
            parents.push(joint.parent.unwrap_or(SOA_NO_PARENT));
            bind_translations.push(joint.bind_local.translation);
            bind_rotations.push(joint.bind_local.rotation);
            bind_scales.push(joint.bind_local.scale);
            inverse_binds.push(joint.inverse_bind);
        }
        Self {
            rig: Arc::clone(&skeleton.joints),
            parents: parents.into(),
            bind_translations: bind_translations.into(),
            bind_rotations: bind_rotations.into(),
            bind_scales: bind_scales.into(),
            inverse_binds: inverse_binds.into(),
        }
    }

    #[must_use]
    pub fn joint_count(&self) -> usize {
        self.inverse_binds.len()
    }

    #[must_use]
    pub fn parents(&self) -> &[u16] {
        &self.parents
    }

    #[must_use]
    pub fn inverse_binds(&self) -> &[Mat4] {
        &self.inverse_binds
    }

    #[must_use]
    pub fn bind_pose(&self) -> SoaPose {
        SoaPose {
            rig: Arc::clone(&self.rig),
            translations: self.bind_translations.to_vec(),
            rotations: self.bind_rotations.to_vec(),
            scales: self.bind_scales.to_vec(),
            validated: true,
        }
    }
}

/// Structure-of-Arrays (SoA) local pose separating translations, rotations, and scales
/// into contiguous SIMD-friendly streams.
#[derive(Clone, Debug, PartialEq)]
pub struct SoaPose {
    pub(crate) rig: Arc<[Joint]>,
    pub(crate) translations: Vec<Vec3>,
    pub(crate) rotations: Vec<Quat>,
    pub(crate) scales: Vec<Vec3>,
    pub(crate) validated: bool,
}

impl SoaPose {
    /// Converts an AoS [`Pose`] into a contiguous SoA [`SoaPose`].
    #[must_use]
    pub fn from_pose(pose: &Pose) -> Self {
        let len = pose.local.len();
        let mut translations = Vec::with_capacity(len);
        let mut rotations = Vec::with_capacity(len);
        let mut scales = Vec::with_capacity(len);
        let mut validated = true;
        for transform in &pose.local {
            validated &= transform.is_valid();
            translations.push(transform.translation);
            rotations.push(transform.rotation);
            scales.push(transform.scale);
        }
        Self {
            rig: Arc::clone(&pose.rig),
            translations,
            rotations,
            scales,
            validated,
        }
    }

    /// Converts this [`SoaPose`] back into an AoS [`Pose`].
    #[must_use]
    pub fn to_pose(&self) -> Pose {
        let local = self
            .translations
            .iter()
            .zip(&self.rotations)
            .zip(&self.scales)
            .map(|((&translation, &rotation), &scale)| Transform {
                translation,
                rotation,
                scale,
            })
            .collect();
        Pose {
            rig: Arc::clone(&self.rig),
            local,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.translations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    #[must_use]
    pub fn translations(&self) -> &[Vec3] {
        &self.translations
    }

    #[must_use]
    pub fn rotations(&self) -> &[Quat] {
        &self.rotations
    }

    #[must_use]
    pub fn scales(&self) -> &[Vec3] {
        &self.scales
    }

    /// Blends two SoA poses channel-by-channel into `out`, reusing `out`'s allocations.
    ///
    /// Separating translation, rotation, and scale loops allows sequential cache-line
    /// streaming without AoS striding.
    ///
    /// # Errors
    /// Rejects mismatched pose lengths, incompatible rigs, non-finite weights, or invalid TRS.
    pub fn blend_into(
        a: &Self,
        b: &Self,
        weight: f32,
        out: &mut Self,
    ) -> Result<(), AnimationError> {
        let len = a.len();
        if len != b.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&a.rig, &b.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = weight.clamp(0.0, 1.0);
        if !a.validated || !b.validated {
            for index in 0..len {
                let ta = Transform {
                    translation: a.translations[index],
                    rotation: a.rotations[index],
                    scale: a.scales[index],
                };
                let tb = Transform {
                    translation: b.translations[index],
                    rotation: b.rotations[index],
                    scale: b.scales[index],
                };
                if !ta.is_valid() || !tb.is_valid() {
                    return Err(AnimationError::InvalidPose(index));
                }
            }
        }
        out.rig = Arc::clone(&a.rig);
        out.translations.clear();
        out.rotations.clear();
        out.scales.clear();
        out.translations.reserve(len);
        out.rotations.reserve(len);
        out.scales.reserve(len);

        // Stream 1: contiguous translation lerp
        for (index, (&ta, &tb)) in a.translations.iter().zip(&b.translations).enumerate() {
            let t = lerp_vec3(ta, tb, weight);
            if !t.is_finite() {
                out.validated = false;
                return Err(AnimationError::InvalidPose(index));
            }
            out.translations.push(t);
        }
        // Stream 2: contiguous quaternion slerp
        for (index, (&ra, &rb)) in a.rotations.iter().zip(&b.rotations).enumerate() {
            let r = ra.slerp(rb, weight).normalize();
            if !r.is_finite() || !r.is_normalized() {
                out.validated = false;
                return Err(AnimationError::InvalidPose(index));
            }
            out.rotations.push(r);
        }
        // Stream 3: contiguous scale lerp
        for (index, (&sa, &sb)) in a.scales.iter().zip(&b.scales).enumerate() {
            let s = lerp_vec3(sa, sb, weight);
            if !s.is_finite() || s.abs().min_element() <= 0.0 {
                out.validated = false;
                return Err(AnimationError::InvalidPose(index));
            }
            out.scales.push(s);
        }
        out.validated = true;
        Ok(())
    }

    /// Blends two SoA poses and returns a new [`SoaPose`].
    ///
    /// # Errors
    /// Rejects mismatched lengths, incompatible rigs, non-finite weights, or invalid TRS.
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Result<Self, AnimationError> {
        let mut out = Self {
            rig: Arc::clone(&a.rig),
            translations: Vec::with_capacity(a.len()),
            rotations: Vec::with_capacity(a.len()),
            scales: Vec::with_capacity(a.len()),
            validated: false,
        };
        Self::blend_into(a, b, weight, &mut out)?;
        Ok(out)
    }

    /// Computes skinning palette matrices into caller-supplied `global_scratch` and `skin_out`
    /// buffers using [`SoaSkeleton`]'s contiguous topology and inverse-bind streams.
    ///
    /// # Errors
    /// Rejects incompatible skeletons, invalid local TRS, or singular/overflowing matrices.
    pub fn skin_matrices_into(
        &self,
        skeleton: &SoaSkeleton,
        global_scratch: &mut Vec<Mat4>,
        skin_out: &mut Vec<Mat4>,
    ) -> Result<(), AnimationError> {
        let len = self.len();
        if len != skeleton.joint_count() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        global_scratch.clear();
        skin_out.clear();
        global_scratch.reserve(len);
        skin_out.reserve(len);

        let parents = skeleton.parents();
        let inverse_binds = skeleton.inverse_binds();
        let prevalidated = self.validated;

        for index in 0..len {
            let translation = self.translations[index];
            let rotation = self.rotations[index];
            let scale = self.scales[index];
            if !prevalidated {
                let local = Transform {
                    translation,
                    rotation,
                    scale,
                };
                if !local.is_valid() {
                    return Err(AnimationError::InvalidPose(index));
                }
            }
            let local_matrix = Mat4::from_scale_rotation_translation(scale, rotation, translation);
            let parent = parents[index];
            let matrix = if parent == SOA_NO_PARENT {
                local_matrix
            } else {
                global_scratch[usize::from(parent)] * local_matrix
            };
            let palette = matrix * inverse_binds[index];
            if !usable_linear(matrix.as_dmat4()) || !usable_linear(palette.as_dmat4()) {
                return Err(AnimationError::InvalidPose(index));
            }
            global_scratch.push(matrix);
            skin_out.push(palette);
        }
        Ok(())
    }

    /// Computes `global_joint * inverse_bind` matrices in skeleton order.
    ///
    /// # Errors
    /// Rejects incompatible skeletons, invalid local TRS, or singular/overflowing matrices.
    pub fn skin_matrices(&self, skeleton: &SoaSkeleton) -> Result<Vec<Mat4>, AnimationError> {
        let mut global = Vec::with_capacity(self.len());
        let mut skin = Vec::with_capacity(self.len());
        self.skin_matrices_into(skeleton, &mut global, &mut skin)?;
        Ok(skin)
    }
}

/// A queued skeletal animation evaluation job for batch pre-fetching before skinning.
#[derive(Clone, Debug)]
pub struct AnimationUpdateJob {
    pub instance_id: usize,
    pub clip: Arc<AnimationClip>,
    pub time: f32,
}

/// Completed skinning palette output produced by [`AnimationUpdateQueue`].
#[derive(Clone, Debug, PartialEq)]
pub struct CompletedAnimationUpdate {
    pub instance_id: usize,
    pub pose: SoaPose,
    pub skin_matrices: Vec<Mat4>,
}

/// Lock-free atomic work-stealing batch queue for pre-fetching and evaluating
/// skeletal animations across worker threads before the GPU skinning pass.
#[derive(Debug)]
pub struct AnimationUpdateQueue {
    jobs: Vec<AnimationUpdateJob>,
    next_index: AtomicUsize,
}

impl AnimationUpdateQueue {
    /// Creates an empty batch queue with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            jobs: Vec::with_capacity(capacity),
            next_index: AtomicUsize::new(0),
        }
    }

    /// Stages a new animation update job prior to parallel dispatch.
    pub fn push(&mut self, job: AnimationUpdateJob) {
        self.jobs.push(job);
    }

    /// Clears staged jobs and resets the atomic cursor for the next frame.
    pub fn clear(&mut self) {
        self.jobs.clear();
        self.next_index.store(0, Ordering::Relaxed);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Atomically claims the next animation job without locks.
    /// Safe to call concurrently from multiple worker threads during batch evaluation.
    #[must_use]
    pub fn claim_next(&self) -> Option<&AnimationUpdateJob> {
        let idx = self.next_index.fetch_add(1, Ordering::Relaxed);
        self.jobs.get(idx)
    }

    /// Evaluates all queued jobs in parallel (or inline for small batches),
    /// pre-fetching keyframe tracks and computing SoA poses and skin matrices.
    ///
    /// Results are returned in deterministic submission order.
    ///
    /// # Errors
    /// Returns the first [`AnimationError`] encountered if any job fails validation.
    pub fn evaluate_all(
        &self,
        skeleton: &Skeleton,
        soa_skeleton: &SoaSkeleton,
    ) -> Result<Vec<CompletedAnimationUpdate>, AnimationError> {
        self.next_index.store(0, Ordering::Relaxed);
        if self.jobs.is_empty() {
            return Ok(Vec::new());
        }
        if self.jobs.len() <= 2 {
            let mut global_scratch = Vec::with_capacity(soa_skeleton.joint_count());
            let mut outputs = Vec::with_capacity(self.jobs.len());
            while let Some(job) = self.claim_next() {
                let pose = job.clip.try_sample_soa(skeleton, job.time)?;
                let mut skin_matrices = Vec::with_capacity(soa_skeleton.joint_count());
                pose.skin_matrices_into(soa_skeleton, &mut global_scratch, &mut skin_matrices)?;
                outputs.push(CompletedAnimationUpdate {
                    instance_id: job.instance_id,
                    pose,
                    skin_matrices,
                });
            }
            return Ok(outputs);
        }

        let worker_count = std::thread::available_parallelism()
            .map_or(2, usize::from)
            .min(self.jobs.len());
        let mut worker_batches: Vec<
            Result<Vec<(usize, CompletedAnimationUpdate)>, AnimationError>,
        > = Vec::with_capacity(worker_count);

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(worker_count);
            for _ in 0..worker_count {
                handles.push(scope.spawn(|| {
                    let mut global_scratch = Vec::with_capacity(soa_skeleton.joint_count());
                    let mut local_out = Vec::new();
                    loop {
                        let idx = self.next_index.fetch_add(1, Ordering::Relaxed);
                        let Some(job) = self.jobs.get(idx) else {
                            break;
                        };
                        let pose = job.clip.try_sample_soa(skeleton, job.time)?;
                        let mut skin_matrices = Vec::with_capacity(soa_skeleton.joint_count());
                        pose.skin_matrices_into(
                            soa_skeleton,
                            &mut global_scratch,
                            &mut skin_matrices,
                        )?;
                        local_out.push((
                            idx,
                            CompletedAnimationUpdate {
                                instance_id: job.instance_id,
                                pose,
                                skin_matrices,
                            },
                        ));
                    }
                    Ok(local_out)
                }));
            }
            for handle in handles {
                worker_batches.push(handle.join().expect("animation worker thread panicked"));
            }
        });

        let mut indexed = Vec::with_capacity(self.jobs.len());
        for batch in worker_batches {
            indexed.extend(batch?);
        }
        indexed.sort_unstable_by_key(|(idx, _)| *idx);
        Ok(indexed.into_iter().map(|(_, update)| update).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointTrack, Playback, QuatKey, Vec3Key};

    fn test_skeleton() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                name: "root".into(),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: "child".into(),
                parent: Some(0),
                bind_local: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
                inverse_bind: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            },
        ])
        .unwrap()
    }

    #[test]
    fn soa_pose_blend_and_skin_matrices_match_aos_exact() {
        let skeleton = test_skeleton();
        let soa_skeleton = skeleton.to_soa();
        let mut pose_a = skeleton.bind_pose();
        pose_a
            .set_joint_translation(0, Vec3::new(1.0, 2.0, 3.0))
            .unwrap();
        let mut pose_b = skeleton.bind_pose();
        pose_b
            .set_joint_translation(0, Vec3::new(5.0, 6.0, 7.0))
            .unwrap();
        pose_b
            .set_joint_rotation(1, Quat::from_rotation_z(0.5))
            .unwrap();

        let aos_blended = Pose::blend(&pose_a, &pose_b, 0.35).unwrap();
        let aos_skin = aos_blended.skin_matrices(&skeleton).unwrap();

        let soa_a = pose_a.to_soa();
        let soa_b = pose_b.to_soa();
        let soa_blended = SoaPose::blend(&soa_a, &soa_b, 0.35).unwrap();
        let soa_skin = soa_blended.skin_matrices(&soa_skeleton).unwrap();

        assert_eq!(soa_blended.to_pose(), aos_blended);
        assert_eq!(soa_skin, aos_skin);
    }

    #[test]
    fn lock_free_animation_update_queue_evaluates_batches_deterministically() {
        let skeleton = test_skeleton();
        let soa_skeleton = skeleton.to_soa();
        let tracks = vec![
            JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.0,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 1.0,
                        value: Vec3::new(2.0, 0.0, 0.0),
                    },
                ],
                rotations: vec![],
                scales: vec![],
            },
            JointTrack {
                translations: vec![],
                rotations: vec![
                    QuatKey {
                        time: 0.0,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 1.0,
                        value: Quat::from_rotation_y(0.4),
                    },
                ],
                scales: vec![],
            },
        ];
        let clip =
            Arc::new(AnimationClip::new("walk", 1.0, Playback::Loop, tracks, &skeleton).unwrap());
        let mut queue = AnimationUpdateQueue::with_capacity(8);
        for i in 0..8 {
            queue.push(AnimationUpdateJob {
                instance_id: i,
                clip: Arc::clone(&clip),
                time: (i as f32) * 0.125,
            });
        }
        let results = queue.evaluate_all(&skeleton, &soa_skeleton).unwrap();
        assert_eq!(results.len(), 8);
        for (i, item) in results.iter().enumerate() {
            assert_eq!(item.instance_id, i);
            let expected_pose = clip.try_sample(&skeleton, (i as f32) * 0.125).unwrap();
            let expected_skin = expected_pose.skin_matrices(&skeleton).unwrap();
            assert_eq!(item.skin_matrices, expected_skin);
        }
    }
}
