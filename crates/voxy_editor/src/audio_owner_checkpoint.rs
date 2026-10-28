//! Audio playback and reload composition. Native callback queues remain a required gate.
use super::playback_checkpoint::AudioPlaybackCheckpoint;
use super::*;
use crate::audio_reload::checkpoint::AudioReloadCheckpoint;
use serde::{Deserialize, Serialize};
use voxy_scene::{NodeId, ObjectId};
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioOwnerCheckpoint {
    version: u32,
    playback: AudioPlaybackCheckpoint,
    reload: Option<AudioReloadCheckpoint>,
}
#[allow(dead_code)]
impl AudioPlay {
    pub(crate) fn begin_owner_checkpoint(&mut self) -> Result<(), EditorError> {
        if self.output.is_some() {
            return Err("native device save barrier not established".into());
        }
        if let Some(reload) = &mut self.reload {
            reload.begin_checkpoint()?;
        }
        Ok(())
    }
    pub(crate) fn begin_owner_checkpoint_with_output(&mut self) -> Result<(), EditorError> {
        if self.output.is_none() {
            return Err("missing native output owner".into());
        }
        if let Some(reload) = &mut self.reload {
            reload.begin_checkpoint()?;
        }
        Ok(())
    }
    pub(crate) fn poll_owner_checkpoint_with_output(
        &mut self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
        frozen: &mut voxy_audio_device::FrozenOutputQueue,
    ) -> Result<Option<AudioOwnerCheckpoint>, EditorError> {
        let reload = if let Some(reload) = &mut self.reload {
            let Some(saved) = reload.poll_checkpoint(&mut self.catalog)? else {
                return Ok(None);
            };
            Some(saved)
        } else {
            None
        };
        let Some(playback) =
            self.checkpoint_playback_with_output_queue(identify, max_bytes, frozen)?
        else {
            return Ok(None);
        };
        let saved = AudioOwnerCheckpoint {
            version: 1,
            playback,
            reload,
        };
        saved.to_json(max_bytes)?;
        Ok(Some(saved))
    }
    pub(crate) fn poll_owner_checkpoint(
        &mut self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<Option<AudioOwnerCheckpoint>, EditorError> {
        let reload = if let Some(reload) = &mut self.reload {
            let Some(saved) = reload.poll_checkpoint(&mut self.catalog)? else {
                return Ok(None);
            };
            Some(saved)
        } else {
            None
        };
        let saved = AudioOwnerCheckpoint {
            version: 1,
            playback: self.checkpoint_playback(identify, max_bytes)?,
            reload,
        };
        saved.to_json(max_bytes)?;
        Ok(Some(saved))
    }
    pub(crate) fn resume_owner_checkpoint(&mut self) -> Result<bool, EditorError> {
        self.reload
            .as_mut()
            .map_or(Ok(true), |reload| reload.resume_after_checkpoint())
    }
}
#[allow(dead_code)]
impl AudioOwnerCheckpoint {
    pub(crate) fn playback_rate(&self) -> u32 {
        self.playback.playback_rate()
    }
    pub(crate) fn native_output_rate(&self) -> Option<u32> {
        self.playback.native_output_rate()
    }
    pub(crate) fn validate_native_output(&self, max_bytes: usize) -> Result<(), EditorError> {
        if self.version != 1 {
            return Err("unsupported audio owner checkpoint".into());
        }
        self.playback.validate_native_output(max_bytes)
    }
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("audio owner checkpoint byte budget".into());
        }
        Ok(json)
    }
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("audio owner checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    /// Stage everything before returning a complete offline-output AudioPlay.
    /// Factories are supplied by the host to preserve project-root ownership.
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
        make_worker: impl FnOnce(
            u32,
            std::collections::BTreeMap<AssetId, Option<String>>,
        ) -> Result<voxy_assets::AssetImportWorker<Clip>, EditorError>,
        make_watcher: impl FnOnce(
            &voxy_assets::SourcePollerCheckpoint,
        ) -> Result<voxy_assets::SourcePollWorker, EditorError>,
    ) -> Result<AudioPlay, EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 {
            return Err("unsupported audio owner checkpoint".into());
        }
        let prepared = self.playback.prepare(scene, resolve, max_bytes)?;
        self.prepare_playback(prepared, make_worker, make_watcher)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_native(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
        connection: &voxy_audio_device::OutputConnection,
        make_worker: impl FnOnce(
            u32,
            std::collections::BTreeMap<AssetId, Option<String>>,
        ) -> Result<voxy_assets::AssetImportWorker<Clip>, EditorError>,
        make_watcher: impl FnOnce(
            &voxy_assets::SourcePollerCheckpoint,
        ) -> Result<voxy_assets::SourcePollWorker, EditorError>,
    ) -> Result<(AudioPlay, voxy_audio_device::PreparedOutputCheckpoint), EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 {
            return Err("unsupported audio owner checkpoint".into());
        }
        let (prepared, output) = self
            .playback
            .prepare_native(scene, resolve, max_bytes, connection)?;
        Ok((
            self.prepare_playback(prepared, make_worker, make_watcher)?,
            output,
        ))
    }
    fn prepare_playback(
        &self,
        prepared: super::playback_checkpoint::PreparedAudioPlayback,
        make_worker: impl FnOnce(
            u32,
            std::collections::BTreeMap<AssetId, Option<String>>,
        ) -> Result<voxy_assets::AssetImportWorker<Clip>, EditorError>,
        make_watcher: impl FnOnce(
            &voxy_assets::SourcePollerCheckpoint,
        ) -> Result<voxy_assets::SourcePollWorker, EditorError>,
    ) -> Result<AudioPlay, EditorError> {
        let rate = prepared.clock.sample_rate();
        let reload = if let Some(saved) = &self.reload {
            Some(saved.prepare(
                &prepared.catalog,
                || make_worker(rate, saved.recipes.clone()),
                make_watcher,
            )?)
        } else {
            None
        };
        let mut play = AudioPlay {
            catalog: prepared.catalog,
            runtime: prepared.runtime,
            reload,
            output: None,
            offline_output: prepared.offline_output,
            rate,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        // Clock validation already succeeded during playback preparation.
        if let Err(error) = play.restore_clock(&prepared.clock) {
            drop(play.close());
            return Err(error);
        }
        Ok(play)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputRecipe, audio_reload::AudioReload};
    use voxy_assets::{AssetImportWorker, FileInputs, ImportInputs, SourcePollWorker};
    use voxy_gameplay::AudioSource;
    use voxy_scene::Transform;
    fn worker(path: &std::path::Path) -> Result<AssetImportWorker<Clip>, EditorError> {
        Ok(AssetImportWorker::new(
            FileInputs::new(path)?,
            1,
            4,
            |_, provider, inputs| {
                let input = inputs
                    .read(AssetId("source.wav".into()), |id, max| {
                        provider.read(id, max)
                    })
                    .map_err(|e| format!("{e:?}"))?;
                Clip::new(48000, vec![[f32::from(input.bytes[0]) / 255.; 2]; 3])
                    .map_err(|e| e.to_string())
            },
        )?)
    }
    #[test]
    fn whole_offline_audio_record_restores_playback_and_queued_reload_from_json() {
        let path = std::env::temp_dir().join(format!(
            "voxy-owner-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        std::fs::write(path.join("source.wav"), [1]).unwrap();
        let project = AuthoringProject::new(&path, &InputRecipe::Direct).unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let source = AudioSource {
            asset: "tone".into(),
            import_settings: None,
            bus: 0,
            gain: 0.8,
            looping: true,
            spatial: false,
            near: 1.,
            far: 5.,
        };
        scene.insert_component(owner, source.clone()).unwrap();
        let mut inputs = ImportInputs::new(1, 4);
        inputs
            .read(AssetId("source.wav".into()), |_, _| Ok(vec![1]))
            .unwrap();
        let imported = inputs
            .finish(Clip::new(48000, vec![[0.1; 2]; 3]).unwrap(), |_, _| {
                Ok(vec![1])
            })
            .unwrap();
        let mut catalog = AssetCatalog::new(128, 128).unwrap();
        let id = AssetId("tone".into());
        let ticket = catalog.request(id.clone()).unwrap();
        catalog.complete(&ticket, Ok(imported)).unwrap();
        let recipes = std::collections::BTreeMap::from([(id.clone(), None)]);
        let reload =
            AudioReload::new(&project, worker(&path).unwrap(), recipes.clone(), &catalog).unwrap();
        let mut original = AudioPlay {
            catalog,
            runtime: SceneAudioRuntime::new(48000, 128, 16).unwrap(),
            reload: Some(reload),
            output: None,
            offline_output: None,
            rate: 48000,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        original.advance(&scene, 1).unwrap();
        std::fs::write(path.join("source.wav"), [7]).unwrap();
        original.poll_reload(&scene).unwrap(); // Schedule a scan before the save fence.
        original.begin_owner_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let saved = loop {
            if let Some(saved) = original
                .poll_owner_checkpoint(|_| Some(ObjectId("source".into())), 1_000_000)
                .unwrap()
            {
                break saved;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        };
        let json = saved.to_json(1_000_000).unwrap();
        let decoded = AudioOwnerCheckpoint::from_json(&json, 1_000_000).unwrap();
        assert_eq!(decoded.to_json(1_000_000).unwrap(), json);
        let mut fresh_scene = SceneGraph::new(2);
        fresh_scene.spawn(None, Transform::default()).unwrap();
        let fresh = fresh_scene.spawn(None, Transform::default()).unwrap();
        fresh_scene.insert_component(fresh, source).unwrap();
        let mut loaded = decoded
            .prepare(
                &fresh_scene,
                |_| Some(fresh),
                1_000_000,
                |rate, restored_recipes| {
                    assert_eq!(rate, 48000);
                    assert_eq!(restored_recipes, recipes);
                    worker(&path)
                },
                |baseline| {
                    Ok(SourcePollWorker::new_with_checkpoint(
                        FileInputs::new(&path)?,
                        baseline,
                        768,
                        8 * 1024 * 1024,
                    )?)
                },
            )
            .unwrap();
        assert!(original.resume_owner_checkpoint().unwrap());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            original.poll_reload(&scene).unwrap();
            loaded.poll_reload(&fresh_scene).unwrap();
            if matches!(
                original.catalog.status(&id),
                Some(voxy_assets::AssetStatus::Ready)
            ) && matches!(
                loaded.catalog.status(&id),
                Some(voxy_assets::AssetStatus::Ready)
            ) {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            original.catalog.snapshot_with_revision(&id).unwrap().0,
            loaded.catalog.snapshot_with_revision(&id).unwrap().0
        );
        for step in 0..120 {
            let count = [0, 1, 2, 5][step % 4];
            original.advance(&scene, count).unwrap();
            loaded.advance(&fresh_scene, count).unwrap();
            assert_eq!(
                original
                    .checkpoint_playback(|_| Some(ObjectId("source".into())), 1_000_000)
                    .unwrap()
                    .to_json(1_000_000)
                    .unwrap(),
                loaded
                    .checkpoint_playback(|_| Some(ObjectId("source".into())), 1_000_000)
                    .unwrap()
                    .to_json(1_000_000)
                    .unwrap()
            );
        }
        let mut duplicate: serde_json::Value = serde_json::from_str(&json).unwrap();
        let entries = duplicate["reload"]["recipes"].as_array_mut().unwrap();
        entries.push(entries[0].clone());
        assert!(
            AudioOwnerCheckpoint::from_json(&serde_json::to_string(&duplicate).unwrap(), 1_000_000)
                .is_err()
        );
        assert!(AudioOwnerCheckpoint::from_json(&json, 1).is_err());
        let mut bad = decoded;
        bad.version = 2;
        assert!(
            bad.prepare(
                &fresh_scene,
                |_| Some(fresh),
                1_000_000,
                |_, _| panic!("must validate before starting worker"),
                |_| panic!("must validate before starting watcher")
            )
            .is_err()
        );
        for join in original.close().into_iter().chain(loaded.close()) {
            join.join().unwrap();
        }
    }
}
