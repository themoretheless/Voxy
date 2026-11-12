//! Small native editor panels rendered through the existing scene renderer.
// Glyph metrics are bounded to a 512x128 atlas, UI rows to 128 scene objects.

mod hierarchy;
mod inspector_components;
mod inspector_modes;

use crate::InspectorMode;
use glam::Vec2;
use std::collections::BTreeMap;
use voxy_render::{SceneMesh, Sprite, SpriteBatch};
use voxy_scene::SceneDocument;
use voxy_text::{FontLimits, GlyphAtlas, GlyphRegion, RasterFont};

pub(crate) fn side_widths(size: Vec2) -> (f32, f32) {
    (150.0_f32.min(size.x * 0.25), 180.0_f32.min(size.x * 0.3))
}

/// Visible physical-pixel area inside one render view after side-panel overlap.
pub(crate) fn safe_view_rect(viewport: [u32; 2], region: [u32; 4], scale: f32) -> Option<[f32; 4]> {
    if !scale.is_finite()
        || scale <= 0.
        || viewport.contains(&0)
        || region[2..].contains(&0)
        || region[0].checked_add(region[2])? > viewport[0]
        || region[1].checked_add(region[3])? > viewport[1]
    {
        return None;
    }
    let logical = Vec2::new(viewport[0] as f32, viewport[1] as f32) / scale;
    let (left, right) = side_widths(logical);
    let x0 = (left * scale - region[0] as f32).max(0.);
    let x1 = ((logical.x - right) * scale - region[0] as f32).min(region[2] as f32);
    (x1 > x0).then_some([x0, 0., x1 - x0, region[3] as f32])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Select(usize),
    SearchHierarchy,
    FollowCamera([u8; 32]),
    FollowTarget([u8; 32]),
    FollowTargetChoice([u8; 32]),
    Joint([u8; 32]),
    JointTarget([u8; 32]),
    JointKind([u8; 32]),
    NpcDestination([u8; 32]),
    Field(usize),
    ResetField(usize),
    PreviewSeek,
    MorphDefaults([u8; 32]),
    MorphResume([u8; 32]),
    PreviewPhase(f64),
    MarkerAdd([u8; 32]),
    MarkerPreview(usize, [u8; 32]),
    MarkerDelete(usize, [u8; 32]),
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
    RetryModel,
    RelinkModel,
    RelinkCancel,
    Parent,
    Active,
    Play,
    Save,
    BuildGame,
    RunBuiltGame,
    Load,
    CancelLoad,
    RevertPrefab,
    PlacePrefab,
    CreatePrefab,
    PrefabChoice,
    Character,
    Collider,
    Trigger,
    DynamicBody,
    KinematicBody,
    Physics,
    Behavior,
    Motion,
    Animation,
    Retarget,
    RetargetApply,
    RetargetCancel,
    RetargetRemove,
    RetargetPair(bool),
    RetargetDeletePair(usize),
    RetargetBones(usize),
    RetargetBone(usize),
    RetargetBonePage(bool),
    RetargetBoneClose,
    AudioSource,
    AudioListener,
    AudioBus,
    AudioSettingsLoad,
    AudioSettingsSave,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResourceNotice {
    pub lines: [String; 3],
    pub retry: bool,
    pub relink: bool,
    pub editing: bool,
}

pub(crate) struct Panels {
    registry: std::sync::Arc<voxy_scene::ComponentRegistry>,
    pub collection_resets: std::collections::BTreeSet<([u8; 32], [u8; 32])>,
    pub collection_order_resets: std::collections::BTreeSet<[u8; 32]>,
    pub collection_deleted_resets: std::collections::BTreeSet<[u8; 32]>,
    pub collection_deleted_items: crate::component_collections::DeletedItems,
    pub deleted_page: usize,
    pub preview_phase: Option<f64>,
    pub build_label: Option<&'static str>,
    pub package_run_label: Option<&'static str>,
    pub load_status: Option<crate::app::restore_barrier::load::LoadStatus>,
    pub resource_notice: Option<ResourceNotice>,
    pub retarget_draft: bool,
    pub follow_pick: Option<[u8; 32]>,
    pub pick_title: &'static str,
    pub hierarchy_search: Option<String>,
    pub bone_picker: Option<(Vec<String>, usize)>,
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
            preview_phase: None,
            build_label: None,
            package_run_label: None,
            load_status: None,
            resource_notice: None,
            presented: false,
            retarget_draft: false,
            bone_picker: None,
            follow_pick: None,
            pick_title: crate::camera_authoring::PickPurpose::FollowCamera.title(),
            hierarchy_search: None,
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

    // Texture white texel is the final unused pixel of the alpha atlas.
    pub(super) fn solid(
        batch: &mut SpriteBatch,
        r: [f32; 4],
        size: Vec2,
        color: [f32; 4],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut sprite =
            Sprite::from_logical_rect(Vec2::new(r[0], r[1]), Vec2::new(r[2], r[3]), size, color)?;
        sprite.uv_min = Vec2::new(511.0 / 512.0, 127.0 / 128.0);
        sprite.uv_max = Vec2::ONE;
        batch.push(sprite)?;
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "editor panel layout consumes current authoring selection, scroll, field draft, and mode state"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
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
        let (left, right) = side_widths(size);
        let rx = size.x - right;
        Self::solid(
            &mut batch,
            [0.0, 0.0, left, size.y],
            size,
            [0.06, 0.075, 0.1, 1.0],
        )?;
        Self::solid(
            &mut batch,
            [rx, 0.0, right, size.y],
            size,
            [0.06, 0.075, 0.1, 1.0],
        )?;
        let hierarchy_title = match (&self.hierarchy_search, self.follow_pick) {
            (_, Some(_)) => self.pick_title.into(),
            (Some(query), None) => format!("Search: {query}"),
            (None, None) if !playing => "Find objects…".into(),
            (None, None) => "SCENE".into(),
        };
        self.text(
            &mut batch,
            &hierarchy_title,
            Vec2::new(10.0, 23.0),
            left - 16.0,
            size,
        )?;
        if !playing && self.follow_pick.is_none() {
            self.regions
                .push(([4.0, 4.0, left - 8.0, 29.0], Action::SearchHierarchy));
        }
        self.text(
            &mut batch,
            "INSPECTOR",
            Vec2::new(rx + 10.0, 23.0),
            right - 16.0,
            size,
        )?;
        self.build_resource_notice(&mut batch, left, rx, size)?;
        self.build_hierarchy_and_toolbar(
            &mut batch,
            document,
            selected,
            left,
            size,
            scroll,
            parenting,
            playing,
            inspector,
            prefab_label,
        )?;
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
            if let Some((choices, page)) = self.bone_picker.clone() {
                for (row, (index, name)) in choices
                    .iter()
                    .enumerate()
                    .skip(page * 6)
                    .take(6)
                    .enumerate()
                {
                    let y = 116. + row as f32 * 26.;
                    self.text(
                        &mut batch,
                        name,
                        Vec2::new(rx + 10., y + 18.),
                        right - 20.,
                        size,
                    )?;
                    self.regions
                        .push(([rx + 4., y, right - 8., 25.], Action::RetargetBone(index)));
                }
                for (column, label, action) in [
                    (0, "Previous", Action::RetargetBonePage(false)),
                    (1, "Next", Action::RetargetBonePage(true)),
                    (2, "Close", Action::RetargetBoneClose),
                ] {
                    let width = (right - 8.) / 3.;
                    let x = rx + 4. + column as f32 * width;
                    self.text(&mut batch, label, Vec2::new(x + 4., 302.), width - 8., size)?;
                    self.regions.push(([x, 284., width, 25.], action));
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
            } else if let InspectorMode::Components(page) = inspector {
                self.build_components_page(
                    &mut batch, document, object, page, playing, field, rx, right, size,
                )?;
            } else if let InspectorMode::Collections(collection, page) = inspector {
                self.build_collections_page(
                    &mut batch, object, collection, page, playing, rx, right, size,
                )?;
            } else {
                if inspector == InspectorMode::Material {
                    self.text(
                        &mut batch,
                        texture_label,
                        Vec2::new(rx + 10., 286.),
                        right - 16.,
                        size,
                    )?;
                }
                let values = inspector_modes::collect_mode_values(object, inspector, import_config);
                let row = if inspector == InspectorMode::Physics && values.len() > 11 {
                    19.0
                } else if inspector == InspectorMode::Physics && values.len() > 8 {
                    20.0
                } else {
                    26.0
                };
                for (index, (label, value)) in values.into_iter().enumerate() {
                    let y = 116.0 + index as f32 * row;
                    let overridden = !playing && self.overridden_fields.contains(&index);
                    let reset_text_width: f32 = "Reset"
                        .chars()
                        .map(|character| self.glyphs[&character].0.advance)
                        .sum();
                    let reset_width = reset_text_width.ceil() + 12.0;
                    let field_width = right - if overridden { reset_width + 12.0 } else { 8.0 };
                    if field.is_some_and(|(id, _)| id == index) {
                        Self::solid(
                            &mut batch,
                            [rx + 4.0, y, field_width, row - 1.0],
                            size,
                            [0.2, 0.25, 0.32, 1.0],
                        )?;
                    }
                    let value = field.filter(|(id, _)| *id == index).map_or_else(
                        || {
                            if inspector == InspectorMode::ImportSettings
                                || (inspector == InspectorMode::Physics
                                    && crate::prefab_fields::physics_layer_field(object, index)
                                        .is_some())
                            {
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
                        Vec2::new(rx + 10.0, y + row - 8.0),
                        field_width - 8.0,
                        size,
                    )?;
                    self.regions
                        .push(([rx + 4.0, y, field_width, row - 1.0], Action::Field(index)));
                    if overridden {
                        let rect = [rx + right - reset_width - 4.0, y, reset_width, row - 1.0];
                        Self::solid(&mut batch, rect, size, [0.18, 0.24, 0.3, 1.0])?;
                        self.text(
                            &mut batch,
                            "Reset",
                            Vec2::new(rect[0] + 6.0, y + row - 8.0),
                            reset_width - 12.0,
                            size,
                        )?;
                        self.regions.push((rect, Action::ResetField(index)));
                    }
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
            self.build_inspector_footer(
                &mut batch, object, inspector, playing, field, rx, right, size,
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
                Self::solid(&mut batch, [r[0], r[1], r[2], 2.0], size, color)?;
                Self::solid(
                    &mut batch,
                    [r[0], r[1] + r[3] - 2.0, r[2], 2.0],
                    size,
                    color,
                )?;
                Self::solid(&mut batch, [r[0], r[1], 2.0, r[3]], size, color)?;
                Self::solid(
                    &mut batch,
                    [r[0] + r[2] - 2.0, r[1], 2.0, r[3]],
                    size,
                    color,
                )?;
            }
        }
        batch.mesh().map_err(Into::into)
    }

    pub(super) fn text(
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

#[cfg(test)]
mod gpu_tests;

#[cfg(test)]
mod safe_view_tests {
    use super::*;
    #[test]
    fn panel_safe_areas_follow_dpi_and_odd_split_regions() {
        assert_eq!(
            safe_view_rect([1001, 700], [0, 0, 500, 700], 1.),
            Some([150., 0., 350., 700.])
        );
        assert_eq!(
            safe_view_rect([1001, 700], [500, 0, 501, 700], 1.),
            Some([0., 0., 321., 700.])
        );
        assert_eq!(
            safe_view_rect([2002, 1400], [0, 0, 1001, 1400], 2.),
            Some([300., 0., 701., 1400.])
        );
        assert_eq!(
            safe_view_rect([2002, 1400], [1001, 0, 1001, 1400], 2.),
            Some([0., 0., 641., 1400.])
        );
        assert!(safe_view_rect([1001, 700], [0, 0, 100, 700], 1.).is_none());
        assert!(safe_view_rect([1001, 700], [0, 0, 500, 700], 0.).is_none());
    }
}
