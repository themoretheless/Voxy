//! Component field pages, marker/morph buttons, and identified collection inspector views.

use glam::Vec2;
use voxy_render::SpriteBatch;
use voxy_scene::{SceneDocument, SceneObject};

use super::{Action, Panels};

// Keep foot authoring names readable in the existing component inspector.
fn foot_field_label(path: &str) -> Option<String> {
    let mut parts = path.strip_prefix("/feet/")?.split('/');
    let foot = parts.next()?.parse::<usize>().ok()?;
    let field = parts.next()?;
    let member = parts.next();
    let label = match (field, member) {
        ("bones", Some("0")) => "Hip bone name",
        ("bones", Some("1")) => "Knee bone name",
        ("bones", Some("2")) => "Foot bone name",
        ("plant", None) => "Enable foot planting",
        ("weight", None) => "Foot IK weight",
        ("sole_offset", Some("0")) => "Sole offset X",
        ("sole_offset", Some("1")) => "Sole offset Y",
        ("sole_offset", Some("2")) => "Sole offset Z",
        ("sole_up", Some("0")) => "Sole up X",
        ("sole_up", Some("1")) => "Sole up Y",
        ("sole_up", Some("2")) => "Sole up Z",
        ("pole", Some("0")) => "Knee pole X",
        ("pole", Some("1")) => "Knee pole Y",
        ("pole", Some("2")) => "Knee pole Z",
        ("contact", Some("plant_distance")) => "Plant distance",
        ("contact", Some("release_distance")) => "Release distance",
        ("contact", Some("min_up_dot")) => "Minimum ground up dot",
        _ => return None,
    };
    parts
        .next()
        .is_none()
        .then(|| format!("Foot {foot}: {label}"))
}

fn retarget_field_label(path: &str) -> Option<String> {
    if path == "/source" {
        return Some("Animation source asset".into());
    }
    let mut parts = path.strip_prefix("/joints/")?.split('/');
    let pair = parts.next()?.parse::<usize>().ok()? + 1;
    let member = parts.next()?;
    let label = match member {
        "source" => "Source bone".into(),
        "target" => "Target bone".into(),
        "translation_scale" => "Translation scale".into(),
        "rotation_basis" | "translation_basis" => {
            let axis = parts.next()?.parse::<usize>().ok()?;
            let component = ["X", "Y", "Z", "W"].get(axis)?;
            format!(
                "{} basis {component}",
                if member == "rotation_basis" {
                    "Rotation"
                } else {
                    "Translation"
                }
            )
        }
        _ => return None,
    };
    parts
        .next()
        .is_none()
        .then(|| format!("Pair {pair}: {label}"))
}

