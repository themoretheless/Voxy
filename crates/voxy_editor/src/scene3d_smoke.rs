//! Native acceptance shares real editor commands, camera rays and GPU publication.
use crate::{App, ElementState, KeyCode, ModelInstance, SceneDocument, Vec2};
#[derive(Debug, Default)]
pub(super) struct Smoke {
    phase: u8,
    frame: u64,
    expected: Option<SceneDocument>,
    start_x: f32,
    ticks: u64,
}
impl App {
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    pub(super) fn scene3d_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(mut smoke) = self.scene3d_smoke.take() else {
            return Ok(false);
        };
        let result = (|| {
            if self.frames < smoke.frame + 3 {
                return Ok(false);
            }
            match smoke.phase {
                0 => {
                    if self.instances.iter().any(|node| {
                        self.scene
                            .component::<ModelInstance>(*node)
                            .ok()
                            .flatten()
                            .is_some_and(|model| {
                                !self
                                    .graphics
                                    .as_ref()
                                    .is_some_and(|g| g.models.contains_key(&model.asset))
                            })
                    }) {
                        return Ok(false);
                    }
                    let textured = self.graphics.as_ref().is_some_and(|g| {
                        g.models
                            .values()
                            .any(|model| model.parts.values().any(|part| part.texture.is_some()))
                    });
                    if !textured {
                        return Err("3D acceptance needs a published textured glTF".into());
                    }
                    let asset = self
                        .catalog
                        .snapshot(&crate::AssetId("assembly".into()))
                        .ok_or("missing assembly")?;
                    let model = self
                        .graphics
                        .as_ref()
                        .and_then(|g| g.models.get(&crate::AssetId("assembly".into())))
                        .ok_or("missing assembly GPU")?;
                    for (index, node) in asset.value().nodes.iter().enumerate() {
                        if node.mesh.is_none() {
                            continue;
                        }
                        let gpu = &model.parts[&u32::try_from(index)?];
                        if node.use_mips
                            && let Some(texture) = &gpu.texture
                            && texture.texture().mip_level_count() <= 1
                        {
                            return Err("missing native mip chain".into());
                        }
                        for (previous_index, previous) in
                            asset.value().nodes[..index].iter().enumerate()
                        {
                            if previous.mesh.is_none() {
                                continue;
                            }
                            let cached = &model.parts[&u32::try_from(previous_index)?];
                            if node.geometry_key == previous.geometry_key
                                && !std::sync::Arc::ptr_eq(&gpu.geometry, &cached.geometry)
                            {
                                return Err("mesh instances duplicated GPU geometry".into());
                            }
                            if node.image.is_some()
                                && node.image == previous.image
                                && gpu
                                    .texture
                                    .as_ref()
                                    .zip(cached.texture.as_ref())
                                    .is_none_or(|(a, b)| a.texture() != b.texture())
                            {
                                return Err("sampler variants duplicated GPU image storage".into());
                            }
                            if node.image.is_some()
                                && node.image == previous.image
                                && node.sampling == previous.sampling
                                && node.use_mips == previous.use_mips
                                && !gpu
                                    .texture
                                    .as_ref()
                                    .zip(cached.texture.as_ref())
                                    .is_some_and(|(a, b)| std::sync::Arc::ptr_eq(a, b))
                            {
                                return Err("mesh instances duplicated GPU textures".into());
                            }
                        }
                    }
                    if let Some(copy) = self
                        .graphics
                        .as_ref()
                        .and_then(|g| g.models.get(&crate::AssetId("assembly-copy".into())))
                    {
                        let shared = model
                            .parts
                            .values()
                            .filter_map(|part| part.texture.as_ref())
                            .any(|texture| {
                                copy.parts
                                    .values()
                                    .filter_map(|part| part.texture.as_ref())
                                    .any(|other| texture.texture() == other.texture())
                            });
                        if !shared {
                            return Err(
                                "separate resources duplicated identical image storage".into()
                            );
                        }
                        println!("SCENE3D CROSS RESOURCE IMAGE PASS");
                    }
                    if !model.geometry_residency_valid(asset.value()) {
                        return Err("geometry estimate differs from actual buffers".into());
                    }
                    let naive = model.geometry.allocation_bytes()
                        + model
                            .outline
                            .as_ref()
                            .map_or(0, voxy_render::SceneGeometry::allocation_bytes)
                        + model
                            .parts
                            .values()
                            .map(|part| {
                                part.geometry.allocation_bytes()
                                    + part
                                        .outline
                                        .as_ref()
                                        .map_or(0, |geometry| geometry.allocation_bytes())
                            })
                            .sum::<u64>();
                    if model.geometry_bytes() >= naive {
                        return Err("geometry accounting failed to deduplicate instances".into());
                    }
                    println!(
                        "SCENE3D GPU GEOMETRY BYTES unique={} naive={naive}",
                        model.geometry_bytes()
                    );
                    println!("SCENE3D GPU SHARING MIPS PASS frames={}", self.frames);
                    let index = self
                        .instances
                        .iter()
                        .position(|node| {
                            self.scene.name(*node).is_ok_and(|name| {
                                name == "Textured cube/primitive-0"
                                    || name == "Textured cube"
                                        && self
                                            .scene
                                            .component::<crate::ModelPart>(*node)
                                            .ok()
                                            .flatten()
                                            .is_some_and(|part| {
                                                self.catalog
                                                    .snapshot(&crate::AssetId("assembly".into()))
                                                    .is_some_and(|asset| {
                                                        asset
                                                            .value()
                                                            .mesh_for(Some(part.node))
                                                            .is_some()
                                                    })
                                            })
                            })
                        })
                        .ok_or("missing textured cube")?;
                    let window = self.window.as_ref().ok_or("missing native window")?;
                    let size = window.inner_size();
                    let size = Vec2::new(size.width as f32, size.height as f32);
                    for perspective in [false, true] {
                        self.camera.perspective = perspective;
                        self.camera.legacy = false;
                        self.camera.orbit(Vec2::new(5., -3.));
                        self.camera.zoom(0.1);
                        self.camera.pan(Vec2::new(2., -1.), size)?;
                        let center = self
                            .scene
                            .world_matrix(self.instances[index])?
                            .w_axis
                            .truncate();
                        let ndc = self.camera.matrix(size)?.project_point3(center);
                        let cursor = Vec2::new(ndc.x + 1., 1. - ndc.y) * size * 0.5;
                        if !self.pick_model(cursor, size)? || self.selected != index {
                            return Err("camera ray selected wrong model".into());
                        }
                    }
                    let hidden: Vec<_> = self
                        .scene
                        .components::<ModelInstance>()
                        .filter(|(_, model)| model.asset.0 == "assembly-copy")
                        .map(|(node, _)| (node, self.scene.active_self(node).unwrap_or(false)))
                        .collect();
                    if !hidden.is_empty() {
                        for (node, _) in &hidden {
                            self.scene.set_active(*node, false)?;
                        }
                        self.synchronize_gpu_residency();
                        if self.graphics.as_ref().is_some_and(|g| {
                            g.models
                                .contains_key(&crate::AssetId("assembly-copy".into()))
                        }) {
                            return Err("hidden model retained geometry residency".into());
                        }
                        for (node, active) in hidden {
                            self.scene.set_active(node, active)?;
                        }
                        self.synchronize_gpu_residency();
                        if !self.graphics.as_ref().is_some_and(|g| {
                            g.models
                                .contains_key(&crate::AssetId("assembly-copy".into()))
                        }) {
                            return Err("reactivated model failed to regain residency".into());
                        }
                        println!("SCENE3D GPU ACTIVITY EVICTION PASS");
                    }
                    let removed: Vec<_> = self
                        .scene
                        .components::<ModelInstance>()
                        .filter(|(_, model)| model.asset.0 == "assembly-copy")
                        .map(|(node, model)| (node, model.clone()))
                        .collect();
                    if !removed.is_empty() {
                        for (node, _) in &removed {
                            self.scene.remove_component::<ModelInstance>(*node)?;
                        }
                        self.synchronize_gpu_residency();
                        if self.graphics.as_ref().is_some_and(|g| {
                            g.models
                                .contains_key(&crate::AssetId("assembly-copy".into()))
                        }) {
                            return Err("unreferenced model retained GPU residency".into());
                        }
                        for (node, model) in removed {
                            self.scene.insert_component(node, model)?;
                        }
                        self.synchronize_gpu_residency();
                        if !self.graphics.as_ref().is_some_and(|g| {
                            g.models
                                .contains_key(&crate::AssetId("assembly-copy".into()))
                        }) {
                            return Err(
                                "restored scene reference did not restore GPU residency".into()
                            );
                        }
                        println!("SCENE3D GPU EVICT RESTORE PASS");
                    }
                    let mut candidate = asset.value().clone();
                    candidate.images[0] = candidate.images[0].mip_chain()[1].clone();
                    let graphics = self.graphics.as_mut().ok_or("missing graphics")?;
                    let budget = graphics.residency_cache.budget;
                    graphics.residency_cache.budget = graphics.residency_cache.live_bytes();
                    let rejected = crate::ModelGraphics::upload(
                        &graphics.renderer,
                        &graphics.host,
                        &candidate,
                        &mut graphics.residency_cache,
                    )
                    .err();
                    graphics.residency_cache.budget = budget;
                    if !rejected.is_some_and(|error| {
                        error.to_string().contains("GPU image budget exceeded")
                    }) {
                        return Err(
                            "image pressure did not reject candidate before publication".into()
                        );
                    }
                    println!("SCENE3D GPU IMAGE BUDGET PASS");
                    let geometry_budget = graphics.residency_cache.geometry_budget;
                    graphics.residency_cache.geometry_budget =
                        graphics.residency_cache.geometry_live;
                    let rejected = crate::ModelGraphics::upload(
                        &graphics.renderer,
                        &graphics.host,
                        asset.value(),
                        &mut graphics.residency_cache,
                    )
                    .err();
                    graphics.residency_cache.geometry_budget = geometry_budget;
                    if !rejected.is_some_and(|error| {
                        error.to_string().contains("GPU geometry budget exceeded")
                    }) {
                        return Err("geometry pressure failed to reject before upload".into());
                    }
                    println!("SCENE3D GPU GEOMETRY BUDGET PASS");
                    self.edit_key(KeyCode::KeyG)?;
                    self.save_authoring()?;
                    smoke.expected = Some(self.authoring_document()?);
                    self.load_authoring()?;
                    if self.authoring_document()?
                        != *smoke.expected.as_ref().ok_or("missing expected scene")?
                    {
                        return Err("3D persistence changed scene".into());
                    }
                    println!(
                        "SCENE3D CAMERA PICK TEXTURE PERSIST PASS frames={}",
                        self.frames
                    );
                }
                1 => {
                    smoke.start_x = self.scene.world_matrix(self.instances[0])?.w_axis.x;
                    self.toggle_play()?;
                    self.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
                    smoke.ticks = self.play.simulation_ticks;
                }
                2 => {
                    if self.play.simulation_ticks < smoke.ticks + 25 {
                        return Ok(false);
                    }
                    let position = self
                        .scene
                        .world_matrix(self.instances[0])?
                        .w_axis
                        .truncate();
                    if position.x <= smoke.start_x + 0.04
                        || position.y < -0.445
                        || !position.is_finite()
                    {
                        return Err("3D rotated-collider character did not advance".into());
                    }
                    self.game_key(KeyCode::ArrowRight, ElementState::Released)?;
                    self.toggle_play()?;
                    if self.authoring_document()?
                        != *smoke.expected.as_ref().ok_or("missing expected scene")?
                    {
                        return Err("3D Stop changed authoring".into());
                    }
                    self.load_authoring()?;
                    println!(
                        "SCENE3D PLAY STOP RELOAD PASS ticks={} frames={}",
                        self.play.simulation_ticks, self.frames
                    );
                }
                _ => {
                    println!("SCENE3D NATIVE PASS frames={}", self.frames);
                    return Ok(true);
                }
            }
            smoke.phase += 1;
            smoke.frame = self.frames;
            Ok(false)
        })();
        if !matches!(result, Ok(true)) {
            self.scene3d_smoke = Some(smoke);
        }
        result
    }
}
