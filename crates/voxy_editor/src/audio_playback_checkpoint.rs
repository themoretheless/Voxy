//! Composite playback participant. Reload jobs and native device queues have separate owners.
use super::*;
use crate::asset_checkpoint_wire::CatalogWire;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::{AssetCatalogCheckpoint, ImportedArchiveLimits, InputError};
use voxy_gameplay::{SceneAudioCheckpoint, SceneAudioCheckpointLimits};
use voxy_scene::{NodeId, ObjectId};
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AudioPlaybackCheckpoint {
    version: u32,
    catalog: CatalogWire,
    imported: Vec<Vec<u8>>,
    pcm: Vec<Vec<u8>>,
    runtime: SceneAudioCheckpoint,
    clock: AudioClockCheckpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output: Option<Vec<u8>>,
}
#[allow(dead_code)]
pub(super) struct PreparedAudioPlayback {
    pub(super) catalog: AssetCatalog<ImportedAsset<Clip>>,
    pub(super) runtime: SceneAudioRuntime,
    pub(super) clock: AudioClockCheckpoint,
    pub(super) offline_output: Option<voxy_audio_device::OfflineOutput>,
}
fn output_limits(rate: u32, max_bytes: usize) -> voxy_audio_device::OutputCheckpointLimits {
    voxy_audio_device::OutputCheckpointLimits {
        max_bytes,
        max_queue_frames: 4096,
        max_pump_frames: 256,
        sample_rate: rate,
    }
}
pub(crate) fn empty_native_output(
    connection: &voxy_audio_device::OutputConnection,
    max_bytes: usize,
) -> Result<
    (
        voxy_audio_device::PcmPump,
        voxy_audio_device::PreparedOutputCheckpoint,
    ),
    EditorError,
