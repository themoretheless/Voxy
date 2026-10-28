//! Complete native owner snapshots and bounded exact file encoding.
use super::*;
use std::collections::BTreeSet;
use voxy_scene::ObjectId;

#[derive(Clone, Copy, Debug)]
pub struct SceneLiquidCheckpointLimits {
    pub liquid: physics::liquid::LiquidStateCheckpointLimits,
    pub max_sources: usize,
    pub max_bodies: usize,
    pub max_pulses_per_source: usize,
    pub max_binding_entries: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidCheckpoint {
    version: u32,
    liquid: physics::liquid::LiquidCheckpoint,
    sources: Vec<(
        ObjectId,
        LiquidSource,
        physics::liquid::PulsedEmitterCheckpoint,
        [u64; 3],
    )>,
    bodies: Vec<BodyCheckpoint>,
}
#[derive(Clone, Debug, PartialEq)]
struct ColliderCheckpoint {
    node: ObjectId,
    collider: crate::BoxCollider,
    path: Vec<(ObjectId, voxy_scene::Transform)>,
}
#[derive(Clone, Debug, PartialEq)]
struct BodyCheckpoint {
    node: ObjectId,
    descriptor: crate::LiquidBody,
    state: physics::contact::ContactBody,
    principal_templates: Option<Vec<crate::convex::AffineBox>>,
    published: voxy_scene::Transform,
    colliders: Vec<ColliderCheckpoint>,
    mass_descriptor: Option<crate::LiquidMassDistribution>,
    mass_properties: Option<physics::mass_properties::MassProperties>,
    rigid_frame: Option<crate::RigidBodyFrame>,
    point_loads: Option<crate::RigidPointLoads>,
    point_elapsed_s: u64,
}
impl SceneLiquidRuntime {
    /// Captures accepted bindings and all persistent physical state without changing the scene.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<SceneLiquidCheckpoint, GameplayError> {
        let Self {
            scene: _,
            sources,
            liquid,
            body,
        } = self;
        self.validate_bindings(scene)?;
        let mut identities = BTreeMap::new();
        let mut reverse = BTreeMap::new();
        let mut identify = |node| -> Result<ObjectId, GameplayError> {
            scene
                .local(node)
                .map_err(|e| format!("liquid checkpoint node: {e:?}"))?;
            let id = identify(node).ok_or("missing liquid object identity")?;
            if identities
                .insert(node, id.clone())
                .is_some_and(|old| old != id)
                || reverse
                    .insert(id.clone(), node)
                    .is_some_and(|old| old != node)
            {
                return Err("ambiguous liquid object identity".into());
            }
            Ok(id)
        };
        let sources = sources
            .iter()
            .map(|(node, source)| {
                let Source {
                    descriptor,
                    emitter,
                    position,
                } = source;
                Ok((
                    identify(*node)?,
                    descriptor.clone(),
                    emitter.checkpoint(),
                    position.map(f64::to_bits),
                ))
            })
            .collect::<Result<_, GameplayError>>()?;
        let bodies = body
            .iter()
            .map(|body| {
                let BodyOwner {
                    node,
                    descriptor,
                    state,
                    principal_templates,
                    published,
                    colliders,
                    mass_descriptor,
                    mass_properties,
                    rigid_frame,
                    point_loads,
                    point_elapsed_s,
                } = body;
                Ok(BodyCheckpoint {
                    node: identify(*node)?,
                    descriptor: *descriptor,
                    state: *state,
                    principal_templates: principal_templates.clone(),
                    published: *published,
                    colliders: colliders
                        .iter()
                        .map(|collider| {
                            let ColliderOwner {
                                node,
                                collider,
                                path,
                            } = collider;
                            Ok(ColliderCheckpoint {
                                node: identify(*node)?,
                                collider: *collider,
                                path: path
                                    .iter()
                                    .map(|(node, pose)| Ok((identify(*node)?, *pose)))
                                    .collect::<Result<_, GameplayError>>()?,
                            })
                        })
                        .collect::<Result<_, GameplayError>>()?,
                    mass_descriptor: mass_descriptor.clone(),
                    mass_properties: *mass_properties,
                    rigid_frame: *rigid_frame,
                    point_loads: point_loads.clone(),
                    point_elapsed_s: point_elapsed_s.to_bits(),
                })
            })
            .collect::<Result<_, GameplayError>>()?;
        Ok(SceneLiquidCheckpoint {
            version: 1,
            liquid: liquid
                .checkpoint()
                .map_err(|e| format!("liquid capture: {e:?}"))?,
            sources,
            bodies,
        })
    }
}
impl SceneLiquidCheckpoint {
    /// Rebinds a complete independent owner into an already restored scene.
    /// Source and body order are retained even when fresh handles sort differently.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        limits: SceneLiquidCheckpointLimits,
    ) -> Result<SceneLiquidRuntime, GameplayError> {
        if self.version != 1
            || self.sources.len() > limits.max_sources
            || self.bodies.len() > limits.max_bodies
        {
            return Err("liquid owner version or capacity".into());
        }
        let mut remaining = limits.max_binding_entries;
        let mut nodes = BTreeMap::new();
        let mut ids = BTreeMap::new();
        let mut resolve = |id: &ObjectId| -> Result<NodeId, GameplayError> {
            remaining = remaining.checked_sub(1).ok_or("liquid binding budget")?;
            let node = resolve(id).ok_or("missing restored liquid object")?;
            scene
                .local(node)
                .map_err(|e| format!("liquid restored node: {e:?}"))?;
            if nodes.insert(node, id.clone()).is_some_and(|old| old != *id)
                || ids.insert(id.clone(), node).is_some_and(|old| old != node)
            {
                return Err("ambiguous restored liquid identity".into());
            }
            Ok(node)
        };
        let mut source_nodes = BTreeSet::new();
        let sources = self
            .sources
            .iter()
            .map(|(id, descriptor, emitter, position)| {
                let node = resolve(id)?;
                if position.iter().any(|v| !f64::from_bits(*v).is_finite()) {
                    return Err("nonfinite restored source history".into());
                }
                if !source_nodes.insert(node) {
                    return Err("duplicate liquid source".into());
                }
                if emitter.material >= self.liquid.particles.materials.len() {
                    return Err("restored source material slot".into());
                }
                let mut expected = descriptor
                    .prepare(position.map(f64::from_bits), emitter.material)
                    .map_err(|e| format!("restored source recipe: {e:?}"))?
                    .checkpoint();
                expected.elapsed = emitter.elapsed;
                expected.emitted_samples = emitter.emitted_samples;
                expected.source_velocity = emitter.source_velocity;
                if expected != *emitter
                    || emitter
                        .source_velocity
                        .iter()
                        .any(|v| !f64::from_bits(*v).is_finite())
                {
                    return Err("restored source recipe and emitter differ".into());
                }
                Ok((
                    node,
                    Source {
                        descriptor: descriptor.clone(),
                        emitter: emitter
                            .prepare(limits.max_pulses_per_source)
                            .map_err(|e| format!("liquid emitter restore: {e:?}"))?,
                        position: position.map(f64::from_bits),
                    },
                ))
            })
            .collect::<Result<_, GameplayError>>()?;
        let mut body_nodes = BTreeSet::new();
        let body = self
            .bodies
            .iter()
            .map(|body| {
                let node = resolve(&body.node)?;
                if !body_nodes.insert(node) {
                    return Err("duplicate liquid body".into());
                }
                body.state
                    .energy()
                    .map_err(|e| format!("restored body energy: {e:?}"))?;
                let elapsed = f64::from_bits(body.point_elapsed_s);
                let rigid = body.rigid_frame.is_some();
                if !elapsed.is_finite()
                    || elapsed < 0.
                    || rigid != body.state.spin.is_some()
                    || rigid != body.principal_templates.is_some()
                    || rigid != body.mass_properties.is_some()
                    || rigid != body.mass_descriptor.is_some()
                    || (body.point_loads.is_some() && !rigid)
                    || body.colliders.is_empty()
                    || body.colliders.len() > 128
                    || body
                        .principal_templates
                        .as_ref()
                        .is_some_and(|templates| templates.len() != body.colliders.len())
                {
                    return Err("inconsistent restored liquid body".into());
                }
                if let Some(templates) = &body.principal_templates {
                    for shape in templates {
                        if !shape.center.is_finite()
                            || !glam::DMat3::from_cols(
                                shape.edges[0],
                                shape.edges[1],
                                shape.edges[2],
                            )
                            .inverse()
                            .is_finite()
                        {
                            return Err("invalid restored body template".into());
                        }
                    }
                }
                if let Some(properties) = body.mass_properties {
                    crate::RigidBodyFrame::new(body.published, properties)
                        .map_err(|e| format!("restored mass properties: {e:?}"))?;
                }
                if let Some(frame) = body.rigid_frame {
                    frame
                        .prepare_pose(body.state, 0., 0.00001)
                        .map_err(|e| format!("restored body frame: {e:?}"))?;
                    if let Some(loads) = &body.point_loads {
                        loads.prepare(frame, elapsed, 128)?;
                    }
                }
                Ok(BodyOwner {
                    node,
                    descriptor: body.descriptor,
                    state: body.state,
                    principal_templates: body.principal_templates.clone(),
                    published: body.published,
                    colliders: body
                        .colliders
                        .iter()
                        .map(|collider| {
                            Ok(ColliderOwner {
                                node: resolve(&collider.node)?,
                                collider: collider.collider,
                                path: collider
                                    .path
                                    .iter()
                                    .map(|(id, pose)| Ok((resolve(id)?, *pose)))
                                    .collect::<Result<_, GameplayError>>()?,
                            })
                        })
                        .collect::<Result<_, GameplayError>>()?,
                    mass_descriptor: body.mass_descriptor.clone(),
                    mass_properties: body.mass_properties,
                    rigid_frame: body.rigid_frame,
                    point_loads: body.point_loads.clone(),
                    point_elapsed_s: f64::from_bits(body.point_elapsed_s),
                })
            })
            .collect::<Result<_, GameplayError>>()?;
        let candidate = SceneLiquidRuntime {
            scene: scene.identity(),
            sources,
            liquid: self
                .liquid
                .prepare(limits.liquid)
                .map_err(|e| format!("liquid world restore: {e:?}"))?,
            body,
        };
        candidate.validate_bindings(scene)?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;
    fn limits() -> SceneLiquidCheckpointLimits {
        SceneLiquidCheckpointLimits {
            liquid: physics::liquid::LiquidStateCheckpointLimits {
                core: physics::liquid::LiquidCheckpointLimits {
                    max_particles: 512,
                    max_materials: 2,
                    max_pairs: 200000,
                    max_neighbor_checks: 2000000,
                    max_substeps: 1024,
                },
                max_boundary_samples: 16,
                max_interfaces: 4,
                max_species: 4,
                max_name_bytes: 256,
                max_reaction_attempts: 4096,
            },
            max_sources: 4,
            max_bodies: 4,
            max_pulses_per_source: 16,
            max_binding_entries: 32,
        }
    }
    fn scene(reverse: bool, pose: Transform) -> (SceneGraph, BTreeMap<ObjectId, NodeId>) {
        let mut scene = SceneGraph::new(8);
        let mut nodes = BTreeMap::new();
        let order = if reverse { [1, 0] } else { [0, 1] };
        for i in order {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(10. + 10. * i as f32, 0., 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    LiquidSource {
                        pulses: vec![crate::LiquidPulse {
                            start_s: 0.,
                            duration_s: 1.,
                            volume_m3: 0.00002,
                            speed_m_s: 1.,
                        }],
                        density_kg_m3: 1000.,
                        particle_volume_m3: 0.000001,
                        nozzle_radius_m: 0.001,
                        direction: [1., 0., 0.],
                        material_asset: "water".into(),
                    },
                )
                .unwrap();
            nodes.insert(ObjectId(format!("source-{i}")), node);
        }
        let root = scene.spawn(None, pose).unwrap();
        scene
            .insert_component(
                root,
                crate::LiquidBody {
                    mass_kg: 2.,
                    initial_velocity_m_s: [0.1, 0., 0.],
                },
            )
            .unwrap();
        scene
            .insert_component(
                root,
                crate::LiquidMassDistribution {
                    parts: vec![crate::LiquidMassPart {
                        mass_kg: 2.,
                        center_m: [0.01, 0., 0.],
                        half_edges_m: [[0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
                    }],
                },
            )
            .unwrap();
        scene
            .insert_component(
                root,
                crate::RigidPointLoads {
                    points: vec![crate::RigidPointForce {
                        root_point_m: [0.04, 0., 0.],
                        world_force_n: [0., 0.1, 0.],
                        world_force_rate_n_s: [0., 0.2, 0.],
                    }],
                },
            )
            .unwrap();
        scene
            .insert_component(
                root,
                crate::BoxCollider {
                    half_extents: [0.1; 3],
                },
            )
            .unwrap();
        nodes.insert(ObjectId("body".into()), root);
        let child = scene
            .spawn(
                Some(root),
                Transform {
                    translation: glam::Vec3::new(0.15, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        nodes.insert(ObjectId("collider".into()), child);
        (scene, nodes)
    }
    fn identify(nodes: &BTreeMap<ObjectId, NodeId>, node: NodeId) -> Option<ObjectId> {
        nodes
            .iter()
            .find(|(_, value)| **value == node)
            .map(|(key, _)| key.clone())
    }
    #[test]
    fn complete_scene_owner_rebinds_reversed_handles_and_continues_120_steps() {
        let (mut original_scene, original_ids) = scene(
            false,
            Transform {
                rotation: glam::Quat::from_rotation_z(0.2),
                scale: glam::Vec3::new(1.25, 0.75, 1.),
                ..Default::default()
            },
        );
        let mut original = SceneLiquidRuntime::new(
            &original_scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                max_particles: 512,
                max_pairs: 200000,
                max_neighbor_checks: 2000000,
                max_substeps: 1024,
                ..Default::default()
            },
            4,
        )
        .unwrap();
        let rotation = physics::spin_path::Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        for _ in 0..7 {
            original
                .tick_and_publish_with_dynamics(
                    &mut original_scene,
                    0.001,
                    None,
                    Default::default(),
                    rotation,
                )
                .unwrap();
        }
        let saved = original
            .checkpoint(&original_scene, |node| identify(&original_ids, node))
            .unwrap();
        let bytes = saved.to_bytes(1_000_000).unwrap();
        let path =
            std::env::temp_dir().join(format!("voxy-scene-liquid-{}.bin", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let decoded = SceneLiquidCheckpoint::from_bytes(
            &bytes,
            SceneLiquidWireLimits {
                max_bytes: 1_000_000,
                max_allocation_bytes: 1_000_000,
                max_values: 100_000,
                liquid: physics::liquid::LiquidWireLimits {
                    max_bytes: 1_000_000,
                    max_allocation_bytes: 1_000_000,
                    max_values: 100_000,
                },
            },
        )
        .unwrap();
        assert_eq!(decoded.to_bytes(1_000_000).unwrap(), bytes);
        assert_eq!(decoded, saved);
        let saved = decoded;
        let pose = original_scene
            .local(original_ids[&ObjectId("body".into())])
            .unwrap();
        let (mut restored_scene, restored_ids) = scene(true, pose);
        assert_ne!(original_scene.identity(), restored_scene.identity());
        let mut restored = saved
            .prepare(
                &restored_scene,
                |id| restored_ids.get(id).copied(),
                limits(),
            )
            .unwrap();
        assert!(restored.sources[0].0 > restored.sources[1].0);
        assert_eq!(
            saved,
            restored
                .checkpoint(&restored_scene, |node| identify(&restored_ids, node))
                .unwrap()
        );
        let mut invalid = saved.clone();
        invalid.sources[0].2.material = usize::MAX;
        assert!(
            invalid
                .prepare(
                    &restored_scene,
                    |id| restored_ids.get(id).copied(),
                    limits()
                )
                .is_err()
        );
        invalid = saved.clone();
        invalid.bodies[0].principal_templates = None;
        assert!(
            invalid
                .prepare(
                    &restored_scene,
                    |id| restored_ids.get(id).copied(),
                    limits()
                )
                .is_err()
        );
        invalid = saved.clone();
        invalid.bodies[0].point_elapsed_s = f64::NAN.to_bits();
        assert!(
            invalid
                .prepare(
                    &restored_scene,
                    |id| restored_ids.get(id).copied(),
                    limits()
                )
                .is_err()
        );
        assert!(saved.prepare(&restored_scene, |_| None, limits()).is_err());
        assert!(
            saved
                .prepare(
                    &restored_scene,
                    |id| restored_ids.get(id).copied(),
                    SceneLiquidCheckpointLimits {
                        max_binding_entries: 0,
                        ..limits()
                    }
                )
                .is_err()
        );
        assert!(
            saved
                .prepare(
                    &restored_scene,
                    |id| restored_ids.get(id).copied(),
                    SceneLiquidCheckpointLimits {
                        max_pulses_per_source: 0,
                        ..limits()
                    }
                )
                .is_err()
        );
        assert!(
            saved
                .prepare(
                    &restored_scene,
                    |_| restored_ids.values().next().copied(),
                    limits()
                )
                .is_err()
        );
        let before = original.clone();
        assert!(
            original
                .checkpoint(&original_scene, |_| Some(ObjectId("same".into())))
                .is_err()
        );
        assert_eq!(original, before);
        for step in 0..120 {
            // Moving/inactive sources retain history and pause their clocks identically.
            for (current, ids) in [
                (&mut original_scene, &original_ids),
                (&mut restored_scene, &restored_ids),
            ] {
                let node = ids[&ObjectId("source-0".into())];
                let mut pose = current.local(node).unwrap();
                pose.translation.y = step as f32 * 0.00001;
                current.set_local(node, pose).unwrap();
                current.set_active(node, !(30..60).contains(&step)).unwrap();
            }
            let a = original
                .tick_and_publish_with_dynamics(
                    &mut original_scene,
                    0.001,
                    None,
                    Default::default(),
                    rotation,
                )
                .unwrap();
            let b = restored
                .tick_and_publish_with_dynamics(
                    &mut restored_scene,
                    0.001,
                    None,
                    Default::default(),
                    rotation,
                )
                .unwrap();
            assert_eq!(a.physics, b.physics);
            assert_eq!(a.dynamics, b.dynamics);
            assert_eq!(original.liquid, restored.liquid);
            assert_eq!(
                original
                    .checkpoint(&original_scene, |node| identify(&original_ids, node))
                    .unwrap(),
                restored
                    .checkpoint(&restored_scene, |node| identify(&restored_ids, node))
                    .unwrap()
            );
        }
        assert!(original.body[0].point_elapsed_s > 0.12);
        assert!(original.liquid.particles().len() > 2);
        assert!(original.body[0].state.spin.unwrap().angular_momentum[2] != 0.);
        assert!(original.body[0].principal_templates.is_some());
        assert!(original.validate_bindings(&restored_scene).is_err());
    }
}

mod wire;
pub use wire::SceneLiquidWireLimits;
