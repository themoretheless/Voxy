//! Explicit scene simulation ownership, lifecycle dispatch and command barriers.
use crate::Transform;
use crate::{Behavior, BehaviorRunner, NodeId, SceneCommands, SceneGraph, SceneGraphError};
use glam::Mat4;
use std::collections::HashMap;
use voxy_time::{SimulationClock, TimeDrop, TimeFrame};
#[path = "simulation_checkpoint.rs"]
mod checkpoint;
pub use checkpoint::SimulationPresentationCheckpoint;
#[path = "scene_simulation_checkpoint.rs"]
mod scene_checkpoint;
pub use scene_checkpoint::{SceneCheckpointError, SceneSimulationCheckpoint};

#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
#[error("scene simulation error: {self:?}")]
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
    previous: PoseMap,
    current: PoseMap,
    alpha: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    local: Transform,
    parent: Option<NodeId>,
    active: bool,
}
type PoseMap = HashMap<NodeId, Pose, crate::FastHashBuilder>;

/// Per-frame memo of interpolated world matrices, indexed by node slot.
#[derive(Debug, Default)]
pub struct RenderWorldCache {
    worlds: Vec<Option<(u64, Mat4)>>,
}

impl RenderWorldCache {
    /// Forgets every memoised matrix; call once per frame before extraction.
    pub fn clear(&mut self) {
        self.worlds.clear();
    }
}
fn poses(scene: &SceneGraph) -> PoseMap {
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
    /// Exact fixed-step clock state for an enclosing whole-session checkpoint.
    /// # Errors
    /// Rejects foreign/stopped scenes or invalid clock configuration/state.
    pub fn clock_checkpoint(&self, scene: &SceneGraph) -> Result<[u64; 10], SimulationError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped);
        }
        let clock = self
            .clock
            .checkpoint(self.step)
            .ok_or(SimulationError::InvalidConfiguration)?;
        let budget =
            u64::try_from(self.max_steps).map_err(|_| SimulationError::InvalidConfiguration)?;
        Ok(std::array::from_fn(
            |i| if i < 9 { clock[i] } else { budget },
        ))
    }
    /// Restore clock state only. Scene/interpolation/behavior state must be
    /// restored by the enclosing session transaction; this does not reset it.
    /// # Errors
    /// Rejects foreign/stopped scenes, changed fixed-step/catch-up configuration or invalid
    /// checkpoint data without changing the clock.
    pub fn restore_clock_checkpoint(
        &mut self,
        scene: &SceneGraph,
        words: [u64; 10],
    ) -> Result<(), SimulationError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped);
        }
        if u64::try_from(self.max_steps).ok() != Some(words[9])
            || !self
                .clock
                .restore_checkpoint(std::array::from_fn(|i| words[i]), self.step)
        {
            return Err(SimulationError::InvalidConfiguration);
        }
        Ok(())
    }

    /// Capture opted-in behaviors with their attachment/lifecycle state.
    /// # Errors
    /// Rejects foreign/stopped scenes, unsupported behaviors and codec/budget errors.
    pub fn behavior_checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<crate::ObjectId>,
        max_bytes: usize,
    ) -> Result<crate::BehaviorCheckpoint, crate::BehaviorCheckpointError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped.into());
        }
        self.behaviors
            .checkpoint(scene, identify, self.max_behaviors, max_bytes)
    }
    /// Install restored behaviors into a fresh empty simulation without awake or
    /// lifecycle replay. Whole-session loading must stage this simulation first.
    /// # Errors
    /// Rejects nonempty/stopped/foreign targets and invalid saved participants.
    /// Existing live behaviors are never silently replaced or dropped.
    pub fn restore_behavior_checkpoint(
        &mut self,
        scene: &SceneGraph,
        saved: &crate::BehaviorCheckpoint,
        registry: &crate::BehaviorCheckpointRegistry,
        resolve: impl Fn(&crate::ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<(), crate::BehaviorCheckpointError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped.into());
        }
        if !self.behaviors.is_empty() {
            return Err(crate::BehaviorCheckpointError::Invalid(
                "target behavior runner is not empty".into(),
            ));
        }
        let candidate = registry.prepare(scene, saved, resolve, self.max_behaviors, max_bytes)?;
        self.behaviors = candidate;
        Ok(())
    }

    pub fn commands(&mut self) -> &mut SceneCommands {
        &mut self.commands
    }
    pub fn clock(&mut self) -> &mut SimulationClock {
        &mut self.clock
    }
    /// Delivers an accepted animation occurrence to the owner's Rust behaviors.
    /// Call only after publishing the matching animation/physics state.
    pub fn animation_event(
        &mut self,
        scene: &mut SceneGraph,
        owner: NodeId,
        name: &str,
        phase: f64,
    ) -> Result<(), SimulationError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped);
        }
        self.behaviors.animation_event(scene, owner, name, phase);
        // Hooks can change transforms or structure after the frame pose capture.
        self.current = poses(scene);
        self.previous
            .retain(|owner, _| self.current.contains_key(owner));
        Ok(())
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
        self.advance_with_delivery(scene, elapsed, |scene, _, dt| fixed(scene, dt))
    }
    fn advance_with_delivery<E>(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
        mut fixed: impl FnMut(&mut SceneGraph, &mut BehaviorRunner, f64) -> Result<(), E>,
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
            if completed_steps == 0 {
                self.previous = poses(scene);
            } else {
                // `current` was captured at the end of the previous step and nothing
                // touched the scene since, so it is exactly this step's start pose.
                std::mem::swap(&mut self.previous, &mut self.current);
            }
            self.behaviors.fixed_update(scene, self.step);
            if let Err(error) = fixed(scene, &mut self.behaviors, self.step) {
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
    /// The frame command barrier and pose capture use the same path as `advance_with`.
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
    /// Delivers occurrences from accepted systems before the next fixed tick.
    /// Earlier successful systems remain committed if a later system fails;
    /// their occurrences are delivered before reporting that failure.
    pub fn advance_scoped_with_animation_events<E>(
        &mut self,
        scene: &mut SceneGraph,
        elapsed: f64,
        plan: &crate::SchedulePlan,
        mut execute: impl FnMut(
            &str,
            crate::SceneSystemAccess<'_>,
            f64,
        ) -> Result<Vec<(NodeId, std::sync::Arc<str>, f64)>, E>,
    ) -> Result<SimulationFrame, SimulationStepError<crate::SystemFailure<E>>> {
        self.advance_with_delivery(scene, elapsed, |scene, behaviors, dt| {
            let mut events = Vec::new();
            let result = plan.run_scene(scene, |system, access| {
                events.extend(execute(system, access, dt)?);
                Ok(())
            });
            for (owner, name, phase) in events {
                behaviors.animation_event(scene, owner, &name, phase);
            }
            result
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
            let (rendered, parent) = self.rendered_local(scene, id)?;
            chain.push(rendered);
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

    /// [`Self::render_world`] with ancestor matrices memoised in `cache` so one
    /// extraction pass walks each ancestor once instead of once per descendant.
    /// The cache is only valid within one frame: clear it after every `advance`,
    /// reset or scene edit. Results are identical to the uncached call.
    /// # Errors
    /// Same as [`Self::render_world`].
    pub fn render_world_cached(
        &self,
        scene: &SceneGraph,
        cache: &mut RenderWorldCache,
        owner: NodeId,
    ) -> Result<Mat4, SimulationError> {
        self.validate(scene)?;
        self.render_world_memo(scene, cache, owner)
    }

    fn render_world_memo(
        &self,
        scene: &SceneGraph,
        cache: &mut RenderWorldCache,
        owner: NodeId,
    ) -> Result<Mat4, SimulationError> {
        if let Some(Some((generation, world))) = cache.worlds.get(owner.slot)
            && *generation == owner.generation
        {
            return Ok(*world);
        }
        let (rendered, parent) = self.rendered_local(scene, owner)?;
        // Same association as the uncached fold: ((I * root) * child) * ...
        let world = match parent {
            Some(parent) => self.render_world_memo(scene, cache, parent)? * rendered,
            None => Mat4::IDENTITY * rendered,
        };
        if !world.is_finite() {
            return Err(SceneGraphError::WorldOverflow.into());
        }
        if cache.worlds.len() <= owner.slot {
            cache.worlds.resize(owner.slot + 1, None);
        }
        cache.worlds[owner.slot] = Some((owner.generation, world));
        Ok(world)
    }

    /// Interpolated local matrix of one node and its parent.
    fn rendered_local(
        &self,
        scene: &SceneGraph,
        id: NodeId,
    ) -> Result<(Mat4, Option<NodeId>), SimulationError> {
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
        Ok((rendered.matrix()?, parent))
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
    #[test]
    fn scoped_animation_delivery_precedes_next_tick_and_survives_later_system_failure() {
        #[derive(Debug)]
        struct Listener(Arc<Mutex<Vec<f32>>>);
        impl Behavior for Listener {
            fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
                self.0
                    .lock()
                    .unwrap()
                    .push(scene.local(owner).unwrap().translation.x);
            }
            fn animation_event(&mut self, scene: &mut SceneGraph, owner: NodeId, _: &str, _: f64) {
                let mut local = scene.local(owner).unwrap();
                local.translation.x += 1.;
                scene.set_local(owner, local).unwrap();
            }
        }
        let plan = crate::SchedulePlan::build(
            &[
                crate::SystemSpec {
                    name: "animation".into(),
                    phase: 0,
                    after: vec![],
                    access: vec![],
                },
                crate::SystemSpec {
                    name: "late".into(),
                    phase: 1,
                    after: vec!["animation".into()],
                    access: vec![],
                },
            ],
            2,
        )
        .unwrap();
        for fail in [false, true] {
            let mut scene = SceneGraph::new(1);
            let owner = scene.spawn(None, Transform::default()).unwrap();
            let log = Arc::new(Mutex::new(Vec::new()));
            let mut simulation = SceneSimulation::new(
                &scene,
                SimulationLimits {
                    fixed_step: 0.01,
                    max_steps: 8,
                    max_behaviors: 1,
                    max_commands: 1,
                },
            )
            .unwrap();
            simulation
                .attach(&mut scene, owner, Listener(log.clone()))
                .unwrap();
            let result = simulation.advance_scoped_with_animation_events(
                &mut scene,
                0.03,
                &plan,
                |system, _, _| {
                    if system == "animation" {
                        Ok(vec![(owner, Arc::from("step"), 0.1)])
                    } else if fail {
                        Err("late failure")
                    } else {
                        Ok(vec![])
                    }
                },
            );
            if fail {
                assert!(matches!(
                    result,
                    Err(SimulationStepError::System {
                        completed_steps: 0,
                        ..
                    })
                ));
                assert_eq!(*log.lock().unwrap(), vec![0.]);
                assert_eq!(scene.local(owner).unwrap().translation.x, 1.);
            } else {
                assert_eq!(result.unwrap().time.steps, 3);
                assert_eq!(*log.lock().unwrap(), vec![0., 1., 2.]);
                assert_eq!(scene.local(owner).unwrap().translation.x, 3.);
            }
            let result = simulation.advance_scoped_with_animation_events(
                &mut scene,
                0.,
                &plan,
                |_, _, _| -> Result<_, &'static str> { panic!("zero tick system") },
            );
            assert_eq!(result.unwrap().time.steps, 0);
        }
    }

    #[test]
    fn cached_render_world_matches_uncached_across_a_chain() {
        let mut scene = SceneGraph::new(64);
        let mut ids = Vec::new();
        let mut parent = None;
        for depth in 0..6 {
            let id = scene
                .spawn(
                    parent,
                    Transform {
                        translation: glam::Vec3::new(depth as f32, 0.5, -1.0),
                        rotation: glam::Quat::from_rotation_y(0.3 * depth as f32),
                        scale: glam::Vec3::splat(1.0 + 0.1 * depth as f32),
                    },
                )
                .unwrap();
            ids.push(id);
            parent = Some(id);
        }
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 0.02,
                max_steps: 8,
                max_behaviors: 8,
                max_commands: 2,
            },
        )
        .unwrap();
        for id in &ids {
            simulation.attach(&mut scene, *id, Move).unwrap();
        }
        simulation.advance(&mut scene, 0.03).unwrap();
        simulation.advance(&mut scene, 0.007).unwrap();
        let mut cache = RenderWorldCache::default();
        for id in ids.iter().rev() {
            let cached = simulation
                .render_world_cached(&scene, &mut cache, *id)
                .unwrap();
            let direct = simulation.render_world(&scene, *id).unwrap();
            assert_eq!(cached.to_cols_array(), direct.to_cols_array(), "{id:?}");
        }
        assert!(cache.worlds.iter().flatten().count() >= ids.len());
    }
}

#[cfg(test)]
mod clock_checkpoint_tests {
    use super::*;
    fn limits() -> SimulationLimits {
        SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 2,
            max_behaviors: 0,
            max_commands: 0,
        }
    }
    #[test]
    fn scene_clock_checkpoint_json_continues_schedule_on_new_scene() {
        let mut scene = SceneGraph::new(1);
        let mut original = SceneSimulation::new(&scene, limits()).unwrap();
        original.clock().set_target(3.);
        for _ in 0..7 {
            original.advance(&mut scene, 0.011).unwrap();
        }
        let words = original.clock_checkpoint(&scene).unwrap();
        let text = serde_json::to_string(&words).unwrap();
        let words = serde_json::from_str(&text).unwrap();
        let mut changed_limits = limits();
        changed_limits.max_steps = 3;
        let mut incompatible = SceneSimulation::new(&scene, changed_limits).unwrap();
        assert!(
            incompatible
                .restore_clock_checkpoint(&scene, words)
                .is_err()
        );
        let mut other_scene = SceneGraph::new(1);
        let mut resumed = SceneSimulation::new(&other_scene, limits()).unwrap();
        resumed
            .restore_clock_checkpoint(&other_scene, words)
            .unwrap();
        assert!(resumed.restore_clock_checkpoint(&scene, words).is_err());
        for i in 0..120 {
            let elapsed = [0.003, 0.017, 0.25][i % 3];
            let a = original.advance(&mut scene, elapsed).unwrap();
            let b = resumed.advance(&mut other_scene, elapsed).unwrap();
            assert_eq!(a.time.steps, b.time.steps);
            assert_eq!(a.time.alpha.to_bits(), b.time.alpha.to_bits());
            assert_eq!(
                a.dropped.real_seconds.to_bits(),
                b.dropped.real_seconds.to_bits()
            );
            assert_eq!(
                a.dropped.simulation_seconds.to_bits(),
                b.dropped.simulation_seconds.to_bits()
            );
        }
        let before = resumed.clock_checkpoint(&other_scene).unwrap();
        let mut invalid = before;
        invalid[0] = 2;
        assert!(
            resumed
                .restore_clock_checkpoint(&other_scene, invalid)
                .is_err()
        );
        assert_eq!(resumed.clock_checkpoint(&other_scene).unwrap(), before);
        resumed.stop(&mut other_scene).unwrap();
        assert!(resumed.clock_checkpoint(&other_scene).is_err());
        assert!(
            resumed
                .restore_clock_checkpoint(&other_scene, before)
                .is_err()
        );
    }
}
