//! Build the dependency closure of declared models and prefabs.
use crate::*;

// Use the same recipe, decoder, input quotas and final source validation as the
// background import worker. Decoded models are dropped one at a time; package
// publication revalidates every captured byte through its existing extra inputs.
pub(super) fn declared_sources(
    app: &App,
) -> Result<Vec<(SourcePath, Option<Vec<u8>>)>, Box<dyn std::error::Error>> {
    if !matches!(app.import_recipe, InputRecipe::Manifest(_)) {
        return Ok(Vec::new());
    }
    let required = app.required_cpu_model_assets();
    let provider = app.authoring.authoring_project.input_provider()?;
    let mut captured = CapturedSources::new(4096, 64 * 1024 * 1024);
    for asset in app.available.difference(&required) {
        let accepted = decode_model(app, &provider, asset, drop)?;
        captured.retain(accepted.inputs())?;
    }
    for asset in app.authoring.authoring_project.prefabs()? {
        let imported = app
            .authoring
            .authoring_project
            .load_prefab(&asset)
            .map_err(|error| format!("declared prefab {}: {error}", asset.0))?;
        if imported.value().expanded.objects.iter().any(|object| {
            object.components.contains_key("game.liquid-source.v1")
                || object.components.contains_key("game.ui-element.v1")
        }) {
            let loaded = imported.value().expanded.load(
                &app.authoring.authoring_project.registry,
                scene_limits::OBJECTS,
            )?;
            if let Some(materials) = app
                .authoring
                .authoring_project
                .liquid_materials_observed(&loaded.graph)?
            {
                captured.retain(materials.inputs())?;
            }
            if loaded
                .graph
                .components::<voxy_gameplay::UiElement>()
                .next()
                .is_some()
            {
                // Same bounded text preparation, glyph/layout admission and
                // build viewport as the ordinary startup scene exporter.
                let text = ui_text::UiTextPreparation::prepare_import(
                    &loaded.graph,
                    &app.authoring.authoring_project,
                    [1280., 720.],
                )?;
                captured.retain(text.inputs())?;
            }
        }
        let mut audio_recipes = BTreeMap::new();
        for object in &imported.value().expanded.objects {
            if let Some(value) = object.components.get("game.audio-source.v1") {
                let source: voxy_gameplay::AudioSource = serde_json::from_value(value.clone())?;
                if !source.valid() {
                    return Err(format!(
                        "declared prefab {}: object {} invalid audio source",
                        asset.0, object.id.0
                    )
                    .into());
                }
                if audio_recipes
                    .insert(source.asset, source.import_settings.clone())
                    .is_some_and(|previous| previous != source.import_settings)
                {
                    return Err("conflicting audio settings for one logical asset".into());
                }
            }
        }
        for (audio, settings) in audio_recipes {
            // Use the native offline mix rate and original decoder/settings caps.
            let accepted =
                app.authoring
                    .authoring_project
                    .import_audio(&audio, 48000, settings.as_deref())?;
            captured.retain(accepted.inputs())?;
        }
        // Inspect native expanded objects so nested instance overrides cannot
        // bypass binding checks. Editor repair/last-good policies remain separate.
        for object in &imported.value().expanded.objects {
            if let Some(model) = object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
                && !app.available.contains(&AssetId(model.into()))
            {
                return Err(format!(
                    "declared prefab {}: object {} references unavailable model {model:?}",
                    asset.0, object.id.0
                )
                .into());
            }
        }
        for object in &imported.value().expanded.objects {
            if let (Some(value), Some(model)) = (
                object.components.get("editor.model-part.v1"),
                object
                    .components
                    .get("editor.model.v1")
                    .and_then(serde_json::Value::as_str),
            ) {
                let part: ModelPart = serde_json::from_value(value.clone())?;
                if part.node != u32::MAX {
                    let model_id = AssetId(model.into());
                    let nodes = decode_model(app, &provider, &model_id, |asset| asset.nodes.len())?;
                    if usize::try_from(part.node)? >= *nodes.value() {
                        return Err(format!("declared prefab {}: object {} has invalid imported model part {} for {model:?}", asset.0, object.id.0, part.node).into());
                    }
                    captured.retain(nodes.inputs())?;
                }
            }
        }
        for object in &imported.value().expanded.objects {
            if let Some(value) = object.components.get("editor.model-retarget.v1") {
                let profile: ModelRetarget = serde_json::from_value(value.clone())?;
                profile.validate()?;
                let source = AssetId(profile.source.clone());
                if !app.available.contains(&source) {
                    return Err(format!(
                        "declared prefab {}: object {} references unavailable retarget source {:?}",
                        asset.0, object.id.0, source.0
                    )
                    .into());
                }
                let target = AssetId(
                    object
                        .components
                        .get("editor.model.v1")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("retarget profile requires a model target")?
                        .into(),
                );
                let target_asset = decode_model(app, &provider, &target, std::convert::identity)?;
                captured.retain(target_asset.inputs())?;
                let source_asset = if source == target {
                    None
                } else {
                    let imported = decode_model(app, &provider, &source, std::convert::identity)?;
                    captured.retain(imported.inputs())?;
                    Some(imported)
                };
                let source_asset = source_asset.as_ref().unwrap_or(&target_asset);
                profile
                    .compile_models(
                        source_asset
                            .value()
                            .animated
                            .as_deref()
                            .ok_or("retarget source requires an animated model")?,
                        target_asset
                            .value()
                            .animated
                            .as_deref()
                            .ok_or("retarget target requires an animated model")?,
                    )
                    .map_err(|error| {
                        format!(
                            "declared prefab {}: object {} retarget binding: {error}",
                            asset.0, object.id.0
                        )
                    })?;
            }
        }
        for object in &imported.value().expanded.objects {
            if object.components.contains_key("editor.model-locomotion.v1")
                && object.components.contains_key("editor.model-part.v1")
            {
                return Err(format!("declared prefab {}: object {} locomotion requires a logical model owner, not an imported hierarchy part", asset.0, object.id.0).into());
            }
            if object.components.contains_key("editor.model-locomotion.v1")
                && !object.components.contains_key("editor.model-animation.v1")
            {
                return Err("locomotion requires a model animation component".into());
            }
            if let Some(value) = object.components.get("editor.model-animation.v1") {
                let settings: ModelAnimation = serde_json::from_value(value.clone())?;
                let target = object
                    .components
                    .get("editor.model.v1")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("animation requires a model owner")?;
                let source = object
                    .components
                    .get("editor.model-retarget.v1")
                    .map(|value| serde_json::from_value::<ModelRetarget>(value.clone()))
                    .transpose()?
                    .map_or_else(|| target.to_owned(), |profile| profile.source);
                // Morph overrides belong to the displayed target geometry even
                // when the playback clips come from a separate retarget source.
                if !settings.morph_weights.is_empty() {
                    let target_id = AssetId(target.to_owned());
                    let target_asset =
                        decode_model(app, &provider, &target_id, std::convert::identity)?;
                    captured.retain(target_asset.inputs())?;
                    let model = target_asset
                        .value()
                        .animated
                        .as_ref()
                        .ok_or("authored morph weights require a morph-capable model")?;
                    let mut frame = model.bind_frame()?;
                    settings.apply_morph_weights(model, &mut frame)?;
                    model.validate_morph_frame(&frame)?;
                }
                let source = AssetId(source);
                let decoded = decode_model(app, &provider, &source, std::convert::identity)?;
                captured.retain(decoded.inputs())?;
                if let Some(value) = object.components.get("editor.model-locomotion.v1") {
                    let controller: ModelLocomotion = serde_json::from_value(value.clone())?;
                    controller
                        .validate_model(
                            decoded
                                .value()
                                .animated
                                .as_deref()
                                .ok_or("locomotion requires an animated model")?,
                        )
                        .map_err(|error| {
                            format!(
                                "declared prefab {}: object {} locomotion: {error}",
                                asset.0, object.id.0
                            )
                        })?;
                }
                if let Some(model) = &decoded.value().animated {
                    let _ = crate::model_playback::ModelPlayback::new(model.clone(), settings)
                        .map_err(|error| {
                            format!(
                                "declared prefab {}: object {} animation: {error}",
                                asset.0, object.id.0
                            )
                        })?;
                } else {
                    settings.validate(Some(0), Some(0))?;
                    if !settings.clip_name.is_empty() {
                        return Err("named animation clip requires an animated model".into());
                    }
                }
            }
        }
        captured.retain(imported.inputs())?;
    }
    captured
        .sources
        .into_iter()
        .map(|(id, input)| Ok((SourcePath::new(id.0)?, Some(input.bytes.to_vec()))))
        .collect()
}

