//! Model picking and drag interaction in the viewport.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    #[cfg(test)]
    pub(crate) fn pick_bounds(
        &mut self,
        cursor: Vec2,
        size: Vec2,
        bounds: PickBounds,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if size.x <= 0.0 || size.y <= 0.0 {
            return Ok(false);
        }
        let Some(ray) = (PickViewport {
            origin: Vec2::ZERO,
            size,
        })
        .ray(Mat4::IDENTITY, cursor, 1.0)?
        else {
            return Ok(false);
        };
        for node in &self.instances {
            self.scene.insert_component(*node, bounds)?;
        }
        let hit = self.scene.pick(ray, 1)?.hit;
        if let Some(hit) = hit {
            self.selected = self
                .instances
                .iter()
                .position(|node| *node == hit.node)
                .ok_or("picked foreign instance")?;
            if let Some(window) = &self.window {
                window.set_title(&format!(
                    "Voxy — selected instance {}/{}",
                    self.selected + 1,
                    self.instances.len()
                ));
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// CPU reference geometry for the same accepted revision/frame as display.
    /// Used only by explicit authoring interactions, never per render frame.
    pub(crate) fn displayed_model_meshes(
        &self,
        node: NodeId,
    ) -> Result<Vec<voxy_render::SceneMesh>, Box<dyn std::error::Error>> {
        let Some(model) = self.scene.component::<ModelInstance>(node)? else {
            return Ok(Vec::new());
        };
        let Some(asset) = self.catalog.snapshot(&model.asset) else {
            return Ok(Vec::new());
        };
        let part = self
            .scene
            .component::<ModelPart>(node)?
            .map(|part| part.node);
        let meshes = if part.is_none() {
            if let Some((model, frame)) = self
                .graphics
                .as_ref()
                .and_then(|graphics| graphics.animated_models.accepted_frame(node))
            {
                model.scene_meshes_frame(frame)?
            } else if let Some(model) = &asset.value().animated {
                let settings = self
                    .scene
                    .component::<ModelAnimation>(node)?
                    .cloned()
                    .unwrap_or(ModelAnimation {
                        clip: None,
                        ..Default::default()
                    });
                let frame = if self.play.playing.is_some() || self.standalone {
                    self.play.animations.frame(node, model)
                } else {
                    self.preview_animation_frame(node, model, &settings)
                };
                let mut frame =
                    frame.map_or_else(|| model.bind_frame(), |frame| Ok((*frame).clone()))?;
                if self.play.playing.is_none() && !self.standalone {
                    settings.apply_morph_weights(model, &mut frame)?;
                }
                model.scene_meshes_frame(&frame)?
            } else {
                asset.value().mesh_for(None).cloned().into_iter().collect()
            }
        } else {
            asset.value().mesh_for(part).cloned().into_iter().collect()
        };
        Ok(meshes)
    }

    pub(crate) fn focus_displayed_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(node) = self.instances.get(self.selected).copied() else {
            return Ok(());
        };
        let world = self.scene.world_matrix(node)?.as_dmat4();
        let mut min = glam::DVec3::splat(f64::INFINITY);
        let mut max = glam::DVec3::splat(f64::NEG_INFINITY);
        for mesh in self.displayed_model_meshes(node)? {
            for vertex in mesh.vertices() {
                let p = world.transform_point3(Vec3::from_array(vertex.position).as_dvec3());
                if !p.is_finite() {
                    return Err("nonfinite displayed model bounds".into());
                }
                min = min.min(p);
                max = max.max(p);
            }
        }
        let mut camera = self.camera.clone();
        if min.is_finite() {
            let viewport = self.window.as_ref().map_or([800, 600], |window| {
                let size = window.inner_size();
                [size.width, size.height]
            });
            let regions = view_layout::regions(viewport, self.split_views());
            let region = regions
                .get(usize::from(self.active_view))
                .or_else(|| regions.first())
                .ok_or("empty focus viewport")?;
            let size = Vec2::new(region[2] as f32, region[3] as f32);
            if self.panels.is_some() {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1., |window| window.scale_factor() as f32);
                let safe = panels::safe_view_rect(viewport, *region, scale)
                    .ok_or("focus viewport is obscured by panels")?;
                camera.fit_bounds_in_rect(min, max, size, safe)?;
            } else {
                camera.fit_bounds(min, max, size)?;
            }
        } else {
            camera.target = world.w_axis.truncate().as_vec3();
            camera.legacy = false;
            if !camera.valid() {
                return Err("displayed model center exceeds camera limits".into());
            }
        }
        self.camera = camera;
        Ok(())
    }

    pub(crate) fn pick_model(
        &mut self,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut nearest = None;
        for (index, node) in self.instances.iter().enumerate() {
            let meshes = self.displayed_model_meshes(*node)?;
            for mesh in meshes {
                if let Some((_, depth)) = picking::pick_mesh_depth(
                    &self.scene,
                    &[*node],
                    &mesh,
                    cursor,
                    size,
                    self.camera.matrix(size)?,
                )? && nearest.is_none_or(|(_, previous)| depth < previous)
                {
                    nearest = Some((index, depth));
                }
            }
        }
        if let Some((index, _)) = nearest {
            self.selected = index;
            self.update_edit_title();
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn begin_drag(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.finish_drag(false)?;
        if viewport.x <= 0.0 || viewport.y <= 0.0 || !viewport.is_finite() || !cursor.is_finite() {
            return Ok(());
        }
        let Some(node) = self.instances.get(self.selected).copied() else {
            return Ok(());
        };
        self.drag = Some(ModelDrag {
            node,
            origin: cursor,
            viewport,
            viewport_origin: Vec2::ZERO,
            original: self.scene.local(node)?,
            world_origin: self.scene.world_matrix(node)?.w_axis.truncate(),
            axis: DragAxis::Plane,
            edit: self
                .authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .begin_edit(),
        });
        Ok(())
    }

    // Winit coordinates and finite monitor DPI are intentionally represented as f32.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn pointer_press(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.authoring.preview_seek = None;
        if self.play.playing.is_none()
            && self
                .panels
                .as_ref()
                .is_some_and(|panels| !panels.input_ready())
        {
            return Ok(());
        }
        if let Some(window) = &self.window
            && self.panels.is_some()
        {
            let point = cursor / window.scale_factor() as f32;
            let size = viewport / window.scale_factor() as f32;
            if let Some(action) = self.panel_target(point)? {
                if matches!(action, panels::Action::PreviewPhase(_)) {
                    let rect = self
                        .panels
                        .as_ref()
                        .unwrap()
                        .regions
                        .iter()
                        .find(|(_, action)| *action == panels::Action::PreviewSeek)
                        .ok_or("missing seek region")?
                        .0;
                    self.authoring.preview_seek = self
                        .instances
                        .get(self.selected)
                        .copied()
                        .map(|owner| (owner, rect));
                }
                let result = self.panel_action(action);
                if result.is_err() {
                    self.authoring.preview_seek = None;
                }
                return result;
            }
            if point.x < 150.0_f32.min(size.x * 0.25)
                || point.x >= size.x - 180.0_f32.min(size.x * 0.3)
            {
                return Ok(());
            }
        }
        self.field = None;
        self.parenting = None;
        self.follow_pick = None;
        if self.play.playing.is_some() || self.standalone {
            self.game_ui_pointer(cursor, viewport, true)?;
            return Ok(());
        }
        let Some((cursor, viewport, viewport_origin)) = self.view_pointer(cursor, viewport)? else {
            return Ok(());
        };
        let axis = if let Some(node) = self.instances.get(self.selected)
            && self
                .scene
                .component::<ModelInstance>(*node)?
                .is_some_and(|model| self.catalog.snapshot(&model.asset).is_some())
            && self.scene.active_in_hierarchy(*node)?
        {
            gizmo::hit_axis_camera(
                self.camera.matrix(viewport)?,
                self.scene.world_matrix(*node)?.w_axis.truncate(),
                cursor,
                viewport,
            )
        } else {
            None
        };
        if let Some(axis) = axis {
            self.begin_drag(cursor, viewport)?;
            if let Some(drag) = &mut self.drag {
                drag.axis = axis;
            }
        } else if self.pick_model(cursor, viewport)? {
            self.begin_drag(cursor, viewport)?;
        }
        if let Some(drag) = &mut self.drag {
            drag.viewport_origin = viewport_origin;
        }
        Ok(())
    }

    pub(crate) fn preview_drag(&mut self, cursor: Vec2) -> Result<(), Box<dyn std::error::Error>> {
        self.trace.pending |= self.drag.is_some();
        if let Some(drag) = &self.drag {
            let cursor = cursor - drag.viewport_origin;
            let mut local = drag.original;
            let vp = self.camera.matrix(drag.viewport)?;
            let world_origin = drag.world_origin;
            let depth = vp.project_point3(world_origin).z;
            let plane_delta = camera::unproject(vp, cursor, drag.viewport, depth)?
                - camera::unproject(vp, drag.origin, drag.viewport, depth)?;
            let world_delta = match drag.axis {
                DragAxis::Plane => plane_delta,
                DragAxis::X => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::X,
                    cursor - drag.origin,
                    drag.viewport,
                ),
                DragAxis::Y => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::Y,
                    cursor - drag.origin,
                    drag.viewport,
                ),
                DragAxis::Z => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::Z,
                    cursor - drag.origin,
                    drag.viewport,
                ),
            };
            let delta = if let Some(parent) = self.scene.parent(drag.node)? {
                let inverse = self.scene.world_matrix(parent)?.inverse();
                if !inverse.is_finite() {
                    return Ok(());
                }
                inverse.transform_vector3(world_delta)
            } else {
                world_delta
            };
            local.translation += delta;
            self.scene.set_local(drag.node, local)?;
        }
        Ok(())
    }

    pub(crate) fn finish_drag(&mut self, commit: bool) -> Result<(), Box<dyn std::error::Error>> {
        let Some(mut drag) = self.drag.take() else {
            return Ok(());
        };
        if self.trace.enabled {
            println!("EDITOR DRAG FINISH frames={} commit={commit}", self.frames);
        }
        self.trace.pending = true;
        self.trace.last_outcome = None;
        if !commit {
            self.scene.set_local(drag.node, drag.original)?;
            return Ok(());
        }
        *drag.edit.document_mut() = match self.authoring_document() {
            Ok(document) => document,
            Err(error) => {
                self.scene.set_local(drag.node, drag.original)?;
                return Err(error.into());
            }
        };
        if let Err(error) = self
            .authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_edit(drag.edit, &self.authoring.authoring_project.registry)
        {
            self.scene.set_local(drag.node, drag.original)?;
            return Err(error.into());
        }
        Ok(())
    }
}
