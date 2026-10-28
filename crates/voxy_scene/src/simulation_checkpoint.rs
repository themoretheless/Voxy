//! Clock and presentation history participant; not a whole behavior/session save.
use super::*;
use crate::ObjectId;
use serde::{Deserialize, Serialize};

/// Exact interpolation history and clock for an enclosing session checkpoint.
/// Does not include behaviors, commands, scene data or other runtime systems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationPresentationCheckpoint {
    version: u32,
    clock: [u64; 10],
    alpha: u32,
    previous: Vec<PoseWire>,
    current: Vec<PoseWire>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseWire {
    owner: ObjectId,
    parent: Option<ObjectId>,
    active: bool,
    translation: [u32; 3],
    rotation: [u32; 4],
    scale: [u32; 3],
}
impl SceneSimulation {
    /// Capture exact clock/interpolation history with durable scene identities.
    /// Unreachable deleted-node cache entries are omitted; they cannot affect
    /// rendering or the next tick. Behaviors and pending commands are NOT saved.
    /// # Errors
    /// Rejects foreign/stopped scenes, invalid clock state or missing/aliased IDs.
    pub fn presentation_checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<SimulationPresentationCheckpoint, SimulationError> {
        let clock = self.clock_checkpoint(scene)?;
        let mut ids: HashMap<NodeId, ObjectId> = HashMap::new();
        let mut owners: HashMap<ObjectId, NodeId> = HashMap::new();
        let mut identify = |node| {
            if let Some(id) = ids.get(&node) {
                return Ok(id.clone());
            }
            let id: ObjectId = identify(node).ok_or(SimulationError::InvalidConfiguration)?;
            if id.0.is_empty() || owners.get(&id).is_some_and(|old| *old != node) {
                return Err(SimulationError::InvalidConfiguration);
            }
            ids.insert(node, id.clone());
            owners.insert(id.clone(), node);
            Ok(id)
        };
        fn capture(
            scene: &SceneGraph,
            poses: &PoseMap,
            identify: &mut impl FnMut(NodeId) -> Result<ObjectId, SimulationError>,
        ) -> Result<Vec<PoseWire>, SimulationError> {
            let mut output = Vec::new();
            for (&owner, pose) in poses {
                if scene.local(owner).is_err()
                    || pose.parent.is_some_and(|p| scene.local(p).is_err())
                {
                    continue;
                }
                output.push(PoseWire {
                    owner: identify(owner)?,
                    parent: pose.parent.map(&mut *identify).transpose()?,
                    active: pose.active,
                    translation: pose.local.translation.to_array().map(f32::to_bits),
                    rotation: pose.local.rotation.to_array().map(f32::to_bits),
                    scale: pose.local.scale.to_array().map(f32::to_bits),
                });
            }
            output.sort_by(|a, b| a.owner.0.cmp(&b.owner.0));
            Ok(output)
        }
        let previous = capture(scene, &self.previous, &mut identify)?;
        let current = capture(scene, &self.current, &mut identify)?;
        Ok(SimulationPresentationCheckpoint {
            version: 1,
            clock,
            alpha: self.alpha.to_bits(),
            previous,
            current,
        })
    }

