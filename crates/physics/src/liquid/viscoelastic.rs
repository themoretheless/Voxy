//! Upper-convected Maxwell polymer stress, superposed on the solvent viscosity.
//! Constitutive equation: dC/dt = LC + CL^T - (C-I)/lambda.
//! https://arxiv.org/html/2401.03981v1 (equations 1--2).
use super::{Error, Liquid, Material, Particle, density_kernel_gradient, norm, positive, sub};

pub type Conformation = [[f64; 3]; 3];
pub(super) const IDENTITY: Conformation = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

/// Illustrative or measured polymer coefficients in SI, independent of solvent viscosity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxwellFluid {
    /// Polymer shear modulus, Pa. The zero-rate polymer viscosity is G*lambda.
    pub modulus: f64,
    pub relaxation_time: f64,
}

fn multiply(a: Conformation, b: Conformation) -> Conformation {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}
fn transpose(a: Conformation) -> Conformation {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}
fn determinant(a: Conformation) -> f64 {
    a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
}
fn valid(c: Conformation) -> bool {
    c.iter().flatten().all(|v| v.is_finite())
        && (0..3).all(|i| {
            (0..3).all(|j| {
                (c[i][j] - c[j][i]).abs() <= 1e-12 * c[i][j].abs().max(c[j][i].abs()).max(1.)
            })
        })
        && positive(c[0][0])
        && positive(c[0][0] * c[1][1] - c[0][1] * c[1][0])
        && positive(determinant(c))
}
fn difference(a: Conformation, b: Conformation) -> Conformation {
    std::array::from_fn(|i| std::array::from_fn(|j| a[i][j] - b[i][j]))
}
fn pseudoinverse(a: Conformation) -> (Conformation, Conformation) {
    let scale = a.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
    if !positive(scale) {
        return ([[0.; 3]; 3], [[0.; 3]; 3]);
    }
    let mut diagonal = a.map(|r| r.map(|v| v / scale));
    let mut vectors = IDENTITY;
    for _ in 0..24 {
        let (p, q) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|(a, b), (c, d)| diagonal[*a][*b].abs().total_cmp(&diagonal[*c][*d].abs()))
            .unwrap();
        if diagonal[p][q].abs() < 1e-14 {
            break;
        }
        let angle = 0.5 * (2. * diagonal[p][q]).atan2(diagonal[q][q] - diagonal[p][p]);
        let mut rotation = IDENTITY;
        rotation[p][p] = angle.cos();
        rotation[q][q] = angle.cos();
        rotation[p][q] = angle.sin();
        rotation[q][p] = -angle.sin();
        diagonal = multiply(multiply(transpose(rotation), diagonal), rotation);
        vectors = multiply(vectors, rotation);
    }
    let largest = (0..3).map(|i| diagonal[i][i]).fold(0., f64::max);
    let active = |k: usize| diagonal[k][k] > largest * 1e-10 && diagonal[k][k] > 0.;
    let inverse = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .filter(|k| active(*k))
                .map(|k| vectors[i][k] * vectors[j][k] / (diagonal[k][k] * scale))
                .sum()
        })
    });
    let projection = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .filter(|k| active(*k))
                .map(|k| vectors[i][k] * vectors[j][k])
                .sum()
        })
    });
    (inverse, projection)
}
fn exponential(mut a: Conformation) -> Result<Conformation, Error> {
    let size = a
        .iter()
        .map(|r| r.iter().map(|v| v.abs()).sum::<f64>())
        .fold(0., f64::max);
    if !size.is_finite() {
        return Err(Error::NumericalFailure);
    }
    let squarings = if size > 0.25 {
        (size / 0.25).log2().ceil() as u32
    } else {
        0
    };
    if squarings > 32 {
        return Err(Error::NumericalFailure);
    }
    a = a.map(|r| r.map(|v| v / 2_f64.powi(squarings as i32)));
    let mut result = IDENTITY;
    let mut term = IDENTITY;
    for n in 1..=18 {
        term = multiply(term, a).map(|r| r.map(|v| v / n as f64));
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] += term[i][j];
            }
        }
    }
    for _ in 0..squarings {
        result = multiply(result, result);
    }
    if result.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}
