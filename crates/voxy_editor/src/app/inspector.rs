//! Inspector panels: panel documents, keys, actions and field editing.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    // Do not carry captured activation between authoring and runtime sessions.
    pub(crate) fn clear_panel_interactions(&mut self) {
        self.authoring.preview_seek = None;
        if let Some(panels) = &mut self.panels {
            panels.focus.cancel();
        }
        self.field = None;
        self.parenting = None;
    }

    pub(crate) fn panel_document(&self) -> Result<SceneDocument, Box<dyn std::error::Error>> {
        if let Some(draft) = &self.retarget_draft {
            return Ok(draft.document.clone());
        }
        if self.play.playing.is_some() {
            Ok(self
                .authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .current()
                .clone())
        } else {
            Ok(self.authoring_document()?)
        }
    }

    pub(crate) fn panel_target(
        &mut self,
        point: Vec2,
    ) -> Result<Option<panels::Action>, Box<dyn std::error::Error>> {
        let document = self.panel_document()?;
        let Some(panels) = &mut self.panels else {
            return Ok(None);
        };
        if !panels
            .focus
            .matches_context(&document, self.selected, self.inspector)
        {
            panels.focus.cancel();
            self.panel_cache = None;
            return Ok(None);
        }
        let action = panels.hit(point);
        panels.focus.pointer(action)?;
        if action == Some(panels::Action::PreviewSeek) {
            let rect = panels
                .regions
                .iter()
                .find(|(_, action)| *action == panels::Action::PreviewSeek)
                .ok_or("missing preview seek region")?
                .0;
            return Ok(Some(panels::Action::PreviewPhase(f64::from(
                ((point.x - rect[0]) / rect[2]).clamp(0., 1.),
            ))));
        }
        Ok(action)
    }

    pub(crate) fn panel_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.standalone
            || self.play.playing.is_some()
            || self.field.is_some()
            || !matches!(key, KeyCode::Tab | KeyCode::Enter | KeyCode::Space)
            || self.panels.is_none()
        {
            return Ok(false);
        }
        let document = self.panel_document()?;
        let panels = self.panels.as_mut().ok_or("missing panels")?;
        if !panels
            .focus
            .matches_context(&document, self.selected, self.inspector)
        {
            panels.focus.cancel();
            // The next frame rebuilds geometry and targets; stale indices cannot
            // dispatch before that frame after a shortcut changes the hierarchy.
            self.panel_cache = None;
            return Ok(true);
        }
        if !panels.input_ready() {
            panels.focus.cancel();
            return Ok(true);
        }
        let action = match key {
            KeyCode::Tab => {
                if state == ElementState::Pressed && !repeat {
                    panels.focus.traverse(self.modifiers.shift_key());
                }
                None
            }
            KeyCode::Enter | KeyCode::Space => panels.focus.activate(
                if key == KeyCode::Enter { 1 } else { 2 },
                state == ElementState::Pressed,
                repeat,
            ),
            _ => None,
        };
        if let Some(action) = action {
            self.panel_action(action)?;
        }
        Ok(true)
    }

    pub(crate) fn authoring_input_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_none()
            && key != KeyCode::Escape
            && self
                .panels
                .as_ref()
                .is_some_and(|panels| !panels.input_ready())
        {
            if self.trace.enabled {
                println!(
                    "EDITOR KEY REJECTED frames={} key={key:?} reason=panels-not-presented",
                    self.frames
                );
            }
            return Ok(());
        }
        if self.field.is_some() {
            self.field_key(key, text)
        } else {
            self.edit_key(key)
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn edit_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some() {
            if key == KeyCode::Escape {
                self.cancel_retarget();
                return Ok(());
            }
            return Err("apply or cancel the retarget profile edit first".into());
        }
        if self.standalone {
            return Ok(());
        }
        if self.trace.enabled {
            println!("EDITOR KEY frames={} key={key:?}", self.frames);
        }
        self.trace.pending = true;
        self.trace.last_outcome = None;
        self.finish_drag(false)?;
        if key == KeyCode::KeyH && self.play.playing.is_none() {
            return self.expand_model();
        }
        if key == KeyCode::F10 {
            let enabled = !self.msaa4;
            if let Some(graphics) = &mut self.graphics {
                if enabled {
                    pollster::block_on(graphics.renderer.enable_msaa4(graphics.host.device()))?;
                    graphics.host.enable_msaa4()?;
                } else {
                    graphics.host.disable_msaa4();
                }
            }
            self.msaa4 = enabled;
            if let Some(panels) = &mut self.panels {
                panels.invalidate_presentation();
            }
            self.update_edit_title();
            return Ok(());
        }
        if key == KeyCode::F4 && self.play.playing.is_none() {
            self.camera_drag = None;
            if self.secondary_camera.take().is_none() {
                let mut other = self.camera.clone();
                other.legacy = false;
                other.perspective = !self.camera.perspective;
                self.secondary_camera = Some(other);
            }
            self.active_view = 0;
            if let Some(graphics) = &mut self.graphics {
                graphics.lod_history.clear();
            }
            if let Some(panels) = &mut self.panels {
                panels.invalidate_presentation();
            }
            self.update_edit_title();
            return Ok(());
        }
        if key == KeyCode::F7 {
            self.camera.legacy = false;
            self.camera.perspective = !self.camera.perspective;
            return Ok(());
        }
        if key == KeyCode::F8 {
            self.camera = camera::ViewportCamera::default();
            return Ok(());
        }
        if key == KeyCode::KeyF {
            if let Some(node) = self.instances.get(self.selected) {
                self.camera.target = self.scene.world_matrix(*node)?.w_axis.truncate();
                self.camera.legacy = false;
            }
            return Ok(());
        }
        if key == KeyCode::F6 {
            return self.toggle_play();
        }
        if self.play.playing.is_some() {
            return Ok(());
        }
        match key {
            KeyCode::F5 if self.inspector == InspectorMode::ImportSettings => {
                return self.save_audio_settings();
            }
            KeyCode::F9 if self.inspector == InspectorMode::ImportSettings => {
                return self.open_audio_settings();
            }
            KeyCode::F5 => return self.save_authoring(),
            KeyCode::F9 => return self.load_authoring(),
            _ => {}
        }
        if key == KeyCode::Escape {
            self.parenting = None;
            return Ok(());
        }
        if matches!(key, KeyCode::KeyZ | KeyCode::KeyY) {
            self.parenting = None;
            return self.history_key(key);
        }
        if key != KeyCode::KeyP {
            self.parenting = None;
        }
        let node = self.instances.get(self.selected).copied();
        if node.is_none() && key != KeyCode::KeyD {
            return Ok(());
        }
        let mut local = node
            .map(|node| self.scene.local(node))
            .transpose()?
            .unwrap_or(Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                ..Transform::default()
            });
        match key {
            KeyCode::KeyG => {
                let node = node.ok_or("missing selected node")?;
                let owners: Vec<_> = self
                    .scene
                    .components::<EditorCamera>()
                    .map(|(owner, _)| owner)
                    .collect();
                for owner in owners {
                    self.scene.remove_component::<EditorCamera>(owner)?;
                }
                self.scene.insert_component(node, self.camera.clone())?;
            }
            KeyCode::KeyM => {
                self.inspector = if self.inspector == InspectorMode::Material {
                    InspectorMode::Transform
                } else {
                    InspectorMode::Material
                };
                self.field = None;
                if self.inspector == InspectorMode::Material
                    && let Some(node) = node
                    && self.scene.component::<SceneMaterial>(node)?.is_none()
                {
                    self.scene
                        .insert_component(node, SceneMaterial::default())?;
                } else {
                    return Ok(());
                }
            }
            KeyCode::KeyL => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<DirectionalLight>(node)?.is_some() {
                    self.scene.remove_component::<DirectionalLight>(node)?;
                } else {
                    self.scene
                        .insert_component(node, DirectionalLight::default())?;
                }
            }
            KeyCode::KeyI => {
                self.inspector = if self.inspector == InspectorMode::Physics {
                    InspectorMode::Transform
                } else {
                    InspectorMode::Physics
                };
                self.field = None;
                return Ok(());
            }
            KeyCode::KeyC => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<CharacterBody>(node)?.is_some() {
                    self.scene.remove_component::<CharacterBody>(node)?;
                } else {
                    self.scene.remove_component::<BoxCollider>(node)?;
                    self.scene
                        .insert_component(node, CharacterBody::default())?;
                }
            }
            KeyCode::KeyB => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<BoxCollider>(node)?.is_some() {
                    self.scene.remove_component::<BoxCollider>(node)?;
                } else {
                    self.scene.remove_component::<CharacterBody>(node)?;
                    self.scene.insert_component(node, BoxCollider::default())?;
                }
            }
            KeyCode::KeyP => return self.panel_action(panels::Action::Parent),
            KeyCode::KeyA => {
                let node = node.ok_or("missing selected node")?;
                self.scene
                    .set_active(node, !self.scene.active_self(node)?)?;
            }
            KeyCode::KeyR => {
                if let Some(node) = node {
                    self.scene.remove_component::<ModelPart>(node)?;
                }
                let node = node.ok_or("missing selected node")?;
                let current = &self
                    .scene
                    .component::<ModelInstance>(node)?
                    .ok_or("missing model")?
                    .asset;
                let next = self
                    .available
                    .range((
                        std::ops::Bound::Excluded(current),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .or_else(|| self.available.first())
                    .ok_or("empty resources")?
                    .clone();
                self.scene.insert_component(
                    node,
                    ModelInstance {
                        asset: next.clone(),
                    },
                )?;
                if self.catalog.snapshot(&next).is_none() {
                    self.reload.insert(next);
                }
            }
            KeyCode::KeyD => {
                if self.instances.len() >= scene_limits::OBJECTS {
                    return Ok(());
                }
                if node.is_some() {
                    local.translation.x += 0.15;
                }
                let next = self
                    .authoring
                    .next_object_id
                    .checked_add(1)
                    .ok_or("object identity exhausted")?;
                let parent = node
                    .map(|node| self.scene.parent(node))
                    .transpose()?
                    .flatten();
                let duplicate = self.scene.spawn(parent, local)?;
                let model = node
                    .and_then(|node| self.scene.component::<ModelInstance>(node).ok().flatten())
                    .map(|model| model.asset.clone())
                    .or_else(|| {
                        if node.is_none() {
                            self.id.clone().or_else(|| self.available.first().cloned())
                        } else {
                            None
                        }
                    });
                let is_model = model.is_some();
                if let Some(asset) = model {
                    self.scene
                        .insert_component(duplicate, ModelInstance { asset })?;
                }
                if let Some(source) = node {
                    self.authoring
                        .authoring_project
                        .registry
                        .copy_registered_components(&mut self.scene, source, duplicate)?;
                    self.scene
                        .set_active(duplicate, self.scene.active_self(source)?)?;
                }
                self.scene.set_name(
                    duplicate,
                    format!(
                        "{} {}",
                        if is_model { "Model" } else { "Object" },
                        self.authoring.next_object_id + 1
                    ),
                )?;
                self.instances.push(duplicate);
                self.object_ids.push(ObjectId(format!(
                    "{}-{}",
                    if is_model { "model" } else { "object" },
                    self.authoring.next_object_id
                )));
                self.authoring.next_object_id = next;
                self.selected = self.instances.len() - 1;
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.scene
                    .remove_subtree(node.ok_or("missing selected node")?)?;
                let surviving: Vec<_> = self
                    .instances
                    .iter()
                    .copied()
                    .zip(self.object_ids.iter().cloned())
                    .filter(|(node, _)| self.scene.local(*node).is_ok())
                    .collect();
                (self.instances, self.object_ids) = surviving.into_iter().unzip();
                self.selected = self.selected.min(self.instances.len().saturating_sub(1));
                if let Some(graphics) = &mut self.graphics {
                    graphics.transforms.clear();
                }
            }
            KeyCode::Tab => self.selected = (self.selected + 1) % self.instances.len(),
            KeyCode::ArrowLeft => {
                local.translation.x -= 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowRight => {
                local.translation.x += 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowUp => {
                local.translation.y += 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowDown => {
                local.translation.y -= 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            _ => return Ok(()),
        }
        if key != KeyCode::Tab {
            self.commit_authoring()?;
        }
        self.update_edit_title();
        Ok(())
    }

    pub(crate) fn panel_action(
        &mut self,
        action: panels::Action,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some()
            && !matches!(
                action,
                panels::Action::Field(_)
                    | panels::Action::ComponentPage(_)
                    | panels::Action::RetargetApply
                    | panels::Action::RetargetCancel
                    | panels::Action::RetargetRemove
                    | panels::Action::RetargetPair(_)
                    | panels::Action::RetargetDeletePair(_)
                    | panels::Action::RetargetBones(_)
                    | panels::Action::RetargetBone(_)
                    | panels::Action::RetargetBonePage(_)
                    | panels::Action::RetargetBoneClose
            )
        {
            return Err("apply or cancel the retarget profile edit first".into());
        }
        self.finish_drag(false)?;
        self.trace.pending = true;
        self.trace.last_outcome = None;
        self.field = None;
        self.component_edit = None;
        if !matches!(action, panels::Action::Select(_) | panels::Action::Parent) {
            self.parenting = None;
        }
        if self.play.playing.is_some() && action != panels::Action::Play {
            return Ok(());
        }
        match action {
            panels::Action::Select(index) => {
                if index < self.instances.len() {
                    if let Some(node) = self.parenting.take() {
                        self.scene.reparent(node, Some(self.instances[index]))?;
                        self.commit_authoring()?;
                    } else {
                        self.selected = index;
                    }
                    self.update_edit_title();
                }
                Ok(())
            }
            panels::Action::Field(index) => {
                if matches!(self.inspector, InspectorMode::Components(_)) {
                    let document = self.panel_document()?;
                    let object = document
                        .objects
                        .get(self.selected)
                        .ok_or("missing selected object")?;
                    let field = component_fields::fields(object)?
                        .into_iter()
                        .nth(index)
                        .ok_or("missing component field")?;
                    self.component_edit =
                        Some(field.bind(object, &self.authoring.authoring_project.registry)?);
                }
                self.field = Some((index, String::new()));
                Ok(())
            }
            panels::Action::ResetField(index) => self.reset_prefab_field(index),
            panels::Action::PreviewSeek => self.preview_animation_phase(0.5, false),
            panels::Action::PreviewPhase(phase) => self.preview_animation_phase(phase, false),
            panels::Action::MarkerAdd(token) => self.edit_animation_marker(None, token),
            panels::Action::MarkerPreview(index, token) => {
                self.preview_animation_marker(index, token)
            }
            panels::Action::MarkerDelete(index, token) => {
                self.edit_animation_marker(Some(index), token)
            }
            panels::Action::Duplicate => self.edit_key(KeyCode::KeyD),
            panels::Action::Delete => self.edit_key(KeyCode::Delete),
            panels::Action::Resource => self.edit_key(KeyCode::KeyR),
            panels::Action::Parent => {
                if let Some(node) = self.instances.get(self.selected).copied() {
                    if self.scene.parent(node)?.is_some() {
                        self.scene.reparent(node, None)?;
                        self.commit_authoring()?;
                    } else {
                        self.parenting = Some(node);
                    }
                }
                Ok(())
            }
            panels::Action::Character => self.edit_key(KeyCode::KeyC),
            panels::Action::Collider => self.edit_key(KeyCode::KeyB),
            panels::Action::AudioSource => self.toggle_audio_source(),
            panels::Action::AudioListener => self.toggle_audio_listener(),
            panels::Action::AudioBus => self.toggle_audio_bus(),
            panels::Action::AudioSettingsLoad => self.open_audio_settings(),
            panels::Action::AudioSettingsSave => self.save_audio_settings(),
            panels::Action::RetargetBones(index) => self.open_retarget_bones(index),
            panels::Action::RetargetBone(index) => self.choose_retarget_bone(index),
            panels::Action::RetargetBonePage(forward) => self.retarget_bone_page(forward),
            panels::Action::RetargetBoneClose => {
                self.retarget_picker = None;
                self.panel_cache = None;
                Ok(())
            }
            panels::Action::Retarget => self.begin_retarget(),
            panels::Action::RetargetApply => self.apply_retarget(),
            panels::Action::RetargetCancel => {
                self.cancel_retarget();
                Ok(())
            }
            panels::Action::RetargetPair(add) => self.retarget_pair(add),
            panels::Action::RetargetDeletePair(index) => self.delete_retarget_pair(index),
            panels::Action::RetargetRemove => {
                self.remove_retarget_draft()?;
                Ok(())
            }
            panels::Action::Animation => {
                if self.play.playing.is_some() {
                    return Err("stop play before editing animation".into());
                }
                let node = self
                    .instances
                    .get(self.selected)
                    .copied()
                    .ok_or("missing selected model")?;
                if self.scene.component::<ModelAnimation>(node)?.is_some() {
                    self.scene.remove_component::<ModelAnimation>(node)?;
                } else {
                    self.scene
                        .insert_component(node, ModelAnimation::default())?;
                }
                self.commit_authoring()?;
                self.inspector = InspectorMode::Components(0);
                self.field = None;
                Ok(())
            }
            panels::Action::Behavior => {
                self.inspector = match self.inspector {
                    InspectorMode::Behavior | InspectorMode::ImportSettings => InspectorMode::Audio,
                    InspectorMode::Audio => InspectorMode::Mixer,
                    InspectorMode::Mixer => InspectorMode::Components(0),
                    InspectorMode::Components(_) => InspectorMode::Collections(0, 0),
                    InspectorMode::Collections(_, _) => InspectorMode::Transform,
                    _ => InspectorMode::Behavior,
                };
                self.field = None;
                Ok(())
            }
            panels::Action::ComponentPage(forward) => {
                if let InspectorMode::Components(page) = self.inspector {
                    let document = self.panel_document()?;
                    let object = document
                        .objects
                        .get(self.selected)
                        .ok_or("missing selected object")?;
                    let count = component_fields::fields(object)?.len();
                    let pages = count.div_ceil(6).max(1);
                    self.inspector = InspectorMode::Components(if forward {
                        (page + 1) % pages
                    } else {
                        (page + pages - 1) % pages
                    });
                }
                Ok(())
            }
            panels::Action::CollectionChoice => self.collection_page(true, false),
            panels::Action::CollectionPage(forward) => self.collection_page(false, forward),
            panels::Action::CollectionAdd(key) => self.collection_action(key, None, None),
            panels::Action::CollectionDelete(key, item) => {
                self.collection_action(key, Some(item), None)
            }
            panels::Action::CollectionMove(key, item, forward) => {
                self.collection_action(key, Some(item), Some(forward))
            }
            panels::Action::CollectionReset(key, item) => {
                self.reset_prefab_collection_item(key, item)
            }
            panels::Action::CollectionResetOrder(key) => self.reset_prefab_collection_order(key),
            panels::Action::CollectionRestoreDeleted(key) => {
                self.restore_prefab_collection_deleted(key)
            }
            panels::Action::CollectionRestoreItem(key, item) => {
                self.restore_prefab_collection_item(key, item)
            }
            panels::Action::CollectionDeletedPage => {
                let panels = self.panels.as_mut().ok_or("missing panels")?;
                panels.deleted_page = panels.deleted_page.wrapping_add(1);
                panels.invalidate_presentation();
                self.panel_cache = None;
                Ok(())
            }
            panels::Action::Motion => {
                let node = *self
                    .instances
                    .get(self.selected)
                    .ok_or("missing selected node")?;
                if self
                    .scene
                    .component::<voxy_gameplay::AngularMotion>(node)?
                    .is_some()
                {
                    self.scene
                        .remove_component::<voxy_gameplay::AngularMotion>(node)?;
                } else {
                    self.scene.insert_component(
                        node,
                        voxy_gameplay::AngularMotion {
                            axis: [0., 1., 0.],
                            radians_per_second: 1.,
                        },
                    )?;
                }
                self.commit_authoring()
            }
            panels::Action::Physics => self.edit_key(KeyCode::KeyI),
            panels::Action::Active => self.edit_key(KeyCode::KeyA),
            panels::Action::Play => self.edit_key(KeyCode::F6),
            panels::Action::Save => self.edit_key(KeyCode::F5),
            panels::Action::Load => self.edit_key(KeyCode::F9),
            panels::Action::RevertPrefab => self.revert_prefab(),
            panels::Action::PlacePrefab => self.place_prefab(),
            panels::Action::CreatePrefab => self.create_prefab_from_selection(),
            panels::Action::PrefabChoice => {
                self.authoring.prefab_assets = self.authoring.authoring_project.prefabs()?;
                if !self.authoring.prefab_assets.is_empty() {
                    self.authoring.prefab_choice =
                        (self.authoring.prefab_choice + 1) % self.authoring.prefab_assets.len();
                }
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn field_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if key == KeyCode::Escape {
            self.field = None;
            return Ok(());
        }
        if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter)
            && matches!(self.inspector, InspectorMode::Components(_))
        {
            let (_, text) = self.field.clone().ok_or("missing component field")?;
            let binding = self
                .component_edit
                .clone()
                .ok_or("missing component field binding")?;
            return self.commit_component_binding(&binding, &text);
        }
        let Some((index, value)) = &mut self.field else {
            return Ok(());
        };
        if key == KeyCode::Backspace {
            value.pop();
            return Ok(());
        }
        if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter) {
            self.trace.pending = true;
            self.trace.last_outcome = None;
            if self.inspector == InspectorMode::Audio && matches!(*index, 6 | 7) {
                let node = *self
                    .instances
                    .get(self.selected)
                    .ok_or("missing selected node")?;
                let mut source = self
                    .scene
                    .component::<voxy_gameplay::AudioSource>(node)?
                    .cloned()
                    .ok_or("selected object has no audio source")?;
                if *index == 6 {
                    source.asset.clone_from(value);
                } else {
                    source.import_settings = if value.is_empty() {
                        None
                    } else {
                        Some(value.clone())
                    };
                }
                if !source.valid() {
                    return Err("invalid audio asset ID".into());
                }
                self.scene.insert_component(node, source)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::ImportSettings {
                let index = *index;
                let text = value.clone();
                return self.edit_audio_settings(index, &text);
            }
            let value: f32 = value.parse()?;
            if !value.is_finite() {
                return Err("property must be finite".into());
            }
            let node = *self
                .instances
                .get(self.selected)
                .ok_or("missing selected node")?;
            if self.inspector == InspectorMode::Mixer {
                let mut bus = self
                    .scene
                    .component::<voxy_gameplay::AudioBus>(node)?
                    .copied()
                    .ok_or("selected object has no audio bus")?;
                match *index {
                    0 if value.fract().abs() < f32::EPSILON && (0.0..=15.0).contains(&value) => {
                        bus.bus = value.to_string().parse()?;
                    }
                    1 => bus.gain = value,
                    _ => return Err("audio mixer requires bus 0..15".into()),
                }
                if !bus.valid() {
                    return Err("audio bus gain must be 0..1".into());
                }
                self.scene.insert_component(node, bus)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Audio {
                let mut source = self
                    .scene
                    .component::<voxy_gameplay::AudioSource>(node)?
                    .cloned()
                    .ok_or("selected object has no audio source")?;
                match *index {
                    0 => source.gain = value,
                    1 if value.fract().abs() < f32::EPSILON && (0.0..=15.0).contains(&value) => {
                        source.bus = value.to_string().parse()?;
                    }
                    2 | 3 if value == 0. || value.to_bits() == 1_f32.to_bits() => {
                        let enabled = value.to_bits() == 1_f32.to_bits();
                        if *index == 2 {
                            source.looping = enabled;
                        } else {
                            source.spatial = enabled;
                        }
                    }
                    4 => source.near = value,
                    5 => source.far = value,
                    _ => return Err("audio requires bus 0..15 and toggle 0/1".into()),
                }
                if !source.valid() {
                    return Err("invalid audio gain or attenuation range".into());
                }
                self.scene.insert_component(node, source)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Material {
                let index = *index;
                let mut material = self
                    .scene
                    .component::<SceneMaterial>(node)?
                    .copied()
                    .unwrap_or_default();
                match index {
                    0..=3 => material.tint[index] = value,
                    4 if value == 0. || value.to_bits() == 1_f32.to_bits() => {
                        material.lit = value.to_bits() == 1_f32.to_bits();
                    }
                    _ => return Err("material field requires RGBA [0,1], Lit 0/1".into()),
                }
                if !material.valid() {
                    return Err("opaque material requires RGB in [0,1] and alpha 1".into());
                }
                self.scene.insert_component(node, material)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Behavior {
                let index = *index;
                let mut motion = self
                    .scene
                    .component::<voxy_gameplay::AngularMotion>(node)?
                    .copied()
                    .ok_or("selected object has no angular motion")?;
                match index {
                    0..=2 => motion.axis[index] = value,
                    3 => motion.radians_per_second = f64::from(value),
                    _ => return Err("unknown angular motion property".into()),
                }
                if !motion.valid() {
                    return Err(
                        "motion requires a nonzero finite axis and bounded finite rate".into(),
                    );
                }
                self.scene.insert_component(node, motion)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Physics {
                let index = *index;
                if let Some(mut body) = self.scene.component::<CharacterBody>(node)?.copied() {
                    match index {
                        0..=2 => body.half_extents[index] = value,
                        3 => body.speed = f64::from(value),
                        4 => body.gravity = f64::from(value),
                        5 => body.jump_speed = f64::from(value),
                        _ => return Err("unknown character property".into()),
                    }
                    self.scene.insert_component(node, body)?;
                } else if let Some(mut collider) =
                    self.scene.component::<BoxCollider>(node)?.copied()
                {
                    if index > 2 {
                        return Err("unknown collider property".into());
                    }
                    collider.half_extents[index] = value;
                    self.scene.insert_component(node, collider)?;
                } else {
                    return Err("selected object has no physics component".into());
                }
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            let original = self.scene.local(node)?;
            let mut local = original;
            match *index {
                0..=2 => local.translation[*index] = value,
                3..=5 => {
                    let (x, y, z) = local.rotation.to_euler(glam::EulerRot::XYZ);
                    let mut rotation = [x, y, z];
                    rotation[*index - 3] = value.to_radians();
                    local.rotation = glam::Quat::from_euler(
                        glam::EulerRot::XYZ,
                        rotation[0],
                        rotation[1],
                        rotation[2],
                    );
                }
                6..=8 => local.scale[*index - 6] = value,
                _ => return Err("unknown transform field".into()),
            }
            self.scene.set_local(node, local)?;
            self.commit_authoring()?;
            self.field = None;
            return Ok(());
        }
        if matches!(self.inspector, InspectorMode::Components(_))
            || self.inspector == InspectorMode::Audio && matches!(*index, 6 | 7)
        {
            if let Some(text) = text
                && value.len() + text.len() <= 1024
                && !text.chars().any(char::is_control)
            {
                value.push_str(text);
            }
            return Ok(());
        }
        if let Some(text) = text
            && value.len() + text.len() <= 32
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        {
            value.push_str(text);
        }
        Ok(())
    }

    pub(crate) fn expand_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("no selected model")?;
        if self.scene.component::<ModelPart>(owner)?.is_some() {
            return Err("model hierarchy already expanded".into());
        }
        let id = self
            .scene
            .component::<ModelInstance>(owner)?
            .ok_or("missing model")?
            .asset
            .clone();
        let asset = self.catalog.snapshot(&id).ok_or("model still loading")?;
        if asset.value().nodes.is_empty() {
            return Err("selected asset has no glTF hierarchy".into());
        }
        let mut document = self.authoring_document()?;
        if document.objects.len() + asset.value().nodes.len() > scene_limits::OBJECTS {
            return Err("scene capacity exceeded".into());
        }
        let old_id = document.objects[self.selected].id.clone();
        // Keep the selected object as a container; its part index is one past the nodes.
        document.objects[self.selected].components.insert(
            "editor.model-part.v1".into(),
            serde_json::to_value(ModelPart { node: u32::MAX })?,
        );
        let next = self
            .authoring
            .next_object_id
            .checked_add(u64::try_from(asset.value().nodes.len())?)
            .ok_or("object identity exhausted")?;
        let ids: Vec<_> = (self.authoring.next_object_id..next)
            .map(|index| ObjectId(format!("model-{index}")))
            .collect();
        for (index, node) in asset.value().nodes.iter().enumerate() {
            document.objects.push(SceneObject {
                id: ids[index].clone(),
                parent: Some(
                    node.parent
                        .map_or_else(|| old_id.clone(), |parent| ids[parent].clone()),
                ),
                name: node.name.clone(),
                active: true,
                translation: node.local.translation.to_array(),
                rotation: node.local.rotation.to_array(),
                scale: node.local.scale.to_array(),
                components: BTreeMap::from([
                    (
                        "editor.model.v1".into(),
                        serde_json::Value::String(id.0.clone()),
                    ),
                    (
                        "editor.model-part.v1".into(),
                        serde_json::to_value(ModelPart {
                            node: u32::try_from(index)?,
                        })?,
                    ),
                ]),
            });
        }
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.authoring.next_object_id = next;
        self.restore_authoring()?;
        Ok(())
    }
}
