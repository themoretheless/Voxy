//! Model picking and drag interaction in the viewport.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

struct DragPreviewParams {
    node: NodeId,
    origin: Vec2,
    viewport: Vec2,
    original: Transform,
    world_origin: Vec3,
    axis: DragAxis,
    cursor: Vec2,
    snap_active: bool,
    companions: Vec<(NodeId, Transform, Vec3)>,
}

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
            let picked = self
                .instances
                .iter()
                .position(|node| *node == hit.node)
                .ok_or("picked foreign instance")?;
            self.select_only(picked);
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
        self.focus_selected_bounds()
    }

    fn nearest_model_at(
        &self,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<Option<usize>, Box<dyn std::error::Error>> {
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
        Ok(nearest.map(|(index, _)| index))
    }

    pub(crate) fn pick_model(
        &mut self,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(index) = self.nearest_model_at(cursor, size)? else {
            return Ok(false);
        };
        if self.modifiers.super_key() || self.modifiers.control_key() || self.modifiers.shift_key()
        {
            self.toggle_selected_index(index);
        } else {
            self.select_only(index);
        }
        self.update_edit_title();
        Ok(true)
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
        let roots = self.selected_transform_roots()?;
        if roots.is_empty() {
            return Ok(());
        }
        let primary = self.instances.get(self.selected).copied();
        let node = primary
            .filter(|p| roots.iter().any(|(_, r)| r == p))
            .unwrap_or(roots[0].1);
        let mut companions = Vec::with_capacity(roots.len().saturating_sub(1));
        for (_, other) in roots {
            if other != node {
                companions.push((
                    other,
                    self.scene.local(other)?,
                    self.scene.world_matrix(other)?.w_axis.truncate(),
                ));
            }
        }
        self.drag = Some(ModelDrag {
            node,
            origin: cursor,
            viewport,
            viewport_origin: Vec2::ZERO,
            original: self.scene.local(node)?,
            world_origin: self.scene.world_matrix(node)?.w_axis.truncate(),
            axis: DragAxis::Plane,
            mode: self.gizmo_mode,
            companions,
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
        } else if let Some(hit_index) = self.nearest_model_at(cursor, viewport)? {
            if self.modifiers.super_key()
                || self.modifiers.control_key()
                || self.modifiers.shift_key()
            {
                self.toggle_selected_index(hit_index);
                self.update_edit_title();
            } else {
                if !self.selected_indices().contains(&hit_index) {
                    self.select_only(hit_index);
                    self.update_edit_title();
                }
                self.begin_drag(cursor, viewport)?;
            }
        }
        if let Some(drag) = &mut self.drag {
            drag.viewport_origin = viewport_origin;
        }
        Ok(())
    }

    fn apply_world_translation_delta(
        &mut self,
        node: NodeId,
        original: Transform,
        world_delta: Vec3,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut local = original;
        let delta = if let Some(parent) = self.scene.parent(node)? {
            let inverse = self.scene.world_matrix(parent)?.inverse();
            if !inverse.is_finite() {
                return Ok(());
            }
            inverse.transform_vector3(world_delta)
        } else {
            world_delta
        };
        local.translation += delta;
        self.scene.set_local(node, local)?;
        Ok(())
    }

    fn apply_world_rotation_pivot(
        &mut self,
        node: NodeId,
        original: Transform,
        world_origin: Vec3,
        pivot: Vec3,
        world_rot: glam::Quat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut local = original;
        if let Some(parent) = self.scene.parent(node)? {
            let parent_world = self.scene.world_matrix(parent)?;
            let inverse = parent_world.inverse();
            if !parent_world.is_finite() || !inverse.is_finite() {
                return Ok(());
            }
            let (_, parent_rot, _) = parent_world.to_scale_rotation_translation();
            local.rotation =
                (parent_rot.inverse() * world_rot * parent_rot * original.rotation).normalize();
            let world_delta = pivot + world_rot * (world_origin - pivot) - world_origin;
            local.translation += inverse.transform_vector3(world_delta);
        } else {
            local.rotation = (world_rot * original.rotation).normalize();
            local.translation += pivot + world_rot * (world_origin - pivot) - world_origin;
        }
        self.scene.set_local(node, local)?;
        Ok(())
    }

    fn snap_translation_world_delta(
        &self,
        axis: DragAxis,
        world_origin: Vec3,
        raw_delta: Vec3,
        snap_active: bool,
    ) -> Vec3 {
        if !snap_active {
            return raw_delta;
        }
        let step = self.gizmo_snap.translation_step;
        match self.gizmo_snap.space {
            gizmo::GizmoSnapSpace::Displacement => gizmo::snap_translation_to_grid(raw_delta, step),
            gizmo::GizmoSnapSpace::WorldGrid => {
                let target = world_origin + raw_delta;
                let snapped = gizmo::snap_translation_to_grid(target, step);
                match axis {
                    DragAxis::Plane => snapped - world_origin,
                    DragAxis::X => Vec3::new(snapped.x - world_origin.x, raw_delta.y, raw_delta.z),
                    DragAxis::Y => Vec3::new(raw_delta.x, snapped.y - world_origin.y, raw_delta.z),
                    DragAxis::Z => Vec3::new(raw_delta.x, raw_delta.y, snapped.z - world_origin.z),
                }
            }
        }
    }

    fn restore_drag_transforms(
        scene: &mut SceneGraph,
        node: NodeId,
        original: Transform,
        companions: &[(NodeId, Transform, Vec3)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        scene.set_local(node, original)?;
        for &(companion, companion_original, _) in companions {
            scene.set_local(companion, companion_original)?;
        }
        Ok(())
    }

    pub(crate) fn preview_drag(&mut self, cursor: Vec2) -> Result<(), Box<dyn std::error::Error>> {
        self.trace.pending |= self.drag.is_some();
        let Some(drag) = self.drag.as_ref() else {
            return Ok(());
        };
        let params = DragPreviewParams {
            node: drag.node,
            origin: drag.origin,
            viewport: drag.viewport,
            original: drag.original,
            world_origin: drag.world_origin,
            axis: drag.axis,
            cursor: cursor - drag.viewport_origin,
            snap_active: self.gizmo_snap.enabled ^ self.modifiers.shift_key(),
            companions: drag.companions.clone(),
        };
        match drag.mode {
            gizmo::GizmoMode::Translate => self.preview_translate_drag(params),
            gizmo::GizmoMode::Rotate => self.preview_rotate_drag(params),
            gizmo::GizmoMode::Scale => self.preview_scale_drag(params),
        }
    }

    fn preview_translate_drag(
        &mut self,
        p: DragPreviewParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vp = self.camera.matrix(p.viewport)?;
        let depth = vp.project_point3(p.world_origin).z;
        let plane_delta = camera::unproject(vp, p.cursor, p.viewport, depth)?
            - camera::unproject(vp, p.origin, p.viewport, depth)?;
        let raw_delta = match p.axis {
            DragAxis::Plane => plane_delta,
            DragAxis::X => {
                gizmo::axis_delta(vp, p.world_origin, Vec3::X, p.cursor - p.origin, p.viewport)
            }
            DragAxis::Y => {
                gizmo::axis_delta(vp, p.world_origin, Vec3::Y, p.cursor - p.origin, p.viewport)
            }
            DragAxis::Z => {
                gizmo::axis_delta(vp, p.world_origin, Vec3::Z, p.cursor - p.origin, p.viewport)
            }
        };
        let world_delta =
            self.snap_translation_world_delta(p.axis, p.world_origin, raw_delta, p.snap_active);
        self.apply_world_translation_delta(p.node, p.original, world_delta)?;
        for (other, orig, _) in p.companions {
            self.apply_world_translation_delta(other, orig, world_delta)?;
        }
        Ok(())
    }

    fn preview_rotate_drag(
        &mut self,
        p: DragPreviewParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw_angle = ((p.cursor.x - p.origin.x) - (p.cursor.y - p.origin.y)) * 0.015;
        let angle = if p.snap_active {
            gizmo::snap_angle_radians(raw_angle, self.gizmo_snap.angle_step_rad)
        } else {
            raw_angle
        };
        let axis_vec = match p.axis {
            DragAxis::X => Vec3::X,
            DragAxis::Y => Vec3::Y,
            DragAxis::Z | DragAxis::Plane => Vec3::Z,
        };
        let world_rot = glam::Quat::from_axis_angle(axis_vec, angle);
        self.apply_world_rotation_pivot(
            p.node,
            p.original,
            p.world_origin,
            p.world_origin,
            world_rot,
        )?;
        for (other, orig, other_world) in p.companions {
            self.apply_world_rotation_pivot(other, orig, other_world, p.world_origin, world_rot)?;
        }
        Ok(())
    }

    fn preview_scale_drag(
        &mut self,
        p: DragPreviewParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let delta = match p.axis {
            DragAxis::X => (p.cursor.x - p.origin.x) * 0.01,
            DragAxis::Y => -(p.cursor.y - p.origin.y) * 0.01,
            DragAxis::Z | DragAxis::Plane => {
                ((p.cursor.x - p.origin.x) - (p.cursor.y - p.origin.y)) * 0.01
            }
        };
        let ratio = (1.0 + delta).max(0.05);
        let factor = match p.axis {
            DragAxis::X => Vec3::new(ratio, 1.0, 1.0),
            DragAxis::Y => Vec3::new(1.0, ratio, 1.0),
            DragAxis::Z | DragAxis::Plane => Vec3::splat(ratio),
        };
        let mut apply_scale =
            |node: NodeId, orig: Transform| -> Result<(), Box<dyn std::error::Error>> {
                let mut local = orig;
                let mut scaled = (orig.scale * factor).max(Vec3::splat(0.05));
                if p.snap_active {
                    let snapped = gizmo::snap_scale_factor(scaled, self.gizmo_snap.scale_step);
                    scaled = match p.axis {
                        DragAxis::X => Vec3::new(snapped.x, orig.scale.y, orig.scale.z),
                        DragAxis::Y => Vec3::new(orig.scale.x, snapped.y, orig.scale.z),
                        DragAxis::Z | DragAxis::Plane => snapped,
                    };
                }
                local.scale = scaled.max(Vec3::splat(0.05));
                self.scene.set_local(node, local)?;
                Ok(())
            };
        apply_scale(p.node, p.original)?;
        for (other, orig, _) in p.companions {
            apply_scale(other, orig)?;
        }
        Ok(())
    }

    pub(crate) fn finish_drag(&mut self, commit: bool) -> Result<(), Box<dyn std::error::Error>> {
        let Some(drag) = self.drag.take() else {
            return Ok(());
        };
        if self.trace.enabled {
            println!("EDITOR DRAG FINISH frames={} commit={commit}", self.frames);
        }
        self.trace.pending = true;
        self.trace.last_outcome = None;
        let ModelDrag {
            node,
            original,
            companions,
            mut edit,
            ..
        } = drag;
        if !commit {
            Self::restore_drag_transforms(&mut self.scene, node, original, &companions)?;
            return Ok(());
        }
        *edit.document_mut() = match self.authoring_document() {
            Ok(document) => document,
            Err(error) => {
                Self::restore_drag_transforms(&mut self.scene, node, original, &companions)?;
                return Err(error.into());
            }
        };
        if let Err(error) = self
            .authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_edit(edit, &self.authoring.authoring_project.registry)
        {
            Self::restore_drag_transforms(&mut self.scene, node, original, &companions)?;
            return Err(error.into());
        }
        Ok(())
    }
}
