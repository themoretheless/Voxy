//! Locomotion clip requests feed the existing ModelPlayback/certified physics transaction.
use crate::*;
use std::{collections::BTreeMap, sync::Arc};
use voxy_model::ModelAsset;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelLocomotion {
    pub idle: String,
    pub moving: String,
    pub rising: String,
    pub falling: String,
    pub speed_threshold: f64,
    pub transition_seconds: f32,
}
impl ModelLocomotion {
    pub(crate) fn validate(&self) -> Result<(), EditorError> {
        for clip in [&self.idle, &self.moving, &self.rising, &self.falling] {
            if clip.is_empty() || clip.len() > 1024 || clip.contains('\0') {
                return Err("invalid locomotion clip name".into());
            }
        }
        if !self.speed_threshold.is_finite()
            || self.speed_threshold < 0.
            || !self.transition_seconds.is_finite()
            || !(0. ..=60.).contains(&self.transition_seconds)
        {
            return Err("invalid locomotion thresholds or transition duration".into());
        }
        Ok(())
    }
    pub(crate) fn validate_model(&self, model: &ModelAsset) -> Result<(), EditorError> {
        self.validate()?;
        for name in [&self.idle, &self.moving, &self.rising, &self.falling] {
            ModelAnimation {
                clip_name: name.clone(),
                ..ModelAnimation::default()
            }
            .resolve_clip(model)?;
        }
        Ok(())
    }
    fn select(&self, state: Option<physics::CharacterState>, jump: bool, jump_speed: f64) -> &str {
        match state {
            Some(state) if state.grounded && jump && jump_speed > 0. => &self.rising,
            Some(state) if !state.grounded => {
                if state.velocity[1] > 0. {
                    &self.rising
                } else {
                    &self.falling
                }
            }
            Some(state) if state.velocity[0].hypot(state.velocity[2]) > self.speed_threshold => {
                &self.moving
            }
            _ => &self.idle,
        }
    }
}
pub(crate) fn body_owner(scene: &SceneGraph, owner: NodeId) -> Result<NodeId, EditorError> {
    let mut current = Some(owner);
    while let Some(node) = current {
        if scene
            .component::<voxy_gameplay::CharacterBody>(node)?
            .is_some()
        {
            return Ok(node);
        }
        current = scene.parent(node)?;
    }
    Err("locomotion requires a character body on the model or an ancestor".into())
}
fn source_model<'a>(
    scene: &SceneGraph,
    models: &'a BTreeMap<AssetId, Arc<ModelAsset>>,
    owner: NodeId,
) -> Result<&'a ModelAsset, EditorError> {
    let instance = scene
        .component::<ModelInstance>(owner)?
        .ok_or("locomotion requires a model owner")?;
    let asset = scene.component::<ModelRetarget>(owner)?.map_or_else(
        || instance.asset.clone(),
        |profile| AssetId(profile.source.clone()),
    );
    models
        .get(&asset)
        .map(Arc::as_ref)
        .ok_or_else(|| format!("locomotion animation source {} is not ready", asset.0).into())
}
pub(super) fn validate_scene(
    scene: &SceneGraph,
    models: &BTreeMap<AssetId, Arc<ModelAsset>>,
) -> Result<(), EditorError> {
    for (owner, controller) in scene.components::<ModelLocomotion>() {
        controller.validate()?;
        body_owner(scene, owner)?;
        if scene.component::<ModelPart>(owner)?.is_some() {
            return Err(
                "locomotion requires a logical model owner, not an imported hierarchy part".into(),
            );
        }
        scene
            .component::<ModelAnimation>(owner)?
            .ok_or("locomotion requires a model animation component")?;
        controller.validate_model(source_model(scene, models, owner)?)?;
    }
    Ok(())
}
/// Keep request changes transactional with respect to an animation/physics failure.
/// Accepted playback still belongs exclusively to AnimationRuntime/ModelPlayback.
pub(super) fn fixed_step(
    runtime: &animation_runtime::AnimationRuntime,
    scene: &mut SceneGraph,
    models: &BTreeMap<AssetId, Arc<ModelAsset>>,
    physics: &mut voxy_gameplay::CharacterPhysics,
    input: &mut voxy_input::InputMap,
    dt: f64,
) -> Result<animation_runtime::AnimationRuntime, voxy_gameplay::CharacterTickError<EditorError>> {
    use voxy_gameplay::CharacterTickError;
    let prepare = || -> Result<Vec<(NodeId, ModelAnimation, ModelAnimation)>, EditorError> {
        validate_scene(scene, models)?;
        let mut requests = Vec::new();
        for (owner, controller) in scene.active_components::<ModelLocomotion>() {
            let body_owner = body_owner(scene, owner)?;
            let body = scene
                .component::<voxy_gameplay::CharacterBody>(body_owner)?
                .ok_or("missing locomotion body")?;
            let state = physics
                .state(scene, body_owner)
                .map_err(|error| format!("locomotion character state: {error}"))?;
            let jump = input.state("jump").is_some_and(|state| state.pressed);
            let previous = scene
                .component::<ModelAnimation>(owner)?
                .ok_or("missing locomotion playback")?
                .clone();
            let mut requested = previous.clone();
            requested.clip_name = controller.select(state, jump, body.jump_speed).into();
            requested.transition_seconds = controller.transition_seconds;
            if requested != previous {
                requests.push((owner, previous, requested));
            }
        }
        Ok(requests)
    };
    let requests = prepare().map_err(CharacterTickError::Preparation)?;
    for (owner, _, requested) in &requests {
        *scene
            .component_mut::<ModelAnimation>(*owner)
            .map_err(EditorError::from)
            .map_err(CharacterTickError::Preparation)?
            .ok_or_else(|| {
                CharacterTickError::Preparation("locomotion playback disappeared".into())
            })? = requested.clone();
    }
    let result = runtime.fixed_step(scene, models, physics, input, dt);
    if result.is_err() {
        for (owner, previous, _) in requests {
            *scene
                .component_mut::<ModelAnimation>(owner)
                .map_err(EditorError::from)
                .map_err(CharacterTickError::Preparation)?
                .ok_or_else(|| {
                    CharacterTickError::Preparation("locomotion rollback owner disappeared".into())
                })? = previous;
        }
    }
    result
}
