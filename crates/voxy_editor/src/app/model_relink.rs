//! One bounded, two-phase relink worker. Candidate compatibility is checked on
//! the native App owner before the worker may write the manifest.
use crate::prefab_authoring::model_relink::PreparedModelRelink;
use crate::*;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
#[derive(Debug)]
pub(crate) struct Pending {
    pub(super) asset: AssetId,
    project: Arc<()>,
    prepared: Option<Receiver<PreparedModelRelink>>,
    decision: Option<SyncSender<Option<PreparedModelRelink>>>,
    done: Receiver<Result<Option<ImportedAsset<prefab_authoring::AuthoredScene>>, String>>,
    authoring_origin: Option<AuthoringOrigin>,
    rejected: Option<String>,
    thread: std::thread::JoinHandle<()>,
}
const AUTHORING_RECEIPT_BYTES: usize = 256 * 1024 * 1024;
#[derive(Debug)]
struct AuthoringOrigin {
    fingerprint: blake3::Hash,
    path: Option<std::path::PathBuf>,
}
impl Pending {
    fn authoring_unchanged(&self, app: &App) -> bool {
        self.authoring_origin.as_ref().is_none_or(|origin| {
            origin.path == app.authoring.scene_path
                && app
                    .authoring_source_fingerprint(AUTHORING_RECEIPT_BYTES)
                    .ok()
                    .flatten()
                    == Some(origin.fingerprint)
        })
    }
    fn spawn(
        project: prefab_authoring::AuthoringProject,
        asset: AssetId,
        source: SourcePath,
        authoring: Option<ImportedAsset<prefab_authoring::AuthoredScene>>,
        authoring_origin: Option<AuthoringOrigin>,
    ) -> std::io::Result<Self> {
        let (prepared_tx, prepared_rx) = mpsc::sync_channel(1);
        let (decision_tx, decision_rx) = mpsc::sync_channel::<Option<PreparedModelRelink>>(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let target = asset.clone();
        let origin = project.observation_token();
        let thread = std::thread::Builder::new()
            .name("voxy-model-relink".into())
            .spawn(move || {
                let result = (|| -> Result<Option<ImportedAsset<prefab_authoring::AuthoredScene>>, String> {
                    let mut prepared = project
                        .prepare_model_relink(&target, &source)
                        .map_err(|e| e.to_string())?;
                    project.attach_relink_authoring(&mut prepared, &target, authoring)
                        .map_err(|e| e.to_string())?;
                    prepared_tx
                        .send(prepared)
                        .map_err(|_| "model relink cancelled".to_owned())?;
                    let prepared = decision_rx
                        .recv()
                        .ok()
                        .flatten()
                        .ok_or("model relink cancelled")?;
                    project
                        .publish_model_relink(prepared)
                        .map(|publication| publication.authoring)
                        .map_err(|e| e.to_string())
                })();
                let _ = done_tx.send(result);
            })?;
        Ok(Self {
            asset,
            project: origin,
            prepared: Some(prepared_rx),
            decision: Some(decision_tx),
            done: done_rx,
            authoring_origin,
            rejected: None,
            thread,
        })
    }
    pub(crate) fn close(self) -> std::thread::JoinHandle<()> {
        let Self {
            prepared,
            decision,
            done,
            thread,
            ..
        } = self;
        drop(prepared);
        drop(decision);
        drop(done);
        thread
    }
}
#[derive(Debug)]
pub(crate) struct Draft {
    owner: NodeId,
    asset: AssetId,
    scene: voxy_scene::SceneChangeToken,
    project: Arc<()>,
    pub(super) text: String,
}
impl Draft {
    pub(super) fn valid(&self, app: &App) -> bool {
        app.instances.get(app.selected) == Some(&self.owner)
            && app.scene.change_token() == self.scene
            && Arc::ptr_eq(
                &self.project,
                &app.authoring.authoring_project.observation_token(),
            )
            && app
                .scene
                .component::<ModelInstance>(self.owner)
                .ok()
                .flatten()
                .is_some_and(|model| model.asset == self.asset)
    }
}
impl App {
    pub(crate) fn begin_model_relink_edit(&mut self) -> Result<(), EditorError> {
        if self.model_relink.is_some()
            || self.resource_checkpoint.is_some()
            || self.restore_barrier.is_some()
            || self.play.playing.is_some()
            || self.audio.is_pending()
            || !matches!(self.import_recipe, InputRecipe::Manifest(_))
        {
            return Err("model source entry is busy or authoring is fenced".into());
        }
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("select a model to relink")?;
        let asset = self
            .scene
            .component::<ModelInstance>(owner)?
            .ok_or("selected object has no model")?
            .asset
            .clone();
        self.model_relink_draft = Some(Draft {
            owner,
            asset,
            scene: self.scene.change_token(),
            project: self.authoring.authoring_project.observation_token(),
            text: String::new(),
        });
        self.model_relink_error = None;
        Ok(())
    }
    pub(crate) fn model_relink_key(&mut self, key: KeyCode, text: Option<&str>) {
        let Some(mut draft) = self.model_relink_draft.take() else {
            return;
        };
        if key == KeyCode::Escape || !draft.valid(self) {
            self.model_relink_error = None;
            return;
        }
        if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter) {
            let result = SourcePath::new(draft.text.clone())
                .map_err(EditorError::from)
                .and_then(|source| self.request_model_relink(draft.asset.clone(), source));
            if let Err(error) = result {
                self.model_relink_error = Some((draft.asset.clone(), error.to_string()));
                self.model_relink_draft = Some(draft);
            }
            return;
        }
        if key == KeyCode::Backspace {
            draft.text.pop();
        } else if let Some(text) = text
            && draft
                .text
                .len()
                .checked_add(text.len())
                .is_some_and(|len| len <= 4096)
            && !text.chars().any(char::is_control)
        {
            draft.text.push_str(text);
        }
        self.model_relink_draft = Some(draft);
    }
    fn reap_model_relink(&mut self) {
        if self
            .retired_model_relink
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
            && let Some(worker) = self.retired_model_relink.take()
        {
            // Finished threads cannot block waiting for IO or a decision.
            if let Err(payload) = worker.join() {
                self.model_relink_error = Some((
                    AssetId(String::new()),
                    format!(
                        "model relink worker panicked: {}",
                        panic_message(payload.as_ref())
                    ),
                ));
            }
        }
    }
    pub(crate) fn request_model_relink(
        &mut self,
        asset: AssetId,
        source: SourcePath,
    ) -> Result<(), EditorError> {
        self.reap_model_relink();
        if self.model_relink.is_some()
            || self.model_relink_draft.is_some()
            || self.retired_model_relink.is_some()
            || self.resource_checkpoint.is_some()
            || self.restore_barrier.is_some()
            || self.play.playing.is_some()
            || self.audio.is_pending()
        {
            return Err("model relink is busy or authoring is fenced".into());
        }
        if !matches!(self.import_recipe, InputRecipe::Manifest(_))
            || !self.required_cpu_model_assets().contains(&asset)
        {
            return Err("model relink requires a referenced manifest model".into());
        }
        let authoring = match &self.import_recipe {
            InputRecipe::Manifest(manifest) => self
                .authoring
                .authoring_source
                .as_ref()
                .filter(|source| source.inputs().observations().contains_key(manifest))
                .cloned(),
            InputRecipe::Direct => None,
        };
        let authoring_origin = if authoring.is_some() {
            Some(AuthoringOrigin {
                fingerprint: self
                    .authoring_source_fingerprint(AUTHORING_RECEIPT_BYTES)?
                    .ok_or("missing relink authoring origin")?,
                path: self.authoring.scene_path.clone(),
            })
        } else {
            None
        };
        self.model_relink = Some(Pending::spawn(
            self.authoring.authoring_project.clone(),
            asset,
            source,
            authoring,
            authoring_origin,
        )?);
        self.model_relink_error = None;
        Ok(())
    }
    pub(crate) fn poll_model_relink(&mut self) {
        self.reap_model_relink();
        if self
            .model_relink_draft
            .as_ref()
            .is_some_and(|draft| !draft.valid(self))
        {
            self.model_relink_draft = None;
        }
        let Some(mut pending) = self.model_relink.take() else {
            return;
        };
        if let Some(prepared) = pending.prepared.as_ref().and_then(|rx| rx.try_recv().ok()) {
            pending.prepared = None;
            let compatible = if Arc::ptr_eq(
                &pending.project,
                &self.authoring.authoring_project.observation_token(),
            ) {
                if pending.authoring_unchanged(self) {
                    self.validate_reload_candidate(&pending.asset, prepared.candidate())
                        .map_err(|e| e.to_string())
                } else {
                    Err("authoring source changed during model relink".into())
                }
            } else {
                Err("project changed during model relink".into())
            };
            let decision = match compatible {
                Ok(()) => Some(prepared),
                Err(error) => {
                    pending.rejected = Some(error);
                    None
                }
            };
            if pending
                .decision
                .take()
                .is_none_or(|tx| tx.try_send(decision).is_err())
            {
                pending.rejected = Some("model relink worker stopped before decision".into());
            }
        }
        let result = match pending.done.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("model relink worker disconnected".into())),
        };
        if let Some(result) = result {
            match result {
                Err(error) => {
                    self.model_relink_error = Some((
                        pending.asset.clone(),
                        pending.rejected.take().unwrap_or(error),
                    ));
                }
                Ok(Some(authoring)) => {
                    let unchanged = pending.authoring_origin.is_some()
                        && Arc::ptr_eq(
                            &pending.project,
                            &self.authoring.authoring_project.observation_token(),
                        )
                        && pending.authoring_unchanged(self);
                    if unchanged {
                        self.authoring.authoring_source = Some(authoring);
                    } else {
                        self.model_relink_error = Some((
                            pending.asset.clone(),
                            "Model source updated; authoring source changed, reload before saving"
                                .into(),
                        ));
                    }
                }
                Ok(None) => {}
            }
            self.retired_model_relink = Some(pending.close());
        } else {
            self.model_relink = Some(pending);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefab_authoring::model_relink::tests::Fixture;
    fn await_state(app: &mut App, ready: impl Fn(&App) -> bool) {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            app.tick().unwrap();
            if ready(app) {
                break;
            }
            assert!(
                Instant::now() < until,
                "relink did not reach expected owner state"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn background_relink_checks_owner_then_publishes_and_fences_play_and_checkpoint() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let id = AssetId("character".into());
        await_state(&mut app, |app| app.catalog.snapshot(&id).is_some());
        let document = app.authoring_document().unwrap();
        app.request_model_relink(id.clone(), SourcePath::new("new.obj").unwrap())
            .unwrap();
        assert!(
            app.request_model_relink(id.clone(), SourcePath::new("old.obj").unwrap())
                .is_err()
        );
        assert!(app.begin_model_checkpoint().is_err());
        assert!(app.begin_restore_barrier(1024 * 1024).is_err());
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        assert!(app.toggle_play().is_err());
        assert_eq!(app.authoring_document().unwrap(), document);
        await_state(&mut app, |app| {
            app.model_relink.is_none()
                && app.retired_model_relink.is_none()
                && app.catalog.snapshot(&id).is_some_and(|asset| {
                    asset
                        .inputs()
                        .observations()
                        .contains_key(&AssetId("new.obj".into()))
                })
        });
        assert!(app.model_relink_error.is_none());
        assert_eq!(
            app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
            2.
        );
        assert_eq!(app.authoring_document().unwrap(), document);
        let manifest = std::fs::read(fixture.root().join("assets.json")).unwrap();
        app.request_model_relink(id.clone(), SourcePath::new("missing.obj").unwrap())
            .unwrap();
        await_state(&mut app, |app| {
            app.model_relink.is_none() && app.retired_model_relink.is_none()
        });
        assert_eq!(app.model_relink_error.as_ref().unwrap().0, id);
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            manifest
        );
        assert_eq!(app.authoring_document().unwrap(), document);
        assert_eq!(
            app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
            2.
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn shutdown_before_owner_decision_joins_relink_without_manifest_publication() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let original = std::fs::read(fixture.root().join("assets.json")).unwrap();
        app.request_model_relink(
            AssetId("character".into()),
            SourcePath::new("new.obj").unwrap(),
        )
        .unwrap();
        app.stop_workers().unwrap();
        assert!(app.model_relink.is_none() && app.retired_model_relink.is_none());
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            original
        );
    }
    #[test]
    fn owner_rejects_incompatible_hierarchy_before_worker_can_write_manifest() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let id = AssetId("character".into());
        await_state(&mut app, |app| app.catalog.snapshot(&id).is_some());
        app.scene
            .insert_component(app.instances[0], ModelPart { node: u32::MAX })
            .unwrap();
        let document = app.authoring_document().unwrap();
        let original = std::fs::read(fixture.root().join("assets.json")).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/scene3d/assembly.glb"),
            fixture.root().join("changed.glb"),
        )
        .unwrap();
        app.request_model_relink(id.clone(), SourcePath::new("changed.glb").unwrap())
            .unwrap();
        await_state(&mut app, |app| {
            app.model_relink.is_none() && app.retired_model_relink.is_none()
        });
        assert!(
            app.model_relink_error
                .as_ref()
                .unwrap()
                .1
                .contains("hierarchy changed")
        );
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            original
        );
        assert_eq!(app.authoring_document().unwrap(), document);
        assert_eq!(
            app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
            0.
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn panel_path_entry_rebinds_cold_missing_id_and_saves_reopens_without_scene_changes() {
        let fixture = Fixture::new();
        let mut initial = fixture.app();
        initial.edit_key(KeyCode::KeyD).unwrap();
        initial.save_authoring().unwrap();
        let document = initial.authoring_document().unwrap();
        initial.stop_workers().unwrap();
        std::fs::write(
            fixture.root().join("assets.json"),
            br#"{"version":1,"assets":[]}"#,
        )
        .unwrap();
        let mut app = fixture.app();
        let id = AssetId("character".into());
        assert!(app.selected_model_notice().unwrap().relink);
        let manifest = std::fs::read(fixture.root().join("assets.json")).unwrap();
        app.panel_action(panels::Action::RelinkModel).unwrap();
        assert!(app.selected_model_notice().unwrap().editing);
        assert!(app.begin_model_checkpoint().is_err());
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            manifest
        );
        assert!(app.selected_model_notice().unwrap().lines[1].contains("new.obj"));
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.model_relink_draft.is_none() && app.model_relink.is_some());
        assert!(!app.selected_model_notice().unwrap().relink);
        await_state(&mut app, |app| {
            app.model_relink.is_none()
                && app.retired_model_relink.is_none()
                && app.catalog.snapshot(&id).is_some()
        });
        assert!(app.model_relink_error.is_none());
        assert_eq!(app.authoring_document().unwrap(), document);
        assert_eq!(
            app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
            2.
        );
        assert!(app.selected_model_notice().is_none());
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        app.stop_workers().unwrap();
        let mut reopened = fixture.app();
        assert_eq!(reopened.authoring_document().unwrap(), document);
        await_state(&mut reopened, |app| app.catalog.snapshot(&id).is_some());
        assert_eq!(
            reopened
                .catalog
                .snapshot(&id)
                .unwrap()
                .value()
                .mesh
                .vertices()[0]
                .position[0],
            2.
        );
        reopened.stop_workers().unwrap();
    }
    #[test]
    fn stale_retry_action_preserves_relink_path_and_failed_publication() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let id = AssetId("character".into());
        await_state(&mut app, |app| app.catalog.snapshot(&id).is_some());
        let old = app.catalog.snapshot(&id).unwrap();
        let ticket = app.catalog.request(id.clone()).unwrap();
        app.catalog
            .complete(&ticket, Err("transient read failure".into()))
            .unwrap();
        assert!(app.can_retry_selected_model());
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        let document = app.authoring_document().unwrap();
        let source_change = app.sources.change_token();
        let manifest = std::fs::read(fixture.root().join("assets.json")).unwrap();
        assert!(!app.can_retry_selected_model());
        assert!(app.panel_action(panels::Action::RetryModel).is_err());
        assert_eq!(app.model_relink_draft.as_ref().unwrap().text, "new.obj");
        assert!(app.model_relink_draft.as_ref().unwrap().valid(&app));
        assert!(app.model_relink.is_none() && app.reload.is_empty());
        assert!(matches!(
            app.catalog.status(&id),
            Some(voxy_assets::AssetStatus::Failed(_))
        ));
        assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
        assert_eq!(app.sources.change_token(), source_change);
        assert_eq!(app.authoring_document().unwrap(), document);
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            manifest
        );
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        await_state(&mut app, |app| {
            app.model_relink.is_none()
                && app.retired_model_relink.is_none()
                && app
                    .catalog
                    .snapshot(&id)
                    .is_some_and(|asset| asset.value().mesh.vertices()[0].position[0] == 2.)
        });
        assert!(app.model_relink_error.is_none());
        assert_eq!(app.authoring_document().unwrap(), document);
        app.stop_workers().unwrap();
    }
    #[test]
    fn path_entry_cancel_invalid_input_and_stale_owner_never_start_publication() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let manifest = std::fs::read(fixture.root().join("assets.json")).unwrap();
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::Period, Some("../outside.obj"))
            .unwrap();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.model_relink.is_none() && app.model_relink_draft.is_some());
        assert!(app.model_relink_error.is_some());
        app.authoring_input_key(KeyCode::Escape, None).unwrap();
        assert!(app.model_relink_draft.is_none() && app.model_relink_error.is_none());
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        app.scene
            .set_name(app.instances[0], "Changed while entering path")
            .unwrap();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.model_relink_draft.is_none() && app.model_relink.is_none());
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        app.panel_action(panels::Action::RelinkCancel).unwrap();
        assert!(app.model_relink_draft.is_none());
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some(&"x".repeat(4097)))
            .unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("x\0y"))
            .unwrap();
        assert!(app.model_relink_draft.as_ref().unwrap().text.is_empty());
        app.authoring_input_key(KeyCode::Escape, None).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        app.selected = 0;
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.model_relink_draft.is_none() && app.model_relink.is_none());
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        app.authoring.authoring_project = prefab_authoring::AuthoringProject::with_registry(
            &fixture.root(),
            &app.import_recipe,
            app.authoring.authoring_project.registry.clone(),
        )
        .unwrap();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.model_relink_draft.is_none() && app.model_relink.is_none());
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            manifest
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn project_change_after_input_confirmation_rejects_worker_before_manifest_write() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let original = std::fs::read(fixture.root().join("assets.json")).unwrap();
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyN, Some("new.obj"))
            .unwrap();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        app.authoring.authoring_project = prefab_authoring::AuthoringProject::with_registry(
            &fixture.root(),
            &app.import_recipe,
            app.authoring.authoring_project.registry.clone(),
        )
        .unwrap();
        await_state(&mut app, |app| {
            app.model_relink.is_none() && app.retired_model_relink.is_none()
        });
        assert!(
            app.model_relink_error
                .as_ref()
                .unwrap()
                .1
                .contains("project changed")
        );
        assert_eq!(
            std::fs::read(fixture.root().join("assets.json")).unwrap(),
            original
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn nested_prefab_relink_preserves_unsaved_overrides_and_allows_save_reopen_and_package() {
        let temp = crate::project_prefab_tests::Temp::new();
        let (path, _) = temp.setup().unwrap();
        let root = path.parent().unwrap();
        std::fs::write(
            root.join("replacement.obj"),
            "v 2 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let leaf = std::fs::read(root.join("leaf.prefab")).unwrap();
        let nested = std::fs::read(root.join("nested.prefab")).unwrap();
        let mut app = App::from_project(&path).unwrap();
        app.edit_key(KeyCode::ArrowRight).unwrap();
        let edited = app.authoring_document().unwrap();
        let ids = app.object_ids.clone();
        app.panel_action(panels::Action::RelinkModel).unwrap();
        app.authoring_input_key(KeyCode::KeyR, Some("replacement.obj"))
            .unwrap();
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        let id = AssetId("mesh".into());
        await_state(&mut app, |app| {
            app.model_relink.is_none()
                && app.retired_model_relink.is_none()
                && app.catalog.snapshot(&id).is_some_and(|asset| {
                    asset
                        .inputs()
                        .observations()
                        .contains_key(&AssetId("replacement.obj".into()))
                })
        });
        assert!(app.model_relink_error.is_none());
        assert_eq!(app.authoring_document().unwrap(), edited);
        assert_eq!(app.object_ids, ids);
        app.save_authoring().unwrap();
        let saved: voxy_scene::PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(root.join("main.scene.json")).unwrap()).unwrap();
        assert!(saved.objects.is_empty());
        assert_eq!(saved.instances.len(), 2);
        assert_eq!(saved.instances[0].asset, "nested");
        assert!(!saved.instances[0].overrides.is_empty());
        assert!(saved.instances[1].overrides.is_empty());
        assert_eq!(std::fs::read(root.join("leaf.prefab")).unwrap(), leaf);
        assert_eq!(std::fs::read(root.join("nested.prefab")).unwrap(), nested);
        app.stop_workers().unwrap();
        let mut reopened = App::from_project(&path).unwrap();
        assert_eq!(reopened.authoring_document().unwrap(), edited);
        reopened.stop_workers().unwrap();
        let package = temp.0.join("relinked.vpak");
        export_project_package(&path, &package).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        run_packaged_game(&package, ViewportMode::GameCheck).unwrap();
    }
    #[test]
    fn changed_authoring_before_owner_decision_rejects_manifest_publication() {
        let temp = crate::project_prefab_tests::Temp::new();
        let (path, _) = temp.setup().unwrap();
        let root = path.parent().unwrap();
        std::fs::write(
            root.join("replacement.obj"),
            "v 2 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let mut app = App::from_project(&path).unwrap();
        let original = std::fs::read(root.join("assets.json")).unwrap();
        app.request_model_relink(
            AssetId("mesh".into()),
            SourcePath::new("replacement.obj").unwrap(),
        )
        .unwrap();
        app.authoring.authoring_source = None;
        await_state(&mut app, |app| {
            app.model_relink.is_none() && app.retired_model_relink.is_none()
        });
        assert!(
            app.model_relink_error
                .as_ref()
                .unwrap()
                .1
                .contains("authoring source changed")
        );
        assert!(app.authoring.authoring_source.is_none());
        assert_eq!(std::fs::read(root.join("assets.json")).unwrap(), original);
        app.stop_workers().unwrap();
    }
    #[test]
    fn changed_authoring_after_authorization_is_not_overwritten_by_late_receipt() {
        let temp = crate::project_prefab_tests::Temp::new();
        let (path, _) = temp.setup().unwrap();
        let root = path.parent().unwrap();
        std::fs::write(
            root.join("replacement.obj"),
            "v 2 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let mut app = App::from_project(&path).unwrap();
        let document = app.authoring_document().unwrap();
        let asset = AssetId("mesh".into());
        app.request_model_relink(asset.clone(), SourcePath::new("replacement.obj").unwrap())
            .unwrap();
        let mut pending = app.model_relink.take().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        let prepared = loop {
            if let Ok(prepared) = pending.prepared.as_ref().unwrap().try_recv() {
                break prepared;
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        };
        assert!(pending.authoring_unchanged(&app));
        app.validate_reload_candidate(&asset, prepared.candidate())
            .unwrap();
        pending.prepared = None;
        pending
            .decision
            .take()
            .unwrap()
            .try_send(Some(prepared))
            .unwrap();
        app.authoring.authoring_source = None;
        app.model_relink = Some(pending);
        await_state(&mut app, |app| {
            app.model_relink.is_none() && app.retired_model_relink.is_none()
        });
        assert!(app.authoring.authoring_source.is_none());
        assert!(
            app.model_relink_error
                .as_ref()
                .unwrap()
                .1
                .contains("Model source updated")
        );
        let locations =
            AssetLocations::load_manifest(&root.join("assets.json"), 128, 65536, 65536).unwrap();
        assert_eq!(
            locations.source(&asset).unwrap().as_str(),
            "replacement.obj"
        );
        assert_eq!(app.authoring_document().unwrap(), document);
        app.stop_workers().unwrap();
    }
}
