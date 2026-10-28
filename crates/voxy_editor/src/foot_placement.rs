//! Authored rig bindings; contact and palettes are staged against accepted physics.
#[path = "foot_runtime_checkpoint.rs"]
pub(crate) mod checkpoint;
mod contact_events;
use glam::{DMat3, DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use voxy_animation::{AnimatorFrame, TwoBoneChain, TwoBoneTarget};
use voxy_gameplay::{
    CharacterTickPreview, FootContactInput, FootContactSettings, FootContactState,
    SupportQueryBudget,
};
use voxy_model::ModelAsset;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFootPlacement {
    pub feet: Vec<FootBinding>,
}
/// Orientation policy is independent of support-relative pivot locking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FootOrientation {
    /// Align the authored sole normal to the support, retaining authored twist.
    #[default]
    AlignSupport,
    /// Retain the sampled ankle orientation while locking its sole pivot.
    /// This permits authored heel/toe roll; it does not prevent other foot
    /// geometry from penetrating the support or select a new pivot.
    Authored,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootBinding {
    pub bones: [String; 3],
    /// Tip-local sole point and sole outward/up axis.
    pub sole_offset: [f32; 3],
    pub sole_up: [f32; 3],
    #[serde(default)]
    pub orientation: FootOrientation,
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
    /// Step-valued target-rig ankle-local pivot keys; separate from stance weights.
    #[serde(default)]
    pub clip_pivot_keys: std::collections::BTreeMap<String, Vec<FootPivotKey>>,
    /// Optional target-ankle-local reference geometry, compiled once at binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_geometry: Option<FootSupportGeometry>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootSupportGeometry {
    pub points: Vec<[f64; 3]>,
    pub max_projection_distance: f64,
}
impl FootSupportGeometry {
    fn validate(&self) -> Result<(), String> {
        if self.points.is_empty()
            || self.points.len() > voxy_animation::MAX_FOOT_SUPPORT_POINTS
            || self
                .points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e6)
            || !self.max_projection_distance.is_finite()
            || !(0. ..=1e6).contains(&self.max_projection_distance)
        {
            return Err("invalid foot support geometry".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootContactKey {
    pub phase: f32,
    pub weight: f32,
}
/// Explicit pivot request in normalized clip coordinates. Changes are discrete;
/// physical admission and IK own whether the requested point may be accepted.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootPivotKey {
    pub phase: f32,
    pub pivot: [f32; 3],
    pub max_projection_distance: f64,
}
impl FootBinding {
    fn pivot_at_phase(
        &self,
        clip: Option<&str>,
        phase: Option<f64>,
    ) -> Result<Option<FootPivotKey>, String> {
        if self.clip_pivot_keys.is_empty() {
            return Ok(None);
        }
        let keys = self
            .clip_pivot_keys
            .get(clip.ok_or("pivot keys require a named clip")?)
            .ok_or("missing foot pivot clip mapping")?;
        let phase = phase.ok_or("pivot keys require a clip phase")?;
        if !phase.is_finite() || !(0. ..=1.).contains(&phase) {
            return Err("invalid foot pivot phase".into());
        }
        let index = keys
            .partition_point(|key| f64::from(key.phase) <= phase)
            .saturating_sub(1);
        Ok(Some(keys[index]))
    }
    pub(super) fn contact_keys(
        &self,
        clip_name: Option<&str>,
    ) -> Result<&[FootContactKey], String> {
        if self.clip_contact_curves.is_empty() {
            return Ok(&self.contact_curve);
        }
        self.clip_contact_curves
            .get(clip_name.ok_or("foot clip curves require an active clip")?)
            .map(Vec::as_slice)
            .ok_or_else(|| "selected clip has no foot contact curve".into())
    }
}
#[derive(Clone, Debug, Default)]
struct ContactBlendState {
    last_weight: Option<f32>,
    frozen: Option<(Arc<voxy_animation::Pose>, f32)>,
}
impl ContactBlendState {
    fn snapshot_weight(&mut self, snapshot: &Arc<voxy_animation::Pose>) -> Result<f32, String> {
        if self
            .frozen
            .as_ref()
            .is_none_or(|(held, _)| !Arc::ptr_eq(held, snapshot))
        {
            self.frozen = Some((
                snapshot.clone(),
                self.last_weight
                    .ok_or("interrupted foot transition has no accepted contact snapshot")?,
            ));
        }
        Ok(self.frozen.as_ref().unwrap().1)
    }

    fn sample(
        &mut self,
        binding: &FootBinding,
        clip_name: Option<&str>,
        phase: Option<f64>,
        blend: Option<voxy_animation::PoseBlendPhases<'_>>,
    ) -> Result<f32, String> {
        let target = contact_weight(binding.contact_keys(clip_name)?, phase)?;
        let weight = match blend
            .and_then(|blend| blend.source.map(|source| (source, blend.target_weight)))
        {
            None => {
                self.frozen = None;
                target
            }
            Some((source, alpha)) => {
                let source_weight = match source {
                    voxy_animation::PoseBlendSource::Clip(source) => {
                        self.frozen = None;
                        contact_weight(
                            binding.contact_keys(Some(source.clip.name()))?,
                            Some(source.normalized_phase),
                        )?
                    }
                    voxy_animation::PoseBlendSource::FrozenPose(snapshot) => {
                        self.snapshot_weight(snapshot)?
                    }
                };
                (f64::from(source_weight)
                    + (f64::from(target) - f64::from(source_weight)) * f64::from(alpha))
                    as f32
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
            if let Some(geometry) = &foot.support_geometry {
                geometry.validate()?;
                if foot.orientation != FootOrientation::Authored || !foot.clip_pivot_keys.is_empty()
                {
                    return Err(
                        "support geometry conflicts with pivot or orientation ownership".into(),
                    );
                }
            }
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
                || foot
                    .clip_contact_curves
                    .keys()
                    .any(|name| name.is_empty() || name.len() > 1024 || name.contains('\0'))
            {
                return Err("invalid foot clip contact mapping".into());
            }
            if foot.clip_pivot_keys.len() > 64
                || foot
                    .clip_pivot_keys
                    .keys()
                    .any(|name| name.is_empty() || name.len() > 1024 || name.contains('\0'))
                || foot.clip_pivot_keys.values().map(Vec::len).sum::<usize>() > 4096
            {
                return Err("invalid foot pivot key budget or clip name".into());
            }
            for keys in foot.clip_pivot_keys.values() {
                if keys.len() < 2
                    || keys[0].phase != 0.
                    || keys[keys.len() - 1].phase != 1.
                    || keys[0].pivot != keys[keys.len() - 1].pivot
                    || keys[0].max_projection_distance
                        != keys[keys.len() - 1].max_projection_distance
                    || keys.windows(2).any(|pair| pair[0].phase >= pair[1].phase)
                    || keys.iter().any(|key| {
                        !key.phase.is_finite()
                            || !(0. ..=1.).contains(&key.phase)
                            || key.pivot.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
                            || !key.max_projection_distance.is_finite()
                            || !(0. ..=1e6).contains(&key.max_projection_distance)
                    })
                {
                    return Err("invalid foot pivot keys".into());
                }
            }
            let total_keys = foot
                .clip_contact_curves
                .values()
                .try_fold(foot.contact_curve.len(), |total, keys| {
                    total.checked_add(keys.len())
                });
            if total_keys.is_none_or(|count| count > 4096) {
                return Err("aggregate foot contact key budget exceeded".into());
            }
            for keys in
                std::iter::once(&foot.contact_curve).chain(foot.clip_contact_curves.values())
            {
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
    pivot: DVec3,
    pending_pivot: Option<(DVec3, f64)>,
    geometry: Option<(voxy_animation::FootSupportPoints, f64)>,
    blend: ContactBlendState,
}
#[derive(Clone, Debug)]
pub struct FootRuntime {
    skeleton: voxy_animation::Skeleton,
    settings: Arc<ModelFootPlacement>,
    feet: Vec<BoundFoot>,
}
#[allow(
    dead_code,
    reason = "constructors and pivot requests of the in-progress foot placement path"
)]
impl FootRuntime {
    /// Reads only the published support anchor and accepted ankle-local pivot.
    /// Pending proposals are excluded. The supplied physics snapshot owns
    /// resolving the anchor to its current world position.
    pub fn support_lock(
        &self,
        index: usize,
    ) -> Result<Option<(voxy_gameplay::SupportAnchor, DVec3)>, String> {
        let foot = self.feet.get(index).ok_or("unknown foot support lock")?;
        Ok(foot.state.anchor().map(|anchor| (anchor, foot.pivot)))
    }
    pub(super) fn settings(&self) -> &ModelFootPlacement {
        &self.settings
    }
    pub(super) fn matches(&self, settings: &ModelFootPlacement) -> bool {
        self.settings.as_ref() == settings
    }
    pub(super) fn new(model: &ModelAsset, settings: ModelFootPlacement) -> Result<Self, String> {
        Self::new_with_clips(model, settings, &model.animations)
    }
    pub(super) fn new_with_clips(
        model: &ModelAsset,
        settings: ModelFootPlacement,
        clips: &[Arc<voxy_animation::AnimationClip>],
    ) -> Result<Self, String> {
        Self::new_resolved(&model.skeleton, settings, clips, |name| {
            model.resolve_joint_name(name).map_err(|e| e.to_string())
        })
    }
    /// Binds native rigs without requiring a render model or duplicate IK owner.
    pub fn from_skeleton(
        skeleton: &voxy_animation::Skeleton,
        settings: ModelFootPlacement,
        clips: &[Arc<voxy_animation::AnimationClip>],
    ) -> Result<Self, String> {
        Self::new_resolved(skeleton, settings, clips, |name| {
            skeleton
                .joints()
                .iter()
                .position(|joint| joint.name.as_ref() == name)
                .map(|index| index as u16)
                .ok_or_else(|| format!("missing foot joint: {name}"))
        })
    }
    fn new_resolved(
        skeleton: &voxy_animation::Skeleton,
        settings: ModelFootPlacement,
        clips: &[Arc<voxy_animation::AnimationClip>],
        resolve: impl Fn(&str) -> Result<u16, String>,
    ) -> Result<Self, String> {
        settings.validate()?;
        let mut feet = Vec::new();
        for foot in &settings.feet {
            for name in foot
                .clip_contact_curves
                .keys()
                .chain(foot.clip_pivot_keys.keys())
            {
                if clips.iter().filter(|clip| clip.name() == name).count() != 1 {
                    return Err("foot contact clip name is missing or ambiguous".into());
                }
            }
            let joints = [
                resolve(&foot.bones[0])?,
                resolve(&foot.bones[1])?,
                resolve(&foot.bones[2])?,
            ];
            // Sequential solves must not move another foot's ancestors or chain.
            for previous in &feet {
                let previous: &BoundFoot = previous;
                for &a in &joints {
                    for &b in &previous.joints {
                        if ancestor(skeleton, a, b) || ancestor(skeleton, b, a) {
                            return Err("foot IK chains overlap or depend on each other".into());
                        }
                    }
                }
            }
            feet.push(BoundFoot {
                joints,
                chain: TwoBoneChain::new(skeleton, joints).map_err(|e| e.to_string())?,
                state: FootContactState::default(),
                pivot: DVec3::from_array(foot.sole_offset.map(f64::from)),
                pending_pivot: None,
                geometry: foot
                    .support_geometry
                    .as_ref()
                    .map(|geometry| {
                        voxy_animation::FootSupportPoints::new(
                            geometry
                                .points
                                .iter()
                                .copied()
                                .map(DVec3::from_array)
                                .collect(),
                        )
                        .map(|points| (points, geometry.max_projection_distance))
                        .map_err(|e| e.to_string())
                    })
                    .transpose()?,
                blend: ContactBlendState::default(),
            });
        }
        Ok(Self {
            skeleton: skeleton.clone(),
            settings: Arc::new(settings),
            feet,
        })
    }
    /// Queues an explicit authoring/gameplay request; only accepted physics and
    /// successful IK may publish it. Rejection keeps the request pending for retry.
    pub(super) fn request_pivot_transfer(
        &mut self,
        foot: usize,
        pivot: DVec3,
        max_projection: f64,
    ) -> Result<(), String> {
        if !pivot.is_finite()
            || pivot.abs().max_element() > 1e6
            || !max_projection.is_finite()
            || !(0. ..=1e6).contains(&max_projection)
        {
            return Err("invalid foot pivot transfer".into());
        }
        let foot = self
            .feet
            .get_mut(foot)
            .ok_or("unknown foot pivot transfer")?;
        if foot.state.anchor().is_none() {
            return Err("pivot transfer requires an active support".into());
        }
        foot.pending_pivot = Some((pivot, max_projection));
        Ok(())
    }
    /// Installs immutable ankle-local support geometry. Authored pivot keys and
    /// automatic geometry are mutually exclusive owners of pivot selection.
    pub(super) fn set_support_geometry(
        &mut self,
        index: usize,
        points: voxy_animation::FootSupportPoints,
        max_projection: f64,
    ) -> Result<(), String> {
        let binding = self
            .settings
            .feet
            .get(index)
            .ok_or("unknown geometry foot")?;
        if binding.orientation != FootOrientation::Authored
            || !binding.clip_pivot_keys.is_empty()
            || !max_projection.is_finite()
            || !(0. ..=1e6).contains(&max_projection)
        {
            return Err(
                "support geometry requires authored orientation and exclusive pivot ownership"
                    .into(),
            );
        }
        self.feet[index].geometry = Some((points, max_projection));
        Ok(())
    }
    /// Selects a reference-geometry pivot using the current accepted support
    /// normal. This only queues a request; correction still admits it physically.
    pub(super) fn request_geometry_pivot(
        &mut self,
        foot_index: usize,
        points: &voxy_animation::FootSupportPoints,
        model: &ModelAsset,
        frame: &AnimatorFrame,
        actor: DMat4,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
        max_projection: f64,
    ) -> Result<(), String> {
        if !actor.is_finite()
            || !actor.inverse().is_finite()
            || !max_projection.is_finite()
            || !(0. ..=1e6).contains(&max_projection)
            || frame.pose.local().len() != model.skeleton.joints().len()
        {
            return Err("invalid geometry pivot frame".into());
        }
        let foot = self
            .feet
            .get(foot_index)
            .ok_or("unknown geometry pivot foot")?;
        let anchor = foot
            .state
            .anchor()
            .ok_or("geometry pivot requires active support")?;
        let contact = preview
            .support
            .resolve(anchor, budget)
            .map_err(|e| e.to_string())?
            .ok_or("geometry pivot support is missing")?;
        // Reserve all matrix composition and projection work before either loop.
        // Physics resolution already spent its own work and is not rolled back.
        let work = frame
            .pose
            .local()
            .len()
            .checked_add(points.point_count())
            .ok_or("geometry pivot work overflow")?;
        budget.charge_work(work).map_err(|e| e.to_string())?;
        let mut globals: Vec<DMat4> = Vec::with_capacity(frame.pose.local().len());
        for (local, joint) in frame.pose.local().iter().zip(model.skeleton.joints()) {
            let matrix = DMat4::from_scale_rotation_translation(
                local.scale.as_dvec3(),
                DQuat::from_array(local.rotation.to_array().map(f64::from)),
                local.translation.as_dvec3(),
            );
            globals.push(
                joint
                    .parent
                    .map_or(matrix, |p| globals[usize::from(p)] * matrix),
            );
        }
        let linear = DMat3::from_mat4(actor * globals[usize::from(foot.joints[2])]);
        let selected = points
            .support(linear, contact.normal)
            .map_err(|e| e.to_string())?;
        self.request_pivot_transfer(foot_index, selected.local, max_projection)
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
        self.correct_at_phase(
            model, frame, actor, grounded, preview, budget, None, None, None, None, None, None,
        )
    }
    pub(super) fn correct_at_phase(
        &mut self,
        model: &ModelAsset,
        frame: AnimatorFrame,
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
        let mut staged = self.clone();
        let frame = staged.correct_at_phase_inner(
            &model.skeleton,
            frame,
            actor,
            grounded,
            preview,
            budget,
            clip_name,
            phase,
            interval,
            blend,
            source_interval,
            frozen_tick,
        )?;
        *self = staged;
        Ok(frame)
    }
    /// Corrects explicitly configured fixed stance against an accepted physical
    /// snapshot. Caller publishes this staged owner and frame only after its
    /// CharacterPhysics preparation transaction succeeds. Phase-driven bindings
    /// must use the editor's phase-aware playback path instead.
    pub fn correct_fixed_stance(
        &mut self,
        frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
    ) -> Result<AnimatorFrame, String> {
        if self.settings.feet.iter().any(|foot| {
            !foot.contact_curve.is_empty()
                || !foot.clip_contact_curves.is_empty()
                || !foot.clip_pivot_keys.is_empty()
        }) {
            return Err("fixed stance cannot consume clip-driven foot bindings".into());
        }
        let mut staged = self.clone();
        let skeleton = staged.skeleton.clone();
        let frame = staged.correct_at_phase_inner(
            &skeleton, frame, actor, grounded, preview, budget, None, None, None, None, None, None,
        )?;
        *self = staged;
        Ok(frame)
    }
    fn correct_at_phase_inner(
        &mut self,
        skeleton: &voxy_animation::Skeleton,
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
            if !source.active_tick_fraction.is_finite()
                || !(0. ..=1.).contains(&source.active_tick_fraction)
                || !source.phase.start.is_finite()
                || !source.phase.end.is_finite()
                || source.phase.end < source.phase.start
            {
                return Err("invalid source contact interval".into());
            }
            for binding in &self.settings.feet {
                binding.contact_keys(Some(source.clip.name()))?;
            }
        }
        for (foot, binding) in self.feet.iter_mut().zip(&self.settings.feet) {
            let keys = binding.contact_keys(clip_name)?;
            let frozen_event = frozen_tick
                .map(|(snapshot, fraction)| {
                    foot.blend
                        .snapshot_weight(snapshot)
                        .map(|weight| (weight, *fraction))
                })
                .transpose()?;
            let weight = binding.weight * foot.blend.sample(binding, clip_name, phase, blend)?;
            let swing = if let Some(interval) = interval {
                let source = if let Some(source) = source_interval {
                    Some(contact_events::SourceTravel::Clip {
                        keys: binding.contact_keys(Some(source.clip.name()))?,
                        phase: source.phase,
                        active_fraction: source.active_tick_fraction,
                    })
                } else if let Some((weight, active_fraction)) = frozen_event {
                    Some(contact_events::SourceTravel::Frozen {
                        weight,
                        active_fraction,
                    })
                } else {
                    None
                };
                contact_events::mixed_swing(
                    keys,
                    interval,
                    source,
                    blend.is_some_and(|phases| phases.source.is_some()),
                    budget,
                )
                .map_err(|e| e.to_string())?
            } else {
                false
            };
            if swing {
                foot.state = FootContactState::default();
            }
            if swing || !grounded || !binding.plant || weight == 0. {
                foot.pending_pivot = None;
                foot.pivot = DVec3::from_array(binding.sole_offset.map(f64::from));
            }
            let requested = binding.pivot_at_phase(clip_name, phase)?;
            if let Some(key) = requested {
                budget.charge_work(1).map_err(|e| e.to_string())?;
                // Pivot identity is held throughout both live and frozen fades.
                // Resume target phase ownership only after the source disappears.
                if blend.is_none_or(|phases| phases.source.is_none())
                    && grounded
                    && binding.plant
                    && weight > 0.
                {
                    let pivot = DVec3::from_array(key.pivot.map(f64::from));
                    if foot.state.anchor().is_none() {
                        foot.pivot = pivot;
                        foot.pending_pivot = None;
                    } else if pivot != foot.pivot {
                        foot.pending_pivot = Some((pivot, key.max_projection_distance));
                    } else {
                        foot.pending_pivot = None;
                    }
                } else if blend.is_some_and(|phases| phases.source.is_some()) {
                    foot.pending_pivot = None;
                }
            }
            let mut globals: Vec<DMat4> = Vec::with_capacity(frame.pose.local().len());
            let mut rotations: Vec<DQuat> = Vec::with_capacity(globals.capacity());
            for (local, joint) in frame.pose.local().iter().zip(skeleton.joints()) {
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
            let mut offset = matrix.transform_vector3(foot.pivot);
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
            if candidate.contact.is_none() {
                foot.pending_pivot = None;
            }
            let mut acquisition_pivot = None;
            // The first accepted landing must use current geometry too. Waiting
            // until the following frame can plant a neutral pivot through a
            // rolled foot. Reprobe the selected point on the same support.
            if foot.state.anchor().is_none() && grounded && binding.plant && weight == 1. {
                if let (Some((geometry, _)), Some(contact)) = (&foot.geometry, candidate.contact) {
                    budget
                        .charge_work(geometry.point_count())
                        .map_err(|e| e.to_string())?;
                    let selected = geometry
                        .support(DMat3::from_mat4(actor * matrix), contact.normal)
                        .map_err(|e| e.to_string())?;
                    if selected.local != foot.pivot {
                        let proposed_offset = matrix.transform_vector3(selected.local);
                        let proposed =
                            actor.transform_point3(matrix.w_axis.truncate() + proposed_offset);
                        let replacement = foot
                            .state
                            .prepare(
                                &preview.support,
                                binding.contact,
                                FootContactInput {
                                    sole: proposed,
                                    up: DVec3::Y,
                                    grounded,
                                    plant: true,
                                },
                                budget,
                            )
                            .map_err(|e| e.to_string())?;
                        if replacement
                            .contact
                            .is_some_and(|c| c.anchor.owner() == contact.anchor.owner())
                        {
                            candidate = replacement;
                            offset = proposed_offset;
                            acquisition_pivot = Some(selected.local);
                        } else {
                            // Do not publish a known obsolete geometric landing.
                            candidate.contact = None;
                            candidate.state = foot.state;
                        }
                    }
                }
            }
            let mut automatic_request = false;
            // Hold accepted geometry through live/frozen fades. An explicit
            // pending gameplay request has priority over automatic selection.
            if foot.pending_pivot.is_none()
                && foot.state.anchor().is_some()
                && grounded
                && binding.plant
                && weight == 1.
                && blend.is_none_or(|phases| phases.source.is_none())
            {
                if let (Some((geometry, limit)), Some(contact)) =
                    (&foot.geometry, candidate.contact)
                {
                    budget
                        .charge_work(geometry.point_count())
                        .map_err(|e| e.to_string())?;
                    let linear = DMat3::from_mat4(actor * matrix);
                    let selected = geometry
                        .support(linear, contact.normal)
                        .map_err(|e| e.to_string())?;
                    if selected.local != foot.pivot {
                        foot.pending_pivot = Some((selected.local, *limit));
                        automatic_request = true;
                    }
                }
            }
            let mut transferring = false;
            if let Some((pivot, limit)) = foot.pending_pivot {
                let proposed_offset = matrix.transform_vector3(pivot);
                // A pivot replacement starts from the currently admitted lock,
                // not the unconstrained animation ankle. Otherwise pelvis motion
                // spends the replacement's projection limit a second time.
                let lock_correction = candidate
                    .contact
                    .map_or(DVec3::ZERO, |contact| contact.position - sole);
                let proposed = actor.transform_point3(matrix.w_axis.truncate() + proposed_offset)
                    + lock_correction;
                if let Some(admitted) = foot
                    .state
                    .prepare_pivot_transfer(
                        &preview.support,
                        binding.contact,
                        limit,
                        FootContactInput {
                            sole: proposed,
                            up: DVec3::Y,
                            grounded,
                            plant: binding.plant && weight > 0.,
                        },
                        budget,
                    )
                    .map_err(|e| e.to_string())?
                {
                    // Partial-strength IK cannot guarantee the replacement pivot.
                    if weight == 1. {
                        candidate.state = admitted.state;
                        candidate.contact = Some(admitted.contact);
                        offset = proposed_offset;
                        transferring = true;
                    }
                }
            }
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
                let delta = match binding.orientation {
                    FootOrientation::AlignSupport => {
                        DQuat::from_rotation_arc(current_up.normalize(), normal.normalize())
                    }
                    FootOrientation::Authored => DQuat::IDENTITY,
                };
                let target = inverse.transform_point3(contact.position) - delta * offset;
                let rotation = (delta * rotations[tip]).normalize();
                let (corrected, report) = frame
                    .clone()
                    .with_two_bone_ik(
                        skeleton,
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
                    candidate.state = if transferring {
                        foot.state
                    } else {
                        candidate.state.release_until_swing()
                    };
                } else {
                    if let Some(pivot) = acquisition_pivot {
                        foot.pivot = pivot;
                    }
                    if transferring {
                        foot.pivot = foot.pending_pivot.expect("admitted pivot request").0;
                        foot.pending_pivot = None;
                    }
                    frame = corrected;
                }
            }
            // Re-evaluate rejected automatic proposals from next frame's pose;
            // explicit gameplay requests retain their existing retry semantics.
            if automatic_request {
                foot.pending_pivot = None;
            }
            foot.state = candidate.state;
        }
        Ok(frame)
    }
}
fn ancestor(skeleton: &voxy_animation::Skeleton, a: u16, mut b: u16) -> bool {
    loop {
        if a == b {
            return true;
        }
        let Some(parent) = skeleton.joints()[usize::from(b)].parent else {
            return false;
        };
        b = parent;
    }
}

#[cfg(test)]
mod tests;
