//! Explicit scene simulation ownership, lifecycle dispatch and command barriers.
use crate::Transform;
use crate::{Behavior, BehaviorRunner, NodeId, SceneCommands, SceneGraph, SceneGraphError};
use glam::Mat4;
use std::collections::HashMap;
use voxy_time::{SimulationClock, TimeDrop, TimeFrame};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationError {
    Stopped,
    InvalidConfiguration,
    InvalidDelta,
    Scene(SceneGraphError),
}
impl From<SceneGraphError> for SimulationError {
    fn from(error: SceneGraphError) -> Self {
        Self::Scene(error)
    }
}
impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene simulation error: {self:?}")
    }
}
impl std::error::Error for SimulationError {}
/// A fixed-system failure consumes the scheduled frame. Completed ticks and
/// earlier behavior mutations remain committed; callers must recover or Stop.
#[derive(Debug)]
pub enum SimulationStepError<E> {
    Simulation(SimulationError),
    System { completed_steps: usize, error: E },
}
impl<E: std::fmt::Display> std::fmt::Display for SimulationStepError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simulation(error) => error.fmt(f),
            Self::System {
                completed_steps,
                error,
            } => write!(
                f,
                "fixed system failed after {completed_steps} ticks: {error}"
            ),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SimulationStepError<E> {}
