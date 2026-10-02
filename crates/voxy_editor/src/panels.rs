//! Small native editor panels rendered through the existing scene renderer.
// Glyph metrics are bounded to a 512x128 atlas, UI rows to 128 scene objects.
#![allow(clippy::cast_precision_loss)]
use crate::InspectorMode;
use glam::Vec2;
use std::collections::BTreeMap;
use voxy_render::{SceneMesh, Sprite, SpriteBatch};
use voxy_scene::SceneDocument;
use voxy_text::{FontLimits, GlyphAtlas, GlyphRegion, RasterFont};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Select(usize),
    Field(usize),
    ResetField(usize),
    ComponentPage(bool),
    CollectionChoice,
    CollectionPage(bool),
    CollectionAdd([u8; 32]),
    CollectionDelete([u8; 32], [u8; 32]),
    CollectionMove([u8; 32], [u8; 32], bool),
    CollectionReset([u8; 32], [u8; 32]),
    CollectionResetOrder([u8; 32]),
    CollectionRestoreDeleted([u8; 32]),
    CollectionRestoreItem([u8; 32], [u8; 32]),
    CollectionDeletedPage,
    Duplicate,
    Delete,
    Resource,
    Parent,
    Active,
    Play,
    Save,
    Load,
    RevertPrefab,
    PlacePrefab,
    CreatePrefab,
    PrefabChoice,
    Character,
    Collider,
    Physics,
    Behavior,
    Motion,
    AudioSource,
    AudioListener,
    AudioBus,
    AudioSettingsLoad,
    AudioSettingsSave,
}
pub(crate) struct Panels {
    registry: std::sync::Arc<voxy_scene::ComponentRegistry>,
    pub collection_resets: std::collections::BTreeSet<([u8; 32], [u8; 32])>,
    pub collection_order_resets: std::collections::BTreeSet<[u8; 32]>,
    pub collection_deleted_resets: std::collections::BTreeSet<[u8; 32]>,
    pub collection_deleted_items: crate::component_collections::DeletedItems,
    pub deleted_page: usize,
    pub overridden_fields: std::collections::BTreeSet<usize>,
    glyphs: BTreeMap<char, (voxy_text::GlyphBitmap, Option<GlyphRegion>)>,
    pub rgba: Vec<u8>,
    pub regions: Vec<([f32; 4], Action)>,
    pub focus: crate::panel_focus::PanelFocus,
    pointer: voxy_ui::PointerRouter,
    presented: bool,
}
impl std::fmt::Debug for Panels {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Panels")
            .field("focus", &self.focus)
            .finish_non_exhaustive()
    }
}
impl Panels {
    #[cfg(test)]
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_registry(std::sync::Arc::new(crate::model_registry()?))
    }
    pub fn with_registry(
        registry: std::sync::Arc<voxy_scene::ComponentRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let paths = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ];
        let bytes = paths
            .iter()
            .find_map(|path| std::fs::read(path).ok())
            .ok_or("editor panels require a system TrueType font (Arial or DejaVu Sans)")?;
        let font = RasterFont::parse(
            &bytes,
            FontLimits {
                max_font_bytes: 8 * 1024 * 1024,
                max_glyph_pixels: 4096,
                max_size: 32.0,
            },
        )?;
        let mut atlas = GlyphAtlas::new(512, 128, 65536, 128)?;
        let mut glyphs = BTreeMap::new();
        for code in 32_u8..=126 {
            let character = char::from(code);
            let glyph = font.rasterize(character, 14.0)?;
            let region = atlas.insert(&glyph)?;
            glyphs.insert(character, (glyph, region));
        }
        Ok(Self {
            presented: false,
            collection_resets: Default::default(),
            collection_order_resets: Default::default(),
            collection_deleted_resets: Default::default(),
            collection_deleted_items: Default::default(),
            deleted_page: 0,
            registry,
            overridden_fields: std::collections::BTreeSet::new(),
            glyphs,
            rgba: {
                let mut rgba: Vec<u8> = atlas
                    .alpha()
                    .iter()
                    .flat_map(|alpha| [255, 255, 255, *alpha])
                    .collect();
                *rgba.last_mut().unwrap() = 255;
                rgba
            },
            regions: Vec::new(),
            focus: crate::panel_focus::PanelFocus::new(),
            pointer: voxy_ui::PointerRouter::new(256),
        })
    }
    pub fn frame_outcome(&mut self, outcome: voxy_render::RenderOutcome) {
        self.presented = outcome == voxy_render::RenderOutcome::Presented;
        if !self.presented {
            self.focus.cancel();
            self.pointer.move_to(None);
        }
    }
    pub fn input_ready(&self) -> bool {
        self.presented
    }
    pub fn invalidate_presentation(&mut self) {
        self.presented = false;
        self.focus.cancel();
        self.pointer.move_to(None);
    }
    pub fn hit(&mut self, point: Vec2) -> Option<Action> {
        if !self.presented {
            return None;
        }
        self.pointer.move_to(Some(point.to_array()));
        self.pointer
            .hovered()
            .and_then(|id| self.focus.action_for(id))
    }
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn build(
        &mut self,
        document: &SceneDocument,
        selected: usize,
        size: Vec2,
        playing: bool,
        field: Option<(usize, &str)>,
        scroll: usize,
        parenting: bool,
        inspector: InspectorMode,
        texture_label: &str,
        prefab_label: &str,
        import_config: Option<voxy_gameplay::AudioImportConfig>,
    ) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        self.presented = false;
        self.regions.clear();
        let mut batch = SpriteBatch::new(16000);
        let left = 150.0_f32.min(size.x * 0.25);
        let right = 180.0_f32.min(size.x * 0.3);
        let rx = size.x - right;
        // Texture white texel is the final unused pixel of the alpha atlas.
        let solid = |batch: &mut SpriteBatch,
                     r: [f32; 4],
                     color|
         -> Result<(), Box<dyn std::error::Error>> {
            let mut sprite = Sprite::from_logical_rect(
                Vec2::new(r[0], r[1]),
                Vec2::new(r[2], r[3]),
                size,
                color,
            )?;
            sprite.uv_min = Vec2::new(511.0 / 512.0, 127.0 / 128.0);
            sprite.uv_max = Vec2::ONE;
            batch.push(sprite)?;
            Ok(())
        };
        solid(
            &mut batch,
            [0.0, 0.0, left, size.y],
            [0.06, 0.075, 0.1, 1.0],
        )?;
        solid(
            &mut batch,
            [rx, 0.0, right, size.y],
            [0.06, 0.075, 0.1, 1.0],
        )?;
        self.text(
            &mut batch,
            "SCENE",
            Vec2::new(10.0, 23.0),
            left - 16.0,
            size,
        )?;
        self.text(
            &mut batch,
            "INSPECTOR",
            Vec2::new(rx + 10.0, 23.0),
            right - 16.0,
            size,
        )?;
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
                solid(
                    &mut batch,
                    [4.0, y, left - 8.0, 23.0],
                    [0.16, 0.28, 0.42, 1.0],
                )?;
            }
            self.text(
                &mut batch,
                &format!("{}{}", if object.active { "" } else { "- " }, object.name),
                Vec2::new(x, y + 17.0),
                left - x - 4.0,
                size,
            )?;
            self.regions
                .push(([4.0, y, left - 8.0, 23.0], Action::Select(index)));
        }
        for (row, label, action) in [
            (0, "Duplicate", Action::Duplicate),
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
                if inspector == InspectorMode::ImportSettings {
                    "Save settings (F5)"
                } else {
                    "Save (F5)"
                },
                Action::Save,
            ),
            (
                5,
                if inspector == InspectorMode::ImportSettings {
                    "Reload settings (F9)"
                } else {
                    "Load (F9)"
                },
                Action::Load,
            ),
            (6, "Revert instance", Action::RevertPrefab),
            (7, prefab_label, Action::PrefabChoice),
            (8, "Place prefab", Action::PlacePrefab),
            (9, "Create prefab", Action::CreatePrefab),
        ] {
            let y = size.y - 240.0 + row as f32 * 24.0;
            self.text(
                &mut batch,
                label,
                Vec2::new(10.0, y + 17.0),
                left - 16.0,
                size,
            )?;
            self.regions.push(([4.0, y, left - 8.0, 23.0], action));
        }
        if let Some(object) = document.objects.get(selected) {
            self.text(
                &mut batch,
                &object.name,
                Vec2::new(rx + 10.0, 48.0),
                right - 16.0,
                size,
            )?;
            self.text(
                &mut batch,
                if object.active {
                    "Active: yes"
                } else {
                    "Active: no"
                },
                Vec2::new(rx + 10.0, 73.0),
                right - 16.0,
                size,
            )?;
            self.regions
                .push(([rx + 4.0, 55.0, right - 8.0, 24.0], Action::Active));
            let audio_mode = inspector == InspectorMode::Audio;
            let resource = object
                .components
                .get(if audio_mode {
                    "game.audio-source.v1"
                } else {
                    "editor.model.v1"
                })
                .and_then(|value| {
                    if audio_mode {
                        value["asset"].as_str()
                    } else {
                        value.as_str()
                    }
                })
                .unwrap_or("?");
            let resource = field
                .filter(|(id, _)| audio_mode && *id == 6)
                .map_or(resource, |(_, text)| text);
            self.text(
                &mut batch,
                &format!("{}: {resource}", if audio_mode { "Audio" } else { "Model" }),
                Vec2::new(rx + 10.0, 98.0),
                right - 16.0,
                size,
            )?;
            self.regions.push((
                [rx + 4.0, 80.0, right - 8.0, 24.0],
                if audio_mode {
                    Action::Field(6)
                } else {
                    Action::Resource
                },
            ));
            let (x, y, z) = glam::Quat::from_array(object.rotation).to_euler(glam::EulerRot::XYZ);
            let transform_values = [
                object.translation[0],
                object.translation[1],
                object.translation[2],
                x.to_degrees(),
                y.to_degrees(),
                z.to_degrees(),
                object.scale[0],
                object.scale[1],
                object.scale[2],
            ];
            let transform_labels = [
                "Pos X", "Pos Y", "Pos Z", "Rot X", "Rot Y", "Rot Z", "Scale X", "Scale Y",
                "Scale Z",
            ];
            let mut values: Vec<(String, f64)> = transform_labels
                .into_iter()
                .zip(transform_values)
                .map(|(label, value)| (label.into(), f64::from(value)))
                .collect();
            if inspector == InspectorMode::Physics {
                values.clear();
                if let Some(body) = object.components.get("game.character.v1") {
                    for (index, label) in ["Half X", "Half Y", "Half Z"].into_iter().enumerate() {
                        values.push((
                            label.into(),
                            body["half_extents"][index].as_f64().unwrap_or(0.0),
                        ));
                    }
                    for (field, label) in [
                        ("speed", "Speed"),
                        ("gravity", "Gravity"),
                        ("jump_speed", "Jump speed"),
                    ] {
                        values.push((label.into(), body[field].as_f64().unwrap_or(0.0)));
                    }
                } else if let Some(collider) = object.components.get("game.box.v1") {
                    for (index, label) in ["Half X", "Half Y", "Half Z"].into_iter().enumerate() {
                        values.push((
                            label.into(),
                            collider["half_extents"][index].as_f64().unwrap_or(0.0),
                        ));
                    }
                }
            }
            if inspector == InspectorMode::ImportSettings {
                values.clear();
                if let Some(config) = import_config {
                    values.extend([
                        ("Bytes".into(), config.max_input_bytes as f64),
                        ("Frames".into(), config.max_frames as f64),
                        ("Filter work".into(), config.max_filter_evaluations as f64),
                    ]);
                }
            }
            if inspector == InspectorMode::Mixer {
                values.clear();
                if let Some(bus) = object.components.get("game.audio-bus.v1") {
                    for (key, label) in [("bus", "Bus 0..15"), ("gain", "Gain 0..1")] {
                        values.push((label.into(), bus[key].as_f64().unwrap_or(0.)));
                    }
                }
            }
            if inspector == InspectorMode::Audio {
                values.clear();
                if let Some(source) = object.components.get("game.audio-source.v1") {
                    for (field, label) in [("gain", "Gain"), ("bus", "Bus")] {
                        values.push((label.into(), source[field].as_f64().unwrap_or(0.)));
                    }
                    for (field, label) in [("looping", "Loop 0/1"), ("spatial", "Spatial 0/1")] {
                        values.push((
                            label.into(),
                            if source[field].as_bool().unwrap_or(false) {
                                1.
                            } else {
                                0.
                            },
                        ));
                    }
                    for (field, label) in [("near", "Near"), ("far", "Far")] {
                        values.push((label.into(), source[field].as_f64().unwrap_or(0.)));
                    }
                }
            }
            if inspector == InspectorMode::Behavior {
                values.clear();
                if let Some(motion) = object.components.get("game.angular-motion.v1") {
                    for (index, label) in ["Axis X", "Axis Y", "Axis Z"].into_iter().enumerate() {
                        values.push((label.into(), motion["axis"][index].as_f64().unwrap_or(0.)));
                    }
                    values.push((
                        "Radians/sec".into(),
                        motion["radians_per_second"].as_f64().unwrap_or(0.),
                    ));
                }
            }
            if let InspectorMode::Components(page) = inspector {
                values.clear();
                let fields = crate::component_fields::fields(object)?;
                for (row, (index, member)) in
                    fields.iter().enumerate().skip(page * 6).take(6).enumerate()
                {
                    let y = 116.0 + row as f32 * 26.0;
                    let text = field
                        .filter(|(id, _)| *id == index)
                        .map_or_else(|| member.display(), |(_, text)| text.into());
                    let overridden = !playing && self.overridden_fields.contains(&index);
                    let label_width: f32 = "Reset"
                        .chars()
                        .map(|character| self.glyphs[&character].0.advance)
                        .sum();
                    let button_width = label_width.ceil() + 12.0;
                    let width = right - if overridden { button_width + 12.0 } else { 8.0 };
                    let label = if member.path.is_empty() {
                        member.schema.as_str()
                    } else {
                        member.path.as_str()
                    };
                    self.text(
                        &mut batch,
                        &format!("{}{text} {label}", if overridden { "* " } else { "" }),
                        Vec2::new(rx + 10.0, y + 18.0),
                        width - 8.0,
                        size,
                    )?;
                    self.regions
                        .push(([rx + 4.0, y, width, 25.0], Action::Field(index)));
                    if overridden {
                        let rect = [rx + right - button_width - 4.0, y, button_width, 25.0];
                        solid(&mut batch, rect, [0.18, 0.24, 0.3, 1.0])?;
                        self.text(
                            &mut batch,
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
                        &mut batch,
                        &member.schema,
                        Vec2::new(rx + 10.0, 270.0),
                        right - 16.0,
                        size,
                    )?;
                }
                for (y, label, forward) in [
                    (280.0, "Previous fields", false),
                    (306.0, "Next fields", true),
                ] {
                    self.text(
                        &mut batch,
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
            }
            if let InspectorMode::Collections(index, page) = inspector {
                values.clear();
                let lists = crate::component_collections::collections(object, &self.registry)?;
                if let Some(list) = lists.get(index % lists.len().max(1)) {
                    self.text(
                        &mut batch,
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
                            &mut batch,
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
                            &mut batch,
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
                        let reset =
                            !playing && self.collection_resets.contains(&(list.key, target));
                        let reset_width = "Reset"
                            .chars()
                            .map(|c| self.glyphs[&c].0.advance)
                            .sum::<f32>()
                            .ceil()
                            + 12.0;
                        if reset {
                            let rect = [rx + right - reset_width - 4.0, y, reset_width, 18.0];
                            solid(&mut batch, rect, [0.18, 0.24, 0.3, 1.0])?;
                            self.text(
                                &mut batch,
                                "Reset",
                                Vec2::new(rect[0] + 6.0, y + 16.0),
                                reset_width - 12.0,
                                size,
                            )?;
                            self.regions
                                .push((rect, Action::CollectionReset(list.key, target)));
                        }
                        self.text(
                            &mut batch,
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
                                solid(&mut batch, rect, [0.18, 0.24, 0.3, 1.0])?;
                                self.text(
                                    &mut batch,
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
                            &mut batch,
                            "Empty collection",
                            Vec2::new(rx + 10.0, 190.0),
                            right - 16.0,
                            size,
                        )?;
                    }
                    if !playing && self.collection_order_resets.contains(&list.key) {
                        self.text(
                            &mut batch,
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
                        if column == 0
                            && !playing
                            && self.collection_deleted_resets.contains(&list.key)
                        {
                            self.text(
                                &mut batch,
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
                            &mut batch,
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
                    if !playing {
                        if let Some(deleted) = self.collection_deleted_items.get(&list.key).cloned()
                        {
                            let pages = deleted.len().div_ceil(3).max(1);
                            let page = self.deleted_page % pages;
                            self.text(
                                &mut batch,
                                &format!("Deleted {}/{} (next)", page + 1, pages),
                                Vec2::new(rx + 10.0, 568.0),
                                right - 16.0,
                                size,
                            )?;
                            self.regions.push((
                                [rx + 4.0, 550.0, right - 8.0, 24.0],
                                Action::CollectionDeletedPage,
                            ));
                            for (row, (id, label)) in
                                deleted.iter().skip(page * 3).take(3).enumerate()
                            {
                                let y = 576.0 + row as f32 * 26.0;
                                self.text(
                                    &mut batch,
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
                    }
                } else {
                    self.text(
                        &mut batch,
                        "No collections",
                        Vec2::new(rx + 10.0, 134.0),
                        right - 16.0,
                        size,
                    )?;
                }
            }
            if inspector == InspectorMode::Material {
                values.clear();
                if let Some(material) = object.components.get("editor.material.v1") {
                    for (index, label) in ["Tint R", "Tint G", "Tint B", "Tint A"]
                        .into_iter()
                        .enumerate()
                    {
                        values.push((label.into(), material["tint"][index].as_f64().unwrap_or(1.)));
                    }
                    values.push((
                        "Lit 0/1".into(),
                        if material["lit"].as_bool().unwrap_or(true) {
                            1.
                        } else {
                            0.
                        },
                    ));
                }
            }
            if inspector == InspectorMode::Material {
                self.text(
                    &mut batch,
                    texture_label,
                    Vec2::new(rx + 10., 286.),
                    right - 16.,
                    size,
                )?;
            }
            for (index, (label, value)) in values.into_iter().enumerate() {
                let y = 116.0 + index as f32 * 26.0;
                let overridden = !playing && self.overridden_fields.contains(&index);
                let reset_text_width: f32 = "Reset"
                    .chars()
                    .map(|character| self.glyphs[&character].0.advance)
                    .sum();
                let reset_width = reset_text_width.ceil() + 12.0;
                let field_width = right - if overridden { reset_width + 12.0 } else { 8.0 };
                if field.is_some_and(|(id, _)| id == index) {
                    solid(
                        &mut batch,
                        [rx + 4.0, y, field_width, 25.0],
                        [0.2, 0.25, 0.32, 1.0],
                    )?;
                }
                let value = field.filter(|(id, _)| *id == index).map_or_else(
                    || {
                        if inspector == InspectorMode::ImportSettings {
                            format!("{value:.0}")
                        } else {
                            format!("{value:.3}")
                        }
                    },
                    |(_, value)| value.to_owned(),
                );
                self.text(
                    &mut batch,
                    &if overridden {
                        format!("* {value} {label}")
                    } else {
                        format!("{label}: {value}")
                    },
                    Vec2::new(rx + 10.0, y + 18.0),
                    field_width - 8.0,
                    size,
                )?;
                self.regions
                    .push(([rx + 4.0, y, field_width, 25.0], Action::Field(index)));
                if overridden {
                    let rect = [rx + right - reset_width - 4.0, y, reset_width, 25.0];
                    solid(&mut batch, rect, [0.18, 0.24, 0.3, 1.0])?;
                    self.text(
                        &mut batch,
                        "Reset",
                        Vec2::new(rect[0] + 6.0, y + 18.0),
                        reset_width - 12.0,
                        size,
                    )?;
                    self.regions.push((rect, Action::ResetField(index)));
                }
            }
            if audio_mode {
                let settings = field
                    .filter(|(id, _)| *id == 7)
                    .map(|(_, text)| text)
                    .or_else(|| {
                        object
                            .components
                            .get("game.audio-source.v1")
                            .and_then(|source| source["import_settings"].as_str())
                    })
                    .unwrap_or("default");
                self.text(
                    &mut batch,
                    &format!("Settings: {settings}"),
                    Vec2::new(rx + 10., 298.),
                    right - 16.,
                    size,
                )?;
                self.regions
                    .push(([rx + 4., 280., right - 8., 24.], Action::Field(7)));
            }
            if inspector == InspectorMode::Physics {
                self.text(
                    &mut batch,
                    "Physics: scale 1, rot 0",
                    Vec2::new(rx + 8.0, 310.0),
                    right - 12.0,
                    size,
                )?;
            }
            for (row, label, action) in [
                (
                    3,
                    if matches!(
                        inspector,
                        InspectorMode::Audio | InspectorMode::ImportSettings
                    ) {
                        "Open/reload import settings"
                    } else if object.components.contains_key("game.angular-motion.v1") {
                        "Remove angular motion"
                    } else {
                        "Add angular motion"
                    },
                    if matches!(
                        inspector,
                        InspectorMode::Audio | InspectorMode::ImportSettings
                    ) {
                        Action::AudioSettingsLoad
                    } else {
                        Action::Motion
                    },
                ),
                (
                    4,
                    match inspector {
                        InspectorMode::Behavior | InspectorMode::ImportSettings => "Audio fields",
                        InspectorMode::Audio => "Mixer fields",
                        InspectorMode::Mixer => "Component fields",
                        InspectorMode::Components(_) => "Collections",
                        InspectorMode::Collections(_, _) => "Transform fields",
                        _ => "Behavior fields",
                    },
                    Action::Behavior,
                ),
                (
                    0,
                    if inspector == InspectorMode::ImportSettings {
                        "Save import settings"
                    } else if inspector == InspectorMode::Mixer {
                        if object.components.contains_key("game.audio-bus.v1") {
                            "Remove audio bus"
                        } else {
                            "Add audio bus"
                        }
                    } else if audio_mode {
                        if object.components.contains_key("game.audio-source.v1") {
                            "Remove audio source"
                        } else {
                            "Add audio source"
                        }
                    } else if object.components.contains_key("game.character.v1") {
                        "Remove character (C)"
                    } else {
                        "Add character (C)"
                    },
                    if inspector == InspectorMode::ImportSettings {
                        Action::AudioSettingsSave
                    } else if inspector == InspectorMode::Mixer {
                        Action::AudioBus
                    } else if audio_mode {
                        Action::AudioSource
                    } else {
                        Action::Character
                    },
                ),
                (
                    1,
                    if audio_mode {
                        if object.components.contains_key("game.audio-listener.v1") {
                            "Remove audio listener"
                        } else {
                            "Add audio listener"
                        }
                    } else if object.components.contains_key("game.box.v1") {
                        "Remove collider (B)"
                    } else {
                        "Add collider (B)"
                    },
                    if audio_mode {
                        Action::AudioListener
                    } else {
                        Action::Collider
                    },
                ),
                (
                    2,
                    if inspector == InspectorMode::Physics {
                        "Transform fields (I)"
                    } else {
                        "Physics fields (I)"
                    },
                    Action::Physics,
                ),
            ] {
                let y = 354.0
                    + row as f32 * 24.0
                    + if matches!(inspector, InspectorMode::Collections(_, _)) {
                        30.0
                    } else {
                        0.0
                    };
                self.text(
                    &mut batch,
                    label,
                    Vec2::new(rx + 8.0, y + 17.0),
                    right - 12.0,
                    size,
                )?;
                self.regions
                    .push(([rx + 4.0, y, right - 8.0, 23.0], action));
            }
            self.text(
                &mut batch,
                if playing {
                    "Authoring values"
                } else if field.is_none() {
                    "Tab / Shift+Tab focus"
                } else {
                    "Type, Enter applies"
                },
                Vec2::new(
                    rx + 8.0,
                    if matches!(inspector, InspectorMode::Collections(_, _)) {
                        521.0
                    } else {
                        491.0
                    },
                ),
                right - 12.0,
                size,
            )?;
            self.text(
                &mut batch,
                if playing {
                    "Stop to edit"
                } else if field.is_none() {
                    "Enter / Space activate"
                } else {
                    "Escape cancels"
                },
                Vec2::new(
                    rx + 8.0,
                    if matches!(inspector, InspectorMode::Collections(_, _)) {
                        544.0
                    } else {
                        514.0
                    },
                ),
                right - 12.0,
                size,
            )?;
        }
        let actions: Vec<_> = self.regions.iter().map(|(_, action)| *action).collect();
        self.focus.reconcile_with_registry(
            &actions,
            document,
            selected,
            inspector,
            &self.registry,
        )?;
        let mut hit_regions = Vec::with_capacity(self.regions.len());
        for &(rect, action) in &self.regions {
            let region = voxy_ui::HitRegion {
                id: self.focus.id_for(action).ok_or("missing focus target")?,
                origin: [rect[0], rect[1]],
                size: [rect[2], rect[3]],
                enabled: !playing || action == Action::Play,
            };
            if let Some(region) = region.clipped([0.0; 2], size.to_array())? {
                hit_regions.push(region);
            }
        }
        self.pointer.set_regions(&hit_regions)?;
        for &(r, action) in &self.regions {
            if self.focus.is_focused(action) {
                let color = [0.5, 0.8, 1.0, 1.0];
                solid(&mut batch, [r[0], r[1], r[2], 2.0], color)?;
                solid(&mut batch, [r[0], r[1] + r[3] - 2.0, r[2], 2.0], color)?;
                solid(&mut batch, [r[0], r[1], 2.0, r[3]], color)?;
                solid(&mut batch, [r[0] + r[2] - 2.0, r[1], 2.0, r[3]], color)?;
            }
        }
        batch.mesh().map_err(Into::into)
    }
    fn text(
        &self,
        batch: &mut SpriteBatch,
        text: &str,
        mut pen: Vec2,
        max_width: f32,
        size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let end = pen.x + max_width;
        for character in text.chars().take(128) {
            let (glyph, region) = &self.glyphs[&if character.is_ascii() && !character.is_control() {
                character
            } else {
                '?'
            }];
            if pen.x + glyph.advance > end {
                break;
            }
            if let Some(region) = region {
                let mut sprite = Sprite::from_logical_rect(
                    pen + Vec2::new(
                        glyph.bearing[0] as f32,
                        -glyph.bearing[1] as f32 - glyph.height as f32,
                    ),
                    Vec2::new(glyph.width as f32, glyph.height as f32),
                    size,
                    [0.85, 0.9, 0.95, 1.0],
                )?;
                sprite.uv_min = Vec2::new(
                    region.origin[0] as f32 / 512.0,
                    region.origin[1] as f32 / 128.0,
                );
                sprite.uv_max = Vec2::new(
                    (region.origin[0] + region.size[0]) as f32 / 512.0,
                    (region.origin[1] + region.size[1]) as f32 / 128.0,
                );
                batch.push(sprite)?;
            }
            pen.x += glyph.advance;
        }
        Ok(())
    }
}

fn visit(
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
