//! CPU overlay geometry; device upload/publication stays with the graphics owner.
#![allow(clippy::cast_precision_loss)]
use glam::Vec2;
use std::{collections::HashMap, sync::Arc};
use voxy_gameplay::{PreparedUiText, SceneUiSnapshot};
use voxy_render::{SceneMesh, Sprite, SpriteBatch};
use voxy_scene::NodeId;
use voxy_text::TextRun;

#[derive(Debug)]
pub(super) struct UiDrawMesh {
    pub(super) owner: NodeId,
    pub(super) mesh: SceneMesh,
    /// None selects the graphics owner's existing white texture.
    pub(super) atlas: Option<Arc<TextRun>>,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FocusRing {
    pub(super) owner: NodeId,
    pub(super) viewport: [f32; 2],
    pub(super) clip: [f32; 4],
}
impl FocusRing {
    pub(super) fn mesh(&self) -> Result<UiDrawMesh, String> {
        let [x, y, width, height] = self.clip;
        if self.clip.iter().any(|value| !value.is_finite()) || width <= 0.0 || height <= 0.0 {
            return Err("invalid UI focus clip".into());
        }
        let thickness = 2.0_f32.min(width * 0.5).min(height * 0.5);
        let mut batch = SpriteBatch::new(4);
        for (origin, size) in [
            ([x, y], [width, thickness]),
            ([x, y + height - thickness], [width, thickness]),
            ([x, y + thickness], [thickness, height - 2.0 * thickness]),
            (
                [x + width - thickness, y + thickness],
                [thickness, height - 2.0 * thickness],
            ),
        ] {
            if size[0] <= 0.0 || size[1] <= 0.0 {
                continue;
            }
            batch
                .push(
                    Sprite::from_logical_rect(
                        Vec2::from_array(origin),
                        Vec2::from_array(size),
                        Vec2::from_array(self.viewport),
                        [1.0, 0.85, 0.15, 1.0],
                    )
                    .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(UiDrawMesh {
            owner: self.owner,
            mesh: batch.mesh().map_err(|error| error.to_string())?,
            atlas: None,
        })
    }
}
pub(super) fn build(
    snapshot: &SceneUiSnapshot,
    text: &[PreparedUiText],
) -> Result<Vec<UiDrawMesh>, String> {
    if snapshot.elements.len() > 128 || text.len() > 128 {
        return Err("UI mesh element capacity".into());
    }
    let labels = validate_labels(snapshot, text)?;
    let viewport = Vec2::from_array(snapshot.viewport);
    let mut draws = Vec::new();
    let mut glyphs = 0_usize;
    for element in &snapshot.elements {
        if !element.descriptor.valid() {
            return Err("invalid UI draw descriptor".into());
        }
        let Some(clip) = element.clip else {
            continue;
        };
        if element.descriptor.color[3] > 0.0 {
            let mut batch = SpriteBatch::new(1);
            batch
                .push(
                    Sprite::from_logical_rect(
                        Vec2::new(clip[0], clip[1]),
                        Vec2::new(clip[2], clip[3]),
                        viewport,
                        element.descriptor.color,
                    )
                    .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            draws.push(UiDrawMesh {
                owner: element.owner,
                mesh: batch.mesh().map_err(|error| error.to_string())?,
                atlas: None,
            });
        }
        if element
            .descriptor
            .text
            .as_ref()
            .is_some_and(|text| !text.content.is_empty())
            && !labels.contains_key(&element.owner)
        {
            return Err("missing prepared UI text owner".into());
        }
        if let Some(label) = labels.get(&element.owner) {
            let dimensions = label.run.atlas().dimensions();
            if dimensions.iter().any(|value| *value == 0 || *value > 16384) {
                return Err("invalid UI atlas dimensions".into());
            }
            let mut batch = SpriteBatch::new(8192);
            let mut visible = 0_usize;
            for glyph in label.run.glyphs() {
                let origin =
                    Vec2::new(element.rect[0], element.rect[1]) + Vec2::from_array(glyph.origin);
                let size = Vec2::new(glyph.region.size[0] as f32, glyph.region.size[1] as f32);
                let start = origin.max(Vec2::new(clip[0], clip[1]));
                let end = (origin + size).min(Vec2::new(clip[0] + clip[2], clip[1] + clip[3]));
                if (end - start).min_element() <= 0.0 {
                    continue;
                }
                let mut sprite = Sprite::from_logical_rect(
                    origin,
                    size,
                    viewport,
                    element
                        .descriptor
                        .text
                        .as_ref()
                        .ok_or("missing UI text")?
                        .color,
                )
                .map_err(|error| error.to_string())?;
                let atlas_size = Vec2::new(dimensions[0] as f32, dimensions[1] as f32);
                sprite.uv_min =
                    Vec2::new(glyph.region.origin[0] as f32, glyph.region.origin[1] as f32)
                        / atlas_size;
                sprite.uv_max = sprite.uv_min + size / atlas_size;
                batch
                    .push(
                        sprite
                            .cropped((start - origin) / size, (end - origin) / size)
                            .map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
                visible += 1;
                glyphs += 1;
                if glyphs > 8192 {
                    return Err("UI mesh glyph capacity".into());
                }
            }
            if visible > 0 {
                draws.push(UiDrawMesh {
                    owner: element.owner,
                    mesh: batch.mesh().map_err(|error| error.to_string())?,
                    atlas: Some(Arc::clone(&label.run)),
                });
            }
        }
    }
    Ok(draws)
}

fn validate_labels<'a>(
    snapshot: &SceneUiSnapshot,
    text: &'a [PreparedUiText],
) -> Result<HashMap<NodeId, &'a PreparedUiText>, String> {
    let mut labels = HashMap::new();
    for label in text {
        if labels.insert(label.owner, label).is_some() {
            return Err("duplicate UI text owner".into());
        }
        let element = snapshot
            .elements
            .iter()
            .find(|element| element.owner == label.owner)
            .ok_or("stale UI text owner")?;
        let source = element
            .descriptor
            .text
            .as_ref()
            .ok_or("UI text component removed")?;
        if source.font != label.source.font
            || source.content != label.source.content
            || source.size.to_bits() != label.source.size.to_bits()
        {
            return Err("UI text recipe changed before mesh publication".into());
        }
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_gameplay::{UiElement, UiText};
    use voxy_scene::{SceneGraph, Transform};
    #[test]
    #[allow(clippy::too_many_lines)]
    fn glyph_geometry_clips_uvs_and_rejects_stale_owner_publications() {
        let bytes = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("UI mesh integration requires a local TrueType font");
        let font = voxy_text::TextFont::parse(
            &bytes,
            voxy_text::FontLimits {
                max_font_bytes: 4 * 1024 * 1024,
                max_glyph_pixels: 16384,
                max_size: 128.0,
            },
        )
        .unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                UiElement {
                    origin: [0.25, 0.2],
                    size: [0.05, 0.6],
                    color: [0.0; 4],
                    layer: 0,
                    enabled: true,
                    action: None,
                    text: Some(UiText {
                        font: "font".into(),
                        content: "W".into(),
                        size: 40.0,
                        color: [1.0; 4],
                    }),
                },
            )
            .unwrap();
        let snapshot = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 1).unwrap();
        let labels = voxy_gameplay::prepare_ui_text(&snapshot, |_| Some(&font)).unwrap();
        assert!(labels[0].run.glyphs()[0].region.size[0] > 5);
        let draws = build(&snapshot, &labels).unwrap();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].owner, owner);
        assert_eq!(draws[0].mesh.indices().len(), 6);
        for vertex in draws[0].mesh.vertices() {
            assert!(vertex.position[0] >= -0.50001 && vertex.position[0] <= -0.39999);
            assert!((0.0..=1.0).contains(&vertex.uv[0]) && (0.0..=1.0).contains(&vertex.uv[1]));
        }
        let min = draws[0]
            .mesh
            .vertices()
            .iter()
            .map(|vertex| vertex.uv[0])
            .fold(f32::INFINITY, f32::min);
        let max = draws[0]
            .mesh
            .vertices()
            .iter()
            .map(|vertex| vertex.uv[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min <= 5.0 / 512.0 + 1e-6);
        let descriptor = scene.component_mut::<UiElement>(owner).unwrap().unwrap();
        descriptor.origin[0] = 0.5;
        descriptor.text.as_mut().unwrap().color = [0.0, 1.0, 0.0, 1.0];
        let resized = voxy_gameplay::extract_scene_ui(&scene, [200.0, 100.0], 1).unwrap();
        let relayout = build(&resized, &labels).unwrap();
        assert!(Arc::ptr_eq(
            relayout[0].atlas.as_ref().unwrap(),
            &labels[0].run
        ));
        assert!(
            relayout[0]
                .mesh
                .vertices()
                .iter()
                .all(|vertex| vertex.color[1] > 0.99 && vertex.color[0].abs() < 1e-6)
        );
        scene
            .component_mut::<UiElement>(owner)
            .unwrap()
            .unwrap()
            .text
            .as_mut()
            .unwrap()
            .content = "H".into();
        let changed = voxy_gameplay::extract_scene_ui(&scene, [200.0, 100.0], 1).unwrap();
        assert!(
            build(&changed, &labels)
                .unwrap_err()
                .contains("recipe changed")
        );
        scene.remove_subtree(owner).unwrap();
        let next = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                next,
                UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: None,
                    text: None,
                },
            )
            .unwrap();
        let replacement = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 1).unwrap();
        assert!(
            build(&replacement, &labels)
                .unwrap_err()
                .contains("stale UI text owner")
        );
        assert_eq!(draws[0].owner, owner);
    }
    #[test]
    fn backgrounds_follow_scene_painter_order() {
        let mut scene = SceneGraph::new(2);
        let front = scene.spawn(None, Transform::default()).unwrap();
        let back = scene.spawn(None, Transform::default()).unwrap();
        for (owner, layer) in [(front, 2), (back, 1)] {
            scene
                .insert_component(
                    owner,
                    UiElement {
                        origin: [0.0; 2],
                        size: [1.0; 2],
                        color: [1.0; 4],
                        layer,
                        enabled: false,
                        action: None,
                        text: None,
                    },
                )
                .unwrap();
        }
        let snapshot = voxy_gameplay::extract_scene_ui(&scene, [100.0; 2], 2).unwrap();
        let draws = build(&snapshot, &[]).unwrap();
        assert_eq!(
            draws.iter().map(|draw| draw.owner).collect::<Vec<_>>(),
            vec![back, front]
        );
        assert!(draws.iter().all(|draw| draw.atlas.is_none()));
    }
}

#[cfg(test)]
mod gpu_tests;

#[cfg(test)]
mod focus_tests {
    use super::*;
    use voxy_scene::{SceneGraph, Transform};
    #[test]
    fn ring_is_clipped_inward_and_handles_tiny_controls() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        for clip in [[10.0, 20.0, 50.0, 40.0], [10.0, 20.0, 1.0, 1.0]] {
            let ring = FocusRing {
                owner,
                viewport: [100.0; 2],
                clip,
            };
            let draw = ring.mesh().unwrap();
            assert!(draw.atlas.is_none());
            assert_eq!(draw.owner, owner);
            for vertex in draw.mesh.vertices() {
                let x = (vertex.position[0] + 1.0) * 50.0;
                let y = (1.0 - vertex.position[1]) * 50.0;
                assert!(x >= clip[0] - 0.001 && x <= clip[0] + clip[2] + 0.001);
                assert!(y >= clip[1] - 0.001 && y <= clip[1] + clip[3] + 0.001);
            }
        }
    }
}
