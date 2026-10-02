//! Play-session audio owner with optional native output.
use super::prefab_authoring::AuthoringProject;
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
    pub(super) fn admit(&mut self, input_bytes: usize, frames: usize) -> Result<(), String> {
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
fn recipes(scene: &SceneGraph) -> Result<AudioRecipes, String> {
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
    ) -> Result<Self, String> {
        let recipes = recipes(scene)?;
        Ok(Self {
            worker: project.audio_worker(rate, recipes.clone())?,
            queue: recipes.keys().cloned().collect(),
            recipes,
            catalog: AssetCatalog::new(128, 128).map_err(|e| e.to_string())?,
            budget: AudioBudget::default(),
            observations: std::collections::BTreeMap::new(),
            pending: false,
        })
    }
    pub(super) fn matches(&self, scene: &SceneGraph) -> Result<bool, String> {
        Ok(self.recipes == recipes(scene)?)
    }
    pub(super) fn poll(&mut self) -> Result<bool, String> {
        if self.pending {
            let Some(completion) = self.worker.try_result().map_err(|e| e.to_string())? else {
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
            self.catalog
                .complete(&completion.ticket, Ok(imported))
                .map_err(|e| e.to_string())?;
        }
        if let Some(id) = self.queue.pop_front() {
            let ticket = self.catalog.request(id).map_err(|e| e.to_string())?;
            self.worker.submit(&ticket).map_err(|e| e.to_string())?;
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
    ) -> Result<AudioPlay, String> {
        let rate = connection.sample_rate();
        let output = Some((
            connection,
            voxy_audio_device::PcmPump::new(256).map_err(|e| e.to_string())?,
        ));
        let runtime = SceneAudioRuntime::new(rate, 128, 16).map_err(|e| e.to_string())?;
        let reload = super::audio_reload::AudioReload::new(
            project,
            self.worker,
            self.recipes.keys().cloned().collect(),
            &self.catalog,
        )?;
        Ok(AudioPlay {
            catalog: self.catalog,
            runtime,
            output,
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
    pub(super) fn prepare(
        scene: &SceneGraph,
        project: &AuthoringProject,
        connection: Option<voxy_audio_device::OutputConnection>,
    ) -> Result<Option<Self>, String> {
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
            .transpose()
            .map_err(|e| e.to_string())?;
        let rate = output
            .as_ref()
            .map_or(48000, |(connection, _)| connection.sample_rate());
        let mut catalog = AssetCatalog::new(128, 128).map_err(|e| e.to_string())?;
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
            let ticket = catalog.request(id.clone()).map_err(|e| e.to_string())?;
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
            catalog
                .complete(&ticket, Ok(imported))
                .map_err(|e| e.to_string())?;
        }
        let reload = super::audio_reload::AudioReload::new(
            project,
            project.audio_worker(rate, recipes.clone())?,
            recipes.keys().cloned().collect(),
            &catalog,
        )?;
        Ok(Some(Self {
            reload: Some(reload),
            catalog,
            runtime: SceneAudioRuntime::new(rate, 128, 16).map_err(|e| e.to_string())?,
            output,
            rate,
            remainder: 0,
            frames: 0,
            peak: 0.,
        }))
    }
    pub(super) fn poll_reload(&mut self, scene: &SceneGraph) -> Result<(), String> {
        if let Some(reload) = &mut self.reload {
            reload.poll(&mut self.catalog)?;
        }
        synchronize_audio_assets(
            &mut self.runtime,
            &extract_scene_audio(scene, 128)?,
            &self.catalog,
        )
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
        String,
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
    ) -> Result<(), String> {
        access
            .require_write("audio.playback")
            .map_err(|e| e.to_string())?;
        self.advance(access.read().map_err(|e| e.to_string())?, steps)
    }
    pub(super) fn advance(&mut self, scene: &SceneGraph, steps: usize) -> Result<(), String> {
        synchronize_audio_assets(
            &mut self.runtime,
            &extract_scene_audio(scene, 128)?,
            &self.catalog,
        )?;
        if let Some((connection, pump)) = &mut self.output {
            let frames = self
                .remainder
                .checked_add(
                    u64::try_from(steps)
                        .map_err(|e| e.to_string())?
                        .checked_mul(u64::from(self.rate))
                        .ok_or("audio step overflow")?,
                )
                .ok_or("audio step overflow")?;
            let budget = usize::try_from(frames / 60).map_err(|e| e.to_string())?;
            self.remainder = frames % 60;
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
        for _ in 0..steps {
            self.runtime.render(&mut output);
            self.frames = self.frames.saturating_add(800);
            self.peak = output
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
