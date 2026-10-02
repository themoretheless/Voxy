use super::boundary_body::{BoundaryBodyReport, BoundaryForces, Coupling, coupled_kick};
use super::{Error, Liquid, Particle, TranslatingBody, finite, norm, positive, sub};

/// Rigid body with isotropic inertia (e.g. a uniform sphere). Samples carry its orientation.
/// Use `TensorBody` for anisotropic inertia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotatingBody {
    pub translation: TranslatingBody,
    /// World-space angular velocity, radians / time.
    pub angular_velocity: [f64; 3],
    pub moment_of_inertia: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotatingBoundaryReport {
    pub boundary: BoundaryBodyReport,
    pub angular_impulse: [f64; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rotation {
    velocity: [f64; 3],
    inertia: f64,
    impulse: [f64; 3],
    tensor: Option<tensor::TensorRotation>,
}
impl Liquid {
    /// Transfers SPH pressure and viscosity to translation and rotation, and rotates
    /// configured lab-space samples with the body. All samples belong to this body.
    /// The isotropic inertia makes torque-free angular velocity constant.
    /// Viscosity acts at the fluid point with rigid-body velocity continued there;
    /// axis-split pair relaxation conserves linear and angular impulse and dissipates energy.
    /// No swept rotating collision template is supplied in this method.
    /// # Errors
    /// Invalid body/inertia, exhausted budgets or numerical/thermal failure. All
    /// fluid fields, sample positions and body state remain unchanged on failure.
    pub fn step_with_rotating_boundary(
        &mut self,
        dt: f64,
        body: &mut RotatingBody,
    ) -> Result<RotatingBoundaryReport, Error> {
        if !finite(body.angular_velocity) || !positive(body.moment_of_inertia) {
            return Err(Error::InvalidBoundary);
        }
        let mut candidate = self.prepare_boundary_body(body.translation)?;
        candidate
            .boundary_coupling
            .as_mut()
            .ok_or(Error::InvalidBoundary)?
            .rotation = Some(Rotation {
            velocity: body.angular_velocity,
            inertia: body.moment_of_inertia,
            tensor: None,
            impulse: [0.0; 3],
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
        let mut translation = body.translation;
        let boundary = candidate.finish_boundary_body(fluid, &mut translation)?;
        *body = RotatingBody {
            translation,
            angular_velocity: rotation.velocity,
            moment_of_inertia: rotation.inertia,
        };
        *self = candidate;
        Ok(RotatingBoundaryReport {
            boundary,
            angular_impulse: rotation.impulse,
        })
    }
    pub(super) fn update_rotating_surface_velocities(&mut self) -> Result<(), Error> {
        let coupling = self
            .boundary_coupling
            .as_ref()
            .ok_or(Error::InvalidBoundary)?;
        let rotation = coupling.rotation.ok_or(Error::InvalidBoundary)?;
        let velocities = self
            .boundaries
            .iter()
            .map(|sample| {
                let tangent = cross(
                    rotation.velocity,
                    sub(sample.position, coupling.body.position),
                );
                std::array::from_fn(|a| coupling.body.velocity[a] + tangent[a])
            })
            .collect();
        self.configure_boundary_velocities(velocities)
    }
    pub(super) fn rotate_boundary_samples(
        &mut self,
        displacement: [f64; 3],
        dt: f64,
    ) -> Result<(), Error> {
        let mut coupling = self
            .boundary_coupling
            .clone()
            .ok_or(Error::InvalidBoundary)?;
        let rotation = coupling.rotation.as_mut().ok_or(Error::InvalidBoundary)?;
        let increment = if let Some(tensor) = &mut rotation.tensor {
            let increment = tensor.drift(dt)?;
            rotation.velocity = tensor.mobility(tensor.momentum);
            increment
        } else {
            tensor::rotation_matrix(rotation.velocity, dt)?
        };
        let previous_center: [f64; 3] =
            std::array::from_fn(|a| coupling.body.position[a] - displacement[a]);
        let mut samples = self.boundaries.clone();
        for sample in &mut samples {
            let rotated = tensor::matvec(increment, sub(sample.position, previous_center));
            sample.position = std::array::from_fn(|a| coupling.body.position[a] + rotated[a]);
        }
        self.boundary_coupling = Some(coupling);
        self.configure_boundaries(samples)?;
        self.update_rotating_surface_velocities()
    }
    pub(super) fn rotating_boundary_time_limit(
        &self,
        forces: &BoundaryForces,
        particles: &[Particle],
    ) -> Result<f64, Error> {
        let coupling = self
            .boundary_coupling
            .as_ref()
            .ok_or(Error::InvalidBoundary)?;
        let Some(rotation) = coupling.rotation else {
            return Ok(f64::INFINITY);
        };
        let speed = norm(rotation.velocity);
        let mut limit = if speed > 0.0 {
            0.25 / speed
        } else {
            f64::INFINITY
        };
        let mut torque = [0.0; 3];
        let mut rate = 0.0;
        for (i, p) in particles.iter().enumerate() {
            let r = sub(p.position, coupling.body.position);
            let angular = cross(r, forces.pressure[i].map(|a| -p.mass * a));
            for a in 0..3 {
                torque[a] += angular[a];
            }
            rate += p.mass * forces.rates[i] * norm(r).powi(2) * rotation.compliance_bound();
        }
        let acceleration = norm(rotation.mobility(torque));
        if let Some(tensor) = rotation.tensor {
            let gyroscopic_rate = norm(tensor.momentum) * rotation.compliance_bound();
            if gyroscopic_rate > 0.0 {
                limit = limit.min(0.125 / gyroscopic_rate);
            }
        }
        if acceleration > 0.0 {
            limit = limit.min((0.25 / acceleration).sqrt());
        }
        if rate > 0.0 {
            limit = limit.min(0.25 / rate);
        }
        for sample in &self.boundaries {
            let surface_speed = norm(cross(
                rotation.velocity,
                sub(sample.position, coupling.body.position),
            ));
            if surface_speed > 0.0 {
                limit = limit.min(0.25 * self.config.smoothing_radius / surface_speed);
            }
        }
        if !speed.is_finite()
            || !acceleration.is_finite()
            || !rate.is_finite()
            || !finite(torque)
            || limit.is_nan()
            || limit <= 0.0
        {
            return Err(Error::NumericalFailure);
        }
        Ok(limit)
    }
}
pub(super) fn angular_kick(
    p: &Particle,
    coupling: &mut Coupling,
    impulse: [f64; 3],
) -> Result<f64, Error> {
    let Some(rotation) = &mut coupling.rotation else {
        return Ok(0.0);
    };
    let angular = cross(sub(p.position, coupling.body.position), impulse.map(|j| -j));
    let delta = rotation.mobility(angular);
    let work = angular
        .iter()
        .enumerate()
        .map(|(a, j)| j * (rotation.velocity[a] + 0.5 * delta[a]))
        .sum::<f64>();
    if let Some(tensor) = &mut rotation.tensor {
        for (a, j) in angular.into_iter().enumerate() {
            tensor.momentum[a] += j;
        }
        rotation.velocity = tensor.mobility(tensor.momentum);
    } else {
        for (a, value) in delta.into_iter().enumerate() {
            rotation.velocity[a] += value;
        }
    }
    for (a, j) in angular.into_iter().enumerate() {
        rotation.impulse[a] += j;
    }
    if !work.is_finite() || !finite(rotation.velocity) || !finite(rotation.impulse) {
        return Err(Error::NumericalFailure);
    }
    Ok(work)
}
pub(super) fn viscous_kick(
    p: &mut Particle,
    coupling: &mut Coupling,
    rate: f64,
    dt: f64,
    exact: bool,
) -> Result<([f64; 3], f64), Error> {
    let mut total = [0.0; 3];
    let mut work = 0.0;
    for a in 0..3 {
        let rotation = coupling.rotation.ok_or(Error::InvalidBoundary)?;
        let r = sub(p.position, coupling.body.position);
        let mut direction = [0.0; 3];
        direction[a] = 1.0;
        let lever = cross(r, direction);
        let inverse = 1.0 / p.mass
            + 1.0 / coupling.body.mass
            + lever
                .iter()
                .zip(rotation.mobility(lever))
                .map(|(a, b)| a * b)
                .sum::<f64>();
        let conductance = p.mass * rate;
        let factor = if exact {
            -(-conductance * inverse * dt).exp_m1() / inverse
        } else {
            conductance * dt
        };
        let relative = p.velocity[a] - coupling.body.velocity[a] - cross(rotation.velocity, r)[a];
        let mut impulse = [0.0; 3];
        impulse[a] = -factor * relative;
        if !positive(inverse) || !factor.is_finite() {
            return Err(Error::NumericalFailure);
        }
        work += coupled_kick(p, coupling, impulse)?;
        total[a] += impulse[a];
    }
    Ok((total, work))
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl Rotation {
    fn mobility(self, impulse: [f64; 3]) -> [f64; 3] {
        self.tensor.map_or_else(
            || impulse.map(|j| j / self.inertia),
            |tensor| tensor.mobility(impulse),
        )
    }
    fn compliance_bound(self) -> f64 {
        self.tensor
            .map_or(1.0 / self.inertia, tensor::TensorRotation::compliance_bound)
    }
}
#[path = "tensor.rs"]
mod tensor;
pub use tensor::TensorBody;
