//! Editor adapter and model-asset convenience helpers around [`voxy_gameplay::foot_placement`].
pub(crate) mod checkpoint {
    use super::*;
    use std::sync::Arc;
    use voxy_model::ModelAsset;
    use voxy_scene::{NodeId, ObjectId, SceneGraph};

    pub use voxy_gameplay::foot_placement::checkpoint::FootRuntimeCheckpoint;

    #[cfg(test)]
    pub(crate) trait FootRuntimeCheckpointExt {
        fn prepare(
            &self,
            rig: FootCheckpointRig<'_>,
            clips: &[Arc<voxy_animation::AnimationClip>],
            scene: &SceneGraph,
            resolve: impl Fn(&ObjectId) -> Option<NodeId>,
            resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
            max_bytes: usize,
        ) -> Result<FootRuntime, String>;
        #[expect(
            clippy::too_many_arguments,
            reason = "checkpoint restoration forwards target rig, optional retarget source skeleton, clips, scene IDs, and shared pose snapshots"
        )]
        fn prepare_with_source_skeleton(
            &self,
            rig: FootCheckpointRig<'_>,
            source_skeleton: Option<&voxy_animation::Skeleton>,
            clips: &[Arc<voxy_animation::AnimationClip>],
            scene: &SceneGraph,
            resolve: impl Fn(&ObjectId) -> Option<NodeId>,
            resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
            max_bytes: usize,
        ) -> Result<FootRuntime, String>;
    }

    #[cfg(test)]
    impl FootRuntimeCheckpointExt for FootRuntimeCheckpoint {
        fn prepare(
            &self,
            rig: FootCheckpointRig<'_>,
            clips: &[Arc<voxy_animation::AnimationClip>],
            scene: &SceneGraph,
            resolve: impl Fn(&ObjectId) -> Option<NodeId>,
            resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
            max_bytes: usize,
        ) -> Result<FootRuntime, String> {
            prepare_with_rig(
                self,
                rig,
                None,
                clips,
                scene,
                resolve,
                resolve_pose,
                max_bytes,
            )
        }
        fn prepare_with_source_skeleton(
            &self,
            rig: FootCheckpointRig<'_>,
            source_skeleton: Option<&voxy_animation::Skeleton>,
            clips: &[Arc<voxy_animation::AnimationClip>],
            scene: &SceneGraph,
            resolve: impl Fn(&ObjectId) -> Option<NodeId>,
            resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
            max_bytes: usize,
        ) -> Result<FootRuntime, String> {
            prepare_with_rig(
                self,
                rig,
                source_skeleton,
                clips,
                scene,
                resolve,
                resolve_pose,
                max_bytes,
            )
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Skeleton variant exercised by standalone foot runtime checkpoint tests"
        )
    )]
    pub(crate) enum FootCheckpointRig<'a> {
        Model(&'a ModelAsset),
        Skeleton(&'a voxy_animation::Skeleton),
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "checkpoint restoration forwards target rig, optional retarget source skeleton, clips, scene IDs, and shared pose snapshots"
    )]
    pub(crate) fn prepare_with_rig(
        saved: &FootRuntimeCheckpoint,
        rig: FootCheckpointRig<'_>,
        source_skeleton: Option<&voxy_animation::Skeleton>,
        clips: &[Arc<voxy_animation::AnimationClip>],
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        resolve_pose: impl Fn(&str) -> Option<Arc<voxy_animation::Pose>>,
        max_bytes: usize,
    ) -> Result<FootRuntime, String> {
        match rig {
            FootCheckpointRig::Model(model) => saved.prepare_with_joint_resolver(
                &model.skeleton,
                source_skeleton,
                clips,
                |name| model.resolve_joint_name(name).map_err(|e| e.to_string()),
                scene,
                resolve,
                resolve_pose,
                max_bytes,
            ),
            FootCheckpointRig::Skeleton(skeleton) => saved.prepare_with_joint_resolver(
                skeleton,
                source_skeleton,
                clips,
                |name| {
                    skeleton
                        .joints()
                        .iter()
                        .position(|joint| joint.name.as_ref() == name)
                        .map(|index| index as u16)
                        .ok_or_else(|| format!("missing foot joint: {name}"))
                },
                scene,
                resolve,
                resolve_pose,
                max_bytes,
            ),
        }
    }
}

