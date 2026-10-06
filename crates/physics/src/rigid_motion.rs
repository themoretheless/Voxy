//! Prepared COM motion with affine world force and polynomial world torque.
//! Collision geometry and event response remain with the owning world.
use crate::{
    astrophysics_spin::TorquePolynomial,
    contact::ContactBody,
    spin_path::{Config, PathError, SpinPath},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalFailure,
    Rotation(PathError),
    Contact(crate::contact::Error),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RigidMotion {
    initial: ContactBody,
    acceleration: [f64; 3],
    force_rate: [f64; 3],
    jerk: [f64; 3],
    force: [f64; 3],
    torque: TorquePolynomial,
    duration: f64,
    rotation: Option<SpinPath>,
    end: ContactBody,
}

/// Work on the represented prepared trajectory. Torque work integrates the
/// nominal constant-axis arcs; its energy discrepancy is diagnostic, not heat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionWork {
    pub force_work: f64,
    pub torque_work: f64,
    pub kinetic_energy_change: f64,
    pub energy_residual: f64,
}

impl ContactBody {
    /// Prepare without mutation. Force acts at COM; torque is about COM.
    /// A particle cannot receive intrinsic torque. Rotation reuses SpinPath.
    pub fn prepare_motion(
        self,
        force: [f64; 3],
        torque: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        self.prepare_motion_with_torque(force, TorquePolynomial::constant(torque), duration, config)
    }

    /// Constant COM force and polynomial world COM torque on the existing
    /// prepared trajectory owner. No alternate contact or rotation integrator.
    pub fn prepare_motion_with_torque(
        self,
        force: [f64; 3],
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        self.prepare_affine_motion(force, [0.; 3], torque, duration, config)
    }

