//! Scene-owned liquid world. Authored names bind once; particles and clocks live here.
use crate::LiquidSource;
use physics::liquid::{
    Config, Container, Liquid, Material, ParticleExchange, PulsedEmitter, StepStats,
};
use std::collections::BTreeMap;
use voxy_scene::{NodeId, SceneGraph, SceneId};

#[derive(Clone, Debug, PartialEq)]
struct Source {
    descriptor: LiquidSource,
    emitter: PulsedEmitter,
    position: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
struct BodyOwner {
    node: NodeId,
    descriptor: crate::LiquidBody,
    state: physics::liquid::TranslatingBody,
    published: voxy_scene::Transform,
    collider: crate::BoxCollider,
}

/// One fluid world shared by all admitted scene sources.
/// Descriptor edits require constructing a new runtime; activation pauses source clocks.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidRuntime {
    scene: SceneId,
    sources: BTreeMap<NodeId, Source>,
    liquid: Liquid,
    body: Option<BodyOwner>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidStep {
    pub emissions: Vec<(NodeId, ParticleExchange)>,
    pub physics: StepStats,
    pub dynamics: Option<physics::liquid::DynamicEnvironmentReport>,
}

fn position(scene: &SceneGraph, node: NodeId) -> Result<[f64; 3], String> {
    let p = scene
        .world_matrix(node)
        .map_err(|e| format!("liquid source transform: {e:?}"))?
        .transform_point3(glam::Vec3::ZERO);
    if !p.is_finite() {
        return Err("nonfinite liquid source position".into());
    }
    Ok([f64::from(p.x), f64::from(p.y), f64::from(p.z)])
}

struct SceneGeometry(crate::StaticWorld);
impl physics::liquid::LiquidGeometry for SceneGeometry {
    type Error = String;
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::liquid::GeometryHit, String> {
        use physics::liquid::GeometryHit;
        if self.0.0.len() > max_candidates {
            return Err("liquid collider budget exceeded".into());
        }
        let center = glam::DVec3::from_array(center);
        let edges = [
            glam::DVec3::X * radius,
            glam::DVec3::Y * radius,
            glam::DVec3::Z * radius,
        ];
        let displacement = glam::DVec3::from_array(displacement);
        let mut result = GeometryHit::Clear;
        let mut earliest = f64::INFINITY;
        for obstacle in &self.0.0 {
            if obstacle.shape.penetration_affine(center, edges).is_some() {
                return Ok(GeometryHit::Overlap);
            }
            if let Some((fraction, normal)) =
                obstacle.shape.sweep_affine(center, edges, displacement)
            {
                if fraction < earliest {
                    earliest = fraction;
                    result = GeometryHit::Contact {
                        fraction,
                        normal: normal.to_array(),
                    };
                }
            }
        }
        Ok(result)
    }
}

struct BodyEnvironment<'a> {
    environment: &'a SceneGeometry,
    template: crate::convex::AffineBox,
}
impl physics::liquid::DynamicLiquidEnvironment for BodyEnvironment<'_> {
    fn sweep_particle(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        physics::liquid::LiquidGeometry::sweep(
            self.environment,
            center,
            radius,
            displacement,
            budget,
        )
        .map_err(|_| physics::liquid::Error::CollisionBackend)
    }
    fn sweep_body(
        &self,
        body: &physics::liquid::TranslatingBody,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        use physics::liquid::{Error, GeometryHit};
        if self.environment.0.0.len() > budget {
            return Err(Error::CollisionBudget);
        }
        let center = self.template.center + glam::DVec3::from_array(body.position);
        let displacement = glam::DVec3::from_array(displacement);
        let mut earliest = 1.;
        let mut result = GeometryHit::Clear;
        for obstacle in &self.environment.0.0 {
            if obstacle
                .shape
                .penetration_affine(center, self.template.edges)
                .is_some()
            {
                return Ok(GeometryHit::Overlap);
            }
            if let Some((fraction, normal)) =
                obstacle
                    .shape
                    .sweep_affine(center, self.template.edges, displacement)
            {
                if matches!(result, GeometryHit::Clear) || fraction < earliest {
                    earliest = fraction;
                    result = GeometryHit::Contact {
                        fraction,
                        normal: normal.to_array(),
                    };
                }
            }
        }
        Ok(result)
    }
}

