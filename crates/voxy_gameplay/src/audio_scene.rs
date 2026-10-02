//! Durable scene audio descriptors and immutable extraction for the audio owner.
use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxy_scene::{NodeId, SceneGraph};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSource {
    pub asset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_settings: Option<String>,
    pub bus: u16,
    pub gain: f32,
    pub looping: bool,
    pub spatial: bool,
    pub near: f32,
    pub far: f32,
}
impl AudioSource {
    #[must_use]
    pub fn valid(&self) -> bool {
        self.import_settings.as_ref().is_none_or(|id| {
            !id.is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control)
        }) && !self.asset.is_empty()
            && self.asset.len() <= 1024
            && !self.asset.chars().any(char::is_control)
            && self.gain.is_finite()
            && (0.0..=1.0).contains(&self.gain)
            && self.near.is_finite()
            && self.far.is_finite()
            && self.near >= 0.0
            && self.far > self.near
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioListener {}
/// A scene-owned gain for one mixer bus. Only one active owner may set a bus.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioBus {
    pub bus: u16,
    pub gain: f32,
}
impl AudioBus {
    #[must_use]
    pub fn valid(&self) -> bool {
        self.bus < 16 && self.gain.is_finite() && (0.0..=1.0).contains(&self.gain)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AudioSourceSnapshot {
    pub owner: NodeId,
    pub descriptor: AudioSource,
    pub active: bool,
    pub position: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioListenerSnapshot {
    pub position: [f32; 3],
    pub right: [f32; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct SceneAudioSnapshot {
    pub listener: Option<AudioListenerSnapshot>,
    pub buses: Vec<AudioBus>,
    pub sources: Vec<AudioSourceSnapshot>,
}
/// Extracts owned data without keeping references to the live graph. Inactive
/// sources remain present so an audio owner can pause rather than restart them.
/// # Errors
/// Rejects invalid descriptors, capacity, competing active listeners or invalid
/// world geometry. Missing listeners are explicit rather than a guessed camera.
pub fn extract_scene_audio(
    scene: &SceneGraph,
    max_sources: usize,
) -> Result<SceneAudioSnapshot, String> {
    let mut buses = Vec::new();
    for (owner, descriptor) in scene.components::<AudioBus>() {
        if !descriptor.valid() {
            return Err("invalid scene audio bus".into());
        }
        if scene
            .active_in_hierarchy(owner)
            .map_err(|e| e.to_string())?
        {
            if buses.iter().any(|old: &AudioBus| old.bus == descriptor.bus) {
                return Err("multiple active owners of audio bus".into());
            }
            buses.push(*descriptor);
        }
    }
    let mut listeners = scene.active_components::<AudioListener>();
    let listener = listeners
        .next()
        .map(|(owner, _)| -> Result<AudioListenerSnapshot, String> {
            let world = scene.world_matrix(owner).map_err(|e| e.to_string())?;
            let position = world.transform_point3(Vec3::ZERO);
            let right = world.transform_vector3(Vec3::X);
            if !position.is_finite()
                || !right.is_finite()
                || !right.length_squared().is_finite()
                || right.length_squared() <= 1e-12
            {
                return Err("invalid audio listener geometry".into());
            }
            Ok(AudioListenerSnapshot {
                position: position.to_array(),
                right: right.normalize().to_array(),
            })
        })
        .transpose()?;
    if listeners.next().is_some() {
        return Err("multiple active audio listeners".into());
    }
    let mut sources = Vec::new();
    for (owner, descriptor) in scene.components::<AudioSource>() {
        if sources.len() >= max_sources {
            return Err("scene audio source capacity exceeded".into());
        }
        if !descriptor.valid() {
            return Err(format!("invalid audio source {owner:?}"));
        }
        let position = scene
            .world_matrix(owner)
            .map_err(|e| e.to_string())?
            .transform_point3(Vec3::ZERO);
        if !position.is_finite() {
            return Err("invalid audio source geometry".into());
        }
        sources.push(AudioSourceSnapshot {
            owner,
            descriptor: descriptor.clone(),
            active: scene
                .active_in_hierarchy(owner)
                .map_err(|e| e.to_string())?,
            position: position.to_array(),
        });
    }
    Ok(SceneAudioSnapshot {
        listener,
        buses,
        sources,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{ComponentRegistry, ObjectId, SceneDocument, SceneObject, Transform};
    #[test]
    fn invalid_sources_and_competing_listeners_reject_without_mutation() {
        let mut scene = SceneGraph::new(3);
        let first = scene.spawn(None, Transform::default()).unwrap();
        let second = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(first, AudioListener::default())
            .unwrap();
        scene
            .insert_component(second, AudioListener::default())
            .unwrap();
        assert!(
            extract_scene_audio(&scene, 2)
                .unwrap_err()
                .contains("multiple")
        );
        scene.set_active(second, false).unwrap();
        assert!(extract_scene_audio(&scene, 2).unwrap().listener.is_some());
        scene
            .insert_component(
                second,
                AudioSource {
                    import_settings: None,
                    asset: "sound".into(),
                    bus: 0,
                    gain: f32::NAN,
                    looping: false,
                    spatial: true,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        assert!(extract_scene_audio(&scene, 2).is_err());
        assert!(
            scene
                .component::<AudioSource>(second)
                .unwrap()
                .unwrap()
                .gain
                .is_nan()
        );
    }

    #[test]
    fn saved_audio_extracts_parent_motion_activity_and_generation_without_graph_borrow() {
        let mut registry = ComponentRegistry::default();
        crate::register_components(&mut registry).unwrap();
        let mut scene = SceneGraph::new(4);
        let parent = scene.spawn(None, Transform::default()).unwrap();
        let source = scene.spawn(Some(parent), Transform::default()).unwrap();
        scene
            .insert_component(parent, AudioListener::default())
            .unwrap();
        let descriptor = AudioSource {
            import_settings: None,
            asset: "sound".into(),
            bus: 0,
            gain: 0.5,
            looping: true,
            spatial: true,
            near: 1.,
            far: 5.,
        };
        let document = SceneDocument {
            version: 1,
            objects: vec![SceneObject {
                id: ObjectId("source".into()),
                parent: None,
                name: "Sound".into(),
                active: true,
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
                components: [(
                    "game.audio-source.v1".into(),
                    serde_json::to_value(&descriptor).unwrap(),
                )]
                .into(),
            }],
        };
        let loaded = document.load(&registry, 1).unwrap();
        assert_eq!(
            loaded.graph.components::<AudioSource>().next().unwrap().1,
            &descriptor
        );
        scene.insert_component(source, descriptor).unwrap();
        let old = extract_scene_audio(&scene, 1).unwrap();
        scene
            .set_local(
                parent,
                Transform {
                    translation: Vec3::X,
                    ..Transform::default()
                },
            )
            .unwrap();
        let moved = extract_scene_audio(&scene, 1).unwrap();
        assert!(Vec3::from_array(old.sources[0].position).length() < 1e-6);
        assert!((Vec3::from_array(moved.sources[0].position) - Vec3::X).length() < 1e-6);
        scene.set_active(parent, false).unwrap();
        let paused = extract_scene_audio(&scene, 1).unwrap();
        assert!(!paused.sources[0].active && paused.listener.is_none());
        scene.remove_subtree(parent).unwrap();
        assert!(extract_scene_audio(&scene, 1).unwrap().sources.is_empty());
        assert_ne!(scene.spawn(None, Transform::default()).unwrap(), source);
        assert!(extract_scene_audio(&loaded.graph, 0).is_err());
    }
}