    /// Exact nominal cubic COM motion under F(t)=force+force_rate*t, sharing
    /// the same prepared owner and polynomial-torque rotation integrator.
    pub fn prepare_affine_motion(
        self,
        force: [f64; 3],
        force_rate: [f64; 3],
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        self.energy().map_err(|_| Error::InvalidInput)?;
        if force_rate.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        if !duration.is_finite()
            || duration <= 0.
            || force
                .iter()
                .chain(&torque.value)
                .chain(&torque.rate)
                .chain(&torque.acceleration)
                .any(|v| !v.is_finite())
            || (self.spin.is_none() && torque != TorquePolynomial::constant([0.; 3]))
        {
            return Err(Error::InvalidInput);
        }
        if force_rate
            .iter()
            .zip(force)
            .any(|(r, f)| !r.mul_add(duration, f).is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        let acceleration = force.map(|v| v / self.motion.mass);
        if acceleration.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let jerk = force_rate.map(|f| f / self.motion.mass);
        if jerk.iter().any(|x| !x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let rotation = self
            .spin
            .map(|spin| spin.prepare_polynomial_path(torque, duration, config))
            .transpose()
            .map_err(Error::Rotation)?;
        let mut path = RigidMotion {
            initial: self,
            acceleration,
            force_rate,
            jerk,
            force,
            torque,
            duration,
            rotation,
            end: self,
        };
        path.end = path.evaluate(duration)?;
        if force_rate != [0.; 3] {
            let velocity_max: [f64; 3] = std::array::from_fn(|k| {
                let middle = acceleration[k].mul_add(duration * 0.5, self.motion.velocity[k]);
                self.motion.velocity[k]
                    .abs()
                    .max(middle.abs())
                    .max(path.end.motion.velocity[k].abs())
            });
            let scale = self.motion.mass.sqrt() / 2_f64.sqrt();
            let scaled = velocity_max.map(|v| v * scale);
            let bound = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
            let rotational_bound = if let Some(spin) = self.spin {
                let (_, impulse) = torque
                    .envelopes(duration)
                    .map_err(|_| Error::NumericalFailure)?;
                let momentum = spin.angular_momentum[0]
                    .hypot(spin.angular_momentum[1])
                    .hypot(spin.angular_momentum[2])
                    + impulse;
                let minimum = spin.inertia.iter().copied().fold(f64::INFINITY, f64::min);
                let scaled = momentum / (minimum.sqrt() * 2_f64.sqrt());
                scaled * scaled
            } else {
                0.
            };
            if !bound.is_finite() || !(bound * bound + rotational_bound).is_finite() {
                return Err(Error::NumericalFailure);
            }
        }

        // An endpoint alone does not admit a parabolic path: a reversing body
        // can overflow at its interior position extremum and return to range.
        for k in 0..3 {
            if jerk[k] != 0. {
                // Cubic Bezier control hull bounds every nominal COM prefix.
                // Conservative rejection is preferable to admitting an interior
                // overflow from an endpoint-only check.
                let p1 = (self.motion.velocity[k] * (duration / 3.)) + self.motion.position[k];
                let p2 = ((acceleration[k] * (duration / 6.)
                    + self.motion.velocity[k] * (2. / 3.))
                    * duration)
                    + self.motion.position[k];
                if !p1.is_finite() || !p2.is_finite() {
                    return Err(Error::NumericalFailure);
                }
                let extremum = -acceleration[k] / jerk[k];
                if extremum > 0. && extremum < duration {
                    path.evaluate(extremum)?;
                }
            } else if acceleration[k] != 0. {
                let time = -self.motion.velocity[k] / acceleration[k];
                if time > 0. && time < duration {
                    path.evaluate(time)?;
                }
            }
        }
        Ok(path)
    }
}

impl RigidMotion {
    pub fn duration(&self) -> f64 {
        self.duration
    }
    pub fn initial(&self) -> ContactBody {
        self.initial
    }
    /// Initial acceleration. Backends using parabolic bounds must explicitly
    /// require has_constant_acceleration before interpreting this as constant.
    pub fn acceleration(&self) -> [f64; 3] {
        self.acceleration
    }
    pub fn jerk(&self) -> [f64; 3] {
        self.jerk
    }
    pub fn has_constant_acceleration(&self) -> bool {
        self.force_rate == [0.; 3]
    }
    /// Affine nominal acceleration at a valid trajectory time.
    pub fn acceleration_at(&self, time: f64) -> Result<[f64; 3], Error> {
        if !time.is_finite() || !(0. ..=self.duration).contains(&time) {
            return Err(Error::InvalidInput);
        }
        let value = std::array::from_fn(|k| self.jerk[k].mul_add(time, self.acceleration[k]));
        if value.iter().any(|x| !x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(value)
    }
    /// Quadratic Bernstein velocity controls for the complete time interval.
    /// Their convex hull bounds nominal speed, including interior reversals.
    /// Floating guards belong to the consuming geometry admission.
    pub fn velocity_controls(&self, start: f64, end: f64) -> Result<[[f64; 3]; 3], Error> {
        if start > end {
            return Err(Error::InvalidInput);
        }
        let initial = self.sample(start)?.motion.velocity;
        let last = self.sample(end)?.motion.velocity;
        let acceleration = self.acceleration_at(start)?;
        let middle =
            std::array::from_fn(|k| acceleration[k].mul_add((end - start) * 0.5, initial[k]));
        if middle.iter().any(|x| !x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok([initial, middle, last])
    }
    pub fn end(&self) -> ContactBody {
        self.end
    }
    pub fn rotation(&self) -> Option<&SpinPath> {
        self.rotation.as_ref()
    }

    fn evaluate(&self, time: f64) -> Result<ContactBody, Error> {
        let mut body = self.initial;
        for k in 0..3 {
            // Half time avoids t*t overflow and preserves subnormal acceleration.
            let mean_acceleration = self.jerk[k].mul_add(time / 3., self.acceleration[k]);
            let average_velocity = mean_acceleration.mul_add(0.5 * time, body.motion.velocity[k]);
            body.motion.position[k] = average_velocity.mul_add(time, body.motion.position[k]);
            body.motion.velocity[k] = self.jerk[k]
                .mul_add(time * 0.5, self.acceleration[k])
                .mul_add(time, body.motion.velocity[k]);
        }
        body.spin = self
            .rotation
            .as_ref()
            .map(|path| path.sample(time))
            .transpose()
            .map_err(Error::Rotation)?;
        body.energy().map_err(|_| Error::NumericalFailure)?;
        Ok(body)
    }

    /// Evaluate external work through the same trajectory prefix as sampling.
    /// Force work is F dot COM displacement; torque work is the sum of torque
    /// dot nominal arc angular velocity times its clipped duration. The residual
    /// retains integrator/rounding discrepancy and is never dissipated energy.
    pub fn work(&self, time: f64) -> Result<MotionWork, Error> {
        let endpoint = self.sample(time)?;
        let (force_work, torque_work) =
            self.affine_wrench_work(time, self.force, self.force_rate, self.torque)?;
        let kinetic_energy_change = endpoint.energy().map_err(Error::Contact)?
            - self.initial.energy().map_err(Error::Contact)?;
        let energy_residual = kinetic_energy_change - force_work - torque_work;
        if !kinetic_energy_change.is_finite() || !energy_residual.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(MotionWork {
            force_work,
            torque_work,
            kinetic_energy_change,
            energy_residual,
        })
    }

    /// Angular impulse about the world origin of a constant COM wrench on
    /// this prepared COM trajectory. Includes orbital r(t) cross force and
    /// intrinsic torque. This integral is exact for the nominal quadratic COM
    /// path, including prefixes clipped by collision events. A probe wrench
    /// does not alter this path; its torque can describe a fixed-environment
    /// application arm even when the path has no spin degree of freedom.
    pub fn wrench_angular_impulse(
        &self,
        time: f64,
        force: [f64; 3],
        torque: [f64; 3],
    ) -> Result<[f64; 3], Error> {
        self.polynomial_wrench_angular_impulse(time, force, TorquePolynomial::constant(torque))
    }

    /// Same orbital integral plus exactly integrated changing intrinsic torque.
    pub fn polynomial_wrench_angular_impulse(
        &self,
        time: f64,
        force: [f64; 3],
        torque: TorquePolynomial,
    ) -> Result<[f64; 3], Error> {
        if !force.iter().all(|x| x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        if !torque
            .value
            .iter()
            .chain(&torque.rate)
            .chain(&torque.acceleration)
            .all(|x| x.is_finite())
        {
            return Err(Error::InvalidInput);
        }
        self.sample(time)?;
        let torque_impulse = torque.impulse(time).map_err(|_| Error::NumericalFailure)?;
        let mean_position: [f64; 3] = std::array::from_fn(|k| {
            let mean_velocity = self.jerk[k]
                .mul_add(time / 12., self.acceleration[k] / 3.)
                .mul_add(time, self.initial.motion.velocity[k]);
            mean_velocity.mul_add(time * 0.5, self.initial.motion.position[k])
        });
        let result = std::array::from_fn(|k| {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            (mean_position[i] * force[j] - mean_position[j] * force[i]) * time + torque_impulse[k]
        });
        if !mean_position.iter().chain(&result).all(|x| x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }

    /// World-origin angular impulse of an affine force and polynomial torque
    /// evaluated on this same cubic COM trajectory, including clipped prefixes.
    pub fn affine_wrench_angular_impulse(
        &self,
        time: f64,
        force: [f64; 3],
        force_rate: [f64; 3],
        torque: TorquePolynomial,
    ) -> Result<[f64; 3], Error> {
        if force_rate.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let mut result = self.polynomial_wrench_angular_impulse(time, force, torque)?;
        let weighted: [f64; 3] = std::array::from_fn(|k| {
            self.jerk[k]
                .mul_add(time / 30., self.acceleration[k] / 8.)
                .mul_add(time, self.initial.motion.velocity[k] / 3.)
                .mul_add(time, self.initial.motion.position[k] * 0.5)
        });
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            result[k] +=
                ((weighted[i] * force_rate[j] - weighted[j] * force_rate[i]) * time) * time;
        }
        if weighted.iter().chain(&result).any(|x| !x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }

    /// Work of one constant world COM wrench on this actual prepared path.
    /// Allows external and constraint work to be separated without recomputing
    /// motion under either wrench alone. Torque uses the nominal spin arcs.
    pub fn wrench_work(
        &self,
        time: f64,
        force: [f64; 3],
        torque: [f64; 3],
    ) -> Result<(f64, f64), Error> {
        self.polynomial_wrench_work(time, force, TorquePolynomial::constant(torque))
    }

    /// Work of a polynomial torque probe on these same nominal rotation arcs.
    /// Integrates its exact angular impulse on each clipped arc, dotted with
    /// that arc's constant world angular velocity.
    pub fn polynomial_wrench_work(
        &self,
        time: f64,
        force: [f64; 3],
        torque: TorquePolynomial,
    ) -> Result<(f64, f64), Error> {
        if !force
            .iter()
            .chain(&torque.value)
            .chain(&torque.rate)
            .chain(&torque.acceleration)
            .all(|x| x.is_finite())
            || (self.initial.spin.is_none() && torque != TorquePolynomial::constant([0.; 3]))
        {
            return Err(Error::InvalidInput);
        }
        let endpoint = self.sample(time)?;
        let force_work: f64 = (0..3)
            .map(|k| force[k] * (endpoint.motion.position[k] - self.initial.motion.position[k]))
            .sum();
        let mut torque_work = 0.;
        if let Some(rotation) = &self.rotation {
            for segment in rotation.segments() {
                let duration = (time.min(segment.end_s) - segment.start_s).max(0.);
                if duration == 0. {
                    break;
                }
                let omega = segment.arc.angular_velocity();
                let impulse = torque
                    .shifted(segment.start_s)
                    .and_then(|t| t.impulse(duration))
                    .map_err(|_| Error::NumericalFailure)?;
                torque_work += (0..3).map(|k| impulse[k] * omega[k]).sum::<f64>();
            }
        }
        if !force_work.is_finite() || !torque_work.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok((force_work, torque_work))
    }

    /// Exact integral of an affine force probe on this same cubic COM path.
    /// Polynomial torque work retains the admitted nominal arc convention.
    pub fn affine_wrench_work(
        &self,
        time: f64,
        force: [f64; 3],
        force_rate: [f64; 3],
        torque: TorquePolynomial,
    ) -> Result<(f64, f64), Error> {
        if force_rate.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let (mut work, tw) = self.polynomial_wrench_work(time, force, torque)?;
        for k in 0..3 {
            let integral = self.jerk[k]
                .mul_add(time / 8., self.acceleration[k] / 3.)
                .mul_add(time, self.initial.motion.velocity[k] * 0.5);
            work += ((force_rate[k] * integral) * time) * time;
        }
        if !work.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok((work, tw))
    }

    /// Sample the same prepared trajectory used for the accepted endpoint.
    /// Translation analytically integrates affine force in floating point;
    /// SpinPath's model error bounds do not certify translational arithmetic.
    pub fn sample(&self, time: f64) -> Result<ContactBody, Error> {
        if !time.is_finite() || time < 0. || time > self.duration {
            return Err(Error::InvalidInput);
        }
        if time == 0. {
            return Ok(self.initial);
        }
        if time == self.duration {
            return Ok(self.end);
        }
        self.evaluate(time)
    }
}

/// One fully staged impact and its post-impact free trajectories. Owning worlds
/// commit both participants together and search these remainders for later hits.
#[derive(Clone, Debug, PartialEq)]
pub struct ImpactMotion {
    pub time_s: f64,
    pub first: ContactBody,
    pub second: ContactBody,
    pub impulse: crate::contact::NormalImpulse,
    pub first_remainder: Option<RigidMotion>,
    pub second_remainder: Option<RigidMotion>,
}
impl ImpactMotion {
    pub fn endpoints(&self) -> [ContactBody; 2] {
        [
            self.first_remainder
                .as_ref()
                .map_or(self.first, RigidMotion::end),
            self.second_remainder
                .as_ref()
                .map_or(self.second, RigidMotion::end),
        ]
    }
}

/// Prepare a reciprocal point impact at a geometrically admitted event time.
/// Point and normal must be witnessed by the caller's geometry. This function
/// does not detect contact or certify the underlying orbit. Original paths are
/// immutable even when the second remainder fails after the first was prepared.
pub fn prepare_impact(
    first: &RigidMotion,
    second: &RigidMotion,
    time_s: f64,
    point: [f64; 3],
    normal: [f64; 3],
    restitution: f64,
    config: Config,
) -> Result<ImpactMotion, Error> {
    if first.duration != second.duration {
        return Err(Error::InvalidInput);
    }
    let mut a = first.sample(time_s)?;
    let mut b = second.sample(time_s)?;
    let impulse =
        crate::contact::resolve_normal_impact(&mut a, Some(&mut b), point, normal, restitution)
            .map_err(Error::Contact)?;
    if impulse.relative_normal_speed >= 0. {
        return Err(Error::InvalidInput);
    }
    let remaining = first.duration - time_s;
    let first_remainder = if remaining == 0. {
        None
    } else {
        Some(
            a.prepare_affine_motion(
                std::array::from_fn(|k| first.force_rate[k].mul_add(time_s, first.force[k])),
                first.force_rate,
                first
                    .torque
                    .shifted(time_s)
                    .map_err(|_| Error::NumericalFailure)?,
                remaining,
                config,
            )?,
        )
    };
    let second_remainder = if remaining == 0. {
        None
    } else {
        Some(
            b.prepare_affine_motion(
                std::array::from_fn(|k| second.force_rate[k].mul_add(time_s, second.force[k])),
                second.force_rate,
                second
                    .torque
                    .shifted(time_s)
                    .map_err(|_| Error::NumericalFailure)?,
                remaining,
                config,
            )?,
        )
    };
    Ok(ImpactMotion {
        time_s,
        first: a,
        second: b,
        impulse,
        first_remainder,
        second_remainder,
    })
}
