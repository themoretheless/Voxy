//! Scene-owned liquid world. Authored names bind once; particles and clocks live here.
use crate::LiquidSource;
use physics::liquid::{BodyGeometryHit, ContactWitness};
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
    colliders: Vec<ColliderOwner>,
    mass_descriptor: Option<crate::LiquidMassDistribution>,
    mass_properties: Option<physics::mass_properties::MassProperties>,
    rigid_frame: Option<crate::RigidBodyFrame>,
}

#[derive(Clone, Debug, PartialEq)]
struct ColliderOwner {
    node: NodeId,
    collider: crate::BoxCollider,
    path: Vec<(NodeId, voxy_scene::Transform)>,
}
fn body_colliders(scene: &SceneGraph, root: NodeId) -> Result<Vec<ColliderOwner>, String> {
    let mut result = Vec::new();
    for (node, collider) in scene.components::<crate::BoxCollider>() {
        let mut current = Some(node);
        let mut path = Vec::new();
        while let Some(id) = current {
            if id == root {
                for (child, _) in &path {
                    if scene
                        .component::<crate::LiquidBody>(*child)
                        .map_err(|e| format!("collider owner: {e:?}"))?
                        .is_some()
                        || scene
                            .component::<crate::CharacterBody>(*child)
                            .map_err(|e| format!("collider owner: {e:?}"))?
                            .is_some()
                        || scene
                            .component::<crate::AngularMotion>(*child)
                            .map_err(|e| format!("collider owner: {e:?}"))?
                            .is_some()
                    {
                        return Err("compound collider has another transform owner".into());
                    }
                }
                crate::validate_extents(collider.half_extents)
                    .map_err(|e| format!("compound extents: {e:?}"))?;
                crate::affine_box(scene, node, collider.half_extents)
                    .map_err(|e| format!("compound geometry: {e:?}"))?;
                result.push(ColliderOwner {
                    node,
                    collider: *collider,
                    path,
                });
                break;
            }
            path.push((
                id,
                scene
                    .local(id)
                    .map_err(|e| format!("collider pose: {e:?}"))?,
            ));
            current = scene
                .parent(id)
                .map_err(|e| format!("collider parent: {e:?}"))?;
        }
    }
    if result.is_empty() || result.len() > 128 {
        return Err("liquid body requires 1..128 owned BoxColliders".into());
    }
    Ok(result)
}

fn collider_template(
    body: &BodyOwner,
    collider: &ColliderOwner,
) -> Result<crate::convex::AffineBox, String> {
    fn matrix(pose: voxy_scene::Transform, translation: glam::DVec3) -> glam::DMat4 {
        glam::DMat4::from_scale_rotation_translation(
            pose.scale.as_dvec3(),
            pose.rotation.as_dquat(),
            translation,
        )
    }
    // Compose in root-relative f64 coordinates: published root translation must
    // never round a child's physical offset through a world-space f32 matrix.
    let mut transform = matrix(body.published, glam::DVec3::ZERO);
    for (_, pose) in collider.path.iter().rev() {
        transform *= matrix(*pose, pose.translation.as_dvec3());
    }
    let half = collider.collider.half_extents.map(f64::from);
    let shape = crate::convex::AffineBox {
        center: transform.transform_point3(glam::DVec3::ZERO),
        edges: [
            transform.transform_vector3(glam::DVec3::X * half[0]),
            transform.transform_vector3(glam::DVec3::Y * half[1]),
            transform.transform_vector3(glam::DVec3::Z * half[2]),
        ],
    };
    if !transform.inverse().is_finite()
        || !shape.center.is_finite()
        || shape
            .edges
            .iter()
            .any(|edge| !edge.is_finite() || edge.length_squared() < 1e-20)
    {
        return Err("nonfinite compound template".into());
    }
    Ok(shape)
}

