//! Point-contact impulse mechanics, shared by particles and finite rigid bodies.
//! Geometry, event timing and persistent ownership remain with their callers.
use crate::{astrophysics_spin::Spin, gravity::Body};
type Vector = [f64; 3];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalFailure,
}
/// A contact-time snapshot. Position is the center of mass, not a scene pivot.
/// A point particle has no intrinsic spin. Rigid inertia uses the existing Spin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBody {
    pub motion: Body,
    pub spin: Option<Spin>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalImpulse {
    /// World impulse applied to the first body, opposite to the second/boundary.
    pub impulse: Vector,
    pub dissipated_energy: f64,
    pub relative_normal_speed: f64,
    pub inverse_effective_mass: f64,
}
fn finite(v: Vector) -> bool {
    v.iter().all(|v| v.is_finite())
}
fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] - b[k])
}
impl ContactBody {
    fn validate(self) -> Result<(), Error> {
        if !self.motion.mass.is_finite()
            || self.motion.mass <= 0.
            || !finite(self.motion.position)
            || !finite(self.motion.velocity)
        {
            return Err(Error::InvalidInput);
        }
        if let Some(spin) = self.spin {
            spin.energy().map_err(|_| Error::InvalidInput)?;
        }
        Ok(())
    }
    /// Velocity of the material point at the given world coordinate.
    pub fn point_velocity(self, point: Vector) -> Result<Vector, Error> {
        self.validate()?;
        if !finite(point) {
            return Err(Error::InvalidInput);
        }
        let mut velocity = self.motion.velocity;
        if let Some(spin) = self.spin {
            let rotation = cross(
                spin.angular_velocity()
                    .map_err(|_| Error::NumericalFailure)?,
                sub(point, self.motion.position),
            );
            for k in 0..3 {
                velocity[k] += rotation[k];
            }
        }
        if !finite(velocity) {
            return Err(Error::NumericalFailure);
        }
        Ok(velocity)
    }
    /// Kinetic energy including intrinsic rigid rotation.
    pub fn energy(self) -> Result<f64, Error> {
        self.validate()?;
        let energy = self
            .motion
            .velocity
            .iter()
            .map(|v| (0.5 * self.motion.mass * v) * v)
            .sum::<f64>()
            + self
                .spin
                .map_or(Ok(0.), Spin::energy)
                .map_err(|_| Error::NumericalFailure)?;
        if !energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
    fn inverse_contact_mass(self, point: Vector, normal: Vector) -> Result<f64, Error> {
        let mut inverse = 1. / self.motion.mass;
        if let Some(spin) = self.spin {
            let arm = cross(sub(point, self.motion.position), normal);
            let response = spin
                .inverse_inertia(arm)
                .map_err(|_| Error::NumericalFailure)?;
            inverse += dot(arm, response);
        }
        if !inverse.is_finite() || inverse <= 0. {
            return Err(Error::NumericalFailure);
        }
        Ok(inverse)
    }
    /// Apply a world impulse at a world point; failure preserves the snapshot.
    pub fn apply_point_impulse(&mut self, point: Vector, impulse: Vector) -> Result<(), Error> {
        self.validate()?;
        if !finite(point) || !finite(impulse) {
            return Err(Error::InvalidInput);
        }
        let mut candidate = *self;
        for k in 0..3 {
            candidate.motion.velocity[k] += impulse[k] / candidate.motion.mass;
        }
        if let Some(spin) = &mut candidate.spin {
            let torque = cross(sub(point, candidate.motion.position), impulse);
            for k in 0..3 {
                spin.angular_momentum[k] += torque[k];
            }
        }
        candidate.energy()?;
        *self = candidate;
        Ok(())
    }
}
/// Frictionless normal response at one shared point. Normal points toward first.
/// A missing second body denotes a stationary infinite-mass boundary.
/// Geometry must certify the point and normal; this function does not detect hits.
/// Separating/tangent contacts produce zero impulse. No heat is deposited.
pub fn normal_impulse(
    first: &ContactBody,
    second: Option<&ContactBody>,
    point: Vector,
    normal: Vector,
    restitution: f64,
) -> Result<NormalImpulse, Error> {
    let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !finite(point)
        || !norm.is_finite()
        || (norm - 1.).abs() > 5e-11
        || !restitution.is_finite()
        || !(0. ..=1.).contains(&restitution)
    {
        return Err(Error::InvalidInput);
    }
    let normal = normal.map(|v| v / norm);
    let velocity = first.point_velocity(point)?;
    let other = second.map_or(Ok([0.; 3]), |b| b.point_velocity(point))?;
    let speed = dot(sub(velocity, other), normal);
    let inverse = first.inverse_contact_mass(point, normal)?
        + second.map_or(Ok(0.), |b| b.inverse_contact_mass(point, normal))?;
    if !speed.is_finite() || !inverse.is_finite() || inverse <= 0. {
        return Err(Error::NumericalFailure);
    }
    let approaching = speed.min(0.);
    let strength = -(1. + restitution) * approaching / inverse;
    let result = NormalImpulse {
        impulse: normal.map(|v| v * strength),
        dissipated_energy: 0.5
            * (approaching / inverse)
            * approaching
            * (1. - restitution * restitution),
        relative_normal_speed: speed,
        inverse_effective_mass: inverse,
    };
    if !finite(result.impulse) || !result.dissipated_energy.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}
/// Equal and opposite point impulses, committed together after full validation.
pub fn resolve_normal_impact(
    first: &mut ContactBody,
    second: Option<&mut ContactBody>,
    point: Vector,
    normal: Vector,
    restitution: f64,
) -> Result<NormalImpulse, Error> {
    let report = normal_impulse(first, second.as_deref(), point, normal, restitution)?;
    let mut a = *first;
    let mut b = second.as_deref().copied();
    a.apply_point_impulse(point, report.impulse)?;
    if let Some(body) = &mut b {
        body.apply_point_impulse(point, report.impulse.map(|v| -v))?;
    }
    *first = a;
    if let Some(second) = second {
        *second = b.expect("second staged");
    }
    Ok(report)
}
