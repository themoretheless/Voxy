//! Shared input provenance and asset publication integration for scene audio.
use crate::{SceneAudioRuntime, SceneAudioSnapshot};
use std::collections::HashMap;
use voxy_assets::{AssetCatalog, AssetId, ImportInputs, ImportedAsset};
use voxy_audio::{Clip, decode_wav};
#[derive(Clone, Copy, Debug)]
pub struct AudioImportSettings {
    pub target_rate: u32,
    pub max_input_bytes: usize,
    pub max_frames: usize,
    pub max_filter_evaluations: usize,
}
/// Saved logical import-settings asset. Engine limits remain authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioImportConfig {
    pub version: u32,
    pub max_input_bytes: usize,
    pub max_frames: usize,
    pub max_filter_evaluations: usize,
}
impl AudioImportConfig {
    /// Decodes a bounded versioned settings document and validates engine caps.
    /// # Errors
    /// Rejects documents over 4096 bytes, unknown fields/versions and invalid limits.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 4096 {
            return Err("audio settings document byte limit exceeded".into());
        }
        let config: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        config.settings(48000)?;
        Ok(config)
    }
    /// Saves a validated settings document through the common atomic writer.
    /// This performs blocking authoring IO; device rate remains a runtime choice.
    /// # Errors
    /// Rejects invalid settings and filesystem failures, preserving the old file
    /// for every failure before successful rename.
    pub fn save_file(self, path: &std::path::Path) -> Result<(), String> {
        self.settings(48000)?;
        let bytes = serde_json::to_vec_pretty(&self).map_err(|e| e.to_string())?;
        voxy_assets::save_atomic_file(path, &bytes, 4096).map_err(|e| e.to_string())
    }
    /// # Errors
    /// Rejects unknown versions, invalid values or settings exceeding engine caps.
    pub fn settings(self, target_rate: u32) -> Result<AudioImportSettings, String> {
        if self.version != 1
            || self.max_input_bytes == 0
            || self.max_input_bytes > 8 * 1024 * 1024
            || self.max_frames == 0
            || self.max_frames > 480_000
            || self.max_filter_evaluations > 480_000 * 65
        {
            return Err("invalid audio import settings".into());
        }
        Ok(AudioImportSettings {
            target_rate,
            max_input_bytes: self.max_input_bytes,
            max_frames: self.max_frames,
            max_filter_evaluations: self.max_filter_evaluations,
        })
    }
}
/// Imports through the common observation/revalidation boundary, then uses the
/// existing bounded WAV decoder and filtered rate converter.
/// # Errors
/// Rejects input drift, decoding, rate or allocation/work budgets.
pub fn import_wav_asset(
    source: AssetId,
    settings: AudioImportSettings,
    mut read: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
) -> Result<ImportedAsset<Clip>, String> {
    import_wav_with_inputs(
        ImportInputs::new(1, settings.max_input_bytes),
        source,
        settings,
        &mut read,
    )
}
/// Imports WAV while retaining prior observations such as a logical-ID manifest.
/// # Errors
/// Rejects source drift, invalid audio and import budgets.
pub fn import_wav_with_inputs(
    inputs: ImportInputs,
    source: AssetId,
    settings: AudioImportSettings,
    mut read: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
) -> Result<ImportedAsset<Clip>, String> {
    import_wav_cached(inputs, source, settings, None, &mut read)
}
/// Imports through the common artifact cache; invalid cached PCM falls back to WAV.
/// # Errors
/// Rejects source drift, decoding and bounded import settings.
pub fn import_wav_cached(
    mut inputs: ImportInputs,
    source: AssetId,
    settings: AudioImportSettings,
    cache: Option<&voxy_assets::ArtifactCache>,
    mut read: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
) -> Result<ImportedAsset<Clip>, String> {
    let clip = decode_wav_observed(&mut inputs, source, settings, cache, &mut read)?;
    inputs.finish(clip, &mut read).map_err(|e| format!("{e:?}"))
}

