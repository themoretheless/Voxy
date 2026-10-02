//! Keyboard targeting shares panel geometry and the platform-neutral focus owner.
use crate::{InspectorMode, panels::Action};
use std::collections::BTreeMap;
use voxy_scene::SceneDocument;
use voxy_ui::{FocusRouter, KeyAction, UiError, WidgetId};

#[derive(Debug)]
pub(crate) struct PanelFocus {
    router: FocusRouter,
    targets: BTreeMap<String, (WidgetId, Action)>,
    next: u64,
    activation_keys: u8,
    context: Option<(SceneDocument, Option<voxy_scene::ObjectId>, InspectorMode)>,
}
impl PanelFocus {
    pub fn new() -> Self {
        Self {
            router: FocusRouter::new(256),
            targets: BTreeMap::new(),
            next: 1,
            activation_keys: 0,
            context: None,
        }
    }
    pub fn focused(&self) -> Option<WidgetId> {
        self.router.focused()
    }
    #[cfg(test)]
    fn identity(
        action: Action,
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
    ) -> String {
        Self::identity_with_registry(
            action,
            document,
            selected,
            mode,
            &crate::model_registry().unwrap(),
        )
    }
    fn identity_with_registry(
        action: Action,
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
        registry: &voxy_scene::ComponentRegistry,
    ) -> String {
        let owner = match action {
            Action::Select(index) => document.objects.get(index),
            _ => document.objects.get(selected),
        };
        let owner = owner.map_or("", |object| object.id.0.as_str());
        if matches!(mode, InspectorMode::Components(_))
            && let Action::Field(index) | Action::ResetField(index) = action
            && let Some(field) = document
                .objects
                .get(selected)
                .and_then(|object| crate::component_fields::fields(object).ok())
                .and_then(|fields| fields.into_iter().nth(index))
        {
            if let Some(binding) = document
                .objects
                .get(selected)
                .and_then(|object| field.bind(object, registry).ok())
            {
                return format!(
                    "component:{}:{}",
                    binding.identity(),
                    matches!(action, Action::ResetField(_))
                );
            }
            return format!(
                "component:{owner}:{}:{}:{}",
                field.schema,
                field.path,
                matches!(action, Action::ResetField(_))
            );
        }
        match action {
            Action::Select(_) => format!("select:{owner}"),
            Action::Field(index) => format!("field:{owner}:{mode:?}:{index}"),
            Action::ResetField(index) => format!("reset:{owner}:{mode:?}:{index}"),
            _ => format!("action:{owner}:{action:?}"),
        }
    }
    #[cfg(test)]
    pub fn reconcile(
        &mut self,
        actions: &[Action],
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
    ) -> Result<(), UiError> {
        self.reconcile_with_registry(
            actions,
            document,
            selected,
            mode,
            &crate::model_registry().map_err(|_| UiError::Capacity)?,
        )
    }
    pub fn reconcile_with_registry(
        &mut self,
        actions: &[Action],
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
        registry: &voxy_scene::ComponentRegistry,
    ) -> Result<(), UiError> {
        if actions.len() > 256 {
            return Err(UiError::Capacity);
        }
        let mut targets = BTreeMap::new();
        let mut order = Vec::with_capacity(actions.len());
        let mut next = self.next;
        for &action in actions {
            let key = Self::identity_with_registry(action, document, selected, mode, registry);
            let id = if let Some((id, _)) = self.targets.get(&key) {
                *id
            } else {
                let id = WidgetId(next);
                next = next.checked_add(1).ok_or(UiError::Capacity)?;
                id
            };
            if targets.insert(key, (id, action)).is_some() {
                return Err(UiError::DuplicateId);
            }
            order.push(id);
        }
        self.router.set_order(&order)?;
        self.targets = targets;
        self.next = next;
        self.context = Some(Self::context(document, selected, mode));
        Ok(())
    }
    fn context(
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
    ) -> (SceneDocument, Option<voxy_scene::ObjectId>, InspectorMode) {
        (
            document.clone(),
            document
                .objects
                .get(selected)
                .map(|object| object.id.clone()),
            mode,
        )
    }
    pub fn matches_context(
        &self,
        document: &SceneDocument,
        selected: usize,
        mode: InspectorMode,
    ) -> bool {
        self.context.as_ref() == Some(&Self::context(document, selected, mode))
    }
    // Enter and Space form one activation. Releasing one while the other is held
    // cannot click, and repeats cannot add another press.
    pub fn activate(&mut self, bit: u8, pressed: bool, repeat: bool) -> Option<Action> {
        if pressed && repeat {
            return None;
        }
        let before = self.activation_keys;
        if pressed {
            self.activation_keys |= bit;
        } else {
            self.activation_keys &= !bit;
        }
        if before == 0 && self.activation_keys != 0 {
            self.press();
        }
        if before != 0 && self.activation_keys == 0 {
            self.release()
        } else {
            None
        }
    }
    pub fn is_focused(&self, action: Action) -> bool {
        self.targets
            .values()
            .any(|&(id, target)| Some(id) == self.focused() && target == action)
    }
    pub fn id_for(&self, action: Action) -> Option<WidgetId> {
        self.targets
            .values()
            .find(|(_, target)| *target == action)
            .map(|(id, _)| *id)
    }
    pub fn action_for(&self, id: WidgetId) -> Option<Action> {
        self.targets
            .values()
            .find(|(target, _)| *target == id)
            .map(|(_, action)| *action)
    }
    pub fn pointer(&mut self, action: Option<Action>) -> Result<(), UiError> {
        let id = action.and_then(|action| self.id_for(action));
        self.router.focus_to(id)?;
        Ok(())
    }
    pub fn traverse(&mut self, reverse: bool) {
        self.router.traverse(reverse);
    }
    pub fn press(&mut self) {
        self.router.press();
    }
    pub fn release(&mut self) -> Option<Action> {
        if let KeyAction::Release { id, clicked: true } = self.router.release() {
            self.targets
                .values()
                .find(|(target, _)| *target == id)
                .map(|(_, action)| *action)
        } else {
            None
        }
    }
    pub fn cancel(&mut self) {
        self.activation_keys = 0;
        self.router.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{ObjectId, SceneObject};
    fn document(ids: &[&str]) -> SceneDocument {
        SceneDocument {
            version: 1,
            objects: ids
                .iter()
                .map(|id| SceneObject {
                    id: ObjectId((*id).into()),
                    parent: None,
                    name: (*id).into(),
                    active: true,
                    translation: [0.0; 3],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0; 3],
                    components: BTreeMap::new(),
                })
                .collect(),
        }
    }
    #[test]
    fn component_focus_tracks_schema_and_path_when_fields_are_reindexed() {
        let mut before = document(&["owner"]);
        before.objects[0]
            .components
            .insert("z".into(), serde_json::json!({"speed": 1}));
        let mut after = before.clone();
        after.objects[0]
            .components
            .insert("a".into(), serde_json::json!({"gain": 1}));
        let mode = InspectorMode::Components(0);
        assert_eq!(
            PanelFocus::identity(Action::Field(0), &before, 0, mode),
            PanelFocus::identity(Action::Field(1), &after, 0, mode)
        );
        assert_ne!(
            PanelFocus::identity(Action::Field(0), &before, 0, mode),
            PanelFocus::identity(Action::Field(0), &after, 0, mode)
        );
        assert_ne!(
            PanelFocus::identity(Action::Field(1), &after, 0, mode),
            PanelFocus::identity(Action::ResetField(1), &after, 0, mode)
        );
    }
    #[test]
    fn held_activation_cannot_jump_to_reindexed_object() {
        let mut focus = PanelFocus::new();
        focus
            .reconcile(
                &[Action::Select(0), Action::Select(1)],
                &document(&["a", "b"]),
                0,
                InspectorMode::Transform,
            )
            .unwrap();
        focus.traverse(false);
        focus.press();
        focus
            .reconcile(
                &[Action::Select(0)],
                &document(&["b"]),
                0,
                InspectorMode::Transform,
            )
            .unwrap();
        assert_eq!(focus.release(), None);
        assert_eq!(focus.focused(), None);
        focus.traverse(true);
        focus.press();
        focus.press();
        assert_eq!(focus.release(), Some(Action::Select(0)));
        assert_eq!(focus.release(), None);
    }
    #[test]
    fn surviving_identity_updates_action_and_context_changes_cancel() {
        let mut focus = PanelFocus::new();
        focus
            .reconcile(
                &[Action::Select(1), Action::Field(0)],
                &document(&["a", "b"]),
                1,
                InspectorMode::Audio,
            )
            .unwrap();
        focus.pointer(Some(Action::Select(1))).unwrap();
        focus.press();
        focus
            .reconcile(
                &[Action::Select(0), Action::Field(0)],
                &document(&["b"]),
                0,
                InspectorMode::Audio,
            )
            .unwrap();
        assert_eq!(focus.release(), Some(Action::Select(0)));
        focus.pointer(Some(Action::Field(0))).unwrap();
        focus.press();
        focus
            .reconcile(
                &[Action::Select(0), Action::Field(0)],
                &document(&["b"]),
                0,
                InspectorMode::Mixer,
            )
            .unwrap();
        assert_eq!(focus.release(), None);
        focus.traverse(true);
        focus.press();
        focus.cancel();
        assert_eq!(focus.release(), None);
        assert_eq!(focus.focused(), None);
    }
    #[test]
    fn invalid_reconciliation_preserves_live_focus_and_target() {
        let mut focus = PanelFocus::new();
        let doc = document(&["a"]);
        focus
            .reconcile(&[Action::Play], &doc, 0, InspectorMode::Transform)
            .unwrap();
        focus.traverse(false);
        focus.press();
        assert_eq!(
            focus.reconcile(
                &[Action::Play, Action::Play],
                &doc,
                0,
                InspectorMode::Transform
            ),
            Err(UiError::DuplicateId)
        );
        assert_eq!(focus.release(), Some(Action::Play));
    }
    #[test]
    fn repeats_after_focus_loss_cannot_reactivate_a_target() {
        let mut focus = PanelFocus::new();
        focus
            .reconcile(
                &[Action::Play],
                &document(&["a"]),
                0,
                InspectorMode::Transform,
            )
            .unwrap();
        focus.traverse(false);
        assert_eq!(focus.activate(1, true, false), None);
        focus.cancel();
        focus.traverse(false);
        assert_eq!(focus.activate(1, true, true), None);
        assert_eq!(focus.activate(1, false, false), None);
        assert_eq!(focus.activate(1, true, false), None);
        assert_eq!(focus.activate(1, false, false), Some(Action::Play));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{App, panels::Panels};
    use glam::Vec2;
    use winit::{event::ElementState, keyboard::KeyCode};
    fn build(app: &mut App) {
        let document = app.authoring_document().unwrap();
        app.panels
            .as_mut()
            .unwrap()
            .build(
                &document,
                app.selected,
                Vec2::new(1000.0, 700.0),
                false,
                None,
                0,
                false,
                app.inspector,
                "texture",
                "prefab",
                None,
            )
            .unwrap();
        app.panels
            .as_mut()
            .unwrap()
            .frame_outcome(voxy_render::RenderOutcome::Presented);
    }
    #[test]
    fn unpresented_panels_block_hits_and_cancel_held_activation_until_represented() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&path, false).unwrap();
        app.panels = Some(Panels::new().unwrap());
        build(&mut app);
        let original = app.authoring_document().unwrap();
        let rect = app
            .panels
            .as_ref()
            .unwrap()
            .regions
            .iter()
            .find(|(_, action)| *action == Action::Duplicate)
            .unwrap()
            .0;
        let point = Vec2::new(rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
        assert_eq!(app.panel_target(point).unwrap(), Some(Action::Duplicate));
        app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
            .unwrap();
        for outcome in [
            voxy_render::RenderOutcome::SkippedOccluded,
            voxy_render::RenderOutcome::SkippedTimeout,
            voxy_render::RenderOutcome::SkippedValidation,
            voxy_render::RenderOutcome::Suspended,
            voxy_render::RenderOutcome::Reconfigured,
        ] {
            app.panels.as_mut().unwrap().frame_outcome(outcome);
            assert!(!app.panels.as_ref().unwrap().input_ready());
            assert_eq!(app.panel_target(point).unwrap(), None);
            assert!(
                app.panel_key(KeyCode::Tab, ElementState::Pressed, false)
                    .unwrap()
            );
            assert!(
                app.panel_key(KeyCode::Enter, ElementState::Released, false)
                    .unwrap()
            );
            assert_eq!(app.authoring_document().unwrap(), original);
        }
        app.panels
            .as_mut()
            .unwrap()
            .frame_outcome(voxy_render::RenderOutcome::Presented);
        // A release or auto-repeat cannot resurrect a press captured before occlusion.
        app.panel_key(KeyCode::Enter, ElementState::Pressed, true)
            .unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Released, false)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        assert_eq!(app.panel_target(point).unwrap(), Some(Action::Duplicate));
        app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
            .unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Released, false)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap().objects.len(), 2);
        // Same IDs and inspector mode are insufficient if the authored data changed.
        build(&mut app);
        let mut transform = app.scene.local(app.instances[0]).unwrap();
        transform.translation.x += 1.0;
        app.scene.set_local(app.instances[0], transform).unwrap();
        app.commit_authoring().unwrap();
        assert_eq!(app.panel_target(point).unwrap(), None);
        assert!(
            app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
                .unwrap()
        );
        build(&mut app);
        assert!(app.panels.as_ref().unwrap().input_ready());
        let document = app.authoring_document().unwrap();
        app.panels
            .as_mut()
            .unwrap()
            .build(
                &document,
                app.selected,
                Vec2::new(1000.0, 700.0),
                false,
                None,
                0,
                false,
                app.inspector,
                "texture",
                "prefab",
                None,
            )
            .unwrap();
        assert!(!app.panels.as_ref().unwrap().input_ready());
        assert_eq!(app.panel_target(point).unwrap(), None);
        app.panels
            .as_mut()
            .unwrap()
            .frame_outcome(voxy_render::RenderOutcome::Presented);
        assert_eq!(app.panel_target(point).unwrap(), Some(Action::Duplicate));
        app.panels.as_mut().unwrap().invalidate_presentation();
        assert_eq!(app.panel_target(point).unwrap(), None);
        // Native shortcut dispatch cannot mutate history behind an unpresented panel.
        let before_hidden_undo = app.authoring_document().unwrap();
        app.authoring_input_key(KeyCode::KeyZ, None).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before_hidden_undo);
        build(&mut app);
        app.authoring_input_key(KeyCode::KeyZ, None).unwrap();
        assert_ne!(app.authoring_document().unwrap(), before_hidden_undo);
        build(&mut app);
        app.panel_action(Action::Field(0)).unwrap();
        app.field = Some((0, "2".into()));
        let before_enter = app.authoring_document().unwrap();
        app.panels.as_mut().unwrap().invalidate_presentation();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before_enter);
        assert!(app.field.is_some());
        app.authoring_input_key(KeyCode::Escape, None).unwrap();
        assert!(app.field.is_none());
        app.stop_workers().unwrap();
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn real_panels_and_native_key_adapter_share_focus_and_authoring_actions() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&path, false).unwrap();
        app.panels = Some(Panels::new().unwrap());
        for mode in [
            InspectorMode::Transform,
            InspectorMode::Physics,
            InspectorMode::Material,
            InspectorMode::Behavior,
            InspectorMode::Audio,
            InspectorMode::Mixer,
            InspectorMode::ImportSettings,
            InspectorMode::Components(0),
        ] {
            app.inspector = mode;
            build(&mut app);
        }
        app.inspector = InspectorMode::Transform;
        build(&mut app);
        let panels = app.panels.as_mut().unwrap();
        let rect = panels
            .regions
            .iter()
            .find(|(_, action)| *action == Action::Duplicate)
            .unwrap()
            .0;
        let duplicate_point = Vec2::new(rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
        assert_eq!(panels.hit(duplicate_point), Some(Action::Duplicate));
        assert_eq!(panels.hit(Vec2::new(-1.0, duplicate_point.y)), None);
        app.panel_key(KeyCode::Tab, ElementState::Pressed, false)
            .unwrap();
        let first = app.panels.as_ref().unwrap().focus.focused();
        app.panel_key(KeyCode::Tab, ElementState::Pressed, true)
            .unwrap();
        assert_eq!(app.panels.as_ref().unwrap().focus.focused(), first);
        app.panels
            .as_mut()
            .unwrap()
            .focus
            .pointer(Some(Action::Duplicate))
            .unwrap();
        let before = app.authoring_document().unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
            .unwrap();
        app.panel_key(KeyCode::Space, ElementState::Pressed, false)
            .unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Released, false)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.panel_key(KeyCode::Space, ElementState::Released, false)
            .unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects.len(),
            before.objects.len() + 1
        );
        // A shortcut changed the document before another frame: no old target.
        assert_eq!(app.panel_target(duplicate_point).unwrap(), None);
        app.panel_key(KeyCode::Tab, ElementState::Pressed, false)
            .unwrap();
        assert_eq!(app.panels.as_ref().unwrap().focus.focused(), None);
        build(&mut app);
        app.panels
            .as_mut()
            .unwrap()
            .focus
            .pointer(Some(Action::Field(0)))
            .unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
            .unwrap();
        app.panel_key(KeyCode::Enter, ElementState::Released, false)
            .unwrap();
        assert_eq!(app.field.as_ref().unwrap().0, 0);
        assert!(
            !app.panel_key(KeyCode::Enter, ElementState::Pressed, false)
                .unwrap()
        );
        app.field_key(KeyCode::Escape, None).unwrap();
        app.modifiers = winit::keyboard::ModifiersState::SHIFT;
        app.panel_key(KeyCode::Tab, ElementState::Pressed, false)
            .unwrap();
        assert!(app.panels.as_ref().unwrap().focus.focused().is_some());
        app.panels
            .as_mut()
            .unwrap()
            .focus
            .pointer(Some(Action::Duplicate))
            .unwrap();
        app.panel_key(KeyCode::Space, ElementState::Pressed, false)
            .unwrap();
        let authored = app.authoring_document().unwrap();
        app.toggle_play().unwrap();
        assert!(app.play.playing.is_some());
        let panels = app.panels.as_mut().unwrap();
        panels
            .build(
                &authored,
                app.selected,
                Vec2::new(1000.0, 700.0),
                true,
                None,
                0,
                false,
                app.inspector,
                "texture",
                "prefab",
                None,
            )
            .unwrap();
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        assert_eq!(panels.hit(duplicate_point), None);
        let rect = panels
            .regions
            .iter()
            .find(|(_, action)| *action == Action::Play)
            .unwrap()
            .0;
        assert_eq!(
            panels.hit(Vec2::new(rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)),
            Some(Action::Play)
        );
        let stop_point = Vec2::new(rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
        let mut runtime = app.scene.local(app.instances[0]).unwrap();
        runtime.translation.x += 10.0;
        app.scene.set_local(app.instances[0], runtime).unwrap();
        // Authoring panels show the saved authoring snapshot during Play, not runtime poses.
        assert_eq!(app.panel_target(stop_point).unwrap(), Some(Action::Play));
        app.panel_action(Action::Play).unwrap();
        build(&mut app);
        app.panel_key(KeyCode::Space, ElementState::Released, false)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), authored);
        assert_eq!(app.panels.as_ref().unwrap().focus.focused(), None);
        app.stop_workers().unwrap();
    }
}

#[cfg(test)]
mod ui_scene_integration {
    use crate::App;
    use voxy_gameplay::{SceneUiRuntime, UiElement};
    use winit::keyboard::KeyCode;
    #[test]
    fn ui_scene_validation_undo_and_play_restore_use_common_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&path, false).unwrap();
        let original = app.authoring_document().unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                UiElement {
                    origin: [0.1; 2],
                    size: [0.25; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("jump".into()),
                    text: None,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let authored = app.authoring_document().unwrap();
        app.scene
            .component_mut::<UiElement>(app.instances[0])
            .unwrap()
            .unwrap()
            .size[0] = 0.0;
        assert!(app.commit_authoring().is_err());
        assert_eq!(app.authoring_document().unwrap(), authored);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), authored);
        app.toggle_play().unwrap();
        let mut ui = SceneUiRuntime::new(128);
        ui.refresh(&app.scene, [1000.0, 800.0]).unwrap();
        ui.traverse(false);
        ui.key_press();
        assert_eq!(ui.key_release().unwrap().action, "jump");
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), authored);
        app.stop_workers().unwrap();
    }
}
