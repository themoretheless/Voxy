//! Accepted render frames are independent of the newest simulation frame.
#![allow(dead_code)]
use super::*;
use std::collections::BTreeMap;
use voxy_scene::{ObjectId, SceneGraph};
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AcceptedFrames {
    version: u32,
    owners: Vec<SavedOwner>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    frames: Vec<voxy_animation::AnimatorFrameCheckpoint>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedOwner {
    id: ObjectId,
    settings: ModelAnimation,
    ticks: u64,
    frame: FrameRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lod: Option<SavedLod>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum FrameRef {
    Index(usize),
    Legacy(voxy_animation::AnimatorFrameCheckpoint),
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedLod {
    prepared: Option<LodProof>,
    history: Vec<(u8, usize)>,
    resident: Vec<usize>,
}
#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LodProof {
    world: [u32; 16],
    joints: Vec<[u32; 16]>,
    positions: Vec<[u32; 3]>,
    levels: Vec<(u64, u32)>,
}
impl LodProof {
    fn capture(prepared: &voxy_render::PreparedSkinnedLod) -> Self {
        Self {
            world: prepared.model().to_cols_array().map(f32::to_bits),
            joints: prepared
                .joints()
                .iter()
                .map(|m| m.to_cols_array().map(f32::to_bits))
                .collect(),
            positions: prepared
                .positions()
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect(),
            levels: prepared
                .levels()
                .iter()
                .map(|l| (l.object_error.to_bits(), l.index_count))
                .collect(),
        }
    }
}
pub(crate) struct PreparedLod {
    pub source: Arc<voxy_render::SkinnedLodMesh>,
    pub prepared: Option<voxy_render::PreparedSkinnedLod>,
    pub history: BTreeMap<u8, usize>,
    pub resident: Vec<usize>,
}
impl SavedLod {
    fn capture(owner: &Owner) -> Result<Option<Self>, EditorError> {
        let Some(source) = &owner.lod else {
            if owner.prepared_lod.is_some()
                || !owner.lod_history.is_empty()
                || !owner.lod_levels.is_empty()
            {
                return Err("LOD state has no accepted source".into());
            }
            return Ok(None);
        };
        let published = owner
            .publication
            .as_ref()
            .and_then(|p| p.value().skinned_lod.as_ref())
            .ok_or("accepted LOD has no original publication")?;
        if !Arc::ptr_eq(source, published) {
            return Err("accepted LOD differs from original publication".into());
        }
        if let Some(prepared) = &owner.prepared_lod {
            if !prepared.source().shares_source_with(source)
                || prepared.joints() != owner.frame.skin_matrices
            {
                return Err("accepted LOD certificate differs from frame".into());
            }
        }
        let mut history: Vec<_> = owner.lod_history.iter().map(|(v, l)| (*v, *l)).collect();
        history.sort_unstable();
        let mut resident: Vec<_> = owner.lod_levels.keys().copied().collect();
        resident.sort_unstable();
        Ok(Some(Self {
            prepared: owner.prepared_lod.as_ref().map(LodProof::capture),
            history,
            resident,
        }))
    }
    fn prepare(
        &self,
        source: Arc<voxy_render::SkinnedLodMesh>,
        frame: &voxy_animation::AnimatorFrame,
    ) -> Result<PreparedLod, EditorError> {
        if self.history.windows(2).any(|w| w[0].0 >= w[1].0)
            || self.resident.windows(2).any(|w| w[0] >= w[1])
            || self
                .resident
                .iter()
                .any(|l| *l == 0 || source.indices(*l).is_none())
            || self.history.iter().any(|(_, l)| {
                source.indices(*l).is_none() || (*l > 0 && self.resident.binary_search(l).is_err())
            })
            || (self.prepared.is_none() && (!self.history.is_empty() || !self.resident.is_empty()))
        {
            return Err("invalid accepted LOD history or residency".into());
        }
        let prepared = self
            .prepared
            .as_ref()
            .map(|proof| {
                let candidate = source.prepare(
                    &frame.skin_matrices,
                    glam::Mat4::from_cols_array(&proof.world.map(f32::from_bits)),
                )?;
                if LodProof::capture(&candidate) != *proof {
                    return Err(EditorError::from(
                        "accepted LOD certificate differs on restore",
                    ));
                }
                Ok(candidate)
            })
            .transpose()?;
        Ok(PreparedLod {
            source,
            prepared,
            history: self.history.iter().copied().collect(),
            resident: self.resident.clone(),
        })
    }
}
pub(crate) struct PreparedFrame {
    pub model: Arc<ModelAsset>,
    pub settings: ModelAnimation,
    pub ticks: u64,
    pub frame: Arc<voxy_animation::AnimatorFrame>,
    pub lod: Option<PreparedLod>,
}
impl AnimatedModels {
    pub(crate) fn restore_accepted_owner(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request: Request,
        ticks: u64,
        other_live: u64,
        budget: u64,
    ) -> Result<(), EditorError> {
        self.update(
            renderer,
            device,
            queue,
            request.owner,
            request.model,
            request.settings,
            request.frame,
            request.lod,
            request.textures,
            request.texture_storage,
            request.publication,
            ticks,
            other_live,
            budget,
        )
    }
    /// Rebuild all retained levels before replacing the accepted per-view history.
    /// The caller stages the enclosing renderer and owns device error scopes.
    pub(crate) fn restore_accepted_lod(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        owner: NodeId,
        saved: PreparedLod,
        other_live: u64,
        budget: u64,
    ) -> Result<(), EditorError> {
        let mut live = other_live
            .checked_add(self.allocation_bytes())
            .ok_or("LOD allocation overflow")?;
        admit(live, 0, budget)?;
        let state = self
            .owners
            .get(&owner)
            .ok_or("missing accepted LOD owner")?;
        if state
            .lod
            .as_ref()
            .is_none_or(|source| !Arc::ptr_eq(source, &saved.source))
            || saved.prepared.as_ref().is_some_and(|p| {
                p.joints() != state.frame.skin_matrices
                    || !p.source().shares_source_with(&saved.source)
            })
            || saved.resident.windows(2).any(|w| w[0] >= w[1])
            || saved
                .resident
                .iter()
                .any(|l| *l == 0 || saved.source.indices(*l).is_none())
            || saved.history.values().any(|l| {
                saved.source.indices(*l).is_none()
                    || (*l > 0 && saved.resident.binary_search(l).is_err())
            })
            || (saved.prepared.is_none()
                && (!saved.history.is_empty() || !saved.resident.is_empty()))
        {
            return Err("accepted LOD restore differs from target owner".into());
        }
        let mut staged = HashMap::new();
        for level in &saved.resident {
            let geometry = LodUpload {
                renderer,
                device,
                skinner: self.skinner.as_ref(),
                live,
                budget,
            }
            .stage(state, &saved.source, *level)?;
            live = live
                .checked_add(geometry.bytes())
                .ok_or("LOD allocation overflow")?;
            staged.insert(*level, geometry);
        }
        let state = self.owners.get_mut(&owner).unwrap();
        state.lod_levels = staged;
        state.lod_history = saved.history.into_iter().collect();
        state.prepared_lod = saved.prepared;
        Ok(())
    }
    pub(crate) fn checkpoint_accepted_frames(
        &self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<AcceptedFrames, EditorError> {
        // Validate original model/publication identity before assigning stable roles.
        self.source_publication_owners()?;
        let mut ordered: Vec<_> = self.owners.iter().collect();
        ordered.sort_by_key(|(node, _)| **node);
        let mut ids = HashSet::new();
        let mut frames = Vec::new();
        let mut frame_ids = HashMap::new();
        let owners = ordered
            .into_iter()
            .map(|(node, owner)| {
                let id = identify(*node)
                    .filter(|id| !id.0.is_empty())
                    .ok_or("accepted GPU frame has no stable owner")?;
                if !ids.insert(id.clone()) {
                    return Err("accepted GPU frames share a stable owner".into());
                }
                owner.settings.validate(None, None)?;
                owner
                    .model
                    .validate_morph_frame(&owner.frame)
                    .map_err(|error| format!("accepted GPU frame {}: {error}", id.0))?;
                let index = if let Some(index) = frame_ids.get(&Arc::as_ptr(&owner.frame)) {
                    *index
                } else {
                    let index = frames.len();
                    frames.push(
                        owner
                            .frame
                            .checkpoint(&owner.model.skeleton, max_bytes)
                            .map_err(|e| e.to_string())?,
                    );
                    frame_ids.insert(Arc::as_ptr(&owner.frame), index);
                    index
                };
                Ok(SavedOwner {
                    id,
                    settings: owner.settings.clone(),
                    ticks: owner.ticks,
                    frame: FrameRef::Index(index),
                    lod: SavedLod::capture(owner)?,
                })
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let saved = AcceptedFrames {
            version: 2,
            owners,
            frames,
        };
        saved.validate_bytes(max_bytes)?;
        Ok(saved)
    }
}
impl AcceptedFrames {
    fn validate_bytes(&self, max_bytes: usize) -> Result<(), EditorError> {
        if !matches!(self.version, 1 | 2)
            || (self.version == 1 && !self.frames.is_empty())
            || serde_json::to_vec(self)?.len() > max_bytes
        {
            return Err("accepted GPU frames exceed policy or version".into());
        }
        Ok(())
    }
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        bindings: &crate::gpu_source_bindings::PreparedBindings,
        max_bytes: usize,
    ) -> Result<BTreeMap<NodeId, PreparedFrame>, EditorError> {
        self.validate_bytes(max_bytes)?;
        if scene.identity() != bindings.scene || self.owners.len() != bindings.owners.len() {
            return Err("accepted GPU frame roles differ from target scene".into());
        }
        let mut frame_cache: Vec<Arc<voxy_animation::AnimatorFrame>> = Vec::new();
        let mut result = BTreeMap::new();
        let mut ids = HashSet::new();
        for owner in &self.owners {
            if owner.id.0.is_empty() || !ids.insert(&owner.id) {
                return Err("duplicate accepted GPU frame identity".into());
            }
            let node = resolve(&owner.id).ok_or("missing accepted GPU frame owner")?;
            scene.local(node)?;
            if result.contains_key(&node) {
                return Err("collapsed accepted GPU frame owners".into());
            }
            let publication = bindings
                .owners
                .get(&node)
                .ok_or("missing GPU publication role")?;
            let model = publication
                .value()
                .animated
                .as_ref()
                .ok_or("missing accepted model")?;
            owner.settings.validate(None, None)?;
            let frame = match (&owner.frame, self.version) {
                (FrameRef::Legacy(saved), 1) => Arc::new(
                    saved
                        .prepare(&model.skeleton, max_bytes)
                        .map_err(|e| e.to_string())?,
                ),
                (FrameRef::Index(index), 2) => {
                    let saved = self
                        .frames
                        .get(*index)
                        .ok_or("invalid accepted frame reference")?;
                    let candidate = saved
                        .prepare(&model.skeleton, max_bytes)
                        .map_err(|e| e.to_string())?;
                    if *index == frame_cache.len() {
                        frame_cache.push(Arc::new(candidate));
                    }
                    frame_cache
                        .get(*index)
                        .ok_or("noncanonical accepted frame reference")?
                        .clone()
                }
                _ => return Err("accepted frame encoding differs from version".into()),
            };
            model
                .validate_morph_frame(&frame)
                .map_err(|error| format!("accepted GPU frame {}: {error}", owner.id.0))?;
            let lod = owner
                .lod
                .as_ref()
                .map(|saved| {
                    let source = publication
                        .value()
                        .skinned_lod
                        .as_ref()
                        .ok_or("missing accepted LOD source")?;
                    saved.prepare(source.clone(), &frame)
                })
                .transpose()?;
            result.insert(
                node,
                PreparedFrame {
                    model: model.clone(),
                    settings: owner.settings.clone(),
                    ticks: owner.ticks,
                    frame,
                    lod,
                },
            );
        }
        if frame_cache.len() != self.frames.len() {
            return Err("unused accepted frame resources".into());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_frame_joins_original_publication_and_rejects_invalid_bindings() {
        let provider = voxy_assets::FileInputs::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets"),
        )
        .unwrap();
        let mut inputs = voxy_assets::ImportInputs::new(4, 4 * 1024 * 1024);
        let asset = crate::import::load(
            &voxy_assets::SourcePath::new("animated-triangle.glb").unwrap(),
            &provider,
            &mut inputs,
        )
        .unwrap();
        let model = asset.animated.as_ref().unwrap().clone();
        let frame = model.bind_frame().unwrap();
        let publication = Arc::new(
            inputs
                .finish(asset, |id, max| provider.read(id, max))
                .unwrap(),
        );
        let mut scene = SceneGraph::new(4);
        scene.spawn(None, Default::default()).unwrap();
        let node = scene.spawn(None, Default::default()).unwrap();
        let mut bindings = crate::gpu_source_bindings::PreparedBindings {
            scene: scene.identity(),
            models: Default::default(),
            owners: [(node, publication)].into(),
        };
        let mut saved = AcceptedFrames {
            version: 1,
            frames: Vec::new(),
            owners: vec![SavedOwner {
                id: ObjectId("actor".into()),
                settings: Default::default(),
                ticks: u64::MAX,
                lod: None,
                frame: FrameRef::Legacy(frame.checkpoint(&model.skeleton, 65536).unwrap()),
            }],
        };
        let bytes = serde_json::to_vec(&saved).unwrap();
        saved = serde_json::from_slice(&bytes).unwrap();
        let prepared = saved
            .prepare(&scene, |_| Some(node), &bindings, 65536)
            .unwrap();
        assert_eq!(prepared[&node].ticks, u64::MAX);
        assert!(Arc::ptr_eq(&prepared[&node].model, &model));
        assert_eq!(
            prepared[&node]
                .frame
                .checkpoint(&model.skeleton, 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            frame
                .checkpoint(&model.skeleton, 65536)
                .unwrap()
                .to_json(65536)
                .unwrap()
        );
        assert!(saved.prepare(&scene, |_| Some(node), &bindings, 1).is_err());
        assert!(saved.prepare(&scene, |_| None, &bindings, 65536).is_err());
        saved.owners[0].settings.speed = f32::NAN;
        assert!(
            saved
                .prepare(&scene, |_| Some(node), &bindings, 65536)
                .is_err()
        );
        saved.owners[0].settings = Default::default();
        saved.version = 3;
        assert!(
            saved
                .prepare(&scene, |_| Some(node), &bindings, 65536)
                .is_err()
        );
        saved.version = 1;
        // Preserve shared frame owners without deduplicating independent equal frames.
        let other = scene.spawn(None, Default::default()).unwrap();
        let independent = scene.spawn(None, Default::default()).unwrap();
        let publication = bindings.owners[&node].clone();
        bindings.owners.insert(other, publication.clone());
        bindings.owners.insert(independent, publication);
        saved.version = 2;
        saved.frames = vec![frame.checkpoint(&model.skeleton, 65536).unwrap(); 2];
        saved.owners = [("actor", 0), ("other", 0), ("independent", 1)]
            .into_iter()
            .map(|(id, index)| SavedOwner {
                id: ObjectId(id.into()),
                settings: Default::default(),
                ticks: 7,
                frame: FrameRef::Index(index),
                lod: None,
            })
            .collect();
        let saved_bytes = serde_json::to_vec(&saved).unwrap();
        saved = serde_json::from_slice(&saved_bytes).unwrap();
        let resolve = |id: &ObjectId| match id.0.as_str() {
            "actor" => Some(node),
            "other" => Some(other),
            "independent" => Some(independent),
            _ => None,
        };
        let joined = saved.prepare(&scene, resolve, &bindings, 65536).unwrap();
        assert!(Arc::ptr_eq(&joined[&node].frame, &joined[&other].frame));
        assert!(!Arc::ptr_eq(
            &joined[&node].frame,
            &joined[&independent].frame
        ));
        saved.owners[0].frame = FrameRef::Index(1);
        assert!(saved.prepare(&scene, resolve, &bindings, 65536).is_err());
        saved.owners[0].frame = FrameRef::Index(0);
        saved
            .frames
            .push(frame.checkpoint(&model.skeleton, 65536).unwrap());
        assert!(saved.prepare(&scene, resolve, &bindings, 65536).is_err());
        bindings.owners.clear();
        assert!(
            saved
                .prepare(&scene, |_| Some(node), &bindings, 65536)
                .is_err()
        );
    }
}

#[cfg(test)]
mod morph_admission_tests {
    use super::*;
    #[test]
    fn accepted_frame_pool_checks_target_morph_schema_and_preserves_aliases()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::locomotion_tests::Temp::new();
        let (path, _) = crate::locomotion_tests::morph_project(&fixture)?;
        let provider = voxy_assets::FileInputs::new(path.parent().unwrap())?;
        let mut inputs = voxy_assets::ImportInputs::new(4, 4 * 1024 * 1024);
        let asset = crate::import::load(
            &voxy_assets::SourcePath::new("rig.glb")?,
            &provider,
            &mut inputs,
        )?;
        let model = asset.animated.as_ref().unwrap().clone();
        let frame = model.sample_frame_phase(Some(0), 0.25)?;
        let publication = Arc::new(
            inputs
                .finish(asset, |id, max| provider.read(id, max))
                .map_err(|error| format!("{error:?}"))?,
        );
        let mut scene = SceneGraph::new(3);
        let first = scene.spawn(None, Default::default())?;
        let shared = scene.spawn(None, Default::default())?;
        let independent = scene.spawn(None, Default::default())?;
        let nodes = [
            ("first", first),
            ("shared", shared),
            ("independent", independent),
        ];
        let bindings = crate::gpu_source_bindings::PreparedBindings {
            scene: scene.identity(),
            models: Default::default(),
            owners: nodes
                .iter()
                .map(|(_, node)| (*node, publication.clone()))
                .collect(),
        };
        let resolve = |id: &ObjectId| {
            nodes
                .iter()
                .find(|(name, _)| *name == id.0)
                .map(|(_, node)| *node)
        };
        let max = 8 * 1024 * 1024;
        let mut saved = AcceptedFrames {
            version: 2,
            owners: nodes
                .iter()
                .enumerate()
                .map(|(i, (id, _))| SavedOwner {
                    id: ObjectId((*id).into()),
                    settings: Default::default(),
                    ticks: 7,
                    frame: FrameRef::Index(usize::from(i == 2)),
                    lod: None,
                })
                .collect(),
            frames: vec![frame.checkpoint(&model.skeleton, max)?; 2],
        };
        let prepared = saved.prepare(&scene, resolve, &bindings, max)?;
        assert!(Arc::ptr_eq(
            &prepared[&first].frame,
            &prepared[&shared].frame
        ));
        assert!(!Arc::ptr_eq(
            &prepared[&first].frame,
            &prepared[&independent].frame
        ));
        assert_eq!(prepared[&first].frame.properties, frame.properties);
        let mut invalid: serde_json::Value =
            serde_json::from_str(&frame.checkpoint(&model.skeleton, max)?.to_json(max)?)?;
        invalid["properties"] = serde_json::json!([]);
        let invalid = voxy_animation::AnimatorFrameCheckpoint::from_json(
            &serde_json::to_string(&invalid)?,
            max,
        )?;
        assert!(
            invalid.prepare(&model.skeleton, max).is_ok(),
            "generic frame codec has no target morph schema"
        );
        saved.frames[0] = invalid.clone();
        assert!(
            saved.prepare(&scene, resolve, &bindings, max).is_err(),
            "pooled GPU frame restored without required morph properties"
        );
        saved.version = 1;
        saved.frames.clear();
        for owner in &mut saved.owners {
            owner.frame = FrameRef::Legacy(invalid.clone());
        }
        assert!(
            saved.prepare(&scene, resolve, &bindings, max).is_err(),
            "legacy GPU frame restored without required morph properties"
        );
        Ok(())
    }
}
