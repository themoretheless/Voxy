//! Read-only phase metadata from the same clocks that produced the blended pose.
use super::{AnimationClip, Animator};

#[derive(Clone, Copy, Debug)]
pub struct ClipPhase<'a> {
    pub clip: &'a AnimationClip,
    pub normalized_phase: f64,
}
#[derive(Clone, Copy, Debug)]
pub enum PoseBlendSource<'a> {
    Clip(ClipPhase<'a>),
    /// An interrupted fade holds a captured pose; it has no single clip phase.
    FrozenPose(&'a std::sync::Arc<super::Pose>),
}
#[derive(Clone, Copy, Debug)]
pub struct PoseBlendPhases<'a> {
    pub target: ClipPhase<'a>,
    pub source: Option<PoseBlendSource<'a>>,
    pub target_weight: f32,
}
/// Source phase travel is bounded by the portion of this tick still in the fade.
#[derive(Clone, Copy, Debug)]
pub struct SourcePhaseInterval<'a> {
    pub clip: &'a std::sync::Arc<AnimationClip>,
    pub phase: super::AnimationPhaseInterval,
    pub active_tick_fraction: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct FrozenSourceTick<'a> {
    pub snapshot: &'a std::sync::Arc<super::Pose>,
    pub active_tick_fraction: f64,
}
impl Animator {
    pub fn frozen_source_tick(
        &self,
        dt: f32,
    ) -> Result<Option<FrozenSourceTick<'_>>, super::AnimationError> {
        self.phase_interval(dt)?;
        let Some(transition) = &self.transition else {
            return Ok(None);
        };
        let Some(snapshot) = &transition.source_pose else {
            return Ok(None);
        };
        let dt = f64::from(dt);
        let active = dt.min((transition.duration - transition.elapsed).max(0.));
        Ok(Some(FrozenSourceTick {
            snapshot,
            active_tick_fraction: if dt == 0. { 0. } else { active / dt },
        }))
    }

    /// Predicts the live source interval without advancing either clock.
    /// Frozen source poses have no clip interval. A completing fade exposes only
    /// its active prefix of the tick; the target continues through the full tick.
    pub fn source_phase_interval(
        &self,
        dt: f32,
    ) -> Result<Option<SourcePhaseInterval<'_>>, super::AnimationError> {
        self.phase_interval(dt)?;
        let Some(transition) = &self.transition else {
            return Ok(None);
        };
        if transition.source_pose.is_some() {
            return Ok(None);
        }
        let dt = f64::from(dt);
        let active = dt.min((transition.duration - transition.elapsed).max(0.));
        let duration = f64::from(transition.source.duration);
        let start = transition.source.phase(transition.source_time);
        let end = start + active * f64::from(self.speed);
        let looping = transition.source.playback == super::Playback::Loop;
        Ok(Some(SourcePhaseInterval {
            clip: &transition.source,
            phase: super::AnimationPhaseInterval {
                start: start / duration,
                end: if looping {
                    end / duration
                } else {
                    end.min(duration) / duration
                },
                looping,
            },
            active_tick_fraction: if dt == 0. { 0. } else { active / dt },
        }))
    }

    /// Sampling contacts from this state must preserve a separate contact snapshot
    /// for FrozenPose. Assigning that source a clip phase would break continuity.
    pub fn pose_blend_phases(&self) -> PoseBlendPhases<'_> {
        let target = ClipPhase {
            clip: &self.current,
            normalized_phase: self.normalized_phase(),
        };
        match &self.transition {
            None => PoseBlendPhases {
                target,
                source: None,
                target_weight: 1.,
            },
            Some(transition) => PoseBlendPhases {
                target,
                source: Some(if let Some(snapshot) = &transition.source_pose {
                    PoseBlendSource::FrozenPose(snapshot)
                } else {
                    PoseBlendSource::Clip(ClipPhase {
                        clip: &transition.source,
                        normalized_phase: transition.source.phase(transition.source_time)
                            / f64::from(transition.source.duration),
                    })
                }),
                target_weight: (transition.elapsed / transition.duration) as f32,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, JointTrack, Playback, Skeleton, Transform};
    use std::sync::Arc;
    #[test]
    fn blend_phases_share_pose_clocks_and_identify_interrupted_snapshots() {
        let rig = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: glam::Mat4::IDENTITY,
        }])
        .unwrap();
        let clip = |name| {
            Arc::new(
                AnimationClip::new(name, 2., Playback::Loop, vec![JointTrack::default()], &rig)
                    .unwrap(),
            )
        };
        let mut animator = Animator::new(clip("walk"));
        animator.advance(&rig, 0.5).unwrap();
        animator.transition_to(clip("jump"), 1.).unwrap();
        let frame = animator.advance(&rig, 0.25).unwrap();
        let phases = animator.pose_blend_phases();
        assert_eq!(phases.target.clip.name(), "jump");
        assert_eq!(phases.target.normalized_phase, 0.125);
        assert_eq!(phases.target_weight, frame.transition_weight);
        let Some(PoseBlendSource::Clip(source)) = phases.source else {
            panic!("live source")
        };
        assert_eq!(source.clip.name(), "walk");
        assert_eq!(source.normalized_phase, 0.375);
        animator.transition_to(clip("land"), 0.5).unwrap();
        assert!(matches!(
            animator.pose_blend_phases().source,
            Some(PoseBlendSource::FrozenPose(_))
        ));
        let frame = animator.advance(&rig, 0.25).unwrap();
        assert_eq!(
            animator.pose_blend_phases().target_weight,
            frame.transition_weight
        );
        animator.advance(&rig, 0.25).unwrap();
        assert!(animator.pose_blend_phases().source.is_none());
        animator.transition_to(clip("idle"), 0.).unwrap();
        assert!(animator.pose_blend_phases().source.is_none());
        assert_eq!(animator.pose_blend_phases().target.normalized_phase, 0.);
    }
    #[test]
    fn source_interval_clips_completion_tail_and_preserves_loop_pause_and_frozen_semantics() {
        let rig = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: glam::Mat4::IDENTITY,
        }])
        .unwrap();
        let clip = |name| {
            Arc::new(
                AnimationClip::new(name, 1., Playback::Loop, vec![JointTrack::default()], &rig)
                    .unwrap(),
            )
        };
        let mut animator = Animator::new(clip("walk"));
        animator.advance(&rig, 0.75).unwrap();
        animator.transition_to(clip("jump"), 0.25).unwrap();
        animator.set_speed(2.).unwrap();
        let source = animator.source_phase_interval(0.5).unwrap().unwrap();
        assert_eq!(source.clip.name(), "walk");
        assert_eq!(source.phase.start, 0.75);
        assert_eq!(source.phase.end, 1.25);
        assert_eq!(source.active_tick_fraction, 0.5);
        assert_eq!(animator.phase_interval(0.5).unwrap().end, 1.);
        assert_eq!(
            animator
                .source_phase_interval(0.)
                .unwrap()
                .unwrap()
                .active_tick_fraction,
            0.
        );
        animator.set_speed(0.).unwrap();
        let paused = animator.source_phase_interval(0.1).unwrap().unwrap();
        assert_eq!(paused.phase.start, paused.phase.end);
        assert!(animator.source_phase_interval(f32::NAN).is_err());
        animator.transition_to(clip("land"), 0.5).unwrap();
        assert!(animator.source_phase_interval(0.1).unwrap().is_none());
        animator.advance(&rig, 0.5).unwrap();
        assert!(animator.source_phase_interval(0.1).unwrap().is_none());
    }
}
