//! Continuous static geometry contract with arbitrary unit contact normals.
use super::{ContactConfig, Container, Error, Liquid, StepStats};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryHit {
    Clear,
    Overlap,
    Contact { fraction: f64, normal: [f64; 3] },
}
pub trait LiquidGeometry {
    type Error;
    /// Sweep the liquid particle's axis-aligned collision box against static geometry.
    /// # Errors
    /// Backend admission, malformed geometry or candidate budget exhaustion.
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<GeometryHit, Self::Error>;
}
impl Liquid {
    /// Continuous static collisions, including oblique planes and affine geometry.
    /// # Errors
    /// Invalid response/normal, overlap, backend failure or contact budgets.
    /// All fluid state is transactional. Geometry is read-only.
    pub fn step_with_geometry(
        &mut self,
        dt: f64,
        container: Option<Container>,
        geometry: &impl LiquidGeometry,
        contact: ContactConfig,
    ) -> Result<StepStats, Error> {
        contact.validate()?;
        let radius = self.config.particle_radius;
        self.advance(dt, container, |particles, time| {
            for p in particles {
                let mut remaining = time;
                for index in 0..=contact.max_contacts {
                    let displacement = p.velocity.map(|v| v * remaining);
                    match geometry
                        .sweep(p.position, radius, displacement, contact.max_candidates)
                        .map_err(|_| Error::CollisionBackend)?
                    {
                        GeometryHit::Clear => {
                            for axis in 0..3 {
                                p.position[axis] += displacement[axis];
                            }
                            break;
                        }
                        GeometryHit::Overlap => return Err(Error::InitialOverlap),
                        GeometryHit::Contact { fraction, normal } => {
                            let length2: f64 = normal.iter().map(|n| n * n).sum();
                            let vn: f64 = p.velocity.iter().zip(normal).map(|(v, n)| v * n).sum();
                            if !fraction.is_finite()
                                || !(0. ..=1.).contains(&fraction)
                                || !length2.is_finite()
                                || (length2 - 1.).abs() > 1e-10
                                || !vn.is_finite()
                                || vn >= 0.
                            {
                                return Err(Error::InvalidCollision);
                            }
                            if index == contact.max_contacts {
                                return Err(Error::CollisionBudget);
                            }
                            let epsilon = 64.
                                * f64::EPSILON
                                * p.position
                                    .iter()
                                    .map(|v| v.abs())
                                    .fold(radius.max(1.), f64::max);
                            for axis in 0..3 {
                                p.position[axis] +=
                                    displacement[axis] * fraction + normal[axis] * epsilon;
                                let tangent = p.velocity[axis] - vn * normal[axis];
                                p.velocity[axis] = tangent * (1. - contact.friction)
                                    - vn * contact.restitution * normal[axis];
                            }
                            remaining *= 1. - fraction;
                            if remaining <= 0. {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(())
        })
    }
}
