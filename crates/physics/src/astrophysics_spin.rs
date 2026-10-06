//! Principal-axis rigid rotation, with inertial angular momentum.
use crate::astrophysics::Error;
type Vector = [f64; 3];
pub(crate) fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn multiply(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let c = cross([a[0], a[1], a[2]], [b[0], b[1], b[2]]);
    [
        a[3] * b[0] + b[3] * a[0] + c[0],
        a[3] * b[1] + b[3] * a[1] + c[1],
        a[3] * b[2] + b[3] * a[2] + c[2],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}
pub(crate) fn rotate(q: [f64; 4], v: Vector) -> Vector {
    let axis = [q[0], q[1], q[2]];
    let first = cross(axis, v);
    let second = cross(axis, first);
    std::array::from_fn(|k| v[k] + 2.0 * (q[3] * first[k] + second[k]))
}
fn advance(q: [f64; 4], omega: Vector, dt: f64) -> Result<[f64; 4], Error> {
    let speed = omega[0].hypot(omega[1]).hypot(omega[2]);
    if !speed.is_finite() {
        return Err(Error::NumericalOverflow);
    }
    if speed == 0.0 {
        return Ok(q);
    }
    let (sin, cos) = (speed * dt * 0.5).sin_cos();
    let next = multiply(
        [
            omega[0] / speed * sin,
            omega[1] / speed * sin,
            omega[2] / speed * sin,
            cos,
        ],
        q,
    );
    let norm = next[0].hypot(next[1]).hypot(next[2]).hypot(next[3]);
    if !norm.is_finite() || norm == 0.0 {
        return Err(Error::NumericalOverflow);
    }
    Ok(next.map(|v| v / norm))
}
/// World torque τ(t) = value + rate*t + acceleration*t²/2 + jerk*t³/6 + snap*t⁴/24.
/// Coefficients are about COM and time is relative to the current interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TorquePolynomial {
    pub value: [f64; 3],
    pub rate: [f64; 3],
    pub acceleration: [f64; 3],
    pub jerk: [f64; 3],
    pub snap: [f64; 3],
}
impl TorquePolynomial {
    pub fn constant(value: [f64; 3]) -> Self {
        Self {
            value,
            rate: [0.; 3],
            acceleration: [0.; 3],
            jerk: [0.; 3],
            snap: [0.; 3],
        }
    }
    /// Moment of a constant world force on an application arm that translates
    /// quadratically relative to COM. Arms and derivatives share world axes.
    pub fn moving_arm(
        arm: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        force: [f64; 3],
    ) -> Result<Self, Error> {
        if arm
            .iter()
            .chain(&velocity)
            .chain(&acceleration)
            .chain(&force)
            .any(|x| !x.is_finite())
        {
            return Err(Error::InvalidInput);
        }
        let result = Self {
            value: cross(arm, force),
            rate: cross(velocity, force),
            acceleration: cross(acceleration, force),
            ..Self::constant([0.; 3])
        };
        result.validate().map_err(|_| Error::NumericalOverflow)?;
        Ok(result)
    }
    /// Moment r(t) × F(t) for cubic world arm and affine force.
    /// Derivative coefficients use the same factorial convention as this law.
    pub fn moving_affine_arm(
        arm: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        jerk: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
    ) -> Result<Self, Error> {
        if arm
            .iter()
            .chain(&velocity)
            .chain(&acceleration)
            .chain(&jerk)
            .chain(&force)
            .chain(&force_rate)
            .any(|x| !x.is_finite())
        {
            return Err(Error::InvalidInput);
        }
        let add = |a: [f64; 3], b: [f64; 3], scale: f64| {
            std::array::from_fn(|k| scale.mul_add(b[k], a[k]))
        };
        let result = Self {
            value: cross(arm, force),
            rate: add(cross(velocity, force), cross(arm, force_rate), 1.),
            acceleration: add(cross(acceleration, force), cross(velocity, force_rate), 2.),
            jerk: add(cross(jerk, force), cross(acceleration, force_rate), 3.),
            snap: cross(jerk, force_rate).map(|x| x * 4.),
        };
        result.validate().map_err(|_| Error::NumericalOverflow)?;
        Ok(result)
    }
    /// Evaluate the nominal world torque at a nonnegative time.
    pub fn value_at(self, time: f64) -> Result<[f64; 3], Error> {
        self.shifted(time).map(|law| law.value)
    }
    fn validate(self) -> Result<(), Error> {
        if self
            .value
            .iter()
            .chain(&self.rate)
            .chain(&self.acceleration)
            .chain(&self.jerk)
            .chain(&self.snap)
            .any(|x| !x.is_finite())
        {
            Err(Error::InvalidInput)
        } else {
            Ok(())
        }
    }
    /// Exact polynomial angular impulse in nominal floating arithmetic.
    pub fn impulse(self, time: f64) -> Result<[f64; 3], Error> {
        self.validate()?;
        if !time.is_finite() || time < 0. {
            return Err(Error::InvalidInput);
        }
        let impulse: [f64; 3] = std::array::from_fn(|k| {
            let acceleration = if self.jerk == [0.; 3] && self.snap == [0.; 3] {
                self.acceleration[k]
            } else {
                self.snap[k]
                    .mul_add(time / 5., self.jerk[k])
                    .mul_add(time / 4., self.acceleration[k])
            };
            let rate = acceleration.mul_add(time / 3., self.rate[k]);
            rate.mul_add(time * 0.5, self.value[k]) * time
        });
        if impulse.iter().any(|x| !x.is_finite()) {
            Err(Error::NumericalOverflow)
        } else {
            Ok(impulse)
        }
    }
    /// Rebase this law to a later interval without changing its world values.
    pub fn shifted(self, time: f64) -> Result<Self, Error> {
        self.validate()?;
        if !time.is_finite() || time < 0. {
            return Err(Error::InvalidInput);
        }
        let shifted = Self {
            value: std::array::from_fn(|k| {
                self.snap[k]
                    .mul_add(time / 4., self.jerk[k])
                    .mul_add(time / 3., self.acceleration[k])
                    .mul_add(time * 0.5, self.rate[k])
                    .mul_add(time, self.value[k])
            }),
            rate: std::array::from_fn(|k| {
                self.snap[k]
                    .mul_add(time / 3., self.jerk[k])
                    .mul_add(time * 0.5, self.acceleration[k])
                    .mul_add(time, self.rate[k])
            }),
            acceleration: std::array::from_fn(|k| {
                self.snap[k]
                    .mul_add(time * 0.5, self.jerk[k])
                    .mul_add(time, self.acceleration[k])
            }),
            jerk: std::array::from_fn(|k| self.snap[k].mul_add(time, self.jerk[k])),
            snap: self.snap,
        };
        shifted.validate().map_err(|_| Error::NumericalOverflow)?;
        Ok(shifted)
    }
    /// Triangle bound on |τ| and on the accumulated |τ| integral over a prefix.
    /// Bounds support model error admission; they are not directed rounding.
    pub(crate) fn envelopes(self, duration: f64) -> Result<(f64, f64), Error> {
        self.validate()?;
        if !duration.is_finite() || duration < 0. {
            return Err(Error::InvalidInput);
        }
        let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
        let base = norm(self.value);
        let rate = norm(self.rate);
        let acceleration = norm(self.acceleration);
        let maximum = base
            + rate * duration
            + (acceleration * (duration * 0.5)) * duration
            + ((norm(self.jerk) * (duration / 6.)) * duration) * duration
            + (((norm(self.snap) * (duration / 24.)) * duration) * duration) * duration;
        let integral = base * duration
            + (rate * (duration * 0.5)) * duration
            + ((acceleration * (duration / 6.)) * duration) * duration
            + (((norm(self.jerk) * (duration / 24.)) * duration) * duration) * duration
            + ((((norm(self.snap) * (duration / 120.)) * duration) * duration) * duration)
                * duration;
        if !maximum.is_finite() || !integral.is_finite() {
            Err(Error::NumericalOverflow)
        } else {
            Ok((maximum, integral))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spin {
    /// Body-to-world unit quaternion [x,y,z,w].
    pub orientation: [f64; 4],
    pub angular_momentum: Vector,
    /// Physical positive principal moments in the body frame.
    pub inertia: Vector,
}
impl Spin {
    fn validate(self) -> Result<(), Error> {
        let norm = self.orientation[0]
            .hypot(self.orientation[1])
            .hypot(self.orientation[2])
            .hypot(self.orientation[3]);
        if self
            .orientation
            .iter()
            .chain(&self.angular_momentum)
            .chain(&self.inertia)
            .any(|v| !v.is_finite())
            || (norm - 1.0).abs() > 1e-9
            || self.inertia.iter().any(|v| *v <= 0.0)
            || (0..3)
                .any(|k| self.inertia[k] > self.inertia[(k + 1) % 3] + self.inertia[(k + 2) % 3])
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    /// # Errors
    /// Invalid state or overflow in the inertia transform.
    pub fn angular_velocity(self) -> Result<Vector, Error> {
        self.inverse_inertia(self.angular_momentum)
    }
    /// # Errors
    /// Invalid state or unrepresentable kinetic energy.
    pub fn energy(self) -> Result<f64, Error> {
        let omega = self.angular_velocity()?;
        let energy = 0.5
            * self
                .angular_momentum
                .iter()
                .zip(omega)
                .map(|(a, b)| a * b)
                .sum::<f64>();
        if !energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        Ok(energy)
    }
    /// Apply inverse world inertia to a world-frame angular impulse or torque.
    /// # Errors
    /// Invalid inertia/orientation or an unrepresentable transformed vector.
    pub fn inverse_inertia(self, vector: Vector) -> Result<Vector, Error> {
        self.validate()?;
        if vector.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let q = self.orientation;
        let local = rotate([-q[0], -q[1], -q[2], q[3]], vector);
        let result = rotate(q, std::array::from_fn(|k| local[k] / self.inertia[k]));
        if result.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(result)
    }

    /// Instantaneous world angular acceleration under a COM torque.
    pub fn angular_acceleration(self, torque: Vector) -> Result<Vector, Error> {
        let omega = self.angular_velocity()?;
        let gyro = cross(omega, self.angular_momentum);
        self.inverse_inertia(std::array::from_fn(|k| torque[k] - gyro[k]))
    }

    /// World-frame inertia, for gravity-gradient torque.
    /// # Errors
    /// Invalid state or numerical overflow.
    pub fn world_inertia(self) -> Result<[[f64; 3]; 3], Error> {
        self.validate()?;
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
            .map(|v| rotate(self.orientation, v));
        let tensor: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                (0..3)
                    .map(|k| self.inertia[k] * axes[k][i] * axes[k][j])
                    .sum()
            })
        });
        if tensor.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(tensor)
    }
    /// Constant world torque with a bounded implicit midpoint attitude solve.
    /// Reduce dt if the midpoint iteration does not converge.
    /// # Errors
    /// Invalid input, overflow or failed convergence leave state unchanged.
    pub fn step(&mut self, torque: Vector, dt: f64) -> Result<(), Error> {
        *self = self.prepare_arc(torque, dt)?.end;
        Ok(())
    }

