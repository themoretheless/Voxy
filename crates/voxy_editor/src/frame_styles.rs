//! Owned presentation inputs; no scene borrowing survives preparation.
use super::{DirectionalLight, EditorCamera, ModelInstance, ModelPart, SceneMaterial};
use std::collections::{HashMap, HashSet};
use voxy_scene::{NodeId, SceneSystemAccess};
#[derive(Debug)]
pub(super) struct FrameStyles {
    pub camera: EditorCamera,
    pub light: Option<DirectionalLight>,
    pub materials: HashMap<NodeId, SceneMaterial>,
    pub parts: HashMap<NodeId, u32>,
    pub ui_owners: HashSet<NodeId>,
}
impl FrameStyles {
    pub fn prepare(
        access: &SceneSystemAccess<'_>,
        camera: &EditorCamera,
        capacity: usize,
    ) -> Result<Self, String> {
        access
            .require_write("render.extraction")
            .map_err(|e| e.to_string())?;
        let scene = access.read().map_err(|e| e.to_string())?;
        if !camera.valid() {
            return Err("invalid presentation camera".into());
        }
        let light = scene
            .active_components::<DirectionalLight>()
            .next()
            .map(|(_, light)| *light);
        if light.is_some_and(|light| !light.valid()) {
            return Err("invalid presentation light".into());
        }
        let mut materials = HashMap::new();
        let mut parts = HashMap::new();
        for (owner, _) in scene.active_components::<ModelInstance>() {
            if materials.len() >= capacity {
                return Err("presentation material capacity exceeded".into());
            }
            let material = scene
                .component::<SceneMaterial>(owner)
                .map_err(|e| e.to_string())?
                .copied()
                .unwrap_or_default();
            if !material.valid() {
                return Err("invalid presentation material".into());
            }
            materials.insert(owner, material);
            if let Some(part) = scene
                .component::<ModelPart>(owner)
                .map_err(|e| e.to_string())?
            {
                parts.insert(owner, part.node);
            }
        }
        let mut ui_owners = HashSet::new();
        for (owner, _) in scene.active_components::<voxy_gameplay::UiElement>() {
            if ui_owners.len() >= capacity {
                return Err("presentation UI owner capacity exceeded".into());
            }
            ui_owners.insert(owner);
        }
        Ok(Self {
            camera: camera.clone(),
            light,
            materials,
            parts,
            ui_owners,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{SceneGraph, Transform, extraction_schedule};
    #[test]
    fn ui_owner_snapshot_tracks_activity_removal_and_reused_generation() {
        use voxy_gameplay::UiElement;
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let element = UiElement {
            origin: [0., 0.],
            size: [1., 1.],
            color: [1.; 4],
            layer: 0,
            enabled: false,
            action: None,
            text: None,
        };
        scene.insert_component(child, element.clone()).unwrap();
        let capture = |scene: &mut SceneGraph, capacity| {
            let mut result = None;
            extraction_schedule()
                .unwrap()
                .run_scene(scene, |_, access| {
                    result = Some(FrameStyles::prepare(
                        &access,
                        &EditorCamera::default(),
                        capacity,
                    )?);
                    Ok::<(), String>(())
                })?;
            Ok::<_, voxy_scene::SystemFailure<String>>(result.unwrap())
        };
        let first = capture(&mut scene, 1).unwrap();
        // Disabled input is still visible; scene activity controls visibility.
        assert!(first.ui_owners.contains(&child));
        assert!(capture(&mut scene, 0).is_err());
        scene.set_active(root, false).unwrap();
        assert!(capture(&mut scene, 1).unwrap().ui_owners.is_empty());
        scene.set_active(root, true).unwrap();
        scene.remove_component::<UiElement>(child).unwrap();
        assert!(scene.active_in_hierarchy(child).unwrap());
        assert!(capture(&mut scene, 1).unwrap().ui_owners.is_empty());
        scene.remove_subtree(child).unwrap();
        let reused = scene.spawn(Some(root), Transform::default()).unwrap();
        scene.insert_component(reused, element).unwrap();
        let current = capture(&mut scene, 1).unwrap();
        assert!(current.ui_owners.contains(&reused));
        assert!(!current.ui_owners.contains(&child));
        assert!(first.ui_owners.contains(&child));
    }
    #[test]
    fn styles_own_camera_material_and_light_and_reject_invalid_inputs() {
        let mut scene = SceneGraph::new(2);
        let model = scene.spawn(None, Transform::default()).unwrap();
        let light = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                model,
                ModelInstance {
                    asset: voxy_assets::AssetId("model".into()),
                },
            )
            .unwrap();
        scene
            .insert_component(
                model,
                SceneMaterial {
                    tint: [0.2, 0.3, 0.4, 1.],
                    lit: true,
                },
            )
            .unwrap();
        scene
            .insert_component(light, DirectionalLight::default())
            .unwrap();
        scene
            .insert_component(model, ModelPart { node: 7 })
            .unwrap();
        let mut camera = EditorCamera::default();
        let mut first = None;
        extraction_schedule()
            .unwrap()
            .run_scene(&mut scene, |_, access| {
                first = Some(FrameStyles::prepare(&access, &camera, 1)?);
                Ok::<(), String>(())
            })
            .unwrap();
        let first = first.unwrap();
        let old_distance = camera.distance;
        camera.distance = 10.;
        scene
            .insert_component(model, ModelPart { node: 9 })
            .unwrap();
        assert_eq!(first.parts[&model], 7);
        scene
            .insert_component(model, SceneMaterial::default())
            .unwrap();
        scene.set_active(light, false).unwrap();
        assert_eq!(first.camera.distance, old_distance);
        assert_eq!(first.materials[&model].tint, [0.2, 0.3, 0.4, 1.]);
        assert!(first.light.is_some());
        extraction_schedule()
            .unwrap()
            .run_scene(&mut scene, |_, access| {
                let next = FrameStyles::prepare(&access, &camera, 1)?;
                assert!(next.light.is_none());
                assert_eq!(next.parts[&model], 9);
                assert_eq!(next.camera.distance, 10.);
                assert!(FrameStyles::prepare(&access, &camera, 0).is_err());
                Ok::<(), String>(())
            })
            .unwrap();
        scene
            .insert_component(
                model,
                SceneMaterial {
                    tint: [f32::NAN, 0., 0., 1.],
                    lit: true,
                },
            )
            .unwrap();
        assert!(
            extraction_schedule()
                .unwrap()
                .run_scene(&mut scene, |_, access| {
                    FrameStyles::prepare(&access, &camera, 1).map(|_| ())
                })
                .is_err()
        );
        assert_eq!(first.materials[&model].tint, [0.2, 0.3, 0.4, 1.]);
    }
}
