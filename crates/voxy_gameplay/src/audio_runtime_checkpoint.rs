//! Scene audio owner state plus bounded mixer wire representation.
use super::*;
use serde::{Deserialize, Serialize};
use voxy_audio::{GainCheckpoint, MixerCheckpoint, VoiceCheckpoint};
use voxy_scene::{ObjectId, SceneGraph};

/// Host output compatibility and admission policy; counts bound logical state.
#[derive(Clone, Copy, Debug)]
pub struct SceneAudioCheckpointLimits {
    pub sample_rate: u32,
    pub max_sources: usize,
    pub max_buses: usize,
    pub max_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAudioCheckpoint {
    version: u32,
    mixer: MixerWire,
    sources: Vec<Source>,
    authored_buses: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    owner: ObjectId,
    descriptor: AudioSource,
    voice: Option<u64>,
    revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GainWire {
    current: u64,
    target: u64,
    remaining: u32,
}
impl From<GainCheckpoint> for GainWire {
    fn from(g: GainCheckpoint) -> Self {
        Self {
            current: g.current,
            target: g.target,
            remaining: g.remaining,
        }
    }
}
impl From<GainWire> for GainCheckpoint {
    fn from(g: GainWire) -> Self {
        Self {
            current: g.current,
            target: g.target,
            remaining: g.remaining,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoiceWire {
    serial: u64,
    clip: String,
    cursor: usize,
    bus: usize,
    gain: GainWire,
    channels: [GainWire; 2],
    looping: bool,
    paused: bool,
}
impl From<VoiceCheckpoint> for VoiceWire {
    fn from(v: VoiceCheckpoint) -> Self {
        Self {
            serial: v.serial,
            clip: v.clip,
            cursor: v.cursor,
            bus: v.bus,
            gain: v.gain.into(),
            channels: v.channels.map(Into::into),
            looping: v.looping,
            paused: v.paused,
        }
    }
}
impl From<VoiceWire> for VoiceCheckpoint {
    fn from(v: VoiceWire) -> Self {
        Self {
            serial: v.serial,
            clip: v.clip,
            cursor: v.cursor,
            bus: v.bus,
            gain: v.gain.into(),
            channels: v.channels.map(Into::into),
            looping: v.looping,
            paused: v.paused,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MixerWire {
    version: u32,
    serial: u64,
    rate: u32,
    voices: Vec<Option<VoiceWire>>,
    buses: Vec<GainWire>,
}
impl From<MixerCheckpoint> for MixerWire {
    fn from(m: MixerCheckpoint) -> Self {
        Self {
            version: m.version,
            serial: m.serial,
            rate: m.rate,
            voices: m.voices.into_iter().map(|v| v.map(Into::into)).collect(),
            buses: m.buses.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<MixerWire> for MixerCheckpoint {
    fn from(m: MixerWire) -> Self {
        Self {
            version: m.version,
            serial: m.serial,
            rate: m.rate,
            voices: m.voices.into_iter().map(|v| v.map(Into::into)).collect(),
            buses: m.buses.into_iter().map(Into::into).collect(),
        }
    }
}
impl SceneAudioRuntime {
    /// # Errors
    /// Rejects unidentified/duplicate owners, clip revision keys and byte overflow.
    pub fn checkpoint(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        identify_clip: impl FnMut(&Clip) -> Option<String>,
        max_bytes: usize,
    ) -> Result<SceneAudioCheckpoint, GameplayError> {
        let mixer = self.mixer.checkpoint(identify_clip)?.into();
        let mut sources = Vec::new();
        let mut ids = HashSet::new();
        for (owner, source) in &self.sources {
            let owner = identify(*owner).ok_or("unidentified scene audio owner")?;
            if owner.0.is_empty() || !ids.insert(owner.clone()) {
                return Err("duplicate scene audio owner".into());
            }
            sources.push(Source {
                owner,
                descriptor: source.descriptor.clone(),
                voice: source
                    .voice
                    .map(|v| self.mixer.voice_reference_key(v))
                    .transpose()?,
                revision: source.revision,
            });
        }
        sources.sort_by(|a, b| a.owner.0.cmp(&b.owner.0));
        let mut authored_buses: Vec<_> = self.authored_buses.iter().copied().collect();
        authored_buses.sort_unstable();
        let saved = SceneAudioCheckpoint {
            version: 1,
            mixer,
            sources,
            authored_buses,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl SceneAudioCheckpoint {
    /// # Errors
    /// Rejects byte overflow.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, GameplayError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("scene audio checkpoint byte budget".into());
        }
        Ok(json)
    }
    /// # Errors
    /// Rejects byte overflow and invalid encoding; prepare checks semantics.
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, GameplayError> {
        if json.len() > max_bytes {
            return Err("scene audio checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    /// Resolves exact PCM revisions and scene identities into an independent owner.
    /// Accepted descriptors may precede a pending scene edit; next synchronize reconciles it.
    /// # Errors
    /// Rejects missing resources/owners, invalid state and caller limits.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        resolve_clip: impl FnMut(&str) -> Option<Clip>,
        limits: SceneAudioCheckpointLimits,
    ) -> Result<SceneAudioRuntime, GameplayError> {
        self.to_json(limits.max_bytes)?;
        if self.version != 1
            || self.sources.len() > limits.max_sources
            || self.mixer.rate != limits.sample_rate
        {
            return Err("scene audio checkpoint capacity or version".into());
        }
        let wire: MixerCheckpoint = self.mixer.clone().into();
        let mixer = wire.prepare(resolve_clip, limits.max_sources, limits.max_buses)?;
        let mut authored_buses = HashSet::new();
        for bus in &self.authored_buses {
            if *bus >= wire.buses.len() || !authored_buses.insert(*bus) {
                return Err("invalid authored audio bus".into());
            }
        }
        let mut sources = HashMap::new();
        let mut ids = HashSet::new();
        let mut references = HashSet::new();
        for saved in &self.sources {
            if saved.owner.0.is_empty()
                || !ids.insert(saved.owner.clone())
                || !saved.descriptor.valid()
                || usize::from(saved.descriptor.bus) >= wire.buses.len()
            {
                return Err("invalid scene audio source checkpoint".into());
            }
            let owner = resolve(&saved.owner).ok_or("missing scene audio owner")?;
            scene.local(owner)?;
            if sources.contains_key(&owner) {
                return Err("aliased scene audio owner".into());
            }
            let voice = if let Some(serial) = saved.voice {
                if !references.insert(serial) {
                    return Err("aliased scene audio voice".into());
                }
                if let Some(v) = wire.voices.iter().flatten().find(|v| v.serial == serial)
                    && (v.bus != usize::from(saved.descriptor.bus)
                        || v.looping != saved.descriptor.looping)
                {
                    return Err("incompatible scene audio voice".into());
                }
                Some(mixer.restore_voice_reference(serial)?)
            } else {
                None
            };
            sources.insert(
                owner,
                SourceVoice {
                    descriptor: saved.descriptor.clone(),
                    voice,
                    revision: saved.revision,
                },
            );
        }
        if wire
            .voices
            .iter()
            .flatten()
            .any(|v| !references.contains(&v.serial))
        {
            return Err("orphaned scene audio voice".into());
        }
        Ok(SceneAudioRuntime {
            mixer,
            sources,
            rate: wire.rate,
            capacity: wire.voices.len(),
            buses: wire.buses.len(),
            authored_buses,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;
    #[test]
    fn json_audio_owner_continues_and_does_not_restart_completed_one_shot() {
        let mut source_scene = SceneGraph::new(2);
        let looping = source_scene.spawn(None, Transform::default()).unwrap();
        let once = source_scene.spawn(None, Transform::default()).unwrap();
        let descriptor = AudioSource {
            asset: "tone".into(),
            import_settings: None,
            bus: 0,
            gain: 0.8,
            looping: true,
            spatial: false,
            near: 1.,
            far: 5.,
        };
        source_scene
            .insert_component(looping, descriptor.clone())
            .unwrap();
        let mut one = descriptor.clone();
        one.looping = false;
        source_scene.insert_component(once, one.clone()).unwrap();
        let clip = Clip::new(48000, vec![[0.1, -0.2], [0.3, -0.4], [0.5, -0.6]]).unwrap();
        let mut original = SceneAudioRuntime::new(48000, 2, 1).unwrap();
        original
            .synchronize_published(
                &crate::extract_scene_audio(&source_scene, 2).unwrap(),
                |_| Ok(7),
                |_| Ok(clip.clone()),
            )
            .unwrap();
        original.render(&mut [[0.; 2]; 5]);
        // The source still retains its completed VoiceId before the next synchronization.
        let identify = |n| Some(ObjectId(if n == looping { "loop" } else { "once" }.into()));
        let saved = original
            .checkpoint(identify, |_| Some("pcm-v7".into()), 65536)
            .unwrap();
        let json = saved.to_json(65536).unwrap();
        let saved = SceneAudioCheckpoint::from_json(&json, 65536).unwrap();
        let mut scene = SceneGraph::new(3);
        let _padding = scene.spawn(None, Transform::default()).unwrap();
        let fresh_loop = scene.spawn(None, Transform::default()).unwrap();
        let fresh_once = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(fresh_loop, descriptor).unwrap();
        scene.insert_component(fresh_once, one).unwrap();
        let resolve = |id: &ObjectId| match id.0.as_str() {
            "loop" => Some(fresh_loop),
            "once" => Some(fresh_once),
            _ => None,
        };
        let limits = SceneAudioCheckpointLimits {
            sample_rate: 48000,
            max_sources: 2,
            max_buses: 1,
            max_bytes: 65536,
        };
        let bytes = clip.to_pcm_bytes().unwrap();
        let mut restored = saved
            .prepare(
                &scene,
                resolve,
                |_| Clip::from_pcm_bytes(&bytes, 3).ok(),
                limits,
            )
            .unwrap();
        let identify_fresh = |n| {
            Some(ObjectId(
                if n == fresh_loop { "loop" } else { "once" }.into(),
            ))
        };
        assert_eq!(
            restored
                .checkpoint(identify_fresh, |_| Some("pcm-v7".into()), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            json
        );
        for frame in 0..120 {
            original
                .synchronize_published(
                    &crate::extract_scene_audio(&source_scene, 2).unwrap(),
                    |_| Ok(7),
                    |_| panic!("unchanged publication must not restart"),
                )
                .unwrap();
            restored
                .synchronize_published(
                    &crate::extract_scene_audio(&scene, 2).unwrap(),
                    |_| Ok(7),
                    |_| panic!("restored publication must not restart"),
                )
                .unwrap();
            let mut a = vec![[0.; 2]; [1, 7, 0, 11][frame % 4]];
            let mut b = a.clone();
            original.render(&mut a);
            restored.render(&mut b);
            assert_eq!(
                a.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>(),
                b.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            assert_eq!(
                original
                    .checkpoint(identify, |_| Some("pcm-v7".into()), 65536)
                    .unwrap()
                    .to_json(65536)
                    .unwrap(),
                restored
                    .checkpoint(identify_fresh, |_| Some("pcm-v7".into()), 65536)
                    .unwrap()
                    .to_json(65536)
                    .unwrap()
            );
        }
        assert!(
            saved
                .prepare(&scene, |_| None, |_| Some(clip.clone()), limits)
                .is_err()
        );
        assert!(saved.prepare(&scene, resolve, |_| None, limits).is_err());
        assert!(
            saved
                .prepare(
                    &scene,
                    resolve,
                    |_| Some(clip.clone()),
                    SceneAudioCheckpointLimits {
                        sample_rate: 44100,
                        ..limits
                    }
                )
                .is_err()
        );
        assert!(SceneAudioCheckpoint::from_json(&json, 1).is_err());
        let mut bad = saved;
        bad.sources[0].voice = Some(u64::MAX);
        assert!(
            bad.prepare(&scene, resolve, |_| Some(clip.clone()), limits)
                .is_err()
        );
    }
}

#[cfg(test)]
mod archive_integration_tests {
    use super::*;
    use voxy_assets::{
        AssetCatalog, AssetId, ImportInputs, ImportedArchiveLimits, ImportedAsset, InputError,
    };
    use voxy_scene::Transform;
    #[test]
    fn archived_publication_catalog_and_audio_continue_after_failed_reload() {
        let mut scene = SceneGraph::new(1);
        let node = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                node,
                AudioSource {
                    asset: "tone".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 0.5,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        let mut inputs = ImportInputs::new(1, 100);
        inputs
            .read(AssetId("tone.source".into()), |_, _| Ok(vec![1, 2, 3]))
            .unwrap();
        let imported = inputs
            .finish(
                Clip::new(48000, vec![[0.1; 2], [0.3; 2], [0.5; 2]]).unwrap(),
                |_, _| Ok(vec![1, 2, 3]),
            )
            .unwrap();
        let mut catalog = AssetCatalog::new(2, 1).unwrap();
        let id = AssetId("tone".into());
        let ticket = catalog.request(id.clone()).unwrap();
        catalog.complete(&ticket, Ok(imported)).unwrap();
        let mut audio = SceneAudioRuntime::new(48000, 1, 1).unwrap();
        let snapshot = crate::extract_scene_audio(&scene, 1).unwrap();
        crate::synchronize_audio_assets(&mut audio, &snapshot, &catalog).unwrap();
        audio.render(&mut [[0.; 2]; 7]);
        let ticket = catalog.request(id.clone()).unwrap();
        catalog
            .complete(&ticket, Err("failed reload".into()))
            .unwrap();
        let policy = ImportedArchiveLimits {
            max_inputs: 1,
            max_input_bytes: 100,
            max_archive_bytes: 4096,
        };
        let bytes = catalog
            .snapshot(&id)
            .unwrap()
            .to_checkpoint_bytes(
                |clip| {
                    clip.to_pcm_bytes()
                        .map_err(|e| InputError::Read(e.to_string()))
                },
                policy,
            )
            .unwrap();
        let saved_catalog = catalog
            .checkpoint(|_| Some("publication-1".into()))
            .unwrap();
        let loaded_catalog = saved_catalog
            .prepare(
                |_| {
                    Some(std::sync::Arc::new(
                        ImportedAsset::from_checkpoint_bytes(
                            &bytes,
                            |pcm| {
                                Clip::from_pcm_bytes(pcm, 3)
                                    .map_err(|e| InputError::Read(e.to_string()))
                            },
                            policy,
                        )
                        .unwrap(),
                    ))
                },
                2,
                1,
            )
            .unwrap();
        assert_eq!(loaded_catalog.status(&id), catalog.status(&id));
        assert_eq!(loaded_catalog.snapshot_with_revision(&id).unwrap().0, 1);
        let saved_audio = audio
            .checkpoint(
                |_| Some(ObjectId("source".into())),
                |_| Some("publication-1".into()),
                65536,
            )
            .unwrap();
        let saved_audio =
            SceneAudioCheckpoint::from_json(&saved_audio.to_json(65536).unwrap(), 65536).unwrap();
        let mut loaded = saved_audio
            .prepare(
                &scene,
                |_| Some(node),
                |_| Some(loaded_catalog.snapshot(&id).unwrap().value().clone()),
                SceneAudioCheckpointLimits {
                    sample_rate: 48000,
                    max_sources: 1,
                    max_buses: 1,
                    max_bytes: 65536,
                },
            )
            .unwrap();
        for _ in 0..120 {
            crate::synchronize_audio_assets(&mut audio, &snapshot, &catalog).unwrap();
            crate::synchronize_audio_assets(&mut loaded, &snapshot, &loaded_catalog).unwrap();
            let mut a = [[0.; 2]; 7];
            let mut b = a;
            audio.render(&mut a);
            loaded.render(&mut b);
            assert_eq!(
                a.map(|f| f.map(f32::to_bits)),
                b.map(|f| f.map(f32::to_bits))
            );
        }
    }
}
