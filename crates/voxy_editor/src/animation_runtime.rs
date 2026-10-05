//! Fixed-tick animation authority. Rendering receives immutable accepted frames.
use crate::{ModelAnimation, ModelInstance, ModelPart, model_playback::ModelPlayback};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use voxy_animation::AnimatorFrame;
use voxy_assets::AssetId;
use voxy_render::ModelAsset;
use voxy_scene::{NodeId, SceneGraph, SceneId};

pub(super) const MAX_OWNERS: usize = 128;

/// The editor adds its playback authority to the existing character transaction.
pub(super) fn schedule() -> Result<&'static voxy_scene::SchedulePlan, voxy_scene::ScheduleError> {
    static PLAN: std::sync::OnceLock<Result<voxy_scene::SchedulePlan, voxy_scene::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        voxy_gameplay::gameplay_schedule()?.with_resource_access(
            "character.step",
            voxy_scene::SystemAccess {
                resource: "animation.playback".into(),
                write: true,
            },
        )
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[derive(Clone, Copy, Debug)]
struct RootReference {
    scale: voxy_animation::RootUniformScaleEnclosure,
    body_to_world: voxy_animation::RootRigidEnclosure,
    authored_to_body: voxy_animation::RootRigidEnclosure,
}
impl RootReference {
    fn matches(&self, other: &Self) -> bool {
        self.scale.bounds() == other.scale.bounds()
            && self.body_to_world.translation_bounds() == other.body_to_world.translation_bounds()
            && self.body_to_world.rotation_bounds() == other.body_to_world.rotation_bounds()
            && self.authored_to_body.translation_bounds()
                == other.authored_to_body.translation_bounds()
            && self.authored_to_body.rotation_bounds() == other.authored_to_body.rotation_bounds()
    }
}

#[derive(Debug)]
pub(super) struct PreparedOwnerFade {
    motion: crate::model_playback::PreparedModelFadeMotion,
    scene: SceneId,
    owner: NodeId,
    asset: AssetId,
    settings: ModelAnimation,
    reference: RootReference,
    model: Arc<ModelAsset>,
    animation_model: Arc<ModelAsset>,
    retarget_profile: Option<crate::ModelRetarget>,
    feet_settings: Option<crate::ModelFootPlacement>,
}
/// Collision admission policy for a fade already expressed in body-local space.
#[derive(Clone, Copy)]
pub(super) struct OwnerFadeAdmission {
    pub coordinate_axis: usize,
    pub evaluation_radius: f64,
    pub evaluation_axes: Option<[f64; 3]>,
}

pub(super) struct AdmittedOwnerFade<'a> {
    prepared: &'a PreparedOwnerFade,
}
impl AdmittedOwnerFade<'_> {
    /// Common-reference preparation already maps the field into body-local space.
    pub(super) fn request(
        &self,
        coordinate_axis: usize,
        evaluation_radius: f64,
    ) -> voxy_gameplay::CharacterCertifiedFadeMotion<'_> {
        self.prepared.motion.request(
            glam::DQuat::IDENTITY,
            glam::Vec3::ZERO,
            1.,
            coordinate_axis,
            evaluation_radius,
        )
    }
}
impl PreparedOwnerFade {
    /// Call at the transaction boundary before borrowing the scene for physics.
    pub(super) fn admit_scene<'a>(
        &'a self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
    ) -> Result<AdmittedOwnerFade<'a>, String> {
        self.validate_scene(scene, models)?;
        Ok(AdmittedOwnerFade { prepared: self })
    }
    fn validate_scene(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
    ) -> Result<(), String> {
        if scene.identity() != self.scene {
            return Err("staged fade scene changed".into());
        }
        let instance = scene
            .component::<ModelInstance>(self.owner)
            .map_err(|e| e.to_string())?
            .ok_or("fade model instance disappeared")?;
        if instance.asset != self.asset
            || !models
                .get(&instance.asset)
                .is_some_and(|model| Arc::ptr_eq(model, &self.model))
        {
            return Err("staged fade scene asset changed".into());
        }
        if scene
            .component::<ModelPart>(self.owner)
            .map_err(|e| e.to_string())?
            .is_some_and(|part| part.node != u32::MAX)
        {
            return Err("staged fade owner became a hierarchy part".into());
        }
        let profile = scene
            .component::<crate::ModelRetarget>(self.owner)
            .map_err(|e| e.to_string())?
            .cloned();
        if profile != self.retarget_profile {
            return Err("staged fade scene retarget profile changed".into());
        }
        let mut settings = scene
            .component::<ModelAnimation>(self.owner)
            .map_err(|e| e.to_string())?
            .cloned()
            .unwrap_or(ModelAnimation {
                clip: (!self.animation_model.animations.is_empty()).then_some(0),
                ..Default::default()
            });
        settings.validate(
            Some(self.animation_model.animations.len()),
            Some(self.animation_model.skeleton.joints().len()),
        )?;
        settings.clip = settings.resolve_clip(&self.animation_model)?;
        let feet = scene
            .component::<crate::ModelFootPlacement>(self.owner)
            .map_err(|e| e.to_string())?
            .filter(|feet| !feet.feet.is_empty());
        if feet != self.feet_settings.as_ref() {
            return Err("staged fade scene foot binding changed".into());
        }
        if settings != self.settings {
            return Err("staged fade scene animation settings changed".into());
        }
        Ok(())
    }
    fn matches(&self, owner: &Owner) -> bool {
        Arc::ptr_eq(&self.model, &owner.model)
            && Arc::ptr_eq(&self.animation_model, &owner.animation_model)
            && self.settings == owner.settings
            && self.retarget_profile == owner.retarget_profile
            && self.feet_settings.as_ref() == owner.feet.as_ref().map(|feet| feet.settings())
            && owner
                .root_reference
                .is_some_and(|reference| self.reference.matches(&reference))
    }
}

