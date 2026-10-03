//! Authored rig bindings; contact and palettes are staged against accepted physics.
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
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootContactKey {
    pub phase: f32,
    pub weight: f32,
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
#[derive(Clone, Debug)]
struct BoundFoot {
    joints: [u16; 3],
    chain: TwoBoneChain,
    state: FootContactState,
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
        if settings.feet.len() > 8 {
            return Err("foot binding capacity exceeded".into());
        }
        let mut feet = Vec::new();
        for foot in &settings.feet {
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
            let keys = &foot.contact_curve;
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
        self.correct_at_phase(model, frame, actor, grounded, preview, budget, None, None)
    }
    pub(super) fn correct_at_phase(
        &mut self,
        model: &ModelAsset,
        mut frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
        phase: Option<f64>,
        interval: Option<voxy_animation::AnimationPhaseInterval>,
    ) -> Result<AnimatorFrame, String> {
        let inverse = actor.inverse();
        if !inverse.is_finite() {
            return Err("foot actor frame is singular".into());
        }
        for (foot, binding) in self.feet.iter_mut().zip(&self.settings.feet) {
            let weight = binding.weight * contact_weight(&binding.contact_curve, phase)?;
            if crossed_swing(&binding.contact_curve, interval) {
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