impl<E> From<SimulationError> for SimulationStepError<E> {
    fn from(error: SimulationError) -> Self {
        Self::Simulation(error)
    }
}
#[derive(Debug)]
pub struct SimulationFrame {
    pub time: TimeFrame,
    pub dropped: TimeDrop,
    /// One result for each admitted command, in submission order.
    pub commands: Vec<Result<(), SceneGraphError>>,
}
/// Explicit per-runtime admission and catch-up limits.
#[derive(Clone, Copy, Debug)]
pub struct SimulationLimits {
    pub fixed_step: f64,
    pub max_steps: usize,
    pub max_behaviors: usize,
    pub max_commands: usize,
}
/// Owns runtime behaviors and scheduling independently of authoring history.
/// Existing Behavior hooks may mutate scene data directly; external structural
/// mutations should use commands at barriers. Parallel dispatch is not implied.
#[derive(Debug)]
pub struct SceneSimulation {
    scene: u64,
    stopped: bool,
    behaviors: BehaviorRunner,
    commands: SceneCommands,
    clock: SimulationClock,
    step: f64,
    max_steps: usize,
    max_behaviors: usize,
    previous: HashMap<NodeId, Pose>,
    current: HashMap<NodeId, Pose>,
    alpha: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    local: Transform,
    parent: Option<NodeId>,
    active: bool,
}
fn poses(scene: &SceneGraph) -> HashMap<NodeId, Pose> {
    scene
        .nodes()
        .map(|(id, local, parent)| {
            (
                id,
                Pose {
                    local,
                    parent,
                    active: scene.active_in_hierarchy(id).unwrap_or(false),
                },
            )
        })
        .collect()
}
impl SceneSimulation {
    /// # Errors
    /// Rejects nonfinite/nonpositive steps and a zero catch-up budget.
    /// Zero behavior/command capacities disable admission.
    pub fn new(scene: &SceneGraph, limits: SimulationLimits) -> Result<Self, SimulationError> {
        if !limits.fixed_step.is_finite() || limits.fixed_step <= 0.0 || limits.max_steps == 0 {
            return Err(SimulationError::InvalidConfiguration);
        }
        Ok(Self {
            scene: scene.id,
            stopped: false,
            behaviors: BehaviorRunner::default(),
            commands: SceneCommands::new(scene, limits.max_commands),
            clock: SimulationClock::default(),
            step: limits.fixed_step,
            max_steps: limits.max_steps,
            max_behaviors: limits.max_behaviors,
            previous: poses(scene),
            current: poses(scene),
            alpha: 0.0,
        })
    }
    fn validate(&self, scene: &SceneGraph) -> Result<(), SimulationError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        Ok(())
    }
    /// # Errors
    /// Rejects foreign scenes and stale owners before invoking lifecycle hooks.
    pub fn attach<B: Behavior + 'static>(
        &mut self,
        scene: &mut SceneGraph,
        owner: NodeId,
        behavior: B,
    ) -> Result<(), SimulationError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped);
        }
        scene.local(owner)?;
        if self.behaviors.len() >= self.max_behaviors {
            return Err(SceneGraphError::Capacity.into());
        }
        self.behaviors.attach(scene, owner, behavior)?;
        Ok(())
    }
    pub fn commands(&mut self) -> &mut SceneCommands {
        &mut self.commands
    }
    pub fn clock(&mut self) -> &mut SimulationClock {
        &mut self.clock
    }
    /// Applies admitted commands at the frame barrier, synchronizes lifecycle,
    /// executes bounded fixed ticks, then invokes the variable update once.
    /// Clock policy caps elapsed time at 100 ms and discards excess whole ticks.
    /// Render interpolation is available separately through `render_world`.
    /// # Errors
    /// Rejects invalid elapsed time/foreign scenes before advancing or draining.
    pub fn advance(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
    ) -> Result<SimulationFrame, SimulationError> {
        match self.advance_with(
            scene,
            elapsed,
            |_, _| Ok::<(), std::convert::Infallible>(()),
        ) {
            Ok(frame) => Ok(frame),
            Err(SimulationStepError::Simulation(error)) => Err(error),
            Err(SimulationStepError::System { error, .. }) => match error {},
        }
    }
    /// Runs an owner-supplied system after behaviors at each fixed tick, before
    /// capturing the render pose. No call is made on zero-tick frames.
    /// # Errors
    /// Validation preserves queues. A system error consumes the clock frame,
    /// skips remaining ticks/update, and resets presentation to current scene data.
    pub fn advance_with<E>(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
        mut fixed: impl FnMut(&mut SceneGraph, f64) -> Result<(), E>,
    ) -> Result<SimulationFrame, SimulationStepError<E>> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped.into());
        }
        if !elapsed.is_finite() || elapsed < 0.0 {
            return Err(SimulationError::InvalidDelta.into());
        }
        let commands = self.commands.apply(scene).map_err(SimulationError::from)?;
        self.behaviors.sync(scene);
        let time = self.clock.advance(elapsed, self.step, self.max_steps);
        for completed_steps in 0..time.steps {
            self.previous = poses(scene);
            self.behaviors.fixed_update(scene, self.step);
            if let Err(error) = fixed(scene, self.step) {
                self.current = poses(scene);
                self.previous.clone_from(&self.current);
                return Err(SimulationStepError::System {
                    completed_steps,
                    error,
                });
            }
            self.current = poses(scene);
        }
        #[allow(clippy::cast_possible_truncation)]
        {
            self.alpha = time.alpha as f32;
        }
        self.behaviors
            .update(scene, elapsed.min(0.1) * self.clock.scale());
        Ok(SimulationFrame {
            time,
            dropped: self.clock.last_drop(),
            commands,
        })
    }
    /// Runs a validated serial phase plan at each fixed tick after behavior hooks.
    /// The frame command barrier and pose capture use the same path as advance_with.
    /// Access declarations describe intent; callbacks still receive mutable scene access.
    /// # Errors
    /// Preserves simulation validation errors and the failing system's typed error.
    pub fn advance_scheduled<E>(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
        plan: &crate::SchedulePlan,
        mut execute: impl FnMut(&str, &mut SceneGraph, f64) -> Result<(), E>,
    ) -> Result<SimulationFrame, SimulationStepError<crate::SystemFailure<E>>> {
        self.advance_with(scene, elapsed, |scene, dt| {
            plan.run_typed(|system| execute(system, scene, dt))
        })
    }
    /// Dispatches fixed phases with scene capabilities restricted by the plan.
    /// Behavior hooks remain the legacy serial adapter before scheduled phases.
    /// # Errors
    /// Preserves simulation errors and the first system's typed failure.
    pub fn advance_scoped<E>(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
        plan: &crate::SchedulePlan,
        mut execute: impl FnMut(&str, crate::SceneSystemAccess<'_>, f64) -> Result<(), E>,
    ) -> Result<SimulationFrame, SimulationStepError<crate::SystemFailure<E>>> {
        self.advance_with(scene, elapsed, |scene, dt| {
            plan.run_scene(scene, |system, access| execute(system, access, dt))
        })
    }
    /// Resets presentation history after a teleport or external scene edit.
    /// # Errors
    /// Rejects foreign scenes before replacing history.
    pub fn reset_interpolation(&mut self, scene: &SceneGraph) -> Result<(), SimulationError> {
        self.validate(scene)?;
        self.current = poses(scene);
        self.previous.clone_from(&self.current);
        Ok(())
    }
    /// Composes interpolated local TRS without modifying simulation/authoring data.
    /// New owners, changed parents/activity, and edits outside the last fixed tick
    /// use their current pose. World matrices are never decomposed.
    /// # Errors
    /// Rejects foreign scenes, stale owners and nonfinite composed matrices.
    pub fn render_world(&self, scene: &SceneGraph, owner: NodeId) -> Result<Mat4, SimulationError> {
        self.validate(scene)?;
        let mut chain = Vec::new();
        let mut cursor = Some(owner);
        while let Some(id) = cursor {
            let local = scene.local(id)?;
            let parent = scene.parent(id)?;
            let pose = Pose {
                local,
                parent,
                active: scene.active_in_hierarchy(id)?,
            };
            let mut rendered = local;
            if !self.stopped
                && self.current.get(&id) == Some(&pose)
                && let Some(previous) = self.previous.get(&id)
                && previous.parent == parent
                && previous.active == pose.active
                && previous.local != local
            {
                rendered.translation = previous
                    .local
                    .translation
                    .lerp(local.translation, self.alpha);
                rendered.scale = previous.local.scale.lerp(local.scale, self.alpha);
                rendered.rotation = previous
                    .local
                    .rotation
                    .slerp(local.rotation, self.alpha)
                    .normalize();
            }
            chain.push(rendered.matrix()?);
            cursor = parent;
        }
        let world = chain
            .into_iter()
            .rev()
            .fold(Mat4::IDENTITY, |world, local| world * local);
        if !world.is_finite() {
            return Err(SceneGraphError::WorldOverflow.into());
        }
        Ok(world)
    }
    /// Dispatches disable/destroy and discards queued changes at Stop.
    /// # Errors
    /// Rejects foreign scenes before invoking hooks or discarding commands.
    pub fn stop(&mut self, scene: &mut SceneGraph) -> Result<(), SimulationError> {
        self.validate(scene)?;
        self.behaviors.clear(scene);
        self.stopped = true;
        self.commands = SceneCommands::new(scene, 0);
        self.clock.freeze();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SceneCommand, Transform};
    use std::sync::{Arc, Mutex};
    #[derive(Debug)]
    struct Probe(Arc<Mutex<Vec<&'static str>>>);
    impl Behavior for Probe {
        fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.0.lock().unwrap().push("awake");
        }
        fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.0.lock().unwrap().push("start");
        }
        fn fixed_update(&mut self, _: &mut SceneGraph, _: NodeId, _: f64) {
            self.0.lock().unwrap().push("fixed");
        }
        fn on_disable(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.0.lock().unwrap().push("disable");
        }
        fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.0.lock().unwrap().push("destroy");
        }
    }
    #[derive(Debug)]
    struct Move;
    impl Behavior for Move {
        fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
            let mut local = scene.local(owner).unwrap();
            local.translation.x += 1.0;
            scene.set_local(owner, local).unwrap();
        }
    }
    #[test]
    fn interpolation_composes_local_hierarchy_and_snaps_external_edits_and_reuse() {
        let mut scene = SceneGraph::new(3);
        let parent = scene
            .spawn(
                None,
                Transform {
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                    scale: glam::Vec3::new(2.0, -3.0, 1.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        let child = scene.spawn(Some(parent), Transform::default()).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.02,
                max_steps: 8,
                max_behaviors: 1,
                max_commands: 2,
            },
        )
        .unwrap();
        simulation.attach(&mut scene, child, Move).unwrap();
        simulation.advance(&mut scene, 0.03).unwrap();
        let rendered = simulation
            .render_world(&scene, child)
            .unwrap()
            .transform_point3(glam::Vec3::ZERO);
        assert!((rendered - glam::Vec3::Y).length() < 1e-5);
        assert!((scene.local(child).unwrap().translation.x - 1.0).abs() < 1e-5);
        simulation.advance(&mut scene, 0.002).unwrap();
        let rendered = simulation
            .render_world(&scene, child)
            .unwrap()
            .transform_point3(glam::Vec3::ZERO);
        assert!((rendered.y - 1.2).abs() < 1e-5);
        scene
            .set_local(
                child,
                Transform {
                    translation: glam::Vec3::X * 7.0,
                    ..Transform::default()
                },
            )
            .unwrap();
        assert_eq!(
            simulation.render_world(&scene, child).unwrap(),
            scene.world_matrix(child).unwrap()
        );
        scene.reparent(child, None).unwrap();
        assert_eq!(
            simulation.render_world(&scene, child).unwrap(),
            scene.world_matrix(child).unwrap()
        );
        scene.remove_subtree(child).unwrap();
        assert!(simulation.render_world(&scene, child).is_err());
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(
            simulation.render_world(&scene, replacement).unwrap(),
            Mat4::IDENTITY
        );
    }
    #[test]
    fn catch_up_interpolates_only_last_tick_and_reset_removes_latency() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.02,
                max_steps: 8,
                max_behaviors: 1,
                max_commands: 0,
            },
        )
        .unwrap();
        simulation.attach(&mut scene, owner, Move).unwrap();
        let frame = simulation.advance(&mut scene, 0.07).unwrap();
        assert_eq!(frame.time.steps, 3);
        let rendered = simulation.render_world(&scene, owner).unwrap().w_axis.x;
        assert!((rendered - 2.5).abs() < 1e-5);
        simulation.reset_interpolation(&scene).unwrap();
        assert_eq!(
            simulation.render_world(&scene, owner).unwrap(),
            scene.world_matrix(owner).unwrap()
        );
        simulation.stop(&mut scene).unwrap();
        assert_eq!(
            simulation.render_world(&scene, owner).unwrap(),
            scene.world_matrix(owner).unwrap()
        );
    }
    #[test]
    fn external_fixed_system_participates_in_pose_capture_and_reports_partial_failure() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.02,
                max_steps: 8,
                max_behaviors: 0,
                max_commands: 0,
            },
        )
        .unwrap();
        let mut calls = 0;
        let error = simulation
            .advance_with(&mut scene, 0.07, |scene, _| {
                calls += 1;
                if calls == 2 {
                    return Err("backend query failed");
                }
                scene
                    .set_local(
                        owner,
                        Transform {
                            translation: glam::Vec3::X,
                            ..Transform::default()
                        },
                    )
                    .unwrap();
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(
            error,
            SimulationStepError::System {
                completed_steps: 1,
                error: "backend query failed"
            }
        ));
        assert_eq!(
            simulation.render_world(&scene, owner).unwrap(),
            scene.world_matrix(owner).unwrap()
        );
        assert_eq!(calls, 2);
        let mut calls = 0;
        simulation
            .advance_with(&mut scene, 0.0, |_, _| {
                calls += 1;
                Ok::<(), &str>(())
            })
            .unwrap();
        assert_eq!(calls, 0);
    }
    #[test]
    fn behavior_admission_precedes_awake_and_stop_is_terminal() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.01,
                max_steps: 2,
                max_behaviors: 1,
                max_commands: 1,
            },
        )
        .unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        simulation
            .attach(&mut scene, owner, Probe(log.clone()))
            .unwrap();
        assert!(matches!(
            simulation.attach(&mut scene, owner, Probe(log.clone())),
            Err(SimulationError::Scene(SceneGraphError::Capacity))
        ));
        assert_eq!(*log.lock().unwrap(), vec!["awake"]);
        simulation.stop(&mut scene).unwrap();
        simulation.stop(&mut scene).unwrap();
        assert_eq!(*log.lock().unwrap(), vec!["awake", "destroy"]);
        assert!(matches!(
            simulation.attach(&mut scene, owner, Probe(log)),
            Err(SimulationError::Stopped)
        ));
    }

    #[test]
    fn barriers_observe_parent_activity_and_destroy_once() {
        let mut scene = SceneGraph::new(2);
        let parent = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(parent), Transform::default()).unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.01,
                max_steps: 8,
                max_behaviors: 2,
                max_commands: 4,
            },
        )
        .unwrap();
        simulation
            .attach(&mut scene, child, Probe(log.clone()))
            .unwrap();
        simulation.advance(&mut scene, 0.02).unwrap();
        assert_eq!(
            *log.lock().unwrap(),
            vec!["awake", "start", "fixed", "fixed"]
        );
        simulation
            .commands()
            .push(SceneCommand::SetActive(parent, false))
            .unwrap();
        simulation.advance(&mut scene, 0.02).unwrap();
        assert_eq!(log.lock().unwrap().last(), Some(&"disable"));
        simulation
            .commands()
            .push(SceneCommand::RemoveSubtree(parent))
            .unwrap();
        simulation.advance(&mut scene, 0.0).unwrap();
        simulation.stop(&mut scene).unwrap();
        assert_eq!(
            log.lock()
                .unwrap()
                .iter()
                .filter(|e| **e == "destroy")
                .count(),
            1
        );
    }
    #[test]
    fn foreign_and_invalid_frames_preserve_pending_commands() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.01,
                max_steps: 2,
                max_behaviors: 1,
                max_commands: 1,
            },
        )
        .unwrap();
        simulation
            .commands()
            .push(SceneCommand::SetActive(owner, false))
            .unwrap();
        assert!(simulation.advance(&mut SceneGraph::new(1), 0.02).is_err());
        assert!(simulation.advance(&mut scene, f64::NAN).is_err());
        assert!(scene.active_self(owner).unwrap());
        let frame = simulation.advance(&mut scene, 1.0).unwrap();
        assert_eq!(frame.commands, vec![Ok(())]);
        assert_eq!(frame.time.steps, 2);
        assert!(frame.time.overloaded);
        assert!((frame.dropped.real_seconds - 0.9).abs() < 1e-12);
        assert!((frame.dropped.simulation_seconds - 0.08).abs() < 1e-12);
        assert!(!scene.active_self(owner).unwrap());
        simulation.stop(&mut scene).unwrap();
        assert!(matches!(
            simulation.advance(&mut scene, 0.01),
            Err(SimulationError::Stopped)
        ));
    }
}
