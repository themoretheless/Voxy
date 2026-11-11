//! Bounded support queries over the same affine boxes used by character physics.
mod queries;
pub use queries::{AngularBoxHit, BoxSweepHit, RayHit, RayQuery, SceneQueries};
#[path = "support_checkpoint.rs"]
mod checkpoint;
use super::{
    BoxCollider, PhysicsError, StaticWorld, static_world, validate_extents,
    validate_static_collider,
};
pub use checkpoint::SupportAnchorCheckpoint;
use glam::{DMat3, DVec3};
use voxy_scene::{NodeId, SceneGraph, SceneId};
pub(crate) const MAX_COLLIDERS: usize = 4096;
pub(crate) const MAX_QUERIES: usize = 65536;

/// Budget shared by all probes and anchor resolutions in a staged tick.
#[derive(Clone, Copy, Debug)]
pub struct SupportQueryBudget {
    remaining: usize,
}
impl SupportQueryBudget {
    /// # Errors
    /// Rejects limits above the aggregate per-tick query ceiling.
    pub fn new(limit: usize) -> Result<Self, PhysicsError> {
        if limit > MAX_QUERIES {
            return Err(PhysicsError::Capacity);
        }
        Ok(Self { remaining: limit })
    }
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    /// Charges bounded preparation work against the same accepted-tick budget
    /// used by support queries. Failed reservations preserve the remaining budget.
    pub fn charge_work(&mut self, units: usize) -> Result<(), PhysicsError> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or(PhysicsError::SweepBudget)?;
        Ok(())
    }
    fn spend(&mut self) -> Result<(), PhysicsError> {
        self.charge_work(1)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SupportProbe {
    pub origin: DVec3,
    pub direction: DVec3,
    pub max_distance: f64,
    pub up: DVec3,
    pub min_up_dot: f64,
}
/// Opaque scene/generation-bound object-local point on one collider face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportAnchor {
    scene: SceneId,
    owner: NodeId,
    local_position: DVec3,
    face: usize,
    sign: f64,
}
impl SupportAnchor {
    #[must_use]
    pub fn owner(&self) -> NodeId {
        self.owner
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportContact {
    pub anchor: SupportAnchor,
    pub position: DVec3,
    pub normal: DVec3,
    /// Ray distance for a probe; zero for an existing anchor resolution.
    pub distance: f64,
}
#[derive(Clone, Debug)]
struct Surface {
    owner: NodeId,
    layers: crate::CollisionLayers,
    center: DVec3,
    edges: DMat3,
    inverse: DMat3,
    normals: [DVec3; 3],
    half: DVec3,
}
fn normal_direction(vector: DVec3) -> Result<DVec3, PhysicsError> {
    let scale = vector.abs().max_element();
    if !vector.is_finite() || scale == 0. {
        return Err(PhysicsError::UnsupportedTransform);
    }
    Ok((vector / scale).normalize())
}
#[derive(Clone, Debug)]
pub struct SupportWorld {
    scene: SceneId,
    surfaces: Vec<Surface>,
    character_layers: std::collections::BTreeMap<NodeId, crate::CollisionLayers>,
}
impl SupportWorld {
    /// Immutable geometry snapshot. Rebuild once after platform transform updates.
    /// # Errors
    /// Rejects capacity, invalid collider extents/transforms and coordinate overflow.
    pub fn from_scene(scene: &SceneGraph, capacity: usize) -> Result<Self, PhysicsError> {
        if capacity > MAX_COLLIDERS || scene.active_components::<BoxCollider>().count() > capacity {
            return Err(PhysicsError::Capacity);
        }
        for (owner, collider) in scene.active_components::<BoxCollider>() {
            validate_static_collider(scene, owner, *collider)?;
        }
        let mut world = Self::from_static_world(&static_world(scene)?)?;
        world.bind_character_layers(scene);
        Ok(world)
    }
    pub(crate) fn from_static_world(world: &StaticWorld) -> Result<Self, PhysicsError> {
        if world.0.len() > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        let mut surfaces = Vec::with_capacity(world.0.len());
        for obstacle in &world.0 {
            validate_extents(obstacle.half_extents)?;
            let edges = DMat3::from_cols(
                obstacle.shape.edges[0],
                obstacle.shape.edges[1],
                obstacle.shape.edges[2],
            );
            let inverse = edges.inverse();
            if !inverse.is_finite() {
                return Err(PhysicsError::UnsupportedTransform);
            }
            let normals = [
                normal_direction(inverse.transpose() * DVec3::X)?,
                normal_direction(inverse.transpose() * DVec3::Y)?,
                normal_direction(inverse.transpose() * DVec3::Z)?,
            ];
            surfaces.push(Surface {
                owner: obstacle.owner,
                layers: obstacle.layers,
                center: obstacle.shape.center,
                edges,
                inverse,
                normals,
                half: DVec3::from_array(obstacle.half_extents.map(f64::from)),
            });
        }
        surfaces.sort_by_key(|surface| surface.owner);
        Ok(Self {
            scene: world.1,
            surfaces,
            character_layers: Default::default(),
        })
    }
    pub(crate) fn bind_character_layers(&mut self, scene: &SceneGraph) {
        self.character_layers = scene
            .active_components::<crate::CollisionLayers>()
            .map(|(owner, layers)| (owner, *layers))
            .collect();
    }

    /// Selects supports accepted by the character's collision layers.
    /// # Errors
    /// Rejects a snapshot scan that exceeds the remaining query budget.
    pub fn for_character(
        &self,
        owner: NodeId,
        budget: &mut SupportQueryBudget,
    ) -> Result<std::borrow::Cow<'_, Self>, PhysicsError> {
        if self.character_layers.is_empty() {
            return Ok(std::borrow::Cow::Borrowed(self));
        }
        budget.charge_work(self.surfaces.len())?;
        let layers = self
            .character_layers
            .get(&owner)
            .copied()
            .unwrap_or_default();
        let Some(excluded) = self
            .surfaces
            .iter()
            .position(|surface| !layers.interacts(surface.layers))
        else {
            return Ok(std::borrow::Cow::Borrowed(self));
        };
        let mut surfaces = self.surfaces[..excluded].to_vec();
        surfaces.extend(
            self.surfaces[excluded + 1..]
                .iter()
                .filter(|surface| layers.interacts(surface.layers))
                .cloned(),
        );
        Ok(std::borrow::Cow::Owned(Self {
            scene: self.scene,
            surfaces,
            character_layers: Default::default(),
        }))
    }

    /// Finds the nearest physical surface. An unwalkable nearest hit blocks
    /// support below it; rays never look through an occluding steep surface.
    /// Strictly interior origins have no support. Equal-distance ties use `NodeId` order.
    /// # Errors
    /// Rejects invalid probes, exhausted aggregate budgets or unstable arithmetic.
    /// Budget failure never returns a previously found partial hit.
    pub fn probe(
        &self,
        probe: SupportProbe,
        budget: &mut SupportQueryBudget,
    ) -> Result<Option<SupportContact>, PhysicsError> {
        let unit = |v: DVec3| v.is_finite() && (v.length_squared() - 1.).abs() <= 1e-12;
        if !probe.origin.is_finite()
            || probe.origin.abs().max_element() > 1e6
            || !unit(probe.direction)
            || !unit(probe.up)
            || !probe.max_distance.is_finite()
            || !(0. ..=1e6).contains(&probe.max_distance)
            || !probe.min_up_dot.is_finite()
            || !(0. ..=1.).contains(&probe.min_up_dot)
        {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut best: Option<SupportContact> = None;
        let mut interior = false;
        for surface in &self.surfaces {
            budget.spend()?;
            let p = surface.inverse * (probe.origin - surface.center);
            let v = surface.inverse * probe.direction;
            if !p.is_finite() || !v.is_finite() {
                return Err(PhysicsError::Solver);
            }
            if p.abs().max_element() < 1. {
                interior = true;
                continue;
            }
            let (mut enter, mut exit) = (f64::NEG_INFINITY, f64::INFINITY);
            let (mut face, mut sign) = (0, 0.);
            let mut missed = false;
            for axis in 0..3 {
                if v[axis] == 0. {
                    if p[axis].abs() > 1. {
                        missed = true;
                        break;
                    }
                    continue;
                }
                let mut near = (-1. - p[axis]) / v[axis];
                let mut far = (1. - p[axis]) / v[axis];
                let mut side = -1.;
                if near > far {
                    std::mem::swap(&mut near, &mut far);
                    side = 1.;
                }
                let tied_better = near == enter
                    && sign != 0.
                    && (surface.normals[axis] * side).dot(probe.up)
                        > (surface.normals[face] * sign).dot(probe.up);
                if near > enter || tied_better {
                    enter = near;
                    face = axis;
                    sign = side;
                }
                exit = exit.min(far);
                if enter > exit {
                    missed = true;
                    break;
                }
            }
            if missed || enter < 0. || enter > probe.max_distance || sign == 0. {
                continue;
            }
            if best.is_some_and(|hit| enter >= hit.distance) {
                continue;
            }
            let mut local = p + v * enter;
            local[face] = sign;
            let guard =
                128. * f64::EPSILON * (1. + p.abs().max_element() + v.abs().max_element() * enter);
            if !local.is_finite() || local.abs().max_element() > 1. + guard {
                return Err(PhysicsError::Solver);
            }
            local = local.clamp(DVec3::NEG_ONE, DVec3::ONE);
            let position = surface.center + surface.edges * local;
            let ray = probe.origin + probe.direction * enter;
            let tolerance = 256.
                * f64::EPSILON
                * (1. + probe.origin.abs().max_element() + position.abs().max_element());
            if !position.is_finite() || !position.abs_diff_eq(ray, tolerance) {
                return Err(PhysicsError::Solver);
            }
            let normal = surface.normals[face] * sign;
            if !normal.is_finite() {
                return Err(PhysicsError::Solver);
            }
            best = Some(SupportContact {
                anchor: SupportAnchor {
                    scene: self.scene,
                    owner: surface.owner,
                    local_position: local * surface.half,
                    face,
                    sign,
                },
                position,
                normal,
                distance: enter,
            });
        }
        if interior {
            return Ok(None);
        }
        Ok(best.filter(|hit| hit.normal.dot(probe.up) >= probe.min_up_dot))
    }
    /// Resolves a planted point after a platform moves. Removed/inactive/recycled
    /// supports or changed face dimensions release the anchor rather than alias it.
    /// # Errors
    /// Rejects foreign scenes or exhausted budgets before returning a contact.
    pub fn resolve(
        &self,
        anchor: SupportAnchor,
        budget: &mut SupportQueryBudget,
    ) -> Result<Option<SupportContact>, PhysicsError> {
        if anchor.scene != self.scene {
            return Err(PhysicsError::InvalidMotion);
        }
        budget.spend()?;
        let Ok(index) = self
            .surfaces
            .binary_search_by_key(&anchor.owner, |surface| surface.owner)
        else {
            return Ok(None);
        };
        let surface = &self.surfaces[index];
        // These are captured object-local values from authored f32 extents, not
        // re-evaluated world coordinates: exact comparison detects even one-ULP
        // face replacement on a wide, thin platform.
        if (anchor.local_position.abs() - surface.half).max_element() > 0.
            || anchor.local_position[anchor.face] != anchor.sign * surface.half[anchor.face]
        {
            return Ok(None);
        }
        let normalized = anchor.local_position / surface.half;
        let position = surface.center + surface.edges * normalized;
        let normal = surface.normals[anchor.face] * anchor.sign;
        if !position.is_finite() || !normal.is_finite() {
            return Err(PhysicsError::Solver);
        }
        Ok(Some(SupportContact {
            anchor,
            position,
            normal,
            distance: 0.,
        }))
    }
}

/// Computes the character acceleration and traction multiplier in `[0.0, 1.0]` on an inclined
/// or slippery surface from the cosine of the slope angle (`normal_up_dot` = $\mathbf{n} \cdot \mathbf{u}$),
/// the minimum walkable cosine `min_walkable_up_dot`, and surface `friction_coeff`.
#[must_use]
pub fn slope_traction_acceleration_scale(
    normal_up_dot: f32,
    min_walkable_up_dot: f32,
    friction_coeff: f32,
) -> f32 {
    if !normal_up_dot.is_finite()
        || !min_walkable_up_dot.is_finite()
        || !friction_coeff.is_finite()
        || friction_coeff <= 0.0
    {
        return 0.0;
    }
    let min_dot = min_walkable_up_dot.clamp(0.0, 0.9999);
    let cos_theta = normal_up_dot.clamp(-1.0, 1.0);
    if cos_theta < min_dot {
        return 0.0;
    }
    let slope_t = ((cos_theta - min_dot) / (1.0 - min_dot)).clamp(0.0, 1.0);
    let smooth_slope = slope_t * slope_t * (3.0 - 2.0 * slope_t);
    let mu = friction_coeff.clamp(0.0, 2.0);
    let grip = (mu / (1.0 + mu)) * 2.0;
    (smooth_slope * grip.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod traction_tests {
    use super::*;

    #[test]
    fn slope_traction_acceleration_scale_drops_on_steep_or_icy_slopes() {
        assert!((slope_traction_acceleration_scale(1.0, 0.7, 1.0) - 1.0).abs() < 1e-6);
        assert_eq!(slope_traction_acceleration_scale(0.5, 0.7, 1.0), 0.0);
        let ice = slope_traction_acceleration_scale(0.9, 0.7, 0.1);
        let dry = slope_traction_acceleration_scale(0.9, 0.7, 1.0);
        assert!(ice > 0.0 && ice < dry);
    }
}
#[cfg(test)]
mod tests;