fn free_energy(c: Conformation) -> Result<f64, Error> {
    if !valid(c) {
        return Err(Error::NumericalFailure);
    }
    let value = c[0][0] + c[1][1] + c[2][2] - determinant(c).ln() - 3.;
    if !value.is_finite() || value < -1e-12 {
        return Err(Error::NumericalFailure);
    }
    Ok(value.max(0.))
}
impl MaxwellFluid {
    fn validate(self) -> Result<(), Error> {
        if !positive(self.modulus) || !positive(self.relaxation_time) {
            return Err(Error::InvalidMaterial);
        }
        Ok(())
    }
    /// Frozen-gradient Strang step: relaxation / exp(dt*L) congruence / relaxation.
    /// Returns new positive-definite C and relaxation heat per rest volume (J/m^3).
    /// Deformation work is not included in this heat; it is coupled to momentum.
    pub fn advance_conformation(
        self,
        c: Conformation,
        gradient: Conformation,
        dt: f64,
    ) -> Result<(Conformation, f64), Error> {
        self.validate()?;
        if !dt.is_finite() || dt < 0. || !valid(c) {
            return Err(Error::InvalidConfig);
        }
        let relax = (-0.5 * dt / self.relaxation_time).exp();
        let relaxed = |a: Conformation| {
            std::array::from_fn(|i| {
                std::array::from_fn(|j| IDENTITY[i][j] + relax * (a[i][j] - IDENTITY[i][j]))
            })
        };
        let first = relaxed(c);
        let f = exponential(gradient.map(|r| r.map(|v| v * dt)))?;
        let moved = multiply(multiply(f, first), transpose(f));
        let next = relaxed(moved);
        let heat = 0.5
            * self.modulus
            * (free_energy(c)? - free_energy(first)? + free_energy(moved)? - free_energy(next)?);
        if !heat.is_finite() || heat < -1e-10 * self.modulus {
            return Err(Error::NumericalFailure);
        }
        Ok((next, heat.max(0.)))
    }
    pub fn energy_density(self, c: Conformation) -> Result<f64, Error> {
        self.validate()?;
        let value = 0.5 * self.modulus * free_energy(c)?;
        if !value.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(value)
    }
}