/// One fluid world shared by all admitted scene sources.
/// Descriptor edits require constructing a new runtime; activation pauses source clocks.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidRuntime {
    scene: SceneId,
    sources: BTreeMap<NodeId, Source>,
    liquid: Liquid,
    body: Vec<BodyOwner>,
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
impl SceneGeometry {
    fn sweep_contact(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, String> {
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
        let mut result = BodyGeometryHit::from(GeometryHit::Clear);
        let mut earliest = f64::INFINITY;
        for obstacle in &self.0.0 {
            if obstacle.shape.penetration_affine(center, edges).is_some() {
                return Ok(GeometryHit::Overlap.into());
            }
            if let Some(contact) = obstacle
                .shape
                .sweep_affine_contact(center, edges, displacement)
                .map_err(|e| format!("liquid contact witness: {e:?}"))?
            {
                let fraction = contact.fraction;
                let normal = contact.normal;
                if fraction < earliest {
                    earliest = fraction;
                    result = BodyGeometryHit {
                        geometry: GeometryHit::Contact {
                            fraction,
                            normal: normal.to_array(),
                        },
                        witness: Some(ContactWitness {
                            point: contact.point.to_array(),
                            tolerance_m: contact.tolerance,
                        }),
                    };
                }
            }
        }
        Ok(result)
    }
}

impl physics::liquid::LiquidGeometry for SceneGeometry {
    type Error = String;
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::liquid::GeometryHit, String> {
        self.sweep_contact(center, radius, displacement, max_candidates)
            .map(|hit| hit.geometry)
    }
}

struct SceneBodyWorld {
    environment: SceneGeometry,
    // Root-relative world-oriented shapes for constrained translation; principal
    // COM-frame shapes when the corresponding trajectory has intrinsic Spin.
    templates: Vec<Vec<crate::convex::AffineBox>>,
}

fn sampling_frame(
    position: [f64; 3],
    duration: f64,
) -> Result<physics::rigid_motion::RigidMotion, physics::liquid::Error> {
    // Unit mass is only metadata for this stationary geometry sampling frame.
    // Environment walls never enter the solver's finite-body participant array.
    physics::contact::ContactBody {
        motion: physics::gravity::Body {
            mass: 1.,
            position,
            velocity: [0.; 3],
        },
        spin: None,
    }
    .prepare_motion(
        [0.; 3],
        [0.; 3],
        duration,
        physics::spin_path::Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 1,
            max_trials: 1,
        },
    )
    .map_err(|_| physics::liquid::Error::CollisionBackend)
}

