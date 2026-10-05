//! Stateless event preview; publish only after the matching animation tick commits.
use super::{AnimationClip, Animator};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq)]
pub struct ClipEvent {
    pub name: Arc<str>,
    /// Normalized authored phase, inclusive endpoints 0 and 1.
    pub phase: f64,
}
#[derive(Clone, Debug)]
pub struct ClipEvents {
    clip: Arc<AnimationClip>,
    events: Vec<ClipEvent>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClipEventOccurrence {
    pub name: Arc<str>,
    /// Unwrapped normalized phase relative to this target tick's cycle.
    pub phase: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum EventTickError {
    Events(&'static str),
    Animation(super::AnimationError),
}
impl std::fmt::Display for EventTickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Events(message) => write!(f, "{message}"),
            Self::Animation(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for EventTickError {}
impl Animator {
    /// Publishes target events only together with a successfully accepted frame.
    /// Budget or animation failure preserves the clock and transition state.
    /// Caller callbacks run after this result; crossfade source events are omitted.
    pub fn advance_wall_with_events(
        &mut self,
        skeleton: &super::Skeleton,
        dt: f64,
        track: &ClipEvents,
        budget: usize,
    ) -> Result<(super::AnimatorFrame, Vec<ClipEventOccurrence>), EventTickError> {
        let events = track
            .preview_target_tick(self, dt, budget)
            .map_err(EventTickError::Events)?;
        let frame = self
            .advance_wall(skeleton, dt)
            .map_err(EventTickError::Animation)?;
        Ok((frame, events))
    }
}
impl ClipEvents {
    pub fn new(clip: Arc<AnimationClip>, mut events: Vec<ClipEvent>) -> Result<Self, &'static str> {
        if events
            .iter()
            .any(|e| e.name.is_empty() || !e.phase.is_finite() || !(0.0..=1.0).contains(&e.phase))
        {
            return Err("invalid clip event");
        }
        events.sort_by(|a, b| a.phase.total_cmp(&b.phase));
        Ok(Self { clip, events })
    }
    pub fn matches_target(&self, animator: &Animator) -> bool {
        Arc::ptr_eq(&self.clip, &animator.current)
            || self.clip.has_same_authored_animation(&animator.current)
    }
    /// Collects target-clip events in (start,end], without changing clocks.
    /// Paused ticks emit none. Loop seams preserve end events before next-cycle
    /// start events. Source events in a crossfade need a separate source policy.
    pub fn preview_target_tick(
        &self,
        animator: &Animator,
        dt: f64,
        budget: usize,
    ) -> Result<Vec<ClipEventOccurrence>, &'static str> {
        if !self.matches_target(animator) {
            return Err("foreign animation event clip");
        }
        let interval = animator
            .phase_interval_wall(dt)
            .map_err(|_| "invalid event tick")?;
        if self.events.is_empty() || interval.end == interval.start {
            return Ok(Vec::new());
        }
        let first = interval.start.floor();
        let last = if interval.looping {
            interval.end.floor()
        } else {
            0.
        };
        if !interval.end.is_finite() || last - first > budget as f64 + 1. {
            return Err("animation event budget");
        }
        let mut found = Vec::new();
        for cycle in 0..=((last - first) as usize) {
            let base = first + cycle as f64;
            for event in &self.events {
                let phase = base + event.phase;
                if phase > interval.start && phase <= interval.end {
                    if found.len() == budget {
                        return Err("animation event budget");
                    }
                    found.push(ClipEventOccurrence {
                        name: event.name.clone(),
                        phase,
                    });
                }
            }
        }
        Ok(found)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, Playback, Skeleton, Transform};
    use glam::Mat4;
    fn animator() -> (Arc<AnimationClip>, Animator) {
        let rig = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let clip = Arc::new(
            AnimationClip::new(
                "walk",
                1.,
                Playback::Loop,
                vec![crate::JointTrack::default()],
                &rig,
            )
            .unwrap(),
        );
        (clip.clone(), Animator::new(clip))
    }
    #[test]
    fn accepted_frame_delivers_once_and_failures_keep_event_clock() {
        let (clip, mut player) = animator();
        let rig = Skeleton {
            joints: clip.rig.clone(),
        };
        let track = ClipEvents::new(
            clip,
            vec![ClipEvent {
                name: "step".into(),
                phase: 0.25,
            }],
        )
        .unwrap();
        assert!(matches!(
            player.advance_wall_with_events(&rig, 0.25, &track, 0),
            Err(EventTickError::Events(_))
        ));
        assert_eq!(player.normalized_phase(), 0.);
        let (_, events) = player
            .advance_wall_with_events(&rig, 0.25, &track, 1)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            player
                .advance_wall_with_events(&rig, 0.1, &track, 1)
                .unwrap()
                .1
                .is_empty()
        );
    }
    #[test]
    fn late_singular_pose_discards_previewed_events_and_tick_recovers() {
        let (base, _) = animator();
        let rig = Skeleton {
            joints: base.rig.clone(),
        };
        let clip = Arc::new(
            AnimationClip::new(
                "scale",
                1.,
                Playback::Clamp,
                vec![crate::JointTrack {
                    scales: vec![
                        crate::Vec3Key {
                            time: 0.,
                            value: glam::Vec3::ONE,
                        },
                        crate::Vec3Key {
                            time: 1.,
                            value: -glam::Vec3::ONE,
                        },
                    ],
                    ..Default::default()
                }],
                &rig,
            )
            .unwrap(),
        );
        let mut player = Animator::new(clip.clone());
        let track = ClipEvents::new(
            clip,
            vec![ClipEvent {
                name: "step".into(),
                phase: 0.25,
            }],
        )
        .unwrap();
        assert_eq!(track.preview_target_tick(&player, 0.5, 1).unwrap().len(), 1);
        assert!(matches!(
            player.advance_wall_with_events(&rig, 0.5, &track, 1),
            Err(EventTickError::Animation(_))
        ));
        assert_eq!(player.normalized_phase(), 0.);
        assert_eq!(
            player
                .advance_wall_with_events(&rig, 0.25, &track, 1)
                .unwrap()
                .1
                .len(),
            1
        );
    }
    #[test]
    fn malformed_and_foreign_tracks_reject_and_nonloop_end_emits_once() {
        let (clip, animator) = animator();
        assert!(
            ClipEvents::new(
                clip.clone(),
                vec![ClipEvent {
                    name: "bad".into(),
                    phase: f64::NAN
                }]
            )
            .is_err()
        );
        let mut foreign = (*clip).clone();
        foreign.name = "other".into();
        let foreign = ClipEvents::new(Arc::new(foreign), vec![]).unwrap();
        assert!(foreign.preview_target_tick(&animator, 0., 0).is_err());
        let mut once = (*clip).clone();
        once.playback = Playback::Clamp;
        let once = Arc::new(once);
        let track = ClipEvents::new(
            once.clone(),
            vec![ClipEvent {
                name: "end".into(),
                phase: 1.,
            }],
        )
        .unwrap();
        let mut player = Animator::new(once);
        assert_eq!(track.preview_target_tick(&player, 1., 1).unwrap().len(), 1);
        let rig = crate::Skeleton {
            joints: player.current.rig.clone(),
        };
        player.advance(&rig, 1.).unwrap();
        assert!(
            track
                .preview_target_tick(&player, 1., 1)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn preview_orders_seams_preserves_clock_and_bounds_work() {
        let (clip, mut animator) = animator();
        let track = ClipEvents::new(
            clip,
            vec![
                ClipEvent {
                    name: "start".into(),
                    phase: 0.,
                },
                ClipEvent {
                    name: "step".into(),
                    phase: 0.5,
                },
                ClipEvent {
                    name: "end".into(),
                    phase: 1.,
                },
            ],
        )
        .unwrap();
        animator.set_speed(2.).unwrap();
        let events = track.preview_target_tick(&animator, 1., 10).unwrap();
        assert_eq!(
            events.iter().map(|e| e.name.as_ref()).collect::<Vec<_>>(),
            ["step", "end", "start", "step", "end", "start"]
        );
        assert_eq!(animator.normalized_phase(), 0.);
        assert!(track.preview_target_tick(&animator, 1., 5).is_err());
        animator.set_speed(0.).unwrap();
        assert!(
            track
                .preview_target_tick(&animator, 1., 0)
                .unwrap()
                .is_empty()
        );
        assert!(track.preview_target_tick(&animator, f64::NAN, 10).is_err());
    }
}
