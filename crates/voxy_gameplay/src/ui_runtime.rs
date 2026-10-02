//! Scene binding for the existing platform-neutral pointer and focus routers.
use crate::{SceneUiSnapshot, extract_scene_ui};
use std::collections::{BTreeMap, HashMap};
use voxy_scene::{NodeId, SceneGraph};
use voxy_ui::{FocusRouter, HitRegion, KeyAction, PointerAction, PointerRouter, WidgetId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiActionEvent {
    pub owner: NodeId,
    pub action: String,
}
#[derive(Debug)]
pub struct SceneUiRuntime {
    pointer: PointerRouter,
    focus: FocusRouter,
    bindings: BTreeMap<WidgetId, (NodeId, Option<String>)>,
    identities: HashMap<NodeId, WidgetId>,
    snapshot: Option<SceneUiSnapshot>,
    next: u64,
    capacity: usize,
}
impl SceneUiRuntime {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            pointer: PointerRouter::new(capacity),
            focus: FocusRouter::new(capacity),
            bindings: BTreeMap::new(),
            identities: HashMap::new(),
            snapshot: None,
            next: 1,
            capacity,
        }
    }
    #[must_use]
    pub fn snapshot(&self) -> Option<&SceneUiSnapshot> {
        self.snapshot.as_ref()
    }
    #[must_use]
    pub fn focused_owner(&self) -> Option<NodeId> {
        self.focus
            .focused()
            .and_then(|id| self.bindings.get(&id).map(|(owner, _)| *owner))
    }
    /// Reconciles scene generation/activity/layout and both UI routers. A failed
    /// descriptor/layout admission preserves the last snapshot and input targets.
    /// Callers must handle failure before drawing/dispatching a deleted old owner.
    /// # Errors
    /// Returns bounded extraction, monotonic identity exhaustion or router errors.
    pub fn refresh(&mut self, scene: &SceneGraph, viewport: [f32; 2]) -> Result<(), String> {
        let snapshot = extract_scene_ui(scene, viewport, self.capacity)?;
        let mut identities = HashMap::new();
        let mut bindings = BTreeMap::new();
        let mut regions = Vec::new();
        let mut order = Vec::new();
        let mut next = self.next;
        for element in &snapshot.elements {
            let retained = self.identities.get(&element.owner).copied().filter(|id| {
                self.bindings
                    .get(id)
                    .is_some_and(|(_, action)| *action == element.descriptor.action)
            });
            let id = if let Some(id) = retained {
                id
            } else {
                let id = WidgetId(next);
                next = next.checked_add(1).ok_or("scene UI identity exhausted")?;
                id
            };
            identities.insert(element.owner, id);
            bindings.insert(id, (element.owner, element.descriptor.action.clone()));
            if let Some(clip) = element.clip {
                let enabled = element.enabled && element.descriptor.action.is_some();
                regions.push(HitRegion {
                    id,
                    origin: [clip[0], clip[1]],
                    size: [clip[2], clip[3]],
                    enabled,
                });
                if enabled {
                    order.push(id);
                }
            }
        }
        // Extraction guarantees finite clipped geometry and unique live owners.
        // Both lists fit the same capacity, so router admission is prevalidated.
        self.pointer
            .set_regions(&regions)
            .map_err(|error| error.to_string())?;
        self.focus
            .set_order(&order)
            .map_err(|error| error.to_string())?;
        self.identities = identities;
        self.bindings = bindings;
        self.next = next;
        self.snapshot = Some(snapshot);
        Ok(())
    }
    pub fn pointer_move(&mut self, position: Option<[f32; 2]>) -> bool {
        self.pointer.move_to(position)
    }
    /// Disabled visual regions still consume/occlude pointer presses.
    /// # Errors
    /// Reports an inconsistent focus target; successful refresh prevalidates it.
    pub fn pointer_press(&mut self) -> Result<bool, String> {
        let result = self.pointer.press();
        let target = match result.action {
            PointerAction::Press(id) => Some(id),
            _ => None,
        };
        self.focus
            .focus_to(target)
            .map_err(|error| error.to_string())?;
        Ok(result.consumed)
    }
    fn event(&self, id: WidgetId) -> Option<UiActionEvent> {
        self.bindings.get(&id).and_then(|(owner, action)| {
            action.as_ref().map(|action| UiActionEvent {
                owner: *owner,
                action: action.clone(),
            })
        })
    }
    pub fn pointer_release(&mut self) -> (bool, Option<UiActionEvent>) {
        let result = self.pointer.release();
        let event = match result.action {
            PointerAction::Release { id, clicked: true } => self.event(id),
            _ => None,
        };
        (result.consumed, event)
    }
    pub fn traverse(&mut self, reverse: bool) {
        self.focus.traverse(reverse);
    }
    /// Native adapters supply one combined activation edge for Enter/Space.
    pub fn key_press(&mut self) {
        self.focus.press();
    }
    pub fn key_release(&mut self) -> Option<UiActionEvent> {
        match self.focus.release() {
            KeyAction::Release { id, clicked: true } => self.event(id),
            _ => None,
        }
    }
    pub fn cancel(&mut self) {
        self.pointer.cancel();
        self.focus.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiElement;
    use crate::ui_scene::tests::element;
    use voxy_scene::Transform;
    #[test]
    fn shared_pointer_focus_disabled_occlusion_and_generation_replacement() {
        let mut scene = SceneGraph::new(2);
        let lower = scene.spawn(None, Transform::default()).unwrap();
        let upper = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(lower, element(Some("lower")))
            .unwrap();
        let mut disabled = element(Some("upper"));
        disabled.layer = 1;
        disabled.enabled = false;
        scene.insert_component(upper, disabled).unwrap();
        let mut ui = SceneUiRuntime::new(2);
        ui.refresh(&scene, [100.0; 2]).unwrap();
        ui.pointer_move(Some([50.0; 2]));
        assert!(ui.pointer_press().unwrap());
        assert!(ui.pointer_release().1.is_none());
        ui.traverse(false);
        assert_eq!(ui.focused_owner(), Some(lower));
        ui.key_press();
        assert_eq!(ui.key_release().unwrap().action, "lower");
        scene.set_active(upper, false).unwrap();
        ui.refresh(&scene, [100.0; 2]).unwrap();
        ui.pointer_move(Some([50.0; 2]));
        ui.pointer_press().unwrap();
        scene.remove_subtree(lower).unwrap();
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        assert_ne!(lower, replacement);
        scene
            .insert_component(replacement, element(Some("replacement")))
            .unwrap();
        ui.refresh(&scene, [100.0; 2]).unwrap();
        assert_eq!(ui.pointer_release().1, None);
        ui.pointer_press().unwrap();
        assert_eq!(ui.pointer_release().1.unwrap().owner, replacement);
    }
    #[test]
    fn invalid_refresh_preserves_last_good_and_action_changes_cancel_capture() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(owner, element(Some("first")))
            .unwrap();
        let mut ui = SceneUiRuntime::new(1);
        ui.refresh(&scene, [100.0; 2]).unwrap();
        let previous = ui.snapshot().unwrap().clone();
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .size[0] = 0.0;
        assert!(ui.refresh(&scene, [100.0; 2]).is_err());
        assert_eq!(ui.snapshot(), Some(&previous));
        scene
            .insert_component(owner, element(Some("first")))
            .unwrap();
        ui.traverse(false);
        ui.key_press();
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .action = Some("second".into());
        ui.refresh(&scene, [200.0; 2]).unwrap();
        assert_eq!(ui.key_release(), None);
        assert_eq!(ui.focused_owner(), None);
        ui.traverse(false);
        ui.key_press();
        assert_eq!(ui.key_release().unwrap().action, "second");
        ui.key_press();
        ui.cancel();
        assert_eq!(ui.key_release(), None);
    }
}
