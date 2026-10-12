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
    sources: BTreeMap<NodeId, Source>,
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
    )
}
fn trajectory_events_counted(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
    steps: &mut usize,
    queries: &mut usize,
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
            let contact = crate::angular_sweep::sweep_nominal_rigid_contact(
                first, *shape, second, *obstacle, steps, queries,
            );
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
        let frame = sampling_frame(obstacle.shape.center.to_array(), body.duration())?;
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
fn supported_shape_events(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    supports: &[physics::liquid::RigidSupportPoint],
    point_models: Option<&[Option<physics::liquid::RigidSupportPointMotion>]>,
    budget: usize,
    fixed: bool,
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
                error = error.max(rigid_support::admit_point_motion(
                    first, *shape, second, *obstacle, fixed, &active, &models,
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
impl SceneBodyWorld {
    fn support_point_shapes(
        &self,
        bodies: &[physics::contact::ContactBody],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<
        (
            physics::contact::ContactBody,
            crate::convex::AffineBox,
            Option<physics::contact::ContactBody>,
            crate::convex::AffineBox,
        ),
        physics::liquid::Error,
    > {
        use physics::liquid::Error;
        let key = FeatureKey::decode(geometry.feature.ok_or(Error::CollisionBackend)?)?;
        let first = *bodies.get(support.first).ok_or(Error::InvalidCollision)?;
        let shape = *self
            .templates
            .get(support.first)
            .and_then(|shapes| shapes.get(key.first))
            .ok_or(Error::InvalidCollision)?;
        let (second, obstacle) = if let Some(index) = support.second {
            (
                Some(*bodies.get(index).ok_or(Error::InvalidCollision)?),
                *self
                    .templates
                    .get(index)
                    .and_then(|shapes| shapes.get(key.second))
                    .ok_or(Error::InvalidCollision)?,
            )
        } else {
            (
                None,
                self.environment
                    .0
                    .0
                    .get(key.second)
                    .ok_or(Error::InvalidCollision)?
                    .shape,
            )
        };
        Ok((first, shape, second, obstacle))
    }
}
impl physics::liquid::LiquidBodyWorld for SceneBodyWorld {
    fn rigid_support_point_velocity(
        &self,
        bodies: &[physics::contact::ContactBody],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<[f64; 3], physics::liquid::Error> {
        let (first, shape, second, obstacle) =
            self.support_point_shapes(bodies, support, geometry)?;
        rigid_support::point_velocity(
            first,
            shape,
            second,
            obstacle,
            support.support.contact.point,
            geometry.tolerance_m + geometry.admission_error_m,
        )
    }

    fn rigid_support_point_motion(
        &self,
        bodies: &[physics::contact::ContactBody],
        loads: &[physics::contact::ContactWrench],
        rates: &[physics::contact::ContactWrench],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<Option<physics::liquid::RigidSupportPointMotion>, physics::liquid::Error> {
        use physics::liquid::Error;
        let (first, shape, second, obstacle) =
            self.support_point_shapes(bodies, support, geometry)?;
        let p = support.support.contact.point;
        let tolerance = geometry.tolerance_m + geometry.admission_error_m;
        let velocity =
            match rigid_support::point_velocity(first, shape, second, obstacle, p, tolerance) {
                Ok(velocity) => velocity,
                // A clipped-vertex kink has no unique derivative. The existing
                // common-point model is still available, but must independently
                // prove whole-interval ownership; no arbitrary vertex is chosen.
                Err(Error::CollisionBackend) => return Ok(None),
                Err(error) => return Err(error),
            };
        let pair = |all: &[physics::contact::ContactWrench]| -> Result<_, Error> {
            Ok([
                *all.get(support.first).ok_or(Error::InvalidCollision)?,
                if let Some(index) = support.second {
                    *all.get(index).ok_or(Error::InvalidCollision)?
                } else {
                    Default::default()
                },
            ])
        };
        let acceleration = match rigid_support::point_acceleration(
            first,
            shape,
            second,
            obstacle,
            p,
            tolerance,
            pair(loads)?,
        ) {
            Ok(value) => value,
            Err(Error::CollisionBackend) => return Ok(None),
            Err(error) => return Err(error),
        };
        let jerk = match rigid_support::point_jerk(
            first,
            shape,
            second,
            obstacle,
            p,
            tolerance,
            pair(loads)?,
            pair(rates)?,
        ) {
            Ok(value) => value,
            Err(Error::CollisionBackend) => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(physics::liquid::RigidSupportPointMotion {
            velocity,
            acceleration,
            jerk,
        }))
    }

    fn rigid_support_normal_acceleration(
        &self,
        bodies: &[physics::contact::ContactBody],
        loads: &[physics::contact::ContactWrench],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<Option<[f64; 3]>, physics::liquid::Error> {
        let physics::contact::SupportPlane::Rate { normal_rate } = support.support.plane else {
            return Ok(None);
        };
        let key = FeatureKey::decode(
            geometry
                .feature
                .ok_or(physics::liquid::Error::CollisionBackend)?,
        )?;
        let first_shape = *self
            .templates
            .get(support.first)
            .and_then(|shapes| shapes.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let second_shape = if let Some(j) = support.second {
            *self
                .templates
                .get(j)
                .and_then(|shapes| shapes.get(key.second))
                .ok_or(physics::liquid::Error::InvalidCollision)?
        } else {
            self.environment
                .0
                .0
                .get(key.second)
                .ok_or(physics::liquid::Error::InvalidCollision)?
                .shape
        };
        rigid_support::edge_normal_acceleration(
            bodies[support.first],
            first_shape,
            support.second.map(|j| bodies[j]),
            second_shape,
            key.axis,
            support.support.contact.normal,
            normal_rate,
            loads[support.first],
            support.second.map_or(Default::default(), |j| loads[j]),
        )
        .map(Some)
    }
    fn rigid_pair_support_contacts(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_pair_support_contacts_with_error(i, first, j, second, budget, 0.)
    }
    fn rigid_environment_support_contacts(
        &self,
        i: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_environment_support_contacts_with_error(i, body, budget, 0.)
    }
    fn rigid_pair_support_contacts_with_error(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let a = self.templates.get(i).ok_or(Error::InvalidCollision)?;
        let b = self.templates.get(j).ok_or(Error::InvalidCollision)?;
        if i == j || budget == 0 || a.len().checked_mul(b.len()).is_none_or(|n| n > budget) {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in a.iter().enumerate() {
            for (ib, obstacle) in b.iter().enumerate() {
                for (witness, normal, axis) in rigid_support::snapshot_with_error(
                    *first,
                    *shape,
                    Some(*second),
                    *obstacle,
                    error_m,
                )? {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        first,
                        &[*obstacle],
                        second,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane = rigid_support::plane(
                        *first,
                        *shape,
                        Some(*second),
                        *obstacle,
                        axis,
                        normal,
                    )?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn rigid_environment_support_contacts_with_error(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let shapes = self.templates.get(index).ok_or(Error::InvalidCollision)?;
        let walls = &self.environment.0.0;
        if budget == 0
            || shapes
                .len()
                .checked_mul(walls.len())
                .is_none_or(|n| n > budget)
        {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in shapes.iter().enumerate() {
            for (ib, wall) in walls.iter().enumerate() {
                for (witness, normal, axis) in
                    rigid_support::snapshot_with_error(*body, *shape, None, wall.shape, error_m)?
                {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let fixed = physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            mass: 1.,
                            position: [0.; 3],
                            velocity: [0.; 3],
                        },
                        spin: None,
                    };
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        body,
                        &[wall.shape],
                        &fixed,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane =
                        rigid_support::plane(*body, *shape, None, wall.shape, axis, normal)?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn sweep_supported_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        supported_shape_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            supports,
            None,
            budget,
            false,
        )
    }
    fn sweep_supported_rigid_environment_event(
        &self,
        i: usize,
        body: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        let fixed = sampling_frame([0.; 3], body.duration())?;
        let walls: Vec<_> = self.environment.0.0.iter().map(|w| w.shape).collect();
        supported_shape_events(
            body,
            &self.templates[i],
            &fixed,
            &walls,
            supports,
            None,
            budget,
            true,
        )
    }
    fn sweep_supported_rigid_pair_point_motion(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        motion: &[Option<physics::liquid::RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        supported_shape_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            supports,
            Some(motion),
            budget,
            false,
        )
    }
    fn sweep_supported_rigid_environment_point_motion(
        &self,
        i: usize,
        body: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        motion: &[Option<physics::liquid::RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        let fixed = sampling_frame([0.; 3], body.duration())?;
        let walls: Vec<_> = self.environment.0.0.iter().map(|wall| wall.shape).collect();
        supported_shape_events(
            body,
            &self.templates[i],
            &fixed,
            &walls,
            supports,
            Some(motion),
            budget,
            true,
        )
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_pair_patch(i, first, j, second, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        contact_patch_points(&[*a], first, &[*b], second, witness, normal, budget)
    }
    fn rigid_environment_patch_for_feature(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_environment_patch(index, body, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let shape = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &[*shape],
            body,
            &[wall.shape],
            &fixed,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_pair_supports(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_pair_patch_for_feature(
            i,
            first,
            j,
            second,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*first, *a, Some(*second), *b, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }
    fn rigid_environment_supports(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_environment_patch_for_feature(
            index,
            body,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*body, *a, None, wall.shape, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }

    fn rigid_pair_patch(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        contact_patch_points(
            &self.templates[i],
            first,
            &self.templates[j],
            second,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_environment_patch(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        if budget == 0
            || self.templates[index]
                .len()
                .checked_mul(self.environment.0.0.len())
                .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        let obstacles: Vec<_> = self.environment.0.0.iter().map(|b| b.shape).collect();
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &self.templates[index],
            body,
            &obstacles,
            &fixed,
            witness,
            normal,
            budget,
        )
    }

    fn sweep_particle_rigid_contact(
        &self,
        p: &physics::liquid::Particle,
        r: f64,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_particle_rigid_event(p, r, i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        a: &physics::rigid_motion::RigidMotion,
        j: usize,
        b: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_pair_event(i, a, j, b, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_environment_contact(
        &self,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_environment_event(i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_particle_motion_body_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() || particle.duration() != body.duration() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if body.has_constant_acceleration()
            && particle.has_constant_acceleration()
            && body.initial().spin.is_none()
            && particle.acceleration() == body.acceleration()
        {
            let state = body.initial().motion;
            let hit = self.sweep_particle_body_contact(
                p,
                radius,
                index,
                &physics::liquid::TranslatingBody {
                    mass: state.mass,
                    position: state.position,
                    velocity: state.velocity,
                },
                body.duration(),
                budget,
            )?;
            return transport_acceleration(hit, body.acceleration(), body.duration())
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_events(particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_particle_motion_environment_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if particle.has_constant_acceleration() && particle.acceleration() == [0.; 3] {
            return self
                .sweep_particle_environment_contact(p, radius, particle.duration(), budget)
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_environment_events(particle, &[shape], &self.environment, budget)
    }
    fn sweep_particle_rigid_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_particle_body_contact(
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
                )
                .map(Into::into);
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
        trajectory_events(&particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if first.has_constant_acceleration()
            && second.has_constant_acceleration()
            && first.initial().spin.is_none()
            && second.initial().spin.is_none()
            && first.acceleration() == [0.; 3]
            && second.acceleration() == [0.; 3]
        {
            let a = first.initial().motion;
            let b = second.initial().motion;
            return self
                .sweep_body_pair_contact(
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
                )
                .map(Into::into);
        }
        trajectory_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            budget,
        )
    }
    fn sweep_rigid_environment_event(
        &self,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_body_environment_contact(
                    index,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    body.duration(),
                    budget,
                )
                .map(Into::into);
        }
        trajectory_environment_events(body, &self.templates[index], &self.environment, budget)
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

fn scene_rotation_config() -> physics::spin_path::Config {
    physics::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
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
    ) -> Result<Self, GameplayError> {
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
        for (node, _) in scene.components::<crate::RigidPointLoads>() {
            if scene
                .component::<crate::LiquidBody>(node)
                .map_err(|e| format!("point load owner: {e:?}"))?
                .is_none()
            {
                return Err("point loads require a liquid body owner".into());
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
                    return Err("invalid liquid body mass/velocity".into());
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
                let pivot = position(scene, *node)?;
                let state = if let Some(frame) = rigid_frame {
                    frame
                        .prepare_body(pivot, descriptor.initial_velocity_m_s, [0.; 3])
                        .map_err(|e| format!("rigid body seed: {e:?}"))?
                } else {
                    physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            position: pivot,
                            velocity: descriptor.initial_velocity_m_s,
                            mass: descriptor.mass_kg,
                        },
                        spin: None,
                    }
                };
                let point_loads = scene
                    .component::<crate::RigidPointLoads>(*node)
                    .map_err(|e| format!("point load descriptor: {e:?}"))?
                    .cloned();
                if let Some(loads) = &point_loads {
                    loads.prepare(
                        rigid_frame.ok_or("point loads require an explicit rigid mass frame")?,
                        0.,
                        128,
                    )?;
                }
                let mut owner = BodyOwner {
                    node: *node,
                    descriptor: **descriptor,
                    state,
                    principal_templates: None,
                    published,
                    colliders,
                    mass_descriptor,
                    mass_properties,
                    rigid_frame,
                    point_loads,
                    point_elapsed_s: 0.,
                };
                if let (Some(properties), Some(spin)) = (owner.mass_properties, owner.state.spin) {
                    let inverse = glam::DQuat::from_array(spin.orientation).conjugate();
                    owner.principal_templates = Some(
                        owner
                            .colliders
                            .iter()
                            .map(|collider| {
                                let shape = collider_template(&owner, collider)?;
                                Ok(crate::convex::AffineBox {
                                    center: inverse
                                        * (shape.center
                                            - glam::DVec3::from_array(properties.center)),
                                    edges: shape.edges.map(|edge| inverse * edge),
                                })
                            })
                            .collect::<Result<Vec<_>, GameplayError>>()?,
                    );
                }
                Ok(owner)
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
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
    pub fn validate_bindings(&self, scene: &SceneGraph) -> Result<(), GameplayError> {
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
        let point_descriptors: BTreeMap<_, _> =
            scene.components::<crate::RigidPointLoads>().collect();
        if point_descriptors.len()
            != self
                .body
                .iter()
                .filter(|body| body.point_loads.is_some())
                .count()
            || self.body.iter().any(|body| {
                point_descriptors.get(&body.node).map(|v| &**v) != body.point_loads.as_ref()
            })
        {
            return Err("point load ownership or descriptor changed; rebind runtime".into());
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
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_and_publish_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Prepare and atomically publish with explicit restitution/contact and angular budgets.
    /// The normal angular solver currently requires zero tangential friction.
    /// # Errors
    /// Invalid settings, unresolved contacts or publication errors preserve both owners and scene.
    pub fn tick_and_publish_with_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, GameplayError> {
        let mut candidate = self.clone();
        let report = candidate.tick_with_dynamics(scene, dt, container, dynamics, rotation)?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    /// Publish an already prepared body pose after the host's other preparation succeeds.
    /// # Errors
    /// Foreign/edited scene bindings or unrepresentable pose reject before mutation.
    pub fn publish_body_pose(&mut self, scene: &mut SceneGraph) -> Result<(), GameplayError> {
        self.validate_bindings(scene)?;
        let poses = self
            .body
            .iter()
            .map(|body| {
                let mut pose = body.published;
                if let Some(frame) = body.rigid_frame {
                    let radius = body
                        .principal_templates
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|s| {
                            s.center.length() + s.edges.iter().map(|e| e.length()).sum::<f64>()
                        })
                        .fold(0., f64::max);
                    pose = frame
                        .prepare_pose(body.state, radius, 1e-5)
                        .map_err(|e| format!("rigid pose publication: {e:?}"))?
                        .pose;
                } else {
                    pose.translation =
                        glam::DVec3::from_array(body.state.motion.position).as_vec3();
                }
                pose.matrix()
                    .map_err(|e| format!("liquid body publication: {e:?}"))?;
                Ok((body.node, pose))
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        scene
            .set_locals(&poses)
            .map_err(|e| format!("liquid body publication: {e:?}"))?;
        for (body, (_, pose)) in self.body.iter_mut().zip(poses) {
            body.published = pose;
        }
        Ok(())
    }
    /// First admitted body for compatibility; use `body_states` for the entire world.
    #[must_use]
    pub fn body_state(&self) -> Option<(NodeId, physics::liquid::TranslatingBody)> {
        self.body_states().next()
    }

    /// All admitted COM states (legacy bodies use the root pivot), including paused
    /// inactive bodies, in stable owner order. Rotation is exposed by `body_rigid_states`.
    pub fn body_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::liquid::TranslatingBody)> + '_ {
        self.body.iter().map(|b| {
            (
                b.node,
                physics::liquid::TranslatingBody {
                    position: b.state.motion.position,
                    velocity: b.state.motion.velocity,
                    mass: b.state.motion.mass,
                },
            )
        })
    }

    /// Admitted mass tensor and COM offset in the root-relative world-oriented frame.
    /// Legacy bodies without an explicit distribution have no inferred inertia.
    pub fn body_mass_properties(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<physics::mass_properties::MassProperties>)> + '_ {
        self.body.iter().map(|b| (b.node, b.mass_properties))
    }

    /// Persistent mechanical COM states, including intrinsic rotation when mass is authored.
    pub fn body_rigid_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::contact::ContactBody)> + '_ {
        self.body.iter().map(|b| (b.node, b.state))
    }

    /// Admitted COM/principal adapters.
    pub fn body_rigid_frames(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<crate::RigidBodyFrame>)> + '_ {
        self.body.iter().map(|b| (b.node, b.rigid_frame))
    }

    /// Build the active rigid geometry world once for both dynamics and support queries.
    fn mechanical_world(
        &self,
        scene: &SceneGraph,
    ) -> Result<
        (
            SceneBodyWorld,
            Vec<usize>,
            Vec<physics::contact::ContactBody>,
        ),
        String,
    > {
        let mut geometry = SceneGeometry(
            crate::static_world(scene).map_err(|e| format!("liquid collider: {e:?}"))?,
        );
        let mut active = Vec::new();
        let mut templates = Vec::new();
        let mut states = Vec::new();
        for (owner, body) in self.body.iter().enumerate() {
            if !scene
                .active_in_hierarchy(body.node)
                .map_err(|e| format!("liquid body activity: {e:?}"))?
            {
                continue;
            }
            let mut shapes = Vec::new();
            for (slot, collider) in body.colliders.iter().enumerate() {
                if let Some(index) = geometry.0.0.iter().position(|b| b.owner == collider.node) {
                    geometry.0.0.remove(index);
                    shapes.push(if let Some(principal) = &body.principal_templates {
                        principal[slot]
                    } else {
                        collider_template(body, collider).map_err(|e| e.to_string())?
                    });
                }
            }
            active.push(owner);
            templates.push(shapes);
            states.push(body.state);
        }
        Ok((
            SceneBodyWorld {
                environment: geometry,
                templates,
            },
            active,
            states,
        ))
    }

    /// Gather current supporting patches and resolve coupled reactions under
    /// configured gravity plus additional COM loads. This is read-only and does
    /// not certify or advance a constrained trajectory. Returned indices follow owners.
    /// # Errors
    /// Invalid/edited scene bindings, no active bodies, duplicate/foreign/inactive
    /// load owners, geometry overlap, limits or reaction nonconvergence.
    pub fn support_reactions(
        &self,
        scene: &SceneGraph,
        additional: &[(NodeId, physics::contact::ContactWrench)],
        limits: physics::liquid::DynamicWorldConfig,
        config: physics::contact::ReactionConfig,
    ) -> Result<SceneSupportReactions, GameplayError> {
        self.validate_bindings(scene)?;
        let (world, active, states) = self.mechanical_world(scene)?;
        let owners: Vec<_> = active.iter().map(|i| self.body[*i].node).collect();
        let mut wrenches = vec![physics::contact::ContactWrench::default(); owners.len()];
        let mut used = Vec::new();
        for (node, wrench) in additional {
            let index = owners
                .iter()
                .position(|owner| owner == node)
                .ok_or_else(|| "support load has a foreign or inactive owner".to_string())?;
            if used.contains(&index) {
                return Err("duplicate support load owner".into());
            }
            used.push(index);
            wrenches[index] = *wrench;
        }
        for (index, owner_index) in active.iter().enumerate() {
            let owner = &self.body[*owner_index];
            if let Some(loads) = &owner.point_loads {
                let points = loads.prepare(
                    owner.rigid_frame.ok_or("point load mass frame missing")?,
                    owner.point_elapsed_s,
                    128,
                )?;
                let moment = physics::rigid_motion::MaterialForceMoment::aggregate(points)
                    .map_err(|e| format!("point load resultant: {e:?}"))?;
                let orientation = states[index]
                    .spin
                    .ok_or("point loads require intrinsic rotation")?
                    .orientation;
                let torque = moment
                    .torque_at(orientation, 0.)
                    .map_err(|e| format!("point load moment: {e:?}"))?;
                let old = wrenches[index];
                let total = physics::rigid_motion::MotionLoad::aggregate([
                    physics::rigid_motion::MotionLoad {
                        force: old.force,
                        torque: physics::astrophysics_spin::TorquePolynomial::constant(old.torque),
                        ..physics::rigid_motion::MotionLoad::zero()
                    },
                    physics::rigid_motion::MotionLoad {
                        force: moment.force,
                        torque: physics::astrophysics_spin::TorquePolynomial::constant(torque),
                        ..physics::rigid_motion::MotionLoad::zero()
                    },
                ])
                .map_err(|e| format!("support load aggregation: {e:?}"))?;
                wrenches[index] = physics::contact::ContactWrench {
                    force: total.force,
                    torque: total.torque.value,
                };
            }
        }
        let reactions = self
            .liquid
            .rigid_world_reactions(&states, &world, &wrenches, limits, config)
            .map_err(|e| format!("scene support reactions: {e:?}"))?;
        Ok(SceneSupportReactions { owners, reactions })
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
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Simulate using caller-specified mechanical response and angular admission budgets.
    /// # Errors
    /// Same transactional binding, emission and geometry guarantees as tick.
    pub fn tick_with_dynamics(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, GameplayError> {
        self.tick_impl(scene, dt, container, dynamics, rotation, None)
            .map(|report| report.step)
    }

    /// Advance and publish through the existing scene owner/event loop with
    /// geometry-admitted finite normal reactions. Unsupported changing branches
    /// reject with complete runtime/scene rollback, including emitted sources.
    /// # Errors
    /// Invalid budgets/bindings, unresolved support evolution or pose publication.
    pub fn tick_and_publish_with_supported_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: physics::liquid::SupportedWorldConfig,
    ) -> Result<SceneSupportedLiquidStep, GameplayError> {
        supports
            .validate()
            .map_err(|e| format!("supported liquid config: {e:?}"))?;
        let mut candidate = self.clone();
        let report =
            candidate.tick_impl(scene, dt, container, dynamics, rotation, Some(supports))?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    fn tick_impl(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: Option<physics::liquid::SupportedWorldConfig>,
    ) -> Result<SceneSupportedLiquidStep, GameplayError> {
        self.validate_bindings(scene)?;
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid liquid fixed interval".into());
        }
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
        let (world, active, mut states) = candidate.mechanical_world(scene)?;
        let point_loads = active
            .iter()
            .map(|index| {
                let owner = &candidate.body[*index];
                owner.point_loads.as_ref().map_or(Ok(Vec::new()), |loads| {
                    loads.prepare(
                        owner.rigid_frame.ok_or("point load mass frame missing")?,
                        owner.point_elapsed_s,
                        128,
                    )
                })
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        let has_points = point_loads.iter().any(|points| !points.is_empty());
        let mut support_report = None;
        let (physics, dynamics) = if states.is_empty() {
            (
                candidate
                    .liquid
                    .step_with_geometry(dt, container, &world.environment, dynamics.contact)
                    .map_err(|e| format!("scene liquid step: {e:?}"))?,
                None,
            )
        } else {
            let report = if let Some(config) = supports {
                let loads = vec![physics::contact::ContactWrench::default(); states.len()];
                let report = candidate
                    .liquid
                    .step_with_supported_rigid_body_point_forces(
                        dt,
                        &mut states,
                        &world,
                        dynamics,
                        128,
                        rotation,
                        &loads,
                        &point_loads,
                        config,
                    )
                    .map_err(|e| format!("scene supported liquid body step: {e:?}"))?;
                support_report = Some(report);
                report.rigid.world
            } else if has_points {
                let additional = vec![physics::contact::ContactWrench::default(); states.len()];
                candidate
                    .liquid
                    .step_with_rigid_body_point_forces(
                        dt,
                        &mut states,
                        &world,
                        dynamics,
                        128,
                        rotation,
                        &additional,
                        &point_loads,
                    )
                    .map_err(|e| format!("scene point load step: {e:?}"))?
                    .world
            } else {
                candidate
                    .liquid
                    .step_with_rigid_body_world(dt, &mut states, &world, dynamics, 128, rotation)
                    .map_err(|e| format!("scene liquid body step: {e:?}"))?
            };
            for (owner, state) in active.into_iter().zip(states) {
                candidate.body[owner].state = state;
                if candidate.body[owner].point_loads.is_some() {
                    candidate.body[owner].point_elapsed_s += dt;
                    if !candidate.body[owner].point_elapsed_s.is_finite() {
                        return Err("point load clock overflow".into());
                    }
                }
            }
            (report.dynamics.fluid, Some(report))
        };
        *self = candidate;
        Ok(SceneSupportedLiquidStep {
            step: SceneLiquidStep {
                emissions,
                physics,
                dynamics,
            },
            support: support_report,
        })
    }
}

#[cfg(test)]
mod tests;
