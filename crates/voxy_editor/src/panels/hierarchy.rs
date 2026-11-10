//! Scene hierarchy tree, resource notice banner, and left-column action toolbar for editor panels.

use glam::Vec2;
use voxy_render::SpriteBatch;
use voxy_scene::SceneDocument;

use super::{Action, Panels};
use crate::InspectorMode;

pub(super) fn visit(
    document: &SceneDocument,
    parent: Option<&voxy_scene::ObjectId>,
    depth: u16,
    order: &mut Vec<(usize, u16)>,
) {
    if depth >= 128 {
        return;
    }
    for (index, object) in document
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.parent.as_ref() == parent)
    {
        order.push((index, depth));
        visit(document, Some(&object.id), depth + 1, order);
    }
}

impl Panels {
    pub(super) fn build_resource_notice(
        &mut self,
        batch: &mut SpriteBatch,
        left: f32,
        rx: f32,
        size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(notice) = self.resource_notice.clone() else {
            return Ok(());
        };
        let width = rx - left - 16.0;
        if width <= 0.0 || size.y < 80.0 {
            return Ok(());
        }
        Self::solid(
            batch,
            [left + 8.0, 8.0, width, 68.0],
            size,
            [0.22, 0.12, 0.06, 1.0],
        )?;
        for (row, text) in notice.lines.iter().enumerate() {
            self.text(
                batch,
                text,
                Vec2::new(left + 16.0, 27.0 + row as f32 * 20.0),
                width - 16.0,
                size,
            )?;
        }
        if notice.retry && !notice.editing && size.y >= 108.0 {
            let x = left
                + 8.0
                + if notice.relink {
                    width.min(240.0) + 8.0
                } else {
                    0.0
                };
            let remaining = left + 8.0 + width - x;
            if remaining >= 120.0 {
                let rect = [x, 80.0, remaining.min(200.0), 24.0];
                Self::solid(batch, rect, size, [0.16, 0.28, 0.42, 1.0])?;
                self.text(
                    batch,
                    "Retry import",
                    Vec2::new(x + 8.0, 97.0),
                    rect[2] - 16.0,
                    size,
                )?;
                self.regions.push((rect, Action::RetryModel));
            }
        }
        if notice.relink && size.y >= 108.0 {
            let rect = [left + 8.0, 80.0, width.min(240.0), 24.0];
            Self::solid(batch, rect, size, [0.16, 0.28, 0.42, 1.0])?;
            self.text(
                batch,
                if notice.editing {
                    "Cancel path entry (Esc)"
                } else {
                    "Relink source file"
                },
                Vec2::new(rect[0] + 8.0, rect[1] + 17.0),
                rect[2] - 16.0,
                size,
            )?;
            self.regions.push((
                rect,
                if notice.editing {
                    Action::RelinkCancel
                } else {
                    Action::RelinkModel
                },
            ));
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "hierarchy and left toolbar share panel layout dimensions and current mode flags"
    )]
    pub(super) fn build_hierarchy_and_toolbar(
        &mut self,
        batch: &mut SpriteBatch,
        document: &SceneDocument,
        selected: usize,
        left: f32,
        size: Vec2,
        scroll: usize,
        parenting: bool,
        playing: bool,
        inspector: InspectorMode,
        prefab_label: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut order = Vec::with_capacity(document.objects.len());
        visit(document, None, 0, &mut order);
        for (row, (index, depth)) in order
            .into_iter()
            .skip(scroll)
            .take(((size.y - 281.0) / 24.0).max(0.0) as usize)
            .enumerate()
        {
            let object = &document.objects[index];
            let y = 38.0 + row as f32 * 24.0;
            let x = 8.0 + (f32::from(depth) * 8.0).min(40.0);
            if index == selected {
                Self::solid(
                    batch,
                    [4.0, y, left - 8.0, 23.0],
                    size,
                    [0.16, 0.28, 0.42, 1.0],
                )?;
            }
            self.text(
                batch,
                &format!("{}{}", if object.active { "" } else { "- " }, object.name),
                Vec2::new(x, y + 17.0),
                left - x - 4.0,
                size,
            )?;
            self.regions.push((
                [4.0, y, left - 8.0, 23.0],
                if let Some(request) = self.follow_pick {
                    Action::FollowTargetChoice(crate::camera_authoring::choice_token(
                        request, &object.id,
                    ))
                } else {
                    Action::Select(index)
                },
            ));
        }
        for (row, label, action) in [
            (
                0,
                if document.objects.is_empty() {
                    "Create object (D)"
                } else {
                    "Duplicate"
                },
                Action::Duplicate,
            ),
            (1, "Delete subtree", Action::Delete),
            (
                2,
                if parenting {
                    "Choose parent in tree"
                } else {
                    "Parent / detach"
                },
                Action::Parent,
            ),
            (
                3,
                if playing { "Stop (F6)" } else { "Play (F6)" },
                Action::Play,
            ),
            (
                4,
                if let Some(status) = self.load_status {
                    status.label
                } else if inspector == InspectorMode::ImportSettings {
                    "Save settings (F5)"
                } else {
                    "Save (F5)"
                },
                Action::Save,
            ),
            (
                5,
                if self.load_status.is_some_and(|status| status.cancellable) {
                    "Cancel load (Esc)"
                } else if self.load_status.is_some() {
                    "Please wait..."
                } else if inspector == InspectorMode::ImportSettings {
                    "Reload settings (F9)"
                } else {
                    "Load (F9)"
                },
                if self.load_status.is_some() {
                    Action::CancelLoad
                } else {
                    Action::Load
                },
            ),
            (6, "Revert instance", Action::RevertPrefab),
            (7, prefab_label, Action::PrefabChoice),
            (8, "Place prefab", Action::PlacePrefab),
            (9, "Create prefab", Action::CreatePrefab),
        ] {
            let y = size.y - 240.0 + row as f32 * 24.0;
            self.text(batch, label, Vec2::new(10.0, y + 17.0), left - 16.0, size)?;
            let enabled = match (row, self.load_status) {
                (4, Some(_)) => false,
                (5, Some(status)) => status.cancellable,
                _ => true,
            };
            if enabled {
                self.regions.push(([4.0, y, left - 8.0, 23.0], action));
            }
        }
        Ok(())
    }
}
