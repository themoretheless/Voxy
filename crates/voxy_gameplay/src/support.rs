//! Bounded support queries over the same affine boxes used by character physics.
use super::{
    BoxCollider, PhysicsError, StaticWorld, static_world, validate_extents,
    validate_static_collider,
};
use glam::{DMat3, DVec3};
use voxy_scene::{NodeId, SceneGraph, SceneId};
const MAX_COLLIDERS: usize = 4096;
const MAX_QUERIES: usize = 65536;

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
    fn spend(&mut self) -> Result<(), PhysicsError> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(PhysicsError::SweepBudget)?;
        Ok(())
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
        Self::from_static_world(&static_world(scene)?)
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
        })
    }
    /// Finds the nearest physical surface. An unwalkable nearest hit blocks
    /// support below it; rays never look through an occluding steep surface.
    /// Strictly interior origins have no support. Equal-distance ties use NodeId order.
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
#[cfg(test)]
mod tests;
