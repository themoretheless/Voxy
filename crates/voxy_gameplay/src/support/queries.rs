//! Borrowed read-only scene queries over the existing affine collision boxes.
use super::*;
use crate::{CharacterPhysics, StaticBox, affine_box, precise_center};

#[derive(Clone, Copy, Debug)]
pub struct RayQuery {
    pub origin: DVec3,
    /// Unit world-space direction; max_distance is measured in world units.
    pub direction: DVec3,
    pub max_distance: f64,
    /// Inside a solid returns distance zero; otherwise return its exit surface.
    pub solid: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub owner: NodeId,
    pub position: DVec3,
    /// None only for a solid interior hit, which has no unique surface normal.
    pub normal: Option<DVec3>,
    pub distance: f64,
    pub started_inside: bool,
}
/// Closed interval of contact with a collider during a translation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxSweepHit {
    pub owner: NodeId,
    pub enter: f64,
    pub exit: f64,
    /// Unit world-space normal pointing out of the target at entry.
    /// An initial overlap or contact with no entering axis has no normal.
    /// Tied entry axes select the first axis in the fixed SAT order.
    pub normal: Option<DVec3>,
}

/// First reported contact while rotating a box about its center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngularBoxHit {
    pub owner: NodeId,
    pub fraction: f64,
    /// World-space separating axis, when the advancement solver supplies one.
    pub normal: Option<DVec3>,
}

/// Borrows both source owners, so neither scene nor accepted physical state can
/// change while this snapshot is used. No second solver or collision store.
#[derive(Debug)]
pub struct SceneQueries<'a> {
    scene: &'a SceneGraph,
    physics: Option<&'a CharacterPhysics>,
    world: SupportWorld,
}
fn sweep_hit_order(a: &BoxSweepHit, b: &BoxSweepHit) -> std::cmp::Ordering {
    a.enter.total_cmp(&b.enter).then(a.owner.cmp(&b.owner))
}

fn surface_box(surface: &Surface) -> crate::convex::AffineBox {
    crate::convex::AffineBox {
        center: surface.center,
        edges: [
            surface.edges.x_axis,
            surface.edges.y_axis,
            surface.edges.z_axis,
        ],
    }
}