#[derive(Clone, Debug)]
struct Owner {
    root_reference: Option<RootReference>,
    model: Arc<ModelAsset>,
    animation_model: Arc<ModelAsset>,
    retarget_profile: Option<crate::ModelRetarget>,
    retarget_binding: Option<voxy_animation::RetargetBinding>,
    settings: ModelAnimation,
    playback: ModelPlayback,
    frame: Arc<AnimatorFrame>,
    feet: Option<crate::foot_placement::FootRuntime>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct AnimationRuntime {
    scene: Option<SceneId>,
    owners: HashMap<NodeId, Owner>,
    serial: u64,
    motions: Vec<(NodeId, glam::Vec3)>,
    rotations: Vec<(
        NodeId,
        voxy_animation::RootRigidPath,
        glam::DQuat,
        glam::Vec3,
        f64,
    )>,
}
impl AnimationRuntime {
    /// Captures a caller-selected authored reference at a known body pose.
    /// A live reference cannot be silently reset by another clip switch.
    pub(super) fn initialize_root_reference(
        &mut self,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        body_to_world: voxy_animation::RootRigidTransform,
        authored_to_body: voxy_animation::RootRigidEnclosure,
    ) -> Result<(), String> {
        let body_to_world = voxy_animation::RootRigidEnclosure::from_transform(body_to_world)
            .map_err(|error| error.to_string())?;
        let state = self
            .owners
            .get_mut(&owner)
            .ok_or("root reference owner disappeared")?;
        if !Arc::ptr_eq(expected_model, &state.model) {
            return Err("root reference model changed".into());
        }
        if state.root_reference.is_some() {
            return Err("root reference is already initialized".into());
        }
        state.root_reference = Some(RootReference {
            scale: voxy_animation::RootUniformScaleEnclosure::ONE,
            body_to_world,
            authored_to_body,
        });
        Ok(())
    }

    /// Captures an unblended root phase at an explicit accepted body pose.
    /// The current root frame is chosen by the caller; later clip switches do
    /// not replace this anchor. Stored factor/compiler error remains conditional.
    pub(super) fn initialize_root_reference_at_phase(
        &mut self,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        body_to_world: voxy_animation::RootRigidTransform,
        current_root_to_body: voxy_animation::RootRigidEnclosure,
    ) -> Result<(), String> {
        self.initialize_root_reference_at_scaled_phase(
            owner,
            expected_model,
            body_to_world,
            current_root_to_body,
            voxy_animation::RootUniformScaleEnclosure::ONE,
        )
    }

    fn initialize_root_reference_at_scaled_phase(
        &mut self,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        body_to_world: voxy_animation::RootRigidTransform,
        current_root_to_body: voxy_animation::RootRigidEnclosure,
        scale: voxy_animation::RootUniformScaleEnclosure,
    ) -> Result<(), String> {
        let state = self
            .owners
            .get(&owner)
            .ok_or("root reference owner disappeared")?;
        if !Arc::ptr_eq(expected_model, &state.model) || state.retarget_binding.is_some() {
            return Err("root reference model or binding changed".into());
        }
        if !state.settings.root_motion_rotation {
            return Err("phase root reference requires rigid root extraction".into());
        }
        let reference = state.playback.reference_at_current_phase(
            state.settings.root_motion_axes,
            current_root_to_body,
            scale,
        )?;
        self.initialize_root_reference(owner, expected_model, body_to_world, reference)?;
        self.owners
            .get_mut(&owner)
            .unwrap()
            .root_reference
            .as_mut()
            .unwrap()
            .scale = scale;
        Ok(())
    }

    /// Captures an explicit signed parent similarity at the published solver pose.
    pub(super) fn capture_owner_reference_with_scale(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &voxy_gameplay::CharacterPhysics,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        current_root_to_body: voxy_animation::RootRigidEnclosure,
        scale: f64,
    ) -> Result<Self, String> {
        self.capture_owner_reference_with_similarity(
            scene,
            models,
            physics,
            owner,
            expected_model,
            current_root_to_body,
            voxy_animation::RootUniformScaleEnclosure::from_scale(scale)
                .map_err(|e| e.to_string())?,
        )
    }

    fn capture_owner_reference_with_similarity(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &voxy_gameplay::CharacterPhysics,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        current_root_to_body: voxy_animation::RootRigidEnclosure,
        scale: voxy_animation::RootUniformScaleEnclosure,
    ) -> Result<Self, String> {
        let pose = physics
            .accepted_pose(scene, owner)
            .map_err(|e| e.to_string())?
            .ok_or("root reference needs a current accepted body pose")?;
        let mut candidate = self.stage_owner_selection(scene, models, owner, expected_model)?;
        candidate.initialize_root_reference_at_scaled_phase(
            owner,
            expected_model,
            voxy_animation::RootRigidTransform {
                translation: pose.physical_center,
                rotation: pose.physical_rotation,
            },
            current_root_to_body,
            scale,
        )?;
        Ok(candidate)
    }

    pub(super) fn capture_owner_reference_from_parent(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &voxy_gameplay::CharacterPhysics,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
    ) -> Result<Self, String> {
        let candidate = self.stage_owner_selection(scene, models, owner, expected_model)?;
        let state = &candidate.owners[&owner];
        if state.retarget_binding.is_some() {
            return Err("root parent reference retarget frame is not qualified".into());
        }
        let root = state
            .settings
            .resolve_motion_joint(&state.animation_model)?;
        let (frame, scale) = constant_parent_similarity_enclosure(expected_model, root)?;
        candidate.capture_owner_reference_with_similarity(
            scene,
            models,
            physics,
            owner,
            expected_model,
            frame,
            scale,
        )
    }