    /// Restore clock and presentation together without mutating scene/behaviors.
    /// The caller must restore the corresponding scene and all other participants
    /// before exposing the session. File reads/JSON parsing need outer byte limits.
    /// # Errors
    /// Rejects corrupt/version/config data, oversized histories, missing/aliased
    /// IDs and invalid poses. All validation precedes publication.
    pub fn restore_presentation_checkpoint(
        &mut self,
        scene: &SceneGraph,
        saved: &SimulationPresentationCheckpoint,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    ) -> Result<(), SimulationError> {
        self.validate(scene)?;
        if self.stopped {
            return Err(SimulationError::Stopped);
        }
        let alpha = f32::from_bits(saved.alpha);
        if saved.version != 1
            || !alpha.is_finite()
            || !(0. ..=1.).contains(&alpha)
            || saved.previous.len() > scene.len()
            || saved.current.len() > scene.len()
            || u64::try_from(self.max_steps).ok() != Some(saved.clock[9])
        {
            return Err(SimulationError::InvalidConfiguration);
        }
        let mut clock = self.clock.clone();
        if !clock.restore_checkpoint(std::array::from_fn(|i| saved.clock[i]), self.step) {
            return Err(SimulationError::InvalidConfiguration);
        }
        let mut ids: HashMap<ObjectId, NodeId> = HashMap::new();
        let mut owners: HashMap<NodeId, ObjectId> = HashMap::new();
        let mut resolve = |id: &ObjectId| {
            if let Some(&node) = ids.get(id) {
                return Ok(node);
            }
            let node = resolve(id).ok_or(SimulationError::InvalidConfiguration)?;
            scene.local(node)?;
            if id.0.is_empty() || owners.get(&node).is_some_and(|old| old != id) {
                return Err(SimulationError::InvalidConfiguration);
            }
            ids.insert(id.clone(), node);
            owners.insert(node, id.clone());
            Ok(node)
        };
        fn load(
            poses: &[PoseWire],
            resolve: &mut impl FnMut(&ObjectId) -> Result<NodeId, SimulationError>,
        ) -> Result<PoseMap, SimulationError> {
            let mut output = PoseMap::default();
            for wire in poses {
                let owner = resolve(&wire.owner)?;
                let parent = wire.parent.as_ref().map(&mut *resolve).transpose()?;
                let local = Transform {
                    translation: glam::Vec3::from_array(wire.translation.map(f32::from_bits)),
                    rotation: glam::Quat::from_array(wire.rotation.map(f32::from_bits)),
                    scale: glam::Vec3::from_array(wire.scale.map(f32::from_bits)),
                };
                local.matrix()?;
                if output
                    .insert(
                        owner,
                        Pose {
                            local,
                            parent,
                            active: wire.active,
                        },
                    )
                    .is_some()
                {
                    return Err(SimulationError::InvalidConfiguration);
                }
            }
            Ok(output)
        }
        let previous = load(&saved.previous, &mut resolve)?;
        let current = load(&saved.current, &mut resolve)?;
        // Validate a presentation-only candidate through the same world composer.
        // Empty behavior/command owners invoke no user callbacks or scene writes.
        let candidate = Self {
            scene: self.scene,
            stopped: false,
            behaviors: BehaviorRunner::default(),
            commands: SceneCommands::new(scene, 0),
            clock,
            step: self.step,
            max_steps: self.max_steps,
            max_behaviors: 0,
            previous,
            current,
            alpha,
        };
        let mut cache = RenderWorldCache::default();
        for (owner, _, _) in scene.nodes() {
            candidate.render_world_cached(scene, &mut cache, owner)?;
        }
        self.clock = candidate.clock;
        self.previous = candidate.previous;
        self.current = candidate.current;
        self.alpha = candidate.alpha;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentRegistry, SceneDocument};
    fn limits() -> SimulationLimits {
        SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 4,
            max_behaviors: 0,
            max_commands: 0,
        }
    }
    fn fixture() -> crate::LoadedScene {
        SceneDocument::from_json(r#"{"version":1,"objects":[
         {"id":"parent","parent":null,"name":"parent","active":true,"translation":[1,0,0],"rotation":[0,0,0,1],"scale":[2,1,1],"components":{}},
         {"id":"child","parent":"parent","name":"child","active":true,"translation":[0,1,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}}
        ]}"#).unwrap().load(&ComponentRegistry::default(),4).unwrap()
    }
    fn advance(
        sim: &mut SceneSimulation,
        scene: &mut SceneGraph,
        owner: NodeId,
        elapsed: f64,
    ) -> SimulationFrame {
        sim.advance_with(scene, elapsed, |scene, dt| {
            let mut local = scene.local(owner)?;
            local.translation.x += dt as f32;
            local.rotation = (local.rotation * glam::Quat::from_rotation_z(dt as f32)).normalize();
            scene.set_local(owner, local)
        })
        .unwrap()
    }
    #[test]
    fn presentation_json_rebind_preserves_interpolated_world_and_following_frames() {
        let mut original = fixture();
        let child = original.resolve(&ObjectId("child".into())).unwrap();
        let mut sim = SceneSimulation::new(&original.graph, limits()).unwrap();
        advance(&mut sim, &mut original.graph, child, 0.023);
        let saved = sim
            .presentation_checkpoint(&original.graph, |node| original.identity(node).cloned())
            .unwrap();
        assert_ne!(
            sim.render_world(&original.graph, child).unwrap(),
            original.graph.world_matrix(child).unwrap()
        );
        let json = serde_json::to_string(&saved).unwrap();
        let saved = serde_json::from_str(&json).unwrap();
        let mut document = original.capture(&ComponentRegistry::default()).unwrap();
        document.objects.reverse();
        let mut resumed = document.load(&ComponentRegistry::default(), 4).unwrap();
        let other_child = resumed.resolve(&ObjectId("child".into())).unwrap();
        let mut restored = SceneSimulation::new(&resumed.graph, limits()).unwrap();
        restored
            .restore_presentation_checkpoint(&resumed.graph, &saved, |id| resumed.resolve(id))
            .unwrap();
        let world = |sim: &SceneSimulation, scene: &SceneGraph, owner| {
            sim.render_world(scene, owner)
                .unwrap()
                .to_cols_array()
                .map(f32::to_bits)
        };
        assert_eq!(
            world(&sim, &original.graph, child),
            world(&restored, &resumed.graph, other_child)
        );
        for i in 0..120 {
            let dt = [0.003, 0.021, 0.25][i % 3];
            let a = advance(&mut sim, &mut original.graph, child, dt);
            let b = advance(&mut restored, &mut resumed.graph, other_child, dt);
            assert_eq!(a.time.steps, b.time.steps);
            assert_eq!(a.time.alpha.to_bits(), b.time.alpha.to_bits());
            assert_eq!(
                world(&sim, &original.graph, child),
                world(&restored, &resumed.graph, other_child)
            );
        }
    }
    #[test]
    fn deleted_historical_parent_is_pruned_without_resetting_visible_pose() {
        let mut original = fixture();
        let child = original.resolve(&ObjectId("child".into())).unwrap();
        let parent = original.resolve(&ObjectId("parent".into())).unwrap();
        let mut sim = SceneSimulation::new(&original.graph, limits()).unwrap();
        advance(&mut sim, &mut original.graph, child, 0.023);
        original.graph.reparent(child, None).unwrap();
        original.graph.remove_subtree(parent).unwrap();
        let saved = sim
            .presentation_checkpoint(&original.graph, |node| original.identity(node).cloned())
            .unwrap();
        let document = original.capture(&ComponentRegistry::default()).unwrap();
        let mut resumed = document.load(&ComponentRegistry::default(), 4).unwrap();
        let other = resumed.resolve(&ObjectId("child".into())).unwrap();
        let mut restored = SceneSimulation::new(&resumed.graph, limits()).unwrap();
        restored
            .restore_presentation_checkpoint(&resumed.graph, &saved, |id| resumed.resolve(id))
            .unwrap();
        for dt in [0., 0.002, 0.023] {
            advance(&mut sim, &mut original.graph, child, dt);
            advance(&mut restored, &mut resumed.graph, other, dt);
            assert_eq!(
                sim.render_world(&original.graph, child)
                    .unwrap()
                    .to_cols_array()
                    .map(f32::to_bits),
                restored
                    .render_world(&resumed.graph, other)
                    .unwrap()
                    .to_cols_array()
                    .map(f32::to_bits)
            );
        }
    }

    #[test]
    fn invalid_pose_identity_or_clock_never_partially_publishes() {
        let mut scene = fixture();
        let child = scene.resolve(&ObjectId("child".into())).unwrap();
        let mut sim = SceneSimulation::new(&scene.graph, limits()).unwrap();
        advance(&mut sim, &mut scene.graph, child, 0.023);
        let saved = sim
            .presentation_checkpoint(&scene.graph, |node| scene.identity(node).cloned())
            .unwrap();
        let mut bad_pose = saved.clone();
        bad_pose.clock[2] = 2f64.to_bits();
        bad_pose.current[0].rotation = [0; 4];
        let mut bad_alpha = saved.clone();
        bad_alpha.alpha = f32::NAN.to_bits();
        let mut bad_id = saved.clone();
        bad_id.current[0].owner = ObjectId("missing".into());
        let mut overflow = saved.clone();
        overflow
            .previous
            .iter_mut()
            .find(|pose| pose.owner.0 == "child")
            .unwrap()
            .translation[0] = f32::MAX.to_bits();
        for bad in [bad_pose, bad_alpha, bad_id, overflow] {
            assert!(
                sim.restore_presentation_checkpoint(&scene.graph, &bad, |id| scene.resolve(id))
                    .is_err()
            );
            assert_eq!(
                sim.presentation_checkpoint(&scene.graph, |node| scene.identity(node).cloned())
                    .unwrap(),
                saved
            );
        }
        assert!(
            sim.restore_presentation_checkpoint(&scene.graph, &saved, |_| Some(child))
                .is_err()
        );
        assert_eq!(
            sim.presentation_checkpoint(&scene.graph, |node| scene.identity(node).cloned())
                .unwrap(),
            saved
        );
    }
}
