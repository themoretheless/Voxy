use super::{
    Error, Liquid, RotatingBoundaryReport, Rotation, TranslatingBody, cross, finite, norm, positive,
};
type Matrix = [[f64; 3]; 3];
/// General rigid-body inertia in body coordinates, orientation [w,x,y,z], and
/// world-space angular momentum. SPH samples are supplied in lab coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TensorBody {
    pub translation: TranslatingBody,
    pub inertia: Matrix,
    pub orientation: [f64; 4],
    pub angular_momentum: [f64; 3],
}
impl TensorBody {
    /// # Errors
    /// Invalid positive-definite tensor, non-unit quaternion or nonfinite momentum.
    pub fn angular_velocity(self) -> Result<[f64; 3], Error> {
        let state = TensorRotation::new(self)?;
        Ok(state.mobility(state.momentum))
    }
    /// # Errors
    /// Invalid inertia/orientation/momentum or energy overflow.
    pub fn rotational_energy(self) -> Result<f64, Error> {
        let velocity = self.angular_velocity()?;
        let energy = 0.5
            * self
                .angular_momentum
                .iter()
                .zip(velocity)
                .map(|(l, w)| l * w)
                .sum::<f64>();
        if !energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TensorRotation {
    inverse_body: Matrix,
    pub(super) orientation: [f64; 4],
    pub(super) momentum: [f64; 3],
}
impl Liquid {
    /// General-inertia SPH body coupling, including torque-free gyroscopic motion.
    /// Uses bounded implicit-midpoint Euler-top dynamics and a matching Cayley
    /// orientation update. All samples belong to this body. No rotating CCD yet.
    /// # Errors
    /// Invalid body/tensor/quaternion, budgets, nonlinear nonconvergence, or
    /// numerical/thermal failure. Body and all fluid/sample state roll back together.
    pub fn step_with_tensor_boundary(
        &mut self,
        dt: f64,
        body: &mut TensorBody,
    ) -> Result<RotatingBoundaryReport, Error> {
        let tensor = TensorRotation::new(*body)?;
        let mut candidate = self.prepare_boundary_body(body.translation)?;
        candidate
            .boundary_coupling
            .as_mut()
            .ok_or(Error::InvalidBoundary)?
            .rotation = Some(Rotation {
            velocity: tensor.mobility(tensor.momentum),
            inertia: 1.0,
            impulse: [0.0; 3],
            tensor: Some(tensor),
        });
        candidate.update_rotating_surface_velocities()?;
        let fluid = candidate.advance(dt, None, |particles, time| {
            for p in particles {
                for a in 0..3 {
                    p.position[a] += p.velocity[a] * time;
                }
            }
            Ok(())
        })?;
        let rotation = candidate
            .boundary_coupling
            .as_ref()
            .and_then(|c| c.rotation)
            .ok_or(Error::InvalidBoundary)?;
        let tensor = rotation.tensor.ok_or(Error::InvalidBoundary)?;
        let mut translation = body.translation;
        let boundary = candidate.finish_boundary_body(fluid, &mut translation)?;
        let result = TensorBody {
            translation,
            inertia: body.inertia,
            orientation: tensor.orientation,
            angular_momentum: tensor.momentum,
        };
        result.rotational_energy()?;
        *body = result;
        *self = candidate;
        Ok(RotatingBoundaryReport {
            boundary,
            angular_impulse: rotation.impulse,
        })
    }
}
impl TensorRotation {
    fn new(body: TensorBody) -> Result<Self, Error> {
        if !finite(body.angular_momentum) {
            return Err(Error::InvalidBoundary);
        }
        let length = body.orientation.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !length.is_finite() || (length - 1.0).abs() > 1e-10 {
            return Err(Error::InvalidBoundary);
        }
        let orientation = body.orientation.map(|q| q / length);
        let inverse_body = inverse_spd(body.inertia)?;
        let state = Self {
            inverse_body,
            orientation,
            momentum: body.angular_momentum,
        };
        if !finite(state.mobility(state.momentum)) {
            return Err(Error::NumericalFailure);
        }
        Ok(state)
    }
    pub(super) fn mobility(self, vector: [f64; 3]) -> [f64; 3] {
        let rotation = quaternion_matrix(self.orientation);
        matvec(
            rotation,
            matvec(self.inverse_body, matvec(transpose(rotation), vector)),
        )
    }
    pub(super) fn compliance_bound(self) -> f64 {
        self.inverse_body
            .iter()
            .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
            .fold(0.0, f64::max)
    }
    pub(super) fn drift(&mut self, dt: f64) -> Result<Matrix, Error> {
        let initial_rotation = quaternion_matrix(self.orientation);
        let initial = matvec(transpose(initial_rotation), self.momentum);
        let mut final_momentum = initial;
        let tolerance = 128.0 * f64::EPSILON * norm(initial).max(f64::MIN_POSITIVE);
        let mut converged = false;
        for _ in 0..96 {
            let middle = std::array::from_fn(|a| 0.5 * (initial[a] + final_momentum[a]));
            let derivative = cross(middle, matvec(self.inverse_body, middle));
            let next: [f64; 3] = std::array::from_fn(|a| initial[a] + dt * derivative[a]);
            let residual = norm(std::array::from_fn(|a| next[a] - final_momentum[a]));
            if !finite(next) || !residual.is_finite() {
                return Err(Error::NumericalFailure);
            }
            final_momentum = next;
            if residual <= tolerance {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(Error::NumericalFailure);
        }
        let middle = std::array::from_fn(|a| 0.5 * (initial[a] + final_momentum[a]));
        let tangent = matvec(self.inverse_body, middle).map(|w| 0.5 * dt * w);
        let increment = normalize([1.0, tangent[0], tangent[1], tangent[2]])?;
        self.orientation = normalize(quaternion_product(self.orientation, increment))?;
        let final_rotation = quaternion_matrix(self.orientation);
        if !finite(self.mobility(self.momentum)) {
            return Err(Error::NumericalFailure);
        }
        Ok(multiply(final_rotation, transpose(initial_rotation)))
    }
}
fn inverse_spd(matrix: Matrix) -> Result<Matrix, Error> {
    if matrix.iter().any(|r| !finite(*r)) {
        return Err(Error::InvalidBoundary);
    }
    let scale = matrix.iter().flatten().map(|x| x.abs()).fold(0.0, f64::max);
    if !positive(scale) {
        return Err(Error::InvalidBoundary);
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().take(i).enumerate() {
            if (*value - matrix[j][i]).abs() > 32.0 * f64::EPSILON * scale {
                return Err(Error::InvalidBoundary);
            }
        }
    }
    let mut lower = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let mut value = (0.5 * (matrix[i][j] / scale + matrix[j][i] / scale))
                - (0..j).map(|k| lower[i][k] * lower[j][k]).sum::<f64>();
            if i == j {
                if !positive(value) {
                    return Err(Error::InvalidBoundary);
                }
                value = value.sqrt();
            } else {
                value /= lower[j][j];
            }
            lower[i][j] = value;
        }
    }
    let inverse = transpose(std::array::from_fn(|column| {
        let mut y = [0.0; 3];
        for i in 0..3 {
            y[i] = (f64::from(i == column) - (0..i).map(|j| lower[i][j] * y[j]).sum::<f64>())
                / lower[i][i];
        }
        let mut x = [0.0; 3];
        for i in (0..3).rev() {
            x[i] = (y[i] - (i + 1..3).map(|j| lower[j][i] * x[j]).sum::<f64>()) / lower[i][i];
        }
        x.map(|value| value / scale)
    }));
    if inverse.iter().any(|r| !finite(*r)) {
        return Err(Error::InvalidBoundary);
    }
    Ok(inverse)
}
pub(super) fn matvec(matrix: Matrix, vector: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| matrix[i].iter().zip(vector).map(|(a, b)| a * b).sum())
}
fn transpose(matrix: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| matrix[j][i]))
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}
fn normalize(q: [f64; 4]) -> Result<[f64; 4], Error> {
    let norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !positive(norm) {
        return Err(Error::NumericalFailure);
    }
    Ok(q.map(|v| v / norm))
}
fn quaternion_product(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}
fn quaternion_matrix(quaternion: [f64; 4]) -> Matrix {
    let [w, x, y, z] = quaternion;
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}
pub(super) fn rotation_matrix(velocity: [f64; 3], dt: f64) -> Result<Matrix, Error> {
    let speed = norm(velocity);
    let angle = speed * dt;
    if !angle.is_finite() {
        return Err(Error::NumericalFailure);
    }
    let q = if speed > 0.0 {
        let axis = velocity.map(|w| w / speed * (0.5 * angle).sin());
        [(0.5 * angle).cos(), axis[0], axis[1], axis[2]]
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };
    Ok(quaternion_matrix(q))
}
