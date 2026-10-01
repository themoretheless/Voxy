//! Isolated binary: monopole + permanent quadrupoles + constant-time-lag tides.
//! In the barycentric rest frame, spin/orbit reactions conserve total angular
//! momentum. Dissipative mechanical work is deposited as heat in each body.
use crate::{
    astrophysics::{Error, OrbitalState},
    astrophysics_spin::Spin,
};
type Vector = [f64; 3];
fn dot(a: Vector, b: Vector) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(v: Vector) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn matrix_vector(matrix: [[f64; 3]; 3], v: Vector) -> Vector {
    matrix.map(|row| dot(row, v))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tide {
    pub love_number: f64,
    pub time_lag: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub mass: f64,
    pub radius: f64,
    pub spin: Spin,
    pub tide: Tide,
    pub heat: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binary {
    pub relative: OrbitalState,
    pub primary: Body,
    pub secondary: Body,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diagnostics {
    pub mechanical_energy: f64,
    pub heat: f64,
    pub angular_momentum: Vector,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepError {
    Physics(Error),
    Contact,
    BudgetExceeded,
}
impl From<Error> for StepError {
    fn from(value: Error) -> Self {
        Self::Physics(value)
    }
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    radius: f64,
    normal: Vector,
    reduced_mass: f64,
    inertia: [[[f64; 3]; 3]; 2],
    static_tide: [f64; 2],
    drag: [f64; 2],
}
impl Binary {
    fn geometry(self, constant: f64) -> Result<Geometry, StepError> {
        if !constant.is_finite()
            || constant <= 0.0
            || self
                .relative
                .position
                .iter()
                .chain(&self.relative.velocity)
                .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidInput.into());
        }
        for body in [self.primary, self.secondary] {
            body.spin.angular_velocity()?;
            if !body.mass.is_finite()
                || body.mass <= 0.0
                || !body.radius.is_finite()
                || body.radius <= 0.0
                || !body.heat.is_finite()
                || body.heat < 0.0
                || !body.tide.love_number.is_finite()
                || body.tide.love_number < 0.0
                || !body.tide.time_lag.is_finite()
                || body.tide.time_lag < 0.0
            {
                return Err(Error::InvalidInput.into());
            }
        }
        let radius = length(self.relative.position);
        if !radius.is_finite() {
            return Err(Error::NumericalOverflow.into());
        }
        if radius <= self.primary.radius + self.secondary.radius {
            return Err(StepError::Contact);
        }
        let normal = self.relative.position.map(|v| v / radius);
        let total = self.primary.mass + self.secondary.mass;
        let reduced_mass = self.primary.mass / total * self.secondary.mass;
        let inertia = [
            self.primary.spin.world_inertia()?,
            self.secondary.spin.world_inertia()?,
        ];
        let mut static_tide = [0.0; 2];
        let mut drag = [0.0; 2];
        for (index, (body, perturber)) in [
            (self.primary, self.secondary),
            (self.secondary, self.primary),
        ]
        .into_iter()
        .enumerate()
        {
            if body.tide.love_number > 0.0 {
                static_tide[index] = constant * perturber.mass * perturber.mass / radius
                    * body.tide.love_number
                    * (body.radius / radius).powi(5);
                drag[index] = 3.0 * static_tide[index] * body.tide.time_lag / radius / radius;
            }
        }
        if !total.is_finite()
            || !reduced_mass.is_finite()
            || reduced_mass <= 0.0
            || static_tide.into_iter().chain(drag).any(|v| !v.is_finite())
        {
            return Err(Error::NumericalOverflow.into());
        }
        Ok(Geometry {
            radius,
            normal,
            reduced_mass,
            inertia,
            static_tide,
            drag,
        })
    }
    fn conservative(
        self,
        constant: f64,
        geometry: Geometry,
    ) -> Result<(Vector, [Vector; 2], f64), StepError> {
        let combined = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                self.secondary.mass * geometry.inertia[0][i][j]
                    + self.primary.mass * geometry.inertia[1][i][j]
            })
        });
        let product = matrix_vector(combined, geometry.normal);
        let projection = dot(geometry.normal, product);
        let trace = combined[0][0] + combined[1][1] + combined[2][2];
        let radius = geometry.radius;
        let point = constant * self.primary.mass * self.secondary.mass / radius;
        let quad = constant / radius / radius / radius * 0.5;
        let static_sum = geometry.static_tide.iter().sum::<f64>();
        let potential = -point - quad * (trace - 3.0 * projection) - 0.5 * static_sum;
        let force = std::array::from_fn(|k| {
            -(point + 3.0 * static_sum) / radius * geometry.normal[k]
                - 3.0 * quad / radius
                    * ((trace - 5.0 * projection) * geometry.normal[k] + 2.0 * product[k])
        });
        let torques = std::array::from_fn(|index| {
            let moment = matrix_vector(geometry.inertia[index], geometry.normal);
            let companion = if index == 0 {
                self.secondary.mass
            } else {
                self.primary.mass
            };
            cross(geometry.normal, moment)
                .map(|v| 3.0 * constant * companion / radius / radius / radius * v)
        });
        if !potential.is_finite()
            || force
                .iter()
                .chain(torques.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(Error::NumericalOverflow.into());
        }
        Ok((force, torques, potential))
    }
    /// Energy includes the permanent-figure and equilibrium-tide potentials.
    /// Angular momentum is about the barycenter; no barycenter translational energy.
    /// # Errors
    /// Invalid physical data, body contact, or numerical overflow.
    pub fn diagnostics(self, constant: f64) -> Result<Diagnostics, StepError> {
        let geometry = self.geometry(constant)?;
        let potential = self.conservative(constant, geometry)?.2;
        let mechanical_energy =
            0.5 * geometry.reduced_mass * dot(self.relative.velocity, self.relative.velocity)
                + self.primary.spin.energy()?
                + self.secondary.spin.energy()?
                + potential;
        let heat = self.primary.heat + self.secondary.heat;
        let orbital = cross(self.relative.position, self.relative.velocity)
            .map(|v| v * geometry.reduced_mass);
        let angular_momentum = std::array::from_fn(|k| {
            orbital[k]
                + self.primary.spin.angular_momentum[k]
                + self.secondary.spin.angular_momentum[k]
        });
        if !mechanical_energy.is_finite()
            || !heat.is_finite()
            || angular_momentum.iter().any(|v| !v.is_finite())
        {
            return Err(Error::NumericalOverflow.into());
        }
        Ok(Diagnostics {
            mechanical_energy,
            heat,
            angular_momentum,
        })
    }
    fn kick(&mut self, constant: f64, dt: f64) -> Result<(), StepError> {
        let geometry = self.geometry(constant)?;
        let (force, torques, _) = self.conservative(constant, geometry)?;
        for (k, f) in force.iter().enumerate() {
            self.relative.velocity[k] += f / geometry.reduced_mass * dt;
            self.primary.spin.angular_momentum[k] += torques[0][k] * dt;
            self.secondary.spin.angular_momentum[k] += torques[1][k] * dt;
        }
        Ok(())
    }
    fn friction_derivative(
        self,
        geometry: Geometry,
        state: [f64; 9],
    ) -> Result<([f64; 9], [f64; 2]), StepError> {
        let velocity = [state[0], state[1], state[2]];
        let radial = dot(velocity, geometry.normal);
        let mut derivative = [0.0; 9];
        let mut power = [0.0; 2];
        for (index, body) in [self.primary, self.secondary].into_iter().enumerate() {
            let mut spin = body.spin;
            spin.angular_momentum = std::array::from_fn(|k| state[3 + index * 3 + k]);
            let spin_velocity = cross(spin.angular_velocity()?, self.relative.position);
            let slip = std::array::from_fn(|k| velocity[k] - spin_velocity[k]);
            let force = std::array::from_fn(|k| {
                -geometry.drag[index] * (slip[k] + 2.0 * radial * geometry.normal[k])
            });
            let torque = cross(self.relative.position, force).map(|v| -v);
            for k in 0..3 {
                derivative[k] += force[k] / geometry.reduced_mass;
                derivative[3 + index * 3 + k] = torque[k];
            }
            power[index] = geometry.drag[index] * (dot(slip, slip) + 2.0 * radial * radial);
        }
        if derivative.into_iter().chain(power).any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow.into());
        }
        Ok((derivative, power))
    }
    fn dissipate(&mut self, constant: f64, dt: f64) -> Result<(), StepError> {
        let geometry = self.geometry(constant)?;
        if geometry.drag.iter().all(|v| *v == 0.0) {
            return Ok(());
        }
        let initial = std::array::from_fn(|k| {
            if k < 3 {
                self.relative.velocity[k]
            } else if k < 6 {
                self.primary.spin.angular_momentum[k - 3]
            } else {
                self.secondary.spin.angular_momentum[k - 6]
            }
        });
        let rate = self.friction_derivative(geometry, initial)?.0;
        let mut system = [[0.0; 10]; 9];
        for (column, _) in initial.iter().enumerate() {
            let mut basis = [0.0; 9];
            basis[column] = 1.0;
            let derivative = self.friction_derivative(geometry, basis)?.0;
            for row in 0..9 {
                system[row][column] =
                    if row == column { 1.0 } else { 0.0 } - 0.5 * dt * derivative[row];
            }
        }
        for row in 0..9 {
            system[row][9] = initial[row] + 0.5 * dt * rate[row];
        }
        let end = solve(system)?;
        let midpoint = std::array::from_fn(|k| 0.5 * (initial[k] + end[k]));
        let power = self.friction_derivative(geometry, midpoint)?.1;
        self.relative.velocity = [end[0], end[1], end[2]];
        self.primary.spin.angular_momentum = [end[3], end[4], end[5]];
        self.secondary.spin.angular_momentum = [end[6], end[7], end[8]];
        self.primary.heat += power[0] * dt;
        self.secondary.heat += power[1] * dt;
        Ok(())
    }
    /// Advances all orbital/spin/heat state atomically. Constant-time-lag tides
    /// assume small lags and equilibrium response; this is not a rheology solver.
    /// Substeps bound changing orbital forces and conservative rotation error.
    /// # Errors
    /// Invalid inputs, contact, numerical failures or exhausted substeps leave
    /// the original binary unchanged. Collisions need a separate merger/contact policy.
    pub fn step(
        &mut self,
        constant: f64,
        dt: f64,
        max_step: f64,
        max_substeps: usize,
    ) -> Result<usize, StepError> {
        self.geometry(constant)?;
        if !dt.is_finite() || dt <= 0.0 || !max_step.is_finite() || max_step <= 0.0 {
            return Err(Error::InvalidInput.into());
        }
        let mut next = *self;
        let mut remaining = dt;
        let mut steps = 0;
        while remaining > 0.0 {
            if steps == max_substeps {
                return Err(StepError::BudgetExceeded);
            }
            let h = remaining.min(max_step);
            next.dissipate(constant, h * 0.5)?;
            next.kick(constant, h * 0.5)?;
            // Detect straight drift contacts even if both endpoints lie outside.
            let speed_squared = dot(next.relative.velocity, next.relative.velocity);
            let closest = if speed_squared > 0.0 {
                (-dot(next.relative.position, next.relative.velocity) / speed_squared).clamp(0.0, h)
            } else {
                0.0
            };
            let closest_position = std::array::from_fn(|k| {
                next.relative.position[k] + next.relative.velocity[k] * closest
            });
            if length(closest_position) <= next.primary.radius + next.secondary.radius {
                return Err(StepError::Contact);
            }
            for k in 0..3 {
                next.relative.position[k] += next.relative.velocity[k] * h;
            }
            next.primary.spin.step([0.0; 3], h)?;
            next.secondary.spin.step([0.0; 3], h)?;
            next.kick(constant, h * 0.5)?;
            next.dissipate(constant, h * 0.5)?;
            remaining -= h;
            steps += 1;
        }
        next.diagnostics(constant)?;
        *self = next;
        Ok(steps)
    }
}

fn solve(mut system: [[f64; 10]; 9]) -> Result<[f64; 9], StepError> {
    for column in 0..9 {
        let pivot = (column..9)
            .max_by(|&a, &b| system[a][column].abs().total_cmp(&system[b][column].abs()))
            .ok_or(Error::NoConvergence)?;
        if system[pivot][column] == 0.0 || !system[pivot][column].is_finite() {
            return Err(Error::NoConvergence.into());
        }
        system.swap(column, pivot);
        let divisor = system[column][column];
        for entry in &mut system[column][column..] {
            *entry /= divisor;
        }
        let pivot_row = system[column];
        for (row, entries) in system.iter_mut().enumerate() {
            if row == column {
                continue;
            }
            let scale = entries[column];
            for (entry, pivot_entry) in entries[column..].iter_mut().zip(&pivot_row[column..]) {
                *entry -= scale * pivot_entry;
            }
        }
    }
    let result = std::array::from_fn(|k| system[k][9]);
    if result.iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericalOverflow.into());
    }
    Ok(result)
}
