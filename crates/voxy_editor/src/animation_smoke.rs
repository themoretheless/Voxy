//! Native acceptance uses presented frames and the ordinary editor Play/Stop path.
use crate::App;
use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub(super) struct Smoke {
    pub(super) profile: bool,
    pub(super) root_motion: bool,
    phase: u8,
    since: u64,
    authoring: Option<voxy_scene::SceneDocument>,
    lod_bytes: u64,
    base_indices: u32,
}
impl App {
    fn animation_inspector_input(
        &mut self,
        path: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let document = self.authoring_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing inspector owner")?;
        let index = crate::component_fields::fields(object)?
            .iter()
            .position(|field| field.schema == "editor.model-animation.v1" && field.path == path)
            .ok_or("missing animation inspector field")?;
        self.inspector = crate::InspectorMode::Components(index / 6);
        self.panel_action(crate::panels::Action::Field(index))?;
        self.field_key(KeyCode::Digit0, Some(text))?;
        self.field_key(KeyCode::Enter, None)
    }
    pub(super) fn animation_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(smoke) = &self.animation_smoke else {
            return Ok(false);
        };
        if self.frames < smoke.since + 3 {
            return Ok(false);
        }
        match smoke.phase {
            0 => {
                let Some(asset) = self.catalog.snapshot(&self.id) else {
                    return Ok(false);
                };
                let model = asset
                    .value()
                    .animated
                    .as_ref()
                    .ok_or("animation acceptance requires an animated model")?;
                if model.animations.is_empty() {
                    return Err("animation acceptance requires a clip".into());
                }
                let clip_count = model.animations.len();
                let motion_name = model.joint_names().iter().flatten()
                    .find(|name| name.starts_with("b_Hip_01"))
                    .or_else(|| model.joint_names().get(1).and_then(Option::as_ref))
                    .filter(|name| model.resolve_joint_name(name).is_ok()).map(|name| name.to_string());
                if self
                    .graphics
                    .as_ref()
                    .is_none_or(|g| !g.models.contains_key(&self.id))
                {
                    return Ok(false);
                }
                if asset.value().skinned_lod.is_some() {
                    self.camera.legacy = false;
                    self.camera.perspective = false;
                    self.camera.distance = 2000.;
                }
                self.edit_key(KeyCode::KeyD)?;
                if self.instances.len() != 2 {
                    return Err("native animation setup did not duplicate the model".into());
                }
                self.panel_action(crate::panels::Action::Select(0))?;
                self.panel_action(crate::panels::Action::Animation)?;
                let root_before = self.authoring_document()?;
                self.animation_inspector_input("/root_motion_joint", "1")?;
                let root_selected = self.authoring_document()?;
                if self.scene.component::<crate::ModelAnimation>(self.instances[0])?
                    .is_none_or(|settings| settings.root_motion_joint != 1) {
                    return Err("native inspector did not select motion joint".into());
                }
                self.edit_key(KeyCode::KeyZ)?;
                if self.authoring_document()? != root_before {
                    return Err("native motion joint undo failed".into());
                }
                self.edit_key(KeyCode::KeyY)?;
                if self.authoring_document()? != root_selected {
                    return Err("native motion joint redo failed".into());
                }
                println!("VOXY_NATIVE_ROOT_JOINT selected=1 undo=true redo=true");
                if let Some(name) = motion_name {
                    self.animation_inspector_input("/root_motion_bone", &name)?;
                    let named = self.authoring_document()?;
                    self.edit_key(KeyCode::KeyZ)?;
                    if self.authoring_document()? != root_selected {
                        return Err("native named motion bone undo failed".into());
                    }
                    self.edit_key(KeyCode::KeyY)?;
                    if self.authoring_document()? != named {
                        return Err("native named motion bone redo failed".into());
                    }
                    println!("VOXY_NATIVE_ROOT_NAME name={name:?} undo=true redo=true");
                }
                let before = self.authoring_document()?;
                self.animation_inspector_input("/clip", "null")?;
                let bind = self.authoring_document()?;
                if bind == before {
                    return Err("native inspector did not switch to bind pose".into());
                }
                self.edit_key(KeyCode::KeyZ)?;
                if self.authoring_document()? != before {
                    return Err("native animation inspector undo failed".into());
                }
                self.edit_key(KeyCode::KeyY)?;
                if self.authoring_document()? != bind {
                    return Err("native animation inspector redo failed".into());
                }
                for clip in 0..clip_count {
                    self.animation_inspector_input("/clip", &clip.to_string())?;
                    let selected = self
                        .scene
                        .component::<crate::ModelAnimation>(self.instances[0])?
                        .ok_or("missing animation after clip switch")?;
                    if selected.clip != Some(clip) {
                        return Err("native inspector clip switch failed".into());
                    }
                }
                println!(
                    "ANIMATION NATIVE CLIPS PASS clips={clip_count} active_clip={}",
                    clip_count - 1
                );
                self.panel_action(crate::panels::Action::Select(1))?;
                self.panel_action(crate::panels::Action::Animation)?;
                self.animation_inspector_input("/speed", "0")?;
                let paused = self
                    .scene
                    .component::<crate::ModelAnimation>(self.instances[1])?
                    .ok_or("native inspector failed to author animation")?;
                if paused.speed != 0.0 || paused.clip != Some(0) {
                    return Err("native inspector failed to pause selected owner".into());
                }
                println!(
                    "ANIMATION NATIVE INSPECTOR PASS clip_bind_clip=true pause=true undo_redo=true"
                );
                if self.animation_smoke.as_ref().unwrap().root_motion {
                    self.panel_action(crate::panels::Action::Select(0))?;
                    self.edit_key(KeyCode::KeyC)?;
                    let body = self.scene.component_mut::<voxy_gameplay::CharacterBody>(self.instances[0])?
                        .ok_or("missing native root-motion body")?;
                    body.gravity = 0.0;
                    body.speed = 0.0;
                    self.commit_authoring()?;
                    self.animation_inspector_input("/root_motion_joint", "0")?;
                    self.animation_inspector_input("/root_motion_bone", "root")?;
                    self.animation_inspector_input("/root_motion_axes/0", "true")?;
                    self.panel_action(crate::panels::Action::Select(1))?;
                    self.edit_key(KeyCode::KeyB)?;
                    let mut local = self.scene.local(self.instances[1])?;
                    local.translation = self.scene.local(self.instances[0])?.translation + glam::Vec3::X * 0.15;
                    self.scene.set_local(self.instances[1], local)?;
                    self.commit_authoring()?;
                }
                let authoring = self.authoring_document()?;
                self.toggle_play()?;
                let smoke = self.animation_smoke.as_mut().unwrap();
                smoke.authoring = Some(authoring);
                smoke.phase = 1;
                smoke.since = self.frames;
            }
            1 => {
                if self.play.simulation_ticks < if smoke.profile { 120 } else { 12 } {
                    return Ok(false);
                }
                let graphics = self
                    .graphics
                    .as_mut()
                    .ok_or("missing native animation graphics")?;
                let first = *self.instances.first().ok_or("missing moving owner")?;
                let second = *self.instances.get(1).ok_or("missing paused owner")?;
                let Some(moving) = graphics.animated_models.pose_signature(first)? else {
                    return Ok(false);
                };
                let Some(paused) = graphics.animated_models.pose_signature(second)? else {
                    return Ok(false);
                };
                if smoke.root_motion {
                    let position = self.scene.local(first)?.translation;
                    if (position.x - 0.05).abs() > 1e-5 || moving != paused {
                        return Err(format!("native root motion failed wall/in-place admission: {position:?}").into());
                    }
                    let requested = self.play.animations.motions().iter()
                        .find(|(owner, _)| *owner == first).ok_or("missing native root motion request")?.1;
                    println!("VOXY_NATIVE_ROOT_PHYSICS x={} wall_limited=true in_place=true fixed_serial={} requested={requested:?}",
                        position.x, self.play.animations.serial());
                } else if moving == paused {
                    return Err("native owners failed to independently animate/pause".into());
                }
                let (owners, gpu_primitives, sources) = graphics.animated_models.counts();
                if owners != 2 {
                    return Err("native animation retained the wrong owner count".into());
                }
                if gpu_primitives > 0 && sources * 2 != gpu_primitives {
                    return Err("native animation sources were not shared".into());
                }
                let mut textured = 0;
                if let Some(model) = graphics
                    .models
                    .get(&self.id)
                    .and_then(|m| m.animated_model.as_ref())
                {
                    for (primitive, material) in model.primitives.iter().enumerate() {
                        if material.base_color_texture.is_some() {
                            let first_texture = graphics
                                .animated_models
                                .texture(first, primitive)
                                .ok_or("moving owner's texture missing")?;
                            let second_texture = graphics
                                .animated_models
                                .texture(second, primitive)
                                .ok_or("paused owner's texture missing")?;
                            if !std::ptr::eq(first_texture, second_texture) {
                                return Err("native owners did not share texture binding".into());
                            }
                            textured += 1;
                        }
                    }
                }
                println!(
                    "ANIMATION NATIVE TEXTURES PASS textured_primitives={textured} shared_bindings=true"
                );
                println!(
                    "ANIMATION NATIVE PLAY PASS frames={} ticks={} owners={owners} gpu_primitives={gpu_primitives} shared_sources={sources} bytes={}",
                    self.frames,
                    self.play.simulation_ticks,
                    graphics.animated_models.allocation_bytes()
                );
                if self
                    .catalog
                    .snapshot(&self.id)
                    .is_some_and(|asset| asset.value().skinned_lod.is_some())
                {
                    let imported = self
                        .catalog
                        .snapshot(&self.id)
                        .ok_or("missing native LOD source")?;
                    let lod = imported
                        .value()
                        .skinned_lod
                        .as_ref()
                        .ok_or("missing native LOD source")?;
                    let reduced_indices =
                        lod.indices(1).ok_or("missing native reduced level")?.len() as u32;
                    let base_indices =
                        lod.indices(0).ok_or("missing native base level")?.len() as u32;
                    if graphics.animated_models.lod_selection(first, 0)
                        != Some((1, reduced_indices))
                        || graphics.animated_models.lod_selection(second, 0)
                            != Some((1, reduced_indices))
                    {
                        return Err("native skeletal LOD did not select reduced indices".into());
                    }
                    let world = self.scene.world_matrix(first)?;
                    let (min, max) = graphics
                        .animated_models
                        .lod_world_bounds(first, world)?
                        .ok_or("missing native LOD bounds")?;
                    self.camera.perspective = true;
                    self.camera.distance = 0.01;
                    self.camera.target = (min + max) * 0.5;
                    self.camera.target.z =
                        max.z + self.camera.distance * 0.0005 - self.camera.distance;
                    let smoke = self.animation_smoke.as_mut().unwrap();
                    smoke.lod_bytes = graphics.animated_models.allocation_bytes();
                    smoke.base_indices = base_indices;
                    smoke.phase = 3;
                    smoke.since = self.frames;
                    println!(
                        "ANIMATION NATIVE LOD FAR PASS owners=2 level=1 indices={reduced_indices}"
                    );
                    return Ok(false);
                }
                self.toggle_play()?;
                let smoke = self.animation_smoke.as_mut().unwrap();
                smoke.phase = 2;
                smoke.since = self.frames;
            }
            3 => {
                let graphics = self
                    .graphics
                    .as_ref()
                    .ok_or("missing native LOD graphics")?;
                let first = self.instances[0];
                let base_indices = self.animation_smoke.as_ref().unwrap().base_indices;
                if graphics.animated_models.lod_selection(first, 0) != Some((0, base_indices)) {
                    return Err("native near-plane camera did not restore base indices".into());
                }
                let before = self.animation_smoke.as_ref().unwrap().lod_bytes;
                let after = graphics.animated_models.allocation_bytes();
                if after >= before {
                    return Err("native near-plane camera did not evict unused LOD indices".into());
                }
                println!(
                    "ANIMATION NATIVE LOD NEAR PASS level=0 indices={base_indices} before={before} after={after}"
                );
                self.toggle_play()?;
                let smoke = self.animation_smoke.as_mut().unwrap();
                smoke.phase = 2;
                smoke.since = self.frames;
            }
            2 => {
                let graphics = self
                    .graphics
                    .as_ref()
                    .ok_or("missing native graphics after Stop")?;
                if graphics.animated_models.allocation_bytes() != 0 {
                    return Err("Stop retained animated GPU allocations".into());
                }
                if self.authoring_document()?
                    != *self
                        .animation_smoke
                        .as_ref()
                        .unwrap()
                        .authoring
                        .as_ref()
                        .unwrap()
                {
                    return Err("Stop changed authored animation settings or transforms".into());
                }
                println!(
                    "ANIMATION NATIVE STOP PASS frames={} animated_bytes=0 authoring_restored=true",
                    self.frames
                );
                return Ok(true);
            }
            _ => unreachable!(),
        }
        Ok(false)
    }
}
