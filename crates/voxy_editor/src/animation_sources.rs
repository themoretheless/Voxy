//! Playback source keys preserve clip and property allocation identity separately.
#![allow(dead_code)]
use crate::EditorError;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use voxy_animation::{AnimationClip, PropertyClip};
pub(crate) type Source = (Arc<AnimationClip>, Option<Arc<PropertyClip>>);
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    clip: usize,
    property: Option<usize>,
}
#[derive(Default)]
pub(crate) struct Sources {
    pub clips: Vec<Arc<AnimationClip>>,
    pub properties: Vec<Arc<PropertyClip>>,
    pub bindings: Vec<Binding>,
    clips_by_ptr: HashMap<usize, usize>,
    properties_by_ptr: HashMap<usize, usize>,
    pairs: HashMap<(usize, Option<usize>), usize>,
}
impl Sources {
    pub fn matches(&self, prepared: &PreparedSources) -> bool {
        let expected: HashSet<_> = prepared
            .sources
            .iter()
            .map(|(clip, property)| (Arc::as_ptr(clip), property.as_ref().map(Arc::as_ptr)))
            .collect();
        let actual: HashSet<_> = self
            .bindings
            .iter()
            .map(|binding| {
                (
                    Arc::as_ptr(&self.clips[binding.clip]),
                    binding.property.map(|id| Arc::as_ptr(&self.properties[id])),
                )
            })
            .collect();
        self.bindings.len() == prepared.sources.len() && expected == actual
    }
    pub fn intern(
        &mut self,
        clip: &Arc<AnimationClip>,
        property: Option<&Arc<PropertyClip>>,
        max: usize,
    ) -> Option<String> {
        let clip_id = self
            .clips_by_ptr
            .get(&(Arc::as_ptr(clip) as usize))
            .copied();
        let property_id = property.and_then(|p| {
            self.properties_by_ptr
                .get(&(Arc::as_ptr(p) as usize))
                .copied()
        });
        if let Some(index) = clip_id.and_then(|c| {
            if property.is_some() && property_id.is_none() {
                None
            } else {
                self.pairs.get(&(c, property_id)).copied()
            }
        }) {
            return Some(format!("playback:{index}"));
        }
        if self.bindings.len() >= max {
            return None;
        }
        let clip_id = *self
            .clips_by_ptr
            .entry(Arc::as_ptr(clip) as usize)
            .or_insert_with(|| {
                let id = self.clips.len();
                self.clips.push(clip.clone());
                id
            });
        let property_id = property.map(|p| {
            *self
                .properties_by_ptr
                .entry(Arc::as_ptr(p) as usize)
                .or_insert_with(|| {
                    let id = self.properties.len();
                    self.properties.push(p.clone());
                    id
                })
        });
        let index = self.bindings.len();
        self.bindings.push(Binding {
            clip: clip_id,
            property: property_id,
        });
        self.pairs.insert((clip_id, property_id), index);
        Some(format!("playback:{index}"))
    }
}
pub(crate) struct PreparedSources {
    pub sources: Vec<Source>,
}
impl PreparedSources {
    pub fn resolve(&self, key: &str) -> Option<Source> {
        let index = key.strip_prefix("playback:")?.parse::<usize>().ok()?;
        if format!("playback:{index}") != key {
            return None;
        }
        self.sources.get(index).cloned()
    }
}
pub(crate) fn prepare(
    bindings: &[Binding],
    clips: &[Arc<AnimationClip>],
    properties: Vec<PropertyClip>,
    max: usize,
) -> Result<PreparedSources, EditorError> {
    let invalid = || EditorError::from("invalid playback source bindings");
    if bindings.len() > max {
        return Err(invalid());
    }
    let mut pointers = HashSet::new();
    if clips.iter().any(|clip| !pointers.insert(Arc::as_ptr(clip))) {
        return Err(invalid());
    }
    let properties: Vec<_> = properties.into_iter().map(Arc::new).collect();
    let mut next_clip = 0;
    let mut next_property = 0;
    let mut pairs = HashSet::new();
    let mut sources = Vec::new();
    for binding in bindings {
        if binding.clip > next_clip || !pairs.insert((binding.clip, binding.property)) {
            return Err(invalid());
        }
        let clip = clips.get(binding.clip).cloned().ok_or_else(invalid)?;
        if binding.clip == next_clip {
            next_clip += 1;
        }
        let property = binding
            .property
            .map(|id| {
                if id > next_property {
                    return Err(invalid());
                }
                let property = properties.get(id).cloned().ok_or_else(invalid)?;
                if id == next_property {
                    next_property += 1;
                }
                Ok(property)
            })
            .transpose()?;
        sources.push((clip, property));
    }
    if next_clip != clips.len() || next_property != properties.len() {
        return Err(invalid());
    }
    Ok(PreparedSources { sources })
}

#[cfg(test)]
mod tests;
