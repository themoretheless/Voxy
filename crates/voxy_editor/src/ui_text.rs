//! One bounded decoder shared by headless preparation and asynchronous import.
use super::prefab_authoring::AuthoringProject;
use std::collections::{BTreeMap, BTreeSet};
use voxy_assets::{AssetId, ImportInputs, InputError, InputSnapshot};
use voxy_gameplay::{PreparedUiText, UiElement};
use voxy_scene::SceneGraph;

#[derive(Debug)]
pub(super) struct UiTextPreparation {
    fonts: BTreeMap<AssetId, voxy_text::TextFont>,
    pub(super) runs: Vec<PreparedUiText>,
    pub(super) observations: BTreeMap<AssetId, Result<InputSnapshot, InputError>>,
}
impl UiTextPreparation {
    #[cfg(test)]
    pub(super) fn prepare(
        scene: &SceneGraph,
        project: &AuthoringProject,
        viewport: [f32; 2],
    ) -> Result<Self, String> {
        let plan = UiTextPlan::new(scene, viewport)?;
        let mut inputs = ImportInputs::new(9, 16 * 1024 * 1024);
        let prepared = Self::decode(&plan, project, &mut inputs)?;
        project.validate_import_inputs(&inputs)?;
        Ok(prepared)
    }
    pub(super) fn prepare_import(
        scene: &SceneGraph,
        project: &AuthoringProject,
        viewport: [f32; 2],
    ) -> Result<std::sync::Arc<voxy_assets::ImportedAsset<Self>>, String> {
        let mut worker = UiTextPlan::new(scene, viewport)?.worker(project)?;
        let result = (|| {
            let mut catalog =
                voxy_assets::AssetCatalog::new(1, 1).map_err(|error| error.to_string())?;
            let ticket = catalog
                .request(AssetId("game-ui-text".into()))
                .map_err(|error| error.to_string())?;
            worker.submit(&ticket).map_err(|error| error.to_string())?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_mins(3);
            let completion = loop {
                if let Some(completion) = worker.try_result().map_err(|error| error.to_string())? {
                    break completion;
                }
                if std::time::Instant::now() >= deadline {
                    return Err("game UI text preparation timed out".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            };
            let imported = completion.result.map_err(|failed| failed.error)?;
            catalog
                .complete(&completion.ticket, Ok(imported))
                .map_err(|error| error.to_string())?;
            catalog
                .snapshot(ticket.asset())
                .ok_or_else(|| "missing prepared UI publication".into())
        })();
        worker
            .close()
            .join()
            .map_err(|_| "UI import worker panicked")?;
        result
    }
    fn decode(
        plan: &UiTextPlan,
        project: &AuthoringProject,
        inputs: &mut ImportInputs,
    ) -> Result<Self, String> {
        let mut fonts = BTreeMap::new();
        for id in &plan.fonts {
            let font = project.decode_ui_font(&id.0, inputs)?;
            fonts.insert(id.clone(), font);
        }
        let runs =
            voxy_gameplay::prepare_ui_text(&plan.snapshot, |id| fonts.get(&AssetId(id.into())))?;
        Ok(Self {
            fonts,
            runs,
            observations: inputs.observations().clone(),
        })
    }
    pub(super) fn font_count(&self) -> usize {
        self.fonts.len()
    }
}

#[derive(Debug)]
pub(super) struct UiTextPlan {
    snapshot: voxy_gameplay::SceneUiSnapshot,
    fonts: BTreeSet<AssetId>,
}
impl UiTextPlan {
    pub(super) fn new(scene: &SceneGraph, viewport: [f32; 2]) -> Result<Self, String> {
        let snapshot = voxy_gameplay::extract_scene_ui(scene, viewport, 128)?;
        Self::from_snapshot(scene, snapshot)
    }
    /// Uses the caller's validated owned layout while retaining all font budgets.
    pub(super) fn from_snapshot(
        scene: &SceneGraph,
        snapshot: voxy_gameplay::SceneUiSnapshot,
    ) -> Result<Self, String> {
        let fonts: BTreeSet<_> = scene
            .components::<UiElement>()
            .filter_map(|(_, ui)| ui.text.as_ref().map(|text| AssetId(text.font.clone())))
            .collect();
        if fonts.len() > 8 {
            return Err("game UI font count budget".into());
        }
        Ok(Self { snapshot, fonts })
    }
    pub(super) fn worker(
        self,
        project: &AuthoringProject,
    ) -> Result<voxy_assets::AssetImportWorker<UiTextPreparation>, String> {
        let decoder_project = project.worker_project()?;
        voxy_assets::AssetImportWorker::new(
            project.input_provider()?,
            9,
            16 * 1024 * 1024,
            move |_, _, inputs| UiTextPreparation::decode(&self, &decoder_project, inputs),
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_gameplay::UiText;
    use voxy_scene::Transform;
    pub(super) fn font_bytes() -> Vec<u8> {
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("integration test requires a local Unicode TrueType font")
    }
    #[test]
    fn observed_logical_font_unicode_runs_share_atlas_and_invalid_font_retains_prior() {
        let root = std::env::temp_dir().join(format!(
            "voxy-ui-font-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let bytes = font_bytes();
        std::fs::write(root.join("font.ttf"), &bytes).unwrap();
        std::fs::write(
            root.join("assets.json"),
            r#"{"version":1,"assets":[{"asset":"interface","source":"font.ttf"}]}"#,
        )
        .unwrap();
        let project = AuthoringProject::new(
            &root,
            &crate::InputRecipe::Manifest(AssetId("assets.json".into())),
        )
        .unwrap();
        let mut scene = SceneGraph::new(2);
        let text = "Привет e\u{301}";
        for origin in [[0.0, 0.0], [0.5, 0.5]] {
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    UiElement {
                        origin,
                        size: [0.4; 2],
                        color: [1.0; 4],
                        layer: 0,
                        enabled: true,
                        action: Some("jump".into()),
                        text: Some(UiText {
                            font: "interface".into(),
                            content: text.into(),
                            size: 20.0,
                            color: [1.0; 4],
                        }),
                    },
                )
                .unwrap();
        }
        let prepared = UiTextPreparation::prepare(&scene, &project, [1000.0, 800.0]).unwrap();
        assert_eq!(prepared.font_count(), 1);
        assert_eq!(prepared.observations.len(), 2);
        assert_eq!(prepared.runs.len(), 2);
        assert!(std::sync::Arc::ptr_eq(
            &prepared.runs[0].run,
            &prepared.runs[1].run
        ));
        let run = &prepared.runs[0].run;
        assert!(!run.glyphs().is_empty());
        assert!(run.atlas().alpha().iter().any(|coverage| *coverage > 0));
        for glyph in run.glyphs() {
            assert!(text.is_char_boundary(usize::try_from(glyph.cluster).unwrap()));
        }
        std::fs::write(root.join("font.ttf"), b"broken font").unwrap();
        assert!(UiTextPreparation::prepare(&scene, &project, [1000.0, 800.0]).is_err());
        assert!(!prepared.runs[0].run.glyphs().is_empty());
        std::fs::write(root.join("font.ttf"), bytes).unwrap();
        let first = scene.nodes().next().unwrap().0;
        scene.set_active(first, false).unwrap();
        let inactive = UiTextPreparation::prepare(&scene, &project, [1000.0, 800.0]).unwrap();
        assert_eq!(inactive.font_count(), 1);
        assert_eq!(inactive.runs.len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn font_count_budget_rejects_before_any_file_reads() {
        let root = std::env::temp_dir();
        let project = AuthoringProject::new(
            &root,
            &crate::InputRecipe::Direct(voxy_assets::SourcePath::new("unused.obj").unwrap()),
        )
        .unwrap();
        let mut scene = SceneGraph::new(9);
        for i in 0..9 {
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    UiElement {
                        origin: [0.0; 2],
                        size: [1.0; 2],
                        color: [1.0; 4],
                        layer: 0,
                        enabled: true,
                        action: None,
                        text: Some(UiText {
                            font: format!("missing-{i}.ttf"),
                            content: "Label".into(),
                            size: 20.0,
                            color: [1.0; 4],
                        }),
                    },
                )
                .unwrap();
        }
        assert!(
            UiTextPreparation::prepare(&scene, &project, [1000.0, 800.0])
                .unwrap_err()
                .contains("font count budget")
        );
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use voxy_gameplay::UiText;
    use voxy_scene::Transform;
    #[test]
    fn repeated_font_content_still_counts_each_retained_input() {
        let root = std::env::temp_dir().join(format!(
            "voxy-ui-font-alias-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut bytes = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("integration requires local TrueType font");
        bytes.resize(4 * 1024 * 1024, 0);
        for i in 0..5 {
            std::fs::write(root.join(format!("font-{i}.ttf")), &bytes).unwrap();
        }
        let assets: Vec<_> = (0..5)
            .map(|i| serde_json::json!({"asset":format!("font-{i}"),"source":format!("font-{i}.ttf")}))
            .collect();
        std::fs::write(
            root.join("assets.json"),
            serde_json::to_vec(&serde_json::json!({"version":1,"assets":assets})).unwrap(),
        )
        .unwrap();
        let project = AuthoringProject::new(
            &root,
            &crate::InputRecipe::Manifest(AssetId("assets.json".into())),
        )
        .unwrap();
        project.import_ui_font("font-0").unwrap(); // Font parsing accepts valid SFNT with trailing padding.
        let mut scene = SceneGraph::new(5);
        for i in 0..5 {
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    UiElement {
                        origin: [0.0; 2],
                        size: [1.0; 2],
                        color: [1.0; 4],
                        layer: 0,
                        enabled: true,
                        action: None,
                        text: Some(UiText {
                            font: format!("font-{i}"),
                            content: "Label".into(),
                            size: 20.0,
                            color: [1.0; 4],
                        }),
                    },
                )
                .unwrap();
        }
        assert!(
            UiTextPreparation::prepare(&scene, &project, [1000.0, 800.0])
                .unwrap_err()
                .contains("font input")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    use voxy_assets::{AssetCatalog, AssetImportWorker, ImportCompletion};
    use voxy_gameplay::UiText;
    use voxy_scene::Transform;
    fn wait(
        worker: &mut AssetImportWorker<UiTextPreparation>,
    ) -> ImportCompletion<UiTextPreparation> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(completion) = worker.try_result().unwrap() {
                return completion;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "UI worker did not complete"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn worker_publication_failure_retains_old_text_and_source_observations() {
        let root = std::env::temp_dir().join(format!(
            "voxy-ui-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("font.ttf"), super::tests::font_bytes()).unwrap();
        let project = AuthoringProject::new(
            &root,
            &crate::InputRecipe::Direct(voxy_assets::SourcePath::new("unused.obj").unwrap()),
        )
        .unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: None,
                    text: Some(UiText {
                        font: "font.ttf".into(),
                        content: "Привет".into(),
                        size: 20.0,
                        color: [1.0; 4],
                    }),
                },
            )
            .unwrap();
        let mut worker = UiTextPlan::new(&scene, [1000.0, 800.0])
            .unwrap()
            .worker(&project)
            .unwrap();
        let id = AssetId("ui-publication".into());
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let ticket = catalog.request(id.clone()).unwrap();
        worker.submit(&ticket).unwrap();
        let completion = wait(&mut worker);
        catalog
            .complete(
                &completion.ticket,
                completion.result.map_err(|failed| failed.error),
            )
            .unwrap();
        let original = catalog.snapshot_with_revision(&id).unwrap();
        assert_eq!(original.1.value().runs[0].run.glyphs().len(), 6);
        std::fs::write(root.join("font.ttf"), b"invalid font").unwrap();
        let ticket = catalog.request(id.clone()).unwrap();
        worker.submit(&ticket).unwrap();
        let completion = wait(&mut worker);
        let failed = completion.result.unwrap_err();
        assert!(
            failed
                .inputs
                .observations()
                .contains_key(&AssetId("font.ttf".into()))
        );
        catalog
            .complete(&completion.ticket, Err(failed.error))
            .unwrap();
        let retained = catalog.snapshot_with_revision(&id).unwrap();
        assert_eq!(retained.0, original.0);
        assert!(std::sync::Arc::ptr_eq(&original.1, &retained.1));
        worker.close().join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
