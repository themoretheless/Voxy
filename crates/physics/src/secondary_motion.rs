//! Inertial secondary motion relative to an accelerating attachment.
//! Exact linear damped-spring integration for acceleration constant during a step.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Natural frequency in Hz, strictly positive.
    pub frequency: f64,
    /// Damping ratio: zero is undamped, one is critical damping.
    pub damping_ratio: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            frequency: 4.0,
            damping_ratio: 0.16,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SecondaryMotion {
    config: Config,
    offset: [f64; 3],
    velocity: [f64; 3],
}
impl SecondaryMotion {
    /// # Errors
    /// Rejects nonfinite or nonpositive frequency and negative damping.
    pub fn new(config: Config) -> Result<Self, &'static str> {
        if !config.frequency.is_finite()
            || config.frequency <= 0.0
            || !config.damping_ratio.is_finite()
            || config.damping_ratio < 0.0
        {
            return Err("invalid spring configuration");
        }
        Ok(Self {
            config,
            offset: [0.0; 3],
            velocity: [0.0; 3],
        })
    }
    #[must_use]
    pub fn offset(&self) -> [f64; 3] {
        self.offset
    }
    #[must_use]
    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }
    pub fn reset(&mut self) {
        self.offset = [0.0; 3];
        self.velocity = [0.0; 3];
    }
    /// Changes material response while preserving current displacement and velocity.
    /// # Errors
    /// Rejects invalid configuration without changing state.
    pub fn configure(&mut self, config: Config) -> Result<(), &'static str> {
        Self::new(config)?;
        self.config = config;
        Ok(())
    }
    /// Anchor acceleration and external acceleration use the same world-space axes.
    /// Pass gravity as external acceleration for static sag; zero gives rest-centred motion.
    /// Rotating frames require caller-computed inertial terms. There is no contact solver.
    /// # Errors
    /// Rejects nonfinite inputs, steps outside (0, 0.1] s, or numerical overflow.
    /// A rejected step leaves all state unchanged.
    pub fn step(
        &mut self,
        dt: f64,
        anchor_acceleration: [f64; 3],
        external_acceleration: [f64; 3],
    ) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 0.1
            || anchor_acceleration
                .iter()
                .chain(&external_acceleration)
                .any(|x| !x.is_finite())
        {
            return Err("invalid spring step");
        }
        let omega = std::f64::consts::TAU * self.config.frequency;
        let decay = self.config.damping_ratio * omega;
        let discriminant = omega * omega - decay * decay;
        let (c, s) = if discriminant.abs() < 1e-12 * omega * omega {
            let e = (-decay * dt).exp();
            (e, e * dt)
        } else if discriminant > 0.0 {
            let f = discriminant.sqrt();
            let e = (-decay * dt).exp();
            (e * (f * dt).cos(), e * (f * dt).sin() / f)
        } else {
            // Exponentials of nonpositive roots avoid overflowing cosh for stiff springs.
            let f = (-discriminant).sqrt();
            let a = (-(decay - f) * dt).exp();
            let b = (-(decay + f) * dt).exp();
            ((a + b) * 0.5, (a - b) / (2.0 * f))
        };
        let mut offset = [0.0; 3];
        let mut velocity = [0.0; 3];
        for axis in 0..3 {
            let equilibrium =
                (external_acceleration[axis] - anchor_acceleration[axis]) / (omega * omega);
            let x = self.offset[axis] - equilibrium;
            let v = self.velocity[axis];
            offset[axis] = equilibrium + (c + decay * s) * x + s * v;
            velocity[axis] = -omega * omega * s * x + (c - decay * s) * v;
        }
        if offset.iter().chain(&velocity).any(|x| !x.is_finite()) {
            return Err("spring overflow");
        }
        self.offset = offset;
        self.velocity = velocity;
        Ok(())
    }
}

