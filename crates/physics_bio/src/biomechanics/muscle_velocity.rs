//! Objective force-velocity correction to the held-activation active potential.
use super::{
    Body, Element, ElementStress, Matrix, Stress, Vec3, columns, dot, mm, mv, outer, scale, sub,
    transpose,
};

/// Hill shortening branch and separately parameterized saturating eccentric branch.
#[derive(Clone, Copy, Debug)]
pub struct ActiveFiberVelocityLaw {
    /// Maximum shortening of dimensionless fiber stretch per SI second.
    pub max_shortening_per_s: f64,
    /// Hill a/P0, strictly positive.
    pub shortening_curvature: f64,
    pub eccentric_limit: f64,
    pub eccentric_rate_per_s: f64,
}
impl ActiveFiberVelocityLaw {
    /// Normalized tension; negative rate denotes shortening.
    /// # Errors
    /// Invalid parameters, nonfinite rate or derived overflow.
    pub fn factor(self, rate: f64) -> Result<f64, &'static str> {
        if [
            self.max_shortening_per_s,
            self.shortening_curvature,
            self.eccentric_rate_per_s,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
            || !self.eccentric_limit.is_finite()
            || self.eccentric_limit < 1.
            || !rate.is_finite()
        {
            return Err("invalid muscle velocity law");
        }
        let value = if rate <= -self.max_shortening_per_s {
            0.
        } else if rate <= 0. {
            let s = -rate / self.max_shortening_per_s;
            (1. - s) * (self.shortening_curvature / (self.shortening_curvature + s))
        } else {
            1. + (self.eccentric_limit - 1.) / (1. + self.eccentric_rate_per_s / rate)
        };
        if !value.is_finite() {
            return Err("muscle velocity overflow");
        }
        Ok(value)
    }
}
/// Additional nodal forces in N and their mechanical power in W.
#[derive(Clone, Debug)]
pub struct MuscleVelocityResponse {
    pub forces_n: Vec<Vec3>,
    /// Correction relative to existing active potential, not total metabolic power.
    pub correction_power_w: f64,
}
impl Body {
    /// Assemble a rate correction using current geometry and nodal velocities (m/s).
    /// Does not advance geometry or alter conservative energy. A dynamic integrator
    /// must account for this correction's work separately.
    /// # Errors
    /// Invalid velocities/law, incompatible cardiac/viscoelastic elements or geometry.
    pub fn active_velocity_forces(
        &self,
        velocities: &[Vec3],
        law: ActiveFiberVelocityLaw,
    ) -> Result<MuscleVelocityResponse, &'static str> {
        law.factor(0.)?;
        if velocities.len() != self.positions.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid muscle nodal velocities");
        }
        let mut forces = vec![[0.; 3]; velocities.len()];
        for e in &self.elements {
            if e.activation == 0. || e.material.fibers.iter().all(|f| f.active_pa == 0.) {
                continue;
            }
            if e.myocardium.is_some() || e.viscoelastic.is_some() {
                return Err("conflicting velocity muscle material");
            }
            let edges = |values: &[Vec3]| {
                let p = e.nodes.map(|i| values[i]);
                columns(sub(p[1], p[0]), sub(p[2], p[0]), sub(p[3], p[0]))
            };
            let f = mm(edges(&self.positions), e.inv_rest);
            e.response(f)?;
            let fdot = mm(edges(velocities), e.inv_rest);
            let piola = e.velocity_piola(f, fdot, law)?;
            for (local, node) in e.nodes.iter().enumerate() {
                let force = scale(mv(piola, e.gradients[local]), -e.volume);
                for axis in 0..3 {
                    forces[*node][axis] += force[axis];
                }
            }
        }
        let power = forces
            .iter()
            .zip(velocities)
            .map(|(f, v)| dot(*f, *v))
            .sum::<f64>();
        if !power.is_finite() || forces.iter().flatten().any(|f| !f.is_finite()) {
            return Err("muscle velocity assembly overflow");
        }
        Ok(MuscleVelocityResponse {
            forces_n: forces,
            correction_power_w: power,
        })
    }
}

impl Element {
    fn velocity_piola(
        &self,
        f: Matrix,
        fdot: Matrix,
        law: ActiveFiberVelocityLaw,
    ) -> Result<Matrix, &'static str> {
        let mut piola = [[0.; 3]; 3];
        if self.activation == 0. || self.material.fibers.iter().all(|f| f.active_pa == 0.) {
            return Ok(piola);
        }
        if self.myocardium.is_some() || self.viscoelastic.is_some() {
            return Err("conflicting velocity muscle material");
        }
        for fiber in &self.material.fibers {
            let a = mv(f, fiber.direction);
            let stretch = dot(a, a).sqrt();
            let rate = dot(a, mv(fdot, fiber.direction)) / stretch;
            let length_factor = match self.active_length_law {
                Some(length) => length.response(stretch)?.0,
                None => 1.,
            };
            let tension =
                self.activation * fiber.active_pa * length_factor * (law.factor(rate)? - 1.);
            let term = outer(scale(a, tension / stretch), fiber.direction);
            for i in 0..3 {
                for j in 0..3 {
                    piola[i][j] += term[i][j];
                }
            }
        }
        if piola.iter().flatten().any(|x| !x.is_finite()) {
            return Err("muscle velocity stress overflow");
        }
        Ok(piola)
    }
}
impl Body {
    /// Total current Cauchy stress including active force–velocity correction.
    /// Includes existing passive, active-length and pore-pressure stresses.
    /// Velocity forces and stress use the same first-Piola constitutive correction.
    /// # Errors
    /// Invalid velocities/law, incompatible material, invalid geometry or overflow.
    pub fn muscle_stresses(
        &self,
        velocities: &[Vec3],
        law: ActiveFiberVelocityLaw,
    ) -> Result<Vec<ElementStress>, &'static str> {
        law.factor(0.)?;
        if velocities.len() != self.positions.len()
            || velocities.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid muscle stress velocities");
        }
        let mut result = self.stresses_at(&self.positions)?;
        for (element, report) in self.elements.iter().zip(&mut result) {
            let edges = |values: &[Vec3]| {
                let p = element.nodes.map(|i| values[i]);
                columns(sub(p[1], p[0]), sub(p[2], p[0]), sub(p[3], p[0]))
            };
            let f = mm(edges(&self.positions), element.inv_rest);
            let fdot = mm(edges(velocities), element.inv_rest);
            let correction = mm(element.velocity_piola(f, fdot, law)?, transpose(f));
            let mut spatial = report.stress.cauchy_pa;
            for i in 0..3 {
                for j in 0..3 {
                    spatial[i][j] += correction[i][j] / report.volume_ratio;
                }
            }
            report.stress = Stress::from_cauchy(spatial)?;
        }
        Ok(result)
    }
}