impl Liquid {
    /// Enables a polymer stress branch for a material. Changes require relaxed memory.
    /// Thermal transport receives relaxation heat; without it heat is an explicit ledger.
    /// Material coefficients are supplied explicitly.
    pub fn set_maxwell_fluid(
        &mut self,
        material: usize,
        law: Option<MaxwellFluid>,
    ) -> Result<(), Error> {
        if material >= self.materials.len() {
            return Err(Error::InvalidMaterial);
        }
        if let Some(law) = law {
            law.validate()?;
            if self.gas_active() || self.phase_fractions().is_some() {
                return Err(Error::InvalidConfig);
            }
        }
        if self
            .particles
            .iter()
            .enumerate()
            .any(|(i, p)| p.material == material && self.conformation[i] != IDENTITY)
        {
            return Err(Error::InvalidConfig);
        }
        self.maxwell_fluids[material] = law;
        Ok(())
    }
    pub fn conformations(&self) -> &[Conformation] {
        &self.conformation
    }
    /// Prescribed initial polymer memory, useful for controlled stress experiments.
    pub fn set_conformations(&mut self, values: Vec<Conformation>) -> Result<(), Error> {
        if values.len() != self.particles.len()
            || values.iter().any(|c| !valid(*c))
            || values
                .iter()
                .zip(&self.particles)
                .any(|(c, p)| self.maxwell_fluids[p.material].is_none() && *c != IDENTITY)
        {
            return Err(Error::InvalidConfig);
        }
        self.conformation = values;
        Ok(())
    }
    pub fn polymer_energy(&self) -> Result<f64, Error> {
        let properties = self.effective_materials()?;
        self.particles
            .iter()
            .enumerate()
            .map(|(i, p)| self.polymer_particle_energy(i, p, properties[i]))
            .sum()
    }
    /// Relaxation heat produced since creation, including heat already deposited in transport.
    pub fn polymer_relaxation_heat(&self) -> f64 {
        self.polymer_heat
    }
    pub(super) fn polymer_particle_energy(
        &self,
        i: usize,
        p: &Particle,
        m: Material,
    ) -> Result<f64, Error> {
        if self.maxwell_fluids[p.material].is_some()
            && ((m.rest_density - self.materials[p.material].rest_density).abs()
                > 1e-12 * self.materials[p.material].rest_density
                || self.gas_active()
                || self.phase_fractions().is_some())
        {
            return Err(Error::InvalidConfig);
        }
        self.maxwell_fluids[p.material].map_or(Ok(0.), |law| {
            let value = p.mass / m.rest_density * law.energy_density(self.conformation[i])?;
            if !value.is_finite() {
                return Err(Error::NumericalFailure);
            }
            Ok(value)
        })
    }
    pub(super) fn add_polymer_forces(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        a: &mut [[f64; 3]],
    ) -> Result<(), Error> {
        if self.maxwell_fluids.iter().all(Option::is_none) {
            return Ok(());
        }
        for (i, p) in particles.iter().enumerate() {
            self.polymer_particle_energy(i, p, properties[i])?;
        }
        let (_, operators, projections) = self.polymer_gradient_operators(particles, pairs);
        let stresses: Vec<_> = particles
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let stress = self.maxwell_fluids[p.material].map_or([[0.; 3]; 3], |law| {
                    difference(self.conformation[i], IDENTITY).map(|r| r.map(|v| v * law.modulus))
                });
                // Adjoint of the rank-aware rotational completion L=G-PG^TQ.
                difference(
                    stress,
                    multiply(
                        multiply(difference(IDENTITY, projections[i]), stress),
                        projections[i],
                    ),
                )
            })
            .collect();
        for &(i, j) in pairs {
            let delta = sub(particles[j].position, particles[i].position);
            let r = norm(delta);
            if r <= 1e-12 {
                continue;
            }
            let w = density_kernel_gradient(self.formulation, self.config.smoothing_radius, r) / r;
            for axis in 0..3 {
                let stress = |k: usize| {
                    self.maxwell_fluids[particles[k].material].map_or(0., |_| {
                        let grad: [f64; 3] = std::array::from_fn(|b| {
                            (0..3).map(|c| operators[k][b][c] * delta[c]).sum()
                        });
                        particles[k].mass / properties[k].rest_density
                            * w
                            * (0..3).map(|b| stresses[k][axis][b] * grad[b]).sum::<f64>()
                    })
                };
                // Negative adjoint of the same least-squares gradient used by C.
                // This pairs polymer deformation work with kinetic work.
                let force = stress(i) + stress(j);
                a[i][axis] += force / particles[i].mass;
                a[j][axis] -= force / particles[j].mass;
            }
        }
        if a.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(())
    }
    fn polymer_gradient_operators(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
    ) -> (Vec<Conformation>, Vec<Conformation>, Vec<Conformation>) {
        let mut covariance = vec![[[0.; 3]; 3]; particles.len()];
        let mut fit = covariance.clone();
        for &(i, j) in pairs {
            let dx = sub(particles[j].position, particles[i].position);
            let dv = sub(particles[j].velocity, particles[i].velocity);
            let r = norm(dx);
            if r <= 1e-12 {
                continue;
            }
            let w = density_kernel_gradient(self.formulation, self.config.smoothing_radius, r) / r;
            for k in [i, j] {
                for a in 0..3 {
                    for b in 0..3 {
                        covariance[k][a][b] += w * dx[a] * dx[b];
                        fit[k][a][b] += w * dv[a] * dx[b];
                    }
                }
            }
        }
        let (operators, projections): (Vec<_>, Vec<_>) =
            covariance.into_iter().map(pseudoinverse).unzip();
        let gradients = fit
            .into_iter()
            .zip(&operators)
            .zip(&projections)
            .map(|((f, b), p)| {
                let g = multiply(f, *b);
                difference(
                    g,
                    multiply(multiply(*p, transpose(g)), difference(IDENTITY, *p)),
                )
            })
            .collect();
        (gradients, operators, projections)
    }
    pub(super) fn advance_polymer_memory(
        &mut self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        transport: Option<&mut super::transport::Transport>,
        dt: f64,
    ) -> Result<(), Error> {
        if self.maxwell_fluids.iter().all(Option::is_none) {
            return Ok(());
        }
        let (gradients, _, _) = self.polymer_gradient_operators(particles, pairs);
        let mut heat = vec![0.; particles.len()];
        for (i, p) in particles.iter().enumerate() {
            if let Some(law) = self.maxwell_fluids[p.material] {
                // Rank-aware completion resolves longitudinal strain and observed
                // spin without inventing normal strain in unobserved directions.
                let (c, q) = law.advance_conformation(self.conformation[i], gradients[i], dt)?;
                self.conformation[i] = c;
                heat[i] = q * p.mass / properties[i].rest_density;
            }
        }
        let produced: f64 = heat.iter().sum();
        self.polymer_heat += produced;
        if !self.polymer_heat.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if let Some(fields) = transport {
            for (i, p) in particles.iter().enumerate() {
                if heat[i] != 0. {
                    let e = fields.energy(p, &fields.fields[i], i)? + heat[i];
                    fields.set_energy(i, p, e)?;
                }
            }
        }
        Ok(())
    }
}