/// Half-space contact in attachment-relative coordinates: normal dot offset >= limit.
/// Normals must be unit length. Obstacles are stationary in this relative frame.
#[derive(Clone, Copy, Debug)]
pub struct ContactPlane {
    pub normal: [f64; 3],
    pub limit: f64,
    /// Coulomb coefficient, nonnegative.
    pub friction: f64,
}
impl SecondaryMotion {
    /// Implicit Euler step for a hardening spring, with discrete unilateral contact.
    /// Restoring acceleration is -omega² * (1 + hardening * |offset|²) * offset.
    /// Hardening is in inverse square metres. Zero recovers a linear implicit spring,
    /// not the exact linear integrator used by `step`. Acceleration is held constant.
    /// # Errors
    /// Rejects invalid input, nonconverged/inconsistent contacts, and numerical overflow.
    /// State is unchanged on every failure. Contact is discrete, without CCD.
    pub fn step_nonlinear(
        &mut self,
        dt: f64,
        anchor_acceleration: [f64; 3],
        external_acceleration: [f64; 3],
        hardening: f64,
        contacts: &[ContactPlane],
    ) -> Result<(), &'static str> {
        let dot =
            |a: [f64; 3], b: [f64; 3]| -> f64 { a.into_iter().zip(b).map(|(x, y)| x * y).sum() };
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 0.1
            || !hardening.is_finite()
            || hardening < 0.0
            || anchor_acceleration
                .iter()
                .chain(&external_acceleration)
                .any(|x| !x.is_finite())
            || contacts.iter().any(|p| {
                p.normal.iter().any(|x| !x.is_finite())
                    || (dot(p.normal, p.normal) - 1.0).abs() > 1e-8
                    || !p.limit.is_finite()
                    || !p.friction.is_finite()
                    || p.friction < 0.0
            })
        {
            return Err("invalid nonlinear step");
        }
        let omega = std::f64::consts::TAU * self.config.frequency;
        let drag = 1.0 + 2.0 * self.config.damping_ratio * omega * dt;
        let linear = drag + omega * omega * dt * dt;
        let cubic = omega * omega * dt * dt * hardening;
        let rhs: [f64; 3] = std::array::from_fn(|i| {
            drag * self.offset[i]
                + dt * self.velocity[i]
                + dt * dt * (external_acceleration[i] - anchor_acceleration[i])
        });
        let magnitude = rhs.into_iter().fold(0.0_f64, f64::hypot);
        if !linear.is_finite() || !cubic.is_finite() || !magnitude.is_finite() {
            return Err("nonlinear overflow");
        }
        // Positive cubic is monotone, giving a unique implicit solution and safe bracket.
        let mut low = 0.0;
        let mut high = magnitude / linear;
        for _ in 0..80 {
            let r = (low + high) * 0.5;
            if linear * r + cubic * r * r * r > magnitude {
                high = r;
            } else {
                low = r;
            }
        }
        let radius = (low + high) * 0.5;
        let mut position = if magnitude == 0.0 {
            [0.0; 3]
        } else {
            rhs.map(|x| x / magnitude * radius)
        };
        for _ in 0..32 {
            for p in contacts {
                let penetration = p.limit - dot(p.normal, position);
                if penetration > 0.0 {
                    for (x, n) in position.iter_mut().zip(p.normal) {
                        *x += penetration * n;
                    }
                }
            }
        }
        if contacts
            .iter()
            .any(|p| dot(p.normal, position) < p.limit - 1e-10)
        {
            return Err("inconsistent contacts");
        }
        let mut velocity = std::array::from_fn(|i| (position[i] - self.offset[i]) / dt);
        for _ in 0..8 {
            for p in contacts {
                if dot(p.normal, position) <= p.limit + 1e-10 {
                    let incoming = dot(p.normal, velocity).min(0.0);
                    for (v, n) in velocity.iter_mut().zip(p.normal) {
                        *v -= incoming * n;
                    }
                    let normal_speed = dot(p.normal, velocity);
                    let tangent: [f64; 3] =
                        std::array::from_fn(|i| velocity[i] - normal_speed * p.normal[i]);
                    let speed = tangent.into_iter().fold(0.0_f64, f64::hypot);
                    if speed > 0.0 {
                        let fraction = (p.friction * (-incoming) / speed).min(1.0);
                        for (v, t) in velocity.iter_mut().zip(tangent) {
                            *v -= fraction * t;
                        }
                    }
                }
            }
        }
        if position.iter().chain(&velocity).any(|x| !x.is_finite()) {
            return Err("nonlinear overflow");
        }
        self.offset = position;
        self.velocity = velocity;
        Ok(())
    }
}
