//! Scene-owned liquid world. Authored names bind once; particles and clocks live here.
use crate::GameplayError;
use crate::LiquidSource;
use physics::liquid::{BodyGeometryHit, ContactWitness, RigidGeometryHit};
mod rigid_support;
use physics::liquid::{
    Config, Container, Liquid, Material, ParticleExchange, PulsedEmitter, StepStats,
};
use rigid_support::FeatureKey;
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
    state: physics::contact::ContactBody,
    principal_templates: Option<Vec<crate::convex::AffineBox>>,
    published: voxy_scene::Transform,
    colliders: Vec<ColliderOwner>,
    mass_descriptor: Option<crate::LiquidMassDistribution>,
    mass_properties: Option<physics::mass_properties::MassProperties>,
    rigid_frame: Option<crate::RigidBodyFrame>,
    point_loads: Option<crate::RigidPointLoads>,
    point_elapsed_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct ColliderOwner {
    node: NodeId,
    collider: crate::BoxCollider,
    path: Vec<(NodeId, voxy_scene::Transform)>,
}
fn body_colliders(scene: &SceneGraph, root: NodeId) -> Result<Vec<ColliderOwner>, GameplayError> {
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
) -> Result<crate::convex::AffineBox, GameplayError> {
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
    sources: Vec<(NodeId, Source)>,
    liquid: Liquid,
    body: Vec<BodyOwner>,
}

/// Snapshot-owned coupled reaction report; indices refer to this stable owner list.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSupportReactions {
    pub owners: Vec<NodeId>,
    pub reactions: physics::liquid::RigidWorldReactions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidStep {
    pub emissions: Vec<(NodeId, ParticleExchange)>,
    pub physics: StepStats,
    pub dynamics: Option<physics::liquid::DynamicEnvironmentReport>,
}

/// The same scene step plus admitted finite normal-reaction diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSupportedLiquidStep {
    pub step: SceneLiquidStep,
    pub support: Option<physics::liquid::SupportedWorldReport>,
}

fn position(scene: &SceneGraph, node: NodeId) -> Result<[f64; 3], GameplayError> {
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
    ) -> Result<BodyGeometryHit, GameplayError> {
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
    type Error = GameplayError;
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::liquid::GeometryHit, GameplayError> {
        self.sweep_contact(center, radius, displacement, max_candidates)
            .map(|hit| hit.geometry)
    }
}

#[derive(Clone, Copy)]
struct MovingBoundary {
    owner: NodeId,
    displacement: glam::DVec3,
    duration: f64,
}
impl MovingBoundary {
    fn velocity(self) -> glam::DVec3 {
        self.displacement / self.duration
    }
    fn center_at(self, end: glam::DVec3, time: f64) -> glam::DVec3 {
        end - self.displacement * (1. - (time / self.duration).clamp(0., 1.))
    }
}

struct SceneBodyWorld {
    boundaries: Vec<MovingBoundary>,
    environment: SceneGeometry,
    body_environments: Vec<Option<SceneGeometry>>,
    particle_environment: Option<SceneGeometry>,
    body_layers: Vec<crate::CollisionLayers>,
    // Root-relative world-oriented shapes for constrained translation; principal
    // COM-frame shapes when the corresponding trajectory has intrinsic Spin.
    templates: Vec<Vec<crate::convex::AffineBox>>,
}

fn sampling_frame(
    position: [f64; 3],
    duration: f64,
) -> Result<physics::rigid_motion::RigidMotion, physics::liquid::Error> {
    boundary_frame(position, [0.; 3], duration)
}