fn trajectory_hits(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    let mut steps = budget
        .checked_mul(64)
        .ok_or(physics::liquid::Error::CollisionBudget)?;
    let mut queries = steps;
    trajectory_hits_counted(
        first,
        shapes,
        second,
        obstacles,
        budget,
        &mut steps,
        &mut queries,
    )
}
fn trajectory_hits_counted(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    use physics::liquid::{Error, GeometryHit};
    if budget == 0
        || shapes
            .len()
            .checked_mul(obstacles.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    let mut result = BodyGeometryHit::from(GeometryHit::Clear);
    let mut earliest = f64::INFINITY;
    for shape in shapes {
        for obstacle in obstacles {
            let contact = crate::angular_sweep::sweep_nominal_rigid_contact(
                first, *shape, second, *obstacle, steps, queries,
            );
            let hit = match contact {
                Ok(None) => BodyGeometryHit::from(GeometryHit::Clear),
                Ok(Some(contact)) => BodyGeometryHit {
                    geometry: GeometryHit::Contact {
                        fraction: contact.time_s / first.duration(),
                        normal: contact.normal.to_array(),
                    },
                    witness: Some(ContactWitness {
                        point: contact.point.to_array(),
                        tolerance_m: contact.tolerance_m,
                    }),
                },
                Err(crate::PhysicsError::InitialOverlap) => return Ok(GeometryHit::Overlap.into()),
                Err(crate::PhysicsError::SweepBudget) => return Err(Error::CollisionBudget),
                Err(_) => return Err(Error::CollisionBackend),
            };
            if let GeometryHit::Contact { fraction, .. } = hit.geometry {
                if fraction < earliest {
                    earliest = fraction;
                    result = hit;
                }
            }
        }
    }
    Ok(result)
}

fn affine_hit(
    shape: crate::convex::AffineBox,
    center: glam::DVec3,
    edges: [glam::DVec3; 3],
    displacement: glam::DVec3,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    if shape.penetration_affine(center, edges).is_some() {
        return Ok(GeometryHit::Overlap.into());
    }
    use crate::angular_sweep::{RigidBoxMotion, sweep_rigid_pair};
    let motion = |origin, edges, displacement| RigidBoxMotion {
        origin,
        displacement,
        orientation: glam::DQuat::IDENTITY,
        angular: glam::DVec3::ZERO,
        shape: crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges,
        },
    };
    let contact = sweep_rigid_pair(
        motion(center, edges, displacement),
        motion(shape.center, shape.edges, glam::DVec3::ZERO),
        &mut 1,
        &mut 1,
    )
    .map_err(|_| physics::liquid::Error::CollisionBackend)?;
    Ok(contact.map_or(GeometryHit::Clear.into(), |contact| {
        // Witness validation is part of admission even while angular motion is constrained.
        debug_assert!(contact.point.is_finite() && contact.tolerance.is_finite());
        BodyGeometryHit {
            geometry: GeometryHit::Contact {
                fraction: contact.fraction,
                normal: contact.normal.to_array(),
            },
            witness: Some(ContactWitness {
                point: contact.point.to_array(),
                tolerance_m: contact.tolerance,
            }),
        }
    }))
}
fn nearest_hits(
    hits: impl Iterator<Item = Result<BodyGeometryHit, physics::liquid::Error>>,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    let mut result = BodyGeometryHit::from(GeometryHit::Clear);
    let mut earliest = f64::INFINITY;
    for hit in hits {
        let hit = hit?;
        match hit.geometry {
            GeometryHit::Overlap => return Ok(hit),
            GeometryHit::Contact { fraction, .. } if fraction < earliest => {
                earliest = fraction;
                result = hit;
            }
            _ => {}
        }
    }
    Ok(result)
}
fn transport_contact(
    mut hit: BodyGeometryHit,
    displacement: glam::DVec3,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    if let (physics::liquid::GeometryHit::Contact { fraction, .. }, Some(witness)) =
        (hit.geometry, &mut hit.witness)
    {
        let offset = displacement * fraction;
        let point = glam::DVec3::from_array(witness.point) + offset;
        witness.point = point.to_array();
        witness.tolerance_m +=
            8. * f64::EPSILON * (point.abs().max_element() + offset.abs().max_element());
        if !point.is_finite() || !witness.tolerance_m.is_finite() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
    }
    Ok(hit)
}
impl physics::liquid::LiquidBodyWorld for SceneBodyWorld {
    fn sweep_particle_rigid_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if body.initial().spin.is_none() {
            let b = body.initial().motion;
            return self.sweep_particle_body_contact(
                p,
                radius,
                index,
                &physics::liquid::TranslatingBody {
                    mass: b.mass,
                    position: b.position,
                    velocity: b.velocity,
                },
                body.duration(),
                budget,
            );
        }
        let particle = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: p.mass,
                position: p.position,
                velocity: p.velocity,
            },
            spin: None,
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            body.duration(),
            physics::spin_path::Config {
                max_angular_error_rad: 1e-5,
                min_step_s: 1e-9,
                max_arcs: 1,
                max_trials: 1,
            },
        )
        .map_err(|_| physics::liquid::Error::CollisionBackend)?;
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_hits(&particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if first.initial().spin.is_none() && second.initial().spin.is_none() {
            let a = first.initial().motion;
            let b = second.initial().motion;
            return self.sweep_body_pair_contact(
                i,
                &physics::liquid::TranslatingBody {
                    mass: a.mass,
                    position: a.position,
                    velocity: a.velocity,
                },
                j,
                &physics::liquid::TranslatingBody {
                    mass: b.mass,
                    position: b.position,
                    velocity: b.velocity,
                },
                first.duration(),
                budget,
            );
        }
        trajectory_hits(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            budget,
        )
    }
    fn sweep_rigid_environment_contact(
        &self,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if body.initial().spin.is_none() {
            let b = body.initial().motion;
            return self.sweep_body_environment_contact(
                index,
                &physics::liquid::TranslatingBody {
                    mass: b.mass,
                    position: b.position,
                    velocity: b.velocity,
                },
                body.duration(),
                budget,
            );
        }
        if self.templates[index]
            .len()
            .checked_mul(self.environment.0.0.len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        let mut steps = budget
            .checked_mul(64)
            .ok_or(physics::liquid::Error::CollisionBudget)?;
        let mut queries = steps;
        nearest_hits(self.environment.0.0.iter().map(|obstacle| {
            let frame = sampling_frame(obstacle.shape.center.to_array(), body.duration())?;
            let shape = crate::convex::AffineBox {
                center: glam::DVec3::ZERO,
                edges: obstacle.shape.edges,
            };
            trajectory_hits_counted(
                body,
                &self.templates[index],
                &frame,
                &[shape],
                budget,
                &mut steps,
                &mut queries,
            )
        }))
    }
    fn sweep_particle_body(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_body_contact(p, radius, index, body, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_pair(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_pair_contact(first_index, first, second_index, second, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_particle_environment(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_environment_contact(p, radius, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_environment(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_environment_contact(index, body, dt, budget)
            .map(|hit| hit.geometry)
    }

    fn sweep_particle_body_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index].len() > budget {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().map(|template| {
            let mut shape = *template;
            shape.center += glam::DVec3::from_array(body.position);
            affine_hit(
                shape,
                glam::DVec3::from_array(p.position),
                [
                    glam::DVec3::X * radius,
                    glam::DVec3::Y * radius,
                    glam::DVec3::Z * radius,
                ],
                (glam::DVec3::from_array(p.velocity) - glam::DVec3::from_array(body.velocity)) * dt,
            )
            .and_then(|hit| transport_contact(hit, glam::DVec3::from_array(body.velocity) * dt))
        }))
    }
    fn sweep_body_pair_contact(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[first_index]
            .len()
            .checked_mul(self.templates[second_index].len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[first_index].iter().flat_map(|template| {
            self.templates[second_index].iter().map(move |obstacle| {
                let mut shape = *obstacle;
                shape.center += glam::DVec3::from_array(second.position);
                affine_hit(
                    shape,
                    template.center + glam::DVec3::from_array(first.position),
                    template.edges,
                    (glam::DVec3::from_array(first.velocity)
                        - glam::DVec3::from_array(second.velocity))
                        * dt,
                )
                .and_then(|hit| {
                    transport_contact(hit, glam::DVec3::from_array(second.velocity) * dt)
                })
            })
        }))
    }
    fn sweep_particle_environment_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.environment
            .sweep_contact(p.position, radius, p.velocity.map(|v| v * dt), budget)
            .map_err(|_| physics::liquid::Error::CollisionBackend)
    }
    fn sweep_body_environment_contact(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index]
            .len()
            .checked_mul(self.environment.0.0.len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().flat_map(|template| {
            self.environment.0.0.iter().map(move |obstacle| {
                affine_hit(
                    obstacle.shape,
                    template.center + glam::DVec3::from_array(body.position),
                    template.edges,
                    glam::DVec3::from_array(body.velocity) * dt,
                )
            })
        }))
    }
    fn has_environment(&self) -> bool {
        !self.environment.0.0.is_empty()
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
        for (node, _) in scene.components::<crate::LiquidMassDistribution>() {
            if scene
                .component::<crate::LiquidBody>(node)
                .map_err(|e| format!("mass owner: {e:?}"))?
                .is_none()
            {
                return Err("mass distribution requires a liquid body owner".into());
            }
        }
        let owners: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if owners.len() > 128 {
            return Err("scene liquid translating-body budget exceeded (128)".into());
        }
        let body = owners
            .iter()
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
                let colliders = body_colliders(scene, *node)?;
                let world =
                    crate::static_world(scene).map_err(|e| format!("liquid environment: {e:?}"))?;
                for own in world
                    .0
                    .iter()
                    .filter(|b| colliders.iter().any(|c| c.node == b.owner))
                {
                    if world.0.iter().any(|b| {
                        !colliders.iter().any(|c| c.node == b.owner)
                            && b.shape
                                .penetration_affine(own.shape.center, own.shape.edges)
                                .is_some()
                    }) {
                        return Err("liquid body initially overlaps scene geometry".into());
                    }
                }
                let published = scene
                    .local(*node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?;
                let mass_descriptor = scene
                    .component::<crate::LiquidMassDistribution>(*node)
                    .map_err(|e| format!("mass descriptor: {e:?}"))?
                    .cloned();
                let mass_properties = mass_descriptor
                    .as_ref()
                    .map(|d| d.prepare(descriptor.mass_kg, published))
                    .transpose()?;
                let rigid_frame = mass_properties
                    .map(|p| crate::RigidBodyFrame::new(published, p))
                    .transpose()
                    .map_err(|e| format!("mass principal frame: {e:?}"))?;
                Ok(BodyOwner {
                    node: *node,
                    descriptor: **descriptor,
                    state: physics::liquid::TranslatingBody {
                        position: position(scene, *node)?,
                        velocity: descriptor.initial_velocity_m_s,
                        mass: descriptor.mass_kg,
                    },
                    published,
                    colliders,
                    mass_descriptor,
                    mass_properties,
                    rigid_frame,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
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
        if bodies.len() != self.body.len() {
            return Err("liquid body ownership changed; rebind runtime".into());
        }
        if scene.components::<crate::LiquidMassDistribution>().count()
            != self
                .body
                .iter()
                .filter(|b| b.mass_descriptor.is_some())
                .count()
        {
            return Err("mass distribution ownership changed; rebind runtime".into());
        }
        for body in &self.body {
            if scene
                .component::<crate::LiquidMassDistribution>(body.node)
                .map_err(|e| format!("mass binding: {e:?}"))?
                != body.mass_descriptor.as_ref()
            {
                return Err("mass distribution changed; rebind runtime".into());
            }
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
            if body_colliders(scene, body.node)? != body.colliders {
                return Err("compound collider bindings changed; rebind runtime".into());
            }
            if !bodies
                .iter()
                .any(|(node, descriptor)| *node == body.node && **descriptor == body.descriptor)
                || scene
                    .local(body.node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?
                    != body.published
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
        let poses = self
            .body
            .iter()
            .map(|body| {
                let mut pose = body.published;
                pose.translation = glam::DVec3::from_array(body.state.position).as_vec3();
                pose.matrix()
                    .map_err(|e| format!("liquid body publication: {e:?}"))?;
                Ok((body.node, pose))
            })
            .collect::<Result<Vec<_>, String>>()?;
        scene
            .set_locals(&poses)
            .map_err(|e| format!("liquid body publication: {e:?}"))?;
        for (body, (_, pose)) in self.body.iter_mut().zip(poses) {
            body.published = pose;
        }
        Ok(())
    }
    /// First admitted body for compatibility; use body_states for the entire world.
    #[must_use]
    pub fn body_state(&self) -> Option<(NodeId, physics::liquid::TranslatingBody)> {
        self.body.first().map(|b| (b.node, b.state))
    }

    /// All admitted bodies, including paused inactive bodies, in stable owner order.
    pub fn body_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::liquid::TranslatingBody)> + '_ {
        self.body.iter().map(|b| (b.node, b.state))
    }

    /// Admitted mass tensor and COM offset in the root-relative world-oriented frame.
    /// Legacy bodies without an explicit distribution have no inferred inertia.
    pub fn body_mass_properties(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<physics::mass_properties::MassProperties>)> + '_ {
        self.body.iter().map(|b| (b.node, b.mass_properties))
    }

    /// Admitted COM/principal adapters; persistent angular state is not activated here.
    pub fn body_rigid_frames(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<crate::RigidBodyFrame>)> + '_ {
        self.body.iter().map(|b| (b.node, b.rigid_frame))
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
        if !candidate.body.is_empty() && container.is_some() {
            return Err(
                "authored liquid bodies use scene colliders, not an extra container".into(),
            );
        }
        let mut active = Vec::new();
        let mut templates = Vec::new();
        let mut states = Vec::new();
        for (owner, body) in candidate.body.iter().enumerate() {
            if !scene
                .active_in_hierarchy(body.node)
                .map_err(|e| format!("liquid body activity: {e:?}"))?
            {
                continue;
            }
            let mut shapes = Vec::new();
            for collider in &body.colliders {
                if let Some(index) = geometry.0.0.iter().position(|b| b.owner == collider.node) {
                    geometry.0.0.remove(index);
                    shapes.push(collider_template(body, collider)?);
                }
            }
            active.push(owner);
            templates.push(shapes);
            states.push(body.state);
        }
        let (physics, dynamics) = if states.is_empty() {
            (
                candidate
                    .liquid
                    .step_with_geometry(dt, container, &geometry, Default::default())
                    .map_err(|e| format!("scene liquid step: {e:?}"))?,
                None,
            )
        } else {
            let world = SceneBodyWorld {
                environment: geometry,
                templates,
            };
            let report = candidate
                .liquid
                .step_with_body_world(dt, &mut states, &world, Default::default(), 128)
                .map_err(|e| format!("scene liquid body step: {e:?}"))?;
            for (owner, state) in active.into_iter().zip(states) {
                candidate.body[owner].state = state;
            }
            (report.dynamics.fluid, Some(report))
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
    fn rigid_config() -> physics::spin_path::Config {
        physics::spin_path::Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        }
    }
    fn rigid_body(position: [f64; 3], velocity: [f64; 3], z: f64) -> physics::contact::ContactBody {
        physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position,
                velocity,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., z],
                inertia: [1.; 3],
            }),
        }
    }
    fn rigid_liquid(particles: Vec<physics::liquid::Particle>) -> Liquid {
        Liquid::new(
            particles,
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                max_particles: 8,
                ..Config::default()
            },
        )
        .unwrap()
    }
    fn elastic_rigid() -> physics::liquid::DynamicWorldConfig {
        physics::liquid::DynamicWorldConfig {
            contact: physics::liquid::ContactConfig {
                restitution: 1.,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    fn rigid_shapes() -> [crate::convex::AffineBox; 2] {
        [
            crate::convex::AffineBox {
                center: glam::DVec3::ZERO,
                edges: [
                    glam::DVec3::X * 0.04,
                    glam::DVec3::Y * 0.02,
                    glam::DVec3::Z * 0.02,
                ],
            },
            crate::convex::AffineBox {
                center: glam::DVec3::X * 0.04,
                edges: [
                    glam::DVec3::X * 0.04,
                    glam::DVec3::Y * 2.,
                    glam::DVec3::Z * 0.02,
                ],
            },
        ]
    }
    #[test]
    fn scene_geometry_and_shared_core_advance_real_off_center_rigid_collision() {
        let scene = SceneGraph::new(1);
        let shapes = rigid_shapes();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: shapes.map(|s| vec![s]).to_vec(),
        };
        let mut liquid = rigid_liquid(Vec::new());
        let mut bodies = [
            rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.),
            rigid_body([0.; 3], [0.; 3], 0.),
        ];
        let before = bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>();
        let report = liquid
            .step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &world,
                elastic_rigid(),
                2,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!((bodies[0].motion.velocity[0] - 1.).abs() < 1e-11);
        assert!((bodies[1].motion.velocity[0] - 2.).abs() < 1e-11);
        assert!((bodies[1].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
        assert!((bodies[1].spin.unwrap().orientation[2] - (-0.08_f64).sin()).abs() < 1e-11);
        assert!(
            (bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>()
                + report.dynamics.dissipated_energy
                - before)
                .abs()
                < 1e-11
        );
    }
    #[test]
    fn scene_geometry_particle_recoil_rotates_the_body_and_restores_on_budget_failure() {
        let scene = SceneGraph::new(1);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[1]]],
        };
        let mut liquid = rigid_liquid(vec![physics::liquid::Particle {
            position: [-0.07, 1., 0.],
            velocity: [3., 0., 0.],
            mass: 1.,
            material: 0,
        }]);
        let mut bodies = [rigid_body([0.; 3], [0.; 3], 0.)];
        let originals = (liquid.clone(), bodies);
        let mut limited = elastic_rigid();
        limited.contact.max_candidates = 0;
        assert!(
            liquid
                .step_with_rigid_body_world(0.03, &mut bodies, &world, limited, 1, rigid_config())
                .is_err()
        );
        assert_eq!((liquid.clone(), bodies), originals);
        let report = liquid
            .step_with_rigid_body_world(
                0.03,
                &mut bodies,
                &world,
                elastic_rigid(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!(
            (liquid.particles()[0].velocity[0] + bodies[0].motion.velocity[0] - 3.).abs() < 1e-11
        );
        assert!((bodies[0].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
        assert_ne!(bodies[0].spin.unwrap().orientation, [0., 0., 0., 1.]);
    }
    #[test]
    fn authored_static_wall_reflects_a_spinning_body_with_boundary_impulse_balance() {
        let mut scene = SceneGraph::new(1);
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.04, 0., 0.),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.04, 2., 0.02],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[0]]],
        };
        let mut liquid = rigid_liquid(Vec::new());
        let mut bodies = [rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.3)];
        let energy = bodies[0].energy().unwrap();
        let report = liquid
            .step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &world,
                elastic_rigid(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!(bodies[0].motion.velocity[0] < 0.);
        assert!((bodies[0].motion.velocity[0] + report.environment_impulse[0] - 3.).abs() < 1e-11);
        assert!(
            (bodies[0].energy().unwrap() + report.dynamics.dissipated_energy - energy).abs()
                < 1e-11
        );
        assert!((bodies[0].spin.unwrap().angular_momentum[2] - 0.3).abs() > 1e-3);
    }
    #[test]
    fn moving_body_contact_witness_is_transported_to_contact_time() {
        use physics::liquid::LiquidBodyWorld;
        let scene = SceneGraph::new(1);
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * 0.5,
                glam::DVec3::Y * 0.5,
                glam::DVec3::Z * 0.5,
            ],
        };
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![shape], vec![shape]],
        };
        let a = physics::liquid::TranslatingBody {
            mass: 1.,
            position: [-3., 0., 0.],
            velocity: [4., 0., 0.],
        };
        let b = physics::liquid::TranslatingBody {
            mass: 1.,
            position: [0.; 3],
            velocity: [1., 0., 0.],
        };
        let hit = world.sweep_body_pair_contact(0, &a, 1, &b, 1., 10).unwrap();
        let physics::liquid::GeometryHit::Contact { fraction, .. } = hit.geometry else {
            panic!("contact expected");
        };
        assert!((fraction - 2. / 3.).abs() < 1e-12);
        let witness = hit.witness.unwrap();
        assert!((witness.point[0] - (-0.5 + fraction)).abs() < witness.tolerance_m + 1e-12);
        let particle = physics::liquid::Particle {
            position: [-3., 0., 0.],
            velocity: [4., 0., 0.],
            mass: 1.,
            material: 0,
        };
        let hit = world
            .sweep_particle_body_contact(&particle, 0.5, 1, &b, 1., 10)
            .unwrap();
        assert_eq!(
            hit.geometry,
            world
                .sweep_particle_body(&particle, 0.5, 1, &b, 1., 10)
                .unwrap()
        );
        assert!((hit.witness.unwrap().point[0] - witness.point[0]).abs() < 1e-12);
    }

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
    fn body_pair_fixture() -> (SceneGraph, Vec<NodeId>, SceneLiquidRuntime) {
        let mut scene = SceneGraph::new(8);
        let mut nodes = Vec::new();
        for (x, velocity) in [(-0.2, 3.), (0., 0.), (2., 7.)] {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(x, 0., 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    crate::BoxCollider {
                        half_extents: [0.05; 3],
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    crate::LiquidBody {
                        mass_kg: 1.,
                        initial_velocity_m_s: [velocity, 0., 0.],
                    },
                )
                .unwrap();
            nodes.push(node);
        }
        scene.set_active(nodes[2], false).unwrap();
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        (scene, nodes, runtime)
    }

    #[test]
    fn multiple_scene_bodies_exchange_impulse_publish_and_pause_inactive_owner() {
        let (mut scene, nodes, mut runtime) = body_pair_fixture();
        let paused = scene.local(nodes[2]).unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let states: Vec<_> = runtime.body_states().collect();
        assert_eq!(states.len(), 3);
        for (_, state) in &states[..2] {
            assert!((state.velocity[0] - 1.5).abs() < 1e-12);
        }
        assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
        assert_eq!(states[2].1.position, [2., 0., 0.]);
        assert_eq!(states[2].1.velocity, [7., 0., 0.]);
        assert_eq!(scene.local(nodes[2]).unwrap(), paused);
        for (node, state) in &states[..2] {
            assert_eq!(
                scene.local(*node).unwrap().translation,
                glam::DVec3::from_array(state.position).as_vec3()
            );
        }
        runtime.validate_bindings(&scene).unwrap();
        runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
    }

    #[test]
    fn later_body_publication_failure_preserves_all_scene_poses() {
        let (mut scene, nodes, mut runtime) = body_pair_fixture();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        runtime.body[0].state.position[0] += 1.;
        runtime.body[1].state.position[0] = 1e100;
        let before = runtime.clone();
        assert!(runtime.publish_body_pose(&mut scene).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
    }
    #[test]
    fn child_only_compound_hits_another_body_and_shape_edits_reject_atomically() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
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
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let states: Vec<_> = runtime.body_states().collect();
        assert!((states[0].1.velocity[0] - 1.5).abs() < 1e-12);
        assert!((states[1].1.velocity[0] - 1.5).abs() < 1e-12);
        assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
        runtime.validate_bindings(&scene).unwrap();
        let before = runtime.clone();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        scene
            .set_local(
                child,
                Transform {
                    translation: glam::Vec3::new(0.2, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(runtime.tick_and_publish(&mut scene, 0.01, None).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
    }

    #[test]
    fn compound_gap_is_not_filled_by_a_bounding_box() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        for y in [-1., 1.] {
            let child = scene
                .spawn(
                    Some(nodes[0]),
                    Transform {
                        translation: glam::Vec3::new(0., y, 0.),
                        rotation: glam::Quat::from_rotation_z(0.4),
                        scale: glam::Vec3::new(1., 2., 1.),
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
        }
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 0);
        let states: Vec<_> = runtime.body_states().collect();
        assert_eq!(states[0].1.velocity, [3., 0., 0.]);
        assert_eq!(states[1].1.velocity, [0.; 3]);
        assert!((states[0].1.position[0] - 0.1).abs() < 1e-7);
    }
    #[test]
    fn compound_child_strikes_static_wall_and_balances_external_impulse() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene.set_active(nodes[1], false).unwrap();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.2, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
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
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert_eq!(runtime.body_state().unwrap().1.velocity, [0.; 3]);
        assert!((report.environment_impulse[0] - 3.).abs() < 1e-12);
        assert!((report.dynamics.dissipated_energy - 4.5).abs() < 1e-12);
        runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
    }

    #[test]
    fn root_relative_compound_template_preserves_small_offsets_far_from_origin() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .set_local(
                nodes[0],
                Transform {
                    translation: glam::Vec3::new(1e5, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.03, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.01; 3],
                },
            )
            .unwrap();
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0,
        )
        .unwrap();
        let body = &runtime.body[0];
        let collider = body.colliders.iter().find(|c| c.node == child).unwrap();
        let template = collider_template(body, collider).unwrap();
        assert_eq!(template.center.x, f64::from(0.03_f32));
    }
    #[test]
    fn compound_candidates_are_bounded_and_invalid_extents_reject() {
        use physics::liquid::LiquidBodyWorld;
        let (mut scene, nodes, runtime) = body_pair_fixture();
        let template = collider_template(&runtime.body[0], &runtime.body[0].colliders[0]).unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![template; 2], vec![template; 2]],
        };
        assert_eq!(
            world.sweep_body_pair(0, &runtime.body[0].state, 1, &runtime.body[1].state, 0.1, 3),
            Err(physics::liquid::Error::CollisionBudget)
        );
        assert_eq!(
            world.sweep_body_environment(0, &runtime.body[0].state, 0.1, 1),
            Err(physics::liquid::Error::CollisionBudget)
        );
        scene
            .component_mut::<crate::BoxCollider>(nodes[0])
            .unwrap()
            .unwrap()
            .half_extents = [-0.05; 3];
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                0
            )
            .is_err()
        );
    }
    #[test]
    fn liquid_impulse_recoils_child_only_compound_owner() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                root,
                crate::LiquidBody {
                    mass_kg: 1.,
                    initial_velocity_m_s: [0.; 3],
                },
            )
            .unwrap();
        let child = scene
            .spawn(
                Some(root),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let config = Config {
            gravity: [0.; 3],
            particle_radius: 0.01,
            ..Default::default()
        };
        let mut runtime =
            SceneLiquidRuntime::new(&scene, vec![("water".into(), Material::WATER)], config, 0)
                .unwrap();
        runtime.liquid = Liquid::new(
            vec![physics::liquid::Particle {
                position: [0.; 3],
                velocity: [3., 0., 0.],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            config,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.05, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let body = runtime.body_state().unwrap().1;
        let particle = &runtime.liquid.particles()[0];
        assert!((particle.velocity[0] - 1.5).abs() < 1e-12);
        assert!((body.velocity[0] - 1.5).abs() < 1e-12);
        assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
        assert_eq!(report.environment_impulse, [0.; 3]);
        assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
    }
    #[test]
    fn explicit_mass_distribution_admits_scaled_tensor_and_edits_rollback() {
        let (mut scene, nodes, _) = body_pair_fixture();
        let distribution = crate::LiquidMassDistribution {
            parts: vec![crate::LiquidMassPart {
                mass_kg: 1.,
                center_m: [0.25, 0., 0.],
                half_edges_m: [[0.1, 0., 0.], [0., 0.2, 0.], [0., 0., 0.3]],
            }],
        };
        scene
            .insert_component(nodes[0], distribution.clone())
            .unwrap();
        let mut pose = scene.local(nodes[0]).unwrap();
        pose.scale = glam::Vec3::new(2., 1., 1.);
        scene.set_local(nodes[0], pose).unwrap();
        // Duplicate collision proxies do not duplicate explicitly authored mass.
        let duplicate = scene
            .spawn(Some(nodes[0]), voxy_scene::Transform::default())
            .unwrap();
        scene
            .insert_component(duplicate, crate::BoxCollider::default())
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let properties = runtime.body_mass_properties().next().unwrap().1.unwrap();
        assert_eq!(properties.center, [0.5, 0., 0.]);
        assert!((properties.inertia[0][0] - (0.04 + 0.09) / 3.).abs() < 1e-12);
        assert!((properties.inertia[1][1] - (0.04 + 0.09) / 3.).abs() < 1e-12);
        assert!(runtime.body_mass_properties().nth(1).unwrap().1.is_none());
        runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
        let before = runtime.clone();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        scene
            .component_mut::<crate::LiquidMassDistribution>(nodes[0])
            .unwrap()
            .unwrap()
            .parts[0]
            .mass_kg = 2.;
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                0
            )
            .is_err()
        );
        let orphan = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
        scene.insert_component(orphan, distribution).unwrap();
        let error = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0,
        )
        .unwrap_err();
        assert!(error.contains("requires a liquid body owner"));
    }
}
