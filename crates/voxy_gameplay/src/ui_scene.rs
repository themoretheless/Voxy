//! Saved screen-space UI and owned layout. Scene transforms remain 3D-owned;
//! normalized UI rectangles compose through the nearest UI ancestor instead.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use voxy_scene::{NodeId, SceneGraph};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiText {
    pub font: String,
    pub content: String,
    pub size: f32,
    pub color: [f32; 4],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiElement {
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub layer: i16,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<UiText>,
}
fn color_valid(color: [f32; 4]) -> bool {
    color
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
}
impl UiElement {
    #[must_use]
    pub fn valid(&self) -> bool {
        self.origin
            .iter()
            .all(|x| x.is_finite() && (-2.0..=2.0).contains(x))
            && self
                .size
                .iter()
                .all(|x| x.is_finite() && *x > 0.0 && *x <= 2.0)
            && color_valid(self.color)
            && self.action.as_ref().is_none_or(|action| {
                !action.is_empty() && action.len() <= 128 && !action.chars().any(char::is_control)
            })
            && self.text.as_ref().is_none_or(|text| {
                !text.font.is_empty()
                    && text.font.len() <= 1024
                    && !text.font.chars().any(char::is_control)
                    && text.content.len() <= 4096
                    && !text.content.chars().any(char::is_control)
                    && text.size.is_finite()
                    && (1.0..=128.0).contains(&text.size)
                    && color_valid(text.color)
            })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct UiElementSnapshot {
    pub owner: NodeId,
    pub descriptor: UiElement,
    /// Logical viewport pixels, before clipping, used for text placement.
    pub rect: [f32; 4],
    /// Visible intersection with every UI ancestor and the viewport.
    pub clip: Option<[f32; 4]>,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SceneUiSnapshot {
    pub viewport: [f32; 2],
    /// Painter order: layer first, then stable scene slot order.
    pub elements: Vec<UiElementSnapshot>,
}
fn intersection(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    let width = (a[0] + a[2]).min(b[0] + b[2]) - x;
    let height = (a[1] + a[3]).min(b[1] + b[3]) - y;
    (width > 0.0 && height > 0.0).then_some([x, y, width, height])
}
/// Produces bounded owned data, validating inactive descriptors as well as active
/// ones. Disabled UI ancestry propagates to controls; inactive scene ancestry
/// excludes drawing and input. UI geometry is independent of 3D transform poses.
/// # Errors
/// Rejects malformed descriptors, viewport/count/text budgets, invalid handles
/// and overflow during nested layout before returning a publication.
pub fn extract_scene_ui(
    scene: &SceneGraph,
    viewport: [f32; 2],
    capacity: usize,
) -> Result<SceneUiSnapshot, String> {
    if viewport
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.0 || *x > 16384.0)
    {
        return Err("invalid UI viewport".into());
    }
    let mut descriptors = HashMap::new();
    let mut text_bytes = 0_usize;
    for (owner, descriptor) in scene.components::<UiElement>() {
        if descriptors.len() >= capacity {
            return Err("scene UI element capacity".into());
        }
        if !descriptor.valid() {
            return Err("invalid scene UI descriptor".into());
        }
        text_bytes = text_bytes
            .checked_add(
                descriptor
                    .text
                    .as_ref()
                    .map_or(0, |text| text.content.len()),
            )
            .ok_or("scene UI text capacity")?;
        if text_bytes > 65536 {
            return Err("scene UI text capacity".into());
        }
        descriptors.insert(owner, descriptor);
    }
    let viewport_rect = [0.0, 0.0, viewport[0], viewport[1]];
    let mut elements = Vec::with_capacity(descriptors.len());
    for (owner, _, _) in scene.nodes() {
        let Some(descriptor) = descriptors.get(&owner) else {
            continue;
        };
        if !scene
            .active_in_hierarchy(owner)
            .map_err(|error| error.to_string())?
        {
            continue;
        }
        let mut chain = Vec::new();
        let mut current = Some(owner);
        while let Some(node) = current {
            if let Some(element) = descriptors.get(&node) {
                chain.push(*element);
            }
            current = scene.parent(node).map_err(|error| error.to_string())?;
        }
        let mut rect = viewport_rect;
        let mut clip = Some(viewport_rect);
        let mut enabled = true;
        for element in chain.into_iter().rev() {
            rect = [
                rect[0] + element.origin[0] * rect[2],
                rect[1] + element.origin[1] * rect[3],
                element.size[0] * rect[2],
                element.size[1] * rect[3],
            ];
            if rect.iter().any(|x| !x.is_finite())
                || rect[2] <= 0.0
                || rect[3] <= 0.0
                || !(rect[0] + rect[2]).is_finite()
                || !(rect[1] + rect[3]).is_finite()
            {
                return Err("scene UI layout overflow".into());
            }
            clip = clip.and_then(|parent| intersection(parent, rect));
            enabled &= element.enabled;
        }
        elements.push(UiElementSnapshot {
            owner,
            descriptor: (*descriptor).clone(),
            rect,
            clip,
            enabled,
        });
    }
    elements.sort_by_key(|element| element.descriptor.layer);
    Ok(SceneUiSnapshot { viewport, elements })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use voxy_scene::{ComponentRegistry, ObjectId, SceneDocument, SceneObject, Transform};
    pub(crate) fn element(action: Option<&str>) -> UiElement {
        UiElement {
            origin: [0.0; 2],
            size: [1.0; 2],
            color: [0.2, 0.3, 0.4, 1.0],
            layer: 0,
            enabled: true,
            action: action.map(str::to_owned),
            text: None,
        }
    }
    #[test]
    #[allow(clippy::float_cmp)]
    fn nested_layout_uses_ui_ancestry_clips_and_inherited_activity() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let bridge = scene
            .spawn(
                Some(root),
                Transform {
                    translation: glam::Vec3::splat(100.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        let child = scene.spawn(Some(bridge), Transform::default()).unwrap();
        let mut parent = element(None);
        parent.origin = [0.1, 0.2];
        parent.size = [0.5, 0.5];
        parent.enabled = false;
        scene.insert_component(root, parent).unwrap();
        let mut button = element(Some("jump"));
        button.origin = [0.75, 0.0];
        button.size = [0.5, 1.0];
        button.layer = 1;
        scene.insert_component(child, button).unwrap();
        let snapshot = extract_scene_ui(&scene, [1000.0, 800.0], 2).unwrap();
        assert_eq!(snapshot.elements[1].owner, child);
        assert_eq!(snapshot.elements[1].rect, [475.0, 160.0, 250.0, 400.0]);
        assert_eq!(
            snapshot.elements[1].clip,
            Some([475.0, 160.0, 125.0, 400.0])
        );
        assert!(!snapshot.elements[1].enabled);
        scene.set_active(bridge, false).unwrap();
        assert_eq!(
            extract_scene_ui(&scene, [1000.0, 800.0], 2)
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(extract_scene_ui(&scene, [1000.0, 800.0], 1).is_err());
    }
    #[test]
    fn strict_registered_unicode_descriptors_round_trip_and_validate_bounds() {
        let mut registry = ComponentRegistry::default();
        crate::register_components(&mut registry).unwrap();
        let mut ui = element(Some("jump"));
        ui.text = Some(UiText {
            font: "fonts/interface.ttf".into(),
            content: "Играть — مرحبا".into(),
            size: 20.0,
            color: [1.0; 4],
        });
        let doc = SceneDocument {
            version: 1,
            objects: vec![SceneObject {
                id: ObjectId("menu".into()),
                parent: None,
                name: "Menu".into(),
                active: true,
                translation: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
                components: std::collections::BTreeMap::from([(
                    "game.ui-element.v1".into(),
                    serde_json::to_value(&ui).unwrap(),
                )]),
            }],
        };
        let loaded = SceneDocument::from_json(&doc.to_json().unwrap())
            .unwrap()
            .load(&registry, 1)
            .unwrap();
        assert_eq!(loaded.capture(&registry).unwrap(), doc);
        crate::validate_game_descriptors(&loaded.graph, 1).unwrap();
        let mut invalid = ui.clone();
        invalid.size[0] = 0.0;
        assert!(!invalid.valid());
        invalid = ui.clone();
        invalid.action = Some("a".repeat(129));
        assert!(!invalid.valid());
        invalid = ui.clone();
        invalid.text.as_mut().unwrap().content = "a".repeat(4097);
        assert!(!invalid.valid());
        invalid = ui;
        invalid.color[0] = f32::NAN;
        assert!(!invalid.valid());
        assert!(extract_scene_ui(&loaded.graph, [0.0, 800.0], 1).is_err());
        let mut value = serde_json::to_value(element(None)).unwrap();
        value["unknown"] = true.into();
        assert!(serde_json::from_value::<UiElement>(value).is_err());
    }
    #[test]
    fn aggregate_text_budget_counts_inactive_elements_before_publication() {
        let mut scene = SceneGraph::new(17);
        for _ in 0..17 {
            let owner = scene.spawn(None, Transform::default()).unwrap();
            let mut ui = element(None);
            ui.text = Some(UiText {
                font: "font.ttf".into(),
                content: "a".repeat(4096),
                size: 20.0,
                color: [1.0; 4],
            });
            scene.insert_component(owner, ui).unwrap();
            scene.set_active(owner, false).unwrap();
        }
        assert!(
            extract_scene_ui(&scene, [1000.0, 800.0], 17)
                .unwrap_err()
                .contains("text capacity")
        );
    }
}
