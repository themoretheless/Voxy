//! Viewport drawing, view splitting and pointer-to-view mapping.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub(crate) fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let profile_start = self
            .animation_smoke
            .as_ref()
            .is_some_and(|s| s.profile)
            .then(Instant::now);
        self.tick()?;
        let elapsed = self.play.simulation_time.elapsed().as_secs_f64();
        self.play.simulation_time = Instant::now();
        self.advance_game(elapsed)?;
        // Interpolated ancestor matrices are shared across the instances of one frame.
        let render_world_cache = std::cell::RefCell::new(voxy_scene::RenderWorldCache::default());
        voxy_scene::extraction_schedule()?.run_scene(&mut self.scene, |_, access| {
            let render_world = |scene: &voxy_scene::SceneGraph, owner| {
                if let Some(simulation) = &self.play.simulation {
                    simulation
                        .render_world_cached(scene, &mut render_world_cache.borrow_mut(), owner)
                        .map_err(|error| match error {
                            voxy_scene::SimulationError::Scene(error) => error,
                            _ => voxy_scene::SceneGraphError::InvalidNode,
                        })
                } else {
                    scene.world_matrix(owner)
                }
            };
            let styles = frame_styles::FrameStyles::prepare_with_world(
                &access,
                &self.camera,
                scene_limits::OBJECTS,
                render_world,
            )
            .map_err(|e| e.to_string())?;
            self.extraction
                .refresh_scoped_with(access, render_world)
                .map_err(|e| e.to_string())?;
            self.frame_styles = Some(styles);
            Ok::<(), EditorError>(())
        })?;
        if let Some(window) = &self.window
            && self.panels.is_some()
        {
            let size = window.inner_size().to_logical::<f32>(window.scale_factor());
            if size.width > 0.0 && size.height > 0.0 {
                let document = self.panel_document()?;
                self.reconcile_component_edit(&document)?;
                self.panels.as_mut().unwrap().preview_phase = self
                    .authoring
                    .animation_preview
                    .as_ref()
                    .filter(|preview| self.instances.get(self.selected) == Some(&preview.owner))
                    .map(|preview| preview.phase);
                self.panels.as_mut().unwrap().retarget_draft = self.retarget_draft.is_some();
                self.panels.as_mut().unwrap().bone_picker = self
                    .retarget_picker
                    .as_ref()
                    .map(|p| (p.choices.clone(), p.page));
                let input = PanelInput {
                    prefab_metadata: self
                        .authoring
                        .history
                        .as_ref()
                        .map_or(serde_json::Value::Null, |history| {
                            history.metadata().clone()
                        }),
                    focused: self
                        .panels
                        .as_ref()
                        .and_then(|panels| panels.focus.focused()),
                    document,
                    selected: self.selected,
                    size: [size.width, size.height],
                    playing: self.play.playing.is_some(),
                    inspector: self.inspector,
                    texture_label: self
                        .instances
                        .get(self.selected)
                        .and_then(|node| {
                            let model = self
                                .scene
                                .component::<ModelInstance>(*node)
                                .ok()
                                .flatten()?;
                            let asset = self.catalog.snapshot(&model.asset)?;
                            let part = self.scene.component::<ModelPart>(*node).ok().flatten()?;
                            asset.value().nodes.get(part.node as usize)?.image
                        })
                        .map_or_else(|| "Texture: none".into(), |_| "Texture: PNG/JPEG".into()),
                    prefab_label: self
                        .authoring
                        .prefab_assets
                        .get(self.authoring.prefab_choice)
                        .map_or_else(|| "Prefab: none".into(), |id| format!("Prefab: {}", id.0)),
                    import_config: self.settings_config(),
                    field: self.field.clone(),
                    scroll: self.tree_scroll,
                    parenting: self.parenting.is_some(),
                };
                if self.panel_cache.as_ref() != Some(&input) {
                    let overrides = self.prefab_overridden_fields(&input.document)?;
                    let collection_resets = self.prefab_collection_resets(&input.document)?;
                    let order_resets = self.prefab_collection_order_resets(&input.document)?;
                    let deleted_items = self.prefab_collection_deleted_items(&input.document)?;
                    let deleted_resets = deleted_items.keys().copied().collect();
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_deleted_resets = deleted_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_deleted_items = deleted_items;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_order_resets = order_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_resets = collection_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .overridden_fields = overrides;
                    let mesh = self.panels.as_mut().ok_or("missing panels")?.build(
                        &input.document,
                        input.selected,
                        Vec2::from_array(input.size),
                        input.playing,
                        input.field.as_ref().map(|(id, text)| (*id, text.as_str())),
                        input.scroll,
                        input.parenting,
                        input.inspector,
                        &input.texture_label,
                        &input.prefab_label,
                        input.import_config,
                    )?;
                    if let Some(graphics) = &mut self.graphics {
                        graphics.panel_geometry = Some(
                            graphics
                                .renderer
                                .upload_mesh(graphics.host.device(), &mesh)?,
                        );
                    }
                    self.panel_cache = Some(input);
                }
            }
        }
        let viewport = self.window.as_ref().map_or([1, 1], |window| {
            let size = window.inner_size();
            [size.width.max(1), size.height.max(1)]
        });
        if let Some(graphics) = &mut self.graphics {
            if self.play.playing.is_some() || self.standalone {
                if let Err(error) = self.ui_live.poll(
                    &self.scene,
                    &self.authoring.authoring_project,
                    [viewport[0] as f32, viewport[1] as f32],
                    graphics,
                ) {
                    eprintln!("GAME UI publication retained previous resources: {error}");
                }
            } else {
                self.retired_ui_workers.extend(self.ui_live.close());
                graphics.ui_draws.clear();
                graphics.ui_presented = false;
            }
        }
        let ring = self
            .ui_live
            .focus_snapshot()
            .and_then(|snapshot| self.game_ui_input.focus_ring(snapshot));
        if let Some(graphics) = &mut self.graphics {
            graphics.residency_cache.geometry_live = graphics.geometry_bytes();
            let deferred = ring.as_ref().is_some_and(|ring| {
                graphics
                    .ui_focus_deferred
                    .as_ref()
                    .is_some_and(|(prior, state)| {
                        prior == ring && *state == graphics.residency_cache.state()
                    })
            });
            if graphics.ui_focus_key != ring && !deferred {
                if let Some(ring) = &ring {
                    let mesh = ring.mesh()?;
                    graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                    match ui_live::upload_on(
                        &graphics.renderer,
                        graphics.host.device(),
                        graphics.host.queue(),
                        &mut graphics.residency_cache,
                        &[mesh],
                    ) {
                        Ok(draws) => {
                            graphics.ui_focus = draws;
                            graphics.ui_focus_key = Some(ring.clone());
                            graphics.ui_focus_deferred = None;
                        }
                        Err(error) => {
                            graphics.ui_focus.clear();
                            graphics.ui_focus_key = None;
                            graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                            graphics.ui_focus_deferred =
                                Some((ring.clone(), graphics.residency_cache.state()));
                            eprintln!("GAME UI FOCUS DEFERRED: {error}");
                        }
                    }
                } else {
                    graphics.ui_focus.clear();
                    graphics.ui_focus_key = None;
                    graphics.ui_focus_deferred = None;
                }
            }
        }
        let styles = self
            .frame_styles
            .as_ref()
            .ok_or("missing presentation styles")?;
        let split = self.split_views() && viewport[0] >= 2;
        let mut views = Vec::new();
        for (index, region) in view_layout::regions(viewport, split)
            .into_iter()
            .enumerate()
        {
            let id = index as u8;
            let camera = if !split || id == self.active_view {
                &styles.camera
            } else {
                self.secondary_camera
                    .as_ref()
                    .ok_or("missing second camera")?
            };
            let size = Vec2::new(region[2] as f32, region[3] as f32);
            views.push((id, region, camera.matrix(size)?, camera.scene_camera(size)?));
        }
        if views.is_empty() {
            return Ok(());
        }
        let animation_requests = if self.play.playing.is_some()
            || self.standalone
            || self.authoring.animation_preview.is_some()
        {
            self.extraction
                .instances()
                .iter()
                .filter_map(|instance| {
                    if styles.parts.contains_key(&instance.owner) {
                        return None;
                    }
                    let published = self
                        .graphics
                        .as_ref()?
                        .models
                        .get(&instance.component.asset)?;
                    let model = published.animated_model.as_ref()?.clone();
                    let settings = self
                        .scene
                        .component::<ModelAnimation>(instance.owner)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(ModelAnimation {
                            clip: (!model.animations.is_empty()).then_some(0),
                            speed: 1.0,
                            ..ModelAnimation::default()
                        });
                    let frame = if self.play.playing.is_some() || self.standalone {
                        self.play.animations.frame(instance.owner, &model)
                    } else {
                        Some(self.preview_animation_frame(instance.owner, &model, &settings)?)
                    };
                    Some(animated_models::Request {
                        owner: instance.owner,
                        frame,
                        model,
                        settings,
                        lod: published.animated_lod.clone(),
                        textures: published.animated_textures.clone(),
                        texture_storage: published._images.clone(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let optical_particles = if !self.msaa4 && views.iter().any(|v| v.3.is_some()) {
            self.play
                .liquid
                .as_ref()
                .map(|runtime| {
                    liquid_draw::optical_particles(runtime.liquid(), &self.play.liquid_optics)
                })
                .transpose()?
                .flatten()
        } else {
            None
        };
        if let Some(graphics) = &mut self.graphics {
            let liquid_revision = self
                .play
                .liquid
                .as_ref()
                .map(|_| (self.scene.identity(), self.play.simulation_ticks));
            if graphics.liquid_revision != liquid_revision {
                let mesh = self
                    .play
                    .liquid
                    .as_ref()
                    .map(|r| liquid_draw::mesh(r.liquid()))
                    .transpose()?
                    .flatten();
                if let Some(mesh) = &mesh {
                    let bytes =
                        (std::mem::size_of_val(mesh.vertices()) + mesh.indices().len() * 4) as u64;
                    if graphics.geometry_bytes().saturating_add(bytes)
                        > graphics.residency_cache.geometry_budget
                    {
                        return Err("liquid GPU geometry budget exceeded".into());
                    }
                }
                graphics.liquid_geometry = mesh
                    .as_ref()
                    .map(|mesh| graphics.renderer.upload_mesh(graphics.host.device(), mesh))
                    .transpose()?;
                graphics.liquid_revision = liquid_revision;
            }
            if let Some(particles) = &optical_particles {
                graphics
                    .optical_liquids
                    .retain(|id, _| views.iter().any(|v| v.0 == *id && v.3.is_some()));
                for &(id, region, _, camera) in &views {
                    let Some(camera) = camera else {
                        continue;
                    };
                    if graphics
                        .optical_liquids
                        .get(&id)
                        .is_none_or(|r| r.size() != region[2..])
                    {
                        let candidate =
                            voxy_render::ScreenSpaceFluidRenderer::new_with_adapter_budget(
                                graphics.host.device(),
                                graphics.host.adapter(),
                                graphics.host.color_format(),
                                region[2],
                                region[3],
                                16384,
                                graphics.geometry_bytes(),
                                graphics.residency_cache.geometry_budget,
                            )?;
                        graphics.optical_liquids.insert(id, candidate);
                    }
                    graphics.optical_liquids.get_mut(&id).unwrap().update(
                        graphics.host.queue(),
                        camera,
                        particles,
                        1.,
                        voxy_render::FluidDepthFilter::default(),
                    )?;
                }
            } else {
                graphics.optical_liquids.clear();
            }
            let animation_start = profile_start.map(|_| Instant::now());
            let other_live = graphics
                .geometry_bytes()
                .saturating_sub(graphics.animated_models.allocation_bytes());
            if self.play.playing.is_none()
                && !self.standalone
                && self.authoring.animation_preview.is_none()
            {
                graphics.animated_models.clear();
            } else {
                for (owner, error) in graphics.animated_models.synchronize(
                    &graphics.renderer,
                    graphics.host.device(),
                    graphics.host.queue(),
                    animation_requests,
                    self.play.animations.serial(),
                    other_live,
                    graphics.residency_cache.geometry_budget,
                ) {
                    eprintln!("MODEL ANIMATION retained previous frame for {owner:?}: {error}");
                }
            }
            let animation_us =
                animation_start.map_or(0., |start| start.elapsed().as_secs_f64() * 1e6);
            graphics.transforms.retain(|(view, owner), _| {
                views.iter().any(|v| v.0 == *view) && styles.materials.contains_key(owner)
            });
            graphics.lod_history.retain(|(view, owner), _| {
                views.iter().any(|v| v.0 == *view) && styles.materials.contains_key(owner)
            });
            graphics
                .gizmo_transforms
                .retain(|view, _| views.iter().any(|v| v.0 == *view));
            let selected = self.instances.get(self.selected).and_then(|node| {
                self.extraction
                    .instances()
                    .iter()
                    .find(|instance| instance.owner == *node)
            });
            graphics.animated_models.retain_lod_views(
                &views
                    .iter()
                    .filter(|view| view.3.is_some())
                    .map(|view| view.0)
                    .collect::<Vec<_>>(),
            );
            let animation_other_live = graphics
                .geometry_bytes()
                .saturating_sub(graphics.animated_models.allocation_bytes());
            let lod_start = profile_start.map(|_| Instant::now());
            for &(view, region, _, camera) in &views {
                for instance in self.extraction.instances() {
                    if let Err(error) = graphics.animated_models.select_lod(
                        &graphics.renderer,
                        graphics.host.device(),
                        instance.owner,
                        view,
                        camera,
                        instance.world,
                        [region[2], region[3]],
                        animation_other_live,
                        graphics.residency_cache.geometry_budget,
                    ) {
                        eprintln!("ANIMATED LOD retained previous selection: {error}");
                    }
                }
            }
            graphics.animated_models.evict_unused_lod();
            let lod_us = lod_start.map_or(0., |start| start.elapsed().as_secs_f64() * 1e6);
            let mut requirements = Vec::new();
            for &(view, region, view_projection, lod_camera) in &views {
                if graphics.liquid_geometry.is_some() {
                    let transform = match graphics.liquid_transforms.entry(view) {
                        std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                        std::collections::btree_map::Entry::Vacant(e) => e.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), Mat4::IDENTITY)?,
                        ),
                    };
                    transform.update(graphics.host.queue(), view_projection)?;
                }
                for instance in self.extraction.instances() {
                    let key = (view, instance.owner);
                    let transform = match graphics.transforms.entry(key) {
                        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                        std::collections::hash_map::Entry::Vacant(entry) => entry.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), instance.world)?,
                        ),
                    };
                    transform.update(graphics.host.queue(), view_projection * instance.world)?;
                    let material = styles
                        .materials
                        .get(&instance.owner)
                        .copied()
                        .unwrap_or_default();
                    let light =
                        styles
                            .light
                            .filter(|_| material.lit)
                            .map_or([0., 0., 1., 0.], |light| {
                                [
                                    light.direction[0],
                                    light.direction[1],
                                    light.direction[2],
                                    light.intensity,
                                ]
                            });
                    transform.update_scene_material(
                        graphics.host.queue(),
                        instance.world,
                        material.tint,
                        light,
                    )?;
                    if !styles.parts.contains_key(&instance.owner) {
                        requirements.push(gpu_model::LodViewRequest {
                            asset: &instance.component.asset,
                            camera: lod_camera,
                            world: instance.world,
                            viewport: [region[2], region[3]],
                            previous: graphics
                                .lod_history
                                .get(&key)
                                .and_then(voxy_render::SceneLodHistory::previous),
                        });
                    }
                }
                if let Some(instance) = selected {
                    let transform = match graphics.gizmo_transforms.entry(view) {
                        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                        std::collections::btree_map::Entry::Vacant(entry) => entry.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), Mat4::IDENTITY)?,
                        ),
                    };
                    transform.update(
                        graphics.host.queue(),
                        view_projection * Mat4::from_translation(instance.world.w_axis.truncate()),
                    )?;
                }
            }
            let requested = gpu_model::collect_lod_requests(&graphics.models, requirements);
            let other_bytes = graphics.geometry_bytes()
                - graphics
                    .models
                    .values()
                    .map(ModelGraphics::geometry_bytes)
                    .sum::<u64>();
            let (live, pending) = gpu_model::reconcile_lod_residency(
                &graphics.renderer,
                graphics.host.device(),
                &mut graphics.models,
                &requested,
                other_bytes,
                graphics.residency_cache.geometry_budget,
            );
            if pending != graphics.lod_pending && std::env::var_os("VOXY_LOD_TRACE").is_some() {
                eprintln!(
                    "LOD RESIDENCY live={live} budget={} pending={pending:?}",
                    graphics.residency_cache.geometry_budget
                );
            }
            graphics.lod_pending = pending;
            graphics.residency_cache.geometry_live = live;
            let mut view_draws = Vec::new();
            let mut view_casters = Vec::new();
            let mut gpu_caster_draws = 0;
            let mut gpu_caster_owners = Vec::new();
            for &(view, region, _, lod_camera) in &views {
                let mut draws = Vec::with_capacity(self.extraction.instances().len() + 2);
                let mut casters = Vec::new();
                for instance in self.extraction.instances() {
                    if let Some(geometries) = graphics
                        .animated_models
                        .geometry_inputs_for_view(instance.owner, view)
                    {
                        for (primitive, posed) in geometries.enumerate() {
                            let geometry = posed.geometry;
                            let texture = graphics
                                .animated_models
                                .texture(instance.owner, primitive)
                                .unwrap_or(&graphics.texture);
                            if styles.casters.contains(&instance.owner) {
                                gpu_caster_draws += usize::from(posed.gpu_deformed);
                                if self.trace.enabled
                                    && posed.gpu_deformed
                                    && !gpu_caster_owners.contains(&instance.owner)
                                {
                                    gpu_caster_owners.push(instance.owner);
                                }
                                casters.push(fog_draw::OpaqueCaster {
                                    geometry,
                                    world: instance.world,
                                });
                            }
                            draws.push(SceneDraw {
                                geometry,
                                texture,
                                transform: &graphics.transforms[&(view, instance.owner)],
                                overlay: false,
                            });
                        }
                        continue;
                    }
                    if let Some(model) = graphics.models.get(&instance.component.asset)
                        && !styles.parts.contains_key(&instance.owner)
                        && !model.animated_preview.is_empty()
                    {
                        for (primitive, geometry) in model.animated_preview.iter().enumerate() {
                            if styles.casters.contains(&instance.owner) {
                                casters.push(fog_draw::OpaqueCaster {
                                    geometry,
                                    world: instance.world,
                                });
                            }
                            draws.push(SceneDraw {
                                geometry,
                                texture: model.animated_textures[primitive]
                                    .as_deref()
                                    .unwrap_or(&graphics.texture),
                                transform: &graphics.transforms[&(view, instance.owner)],
                                overlay: false,
                            });
                        }
                        continue;
                    }
                    if let Some(model) = graphics.models.get(&instance.component.asset)
                        && let Some((geometry, _, texture)) =
                            model.part(styles.parts.get(&instance.owner).copied())
                    {
                        let geometry = if styles.parts.contains_key(&instance.owner) {
                            geometry
                        } else {
                            let history = graphics
                                .lod_history
                                .entry((view, instance.owner))
                                .or_default();
                            let previous = history.previous();
                            let geometry = model.geometry.for_view(
                                lod_camera,
                                instance.world,
                                [region[2], region[3]],
                                history,
                            );
                            if previous != history.previous()
                                && std::env::var_os("VOXY_LOD_TRACE").is_some()
                            {
                                eprintln!(
                                    "LOD DRAW view={view} asset={} level={:?} triangles={} viewport={:?}",
                                    instance.component.asset.0,
                                    history.previous(),
                                    geometry.index_count() / 3,
                                    [region[2], region[3]]
                                );
                            }
                            geometry
                        };
                        if styles.casters.contains(&instance.owner) {
                            casters.push(fog_draw::OpaqueCaster {
                                geometry,
                                world: instance.world,
                            });
                        }
                        draws.push(SceneDraw {
                            geometry,
                            texture: texture.unwrap_or(&graphics.texture),
                            transform: &graphics.transforms[&(view, instance.owner)],
                            overlay: false,
                        });
                    }
                }
                if !self.standalone
                    && let Some(instance) = selected
                    && graphics
                        .animated_models
                        .geometries(instance.owner)
                        .is_none()
                    && let Some(model) = graphics.models.get(&instance.component.asset)
                    && let Some((_, Some(outline), _)) =
                        model.part(styles.parts.get(&instance.owner).copied())
                    && let Some(transform) = graphics.transforms.get(&(view, instance.owner))
                {
                    draws.push(SceneDraw {
                        geometry: outline,
                        texture: &graphics.texture,
                        transform,
                        overlay: true,
                    });
                }
                if self.play.playing.is_none()
                    && let Some(instance) = selected
                    && graphics.models.contains_key(&instance.component.asset)
                    && let Some(transform) = graphics.gizmo_transforms.get(&view)
                {
                    draws.push(SceneDraw {
                        geometry: &graphics.gizmo_geometry,
                        texture: &graphics.texture,
                        transform,
                        overlay: true,
                    });
                }
                if !graphics.optical_liquids.contains_key(&view)
                    && let Some(geometry) = &graphics.liquid_geometry
                {
                    draws.push(SceneDraw {
                        geometry,
                        texture: &graphics.texture,
                        transform: &graphics.liquid_transforms[&view],
                        overlay: false,
                    });
                }
                view_draws.push(draws);
                view_casters.push(casters);
            }
            let mut overlays = Vec::new();
            for ui in graphics.ui_draws.iter().chain(graphics.ui_focus.iter()) {
                if styles.ui_owners.contains(&ui.owner) {
                    overlays.push(SceneDraw {
                        geometry: &ui.geometry,
                        texture: ui.texture.as_deref().unwrap_or(&graphics.texture),
                        transform: &graphics.panel_transform,
                        overlay: true,
                    });
                }
            }
            if let (Some(geometry), Some(texture)) =
                (&graphics.panel_geometry, &graphics.panel_texture)
            {
                overlays.push(SceneDraw {
                    geometry,
                    texture,
                    transform: &graphics.panel_transform,
                    overlay: true,
                });
            }
            let draw_count = view_draws.iter().map(Vec::len).sum::<usize>() + overlays.len();
            let present_start = profile_start.map(|_| Instant::now());
            if styles.fog.is_empty()
                && let Some(fog) = &mut graphics.fog
                && fog.deactivate(graphics.host.device())?
            {
                graphics.fog = None;
            }
            if std::env::var_os("VOXY_WINDOW_PIXEL_PROBE").is_some() {
                if let Some((frame, pixel)) = graphics.host.take_pixel_probe() {
                    println!("WINDOW PIXEL frame={frame} rgba_storage={pixel:?}");
                }
                if ((self.trace.pending && self.trace.last_outcome.is_none())
                    || (self.frames > 0 && self.frames.is_multiple_of(60)))
                    && !graphics.host.pixel_probe_pending()
                {
                    let size = self.window.as_ref().ok_or("missing window")?.inner_size();
                    // Stable floor point in the standalone gameplay diagnostic fixture.
                    let point = [size.width / 2, (f64::from(size.height) * 0.775) as u32];
                    if let Err(error) = graphics.host.request_pixel_probe(point) {
                        eprintln!("WINDOW PIXEL unavailable: {error}");
                    }
                }
            }
            let outcome = if let Some((_, volume, bounds)) = styles.fog.first() {
                if styles.fog.len() != 1 {
                    return Err("multiple overlapping fog volumes are not yet composed".into());
                }
                if self.msaa4 {
                    return Err("editor fog requires single-sample scene inputs".into());
                }
                if !graphics.optical_liquids.is_empty() {
                    return Err("fog/liquid merged optical depth is not yet available".into());
                }
                if graphics.fog.is_none() {
                    graphics.fog = Some(pollster::block_on(fog_draw::FogDraw::new(
                        graphics.host.device(),
                        graphics.host.color_format(),
                    ))?);
                }
                let window = self
                    .window
                    .as_ref()
                    .ok_or("missing fog window")?
                    .inner_size();
                let device = graphics.host.device().clone();
                let other = graphics
                    .geometry_bytes()
                    .saturating_sub(graphics.fog.as_ref().unwrap().allocation_bytes());
                let budget = graphics
                    .residency_cache
                    .geometry_budget
                    .checked_sub(other)
                    .ok_or("fog scene memory budget exceeded")?;
                let fog = graphics.fog.as_mut().unwrap();
                fog.prepare_targets_with_shadow(
                    &device,
                    [window.width, window.height],
                    &views,
                    styles.shadow,
                    view_casters.iter().map(Vec::len).sum(),
                    budget,
                )?;
                let result = graphics.host.render_custom(|encoder, output| {
                    fog.encode(
                        &device,
                        encoder,
                        output,
                        &graphics.renderer,
                        &views,
                        &view_draws,
                        &overlays,
                        *volume,
                        *bounds,
                        styles.light,
                        &view_casters,
                        styles.shadow.map(|(_, settings)| settings),
                    )
                });
                if matches!(&result, Ok(voxy_render::RenderOutcome::Presented)) {
                    fog.submitted(graphics.host.queue());
                } else {
                    // render_custom has already discarded any failed encoder.
                    fog.discard_unsubmitted();
                }
                result?
            } else if !split || views.len() == 1 {
                let mut draws = view_draws.pop().ok_or("missing view draws")?;
                draws.extend(overlays);
                if let Some(fluid) = graphics.optical_liquids.get(&views[0].0) {
                    graphics.host.render_custom(|encoder, output| {
                        fluid.encode(
                            &graphics.renderer,
                            encoder,
                            output,
                            wgpu::Color::BLACK,
                            &draws,
                        );
                        Ok::<_, voxy_render::RendererError>(())
                    })?
                } else {
                    graphics.host.render_scene(&graphics.renderer, &draws)?
                }
            } else {
                let scene_views: Vec<_> = views
                    .iter()
                    .zip(&view_draws)
                    .map(|(view, draws)| voxy_render::SceneView {
                        viewport: view.1,
                        draws,
                    })
                    .collect();
                if graphics.optical_liquids.is_empty() {
                    graphics
                        .host
                        .render_scene_views(&graphics.renderer, &scene_views, &overlays)?
                } else {
                    let fluids: Vec<_> = views
                        .iter()
                        .enumerate()
                        .filter_map(|(index, view)| {
                            graphics.optical_liquids.get(&view.0).map(|f| (index, f))
                        })
                        .collect();
                    graphics.host.render_scene_views_with_fluids(
                        &graphics.renderer,
                        &scene_views,
                        &fluids,
                        &overlays,
                    )?
                }
            };
            if let Some(start) = profile_start {
                println!(
                    "ANIMATION FRAME PROFILE frame={} playing={} views={} draws={} animation_us={animation_us:.3} lod_us={lod_us:.3} submit_present_us={:.3} total_cpu_us={:.3} outcome={outcome:?}",
                    self.frames,
                    self.play.playing.is_some(),
                    views.len(),
                    draw_count,
                    present_start.unwrap().elapsed().as_secs_f64() * 1e6,
                    start.elapsed().as_secs_f64() * 1e6
                );
            }
            if self.trace.enabled && self.trace.pending && self.trace.last_outcome != Some(outcome)
            {
                println!(
                    "EDITOR PRESENT frames={} draws={} outcome={outcome:?} positions={:?}",
                    self.frames,
                    draw_count,
                    self.extraction
                        .instances()
                        .iter()
                        .map(|instance| (instance.owner, instance.world.w_axis))
                        .collect::<Vec<_>>()
                );
            }
            self.trace.last_outcome = Some(outcome);
            if let Some(panels) = &mut self.panels {
                panels.frame_outcome(outcome);
            }
            if outcome == RenderOutcome::Presented {
                if !styles.fog.is_empty() && self.trace.enabled {
                    println!(
                        "EDITOR FOG PRESENT frame={} views={} owner={:?} shadow={} casters={} gpu_casters={}",
                        self.frames + 1,
                        views.len(),
                        styles.fog[0].0,
                        styles.shadow.is_some(),
                        view_casters.iter().map(Vec::len).sum::<usize>(),
                        gpu_caster_draws
                    );
                    for owner in &gpu_caster_owners {
                        if let Some((ticks, palette)) =
                            graphics.animated_models.pose_evidence(*owner)
                        {
                            println!(
                                "EDITOR FOG GPU POSE frame={} owner={owner:?} ticks={ticks} palette={palette}",
                                self.frames + 1
                            );
                        }
                    }
                }
                self.ui_live.frame_presented();
                graphics.ui_presented |= self.ui_live.ready();
                self.trace.pending = false;
                self.frames += 1;
            }
        }
        Ok(())
    }

    pub(crate) fn split_views(&self) -> bool {
        self.secondary_camera.is_some() && self.play.playing.is_none() && !self.standalone
    }

    pub(crate) fn activate_view(&mut self, view: u8) -> Result<(), Box<dyn std::error::Error>> {
        if view != self.active_view && self.split_views() {
            self.finish_drag(false)?;
            self.camera_drag = None;
            if let Some(other) = &mut self.secondary_camera {
                std::mem::swap(&mut self.camera, other);
            }
            self.active_view = view;
            self.update_edit_title();
        }
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn view_pointer(
        &mut self,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<Option<(Vec2, Vec2, Vec2)>, Box<dyn std::error::Error>> {
        if !self.split_views() || size.x < 2. {
            return Ok(Some((cursor, size, Vec2::ZERO)));
        }
        let regions = view_layout::regions([size.x as u32, size.y as u32], true);
        let Some((view, local, view_size)) = view_layout::hit(&regions, cursor) else {
            return Ok(None);
        };
        self.activate_view(view)?;
        Ok(Some((local, view_size, cursor - local)))
    }

    pub(crate) fn active_camera_size(&self, size: Vec2) -> Vec2 {
        if self.split_views() && size.x >= 2. {
            let left = (size.x * 0.5).floor();
            Vec2::new(
                if self.active_view == 0 {
                    left
                } else {
                    size.x - left
                },
                size.y,
            )
        } else {
            size
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub(crate) fn cursor_in_viewport(&self, cursor: Vec2) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        let scale = window.scale_factor() as f32;
        let size = window.inner_size();
        let size = Vec2::new(size.width as f32, size.height as f32) / scale;
        let cursor = cursor / scale;
        cursor.x >= 150_f32.min(size.x * 0.25)
            && cursor.x < size.x - 180_f32.min(size.x * 0.3)
            && cursor.y >= 0.
            && cursor.y < size.y
    }
}
