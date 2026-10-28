//! Immutable skeleton/clip ownership plus lazy cache state. Compiled f64 data is
//! reconstructed by the native compiler and checked against exact receipts.
use super::*;
use root_curve::RootCurveCheckpoint;
use root_rotation::RootRotationCheckpoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct AnimationAssetLimits {
    pub max_bytes: usize,
    pub max_rigs: usize,
    pub max_total_joints: usize,
    pub max_groups: usize,
    pub max_clip_allocations: usize,
    pub max_references: usize,
    pub max_keys: usize,
    pub max_string_bytes: usize,
}
impl Default for AnimationAssetLimits {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024,
            max_rigs: 128,
            max_total_joints: 32768,
            max_groups: 1024,
            max_clip_allocations: 1024,
            max_references: 4096,
            max_keys: 1_000_000,
            max_string_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AnimationAssetError {
    #[error("invalid animation asset ownership or state")]
    State,
    #[error("animation asset checkpoint exceeds policy")]
    Capacity,
    #[error("animation asset cache is changing; capture at an asset-query barrier")]
    Changing,
    #[error("animation asset compiler result differs from its exact receipt")]
    CompilerMismatch,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Animation(#[from] AnimationError),
}
type Error = AnimationAssetError;
pub(crate) fn playback(value: Playback) -> u8 {
    match value {
        Playback::Loop => 0,
        Playback::Clamp => 1,
    }
}
fn decode_playback(value: u8) -> Result<Playback, Error> {
    match value {
        0 => Ok(Playback::Loop),
        1 => Ok(Playback::Clamp),
        _ => Err(Error::State),
    }
}
pub(crate) fn mode(value: Interpolation) -> u8 {
    match value {
        Interpolation::Linear => 0,
        Interpolation::Step => 1,
        Interpolation::CubicSpline => 2,
    }
}
fn decode_mode(value: u8) -> Result<Interpolation, Error> {
    match value {
        0 => Ok(Interpolation::Linear),
        1 => Ok(Interpolation::Step),
        2 => Ok(Interpolation::CubicSpline),
        _ => Err(Error::State),
    }
}
fn transform(value: Transform) -> [u32; 10] {
    let Transform {
        translation,
        rotation,
        scale,
    } = value;
    let t = translation.to_array();
    let r = rotation.to_array();
    let s = scale.to_array();
    [t[0], t[1], t[2], r[0], r[1], r[2], r[3], s[0], s[1], s[2]].map(f32::to_bits)
}
fn decode_transform(value: [u32; 10]) -> Transform {
    let v = value.map(f32::from_bits);
    Transform {
        translation: Vec3::new(v[0], v[1], v[2]),
        rotation: Quat::from_array([v[3], v[4], v[5], v[6]]),
        scale: Vec3::new(v[7], v[8], v[9]),
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedJoint {
    name: usize,
    parent: Option<u16>,
    local: [u32; 10],
    inverse: [u32; 16],
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    translations: Vec<[u32; 4]>,
    rotations: Vec<[u32; 5]>,
    scales: Vec<[u32; 4]>,
    interpolation: [u8; 3],
    translation_tangents: Vec<[[u32; 3]; 2]>,
    rotation_tangents: Vec<[[u32; 4]; 2]>,
    scale_tangents: Vec<[[u32; 3]; 2]>,
}
fn vectors(keys: &[Vec3Key]) -> Vec<[u32; 4]> {
    keys.iter()
        .map(|value| {
            let Vec3Key { time, value } = value;
            let v = value.to_array();
            [
                time.to_bits(),
                v[0].to_bits(),
                v[1].to_bits(),
                v[2].to_bits(),
            ]
        })
        .collect()
}
fn decode_vectors(keys: &[[u32; 4]]) -> Vec<Vec3Key> {
    keys.iter()
        .map(|v| {
            let v = v.map(f32::from_bits);
            Vec3Key {
                time: v[0],
                value: Vec3::new(v[1], v[2], v[3]),
            }
        })
        .collect()
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    rig: usize,
    name: usize,
    duration: u32,
    playback: u8,
    tracks: Vec<Track>,
    root: RootCurveCheckpoint,
    motion: Vec<Option<Result<RootCurveCheckpoint, AnimationError>>>,
    rotation: Vec<Option<Result<RootRotationCheckpoint, AnimationError>>>,
    rigid: Vec<Option<Result<[u64; 8], AnimationError>>>,
    motion_keys: usize,
    rotation_keys: usize,
    constants: Vec<Option<[u32; 10]>>,
    translations: Vec<Option<[u32; 3]>>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationAssetCheckpoint {
    version: u32,
    strings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    name_bindings: Vec<usize>,
    rigs: Vec<Vec<SavedJoint>>,
    skeletons: Vec<usize>,
    groups: Vec<Group>,
    allocations: Vec<usize>,
    clips: Vec<usize>,
}
#[derive(Debug)]
pub struct PreparedAnimationAssets {
    pub skeletons: Vec<Skeleton>,
    pub clips: Vec<Arc<AnimationClip>>,
    /// Ordered names requested by the enclosing resource archive.
    pub names: Vec<Arc<str>>,
}
fn add(total: &mut usize, n: usize) -> Result<(), Error> {
    *total = total.checked_add(n).ok_or(Error::Capacity)?;
    Ok(())
}
#[derive(Default)]
struct Sources<'a> {
    names: Vec<Arc<str>>,
    name_ids: HashMap<usize, usize>,
    rigs: Vec<Arc<[Joint]>>,
    rig_ids: HashMap<usize, usize>,
    groups: Vec<&'a AnimationClip>,
    group_ids: HashMap<usize, usize>,
    allocations: Vec<&'a Arc<AnimationClip>>,
    allocation_ids: HashMap<usize, usize>,
}
impl Sources<'_> {
    fn name(&mut self, name: &Arc<str>) -> usize {
        let key = Arc::as_ptr(name).cast::<u8>() as usize;
        if let Some(id) = self.name_ids.get(&key) {
            return *id;
        }
        let id = self.names.len();
        self.names.push(name.clone());
        self.name_ids.insert(key, id);
        id
    }
    fn rig(&mut self, rig: &Arc<[Joint]>) -> usize {
        let key = Arc::as_ptr(rig).cast::<Joint>() as usize;
        if let Some(id) = self.rig_ids.get(&key) {
            return *id;
        }
        let id = self.rigs.len();
        self.rigs.push(rig.clone());
        self.rig_ids.insert(key, id);
        id
    }
}
fn same_group(a: &AnimationClip, b: &AnimationClip) -> bool {
    let AnimationClip {
        rig,
        root_curve,
        motion_curves,
        motion_cache_keys,
        rigid_curves,
        rotation_curves,
        rotation_cache_keys,
        name,
        duration,
        playback,
        tracks,
        interpolation,
        tangents,
        constant_transforms,
        constant_translations,
    } = a;
    Arc::ptr_eq(rig, &b.rig)
        && Arc::ptr_eq(root_curve, &b.root_curve)
        && Arc::ptr_eq(motion_curves, &b.motion_curves)
        && Arc::ptr_eq(motion_cache_keys, &b.motion_cache_keys)
        && Arc::ptr_eq(rigid_curves, &b.rigid_curves)
        && Arc::ptr_eq(rotation_curves, &b.rotation_curves)
        && Arc::ptr_eq(rotation_cache_keys, &b.rotation_cache_keys)
        && Arc::ptr_eq(name, &b.name)
        && duration.to_bits() == b.duration.to_bits()
        && *playback == b.playback
        && Arc::ptr_eq(tracks, &b.tracks)
        && Arc::ptr_eq(interpolation, &b.interpolation)
        && Arc::ptr_eq(tangents, &b.tangents)
        && Arc::ptr_eq(constant_transforms, &b.constant_transforms)
        && Arc::ptr_eq(constant_translations, &b.constant_translations)
}
impl Group {
    fn capture(clip: &AnimationClip, sources: &mut Sources<'_>) -> Result<Self, Error> {
        let AnimationClip {
            rig,
            root_curve,
            motion_curves,
            motion_cache_keys,
            rigid_curves,
            rotation_curves,
            rotation_cache_keys,
            name,
            duration,
            playback: playing,
            tracks,
            interpolation,
            tangents,
            constant_transforms,
            constant_translations,
        } = clip;
        if tracks.len() != rig.len()
            || interpolation.len() != tracks.len()
            || tangents.len() != tracks.len()
        {
            return Err(Error::State);
        }
        let counts = [
            motion_cache_keys.load(Ordering::Acquire),
            rotation_cache_keys.load(Ordering::Acquire),
        ];
        let masks = || {
            (0..tracks.len())
                .map(|i| {
                    [
                        motion_curves.get(i).is_some_and(|v| v.get().is_some()),
                        rotation_curves.get(i).is_some_and(|v| v.get().is_some()),
                        rigid_curves.get(i).is_some_and(|v| v.get().is_some()),
                    ]
                })
                .collect::<Vec<_>>()
        };
        let initialized = masks();
        let tracks = tracks
            .iter()
            .zip(interpolation.iter())
            .zip(tangents.iter())
            .map(|((track, mode), tangents)| {
                let JointTrack {
                    translations,
                    rotations,
                    scales,
                } = track;
                let TrackInterpolation {
                    translation,
                    rotation,
                    scale,
                } = mode;
                let JointTangents {
                    translation: tt,
                    rotation: rt,
                    scale: st,
                } = tangents;
                Track {
                    translations: vectors(translations),
                    scales: vectors(scales),
                    rotations: rotations
                        .iter()
                        .map(|key| {
                            let QuatKey { time, value } = key;
                            let v = value.to_array();
                            [
                                time.to_bits(),
                                v[0].to_bits(),
                                v[1].to_bits(),
                                v[2].to_bits(),
                                v[3].to_bits(),
                            ]
                        })
                        .collect(),
                    interpolation: [
                        self::mode(*translation),
                        self::mode(*rotation),
                        self::mode(*scale),
                    ],
                    translation_tangents: tt
                        .iter()
                        .map(|v| v.map(|v| v.to_array().map(f32::to_bits)))
                        .collect(),
                    rotation_tangents: rt
                        .iter()
                        .map(|v| v.map(|v| v.to_array().map(f32::to_bits)))
                        .collect(),
                    scale_tangents: st
                        .iter()
                        .map(|v| v.map(|v| v.to_array().map(f32::to_bits)))
                        .collect(),
                }
            })
            .collect();
        let saved = Self {
            rig: sources.rig(rig),
            name: sources.name(name),
            duration: duration.to_bits(),
            playback: playback(*playing),
            tracks,
            root: root_curve.checkpoint(),
            motion: motion_curves
                .iter()
                .map(|c| {
                    c.get()
                        .map(|v| v.as_ref().map(|c| c.checkpoint()).map_err(Clone::clone))
                })
                .collect(),
            rotation: rotation_curves
                .iter()
                .map(|c| {
                    c.get().map(|v| {
                        v.as_ref()
                            .map(RootRotationCurve::checkpoint)
                            .map_err(Clone::clone)
                    })
                })
                .collect(),
            rigid: rigid_curves
                .iter()
                .map(|c| {
                    c.get().map(|v| {
                        v.as_ref()
                            .map(RootRigidCurve::asset_checkpoint_words)
                            .map_err(Clone::clone)
                    })
                })
                .collect(),
            motion_keys: counts[0],
            rotation_keys: counts[1],
            constants: constant_transforms
                .iter()
                .map(|v| v.map(transform))
                .collect(),
            translations: constant_translations
                .iter()
                .map(|v| v.map(|v| v.to_array().map(f32::to_bits)))
                .collect(),
        };
        if initialized != masks()
            || counts
                != [
                    motion_cache_keys.load(Ordering::Acquire),
                    rotation_cache_keys.load(Ordering::Acquire),
                ]
        {
            return Err(Error::Changing);
        }
        saved.check_caches()?;
        Ok(saved)
    }
    fn check_caches(&self) -> Result<(), Error> {
        let n = self.tracks.len();
        if n == 0
            || [
                &self.motion.len(),
                &self.rotation.len(),
                &self.rigid.len(),
                &self.constants.len(),
                &self.translations.len(),
            ]
            .into_iter()
            .any(|len| *len != n)
        {
            return Err(Error::State);
        }
        if self.motion[0].is_some() {
            return Err(Error::State);
        }
        let mut motion = self.tracks[0].translations.len();
        let mut rotation = 0;
        for i in 0..n {
            if matches!(&self.motion[i], Some(Ok(_))) {
                add(&mut motion, self.tracks[i].translations.len())?;
            }
            if self.rotation[i]
                .as_ref()
                .is_some_and(|v| !matches!(v, Err(AnimationError::RootRotationBudget)))
            {
                add(&mut rotation, self.tracks[i].rotations.len())?;
            }
            if let Some(rigid) = &self.rigid[i] {
                let motion = if i == 0 {
                    Some(Ok(()))
                } else {
                    self.motion[i]
                        .as_ref()
                        .map(|v| v.as_ref().map(|_| ()).map_err(Clone::clone))
                };
                match motion {
                    None => return Err(Error::Changing),
                    Some(Err(error)) => {
                        if rigid.as_ref().err() != Some(&error) {
                            return Err(Error::Changing);
                        }
                    }
                    Some(Ok(())) => match &self.rotation[i] {
                        None => return Err(Error::Changing),
                        Some(Err(error)) => {
                            if rigid.as_ref().err() != Some(error) {
                                return Err(Error::Changing);
                            }
                        }
                        Some(Ok(_)) => {}
                    },
                }
            }
        }
        if motion != self.motion_keys || rotation != self.rotation_keys {
            return Err(Error::Changing);
        }
        Ok(())
    }
    fn restore(&self, rigs: &[Skeleton], names: &[Arc<str>]) -> Result<AnimationClip, Error> {
        self.check_caches()?;
        let skeleton = rigs.get(self.rig).ok_or(Error::State)?;
        let mut tracks = Vec::new();
        let mut modes = Vec::new();
        let mut tangents = Vec::new();
        for track in &self.tracks {
            tracks.push(JointTrack {
                translations: decode_vectors(&track.translations),
                scales: decode_vectors(&track.scales),
                rotations: track
                    .rotations
                    .iter()
                    .map(|v| {
                        let v = v.map(f32::from_bits);
                        QuatKey {
                            time: v[0],
                            value: Quat::from_array([v[1], v[2], v[3], v[4]]),
                        }
                    })
                    .collect(),
            });
            modes.push(TrackInterpolation {
                translation: decode_mode(track.interpolation[0])?,
                rotation: decode_mode(track.interpolation[1])?,
                scale: decode_mode(track.interpolation[2])?,
            });
            tangents.push(JointTangents {
                translation: track
                    .translation_tangents
                    .iter()
                    .map(|v| v.map(|v| Vec3::from_array(v.map(f32::from_bits))))
                    .collect(),
                rotation: track
                    .rotation_tangents
                    .iter()
                    .map(|v| v.map(|v| Vec4::from_array(v.map(f32::from_bits))))
                    .collect(),
                scale: track
                    .scale_tangents
                    .iter()
                    .map(|v| v.map(|v| Vec3::from_array(v.map(f32::from_bits))))
                    .collect(),
            });
        }
        let clip = AnimationClip::new_with_tangents(
            names.get(self.name).ok_or(Error::State)?.clone(),
            f32::from_bits(self.duration),
            decode_playback(self.playback)?,
            tracks,
            modes,
            tangents,
            skeleton,
        )?;
        if clip.root_curve.checkpoint() != self.root {
            return Err(Error::CompilerMismatch);
        }
        // Reserve every originally successful/consuming cell first. Budget failures
        // are then replayed against the same final monotonic key reservations.
        for failure in [false, true] {
            for (i, saved) in self.motion.iter().enumerate().filter(|(_, s)| s.is_some()) {
                let saved = saved.as_ref().ok_or(Error::State)?;
                if matches!(saved, Err(AnimationError::RootMotionBudget)) != failure {
                    continue;
                }
                let actual = clip.motion_curve(i as u16).map(|v| v.checkpoint());
                if &actual != saved {
                    return Err(Error::CompilerMismatch);
                }
            }
            for (i, saved) in self
                .rotation
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_some())
            {
                let saved = saved.as_ref().ok_or(Error::State)?;
                if matches!(saved, Err(AnimationError::RootRotationBudget)) != failure {
                    continue;
                }
                let actual = clip.root_rotation_curve(i as u16).map(|v| v.checkpoint());
                if &actual != saved {
                    return Err(Error::CompilerMismatch);
                }
            }
        }
        for (i, saved) in self.rigid.iter().enumerate() {
            if let Some(saved) = saved {
                if clip
                    .root_rigid_curve(i as u16)
                    .map(|v| v.asset_checkpoint_words())
                    != *saved
                {
                    return Err(Error::CompilerMismatch);
                }
            }
        }
        if clip.motion_cache_keys.load(Ordering::Relaxed) != self.motion_keys
            || clip.rotation_cache_keys.load(Ordering::Relaxed) != self.rotation_keys
        {
            return Err(Error::CompilerMismatch);
        }
        Ok(clip)
    }
}
impl AnimationAssetCheckpoint {
    /// Capture at a caller-owned asset-query barrier. Observed partial reservations
    /// reject rather than resetting cached errors, counters or compiled coefficients.
    pub fn capture(
        skeletons: &[Skeleton],
        clips: &[Arc<AnimationClip>],
        limits: AnimationAssetLimits,
    ) -> Result<Self, Error> {
        Self::capture_with_names(skeletons, clips, &[], limits)
    }
    /// Return native names without merging equal but independently allocated strings.
    /// The enclosing archive may intern these by Arc identity with other participants.
    pub fn source_names(skeletons: &[Skeleton], clips: &[Arc<AnimationClip>]) -> Vec<Arc<str>> {
        let mut names = Vec::new();
        for skeleton in skeletons {
            names.extend(skeleton.joints.iter().map(|j| j.name.clone()));
        }
        for clip in clips {
            names.push(clip.name.clone());
            names.extend(clip.rig.iter().map(|j| j.name.clone()));
        }
        names
    }
    /// Include name references held by an enclosing model/asset owner.
    pub fn capture_with_names(
        skeletons: &[Skeleton],
        clips: &[Arc<AnimationClip>],
        names: &[Arc<str>],
        limits: AnimationAssetLimits,
    ) -> Result<Self, Error> {
        if skeletons.len() > limits.max_references
            || clips.len() > limits.max_references
            || names.len() > limits.max_references
        {
            return Err(Error::Capacity);
        }
        let mut sources = Sources::default();
        let name_bindings = names.iter().map(|n| sources.name(n)).collect();
        let skeletons = skeletons
            .iter()
            .map(|s| {
                let Skeleton { joints } = s;
                sources.rig(joints)
            })
            .collect();
        let mut references = Vec::new();
        let mut allocations = Vec::new();
        let mut groups = Vec::new();
        for clip in clips {
            let key = Arc::as_ptr(clip) as usize;
            if let Some(id) = sources.allocation_ids.get(&key) {
                references.push(*id);
                continue;
            }
            let group_key = Arc::as_ptr(&clip.motion_cache_keys) as usize;
            let group = if let Some(id) = sources.group_ids.get(&group_key) {
                if !same_group(clip, sources.groups[*id]) {
                    return Err(Error::State);
                }
                *id
            } else {
                let id = groups.len();
                groups.push(Group::capture(clip, &mut sources)?);
                sources.groups.push(clip);
                sources.group_ids.insert(group_key, id);
                id
            };
            let id = allocations.len();
            allocations.push(group);
            references.push(id);
            sources.allocations.push(clip);
            sources.allocation_ids.insert(key, id);
        }
        let native_rigs = sources.rigs.clone();
        let mut rigs = Vec::new();
        for joints in &native_rigs {
            rigs.push(
                joints
                    .iter()
                    .map(|joint| {
                        let Joint {
                            name,
                            parent,
                            bind_local,
                            inverse_bind,
                        } = joint;
                        SavedJoint {
                            name: sources.name(name),
                            parent: *parent,
                            local: transform(*bind_local),
                            inverse: inverse_bind.to_cols_array().map(f32::to_bits),
                        }
                    })
                    .collect(),
            );
        }
        let saved = Self {
            version: 1,
            strings: sources.names.iter().map(ToString::to_string).collect(),
            name_bindings,
            rigs,
            skeletons,
            groups,
            allocations,
            clips: references,
        };
        saved.admit(limits)?;
        saved.to_json(limits.max_bytes)?;
        Ok(saved)
    }
    fn admit(&self, limits: AnimationAssetLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::State);
        }
        if self.rigs.len() > limits.max_rigs
            || self.groups.len() > limits.max_groups
            || self.allocations.len() > limits.max_clip_allocations
            || self.skeletons.len() > limits.max_references
            || self.clips.len() > limits.max_references
            || self.name_bindings.len() > limits.max_references
        {
            return Err(Error::Capacity);
        }
        let mut joints = 0;
        let mut keys = 0;
        let mut bytes = 0;
        for s in &self.strings {
            add(&mut bytes, s.len())?;
        }
        for rig in &self.rigs {
            if rig.is_empty() || rig.len() > MAX_JOINTS {
                return Err(Error::State);
            }
            add(&mut joints, rig.len())?;
        }
        for group in &self.groups {
            if self
                .rigs
                .get(group.rig)
                .is_none_or(|rig| rig.len() != group.tracks.len())
            {
                return Err(Error::State);
            }
            for track in &group.tracks {
                for count in [
                    track.translations.len(),
                    track.rotations.len(),
                    track.scales.len(),
                    track
                        .translation_tangents
                        .len()
                        .checked_mul(2)
                        .ok_or(Error::Capacity)?,
                    track
                        .rotation_tangents
                        .len()
                        .checked_mul(2)
                        .ok_or(Error::Capacity)?,
                    track
                        .scale_tangents
                        .len()
                        .checked_mul(2)
                        .ok_or(Error::Capacity)?,
                ] {
                    add(&mut keys, count)?;
                }
            }
        }
        if joints > limits.max_total_joints
            || keys > limits.max_keys
            || bytes > limits.max_string_bytes
        {
            return Err(Error::Capacity);
        }
        Ok(())
    }
    pub fn to_json(&self, max_bytes: usize) -> Result<String, Error> {
        struct Writer {
            bytes: Vec<u8>,
            limit: usize,
        }
        impl std::io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                    return Err(std::io::Error::other("animation asset byte budget"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Writer {
            bytes: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut writer, self)?;
        String::from_utf8(writer.bytes).map_err(|_| Error::State)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, Error> {
        if json.len() > max_bytes {
            return Err(Error::Capacity);
        }
        Ok(serde_json::from_str(json)?)
    }
    pub fn prepare(&self, limits: AnimationAssetLimits) -> Result<PreparedAnimationAssets, Error> {
        self.admit(limits)?;
        self.to_json(limits.max_bytes)?;
        let names: Vec<Arc<str>> = self.strings.iter().map(|s| Arc::from(s.as_str())).collect();
        let mut rigs = Vec::new();
        for rig in &self.rigs {
            let joints = rig
                .iter()
                .map(|j| {
                    Ok(Joint {
                        name: names.get(j.name).ok_or(Error::State)?.clone(),
                        parent: j.parent,
                        bind_local: decode_transform(j.local),
                        inverse_bind: Mat4::from_cols_array(&j.inverse.map(f32::from_bits)),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            rigs.push(Skeleton::new(joints)?);
        }
        let mut groups = Vec::new();
        for group in &self.groups {
            groups.push(group.restore(&rigs, &names)?);
        }
        let allocations = self
            .allocations
            .iter()
            .map(|id| groups.get(*id).cloned().map(Arc::new).ok_or(Error::State))
            .collect::<Result<Vec<_>, _>>()?;
        let skeletons = self
            .skeletons
            .iter()
            .map(|id| rigs.get(*id).cloned().ok_or(Error::State))
            .collect::<Result<Vec<_>, _>>()?;
        let clips = self
            .clips
            .iter()
            .map(|id| allocations.get(*id).cloned().ok_or(Error::State))
            .collect::<Result<Vec<_>, _>>()?;
        // Verify constants, exact inverse-bind bits and canonical ownership tables
        // too; equal content is never permission to merge distinct allocations.
        let bound_names = self
            .name_bindings
            .iter()
            .map(|id| names.get(*id).cloned().ok_or(Error::State))
            .collect::<Result<Vec<_>, _>>()?;
        if Self::capture_with_names(&skeletons, &clips, &bound_names, limits)? != *self {
            return Err(Error::CompilerMismatch);
        }
        Ok(PreparedAnimationAssets {
            skeletons,
            clips,
            names: bound_names,
        })
    }
}

#[cfg(test)]
mod tests;
