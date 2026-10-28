//! Durable scene UI participant. Retains the accepted snapshot even when authoring
//! descriptors changed after the last refresh; the next refresh reconciles normally.
use super::*;
use serde::{Deserialize, Serialize};
use voxy_scene::ObjectId;
use voxy_ui::{FocusCheckpoint, PointerCheckpoint};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneUiCheckpoint {
    version: u32,
    capacity: usize,
    next: u64,
    viewport: Option<[u32; 2]>,
    elements: Vec<Element>,
    pointer: Pointer,
    focus: Focus,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Element {
    id: u64,
    owner: ObjectId,
    descriptor: crate::UiElement,
    rect: [u32; 4],
    clip: Option<[u32; 4]>,
    enabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    position: Option<[u32; 2]>,
    captured: Option<u64>,
    held: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Focus {
    focused: Option<u64>,
    pressed: Option<u64>,
    held: bool,
}
impl SceneUiRuntime {
    /// # Errors
    /// Rejects unidentified owners, inconsistent state and byte-budget overflow.
    pub fn checkpoint(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<SceneUiCheckpoint, GameplayError> {
        let mut elements = Vec::new();
        for element in self.snapshot.iter().flat_map(|s| &s.elements) {
            let id = self
                .identities
                .get(&element.owner)
                .ok_or("missing UI identity")?;
            let owner = identify(element.owner).ok_or("unidentified UI owner")?;
            elements.push(Element {
                id: id.0,
                owner,
                descriptor: element.descriptor.clone(),
                rect: element.rect.map(f32::to_bits),
                clip: element.clip.map(|c| c.map(f32::to_bits)),
                enabled: element.enabled,
            });
        }
        let pointer = self.pointer.checkpoint();
        let focus = self.focus.checkpoint();
        let saved = SceneUiCheckpoint {
            version: 1,
            capacity: self.capacity,
            next: self.next,
            viewport: self.snapshot.as_ref().map(|s| s.viewport.map(f32::to_bits)),
            elements,
            pointer: Pointer {
                position: pointer.position.map(|p| p.map(f32::to_bits)),
                captured: pointer.captured.map(|id| id.0),
                held: pointer.held,
            },
            focus: Focus {
                focused: focus.focus.map(|id| id.0),
                pressed: focus.pressed.map(|id| id.0),
                held: focus.held,
            },
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl SceneUiCheckpoint {
    /// # Errors
    /// Rejects oversized output.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, GameplayError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("UI checkpoint byte budget".into());
        }
        Ok(json)
    }
    /// # Errors
    /// Rejects oversized input and malformed records; prepare validates semantics.
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, GameplayError> {
        if json.len() > max_bytes {
            return Err("UI checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    /// Rebinds accepted UI state to a staged scene without emitting actions.
    /// # Errors
    /// Rejects versions, limits, missing owners and inconsistent interaction state.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<SceneUiRuntime, GameplayError> {
        self.prepare_inner(scene, resolve, max_elements, max_bytes, false)
    }
    /// Restore accepted visual/input history through the scene's reserved identity
    /// index. Non-live tokens stay invalid to every gameplay lookup and next refresh.
    pub fn prepare_historical(
        &self,
        view: &voxy_scene::SceneCheckpointView<'_>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<SceneUiRuntime, GameplayError> {
        self.prepare_inner(
            view.graph,
            |id: &ObjectId| view.reserved_node(id),
            max_elements,
            max_bytes,
            true,
        )
    }
    fn prepare_inner(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_elements: usize,
        max_bytes: usize,
        history: bool,
    ) -> Result<SceneUiRuntime, GameplayError> {
        self.to_json(max_bytes)?;
        if self.version != 1
            || self.capacity > max_elements
            || self.elements.len() > self.capacity
            || self.next == 0
            || (self.viewport.is_none() && !self.elements.is_empty())
        {
            return Err("invalid UI checkpoint header or capacity".into());
        }
        let viewport = self.viewport.map(|v| v.map(f32::from_bits));
        if viewport.is_some_and(|v| v.iter().any(|x| !x.is_finite() || *x <= 0. || *x > 16384.)) {
            return Err("invalid UI checkpoint viewport".into());
        }
        let mut runtime = SceneUiRuntime::new(self.capacity);
        let mut elements = Vec::new();
        let mut regions = Vec::new();
        let mut order = Vec::new();
        let mut ids = std::collections::HashSet::new();
        let mut owners = std::collections::HashSet::new();
        let mut last_layer = None;
        let mut text_bytes = 0_usize;
        for saved in &self.elements {
            if saved.id == 0
                || saved.id >= self.next
                || !ids.insert(saved.id)
                || saved.owner.0.is_empty()
                || !owners.insert(saved.owner.clone())
                || !saved.descriptor.valid()
                || last_layer.is_some_and(|layer| layer > saved.descriptor.layer)
            {
                return Err("invalid UI checkpoint element".into());
            }
            text_bytes = text_bytes
                .checked_add(
                    saved
                        .descriptor
                        .text
                        .as_ref()
                        .map_or(0, |t| t.content.len()),
                )
                .ok_or("UI checkpoint text budget")?;
            if text_bytes > 65536 {
                return Err("UI checkpoint text budget".into());
            }
            last_layer = Some(saved.descriptor.layer);
            let owner = resolve(&saved.owner).ok_or("missing UI checkpoint owner")?;
            if history {
                if !scene.owns_handle(owner) {
                    return Err("foreign historical UI owner".into());
                }
            } else {
                scene.local(owner)?;
            }
            if runtime.identities.contains_key(&owner) {
                return Err("aliased UI checkpoint owner".into());
            }
            let id = WidgetId(saved.id);
            runtime.identities.insert(owner, id);
            runtime
                .bindings
                .insert(id, (owner, saved.descriptor.action.clone()));
            let rect = saved.rect.map(f32::from_bits);
            let clip = saved.clip.map(|c| c.map(f32::from_bits));
            if rect.iter().any(|x| !x.is_finite())
                || rect[2] <= 0.
                || rect[3] <= 0.
                || !(rect[0] + rect[2]).is_finite()
                || !(rect[1] + rect[3]).is_finite()
            {
                return Err("invalid UI checkpoint rectangle".into());
            }
            if let Some(c) = clip {
                let enabled = saved.enabled && saved.descriptor.action.is_some();
                regions.push(HitRegion {
                    id,
                    origin: [c[0], c[1]],
                    size: [c[2], c[3]],
                    enabled,
                });
                if enabled {
                    order.push(id);
                }
            }
            elements.push(crate::UiElementSnapshot {
                owner,
                descriptor: saved.descriptor.clone(),
                rect,
                clip,
                enabled: saved.enabled,
            });
        }
        runtime.pointer = PointerCheckpoint {
            regions,
            position: self.pointer.position.map(|p| p.map(f32::from_bits)),
            captured: self.pointer.captured.map(WidgetId),
            held: self.pointer.held,
        }
        .prepare(self.capacity)?;
        runtime.focus = FocusCheckpoint {
            order,
            focus: self.focus.focused.map(WidgetId),
            pressed: self.focus.pressed.map(WidgetId),
            held: self.focus.held,
        }
        .prepare(self.capacity)?;
        runtime.next = self.next;
        runtime.snapshot = viewport.map(|viewport| SceneUiSnapshot { viewport, elements });
        Ok(runtime)
    }
}

/// Immutable accepted layout, distinct from a live router's current snapshot.
/// Uses the same checked geometry/binding codec without carrying interaction state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneUiSnapshotCheckpoint {
    layout: SceneUiCheckpoint,
}
impl SceneUiSnapshot {
    pub fn checkpoint(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<SceneUiSnapshotCheckpoint, GameplayError> {
        let elements = self
            .elements
            .iter()
            .enumerate()
            .map(|(index, element)| {
                Ok(Element {
                    id: u64::try_from(index)
                        .map_err(|_| "UI snapshot identity overflow")?
                        .checked_add(1)
                        .ok_or("UI snapshot identity overflow")?,
                    owner: identify(element.owner).ok_or("unidentified accepted UI owner")?,
                    descriptor: element.descriptor.clone(),
                    rect: element.rect.map(f32::to_bits),
                    clip: element.clip.map(|c| c.map(f32::to_bits)),
                    enabled: element.enabled,
                })
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        let saved = SceneUiSnapshotCheckpoint {
            layout: SceneUiCheckpoint {
                version: 1,
                capacity: elements.len(),
                next: u64::try_from(elements.len())
                    .map_err(|_| "UI snapshot identity overflow")?
                    .checked_add(1)
                    .ok_or("UI snapshot identity overflow")?,
                viewport: Some(self.viewport.map(f32::to_bits)),
                elements,
                pointer: Pointer {
                    position: None,
                    captured: None,
                    held: false,
                },
                focus: Focus {
                    focused: None,
                    pressed: None,
                    held: false,
                },
            },
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl SceneUiSnapshotCheckpoint {
    pub fn to_json(&self, max_bytes: usize) -> Result<String, GameplayError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("accepted UI snapshot byte budget".into());
        }
        Ok(json)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, GameplayError> {
        if json.len() > max_bytes {
            return Err("accepted UI snapshot byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    /// Restores the accepted layout without consulting current descriptors or
    /// claiming a GPU frame was presented. Caller owns its presentation barrier.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<SceneUiSnapshot, GameplayError> {
        self.prepare_inner(scene, resolve, max_elements, max_bytes, false)
    }
    /// Restore accepted visual/input history through the scene's reserved identity
    /// index. Non-live tokens stay invalid to every gameplay lookup and next refresh.
    pub fn prepare_historical(
        &self,
        view: &voxy_scene::SceneCheckpointView<'_>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<SceneUiSnapshot, GameplayError> {
        self.prepare_inner(
            view.graph,
            |id: &ObjectId| view.reserved_node(id),
            max_elements,
            max_bytes,
            true,
        )
    }
    fn prepare_inner(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_elements: usize,
        max_bytes: usize,
        history: bool,
    ) -> Result<SceneUiSnapshot, GameplayError> {
        self.to_json(max_bytes)?;
        let layout = &self.layout;
        if layout.viewport.is_none()
            || layout.capacity != layout.elements.len()
            || layout.next
                != u64::try_from(layout.elements.len())
                    .map_err(|_| "UI snapshot identity overflow")?
                    .checked_add(1)
                    .ok_or("UI snapshot identity overflow")?
            || layout
                .elements
                .iter()
                .enumerate()
                .any(|(i, element)| element.id != i as u64 + 1)
            || layout.pointer.position.is_some()
            || layout.pointer.captured.is_some()
            || layout.pointer.held
            || layout.focus.focused.is_some()
            || layout.focus.pressed.is_some()
            || layout.focus.held
        {
            return Err("invalid immutable UI snapshot state".into());
        }
        layout
            .prepare_inner(scene, resolve, max_elements, max_bytes, history)?
            .snapshot
            .ok_or_else(|| "missing accepted UI snapshot".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_scene::tests::element;
    use voxy_scene::Transform;

    #[test]
    fn immutable_layout_keeps_painter_order_text_and_old_descriptors_with_fresh_handles() {
        let mut source = SceneGraph::new(4);
        let mut originals = Vec::new();
        for name in ["first", "second"] {
            let node = source.spawn(None, Transform::default()).unwrap();
            let mut descriptor = element(Some(name));
            descriptor.text = Some(crate::UiText {
                font: "font.logical".into(),
                content: format!("Текст {name}"),
                size: 14.,
                color: [1.; 4],
            });
            source.insert_component(node, descriptor).unwrap();
            originals.push(node);
        }
        let snapshot = crate::extract_scene_ui(&source, [100.; 2], 4).unwrap();
        let identify = |node| {
            originals
                .iter()
                .position(|old| *old == node)
                .map(|i| ObjectId(format!("owner-{i}")))
        };
        let saved = snapshot.checkpoint(identify, 100000).unwrap();
        let json = saved.to_json(100000).unwrap();
        let saved = SceneUiSnapshotCheckpoint::from_json(&json, 100000).unwrap();
        let mut target = SceneGraph::new(4);
        let second = target.spawn(None, Transform::default()).unwrap();
        let first = target.spawn(None, Transform::default()).unwrap();
        target
            .insert_component(first, element(Some("changed")))
            .unwrap();
        target
            .insert_component(second, element(Some("changed")))
            .unwrap();
        let resolve = |id: &ObjectId| match id.0.as_str() {
            "owner-0" => Some(first),
            "owner-1" => Some(second),
            _ => None,
        };
        let restored = saved.prepare(&target, resolve, 4, 100000).unwrap();
        assert_eq!(restored.elements[0].owner, first);
        assert_eq!(restored.elements[1].owner, second);
        assert_eq!(
            restored.elements[0].descriptor.action.as_deref(),
            Some("first")
        );
        assert_eq!(
            restored
                .checkpoint(
                    |node| if node == first {
                        Some(ObjectId("owner-0".into()))
                    } else if node == second {
                        Some(ObjectId("owner-1".into()))
                    } else {
                        None
                    },
                    100000
                )
                .unwrap()
                .to_json(100000)
                .unwrap(),
            json
        );
        assert!(saved.prepare(&target, |_| None, 4, 100000).is_err());
        assert!(saved.prepare(&target, |_| Some(first), 4, 100000).is_err());
        assert!(saved.prepare(&target, resolve, 1, 100000).is_err());
        assert!(SceneUiSnapshotCheckpoint::from_json(&json, json.len() - 1).is_err());
        let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
        bad["layout"]["elements"][0]["id"] = serde_json::json!(9);
        let bad = SceneUiSnapshotCheckpoint::from_json(&bad.to_string(), 100000).unwrap();
        assert!(bad.prepare(&target, resolve, 4, 100000).is_err());
    }

    #[test]
    fn fresh_scene_restores_pointer_and_key_hold_and_reconciles_pending_action_change() {
        let mut source = SceneGraph::new(2);
        let owner = source.spawn(None, Transform::default()).unwrap();
        source
            .insert_component(owner, element(Some("old")))
            .unwrap();
        let mut ui = SceneUiRuntime::new(2);
        ui.refresh(&source, [100.; 2]).unwrap();
        ui.pointer_move(Some([50.; 2]));
        ui.pointer_press().unwrap();
        ui.key_press();
        // Pending authoring revision must not overwrite the accepted input snapshot.
        source
            .component_mut::<crate::UiElement>(owner)
            .unwrap()
            .unwrap()
            .action = Some("new".into());
        let id = ObjectId("button".into());
        let saved = ui
            .checkpoint(|n| (n == owner).then(|| id.clone()), 65536)
            .unwrap();
        let json = saved.to_json(65536).unwrap();
        let saved = SceneUiCheckpoint::from_json(&json, 65536).unwrap();
        let mut target = SceneGraph::new(3);
        let _padding = target.spawn(None, Transform::default()).unwrap();
        let fresh = target.spawn(None, Transform::default()).unwrap();
        target
            .insert_component(fresh, element(Some("new")))
            .unwrap();
        let mut restored = saved
            .prepare(&target, |key| (key == &id).then_some(fresh), 2, 65536)
            .unwrap();
        assert_eq!(
            restored
                .checkpoint(|_| Some(id.clone()), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            json
        );
        assert_eq!(
            ui.pointer_press().unwrap(),
            restored.pointer_press().unwrap()
        );
        assert_eq!(
            ui.key_release().unwrap().action,
            restored.key_release().unwrap().action
        );
        assert_eq!(ui.pointer_release().1.unwrap().action, "old");
        let event = restored.pointer_release().1.unwrap();
        assert_eq!(event.owner, fresh);
        assert_eq!(event.action, "old");
        ui.pointer_press().unwrap();
        restored.pointer_press().unwrap();
        ui.key_press();
        restored.key_press();
        ui.refresh(&source, [200.; 2]).unwrap();
        restored.refresh(&target, [200.; 2]).unwrap();
        assert!(ui.pointer_release().1.is_none());
        assert!(restored.pointer_release().1.is_none());
        assert!(ui.key_release().is_none());
        assert!(restored.key_release().is_none());
        assert_eq!(
            ui.checkpoint(|_| Some(id.clone()), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            restored
                .checkpoint(|_| Some(id.clone()), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap()
        );
        assert!(saved.prepare(&target, |_| None, 2, 65536).is_err());
        assert!(saved.prepare(&target, |_| Some(fresh), 0, 65536).is_err());
        assert!(SceneUiCheckpoint::from_json(&json, 1).is_err());
        let mut bad = saved.clone();
        bad.pointer.held = false;
        assert!(bad.prepare(&target, |_| Some(fresh), 2, 65536).is_err());
        let mut bad = saved;
        bad.next = 1;
        assert!(bad.prepare(&target, |_| Some(fresh), 2, 65536).is_err());
    }
}
