//! Model playback participant. Owner/asset identity and final App publication are outer contracts.
use super::*;
use serde::{Deserialize, Serialize};
use voxy_animation::{
    AnimationClip, AnimationPhaseInterval, AnimatorCheckpoint, PoseCheckpoint, PropertyClip,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interval {
    start: u64,
    end: u64,
    looping: bool,
}
impl Interval {
    fn capture(p: AnimationPhaseInterval) -> Self {
        Self {
            start: p.start.to_bits(),
            end: p.end.to_bits(),
            looping: p.looping,
        }
    }
    fn prepare(&self) -> Result<AnimationPhaseInterval, EditorError> {
        let start = f64::from_bits(self.start);
        let end = f64::from_bits(self.end);
        if !start.is_finite()
            || !end.is_finite()
            || !(0. ..=1.).contains(&start)
            || end < start
            || end > 1. && !self.looping
        {
            return Err("invalid saved animation contact interval".into());
        }
        Ok(AnimationPhaseInterval {
            start,
            end,
            looping: self.looping,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    name: String,
    phase: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    index: usize,
    #[serde(default)]
    clip: String,
    events: Vec<Event>,
    windows: Vec<(String, u64, u64)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    clip: String,
    interval: Interval,
    fraction: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelPlaybackCheckpoint {
    version: u32,
    animator: Option<AnimatorCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root_motion: Option<voxy_animation::RootRigidMotionCheckpoint>,
    tracks: Vec<Track>,
    pending: Vec<Event>,
    root_motion_joint: u16,
    contact: Option<Interval>,
    source: Option<Source>,
    frozen: Option<(PoseCheckpoint, u64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    frozen_key: Option<(String, u64)>,
}
fn path_limits(max_bytes: usize) -> voxy_animation::RootRigidPathCheckpointLimits {
    voxy_animation::RootRigidPathCheckpointLimits {
        max_bytes,
        max_nodes: 1,
        max_spans: voxy_animation::MAX_ROOT_ROTATION_SPANS,
        max_work_spans: voxy_animation::MAX_ROOT_ROTATION_SPANS,
    }
}
#[allow(dead_code)]
impl ModelPlayback {
    pub(crate) fn checkpoint(
        &self,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<ModelPlaybackCheckpoint, EditorError> {
        let animator = self
            .animator
            .as_ref()
            .map(|a| {
                a.checkpoint(&identify, max_bytes)
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        let source = self
            .source_contact_interval
            .as_ref()
            .map(|s| {
                let key = identify(&s.clip, None)
                    .filter(|k| !k.is_empty())
                    .ok_or("unresolved saved source contact clip")?;
                Ok::<_, EditorError>(Source {
                    clip: key,
                    interval: Interval::capture(s.phase),
                    fraction: s.active_tick_fraction.to_bits(),
                })
            })
            .transpose()?;
        let saved = ModelPlaybackCheckpoint {
            version: 2,
            root_motion: self
                .root_motion_receipt
                .as_ref()
                .map(|r| {
                    r.checkpoint(|c| identify(c, None), path_limits(max_bytes))
                        .map_err(|e| e.to_string())
                })
                .transpose()?,
            frozen_key: None,
            animator,
            tracks: self
                .event_tracks
                .iter()
                .map(|(&index, t)| {
                    Ok::<_, EditorError>(Track {
                        index,
                        clip: identify(t.clip(), None)
                            .filter(|k| !k.is_empty())
                            .ok_or("unidentified event clip revision")?,
                        events: t
                            .events()
                            .iter()
                            .map(|e| Event {
                                name: e.name.to_string(),
                                phase: e.phase.to_bits(),
                            })
                            .collect(),
                        windows: t
                            .windows()
                            .iter()
                            .map(|w| (w.name.to_string(), w.start.to_bits(), w.end.to_bits()))
                            .collect(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            pending: self
                .pending_events
                .iter()
                .map(|e| Event {
                    name: e.name.to_string(),
                    phase: e.phase.to_bits(),
                })
                .collect(),
            root_motion_joint: self.root_motion_joint,
            contact: self.contact_interval.map(Interval::capture),
            source,
            frozen: self
                .frozen_source_tick
                .as_ref()
                .map(|(p, f)| (p.checkpoint(), f.to_bits())),
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl ModelPlayback {
    pub(crate) fn checkpoint_with_snapshots(
        &self,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        identify_pose: impl Fn(&Arc<voxy_animation::Pose>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<ModelPlaybackCheckpoint, EditorError> {
        let mut saved = self.checkpoint(&identify, usize::MAX)?;
        saved.animator = self
            .animator
            .as_ref()
            .map(|a| {
                a.checkpoint_with_snapshots(&identify, &identify_pose, usize::MAX)
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        if let Some((pose, fraction)) = &self.frozen_source_tick {
            let key = identify_pose(pose)
                .filter(|k| !k.is_empty())
                .ok_or("missing model frozen snapshot identity")?;
            saved.frozen = None;
            saved.frozen_key = Some((key, fraction.to_bits()));
        }
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}

#[allow(dead_code)]
impl ModelPlaybackCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        if self.version != 2 {
            return Err("unsupported model playback checkpoint".into());
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err("model playback checkpoint exceeds byte budget".into());
        }
        Ok(text)
    }
    pub(crate) fn from_json(text: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if text.len() > max_bytes {
            return Err("model playback checkpoint exceeds byte budget".into());
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 2 {
            return Err("unsupported model playback checkpoint".into());
        }
        Ok(saved)
    }
    pub(crate) fn prepare(
        &self,
        model: Arc<ModelAsset>,
        resolve: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        max_bytes: usize,
    ) -> Result<ModelPlayback, EditorError> {
        self.prepare_with_snapshots(model, resolve, |_| None, max_bytes)
    }
    pub(crate) fn prepare_with_snapshots(
        &self,
        model: Arc<ModelAsset>,
        resolve: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
        max_bytes: usize,
    ) -> Result<ModelPlayback, EditorError> {
        self.to_json(max_bytes)?;
        if self.pending.len() > 4096
            || usize::from(self.root_motion_joint) >= model.skeleton.joints().len()
        {
            return Err("invalid saved model playback limits".into());
        }
        let animator = self
            .animator
            .as_ref()
            .map(|a| {
                a.prepare_with_snapshots(&model.skeleton, &resolve, &resolve_pose, max_bytes)
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        if animator
            .as_ref()
            .is_some_and(|a| a.root_motion_joint() != self.root_motion_joint)
        {
            return Err("saved root motion joint mismatch".into());
        }
        let mut event_tracks = std::collections::BTreeMap::new();
        for track in &self.tracks {
            if track.clip.is_empty() {
                return Err("missing saved event clip revision".into());
            }
            let clip = resolve(&track.clip)
                .ok_or("saved event clip revision is unavailable")?
                .0;
            if !clip.is_compatible_with(&model.skeleton) {
                return Err("saved event clip belongs to another rig".into());
            }
            let events = track
                .events
                .iter()
                .map(|e| voxy_animation::ClipEvent {
                    name: e.name.as_str().into(),
                    phase: f64::from_bits(e.phase),
                })
                .collect();
            let windows = track
                .windows
                .iter()
                .map(|(name, start, end)| voxy_animation::EventWindow {
                    name: name.as_str().into(),
                    start: f64::from_bits(*start),
                    end: f64::from_bits(*end),
                })
                .collect();
            let events =
                voxy_animation::ClipEvents::new(clip.clone(), events)?.with_windows(windows)?;
            if event_tracks.insert(track.index, events).is_some() {
                return Err("duplicate saved event clip".into());
            }
        }
        let pending_events = self
            .pending
            .iter()
            .map(|e| {
                let phase = f64::from_bits(e.phase);
                if e.name.is_empty() || !phase.is_finite() || phase < 0. {
                    return Err("invalid saved pending animation event");
                }
                Ok(voxy_animation::ClipEventOccurrence {
                    name: e.name.as_str().into(),
                    phase,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let fraction = |bits| {
            let value = f64::from_bits(bits);
            if !value.is_finite() || !(0. ..=1.).contains(&value) {
                return Err("invalid saved active tick fraction");
            }
            Ok(value)
        };
        let contact_interval = self.contact.as_ref().map(Interval::prepare).transpose()?;
        let source_contact_interval = self
            .source
            .as_ref()
            .map(|s| {
                let clip = resolve(&s.clip)
                    .ok_or("saved source contact clip missing")?
                    .0;
                if !clip.is_compatible_with(&model.skeleton) {
                    return Err("saved source contact rig mismatch".into());
                }
                Ok::<_, EditorError>(SourceContactInterval {
                    clip,
                    phase: s.interval.prepare()?,
                    active_tick_fraction: fraction(s.fraction)?,
                })
            })
            .transpose()?;
        let frozen_source_tick = match (&self.frozen, &self.frozen_key) {
            (Some((pose, f)), None) => {
                Some((Arc::new(pose.prepare(&model.skeleton)?), fraction(*f)?))
            }
            (None, Some((key, f))) if !key.is_empty() => {
                let pose = resolve_pose(key).ok_or("missing model frozen snapshot")?;
                pose.skin_matrices(&model.skeleton)?;
                Some((pose, fraction(*f)?))
            }
            (None, None) => None,
            _ => return Err("conflicting model frozen snapshot representations".into()),
        };
        if source_contact_interval.is_some() && frozen_source_tick.is_some() {
            return Err("conflicting saved fade contact sources".into());
        }
        if animator.is_none()
            && (contact_interval.is_some()
                || source_contact_interval.is_some()
                || frozen_source_tick.is_some()
                || !pending_events.is_empty())
        {
            return Err("bind playback has animated state".into());
        }
        let root_motion_receipt = self
            .root_motion
            .as_ref()
            .map(|r| {
                r.prepare(
                    &model.skeleton,
                    |key| resolve(key).map(|a| a.0),
                    path_limits(max_bytes),
                )
                .map(|(r, _)| r)
                .map_err(|e| e.to_string())
            })
            .transpose()?;
        if animator.is_none() && root_motion_receipt.is_some() {
            return Err("bind playback has saved root motion".into());
        }
        Ok(ModelPlayback {
            model,
            animator,
            root_motion_receipt,
            event_tracks,
            pending_events,
            root_motion_joint: self.root_motion_joint,
            contact_interval,
            source_contact_interval,
            frozen_source_tick,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Arc<ModelAsset> {
        let bytes = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        let mut asset = ModelAsset::parse(bytes, &[], voxy_model::ModelLimits::default()).unwrap();
        let clip = voxy_animation::AnimationClip::new(
            "target",
            1.,
            voxy_animation::Playback::Loop,
            vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
            &asset.skeleton,
        )
        .unwrap();
        asset.animations.push(Arc::new(clip));
        Arc::new(asset)
    }
    fn capture(playback: &ModelPlayback) -> ModelPlaybackCheckpoint {
        playback
            .checkpoint(
                |clip, _| {
                    playback
                        .model
                        .animations
                        .iter()
                        .position(|c| Arc::ptr_eq(c, clip))
                        .map(|i| format!("asset-revision/clip/{i}"))
                },
                1024 * 1024,
            )
            .unwrap()
    }
    fn prepare(
        saved: &ModelPlaybackCheckpoint,
        model: Arc<ModelAsset>,
    ) -> Result<ModelPlayback, EditorError> {
        saved.prepare(
            model.clone(),
            |key| {
                model
                    .animations
                    .iter()
                    .enumerate()
                    .find(|(i, _)| key == format!("asset-revision/clip/{i}"))
                    .map(|(_, clip)| (clip.clone(), None))
            },
            1024 * 1024,
        )
    }
    #[test]
    fn saved_pending_events_and_frozen_contact_pose_continue_and_drain_once() {
        let mut original = ModelPlayback::new(model(), ModelAnimation::default()).unwrap();
        for index in 0..2 {
            original
                .set_clip_events(
                    index,
                    vec![
                        voxy_animation::ClipEvent {
                            name: "step".into(),
                            phase: 0.1,
                        },
                        voxy_animation::ClipEvent {
                            name: "end".into(),
                            phase: 1.,
                        },
                    ],
                )
                .unwrap();
        }
        original.advance_with(0.12, |_, _| Ok(())).unwrap();
        assert!(!original.pending_events.is_empty());
        original.transition_to_clip(1, 0.5).unwrap();
        original.advance_with(0.12, |_, _| Ok(())).unwrap();
        original.transition_to_clip(0, 0.3).unwrap();
        original.advance_with(0.07, |_, _| Ok(())).unwrap();
        assert!(original.frozen_source_tick().is_some());
        let text = capture(&original).to_json(1024 * 1024).unwrap();
        let decoded = ModelPlaybackCheckpoint::from_json(&text, 1024 * 1024).unwrap();
        let mut resumed = prepare(&decoded, model()).unwrap();
        assert_eq!(capture(&resumed).to_json(1024 * 1024).unwrap(), text);
        assert!(!Arc::ptr_eq(&original.model, &resumed.model));
        let mut delivered = 0;
        for tick in 0..240 {
            if tick % 7 == 0 {
                let a = original.take_events();
                let b = resumed.take_events();
                delivered += a.len();
                assert_eq!(a, b);
                assert!(original.take_events().is_empty());
                assert!(resumed.take_events().is_empty());
            }
            if tick == 40 {
                original.transition_to_clip(1, 0.2).unwrap();
                resumed.transition_to_clip(1, 0.2).unwrap();
            }
            let dt = [0.003, 0.021, 0.037][tick % 3];
            let a = original
                .advance_with_motion_wall(dt, false, [false; 3], |_, f, _| Ok(f.clone()))
                .unwrap();
            let b = resumed
                .advance_with_motion_wall(dt, false, [false; 3], |_, f, _| Ok(f.clone()))
                .unwrap();
            assert_eq!(a.pose, b.pose);
            assert_eq!(a.skin_matrices, b.skin_matrices);
            assert_eq!(
                a.root_motion.to_array().map(f32::to_bits),
                b.root_motion.to_array().map(f32::to_bits)
            );
            assert_eq!(a.transition_weight.to_bits(), b.transition_weight.to_bits());
            assert_eq!(
                capture(&original).to_json(1024 * 1024).unwrap(),
                capture(&resumed).to_json(1024 * 1024).unwrap()
            );
        }
        assert!(delivered > 3);
    }
    #[test]
    fn ordinary_source_interval_and_bind_playback_restore_and_bad_data_is_rejected() {
        let asset = model();
        let mut original = ModelPlayback::new(asset.clone(), ModelAnimation::default()).unwrap();
        original.advance_with(0.1, |_, _| Ok(())).unwrap();
        original.transition_to_clip(1, 0.5).unwrap();
        original.advance_with(0.1, |_, _| Ok(())).unwrap();
        assert!(original.source_contact_interval().is_some());
        let saved = capture(&original);
        let text = saved.to_json(1024 * 1024).unwrap();
        let resumed = prepare(&saved, model()).unwrap();
        assert_eq!(capture(&resumed).to_json(1024 * 1024).unwrap(), text);
        let mut corrupt = saved.clone();
        corrupt.source.as_mut().unwrap().fraction = f64::NAN.to_bits();
        assert!(prepare(&corrupt, model()).is_err());
        let mut corrupt = saved.clone();
        corrupt.tracks.push(Track {
            index: usize::MAX,
            clip: "missing-clip".into(),
            events: vec![],
            windows: vec![],
        });
        assert!(prepare(&corrupt, model()).is_err());
        assert!(ModelPlaybackCheckpoint::from_json(&text, 1).is_err());
        assert_eq!(capture(&original).to_json(1024 * 1024).unwrap(), text);
        let bind = ModelPlayback::new(
            asset,
            ModelAnimation {
                clip: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            prepare(&capture(&bind), model())
                .unwrap()
                .animator
                .is_none()
        );
    }
}

#[cfg(test)]
mod receipt_tests {
    use super::*;
    fn model() -> Arc<ModelAsset> {
        Arc::new(
            ModelAsset::parse(
                include_bytes!("../../voxy_render/examples/assets/root-pivot-turn.glb"),
                &[],
                voxy_model::ModelLimits::default(),
            )
            .unwrap(),
        )
    }
    fn capture(playback: &ModelPlayback) -> ModelPlaybackCheckpoint {
        playback
            .checkpoint(
                |clip, _| {
                    playback
                        .model
                        .animations
                        .iter()
                        .position(|c| Arc::ptr_eq(c, clip))
                        .map(|i| format!("clip-{i}"))
                },
                2 * 1024 * 1024,
            )
            .unwrap()
    }
    fn restore(
        saved: &ModelPlaybackCheckpoint,
        model: Arc<ModelAsset>,
    ) -> Result<ModelPlayback, EditorError> {
        saved.prepare(
            model.clone(),
            |key| {
                model
                    .animations
                    .iter()
                    .enumerate()
                    .find(|(i, _)| key == format!("clip-{i}"))
                    .map(|(_, c)| (c.clone(), None))
            },
            2 * 1024 * 1024,
        )
    }
    #[test]
    fn accepted_root_receipt_saves_exact_clocks_replays_queue_and_retains_failed_tick() {
        let mut original = ModelPlayback::new(model(), ModelAnimation::default()).unwrap();
        let actual = original
            .advance_with_motion_wall(0.17300000000000004, true, [true; 3], |_, _, p| {
                Ok(p.unwrap().clone())
            })
            .unwrap();
        let receipt = original.root_motion_receipt().unwrap();
        let limits = path_limits(2 * 1024 * 1024);
        let saved_path = receipt
            .checkpoint_path(&actual, |_| Some("clip-0".into()), limits)
            .unwrap();
        assert_eq!(
            saved_path.to_json(limits.max_bytes).unwrap(),
            receipt
                .checkpoint(|_| Some("clip-0".into()), limits)
                .unwrap()
                .to_json(limits.max_bytes)
                .unwrap()
        );
        let before = capture(&original).to_json(limits.max_bytes).unwrap();
        assert!(
            original
                .advance_with_motion_wall(0.021, true, [true; 3], |_, _, _| Err::<(), _>(
                    "late physical rejection".into()
                ))
                .is_err()
        );
        assert_eq!(
            capture(&original).to_json(limits.max_bytes).unwrap(),
            before
        );
        let saved = ModelPlaybackCheckpoint::from_json(&before, limits.max_bytes).unwrap();
        let mut resumed = restore(&saved, model()).unwrap();
        assert_eq!(capture(&resumed).to_json(limits.max_bytes).unwrap(), before);
        for frame in 0..120 {
            let dt = [
                0.0030000000000000005,
                0.021000000000000005,
                0.03700000000000001,
            ][frame % 3];
            let a = original
                .advance_with_motion_wall(dt, true, [true; 3], |_, f, p| {
                    Ok((f.clone(), p.unwrap().clone()))
                })
                .unwrap();
            let b = resumed
                .advance_with_motion_wall(dt, true, [true; 3], |_, f, p| {
                    Ok((f.clone(), p.unwrap().clone()))
                })
                .unwrap();
            assert_eq!(a.0.pose, b.0.pose);
            assert_eq!(a.0.skin_matrices, b.0.skin_matrices);
            let a = original
                .root_motion_receipt()
                .unwrap()
                .checkpoint_path(&a.1, |_| Some("clip-0".into()), limits)
                .unwrap();
            let b = resumed
                .root_motion_receipt()
                .unwrap()
                .checkpoint_path(&b.1, |_| Some("clip-0".into()), limits)
                .unwrap();
            assert_eq!(
                a.to_json(limits.max_bytes).unwrap(),
                b.to_json(limits.max_bytes).unwrap()
            );
            assert_eq!(
                capture(&original).to_json(limits.max_bytes).unwrap(),
                capture(&resumed).to_json(limits.max_bytes).unwrap()
            );
        }
        original
            .advance_with_motion_wall(0.01, false, [false; 3], |_, _, _| Ok(()))
            .unwrap();
        assert!(original.root_motion_receipt().is_none());
    }
    #[test]
    fn changed_receipt_program_or_missing_source_rejects_candidate() {
        let mut original = ModelPlayback::new(model(), ModelAnimation::default()).unwrap();
        original
            .advance_with_motion_wall(0.173, true, [true; 3], |_, _, _| Ok(()))
            .unwrap();
        let before = capture(&original).to_json(2 * 1024 * 1024).unwrap();
        let saved = capture(&original);
        assert!(saved.prepare(model(), |_| None, 2 * 1024 * 1024).is_err());
        let mut wire: serde_json::Value = serde_json::from_str(&before).unwrap();
        wire["root_motion"]["end"] = 0.4f64.to_bits().into();
        let bad = ModelPlaybackCheckpoint::from_json(&wire.to_string(), 2 * 1024 * 1024).unwrap();
        assert!(restore(&bad, model()).is_err());
        assert_eq!(capture(&original).to_json(2 * 1024 * 1024).unwrap(), before);
    }
}

#[cfg(test)]
mod event_revision_tests {
    use super::*;
    #[test]
    fn rebind_keeps_old_event_source_revision_in_checkpoint() {
        let original = Arc::new(
            ModelAsset::parse(
                include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb"),
                &[],
                voxy_model::ModelLimits::default(),
            )
            .unwrap(),
        );
        let old_clip = original.animations[0].clone();
        let mut replacement = (*original).clone();
        replacement.animations = vec![Arc::new(
            voxy_animation::AnimationClip::new(
                "replacement",
                1.,
                voxy_animation::Playback::Loop,
                vec![voxy_animation::JointTrack::default(); replacement.skeleton.joints().len()],
                &replacement.skeleton,
            )
            .unwrap(),
        )];
        let replacement = Arc::new(replacement);
        let new_clip = replacement.animations[0].clone();
        let mut source = ModelPlayback::new(original, ModelAnimation::default()).unwrap();
        source
            .set_clip_events(
                0,
                vec![voxy_animation::ClipEvent {
                    name: "retained-marker".into(),
                    phase: 0.2,
                }],
            )
            .unwrap();
        source.advance_with(0.1, |_, _| Ok(())).unwrap();
        source.rebind(replacement.clone()).unwrap();
        let identify = |clip: &Arc<AnimationClip>, _: Option<&Arc<PropertyClip>>| {
            if Arc::ptr_eq(clip, &old_clip) {
                Some("old-clip".into())
            } else if Arc::ptr_eq(clip, &new_clip) {
                Some("new-clip".into())
            } else {
                None
            }
        };
        let saved = source.checkpoint(identify, 65536).unwrap();
        let text = saved.to_json(65536).unwrap();
        let decoded = ModelPlaybackCheckpoint::from_json(&text, 65536).unwrap();
        let mut restored = decoded
            .prepare(
                replacement,
                |key| match key {
                    "old-clip" => Some((old_clip.clone(), None)),
                    "new-clip" => Some((new_clip.clone(), None)),
                    _ => None,
                },
                65536,
            )
            .unwrap();
        source.advance_with(0.2, |_, _| Ok(())).unwrap();
        restored.advance_with(0.2, |_, _| Ok(())).unwrap();
        let events = source.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name.as_ref(), "retained-marker");
        assert_eq!(events, restored.take_events());
        let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
        legacy["version"] = 1.into();
        assert!(ModelPlaybackCheckpoint::from_json(&legacy.to_string(), 65536).is_err());
    }
}
