//! Native pointer adapter. Revalidate live ownership before both activation edges.
use voxy_gameplay::{SceneUiRuntime, UiActionEvent};
use voxy_scene::SceneGraph;
use winit::{event::ElementState, keyboard::KeyCode};

#[derive(Debug)]
pub(super) struct GameUiInput {
    runtime: SceneUiRuntime,
    activation_keys: u8,
    admitted: Option<voxy_gameplay::SceneUiSnapshot>,
}
impl GameUiInput {
    pub(super) fn new() -> Self {
        Self {
            runtime: SceneUiRuntime::new(128),
            activation_keys: 0,
            admitted: None,
        }
    }
    pub(super) fn cancel(&mut self) {
        self.runtime.cancel();
        self.activation_keys = 0;
        self.admitted = None;
    }
    pub(super) fn admit_presented(
        &mut self,
        scene: &SceneGraph,
        viewport: [f32; 2],
        presented: Option<&voxy_gameplay::SceneUiSnapshot>,
    ) -> Result<bool, String> {
        let current = match voxy_gameplay::extract_scene_ui(scene, viewport, 128) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.cancel();
                return Err(error);
            }
        };
        let admitted = presented == Some(&current);
        if !admitted || self.admitted.as_ref() != Some(&current) {
            self.cancel();
        }
        self.admitted = admitted.then_some(current);
        Ok(admitted)
    }
    pub(super) fn focus_ring(
        &self,
        snapshot: &voxy_gameplay::SceneUiSnapshot,
    ) -> Option<super::ui_draw::FocusRing> {
        if self.admitted.as_ref() != Some(snapshot) {
            return None;
        }
        let owner = self.runtime.focused_owner()?;
        let element = snapshot
            .elements
            .iter()
            .find(|element| element.owner == owner)?;
        if !element.enabled || element.descriptor.action.is_none() {
            return None;
        }
        Some(super::ui_draw::FocusRing {
            owner,
            viewport: snapshot.viewport,
            clip: element.clip?,
        })
    }
    pub(super) fn key(
        &mut self,
        scene: &SceneGraph,
        viewport: [f32; 2],
        key: KeyCode,
        state: ElementState,
        repeat: bool,
        reverse: bool,
    ) -> Result<(bool, Option<UiActionEvent>), String> {
        if !matches!(key, KeyCode::Tab | KeyCode::Enter | KeyCode::Space) {
            return Ok((false, None));
        }
        if let Err(error) = self.runtime.refresh(scene, viewport) {
            self.cancel();
            return Err(error);
        }
        let pressed = state == ElementState::Pressed;
        if key == KeyCode::Tab {
            if pressed && !repeat {
                // Moving focus during an activation cancels its target. Keep the
                // held-key mask until release so no extra activation is synthesized.
                let _ = self.runtime.key_release();
                self.runtime.traverse(reverse);
            }
            return Ok((self.runtime.focused_owner().is_some(), None));
        }
        let bit = if key == KeyCode::Enter { 1 } else { 2 };
        let before = self.activation_keys;
        if before == 0 && self.runtime.focused_owner().is_none() {
            return Ok((false, None));
        }
        if pressed {
            if repeat {
                return Ok((true, None));
            }
            self.activation_keys |= bit;
        } else {
            self.activation_keys &= !bit;
        }
        if before == 0 && self.activation_keys != 0 {
            self.runtime.key_press();
        }
        let event = if before != 0 && self.activation_keys == 0 {
            self.runtime.key_release()
        } else {
            None
        };
        Ok((true, event))
    }
    pub(super) fn pointer(
        &mut self,
        scene: &SceneGraph,
        viewport: [f32; 2],
        cursor: [f32; 2],
        pressed: bool,
    ) -> Result<Option<UiActionEvent>, String> {
        if let Err(error) = self.runtime.refresh(scene, viewport) {
            // Retained visuals must never authorize stale input after failed admission.
            self.cancel();
            return Err(error);
        }
        self.runtime.pointer_move(Some(cursor));
        if pressed {
            self.runtime.pointer_press()?;
            Ok(None)
        } else {
            Ok(self.runtime.pointer_release().1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_gameplay::UiElement;
    use voxy_scene::Transform;
    fn button() -> UiElement {
        UiElement {
            origin: [0.0; 2],
            size: [1.0; 2],
            color: [1.0; 4],
            layer: 0,
            enabled: true,
            action: Some("start".into()),
            text: None,
        }
    }
    #[test]
    fn focus_visual_uses_admitted_visible_owner_and_clears_on_cancel_or_publication_change() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(owner, button()).unwrap();
        let snapshot = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 128).unwrap();
        let mut input = GameUiInput::new();
        assert!(input.focus_ring(&snapshot).is_none());
        assert!(
            input
                .admit_presented(&scene, [100.0; 2], Some(&snapshot))
                .unwrap()
        );
        input
            .key(
                &scene,
                [100.0; 2],
                KeyCode::Tab,
                ElementState::Pressed,
                false,
                false,
            )
            .unwrap();
        let ring = input.focus_ring(&snapshot).unwrap();
        assert_eq!(ring.owner, owner);
        assert_eq!(ring.clip, [0.0, 0.0, 100.0, 100.0]);
        input.cancel();
        assert!(input.focus_ring(&snapshot).is_none());
        input
            .admit_presented(&scene, [100.0; 2], Some(&snapshot))
            .unwrap();
        input
            .key(
                &scene,
                [100.0; 2],
                KeyCode::Tab,
                ElementState::Pressed,
                false,
                false,
            )
            .unwrap();
        scene.remove_component::<UiElement>(owner).unwrap();
        let changed = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 128).unwrap();
        assert!(input.focus_ring(&changed).is_none());
        input
            .admit_presented(&scene, [100.0; 2], Some(&snapshot))
            .unwrap();
        assert!(input.focus_ring(&snapshot).is_none());
    }

    #[test]
    fn input_waits_for_visible_snapshot_and_cancels_capture_across_publications() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(owner, button()).unwrap();
        let old = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 128).unwrap();
        let mut input = GameUiInput::new();
        assert!(!input.admit_presented(&scene, [100.0; 2], None).unwrap());
        assert!(
            input
                .admit_presented(&scene, [100.0; 2], Some(&old))
                .unwrap()
        );
        input.pointer(&scene, [100.0; 2], [50.0; 2], true).unwrap();
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .origin[0] = 0.1;
        assert!(
            !input
                .admit_presented(&scene, [100.0; 2], Some(&old))
                .unwrap()
        );
        let new = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 128).unwrap();
        assert!(
            input
                .admit_presented(&scene, [100.0; 2], Some(&new))
                .unwrap()
        );
        assert_eq!(
            input.pointer(&scene, [100.0; 2], [50.0; 2], false).unwrap(),
            None
        );
        input.pointer(&scene, [100.0; 2], [50.0; 2], true).unwrap();
        // No intervening input event observes this publication change.
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .origin[0] = 0.2;
        let newest = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 128).unwrap();
        assert!(
            input
                .admit_presented(&scene, [100.0; 2], Some(&newest))
                .unwrap()
        );
        assert_eq!(
            input.pointer(&scene, [100.0; 2], [50.0; 2], false).unwrap(),
            None
        );
        input.pointer(&scene, [100.0; 2], [50.0; 2], true).unwrap();
        assert!(
            input
                .admit_presented(&scene, [100.0; 2], Some(&newest))
                .unwrap()
        );
        assert_eq!(
            input
                .pointer(&scene, [100.0; 2], [50.0; 2], false)
                .unwrap()
                .unwrap()
                .owner,
            owner
        );
        assert!(
            !input
                .admit_presented(&scene, [200.0; 2], Some(&newest))
                .unwrap()
        );
        scene.remove_component::<UiElement>(owner).unwrap();
        assert!(
            !input
                .admit_presented(&scene, [100.0; 2], Some(&newest))
                .unwrap()
        );
    }

    #[test]
    fn keyboard_combines_activation_edges_and_revalidates_live_targets() {
        let mut scene = SceneGraph::new(2);
        let first = scene.spawn(None, Transform::default()).unwrap();
        let second = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(first, button()).unwrap();
        scene.insert_component(second, button()).unwrap();
        let mut input = GameUiInput::new();
        let mut key = |scene: &SceneGraph, code, pressed, repeat, reverse| {
            input
                .key(
                    scene,
                    [100.0; 2],
                    code,
                    if pressed {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                    repeat,
                    reverse,
                )
                .unwrap()
        };
        assert!(!key(&scene, KeyCode::Space, true, false, false).0);
        assert!(key(&scene, KeyCode::Tab, true, false, false).0);
        key(&scene, KeyCode::Tab, true, true, false);
        key(&scene, KeyCode::Enter, true, false, false);
        key(&scene, KeyCode::Space, true, false, false);
        key(&scene, KeyCode::Enter, true, true, false);
        assert_eq!(key(&scene, KeyCode::Enter, false, false, false).1, None);
        assert_eq!(
            key(&scene, KeyCode::Space, false, false, false)
                .1
                .unwrap()
                .owner,
            first
        );
        assert_eq!(key(&scene, KeyCode::Space, false, false, false).1, None);
        key(&scene, KeyCode::Tab, true, false, false);
        key(&scene, KeyCode::Tab, true, false, true);
        key(&scene, KeyCode::Tab, true, false, false);
        key(&scene, KeyCode::Enter, true, false, false);
        assert_eq!(
            key(&scene, KeyCode::Enter, false, false, false)
                .1
                .unwrap()
                .owner,
            second
        );
        key(&scene, KeyCode::Enter, true, false, false);
        key(&scene, KeyCode::Tab, true, false, false);
        assert_eq!(key(&scene, KeyCode::Enter, false, false, false).1, None);
        key(&scene, KeyCode::Tab, true, false, true);
        key(&scene, KeyCode::Enter, true, false, false);
        scene.remove_component::<UiElement>(second).unwrap();
        assert_eq!(key(&scene, KeyCode::Enter, false, false, false).1, None);
        key(&scene, KeyCode::Tab, true, false, false);
        key(&scene, KeyCode::Enter, true, false, false);
        input.cancel();
        assert_eq!(
            input
                .key(
                    &scene,
                    [100.0; 2],
                    KeyCode::Enter,
                    ElementState::Released,
                    false,
                    false
                )
                .unwrap()
                .1,
            None
        );
    }

    #[test]
    fn live_pointer_release_revalidates_removal_disable_and_failed_layout() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(owner, button()).unwrap();
        let mut input = GameUiInput::new();
        let mut edge =
            |scene: &SceneGraph, pressed| input.pointer(scene, [100.0; 2], [50.0; 2], pressed);
        assert_eq!(edge(&scene, true).unwrap(), None);
        assert_eq!(edge(&scene, false).unwrap().unwrap().owner, owner);
        edge(&scene, true).unwrap();
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .enabled = false;
        assert_eq!(edge(&scene, false).unwrap(), None);
        scene.insert_component(owner, button()).unwrap();
        edge(&scene, true).unwrap();
        scene.remove_component::<UiElement>(owner).unwrap();
        assert_eq!(edge(&scene, false).unwrap(), None);
        scene.insert_component(owner, button()).unwrap();
        edge(&scene, true).unwrap();
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .size[0] = 0.0;
        assert!(edge(&scene, false).is_err());
        scene.insert_component(owner, button()).unwrap();
        assert_eq!(edge(&scene, false).unwrap(), None);
        edge(&scene, true).unwrap();
        input.cancel();
        assert_eq!(
            input.pointer(&scene, [100.0; 2], [50.0; 2], false).unwrap(),
            None
        );
    }
}
