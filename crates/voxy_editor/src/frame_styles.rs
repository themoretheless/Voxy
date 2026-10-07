//! Owned presentation inputs; no scene borrowing survives preparation.
use super::{DirectionalLight, EditorCamera, ModelInstance, ModelPart, SceneMaterial};
use std::collections::{HashMap, HashSet};
use voxy_scene::{NodeId, SceneSystemAccess};
#[derive(Debug)]
pub(super) struct FrameStyles {
    pub camera: EditorCamera,
    pub light: Option<DirectionalLight>,
    pub shadow: Option<(crate::DirectionalShadow, voxy_render::ShadowSettings)>,
    pub casters: HashSet<NodeId>,
    pub materials: HashMap<NodeId, SceneMaterial>,
    pub parts: HashMap<NodeId, u32>,
    pub ui_owners: HashSet<NodeId>,
    pub fog: Vec<(NodeId, voxy_scene::FogVolume, voxy_scene::FogBounds)>,
}
impl FrameStyles {
    pub fn prepare(
        access: &SceneSystemAccess<'_>,
        camera: &EditorCamera,
        capacity: usize,
    ) -> Result<Self, String> {
        Self::prepare_with_world(access, camera, capacity, |scene, owner| {
            scene.world_matrix(owner)
        })
    }
    pub fn prepare_with_world<F>(
        access: &SceneSystemAccess<'_>,
        camera: &EditorCamera,
        capacity: usize,
        world: F,
    ) -> Result<Self, String>
    where
        F: Fn(&voxy_scene::SceneGraph, NodeId) -> Result<glam::Mat4, voxy_scene::SceneGraphError>,
    {
        access
            .require_write("render.extraction")
            .map_err(|e| e.to_string())?;
        let scene = access.read().map_err(|e| e.to_string())?;
        if !camera.valid() {
            return Err("invalid presentation camera".into());
        }
        let selected_light = scene
            .active_components::<DirectionalLight>()
            .next()
            .map(|(owner, light)| (owner, *light));
        let light = selected_light.map(|(_, light)| light);
        if light.is_some_and(|light| !light.valid()) {
            return Err("invalid presentation light".into());
        }
        let shadow = if let Some((owner, light)) = selected_light {
            scene
                .component::<crate::DirectionalShadow>(owner)
                .map_err(|e| e.to_string())?
                .copied()
                .map(|s| {
                    s.validate().map_err(str::to_owned)?;
                    if s.enabled {
                        s.settings(light).map(|settings| Some((s, settings)))
                    } else {
                        Ok(None)
                    }
                })
                .transpose()?
                .flatten()
        } else {
            None
        };
        let casters: HashSet<_> = scene
            .active_components::<crate::OpaqueShadowCaster>()
            .filter(|(_, caster)| caster.enabled)
            .map(|(owner, _)| owner)
            .collect();
        if casters.len() > capacity {
            return Err("shadow caster capacity exceeded".into());
        }
        let mut fog = Vec::new();
        for (owner, volume) in scene.active_components::<voxy_scene::FogVolume>() {
            volume.validate().map_err(|e| e.to_string())?;
            if !volume.enabled {
                continue;
            }
            if fog.len() >= capacity {
                return Err("presentation fog capacity exceeded".into());
            }
            let bounds = volume
                .world_bounds(world(scene, owner).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            fog.push((owner, *volume, bounds));
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
            shadow,
            casters,
            materials,
            parts,
            ui_owners,
            fog,
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
    #[test]
    fn fog_snapshots_follow_hierarchy_and_preserve_prior_values() {
        use voxy_scene::FogVolume;
        let mut scene = SceneGraph::new(3);
        let root = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(3., 4., 5.),
                    ..Default::default()
                },
            )
            .unwrap();
        let owner = scene
            .spawn(
                Some(root),
                Transform {
                    scale: glam::Vec3::new(2., 1., 1.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene.insert_component(owner, FogVolume::default()).unwrap();
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
                })
                .map_err(|e| format!("{e:?}"))?;
            Ok::<_, String>(result.unwrap())
        };
        let first = capture(&mut scene, 1).unwrap();
        assert_eq!(first.fog[0].0, owner);
        assert_eq!(first.fog[0].2.origin, [3., 4., 5.]);
        assert_eq!(first.fog[0].2.extent, [2., 1., 1.]);
        extraction_schedule()
            .unwrap()
            .run_scene(&mut scene, |_, access| {
                let resolved = FrameStyles::prepare_with_world(
                    &access,
                    &EditorCamera::default(),
                    1,
                    |scene, owner| {
                        Ok(glam::Mat4::from_translation(glam::Vec3::new(10., 0., 0.))
                            * scene.world_matrix(owner)?)
                    },
                )?;
                assert_eq!(resolved.fog[0].2.origin, [13., 4., 5.]);
                Ok::<(), String>(())
            })
            .unwrap();
        assert!(capture(&mut scene, 0).is_err());
        scene
            .component_mut::<FogVolume>(owner)
            .unwrap()
            .unwrap()
            .extinction_m_inverse = 2.;
        assert_eq!(
            capture(&mut scene, 1).unwrap().fog[0]
                .1
                .extinction_m_inverse,
            2.
        );
        assert_eq!(first.fog[0].1.extinction_m_inverse, 0.5);
        scene.set_active(root, false).unwrap();
        assert!(capture(&mut scene, 1).unwrap().fog.is_empty());
        scene.set_active(root, true).unwrap();
        scene
            .component_mut::<FogVolume>(owner)
            .unwrap()
            .unwrap()
            .enabled = false;
        assert!(capture(&mut scene, 1).unwrap().fog.is_empty());
        scene
            .component_mut::<FogVolume>(owner)
            .unwrap()
            .unwrap()
            .enabled = true;
        scene
            .component_mut::<FogVolume>(owner)
            .unwrap()
            .unwrap()
            .samples = 0;
        assert!(capture(&mut scene, 1).is_err());
        scene
            .component_mut::<FogVolume>(owner)
            .unwrap()
            .unwrap()
            .samples = 32;
        scene
            .set_local(
                owner,
                Transform {
                    rotation: glam::Quat::from_rotation_y(0.3),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(capture(&mut scene, 1).is_err());
        scene.remove_component::<FogVolume>(owner).unwrap();
        assert!(capture(&mut scene, 1).unwrap().fog.is_empty());
    }
    #[test]
    fn built_in_registry_admits_fog_and_rejects_invalid_calibration() {
        let registry = crate::editor_component_registry().unwrap();
        let mut document=voxy_scene::SceneDocument::from_json(r#"{"version":1,"objects":[{"id":"fog","parent":null,"name":"Fog","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"scene.fog.v1":{"enabled":true,"size":[1,2,3],"extinction_m_inverse":0.5,"single_scattering_albedo":0.8,"asymmetry":0,"samples":32}}}]}"#).unwrap();
        let loaded = document.load(&registry, 2).unwrap();
        let captured = loaded.capture(&registry).unwrap();
        let original_fog: voxy_scene::FogVolume =
            serde_json::from_value(document.objects[0].components["scene.fog.v1"].clone()).unwrap();
        let captured_fog: voxy_scene::FogVolume =
            serde_json::from_value(captured.objects[0].components["scene.fog.v1"].clone()).unwrap();
        assert_eq!(captured_fog, original_fog);
        assert_eq!(
            captured
                .load(&registry, 2)
                .unwrap()
                .capture(&registry)
                .unwrap(),
            captured
        );
        let mut canonical = document.clone();
        canonical.objects[0].components.insert(
            "scene.fog.v1".into(),
            serde_json::to_value(original_fog).unwrap(),
        );
        assert_eq!(captured, canonical);
        document.objects[0]
            .components
            .get_mut("scene.fog.v1")
            .unwrap()["extinction_m_inverse"] = serde_json::json!(1e-100);
        assert!(document.load(&registry, 2).is_err());
        document.objects[0]
            .components
            .get_mut("scene.fog.v1")
            .unwrap()["extinction_m_inverse"] = serde_json::json!(-0.5);
        assert!(document.load(&registry, 2).is_err());
    }
}

#[cfg(test)]
mod shadow_snapshot_tests {
    use super::*;
    #[test]
    fn shadows_follow_selected_light_and_caster_activity_without_mutating_prior_snapshot() {
        use voxy_scene::{SceneGraph, Transform, extraction_schedule};
        let mut scene = SceneGraph::new(8);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let light = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(light, crate::DirectionalLight::default())
            .unwrap();
        scene
            .insert_component(light, crate::DirectionalShadow::default())
            .unwrap();
        let caster = scene.spawn(Some(root), Transform::default()).unwrap();
        scene
            .insert_component(
                caster,
                crate::ModelInstance {
                    asset: voxy_assets::AssetId("fixture".into()),
                },
            )
            .unwrap();
        scene
            .insert_component(caster, crate::OpaqueShadowCaster::default())
            .unwrap();
        let capture = |scene: &mut SceneGraph| {
            let mut result = None;
            extraction_schedule()
                .unwrap()
                .run_scene(scene, |_, access| {
                    result = Some(FrameStyles::prepare(&access, &EditorCamera::default(), 8)?);
                    Ok::<(), String>(())
                })
                .unwrap();
            result.unwrap()
        };
        let first = capture(&mut scene);
        assert!(first.shadow.is_some());
        assert!(first.casters.contains(&caster));
        scene.set_active(root, false).unwrap();
        assert!(capture(&mut scene).casters.is_empty());
        scene.set_active(root, true).unwrap();
        scene
            .component_mut::<crate::OpaqueShadowCaster>(caster)
            .unwrap()
            .unwrap()
            .enabled = false;
        assert!(capture(&mut scene).casters.is_empty());
        scene
            .component_mut::<crate::DirectionalShadow>(light)
            .unwrap()
            .unwrap()
            .enabled = false;
        assert!(capture(&mut scene).shadow.is_none());
        scene
            .component_mut::<crate::DirectionalShadow>(light)
            .unwrap()
            .unwrap()
            .enabled = true;
        scene
            .component_mut::<crate::DirectionalLight>(light)
            .unwrap()
            .unwrap()
            .direction = [0., 0., 1.];
        let changed = capture(&mut scene);
        assert_ne!(
            first.shadow.unwrap().1.light_from_world,
            changed.shadow.unwrap().1.light_from_world
        );
        assert!(first.casters.contains(&caster));
        scene
            .remove_component::<crate::DirectionalLight>(light)
            .unwrap();
        assert!(capture(&mut scene).shadow.is_none());
    }
}
