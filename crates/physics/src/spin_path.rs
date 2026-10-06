//! Bounded adaptive paths over the existing Spin midpoint integrator.
use crate::{
    astrophysics::Error,
    astrophysics_spin::{Spin, SpinArc},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub max_angular_error_rad: f64,
    pub min_step_s: f64,
    pub max_arcs: usize,
    pub max_trials: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathError {
    InvalidInput,
    Budget,
    Integrator(Error),
    NumericalBound,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start_s: f64,
    pub end_s: f64,
    pub arc: SpinArc,
    /// Model residual propagation bound for all prefixes through this segment.
    /// Floating evaluation uses a guard, not interval transcendental arithmetic.
    pub model_angular_error_rad: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SpinPath {
    initial: Spin,
    duration: f64,
    segments: Vec<Segment>,
    trials: usize,
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn local_bound(arc: SpinArc) -> Result<(f64, f64), PathError> {
    let dt = arc.duration();
    let initial = arc.start();
    let torque = arc.torque();
    let inverse_min = 1.
        / initial
            .inertia
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    let inverse_max = 1. / initial.inertia.iter().copied().fold(0., f64::max);
    let lmax = norm(initial.angular_momentum) + norm(torque) * dt;
    let lipschitz = 2. * (inverse_min - inverse_max) * lmax;
    let midpoint = arc.sample(dt * 0.5).map_err(PathError::Integrator)?;
    let omega = arc.angular_velocity();
    let actual = midpoint.angular_velocity().map_err(PathError::Integrator)?;
    let defect = norm(std::array::from_fn(|k| actual[k] - omega[k]));
    let guard = 512. * f64::EPSILON * (1. + inverse_min * lmax + norm(omega));
    let residual =
        defect + guard + (lipschitz * norm(omega) + inverse_min * norm(torque)) * dt * 0.5;
    let exponent = lipschitz * dt;
    let growth = exponent.exp();
    let integral = if exponent == 0. {
        dt
    } else {
        dt * (exponent.exp_m1() / exponent)
    };
    let qnorm = initial.orientation[0]
        .hypot(initial.orientation[1])
        .hypot(initial.orientation[2])
        .hypot(initial.orientation[3]);
    let pose_guard = 2. * (qnorm - 1.).abs() + 512. * f64::EPSILON * (1. + norm(omega) * dt);
    let local = residual * integral + pose_guard;
    if !growth.is_finite() || !local.is_finite() || local < 0. {
        return Err(PathError::NumericalBound);
    }
    Ok((local, growth))
}
impl Spin {
    /// Prepare a complete constant-world-torque path without changing this state.
    /// Adaptive trials bound the model residual; no endpoint interpolation is
    /// substituted for admitted constant-axis arcs. Budgets fail explicitly.
    pub fn prepare_path(
        self,
        torque: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<SpinPath, PathError> {
        self.angular_velocity().map_err(PathError::Integrator)?;
        if !duration.is_finite()
            || duration <= 0.
            || torque.iter().any(|v| !v.is_finite())
            || !config.max_angular_error_rad.is_finite()
            || config.max_angular_error_rad <= 0.
            || !config.min_step_s.is_finite()
            || config.min_step_s <= 0.
            || config.max_arcs == 0
            || config.max_trials == 0
        {
            return Err(PathError::InvalidInput);
        }
        let inverse_min = 1. / self.inertia.iter().copied().fold(f64::INFINITY, f64::min);
        let inverse_max = 1. / self.inertia.iter().copied().fold(0., f64::max);
        let global_rate = 2.
            * (inverse_min - inverse_max)
            * (norm(self.angular_momentum) + norm(torque) * duration);
        let reserve = (-global_rate * duration).exp();
        if !global_rate.is_finite() || reserve == 0. {
            return Err(PathError::NumericalBound);
        }
        let mut path = SpinPath {
            initial: self,
            duration,
            segments: Vec::new(),
            trials: 0,
        };
        let mut state = self;
        let mut time = 0.;
        let mut step = duration;
        let mut error = 0.;
        while time < duration {
            if path.segments.len() >= config.max_arcs || path.trials >= config.max_trials {
                return Err(PathError::Budget);
            }
            step = step.min(duration - time);
            let end = if step == duration - time {
                duration
            } else {
                time + step
            };
            step = end - time;
            if end <= time || step < config.min_step_s {
                return Err(PathError::Budget);
            }
            path.trials += 1;
            let trial = state
                .prepare_arc(torque, step)
                .map_err(PathError::Integrator)
                .and_then(|arc| {
                    local_bound(arc).map(|(local, growth)| (arc, local + growth * error))
                });
            // Reserve room for amplification over the remaining horizon. A purely
            // linear budget can strand a valid long path when prior error grows.
            let allowed = config.max_angular_error_rad
                * (end / duration)
                * (-global_rate * (duration - end)).exp();
            match trial {
                Ok((arc, bound)) if bound <= allowed => {
                    path.segments.push(Segment {
                        start_s: time,
                        end_s: end,
                        arc,
                        model_angular_error_rad: bound,
                    });
                    state = arc.end();
                    error = bound;
                    time = end;
                    step *= 2.;
                }
                Err(PathError::Integrator(Error::InvalidInput)) => {
                    return Err(PathError::Integrator(Error::InvalidInput));
                }
                _ => step *= 0.5,
            }
        }
        Ok(path)
    }
}
impl SpinPath {
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn duration(&self) -> f64 {
        self.duration
    }
    pub fn trials(&self) -> usize {
        self.trials
    }
    pub fn end(&self) -> Spin {
        self.segments
            .last()
            .expect("admitted nonempty path")
            .arc
            .end()
    }
    pub fn model_angular_error_rad(&self) -> f64 {
        self.segments
            .last()
            .expect("admitted nonempty path")
            .model_angular_error_rad
    }
    pub fn sample(&self, time: f64) -> Result<Spin, PathError> {
        if !time.is_finite() || !(0. ..=self.duration).contains(&time) {
            return Err(PathError::InvalidInput);
        }
        if time == 0. {
            return Ok(self.initial);
        }
        if time == self.duration {
            return Ok(self.end());
        }
        let index = self.segments.partition_point(|s| s.end_s < time);
        let segment = self.segments[index];
        if time == segment.end_s {
            return Ok(segment.arc.end());
        }
        if time == segment.start_s {
            return Ok(segment.arc.start());
        }
        let local = (time - segment.start_s).clamp(0., segment.arc.duration());
        segment.arc.sample(local).map_err(PathError::Integrator)
    }
}