fn boundary_frame(
    position: [f64; 3],
    velocity: [f64; 3],
    duration: f64,
) -> Result<physics::rigid_motion::RigidMotion, physics::liquid::Error> {
    // Unit mass is metadata for this geometry sampling frame.
    // Environment walls never enter the solver's finite-body participant array.
    physics::contact::ContactBody {
        motion: physics::gravity::Body {
            mass: 1.,
            position,
            velocity,
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

fn trajectory_events(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    let mut steps = budget
        .checked_mul(64)
        .ok_or(physics::liquid::Error::CollisionBudget)?;
    let mut queries = steps;
    trajectory_events_counted(
        first,
        shapes,
        second,
        obstacles,
        budget,
        &mut steps,
        &mut queries,
        &[],
    )
}
#[allow(clippy::too_many_arguments)]
fn trajectory_events_counted(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
    steps: &mut usize,
    queries: &mut usize,
    initial_overlap: &[(FeatureKey, f64)],
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    use physics::liquid::{Error, GeometryHit};
    if budget == 0
        || shapes
            .len()
            .checked_mul(obstacles.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    let mut result = RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear));
    let mut earliest = f64::INFINITY;
    for (first_index, shape) in shapes.iter().enumerate() {
        for (second_index, obstacle) in obstacles.iter().enumerate() {
            let overlap_limit = initial_overlap
                .iter()
                .filter(|(key, _)| key.first == first_index && key.second == second_index)
                .map(|(_, error)| *error)
                .fold(0., f64::max);
            let contact = if overlap_limit > 0. {
                crate::angular_sweep::sweep_nominal_rigid_contact_with_overlap_limit(
                    first,
                    *shape,
                    second,
                    *obstacle,
                    steps,
                    queries,
                    overlap_limit,
                )
            } else {
                crate::angular_sweep::sweep_nominal_rigid_contact(
                    first, *shape, second, *obstacle, steps, queries,
                )
            };
            let hit = match contact {
                Ok(None) => RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear)),
                Ok(Some(contact)) => RigidGeometryHit {
                    feature: Some(
                        FeatureKey {
                            first: first_index,
                            second: second_index,
                            axis: contact.feature,
                        }
                        .encode()?,
                    ),
                    contact: BodyGeometryHit {
                        geometry: GeometryHit::Contact {
                            fraction: contact.time_s / first.duration(),
                            normal: contact.normal.to_array(),
                        },
                        witness: Some(ContactWitness {
                            point: contact.point.to_array(),
                            tolerance_m: contact.tolerance_m,
                        }),
                    },
                },
                Err(crate::PhysicsError::InitialOverlap) => {
                    return Ok(BodyGeometryHit::from(GeometryHit::Overlap).into());
                }
                Err(crate::PhysicsError::SweepBudget) => return Err(Error::CollisionBudget),
                Err(_) => return Err(Error::CollisionBackend),
            };
            if let GeometryHit::Contact { fraction, .. } = hit.contact.geometry
                && fraction < earliest
            {
                earliest = fraction;
                result = hit;
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
fn trajectory_environment_events(
    body: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    environment: &SceneGeometry,
    budget: usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    trajectory_environment_events_at(body, shapes, environment, &[], 0., budget)
}

fn trajectory_environment_events_at(
    body: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    environment: &SceneGeometry,
    boundaries: &[MovingBoundary],
    elapsed_s: f64,
    budget: usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    if shapes
        .len()
        .checked_mul(environment.0.0.len())
        .is_none_or(|n| n > budget)
    {
        return Err(physics::liquid::Error::CollisionBudget);
    }
    let mut steps = budget
        .checked_mul(64)
        .ok_or(physics::liquid::Error::CollisionBudget)?;
    let mut queries = steps;
    nearest_events(environment.0.0.iter().enumerate().map(|(wall, obstacle)| {
        let center = obstacle.shape.center;
        let frame =
            if let Some(boundary) = boundaries.iter().find(|wall| wall.owner == obstacle.owner) {
                boundary_frame(
                    boundary.center_at(center, elapsed_s).to_array(),
                    boundary.velocity().to_array(),
                    body.duration(),
                )?
            } else {
                sampling_frame(center.to_array(), body.duration())?
            };
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: obstacle.shape.edges,
        };
        let mut event = trajectory_events_counted(
            body,
            shapes,
            &frame,
            &[shape],
            budget,
            &mut steps,
            &mut queries,
            &[],
        )?;
        if let Some(token) = event.feature {
            let mut key = FeatureKey::decode(token)?;
            key.second = wall;
            event.feature = Some(key.encode()?);
        }
        Ok(event)
    }))
}

/// Search every shape pair, admitting only explicit active support branches.
#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn supported_shape_events(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    supports: &[physics::liquid::RigidSupportPoint],
    point_models: Option<&[Option<physics::liquid::RigidSupportPointMotion>]>,
    budget: usize,
    pair: rigid_support::SupportPair,
) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
    use physics::liquid::Error;
    if point_models.is_some_and(|models| models.len() != supports.len()) {
        return Err(Error::InvalidCollision);
    }
    if budget == 0
        || supports.is_empty()
        || shapes
            .len()
            .checked_mul(obstacles.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    for support in supports {
        let key = FeatureKey::decode(support.feature.ok_or(Error::CollisionBackend)?)?;
        if key.first >= shapes.len() || key.second >= obstacles.len() {
            return Err(Error::InvalidCollision);
        }
    }
    let mut steps = budget.checked_mul(64).ok_or(Error::CollisionBudget)?;
    let mut queries = steps;
    let mut error: f64 = 0.;
    let mut events = Vec::new();
    for (ia, shape) in shapes.iter().enumerate() {
        for (ib, obstacle) in obstacles.iter().enumerate() {
            let (active, models): (Vec<_>, Vec<_>) = supports
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, s)| {
                    let key = FeatureKey::decode(s.feature.unwrap()).unwrap();
                    key.first == ia && key.second == ib
                })
                .map(|(index, s)| (s, point_models.and_then(|models| models[index])))
                .unzip();
            if active.is_empty() {
                let mut hit = trajectory_events_counted(
                    first,
                    &[*shape],
                    second,
                    &[*obstacle],
                    budget,
                    &mut steps,
                    &mut queries,
                    &[],
                )?;
                if let Some(feature) = hit.feature {
                    let key = FeatureKey::decode(feature)?;
                    hit.feature = Some(
                        FeatureKey {
                            first: ia,
                            second: ib,
                            axis: key.axis,
                        }
                        .encode()?,
                    );
                }
                events.push(Ok(hit));
            } else {
                error = error.max(rigid_support::admit_pair_motion(
                    first, *shape, second, *obstacle, pair, &active, &models,
                )?);
            }
        }
    }
    Ok(physics::liquid::SupportedGeometryHit {
        event: nearest_events(events.into_iter())?,
        support_error_m: error,
    })
}
fn nearest_events(
    events: impl Iterator<Item = Result<RigidGeometryHit, physics::liquid::Error>>,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    let mut selected = RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear));
    let mut earliest = f64::INFINITY;
    for event in events {
        let event = event?;
        match event.contact.geometry {
            GeometryHit::Overlap => return Ok(event),
            GeometryHit::Contact { fraction, .. } if fraction < earliest => {
                earliest = fraction;
                selected = event;
            }
            _ => {}
        }
    }
    Ok(selected)
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
fn transport_acceleration(
    mut hit: BodyGeometryHit,
    acceleration: [f64; 3],
    duration: f64,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    if let (physics::liquid::GeometryHit::Contact { fraction, .. }, Some(witness)) =
        (hit.geometry, &mut hit.witness)
    {
        let time = duration * fraction;
        let offset = glam::DVec3::from_array(acceleration) * (0.5 * time) * time;
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

fn contact_patch_points(
    first: &[crate::convex::AffineBox],
    a: &physics::contact::ContactBody,
    second: &[crate::convex::AffineBox],
    b: &physics::contact::ContactBody,
    witness: ContactWitness,
    normal: [f64; 3],
    budget: usize,
) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
    contact_patch_points_with_error(first, a, second, b, witness, normal, budget, 0.)
}
#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn contact_patch_points_with_error(
    first: &[crate::convex::AffineBox],
    a: &physics::contact::ContactBody,
    second: &[crate::convex::AffineBox],
    b: &physics::contact::ContactBody,
    witness: ContactWitness,
    normal: [f64; 3],
    budget: usize,
    error_m: f64,
) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
    use glam::{DQuat, DVec3};
    use physics::liquid::Error;
    if budget == 0
        || first
            .len()
            .checked_mul(second.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    let qa = a
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = b
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let ca = DVec3::from_array(a.motion.position);
    let cb = DVec3::from_array(b.motion.position);
    let n = DVec3::from_array(normal);
    let mut contacts = Vec::new();
    for first in first {
        for second in second {
            let center = cb + qb * second.center;
            let obstacle = crate::convex::AffineBox {
                center,
                edges: second.edges.map(|e| qb * e),
            };
            let relative = crate::convex::AffineBox {
                center: ca - cb + qa * first.center - qb * second.center,
                edges: first.edges.map(|e| qa * e),
            };
            let w = DVec3::from_array(witness.point) - center;
            let scale = relative.center.length()
                + relative
                    .edges
                    .iter()
                    .chain(&obstacle.edges)
                    .map(|e| e.length())
                    .sum::<f64>();
            let tolerance = witness.tolerance_m
                + 256. * f64::EPSILON * (1. + scale + center.abs().max_element());
            let contains = |point: DVec3, shape: crate::convex::AffineBox| {
                let rows = glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2])
                    .inverse();
                let local = rows * (point - shape.center);
                rows.is_finite()
                    && (0..3).all(|k| {
                        local[k].abs() <= 1. + tolerance * rows.transpose().col(k).length()
                    })
            };
            let local_obstacle = crate::convex::AffineBox {
                center: DVec3::ZERO,
                ..obstacle
            };
            if !contains(w, local_obstacle)
                || !contains(w, relative)
                || (n.dot(w) - obstacle.radius(n)).abs() > tolerance
                || (n.dot(w - relative.center) + relative.radius(n)).abs() > tolerance
            {
                continue;
            }
            let gap = relative.center.dot(n) - obstacle.radius(n) - relative.radius(n);
            if gap.abs() > tolerance {
                continue;
            }
            let projected = crate::convex::AffineBox {
                center: relative.center - n * gap,
                ..relative
            };
            let patch = if error_m == 0. {
                obstacle.contact_patch_relative(&projected, n)
            } else {
                obstacle.contact_patch_relative_with_error(&projected, n, error_m)
            };
            let Ok((points, _)) = patch else {
                continue;
            };
            for point in points {
                if !contacts
                    .iter()
                    .any(|old: &physics::contact::NormalContact| {
                        (DVec3::from_array(old.point) - point).length() <= tolerance
                    })
                {
                    if contacts.len() >= 128 {
                        return Err(Error::CollisionBudget);
                    }
                    contacts.push(physics::contact::NormalContact {
                        point: point.to_array(),
                        normal,
                    });
                }
            }
        }
    }
    if contacts.is_empty() {
        Err(Error::InvalidCollision)
    } else {
        Ok(contacts)
    }
}

mod body_world;

mod runtime;

#[cfg(test)]
mod tests;

mod checkpoint;
pub use checkpoint::{SceneLiquidCheckpoint, SceneLiquidCheckpointLimits, SceneLiquidWireLimits};
