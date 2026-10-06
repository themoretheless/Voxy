//! Bounded adaptive paths over the existing Spin midpoint integrator.
use crate::{
    astrophysics::Error,
    astrophysics_spin::{ArcTorque, RotatingArmForce, Spin, SpinArc, TorquePolynomial, rotate},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub max_angular_error_rad: f64,
    /// Adaptive subdivision floor. A terminal interval clipped by an event or
    /// the requested horizon may be shorter, but must pass the same error bound.
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
    /// Converts the joint attitude/momentum majorant to momentum units.
    momentum_error_per_rad: f64,
    source_momentum_error: f64,
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn local_bound(arc: SpinArc, feedback_rate: f64) -> Result<(f64, f64), PathError> {
    let dt = arc.duration();
    let initial = arc.start();
    let (maximum_torque, impulse_bound) = arc.torque_envelopes().map_err(PathError::Integrator)?;
    let inverse_min = 1.
        / initial
            .inertia
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    let inverse_max = 1. / initial.inertia.iter().copied().fold(0., f64::max);
    let lmax = norm(initial.angular_momentum) + impulse_bound;
    let lipschitz = 2. * (inverse_min - inverse_max) * lmax;
    let midpoint = arc.sample(dt * 0.5).map_err(PathError::Integrator)?;
    let omega = arc.angular_velocity();
    let actual = midpoint.angular_velocity().map_err(PathError::Integrator)?;
    let defect = norm(std::array::from_fn(|k| actual[k] - omega[k]));
    let guard = 512. * f64::EPSILON * (1. + inverse_min * lmax + norm(omega));
    let residual =
        defect + guard + (lipschitz * norm(omega) + inverse_min * maximum_torque) * dt * 0.5;
    // For a body-local force, attitude error also perturbs torque and momentum.
    // The weighted joint error E = angle + momentum / scale grows at most at
    // lipschitz + sqrt(inverse_min * radius * maximum_force).
    let exponent = (lipschitz + feedback_rate) * dt;
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
    let local = residual * integral + pose_guard * if feedback_rate == 0. { 1. } else { growth };
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
        self.prepare_polynomial_path(TorquePolynomial::constant(torque), duration, config)
    }

    /// Prepare changing world torque through the same adaptive midpoint arcs.
    /// Angular momentum integrates the polynomial at every prefix; attitude
    /// admission includes the torque envelope over each complete interval.
    pub fn prepare_polynomial_path(
        self,
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
    ) -> Result<SpinPath, PathError> {
        torque.validate().map_err(|_| PathError::InvalidInput)?;
        if !duration.is_finite() || duration <= 0. {
            return Err(PathError::InvalidInput);
        }
        let (_, impulse_bound) = torque.envelopes(duration).map_err(PathError::Integrator)?;
        self.prepare_forcing_path(duration, config, impulse_bound, |time| {
            Ok((
                ArcTorque::polynomial(torque.shifted(time).map_err(PathError::Integrator)?),
                duration,
            ))
        })
    }

    /// Adaptive motion under an affine world force on this body's own local
    /// material point. Both attitude and momentum feedback enter admission.
    pub fn prepare_material_force_path(
        self,
        local: [f64; 3],
        force: [f64; 3],
        rate: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<SpinPath, PathError> {
        self.prepare_material_force_path_with_torque(
            local,
            force,
            rate,
            TorquePolynomial::constant([0.; 3]),
            duration,
            config,
        )
    }

    /// Own-point feedback composed with an independently supplied polynomial
    /// world torque. The COM load does not enter this rotation forcing law.
    pub fn prepare_material_force_path_with_torque(
        self,
        local: [f64; 3],
        force: [f64; 3],
        rate: [f64; 3],
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
    ) -> Result<SpinPath, PathError> {
        torque.validate().map_err(|_| PathError::InvalidInput)?;
        if !duration.is_finite()
            || duration <= 0.
            || local
                .iter()
                .chain(&force)
                .chain(&rate)
                .any(|x| !x.is_finite())
        {
            return Err(PathError::InvalidInput);
        }
        let law = ArcTorque {
            material: None,
            polynomial: torque,
            rotating: Some(RotatingArmForce {
                arm: rotate(self.orientation, local),
                omega: [0.; 3],
                force,
                rate,
            }),
        };
        let (_, nominal_impulse_bound) = law.envelopes(duration).map_err(PathError::Integrator)?;
        // The physical local radius also bounds later normalized attitudes,
        // even if the accepted initial quaternion is a little off unit norm.
        let force_integral = norm(force) * duration + (norm(rate) * (duration * 0.5)) * duration;
        let external_impulse = torque.envelopes(duration).map_err(PathError::Integrator)?.1;
        let impulse_bound =
            nominal_impulse_bound.max(external_impulse + norm(local) * force_integral);
        let feedback = norm(local) * (norm(force) + norm(rate) * duration);
        self.prepare_forcing_path_impl(
            duration,
            config,
            impulse_bound,
            Some((local, feedback)),
            |time| {
                Ok((
                    ArcTorque {
                        material: None,
                        polynomial: torque.shifted(time).map_err(PathError::Integrator)?,
                        rotating: Some(RotatingArmForce {
                            arm: [0.; 3],
                            omega: [0.; 3],
                            force: std::array::from_fn(|k| rate[k].mul_add(time, force[k])),
                            rate,
                        }),
                    },
                    duration,
                ))
            },
        )
    }

    /// Joint attitude/momentum feedback from affine forces at multiple local
    /// points, using the same midpoint and adaptive admission owner.
    pub fn prepare_material_moment_path(
        self,
        moment: crate::rigid_motion::MaterialForceMoment,
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
    ) -> Result<SpinPath, PathError> {
        if !duration.is_finite() || duration <= 0. {
            return Err(PathError::InvalidInput);
        }
        moment
            .torque_at(self.orientation, 0.)
            .map_err(|_| PathError::InvalidInput)?;
        torque.validate().map_err(|_| PathError::InvalidInput)?;
        let material = crate::astrophysics_spin::RotatingMaterialMoment {
            moment,
            orientation: self.orientation,
            omega: [0.; 3],
        };
        let law = ArcTorque {
            polynomial: torque,
            rotating: None,
            material: Some(material),
        };
        let (_, impulse_bound) = law.envelopes(duration).map_err(PathError::Integrator)?;
        let feedback = (0..3)
            .map(|j| norm(moment.columns[j]) + norm(moment.rate_columns[j]) * duration)
            .sum();
        // The rotating tensor discriminates this own-body plan; the local
        // argument is used only by legacy single-arm plans in the shared arc.
        self.prepare_forcing_path_impl(
            duration,
            config,
            impulse_bound,
            Some(([0.; 3], feedback)),
            |time| {
                let moment = crate::rigid_motion::MaterialForceMoment {
                    columns: std::array::from_fn(|j| {
                        std::array::from_fn(|k| {
                            moment.rate_columns[j][k].mul_add(time, moment.columns[j][k])
                        })
                    }),
                    ..moment
                };
                Ok((
                    ArcTorque {
                        polynomial: torque.shifted(time).map_err(PathError::Integrator)?,
                        rotating: None,
                        material: Some(crate::astrophysics_spin::RotatingMaterialMoment {
                            moment,
                            ..material
                        }),
                    },
                    duration,
                ))
            },
        )
    }

    /// Shared adaptive owner for prescribed world forcing. Each law is rebased
    /// to time and must remain valid until the returned interval boundary.
    pub(crate) fn prepare_forcing_path(
        self,
        duration: f64,
        config: Config,
        impulse_bound: f64,
        forcing: impl FnMut(f64) -> Result<(ArcTorque, f64), PathError>,
    ) -> Result<SpinPath, PathError> {
        self.prepare_forcing_path_impl(duration, config, impulse_bound, None, forcing)
    }

    fn prepare_forcing_path_impl(
        self,
        duration: f64,
        config: Config,
        impulse_bound: f64,
        material: Option<([f64; 3], f64)>,
        mut forcing: impl FnMut(f64) -> Result<(ArcTorque, f64), PathError>,
    ) -> Result<SpinPath, PathError> {
        self.angular_velocity().map_err(PathError::Integrator)?;
        if !duration.is_finite()
            || duration <= 0.
            || !config.max_angular_error_rad.is_finite()
            || config.max_angular_error_rad <= 0.
            || !config.min_step_s.is_finite()
            || config.min_step_s <= 0.
            || !impulse_bound.is_finite()
            || impulse_bound < 0.
            || config.max_arcs == 0
            || config.max_trials == 0
        {
            return Err(PathError::InvalidInput);
        }
        let inverse_min = 1. / self.inertia.iter().copied().fold(f64::INFINITY, f64::min);
        let inverse_max = 1. / self.inertia.iter().copied().fold(0., f64::max);
        let feedback = material.map_or(0., |(_, strength)| strength);
        if !feedback.is_finite() || feedback < 0. {
            return Err(PathError::NumericalBound);
        }
        let momentum_error_per_rad = feedback.sqrt() / inverse_min.sqrt();
        let feedback_rate = inverse_min.sqrt() * feedback.sqrt();
        let global_rate =
            2. * (inverse_min - inverse_max) * (norm(self.angular_momentum) + impulse_bound)
                + feedback_rate;
        if !momentum_error_per_rad.is_finite() {
            return Err(PathError::NumericalBound);
        }
        let reserve = (-global_rate * duration).exp();
        if !global_rate.is_finite() || reserve == 0. {
            return Err(PathError::NumericalBound);
        }
        let mut path = SpinPath {
            initial: self,
            duration,
            segments: Vec::new(),
            trials: 0,
            momentum_error_per_rad,
            source_momentum_error: 0.,
        };
        let mut state = self;
        let mut time = 0.;
        let mut step = duration;
        let mut error = 0.;
        while time < duration {
            if path.segments.len() >= config.max_arcs || path.trials >= config.max_trials {
                return Err(PathError::Budget);
            }
            let (torque, boundary) = forcing(time)?;
            if !boundary.is_finite() || boundary <= time || boundary > duration {
                return Err(PathError::InvalidInput);
            }
            torque.validate().map_err(|_| PathError::InvalidInput)?;
            let proposed_step = step;
            step = step.min(boundary - time);
            let end = if step == boundary - time {
                boundary
            } else {
                time + step
            };
            step = end - time;
            if end <= time || (step < config.min_step_s && end != boundary) {
                return Err(PathError::Budget);
            }
            path.trials += 1;
            let trial = material
                .map_or_else(
                    || state.prepare_forced_arc(torque, step),
                    |(local, _)| state.prepare_material_force_arc(torque, local, step),
                )
                .map_err(PathError::Integrator)
                .and_then(|arc| {
                    local_bound(arc, feedback_rate)
                        .map(|(local, growth)| (arc, local + growth * error))
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
                    // A boundary-clipped remainder must not become the next
                    // interval's adaptive scale (it may be only a few ulps).
                    step = (if end == boundary { proposed_step } else { step } * 2.).min(duration);
                }
                Err(PathError::Integrator(Error::InvalidInput)) => {
                    return Err(PathError::Integrator(Error::InvalidInput));
                }
                _ => step *= 0.5,
            }
        }
        if !(path.model_angular_error_rad() * momentum_error_per_rad).is_finite() {
            return Err(PathError::NumericalBound);
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
    /// Model momentum error from body-local feedback and any accounted shared
    /// source. Standalone prescribed forcing is conditional on its nominal
    /// source path and has no momentum model error. Floating arithmetic is not
    /// certified by this bound.
    pub fn model_angular_momentum_error(&self) -> f64 {
        self.model_angular_error_rad() * self.momentum_error_per_rad + self.source_momentum_error
    }

    /// Add uncertainty from the shared source's physical momentum. Equal and
    /// opposite forces at one point have opposite intrinsic momentum errors
    /// because their nominal and physical COM paths have the same orbital terms.
    pub(crate) fn account_source_momentum_error(
        mut self,
        source_error: f64,
        lipschitz: f64,
        inverse_inertia: f64,
        limit: f64,
    ) -> Result<Self, PathError> {
        if self.momentum_error_per_rad != 0.
            || self.source_momentum_error != 0.
            || [source_error, lipschitz, inverse_inertia, limit]
                .iter()
                .any(|v| !v.is_finite())
            || source_error < 0.
            || lipschitz < 0.
            || inverse_inertia <= 0.
            || limit <= 0.
        {
            return Err(PathError::InvalidInput);
        }
        for segment in &mut self.segments {
            let additional = (inverse_inertia * (source_error * segment.end_s))
                * (lipschitz * segment.end_s).exp();
            let bound = segment.model_angular_error_rad + additional;
            if !bound.is_finite() {
                return Err(PathError::NumericalBound);
            }
            if bound > limit {
                return Err(PathError::Budget);
            }
            segment.model_angular_error_rad = bound;
        }
        self.source_momentum_error = source_error;
        Ok(self)
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