fn decode_model<T>(
    app: &App,
    provider: &FileInputs,
    asset: &AssetId,
    prepare: impl FnOnce(import::EditorAsset) -> T,
) -> Result<ImportedAsset<T>, Box<dyn std::error::Error>> {
    let mut inputs = voxy_assets::ImportInputs::new(34, 32 * 1024 * 1024);
    let decoded = app
        .import_recipe
        .load(asset, provider, &mut inputs)
        .map_err(|error| format!("declared model {}: {error}", asset.0))?;
    inputs
        .finish(prepare(decoded), |id, limit| provider.read(id, limit))
        .map_err(|error| format!("declared model {} inputs changed: {error:?}", asset.0).into())
}

// Bounds retained source payloads before cloning another import's observations.
// This is the package payload ceiling, not a decoded-model or total heap bound.
struct CapturedSources {
    sources: BTreeMap<AssetId, voxy_assets::InputSnapshot>,
    bytes: usize,
    max_sources: usize,
    max_bytes: usize,
}
impl CapturedSources {
    fn new(max_sources: usize, max_bytes: usize) -> Self {
        Self {
            sources: BTreeMap::new(),
            bytes: 0,
            max_sources,
            max_bytes,
        }
    }
    fn retain(
        &mut self,
        inputs: &voxy_assets::ImportInputs,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut count = self.sources.len();
        let mut bytes = self.bytes;
        for (id, input) in inputs.observations() {
            let input = input
                .as_ref()
                .map_err(|error| format!("declared package input: {error:?}"))?;
            if let Some(previous) = self.sources.get(id) {
                if previous != input {
                    return Err("inconsistent declared asset dependency revisions".into());
                }
            } else {
                count = count
                    .checked_add(1)
                    .ok_or("declared package source count overflow")?;
                bytes = bytes
                    .checked_add(input.bytes.len())
                    .ok_or("declared package byte count overflow")?;
            }
            if count > self.max_sources || bytes > self.max_bytes {
                return Err("declared package source budget exceeded".into());
            }
        }
        // All checks precede mutation; snapshots retain original shared payloads.
        for (id, input) in inputs.observations() {
            self.sources
                .entry(id.clone())
                .or_insert_with(|| input.as_ref().unwrap().clone());
        }
        self.bytes = bytes;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inputs(items: &[(&str, &[u8])]) -> voxy_assets::ImportInputs {
        let mut inputs = voxy_assets::ImportInputs::new(8, 64);
        for (id, bytes) in items {
            inputs
                .read(AssetId((*id).into()), |_, _| Ok(bytes.to_vec()))
                .unwrap();
        }
        inputs
    }
    #[test]
    fn aggregate_sources_deduplicate_and_admit_exact_limits_without_copying_payloads() {
        let first = inputs(&[("shared", &[1, 2])]);
        let second = inputs(&[("shared", &[1, 2]), ("other", &[3])]);
        let mut retained = CapturedSources::new(2, 3);
        retained.retain(&first).unwrap();
        let original = retained.sources[&AssetId("shared".into())].bytes.clone();
        retained.retain(&second).unwrap();
        retained.retain(&second).unwrap();
        assert_eq!(retained.bytes, 3);
        assert_eq!(retained.sources.len(), 2);
        assert!(Arc::ptr_eq(
            &original,
            &retained.sources[&AssetId("shared".into())].bytes
        ));
        assert!(Arc::ptr_eq(
            &original,
            &first.observations()[&AssetId("shared".into())]
                .as_ref()
                .unwrap()
                .bytes
        ));
    }
    #[test]
    fn aggregate_count_bytes_conflicts_and_failed_reads_reject_without_partial_retention() {
        for (count, bytes, candidate) in [
            (1, 8, inputs(&[("a", &[1]), ("b", &[2])])),
            (3, 1, inputs(&[("a", &[1]), ("b", &[2])])),
            (3, 8, inputs(&[("a", &[9]), ("b", &[2])])),
        ] {
            let mut retained = CapturedSources::new(count, bytes);
            retained.retain(&inputs(&[("a", &[1])])).unwrap();
            let before = retained.sources.clone();
            assert!(retained.retain(&candidate).is_err());
            assert_eq!(retained.sources, before);
            assert_eq!(retained.bytes, 1);
        }
        let mut failed = voxy_assets::ImportInputs::new(2, 8);
        failed
            .read(AssetId("a".into()), |_, _| Ok(vec![1]))
            .unwrap();
        assert!(
            failed
                .read(AssetId("z".into()), |_, _| Err("unavailable".into()))
                .is_err()
        );
        let mut retained = CapturedSources::new(2, 8);
        assert!(retained.retain(&failed).is_err());
        assert!(retained.sources.is_empty());
        assert_eq!(retained.bytes, 0);
    }
}