    /// Prepare the same implicit midpoint step as a sampleable constant-axis arc.
    /// This arc is the numerical path, not an exact anisotropic free-spin orbit.
    pub fn prepare_arc(self, torque: Vector, dt: f64) -> Result<SpinArc, Error> {
        self.prepare_polynomial_arc(TorquePolynomial::constant(torque), dt)
    }

    /// Same midpoint attitude integrator with exactly integrated polynomial
    /// world angular momentum. Orientation remains an admitted numerical arc.
    pub fn prepare_polynomial_arc(
        self,
        torque: TorquePolynomial,
        dt: f64,
    ) -> Result<SpinArc, Error> {
        self.validate()?;
        torque.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let midpoint_impulse = torque.impulse(dt * 0.5)?;
        let end_impulse = torque.impulse(dt)?;
        let midpoint_momentum =
            std::array::from_fn(|k| self.angular_momentum[k] + midpoint_impulse[k]);
        let end_momentum = std::array::from_fn(|k| self.angular_momentum[k] + end_impulse[k]);
        let mut half = self;
        half.angular_momentum = midpoint_momentum;
        let mut converged = false;
        for _ in 0..32 {
            let next = advance(self.orientation, half.angular_velocity()?, dt * 0.5)?;
            let difference = next
                .iter()
                .zip(half.orientation)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max);
            half.orientation = next;
            if difference < 1e-13 {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(Error::NoConvergence);
        }
        let next = Self {
            orientation: advance(self.orientation, half.angular_velocity()?, dt)?,
            angular_momentum: end_momentum,
            ..self
        };
        next.validate()?;
        Ok(SpinArc {
            start: self,
            end: next,
            torque,
            duration: dt,
            angular_velocity: half.angular_velocity()?,
        })
    }
}

