//! Multi-selection, subtree clipboard copy/paste, and batch authoring operations.

use crate::*;

impl App {
    pub(crate) fn sanitize_selection(&mut self) {
        if self.instances.is_empty() {
            self.selected = 0;
            self.extra_selected.clear();
            return;
        }
        self.selected = self.selected.min(self.instances.len() - 1);
        let primary = self.selected;
        self.extra_selected
            .retain(|&index| index < self.instances.len() && index != primary);
    }

    pub(crate) fn selected_indices(&self) -> Vec<usize> {
        if self.instances.is_empty() {
            return Vec::new();
        }
        let mut indices = BTreeSet::new();
        if self.selected < self.instances.len() {
            indices.insert(self.selected);
        }
        for &index in &self.extra_selected {
            if index < self.instances.len() {
                indices.insert(index);
            }
        }
        indices.into_iter().collect()
    }

    pub(crate) fn select_only(&mut self, index: usize) {
        if self.instances.is_empty() {
            self.selected = 0;
            self.extra_selected.clear();
            return;
        }
        self.selected = index.min(self.instances.len() - 1);
        self.extra_selected.clear();
    }

    pub(crate) fn toggle_selected_index(&mut self, index: usize) {
        if index >= self.instances.len() {
            return;
        }
        if index == self.selected {
            if let Some(next_primary) = self.extra_selected.pop_first() {
                self.selected = next_primary;
            }
        } else if !self.extra_selected.remove(&index) {
            self.extra_selected.insert(index);
        }
        self.sanitize_selection();
    }

