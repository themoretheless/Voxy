//! Read-only rigid fade candidates from the Animator's existing clocks.
use super::*;

#[derive(Clone, Debug)]
pub struct RootRigidFadePlan {
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
        self.phase_interval(dt)?;
        let Some(transition) = &self.transition else {
            return Ok(None);
        };
        if !(1..=MAX_ROOT_ROTATION_SPANS).contains(&max_spans) {
            return Err(AnimationError::RootRigidBudget);
        }
        let wall = f64::from(dt);
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
        let frame = candidate.advance_candidate(skeleton, dt)?;
        Ok(Some(RootRigidFadePlan {
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