impl Panels {
    #[expect(
        clippy::too_many_arguments,
        reason = "component inspector page renders within the caller's sprite batch and column bounds"
    )]
    pub(super) fn build_components_page(
        &mut self,
        batch: &mut SpriteBatch,
        document: &SceneDocument,
        object: &SceneObject,
        page: usize,
        playing: bool,
        field: Option<(usize, &str)>,
        rx: f32,
        right: f32,
        size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fields = crate::component_fields::fields(object)?;
        let marker_token = crate::component_fields::marker_list_token(object)?;
        for (row, (index, member)) in fields.iter().enumerate().skip(page * 6).take(6).enumerate() {
            let y = 116.0 + row as f32 * 26.0;
            let text = field.filter(|(id, _)| *id == index).map_or_else(
                || {
                    if member.schema == "game.follow-camera.v1" && member.path == "/target" {
                        document
                            .objects
                            .iter()
                            .find(|candidate| {
                                Some(candidate.id.0.as_str()) == member.value.as_str()
                            })
                            .map_or_else(|| member.display(), |candidate| candidate.name.clone())
                    } else if member.schema == "editor.model-animation.v1"
                        && member.path == "/events"
                    {
                        format!("{} markers", member.value.as_array().map_or(0, Vec::len))
                    } else {
                        member.display()
                    }
                },
                |(_, text)| text.into(),
            );
            let overridden = !playing && self.overridden_fields.contains(&index);
            let label_width: f32 = "Reset"
                .chars()
                .map(|character| self.glyphs[&character].0.advance)
                .sum();
            let button_width = label_width.ceil() + 12.0;
            let bone_choice = self.retarget_draft
                && member.schema == "editor.model-retarget.v1"
                && member.path.starts_with("/joints/")
                && (member.path.ends_with("/source") || member.path.ends_with("/target"));
            let delete_pair = (bone_choice && member.path.ends_with("/source"))
                .then(|| {
                    member
                        .path
                        .split('/')
                        .nth(2)
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .flatten();
            let marker_button = if !playing
                && member.schema == "game.follow-camera.v1"
                && member.path == "/target"
            {
                Some((
                    "Pick",
                    Action::FollowTarget(crate::camera_authoring::request_token(object)?),
                ))
            } else if !playing && member.schema == "editor.model-animation.v1" {
                let token = marker_token;
                if member.path == "/events" {
                    Some(("Add", Action::MarkerAdd(token)))
                } else if member.path.starts_with("/events/") && member.path.ends_with("/name") {
                    member
                        .path
                        .split('/')
                        .nth(2)
                        .and_then(|index| index.parse().ok())
                        .map(|index| ("Remove", Action::MarkerDelete(index, token)))
                } else if member.path.starts_with("/events/") && member.path.ends_with("/phase") {
                    member
                        .path
                        .split('/')
                        .nth(2)
                        .and_then(|index| index.parse().ok())
                        .map(|index| ("Preview", Action::MarkerPreview(index, token)))
                } else {
                    None
                }
            } else {
                None
            };
            let width = right
                - if overridden { button_width + 12.0 } else { 8.0 }
                - if bone_choice { 42. } else { 0. }
                - if delete_pair.is_some() { 58. } else { 0. }
                - if marker_button.is_some() { 72. } else { 0. };
            let marker_label = if member.schema == "editor.model-animation.v1"
                && member.path.starts_with("/events/")
            {
                member
                    .path
                    .split('/')
                    .nth(2)
                    .and_then(|index| index.parse::<usize>().ok())
                    .map(|index| {
                        format!(
                            "{} {}",
                            if member.path.ends_with("/name") {
                                "Name"
                            } else {
                                "Phase"
                            },
                            index + 1
                        )
                    })
            } else {
                None
            };
            let foot_label = if member.schema == "editor.model-retarget.v1" {
                retarget_field_label(&member.path)
            } else {
                (member.schema == "editor.foot-placement.v1")
                    .then(|| foot_field_label(&member.path))
                    .flatten()
            };
            let label = if member.schema == "game.follow-camera.v1" {
                match member.path.as_str() {
                    "/target" => "Follow target",
                    "/half_life" => "Follow half-life (s)",
                    "/offset/0" => "Follow offset X",
                    "/offset/1" => "Follow offset Y",
                    "/offset/2" => "Follow offset Z",
                    _ => member.path.as_str(),
                }
            } else if member.schema == "editor.model-animation.v1" {
                match member.path.as_str() {
                    "/root_motion_joint" => "Motion bone index",
                    "/clip_name" => "Clip name",
                    "/transition_seconds" => "Clip transition seconds",
                    "/root_motion_bone" => "Motion bone name",
                    "/root_motion_axes/0" => "Root motion X",
                    "/root_motion_axes/1" => "Root motion Y",
                    "/root_motion_axes/2" => "Root motion Z",
                    "/root_motion_rotation" => "Root motion rotation",
                    "/events" => "",
                    _ => marker_label.as_deref().unwrap_or(member.path.as_str()),
                }
            } else if matches!(
                member.schema.as_str(),
                "editor.foot-placement.v1" | "editor.model-retarget.v1"
            ) {
                foot_label.as_deref().unwrap_or(member.path.as_str())
            } else if let Some(label) = marker_label.as_deref() {
                label
            } else if member.path.is_empty() {
                member.schema.as_str()
            } else {
                member.path.as_str()
            };
            self.text(
                batch,
                &format!("{}{text} {label}", if overridden { "* " } else { "" }),
                Vec2::new(rx + 10.0, y + 18.0),
                width - 8.0,
                size,
            )?;
            self.regions
                .push(([rx + 4.0, y, width, 25.0], Action::Field(index)));
            if let Some((label, action)) = marker_button {
                let rect = [rx + width + 4., y, 68., 25.];
                Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.0])?;
                self.text(batch, label, Vec2::new(rect[0] + 3., y + 18.), 62., size)?;
                self.regions.push((rect, action));
            }
            if bone_choice {
                let rect = [rx + width + 4., y, 38., 25.];
                self.text(batch, "Pick", Vec2::new(rect[0] + 3., y + 18.), 34., size)?;
                self.regions.push((rect, Action::RetargetBones(index)));
            }
            if let Some(pair) = delete_pair {
                let rect = [rx + width + 46., y, 54., 25.];
                self.text(batch, "Remove", Vec2::new(rect[0] + 3., y + 18.), 48., size)?;
                self.regions.push((rect, Action::RetargetDeletePair(pair)));
            }
            if overridden {
                let rect = [rx + right - button_width - 4.0, y, button_width, 25.0];
                Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.0])?;
                self.text(
                    batch,
                    "Reset",
                    Vec2::new(rect[0] + 6.0, y + 18.0),
                    button_width - 12.0,
                    size,
                )?;
                self.regions.push((rect, Action::ResetField(index)));
            }
        }
        if let Some(member) = field
            .and_then(|(index, _)| fields.get(index))
            .or_else(|| fields.get(page * 6))
        {
            self.text(
                batch,
                if member.schema == "editor.model-animation.v1" {
                    "Animation"
                } else {
                    &member.schema
                },
                Vec2::new(rx + 10.0, 284.0),
                right - 16.0,
                size,
            )?;
        }
        if !playing
            && !self.retarget_draft
            && fields
                .iter()
                .any(|field| field.schema == "editor.model-animation.v1")
        {
            let rect = [rx + 8., 334., right - 16., 24.];
            Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.])?;
            let phase = self.preview_phase.unwrap_or(0.);
            Self::solid(
                batch,
                [
                    rect[0] + (rect[2] - 3.) * phase as f32,
                    rect[1],
                    3.,
                    rect[3],
                ],
                size,
                [0.5, 0.8, 1., 1.],
            )?;
            self.text(
                batch,
                &format!("Pose phase {phase:.3}"),
                Vec2::new(rect[0] + 6., rect[1] + 18.),
                rect[2] - 12.,
                size,
            )?;
            self.regions.push((rect, Action::PreviewSeek));
            let token = crate::component_fields::morph_edit_token(object);
            for (y, label, action) in [
                (362., "Import morph defaults", Action::MorphDefaults(token)),
                (390., "Resume morph", Action::MorphResume(token)),
            ] {
                let rect = [rx + 8., y, right - 16., 24.];
                Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.])?;
                self.text(
                    batch,
                    label,
                    Vec2::new(rect[0] + 6., y + 18.),
                    rect[2] - 12.,
                    size,
                )?;
                self.regions.push((rect, action));
            }
        }
        for (y, label, forward) in [
            (280.0, "Previous fields", false),
            (306.0, "Next fields", true),
        ] {
            self.text(
                batch,
                label,
                Vec2::new(rx + 10.0, y + 18.0),
                right - 16.0,
                size,
            )?;
            self.regions.push((
                [rx + 4.0, y, right - 8.0, 25.0],
                Action::ComponentPage(forward),
            ));
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "collections inspector renders paginated items and deleted-item recovery in one column"
    )]
    pub(super) fn build_collections_page(
        &mut self,
        batch: &mut SpriteBatch,
        object: &SceneObject,
        index: usize,
        page: usize,
        playing: bool,
        rx: f32,
        right: f32,
        size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lists = crate::component_collections::collections(object, &self.registry)?;
        if let Some(list) = lists.get(index % lists.len().max(1)) {
            self.text(
                batch,
                &format!("{} {}", list.schema, list.path),
                Vec2::new(rx + 10.0, 134.0),
                right - 16.0,
                size,
            )?;
            self.regions.push((
                [rx + 4.0, 116.0, right - 8.0, 25.0],
                Action::CollectionChoice,
            ));
            if self
                .registry
                .new_collection_item(list.schema, list.path, "preview")
                .is_ok()
            {
                self.text(
                    batch,
                    "Add item",
                    Vec2::new(rx + 10.0, 160.0),
                    right - 16.0,
                    size,
                )?;
                self.regions.push((
                    [rx + 4.0, 142.0, right - 8.0, 25.0],
                    Action::CollectionAdd(list.key),
                ));
            } else {
                self.text(
                    batch,
                    "No item default",
                    Vec2::new(rx + 10.0, 160.0),
                    right - 16.0,
                    size,
                )?;
            }
            let page = page % list.items.len().div_ceil(3).max(1);
            for (row, (index, item)) in list
                .items
                .iter()
                .enumerate()
                .skip(page * 3)
                .take(3)
                .enumerate()
            {
                let id = item[list.identity]
                    .as_str()
                    .ok_or("invalid collection ID")?;
                let target = crate::component_collections::item_key(id);
                let y = 168.0 + row as f32 * 44.0;
                let reset = !playing && self.collection_resets.contains(&(list.key, target));
                let reset_width = "Reset"
                    .chars()
                    .map(|c| self.glyphs[&c].0.advance)
                    .sum::<f32>()
                    .ceil()
                    + 12.0;
                if reset {
                    let rect = [rx + right - reset_width - 4.0, y, reset_width, 18.0];
                    Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.0])?;
                    self.text(
                        batch,
                        "Reset",
                        Vec2::new(rect[0] + 6.0, y + 16.0),
                        reset_width - 12.0,
                        size,
                    )?;
                    self.regions
                        .push((rect, Action::CollectionReset(list.key, target)));
                }
                self.text(
                    batch,
                    &item
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .map_or_else(|| format!("Item {}", index + 1), str::to_owned),
                    Vec2::new(rx + 10.0, y + 16.0),
                    right - 16.0 - if reset { reset_width + 4.0 } else { 0.0 },
                    size,
                )?;
                for (column, label, action, enabled) in [
                    (
                        0,
                        "Delete",
                        Action::CollectionDelete(list.key, target),
                        true,
                    ),
                    (
                        1,
                        "Up",
                        Action::CollectionMove(list.key, target, false),
                        index > 0,
                    ),
                    (
                        2,
                        "Down",
                        Action::CollectionMove(list.key, target, true),
                        index + 1 < list.items.len(),
                    ),
                ] {
                    if enabled {
                        let width = (right - 8.0) / 3.0;
                        let rect = [
                            rx + 4.0 + column as f32 * width,
                            y + 18.0,
                            width - 2.0,
                            24.0,
                        ];
                        Self::solid(batch, rect, size, [0.18, 0.24, 0.3, 1.0])?;
                        self.text(
                            batch,
                            label,
                            Vec2::new(rect[0] + 4.0, y + 36.0),
                            width - 8.0,
                            size,
                        )?;
                        self.regions.push((rect, action));
                    }
                }
            }
            if list.items.is_empty() {
                self.text(
                    batch,
                    "Empty collection",
                    Vec2::new(rx + 10.0, 190.0),
                    right - 16.0,
                    size,
                )?;
            }
            if !playing && self.collection_order_resets.contains(&list.key) {
                self.text(
                    batch,
                    "Reset order",
                    Vec2::new(rx + 10.0, 350.0),
                    right - 16.0,
                    size,
                )?;
                self.regions.push((
                    [rx + 4.0, 332.0, right - 8.0, 24.0],
                    Action::CollectionResetOrder(list.key),
                ));
            }
            for (column, label, forward) in [(0, "Previous", false), (1, "Next", true)] {
                if column == 0 && !playing && self.collection_deleted_resets.contains(&list.key) {
                    self.text(
                        batch,
                        "Restore deleted",
                        Vec2::new(rx + 10.0, 376.0),
                        right - 16.0,
                        size,
                    )?;
                    self.regions.push((
                        [rx + 4.0, 358.0, right - 8.0, 24.0],
                        Action::CollectionRestoreDeleted(list.key),
                    ));
                }
                let width = (right - 8.0) / 2.0;
                self.text(
                    batch,
                    label,
                    Vec2::new(rx + 8.0 + column as f32 * width, 324.0),
                    width - 8.0,
                    size,
                )?;
                self.regions.push((
                    [rx + 4.0 + column as f32 * width, 306.0, width - 2.0, 24.0],
                    Action::CollectionPage(forward),
                ));
            }
            if !playing && let Some(deleted) = self.collection_deleted_items.get(&list.key).cloned()
            {
                let pages = deleted.len().div_ceil(3).max(1);
                let page = self.deleted_page % pages;
                self.text(
                    batch,
                    &format!("Deleted {}/{} (next)", page + 1, pages),
                    Vec2::new(rx + 10.0, 568.0),
                    right - 16.0,
                    size,
                )?;
                self.regions.push((
                    [rx + 4.0, 550.0, right - 8.0, 24.0],
                    Action::CollectionDeletedPage,
                ));
                for (row, (id, label)) in deleted.iter().skip(page * 3).take(3).enumerate() {
                    let y = 576.0 + row as f32 * 26.0;
                    self.text(
                        batch,
                        &format!("Restore {label}"),
                        Vec2::new(rx + 10.0, y + 18.0),
                        right - 16.0,
                        size,
                    )?;
                    self.regions.push((
                        [rx + 4.0, y, right - 8.0, 24.0],
                        Action::CollectionRestoreItem(list.key, *id),
                    ));
                }
            }
        } else {
            self.text(
                batch,
                "No collections",
                Vec2::new(rx + 10.0, 134.0),
                right - 16.0,
                size,
            )?;
        }
        Ok(())
    }
}
