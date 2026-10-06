//! Prepared COM motion with constant world force and torque.
//! Collision geometry and event response remain with the owning world.
use crate::{
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
    force: [f64; 3],
    torque: [f64; 3],
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
        self.energy().map_err(|_| Error::InvalidInput)?;
        if !duration.is_finite()
            || duration <= 0.
            || force.iter().chain(torque.iter()).any(|v| !v.is_finite())
            || (self.spin.is_none() && torque != [0.; 3])
        {
            return Err(Error::InvalidInput);
        }
        let acceleration = force.map(|v| v / self.motion.mass);
        if acceleration.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let rotation = self
            .spin
            .map(|spin| spin.prepare_path(torque, duration, config))
            .transpose()
            .map_err(Error::Rotation)?;
        let mut path = RigidMotion {
            initial: self,
            acceleration,
            force,
            torque,
            duration,
            rotation,
            end: self,
        };
        path.end = path.evaluate(duration)?;
        // An endpoint alone does not admit a parabolic path: a reversing body
        // can overflow at its interior position extremum and return to range.
        for k in 0..3 {
            if acceleration[k] != 0. {
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
    pub fn acceleration(&self) -> [f64; 3] {
        self.acceleration
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
            let average_velocity =
                self.acceleration[k].mul_add(0.5 * time, body.motion.velocity[k]);
            body.motion.position[k] = average_velocity.mul_add(time, body.motion.position[k]);
            body.motion.velocity[k] = self.acceleration[k].mul_add(time, body.motion.velocity[k]);
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
        let force_work: f64 = (0..3)
            .map(|k| {
                self.force[k] * (endpoint.motion.position[k] - self.initial.motion.position[k])
            })
            .sum();
        let mut torque_work = 0.;
        if let Some(rotation) = &self.rotation {
            for segment in rotation.segments() {
                let duration = (time.min(segment.end_s) - segment.start_s).max(0.);
                if duration == 0. {
                    break;
                }
                let omega = segment.arc.angular_velocity();
                torque_work += (0..3).map(|k| self.torque[k] * omega[k]).sum::<f64>() * duration;
            }
        }
        let kinetic_energy_change = endpoint.energy().map_err(Error::Contact)?
            - self.initial.energy().map_err(Error::Contact)?;
        let energy_residual = kinetic_energy_change - force_work - torque_work;
        if [
            force_work,
            torque_work,
            kinetic_energy_change,
            energy_residual,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(MotionWork {
            force_work,
            torque_work,
            kinetic_energy_change,
            energy_residual,
        })
    }

    /// Sample the same prepared trajectory used for the accepted endpoint.
    /// Translation is analytic constant-acceleration motion in floating point;
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
        Some(a.prepare_motion(first.force, first.torque, remaining, config)?)
    };
    let second_remainder = if remaining == 0. {
        None
    } else {
        Some(b.prepare_motion(second.force, second.torque, remaining, config)?)
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
