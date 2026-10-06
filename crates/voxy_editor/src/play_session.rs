//! Owns runtime play state independently of the authoring document.
use std::time::Instant;
use voxy_gameplay::CharacterPhysics;
use voxy_input::InputMap;
use voxy_scene::{SceneDocument, SceneGraph, SceneSimulation};
#[derive(Debug)]
pub(super) struct PlaySession {
    pub(super) playing: Option<SceneDocument>,
    pub(super) simulation_time: Instant,
    pub(super) simulation: Option<SceneSimulation>,
    pub(super) animations: crate::animation_runtime::AnimationRuntime,
    pub(super) angular_motion: Option<voxy_gameplay::AngularMotionBatch>,
    pub(super) simulation_ticks: u64,
    pub(super) liquid_optics: Vec<Option<[f32; 4]>>,
    pub(super) liquid: Option<voxy_gameplay::SceneLiquidRuntime>,
    pub(super) physics: Option<CharacterPhysics>,
    pub(super) player_input: InputMap,
    pub(super) ui_actions: Option<voxy_gameplay::UiActionHandlers>,
}

impl PlaySession {
    pub(super) fn stop_simulation(
        &mut self,
        scene: &mut SceneGraph,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut simulation) = self.simulation.take() {
            simulation.stop(scene)?;
        }
        Ok(())
    }
    pub(super) fn stop(
        &mut self,
        scene: &mut SceneGraph,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_simulation(scene)?;
        self.physics = None;
        self.liquid = None;
        self.liquid_optics.clear();
        self.animations.clear();
        self.angular_motion = None;
        self.ui_actions = None;
        self.player_input = voxy_gameplay::player_input()?;
        Ok(())
    }
}
