//! Import-setting drafts share the scene undo store; files change only on Save.
use super::{App, InspectorMode};
use std::collections::BTreeMap;
use voxy_gameplay::AudioImportConfig;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct AudioSettingsDraft {
    pub(super) config: AudioImportConfig,
    pub(super) source: String,
    pub(super) observations: Vec<(String, [u8; 32])>,
}
type Drafts = BTreeMap<String, AudioSettingsDraft>;
impl App {
    fn settings_id(&self) -> Result<String, Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("no selected object")?;
        self.scene
            .component::<voxy_gameplay::AudioSource>(node)?
            .and_then(|source| source.import_settings.clone())
            .ok_or_else(|| "choose an import-settings asset on the audio source first".into())
    }
    fn settings_drafts(&self) -> Result<Drafts, Box<dyn std::error::Error>> {
        let value = self
            .authoring
            .history
            .as_ref()
            .ok_or("missing history")?
            .auxiliary();
        if value.is_null() {
            Ok(Drafts::new())
        } else {
            Ok(serde_json::from_value(value.clone())?)
        }
    }
    pub(super) fn settings_config(&self) -> Option<AudioImportConfig> {
        self.settings_drafts()
            .ok()?
            .get(&self.settings_id().ok()?)
            .map(|draft| draft.config)
    }
    pub(super) fn open_audio_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let id = self.settings_id()?;
        let draft = self.authoring.authoring_project.audio_settings_draft(&id)?;
        let mut drafts = self.settings_drafts()?;
        if drafts.len() >= 128 && !drafts.contains_key(&id) {
            return Err("settings draft capacity exceeded".into());
        }
        let source = draft.source.clone();
        drafts.insert(id, draft);
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_auxiliary(
                serde_json::to_value(drafts)?,
                &self.authoring.authoring_project.registry,
            )?;
        self.authoring.settings_written.remove(&source);
        self.inspector = InspectorMode::ImportSettings;
        Ok(())
    }
    pub(super) fn edit_audio_settings(
        &mut self,
        index: usize,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let value: usize = value
            .parse()
            .map_err(|_| "import limits require whole nonnegative numbers")?;
        let id = self.settings_id()?;
        let mut drafts = self.settings_drafts()?;
        let draft = drafts
            .get_mut(&id)
            .ok_or("reopen import settings after undo/load")?;
        match index {
            0 => draft.config.max_input_bytes = value,
            1 => draft.config.max_frames = value,
            2 => draft.config.max_filter_evaluations = value,
            _ => return Err("unknown import setting".into()),
        }
        draft.config.settings(48000)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_auxiliary(
                serde_json::to_value(drafts)?,
                &self.authoring.authoring_project.registry,
            )?;
        self.field = None;
        Ok(())
    }
    pub(super) fn save_audio_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let id = self.settings_id()?;
        let drafts = self.settings_drafts()?;
        let draft = drafts
            .get(&id)
            .ok_or("open import settings before saving")?;
        let digest = self
            .authoring
            .authoring_project
            .save_audio_settings(draft, &self.authoring.settings_written)?;
        self.authoring
            .settings_written
            .insert(draft.source.clone(), digest);
        if let Some(window) = &self.window {
            window.set_title("Voxy — import settings saved");
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use winit::keyboard::KeyCode;
    #[test]
    fn manifest_relocation_rejects_stale_settings_save_until_explicit_reload() {
        let root = std::env::temp_dir().join(format!(
            "voxy-settings-binding-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let a = root.join("a.json");
        let b = root.join("b.json");
        let config = AudioImportConfig {
            version: 1,
            max_input_bytes: 1024,
            max_frames: 10,
            max_filter_evaluations: 650,
        };
        config.save_file(&a).unwrap();
        config.save_file(&b).unwrap();
        let manifest = root.join("assets.json");
        let write_manifest = |source| {
            std::fs::write(
                &manifest,
                serde_json::to_vec(&serde_json::json!({
                    "version": 1, "assets": [{"asset": "settings", "source": source}]
                }))
                .unwrap(),
            )
            .unwrap();
        };
        write_manifest("a.json");
        let project = super::super::prefab_authoring::AuthoringProject::new(
            &root,
            &super::super::InputRecipe::Manifest(voxy_assets::AssetId("assets.json".into())),
        )
        .unwrap();
        let mut stale = project.audio_settings_draft("settings").unwrap();
        stale.config.max_frames = 9;
        write_manifest("b.json");
        assert!(
            project
                .save_audio_settings(&stale, &BTreeMap::new())
                .is_err()
        );
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(&a).unwrap()).unwrap(),
            config
        );
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(&b).unwrap()).unwrap(),
            config
        );
        let mut refreshed = project.audio_settings_draft("settings").unwrap();
        refreshed.config.max_frames = 9;
        project
            .save_audio_settings(&refreshed, &BTreeMap::new())
            .unwrap();
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(&b).unwrap())
                .unwrap()
                .max_frames,
            9
        );
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(&a).unwrap()).unwrap(),
            config
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    pub(crate) fn verify_settings_editor(app: &mut App, path: &std::path::Path) {
        let scene = app.authoring_document().unwrap();
        let original = std::fs::read(path).unwrap();
        let composition = app.authoring.history.as_ref().unwrap().metadata().clone();
        app.panel_action(super::super::panels::Action::AudioSettingsLoad)
            .unwrap();
        assert_eq!(app.inspector, InspectorMode::ImportSettings);
        app.panel_action(super::super::panels::Action::Field(1))
            .unwrap();
        app.field_key(KeyCode::Digit1, Some("9")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        assert_eq!(app.settings_config().unwrap().max_frames, 9);
        app.panel_action(super::super::panels::Action::Field(2))
            .unwrap();
        app.field_key(KeyCode::Digit1, Some("27000001")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        assert_eq!(
            app.settings_config().unwrap().max_filter_evaluations,
            27_000_001
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.settings_config().unwrap().max_filter_evaluations, 650);
        assert_eq!(std::fs::read(path).unwrap(), original);
        app.edit_key(KeyCode::F5).unwrap();
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(path).unwrap())
                .unwrap()
                .max_frames,
            9
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.settings_config().unwrap().max_frames, 10);
        app.panel_action(super::super::panels::Action::AudioSettingsSave)
            .unwrap();
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(path).unwrap())
                .unwrap()
                .max_frames,
            10
        );
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.settings_config().unwrap().max_frames, 9);
        let outside = AudioImportConfig {
            max_frames: 8,
            ..app.settings_config().unwrap()
        };
        outside.save_file(path).unwrap();
        assert!(
            app.panel_action(super::super::panels::Action::AudioSettingsSave)
                .is_err()
        );
        assert_eq!(
            AudioImportConfig::from_json(&std::fs::read(path).unwrap()).unwrap(),
            outside
        );
        app.edit_key(KeyCode::F9).unwrap();
        assert_eq!(app.settings_config().unwrap().max_frames, 8);
        app.panel_action(super::super::panels::Action::Field(1))
            .unwrap();
        app.field_key(KeyCode::Digit1, Some("0")).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.settings_config().unwrap().max_frames, 8);
        app.panel_action(super::super::panels::Action::Field(1))
            .unwrap();
        app.field_key(KeyCode::Digit1, Some("10")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        app.panel_action(super::super::panels::Action::AudioSettingsSave)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), scene);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &composition
        );
        println!(
            "EDITOR IMPORT SETTINGS PASS: staged edits, undo/redo after save, external conflict and validated reload"
        );
    }
}