> {
    let pump = voxy_audio_device::PcmPump::new(256).map_err(|e| e.to_string())?;
    let output = voxy_audio_device::OutputCheckpoint {
        sample_rate: connection.sample_rate(),
        queue: voxy_audio_device::OutputQueueCheckpoint {
            capacity: connection.sender().capacity(),
            frames: Vec::new(),
        },
        pump: pump.checkpoint(),
    }
    .prepare(output_limits(connection.sample_rate(), max_bytes))?;
    Ok((pump, output))
}
fn archive_limits(max_bytes: usize) -> ImportedArchiveLimits {
    ImportedArchiveLimits {
        max_inputs: 768,
        max_input_bytes: 16 * 1024 * 1024,
        max_archive_bytes: max_bytes,
    }
}
fn input_error(error: impl std::fmt::Display) -> InputError {
    InputError::Read(error.to_string())
}
#[allow(dead_code)]
impl AudioPlay {
    pub(super) fn checkpoint_playback(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<AudioPlaybackCheckpoint, EditorError> {
        if self.output.is_some() {
            return Err("native audio checkpoint requires a device queue save barrier".into());
        }
        self.checkpoint_playback_parts(identify, max_bytes)
    }
    /// Logical playback/queue composition. The enclosing native owner must also
    /// admit device pause, reload fences and hardware publication before use.
    pub(super) fn checkpoint_playback_with_output_queue(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
        frozen: &mut voxy_audio_device::FrozenOutputQueue,
    ) -> Result<Option<AudioPlaybackCheckpoint>, EditorError> {
        let (connection, pump) = self.output.as_ref().ok_or("missing native audio output")?;
        let limits = output_limits(self.rate, max_bytes);
        let Some(output) =
            voxy_audio_device::OutputCheckpoint::capture(connection, frozen, pump, limits)?
        else {
            return Ok(None);
        };
        let mut saved = self.checkpoint_playback_parts(identify, max_bytes)?;
        saved.output = Some(output.to_bytes(limits)?);
        saved.to_json(max_bytes)?;
        Ok(Some(saved))
    }
    fn checkpoint_playback_parts(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<AudioPlaybackCheckpoint, EditorError> {
        let mut publications: Vec<Arc<ImportedAsset<Clip>>> = Vec::new();
        let catalog = self
            .catalog
            .checkpoint(|asset| {
                let index = publications
                    .iter()
                    .position(|prior| Arc::ptr_eq(prior, asset))
                    .unwrap_or_else(|| {
                        publications.push(asset.clone());
                        publications.len() - 1
                    });
                Some(index.to_string())
            })?
            .try_into()?;
        let mut imported = Vec::new();
        for publication in publications {
            imported.push(
                publication
                    .to_checkpoint_bytes(
                        |clip| clip.to_pcm_bytes().map_err(input_error),
                        archive_limits(max_bytes),
                    )
                    .map_err(|e| format!("audio import archive: {e:?}"))?,
            );
        }
        let mut clips: Vec<Clip> = Vec::new();
        let runtime = self.runtime.checkpoint(
            identify,
            |clip| {
                let index = clips
                    .iter()
                    .position(|prior| prior.shares_pcm(clip))
                    .unwrap_or_else(|| {
                        clips.push(clip.clone());
                        clips.len() - 1
                    });
                Some(index.to_string())
            },
            max_bytes,
        )?;
        let pcm = clips
            .iter()
            .map(Clip::to_pcm_bytes)
            .collect::<Result<_, _>>()?;
        let saved = AudioPlaybackCheckpoint {
            version: 1,
            catalog,
            imported,
            pcm,
            runtime,
            clock: self.checkpoint_clock(),
            output: self
                .offline_output
                .as_ref()
                .map(|output| {
                    output
                        .checkpoint()
                        .to_bytes(output_limits(self.rate, max_bytes))
                })
                .transpose()?,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl AudioPlaybackCheckpoint {
    pub(super) fn playback_rate(&self) -> u32 {
        self.clock.sample_rate()
    }
    pub(super) fn native_output_rate(&self) -> Option<u32> {
        self.output.as_ref().map(|_| self.clock.sample_rate())
    }
    pub(super) fn validate_native_output(&self, max_bytes: usize) -> Result<(), EditorError> {
        self.prepare_output(max_bytes).map(|_| ())
    }
    pub(super) fn prepare_output(
        &self,
        max_bytes: usize,
    ) -> Result<Option<voxy_audio_device::PreparedOutputCheckpoint>, EditorError> {
        if self.version != 1 {
            return Err("unsupported audio playback checkpoint".into());
        }
        self.to_json(max_bytes)?;
        self.prepare_output_payload(max_bytes)
    }
    fn prepare_output_payload(
        &self,
        max_bytes: usize,
    ) -> Result<Option<voxy_audio_device::PreparedOutputCheckpoint>, EditorError> {
        if self.version != 1 {
            return Err("unsupported audio playback checkpoint".into());
        }
        self.clock.sample_budget(0)?;
        self.output
            .as_ref()
            .map(|bytes| {
                let limits = output_limits(self.clock.sample_rate(), max_bytes);
                let output = voxy_audio_device::OutputCheckpoint::from_bytes(bytes, limits)?;
                Ok(output.prepare(limits)?)
            })
            .transpose()
    }
    pub(super) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("audio playback checkpoint byte budget".into());
        }
        Ok(json)
    }
    pub(super) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("audio playback checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    pub(super) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<PreparedAudioPlayback, EditorError> {
        self.to_json(max_bytes)?;
        self.clock.sample_budget(0)?;
        let pending = self
            .output
            .as_ref()
            .map(|bytes| {
                let limits = output_limits(self.clock.sample_rate(), max_bytes);
                voxy_audio_device::OutputCheckpoint::from_bytes(bytes, limits)?
                    .prepare_offline(limits)
            })
            .transpose()?;
        let mut prepared = self.prepare_parts(scene, resolve, max_bytes)?;
        prepared.offline_output = pending;
        Ok(prepared)
    }
    pub(super) fn prepare_native(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
        connection: &voxy_audio_device::OutputConnection,
    ) -> Result<
        (
            PreparedAudioPlayback,
            voxy_audio_device::PreparedOutputCheckpoint,
        ),
        EditorError,
    > {
        self.clock.sample_budget(0)?;
        if self.clock.sample_rate() != connection.sample_rate() {
            return Err(
                "native restoration sample rate mismatch; resampling is not permitted".into(),
            );
        }
        let output = match self.prepare_output_payload(max_bytes)? {
            Some(output) => output,
            None => empty_native_output(connection, max_bytes)?.1,
        };
        output.validate_connection(connection)?;
        Ok((self.prepare_parts(scene, resolve, max_bytes)?, output))
    }
    fn prepare_parts(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<PreparedAudioPlayback, EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 || self.imported.len() > 128 || self.pcm.len() > 128 {
            return Err("audio playback checkpoint version or capacity".into());
        }
        self.clock.sample_budget(0)?;
        let mut publications = BTreeMap::new();
        let mut total_frames = 0_usize;
        let mut total_inputs = 0_usize;
        for (index, bytes) in self.imported.iter().enumerate() {
            let imported = ImportedAsset::from_checkpoint_bytes(
                bytes,
                |pcm| Clip::from_pcm_bytes(pcm, 480000).map_err(input_error),
                archive_limits(max_bytes),
            )
            .map_err(|e| format!("audio publication archive: {e:?}"))?;
            total_frames = total_frames
                .checked_add(imported.value().frame_count())
                .ok_or("audio archive frame overflow")?;
            total_inputs = total_inputs
                .checked_add(imported.inputs().captured_bytes())
                .ok_or("audio archive input overflow")?;
            if total_frames > 480000
                || total_inputs > 16 * 1024 * 1024
                || imported.value().sample_rate() != self.clock.sample_rate()
            {
                return Err("audio publication aggregate budget or rate".into());
            }
            publications.insert(index.to_string(), Arc::new(imported));
        }
        let catalog_record: AssetCatalogCheckpoint = self.catalog.clone().into();
        let mut used = std::collections::BTreeSet::new();
        let catalog = catalog_record.prepare(
            |key| {
                used.insert(key.to_string());
                publications.get(key).cloned()
            },
            128,
            128,
        )?;
        if used.len() != publications.len() {
            return Err("unreferenced audio publication archive".into());
        }
        let mut clips = BTreeMap::new();
        let mut runtime_frames = 0_usize;
        for (index, bytes) in self.pcm.iter().enumerate() {
            let clip = Clip::from_pcm_bytes(bytes, 480000)?;
            runtime_frames = runtime_frames
                .checked_add(clip.frame_count())
                .ok_or("audio runtime archive overflow")?;
            if runtime_frames > 480000 {
                return Err("audio runtime archive budget".into());
            }
            clips.insert(index.to_string(), clip);
        }
        let mut used = std::collections::BTreeSet::new();
        let runtime = self.runtime.prepare(
            scene,
            resolve,
            |key| {
                used.insert(key.to_string());
                clips.get(key).cloned()
            },
            SceneAudioCheckpointLimits {
                sample_rate: self.clock.sample_rate(),
                max_sources: 128,
                max_buses: 16,
                max_bytes,
            },
        )?;
        if used.len() != clips.len() {
            return Err("unreferenced runtime PCM archive".into());
        }
        Ok(PreparedAudioPlayback {
            catalog,
            runtime,
            clock: self.clock.clone(),
            offline_output: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_assets::ImportInputs;
    use voxy_gameplay::AudioSource;
    use voxy_scene::Transform;
    fn imported(value: f32) -> ImportedAsset<Clip> {
        let mut inputs = ImportInputs::new(1, 100);
        inputs
            .read(AssetId("tone.source".into()), |_, _| {
                Ok(value.to_bits().to_le_bytes().to_vec())
            })
            .unwrap();
        inputs
            .finish(
                Clip::new(48000, vec![[value; 2], [-value; 2], [value * 0.5; 2]]).unwrap(),
                |_, _| Ok(value.to_bits().to_le_bytes().to_vec()),
            )
            .unwrap()
    }
    #[test]
    fn offline_restored_prefix_is_consumed_before_mixer_and_survives_partial_save() {
        let scene = SceneGraph::new(1);
        let limits = output_limits(48000, 1_000_000);
        let record = voxy_audio_device::OutputCheckpoint {
            sample_rate: 48000,
            queue: voxy_audio_device::OutputQueueCheckpoint {
                capacity: 1024,
                frames: vec![[0x3f00_0000, 0x8000_0000]; 1000],
            },
            pump: voxy_audio_device::PcmPumpCheckpoint {
                block: vec![[1, 2]; 64],
                cursor: 14,
            },
        };
        let mut original = AudioPlay {
            catalog: AssetCatalog::new(128, 128).unwrap(),
            runtime: SceneAudioRuntime::new(48000, 128, 16).unwrap(),
            reload: None,
            output: None,
            offline_output: Some(record.prepare_offline(limits).unwrap()),
            rate: 48000,
            remainder: 0,
            frames: 1050,
            peak: 0.5,
        };
        original.advance(&scene, 1).unwrap();
        assert_eq!(original.frames, 1050); // 800 already mixed frames consumed, none mixed twice.
        assert_eq!(
            original.offline_output.as_ref().unwrap().remaining_frames(),
            250
        );
        let saved = original.checkpoint_playback(|_| None, 1_000_000).unwrap();
        let expected = original.offline_output.as_ref().unwrap().checkpoint();
        let bytes = saved.to_json(1_000_000).unwrap();
        let saved = AudioPlaybackCheckpoint::from_json(&bytes, 1_000_000).unwrap();
        let prepared = saved.prepare(&scene, |_| None, 1_000_000).unwrap();
        let mut restored = AudioPlay {
            catalog: prepared.catalog,
            runtime: prepared.runtime,
            reload: None,
            output: None,
            offline_output: prepared.offline_output,
            rate: 48000,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        restored.restore_clock(&prepared.clock).unwrap();
        assert_eq!(
            restored.offline_output.as_ref().unwrap().checkpoint(),
            expected
        );
        for audio in [&mut original, &mut restored] {
            audio.advance(&scene, 1).unwrap();
            assert_eq!(audio.frames, 1600); // 250 queued frames plus exactly 550 new frames.
            assert_eq!(audio.offline_output.as_ref().unwrap().remaining_frames(), 0);
        }
        assert_eq!(
            original
                .checkpoint_playback(|_| None, 1_000_000)
                .unwrap()
                .to_json(1_000_000)
                .unwrap(),
            restored
                .checkpoint_playback(|_| None, 1_000_000)
                .unwrap()
                .to_json(1_000_000)
                .unwrap()
        );
    }
    #[test]
    fn playback_archive_preserves_old_voice_new_publication_and_failed_reload_together() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                AudioSource {
                    asset: "tone".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 0.7,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        let mut catalog = AssetCatalog::new(128, 128).unwrap();
        let id = AssetId("tone".into());
        let ticket = catalog.request(id.clone()).unwrap();
        catalog.complete(&ticket, Ok(imported(0.3))).unwrap();
        let mut original = AudioPlay {
            catalog,
            runtime: SceneAudioRuntime::new(48000, 128, 16).unwrap(),
            reload: None,
            output: None,
            offline_output: None,
            rate: 48000,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        original.advance(&scene, 1).unwrap();
        // Catalog publication changes before the playback synchronization boundary.
        let ticket = original.catalog.request(id.clone()).unwrap();
        original
            .catalog
            .complete(&ticket, Ok(imported(0.8)))
            .unwrap();
        let ticket = original.catalog.request(id.clone()).unwrap();
        original
            .catalog
            .complete(&ticket, Err("bad subsequent reload".into()))
            .unwrap();
        let identify = |_| Some(ObjectId("source".into()));
        let saved = original.checkpoint_playback(identify, 1_000_000).unwrap();
        let json = saved.to_json(1_000_000).unwrap();
        let decoded = AudioPlaybackCheckpoint::from_json(&json, 1_000_000).unwrap();
        let mut fresh_scene = SceneGraph::new(2);
        fresh_scene.spawn(None, Transform::default()).unwrap();
        let fresh_owner = fresh_scene.spawn(None, Transform::default()).unwrap();
        fresh_scene
            .insert_component(
                fresh_owner,
                scene
                    .component::<AudioSource>(owner)
                    .unwrap()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        let prepared = decoded
            .prepare(&fresh_scene, |_| Some(fresh_owner), 1_000_000)
            .unwrap();
        let mut loaded = AudioPlay {
            catalog: prepared.catalog,
            runtime: prepared.runtime,
            reload: None,
            output: None,
            offline_output: None,
            rate: 48000,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        loaded.restore_clock(&prepared.clock).unwrap();
        assert_eq!(
            loaded
                .checkpoint_playback(identify, 1_000_000)
                .unwrap()
                .to_json(1_000_000)
                .unwrap(),
            json
        );
        let mut a = [[0.; 2]; 7];
        let mut b = a;
        original.runtime.render(&mut a);
        loaded.runtime.render(&mut b);
        assert_eq!(
            a.map(|f| f.map(f32::to_bits)),
            b.map(|f| f.map(f32::to_bits))
        );
        for frame in 0..120 {
            let count = [0, 1, 2, 5][frame % 4];
            original.advance(&scene, count).unwrap();
            loaded.advance(&fresh_scene, count).unwrap();
            assert_eq!(
                original
                    .checkpoint_playback(identify, 1_000_000)
                    .unwrap()
                    .to_json(1_000_000)
                    .unwrap(),
                loaded
                    .checkpoint_playback(identify, 1_000_000)
                    .unwrap()
                    .to_json(1_000_000)
                    .unwrap()
            );
        }
        let mut bad = saved.clone();
        let last = bad.imported[0].len() - 1;
        bad.imported[0][last] ^= 1;
        assert!(bad.prepare(&scene, |_| Some(owner), 1_000_000).is_err());
        assert!(saved.prepare(&scene, |_| None, 1_000_000).is_err());
        assert!(AudioPlaybackCheckpoint::from_json(&json, 1).is_err());
        assert!(!json.contains("\"output\"")); // Legacy offline wire stays unchanged.
        let wire = voxy_audio_device::OutputCheckpoint {
            sample_rate: 48000,
            queue: voxy_audio_device::OutputQueueCheckpoint {
                capacity: 4,
                frames: vec![[(-0.0_f32).to_bits(), 1]],
            },
            pump: voxy_audio_device::PcmPumpCheckpoint {
                block: vec![[0.25_f32.to_bits(); 2]; 4],
                cursor: 2,
            },
        };
        let mut native = saved.clone();
        native.output = Some(wire.to_bytes(output_limits(48000, 1000000)).unwrap());
        let native_json = native.to_json(1000000).unwrap();
        let decoded = AudioPlaybackCheckpoint::from_json(&native_json, 1000000).unwrap();
        assert_eq!(decoded.to_json(1000000).unwrap(), native_json);
        assert!(decoded.prepare_output(1000000).unwrap().is_some());
        let offline = decoded.prepare(&scene, |_| Some(owner), 1000000).unwrap();
        assert_eq!(offline.offline_output.as_ref().unwrap().checkpoint(), wire);
        let mut corrupted = decoded;
        corrupted.output.as_mut().unwrap()[80] ^= 1;
        assert!(corrupted.prepare_output(1000000).is_err());
        assert!(corrupted.prepare(&scene, |_| Some(owner), 1000000).is_err());
        assert_eq!(saved.to_json(1000000).unwrap(), json);
    }
}
