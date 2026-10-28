//! Authoring history/order/gesture participant for the enclosing App transaction.
//! Resource drafts, preview, source revisions and other App owners remain separate.
#![allow(dead_code)]
use crate::*;
pub(crate) mod metadata;
mod source;
use serde::{Deserialize, Serialize};
use voxy_scene::{HistoryCheckpointLimits, SceneCheckpointView, SceneHistoryCheckpoint};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Drag {
    node: ObjectId,
    origin: [u32; 2],
    viewport: [u32; 2],
    viewport_origin: [u32; 2],
    translation: [u32; 3],
    rotation: [u32; 4],
    scale: [u32; 3],
    world_origin: [u32; 3],
    axis: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthoringHistoryCheckpoint {
    version: u32,
    #[serde(default)]
    metadata: Option<metadata::Metadata>,
    history: Option<SceneHistoryCheckpoint>,
    order: Vec<ObjectId>,
    selected: usize,
    next_object_id: u64,
    drag: Option<Drag>,
}
#[derive(Debug)]
pub(crate) struct PreparedAuthoringHistory {
    pub(crate) scene_id: voxy_scene::SceneId,
    pub(crate) metadata: Option<metadata::PreparedMetadata>,
    pub(crate) history: Option<SceneHistory>,
    pub(crate) object_ids: Vec<ObjectId>,
    pub(crate) instances: Vec<NodeId>,
    pub(crate) selected: usize,
    pub(crate) next_object_id: u64,
    pub(crate) drag: Option<ModelDrag>,
}
impl App {
    pub(crate) fn authoring_history_checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        limits: HistoryCheckpointLimits,
    ) -> Result<AuthoringHistoryCheckpoint, EditorError> {
        if !std::ptr::eq(view.graph, &raw const self.scene)
            || self.object_ids.len() != self.instances.len()
        {
            return Err("authoring identity arrays do not match scene".into());
        }
        for (id, node) in self.object_ids.iter().zip(&self.instances) {
            if view.reserved_identity(*node) != Some(id) {
                return Err("authoring identity binding differs".into());
            }
        }
        let drag = self
            .drag
            .as_ref()
            .map(|drag| {
                let ModelDrag {
                    node,
                    origin,
                    viewport,
                    viewport_origin,
                    original,
                    world_origin,
                    axis,
                    edit: _,
                } = drag;
                let Transform {
                    translation,
                    rotation,
                    scale,
                } = original;
                Ok::<_, EditorError>(Drag {
                    node: view
                        .reserved_identity(*node)
                        .cloned()
                        .ok_or("unidentified drag owner")?,
                    origin: origin.to_array().map(f32::to_bits),
                    viewport: viewport.to_array().map(f32::to_bits),
                    viewport_origin: viewport_origin.to_array().map(f32::to_bits),
                    translation: translation.to_array().map(f32::to_bits),
                    rotation: rotation.to_array().map(f32::to_bits),
                    scale: scale.to_array().map(f32::to_bits),
                    world_origin: world_origin.to_array().map(f32::to_bits),
                    axis: match axis {
                        DragAxis::Plane => 0,
                        DragAxis::X => 1,
                        DragAxis::Y => 2,
                        DragAxis::Z => 3,
                    },
                })
            })
            .transpose()?;
        let edits: Vec<_> = self.drag.iter().map(|drag| &drag.edit).collect();
        if self.authoring.history.is_none() && drag.is_some() {
            return Err("drag has no history owner".into());
        }
        let saved = AuthoringHistoryCheckpoint {
            version: 3,
            metadata: Some(metadata::Metadata::capture(
                self,
                view,
                limits.max_archive_bytes,
            )?),
            history: self
                .authoring
                .history
                .as_ref()
                .map(|history| history.checkpoint(&edits, limits))
                .transpose()?,
            order: self.object_ids.clone(),
            selected: self.selected,
            next_object_id: self.authoring.next_object_id,
            drag,
        };
        saved.to_json(limits.max_archive_bytes)?;
        // Same admission for capture and load; no caller mutation on rejection.
        saved.prepare(view, &self.authoring.authoring_project.registry, limits)?;
        Ok(saved)
    }
}
impl AuthoringHistoryCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("authoring history archive byte budget".into());
        }
        Ok(json)
    }
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("authoring history archive byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    pub(crate) fn prepare(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        limits: HistoryCheckpointLimits,
    ) -> Result<PreparedAuthoringHistory, EditorError> {
        self.to_json(limits.max_archive_bytes)?;
        if !(1..=3).contains(&self.version)
            || (self.version == 1 && self.metadata.is_some())
            || (self.version >= 2 && self.metadata.is_none())
            || self.order.len() > limits.max_scene_capacity
            || (self.order.is_empty() && self.selected != 0)
            || (!self.order.is_empty() && self.selected >= self.order.len())
            || (self.history.is_none() && self.drag.is_some())
        {
            return Err("invalid authoring history checkpoint header".into());
        }
        let mut unique = BTreeSet::new();
        let instances = self
            .order
            .iter()
            .map(|id| {
                if !unique.insert(id) {
                    return Err("duplicate authoring object order".into());
                }
                view.reserved_node(id)
                    .ok_or_else(|| EditorError::from("missing authoring object identity"))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let (history, mut edits) = if let Some(saved) = &self.history {
            let prepared = saved.prepare(registry, limits)?;
            (Some(prepared.history), prepared.edits)
        } else {
            (None, Vec::new())
        };
        if edits.len() != usize::from(self.drag.is_some()) {
            return Err("authoring detached edit count differs".into());
        }
        let drag = self
            .drag
            .as_ref()
            .map(|saved| {
                let original = Transform {
                    translation: Vec3::from_array(saved.translation.map(f32::from_bits)),
                    rotation: glam::Quat::from_array(saved.rotation.map(f32::from_bits)),
                    scale: Vec3::from_array(saved.scale.map(f32::from_bits)),
                };
                original.matrix()?;
                let origin = Vec2::from_array(saved.origin.map(f32::from_bits));
                let viewport = Vec2::from_array(saved.viewport.map(f32::from_bits));
                let viewport_origin = Vec2::from_array(saved.viewport_origin.map(f32::from_bits));
                let world_origin = Vec3::from_array(saved.world_origin.map(f32::from_bits));
                if !origin.is_finite()
                    || !viewport.is_finite()
                    || viewport.min_element() <= 0.
                    || !viewport_origin.is_finite()
                    || !world_origin.is_finite()
                {
                    return Err("invalid saved drag coordinates".into());
                }
                let axis = match saved.axis {
                    0 => DragAxis::Plane,
                    1 => DragAxis::X,
                    2 => DragAxis::Y,
                    3 => DragAxis::Z,
                    _ => return Err("unknown drag axis".into()),
                };
                let node = view
                    .reserved_node(&saved.node)
                    .ok_or("missing drag owner")?;
                let edit = edits.pop().ok_or("missing detached drag edit")?;
                Ok::<_, EditorError>(ModelDrag {
                    node,
                    origin,
                    viewport,
                    viewport_origin,
                    original,
                    world_origin,
                    axis,
                    edit,
                })
            })
            .transpose()?;
        Ok(PreparedAuthoringHistory {
            scene_id: view.graph.identity(),
            metadata: self
                .metadata
                .as_ref()
                .map(|m| m.prepare(view, registry, limits.max_archive_bytes, self.version >= 3))
                .transpose()?,
            history,
            object_ids: self.order.clone(),
            instances,
            selected: self.selected,
            next_object_id: self.next_object_id,
            drag,
        })
    }
}

#[cfg(test)]
mod tests;
