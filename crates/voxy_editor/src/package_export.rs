//! Immutable source-free game and project package export.

use std::sync::Arc;
use voxy_assets::{FileInputs, SourcePath};
use voxy_scene::ComponentRegistry;

use crate::{
    App, GameHost, ModelSource, configured_app, editor_component_registry, import, package_models,
    project, ui_text,
};

/// Exports the validated scene/import dependency closure as an immutable package.
/// # Errors
/// Rejects changed inputs, invalid resources, quotas and existing output files.
pub fn export_game_package(
    source: &ModelSource,
    scene: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = configured_app(source, Some(scene), false)?;
    let launch = match source {
        ModelSource::File(path) => {
            serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":app.authoring.authoring_project.source_path(path)?.as_str(),"manifest":null})
        }
        ModelSource::Manifest { path, asset } => {
            serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":asset.0,"manifest":app.authoring.authoring_project.source_path(path)?.as_str()})
        }
    };
    export_app_package(app, launch, Vec::new(), output)
}

/// Export a project descriptor and its scene/assets into the existing package format.
/// # Errors
/// Rejects invalid projects, failed imports, changed observed sources, quotas and existing output.
pub fn export_project_package(
    path: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    export_project_package_with_components(path, output, editor_component_registry()?)
}

/// Export with the application's component codecs, shared with project runtime.
/// # Errors
/// Reports project, component, import, source consistency and output errors.
pub fn export_project_package_with_components(
    path: &std::path::Path,
    output: &std::path::Path,
    registry: ComponentRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    export_project_package_with_host(path, output, GameHost::new(registry))
}

/// Build using the same executable setup used for project Play and package launch.
/// Code remains in the host binary; the resource package stores data only.
/// # Errors
/// Reports invalid sources, game setup/import failures and output errors.
pub fn export_project_package_with_host(
    path: &std::path::Path,
    output: &std::path::Path,
    host: GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(host.components);
    let (opened, descriptor_text) = project::open_observed(path, &registry)?;
    let descriptor = SourcePath::new(
        path.canonicalize()?
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("descriptor must have a UTF-8 name")?,
    )?;
    let manifest = SourcePath::new(
        opened
            .descriptor
            .asset_manifest
            .to_str()
            .ok_or("manifest must be UTF-8")?,
    )?;
    let manifest_bytes =
        FileInputs::new(&opened.root)?.read(&manifest.observation_id(), import::MANIFEST_BYTES)?;
    let locations = import::manifest_locations(&manifest_bytes)?;
    // Keep every declared source: reopening a project validates the complete manifest.
    // Referenced models/audio/fonts additionally contribute their observed import closure.
    let mut extra: Vec<_> = locations
        .bindings()
        .map(|(_, source)| (source.clone(), None))
        .collect();
    extra.push((descriptor.clone(), Some(descriptor_text.into_bytes())));
    extra.push((manifest, Some(manifest_bytes)));
    let mut app = App::from_open_project(&opened, registry)?;
    app.behavior_setup = host.behaviors;
    app.behavior_checkpoints = host.behavior_checkpoints;
    app.ui_action_checkpoints = host.ui_action_checkpoints;
    app.ui_action_setup = host.ui_actions;
    let launch = serde_json::json!({"version":2,"project":descriptor.as_str()});
    export_app_package(app, launch, extra, output)
}

fn export_app_package(
    mut app: App,
    launch: serde_json::Value,
    mut extra: Vec<(SourcePath, Option<Vec<u8>>)>,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = (|| {
        extra.extend(package_models::declared_sources(&app)?);
        if let Some(prepared) = app
            .authoring
            .authoring_project
            .liquid_runtime_observed(&app.scene)?
        {
            for (id, observation) in prepared.inputs.observations() {
                let snapshot = observation
                    .as_ref()
                    .map_err(|error| format!("liquid package input: {error:?}"))?;
                extra.push((
                    SourcePath::new(id.0.clone())?,
                    Some(snapshot.bytes.to_vec()),
                ));
            }
        }
        app.start_standalone()?;
        let authored = app
            .authoring
            .authoring_source
            .as_ref()
            .ok_or("packaging requires a project-scoped scene")?;
        let mut observations = authored.inputs().observations().clone();
        for asset in app.required_cpu_model_assets() {
            let imported = app
                .catalog
                .snapshot(&asset)
                .ok_or("missing packaged model")?;
            for (id, input) in imported.inputs().observations() {
                if observations
                    .get(id)
                    .is_some_and(|previous| previous != input)
                {
                    return Err("inconsistent dependency revisions during package export".into());
                }
                observations.insert(id.clone(), input.clone());
            }
        }
        if let Some(audio) = app.audio.play() {
            for (id, input) in audio.input_observations(&app.scene)? {
                if observations
                    .get(&id)
                    .is_some_and(|previous| previous != &input)
                {
                    return Err(
                        "inconsistent audio dependency revisions during package export".into(),
                    );
                }
                observations.insert(id, input);
            }
        }
        let text = ui_text::UiTextPreparation::prepare_import(
            &app.scene,
            &app.authoring.authoring_project,
            [1280.0, 720.0],
        )?;
        for (id, input) in &text.value().observations {
            if observations
                .get(id)
                .is_some_and(|previous| previous != input)
            {
                return Err(
                    "inconsistent UI font dependency revisions during package export".into(),
                );
            }
            observations.insert(id.clone(), input.clone());
        }
        let mut paths: Vec<_> = observations
            .keys()
            .map(|id| voxy_assets::SourcePath::new(id.0.clone()))
            .collect::<Result<_, _>>()?;
        paths.extend(extra.iter().map(|(source, _)| source.clone()));
        paths.sort();
        paths.dedup();
        let package = app
            .authoring
            .authoring_project
            .capture_package(&paths, &serde_json::to_vec(&launch)?)?;
        for (id, original) in observations {
            if package.inputs().observations().get(&id) != Some(&original) {
                return Err(format!("package source {} changed after validation", id.0).into());
            }
        }
        for (source, expected) in extra {
            if let Some(expected) = expected
                && package
                    .value()
                    .read(&source.observation_id(), 64 * 1024 * 1024)?
                    != expected
            {
                return Err(format!(
                    "project input {} changed during package export",
                    source.as_str()
                )
                .into());
            }
        }
        let bytes = package.value().to_bytes(voxy_assets::PackageLimits {
            max_entries: 4096,
            max_payload_bytes: 64 * 1024 * 1024,
            max_document_bytes: 256 * 1024 * 1024,
        })?;
        voxy_assets::save_atomic_new_file(output, &bytes, 256 * 1024 * 1024)?;
        println!(
            "GAME PACKAGE PASS sources={} bytes={}",
            paths.len(),
            bytes.len()
        );
        Ok(())
    })();
    app.stop_workers()?;
    result
}
