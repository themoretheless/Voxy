//! Transactional clip playback, crossfade transitions, and root-motion frame staging.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::{
    AnimationClip, AnimationError, MAX_JOINTS, Playback, Pose, PropertySet, RootRigidEnclosure,
    RootRigidPath, RootRigidTransform, RootRotationPath, Skeleton, Transform, property_playback,
    rigs_match, root_curve, usable_linear,
};

#[derive(Clone, Debug)]
pub(crate) struct Transition {
    pub(crate) source: Arc<AnimationClip>,
    pub(crate) source_curve: Arc<root_curve::RootCurve>,
    pub(crate) source_time: f64,
    // Interruption captures the displayed blend once; staging shares this snapshot.
    pub(crate) source_pose: Option<Arc<Pose>>,
    pub(crate) source_properties: Option<Arc<PropertySet>>,
    pub(crate) elapsed: f64,
    pub(crate) duration: f64,
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
    pub(crate) current: Arc<AnimationClip>,
    pub(crate) motion_joint: u16,
    pub(crate) motion_curve: Arc<root_curve::RootCurve>,
    pub(crate) time: f64,
    pub(crate) speed: f32,
    pub(crate) transition: Option<Transition>,
    pub(crate) property_bindings: Option<Arc<property_playback::PropertyBindings>>,
    pub(crate) accepted_properties: PropertySet,
}

#[derive(Clone, Debug)]
pub struct AnimatorFrame {
    /// Properties sampled/blended transactionally from the same clocks as this pose.
    pub properties: PropertySet,
    pub pose: Pose,
    pub skin_matrices: Vec<Mat4>,
    /// Translation displacement in the selected joint's parent-local frame.
    /// It excludes animated ancestor motion and has not been applied to the pose.
    pub root_motion: Vec3,
    pub root_motion_joint: u16,
    pub transition_weight: f32,
}

impl AnimatorFrame {
    /// Creates a pre-allocated reusable frame buffer for [`Animator::advance_into`].
    #[must_use]
    pub fn reusable(skeleton: &Skeleton) -> Self {
        let pose = skeleton.bind_pose();
        let skin_matrices = vec![Mat4::IDENTITY; skeleton.joints().len()];
        Self {
            properties: PropertySet::default(),
            pose,
            skin_matrices,
            root_motion: Vec3::ZERO,
            root_motion_joint: 0,
            transition_weight: 1.0,
        }
    }

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
        self.pose
            .skin_matrices_vec_into(skeleton, &mut self.skin_matrices)?;
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
        self.pose
            .skin_matrices_vec_into(skeleton, &mut self.skin_matrices)?;
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
            property_bindings: None,
            accepted_properties: PropertySet::default(),
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
        let next_properties = self.sample_bound_properties(&next, next_time)?;
        if duration == 0.0 {
            self.motion_curve = next_curve;
            self.current = next;
            self.time = next_time;
            self.transition = None;
            self.accepted_properties = next_properties;
            return Ok(());
        }
        let source_properties = if self.transition.is_some() && self.property_bindings.is_some() {
            Some(Arc::new(self.sample_displayed_properties()?))
        } else {
            None
        };
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
            source_properties,
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

    /// Advances animation time into a reusable [`AnimatorFrame`], avoiding per-frame
    /// `Pose` and `skin_matrices` `Vec` allocations on steady-state single-clip playback.
    ///
    /// # Errors
    ///
    /// Rejects invalid timestep, clip/skeleton mismatch or numerical overflow.
    /// Every error preserves the prior clock, transition state, and `out` frame.
    pub fn advance_into(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
        out: &mut AnimatorFrame,
    ) -> Result<(), AnimationError> {
        self.advance_wall_into(skeleton, f64::from(dt), out)
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
        let mut out = AnimatorFrame {
            properties: PropertySet::default(),
            pose: Pose {
                rig: skeleton.joints.clone(),
                local: Vec::with_capacity(skeleton.joints.len()),
            },
            skin_matrices: Vec::with_capacity(skeleton.joints.len()),
            root_motion: Vec3::ZERO,
            root_motion_joint: self.motion_joint,
            transition_weight: 1.0,
        };
        self.advance_wall_into(skeleton, dt, &mut out)?;
        Ok(out)
    }