    /// Stages from the owner-held authored frame; never substitutes a clip's
    /// current phase for the reference retained after physical acceptance.
    pub(super) fn prepare_owner_fade(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
        dt: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Option<PreparedOwnerFade>, String> {
        let state = self.owners.get(&owner).ok_or("fade owner disappeared")?;
        if !Arc::ptr_eq(expected_model, &state.model) {
            return Err("fade target model changed".into());
        }
        if state.retarget_binding.is_some() {
            return Err("certified owner fade retarget frame is not qualified".into());
        }
        if !state.settings.root_motion_rotation {
            return Err("certified owner fade requires rigid root extraction".into());
        }
        let reference = state
            .root_reference
            .ok_or("fade root reference is not initialized")?;
        state
            .playback
            .prepare_certified_fade_wall(dt, state.settings.root_motion_axes, max_spans)?
            .map(|prepared| {
                let motion = prepared.bind_original_sources_common_similarity(
                    owner,
                    reference.authored_to_body,
                    reference.scale,
                    origin_tolerance,
                    angular_tolerance,
                    max_spans,
                )?;
                let asset = scene
                    .component::<ModelInstance>(owner)
                    .map_err(|e| e.to_string())?
                    .ok_or("fade model instance disappeared")?
                    .asset
                    .clone();
                let prepared = PreparedOwnerFade {
                    motion,
                    scene: self.scene.ok_or("fade scene disappeared")?,
                    owner,
                    asset,
                    settings: state.settings.clone(),
                    reference,
                    model: state.model.clone(),
                    animation_model: state.animation_model.clone(),
                    retarget_profile: state.retarget_profile.clone(),
                    feet_settings: state.feet.as_ref().map(|feet| feet.settings().clone()),
                };
                prepared.validate_scene(scene, models)?;
                Ok(prepared)
            })
            .transpose()
    }

    /// Admits a selection change without advancing its clocks or replacing the
    /// accepted displayed frame. Resource rebinding remains in ordinary prepare.
    pub(super) fn stage_owner_selection(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        owner: NodeId,
        expected_model: &Arc<ModelAsset>,
    ) -> Result<Self, String> {
        if self.scene != Some(scene.identity()) {
            return Err("animation selection scene changed".into());
        }
        let state = self
            .owners
            .get(&owner)
            .ok_or("animation selection owner disappeared")?;
        let instance = scene
            .component::<ModelInstance>(owner)
            .map_err(|e| e.to_string())?
            .ok_or("animation selection model disappeared")?;
        if !Arc::ptr_eq(expected_model, &state.model)
            || !models
                .get(&instance.asset)
                .is_some_and(|model| Arc::ptr_eq(model, &state.model))
        {
            return Err("animation selection model changed".into());
        }
        let profile = scene
            .component::<crate::ModelRetarget>(owner)
            .map_err(|e| e.to_string())?
            .cloned();
        let animation_model = match &profile {
            Some(profile) => models
                .get(&AssetId(profile.source.clone()))
                .ok_or("retarget source is not loaded")?,
            None => expected_model,
        };
        if profile != state.retarget_profile
            || !Arc::ptr_eq(animation_model, &state.animation_model)
        {
            return Err("animation selection binding changed".into());
        }
        if scene
            .component::<ModelPart>(owner)
            .map_err(|e| e.to_string())?
            .is_some_and(|part| part.node != u32::MAX)
        {
            return Err("animation selection owner became a hierarchy part".into());
        }
        let mut settings = scene
            .component::<ModelAnimation>(owner)
            .map_err(|e| e.to_string())?
            .cloned()
            .unwrap_or(ModelAnimation {
                clip: (!animation_model.animations.is_empty()).then_some(0),
                ..Default::default()
            });
        settings.validate(
            Some(animation_model.animations.len()),
            Some(animation_model.skeleton.joints().len()),
        )?;
        settings.clip = settings.resolve_clip(animation_model)?;
        let feet = scene
            .component::<crate::ModelFootPlacement>(owner)
            .map_err(|e| e.to_string())?
            .filter(|feet| !feet.feet.is_empty());
        if feet != state.feet.as_ref().map(|feet| feet.settings()) {
            return Err("animation selection foot binding changed".into());
        }
        if let Some(feet) = feet {
            let name = settings
                .clip
                .and_then(|index| animation_model.animations.get(index))
                .map(|clip| clip.name());
            for foot in &feet.feet {
                foot.contact_keys(name)?;
            }
        }
        let (playback, _, _, _) =
            prepare_playback_selection(Some(state), animation_model, &settings, true)?;
        let mut candidate = self.clone();
        let next = candidate.owners.get_mut(&owner).unwrap();
        if next.settings.root_motion_joint != settings.root_motion_joint
            || next.settings.root_motion_bone != settings.root_motion_bone
            || next.settings.root_motion_axes != settings.root_motion_axes
            || next.settings.root_motion_rotation != settings.root_motion_rotation
        {
            next.root_reference = None;
        }
        next.settings = settings;
        next.playback = playback;
        candidate.motions.clear();
        candidate.rotations.clear();
        Ok(candidate)
    }

    /// Stage scene selections and compile every initialized rigid fade before any
    /// clock advances. Quality limits are supplied by the caller; this does not
    /// invent a production sampling-error allowance.
    pub(super) fn prepare_scene_fades(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        dt: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<(Self, Vec<PreparedOwnerFade>), String> {
        let mut candidate = self.clone();
        let mut prepared = Vec::new();
        for (owner, instance) in scene.active_components::<ModelInstance>() {
            let Some(state) = candidate.owners.get(&owner) else {
                continue;
            };
            if state.root_reference.is_none() {
                continue;
            }
            let Some(model) = models.get(&instance.asset) else {
                continue;
            };
            // Asset/binding changes belong to ordinary preparation, which resets
            // the retained reference before another certified plan is admitted.
            let profile = scene
                .component::<crate::ModelRetarget>(owner)
                .map_err(|error| error.to_string())?;
            let feet = scene
                .component::<crate::ModelFootPlacement>(owner)
                .map_err(|error| error.to_string())?
                .filter(|settings| !settings.feet.is_empty());
            if !state.settings.root_motion_rotation
                || !Arc::ptr_eq(model, &state.model)
                || profile != state.retarget_profile.as_ref()
                || state.retarget_binding.is_some()
                || feet != state.feet.as_ref().map(|feet| feet.settings())
            {
                continue;
            }
            candidate = candidate.stage_owner_selection(scene, models, owner, model)?;
            let state = &candidate.owners[&owner];
            if state.root_reference.is_some() && state.playback.has_transition() {
                if let Some(fade) = candidate.prepare_owner_fade(
                    scene,
                    models,
                    owner,
                    model,
                    dt,
                    origin_tolerance,
                    angular_tolerance,
                    max_spans,
                )? {
                    prepared.push(fade);
                }
            }
        }
        Ok((candidate, prepared))
    }

    /// Production fixed tick. Stage every selection first, then publish clocks,
    /// physical bodies and displayed poses through one mixed transaction.
    pub(super) fn fixed_step(
        &self,
        scene: &mut SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &mut voxy_gameplay::CharacterPhysics,
        input: &mut voxy_input::InputMap,
        dt: f64,
    ) -> Result<Self, voxy_gameplay::CharacterTickError<String>> {
        use voxy_gameplay::CharacterTickError;
        // Approximation quality targets in the common body/world frame. These
        // bound field approximation; numerical evaluation is derived by physics.
        let (candidate, plans) = self
            .prepare_scene_fades(
                scene,
                models,
                dt,
                1e-4,
                1e-4,
                voxy_gameplay::MAX_CHARACTER_TRAJECTORY_SPANS,
            )
            .map_err(CharacterTickError::Preparation)?;
        let requests = plans
            .iter()
            .map(|plan| {
                let orientation = match physics
                    .accepted_pose(scene, plan.owner)
                    .map_err(|error| error.to_string())?
                {
                    Some(pose) => pose.physical_rotation,
                    None => scene
                        .local(plan.owner)
                        .map_err(|error| error.to_string())?
                        .rotation
                        .as_dquat()
                        .normalize(),
                };
                let preferred = voxy_gameplay::rigid_source_coordinate_axis(
                    orientation,
                    glam::DQuat::IDENTITY,
                    1,
                );
                Ok((
                    plan,
                    OwnerFadeAdmission {
                        coordinate_axis: plan.motion.automatic_coordinate_axis(preferred)?,
                        evaluation_radius: 0.,
                        evaluation_axes: None,
                    },
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(CharacterTickError::Preparation)?;
        let (_, accepted) =
            candidate.fixed_step_owner_fades(scene, models, physics, input, dt, &requests)?;
        Ok(accepted)
    }

    /// Complete scene-driven selection, plan admission and physical publication
    /// through the same mixed-tick transaction used by explicitly prepared fades.
    pub(super) fn fixed_step_scene_fades(
        &self,
        scene: &mut SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &mut voxy_gameplay::CharacterPhysics,
        input: &mut voxy_input::InputMap,
        dt: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
        frames: &BTreeMap<NodeId, OwnerFadeAdmission>,
    ) -> Result<
        (Vec<voxy_gameplay::AppliedCharacterTrajectoryMotion>, Self),
        voxy_gameplay::CharacterTickError<String>,
    > {
        let (candidate, plans) = self
            .prepare_scene_fades(
                scene,
                models,
                dt,
                origin_tolerance,
                angular_tolerance,
                max_spans,
            )
            .map_err(voxy_gameplay::CharacterTickError::Preparation)?;
        let requests = plans
            .iter()
            .map(|plan| {
                frames
                    .get(&plan.owner)
                    .copied()
                    .map(|frame| (plan, frame))
                    .ok_or_else(|| "fade evaluation frame is missing".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(voxy_gameplay::CharacterTickError::Preparation)?;
        if requests.len() != frames.len() {
            return Err(voxy_gameplay::CharacterTickError::Preparation(
                "fade evaluation frame has no active plan".into(),
            ));
        }
        candidate.fixed_step_owner_fades(scene, models, physics, input, dt, &requests)
    }

    /// Revalidates scene admission and performs physics plus accepted-pose
    /// preparation without exposing an intervening scene mutation opportunity.
    pub(super) fn fixed_step_owner_fades(
        &self,
        scene: &mut SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        physics: &mut voxy_gameplay::CharacterPhysics,
        input: &mut voxy_input::InputMap,
        dt: f64,
        fades: &[(&PreparedOwnerFade, OwnerFadeAdmission)],
    ) -> Result<
        (Vec<voxy_gameplay::AppliedCharacterTrajectoryMotion>, Self),
        voxy_gameplay::CharacterTickError<String>,
    > {
        use voxy_gameplay::CharacterTickError;
        let admitted = fades
            .iter()
            .map(|(prepared, _)| {
                let owner = self
                    .owners
                    .get(&prepared.owner)
                    .ok_or("fade owner disappeared")?;
                if !prepared.matches(owner) {
                    return Err("staged fade owner settings or root reference changed".into());
                }
                prepared.admit_scene(scene, models)
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(CharacterTickError::Preparation)?;
        let requests = admitted
            .iter()
            .zip(fades)
            .map(|(admitted, (_, frame))| {
                let mut request = admitted.request(frame.coordinate_axis, frame.evaluation_radius);
                request.evaluation_axes = frame.evaluation_axes;
                request
            })
            .collect::<Vec<_>>();
        let deferred = fades
            .iter()
            .map(|(prepared, _)| prepared.owner)
            .collect::<HashSet<_>>();
        let ordinary = self
            .prepare_deferred(scene, models, dt, &deferred)
            .map_err(CharacterTickError::Preparation)?;
        let ordinary_paths = ordinary.trajectories();
        physics.fixed_step_with_mixed_certified_fade_preparation(
            scene,
            input,
            dt,
            ordinary.motions(),
            &ordinary_paths,
            &requests,
            |preview, budget| {
                let mut candidate = ordinary.clone();
                for admitted in &admitted {
                    let owner = admitted.prepared.owner;
                    let receipt = preview
                        .motions
                        .iter()
                        .find(|receipt| receipt.owner == owner)
                        .ok_or("fade receipt disappeared")?;
                    let pose = preview
                        .characters
                        .iter()
                        .find(|pose| pose.owner == owner)
                        .ok_or("fade accepted pose disappeared")?;
                    candidate =
                        candidate.accept_fade(&admitted.prepared.model, admitted, receipt, pose)?;
                }
                candidate.prepare_accepted_pose(preview, budget)
            },
        )
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    /// Registers markers on the owner's animation source (including retargeting).
    pub(super) fn set_clip_events(
        &mut self,
        owner: NodeId,
        clip: usize,
        events: Vec<voxy_animation::ClipEvent>,
    ) -> Result<(), String> {
        self.owners
            .get_mut(&owner)
            .ok_or("animation event owner is not loaded")?
            .playback
            .set_clip_events(clip, events)
    }
    /// Drain only the runtime adopted after the enclosing scene transaction.
    /// Owner ordering is stable; occurrence ordering within an owner is retained.
    pub(super) fn take_events(&mut self) -> Vec<(NodeId, voxy_animation::ClipEventOccurrence)> {
        let mut owners: Vec<_> = self.owners.keys().copied().collect();
        owners.sort();
        let mut events = Vec::new();
        for owner in owners {
            for event in self
                .owners
                .get_mut(&owner)
                .expect("existing owner")
                .playback
                .take_events()
            {
                events.push((owner, event));
            }
        }
        events
    }
    pub(super) fn motions(&self) -> &[(NodeId, glam::Vec3)] {
        &self.motions
    }
    pub(super) fn trajectories(&self) -> Vec<voxy_gameplay::CharacterRigidTrajectoryMotion<'_>> {
        self.rotations
            .iter()
            .map(|(owner, trajectory, basis, origin, scale)| {
                voxy_gameplay::CharacterRigidTrajectoryMotion {
                    owner: *owner,
                    trajectory,
                    basis: *basis,
                    origin: *origin,
                    scale: *scale,
                }
            })
            .collect()
    }
    pub(super) fn has_foot_placement(&self) -> bool {
        self.owners.values().any(|owner| owner.feet.is_some())
    }
    pub(super) fn requires_pose_preparation(&self) -> bool {
        self.has_foot_placement()
            || self.owners.values().any(|owner| {
                owner.root_reference.is_some()
                    || (owner.settings.root_motion_rotation && owner.settings.clip.is_some())
            })
    }
    pub(super) fn prepare_accepted_pose(
        mut self,
        preview: &voxy_gameplay::CharacterTickPreview,
        budget: &mut voxy_gameplay::SupportQueryBudget,
    ) -> Result<Self, String> {
        for accepted in &preview.characters {
            let Some(owner) = self.owners.get_mut(&accepted.owner) else {
                continue;
            };
            if let Some(reference) = &mut owner.root_reference {
                let body = voxy_animation::RootRigidEnclosure::from_transform(
                    voxy_animation::RootRigidTransform {
                        translation: accepted.physical_center,
                        rotation: accepted.physical_rotation,
                    },
                )
                .map_err(|error| error.to_string())?;
                reference.authored_to_body = reference
                    .authored_to_body
                    .transported_body_reference(&reference.body_to_world, &body)
                    .map_err(|error| error.to_string())?;
                reference.body_to_world = body;
            } else if owner.settings.root_motion_rotation && owner.settings.clip.is_some() {
                let root = owner
                    .settings
                    .resolve_motion_joint(&owner.animation_model)?;
                let (axes, frame, scale) = if let Some(binding) = &owner.retarget_binding {
                    let similarity = binding
                        .root_similarity_enclosure(root, owner.settings.root_motion_axes)
                        .map_err(|e| e.to_string())?;
                    let profile = owner
                        .retarget_profile
                        .as_ref()
                        .ok_or("retarget root reference profile disappeared")?;
                    validate_retarget_motion_parents(
                        &owner.model,
                        &owner.animation_model,
                        profile,
                        similarity.target_joint,
                    )?;
                    let (parent, parent_scale) = static_parent_similarity_enclosure(
                        &owner.model,
                        similarity.target_joint,
                        |_, joint| Ok(joint.bind_local),
                    )?;
                    let similarity = similarity
                        .in_parent(parent, parent_scale)
                        .map_err(|e| e.to_string())?;
                    (similarity.source_axes, similarity.frame, similarity.scale)
                } else {
                    let (frame, scale) = constant_parent_similarity_enclosure(&owner.model, root)?;
                    (owner.settings.root_motion_axes, frame, scale)
                };
                let authored_to_body = owner
                    .playback
                    .reference_at_current_phase(axes, frame, scale)?;
                let body_to_world = voxy_animation::RootRigidEnclosure::from_transform(
                    voxy_animation::RootRigidTransform {
                        translation: accepted.physical_center,
                        rotation: accepted.physical_rotation,
                    },
                )
                .map_err(|error| error.to_string())?;
                owner.root_reference = Some(RootReference {
                    scale,
                    body_to_world,
                    authored_to_body,
                });
            }
            let Some(feet) = &mut owner.feet else {
                continue;
            };
            owner.frame = Arc::new(
                feet.correct_at_phase(
                    &owner.model,
                    (*owner.frame).clone(),
                    accepted.world_matrix.as_dmat4(),
                    accepted.grounded,
                    preview,
                    budget,
                    owner
                        .playback
                        .pose_blend_phases()
                        .map(|phase| phase.target.clip.name()),
                    owner.playback.contact_phase(),
                    owner.playback.contact_interval(),
                    owner.playback.pose_blend_phases(),
                    owner.playback.source_contact_interval(),
                    owner.playback.frozen_source_tick(),
                )?,
            );
        }
        Ok(self)
    }
    /// Rebuilds the displayed frame from the physically accepted playback,
    /// retaining the same extraction/retarget owner used by ordinary ticks.
    pub(super) fn accept_fade(
        mut self,
        expected_model: &Arc<ModelAsset>,
        admitted: &AdmittedOwnerFade<'_>,
        receipt: &voxy_gameplay::AppliedCharacterTrajectoryMotion,
        accepted_pose: &voxy_gameplay::AcceptedCharacterPose,
    ) -> Result<Self, String> {
        let staged = admitted.prepared;
        let owner = self
            .owners
            .get_mut(&receipt.owner)
            .ok_or("fade owner disappeared")?;
        if accepted_pose.owner != receipt.owner || !Arc::ptr_eq(expected_model, &owner.model) {
            return Err("fade target model changed".into());
        }
        if !staged.matches(owner) {
            return Err("staged fade owner settings or root reference changed".into());
        }
        let (playback, frame) = staged.motion.accepted_playback(&owner.playback, receipt)?;
        let (frame, _) = prepare_displayed_frame(
            frame,
            &owner.model,
            &owner.settings,
            owner.retarget_binding.as_ref(),
        )?;
        owner.playback = playback;
        owner.frame = Arc::new(frame);
        // prepare_deferred owns the tick serial; each accepted owner only
        // replaces its staged playback/frame within that same transaction.
        Ok(self)
    }

    pub(super) fn clip_name(&self, owner: NodeId) -> Option<&str> {
        Some(
            self.owners
                .get(&owner)?
                .playback
                .pose_blend_phases()?
                .target
                .clip
                .name(),
        )
    }
    pub(super) fn clip_phase(&self, owner: NodeId) -> Option<f64> {
        self.owners.get(&owner)?.playback.contact_phase()
    }
    pub(super) fn serial(&self) -> u64 {
        self.serial
    }
    pub(super) fn frame(
        &self,
        owner: NodeId,
        model: &Arc<ModelAsset>,
    ) -> Option<Arc<AnimatorFrame>> {
        let state = self.owners.get(&owner)?;
        Arc::ptr_eq(&state.model, model).then(|| state.frame.clone())
    }
    pub(super) fn synchronize(&mut self, scene: &SceneGraph) -> Result<(), String> {
        if self.scene.is_some_and(|id| id != scene.identity()) {
            return Err("foreign animation scene".into());
        }
        self.owners.retain(|owner, _| {
            scene
                .component::<ModelInstance>(*owner)
                .ok()
                .flatten()
                .is_some()
                && scene
                    .component::<ModelPart>(*owner)
                    .ok()
                    .flatten()
                    .is_none_or(|part| part.node == u32::MAX)
        });
        Ok(())
    }
    /// Stage one fixed tick; publish the returned owner only after physics accepts.
    pub(super) fn prepare(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        dt: f32,
    ) -> Result<Self, String> {
        self.prepare_wall(scene, models, f64::from(dt))
    }

    pub(super) fn prepare_wall(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        dt: f64,
    ) -> Result<Self, String> {
        self.prepare_deferred(scene, models, dt, &HashSet::new())
    }

    fn prepare_deferred(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        dt: f64,
        deferred: &HashSet<NodeId>,
    ) -> Result<Self, String> {
        if !dt.is_finite() || dt <= 0.0 || dt > f64::from(0.1_f32) {
            return Err("invalid fixed animation timestep".into());
        }
        let mut next = self.clone();
        next.motions.clear();
        next.rotations.clear();
        let mut rotation_spans = 0_usize;
        next.synchronize(scene)?;
        next.scene = Some(scene.identity());
        // Imported hierarchy parts reference the shared model but own no clocks.
        // Inactive logical owners count too: their retained playback consumes storage.
        let owner_count = scene
            .components::<ModelInstance>()
            .filter(|(owner, instance)| {
                (models.contains_key(&instance.asset) || next.owners.contains_key(owner))
                    && scene
                        .component::<ModelPart>(*owner)
                        .ok()
                        .flatten()
                        .is_none_or(|part| part.node == u32::MAX)
            })
            .count();
        if owner_count > MAX_OWNERS {
            return Err("animation owner capacity exceeded".into());
        }
        for (owner, instance) in scene.active_components::<ModelInstance>() {
            if deferred.contains(&owner) {
                continue;
            }
            if scene
                .component::<ModelPart>(owner)
                .map_err(|e| e.to_string())?
                .is_some_and(|part| part.node != u32::MAX)
            {
                continue;
            }
            let Some(model) = models.get(&instance.asset) else {
                continue;
            };
            let profile = scene
                .component::<crate::ModelRetarget>(owner)
                .map_err(|e| e.to_string())?
                .cloned();
            let animation_model = if let Some(profile) = &profile {
                models
                    .get(&AssetId(profile.source.clone()))
                    .ok_or("retarget source is not loaded")?
            } else {
                model
            };
            let current = next.owners.get(&owner);
            let same_binding = current.is_some_and(|old| {
                old.retarget_profile == profile
                    && old.model.skeleton.joints() == model.skeleton.joints()
            });
            let binding = if let Some(profile) = &profile {
                if let Some(old) = current.filter(|old| {
                    same_binding && Arc::ptr_eq(&old.animation_model, animation_model)
                }) {
                    old.retarget_binding.clone()
                } else {
                    Some(profile.compile_models(animation_model, model)?)
                }
            } else {
                None
            };
            let mut settings = scene
                .component::<ModelAnimation>(owner)
                .map_err(|e| e.to_string())?
                .cloned()
                .unwrap_or(ModelAnimation {
                    clip: (!animation_model.animations.is_empty()).then_some(0),
                    ..Default::default()
                });
            settings.validate(
                Some(animation_model.animations.len()),
                Some(animation_model.skeleton.joints().len()),
            )?;
            settings.clip = settings.resolve_clip(animation_model)?;

            let (mut playback, same_clip, compatible_model, reload_transition) =
                prepare_playback_selection(current, animation_model, &settings, same_binding)?;
            let source_axes = if settings.root_motion_rotation {
                if let Some(binding) = &binding {
                    binding
                        .source_root_motion_axes(
                            settings.resolve_motion_joint(animation_model)?,
                            settings.root_motion_axes,
                        )
                        .map_err(|e| e.to_string())?
                } else {
                    settings.root_motion_axes
                }
            } else {
                settings.root_motion_axes
            };
            let (mut frame, mut trajectory) = playback.advance_with_motion_wall(
                dt,
                settings.root_motion_rotation,
                source_axes,
                |_, frame, path| Ok((frame.clone(), path.cloned())),
            )?;
            if let Some(binding) = &binding {
                if let Some(path) = trajectory.as_ref() {
                    trajectory = Some(
                        binding
                            .apply_root_path(
                                path,
                                frame.root_motion_joint,
                                settings.root_motion_axes,
                            )
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            let (displayed, extracted_displacement) =
                prepare_displayed_frame(frame, model, &settings, binding.as_ref())?;
            frame = displayed;
            if settings.root_motion_rotation
                || settings.root_motion_axes.into_iter().any(|axis| axis)
            {
                if scene
                    .component::<voxy_gameplay::CharacterBody>(owner)
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    return Err("root motion requires a CharacterBody on the model owner".into());
                }
                let (basis, rotation_basis, scale) = if let Some(profile) = &profile {
                    retarget_parent_basis(
                        model,
                        animation_model,
                        profile,
                        frame.root_motion_joint,
                        settings.root_motion_rotation,
                    )?
                } else {
                    constant_parent_basis(model, &settings, frame.root_motion_joint)?
                };
                let displacement = extracted_displacement;
                let world = scene
                    .world_matrix(owner)
                    .map_err(|error| error.to_string())?;
                let displacement = world.transform_vector3(basis.transform_vector3(displacement));
                if !basis.is_finite() || !displacement.is_finite() {
                    return Err("root motion coordinate conversion overflow".into());
                }
                if !settings.root_motion_rotation {
                    next.motions.push((owner, displacement));
                }
                if settings.root_motion_rotation {
                    let origin = basis.w_axis.truncate();
                    if !origin.is_finite() {
                        return Err("root trajectory origin conversion overflow".into());
                    }
                    if let Some(trajectory) = trajectory {
                        rotation_spans = rotation_spans
                            .checked_add(trajectory.spans().len())
                            .ok_or("root rotation span overflow")?;
                        if rotation_spans > voxy_gameplay::MAX_CHARACTER_TRAJECTORY_SPANS {
                            return Err(
                                "aggregate root rotation trajectory capacity exceeded".into()
                            );
                        }
                        next.rotations
                            .push((owner, trajectory, rotation_basis, origin, scale));
                    }
                }
            }
            let feet = if let Some(foot_settings) = scene
                .component::<crate::ModelFootPlacement>(owner)
                .map_err(|e| e.to_string())?
                .filter(|settings| !settings.feet.is_empty())
            {
                if scene
                    .component::<voxy_gameplay::CharacterBody>(owner)
                    .map_err(|e| e.to_string())?
                    .is_none()
                {
                    return Err("foot placement requires a CharacterBody on the model owner".into());
                }
                if let Some(feet) = current
                    .filter(|_| {
                        (compatible_model || reload_transition)
                            && (same_clip
                                || playback.has_transition()
                                || playback.source_contact_interval().is_some()
                                || playback.frozen_source_tick().is_some())
                    })
                    .and_then(|old| old.feet.as_ref())
                    .filter(|feet| feet.matches(foot_settings))
                {
                    Some(feet.clone())
                } else {
                    Some(crate::foot_placement::FootRuntime::new_with_clips(
                        model,
                        foot_settings.clone(),
                        &animation_model.animations,
                    )?)
                }
            } else {
                None
            };
            let frame = Arc::new(frame);
            next.owners.insert(
                owner,
                Owner {
                    root_reference: current
                        .filter(|old| {
                            Arc::ptr_eq(&old.model, model)
                                && Arc::ptr_eq(&old.animation_model, animation_model)
                                && old.retarget_profile == profile
                                && old.settings.root_motion_joint == settings.root_motion_joint
                                && old.settings.root_motion_bone == settings.root_motion_bone
                                && old.settings.root_motion_axes == settings.root_motion_axes
                                && old.settings.root_motion_rotation
                                    == settings.root_motion_rotation
                        })
                        .and_then(|old| old.root_reference),
                    model: model.clone(),
                    animation_model: animation_model.clone(),
                    retarget_profile: profile,
                    retarget_binding: binding,
                    settings,
                    playback,
                    frame,
                    feet,
                },
            );
        }
        next.serial = next
            .serial
            .checked_add(1)
            .ok_or("animation tick overflow")?;
        Ok(next)
    }
}

fn prepare_playback_selection(
    current: Option<&Owner>,
    animation_model: &Arc<ModelAsset>,
    settings: &ModelAnimation,
    same_binding: bool,
) -> Result<(ModelPlayback, bool, bool, bool), String> {
    let named_identity = current.is_some_and(|old| {
        !settings.clip_name.is_empty() && old.settings.clip_name == settings.clip_name
    });
    let same_clip = current.is_some_and(|old| old.settings.clip == settings.clip || named_identity);
    let compatible_model = current.is_some_and(|old| {
        same_binding
            && (Arc::ptr_eq(&old.animation_model, animation_model)
                || old.playback.can_rebind(animation_model, named_identity))
    });
    let reload_transition = same_binding
        && !compatible_model
        && settings.transition_seconds > 0.
        && current.is_some_and(|old| {
            same_clip
                && old.animation_model.skeleton.joints() == animation_model.skeleton.joints()
                && old
                    .settings
                    .clip
                    .zip(settings.clip)
                    .is_some_and(|(old_index, index)| {
                        old.animation_model
                            .animations
                            .get(old_index)
                            .zip(animation_model.animations.get(index))
                            .is_some_and(|(a, b)| a.name() == b.name())
                    })
        });
    let mut playback = if current.is_none_or(|old| {
        !(compatible_model || reload_transition)
            || (!same_clip
                && (settings.transition_seconds == 0.
                    || old.settings.clip.is_none()
                    || settings.clip.is_none()))
    }) {
        ModelPlayback::new(animation_model.clone(), settings.clone())?
    } else {
        let mut playback = current.unwrap().playback.clone();
        playback.rebind(animation_model.clone())?;
        if reload_transition {
            playback.reload_clip(
                settings.clip.ok_or("missing reload target")?,
                settings.transition_seconds,
            )?;
        }
        if !same_clip {
            playback.transition_to_clip(
                settings.clip.ok_or("missing transition target")?,
                settings.transition_seconds,
            )?;
        }
        playback.set_speed(settings.speed)?;
        playback.set_root_motion_joint(settings.resolve_motion_joint(animation_model)?)?;
        playback
    };
    if current.is_some_and(|old| {
        (!settings.events.is_empty() || !old.settings.events.is_empty())
            && (settings.events != old.settings.events
                || settings.clip != old.settings.clip
                || !Arc::ptr_eq(&old.animation_model, animation_model))
    }) {
        playback.bind_authored_events(settings)?;
    }
    Ok((playback, same_clip, compatible_model, reload_transition))
}

pub(super) fn prepare_displayed_frame(
    mut frame: AnimatorFrame,
    model: &ModelAsset,
    settings: &ModelAnimation,
    binding: Option<&voxy_animation::RetargetBinding>,
) -> Result<(AnimatorFrame, glam::Vec3), String> {
    let extracted =
        settings.root_motion_rotation || settings.root_motion_axes.into_iter().any(|axis| axis);
    if let Some(binding) = binding {
        frame = if extracted {
            binding.apply_frame(&frame)
        } else {
            binding.apply_pose_frame(&frame)
        }
        .map_err(|error| error.to_string())?;
    }
    let mut displacement = glam::Vec3::ZERO;
    if extracted {
        (frame, displacement) = frame
            .into_in_place_translation(&model.skeleton, settings.root_motion_axes)
            .map_err(|error| error.to_string())?;
        if settings.root_motion_rotation {
            frame = frame
                .without_root_rotation(&model.skeleton)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok((frame, displacement))
}

// A mapped ancestor must retain its bind transform across source clips and fades.
// Unmapped target ancestors are bind pose by the retarget contract.
fn retarget_parent_basis(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
    rotation: bool,
) -> Result<(glam::Mat4, glam::DQuat, f64), String> {
    validate_retarget_motion_parents(target, source, profile, root)?;
    constant_parent_basis(
        target,
        &ModelAnimation {
            clip: None,
            root_motion_rotation: rotation,
            ..Default::default()
        },
        root,
    )
}

fn validate_retarget_motion_parents(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
) -> Result<(), String> {
    let mut parent = target.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        for mapping in &profile.joints {
            if target
                .resolve_joint_name(&mapping.target)
                .map_err(|e| e.to_string())?
                == index
            {
                let source_index = usize::from(
                    source
                        .resolve_joint_name(&mapping.source)
                        .map_err(|e| e.to_string())?,
                );
                if source
                    .animations
                    .iter()
                    .any(|clip| !clip.joint_uses_bind_pose(source_index))
                {
                    return Err(
                        "retarget motion parent is animated; interval basis is unproved".into(),
                    );
                }
            }
        }
        parent = target.skeleton.joints()[usize::from(index)].parent;
    }
    Ok(())
}

/// Compiles a static ancestor similarity from stored TRS values, enclosing
/// every rotation, translation and scale product before matrix rounding.
fn constant_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    String,
> {
    static_parent_similarity_enclosure(model, root, |index, joint| {
        let transform = model
            .animations
            .first()
            .map_or(Some(joint.bind_local), |clip| {
                clip.constant_joint_transform(index)
            })
            .ok_or("root parent reference is animated")?;
        if model
            .animations
            .iter()
            .any(|clip| clip.constant_joint_transform(index) != Some(transform))
        {
            return Err("root parent reference differs between clips".into());
        }
        Ok(transform)
    })
}

fn static_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
    mut transform_at: impl FnMut(
        usize,
        &voxy_animation::Joint,
    ) -> Result<voxy_animation::Transform, String>,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    String,
> {
    use voxy_animation::{RootRigidEnclosure, RootRigidTransform, RootUniformScaleEnclosure};
    let mut frame = RootRigidEnclosure::IDENTITY;
    let mut scale = RootUniformScaleEnclosure::ONE;
    let mut parent = model
        .skeleton
        .joints()
        .get(usize::from(root))
        .ok_or("invalid root parent reference joint")?
        .parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = transform_at(index, joint)?;
        let magnitude = transform.scale.abs();
        if magnitude.x == 0. || magnitude.x != magnitude.y || magnitude.x != magnitude.z {
            return Err("root parent reference requires nonzero uniform scale".into());
        }
        let signs = transform.scale.signum();
        let determinant = signs.x * signs.y * signs.z;
        let proper = signs * determinant;
        let reflection = if proper == glam::Vec3::ONE {
            glam::DQuat::IDENTITY
        } else if proper.x == 1. {
            glam::DQuat::from_xyzw(1., 0., 0., 0.)
        } else if proper.y == 1. {
            glam::DQuat::from_xyzw(0., 1., 0., 0.)
        } else {
            glam::DQuat::from_xyzw(0., 0., 1., 0.)
        };
        let authored = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: transform.translation.as_dvec3(),
            rotation: glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)),
        })
        .map_err(|e| e.to_string())?;
        let reflection = RootRigidEnclosure::from_transform(RootRigidTransform {
            rotation: reflection,
            ..RootRigidTransform::IDENTITY
        })
        .map_err(|e| e.to_string())?;
        let outer = authored.compose(&reflection).map_err(|e| e.to_string())?;
        let outer_scale =
            RootUniformScaleEnclosure::from_scale(f64::from(magnitude.x) * f64::from(determinant))
                .map_err(|e| e.to_string())?;
        frame = outer
            .compose(
                &frame
                    .with_translation_scale_enclosed(outer_scale)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        scale = outer_scale.multiplied(scale).map_err(|e| e.to_string())?;
        parent = joint.parent;
    }
    Ok((frame, scale))
}

// Only immutable clip admission proofs establish an ancestor for the whole interval.
// For rotation, a uniform signed scale is orthogonal up to magnitude; its axis
// conversion uses the proper pseudovector basis det(S)*S, including reflections.
fn constant_parent_basis(
    model: &ModelAsset,
    settings: &ModelAnimation,
    root: u16,
) -> Result<(glam::Mat4, glam::DQuat, f64), String> {
    let mut matrix = glam::Mat4::IDENTITY;
    let mut rotation = glam::DQuat::IDENTITY;
    let mut uniform_scale = 1_f64;
    let mut parent = model.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = if let Some(clip) = settings.clip {
            model.animations[clip]
                .constant_joint_transform(index)
                .ok_or("root motion parent is moving or unproved; choose a fixed locomotion root")?
        } else {
            joint.bind_local
        };
        if settings.root_motion_rotation {
            let scale = transform.scale.abs();
            if scale.x != scale.y || scale.x != scale.z {
                return Err("A parent of the selected motion bone has nonuniform scale; root rotation requires uniform scale".into());
            }
            let signs = transform.scale.signum().as_dvec3();
            let determinant = signs.x * signs.y * signs.z;
            uniform_scale *= f64::from(scale.x) * determinant;
            let reflection =
                glam::DQuat::from_mat3(&glam::DMat3::from_diagonal(signs * determinant))
                    .normalize();
            let authored =
                glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)).normalize();
            rotation = (authored * reflection * rotation).normalize();
        }
        matrix = transform.matrix() * matrix;
        parent = joint.parent;
    }
    if !matrix.is_finite()
        || !rotation.is_finite()
        || !uniform_scale.is_finite()
        || uniform_scale == 0.
    {
        return Err("root motion parent conversion overflow".into());
    }
    Ok((matrix, rotation, uniform_scale))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rotation_tests;
