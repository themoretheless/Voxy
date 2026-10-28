//! Blocking preparation/publication for a model-source relink. Run on an IO
//! worker, never from frame-critical input or drawing. Scene references stay IDs.
use super::*;
use std::sync::Arc;

pub(crate) struct PreparedModelRelink {
    project: Arc<()>,
    manifest: AssetId,
    locations: AssetLocations,
    candidate: ImportedAsset<crate::import::EditorAsset>,
    authoring_before: Option<ImportInputs>,
    authoring_after: Option<ImportedAsset<AuthoredScene>>,
}
pub(crate) struct RelinkPublication {
    pub(crate) authoring: Option<ImportedAsset<AuthoredScene>>,
}
impl PreparedModelRelink {
    /// Exposes the decoded model for the original scene/GPU compatibility gates.
    pub(crate) fn candidate(&self) -> &crate::import::EditorAsset {
        self.candidate.value()
    }
}
impl AuthoringProject {
    /// Stage a native manifest binding plus the fully decoded candidate. No file
    /// or live owner is modified. Existing IDs relocate; removed IDs are rebound.
    pub(crate) fn prepare_model_relink(
        &self,
        asset: &AssetId,
        source: &SourcePath,
    ) -> Result<PreparedModelRelink, EditorError> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or("model relink requires a manifest project")?;
        let mut inputs = ImportInputs::new(35, 32 * 1024 * 1024 + 65536);
        let bytes = inputs
            .read(manifest.clone(), |id, limit| {
                self.provider.read(id, limit.min(65536))
            })
            .map_err(|e| format!("relink manifest: {e:?}"))?;
        let mut locations =
            AssetLocations::from_json(std::str::from_utf8(&bytes.bytes)?, 128, 65536, 65536)?;
        if locations.source(asset).is_some() {
            locations.relocate(asset, source.clone())?;
        } else {
            locations.bind(asset.clone(), source.clone())?;
        }
        // Admit serialized limits before decoding, so a candidate can be published
        // with the original manifest encoder without dropping unrelated entries.
        locations.to_json(65536)?;
        let model = crate::import::load(source, &self.provider, &mut inputs)?;
        let candidate = inputs
            .finish(model, |id, limit| self.provider.read(id, limit))
            .map_err(|e| format!("relink source changed: {e:?}"))?;
        Ok(PreparedModelRelink {
            project: self.observation_token(),
            manifest: manifest.clone(),
            locations,
            candidate,
            authoring_before: None,
            authoring_after: None,
        })
    }
    /// Prepare the matching authoring receipt for our own manifest edit, retaining
    /// source/expanded/dependency payloads and every non-manifest observation.
    pub(crate) fn attach_relink_authoring(
        &self,
        prepared: &mut PreparedModelRelink,
        asset: &AssetId,
        source: Option<ImportedAsset<AuthoredScene>>,
    ) -> Result<(), EditorError> {
        let Some(source) = source else {
            return Ok(());
        };
        if source.value().dependencies.contains_key(&asset.0) {
            return Err("model relink would replace a prefab dependency".into());
        }
        let before = prepared
            .candidate
            .inputs()
            .observations()
            .get(&prepared.manifest)
            .and_then(|input| input.as_ref().ok())
            .ok_or("missing relink manifest observation")?;
        let bytes = prepared.locations.to_json(65536)?.into_bytes();
        let after = voxy_assets::InputSnapshot {
            digest: *blake3::hash(&bytes).as_bytes(),
            bytes: bytes.into(),
        };
        let refreshed = source
            .prepare_input_replacement(&prepared.manifest, before, after, |id, limit| {
                self.provider.read(id, limit)
            })
            .map_err(|e| format!("relink authoring inputs: {e:?}"))?;
        prepared.authoring_before = Some(source.inputs().clone());
        prepared.authoring_after = Some(refreshed);
        Ok(())
    }
    /// Revalidate the original manifest and every decoded dependency before its
    /// atomic replacement. No renderer or catalog is swapped here: ordinary
    /// invalidation/import/GPU admission publishes the resource afterwards.
    /// Trusted local files only; validation+rename is not a cross-writer CAS.
    pub(crate) fn publish_model_relink(
        &self,
        prepared: PreparedModelRelink,
    ) -> Result<RelinkPublication, EditorError> {
        if !Arc::ptr_eq(&prepared.project, &self.observation_token())
            || self.manifest.as_ref() != Some(&prepared.manifest)
        {
            return Err("relink belongs to another project".into());
        }
        prepared
            .candidate
            .inputs()
            .validate(|id, limit| self.provider.read(id, limit))
            .map_err(|e| format!("relink inputs changed before publication: {e:?}"))?;
        if let Some(inputs) = &prepared.authoring_before {
            inputs
                .validate(|id, limit| self.provider.read(id, limit))
                .map_err(|e| format!("relink authoring changed before publication: {e:?}"))?;
        }
        prepared
            .locations
            .save_manifest(&self.root.join(&prepared.manifest.0), 65536)
            .map_err(|e| format!("relink manifest publication: {e}"))?;
        Ok(RelinkPublication {
            authoring: prepared.authoring_after,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::*;
    pub(crate) struct Fixture(PathBuf);
    impl Fixture {
        pub(crate) fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "voxy-model-relink-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            project::create(&root.join("game"), "Relink").unwrap();
            let fixture = Self(root);
            fixture.write_model("old.obj", 0);
            fixture.write_model("new.obj", 2);
            fixture.write_model("spare.obj", 3);
            std::fs::write(fixture.root().join("assets.json"), br#"{"version":1,"assets":[{"asset":"character","source":"old.obj"},{"asset":"spare","source":"spare.obj"}]}"#).unwrap();
            fixture
        }
        pub(crate) fn root(&self) -> PathBuf {
            self.0.join("game")
        }
        fn write_model(&self, name: &str, x: i32) {
            std::fs::write(
                self.root().join(name),
                format!("v {x} 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"),
            )
            .unwrap();
        }
        pub(crate) fn app(&self) -> App {
            App::from_project(&self.root().join("project.voxy")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn await_model(
        app: &mut App,
        id: &AssetId,
        previous: Option<&Arc<ImportedAsset<crate::import::EditorAsset>>>,
    ) {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            app.tick().unwrap();
            if app
                .catalog
                .snapshot(id)
                .is_some_and(|a| previous.is_none_or(|old| !Arc::ptr_eq(old, &a)))
            {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn prepared_relink_preserves_id_scene_and_unrelated_bindings_through_normal_reload() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        app.edit_key(KeyCode::KeyD).unwrap();
        let id = AssetId("character".into());
        await_model(&mut app, &id, None);
        let document = app.authoring_document().unwrap();
        let old = app.catalog.snapshot(&id).unwrap();
        let manifest_path = fixture.root().join("assets.json");
        let original = std::fs::read(&manifest_path).unwrap();
        let worker = app.authoring.authoring_project.worker_project().unwrap();
        let prepared = worker
            .prepare_model_relink(&id, &SourcePath::new("new.obj").unwrap())
            .unwrap();
        app.validate_reload_candidate(&id, prepared.candidate())
            .unwrap();
        assert_eq!(prepared.candidate().mesh.vertices()[0].position[0], 2.);
        assert_eq!(std::fs::read(&manifest_path).unwrap(), original);
        assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
        app.authoring
            .authoring_project
            .publish_model_relink(prepared)
            .unwrap();
        let locations = AssetLocations::load_manifest(&manifest_path, 128, 65536, 65536).unwrap();
        assert_eq!(locations.source(&id).unwrap().as_str(), "new.obj");
        assert_eq!(
            locations.source(&AssetId("spare".into())).unwrap().as_str(),
            "spare.obj"
        );
        let until = Instant::now() + Duration::from_secs(10);
        while !app.catalog.snapshot(&id).is_some_and(|asset| {
            asset
                .inputs()
                .observations()
                .contains_key(&AssetId("new.obj".into()))
        }) {
            app.tick().unwrap();
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
        assert_eq!(
            app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
            2.
        );
        assert_eq!(app.authoring_document().unwrap(), document);
        app.stop_workers().unwrap();
    }
    #[test]
    fn relink_rejects_changed_inputs_foreign_owner_and_invalid_sources_without_manifest_write() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let owner = &app.authoring.authoring_project;
        let id = AssetId("character".into());
        let path = fixture.root().join("assets.json");
        let original = std::fs::read(&path).unwrap();
        for source in ["missing.obj", "../outside.obj", "assets.json", "spare.obj"] {
            let result = SourcePath::new(source)
                .map_err(EditorError::from)
                .and_then(|source| owner.prepare_model_relink(&id, &source));
            assert!(result.is_err(), "accepted {source}");
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
        let source = SourcePath::new("new.obj").unwrap();
        let prepared = owner.prepare_model_relink(&id, &source).unwrap();
        std::fs::write(fixture.root().join("new.obj"), b"broken after decode").unwrap();
        assert!(owner.publish_model_relink(prepared).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        fixture.write_model("new.obj", 2);
        let prepared = owner.prepare_model_relink(&id, &source).unwrap();
        let mut external = original.clone();
        external.push(b' ');
        std::fs::write(&path, &external).unwrap();
        assert!(owner.publish_model_relink(prepared).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), external);
        std::fs::write(&path, &original).unwrap();
        let prepared = owner.prepare_model_relink(&id, &source).unwrap();
        let foreign = AuthoringProject::with_registry(
            &fixture.root(),
            &InputRecipe::Manifest(AssetId("assets.json".into())),
            owner.registry.clone(),
        )
        .unwrap();
        assert!(foreign.publish_model_relink(prepared).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        app.stop_workers().unwrap();
    }
    #[test]
    fn authoring_receipt_refresh_rejects_external_inputs_and_never_rebinds_prefab_ids() {
        let temp = crate::project_prefab_tests::Temp::new();
        let (path, _) = temp.setup().unwrap();
        let root = path.parent().unwrap();
        std::fs::write(
            root.join("replacement.obj"),
            "v 2 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let mut app = App::from_project(&path).unwrap();
        let source = app.authoring.authoring_source.clone().unwrap();
        let owner = &app.authoring.authoring_project;
        let asset = AssetId("mesh".into());
        let replacement = SourcePath::new("replacement.obj").unwrap();
        let manifest = std::fs::read(root.join("assets.json")).unwrap();
        let leaf = std::fs::read(root.join("leaf.prefab")).unwrap();
        let mut prepared = owner.prepare_model_relink(&asset, &replacement).unwrap();
        owner
            .attach_relink_authoring(&mut prepared, &asset, Some(source.clone()))
            .unwrap();
        let mut external_leaf = leaf.clone();
        external_leaf.push(b' ');
        std::fs::write(root.join("leaf.prefab"), &external_leaf).unwrap();
        assert!(owner.publish_model_relink(prepared).is_err());
        assert_eq!(std::fs::read(root.join("assets.json")).unwrap(), manifest);
        assert_eq!(
            std::fs::read(root.join("leaf.prefab")).unwrap(),
            external_leaf
        );
        std::fs::write(root.join("leaf.prefab"), &leaf).unwrap();
        let mut external_manifest = manifest.clone();
        external_manifest.push(b' ');
        std::fs::write(root.join("assets.json"), &external_manifest).unwrap();
        let mut prepared = owner.prepare_model_relink(&asset, &replacement).unwrap();
        assert!(
            owner
                .attach_relink_authoring(&mut prepared, &asset, Some(source.clone()))
                .is_err()
        );
        assert_eq!(
            std::fs::read(root.join("assets.json")).unwrap(),
            external_manifest
        );
        std::fs::write(root.join("assets.json"), &manifest).unwrap();
        let mut prepared = owner.prepare_model_relink(&asset, &replacement).unwrap();
        owner
            .attach_relink_authoring(&mut prepared, &asset, Some(source))
            .unwrap();
        let refreshed = owner
            .publish_model_relink(prepared)
            .unwrap()
            .authoring
            .unwrap();
        owner.validate(&refreshed).unwrap();
        std::fs::write(root.join("leaf.prefab"), &external_leaf).unwrap();
        assert!(owner.validate(&refreshed).is_err());
        std::fs::write(
            root.join("other.obj"),
            "v 3 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let prefab = AssetId("leaf".into());
        let mut prepared = owner
            .prepare_model_relink(&prefab, &SourcePath::new("other.obj").unwrap())
            .unwrap();
        let error = owner
            .attach_relink_authoring(&mut prepared, &prefab, Some(refreshed))
            .unwrap_err();
        assert!(error.to_string().contains("prefab dependency"));
        app.stop_workers().unwrap();
    }
}