#[cfg(test)]
use self::checkpoint::FootRuntimeCheckpointExt;
use glam::DMat4;
#[cfg(test)]
use glam::DVec3;
use std::sync::Arc;
use voxy_animation::AnimatorFrame;
#[cfg(test)]
use voxy_gameplay::CharacterTickPreview;
use voxy_gameplay::SupportQueryBudget;
#[cfg(test)]
use voxy_gameplay::foot_placement::{ContactBlendState, contact_weight, crossed_swing};
pub use voxy_gameplay::{
    FootBinding, FootContactKey, FootOrientation, FootPivotKey, FootRuntime, FootSupportGeometry,
    ModelFootPlacement,
};
#[cfg(test)]
use voxy_gameplay::{FootContactInput, FootContactSettings, FootContactState};
use voxy_model::ModelAsset;

pub(crate) fn new_with_clips(
    model: &ModelAsset,
    settings: ModelFootPlacement,
    clips: &[Arc<voxy_animation::AnimationClip>],
) -> Result<FootRuntime, String> {
    FootRuntime::new_resolved(&model.skeleton, settings, clips, |name| {
        model.resolve_joint_name(name).map_err(|e| e.to_string())
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "editor model-bound wrapper forwarding skeleton and phase intervals to FootRuntime"
)]
pub(crate) fn correct_at_phase(
    feet: &mut FootRuntime,
    model: &ModelAsset,
    frame: AnimatorFrame,
    actor: DMat4,
    grounded: bool,
    support: &voxy_gameplay::SupportWorld,
    budget: &mut SupportQueryBudget,
    clip_name: Option<&str>,
    phase: Option<f64>,
    interval: Option<voxy_animation::AnimationPhaseInterval>,
    blend: Option<voxy_animation::PoseBlendPhases<'_>>,
    source_interval: Option<&crate::model_playback::SourceContactInterval>,
    frozen_tick: Option<&(Arc<voxy_animation::Pose>, f64)>,
) -> Result<AnimatorFrame, String> {
    feet.correct_skeleton_on_support(
        &model.skeleton,
        frame,
        actor,
        grounded,
        support,
        budget,
        clip_name,
        phase,
        interval,
        blend,
        source_interval,
        frozen_tick,
    )
}

#[cfg(test)]
trait FootRuntimeModelTestExt: Sized {
    fn new(model: &ModelAsset, settings: ModelFootPlacement) -> Result<Self, String>;
    fn new_with_clips(
        model: &ModelAsset,
        settings: ModelFootPlacement,
        clips: &[Arc<voxy_animation::AnimationClip>],
    ) -> Result<Self, String>;
    fn request_geometry_pivot(
        &mut self,
        foot_index: usize,
        points: &voxy_animation::FootSupportPoints,
        model: &ModelAsset,
        frame: &AnimatorFrame,
        actor: DMat4,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
        max_projection: f64,
    ) -> Result<(), String>;
    fn correct(
        &mut self,
        model: &ModelAsset,
        frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
    ) -> Result<AnimatorFrame, String>;
    #[expect(
        clippy::too_many_arguments,
        reason = "test adapter matching editor model-bound phase correction signature"
    )]
    fn correct_at_phase(
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
    ) -> Result<AnimatorFrame, String>;
}

#[cfg(test)]
impl FootRuntimeModelTestExt for FootRuntime {
    fn new(model: &ModelAsset, settings: ModelFootPlacement) -> Result<FootRuntime, String> {
        super::foot_placement::new_with_clips(model, settings, &model.animations)
    }
    fn new_with_clips(
        model: &ModelAsset,
        settings: ModelFootPlacement,
        clips: &[Arc<voxy_animation::AnimationClip>],
    ) -> Result<FootRuntime, String> {
        super::foot_placement::new_with_clips(model, settings, clips)
    }
    fn request_geometry_pivot(
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
        self.request_geometry_pivot_skeleton(
            foot_index,
            points,
            &model.skeleton,
            frame,
            actor,
            preview,
            budget,
            max_projection,
        )
    }
    fn correct(
        &mut self,
        model: &ModelAsset,
        frame: AnimatorFrame,
        actor: DMat4,
        grounded: bool,
        preview: &CharacterTickPreview,
        budget: &mut SupportQueryBudget,
    ) -> Result<AnimatorFrame, String> {
        super::foot_placement::correct_at_phase(
            self,
            model,
            frame,
            actor,
            grounded,
            &preview.support,
            budget,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }
    fn correct_at_phase(
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
        super::foot_placement::correct_at_phase(
            self,
            model,
            frame,
            actor,
            grounded,
            &preview.support,
            budget,
            clip_name,
            phase,
            interval,
            blend,
            source_interval,
            frozen_tick,
        )
    }
}

#[cfg(test)]
mod tests;
