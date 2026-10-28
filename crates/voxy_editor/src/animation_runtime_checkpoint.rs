//! One staged animation authority, with shared snapshots and exact pending motions.
use super::reference_checkpoint::RootReferenceCheckpoint;
use super::*;
use crate::foot_placement::checkpoint::{FootCheckpointRig, FootRuntimeCheckpoint};
use crate::model_playback::checkpoint::ModelPlaybackCheckpoint;
use serde::{Deserialize, Serialize};
use voxy_animation::{
    AnimationClip, AnimatorFrameCheckpoint, PoseSnapshotArchive, PoseSnapshotBuilder, PropertyClip,
    RetargetBindingCheckpoint, RootRigidPathStateRecord,
};
use voxy_scene::ObjectId;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedOwner {
    id: ObjectId,
    model: String,
    animation_model: String,
    settings: ModelAnimation,
    profile: Option<crate::ModelRetarget>,
    binding: Option<RetargetBindingCheckpoint>,
    playback: ModelPlaybackCheckpoint,
    frame: AnimatorFrameCheckpoint,
    feet: Option<FootRuntimeCheckpoint>,
    reference: Option<RootReferenceCheckpoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rotation {
    owner: ObjectId,
    path: RootRigidPathStateRecord,
    basis: [u64; 4],
    origin: [u32; 3],
    scale: u64,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnimationRuntimeCheckpoint {
    version: u32,
    bound: bool,
    serial: u64,
    owners: Vec<SavedOwner>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shared_frames: Vec<(ObjectId, ObjectId)>,
    poses: PoseSnapshotArchive,
    motions: Vec<(ObjectId, [u32; 3])>,
    rotations: Vec<Rotation>,
}
fn queued_path(owner: &Owner) -> Result<voxy_animation::RootRigidPath, EditorError> {
    let receipt = owner
        .playback
        .root_motion_receipt()
        .ok_or("queued animation path has no source receipt")?;
    let mut path = receipt.path(voxy_animation::MAX_ROOT_ROTATION_SPANS)?;
    if let Some(binding) = &owner.retarget_binding {
        path = binding.apply_root_path(
            &path,
            receipt.root_motion_joint(),
            owner.settings.root_motion_axes,
        )?;
    }
    Ok(path)
}
fn model_key(
    asset: &Arc<ModelAsset>,
    models: &mut Vec<(String, Arc<ModelAsset>)>,
    identify: &impl Fn(&Arc<ModelAsset>) -> Option<String>,
) -> Result<String, EditorError> {
    if let Some((key, _)) = models.iter().find(|(_, old)| Arc::ptr_eq(old, asset)) {
        return Ok(key.clone());
    }
    let key = identify(asset)
        .filter(|k| !k.is_empty())
        .ok_or("unidentified animation model revision")?;
    if models.iter().any(|(k, _)| k == &key) {
        return Err("different animation models share a revision key".into());
    }
    models.push((key.clone(), asset.clone()));
    Ok(key)
}
#[allow(dead_code)]
impl AnimationRuntime {
    pub(crate) fn checkpoint_sources(
        &self,
        max_bytes: usize,
        max_sources: usize,
    ) -> Result<crate::animation_sources::Sources, EditorError> {
        let sources = std::cell::RefCell::new(crate::animation_sources::Sources::default());
        let mut owners: Vec<_> = self.owners.keys().copied().collect();
        owners.sort();
        for owner in owners {
            self.owners[&owner].playback.checkpoint(
                |clip, properties| sources.borrow_mut().intern(clip, properties, max_sources),
                max_bytes,
            )?;
        }
        Ok(sources.into_inner())
    }
    /// Stable native model roots, including retained inactive owners. Names and
    /// content do not identify resources; distinct equal allocations stay distinct.
    pub(crate) fn checkpoint_model_roots(&self) -> Vec<Arc<ModelAsset>> {
        let mut owners: Vec<_> = self.owners.keys().copied().collect();
        owners.sort();
        let mut seen = HashSet::new();
        let mut models = Vec::new();
        for owner in owners {
            let state = &self.owners[&owner];
            for model in [&state.model, &state.animation_model] {
                if seen.insert(Arc::as_ptr(model)) {
                    models.push(model.clone());
                }
            }
        }
        models
    }
    pub(crate) fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        identify_model: impl Fn(&Arc<ModelAsset>) -> Option<String>,
        identify_clip: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<AnimationRuntimeCheckpoint, EditorError> {
        if self.scene.is_some_and(|id| id != scene.identity()) || self.owners.len() > MAX_OWNERS {
            return Err("invalid animation checkpoint scene or owner budget".into());
        }
        let mut keys = self.owners.keys().copied().collect::<Vec<_>>();
        keys.sort();
        let poses = std::cell::RefCell::new(PoseSnapshotBuilder::new(MAX_OWNERS * 17));
        let mut models = Vec::new();
        let mut owners = Vec::new();
        let mut frame_roles: HashMap<_, ObjectId> = HashMap::new();
        let mut shared_frames = Vec::new();
        let mut ids = HashMap::new();
        let mut unique = HashSet::new();
        for node in keys {
            scene.local(node)?;
            let id = identify(node)
                .filter(|id| !id.0.is_empty())
                .ok_or("unidentified animation owner")?;
            if !unique.insert(id.clone()) {
                return Err("duplicate animation owner identity".into());
            }
            ids.insert(node, id.clone());
            let state = &self.owners[&node];
            if let Some(canonical) = frame_roles.get(&Arc::as_ptr(&state.frame)) {
                shared_frames.push((id.clone(), canonical.clone()));
            } else {
                frame_roles.insert(Arc::as_ptr(&state.frame), id.clone());
            }
            let model = model_key(&state.model, &mut models, &identify_model)?;
            let animation_model = model_key(&state.animation_model, &mut models, &identify_model)?;
            let identify_pose = |pose: &Arc<voxy_animation::Pose>| {
                poses.borrow_mut().intern(pose, &animation_model).ok()
            };
            owners.push(SavedOwner {
                id,
                model,
                animation_model: animation_model.clone(),
                settings: state.settings.clone(),
                profile: state.retarget_profile.clone(),
                binding: state
                    .retarget_binding
                    .as_ref()
                    .map(|b| b.checkpoint(max_bytes).map_err(|e| e.to_string()))
                    .transpose()?,
                playback: state.playback.checkpoint_with_snapshots(
                    &identify_clip,
                    identify_pose,
                    max_bytes,
                )?,
                frame: state
                    .frame
                    .checkpoint(&state.model.skeleton, max_bytes)
                    .map_err(|e| e.to_string())?,
                feet: state
                    .feet
                    .as_ref()
                    .map(|f| f.checkpoint(scene, &identify, identify_pose, max_bytes))
                    .transpose()?,
                reference: state.root_reference.map(|r| r.checkpoint()),
            });
        }
        let motions = self
            .motions
            .iter()
            .map(|(node, motion)| {
                Ok((
                    ids.get(node)
                        .cloned()
                        .ok_or("queued motion owner missing")?,
                    motion.to_array().map(f32::to_bits),
                ))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let rotations = self
            .rotations
            .iter()
            .map(|(node, path, basis, origin, scale)| {
                let state = self
                    .owners
                    .get(node)
                    .ok_or("queued rotation owner missing")?;
                let record = path.state_record();
                if !record.matches(&queued_path(state)?) {
                    return Err("queued rotation disagrees with source/retarget receipt".into());
                }
                Ok(Rotation {
                    owner: ids
                        .get(node)
                        .cloned()
                        .ok_or("queued rotation identity missing")?,
                    path: record,
                    basis: basis.to_array().map(f64::to_bits),
                    origin: origin.to_array().map(f32::to_bits),
                    scale: scale.to_bits(),
                })
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let saved = AnimationRuntimeCheckpoint {
            version: 1,
            bound: self.scene.is_some(),
            serial: self.serial,
            owners,
            shared_frames,
            poses: poses
                .into_inner()
                .finish(max_bytes)
                .map_err(|e| e.to_string())?,
            motions,
            rotations,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl AnimationRuntimeCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        if self.version != 1 {
            return Err("unsupported animation runtime checkpoint".into());
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err("animation runtime checkpoint exceeds byte budget".into());
        }
        Ok(text)
    }
    pub(crate) fn from_json(text: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if text.len() > max_bytes {
            return Err("animation runtime checkpoint exceeds byte budget".into());
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err("unsupported animation runtime checkpoint".into());
        }
        Ok(saved)
    }
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        resolve_model: impl Fn(&str) -> Option<Arc<ModelAsset>>,
        resolve_clip: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        max_bytes: usize,
    ) -> Result<AnimationRuntime, EditorError> {
        self.to_json(max_bytes)?;
        if self.owners.len() > MAX_OWNERS
            || self.shared_frames.len() > self.owners.len()
            || self.motions.len() > MAX_OWNERS
            || self.rotations.len() > MAX_OWNERS
            || (!self.bound
                && (!self.owners.is_empty()
                    || !self.motions.is_empty()
                    || !self.rotations.is_empty()
                    || self.serial != 0))
        {
            return Err("invalid animation owner/motion budget or binding".into());
        }
        let aliases: HashSet<_> = self
            .shared_frames
            .iter()
            .map(|(alias, _)| alias.clone())
            .collect();
        if aliases.len() != self.shared_frames.len() {
            return Err("duplicate shared runtime frame role".into());
        }
        let ranks: HashMap<_, _> = self
            .owners
            .iter()
            .enumerate()
            .map(|(rank, owner)| (owner.id.clone(), rank))
            .collect();
        for (alias, canonical) in &self.shared_frames {
            let alias_rank = ranks
                .get(alias)
                .ok_or("missing runtime frame alias owner")?;
            let canonical_rank = ranks
                .get(canonical)
                .ok_or("missing canonical runtime frame owner")?;
            if canonical_rank >= alias_rank || aliases.contains(canonical) {
                return Err("noncanonical shared runtime frame role".into());
            }
        }
        let mut models = HashMap::<String, Arc<ModelAsset>>::new();
        for saved in &self.owners {
            for key in [&saved.model, &saved.animation_model] {
                if key.is_empty() {
                    return Err("empty animation revision key".into());
                }
                if !models.contains_key(key) {
                    let model = resolve_model(key).ok_or("missing animation model revision")?;
                    if models.values().any(|old| Arc::ptr_eq(old, &model)) {
                        return Err("different model revision keys alias one resource".into());
                    }
                    models.insert(key.clone(), model);
                }
            }
        }
        let poses = self
            .poses
            .prepare(
                |key| models.get(key).map(|m| m.skeleton.clone()),
                max_bytes,
                MAX_OWNERS * 17,
            )
            .map_err(|e| e.to_string())?;
        let mut owners = HashMap::new();
        let mut identities = HashMap::new();
        for saved in &self.owners {
            if saved.id.0.is_empty() || identities.contains_key(&saved.id) {
                return Err("invalid saved animation object identity".into());
            }
            let node = resolve(&saved.id).ok_or("animation owner missing in restored scene")?;
            scene.local(node)?;
            if owners.contains_key(&node) {
                return Err("different animation owners resolve to one scene node".into());
            }
            let model = models[&saved.model].clone();
            let animation_model = models[&saved.animation_model].clone();
            saved.settings.validate(
                Some(animation_model.animations.len()),
                Some(animation_model.skeleton.joints().len()),
            )?;
            if saved.profile.is_some() != saved.binding.is_some() {
                return Err("retarget profile/binding mismatch".into());
            }
            if let Some(profile) = &saved.profile {
                profile.validate()?;
            }
            let binding = saved
                .binding
                .as_ref()
                .map(|b| {
                    b.prepare(&animation_model.skeleton, &model.skeleton, max_bytes)
                        .map_err(|e| e.to_string())
                })
                .transpose()?;
            let playback = saved.playback.prepare_with_snapshots(
                animation_model.clone(),
                &resolve_clip,
                |key| poses.resolve(key),
                max_bytes,
            )?;
            let frame = Arc::new(
                saved
                    .frame
                    .prepare(&model.skeleton, max_bytes)
                    .map_err(|e| e.to_string())?,
            );
            model.frame_weights(&frame)?;
            let feet = saved
                .feet
                .as_ref()
                .map(|f| {
                    f.prepare_with_source_skeleton(
                        FootCheckpointRig::Model(&model),
                        Some(&animation_model.skeleton),
                        &animation_model.animations,
                        scene,
                        &resolve,
                        |key| poses.resolve(key),
                        max_bytes,
                    )
                })
                .transpose()?;
            let reference = saved
                .reference
                .as_ref()
                .map(RootReferenceCheckpoint::prepare)
                .transpose()?;
            identities.insert(saved.id.clone(), node);
            owners.insert(
                node,
                Owner {
                    model,
                    animation_model,
                    retarget_profile: saved.profile.clone(),
                    retarget_binding: binding,
                    settings: saved.settings.clone(),
                    playback,
                    frame,
                    feet,
                    root_reference: reference,
                },
            );
        }
        let mut queued = HashSet::new();
        let motions = self
            .motions
            .iter()
            .map(|(id, bits)| {
                let node = *identities.get(id).ok_or("restored motion owner missing")?;
                let motion = glam::Vec3::from_array(bits.map(f32::from_bits));
                if !queued.insert(node) || !motion.is_finite() {
                    return Err("invalid or duplicate restored translation motion".into());
                }
                Ok((node, motion))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let mut total_spans = 0usize;
        let rotations = self
            .rotations
            .iter()
            .map(|saved| {
                let node = *identities
                    .get(&saved.owner)
                    .ok_or("restored rotation owner missing")?;
                let path = queued_path(&owners[&node])?;
                let basis = glam::DQuat::from_array(saved.basis.map(f64::from_bits));
                let origin = glam::Vec3::from_array(saved.origin.map(f32::from_bits));
                let scale = f64::from_bits(saved.scale);
                if !queued.insert(node)
                    || !basis.is_finite()
                    || !basis.is_normalized()
                    || !origin.is_finite()
                    || !scale.is_finite()
                    || scale == 0.
                    || !saved.path.matches(&path)
                {
                    return Err("invalid restored rotation receipt or coordinates".into());
                }
                total_spans = total_spans
                    .checked_add(path.spans().len())
                    .ok_or("restored rotation span overflow")?;
                if total_spans > voxy_gameplay::MAX_CHARACTER_TRAJECTORY_SPANS {
                    return Err("restored rotation span budget exceeded".into());
                }
                Ok((node, path, basis, origin, scale))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let mut runtime = AnimationRuntime {
            change: AnimationChangeToken::fresh(),
            scene: self.bound.then(|| scene.identity()),
            owners,
            serial: self.serial,
            motions,
            rotations,
        };
        for (alias, canonical) in &self.shared_frames {
            let canonical = identities
                .get(canonical)
                .ok_or("missing canonical runtime frame")?;
            let alias = identities.get(alias).ok_or("missing runtime frame alias")?;
            let frame = runtime.owners[canonical].frame.clone();
            runtime.restore_shared_frame(*alias, frame, max_bytes)?;
        }
        Ok(runtime)
    }
}
