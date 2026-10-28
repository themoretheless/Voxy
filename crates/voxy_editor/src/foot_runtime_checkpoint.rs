//! Foot history with durable support IDs and shared frozen-pose identities.
use super::*;
use voxy_scene::{NodeId, ObjectId, SceneGraph};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    contact: voxy_gameplay::FootContactCheckpoint,
    pivot: [u64; 3],
    pending: Option<([u64; 3], u64)>,
    geometry: Option<(Vec<[u64; 3]>, u64)>,
    last_weight: Option<u32>,
    frozen: Option<(String, u32)>,
}
#[allow(dead_code)]
pub(crate) enum FootCheckpointRig<'a> {
    Model(&'a ModelAsset),
    Skeleton(&'a voxy_animation::Skeleton),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FootRuntimeCheckpoint {
    version: u32,
    settings: ModelFootPlacement,
    feet: Vec<History>,
}
#[allow(dead_code)]
impl FootRuntime {
    /// Frozen poses use a shared outer identity table so restored contact history
    /// refers to the SAME Arc as restored Animator/ModelPlayback snapshots.
    pub(crate) fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        identify_pose: impl Fn(&Arc<voxy_animation::Pose>) -> Option<String>,
        max_bytes: usize,
    ) -> Result<FootRuntimeCheckpoint, String> {
        let mut poses = std::collections::HashMap::<String, Arc<voxy_animation::Pose>>::new();
        let feet = self
            .feet
            .iter()
            .map(|foot| {
                Ok(History {
                    contact: foot
                        .state
                        .checkpoint(scene, &identify)
                        .map_err(|e| e.to_string())?,
                    pivot: foot.pivot.to_array().map(f64::to_bits),
                    pending: foot
                        .pending_pivot
                        .map(|(p, d)| (p.to_array().map(f64::to_bits), d.to_bits())),
                    geometry: foot.geometry.as_ref().map(|(g, d)| {
                        (
                            g.points()
                                .iter()
                                .map(|p| p.to_array().map(f64::to_bits))
                                .collect(),
                            d.to_bits(),
                        )
                    }),
                    last_weight: foot.blend.last_weight.map(f32::to_bits),
                    frozen: foot
                        .blend
                        .frozen
                        .as_ref()
                        .map(|(pose, weight)| {
                            let key = identify_pose(pose)
                                .filter(|k| !k.is_empty())
                                .ok_or("unresolved frozen foot snapshot")?;
                            if poses
                                .insert(key.clone(), pose.clone())
                                .is_some_and(|old| !Arc::ptr_eq(&old, pose))
                            {
                                return Err("aliased frozen foot snapshot key".into());
                            }
                            Ok::<_, String>((key, weight.to_bits()))
                        })
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let saved = FootRuntimeCheckpoint {
            version: 1,
            settings: (*self.settings).clone(),
            feet,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl FootRuntimeCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, String> {
        if self.version != 1 {
            return Err("unsupported foot runtime checkpoint".into());
        }
        let text = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if text.len() > max_bytes {
            return Err("foot runtime checkpoint exceeds byte budget".into());
        }
        Ok(text)
    }
    pub(crate) fn from_json(text: &str, max_bytes: usize) -> Result<Self, String> {
        if text.len() > max_bytes {
            return Err("foot runtime checkpoint exceeds byte budget".into());
        }
        let saved: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if saved.version != 1 {
            return Err("unsupported foot runtime checkpoint".into());
        }
        Ok(saved)
    }
    pub(crate) fn prepare(
        &self,
        rig: FootCheckpointRig<'_>,
        clips: &[Arc<voxy_animation::AnimationClip>],
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
        max_bytes: usize,
    ) -> Result<FootRuntime, String> {
        self.prepare_with_source_skeleton(rig, None, clips, scene, resolve, resolve_pose, max_bytes)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_with_source_skeleton(
        &self,
        rig: FootCheckpointRig<'_>,
        source_skeleton: Option<&voxy_animation::Skeleton>,
        clips: &[Arc<voxy_animation::AnimationClip>],
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
        max_bytes: usize,
    ) -> Result<FootRuntime, String> {
        self.to_json(max_bytes)?;
        let (skeleton, mut candidate) = match rig {
            FootCheckpointRig::Model(model) => (
                &model.skeleton,
                FootRuntime::new_with_clips(model, self.settings.clone(), clips)?,
            ),
            FootCheckpointRig::Skeleton(skeleton) => (
                skeleton,
                FootRuntime::from_skeleton(skeleton, self.settings.clone(), clips)?,
            ),
        };
        let snapshot_skeleton = source_skeleton.unwrap_or(skeleton);
        if source_skeleton.is_some()
            && clips
                .iter()
                .any(|clip| !clip.is_compatible_with(snapshot_skeleton))
        {
            return Err("foot snapshot source clips belong to another rig".into());
        }
        if self.feet.len() != candidate.feet.len() {
            return Err("saved foot count mismatch".into());
        }
        let vector = |bits: [u64; 3]| {
            let v = DVec3::from_array(bits.map(f64::from_bits));
            if !v.is_finite() || v.abs().max_element() > 1e6 {
                return Err("invalid saved foot pivot".to_string());
            }
            Ok(v)
        };
        let distance = |bits| {
            let d = f64::from_bits(bits);
            if !d.is_finite() || !(0. ..=1e6).contains(&d) {
                return Err("invalid saved foot projection limit".to_string());
            }
            Ok(d)
        };
        let weight = |bits| {
            let w = f32::from_bits(bits);
            if !w.is_finite() || !(0. ..=1.).contains(&w) {
                return Err("invalid saved foot contact weight".to_string());
            }
            Ok(w)
        };
        // Cache each key once: sharing and pointer identity survive across feet.
        let mut poses = std::collections::HashMap::new();
        for ((foot, saved), binding) in candidate
            .feet
            .iter_mut()
            .zip(&self.feet)
            .zip(&self.settings.feet)
        {
            if saved.geometry.is_some()
                && (binding.orientation != FootOrientation::Authored
                    || !binding.clip_pivot_keys.is_empty())
            {
                return Err("conflicting saved foot geometry ownership".into());
            }
            foot.state = saved
                .contact
                .prepare(scene, &resolve)
                .map_err(|e| e.to_string())?;
            foot.pivot = vector(saved.pivot)?;
            foot.pending_pivot = saved
                .pending
                .map(|(p, d)| Ok::<_, String>((vector(p)?, distance(d)?)))
                .transpose()?;
            foot.geometry = saved
                .geometry
                .as_ref()
                .map(|(points, d)| {
                    let points = points
                        .iter()
                        .map(|p| vector(*p))
                        .collect::<Result<Vec<_>, _>>()?;
                    let points = voxy_animation::FootSupportPoints::new(points)
                        .map_err(|e| e.to_string())?;
                    Ok::<_, String>((points, distance(*d)?))
                })
                .transpose()?;
            foot.blend.last_weight = saved.last_weight.map(weight).transpose()?;
            foot.blend.frozen = saved
                .frozen
                .as_ref()
                .map(|(key, w)| {
                    if key.is_empty() {
                        return Err("invalid frozen foot pose key".into());
                    }
                    let pose = if let Some(pose) = poses.get(key) {
                        Arc::clone(pose)
                    } else {
                        let pose = resolve_pose(key).ok_or("missing frozen foot pose")?;
                        pose.skin_matrices(snapshot_skeleton)
                            .map_err(|e| e.to_string())?;
                        poses.insert(key.clone(), pose.clone());
                        pose
                    };
                    Ok::<_, String>((pose, weight(*w)?))
                })
                .transpose()?;
            if foot.blend.frozen.is_some() && foot.blend.last_weight.is_none() {
                return Err("frozen foot contact has no accepted weight".into());
            }
        }
        Ok(candidate)
    }
}
