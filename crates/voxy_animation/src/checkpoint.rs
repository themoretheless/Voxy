//! Exact playback state with externally resolved immutable clip revision identities.
#[path = "frame_checkpoint.rs"]
mod frame_checkpoint;
use super::*;
pub use frame_checkpoint::AnimatorFrameCheckpoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AnimatorCheckpointError {
    #[error("invalid animator checkpoint state or unresolved resource revision")]
    State,
    #[error("animator checkpoint exceeds byte budget")]
    Capacity,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Animation(#[from] AnimationError),
}

/// Exact local pose data. The enclosing resource identity must select its skeleton.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PoseCheckpoint(Vec<[u32; 10]>);
impl Pose {
    #[must_use]
    pub fn checkpoint(&self) -> PoseCheckpoint {
        PoseCheckpoint(
            self.local
                .iter()
                .map(|t| {
                    let a = t.translation.to_array();
                    let r = t.rotation.to_array();
                    let s = t.scale.to_array();
                    [a[0], a[1], a[2], r[0], r[1], r[2], r[3], s[0], s[1], s[2]].map(f32::to_bits)
                })
                .collect(),
        )
    }
}
impl PoseCheckpoint {
    /// Rebuild against the enclosing checkpoint's verified skeleton revision.
    /// # Errors
    /// Rejects joint counts, invalid TRS or an overflowing composed palette.
    pub fn prepare(&self, skeleton: &Skeleton) -> Result<Pose, AnimationError> {
        if self.0.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        let local = self
            .0
            .iter()
            .map(|bits| {
                let v = bits.map(f32::from_bits);
                Transform {
                    translation: Vec3::from_array([v[0], v[1], v[2]]),
                    rotation: Quat::from_array([v[3], v[4], v[5], v[6]]),
                    scale: Vec3::from_array([v[7], v[8], v[9]]),
                }
            })
            .collect::<Vec<_>>();
        let pose = Pose {
            rig: skeleton.joints.clone(),
            local,
        };
        pose.skin_matrices(skeleton)?;
        Ok(pose)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Value {
    Float(u32),
    Vec3([u32; 3]),
    Color([u32; 4]),
    Bool(bool),
}
type Properties = Vec<(String, Value)>;
fn encode(set: &PropertySet) -> Properties {
    set.entries()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                match value {
                    PropertyValue::Float(v) => Value::Float(v.to_bits()),
                    PropertyValue::Vec3(v) => Value::Vec3(v.to_array().map(f32::to_bits)),
                    PropertyValue::Color(v) => Value::Color(v.map(f32::to_bits)),
                    PropertyValue::Bool(v) => Value::Bool(*v),
                },
            )
        })
        .collect()
}
fn decode(set: &Properties) -> Result<PropertySet, AnimationError> {
    PropertySet::new(
        set.iter()
            .map(|(name, value)| {
                (
                    Arc::from(name.as_str()),
                    match value {
                        Value::Float(v) => PropertyValue::Float(f32::from_bits(*v)),
                        Value::Vec3(v) => {
                            PropertyValue::Vec3(Vec3::from_array(v.map(f32::from_bits)))
                        }
                        Value::Color(v) => PropertyValue::Color(v.map(f32::from_bits)),
                        Value::Bool(v) => PropertyValue::Bool(*v),
                    },
                )
            })
            .collect(),
    )
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fade {
    source: String,
    time: u64,
    elapsed: u64,
    duration: u64,
    pose: Option<PoseCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pose_key: Option<String>,
    properties: Option<Properties>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    key: String,
    defaults: Properties,
}
/// The enclosing game must verify model/project revision compatibility. Keys refer
/// to exact skeletal AND property clip revisions, never merely a display name.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimatorCheckpoint {
    version: u32,
    current: String,
    time: u64,
    speed: u32,
    motion_joint: u16,
    fade: Option<Fade>,
    defaults: Option<Properties>,
    bindings: Vec<Binding>,
    accepted: Properties,
}
impl Animator {
    /// Capture clocks, interrupted-fade snapshots and all property binding revisions.
    /// # Errors
    /// Rejects missing/ambiguous revision keys and oversized output.
    pub fn checkpoint(
        &self,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<AnimatorCheckpoint, AnimatorCheckpointError> {
        let identities =
            std::cell::RefCell::new(std::collections::HashMap::<String, Arc<AnimationClip>>::new());
        let key = |clip: &Arc<AnimationClip>| {
            let properties = self
                .property_bindings
                .as_ref()
                .and_then(|b| b.clips.iter().find(|b| Arc::ptr_eq(&b.source, clip)))
                .map(|b| &b.properties);
            let key = identify(clip, properties)
                .filter(|s| !s.is_empty())
                .ok_or(AnimatorCheckpointError::State)?;
            if identities
                .borrow_mut()
                .insert(key.clone(), clip.clone())
                .is_some_and(|old| !Arc::ptr_eq(&old, clip))
            {
                return Err(AnimatorCheckpointError::State);
            }
            Ok(key)
        };
        let bindings = self
            .property_bindings
            .as_ref()
            .map(|b| {
                b.clips
                    .iter()
                    .map(|b| {
                        Ok(Binding {
                            key: key(&b.source)?,
                            defaults: encode(&b.defaults),
                        })
                    })
                    .collect::<Result<Vec<_>, AnimatorCheckpointError>>()
            })
            .transpose()?
            .unwrap_or_default();
        let mut keys = std::collections::HashSet::new();
        if bindings.iter().any(|b| !keys.insert(&b.key)) {
            return Err(AnimatorCheckpointError::State);
        }
        let fade = self
            .transition
            .as_ref()
            .map(|t| {
                Ok::<_, AnimatorCheckpointError>(Fade {
                    source: key(&t.source)?,
                    time: t.source_time.to_bits(),
                    elapsed: t.elapsed.to_bits(),
                    duration: t.duration.to_bits(),
                    pose: t.source_pose.as_ref().map(|p| p.checkpoint()),
                    pose_key: None,
                    properties: t.source_properties.as_ref().map(|p| encode(p)),
                })
            })
            .transpose()?;
        let saved = AnimatorCheckpoint {
            version: 1,
            current: key(&self.current)?,
            time: self.time.to_bits(),
            speed: self.speed.to_bits(),
            motion_joint: self.motion_joint,
            fade,
            defaults: self.property_bindings.as_ref().map(|b| encode(&b.defaults)),
            bindings,
            accepted: encode(&self.accepted_properties),
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl Animator {
    /// Capture frozen sources by shared archive ID rather than duplicating their pose.
    /// # Errors
    /// Rejects missing snapshot identities, unsupported revisions and byte budgets.
    pub fn checkpoint_with_snapshots(
        &self,
        identify: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        identify_pose: impl Fn(&Arc<Pose>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<AnimatorCheckpoint, AnimatorCheckpointError> {
        let mut saved = self.checkpoint(identify, usize::MAX)?;
        if let Some(pose) = self
            .transition
            .as_ref()
            .and_then(|t| t.source_pose.as_ref())
        {
            let key = identify_pose(pose)
                .filter(|k| !k.is_empty())
                .ok_or(AnimatorCheckpointError::State)?;
            let fade = saved.fade.as_mut().ok_or(AnimatorCheckpointError::State)?;
            fade.pose = None;
            fade.pose_key = Some(key);
        }
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}

impl AnimatorCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and byte limits.
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
    /// File reads must be bounded by the caller before this operation.
    /// # Errors
    /// Rejects byte limits before parsing, unknown fields and unsupported versions.
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
    /// Prepare a new animator; never changes existing playback. The resolver must
    /// return the exact immutable revision represented by each key, consistently.
    /// # Errors
    /// Rejects missing revisions, foreign rigs, corrupt clocks/poses/properties or budgets.
    pub fn prepare(
        &self,
        skeleton: &Skeleton,
        resolve: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        max_bytes: usize,
    ) -> Result<Animator, AnimatorCheckpointError> {
        self.prepare_with_snapshots(skeleton, resolve, |_| None, max_bytes)
    }
    /// Restore frozen sources through the enclosing transaction's shared archive.
    /// # Errors
    /// Rejects missing/foreign snapshots, conflicting inline state and all ordinary preparation errors.
    pub fn prepare_with_snapshots(
        &self,
        skeleton: &Skeleton,
        resolve: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        resolve_pose: impl Fn(&str) -> Option<Arc<Pose>>,
        max_bytes: usize,
    ) -> Result<Animator, AnimatorCheckpointError> {
        self.to_json(max_bytes)?;
        if self.bindings.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimatorCheckpointError::State);
        }
        let mut assets = std::collections::HashMap::new();
        for key in std::iter::once(&self.current)
            .chain(self.fade.iter().map(|f| &f.source))
            .chain(self.bindings.iter().map(|b| &b.key))
        {
            if key.is_empty() {
                return Err(AnimatorCheckpointError::State);
            }
            if !assets.contains_key(key) {
                let asset = resolve(key).ok_or(AnimatorCheckpointError::State)?;
                if !rigs_match(&asset.0.rig, &skeleton.joints) {
                    return Err(AnimatorCheckpointError::State);
                }
                assets.insert(key.clone(), asset);
            }
        }
        let current = assets[&self.current].0.clone();
        let mut candidate = Animator::try_new(current)?;
        candidate.set_speed(f32::from_bits(self.speed))?;
        candidate.set_root_motion_joint(self.motion_joint)?;
        candidate.time = f64::from_bits(self.time);
        if !candidate.time.is_finite()
            || candidate.time < 0.
            || candidate.time > f64::from(candidate.current.duration)
        {
            return Err(AnimatorCheckpointError::State);
        }
        if let Some(defaults) = &self.defaults {
            let defaults = decode(defaults)?;
            let mut clips = Vec::new();
            let mut keys = std::collections::HashSet::new();
            for b in &self.bindings {
                if !keys.insert(&b.key) {
                    return Err(AnimatorCheckpointError::State);
                }
                let (source, properties) = &assets[&b.key];
                let properties = properties.clone().ok_or(AnimatorCheckpointError::State)?;
                let per_clip = decode(&b.defaults)?;
                property_playback::PropertyBindings::validate(
                    &per_clip,
                    &[(source.clone(), properties.clone())],
                    &skeleton.joints,
                )?;
                if per_clip.len() != defaults.len()
                    || defaults
                        .entries()
                        .iter()
                        .any(|(n, v)| per_clip.get(n).is_none_or(|x| !x.same_kind(*v)))
                {
                    return Err(AnimatorCheckpointError::State);
                }
                if clips
                    .iter()
                    .any(|b: &property_playback::BoundPropertyClip| Arc::ptr_eq(&b.source, source))
                {
                    return Err(AnimatorCheckpointError::State);
                }
                clips.push(property_playback::BoundPropertyClip {
                    source: source.clone(),
                    properties,
                    defaults: per_clip,
                });
            }
            if clips.is_empty() {
                return Err(AnimatorCheckpointError::State);
            }
            candidate.property_bindings = Some(Arc::new(property_playback::PropertyBindings {
                defaults,
                clips,
            }));
        } else if !self.bindings.is_empty() {
            return Err(AnimatorCheckpointError::State);
        }
        if let Some(f) = &self.fade {
            let source = assets[&f.source].0.clone();
            let time = f64::from_bits(f.time);
            let elapsed = f64::from_bits(f.elapsed);
            let duration = f64::from_bits(f.duration);
            if !time.is_finite()
                || time < 0.
                || time > f64::from(source.duration)
                || !elapsed.is_finite()
                || elapsed < 0.
                || !duration.is_finite()
                || duration <= 0.
                || duration > 60.
                || elapsed >= duration
            {
                return Err(AnimatorCheckpointError::State);
            }
            let pose = match (&f.pose, &f.pose_key) {
                (Some(pose), None) => Some(Arc::new(pose.prepare(skeleton)?)),
                (None, Some(key)) if !key.is_empty() => {
                    let pose = resolve_pose(key).ok_or(AnimatorCheckpointError::State)?;
                    pose.skin_matrices(skeleton)?;
                    Some(pose)
                }
                (None, None) => None,
                _ => return Err(AnimatorCheckpointError::State),
            };
            let properties = f.properties.as_ref().map(decode).transpose()?.map(Arc::new);
            if properties.is_some() != (pose.is_some() && self.defaults.is_some()) {
                return Err(AnimatorCheckpointError::State);
            }
            candidate.transition = Some(Transition {
                source_curve: source.motion_curve(self.motion_joint)?,
                source,
                source_time: time,
                elapsed,
                duration,
                source_pose: pose,
                source_properties: properties,
            });
        }
        candidate.accepted_properties = decode(&self.accepted)?;
        if encode(&candidate.sample_displayed_properties()?) != self.accepted {
            return Err(AnimatorCheckpointError::State);
        }
        // Evaluate a clone so admission cannot normalize or consume saved clocks.
        candidate.clone().advance_wall(skeleton, 0.)?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assets() -> (Skeleton, Vec<(Arc<AnimationClip>, Arc<PropertyClip>)>) {
        let skeleton = Skeleton::new(vec![Joint {
            name: Arc::from("root"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let clips = (0..3)
            .map(|i| {
                let end = (i + 1) as f32;
                let track = JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.,
                            value: Vec3::ZERO,
                        },
                        Vec3Key {
                            time: 1.,
                            value: Vec3::X * end,
                        },
                    ],
                    ..Default::default()
                };
                let clip = Arc::new(
                    AnimationClip::new(
                        format!("clip{i}"),
                        1.,
                        Playback::Loop,
                        vec![track],
                        &skeleton,
                    )
                    .unwrap(),
                );
                let properties = Arc::new(
                    PropertyClip::new(
                        format!("props{i}"),
                        1.,
                        Playback::Loop,
                        vec![
                            PropertyTrack::new(
                                "morph",
                                vec![
                                    PropertyKey {
                                        time: 0.,
                                        value: PropertyValue::Float(0.),
                                    },
                                    PropertyKey {
                                        time: 1.,
                                        value: PropertyValue::Float(end),
                                    },
                                ],
                                1.,
                            )
                            .unwrap(),
                        ],
                    )
                    .unwrap(),
                );
                (clip, properties)
            })
            .collect();
        (skeleton, clips)
    }
    fn identify(
        clips: &[(Arc<AnimationClip>, Arc<PropertyClip>)],
        clip: &Arc<AnimationClip>,
        props: Option<&Arc<PropertyClip>>,
    ) -> Option<String> {
        clips
            .iter()
            .position(|(c, p)| Arc::ptr_eq(c, clip) && props.is_none_or(|v| Arc::ptr_eq(v, p)))
            .map(|i| format!("revision-{i}"))
    }
    #[test]
    fn json_reconstructs_interrupted_fade_and_morph_with_fresh_assets_exactly() {
        let (skeleton, clips) = assets();
        let mut source = Animator::new(clips[0].0.clone());
        source
            .bind_property_clips(
                PropertySet::new(vec![(Arc::from("morph"), PropertyValue::Float(0.))]).unwrap(),
                clips.clone(),
            )
            .unwrap();
        source.set_speed(0.731).unwrap();
        source.advance_wall(&skeleton, 0.173).unwrap();
        source.transition_to(clips[1].0.clone(), 0.8).unwrap();
        source.advance_wall(&skeleton, 0.137).unwrap();
        source
            .transition_to_at_phase(clips[2].0.clone(), 0.7, 0.29)
            .unwrap();
        source.advance_wall(&skeleton, 0.113).unwrap();
        assert!(source.transition.as_ref().unwrap().source_pose.is_some());
        let saved = source
            .checkpoint(|c, p| identify(&clips, c, p), 65536)
            .unwrap();
        let text = saved.to_json(65536).unwrap();
        let saved = AnimatorCheckpoint::from_json(&text, 65536).unwrap();
        let (fresh_rig, fresh_clips) = assets();
        assert!(!Arc::ptr_eq(&clips[0].0, &fresh_clips[0].0));
        let mut restored = saved
            .prepare(
                &fresh_rig,
                |key| {
                    fresh_clips
                        .iter()
                        .enumerate()
                        .find(|(i, _)| key == format!("revision-{i}"))
                        .map(|(_, (c, p))| (c.clone(), Some(p.clone())))
                },
                65536,
            )
            .unwrap();
        assert_eq!(
            text,
            restored
                .checkpoint(|c, p| identify(&fresh_clips, c, p), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap()
        );
        for frame in 0..240 {
            if frame == 30 {
                source.set_speed(0.).unwrap();
                restored.set_speed(0.).unwrap();
            }
            if frame == 40 {
                source.set_speed(1.1).unwrap();
                restored.set_speed(1.1).unwrap();
            }
            if frame == 90 {
                source.transition_to(clips[0].0.clone(), 0.4).unwrap();
                restored
                    .transition_to(fresh_clips[0].0.clone(), 0.4)
                    .unwrap();
            }
            let dt = [0.003, 0.021, 0.037][frame % 3];
            let a = source.advance_wall(&skeleton, dt).unwrap();
            let b = restored.advance_wall(&fresh_rig, dt).unwrap();
            assert_eq!(a.pose, b.pose);
            assert_eq!(a.skin_matrices, b.skin_matrices);
            assert_eq!(
                a.root_motion.to_array().map(f32::to_bits),
                b.root_motion.to_array().map(f32::to_bits)
            );
            assert_eq!(a.transition_weight.to_bits(), b.transition_weight.to_bits());
            assert_eq!(encode(&a.properties), encode(&b.properties));
            assert_eq!(
                source
                    .checkpoint(|c, p| identify(&clips, c, p), 65536)
                    .unwrap()
                    .to_json(65536)
                    .unwrap(),
                restored
                    .checkpoint(|c, p| identify(&fresh_clips, c, p), 65536)
                    .unwrap()
                    .to_json(65536)
                    .unwrap()
            );
        }
    }
    #[test]
    fn ordinary_unbound_fade_and_clamped_playback_continue_exactly() {
        let (rig, assets) = assets();
        let first = assets[0].0.clone();
        let last = Arc::new(
            AnimationClip::new(
                "clamped",
                1.,
                Playback::Clamp,
                assets[1].0.tracks.to_vec(),
                &rig,
            )
            .unwrap(),
        );
        let mut source = Animator::new(first.clone());
        source.advance_wall(&rig, 0.31).unwrap();
        source
            .transition_to_at_phase(last.clone(), 0.8, 0.6)
            .unwrap();
        source.advance_wall(&rig, 0.11).unwrap();
        let saved = source
            .checkpoint(
                |clip, _| {
                    Some(
                        if Arc::ptr_eq(clip, &first) {
                            "first"
                        } else {
                            "last"
                        }
                        .into(),
                    )
                },
                65536,
            )
            .unwrap();
        let mut restored = saved
            .prepare(
                &rig,
                |key| match key {
                    "first" => Some((first.clone(), None)),
                    "last" => Some((last.clone(), None)),
                    _ => None,
                },
                65536,
            )
            .unwrap();
        for _ in 0..120 {
            let a = source.advance_wall(&rig, 0.021).unwrap();
            let b = restored.advance_wall(&rig, 0.021).unwrap();
            assert_eq!(a.pose, b.pose);
            assert_eq!(a.skin_matrices, b.skin_matrices);
            assert_eq!(
                a.root_motion.to_array().map(f32::to_bits),
                b.root_motion.to_array().map(f32::to_bits)
            );
            assert_eq!(a.transition_weight.to_bits(), b.transition_weight.to_bits());
        }
        assert_eq!(source.normalized_phase(), 1.);
        assert_eq!(source.time.to_bits(), restored.time.to_bits());
    }
    #[test]
    fn invalid_records_and_missing_or_ambiguous_revisions_are_rejected() {
        let (skeleton, clips) = assets();
        let mut source = Animator::new(clips[0].0.clone());
        source.advance_wall(&skeleton, 0.13).unwrap();
        source.transition_to(clips[1].0.clone(), 0.8).unwrap();
        assert!(
            source
                .checkpoint(|_, _| Some("same".into()), 65536)
                .is_err()
        );
        let saved = source
            .checkpoint(|c, p| identify(&clips, c, p), 65536)
            .unwrap();
        let text = saved.to_json(65536).unwrap();
        assert!(saved.prepare(&skeleton, |_| None, 65536).is_err());
        assert!(AnimatorCheckpoint::from_json(&text, 1).is_err());
        for bad in [u64::MAX, f64::NAN.to_bits(), (-1f64).to_bits()] {
            let mut saved = saved.clone();
            saved.time = bad;
            assert!(
                saved
                    .prepare(
                        &skeleton,
                        |key| clips
                            .iter()
                            .enumerate()
                            .find(|(i, _)| key == format!("revision-{i}"))
                            .map(|(_, (c, _))| (c.clone(), None)),
                        65536
                    )
                    .is_err()
            );
        }
        assert_eq!(
            source
                .checkpoint(|c, p| identify(&clips, c, p), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            text
        );
    }
}
