//! Executable game setup belongs to the host, separate from serialized resources.
use std::path::Path;
use voxy_scene::{ComponentRegistry, SceneGraph, SceneSimulation};

type Setup = dyn FnMut(&mut SceneGraph, &mut SceneSimulation) -> Result<(), Box<dyn std::error::Error>>
    + Send;
/// Attaches fresh behavior instances to the detached runtime graph on every Play.
/// Uses the existing bounded simulation and lifecycle; does not serialize code.
pub struct BehaviorSetup(Box<Setup>);
impl std::fmt::Debug for BehaviorSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorSetup").finish_non_exhaustive()
    }
}
impl BehaviorSetup {
    pub fn new(
        setup: impl FnMut(
            &mut SceneGraph,
            &mut SceneSimulation,
        ) -> Result<(), Box<dyn std::error::Error>>
        + Send
        + 'static,
    ) -> Self {
        Self(Box::new(setup))
    }
    pub(crate) fn configure(
        &mut self,
        scene: &mut SceneGraph,
        simulation: &mut SceneSimulation,
    ) -> Result<(), Box<dyn std::error::Error>> {
        (self.0)(scene, simulation)
    }
}
/// One host contract for editor Play, standalone, package build and package launch.
/// Register durable component codecs and supply fresh runtime behavior/UI setup.
#[derive(Debug)]
pub struct GameHost {
    pub components: ComponentRegistry,
    pub behaviors: Option<BehaviorSetup>,
    pub ui_actions: Option<voxy_gameplay::UiActionSetup>,
}
impl GameHost {
    #[must_use]
    pub fn new(components: ComponentRegistry) -> Self {
        Self {
            components,
            behaviors: None,
            ui_actions: None,
        }
    }
}
/// Run a project with application-owned codecs, behaviors and UI callbacks.
/// # Errors
/// Reports unsupported modes, invalid project data, setup and runtime failures.
pub fn run_project_with_host(
    path: &Path,
    mode: crate::ViewportMode,
    host: GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::project_runtime::run_project_host(path, mode, host)
}
