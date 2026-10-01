//! Principal-axis rigid rotation, with inertial angular momentum.
use crate::astrophysics::Error;
type Vector = [f64; 3];
fn cross(a: Vector, b: Vector) -> Vector {
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
fn rotate(q: [f64; 4], v: Vector) -> Vector {
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
        self.validate()?;
        let q = self.orientation;
        let local = rotate([-q[0], -q[1], -q[2], q[3]], self.angular_momentum);
        let omega = rotate(q, std::array::from_fn(|k| local[k] / self.inertia[k]));
        if omega.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(omega)
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
        self.validate()?;
        if !dt.is_finite() || dt <= 0.0 || torque.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let midpoint_momentum =
            std::array::from_fn(|k| self.angular_momentum[k] + torque[k] * dt * 0.5);
        let end_momentum = std::array::from_fn(|k| self.angular_momentum[k] + torque[k] * dt);
        let mut half = *self;
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
            ..*self
        };
        next.validate()?;
        *self = next;
        Ok(())
    }
}
