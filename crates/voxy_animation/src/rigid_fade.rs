//! Read-only rigid fade candidates from the Animator's existing clocks.
use super::*;

#[derive(Clone, Debug)]
pub struct RootRigidFadePlan {
    initial: Animator,
    wall_seconds: f64,
    /// Staged full-tick animator. Publish only after motion/physics acceptance.
    pub candidate: Animator,
    pub frame: AnimatorFrame,
    pub source_fade: Option<RootRigidPath>,
    pub target_fade: RootRigidPath,
    pub target_tail: RootRigidPath,
    /// Root extraction factors at the original clip phases, before path rebasing.
    /// These are relative to each curve's authored origin, not actor-world frames.
    pub source_factor: Option<RootRigidTransform>,
    pub target_factor: RootRigidTransform,
    pub tail_factor: RootRigidTransform,
    pub weights: [f64; 2],
    pub fade_wall_seconds: f64,
    pub tail_wall_seconds: f64,
}
impl RootRigidFadePlan {
    /// Maps both interval-local paths through their stored authored-origin
    /// factors into an explicit common frame. This encloses composition rather
    /// than rounding the two composed frames first. Imported factor/compiler
    /// errors preceding the stored data remain a separate caller obligation.
    pub fn integrate_authored_common_frame(
        &self,
        authored_to_common: RootRigidTransform,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<RootRigidCertifiedFadeInterval, AnimationError> {
        let common = RootRigidEnclosure::from_transform(authored_to_common)?;
        let map = |factor| common.compose(&RootRigidEnclosure::from_transform(factor)?);
        let source = match (&self.source_fade, self.source_factor) {
            (Some(path), Some(factor)) => Some(RootRigidMappedPath::from_enclosed_frame(path,map(factor)?,1.)?),
            (None, None) => None,
            _ => return Err(AnimationError::InvalidRetargetBinding),
        };
        let target_frame = map(self.target_factor)?;
        let target = RootRigidMappedPath::from_enclosed_frame(&self.target_fade,target_frame,1.)?;
        let completion = if self.tail_wall_seconds == 0. { None } else {
            let frame = target_frame.compose(&self.target_fade.continuous_end_enclosure(max_spans)?)?;
            Some((RootRigidMappedPath::from_enclosed_frame(&self.target_tail,frame,1.)?,self.wall_seconds))
        };
        RootRigidCertifiedFadeInterval::integrate_paths_with_completion(source,target,
            self.weights,self.fade_wall_seconds,completion,origin_tolerance,angular_tolerance,max_spans)
    }

    /// Exact snapshot admission for a staged candidate. Clip and frozen-pose
    /// identity are checked as well as clocks, selection and transition state.
    pub fn matches_animator(&self, animator: &Animator) -> bool {
        let initial = &self.initial;
        if !Arc::ptr_eq(&initial.current, &animator.current)
            || !Arc::ptr_eq(&initial.motion_curve, &animator.motion_curve)
            || initial.time != animator.time || initial.speed != animator.speed
            || initial.motion_joint != animator.motion_joint {
            return false;
        }
        match (&initial.transition, &animator.transition) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(&a.source, &b.source)
                && Arc::ptr_eq(&a.source_curve, &b.source_curve)
                && a.source_time == b.source_time && a.elapsed == b.elapsed
                && a.duration == b.duration
                && match (&a.source_pose, &b.source_pose) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                },
            _ => false,
        }
    }

    /// Rebuilds clocks and displayed pose for an accepted wall-time prefix from
    /// the immutable pre-tick snapshot. The consumer must still validate final
    /// actor pose and asset identity before publishing this candidate.
    pub fn prepare_accepted_frame(
        &self,
        skeleton: &Skeleton,
        accepted_wall_seconds: f64,
    ) -> Result<(Animator, AnimatorFrame), AnimationError> {
        if !accepted_wall_seconds.is_finite()
            || accepted_wall_seconds < 0. || accepted_wall_seconds > self.wall_seconds {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut candidate = self.initial.clone();
        let frame = candidate.advance_candidate_wall(skeleton, accepted_wall_seconds)?;
        Ok((candidate, frame))
    }

    /// Builds the complete staged moving tick through one outward accumulator.
    /// The total stored wall endpoint is fade plus completion duration. No clock
    /// or actor pose is published; zero-duration fades require a separate policy.
    pub fn integrate_certified_tick(
        &self,
        source_frame: Option<RootRigidTransform>,
        target_frame: RootRigidTransform,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<RootRigidCertifiedFadeInterval, AnimationError> {
        let source = match (&self.source_fade, source_frame) {
            (Some(path), Some(frame)) => Some(RootRigidMappedPath::new(path, frame, 1.)?),
            (None, None) => None,
            _ => return Err(AnimationError::InvalidRetargetBinding),
        };
        let completion = if self.tail_wall_seconds == 0. { None } else {
            Some((self.certified_target_tail_mapping(target_frame, max_spans)?,
                self.wall_seconds))
        };
        RootRigidCertifiedFadeInterval::integrate_paths_with_completion(
            source, RootRigidMappedPath::new(&self.target_fade, target_frame, 1.)?,
            self.weights, self.fade_wall_seconds, completion,
            origin_tolerance, angular_tolerance, max_spans,
        )
    }

    /// Maps the target completion tail through the original target endpoint,
    /// retaining its enclosed pose rather than using the blended endpoint or
    /// the floating cached `end_transform`. This does not integrate the tail.
    pub fn certified_target_tail_mapping(
        &self,
        target_frame: RootRigidTransform,
        max_spans: usize,
    ) -> Result<RootRigidMappedPath<'_>, AnimationError> {
        let frame = RootRigidEnclosure::from_transform(target_frame)?
            .compose(&self.target_fade.continuous_end_enclosure(max_spans)?)?;
        RootRigidMappedPath::from_enclosed_frame(&self.target_tail, frame, 1.)
    }

    /// Encloses the fade portion using the original stored path clocks and keys.
    /// The returned interval ends at `fade_wall_seconds`; a completion tail is
    /// deliberately separate until its original endpoint frame is enclosed.
    /// Neither the staged animator nor its clocks are published here.
    pub fn integrate_certified_fade(
        &self,
        source_frame: Option<RootRigidTransform>,
        target_frame: RootRigidTransform,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<RootRigidCertifiedFadeInterval, AnimationError> {
        let source = match (&self.source_fade, source_frame) {
            (Some(path), Some(frame)) => Some(RootRigidMappedPath::new(path, frame, 1.)?),
            (None, None) => None,
            _ => return Err(AnimationError::InvalidRetargetBinding),
        };
        RootRigidCertifiedFadeInterval::integrate_paths(
            source,
            RootRigidMappedPath::new(&self.target_fade, target_frame, 1.)?,
            self.weights,
            self.fade_wall_seconds,
            origin_tolerance,
            angular_tolerance,
            max_spans,
        )
    }

    /// Builds this staged tick in an explicitly chosen common rigid frame.
    /// Frames map each fade path's interval-local coordinates to that frame.
    /// A frozen source requires `None`; an actual source requires `Some`.
    /// The tail continues the target's spatial field at its own fade endpoint,
    /// rather than restarting its local axes at the blended endpoint.
    /// No animator clock is published and no physical acceptance is implied.
    pub fn integrate_spatial(
        &self,
        source_frame: Option<RootRigidTransform>,
        target_frame: RootRigidTransform,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<RootRigidApproximation, AnimationError> {
        target_frame.compose(RootRigidTransform::IDENTITY)?;
        let source = match (&self.source_fade, source_frame) {
            (Some(path), Some(frame)) => path.transformed(frame.rotation, 1., frame.translation)?,
            (None, None) => RootRigidPath::from_twists(&[], 0)?,
            _ => return Err(AnimationError::InvalidRetargetBinding),
        };
        let target =
            self.target_fade
                .transformed(target_frame.rotation, 1., target_frame.translation)?;
        let tail_frame = target_frame.compose(self.target_fade.end_transform())?;
        let tail = self
            .target_tail
            .transformed(tail_frame.rotation, 1., tail_frame.translation)?
            .retimed(self.tail_wall_seconds)?;
        let capacity = max_spans.min(MAX_ROOT_ROTATION_SPANS);
        let available = capacity
            .checked_sub(tail.spans().len())
            .ok_or(AnimationError::RootRigidBudget)?;
        let fade = if self.fade_wall_seconds == 0. {
            // Preserve input/tolerance validation through the existing integrator.
            RootRigidPath::integrate_spatial(
                0.,
                RootTwistRateBounds {
                    linear: 0.,
                    angular: 0.,
                },
                origin_tolerance,
                angular_tolerance,
                available,
                |_| unreachable!("a zero-time interval has no samples"),
            )?
        } else {
            source.blend_spatial(
                &target,
                self.weights,
                self.fade_wall_seconds,
                origin_tolerance,
                angular_tolerance,
                available,
            )?
        };
        fade.append_spatial(
            &RootRigidApproximation {
                path: tail,
                origin_error_bound: 0.,
                angular_error_bound: 0.,
            },
            capacity,
        )
    }
}
impl Animator {
    /// Stages active-fade paths and displayed pose without publishing either clock.
    /// Frozen interruption sources have zero motion and no invented clip phase.
    /// Frames are explicit: the consumer must choose common rig/actor coordinates.
    /// This method does not enable moving fades in the existing physical API.
    pub fn prepare_root_rigid_fade(
        &self,
        skeleton: &Skeleton,
        dt: f32,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<Option<RootRigidFadePlan>, AnimationError> {
        self.prepare_root_rigid_fade_wall(skeleton, f64::from(dt), axes, max_spans)
    }

    /// Stages the original fixed-step wall endpoint without narrowing to f32.
    pub fn prepare_root_rigid_fade_wall(
        &self,
        skeleton: &Skeleton,
        dt: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<Option<RootRigidFadePlan>, AnimationError> {
        self.phase_interval_wall(dt)?;
        let Some(transition) = &self.transition else {
            return Ok(None);
        };
        if !(1..=MAX_ROOT_ROTATION_SPANS).contains(&max_spans) {
            return Err(AnimationError::RootRigidBudget);
        }
        let wall = dt;
        let fade = wall.min((transition.duration - transition.elapsed).max(0.));
        let start = self.current.phase(self.time);
        let delta = fade * f64::from(self.speed);
        let fade_end = start + delta;
        let end = start + wall * f64::from(self.speed);
        let target_curve = self.current.root_rigid_curve(self.motion_joint)?;
        let target_factor = target_curve.sample(start, axes)?;
        let target_fade = target_curve.path(start, fade_end, axes, max_spans)?;
        let target_tail = target_curve.path(fade_end, end, axes, max_spans)?;
        let tail_factor = target_curve.sample(fade_end, axes)?;
        let (source_fade, source_factor) = if transition.source_pose.is_some() {
            (None, None)
        } else {
            let curve = transition.source.root_rigid_curve(self.motion_joint)?;
            let start = transition.source.phase(transition.source_time);
            (
                Some(curve.path(start, start + delta, axes, max_spans)?),
                Some(curve.sample(start, axes)?),
            )
        };
        let count = target_fade.spans().len()
            + target_tail.spans().len()
            + source_fade.as_ref().map_or(0, |path| path.spans().len());
        if count > max_spans {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut candidate = self.clone();
        let frame = candidate.advance_candidate_wall(skeleton, dt)?;
        Ok(Some(RootRigidFadePlan {
            initial: self.clone(),
            wall_seconds: wall,
            candidate,
            frame,
            source_fade,
            target_fade,
            target_tail,
            source_factor,
            target_factor,
            tail_factor,
            weights: [
                transition.elapsed / transition.duration,
                (transition.elapsed + fade) / transition.duration,
            ],
            fade_wall_seconds: fade,
            tail_wall_seconds: wall - fade,
        }))
    }
}
