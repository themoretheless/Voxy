//! Exact property source data and its native Arc ownership; no playback clocks.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct PropertyAssetLimits {
    pub max_bytes: usize,
    pub max_clips: usize,
    pub max_sets: usize,
    pub max_tracks: usize,
    pub max_keys: usize,
    pub max_tangent_pairs: usize,
    pub max_string_bytes: usize,
}
impl Default for PropertyAssetLimits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_clips: 1024,
            max_sets: 1024,
            max_tracks: 16384,
            max_keys: 1_000_000,
            max_tangent_pairs: 1_000_000,
            max_string_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum PropertyAssetError {
    #[error("invalid property asset ownership or state")]
    State,
    #[error("property asset checkpoint exceeds policy")]
    Capacity,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Animation(#[from] AnimationError),
}
type Error = PropertyAssetError;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Value {
    Float(u32),
    Vec3([u32; 3]),
    Color([u32; 4]),
    Bool(bool),
}
impl From<PropertyValue> for Value {
    fn from(v: PropertyValue) -> Self {
        match v {
            PropertyValue::Float(v) => Self::Float(v.to_bits()),
            PropertyValue::Vec3(v) => Self::Vec3(v.to_array().map(f32::to_bits)),
            PropertyValue::Color(v) => Self::Color(v.map(f32::to_bits)),
            PropertyValue::Bool(v) => Self::Bool(v),
        }
    }
}
impl Value {
    fn native(&self) -> PropertyValue {
        match self {
            Self::Float(v) => PropertyValue::Float(f32::from_bits(*v)),
            Self::Vec3(v) => PropertyValue::Vec3(Vec3::from_array(v.map(f32::from_bits))),
            Self::Color(v) => PropertyValue::Color(v.map(f32::from_bits)),
            Self::Bool(v) => PropertyValue::Bool(*v),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    time: u32,
    value: Value,
}
fn key(v: &PropertyKey) -> Key {
    let PropertyKey { time, value } = v;
    Key {
        time: time.to_bits(),
        value: (*value).into(),
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    name: usize,
    keys: usize,
    mode: u8,
    tangents: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Clip {
    name: usize,
    duration: u32,
    playback: u8,
    tracks: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyAssetCheckpoint {
    version: u32,
    names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    name_bindings: Vec<usize>,
    keys: Vec<Vec<Key>>,
    tangents: Vec<Vec<(Value, Value)>>,
    tracks: Vec<Vec<Track>>,
    clips: Vec<Clip>,
    sets: Vec<Vec<(usize, Value)>>,
}
#[derive(Debug)]
pub struct PreparedPropertyAssets {
    pub clips: Vec<PropertyClip>,
    pub sets: Vec<PropertySet>,
    pub names: Vec<Arc<str>>,
}
#[derive(Default)]
struct Builder {
    names: HashMap<usize, usize>,
    keys: HashMap<usize, usize>,
    tangents: HashMap<usize, usize>,
    tracks: HashMap<usize, usize>,
}
impl Builder {
    fn name(&mut self, v: &Arc<str>, data: &mut PropertyAssetCheckpoint) -> usize {
        let ptr = Arc::as_ptr(v).cast::<u8>() as usize;
        *self.names.entry(ptr).or_insert_with(|| {
            let id = data.names.len();
            data.names.push(v.to_string());
            id
        })
    }
    fn tracks(
        &mut self,
        values: &Arc<[PropertyTrack]>,
        data: &mut PropertyAssetCheckpoint,
    ) -> usize {
        let ptr = Arc::as_ptr(values).cast::<PropertyTrack>() as usize;
        if let Some(id) = self.tracks.get(&ptr) {
            return *id;
        }
        let mut records = Vec::new();
        for v in values.iter() {
            let PropertyTrack {
                name,
                keys,
                interpolation,
                tangents,
            } = v;
            let name = self.name(name, data);
            let kptr = Arc::as_ptr(keys).cast::<PropertyKey>() as usize;
            let keys = *self.keys.entry(kptr).or_insert_with(|| {
                let id = data.keys.len();
                data.keys.push(keys.iter().map(key).collect());
                id
            });
            let tptr = Arc::as_ptr(tangents).cast::<(PropertyValue, PropertyValue)>() as usize;
            let tangents = *self.tangents.entry(tptr).or_insert_with(|| {
                let id = data.tangents.len();
                data.tangents.push(
                    tangents
                        .iter()
                        .map(|(a, b)| ((*a).into(), (*b).into()))
                        .collect(),
                );
                id
            });
            records.push(Track {
                name,
                keys,
                mode: crate::asset_checkpoint::mode(*interpolation),
                tangents,
            });
        }
        let id = data.tracks.len();
        data.tracks.push(records);
        self.tracks.insert(ptr, id);
        id
    }
}
fn sum(mut values: impl Iterator<Item = usize>) -> Result<usize, Error> {
    values.try_fold(0usize, |a, b| a.checked_add(b).ok_or(Error::Capacity))
}
impl PropertyAssetCheckpoint {
    pub fn capture(
        clips: &[PropertyClip],
        sets: &[PropertySet],
        limits: PropertyAssetLimits,
    ) -> Result<Self, Error> {
        Self::capture_with_names(clips, sets, &[], limits)
    }
    pub fn source_names(clips: &[PropertyClip], sets: &[PropertySet]) -> Vec<Arc<str>> {
        let mut names = Vec::new();
        for clip in clips {
            names.push(clip.name.clone());
            names.extend(clip.tracks.iter().map(|t| t.name.clone()));
        }
        for set in sets {
            names.extend(set.entries.iter().map(|(n, _)| n.clone()));
        }
        names
    }
    pub fn capture_with_names(
        clips: &[PropertyClip],
        sets: &[PropertySet],
        names: &[Arc<str>],
        limits: PropertyAssetLimits,
    ) -> Result<Self, Error> {
        if clips.len() > limits.max_clips
            || sets.len() > limits.max_sets
            || names.len() > limits.max_tracks
        {
            return Err(Error::Capacity);
        }
        let mut data = Self {
            version: 1,
            names: Vec::new(),
            name_bindings: Vec::new(),
            keys: Vec::new(),
            tangents: Vec::new(),
            tracks: Vec::new(),
            clips: Vec::new(),
            sets: Vec::new(),
        };
        let mut builder = Builder::default();
        data.name_bindings = names.iter().map(|n| builder.name(n, &mut data)).collect();
        for clip in clips {
            let PropertyClip {
                name,
                duration,
                playback,
                tracks,
            } = clip;
            let name = builder.name(name, &mut data);
            let tracks = builder.tracks(tracks, &mut data);
            data.clips.push(Clip {
                name,
                duration: duration.to_bits(),
                playback: crate::asset_checkpoint::playback(*playback),
                tracks,
            });
        }
        for set in sets {
            let PropertySet { entries } = set;
            let values = entries
                .iter()
                .map(|(name, value)| (builder.name(name, &mut data), (*value).into()))
                .collect();
            data.sets.push(values);
        }
        data.admit(limits)?;
        data.to_json(limits.max_bytes)?;
        Ok(data)
    }
    fn admit(&self, limits: PropertyAssetLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::State);
        }
        if self.name_bindings.len() > limits.max_tracks
            || self.clips.len() > limits.max_clips
            || self.sets.len() > limits.max_sets
            || sum(self
                .tracks
                .iter()
                .map(Vec::len)
                .chain(self.sets.iter().map(Vec::len)))?
                > limits.max_tracks
            || sum(self.keys.iter().map(Vec::len))? > limits.max_keys
            || sum(self.tangents.iter().map(Vec::len))? > limits.max_tangent_pairs
            || sum(self.names.iter().map(String::len))? > limits.max_string_bytes
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
                if self
                    .bytes
                    .len()
                    .checked_add(bytes.len())
                    .is_none_or(|n| n > self.limit)
                {
                    return Err(std::io::Error::other("property archive byte cap"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = Writer {
            bytes: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut w, self)?;
        String::from_utf8(w.bytes).map_err(|_| Error::State)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, Error> {
        if json.len() > max_bytes {
            return Err(Error::Capacity);
        }
        Ok(serde_json::from_str(json)?)
    }
    pub fn prepare(&self, limits: PropertyAssetLimits) -> Result<PreparedPropertyAssets, Error> {
        self.prepare_inner(limits, None)
    }
    /// Reuse names restored by the enclosing owner or skeletal participant.
    /// Equal text cannot override the recorded alias relationships.
    pub fn prepare_with_names(
        &self,
        limits: PropertyAssetLimits,
        names: &[Arc<str>],
    ) -> Result<PreparedPropertyAssets, Error> {
        self.prepare_inner(limits, Some(names))
    }
    fn prepare_inner(
        &self,
        limits: PropertyAssetLimits,
        external: Option<&[Arc<str>]>,
    ) -> Result<PreparedPropertyAssets, Error> {
        self.admit(limits)?;
        self.to_json(limits.max_bytes)?;
        let mut names: Vec<Arc<str>> = self.names.iter().map(|v| Arc::from(v.as_str())).collect();
        if let Some(external) = external {
            if external.len() != self.name_bindings.len() {
                return Err(Error::State);
            }
            let mut bound = HashMap::<usize, Arc<str>>::new();
            for (id, name) in self.name_bindings.iter().zip(external) {
                let slot = names.get_mut(*id).ok_or(Error::State)?;
                if slot.as_ref() != name.as_ref() {
                    return Err(Error::State);
                }
                if let Some(previous) = bound.get(id) {
                    if !Arc::ptr_eq(previous, name) {
                        return Err(Error::State);
                    }
                } else {
                    bound.insert(*id, name.clone());
                }
                *slot = name.clone();
            }
        }
        let keys: Vec<Arc<[PropertyKey]>> = self
            .keys
            .iter()
            .map(|keys| {
                keys.iter()
                    .map(|v| PropertyKey {
                        time: f32::from_bits(v.time),
                        value: v.value.native(),
                    })
                    .collect()
            })
            .collect();
        let tangents: Vec<Arc<[(PropertyValue, PropertyValue)]>> = self
            .tangents
            .iter()
            .map(|values| {
                values
                    .iter()
                    .map(|(a, b)| (a.native(), b.native()))
                    .collect()
            })
            .collect();
        let mut groups: Vec<Arc<[PropertyTrack]>> = Vec::new();
        for (index, group) in self.tracks.iter().enumerate() {
            let duration = f32::from_bits(
                self.clips
                    .iter()
                    .find(|v| v.tracks == index)
                    .ok_or(Error::State)?
                    .duration,
            );
            let mut tracks = Vec::new();
            for v in group {
                let name = names.get(v.name).ok_or(Error::State)?.clone();
                let k = keys.get(v.keys).ok_or(Error::State)?.clone();
                let t = tangents.get(v.tangents).ok_or(Error::State)?.clone();
                let mode = match v.mode {
                    0 => Interpolation::Linear,
                    1 => Interpolation::Step,
                    2 => Interpolation::CubicSpline,
                    _ => return Err(Error::State),
                };
                // The legacy constructor admits boolean Linear tracks; preserve
                // that accepted contract rather than routing it through the stricter explicit-mode constructor.
                let mut track = if mode == Interpolation::Linear && t.is_empty() {
                    PropertyTrack::new(name, k.to_vec(), duration)?
                } else {
                    PropertyTrack::new_with_interpolation(
                        name,
                        k.to_vec(),
                        duration,
                        mode,
                        t.to_vec(),
                    )?
                };
                track.keys = k;
                track.tangents = t;
                tracks.push(track);
            }
            groups.push(tracks.into());
        }
        let mut clips = Vec::new();
        for v in &self.clips {
            let playback = match v.playback {
                0 => Playback::Loop,
                1 => Playback::Clamp,
                _ => return Err(Error::State),
            };
            let tracks = groups.get(v.tracks).ok_or(Error::State)?.clone();
            let mut clip = PropertyClip::new(
                names.get(v.name).ok_or(Error::State)?.clone(),
                f32::from_bits(v.duration),
                playback,
                tracks.to_vec(),
            )?;
            // Only canonical original ordering can be retained. Verify before
            // installing the shared allocation instead of undoing native sorting.
            if clip
                .tracks
                .iter()
                .map(|t| t.name())
                .ne(tracks.iter().map(|t| t.name()))
            {
                return Err(Error::State);
            }
            clip.tracks = tracks;
            clips.push(clip);
        }
        let sets = self
            .sets
            .iter()
            .map(|set| {
                let entries = set
                    .iter()
                    .map(|(name, value)| {
                        Ok((
                            names.get(*name).ok_or(Error::State)?.clone(),
                            value.native(),
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(PropertySet::new(entries)?)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let bound_names = self
            .name_bindings
            .iter()
            .map(|id| names.get(*id).cloned().ok_or(Error::State))
            .collect::<Result<Vec<_>, _>>()?;
        if Self::capture_with_names(&clips, &sets, &bound_names, limits)? != *self {
            return Err(Error::State);
        }
        Ok(PreparedPropertyAssets {
            clips,
            sets,
            names: bound_names,
        })
    }
}
#[cfg(test)]
mod tests;