/// Immutable implicit-midpoint arc, with its exact accepted endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinArc {
    start: Spin,
    end: Spin,
    torque: TorquePolynomial,
    duration: f64,
    angular_velocity: Vector,
}
impl SpinArc {
    pub fn start(self) -> Spin {
        self.start
    }
    pub fn end(self) -> Spin {
        self.end
    }
    pub fn duration(self) -> f64 {
        self.duration
    }
    pub fn angular_velocity(self) -> Vector {
        self.angular_velocity
    }
    /// World torque at the beginning of the arc.
    pub fn torque(self) -> Vector {
        self.torque.value
    }
    pub fn torque_polynomial(self) -> TorquePolynomial {
        self.torque
    }
    /// Sample the admitted arc; preserve accepted endpoints exactly.
    pub fn sample(self, time: f64) -> Result<Spin, Error> {
        if !time.is_finite() || !(0. ..=self.duration).contains(&time) {
            return Err(Error::InvalidInput);
        }
        if time == 0. {
            return Ok(self.start);
        }
        if time == self.duration {
            return Ok(self.end);
        }
        let impulse = self.torque.impulse(time)?;
        let state = Spin {
            orientation: advance(self.start.orientation, self.angular_velocity, time)?,
            angular_momentum: std::array::from_fn(|k| self.start.angular_momentum[k] + impulse[k]),
            ..self.start
        };
        state.validate()?;
        Ok(state)
    }
}
