//! Durable scene audio descriptors and immutable extraction for the audio owner.
use crate::GameplayError;
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
) -> Result<SceneAudioSnapshot, GameplayError> {
    let mut buses = Vec::new();
    for (owner, descriptor) in scene.components::<AudioBus>() {
        if !descriptor.valid() {
            return Err("invalid scene audio bus".into());
        }
        if scene.active_in_hierarchy(owner)? {
            if buses.iter().any(|old: &AudioBus| old.bus == descriptor.bus) {
                return Err("multiple active owners of audio bus".into());
            }
            buses.push(*descriptor);
        }
    }
    let mut listeners = scene.active_components::<AudioListener>();
    let listener = listeners
        .next()
        .map(
            |(owner, _)| -> Result<AudioListenerSnapshot, GameplayError> {
                let world = scene.world_matrix(owner)?;
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
            },
        )
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
            return Err(format!("invalid audio source {owner:?}").into());
        }
        let position = scene.world_matrix(owner)?.transform_point3(Vec3::ZERO);
        if !position.is_finite() {
            return Err("invalid audio source geometry".into());
        }
        sources.push(AudioSourceSnapshot {
            owner,
            descriptor: descriptor.clone(),
            active: scene.active_in_hierarchy(owner)?,
            position: position.to_array(),
        });
    }
    Ok(SceneAudioSnapshot {
        listener,
        buses,
        sources,
    })
}

/// Computes the acoustic Doppler pitch multiplier for a source and listener moving in 3D space.
/// Clamps radial speeds to `0.9 * speed_of_sound` and pitch shift to `[0.25, 4.0]` to avoid
/// sonic-boom singularities.
#[must_use]
pub fn compute_doppler_pitch_shift(
    source_pos: [f32; 3],
    source_vel: [f32; 3],
    listener_pos: [f32; 3],
    listener_vel: [f32; 3],
    speed_of_sound: f32,
) -> f32 {
    if !speed_of_sound.is_finite() || speed_of_sound <= 1.0 {
        return 1.0;
    }
    let dir = Vec3::from_array(listener_pos) - Vec3::from_array(source_pos);
    let dist_sq = dir.length_squared();
    if !dist_sq.is_finite() || dist_sq <= 1e-8 {
        return 1.0;
    }
    let unit_to_listener = dir / dist_sq.sqrt();
    let v_s = Vec3::from_array(source_vel).dot(unit_to_listener);
    let v_l = Vec3::from_array(listener_vel).dot(unit_to_listener);
    if !v_s.is_finite() || !v_l.is_finite() {
        return 1.0;
    }
    let max_radial = 0.9 * speed_of_sound;
    let clamped_vs = v_s.clamp(-max_radial, max_radial);
    let clamped_vl = v_l.clamp(-max_radial, max_radial);
    ((speed_of_sound - clamped_vl) / (speed_of_sound - clamped_vs)).clamp(0.25, 4.0)
}

/// Evaluates inverse-distance spatial audio gain attenuation with a smooth $C^1$ window
/// that reaches exact `0.0` at `max_distance` and stays `1.0` inside `ref_distance`.
#[must_use]
pub fn inverse_distance_windowed_attenuation(
    distance: f32,
    ref_distance: f32,
    max_distance: f32,
    rolloff_factor: f32,
) -> f32 {
    if !distance.is_finite()
        || !ref_distance.is_finite()
        || !max_distance.is_finite()
        || !rolloff_factor.is_finite()
        || ref_distance <= 0.0
        || max_distance <= ref_distance
        || rolloff_factor < 0.0
    {
        return 0.0;
    }
    if distance <= ref_distance {
        return 1.0;
    }
    if distance >= max_distance {
        return 0.0;
    }
    let inv = ref_distance / (ref_distance + rolloff_factor * (distance - ref_distance));
    let u = ((distance - ref_distance) / (max_distance - ref_distance)).clamp(0.0, 1.0);
    let window = (1.0 - u * u).powi(2);
    (inv * window).clamp(0.0, 1.0)
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
                .to_string()
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
    #[test]
    fn doppler_pitch_shift_increases_on_approach_and_clamps_extremes() {
        let approaching = compute_doppler_pitch_shift(
            [0.0, 0.0, 0.0],
            [34.3, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            343.0,
        );
        assert!(approaching > 1.1 && approaching < 1.12);
        let receding = compute_doppler_pitch_shift(
            [0.0, 0.0, 0.0],
            [-34.3, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            343.0,
        );
        assert!(receding < 0.92 && receding > 0.90);
        let supersonic = compute_doppler_pitch_shift(
            [0.0, 0.0, 0.0],
            [5000.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [-5000.0, 0.0, 0.0],
            343.0,
        );
        assert!((supersonic - 4.0).abs() < 1e-6);
        assert_eq!(
            inverse_distance_windowed_attenuation(1.0, 2.0, 20.0, 1.0),
            1.0
        );
        assert_eq!(
            inverse_distance_windowed_attenuation(20.0, 2.0, 20.0, 1.0),
            0.0
        );
        let mid = inverse_distance_windowed_attenuation(6.0, 2.0, 20.0, 1.0);
        assert!(mid > 0.0 && mid < 1.0);
    }
}