impl SceneLiquidRuntime {
    /// Resolve durable material names once and admit every source, including inactive ones.
    /// # Errors
    /// Duplicate/empty material names, invalid physics/source settings or source budget.
    pub fn new(
        scene: &SceneGraph,
        materials: Vec<(String, Material)>,
        config: Config,
        max_sources: usize,
    ) -> Result<Self, String> {
        let mut slots = BTreeMap::new();
        for (slot, (name, _)) in materials.iter().enumerate() {
            if name.is_empty() || slots.insert(name.clone(), slot).is_some() {
                return Err("duplicate or empty liquid material identity".into());
            }
        }
        let owners: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if owners.len() > 1 {
            return Err("scene liquid translating-body budget is currently one".into());
        }
        let body = owners
            .first()
            .map(|(node, descriptor)| {
                if !descriptor.mass_kg.is_finite()
                    || descriptor.mass_kg <= 0.
                    || descriptor
                        .initial_velocity_m_s
                        .iter()
                        .any(|v| !v.is_finite())
                {
                    return Err("invalid liquid body mass/velocity".to_string());
                }
                if scene
                    .parent(*node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
                    || scene
                        .component::<crate::CharacterBody>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                    || scene
                        .component::<crate::AngularMotion>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                {
                    return Err("liquid body requires an exclusively owned root transform".into());
                }
                for (other, _) in scene.components::<crate::BoxCollider>() {
                    let mut parent = scene
                        .parent(other)
                        .map_err(|e| format!("liquid collider parent: {e:?}"))?;
                    while let Some(p) = parent {
                        if p == *node {
                            return Err(
                                "compound liquid body colliders are not admitted yet".into()
                            );
                        }
                        parent = scene
                            .parent(p)
                            .map_err(|e| format!("liquid collider parent: {e:?}"))?;
                    }
                }
                let collider = *scene
                    .component::<crate::BoxCollider>(*node)
                    .map_err(|e| format!("liquid body collider: {e:?}"))?
                    .ok_or("liquid body requires BoxCollider")?;
                crate::affine_box(scene, *node, collider.half_extents)
                    .map_err(|e| format!("liquid body geometry: {e:?}"))?;
                if scene
                    .active_in_hierarchy(*node)
                    .map_err(|e| format!("liquid body activity: {e:?}"))?
                {
                    let shape = crate::affine_box(scene, *node, collider.half_extents)
                        .map_err(|e| format!("liquid body geometry: {e:?}"))?;
                    let world = crate::static_world(scene)
                        .map_err(|e| format!("liquid environment: {e:?}"))?;
                    if world.0.iter().any(|b| {
                        b.owner != *node
                            && b.shape
                                .penetration_affine(shape.center, shape.edges)
                                .is_some()
                    }) {
                        return Err("liquid body initially overlaps scene geometry".into());
                    }
                }
                let published = scene
                    .local(*node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?;
                Ok(BodyOwner {
                    node: *node,
                    descriptor: **descriptor,
                    state: physics::liquid::TranslatingBody {
                        position: position(scene, *node)?,
                        velocity: descriptor.initial_velocity_m_s,
                        mass: descriptor.mass_kg,
                    },
                    published,
                    collider,
                })
            })
            .transpose()?;
        let liquid = Liquid::new(
            Vec::new(),
            materials.into_iter().map(|(_, m)| m).collect(),
            config,
        )
        .map_err(|e| format!("liquid world: {e:?}"))?;
        let mut sources = BTreeMap::new();
        for (node, descriptor) in scene.components::<LiquidSource>() {
            if sources.len() >= max_sources {
                return Err("scene liquid source budget exceeded".into());
            }
            let slot = *slots.get(&descriptor.material_asset).ok_or_else(|| {
                format!("unresolved liquid material: {}", descriptor.material_asset)
            })?;
            let position = position(scene, node)?;
            let emitter = descriptor
                .prepare(position, slot)
                .map_err(|e| format!("liquid source: {e:?}"))?;
            sources.insert(
                node,
                Source {
                    descriptor: descriptor.clone(),
                    emitter,
                    position,
                },
            );
        }
        Ok(Self {
            scene: scene.identity(),
            sources,
            liquid,
            body,
        })
    }

    #[must_use]
    pub fn liquid(&self) -> &Liquid {
        &self.liquid
    }
    #[must_use]
    pub fn source_elapsed(&self, node: NodeId) -> Option<f64> {
        self.sources.get(&node).map(|s| s.emitter.elapsed())
    }

    /// Validate the runtime belongs to this scene and still owns its admitted sources.
    /// # Errors
    /// Foreign scenes, source addition/removal or descriptor edits require rebind.
    pub fn validate_bindings(&self, scene: &SceneGraph) -> Result<(), String> {
        if scene.identity() != self.scene {
            return Err("foreign scene liquid runtime".into());
        }
        let descriptors: BTreeMap<_, _> = scene.components::<LiquidSource>().collect();
        if descriptors.len() != self.sources.len()
            || self.sources.iter().any(|(node, source)| {
                descriptors
                    .get(node)
                    .is_none_or(|d| **d != source.descriptor)
            })
        {
            return Err("liquid source ownership or descriptor changed; rebind runtime".into());
        }
        let bodies: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if bodies.len() != usize::from(self.body.is_some()) {
            return Err("liquid body ownership changed; rebind runtime".into());
        }
        if let Some(body) = &self.body {
            if scene
                .component::<crate::CharacterBody>(body.node)
                .map_err(|e| format!("liquid body ownership: {e:?}"))?
                .is_some()
                || scene
                    .component::<crate::AngularMotion>(body.node)
                    .map_err(|e| format!("liquid body ownership: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body has another transform owner".into());
            }
            for (other, _) in scene.components::<crate::BoxCollider>() {
                let mut parent = scene
                    .parent(other)
                    .map_err(|e| format!("liquid collider parent: {e:?}"))?;
                while let Some(p) = parent {
                    if p == body.node {
                        return Err("compound liquid body colliders are not admitted yet".into());
                    }
                    parent = scene
                        .parent(p)
                        .map_err(|e| format!("liquid collider parent: {e:?}"))?;
                }
            }
            if bodies[0].0 != body.node
                || *bodies[0].1 != body.descriptor
                || scene
                    .local(body.node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?
                    != body.published
                || scene
                    .component::<crate::BoxCollider>(body.node)
                    .map_err(|e| format!("liquid body collider: {e:?}"))?
                    != Some(&body.collider)
                || scene
                    .parent(body.node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body descriptor/pose changed; rebind runtime".into());
            }
        }
        Ok(())
    }

    /// Prepare emission, fluid/body motion and scene publication as one transaction.
    /// # Errors
    /// Any preparation/publication failure preserves this runtime and the scene pose.
    pub fn tick_and_publish(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, String> {
        let mut candidate = self.clone();
        let report = candidate.tick(scene, dt, container)?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    /// Publish an already prepared body pose after the host's other preparation succeeds.
    /// # Errors
    /// Foreign/edited scene bindings or unrepresentable pose reject before mutation.
    pub fn publish_body_pose(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        self.validate_bindings(scene)?;
        if let Some(body) = &mut self.body {
            let mut pose = body.published;
            pose.translation = glam::DVec3::from_array(body.state.position).as_vec3();
            pose.matrix()
                .map_err(|e| format!("liquid body publication: {e:?}"))?;
            scene
                .set_local(body.node, pose)
                .map_err(|e| format!("liquid body publication: {e:?}"))?;
            body.published = pose;
        }
        Ok(())
    }
    #[must_use]
    pub fn body_state(&self) -> Option<(NodeId, physics::liquid::TranslatingBody)> {
        self.body.as_ref().map(|b| (b.node, b.state))
    }

    /// Emit then simulate on an explicitly supplied fixed interval. No caller state changes.
    /// World positions follow scene transforms; nozzle velocity is displacement / interval.
    /// Inactive sources track position without advancing their pulse clock.
    /// # Errors
    /// Foreign scene, descriptor/ownership edits, invalid time, emission or physics failure.
    /// The entire world and every source clock remain unchanged on any error.
    pub fn tick(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, String> {
        self.validate_bindings(scene)?;
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid liquid fixed interval".into());
        }
        let mut geometry = SceneGeometry(
            crate::static_world(scene).map_err(|e| format!("liquid collider: {e:?}"))?,
        );
        let mut candidate = self.clone();
        let mut emissions = Vec::new();
        for (node, source) in &mut candidate.sources {
            let current = position(scene, *node)?;
            source.emitter.template.particle.position = current;
            source.emitter.source_velocity =
                std::array::from_fn(|axis| (current[axis] - source.position[axis]) / dt);
            source.position = current;
            if scene
                .active_in_hierarchy(*node)
                .map_err(|e| format!("liquid source activity: {e:?}"))?
            {
                let exchange = source
                    .emitter
                    .advance(&mut candidate.liquid, dt)
                    .map_err(|e| format!("liquid source {node:?}: {e:?}"))?;
                emissions.push((*node, exchange));
            }
        }
        let (physics, dynamics) = if let Some(body) = &mut candidate.body {
            if container.is_some() {
                return Err(
                    "authored liquid bodies use scene colliders, not an extra container".into(),
                );
            }
            if scene
                .active_in_hierarchy(body.node)
                .map_err(|e| format!("liquid body activity: {e:?}"))?
            {
                let index = geometry
                    .0
                    .0
                    .iter()
                    .position(|b| b.owner == body.node)
                    .ok_or("missing active liquid body collider")?;
                let mut obstacle = geometry.0.0.remove(index);
                obstacle.shape.center -= glam::DVec3::from_array(position(scene, body.node)?);
                let template = obstacle.shape;
                let dynamic = SceneGeometry(crate::StaticWorld(vec![obstacle], scene.identity()));
                let environment = BodyEnvironment {
                    environment: &geometry,
                    template,
                };
                let report = candidate
                    .liquid
                    .step_with_dynamic_geometry_and_environment(
                        dt,
                        &mut body.state,
                        &dynamic,
                        &environment,
                        Default::default(),
                    )
                    .map_err(|e| format!("scene liquid body step: {e:?}"))?;
                (report.dynamics.fluid, Some(report))
            } else {
                (
                    candidate
                        .liquid
                        .step_with_geometry(dt, None, &geometry, Default::default())
                        .map_err(|e| format!("scene liquid step: {e:?}"))?,
                    None,
                )
            }
        } else {
            (
                candidate
                    .liquid
                    .step_with_geometry(dt, container, &geometry, Default::default())
                    .map_err(|e| format!("scene liquid step: {e:?}"))?,
                None,
            )
        };
        *self = candidate;
        Ok(SceneLiquidStep {
            emissions,
            physics,
            dynamics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LiquidPulse;
    use voxy_scene::Transform;

    fn fixture(max_particles: usize) -> (SceneGraph, NodeId, NodeId, SceneLiquidRuntime) {
        let mut scene = SceneGraph::new(2);
        let mut nodes = Vec::new();
        for x in [0., 10.] {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(x, 0., 0.),
                        ..Transform::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    LiquidSource {
                        pulses: vec![LiquidPulse {
                            start_s: 0.,
                            duration_s: 1.,
                            volume_m3: 0.001,
                            speed_m_s: 2.,
                        }],
                        density_kg_m3: 1000.,
                        particle_volume_m3: 0.001,
                        nozzle_radius_m: 0.,
                        direction: [1., 0., 0.],
                        material_asset: "water".into(),
                    },
                )
                .unwrap();
            nodes.push(node);
        }
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                max_particles,
                ..Config::default()
            },
            2,
        )
        .unwrap();
        (scene, nodes[0], nodes[1], runtime)
    }

    #[test]
    fn shared_world_emits_at_authored_origins_and_pauses_inactive_sources() {
        let (mut scene, first, second, mut runtime) = fixture(10);
        let report = runtime.tick(&scene, 0.001, None).unwrap();
        assert_eq!(report.emissions.len(), 2);
        let particles = runtime.liquid().particles();
        assert_eq!(particles.len(), 2);
        assert!((runtime.liquid().mass() - 0.002).abs() < 1e-15);
        for (p, x) in particles.iter().zip([0., 10.]) {
            assert!((p.position[0] - x - 0.002).abs() < 1e-12);
            assert_eq!(p.velocity, [2., 0., 0.]);
        }
        scene.set_active(second, false).unwrap();
        runtime.tick(&scene, 0.001, None).unwrap();
        assert_eq!(runtime.source_elapsed(first), Some(0.002));
        assert_eq!(runtime.source_elapsed(second), Some(0.001));
        assert_eq!(runtime.liquid().particles().len(), 3);
        assert!((runtime.liquid().particles()[1].position[0] - 10.004).abs() < 1e-12);
    }

    #[test]
    fn moving_nozzle_transfers_world_velocity_and_admitted_preflight_succeeds() {
        let (mut scene, first, second, mut runtime) = fixture(10);
        crate::validate_game_descriptors_with_liquid_runtime(&scene, 2, &runtime).unwrap();
        assert!(crate::validate_game_descriptors(&scene, 2).is_err());
        scene.set_active(second, false).unwrap();
        let mut transform = scene.local(first).unwrap();
        transform.translation.x = 0.01;
        scene.set_local(first, transform).unwrap();
        runtime.tick(&scene, 0.01, None).unwrap();
        let p = runtime.liquid().particles()[0];
        let nozzle_velocity = f64::from(transform.translation.x) / 0.01;
        assert!((p.velocity[0] - 2. - nozzle_velocity).abs() < 1e-12);
        assert!(
            (p.position[0] - f64::from(transform.translation.x) - p.velocity[0] * 0.01).abs()
                < 1e-12
        );
    }

    #[test]
    fn rotated_wall_uses_exact_geometry_and_oblique_normal() {
        let (mut scene, first, second, _) = fixture(10);
        scene.set_active(second, false).unwrap();
        scene.remove_component::<LiquidSource>(second).unwrap();
        scene
            .set_local(
                second,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        scene.set_active(second, true).unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Config::default()
            },
            1,
        )
        .unwrap();
        runtime.tick(&scene, 0.05, None).unwrap();
        let p = runtime.liquid().particles()[0];
        assert!((p.velocity[0] - 1.).abs() < 1e-6);
        assert!((p.velocity[1] + 1.).abs() < 1e-6);
        assert!(p.position[1] < 0.);
        assert_eq!(runtime.source_elapsed(first), Some(0.05));
        assert!((runtime.liquid().mass() - 0.05).abs() < 1e-12);
        let before = runtime.clone();
        scene
            .component_mut::<crate::BoxCollider>(second)
            .unwrap()
            .unwrap()
            .half_extents = [1., 1., 1.];
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
    }

    #[test]
    fn finite_translating_body_uses_scene_affine_template_and_recoils() {
        use physics::liquid::{DynamicWorldConfig, Particle, TranslatingBody};
        let mut scene = SceneGraph::new(1);
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        let original = scene.local(node).unwrap();
        let geometry = SceneGeometry(crate::static_world(&scene).unwrap());
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [0.; 3],
                velocity: [2., 0., 0.],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
        )
        .unwrap();
        let mut body = TranslatingBody {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 3.,
        };
        let report = liquid
            .step_with_dynamic_geometry(0.05, &mut body, &geometry, DynamicWorldConfig::default())
            .unwrap();
        assert_eq!(report.contacts, 1);
        let p = liquid.particles()[0];
        assert!((p.velocity[0] - 1.25).abs() < 1e-6);
        assert!((p.velocity[1] + 0.75).abs() < 1e-6);
        assert!((body.velocity[0] - 0.25).abs() < 1e-6);
        assert!((body.velocity[1] - 0.25).abs() < 1e-6);
        for a in 0..3 {
            assert!((p.velocity[a] + 3. * body.velocity[a] - [2., 0., 0.][a]).abs() < 1e-12);
        }
        assert!((report.dissipated_energy - 0.75).abs() < 1e-6);
        assert!(body.position[0] > 0. && body.position[1] > 0.);
        assert_eq!(scene.local(node).unwrap(), original);
    }

    #[test]
    fn authored_body_recoil_publishes_pose_and_failure_preserves_all_owners() {
        let (mut scene, first, second, _) = fixture(10);
        scene.remove_component::<LiquidSource>(second).unwrap();
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .pulses[0]
            .duration_s = 0.05;
        scene
            .set_local(
                second,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::LiquidBody {
                    mass_kg: 3.,
                    initial_velocity_m_s: [0.; 3],
                },
            )
            .unwrap();
        let initial = scene.local(second).unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            1,
        )
        .unwrap();
        let report = runtime.tick_and_publish(&mut scene, 0.05, None).unwrap();
        assert_eq!(report.dynamics.unwrap().dynamics.contacts, 1);
        let state = runtime.body_state().unwrap().1;
        assert!((state.velocity[0] - 0.25).abs() < 1e-6);
        assert!((state.velocity[1] - 0.25).abs() < 1e-6);
        let pose = scene.local(second).unwrap();
        assert!(pose.translation.x > initial.translation.x && pose.translation.y > 0.);
        assert_eq!(pose.rotation, initial.rotation);
        assert_eq!(pose.scale, initial.scale);
        runtime.validate_bindings(&scene).unwrap();
        let before = runtime.clone();
        let pose_before = scene.local(second).unwrap();
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .particle_volume_m3 = 1e-10;
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert_eq!(scene.local(second).unwrap(), pose_before);
    }

    #[test]
    fn body_publication_overflow_and_initial_overlap_are_admitted_atomically() {
        let mut scene = SceneGraph::new(2);
        let body = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(body, crate::BoxCollider::default())
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidBody {
                    mass_kg: 3.,
                    initial_velocity_m_s: [1e100, 0., 0.],
                },
            )
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            1,
        )
        .unwrap();
        let before = runtime.clone();
        let pose = scene.local(body).unwrap();
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert_eq!(scene.local(body).unwrap(), pose);
        let wall = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(wall, crate::BoxCollider::default())
            .unwrap();
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                1
            )
            .is_err()
        );
    }

    #[test]
    fn late_emission_and_physics_failures_roll_back_every_owner() {
        let (scene, _, _, mut runtime) = fixture(1);
        let before = runtime.clone();
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        let (scene, _, _, mut runtime) = fixture(10);
        let before = runtime.clone();
        assert!(
            runtime
                .tick(
                    &scene,
                    0.001,
                    Some(Container {
                        min: [1.; 3],
                        max: [0.; 3],
                        restitution: 0.,
                        friction: 0.
                    })
                )
                .is_err()
        );
        assert_eq!(runtime, before);
    }

    #[test]
    fn foreign_scene_edits_unresolved_material_and_budget_are_rejected() {
        let (mut scene, first, _, mut runtime) = fixture(10);
        let before = runtime.clone();
        assert!(runtime.tick(&SceneGraph::new(2), 0.001, None).is_err());
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .density_kg_m3 = 500.;
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("oil".into(), Material::OIL)],
                Config::default(),
                2
            )
            .is_err()
        );
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("water".into(), Material::WATER)],
                Config::default(),
                1
            )
            .is_err()
        );
    }
}