    /// Advances by `dt` wall seconds into a reusable [`AnimatorFrame`].
    /// Every error preserves the prior clock, transition state, and `out` frame.
    pub fn advance_wall_into(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
        out: &mut AnimatorFrame,
    ) -> Result<(), AnimationError> {
        // Clip references are shared; only the small clock/transition state is
        // staged. Failed pose or root-motion evaluation never publishes it.
        let mut candidate = self.clone();
        candidate.advance_candidate_wall_into(skeleton, dt, out)?;
        *self = candidate;
        Ok(())
    }

    pub(crate) fn advance_candidate(
        &mut self,
        skeleton: &Skeleton,
        dt: f32,
    ) -> Result<AnimatorFrame, AnimationError> {
        self.advance_candidate_wall(skeleton, f64::from(dt))
    }

    pub(crate) fn advance_candidate_wall(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
    ) -> Result<AnimatorFrame, AnimationError> {
        let mut out = AnimatorFrame {
            properties: PropertySet::default(),
            pose: Pose {
                rig: skeleton.joints.clone(),
                local: Vec::with_capacity(skeleton.joints.len()),
            },
            skin_matrices: Vec::with_capacity(skeleton.joints.len()),
            root_motion: Vec3::ZERO,
            root_motion_joint: self.motion_joint,
            transition_weight: 1.0,
        };
        self.advance_candidate_wall_into(skeleton, dt, &mut out)?;
        Ok(out)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn advance_candidate_wall_into(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
        out: &mut AnimatorFrame,
    ) -> Result<(), AnimationError> {
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

        if self.transition.is_none() {
            // Zero-heap-allocation transactional single-clip fast path using bounded MAX_JOINTS stack scratch.
            let joint_count = skeleton.joints.len();
            let mut local_scratch = [Transform::IDENTITY; MAX_JOINTS];
            let mut global_scratch = [Mat4::IDENTITY; MAX_JOINTS];
            let mut skin_scratch = [Mat4::IDENTITY; MAX_JOINTS];
            let local_time = self.current.phase(self.time) as f32;
            self.current
                .sample_local_slice(local_time, &mut local_scratch[..joint_count]);
            for (index, (local, joint)) in local_scratch[..joint_count]
                .iter()
                .zip(skeleton.joints.iter())
                .enumerate()
            {
                if !local.is_valid() {
                    return Err(AnimationError::InvalidPose(index));
                }
                let matrix = joint.parent.map_or_else(
                    || local.matrix(),
                    |parent| global_scratch[usize::from(parent)] * local.matrix(),
                );
                let palette = matrix * joint.inverse_bind;
                if !usable_linear(matrix.as_dmat4()) || !usable_linear(palette.as_dmat4()) {
                    return Err(AnimationError::InvalidPose(index));
                }
                global_scratch[index] = matrix;
                skin_scratch[index] = palette;
            }
            self.time = self.current.phase(self.time);
            let properties = self.sample_displayed_properties()?;
            if !Arc::ptr_eq(&out.pose.rig, &self.current.rig) {
                out.pose.rig = self.current.rig.clone();
            }
            out.pose.local.resize(joint_count, Transform::IDENTITY);
            out.pose
                .local
                .copy_from_slice(&local_scratch[..joint_count]);
            out.skin_matrices.resize(joint_count, Mat4::IDENTITY);
            out.skin_matrices
                .copy_from_slice(&skin_scratch[..joint_count]);
            self.accepted_properties = properties.clone();
            out.properties = properties;
            out.root_motion = root_motion;
            out.root_motion_joint = self.motion_joint;
            out.transition_weight = 1.0;
            return Ok(());
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
        let mut skin_matrices = Vec::with_capacity(skeleton.joints.len());
        pose.skin_matrices_vec_into(skeleton, &mut skin_matrices)?;
        // Bound storage only after integrating this complete tick, including all loops.
        self.time = self.current.phase(self.time);
        if let Some(transition) = &mut self.transition {
            transition.source_time = transition.source.phase(transition.source_time);
        }
        let properties = self.sample_displayed_properties()?;
        self.accepted_properties = properties.clone();
        out.properties = properties;
        out.pose = pose;
        out.skin_matrices = skin_matrices;
        out.root_motion = root_motion;
        out.root_motion_joint = self.motion_joint;
        out.transition_weight = weight;
        Ok(())
    }
}
