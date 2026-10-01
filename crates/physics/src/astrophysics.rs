//! Two-body celestial mechanics in caller-selected units.
//! `mu = G * (primary_mass + secondary_mass)` for relative motion. Perturbed
//! multi-body motion belongs to `gravity`; these conics assume an isolated pair.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitalState {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub specific_energy: f64,
    pub angular_momentum: [f64; 3],
    pub eccentricity_vector: [f64; 3],
    pub eccentricity: f64,
    pub semilatus_rectum: f64,
    pub periapsis: f64,
    /// None for unbound motion.
    pub apoapsis: Option<f64>,
    pub period: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    DegenerateOrbit,
    NumericalOverflow,
    NoConvergence,
}

/// Leading-order torque from a symmetric gravitational acceleration gradient.
/// Both the inertia tensor and field tensor must be expressed in the same frame.
/// This is a conservative gravity-gradient torque, without tidal dissipation.
/// # Errors
/// Nonfinite/asymmetric tensors or nonpositive inertia return an error.
pub fn gravity_gradient_torque(
    tidal: [[f64; 3]; 3],
    inertia: [[f64; 3]; 3],
) -> Result<[f64; 3], Error> {
    if tidal
        .iter()
        .flatten()
        .chain(inertia.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput);
    }
    for i in 0..3 {
        for j in 0..3 {
            if (tidal[i][j] - tidal[j][i]).abs() > 1e-12 * (1.0 + tidal[i][j].abs())
                || (inertia[i][j] - inertia[j][i]).abs() > 1e-12 * (1.0 + inertia[i][j].abs())
            {
                return Err(Error::InvalidInput);
            }
        }
    }
    let minor = inertia[0][0] * inertia[1][1] - inertia[0][1] * inertia[0][1];
    let determinant = inertia[0][0]
        * (inertia[1][1] * inertia[2][2] - inertia[1][2] * inertia[2][1])
        - inertia[0][1] * (inertia[1][0] * inertia[2][2] - inertia[1][2] * inertia[2][0])
        + inertia[0][2] * (inertia[1][0] * inertia[2][1] - inertia[1][1] * inertia[2][0]);
    if !minor.is_finite() || !determinant.is_finite() {
        return Err(Error::NumericalOverflow);
    }
    if inertia[0][0] <= 0.0 || minor <= 0.0 || determinant <= 0.0 {
        return Err(Error::InvalidInput);
    }
    let torque: [f64; 3] = std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        (0..3)
            .map(|l| inertia[k][l] * tidal[j][l] - inertia[j][l] * tidal[k][l])
            .sum()
    });
    if torque.iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericalOverflow);
    }
    Ok(torque)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn validate(state: OrbitalState, mu: f64) -> Result<(), Error> {
    if !mu.is_finite()
        || mu <= 0.0
        || state
            .position
            .iter()
            .chain(&state.velocity)
            .any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput);
    }
    let radius = length(state.position);
    if radius == 0.0 {
        return Err(Error::DegenerateOrbit);
    }
    if !radius.is_finite() {
        return Err(Error::NumericalOverflow);
    }
    Ok(())
}

/// Computes orbit invariants without ambiguous angles for circular/equatorial orbits.
/// # Errors
/// Invalid mu/state, rectilinear motion, or numerical overflow.
pub fn orbit(state: OrbitalState, mu: f64) -> Result<Orbit, Error> {
    validate(state, mu)?;
    let radius = length(state.position);
    let angular_momentum = cross(state.position, state.velocity);
    let h = length(angular_momentum);
    if h == 0.0 {
        return Err(Error::DegenerateOrbit);
    }
    let vector = cross(state.velocity, angular_momentum);
    let eccentricity_vector = std::array::from_fn(|k| vector[k] / mu - state.position[k] / radius);
    let eccentricity = length(eccentricity_vector);
    let specific_energy = 0.5 * dot(state.velocity, state.velocity) - mu / radius;
    let semilatus_rectum = h / mu * h;
    let periapsis = semilatus_rectum / (1.0 + eccentricity);
    let (apoapsis, period) = if specific_energy < 0.0 && eccentricity < 1.0 {
        let major = -mu / (2.0 * specific_energy);
        (
            Some(semilatus_rectum / (1.0 - eccentricity)),
            Some(std::f64::consts::TAU * major * (major / mu).sqrt()),
        )
    } else {
        (None, None)
    };
    if !specific_energy.is_finite()
        || !h.is_finite()
        || !eccentricity.is_finite()
        || !semilatus_rectum.is_finite()
        || !periapsis.is_finite()
        || apoapsis.is_some_and(|v| !v.is_finite())
        || period.is_some_and(|v| !v.is_finite())
    {
        return Err(Error::NumericalOverflow);
    }
    Ok(Orbit {
        specific_energy,
        angular_momentum,
        eccentricity_vector,
        eccentricity,
        semilatus_rectum,
        periapsis,
        apoapsis,
        period,
    })
}

