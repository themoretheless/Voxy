//! A rendered rig can use clips from a separately packaged source model.
use super::{ProjectModel, error, model_upload::load_animation_source};
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::{AssetId, AssetLocations, ResourcePackage};
use voxy_model::{ModelAsset, ModelRetarget};
use wasm_bindgen::JsValue;

#[derive(Clone, Debug)]
pub(super) struct RetargetPlayback {
    pub(super) source: Arc<ModelAsset>,
    pub(super) target: Arc<ModelAsset>,
    pub(super) profile: ModelRetarget,
    pub(super) binding: Arc<voxy_animation::RetargetBinding>,
}

impl RetargetPlayback {
    pub(super) fn load(
        package: &ResourcePackage,
        assets: &AssetLocations,
        models: &BTreeMap<AssetId, ProjectModel>,
        animation_sources: &mut BTreeMap<AssetId, Arc<ModelAsset>>,
        profile: &ModelRetarget,
        target: &Arc<ModelAsset>,
    ) -> Result<Self, JsValue> {
        profile.validate().map_err(error)?;
        let id = AssetId(profile.source.clone());
        let source =
            if let Some(source) = models.get(&id).and_then(|model| model.animation.as_ref()) {
                source.clone()
            } else if let Some(source) = animation_sources.get(&id) {
                source.clone()
            } else {
                let path = assets
                    .source(&id)
                    .ok_or_else(|| error(format!("missing retarget source {}", profile.source)))?;
                let source = load_animation_source(package, path)?;
                animation_sources.insert(id, source.clone());
                source
            };
        let binding = profile.compile_models(&source, target).map_err(error)?;
        Ok(Self {
            source,
            target: target.clone(),
            profile: profile.clone(),
            binding: Arc::new(binding),
        })
    }
}