/// Decodes a WAV within the caller's observed import transaction.
/// The common import worker owns final dependency revalidation and publication.
/// # Errors
/// Rejects invalid audio, cache payloads and allocation/work budget violations.
pub fn decode_wav_observed(
    inputs: &mut ImportInputs,
    source: AssetId,
    settings: AudioImportSettings,
    cache: Option<&voxy_assets::ArtifactCache>,
    mut read: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
) -> Result<Clip, String> {
    let observed = inputs
        .read(source, &mut read)
        .map_err(|e| format!("{e:?}"))?;
    if observed.bytes.len() > settings.max_input_bytes {
        return Err("audio input exceeds configured limit".into());
    }
    let mut options = settings.target_rate.to_le_bytes().to_vec();
    for limit in [
        settings.max_input_bytes,
        settings.max_frames,
        settings.max_filter_evaluations,
    ] {
        options.extend_from_slice(
            &u64::try_from(limit)
                .map_err(|e| e.to_string())?
                .to_le_bytes(),
        );
    }
    let key = inputs
        .build_key("voxy.wav", "1", "pcm1-filtered65", &options)
        .map_err(|e| format!("{e:?}"))?;
    if let Some(cache) = cache
        && let Ok(Some(bytes)) = cache.load(&key)
        && let Ok(clip) = Clip::from_pcm_bytes(&bytes, settings.max_frames)
        && clip.sample_rate() == settings.target_rate
    {
        return Ok(clip);
    }
    let decoded = decode_wav(&observed.bytes, settings.max_frames).map_err(|e| e.to_string())?;
    let clip = decoded
        .resample_filtered(
            settings.target_rate,
            settings.max_frames,
            settings.max_filter_evaluations,
        )
        .map_err(|e| e.to_string())?;
    if let Some(cache) = cache {
        let bytes = clip.to_pcm_bytes().map_err(|e| e.to_string())?;
        if let Err(error) = cache.store(&key, &bytes) {
            eprintln!("audio cache write: {error}");
        }
    }
    Ok(clip)
}
/// Captures publication revisions and immutable clips in one owner operation;
/// failed/ongoing reloads use the catalog's last-good publication.
/// # Errors
/// Rejects missing publications and scene/mixer errors before playback changes.
pub fn synchronize_audio_assets(
    runtime: &mut SceneAudioRuntime,
    snapshot: &SceneAudioSnapshot,
    catalog: &AssetCatalog<ImportedAsset<Clip>>,
) -> Result<(), String> {
    let mut publications = HashMap::new();
    for source in &snapshot.sources {
        let id = &source.descriptor.asset;
        if !publications.contains_key(id) {
            let publication = catalog
                .snapshot_with_revision(&AssetId(id.clone()))
                .ok_or_else(|| format!("audio asset {id} has no publication"))?;
            publications.insert(id.clone(), publication);
        }
    }
    runtime.synchronize_published(
        snapshot,
        |id| {
            publications
                .get(id)
                .map(|(revision, _)| *revision)
                .ok_or_else(|| "missing captured audio publication".into())
        },
        |id| {
            publications
                .get(id)
                .map(|(_, asset)| asset.value().clone())
                .ok_or_else(|| "missing captured audio clip".into())
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioSource, extract_scene_audio};
    use voxy_scene::{SceneGraph, Transform};
    fn wav(sample: i16) -> Vec<u8> {
        let mut bytes = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0".to_vec();
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48000_u32.to_le_bytes());
        bytes.extend_from_slice(&96000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data\x02\0\0\0");
        bytes.extend_from_slice(&sample.to_le_bytes());
        bytes
    }
    #[test]
    fn common_worker_decodes_wav_and_retains_observations() {
        let root = std::env::temp_dir().join(format!(
            "voxy-wav-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("tone.wav"), wav(8192)).unwrap();
        let owner = std::thread::current().id();
        let mut worker = voxy_assets::AssetImportWorker::new(
            voxy_assets::FileInputs::new(&root).unwrap(),
            1,
            1024,
            move |id, provider, inputs| {
                assert_ne!(std::thread::current().id(), owner);
                decode_wav_observed(
                    inputs,
                    id.clone(),
                    AudioImportSettings {
                        target_rate: 48000,
                        max_input_bytes: 1024,
                        max_frames: 10,
                        max_filter_evaluations: 650,
                    },
                    None,
                    |source, limit| provider.read(source, limit),
                )
            },
        )
        .unwrap();
        let mut catalog: AssetCatalog<ImportedAsset<Clip>> = AssetCatalog::new(1, 1).unwrap();
        let ticket = catalog.request(AssetId("tone.wav".into())).unwrap();
        worker.submit(&ticket).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let completion = loop {
            if let Some(result) = worker.try_result().unwrap() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        };
        let imported = completion.result.unwrap();
        assert_eq!(imported.value().sample_rate(), 48000);
        assert_eq!(imported.inputs().observations().len(), 1);
        worker.close().join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_config_save_round_trip_and_invalid_edit_preserve_previous_file() {
        let root = std::env::temp_dir().join(format!(
            "voxy-settings-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("tone.import.json");
        let config = AudioImportConfig {
            version: 1,
            max_input_bytes: 1024,
            max_frames: 10,
            max_filter_evaluations: 650,
        };
        config.save_file(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        assert_eq!(AudioImportConfig::from_json(&original).unwrap(), config);
        assert!(
            AudioImportConfig {
                max_frames: 0,
                ..config
            }
            .save_file(&path)
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(AudioImportConfig::from_json(&vec![b' '; 4097]).is_err());
        let directory = root.join("directory");
        std::fs::create_dir(&directory).unwrap();
        assert!(config.save_file(&directory).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn saved_import_config_rejects_unknown_version_and_engine_cap_escape() {
        let config = AudioImportConfig {
            version: 1,
            max_input_bytes: 1024,
            max_frames: 10,
            max_filter_evaluations: 650,
        };
        assert_eq!(config.settings(48000).unwrap().max_frames, 10);
        assert!(
            AudioImportConfig {
                version: 2,
                ..config
            }
            .settings(48000)
            .is_err()
        );
        assert!(
            AudioImportConfig {
                max_frames: 480_001,
                ..config
            }
            .settings(48000)
            .is_err()
        );
        assert!(
            AudioImportConfig {
                max_input_bytes: 0,
                ..config
            }
            .settings(48000)
            .is_err()
        );
        let serialized = serde_json::to_vec(&config).unwrap();
        assert_eq!(
            serde_json::from_slice::<AudioImportConfig>(&serialized)
                .unwrap()
                .version,
            1
        );
        assert!(serde_json::from_str::<AudioImportConfig>(r#"{"version":1,"max_input_bytes":1,"max_frames":1,"max_filter_evaluations":0,"unknown":true}"#).is_err());
    }

    #[test]
    fn cached_wav_revalidates_inputs_and_repairs_corrupt_artifact() {
        let root = std::env::temp_dir().join(format!(
            "voxy-audio-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = voxy_assets::ArtifactCache::new(&root, 4096).unwrap();
        let settings = AudioImportSettings {
            target_rate: 48000,
            max_input_bytes: 1024,
            max_frames: 10,
            max_filter_evaluations: 650,
        };
        let import = || {
            import_wav_cached(
                ImportInputs::new(1, 1024),
                AssetId("tone.wav".into()),
                settings,
                Some(&cache),
                |_, _| Ok(wav(8192)),
            )
            .unwrap()
        };
        let first = import();
        let path = std::fs::read_dir(&root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let before = std::fs::read(&path).unwrap();
        let second = import();
        assert_eq!(
            first.value().to_pcm_bytes().unwrap(),
            second.value().to_pcm_bytes().unwrap()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut reads = 0;
        assert!(
            import_wav_cached(
                ImportInputs::new(1, 1024),
                AssetId("tone.wav".into()),
                settings,
                Some(&cache),
                |_, _| {
                    reads += 1;
                    Ok(wav(if reads == 1 { 8192 } else { 16384 }))
                }
            )
            .is_err()
        );
        std::fs::write(&path, b"corrupt").unwrap();
        let repaired = import();
        assert_eq!(
            repaired.value().to_pcm_bytes().unwrap(),
            first.value().to_pcm_bytes().unwrap()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn common_catalog_retains_failed_publication_and_scene_uses_successful_replacement() {
        let settings = AudioImportSettings {
            target_rate: 48000,
            max_input_bytes: 1024,
            max_frames: 10,
            max_filter_evaluations: 650,
        };
        let id = AssetId("tone".into());
        let source = AssetId("tone.wav".into());
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let ticket = catalog.request(id.clone()).unwrap();
        catalog
            .complete(
                &ticket,
                import_wav_asset(source.clone(), settings, |_, _| Ok(wav(8192))),
            )
            .unwrap();
        let revision = catalog.snapshot_with_revision(&id).unwrap().0;
        let mut scene = SceneGraph::new(1);
        let node = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                node,
                AudioSource {
                    import_settings: None,
                    asset: id.0.clone(),
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
        let mut runtime = SceneAudioRuntime::new(48000, 1, 1).unwrap();
        synchronize_audio_assets(&mut runtime, &snapshot, &catalog).unwrap();
        let broken = catalog.request(id.clone()).unwrap();
        catalog
            .complete(
                &broken,
                import_wav_asset(source.clone(), settings, |_, _| Ok(vec![0; 8])),
            )
            .unwrap();
        assert_eq!(catalog.snapshot_with_revision(&id).unwrap().0, revision);
        synchronize_audio_assets(&mut runtime, &snapshot, &catalog).unwrap();
        let mut out = [[0.; 2]; 1];
        runtime.render(&mut out);
        assert!((out[0][0] - 0.25).abs() < 1e-6);
        let replacement = catalog.request(id.clone()).unwrap();
        catalog
            .complete(
                &replacement,
                import_wav_asset(source.clone(), settings, |_, _| Ok(wav(16384))),
            )
            .unwrap();
        assert_ne!(catalog.snapshot_with_revision(&id).unwrap().0, revision);
        synchronize_audio_assets(&mut runtime, &snapshot, &catalog).unwrap();
        runtime.render(&mut out);
        assert!((out[0][0] - 0.5).abs() < 1e-6);
        let mut reads = 0;
        assert!(
            import_wav_asset(source, settings, |_, _| {
                reads += 1;
                Ok(wav(if reads == 1 { 8192 } else { 16384 }))
            })
            .is_err()
        );
    }
}
