//! Play-session audio owner with optional native output.
use super::prefab_authoring::AuthoringProject;
use crate::EditorError;
use voxy_assets::{AssetCatalog, AssetId, ImportedAsset};
use voxy_audio::Clip;
use voxy_gameplay::{SceneAudioRuntime, extract_scene_audio, synchronize_audio_assets};
use voxy_scene::SceneGraph;
/// Frame audio follows fixed simulation and reads its final scene state.
pub(super) fn schedule() -> Result<&'static voxy_scene::SchedulePlan, voxy_scene::ScheduleError> {
    static PLAN: std::sync::OnceLock<Result<voxy_scene::SchedulePlan, voxy_scene::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        use voxy_scene::{SystemAccess, SystemSpec};
        voxy_scene::SchedulePlan::build(
            &[SystemSpec {
                name: "audio.advance".into(),
                phase: 0,
                after: vec![],
                access: vec![
                    SystemAccess {
                        resource: "scene".into(),
                        write: false,
                    },
                    SystemAccess {
                        resource: "audio.playback".into(),
                        write: true,
                    },
                ],
            }],
            1,
        )
    })
    .as_ref()
    .map_err(Clone::clone)
}
#[derive(Debug, Default, PartialEq)]
pub(super) struct AudioBudget {
    input_bytes: usize,
    frames: usize,
}
impl AudioBudget {
    pub(super) fn admit(&mut self, input_bytes: usize, frames: usize) -> Result<(), EditorError> {
        let next_bytes = self
            .input_bytes
            .checked_add(input_bytes)
            .ok_or("audio input budget overflow")?;
        let next_frames = self
            .frames
            .checked_add(frames)
            .ok_or("audio frame budget overflow")?;
        if next_bytes > 16 * 1024 * 1024 {
            return Err("play audio retained input budget exceeded".into());
        }
        if next_frames > 480_000 {
            return Err("play audio PCM budget exceeded".into());
        }
        self.input_bytes = next_bytes;
        self.frames = next_frames;
        Ok(())
    }
}
type AudioRecipes = std::collections::BTreeMap<AssetId, Option<String>>;
fn recipes(scene: &SceneGraph) -> Result<AudioRecipes, EditorError> {
    let snapshot = extract_scene_audio(scene, 128)?;
    let mut result = AudioRecipes::new();
    for source in snapshot.sources {
        if source.descriptor.bus >= 16 {
            return Err("play audio bus must be below 16".into());
        }
        if result
            .insert(
                AssetId(source.descriptor.asset),
                source.descriptor.import_settings.clone(),
            )
            .is_some_and(|previous| previous != source.descriptor.import_settings)
        {
            return Err("conflicting audio settings for one logical asset".into());
        }
    }
    Ok(result)
}
/// One common import worker and catalog; no graph or native stream crosses threads.
#[derive(Debug)]
pub(super) struct AudioPreparation {
    worker: voxy_assets::AssetImportWorker<Clip>,
    recipes: AudioRecipes,
    queue: std::collections::VecDeque<AssetId>,
    catalog: AssetCatalog<ImportedAsset<Clip>>,
    budget: AudioBudget,
    observations: std::collections::BTreeMap<AssetId, voxy_assets::InputSnapshot>,
    pending: bool,
}
impl AudioPreparation {
    pub(super) fn new(
        scene: &SceneGraph,
        project: &AuthoringProject,
        rate: u32,
    ) -> Result<Self, EditorError> {
        let recipes = recipes(scene)?;
        Ok(Self {
            worker: project.audio_worker(rate, recipes.clone())?,
            queue: recipes.keys().cloned().collect(),
            recipes,
            catalog: AssetCatalog::new(128, 128)?,
            budget: AudioBudget::default(),
            observations: std::collections::BTreeMap::new(),
            pending: false,
        })
    }
    pub(super) fn matches(&self, scene: &SceneGraph) -> Result<bool, EditorError> {
        Ok(self.recipes == recipes(scene)?)
    }
    pub(super) fn poll(&mut self) -> Result<bool, EditorError> {
        if self.pending {
            let Some(completion) = self.worker.try_result()? else {
                return Ok(false);
            };
            self.pending = false;
            let imported = completion.result.map_err(|e| e.error)?;
            let bytes =
                imported
                    .inputs()
                    .observations()
                    .values()
                    .try_fold(0_usize, |total, input| {
                        let input = input.as_ref().map_err(|e| format!("audio input: {e:?}"))?;
                        total
                            .checked_add(input.bytes.len())
                            .ok_or_else(|| "audio input budget overflow".to_string())
                    })?;
            self.budget.admit(bytes, imported.value().frame_count())?;
            for (id, input) in imported.inputs().observations() {
                let input = input.as_ref().map_err(|e| format!("audio input: {e:?}"))?;
                if self
                    .observations
                    .get(id)
                    .is_some_and(|previous| previous != input)
                {
                    return Err("audio dependency changed between imports".into());
                }
                self.observations.insert(id.clone(), input.clone());
            }
            self.catalog.complete(&completion.ticket, Ok(imported))?;
        }
        if let Some(id) = self.queue.pop_front() {
            let ticket = self.catalog.request(id)?;
            self.worker.submit(&ticket)?;
            self.pending = true;
            Ok(false)
        } else {
            Ok(true)
        }
    }
    pub(super) fn close(self) -> std::thread::JoinHandle<()> {
        self.worker.close()
    }
    pub(super) fn finish(
        self,
        connection: voxy_audio_device::OutputConnection,
        project: &AuthoringProject,
    ) -> Result<AudioPlay, EditorError> {
        let rate = connection.sample_rate();
        let output = Some((connection, voxy_audio_device::PcmPump::new(256)?));
        let runtime = SceneAudioRuntime::new(rate, 128, 16)?;
        let reload = super::audio_reload::AudioReload::new(
            project,
            self.worker,
            self.recipes,
            &self.catalog,
        )?;
        Ok(AudioPlay {
            catalog: self.catalog,
            runtime,
            output,
            offline_output: None,
            rate,
            reload: Some(reload),
            remainder: 0,
            frames: 0,
            peak: 0.,
        })
    }
}
#[derive(Debug)]
pub(super) struct AudioPlay {
    pub(super) catalog: AssetCatalog<ImportedAsset<Clip>>,
    runtime: SceneAudioRuntime,
    reload: Option<super::audio_reload::AudioReload>,
    offline_output: Option<voxy_audio_device::OfflineOutput>,
    output: Option<(
        voxy_audio_device::OutputConnection,
        voxy_audio_device::PcmPump,
    )>,
    rate: u32,
    remainder: u64,
    pub(super) frames: u64,
    pub(super) peak: f32,
}
impl AudioPlay {
    pub(crate) fn attach_prepared_native_output(
        &mut self,
        connection: voxy_audio_device::OutputConnection,
    ) -> Result<(), EditorError> {
        if self.output.is_some() || self.rate != connection.sample_rate() {
            return Err("native playback output rate or owner mismatch".into());
        }
        self.output = Some((connection, voxy_audio_device::PcmPump::new(256)?));
        Ok(())
    }
    pub(crate) fn stage_output_publication(
        &mut self,
        prepared: &mut voxy_audio_device::PreparedOutputCheckpoint,
        frozen: &mut Option<voxy_audio_device::FrozenOutputQueue>,
    ) -> Result<Option<voxy_audio_device::ReadyOutputPublication>, EditorError> {
        let (connection, pump) = self.output.as_ref().ok_or("missing staged native output")?;
        Ok(prepared.stage_publication(connection, frozen, pump)?)
    }
    pub(crate) fn publish_ready_output(
        &mut self,
        publication: voxy_audio_device::ReadyOutputPublication,
    ) -> voxy_audio_device::FrozenOutputQueue {
        let (_, pump) = self.output.as_mut().expect("admitted native output owner");
        publication.publish(pump).0
    }
    #[cfg(test)]
    pub(crate) fn capture_native_output(
        &self,
        frozen: &mut voxy_audio_device::FrozenOutputQueue,
        max_bytes: usize,
    ) -> Result<voxy_audio_device::OutputCheckpoint, EditorError> {
        let (connection, pump) = self.output.as_ref().ok_or("missing native output")?;
        voxy_audio_device::OutputCheckpoint::capture(
            connection,
            frozen,
            pump,
            voxy_audio_device::OutputCheckpointLimits {
                max_bytes,
                max_queue_frames: 4096,
                max_pump_frames: 256,
                sample_rate: self.rate,
            },
        )?
        .ok_or_else(|| "native output queue not ready".into())
    }
    pub(crate) fn output_connection(&self) -> Option<&voxy_audio_device::OutputConnection> {
        self.output.as_ref().map(|(connection, _)| connection)
    }