impl CharacterPhysics {
    /// Include active characters at accepted f64 poses and active static boxes.
    /// Unstepped or externally edited characters use their authored affine pose.
    /// # Errors
    /// Rejects foreign scenes, malformed geometry and collider capacity limits.
    pub fn queries<'a>(
        &'a self,
        scene: &'a SceneGraph,
        capacity: usize,
    ) -> Result<SceneQueries<'a>, PhysicsError> {
        SceneQueries::build(scene, Some(self), capacity)
    }
}
impl<'a> SceneQueries<'a> {
    /// Builds a layer predicate for this snapshot's ray, overlap and sweep queries.
    pub fn layer_filter(&self, layers: crate::CollisionLayers) -> impl Fn(NodeId) -> bool + '_ {
        move |owner| layers.interacts(crate::CollisionLayers::on(self.scene, owner))
    }

    pub(crate) fn source_scene(&self) -> &'a SceneGraph {
        self.scene
    }
    /// Query the published scene geometry without requiring a simulation owner.
    /// For accepted solver precision use CharacterPhysics::queries instead.
    /// The source cannot be mutated while its query snapshot is still used:
    /// ```compile_fail
    /// use voxy_gameplay::{RayQuery, SceneQueries, SupportQueryBudget};
    /// use voxy_scene::SceneGraph;
    /// use glam::DVec3;
    /// let mut scene = SceneGraph::new(1);
    /// let queries = SceneQueries::from_scene(&scene, 1).unwrap();
    /// scene.spawn(None, Default::default()).unwrap();
    /// queries.cast_ray(RayQuery { origin: DVec3::ZERO, direction: DVec3::X,
    ///     max_distance: 1., solid: true }, &mut SupportQueryBudget::new(1).unwrap(), |_| true);
    /// ```
    /// # Errors
    /// Rejects malformed geometry or a collider count above the caller budget.
    pub fn from_scene(scene: &'a SceneGraph, capacity: usize) -> Result<Self, PhysicsError> {
        Self::build(scene, None, capacity)
    }
    fn build(
        scene: &'a SceneGraph,
        physics: Option<&'a CharacterPhysics>,
        capacity: usize,
    ) -> Result<Self, PhysicsError> {
        if capacity > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        if let Some(physics) = physics {
            physics.validate(scene)?;
        } else {
            CharacterPhysics::new(scene, capacity, capacity).validate(scene)?;
        }
        let count = scene
            .active_components::<BoxCollider>()
            .count()
            .checked_add(scene.active_components::<crate::CharacterBody>().count())
            .ok_or(PhysicsError::Capacity)?;
        if count > capacity {
            return Err(PhysicsError::Capacity);
        }
        let mut world = static_world(scene)?;
        for (owner, body) in scene.active_components::<crate::CharacterBody>() {
            let shape = if let Some(physics) = physics
                && physics.accepted_pose(scene, owner)?.is_some()
            {
                let runtime = &physics.states[&owner];
                crate::convex::AffineBox {
                    center: precise_center(runtime.state.body),
                    edges: runtime.edges,
                }
            } else {
                affine_box(scene, owner, body.half_extents)?
            };
            let extent = shape.edges.iter().map(|edge| edge.abs()).sum::<DVec3>();
            world.0.push(StaticBox {
                owner,
                layers: crate::CollisionLayers::on(scene, owner),
                half_extents: body.half_extents,
                shape,
                min: (shape.center - extent).to_array(),
                max: (shape.center + extent).to_array(),
            });
        }
        Ok(Self {
            scene,
            physics,
            world: SupportWorld::from_static_world(&world)?,
        })
    }
    /// Query a box using the node's published affine basis and, when available,
    /// its accepted solver center. The node need not
    /// own a collider. Results use stable owner order and include touching faces.
    /// Targets use the same geometry (including accepted solver poses) as rays.
    /// # Errors
    /// Rejects invalid source nodes or geometry, hit capacity and query budgets.
    /// Failure returns no partial result.
    pub fn overlap_box(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        budget: &mut SupportQueryBudget,
        max_hits: usize,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Vec<NodeId>, PhysicsError> {
        if max_hits > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        let query = self.query_box(owner, half_extents)?;
        let mut hits = Vec::with_capacity(max_hits.min(self.world.surfaces.len()));
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            let target = surface_box(surface);
            let offset = query.center - target.center;
            if target
                .axes_for(query.edges)
                .all(|axis| offset.dot(axis).abs() <= target.radius(axis) + query.radius(axis))
            {
                if hits.len() == max_hits {
                    return Err(PhysicsError::Capacity);
                }
                hits.push(surface.owner);
            }
        }
        Ok(hits)
    }
    /// Finds closed contact intervals for a box translated over one step.
    /// Fractions are in [0, 1]. Colliders are held at their snapshot pose.
    /// Results are ordered by entry fraction, then by owner.
    /// # Errors
    /// Rejects invalid geometry, nonfinite displacement, and exceeded budgets.
    /// Failure returns no partial result.
    pub fn sweep_box(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        displacement: DVec3,
        budget: &mut SupportQueryBudget,
        max_hits: usize,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Vec<BoxSweepHit>, PhysicsError> {
        self.sweep_box_relative(
            owner,
            half_extents,
            displacement,
            budget,
            max_hits,
            |target| accept(target).then_some(DVec3::ZERO),
        )
    }

    /// Sweeps a box against independently translating targets, all starting at
    /// this snapshot and moving over the same normalized interval. The callback
    /// returns each target's displacement, or None to exclude it.
    /// # Errors
    /// Rejects nonfinite relative motion, invalid geometry and exhausted budgets.
    /// No partial results are returned.
    pub fn sweep_box_relative(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        displacement: DVec3,
        budget: &mut SupportQueryBudget,
        max_hits: usize,
        target_displacement: impl Fn(NodeId) -> Option<DVec3>,
    ) -> Result<Vec<BoxSweepHit>, PhysicsError> {
        if max_hits > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        let mut hits = Vec::with_capacity(max_hits.min(self.world.surfaces.len()));
        self.visit_box_sweep(
            owner,
            half_extents,
            displacement,
            budget,
            target_displacement,
            |hit| {
                if hits.len() == max_hits {
                    return Err(PhysicsError::Capacity);
                }
                hits.push(hit);
                Ok(())
            },
        )?;
        hits.sort_unstable_by(sweep_hit_order);
        Ok(hits)
    }

    /// Finds the earliest contact with a collider held at its snapshot pose.
    /// Equal entry fractions are ordered by owner. Uses constant result storage.
    /// # Errors
    /// Rejects invalid geometry, nonfinite motion and exhausted query budgets.
    pub fn cast_box(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        displacement: DVec3,
        budget: &mut SupportQueryBudget,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Option<BoxSweepHit>, PhysicsError> {
        self.cast_box_relative(owner, half_extents, displacement, budget, |target| {
            accept(target).then_some(DVec3::ZERO)
        })
    }

    /// Finds the earliest contact over a shared interval of relative translation.
    /// Each scanned collider consumes budget, including rejected candidates.
    /// # Errors
    /// Rejects invalid geometry, nonfinite motion and exhausted query budgets.
    /// An error discards any hit found before it.
    pub fn cast_box_relative(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        displacement: DVec3,
        budget: &mut SupportQueryBudget,
        target_displacement: impl Fn(NodeId) -> Option<DVec3>,
    ) -> Result<Option<BoxSweepHit>, PhysicsError> {
        let mut best = None;
        self.visit_box_sweep(
            owner,
            half_extents,
            displacement,
            budget,
            target_displacement,
            |hit| {
                if best
                    .as_ref()
                    .is_none_or(|previous| sweep_hit_order(&hit, previous).is_lt())
                {
                    best = Some(hit);
                }
                Ok(())
            },
        )?;
        Ok(best)
    }

    fn visit_box_sweep(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        displacement: DVec3,
        budget: &mut SupportQueryBudget,
        target_displacement: impl Fn(NodeId) -> Option<DVec3>,
        mut visit: impl FnMut(BoxSweepHit) -> Result<(), PhysicsError>,
    ) -> Result<(), PhysicsError> {
        if !displacement.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let query = self.query_box(owner, half_extents)?;
        for surface in &self.world.surfaces {
            budget.spend()?;
            let Some(target_motion) = target_displacement(surface.owner) else {
                continue;
            };
            let relative = displacement - target_motion;
            if !relative.is_finite() {
                return Err(PhysicsError::InvalidMotion);
            }
            let target = surface_box(surface);
            if let Some(contact) = target.translation_interval(&query, relative)? {
                visit(BoxSweepHit {
                    owner: surface.owner,
                    enter: contact.enter,
                    exit: contact.exit,
                    normal: contact.normal,
                })?;
            }
        }
        Ok(())
    }

    /// Casts a fixed-center box along a world-space scaled-axis rotation in radians.
    /// Uses the controller's angular advancement and contact tolerance. Initial
    /// and final overlap are checked directly. Equal fractions use owner order.
    /// # Errors
    /// Rejects invalid geometry, rotation exceeding four turns, and exhausted
    /// scan or advancement budgets. A failure discards any earlier hit.
    pub fn cast_box_rotation(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
        angular: DVec3,
        budget: &mut SupportQueryBudget,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Option<AngularBoxHit>, PhysicsError> {
        if !angular.is_finite() || angular.length() > 4. * std::f64::consts::TAU {
            return Err(PhysicsError::InvalidMotion);
        }
        let query = self.query_box(owner, half_extents)?;
        let rotation = glam::DQuat::from_scaled_axis(angular);
        let end = crate::convex::AffineBox {
            edges: query
                .edges
                .map(|edge| crate::convex::rotate_vector(rotation, edge)),
            ..query
        };
        let mut best: Option<AngularBoxHit> = None;
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            let target = surface_box(surface);
            let (fraction, normal) = if target.translation_interval(&query, DVec3::ZERO)?.is_some()
            {
                (0., None)
            } else {
                let limit = budget.remaining();
                let mut remaining = limit;
                let result = crate::angular_sweep::sweep_with_query_limit(
                    query.center,
                    query.edges,
                    angular,
                    std::slice::from_ref(&target),
                    256,
                    &mut remaining,
                );
                budget.charge_work(limit - remaining)?;
                let hit = result?;
                if hit.normal.is_some() {
                    (hit.fraction, hit.normal)
                } else if target.translation_interval(&end, DVec3::ZERO)?.is_some() {
                    (1., None)
                } else {
                    continue;
                }
            };
            if best.is_none_or(|previous| {
                (fraction, surface.owner) < (previous.fraction, previous.owner)
            }) {
                best = Some(AngularBoxHit {
                    owner: surface.owner,
                    fraction,
                    normal,
                });
            }
        }
        Ok(best)
    }

    pub(crate) fn query_box(
        &self,
        owner: NodeId,
        half_extents: [f32; 3],
    ) -> Result<crate::convex::AffineBox, PhysicsError> {
        validate_extents(half_extents)?;
        let mut query = affine_box(self.scene, owner, half_extents)?;
        if let Some(physics) = self.physics {
            let mut offset = DVec3::ZERO;
            let mut node = owner;
            loop {
                if let Some(pose) = physics.accepted_pose(self.scene, node)? {
                    // Compose the local offset before adding the large world
                    // origin. Subtracting two published f32 world centers
                    // cannot recover the low bits of a small child offset.
                    query.center = pose.physical_center
                        + pose.world_matrix.as_dmat4().transform_vector3(offset);
                    let extent = query.edges.iter().map(|edge| edge.abs()).sum::<DVec3>();
                    if (query.center.abs() + extent).max_element() > 1e6 {
                        return Err(PhysicsError::CoordinateRange);
                    }
                    break;
                }
                let Some(parent) = self.scene.parent(node)? else {
                    break;
                };
                offset = self
                    .scene
                    .local(node)?
                    .matrix()?
                    .as_dmat4()
                    .transform_point3(offset);
                node = parent;
            }
        }
        Ok(query)
    }

    pub(crate) fn collider_boxes(
        &self,
    ) -> impl Iterator<Item = (NodeId, crate::convex::AffineBox)> + '_ {
        self.world
            .surfaces
            .iter()
            .map(|surface| (surface.owner, surface_box(surface)))
    }

    /// Nearest hit with deterministic NodeId ties. Predicate excludes owners
    /// before intersection; every scanned collider consumes the shared budget.
    /// # Errors
    /// Rejects invalid rays, exhausted budgets and unstable geometric evaluation.
    /// No partial hit is returned on any failure.
    pub fn cast_ray(
        &self,
        ray: RayQuery,
        budget: &mut SupportQueryBudget,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Option<RayHit>, PhysicsError> {
        validate_ray(ray)?;
        let mut best: Option<RayHit> = None;
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            if let Some(hit) = intersect(surface, ray)?
                && best.is_none_or(|best| (hit.distance, hit.owner) < (best.distance, best.owner))
            {
                best = Some(hit);
            }
        }
        Ok(best)
    }
    /// All hits sorted by distance then stable owner. Capacity exhaustion rejects
    /// the whole query instead of truncating the result or publishing callbacks.
    /// # Errors
    /// Rejects invalid rays, hit/query budgets and unstable intersections.
    pub fn intersect_ray(
        &self,
        ray: RayQuery,
        budget: &mut SupportQueryBudget,
        max_hits: usize,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Vec<RayHit>, PhysicsError> {
        validate_ray(ray)?;
        if max_hits > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        let mut hits = Vec::with_capacity(max_hits.min(self.world.surfaces.len()));
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            if let Some(hit) = intersect(surface, ray)? {
                if hits.len() == max_hits {
                    return Err(PhysicsError::Capacity);
                }
                hits.push(hit);
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then(a.owner.cmp(&b.owner))
        });
        Ok(hits)
    }
}
fn validate_ray(ray: RayQuery) -> Result<(), PhysicsError> {
    if !ray.origin.is_finite()
        || ray.origin.abs().max_element() > 1e6
        || !ray.direction.is_finite()
        || (ray.direction.length_squared() - 1.).abs() > 1e-12
        || !ray.max_distance.is_finite()
        || !(0. ..=1e6).contains(&ray.max_distance)
    {
        return Err(PhysicsError::InvalidMotion);
    }
    Ok(())
}
fn intersect(surface: &Surface, ray: RayQuery) -> Result<Option<RayHit>, PhysicsError> {
    let p = surface.inverse * (ray.origin - surface.center);
    let v = surface.inverse * ray.direction;
    if !p.is_finite() || !v.is_finite() {
        return Err(PhysicsError::Solver);
    }
    let inside = p.abs().max_element() < 1.;
    if inside && ray.solid {
        return Ok(Some(RayHit {
            owner: surface.owner,
            position: ray.origin,
            normal: None,
            distance: 0.,
            started_inside: true,
        }));
    }
    let (mut enter, mut exit) = (f64::NEG_INFINITY, f64::INFINITY);
    let (mut near_face, mut far_face) = ((0, 0.), (0, 0.));
    for axis in 0..3 {
        if v[axis] == 0. {
            if p[axis].abs() > 1. {
                return Ok(None);
            }
            continue;
        }
        let (mut near, mut far) = ((-1. - p[axis]) / v[axis], (1. - p[axis]) / v[axis]);
        let mut sign = -1.;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            sign = 1.;
        }
        if near > enter {
            enter = near;
            near_face = (axis, sign);
        }
        if far < exit {
            exit = far;
            far_face = (axis, -sign);
        }
        if enter > exit {
            return Ok(None);
        }
    }
    let (distance, (face, sign)) = if enter >= 0. {
        (enter, near_face)
    } else {
        (exit, far_face)
    };
    let distance = if distance == 0. { 0. } else { distance };
    if !distance.is_finite() || distance < 0. || distance > ray.max_distance || sign == 0. {
        return Ok(None);
    }
    let mut local = p + v * distance;
    local[face] = sign;
    let guard =
        128. * f64::EPSILON * (1. + p.abs().max_element() + v.abs().max_element() * distance);
    if !local.is_finite() || local.abs().max_element() > 1. + guard {
        return Err(PhysicsError::Solver);
    }
    local = local.clamp(DVec3::NEG_ONE, DVec3::ONE);
    let position = surface.center + surface.edges * local;
    let on_ray = ray.origin + ray.direction * distance;
    let tolerance =
        256. * f64::EPSILON * (1. + ray.origin.abs().max_element() + position.abs().max_element());
    if !position.is_finite() || !position.abs_diff_eq(on_ray, tolerance) {
        return Err(PhysicsError::Solver);
    }
    Ok(Some(RayHit {
        owner: surface.owner,
        position,
        normal: Some(surface.normals[face] * sign),
        distance,
        started_inside: inside,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};
    use voxy_scene::Transform;
    fn ray(origin: DVec3, direction: DVec3) -> RayQuery {
        RayQuery {
            origin,
            direction,
            max_distance: 10.,
            solid: true,
        }
    }
    fn budget() -> SupportQueryBudget {
        SupportQueryBudget::new(64).unwrap()
    }
    fn box_at(scene: &mut SceneGraph, x: f32) -> NodeId {
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * x,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                BoxCollider {
                    half_extents: [0.5; 3],
                },
            )
            .unwrap();
        owner
    }
    #[test]
    fn box_overlap_distinguishes_affine_separation_and_has_atomic_limits() {
        let mut scene = SceneGraph::new(4);
        let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let zone = scene
            .spawn(
                None,
                Transform {
                    rotation,
                    ..Default::default()
                },
            )
            .unwrap();
        let separated = scene
            .spawn(
                None,
                Transform {
                    rotation,
                    translation: rotation * (Vec3::Y * 0.5),
                    ..Default::default()
                },
            )
            .unwrap();
        let contact = scene
            .spawn(
                None,
                Transform {
                    rotation,
                    ..Default::default()
                },
            )
            .unwrap();
        for id in [separated, contact] {
            scene
                .insert_component(
                    id,
                    BoxCollider {
                        half_extents: [2., 0.1, 0.1],
                    },
                )
                .unwrap();
        }
        let inactive = box_at(&mut scene, 0.);
        scene.set_active(inactive, false).unwrap();
        let q = SceneQueries::from_scene(&scene, 4).unwrap();
        let half = [2., 0.1, 0.1];
        assert_eq!(
            q.overlap_box(zone, half, &mut budget(), 2, |_| true)
                .unwrap(),
            [contact]
        );
        assert!(
            q.overlap_box(zone, half, &mut budget(), 2, |id| id != contact)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            q.overlap_box(zone, half, &mut budget(), 0, |_| true),
            Err(PhysicsError::Capacity)
        );
        assert_eq!(
            q.overlap_box(
                zone,
                half,
                &mut SupportQueryBudget::new(1).unwrap(),
                2,
                |_| true
            ),
            Err(PhysicsError::SweepBudget)
        );
        assert_eq!(
            q.overlap_box(zone, [0.; 3], &mut budget(), 2, |_| true),
            Err(PhysicsError::InvalidBody)
        );
    }

    #[test]
    fn box_overlap_reports_exact_touching_but_not_a_positive_gap() {
        let mut scene = SceneGraph::new(3);
        let zone = scene.spawn(None, Transform::default()).unwrap();
        let touching = box_at(&mut scene, 1.);
        let beyond = box_at(&mut scene, f32::from_bits(1_f32.to_bits() + 1));
        let q = SceneQueries::from_scene(&scene, 2).unwrap();
        let hits = q
            .overlap_box(zone, [0.5; 3], &mut budget(), 2, |_| true)
            .unwrap();
        assert_eq!(hits, [touching]);
        assert!(!hits.contains(&beyond));
    }
    #[test]
    fn nearest_all_hits_filters_and_stable_ties_include_characters() {
        let mut scene = SceneGraph::new(4);
        let first = box_at(&mut scene, 3.);
        let duplicate = box_at(&mut scene, 3.);
        let far = box_at(&mut scene, 5.);
        let body = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::CharacterBody {
                    half_extents: [0.25; 3],
                    ..Default::default()
                },
            )
            .unwrap();
        let queries = SceneQueries::from_scene(&scene, 4).unwrap();
        let r = ray(DVec3::ZERO, DVec3::X);
        let hit = queries
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(hit.owner, body);
        assert_eq!(hit.distance, 0.75);
        assert_eq!(hit.normal, Some(-DVec3::X));
        assert_eq!(
            queries
                .cast_ray(r, &mut budget(), |id| id != body)
                .unwrap()
                .unwrap()
                .owner,
            first
        );
        let hits = queries
            .intersect_ray(r, &mut budget(), 4, |id| id != body)
            .unwrap();
        assert_eq!(
            hits.iter().map(|hit| hit.owner).collect::<Vec<_>>(),
            vec![first, duplicate, far]
        );
        assert!(
            queries
                .cast_ray(r, &mut budget(), |_| false)
                .unwrap()
                .is_none()
        );
        scene.set_active(body, false).unwrap();
        assert_eq!(
            SceneQueries::from_scene(&scene, 3)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
                .owner,
            first
        );
    }
    #[test]
    fn interior_solid_hollow_boundary_and_finite_segment_semantics() {
        let mut scene = SceneGraph::new(1);
        box_at(&mut scene, 0.);
        let q = SceneQueries::from_scene(&scene, 1).unwrap();
        let mut r = ray(DVec3::ZERO, DVec3::X);
        r.max_distance = 0.;
        let hit = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert!(hit.started_inside && hit.normal.is_none());
        assert_eq!(hit.distance, 0.);
        r.solid = false;
        assert!(q.cast_ray(r, &mut budget(), |_| true).unwrap().is_none());
        r.max_distance = 1.;
        let hit = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert_eq!(hit.distance, 0.5);
        assert_eq!(hit.normal, Some(DVec3::X));
        assert!(hit.started_inside);
        r.origin.x = 0.5;
        r.max_distance = 0.;
        let boundary = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert_eq!(boundary.distance.to_bits(), 0_f64.to_bits());
        assert_eq!(boundary.normal, Some(DVec3::X));
        assert!(!boundary.started_inside);
        r = ray(DVec3::new(-1.5, 0., 0.), DVec3::X);
        r.max_distance = 1.;
        assert_eq!(
            q.cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
                .distance,
            1.
        );
        r.max_distance = f64::from_bits(1_f64.to_bits() - 1);
        assert!(q.cast_ray(r, &mut budget(), |_| true).unwrap().is_none());
        assert!(
            q.cast_ray(
                ray(DVec3::new(0.500001, -1., 0.), DVec3::Y),
                &mut budget(),
                |_| true
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn affine_shear_and_reflection_return_world_point_and_outward_normal() {
        let mut scene = SceneGraph::new(2);
        let parent = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(3., -1., 2.),
                    rotation: Quat::from_rotation_z(0.3),
                    scale: Vec3::new(-2., 0.5, 1.5),
                },
            )
            .unwrap();
        let owner = scene
            .spawn(
                Some(parent),
                Transform {
                    rotation: Quat::from_rotation_y(0.4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                BoxCollider {
                    half_extents: [0.2, 0.3, 0.4],
                },
            )
            .unwrap();
        let matrix = scene.world_matrix(owner).unwrap();
        let edges = [
            matrix.x_axis.truncate().as_dvec3() * f64::from(0.2_f32),
            matrix.y_axis.truncate().as_dvec3() * f64::from(0.3_f32),
            matrix.z_axis.truncate().as_dvec3() * f64::from(0.4_f32),
        ];
        let point =
            matrix.w_axis.truncate().as_dvec3() + edges[0] + edges[1] * 0.2 - edges[2] * 0.1;
        let normal =
            edges[1].cross(edges[2]).normalize() * f64::from(matrix.determinant().signum());
        let q = SceneQueries::from_scene(&scene, 1).unwrap();
        let hit = q
            .cast_ray(ray(point + normal * 2.5, -normal), &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - 2.5).abs() < 1e-12);
        assert!(hit.position.abs_diff_eq(point, 1e-12));
        assert!(hit.normal.unwrap().abs_diff_eq(normal, 1e-12));
    }
    #[test]
    fn invalid_queries_and_late_budget_failures_never_return_partial_hits() {
        let mut scene = SceneGraph::new(2);
        box_at(&mut scene, 2.);
        box_at(&mut scene, 4.);
        let token = scene.change_token();
        let q = SceneQueries::from_scene(&scene, 2).unwrap();
        let r = ray(DVec3::ZERO, DVec3::X);
        assert_eq!(
            q.cast_ray(r, &mut SupportQueryBudget::new(1).unwrap(), |_| true),
            Err(PhysicsError::SweepBudget)
        );
        assert_eq!(
            q.intersect_ray(r, &mut budget(), 1, |_| true),
            Err(PhysicsError::Capacity)
        );
        for bad in [
            RayQuery {
                direction: DVec3::ZERO,
                ..r
            },
            RayQuery {
                direction: DVec3::X * 2.,
                ..r
            },
            RayQuery {
                max_distance: f64::NAN,
                ..r
            },
            RayQuery {
                origin: DVec3::splat(f64::INFINITY),
                ..r
            },
        ] {
            let mut b = budget();
            assert_eq!(
                q.cast_ray(bad, &mut b, |_| true),
                Err(PhysicsError::InvalidMotion)
            );
            assert_eq!(b.remaining(), 64);
        }
        assert!(matches!(
            SceneQueries::from_scene(&scene, 1),
            Err(PhysicsError::Capacity)
        ));
        assert_eq!(scene.change_token(), token);
    }
    #[test]
    fn sweep_preserves_touching_interval_and_rejects_positive_gap() {
        let mut scene = SceneGraph::new(3);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        for y in [1.0, 1.0 + f32::EPSILON] {
            let target = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::new(2., y, 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    target,
                    crate::BoxCollider {
                        half_extents: [0.5; 3],
                    },
                )
                .unwrap();
        }
        let queries = SceneQueries::from_scene(&scene, 2).unwrap();
        let hits = queries
            .sweep_box(owner, [0.5; 3], DVec3::X * 4., &mut budget(), 2, |_| true)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].enter, hits[0].exit), (0.25, 0.75));
        assert!(
            queries
                .sweep_box(owner, [0.5; 3], DVec3::X * 4., &mut budget(), 0, |_| true)
                .is_err()
        );
        assert!(
            queries
                .sweep_box(
                    owner,
                    [0.5; 3],
                    DVec3::X * 4.,
                    &mut SupportQueryBudget::new(1).unwrap(),
                    2,
                    |_| true
                )
                .is_err()
        );
    }

    #[test]
    fn sweep_normal_distinguishes_entry_from_initial_overlap_and_separation() {
        let mut scene = SceneGraph::new(3);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let touching = box_at(&mut scene, 1.);
        let inside = box_at(&mut scene, 0.5);
        let queries = SceneQueries::from_scene(&scene, 2).unwrap();
        let sweep = |motion, target| {
            queries
                .sweep_box(owner, [0.5; 3], motion, &mut budget(), 1, |id| id == target)
                .unwrap()[0]
        };
        let entering = sweep(DVec3::X, touching);
        assert_eq!(entering.enter, 0.);
        assert_eq!(entering.normal, Some(DVec3::NEG_X));
        let separating = sweep(DVec3::NEG_X, touching);
        assert_eq!(
            (separating.enter, separating.exit, separating.normal),
            (0., 0., None)
        );
        assert_eq!(sweep(DVec3::Y, touching).normal, None);
        assert_eq!(sweep(DVec3::X, inside).normal, None);
    }

    #[test]
    fn sweep_normal_is_world_space_for_a_rotated_reflected_target() {
        let mut scene = SceneGraph::new(2);
        let rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let direction = (rotation * Vec3::X).as_dvec3().normalize();
        let owner = scene
            .spawn(
                None,
                Transform {
                    rotation,
                    ..Default::default()
                },
            )
            .unwrap();
        let target = scene
            .spawn(
                None,
                Transform {
                    translation: (direction * 3.).as_vec3(),
                    rotation,
                    scale: Vec3::new(-1., 1., 1.),
                },
            )
            .unwrap();
        scene
            .insert_component(
                target,
                BoxCollider {
                    half_extents: [0.5; 3],
                },
            )
            .unwrap();
        let hits = SceneQueries::from_scene(&scene, 1)
            .unwrap()
            .sweep_box(owner, [0.5; 3], direction * 4., &mut budget(), 1, |_| true)
            .unwrap();
        assert!((hits[0].enter - 0.5).abs() < 1e-7);
        assert!(hits[0].normal.unwrap().abs_diff_eq(-direction, 1e-12));
    }

    #[test]
    fn angular_box_cast_detects_contact_between_clear_endpoints_and_charges_advancement() {
        let mut scene = SceneGraph::new(2);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let target = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.6, 0.6, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                target,
                BoxCollider {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        let queries = SceneQueries::from_scene(&scene, 1).unwrap();
        let extents = [1., 0.05, 0.05];
        let angular = DVec3::Z * std::f64::consts::FRAC_PI_2;
        assert!(
            queries
                .overlap_box(owner, extents, &mut budget(), 1, |_| true)
                .unwrap()
                .is_empty()
        );
        assert!(
            queries
                .overlap_box(
                    owner,
                    [extents[1], extents[0], extents[2]],
                    &mut budget(),
                    1,
                    |_| true
                )
                .unwrap()
                .is_empty()
        );
        let mut work = SupportQueryBudget::new(256).unwrap();
        let hit = queries
            .cast_box_rotation(owner, extents, angular, &mut work, |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(hit.owner, target);
        assert!((0.3..0.5).contains(&hit.fraction));
        assert!((hit.normal.unwrap().length() - 1.).abs() < 1e-12);
        assert!(work.remaining() < 255);
        assert_eq!(
            queries.cast_box_rotation(
                owner,
                extents,
                angular,
                &mut SupportQueryBudget::new(1).unwrap(),
                |_| true
            ),
            Err(PhysicsError::SweepBudget),
        );
        assert_eq!(
            queries
                .cast_box_rotation(owner, extents, -angular, &mut work, |_| true)
                .unwrap(),
            None
        );
    }

    #[test]
    fn nearest_box_cast_matches_all_hits_and_rejects_late_failure() {
        let mut scene = SceneGraph::new(4);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let far = box_at(&mut scene, 5.);
        let near = box_at(&mut scene, 2.);
        let tied = box_at(&mut scene, 2.);
        let queries = SceneQueries::from_scene(&scene, 3).unwrap();
        let motion = DVec3::X * 6.;
        let all = queries
            .sweep_box(owner, [0.5; 3], motion, &mut budget(), 3, |_| true)
            .unwrap();
        assert_eq!(
            all.iter().map(|hit| hit.owner).collect::<Vec<_>>(),
            [near, tied, far]
        );
        assert_eq!(
            queries
                .cast_box(owner, [0.5; 3], motion, &mut budget(), |_| true)
                .unwrap(),
            Some(all[0])
        );
        assert_eq!(
            queries.cast_box(
                owner,
                [0.5; 3],
                motion,
                &mut SupportQueryBudget::new(2).unwrap(),
                |_| true
            ),
            Err(PhysicsError::SweepBudget),
        );
        assert_eq!(
            queries.cast_box_relative(owner, [0.5; 3], motion, &mut budget(), |id| {
                Some(if id == tied { DVec3::NAN } else { DVec3::ZERO })
            }),
            Err(PhysicsError::InvalidMotion),
        );
    }

    #[test]
    fn relative_sweep_covers_independent_motion_and_rejects_invalid_target_motion() {
        let mut scene = SceneGraph::new(2);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let target = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 4.,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                target,
                crate::BoxCollider {
                    half_extents: [0.5; 3],
                },
            )
            .unwrap();
        let queries = SceneQueries::from_scene(&scene, 1).unwrap();
        let sweep = |movement| {
            queries.sweep_box_relative(owner, [0.5; 3], DVec3::X * 4., &mut budget(), 1, |_| {
                Some(movement)
            })
        };
        assert_eq!(
            sweep(DVec3::NEG_X * 4.).unwrap(),
            [BoxSweepHit {
                owner: target,
                enter: 0.375,
                exit: 0.625,
                normal: Some(DVec3::NEG_X),
            },]
        );
        assert!(sweep(DVec3::X * 4.).unwrap().is_empty());
        assert!(matches!(
            sweep(DVec3::NAN),
            Err(PhysicsError::InvalidMotion)
        ));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn accepted_solver_precision_survives_controller_checkpoint_and_external_teleport() {
        let mut scene = SceneGraph::new(2);
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 100000.,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                crate::CharacterBody {
                    half_extents: [0.1; 3],
                    speed: 0.6,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 0);
        let mut input = crate::player_input().unwrap();
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        let pose = physics.accepted_pose(&scene, owner).unwrap().unwrap();
        assert_ne!(
            pose.physical_center.x,
            f64::from(pose.world_matrix.w_axis.x)
        );
        let r = ray(DVec3::new(99998., 0., 0.), DVec3::X);
        let expected = pose.physical_center.x - 99998. - f64::from(0.1_f32);
        let hit = physics
            .queries(&scene, 1)
            .unwrap()
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - expected).abs() < 1e-10);
        let published = SceneQueries::from_scene(&scene, 1)
            .unwrap()
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - published.distance).abs() > 1e-4);
        let zone = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 99999.875,
                    ..Default::default()
                },
            )
            .unwrap();
        let half = [0.034, 0.1, 0.1];
        assert!(
            physics
                .queries(&scene, 1)
                .unwrap()
                .overlap_box(zone, half, &mut budget(), 1, |_| true)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            SceneQueries::from_scene(&scene, 1)
                .unwrap()
                .overlap_box(zone, half, &mut budget(), 1, |_| true)
                .unwrap(),
            [owner]
        );
        let saved = physics
            .checkpoint(&scene, |_| Some(voxy_scene::ObjectId("body".into())))
            .unwrap();
        let saved = crate::CharacterPhysicsCheckpoint::from_json(
            &saved.to_json(100000).unwrap(),
            100000,
            1,
        )
        .unwrap();
        let mut restored = CharacterPhysics::new(&scene, 1, 0);
        restored
            .restore_checkpoint(&scene, &saved, |_| Some(owner))
            .unwrap();
        assert_eq!(
            restored
                .queries(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap(),
            hit
        );
        let mut local = scene.local(owner).unwrap();
        local.translation.x += 1.;
        scene.set_local(owner, local).unwrap();
        assert_eq!(
            physics
                .queries(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap(),
            SceneQueries::from_scene(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
        );
        // The moving character can also own the query. Place the static box
        // close enough to overlap only when the character's center is rounded.
        local.translation.x = 100000.;
        scene.set_local(owner, local).unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        scene
            .insert_component(zone, BoxCollider { half_extents: half })
            .unwrap();
        scene
            .insert_component(
                owner,
                crate::BoxTrigger {
                    half_extents: [0.1; 3],
                },
            )
            .unwrap();
        let mut tracker = crate::TriggerTracker::new(&scene, 1).unwrap();
        let precise = physics.queries(&scene, 2).unwrap();
        assert!(tracker.sample(&precise, &mut budget()).unwrap().is_empty());
        let published = SceneQueries::from_scene(&scene, 2).unwrap();
        assert_eq!(
            tracker.sample(&published, &mut budget()).unwrap(),
            [crate::TriggerEvent {
                trigger: owner,
                collider: zone,
                phase: crate::TriggerPhase::Enter
            }]
        );
    }
    #[test]
    fn attached_trigger_preserves_sub_ulp_offset_at_the_accepted_character_pose() {
        let mut scene = SceneGraph::new(4);
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 100_000.,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                crate::CharacterBody {
                    half_extents: [0.1; 3],
                    speed: 0.6,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        let mut input = crate::player_input().unwrap();
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();

        let parent = scene
            .spawn(
                Some(owner),
                Transform {
                    scale: Vec3::new(2., 1., 1.),
                    ..Default::default()
                },
            )
            .unwrap();
        let trigger = scene
            .spawn(
                Some(parent),
                Transform {
                    translation: Vec3::X * 0.0015,
                    ..Default::default()
                },
            )
            .unwrap();
        let target = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 100_000.031_25,
                    ..Default::default()
                },
            )
            .unwrap();
        let trigger_half = [0.00475, 0.0095, 0.0095];
        scene
            .insert_component(
                trigger,
                crate::BoxTrigger {
                    half_extents: trigger_half,
                },
            )
            .unwrap();
        scene
            .insert_component(
                target,
                BoxCollider {
                    half_extents: [0.0095; 3],
                },
            )
            .unwrap();

        let pose = physics.accepted_pose(&scene, owner).unwrap().unwrap();
        assert_eq!(
            scene.world_matrix(trigger).unwrap().w_axis.x,
            pose.world_matrix.w_axis.x
        );
        let queries = physics.queries(&scene, 2).unwrap();
        let query = queries.query_box(trigger, trigger_half).unwrap();
        assert_eq!(
            query.center.x,
            pose.physical_center.x + 2. * f64::from(0.0015_f32)
        );
        let mut tracker = crate::TriggerTracker::new(&scene, 2).unwrap();
        let events = tracker.sample(&queries, &mut budget()).unwrap();
        assert!(events.contains(&crate::TriggerEvent {
            trigger,
            collider: target,
            phase: crate::TriggerPhase::Enter,
        }));
        let published = SceneQueries::from_scene(&scene, 2).unwrap();
        assert!(
            published
                .overlap_box(trigger, trigger_half, &mut budget(), 1, |node| node
                    == target)
                .unwrap()
                .is_empty()
        );
    }
}
