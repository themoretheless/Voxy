//! Scene hierarchy tree, resource notice banner, and left-column action toolbar for editor panels.

use std::collections::HashMap;

use glam::Vec2;
use voxy_render::SpriteBatch;
use voxy_scene::{ObjectId, SceneDocument};

use super::{Action, Panels};
use crate::InspectorMode;

fn hierarchy_order(document: &SceneDocument) -> Vec<(usize, u16)> {
    fn visit(
        document: &SceneDocument,
        children: &HashMap<Option<&ObjectId>, Vec<usize>>,
        parent: Option<&ObjectId>,
        depth: u16,
        order: &mut Vec<(usize, u16)>,
    ) {
        if depth >= 128 {
            return;
        }
        if let Some(indices) = children.get(&parent) {
            for &index in indices {
                order.push((index, depth));
                visit(
                    document,
                    children,
                    Some(&document.objects[index].id),
                    depth + 1,
                    order,
                );
            }
        }
    }

    let mut children = HashMap::<Option<&ObjectId>, Vec<usize>>::new();
    for (index, object) in document.objects.iter().enumerate() {
        children
            .entry(object.parent.as_ref())
            .or_default()
            .push(index);
    }
    let mut order = Vec::with_capacity(document.objects.len());
    visit(document, &children, None, 0, &mut order);
    order
}

fn matching_hierarchy(document: &SceneDocument, query: &str) -> Vec<(usize, u16)> {
    let mut order = hierarchy_order(document);
    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        order.retain(|&(index, _)| {
            let object = &document.objects[index];
            object.name.to_lowercase().contains(&query)
                || object.id.0.to_lowercase().contains(&query)
        });
    }
    order
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
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
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
        let build_label = self.build_label.filter(|_| !playing);
        let run_label = self.package_run_label.filter(|_| !playing);
        let toolbar_height =
            240.0 + 24.0 * (u8::from(build_label.is_some()) + u8::from(run_label.is_some())) as f32;
        let search = self.hierarchy_search.as_deref().filter(|_| !playing);
        let order = matching_hierarchy(document, search.unwrap_or_default());
        if search.is_some() && order.is_empty() {
            self.text(batch, "No matches", Vec2::new(8.0, 55.0), left - 12.0, size)?;
        }
        let scroll = if search.is_some() {
            scroll.min(order.len().saturating_sub(1))
        } else {
            scroll
        };
        for (row, (index, depth)) in order
            .into_iter()
            .skip(scroll)
            .take(((size.y - toolbar_height - 41.0) / 24.0).max(0.0) as usize)
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
            let y = size.y - toolbar_height + row as f32 * 24.0;
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
        if let Some(label) = build_label {
            let y = size.y - if run_label.is_some() { 48.0 } else { 24.0 };
            self.text(batch, label, Vec2::new(10.0, y + 17.0), left - 16.0, size)?;
            if label == "Build saved game" && self.load_status.is_none() {
                self.regions
                    .push(([4.0, y, left - 8.0, 23.0], Action::BuildGame));
            }
        }
        if let Some(label) = run_label {
            let y = size.y - 24.0;
            self.text(batch, label, Vec2::new(10.0, y + 17.0), left - 16.0, size)?;
            if label == "Run built game" && self.load_status.is_none() {
                self.regions
                    .push(([4.0, y, left - 8.0, 23.0], Action::RunBuiltGame));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::SceneObject;

    #[test]
    fn hierarchy_order_preserves_document_indices_and_sibling_order() {
        let object = |id: &str, parent: Option<&str>| SceneObject {
            id: ObjectId(id.into()),
            parent: parent.map(|id| ObjectId(id.into())),
            name: id.into(),
            active: true,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            components: Default::default(),
        };
        let document = SceneDocument {
            version: 1,
            objects: vec![
                object("first-child", Some("root")),
                object("second-root", None),
                object("grandchild", Some("first-child")),
                object("root", None),
                object("second-child", Some("root")),
            ],
        };
        assert_eq!(
            hierarchy_order(&document),
            [(1, 0), (3, 0), (0, 1), (2, 2), (4, 1)]
        );
        let mut renamed = document;
        renamed.objects[0].name = "Камера".into();
        assert_eq!(matching_hierarchy(&renamed, " КАМЕРА "), [(0, 1)]);
        assert_eq!(
            matching_hierarchy(&renamed, "CHILD"),
            [(0, 1), (2, 2), (4, 1)]
        );
        assert!(matching_hierarchy(&renamed, "missing").is_empty());

        let mut panels = Panels::new().unwrap();
        panels.hierarchy_search = Some("КАМЕРА".into());
        panels
            .build(
                &renamed,
                0,
                Vec2::new(1024., 640.),
                false,
                None,
                0,
                false,
                InspectorMode::Transform,
                "Texture: none",
                "",
                None,
            )
            .unwrap();
        assert!(
            panels
                .regions
                .iter()
                .any(|(_, action)| *action == Action::SearchHierarchy)
        );
        let visible_objects: Vec<_> = panels
            .regions
            .iter()
            .filter_map(|(_, action)| match action {
                Action::Select(index) => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(visible_objects, [0]);
    }
}
