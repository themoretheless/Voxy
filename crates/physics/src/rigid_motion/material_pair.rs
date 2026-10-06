//! Shared force and impact ownership for a pair of freely moving bodies.
use super::{
    Config, ContactBody, Error, MaterialPointForce, MaterialPointMotion, MotionLoad, MotionWork,
    RigidMotion, TorquePolynomial,
};

/// Equal and opposite affine forces at the first body's own material point.
/// Independent external COM forces and torques may also be supplied. Magnitudes are supplied by
/// the caller; this does not solve pressure or admit geometry. Angular model
/// budgets apply to each participant. Overlapping pairs cannot replace an
/// aggregate multi-body force plan with one trajectory per body.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialForcePair {
    paths: [RigidMotion; 2],
    material: MaterialPointForce,
    external: [MotionLoad; 2],
}

/// A staged pair impact. Reprepare the shared force plan for both participants;
/// neither participant may continue with an obsolete source trajectory.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialPairImpact {
    pub time_s: f64,
    pub bodies: [ContactBody; 2],
    pub impulse: crate::contact::NormalImpulse,
    pub remainder: Option<MaterialForcePair>,
}
impl MaterialPairImpact {
    pub fn endpoints(&self) -> [ContactBody; 2] {
        self.remainder.as_ref().map_or(self.bodies, |pair| {
            pair.paths.each_ref().map(RigidMotion::end)
        })
    }
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

impl MaterialForcePair {
    pub fn prepare(
        first: ContactBody,
        second: ContactBody,
        local_first: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<Self, Error> {
        Self::prepare_loaded(
            first,
            second,
            MaterialPointForce {
                local: local_first,
                force,
                force_rate,
            },
            [MotionLoad::zero(); 2],
            duration,
            config,
        )
    }
    pub fn prepare_loaded(
        first: ContactBody,
        second: ContactBody,
        material: MaterialPointForce,
        external: [MotionLoad; 2],
        duration: f64,
        config: Config,
    ) -> Result<Self, Error> {
        material.validate()?;
        for load in external {
            load.validate()?;
            load.shifted(duration)?;
        }
        material.shifted(duration)?;
        let MaterialPointForce {
            local: local_first,
            force,
            force_rate,
        } = material;
        first.energy().map_err(|_| Error::InvalidInput)?;
        second.energy().map_err(|_| Error::InvalidInput)?;
        if !duration.is_finite()
            || duration <= 0.
            || !config.max_angular_error_rad.is_finite()
            || config.max_angular_error_rad <= 0.
            || !config.min_step_s.is_finite()
            || config.min_step_s <= 0.
            || config.max_arcs == 0
            || config.max_trials == 0
            || local_first
                .iter()
                .chain(&force)
                .chain(&force_rate)
                .any(|v| !v.is_finite())
            || (first.spin.is_none() && local_first != [0.; 3])
            || second.spin.is_none()
        {
            return Err(Error::InvalidInput);
        }
        let receiver = second.spin.unwrap();
        let inverse_min = 1.
            / receiver
                .inertia
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
        let inverse_max = 1. / receiver.inertia.iter().copied().fold(0., f64::max);
        let opposite = force.map(|v| -v);
        let opposite_rate = force_rate.map(|v| -v);
        let opposite_material = MaterialPointForce {
            force: opposite,
            force_rate: opposite_rate,
            ..material
        };
        let (first_force, first_rate) = external[0].combined_force(material)?;
        let (second_force, second_rate) = external[1].combined_force(opposite_material)?;
        let relative = TorquePolynomial::moving_affine_arm(
            std::array::from_fn(|k| first.motion.position[k] - second.motion.position[k]),
            std::array::from_fn(|k| first.motion.velocity[k] - second.motion.velocity[k]),
            std::array::from_fn(|k| {
                first_force[k] / first.motion.mass - second_force[k] / second.motion.mass
            }),
            std::array::from_fn(|k| {
                first_rate[k] / first.motion.mass - second_rate[k] / second.motion.mass
            }),
            opposite,
            opposite_rate,
        )
        .map_err(|_| Error::NumericalFailure)?;
        let relative = crate::astrophysics_spin::ArcTorque::polynomial(relative)
            .add_polynomial(external[1].torque)
            .map_err(|_| Error::NumericalFailure)?
            .polynomial;
        let initial_arm = first.spin.map_or(local_first, |s| {
            crate::astrophysics_spin::rotate(s.orientation, local_first)
        });
        let radius = norm(local_first).max(norm(initial_arm));
        let force_integral =
            norm(force) * duration + (norm(force_rate) * (duration * 0.5)) * duration;
        let moment_bound = relative
            .envelopes(duration)
            .map_err(|_| Error::NumericalFailure)?
            .1
            + radius * force_integral;
        let lipschitz =
            2. * (inverse_min - inverse_max) * (norm(receiver.angular_momentum) + moment_bound);
        let feedback = norm(local_first) * (norm(force) + norm(force_rate) * duration);
        let source_scale = first.spin.map_or(0., |s| {
            let inverse = 1. / s.inertia.iter().copied().fold(f64::INFINITY, f64::min);
            feedback.sqrt() / inverse.sqrt()
        });
        // Shared-point moment conservation gives |delta L_second| =
        // |delta L_first|. Its contribution to receiver attitude is bounded by
        // inverse_min * source_momentum_error * t * exp(lipschitz*t).
        let amplification =
            (inverse_min * (source_scale * duration)) * (lipschitz * duration).exp();
        if !lipschitz.is_finite() || !amplification.is_finite() || !source_scale.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let source_budget = (config.max_angular_error_rad * 0.25) / amplification.max(1.);
        if !source_budget.is_finite() || source_budget <= 0. {
            return Err(Error::NumericalFailure);
        }
        let a = first.prepare_own_material_point_loaded_motion(
            material,
            external[0],
            duration,
            Config {
                max_angular_error_rad: source_budget,
                ..config
            },
        )?;
        let mut b = second.prepare_material_point_loaded_motion(
            &a,
            opposite_material,
            external[1],
            duration,
            Config {
                max_angular_error_rad: config.max_angular_error_rad * 0.5,
                ..config
            },
        )?;
        let source_error = a
            .rotation()
            .map_or(0., |path| path.model_angular_momentum_error());
        b.rotation = Some(
            b.rotation
                .take()
                .unwrap()
                .account_source_momentum_error(
                    source_error,
                    lipschitz,
                    inverse_min,
                    config.max_angular_error_rad,
                )
                .map_err(Error::Rotation)?,
        );
        Ok(Self {
            paths: [a, b],
            material,
            external,
        })
    }
    /// Read-only paths include the source contribution to the second body's
    /// model bounds. Use this pair's impact method to preserve mutual forcing.
    pub fn paths(&self) -> &[RigidMotion; 2] {
        &self.paths
    }
    pub fn duration(&self) -> f64 {
        self.paths[0].duration()
    }
    pub fn sample(&self, time: f64) -> Result<[ContactBody; 2], Error> {
        Ok([self.paths[0].sample(time)?, self.paths[1].sample(time)?])
    }
    pub fn point(&self, time: f64) -> Result<MaterialPointMotion, Error> {
        self.paths[0].sample_material_point(time, self.material.local)
    }
    pub fn work(&self, time: f64) -> Result<[MotionWork; 2], Error> {
        Ok([self.paths[0].work(time)?, self.paths[1].work(time)?])
    }
    /// External world angular impulse along the accepted COM paths, including
    /// orbital force moments. Internal common-point forces cancel in this sum.
    pub fn external_angular_impulse(&self, time: f64) -> Result<[f64; 3], Error> {
        let impulses = [0, 1].map(|i| {
            let load = self.external[i];
            self.paths[i].affine_wrench_angular_impulse(
                time,
                load.force,
                load.force_rate,
                load.torque,
            )
        });
        let [first, second] = impulses;
        let (first, second) = (first?, second?);
        let result: [f64; 3] = std::array::from_fn(|k| first[k] + second[k]);
        if result.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    /// Geometry and event time must already be admitted by the owning world.
    /// Remainder bounds are conditional on these accepted post-impact states.
    pub fn prepare_impact(
        &self,
        time: f64,
        point: [f64; 3],
        normal: [f64; 3],
        restitution: f64,
        config: Config,
    ) -> Result<MaterialPairImpact, Error> {
        let [mut a, mut b] = self.sample(time)?;
        let impulse =
            crate::contact::resolve_normal_impact(&mut a, Some(&mut b), point, normal, restitution)
                .map_err(Error::Contact)?;
        if impulse.relative_normal_speed >= 0. {
            return Err(Error::InvalidInput);
        }
        let remaining = self.duration() - time;
        let remainder = if remaining == 0. {
            None
        } else {
            Some(Self::prepare_loaded(
                a,
                b,
                self.material.shifted(time)?,
                [
                    self.external[0].shifted(time)?,
                    self.external[1].shifted(time)?,
                ],
                remaining,
                config,
            )?)
        };
        Ok(MaterialPairImpact {
            time_s: time,
            bodies: [a, b],
            impulse,
            remainder,
        })
    }
}