/// An oriented elliptic, parabolic or hyperbolic conic, in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conic {
    pub periapsis: f64,
    pub eccentricity: f64,
    pub inclination: f64,
    pub ascending_node: f64,
    pub argument_periapsis: f64,
    pub true_anomaly: f64,
}
impl Conic {
    /// Converts geometrical elements to a Cartesian relative state.
    /// # Errors
    /// Invalid elements, anomaly outside a hyperbolic branch, or overflow.
    pub fn state(self, mu: f64) -> Result<OrbitalState, Error> {
        if !mu.is_finite()
            || mu <= 0.0
            || !self.periapsis.is_finite()
            || self.periapsis <= 0.0
            || !self.eccentricity.is_finite()
            || self.eccentricity < 0.0
            || [
                self.inclination,
                self.ascending_node,
                self.argument_periapsis,
                self.true_anomaly,
            ]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidInput);
        }
        let parameter = self.periapsis * (1.0 + self.eccentricity);
        let (sin, cos) = self.true_anomaly.sin_cos();
        let denominator = 1.0 + self.eccentricity * cos;
        if denominator <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let radius = parameter / denominator;
        let speed = (mu / parameter).sqrt();
        let (node_sin, node_cos) = self.ascending_node.sin_cos();
        let (peri_sin, peri_cos) = self.argument_periapsis.sin_cos();
        let (inc_sin, inc_cos) = self.inclination.sin_cos();
        let first = [
            node_cos * peri_cos - node_sin * peri_sin * inc_cos,
            node_sin * peri_cos + node_cos * peri_sin * inc_cos,
            peri_sin * inc_sin,
        ];
        let second = [
            -node_cos * peri_sin - node_sin * peri_cos * inc_cos,
            -node_sin * peri_sin + node_cos * peri_cos * inc_cos,
            peri_cos * inc_sin,
        ];
        let state = OrbitalState {
            position: std::array::from_fn(|k| radius * (cos * first[k] + sin * second[k])),
            velocity: std::array::from_fn(|k| {
                speed * (-sin * first[k] + (self.eccentricity + cos) * second[k])
            }),
        };
        validate(state, mu)?;
        Ok(state)
    }
}

fn stumpff(z: f64) -> Result<(f64, f64), Error> {
    let (c, s) = if z.abs() < 1e-4 {
        // Series avoids cancellation near parabolic motion.
        (
            0.5 - z / 24.0 + z * z / 720.0 - z * z * z / 40320.0,
            1.0 / 6.0 - z / 120.0 + z * z / 5040.0 - z * z * z / 362_880.0,
        )
    } else if z > 0.0 {
        let root = z.sqrt();
        ((1.0 - root.cos()) / z, (root - root.sin()) / (root * z))
    } else {
        let root = (-z).sqrt();
        (
            (root.cosh() - 1.0) / (-z),
            (root.sinh() - root) / (root * (-z)),
        )
    };
    if !c.is_finite() || !s.is_finite() {
        return Err(Error::NumericalOverflow);
    }
    Ok((c, s))
}

/// Propagates an isolated, nonrectilinear two-body state with universal variables.
/// A bracketed Newton solve covers elliptic, parabolic and hyperbolic motion,
/// including negative elapsed time. Collision radii are outside this model.
/// # Errors
/// Invalid inputs, radial/singular motion, unrepresentable intermediates or
/// exhaustion of the bounded root solver.
pub fn propagate(state: OrbitalState, mu: f64, dt: f64) -> Result<OrbitalState, Error> {
    validate(state, mu)?;
    if !dt.is_finite() {
        return Err(Error::InvalidInput);
    }
    if length(cross(state.position, state.velocity)) == 0.0 {
        return Err(Error::DegenerateOrbit);
    }
    if dt == 0.0 {
        return Ok(state);
    }
    let radius = length(state.position);
    let root_mu = mu.sqrt();
    let alpha = 2.0 / radius - dot(state.velocity, state.velocity) / mu;
    let radial = dot(state.position, state.velocity) / root_mu;
    let target = root_mu * dt;
    let evaluate = |chi: f64| -> Result<(f64, f64), Error> {
        let z = alpha * chi * chi;
        let (c, s) = stumpff(z)?;
        let value =
            radial * chi * chi * c + (1.0 - alpha * radius) * chi * chi * chi * s + radius * chi
                - target;
        let derivative =
            radial * chi * (1.0 - z * s) + (1.0 - alpha * radius) * chi * chi * c + radius;
        if !value.is_finite() || !derivative.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        Ok((value, derivative))
    };
    let mut endpoint = (target / radius).abs().max(1e-8) * dt.signum();
    let mut bracketed = false;
    for _ in 0..128 {
        let value = evaluate(endpoint)?.0;
        if value * dt.signum() >= 0.0 {
            bracketed = true;
            break;
        }
        endpoint *= 2.0;
    }
    if !bracketed {
        return Err(Error::NoConvergence);
    }
    let (mut low, mut high) = (endpoint.min(0.0), endpoint.max(0.0));
    let mut chi = (low + high) * 0.5;
    let mut converged = false;
    for _ in 0..128 {
        let (value, derivative) = evaluate(chi)?;
        if value.abs() <= 1e-12 * target.abs().max(1.0) {
            converged = true;
            break;
        }
        if value > 0.0 {
            high = chi;
        } else {
            low = chi;
        }
        let candidate = chi - value / derivative;
        chi = if candidate.is_finite() && candidate > low && candidate < high {
            candidate
        } else {
            (low + high) * 0.5
        };
    }
    if !converged {
        return Err(Error::NoConvergence);
    }
    let (c, s) = stumpff(alpha * chi * chi)?;
    let f = 1.0 - chi * chi / radius * c;
    let g = dt - chi * chi * chi / root_mu * s;
    let position = std::array::from_fn(|k| f * state.position[k] + g * state.velocity[k]);
    let end_radius = length(position);
    if end_radius == 0.0 {
        return Err(Error::DegenerateOrbit);
    }
    let fdot = root_mu / end_radius / radius * (alpha * chi * chi * chi * s - chi);
    let gdot = 1.0 - chi * chi / end_radius * c;
    let next = OrbitalState {
        position,
        velocity: std::array::from_fn(|k| fdot * state.position[k] + gdot * state.velocity[k]),
    };
    validate(next, mu)?;
    Ok(next)
}
