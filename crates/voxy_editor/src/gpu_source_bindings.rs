//! Stable roles for retained GPU publications; frame/LOD state is a separate participant.
#![allow(dead_code)]
use crate::{EditorError, import::EditorAsset};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
use voxy_assets::{AssetId, ImportedAsset};
use voxy_scene::{NodeId, ObjectId, SceneGraph};
type Publication = Arc<ImportedAsset<EditorAsset>>;
pub(crate) struct LiveSources {
    pub models: Vec<(AssetId, Publication)>,
    pub owners: Vec<(NodeId, Publication)>,
}
impl LiveSources {
    pub fn publications(&self) -> Vec<Publication> {
        self.models
            .iter()
            .map(|(_, p)| p.clone())
            .chain(self.owners.iter().map(|(_, p)| p.clone()))
            .collect()
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GpuSourceBindings {
    version: u32,
    models: Vec<(String, usize)>,
    owners: Vec<(ObjectId, usize)>,
}
pub(crate) struct PreparedBindings {
    pub scene: voxy_scene::SceneId,
    pub models: BTreeMap<AssetId, Publication>,
    pub owners: BTreeMap<NodeId, Publication>,
}
fn invalid() -> EditorError {
    "invalid GPU publication role bindings".into()
}
impl GpuSourceBindings {
    pub fn capture(
        live: &LiveSources,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max: usize,
    ) -> Result<Self, EditorError> {
        if live
            .models
            .len()
            .checked_add(live.owners.len())
            .is_none_or(|n| n > max)
        {
            return Err(invalid());
        }
        let mut models = Vec::new();
        let mut ids = HashSet::new();
        for (id, _) in &live.models {
            if id.0.is_empty() || !ids.insert(id.0.clone()) {
                return Err(invalid());
            }
            models.push((id.0.clone(), models.len()));
        }
        let mut owners = Vec::new();
        let mut ids = HashSet::new();
        let mut nodes = HashSet::new();
        for (node, publication) in &live.owners {
            let id = identify(*node)
                .filter(|id| !id.0.is_empty())
                .ok_or_else(invalid)?;
            if !ids.insert(id.clone())
                || !nodes.insert(*node)
                || publication.value().animated.is_none()
            {
                return Err(invalid());
            }
            owners.push((id, models.len() + owners.len()));
        }
        Ok(Self {
            version: 1,
            models,
            owners,
        })
    }
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        publications: &[Publication],
        max: usize,
    ) -> Result<PreparedBindings, EditorError> {
        if self.version != 1
            || self.models.len().checked_add(self.owners.len()) != Some(publications.len())
            || publications.len() > max
        {
            return Err(invalid());
        }
        let mut models = BTreeMap::new();
        let mut owners = BTreeMap::new();
        let mut identities = HashSet::new();
        let mut next = 0;
        for (id, index) in &self.models {
            if id.is_empty() || *index != next || models.contains_key(&AssetId(id.clone())) {
                return Err(invalid());
            }
            models.insert(AssetId(id.clone()), publications[next].clone());
            next += 1;
        }
        for (id, index) in &self.owners {
            if id.0.is_empty() || !identities.insert(id) || *index != next {
                return Err(invalid());
            }
            let node = resolve(id).ok_or_else(invalid)?;
            scene.local(node)?;
            if owners.contains_key(&node) || publications[next].value().animated.is_none() {
                return Err(invalid());
            }
            owners.insert(node, publications[next].clone());
            next += 1;
        }
        Ok(PreparedBindings {
            scene: scene.identity(),
            models,
            owners,
        })
    }
}

#[cfg(test)]
mod tests;