    pub(crate) fn select_hierarchy_range(&mut self, target_index: usize) {
        if target_index >= self.instances.len() {
            return;
        }
        let Ok(document) = self.authoring_document() else {
            self.select_only(target_index);
            return;
        };
        let order: Vec<usize> = crate::panels::hierarchy::hierarchy_order(&document)
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        let a = order.iter().position(|&idx| idx == self.selected);
        let b = order.iter().position(|&idx| idx == target_index);
        self.extra_selected.clear();
        if let (Some(a), Some(b)) = (a, b) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            for &idx in &order[lo..=hi] {
                if idx != target_index {
                    self.extra_selected.insert(idx);
                }
            }
            self.selected = target_index;
        } else {
            self.selected = target_index;
        }
        self.sanitize_selection();
    }

    pub(crate) fn select_all_objects(&mut self) {
        if self.instances.is_empty() {
            self.selected = 0;
            self.extra_selected.clear();
            return;
        }
        self.selected = self.selected.min(self.instances.len() - 1);
        let primary = self.selected;
        self.extra_selected = (0..self.instances.len())
            .filter(|&idx| idx != primary)
            .collect();
    }

    pub(crate) fn select_object_ids(&mut self, ids: &[ObjectId]) {
        let mut chosen = Vec::new();
        for id in ids {
            if let Some(index) = self.object_ids.iter().position(|candidate| candidate == id)
                && !chosen.contains(&index)
            {
                chosen.push(index);
            }
        }
        if let Some((&first, rest)) = chosen.split_first() {
            self.selected = first;
            self.extra_selected = rest.iter().copied().collect();
        }
        self.sanitize_selection();
    }

    pub(crate) fn selected_transform_roots(
        &self,
    ) -> Result<Vec<(usize, NodeId)>, Box<dyn std::error::Error>> {
        let indices = self.selected_indices();
        let selected_nodes: BTreeSet<NodeId> =
            indices.iter().map(|&idx| self.instances[idx]).collect();
        let mut roots = Vec::with_capacity(indices.len());
        for idx in indices {
            let node = self.instances[idx];
            let mut ancestor = self.scene.parent(node)?;
            let mut covered = false;
            while let Some(parent) = ancestor {
                if selected_nodes.contains(&parent) {
                    covered = true;
                    break;
                }
                ancestor = self.scene.parent(parent)?;
            }
            if !covered {
                roots.push((idx, node));
            }
        }
        Ok(roots)
    }

    pub(crate) fn selected_subtree_duplicate_needed(
        &self,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.extra_selected.is_empty() {
            return Ok(true);
        }
        let Some(&node) = self.instances.get(self.selected) else {
            return Ok(false);
        };
        for &candidate in &self.instances {
            if self.scene.parent(candidate)? == Some(node) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn copy_selection(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let ids: Vec<ObjectId> = self
            .selected_indices()
            .into_iter()
            .filter_map(|idx| self.object_ids.get(idx).cloned())
            .collect();
        if ids.is_empty() {
            return Err("cannot copy empty selection".into());
        }
        let doc = self.authoring_document()?;
        let clip = voxy_scene::SceneClipboard::capture(&doc, &ids)?;
        let count = clip.len();
        self.authoring.clipboard = Some(clip);
        self.authoring.paste_serial = 0;
        Ok(count)
    }

    pub(crate) fn paste_clipboard(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let Some(clip) = self.authoring.clipboard.clone() else {
            return Err("clipboard is empty".into());
        };
        let next_serial = self.authoring.paste_serial.saturating_add(1);
        let step = u16::try_from(next_serial.min(64)).unwrap_or(64);
        let offset = [0.25 * f32::from(step), 0.0, 0.0];
        let created = self.apply_clipboard_paste(&clip, offset)?;
        self.authoring.paste_serial = next_serial;
        Ok(created.len())
    }

    pub(crate) fn duplicate_selected_subtrees(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ids: Vec<ObjectId> = self
            .selected_indices()
            .into_iter()
            .filter_map(|idx| self.object_ids.get(idx).cloned())
            .collect();
        if ids.is_empty() {
            return Err("missing selected object".into());
        }
        let doc = self.authoring_document()?;
        let clip = voxy_scene::SceneClipboard::capture(&doc, &ids)?;
        let _ = self.apply_clipboard_paste(&clip, [0.15, 0.0, 0.0])?;
        Ok(())
    }

    fn apply_clipboard_paste(
        &mut self,
        clipboard: &voxy_scene::SceneClipboard,
        offset: [f32; 3],
    ) -> Result<Vec<ObjectId>, Box<dyn std::error::Error>> {
        let doc = self.authoring_document()?;
        let mut next_id = self.authoring.next_object_id;
        let paste = clipboard.paste(
            &doc,
            &self.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
            offset,
            &mut next_id,
        )?;
        let next_after = self.validate_authoring_document(&paste.document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(paste.document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next_id.max(next_after);
        self.select_object_ids(&paste.root_ids);
        self.update_edit_title();
        Ok(paste.new_ids)
    }

    pub(crate) fn delete_selected_subtrees(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let roots: Vec<ObjectId> = self
            .selected_indices()
            .into_iter()
            .filter_map(|idx| self.object_ids.get(idx).cloned())
            .collect();
        if roots.is_empty() {
            return Err("missing selected object".into());
        }
        if roots.len() == 1 {
            return self.delete_authoring_subtree(&roots[0]);
        }
        let mut document = self.authoring_document()?;
        let mut children: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
        for object in &document.objects {
            if let Some(parent) = &object.parent {
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(object.id.clone());
            }
        }
        let mut removed = BTreeSet::new();
        let mut pending = roots;
        while let Some(id) = pending.pop() {
            if removed.insert(id.clone()) {
                pending.extend(children.remove(&id).unwrap_or_default());
            }
        }
        document
            .objects
            .retain(|object| !removed.contains(&object.id));
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.update_edit_title();
        Ok(())
    }

    pub(crate) fn toggle_selected_active(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let indices = self.selected_indices();
        if indices.is_empty() {
            return Err("missing selected node".into());
        }
        for idx in indices {
            let node = self.instances[idx];
            self.scene
                .set_active(node, !self.scene.active_self(node)?)?;
        }
        Ok(())
    }

    pub(crate) fn nudge_selected_roots(
        &mut self,
        delta: Vec3,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let roots = self.selected_transform_roots()?;
        if roots.is_empty() {
            return Err("missing selected node".into());
        }
        for (_, node) in roots {
            let mut local = self.scene.local(node)?;
            local.translation += delta;
            self.scene.set_local(node, local)?;
        }
        Ok(())
    }

    pub(crate) fn handle_selection_and_gizmo_shortcut(
        &mut self,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cmd = self.modifiers.super_key() || self.modifiers.control_key();
        if cmd {
            match key {
                KeyCode::KeyC => {
                    self.copy_selection()?;
                    self.panel_cache = None;
                    self.update_edit_title();
                    return Ok(true);
                }
                KeyCode::KeyV => {
                    self.paste_clipboard()?;
                    return Ok(true);
                }
                KeyCode::KeyA => {
                    self.select_all_objects();
                    self.panel_cache = None;
                    self.update_edit_title();
                    return Ok(true);
                }
                KeyCode::KeyZ => {
                    self.parenting = None;
                    self.history_key(if self.modifiers.shift_key() {
                        KeyCode::KeyY
                    } else {
                        KeyCode::KeyZ
                    })?;
                    return Ok(true);
                }
                KeyCode::KeyY => {
                    self.parenting = None;
                    self.history_key(KeyCode::KeyY)?;
                    return Ok(true);
                }
                _ => {}
            }
            return Ok(false);
        }
        match key {
            KeyCode::KeyW => self.gizmo_mode = gizmo::GizmoMode::Translate,
            KeyCode::KeyE => self.gizmo_mode = gizmo::GizmoMode::Rotate,
            KeyCode::KeyT => self.gizmo_mode = gizmo::GizmoMode::Scale,
            KeyCode::KeyN => self.gizmo_snap.enabled = !self.gizmo_snap.enabled,
            KeyCode::Backslash => {
                self.gizmo_snap.space = match self.gizmo_snap.space {
                    gizmo::GizmoSnapSpace::Displacement => gizmo::GizmoSnapSpace::WorldGrid,
                    gizmo::GizmoSnapSpace::WorldGrid => gizmo::GizmoSnapSpace::Displacement,
                };
            }
            KeyCode::BracketLeft => self.gizmo_snap.step_down(self.gizmo_mode),
            KeyCode::BracketRight => self.gizmo_snap.step_up(self.gizmo_mode),
            _ => return Ok(false),
        }
        self.panel_cache = None;
        self.update_edit_title();
        Ok(true)
    }
}
