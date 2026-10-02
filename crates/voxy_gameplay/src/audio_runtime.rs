//! Scene-to-mixer ownership bridge. No device callback or asset registry here.
use crate::{AudioSource, SceneAudioSnapshot};
use std::collections::{HashMap, HashSet};
use voxy_audio::{AudioError, Clip, Mixer, VoiceId, spatial_gains};
use voxy_scene::NodeId;
#[derive(Debug)]
struct SourceVoice {
    descriptor: AudioSource,
    voice: Option<VoiceId>,
    revision: u64,
}
#[derive(Debug)]
pub struct SceneAudioRuntime {
    mixer: Mixer,
    sources: HashMap<NodeId, SourceVoice>,
    rate: u32,
    capacity: usize,
    buses: usize,
    authored_buses: HashSet<usize>,
}
impl SceneAudioRuntime {
    /// # Errors
    /// Rejects invalid mixer limits.
    pub fn new(rate: u32, capacity: usize, buses: usize) -> Result<Self, AudioError> {
        Ok(Self {
            mixer: Mixer::new(rate, capacity, buses)?,
            sources: HashMap::new(),
            rate,
            capacity,
            buses,
            authored_buses: HashSet::new(),
        })
    }
    /// Reconciles scene owners against existing voices. Resolution and geometry
    /// validation finish before changing playback, preserving last-good voices on
    /// asset errors. Completed one-shot sources remain consumed until removed or
    /// their playback identity (asset/bus/looping) changes.
    /// # Errors
    /// Rejects invalid snapshots, missing/mismatched clips or mixer errors.
    pub fn synchronize(
        &mut self,
        snapshot: &SceneAudioSnapshot,
        resolve: impl FnMut(&str) -> Result<Clip, String>,
    ) -> Result<(), String> {
        self.synchronize_published(snapshot, |_| Ok(0), resolve)
    }
    /// Uses immutable publication revisions from the caller's existing asset owner.
    /// A changed revision restarts the source with the newly published clip. Gain,
    /// motion and activity changes keep the cursor; completed one-shots restart
    /// only for a new publication or playback identity.
    /// # Errors
    /// Publication lookup and clip/geometry errors preserve all existing voices.
    pub fn synchronize_published(
        &mut self,
        snapshot: &SceneAudioSnapshot,
        mut revision: impl FnMut(&str) -> Result<u64, String>,
        mut resolve: impl FnMut(&str) -> Result<Clip, String>,
    ) -> Result<(), String> {
        if snapshot.sources.len() > self.capacity {
            return Err("scene audio capacity exceeded".into());
        }
        let authored_buses = self.validate_buses(snapshot)?;
        let mut owners = HashSet::new();
        let mut prepared = Vec::with_capacity(snapshot.sources.len());
        for source in &snapshot.sources {
            let d = &source.descriptor;
            if !owners.insert(source.owner) || !d.valid() || usize::from(d.bus) >= self.buses {
                return Err("invalid or duplicate scene audio source".into());
            }
            let gains = if d.spatial {
                snapshot
                    .listener
                    .map(|listener| {
                        spatial_gains(
                            listener.position,
                            listener.right,
                            source.position,
                            d.near,
                            d.far,
                        )
                    })
                    .transpose()
                    .map_err(|e| e.to_string())?
                    .unwrap_or([0.; 2])
            } else {
                [1.; 2]
            };
            let published_revision = revision(&d.asset)?;
            let replace = self.sources.get(&source.owner).is_none_or(|old| {
                old.revision != published_revision
                    || old.descriptor.asset != d.asset
                    || old.descriptor.bus != d.bus
                    || old.descriptor.looping != d.looping
            });
            let clip = if replace {
                let clip = resolve(&d.asset)?;
                if clip.sample_rate() != self.rate {
                    return Err("scene audio clip sample rate mismatch".into());
                }
                Some(clip)
            } else {
                None
            };
            prepared.push((source, gains, clip, published_revision));
        }
        self.apply_buses(snapshot, authored_buses)?;
        self.remove_absent_sources(&owners)?;
        for (source, gains, clip, published_revision) in prepared {
            if let Some(clip) = clip {
                if let Some(old) = self.sources.remove(&source.owner) {
                    self.stop_voice(old.voice)?;
                }
                let d = &source.descriptor;
                let voice = self
                    .mixer
                    .play(clip, usize::from(d.bus), d.gain, d.looping)
                    .map_err(|e| e.to_string())?;
                self.sources.insert(
                    source.owner,
                    SourceVoice {
                        descriptor: d.clone(),
                        voice: Some(voice),
                        revision: published_revision,
                    },
                );
            }
            let entry = self
                .sources
                .get_mut(&source.owner)
                .ok_or("missing prepared audio source")?;
            if let Some(voice) = entry.voice {
                match self.mixer.pause(
                    voice,
                    !source.active || (source.descriptor.spatial && snapshot.listener.is_none()),
                ) {
                    Err(AudioError::InvalidVoice) => entry.voice = None,
                    Err(e) => return Err(e.to_string()),
                    Ok(()) => {
                        self.mixer
                            .ramp_voice_gain(voice, source.descriptor.gain, 0)
                            .map_err(|e| e.to_string())?;
                        self.mixer
                            .ramp_channel_gains(voice, gains, 0)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            entry.descriptor = source.descriptor.clone();
        }
        Ok(())
    }
    fn validate_buses(&self, snapshot: &SceneAudioSnapshot) -> Result<HashSet<usize>, String> {
        let mut authored_buses = HashSet::new();
        for bus in &snapshot.buses {
            if !bus.valid()
                || usize::from(bus.bus) >= self.buses
                || !authored_buses.insert(usize::from(bus.bus))
            {
                return Err("invalid or duplicate scene audio bus".into());
            }
        }
        Ok(authored_buses)
    }
    fn apply_buses(
        &mut self,
        snapshot: &SceneAudioSnapshot,
        authored_buses: HashSet<usize>,
    ) -> Result<(), String> {
        // All assets and descriptors were preflighted before modifying any gain.
        for bus in self.authored_buses.difference(&authored_buses) {
            self.mixer
                .ramp_bus_gain(*bus, 1., 0)
                .map_err(|e| e.to_string())?;
        }
        for bus in &snapshot.buses {
            self.mixer
                .ramp_bus_gain(usize::from(bus.bus), bus.gain, 0)
                .map_err(|e| e.to_string())?;
        }
        self.authored_buses = authored_buses;
        Ok(())
    }
    fn remove_absent_sources(&mut self, owners: &HashSet<NodeId>) -> Result<(), String> {
        let removed: Vec<_> = self
            .sources
            .keys()
            .copied()
            .filter(|owner| !owners.contains(owner))
            .collect();
        for owner in removed {
            if let Some(old) = self.sources.remove(&owner) {
                self.stop_voice(old.voice)?;
            }
        }
        Ok(())
    }
    fn stop_voice(&mut self, voice: Option<VoiceId>) -> Result<(), String> {
        match voice.map(|id| self.mixer.stop(id)) {
            None | Some(Ok(()) | Err(AudioError::InvalidVoice)) => Ok(()),
            Some(Err(e)) => Err(e.to_string()),
        }
    }
    pub fn render(&mut self, output: &mut [[f32; 2]]) {
        self.mixer.render(output);
    }
    /// # Errors
    /// Rejects invalid bus/gain.
    pub fn set_bus_gain(&mut self, bus: usize, gain: f32) -> Result<(), AudioError> {
        self.mixer.set_bus_gain(bus, gain)
    }
    pub fn stop(&mut self) {
        let entries = std::mem::take(&mut self.sources);
        for entry in entries.into_values() {
            let _ = self.stop_voice(entry.voice);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioListener, AudioSource, extract_scene_audio};
    use glam::Vec3;
    use voxy_scene::{SceneGraph, Transform};
    #[test]
    fn authored_bus_changes_preserve_cursor_and_failed_preflight_preserves_gain() {
        let mut scene = SceneGraph::new(3);
        let source = scene.spawn(None, Transform::default()).unwrap();
        let bus = scene.spawn(None, Transform::default()).unwrap();
        let duplicate = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                source,
                AudioSource {
                    asset: "tone".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 1.,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        scene
            .insert_component(bus, crate::AudioBus { bus: 0, gain: 0.5 })
            .unwrap();
        let clip = Clip::new(48000, vec![[0.8; 2], [0.4; 2]]).unwrap();
        let mut runtime = SceneAudioRuntime::new(48000, 1, 1).unwrap();
        let mut snapshot = extract_scene_audio(&scene, 1).unwrap();
        runtime
            .synchronize_published(&snapshot, |_| Ok(1), |_| Ok(clip.clone()))
            .unwrap();
        let mut output = [[0.; 2]; 1];
        runtime.render(&mut output);
        assert!((output[0][0] - 0.4).abs() < 1e-6);
        snapshot.buses[0].gain = 0.;
        assert!(
            runtime
                .synchronize_published(&snapshot, |_| Ok(2), |_| Err("failed reload".into()))
                .is_err()
        );
        runtime.render(&mut output);
        assert!((output[0][0] - 0.2).abs() < 1e-6);
        snapshot.buses[0].gain = 0.25;
        runtime
            .synchronize_published(&snapshot, |_| Ok(1), |_| panic!("gain must keep clip"))
            .unwrap();
        runtime.render(&mut output);
        assert!((output[0][0] - 0.2).abs() < 1e-6);
        scene
            .insert_component(duplicate, crate::AudioBus { bus: 0, gain: 1. })
            .unwrap();
        assert!(extract_scene_audio(&scene, 1).is_err());
        scene.set_active(duplicate, false).unwrap();
        scene.set_active(bus, false).unwrap();
        runtime
            .synchronize_published(
                &extract_scene_audio(&scene, 1).unwrap(),
                |_| Ok(1),
                |_| panic!("activation must keep clip"),
            )
            .unwrap();
        runtime.render(&mut output);
        assert!((output[0][0] - 0.4).abs() < 1e-6);
    }
    #[test]
    fn same_asset_publications_reload_atomically_and_motion_keeps_cursor() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                AudioSource {
                    import_settings: None,
                    asset: "tone".into(),
                    bus: 0,
                    gain: 1.,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        let snapshot = extract_scene_audio(&scene, 1).unwrap();
        let old = Clip::new(48000, vec![[0.5; 2], [0.25; 2]]).unwrap();
        let new = Clip::new(48000, vec![[0.75; 2]]).unwrap();
        let mut runtime = SceneAudioRuntime::new(48000, 1, 1).unwrap();
        runtime
            .synchronize_published(&snapshot, |_| Ok(1), |_| Ok(old.clone()))
            .unwrap();
        let mut output = [[0.; 2]; 1];
        runtime.render(&mut output);
        runtime
            .synchronize_published(
                &snapshot,
                |_| Ok(1),
                |_| panic!("unchanged publication must not resolve again"),
            )
            .unwrap();
        assert!(
            runtime
                .synchronize_published(&snapshot, |_| Ok(2), |_| Err("decode failed".into()))
                .is_err()
        );
        runtime.render(&mut output);
        assert!((output[0][0] - 0.25).abs() < 1e-6);
        assert!(
            runtime
                .synchronize_published(
                    &snapshot,
                    |_| Err("publication unavailable".into()),
                    |_| unreachable!()
                )
                .is_err()
        );
        runtime
            .synchronize_published(&snapshot, |_| Ok(2), |_| Ok(new.clone()))
            .unwrap();
        runtime.render(&mut output);
        assert!((output[0][0] - 0.75).abs() < 1e-6);
        runtime.stop();
    }

    #[test]
    fn scene_mixer_moves_pauses_preserves_on_failure_and_consumes_one_shots() {
        let mut scene = SceneGraph::new(3);
        let listener = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(listener, AudioListener::default())
            .unwrap();
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X,
                    ..Transform::default()
                },
            )
            .unwrap();
        let descriptor = AudioSource {
            import_settings: None,
            asset: "tone".into(),
            bus: 0,
            gain: 1.,
            looping: true,
            spatial: true,
            near: 1.,
            far: 5.,
        };
        scene.insert_component(owner, descriptor.clone()).unwrap();
        let clip = Clip::new(48000, vec![[0.5; 2], [0.25; 2]]).unwrap();
        let mut runtime = SceneAudioRuntime::new(48000, 1, 1).unwrap();
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| {
                Ok(clip.clone())
            })
            .unwrap();
        let mut out = [[0.; 2]; 1];
        runtime.render(&mut out);
        assert!(out[0][0].abs() < 1e-6 && (out[0][1] - 0.5).abs() < 1e-6);
        scene.set_active(owner, false).unwrap();
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| {
                panic!("existing clip should be retained")
            })
            .unwrap();
        runtime.render(&mut out);
        assert!(out[0].iter().all(|v| v.abs() < 1e-6));
        scene.set_active(owner, true).unwrap();
        scene
            .set_local(
                owner,
                Transform {
                    translation: -Vec3::X,
                    ..Transform::default()
                },
            )
            .unwrap();
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| unreachable!())
            .unwrap();
        let mut broken = extract_scene_audio(&scene, 1).unwrap();
        broken.sources[0].descriptor.asset = "missing".into();
        assert!(
            runtime
                .synchronize(&broken, |_| Err("missing".into()))
                .is_err()
        );
        runtime.render(&mut out);
        assert!((out[0][0] - 0.25).abs() < 1e-6 && out[0][1].abs() < 1e-6);
        scene
            .insert_component(
                owner,
                AudioSource {
                    looping: false,
                    ..descriptor
                },
            )
            .unwrap();
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| {
                Ok(clip.clone())
            })
            .unwrap();
        runtime.render(&mut [[0.; 2]; 2]);
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| unreachable!())
            .unwrap();
        runtime.render(&mut out);
        assert!(out[0].iter().all(|v| v.abs() < 1e-6));
        scene.remove_subtree(owner).unwrap();
        runtime
            .synchronize(&extract_scene_audio(&scene, 1).unwrap(), |_| unreachable!())
            .unwrap();
        assert!(runtime.sources.is_empty());
        runtime.stop();
    }
}
