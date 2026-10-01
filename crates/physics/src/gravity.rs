//! Newtonian mutual gravity in application-defined units.
//! Velocity Verlet is symplectic at a constant timestep. Point masses require
//! either nonzero softening or a collision policy before coincident positions.

pub const SI_GRAVITATIONAL_CONSTANT: f64 = 6.67430e-11;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Positive gravitational and inertial mass.
    pub mass: f64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gravity {
    pub constant: f64,
    /// Plummer softening length; zero gives the exact inverse-square law.
    pub softening: f64,
    pub uniform_acceleration: [f64; 3],
}

impl Default for Gravity {
    fn default() -> Self {
        Self {
            constant: SI_GRAVITATIONAL_CONSTANT,
            softening: 0.0,
            uniform_acceleration: [0.0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    SingularPair,
    NumericalOverflow,
}

/// Conserved quantities for isolated, elastic Newtonian systems.
/// Uniform external gravity contributes potential energy but exchanges momentum.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Diagnostics {
    pub kinetic_energy: f64,
    pub potential_energy: f64,
    pub linear_momentum: [f64; 3],
    pub angular_momentum: [f64; 3],
    pub total_mass: f64,
}

impl Gravity {
    /// Computes translational energy and momenta about the coordinate origin.
    /// Uses the same Plummer potential as the acceleration model.
    /// # Errors
    /// Invalid inputs, singular unsoftened pairs or overflow return an error.
    pub fn diagnostics(self, bodies: &[Body]) -> Result<Diagnostics, Error> {
        self.validate(bodies)?;
        let mut result = Diagnostics::default();
        for body in bodies {
            result.total_mass += body.mass;
            result.kinetic_energy +=
                0.5 * body.mass * body.velocity.iter().map(|v| v * v).sum::<f64>();
            result.potential_energy -= body.mass
                * body
                    .position
                    .iter()
                    .zip(self.uniform_acceleration)
                    .map(|(r, a)| r * a)
                    .sum::<f64>();
            let momentum = body.velocity.map(|v| body.mass * v);
            let r = body.position;
            let angular = [
                r[1] * momentum[2] - r[2] * momentum[1],
                r[2] * momentum[0] - r[0] * momentum[2],
                r[0] * momentum[1] - r[1] * momentum[0],
            ];
            for k in 0..3 {
                result.linear_momentum[k] += momentum[k];
                result.angular_momentum[k] += angular[k];
            }
        }
        if self.constant > 0.0 {
            for i in 0..bodies.len() {
                for j in i + 1..bodies.len() {
                    let distance = (bodies[i]
                        .position
                        .iter()
                        .zip(bodies[j].position)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        + self.softening * self.softening)
                        .sqrt();
                    if distance == 0.0 {
                        return Err(Error::SingularPair);
                    }
                    if !distance.is_finite() {
                        return Err(Error::NumericalOverflow);
                    }
                    result.potential_energy -=
                        self.constant / distance * bodies[i].mass * bodies[j].mass;
                }
            }
        }
        if !result.total_mass.is_finite()
            || !result.kinetic_energy.is_finite()
            || !result.potential_energy.is_finite()
            || result
                .linear_momentum
                .iter()
                .chain(&result.angular_momentum)
                .any(|v| !v.is_finite())
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(result)
    }

    fn validate(self, bodies: &[Body]) -> Result<(), Error> {
        if !self.constant.is_finite()
            || self.constant < 0.0
            || !self.softening.is_finite()
            || self.softening < 0.0
            || self.uniform_acceleration.iter().any(|x| !x.is_finite())
            || bodies.iter().any(|b| {
                !b.mass.is_finite()
                    || b.mass <= 0.0
                    || b.position.iter().chain(&b.velocity).any(|x| !x.is_finite())
            })
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Evaluates each pair once, applying equal and opposite forces.
    /// # Errors
    /// Returns an error for invalid inputs, coincident unsmoothed masses,
    /// or nonfinite intermediate results.
    pub fn accelerations(self, bodies: &[Body]) -> Result<Vec<[f64; 3]>, Error> {
        self.validate(bodies)?;
        let mut acceleration = vec![self.uniform_acceleration; bodies.len()];
        if self.constant == 0.0 {
            return Ok(acceleration);
        }
        for i in 0..bodies.len() {
            for j in i + 1..bodies.len() {
                let delta: [f64; 3] =
                    std::array::from_fn(|k| bodies[j].position[k] - bodies[i].position[k]);
                let radius_squared =
                    delta.iter().map(|x| x * x).sum::<f64>() + self.softening * self.softening;
                if radius_squared == 0.0 {
                    return Err(Error::SingularPair);
                }
                if !radius_squared.is_finite() {
                    return Err(Error::NumericalOverflow);
                }
                let scale = self.constant / radius_squared / radius_squared.sqrt();
                for (k, d) in delta.iter().enumerate() {
                    acceleration[i][k] += d * scale * bodies[j].mass;
                    acceleration[j][k] -= d * scale * bodies[i].mass;
                }
            }
        }
        if acceleration.iter().flatten().any(|x| !x.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(acceleration)
    }

    /// Atomic velocity-Verlet step: errors leave every input body unchanged.
    /// # Errors
    /// Returns an error for a nonpositive/nonfinite timestep, invalid bodies,
    /// singular pairs, or numerical overflow.
    pub fn step(self, bodies: &mut [Body], dt: f64) -> Result<(), Error> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let before = self.accelerations(bodies)?;
        let mut next = bodies.to_vec();
        for (body, acceleration) in next.iter_mut().zip(&before) {
            for (k, a) in acceleration.iter().enumerate() {
                body.velocity[k] += a * (0.5 * dt);
                body.position[k] += body.velocity[k] * dt;
            }
        }
        if next
            .iter()
            .any(|b| b.position.iter().chain(&b.velocity).any(|x| !x.is_finite()))
        {
            return Err(Error::NumericalOverflow);
        }
        let after = self.accelerations(&next)?;
        for (body, acceleration) in next.iter_mut().zip(after) {
            for (k, a) in acceleration.iter().enumerate() {
                body.velocity[k] += a * (0.5 * dt);
            }
        }
        if next
            .iter()
            .any(|b| b.velocity.iter().any(|x| !x.is_finite()))
        {
            return Err(Error::NumericalOverflow);
        }
        bodies.copy_from_slice(&next);
        Ok(())
    }
}