    pub(super) fn prepare(
        scene: &SceneGraph,
        project: &AuthoringProject,
        connection: Option<voxy_audio_device::OutputConnection>,
    ) -> Result<Option<Self>, EditorError> {
        let snapshot = extract_scene_audio(scene, 128)?;
        if snapshot.sources.is_empty() {
            return Ok(None);
        }
        if snapshot
            .sources
            .iter()
            .any(|source| source.descriptor.bus >= 16)
        {
            return Err("play audio bus must be below 16".into());
        }
        let output = connection
            .map(|connection| voxy_audio_device::PcmPump::new(256).map(|pump| (connection, pump)))
            .transpose()?;
        let rate = output
            .as_ref()
            .map_or(48000, |(connection, _)| connection.sample_rate());
        let mut catalog = AssetCatalog::new(128, 128)?;
        let mut budget = AudioBudget::default();
        let mut recipes = std::collections::BTreeMap::new();
        for source in &snapshot.sources {
            let id = AssetId(source.descriptor.asset.clone());
            if recipes
                .insert(id.clone(), source.descriptor.import_settings.clone())
                .is_some_and(|previous| previous != source.descriptor.import_settings)
            {
                return Err("conflicting audio settings for one logical asset".into());
            }
            if catalog.snapshot(&id).is_some() {
                continue;
            }
            let ticket = catalog.request(id.clone())?;
            let imported =
                project.import_audio(&id.0, rate, source.descriptor.import_settings.as_deref())?;
            let input_bytes =
                imported
                    .inputs()
                    .observations()
                    .values()
                    .try_fold(0_usize, |total, input| {
                        let input = input.as_ref().map_err(|e| format!("audio input: {e:?}"))?;
                        total
                            .checked_add(input.bytes.len())
                            .ok_or_else(|| String::from("audio input budget overflow"))
                    })?;
            budget.admit(input_bytes, imported.value().frame_count())?;
            catalog.complete(&ticket, Ok(imported))?;
        }
        let reload = super::audio_reload::AudioReload::new(
            project,
            project.audio_worker(rate, recipes.clone())?,
            recipes,
            &catalog,
        )?;
        Ok(Some(Self {
            reload: Some(reload),
            catalog,
            runtime: SceneAudioRuntime::new(rate, 128, 16)?,
            output,
            offline_output: None,
            rate,
            remainder: 0,
            frames: 0,
            peak: 0.,
        }))
    }
    pub(super) fn poll_reload(&mut self, scene: &SceneGraph) -> Result<(), EditorError> {
        if let Some(reload) = &mut self.reload {
            reload.poll(&mut self.catalog)?;
        }
        synchronize_audio_assets(
            &mut self.runtime,
            &extract_scene_audio(scene, 128)?,
            &self.catalog,
        )
        .map_err(Into::into)
    }
    pub(super) fn close(mut self) -> Vec<std::thread::JoinHandle<()>> {
        self.reload
            .take()
            .map_or_else(Vec::new, super::audio_reload::AudioReload::close)
    }
    pub(super) fn input_observations(
        &self,
        scene: &SceneGraph,
    ) -> Result<
        std::collections::BTreeMap<
            AssetId,
            Result<voxy_assets::InputSnapshot, voxy_assets::InputError>,
        >,
        EditorError,
    > {
        let mut observations = std::collections::BTreeMap::new();
        for (_, source) in scene.components::<voxy_gameplay::AudioSource>() {
            let publication = self
                .catalog
                .snapshot(&AssetId(source.asset.clone()))
                .ok_or("missing packaged audio publication")?;
            for (id, input) in publication.inputs().observations() {
                if observations
                    .get(id)
                    .is_some_and(|previous| previous != input)
                {
                    return Err("inconsistent audio input revisions".into());
                }
                observations.insert(id.clone(), input.clone());
            }
        }
        Ok(observations)
    }
    #[cfg(test)]
    pub(super) fn native_stats(&self) -> Option<voxy_audio_device::OutputStats> {
        self.output
            .as_ref()
            .map(|(connection, _)| connection.stats())
    }
    /// Preflights scene and playback grants before reconciliation or output.
    pub(super) fn advance_scoped(
        &mut self,
        access: voxy_scene::SceneSystemAccess<'_>,
        steps: usize,
    ) -> Result<(), EditorError> {
        access.require_write("audio.playback")?;
        self.advance(access.read()?, steps)
    }
    pub(super) fn advance(&mut self, scene: &SceneGraph, steps: usize) -> Result<(), EditorError> {
        synchronize_audio_assets(
            &mut self.runtime,
            &extract_scene_audio(scene, 128)?,
            &self.catalog,
        )?;
        let (budget, remainder) = self.checkpoint_clock().sample_budget(steps)?;
        self.remainder = remainder;
        if let Some((connection, pump)) = &mut self.output {
            let report = pump.pump(connection.sender(), budget, |block| {
                self.runtime.render(block);
                self.frames = self.frames.saturating_add(block.len() as u64);
                self.peak = block
                    .iter()
                    .flatten()
                    .fold(self.peak, |peak, sample| peak.max(sample.abs()));
            });
            if matches!(
                report.stop,
                Some(
                    voxy_audio_device::SubmitError::Closed
                        | voxy_audio_device::SubmitError::InvalidFrame
                )
            ) || connection.stats().errors != 0
            {
                return Err("native audio output failed".into());
            }
            return Ok(());
        }
        let mut output = [[0.; 2]; 800];
        let consumed = self
            .offline_output
            .as_mut()
            .map_or(0, |output| output.discard_frames(budget));
        let mut remaining = budget - consumed;
        while remaining != 0 {
            let count = remaining.min(output.len());
            let block = &mut output[..count];
            self.runtime.render(block);
            self.frames = self.frames.saturating_add(count as u64);
            remaining -= count;
            self.peak = block
                .iter()
                .flatten()
                .fold(self.peak, |peak, sample| peak.max(sample.abs()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_audio_denial_preserves_counters_and_scene() {
        use voxy_scene::{SchedulePlan, SystemAccess, SystemSpec, Transform};
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut audio = AudioPlay {
            catalog: AssetCatalog::new(128, 128).unwrap(),
            runtime: SceneAudioRuntime::new(48000, 128, 16).unwrap(),
            reload: None,
            output: None,
            offline_output: None,
            rate: 48000,
            remainder: 0,
            frames: 0,
            peak: 0.,
        };
        for grants in [
            vec![("scene", false)],
            vec![("scene", false), ("audio.playback", false)],
            vec![("audio.playback", true)],
        ] {
            let plan = SchedulePlan::build(
                &[SystemSpec {
                    name: "audio.advance".into(),
                    phase: 0,
                    after: vec![],
                    access: grants
                        .into_iter()
                        .map(|(resource, write)| SystemAccess {
                            resource: resource.into(),
                            write,
                        })
                        .collect(),
                }],
                1,
            )
            .unwrap();
            assert!(
                plan.run_scene(&mut scene, |_, access| audio.advance_scoped(access, 1))
                    .is_err()
            );
            assert_eq!(audio.frames, 0);
            assert_eq!(audio.peak, 0.);
            assert_eq!(audio.remainder, 0);
            assert!(scene.active_in_hierarchy(owner).unwrap());
        }
        schedule()
            .unwrap()
            .run_scene(&mut scene, |_, access| {
                assert!(access.read().is_ok());
                audio.advance_scoped(access, 1)
            })
            .unwrap();
        assert_eq!(audio.frames, 800);
        assert!(scene.active_in_hierarchy(owner).unwrap());
    }
    #[test]
    fn aggregate_input_and_pcm_admission_preserve_counters_on_rejection() {
        let mut budget = AudioBudget::default();
        budget.admit(8 * 1024 * 1024, 240_000).unwrap();
        budget.admit(8 * 1024 * 1024, 240_000).unwrap();
        let before = AudioBudget {
            input_bytes: budget.input_bytes,
            frames: budget.frames,
        };
        assert!(budget.admit(1, 0).is_err());
        assert_eq!(budget, before);
        assert!(budget.admit(0, 1).is_err());
        assert_eq!(budget, before);
        assert!(budget.admit(usize::MAX, 0).is_err());
        assert_eq!(budget, before);
    }
}

#[path = "audio_clock_checkpoint.rs"]
mod clock_checkpoint;
#[allow(unused_imports)]
pub(super) use clock_checkpoint::AudioClockCheckpoint;

#[path = "audio_playback_checkpoint.rs"]
pub(super) mod playback_checkpoint;

#[path = "audio_owner_checkpoint.rs"]
pub(crate) mod owner_checkpoint;
