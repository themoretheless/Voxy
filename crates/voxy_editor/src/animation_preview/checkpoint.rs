//! Immutable accepted preview participant; resource keys belong to the outer graph.
use super::*;
use crate::EditorError;
use serde::{Deserialize, Serialize};
use voxy_scene::{ObjectId, SceneCheckpointView};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewCheckpoint {
    version: u32,
    owner: ObjectId,
    phase: u64,
    model: String,
    source: String,
    settings: crate::ModelAnimation,
    profile: Option<crate::ModelRetarget>,
    frame: voxy_animation::AnimatorFrameCheckpoint,
}
pub(crate) struct PreparedPreview {
    owner: ObjectId,
    phase: f64,
    model: Arc<ModelAsset>,
    source: Arc<ModelAsset>,
    settings: crate::ModelAnimation,
    profile: Option<crate::ModelRetarget>,
    frame: Arc<voxy_animation::AnimatorFrame>,
}
impl PreparedPreview {
    pub(crate) fn model_roots(&self) -> Vec<Arc<ModelAsset>> {
        let mut roots = vec![self.model.clone()];
        if !Arc::ptr_eq(&self.model, &self.source) {
            roots.push(self.source.clone());
        }
        roots
    }
    pub(crate) fn bind(self, view: &SceneCheckpointView<'_>) -> Result<Preview, EditorError> {
        Ok(Preview {
            owner: view
                .reserved_node(&self.owner)
                .ok_or("missing preview owner")?,
            phase: self.phase,
            model: self.model,
            source: self.source,
            settings: self.settings,
            profile: self.profile,
            frame: self.frame,
        })
    }
    pub(crate) fn bind_resolved(
        self,
        scene: &voxy_scene::SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<voxy_scene::NodeId>,
    ) -> Result<Preview, EditorError> {
        let owner = resolve(&self.owner).ok_or("missing preview owner")?;
        let view = SceneCheckpointView::new(scene, [(self.owner.clone(), owner)])?;
        self.bind(&view)
    }
}
impl Preview {
    pub(crate) fn accepted_frame(&self) -> &Arc<voxy_animation::AnimatorFrame> {
        &self.frame
    }
    pub(crate) fn restore_shared_frame(
        &mut self,
        frame: Arc<voxy_animation::AnimatorFrame>,
        max: usize,
    ) -> Result<(), EditorError> {
        self.model.validate_morph_frame(&frame)?;
        let encode = |frame: &voxy_animation::AnimatorFrame| {
            frame
                .checkpoint(&self.model.skeleton, max)
                .and_then(|saved| saved.to_json(max))
                .map_err(|error| error.to_string())
        };
        if encode(&self.frame)? != encode(&frame)? {
            return Err("shared preview frame contents differ".into());
        }
        self.frame = frame;
        Ok(())
    }
    /// Canonical allocation roots for the original shared model graph.
    pub(crate) fn model_roots(&self) -> Vec<Arc<ModelAsset>> {
        let mut roots = vec![self.model.clone()];
        if !Arc::ptr_eq(&self.model, &self.source) {
            roots.push(self.source.clone());
        }
        roots
    }
    pub(crate) fn checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        identify: impl Fn(&Arc<ModelAsset>) -> Option<String>,
        max: usize,
    ) -> Result<PreviewCheckpoint, EditorError> {
        if self.profile.is_none() && !Arc::ptr_eq(&self.model, &self.source) {
            return Err("preview source differs without a retarget profile".into());
        }
        self.model.validate_morph_frame(&self.frame)?;
        let saved = PreviewCheckpoint {
            version: 1,
            owner: view
                .reserved_identity(self.owner)
                .cloned()
                .ok_or("unidentified preview owner")?,
            phase: self.phase.to_bits(),
            model: identify(&self.model).ok_or("unidentified preview model")?,
            source: identify(&self.source).ok_or("unidentified preview source")?,
            settings: self.settings.clone(),
            profile: self.profile.clone(),
            frame: self
                .frame
                .checkpoint(&self.model.skeleton, max)
                .map_err(|e| e.to_string())?,
        };
        saved.to_json(max)?;
        if (saved.model == saved.source) != Arc::ptr_eq(&self.model, &self.source) {
            return Err("preview root keys do not preserve model aliases".into());
        }
        Ok(saved)
    }
}
impl PreviewCheckpoint {
    pub(crate) fn to_json(&self, max: usize) -> Result<String, EditorError> {
        let phase = f64::from_bits(self.phase);
        if self.version != 1
            || !phase.is_finite()
            || !(0. ..=1.).contains(&phase)
            || self.owner.0.is_empty()
            || self.model.is_empty()
            || self.source.is_empty()
        {
            return Err("invalid preview checkpoint header".into());
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max {
            return Err("preview checkpoint byte budget".into());
        }
        Ok(text)
    }
    pub(crate) fn from_json(text: &str, max: usize) -> Result<Self, EditorError> {
        if text.len() > max {
            return Err("preview checkpoint byte budget".into());
        }
        let saved: Self = serde_json::from_str(text)?;
        saved.to_json(max)?;
        Ok(saved)
    }
    /// Resolves the original target/source roots. Never resamples a clip or clock.
    pub(crate) fn prepare(
        &self,
        view: &SceneCheckpointView<'_>,
        resolve: impl Fn(&str) -> Option<Arc<ModelAsset>>,
        max: usize,
    ) -> Result<Preview, EditorError> {
        self.prepare_unbound(resolve, max)?.bind(view)
    }
    /// Resource validation precedes scene binding and never samples a pose.
    pub(crate) fn prepare_unbound(
        &self,
        resolve: impl Fn(&str) -> Option<Arc<ModelAsset>>,
        max: usize,
    ) -> Result<PreparedPreview, EditorError> {
        self.to_json(max)?;
        let model = resolve(&self.model).ok_or("missing preview model root")?;
        let source = resolve(&self.source).ok_or("missing preview source root")?;
        if (self.model == self.source) != Arc::ptr_eq(&model, &source) {
            return Err("resolved preview model aliases differ".into());
        }
        if self.profile.is_none() && !Arc::ptr_eq(&model, &source) {
            return Err("preview source differs without a retarget profile".into());
        }
        self.settings.validate(
            Some(source.animations.len()),
            Some(source.skeleton.joints().len()),
        )?;
        self.settings.resolve_clip(&source)?;
        self.settings.resolve_motion_joint(&source)?;
        if let Some(profile) = &self.profile {
            profile.compile_models(&source, &model)?;
        }
        let frame = self
            .frame
            .prepare(&model.skeleton, max)
            .map_err(|e| e.to_string())?;
        model.validate_morph_frame(&frame)?;
        Ok(PreparedPreview {
            owner: self.owner.clone(),
            phase: f64::from_bits(self.phase),
            model,
            source,
            settings: self.settings.clone(),
            profile: self.profile.clone(),
            frame: Arc::new(frame),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_restore_rejects_foreign_morph_schema_and_unbound_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::locomotion_tests::Temp::new();
        let (_, bytes) = crate::locomotion_tests::morph_project(&fixture)?;
        let model = Arc::new(ModelAsset::parse_animated(
            &bytes,
            &[],
            voxy_model::ModelLimits::default(),
        )?);
        let mut scene = voxy_scene::SceneGraph::new(2);
        let owner = scene.spawn(None, Default::default())?;
        let view = SceneCheckpointView::new(&scene, [(ObjectId("visual".into()), owner)])?;
        let preview = Preview {
            owner,
            phase: 0.25,
            model: model.clone(),
            source: model.clone(),
            settings: crate::ModelAnimation::default(),
            profile: None,
            frame: Arc::new(model.sample_frame_phase(Some(0), 0.25)?),
        };
        let max = 8 * 1024 * 1024;
        let saved = preview.checkpoint(&view, |_| Some("model-0".into()), max)?;
        let restored = saved.prepare(&view, |_| Some(model.clone()), max)?;
        assert_eq!(
            restored
                .frame
                .checkpoint(&model.skeleton, max)?
                .to_json(max)?,
            preview
                .frame
                .checkpoint(&model.skeleton, max)?
                .to_json(max)?
        );
        let original: serde_json::Value = serde_json::from_str(&saved.to_json(max)?)?;
        let mut invalid = original.clone();
        invalid["frame"]["properties"] = serde_json::json!([]);
        let decoded = PreviewCheckpoint::from_json(&serde_json::to_string(&invalid)?, max)?;
        assert!(
            decoded
                .prepare(&view, |_| Some(model.clone()), max)
                .is_err(),
            "preview restored a frame with no required morph properties"
        );
        let mut invalid = original;
        invalid["source"] = "unbound-source".into();
        let decoded = PreviewCheckpoint::from_json(&serde_json::to_string(&invalid)?, max)?;
        let other = Arc::new(model.as_ref().clone());
        assert!(
            decoded
                .prepare(
                    &view,
                    |key| Some(if key == "model-0" {
                        model.clone()
                    } else {
                        other.clone()
                    }),
                    max
                )
                .is_err(),
            "preview restored a separate source rig without a retarget profile"
        );
        let mut missing_properties = (*preview.frame).clone();
        missing_properties.properties = Default::default();
        let invalid_preview = Preview {
            owner,
            phase: preview.phase,
            model: model.clone(),
            source: model.clone(),
            settings: preview.settings.clone(),
            profile: None,
            frame: Arc::new(missing_properties),
        };
        assert!(
            invalid_preview
                .checkpoint(&view, |_| Some("model-0".into()), max)
                .is_err()
        );
        let invalid_preview = Preview {
            source: other.clone(),
            frame: preview.frame.clone(),
            ..invalid_preview
        };
        assert!(
            invalid_preview
                .checkpoint(
                    &view,
                    |root| Some(if Arc::ptr_eq(root, &model) {
                        "model-0".into()
                    } else {
                        "unbound-source".into()
                    }),
                    max
                )
                .is_err()
        );
        assert_eq!(preview.phase.to_bits(), 0.25_f64.to_bits());
        assert!(Arc::ptr_eq(&preview.model, &preview.source));
        Ok(())
    }
}
