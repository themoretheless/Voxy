//! Durable behavior configuration binds to the existing simulation lifecycle.
use crate::{BoxCollider, CharacterBody};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use voxy_scene::{Behavior, NodeId, SceneGraph, SceneSimulation};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AngularMotion {
    pub axis: [f32; 3],
    /// Radians per second; bounded to keep configured fixed-step motion finite.
    pub radians_per_second: f64,
}
impl AngularMotion {
    #[must_use]
    pub fn valid(self) -> bool {
        let axis = Vec3::from_array(self.axis);
        axis.is_finite()
            && axis.length_squared().is_finite()
            && axis.length_squared() > 1e-12
            && self.radians_per_second.is_finite()
            && self.radians_per_second.abs() <= 1_000_000.0
    }
}
impl Behavior for AngularMotion {
    #[allow(clippy::cast_possible_truncation)]
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, delta: f64) {
        if scene.component::<CharacterBody>(owner).ok().flatten().is_some() {
            return; // CharacterPhysics is the sole pose writer for this owner.
        }
        if let Ok(mut local) = scene.local(owner) {
            let mut angle = (self.radians_per_second * delta).rem_euclid(std::f64::consts::TAU);
            if angle > std::f64::consts::PI {
                angle -= std::f64::consts::TAU;
            }
            let angle = angle as f32;
            local.rotation =
                (Quat::from_axis_angle(Vec3::from_array(self.axis).normalize(), angle)
                    * local.rotation)
                    .normalize();
            scene
                .set_local(owner, local)
                .expect("validated authored angular motion");
        }
    }
}
/// Typed domain failures from the editor's composed fixed phase plan.
#[derive(Debug)]
pub enum GameplayFixedError {
    Motion(String),
    Physics(crate::PhysicsError),
}
impl std::fmt::Display for GameplayFixedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Motion(error) => f.write_str(error),
            Self::Physics(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for GameplayFixedError {}
/// Live play-scene descriptors, with bounded reusable reconciliation and staging.
#[derive(Debug)]
pub struct AngularMotionBatch {
    scene: voxy_scene::SceneId,
    motions: Vec<(NodeId, AngularMotion)>,
    staging: Vec<(NodeId, AngularMotion)>,
    capacity: usize,
    edits: Vec<(NodeId, voxy_scene::Transform)>,
}
impl AngularMotionBatch {
    /// # Errors
    /// Rejects invalid/physics-owned motion and capture capacity overflow.
    pub fn new(scene: &SceneGraph, capacity: usize) -> Result<Self, String> {
        validate_behavior_descriptors(scene)?;
        let mut motions = Vec::new();
        for (owner, motion) in scene.components::<AngularMotion>() {
            if motions.len() >= capacity {
                return Err("angular motion capacity exceeded".into());
            }
            motions.push((owner, *motion));
        }
        let edits = Vec::with_capacity(motions.len());
        Ok(Self {
            scene: scene.identity(),
            staging: Vec::with_capacity(motions.len()),
            capacity,
            motions,
            edits,
        })
    }
    /// Reconciles runtime component changes before a fixed step.
    /// # Errors
    /// Invalid descriptors, foreign scenes and capacity overflow retain prior admission.
    pub fn reconcile(&mut self, scene: &SceneGraph) -> Result<(), String> {
        if scene.identity() != self.scene {
            return Err("foreign angular motion scene".into());
        }
        validate_behavior_descriptors(scene)?;
        self.staging.clear();
        for (owner, motion) in scene.components::<AngularMotion>() {
            if self.staging.len() >= self.capacity {
                return Err("angular motion capacity exceeded".into());
            }
            self.staging.push((owner, *motion));
        }
        std::mem::swap(&mut self.motions, &mut self.staging);
        Ok(())
    }
    /// # Errors
    /// Rejects foreign scenes, invalid ticks or invalid transform publication.
    /// Deleted owners are skipped; inactive owners retain their captured motion.
    #[allow(clippy::cast_possible_truncation)]
    pub fn fixed_step(&mut self, scene: &mut SceneGraph, delta: f64) -> Result<(), String> {
        if scene.identity() != self.scene || !delta.is_finite() || delta <= 0.0 || delta > 0.1 {
            return Err("invalid angular motion scene/tick".into());
        }
        self.reconcile(scene)?;
        self.edits.clear();
        for (owner, motion) in &self.motions {
            if !scene.active_in_hierarchy(*owner).unwrap_or(false)
                || scene.component::<CharacterBody>(*owner).map_err(|e| e.to_string())?.is_some()
            {
                continue;
            }
            let mut local = scene.local(*owner).map_err(|e| e.to_string())?;
            let mut angle = (motion.radians_per_second * delta).rem_euclid(std::f64::consts::TAU);
            if angle > std::f64::consts::PI {
                angle -= std::f64::consts::TAU;
            }
            local.rotation =
                (Quat::from_axis_angle(Vec3::from_array(motion.axis).normalize(), angle as f32)
                    * local.rotation)
                    .normalize();
            self.edits.push((*owner, local));
        }
        scene.set_locals(&self.edits).map_err(|e| e.to_string())
    }
    /// # Errors
    /// Rejects missing scene/domain writes before preparing any edits.
    pub fn fixed_scoped(
        &mut self,
        mut access: voxy_scene::SceneSystemAccess<'_>,
        delta: f64,
    ) -> Result<(), String> {
        access
            .require_write("angular.motion")
            .map_err(|e| e.to_string())?;
        self.fixed_step(access.write().map_err(|e| e.to_string())?, delta)
    }
}
/// # Errors
/// Returns validation failures for the fixed gameplay phase plan.
pub fn gameplay_schedule() -> Result<&'static voxy_scene::SchedulePlan, voxy_scene::ScheduleError> {
    static PLAN: std::sync::OnceLock<Result<voxy_scene::SchedulePlan, voxy_scene::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        use voxy_scene::{SchedulePlan, SystemAccess, SystemSpec};
        let motion = SchedulePlan::build(
            &[SystemSpec {
                name: "angular.step".into(),
                phase: 0,
                after: vec![],
                access: vec![
                    SystemAccess {
                        resource: "scene".into(),
                        write: true,
                    },
                    SystemAccess {
                        resource: "angular.motion".into(),
                        write: true,
                    },
                ],
            }],
            1,
        )?;
        SchedulePlan::compose(&[&motion, crate::character_schedule()?], 3)
    })
    .as_ref()
    .map_err(Clone::clone)
}
/// # Errors
/// Rejects malformed configurations and rotation of physics-owned hierarchies.
pub fn validate_behavior_descriptors(scene: &SceneGraph) -> Result<(), String> {
    for (_, motion) in scene.components::<AngularMotion>() {
        if !motion.valid() {
            return Err("invalid game.angular-motion.v1 axis/rate".into());
        }
    }
    for node in scene
        .components::<CharacterBody>()
        .map(|(node, _)| node)
        .chain(scene.components::<BoxCollider>().map(|(node, _)| node))
    {
        let mut current = Some(node);
        while let Some(owner) = current {
            let character_self = owner == node
                && scene.component::<CharacterBody>(node).map_err(|e| e.to_string())?.is_some()
                && scene.component::<BoxCollider>(node).map_err(|e| e.to_string())?.is_none();
            if !character_self && scene
                .component::<AngularMotion>(owner)
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err(format!(
                    "game.angular-motion.v1 rotates physics-owned hierarchy at {owner:?}"
                ));
            }
            current = scene.parent(owner).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
/// Runtime copies are separate from descriptors saved in authoring history.
/// # Errors
/// Rejects invalid descriptors, foreign scenes or simulation attachment limits.
pub fn bind_authored_behaviors(
    scene: &mut SceneGraph,
    simulation: &mut SceneSimulation,
) -> Result<(), String> {
    validate_behavior_descriptors(scene)?;
    let motions: Vec<_> = scene
        .components::<AngularMotion>()
        .map(|(node, motion)| (node, *motion))
        .collect();
    for (node, motion) in motions {
        simulation
            .attach(scene, node, motion)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{SimulationLimits, Transform};
    #[test]
    fn live_motion_and_grant_denial_do_not_double_apply_ticks() {
        use voxy_scene::{SchedulePlan, SystemAccess, SystemSpec};
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                AngularMotion {
                    axis: [0., 1., 0.],
                    radians_per_second: 1.,
                },
            )
            .unwrap();
        let mut batch = AngularMotionBatch::new(&scene, 1).unwrap();
        // Runtime descriptor changes reconcile only after access is admitted.
        scene
            .insert_component(
                owner,
                AngularMotion {
                    axis: [1., 0., 0.],
                    radians_per_second: 9.,
                },
            )
            .unwrap();
        for grants in [
            vec![("scene", true)],
            vec![("scene", true), ("angular.motion", false)],
            vec![("scene", false), ("angular.motion", true)],
        ] {
            let plan = SchedulePlan::build(
                &[SystemSpec {
                    name: "angular.step".into(),
                    phase: 0,
                    after: vec![],
                    access: grants
                        .into_iter()
                        .map(|(resource, write)| SystemAccess {
                            resource: resource.into(),
                            write,
                        })
                        .collect(),
                }],
                1,
            )
            .unwrap();
            assert!(
                plan.run_scene(&mut scene, |_, access| batch.fixed_scoped(access, 1. / 60.))
                    .is_err()
            );
            assert_eq!(scene.local(owner).unwrap().rotation, Quat::IDENTITY);
        }
        gameplay_schedule()
            .unwrap()
            .run_scene(&mut scene, |name, access| {
                if name == "angular.step" {
                    batch.fixed_scoped(access, 1. / 60.)
                } else {
                    Ok(())
                }
            })
            .unwrap();
        let expected = Quat::from_axis_angle(Vec3::X, 9. / 60.).normalize();
        assert!(
            scene
                .local(owner)
                .unwrap()
                .rotation
                .abs_diff_eq(expected, 1e-6)
        );
        assert_eq!(
            scene
                .component::<AngularMotion>(owner)
                .unwrap()
                .unwrap()
                .axis
                .map(f32::to_bits),
            [1.0_f32, 0.0, 0.0].map(f32::to_bits)
        );
        scene.remove_component::<AngularMotion>(owner).unwrap();
        batch.fixed_step(&mut scene, 1. / 60.).unwrap();
        assert!(
            scene
                .local(owner)
                .unwrap()
                .rotation
                .abs_diff_eq(expected, 1e-6)
        );
    }
    #[test]
    fn live_admission_capacity_and_physics_conflicts_preserve_poses() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let mut batch = AngularMotionBatch::new(&scene, 1).unwrap();
        let motion = AngularMotion {
            axis: [0., 1., 0.],
            radians_per_second: 1.,
        };
        scene.insert_component(root, motion).unwrap();
        batch.fixed_step(&mut scene, 1. / 60.).unwrap();
        let before = scene.local(root).unwrap();
        scene.insert_component(child, motion).unwrap();
        assert!(batch.fixed_step(&mut scene, 1. / 60.).is_err());
        assert_eq!(scene.local(root).unwrap(), before);
        assert_eq!(scene.local(child).unwrap().rotation, Quat::IDENTITY);
        scene.remove_component::<AngularMotion>(child).unwrap();
        scene
            .insert_component(child, CharacterBody::default())
            .unwrap();
        assert!(batch.fixed_step(&mut scene, 1. / 60.).is_err());
        assert_eq!(scene.local(root).unwrap(), before);
        scene.remove_component::<CharacterBody>(child).unwrap();
        scene.remove_component::<AngularMotion>(root).unwrap();
        scene.insert_component(child, motion).unwrap();
        batch.fixed_step(&mut scene, 1. / 60.).unwrap();
        assert_eq!(scene.local(root).unwrap(), before);
        assert!(
            !scene
                .local(child)
                .unwrap()
                .rotation
                .abs_diff_eq(Quat::IDENTITY, 1e-6)
        );
    }
    #[test]
    fn batch_matches_legacy_activity_capture_deletion_and_reused_handles() {
        let build = || {
            let mut scene = SceneGraph::new(3);
            let root = scene.spawn(None, Transform::default()).unwrap();
            let child = scene.spawn(Some(root), Transform::default()).unwrap();
            for (node, rate) in [(root, -1.), (child, 2.)] {
                scene
                    .insert_component(
                        node,
                        AngularMotion {
                            axis: [0., 1., 0.],
                            radians_per_second: rate,
                        },
                    )
                    .unwrap();
            }
            (scene, [root, child])
        };
        let (mut legacy, old_nodes) = build();
        let (mut scene, nodes) = build();
        let mut runner = voxy_scene::BehaviorRunner::default();
        for node in old_nodes {
            runner
                .attach(
                    &mut legacy,
                    node,
                    AngularMotion {
                        axis: [0., 1., 0.],
                        radians_per_second: if node == old_nodes[0] { -1. } else { 2. },
                    },
                )
                .unwrap();
        }
        let mut batch = AngularMotionBatch::new(&scene, 2).unwrap();
        assert!(AngularMotionBatch::new(&scene, 1).is_err());
        for active in [false, true, true, false, true] {
            legacy.set_active(old_nodes[0], active).unwrap();
            scene.set_active(nodes[0], active).unwrap();
            runner.fixed_update(&mut legacy, 1. / 60.);
            batch.fixed_step(&mut scene, 1. / 60.).unwrap();
            for (old, new) in old_nodes.into_iter().zip(nodes) {
                assert!(
                    legacy
                        .world_matrix(old)
                        .unwrap()
                        .abs_diff_eq(scene.world_matrix(new).unwrap(), 1e-6)
                );
            }
        }
        legacy.remove_subtree(old_nodes[1]).unwrap();
        scene.remove_subtree(nodes[1]).unwrap();
        let reused = scene.spawn(None, Transform::default()).unwrap();
        runner.fixed_update(&mut legacy, 1. / 60.);
        batch.fixed_step(&mut scene, 1. / 60.).unwrap();
        assert_eq!(scene.local(reused).unwrap().rotation, Quat::IDENTITY);
        let before = scene.local(nodes[0]).unwrap();
        assert!(batch.fixed_step(&mut scene, f64::NAN).is_err());
        assert_eq!(scene.local(nodes[0]).unwrap(), before);
        let mut foreign = SceneGraph::new(1);
        assert!(batch.fixed_step(&mut foreign, 1. / 60.).is_err());
    }
    #[test]
    fn inactive_motion_and_physics_ownership_use_existing_lifecycle() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        scene
            .insert_component(
                root,
                AngularMotion {
                    axis: [0., 1., 0.],
                    radians_per_second: -1.,
                },
            )
            .unwrap();
        scene.set_active(root, false).unwrap();
        let mut simulation = SceneSimulation::new(
            &scene,
            SimulationLimits {
                fixed_step: 1.0 / 60.0,
                max_steps: 8,
                max_behaviors: 2,
                max_commands: 2,
            },
        )
        .unwrap();
        bind_authored_behaviors(&mut scene, &mut simulation).unwrap();
        simulation.advance(&mut scene, 1.0 / 60.0).unwrap();
        assert_eq!(scene.local(root).unwrap().rotation, Quat::IDENTITY);
        scene.set_active(root, true).unwrap();
        simulation.advance(&mut scene, 1.0 / 60.0).unwrap();
        assert!(
            scene
                .local(root)
                .unwrap()
                .rotation
                .angle_between(Quat::IDENTITY)
                > 0.01
        );
        assert!(
            scene
                .local(root)
                .unwrap()
                .rotation
                .angle_between(Quat::from_rotation_y(-1.0 / 60.0))
                < 1e-3
        );
        scene
            .insert_component(child, BoxCollider::default())
            .unwrap();
        assert!(
            validate_behavior_descriptors(&scene)
                .unwrap_err()
                .contains("physics-owned")
        );
        simulation.stop(&mut scene).unwrap();
    }
}
