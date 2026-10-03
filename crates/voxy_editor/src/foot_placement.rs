//! Authored rig bindings; contact and palettes are staged against accepted physics.
mod contact_events;
use glam::{DMat3, DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use voxy_animation::{AnimatorFrame, TwoBoneChain, TwoBoneTarget};
use voxy_gameplay::{
    CharacterTickPreview, FootContactInput, FootContactSettings, FootContactState,
    SupportQueryBudget,
};
use voxy_render::ModelAsset;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFootPlacement {
    pub feet: Vec<FootBinding>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootBinding {
    pub bones: [String; 3],
    /// Tip-local sole point and sole outward/up axis.
    pub sole_offset: [f32; 3],
    pub sole_up: [f32; 3],
    pub pole: [f32; 3],
    /// Explicit stance enable, multiplied by the clip contact curve.
    pub plant: bool,
    pub weight: f32,
    pub contact: FootContactSettings,
    /// Normalized target-clip phase keys. Empty retains explicit fixed stance.
    #[serde(default)]
    pub contact_curve: Vec<FootContactKey>,
    /// Optional named-clip curves. Every selected clip must be mapped when used.
    #[serde(default)]
    pub clip_contact_curves: std::collections::BTreeMap<String, Vec<FootContactKey>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootContactKey {
    pub phase: f32,
    pub weight: f32,
}
impl FootBinding {
    pub(super) fn contact_keys(&self, clip_name: Option<&str>) -> Result<&[FootContactKey], String> {
        if self.clip_contact_curves.is_empty() {
            return Ok(&self.contact_curve);
        }
        self.clip_contact_curves.get(clip_name.ok_or("foot clip curves require an active clip")?)
            .map(Vec::as_slice).ok_or_else(|| "selected clip has no foot contact curve".into())
    }
}
#[derive(Clone, Debug, Default)]
struct ContactBlendState {
    last_weight: Option<f32>,
    frozen: Option<(Arc<voxy_animation::Pose>, f32)>,
}
impl ContactBlendState {
    fn snapshot_weight(&mut self, snapshot: &Arc<voxy_animation::Pose>) -> Result<f32, String> {
        if self.frozen.as_ref().is_none_or(|(held, _)| !Arc::ptr_eq(held, snapshot)) {
            self.frozen = Some((snapshot.clone(), self.last_weight
                .ok_or("interrupted foot transition has no accepted contact snapshot")?));
        }
        Ok(self.frozen.as_ref().unwrap().1)
    }

    fn sample(&mut self, binding: &FootBinding, clip_name: Option<&str>, phase: Option<f64>,
        blend: Option<voxy_animation::PoseBlendPhases<'_>>) -> Result<f32, String> {
        let target = contact_weight(binding.contact_keys(clip_name)?, phase)?;
        let weight = match blend.and_then(|blend| blend.source.map(|source| (source, blend.target_weight))) {
            None => { self.frozen = None; target }
            Some((source, alpha)) => {
                let source_weight = match source {
                    voxy_animation::PoseBlendSource::Clip(source) => {
                        self.frozen = None;
                        contact_weight(binding.contact_keys(Some(source.clip.name()))?, Some(source.normalized_phase))?
                    }
                    voxy_animation::PoseBlendSource::FrozenPose(snapshot) => {
                        self.snapshot_weight(snapshot)?
                    }
                };
                (f64::from(source_weight) + (f64::from(target) - f64::from(source_weight)) * f64::from(alpha)) as f32
            }
        };
        self.last_weight = Some(weight);
        Ok(weight)
    }
}
fn contact_weight(keys: &[FootContactKey], phase: Option<f64>) -> Result<f32, String> {
    if keys.is_empty() {
        return Ok(1.);
    }
    let phase = phase.ok_or("foot contact curve requires an active clip")?;
    if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
        return Err("invalid foot clip phase".into());
    }
    let end = keys.partition_point(|key| f64::from(key.phase) <= phase);
    if end == 0 {
        return Ok(keys[0].weight);
    }
    if end == keys.len() {
        return Ok(keys[keys.len() - 1].weight);
    }
    let a = keys[end - 1];
    let b = keys[end];
    let t = (phase - f64::from(a.phase)) / (f64::from(b.phase) - f64::from(a.phase));
    let ease = t * t * (3. - 2. * t);
    Ok((f64::from(a.weight) + (f64::from(b.weight) - f64::from(a.weight)) * ease) as f32)
}
fn crossed_swing(
    keys: &[FootContactKey],
    interval: Option<voxy_animation::AnimationPhaseInterval>,
) -> bool {
    let Some(interval) = interval else {
        return false;
    };
    if interval.end <= interval.start {
        return false;
    }
    keys.iter().filter(|key| key.weight == 0.).any(|key| {
        let phase = f64::from(key.phase);
        (interval.looping && interval.end - interval.start >= 1.)
            || (phase > interval.start && phase <= interval.end)
            || (interval.looping && phase + 1. > interval.start && phase + 1. <= interval.end)
    })
}
impl ModelFootPlacement {
    /// Validates authored values before rig-dependent admission.
    pub fn validate(&self) -> Result<(), String> {
        if self.feet.len() > 8 {
            return Err("foot binding capacity exceeded".into());
        }
        for foot in &self.feet {
            if foot
                .bones
                .iter()
                .any(|name| name.is_empty() || name.len() > 1024 || name.contains('\0'))
                || !foot.weight.is_finite()
                || !(0. ..=1.).contains(&foot.weight)
                || [foot.sole_offset, foot.sole_up, foot.pole]
                    .into_iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 1e6)
                || (DVec3::from_array(foot.sole_up.map(f64::from)).length_squared() - 1.).abs()
                    > 1e-6
            {
                return Err("invalid authored foot binding".into());
            }
            if foot.clip_contact_curves.len() > 64
                || foot.clip_contact_curves.keys().any(|name| name.is_empty() || name.len() > 1024 || name.contains('\0'))
            {
                return Err("invalid foot clip contact mapping".into());
            }
            let total_keys = foot.clip_contact_curves.values().try_fold(
                foot.contact_curve.len(), |total, keys| total.checked_add(keys.len()));
            if total_keys.is_none_or(|count| count > 4096) {
                return Err("aggregate foot contact key budget exceeded".into());
            }
            for keys in std::iter::once(&foot.contact_curve).chain(foot.clip_contact_curves.values()) {
            if keys.len() > 4096
                || (!keys.is_empty()
                    && (keys.len() < 2
                        || keys[0].phase != 0.
                        || keys[keys.len() - 1].phase != 1.
                        || keys.iter().any(|k| {
                            !k.phase.is_finite()
                                || !(0. ..=1.).contains(&k.phase)
                                || !k.weight.is_finite()
                                || !(0. ..=1.).contains(&k.weight)
                        })
                        || keys.windows(2).any(|pair| pair[0].phase >= pair[1].phase)))
            {
                return Err("invalid foot contact curve".into());
            }
            }
            foot.contact.validate().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
struct BoundFoot {
    joints: [u16; 3],
    chain: TwoBoneChain,
    state: FootContactState,
    blend: ContactBlendState,
}
#[derive(Clone, Debug)]
pub(super) struct FootRuntime {
    settings: Arc<ModelFootPlacement>,
    feet: Vec<BoundFoot>,
}
impl FootRuntime {
    pub(super) fn matches(&self, settings: &ModelFootPlacement) -> bool {
        self.settings.as_ref() == settings
    }
    pub(super) fn new(model: &ModelAsset, settings: ModelFootPlacement) -> Result<Self, String> {
        settings.validate()?;
        let mut feet = Vec::new();
        for foot in &settings.feet {
            for name in foot.clip_contact_curves.keys() {
                if model.animations.iter().filter(|clip| clip.name() == name).count() != 1 {
                    return Err("foot contact clip name is missing or ambiguous".into());
                }
            }
            let joints = [
                model
                    .resolve_joint_name(&foot.bones[0])
                    .map_err(|e| e.to_string())?,
                model
                    .resolve_joint_name(&foot.bones[1])
                    .map_err(|e| e.to_string())?,
                model
                    .resolve_joint_name(&foot.bones[2])
                    .map_err(|e| e.to_string())?,
            ];
            // Sequential solves must not move another foot's ancestors or chain.
            for previous in &feet {
                let previous: &BoundFoot = previous;
                for &a in &joints {
                    for &b in &previous.joints {
                        if ancestor(model, a, b) || ancestor(model, b, a) {
                            return Err("foot IK chains overlap or depend on each other".into());
                        }
                    }
                }
            }
            feet.push(BoundFoot {
                joints,
                chain: TwoBoneChain::new(&model.skeleton, joints).map_err(|e| e.to_string())?,
                state: FootContactState::default(),
                blend: ContactBlendState::default(),
            });
        }
        Ok(Self {
            settings: Arc::new(settings),
            feet,
        })
    }
    #[cfg(test)]
    fn correct(
        &mut self,
        model: &ModelAsset,
        frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
    ) -> Result<AnimatorFrame, String> {
        self.correct_at_phase(model, frame, actor, grounded, preview, budget, None, None, None, None, None, None)
    }
    pub(super) fn correct_at_phase(
        &mut self,
        model: &ModelAsset,
        mut frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
        clip_name: Option<&str>,
        phase: Option<f64>,
        interval: Option<voxy_animation::AnimationPhaseInterval>,
        blend: Option<voxy_animation::PoseBlendPhases<'_>>,
        source_interval: Option<&crate::model_playback::SourceContactInterval>,
        frozen_tick: Option<&(Arc<voxy_animation::Pose>, f64)>,
    ) -> Result<AnimatorFrame, String> {
        let inverse = actor.inverse();
        if !inverse.is_finite() {
            return Err("foot actor frame is singular".into());
        }
        if let Some(source) = source_interval {
            if !source.active_tick_fraction.is_finite() || !(0. ..=1.).contains(&source.active_tick_fraction)
                || !source.phase.start.is_finite() || !source.phase.end.is_finite()
                || source.phase.end < source.phase.start {
                return Err("invalid source contact interval".into());
            }
            for binding in &self.settings.feet {
                binding.contact_keys(Some(source.clip.name()))?;
            }
        }
        for (foot, binding) in self.feet.iter_mut().zip(&self.settings.feet) {
            let keys = binding.contact_keys(clip_name)?;
            let frozen_event = frozen_tick.map(|(snapshot, fraction)|
                foot.blend.snapshot_weight(snapshot).map(|weight| (weight, *fraction))).transpose()?;
            let weight = binding.weight * foot.blend.sample(binding, clip_name, phase, blend)?;
            let swing = if let Some(interval) = interval {
                let source = if let Some(source) = source_interval {
                    Some(contact_events::SourceTravel::Clip {
                        keys: binding.contact_keys(Some(source.clip.name()))?, phase: source.phase,
                        active_fraction: source.active_tick_fraction,
                    })
                } else if let Some((weight, active_fraction)) = frozen_event {
                    Some(contact_events::SourceTravel::Frozen { weight, active_fraction })
                } else { None };
                contact_events::mixed_swing(keys, interval, source, budget)?
            } else { false };
            if swing {
                foot.state = FootContactState::default();
            }
            let mut globals: Vec<DMat4> = Vec::with_capacity(frame.pose.local().len());
            let mut rotations: Vec<DQuat> = Vec::with_capacity(globals.capacity());
            for (local, joint) in frame.pose.local().iter().zip(model.skeleton.joints()) {
                let q = DQuat::from_array(local.rotation.to_array().map(f64::from)).normalize();
                let signs = local.scale.signum().as_dvec3();
                let signed =
                    DQuat::from_mat3(&DMat3::from_diagonal(signs * signs.x * signs.y * signs.z));
                let local_matrix = DMat4::from_scale_rotation_translation(
                    local.scale.as_dvec3(),
                    q,
                    local.translation.as_dvec3(),
                );
                let global = joint
                    .parent
                    .map_or(local_matrix, |p| globals[usize::from(p)] * local_matrix);
                let rotation = joint
                    .parent
                    .map_or(q * signed, |p| rotations[usize::from(p)] * q * signed);
                globals.push(global);
                rotations.push(rotation.normalize());
            }
            let tip = usize::from(foot.joints[2]);
            let matrix = globals[tip];
            let offset =
                matrix.transform_vector3(DVec3::from_array(binding.sole_offset.map(f64::from)));
            let sole = actor.transform_point3(matrix.w_axis.truncate() + offset);
            let mut candidate = foot
                .state
                .prepare(
                    &preview.support,
                    binding.contact,
                    FootContactInput {
                        sole,
                        up: DVec3::Y,
                        grounded,
                        plant: binding.plant && weight > 0.,
                    },
                    budget,
                )
                .map_err(|e| e.to_string())?;
            if let Some(contact) = candidate.contact {
                let normal = DMat3::from_mat4(actor).transpose() * contact.normal;
                let current_up = matrix
                    .inverse()
                    .transpose()
                    .transform_vector3(DVec3::from_array(binding.sole_up.map(f64::from)));
                if !current_up.is_finite()
                    || current_up.length() == 0.
                    || !normal.is_finite()
                    || normal.length() == 0.
                {
                    return Err("invalid foot normal transform".into());
                }
                let delta = DQuat::from_rotation_arc(current_up.normalize(), normal.normalize());
                let target = inverse.transform_point3(contact.position) - delta * offset;
                let rotation = (delta * rotations[tip]).normalize();
                let (corrected, report) = frame
                    .clone()
                    .with_two_bone_ik(
                        &model.skeleton,
                        &foot.chain,
                        TwoBoneTarget {
                            position: target.as_vec3(),
                            pole: glam::Vec3::from_array(binding.pole),
                            rotation: Some(
                                glam::Quat::from_array(rotation.to_array().map(|v| v as f32))
                                    .normalize(),
                            ),
                            weight,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                if report.reach_clamped {
                    candidate.state = candidate.state.release_until_swing();
                } else {
                    frame = corrected;
                }
            }
            foot.state = candidate.state;
        }
        Ok(frame)
    }
}
fn ancestor(model: &ModelAsset, a: u16, mut b: u16) -> bool {
    loop {
        if a == b {
            return true;
        }
        let Some(parent) = model.skeleton.joints()[usize::from(b)].parent else {
            return false;
        };
        b = parent;
    }
}

#[cfg(test)]
mod tests;
