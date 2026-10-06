//! Prepared COM motion and shared rotation under affine point forces or polynomial torque.
//! Collision geometry and event response remain with the owning world.
use crate::{
    astrophysics_spin::TorquePolynomial,
    contact::ContactBody,
    spin_path::{Config, PathError, SpinPath},
};

mod loads;
pub use loads::{MaterialForceMoment, MaterialPointForce, MotionLoad};
mod material_pair;
pub use material_pair::{MaterialForcePair, MaterialPairImpact};

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
    /// Retained only to reprepare polynomial forcing after a point impact.
    torque: Option<TorquePolynomial>,
    point_force: Option<PointForcePlan>,
    com_loads: Option<Vec<MotionLoad>>,
    material_loads: Option<(Vec<MaterialPointForce>, Vec<MotionLoad>)>,
    duration: f64,
    rotation: Option<SpinPath>,
    end: ContactBody,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PointForcePlan {
    Own {
        material: MaterialPointForce,
        external: MotionLoad,
    },
    Source {
        material: MaterialPointForce,
        external: MotionLoad,
    },
}
#[derive(Clone, Copy)]
enum MotionForcing<'a> {
    Moment {
        moment: MaterialForceMoment,
        external: MotionLoad,
    },
    Own {
        material: MaterialPointForce,
        external: MotionLoad,
    },
    Source {
        path: &'a RigidMotion,
        material: MaterialPointForce,
        external: MotionLoad,
    },
    Remainder {
        path: &'a RigidMotion,
        time: f64,
        material: MaterialPointForce,
        external: MotionLoad,
    },
}

/// Kinematics of a body-local material point on the prepared nominal path.
/// Derivatives are one-sided at spin-arc knots: right-sided internally and
/// left-sided at the final endpoint. They need not be continuous across arcs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialPointMotion {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub acceleration: [f64; 3],
    pub jerk: [f64; 3],
    pub arc_interval_s: Option<[f64; 2]>,
}

/// Integral of an affine world force applied at a body-local material point.
/// This probes the existing nominal path; it does not drive a new trajectory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialPointWork {
    pub com_force_work: f64,
    pub torque_work: f64,
    pub total_work: f64,
    pub angular_impulse: [f64; 3],
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

    /// Aggregate independent COM loads while retaining each original recipe
    /// for event re-preparation. All loads share world axes and interval origin.
    pub fn prepare_load_motion(
        self,
        loads: impl IntoIterator<Item = MotionLoad>,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        let loads: Vec<_> = loads.into_iter().collect();
        for load in &loads {
            load.shifted(duration)?;
        }
        let total = MotionLoad::aggregate(loads.iter().copied())?;
        let mut path = self.prepare_affine_motion(
            total.force,
            total.force_rate,
            total.torque,
            duration,
            config,
        )?;
        path.com_loads = Some(loads);
        Ok(path)
    }

    /// Multiple own-body material-point forces and independent COM loads,
    /// sharing one trajectory and retaining recipes across contact events.
    pub fn prepare_material_load_motion(
        self,
        points: impl IntoIterator<Item = MaterialPointForce>,
        external: impl IntoIterator<Item = MotionLoad>,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        let points: Vec<_> = points.into_iter().collect();
        let external: Vec<_> = external.into_iter().collect();
        for point in &points {
            point.shifted(duration)?;
            if self.spin.is_none() && point.local != [0.; 3] {
                return Err(Error::InvalidInput);
            }
        }
        for load in &external {
            load.shifted(duration)?;
        }
        let moment = MaterialForceMoment::aggregate(points.iter().copied())?;
        let load = MotionLoad::aggregate(external.iter().copied())?;
        let total = MotionLoad::aggregate([
            load,
            MotionLoad {
                force: moment.force,
                force_rate: moment.force_rate,
                ..MotionLoad::zero()
            },
        ])?;
        let mut path = self.prepare_affine_motion_impl(
            total.force,
            total.force_rate,
            load.torque,
            duration,
            config,
            Some(MotionForcing::Moment {
                moment,
                external: load,
            }),
        )?;
        path.material_loads = Some((points, external));
        Ok(path)
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
        self.prepare_affine_motion_impl(force, force_rate, torque, duration, config, None)
    }

    /// Drive this body with an affine force at a material point on a prescribed
    /// source path. The source is immutable and does not receive reaction force.
    /// Both paths share a time origin; this is not a coupled contact solve.
    pub fn prepare_material_point_force_motion(
        self,
        source: &RigidMotion,
        local: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        self.prepare_material_point_loaded_motion(
            source,
            MaterialPointForce {
                local,
                force,
                force_rate,
            },
            MotionLoad::zero(),
            duration,
            config,
        )
    }

    /// Additional COM loads do not acquire the material point's lever arm.
    pub fn prepare_material_point_loaded_motion(
        self,
        source: &RigidMotion,
        material: MaterialPointForce,
        external: MotionLoad,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        material.validate()?;
        external.validate()?;
        if self.spin.is_none() || (source.initial.spin.is_none() && material.local != [0.; 3]) {
            return Err(Error::InvalidInput);
        }
        source.sample(duration)?;
        material.shifted(duration)?;
        external.shifted(duration)?;
        let (force, rate) = external.combined_force(material)?;
        self.prepare_affine_motion_impl(
            force,
            rate,
            external.torque,
            duration,
            config,
            Some(MotionForcing::Source {
                path: source,
                material,
                external,
            }),
        )
    }

    /// Drive this body's own rotating point through the same midpoint owner.
    pub fn prepare_own_material_point_force_motion(
        self,
        local: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        self.prepare_own_material_point_loaded_motion(
            MaterialPointForce {
                local,
                force,
                force_rate,
            },
            MotionLoad::zero(),
            duration,
            config,
        )
    }

    /// Compose an own-point force with affine COM force and polynomial COM torque.
    /// The original contributions are retained independently for event rebasing.
    pub fn prepare_own_material_point_loaded_motion(
        self,
        material: MaterialPointForce,
        external: MotionLoad,
        duration: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        material.validate()?;
        external.validate()?;
        if self.spin.is_none() && material.local != [0.; 3] {
            return Err(Error::InvalidInput);
        }
        material.shifted(duration)?;
        external.shifted(duration)?;
        let (force, rate) = external.combined_force(material)?;
        self.prepare_affine_motion_impl(
            force,
            rate,
            external.torque,
            duration,
            config,
            Some(MotionForcing::Own { material, external }),
        )
    }

    fn prepare_affine_motion_impl(
        self,
        force: [f64; 3],
        force_rate: [f64; 3],
        torque: TorquePolynomial,
        duration: f64,
        config: Config,
        material: Option<MotionForcing<'_>>,
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
                .chain(&torque.jerk)
                .chain(&torque.snap)
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
            .map(|spin| {
                if let Some(MotionForcing::Source {
                    path: source,
                    material: point,
                    external,
                }) = material
                {
                    use crate::astrophysics_spin::{ArcTorque, RotatingArmForce, rotate};
                    let relative = TorquePolynomial::moving_affine_arm(
                        std::array::from_fn(|k| {
                            source.initial.motion.position[k] - self.motion.position[k]
                        }),
                        std::array::from_fn(|k| {
                            source.initial.motion.velocity[k] - self.motion.velocity[k]
                        }),
                        std::array::from_fn(|k| source.acceleration[k] - acceleration[k]),
                        std::array::from_fn(|k| source.jerk[k] - jerk[k]),
                        point.force,
                        point.force_rate,
                    )
                    .and_then(|law| {
                        ArcTorque::polynomial(law)
                            .add_polynomial(external.torque)
                            .map(|law| law.polynomial)
                    })
                    .map_err(PathError::Integrator)?;
                    let local = point.local;
                    let initial_arm = source
                        .initial
                        .spin
                        .map_or(local, |s| rotate(s.orientation, local));
                    let envelope = ArcTorque {
                        material: None,
                        polynomial: relative,
                        rotating: Some(RotatingArmForce {
                            arm: initial_arm,
                            omega: [0.; 3],
                            force: point.force,
                            rate: point.force_rate,
                        }),
                    };
                    let (_, impulse_bound) = envelope
                        .envelopes(duration)
                        .map_err(PathError::Integrator)?;
                    spin.prepare_forcing_path(duration, config, impulse_bound, |time| {
                        let source_body =
                            source.sample(time).map_err(|_| PathError::NumericalBound)?;
                        let source_arc = source.nominal_arc(time);
                        let arm = source_body
                            .spin
                            .map_or(local, |s| rotate(s.orientation, local));
                        let forcing = ArcTorque {
                            material: None,
                            polynomial: relative.shifted(time).map_err(PathError::Integrator)?,
                            rotating: Some(RotatingArmForce {
                                arm,
                                omega: source_arc.map_or([0.; 3], |s| s.arc.angular_velocity()),
                                force: std::array::from_fn(|k| {
                                    point.force_rate[k].mul_add(time, point.force[k])
                                }),
                                rate: point.force_rate,
                            }),
                        };
                        Ok((
                            forcing,
                            source_arc.map_or(duration, |s| s.end_s.min(duration)),
                        ))
                    })
                } else if let Some(MotionForcing::Remainder {
                    path: old,
                    time: offset,
                    material: point,
                    external: _,
                }) = material
                {
                    let previous = old.sample(offset).map_err(|_| PathError::InvalidInput)?;
                    let previous_acceleration = old
                        .acceleration_at(offset)
                        .map_err(|_| PathError::InvalidInput)?;
                    let correction = TorquePolynomial::moving_affine_arm(
                        std::array::from_fn(|k| {
                            previous.motion.position[k] - self.motion.position[k]
                        }),
                        std::array::from_fn(|k| {
                            previous.motion.velocity[k] - self.motion.velocity[k]
                        }),
                        std::array::from_fn(|k| previous_acceleration[k] - acceleration[k]),
                        std::array::from_fn(|k| old.jerk[k] - jerk[k]),
                        point.force,
                        point.force_rate,
                    )
                    .map_err(PathError::Integrator)?;
                    let segments = old
                        .rotation
                        .as_ref()
                        .ok_or(PathError::InvalidInput)?
                        .segments();
                    let mut impulse_bound = correction
                        .envelopes(duration)
                        .map_err(PathError::Integrator)?
                        .1;
                    for segment in segments.iter().filter(|s| s.end_s > offset) {
                        let start = offset.max(segment.start_s);
                        let (_, bound) = segment
                            .arc
                            .forcing()
                            .shifted(start - segment.start_s)
                            .and_then(|law| law.envelopes(segment.end_s - start))
                            .map_err(PathError::Integrator)?;
                        impulse_bound += bound;
                    }
                    spin.prepare_forcing_path(duration, config, impulse_bound, |time| {
                        let index = segments
                            .partition_point(|s| s.end_s - offset <= time)
                            .min(segments.len() - 1);
                        let segment = segments[index];
                        let forcing = segment
                            .arc
                            .forcing()
                            .shifted((offset - segment.start_s) + time)
                            .and_then(|law| law.add_polynomial(correction.shifted(time)?))
                            .map_err(PathError::Integrator)?;
                        Ok((forcing, (segment.end_s - offset).min(duration)))
                    })
                } else if let Some(MotionForcing::Own {
                    material: point,
                    external,
                }) = material
                {
                    spin.prepare_material_force_path_with_torque(
                        point.local,
                        point.force,
                        point.force_rate,
                        external.torque,
                        duration,
                        config,
                    )
                } else if let Some(MotionForcing::Moment { moment, external }) = material {
                    spin.prepare_material_moment_path(moment, external.torque, duration, config)
                } else {
                    spin.prepare_polynomial_path(torque, duration, config)
                }
            })
            .transpose()
            .map_err(Error::Rotation)?;
        let mut path = RigidMotion {
            initial: self,
            acceleration,
            force_rate,
            jerk,
            force,
            torque: material.is_none().then_some(torque),
            com_loads: None,
            material_loads: None,
            point_force: match material {
                Some(MotionForcing::Own { material, external }) => {
                    Some(PointForcePlan::Own { material, external })
                }
                Some(
                    MotionForcing::Source {
                        material, external, ..
                    }
                    | MotionForcing::Remainder {
                        material, external, ..
                    },
                ) => Some(PointForcePlan::Source { material, external }),
                None | Some(MotionForcing::Moment { .. }) => None,
            },
            duration,
            rotation,
            end: self,
        };
        path.end = path.evaluate(duration)?;
        if force_rate != [0.; 3] || material.is_some() {
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
                let impulse = if material.is_some() {
                    path.rotation
                        .as_ref()
                        .unwrap()
                        .segments()
                        .iter()
                        .try_fold(0., |sum, segment| {
                            segment.arc.torque_envelopes().map(|(_, bound)| sum + bound)
                        })
                        .map_err(|_| Error::NumericalFailure)?
                } else {
                    torque
                        .envelopes(duration)
                        .map_err(|_| Error::NumericalFailure)?
                        .1
                };
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
    fn prepare_remainder(
        &self,
        initial: ContactBody,
        time: f64,
        config: Config,
    ) -> Result<RigidMotion, Error> {
        let force = std::array::from_fn(|k| self.force_rate[k].mul_add(time, self.force[k]));
        let duration = self.duration - time;
        if let Some((points, loads)) = &self.material_loads {
            let points = points
                .iter()
                .map(|point| point.shifted(time))
                .collect::<Result<Vec<_>, _>>()?;
            let loads = loads
                .iter()
                .map(|load| load.shifted(time))
                .collect::<Result<Vec<_>, _>>()?;
            return initial.prepare_material_load_motion(points, loads, duration, config);
        }
        if let Some(loads) = &self.com_loads {
            let shifted = loads
                .iter()
                .map(|load| load.shifted(time))
                .collect::<Result<Vec<_>, _>>()?;
            return initial.prepare_load_motion(shifted, duration, config);
        }
        if let Some(plan) = self.point_force {
            match plan {
                PointForcePlan::Own { material, external } => initial
                    .prepare_own_material_point_loaded_motion(
                        material.shifted(time)?,
                        external.shifted(time)?,
                        duration,
                        config,
                    ),
                PointForcePlan::Source { material, external } => {
                    let material = material.shifted(time)?;
                    let external = external.shifted(time)?;
                    let (force, rate) = external.combined_force(material)?;
                    initial.prepare_affine_motion_impl(
                        force,
                        rate,
                        external.torque,
                        duration,
                        config,
                        Some(MotionForcing::Remainder {
                            path: self,
                            time,
                            material,
                            external,
                        }),
                    )
                }
            }
        } else if let Some(torque) = self.torque {
            initial.prepare_affine_motion(
                force,
                self.force_rate,
                torque.shifted(time).map_err(|_| Error::NumericalFailure)?,
                duration,
                config,
            )
        } else {
            Err(Error::InvalidInput)
        }
    }
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
        let (force_work, _) = self.affine_wrench_work(
            time,
            self.force,
            self.force_rate,
            TorquePolynomial::constant([0.; 3]),
        )?;
        let mut torque_work = 0.;
        if let Some(rotation) = &self.rotation {
            for segment in rotation.segments() {
                let dt = (time.min(segment.end_s) - segment.start_s).max(0.);
                if dt == 0. {
                    break;
                }
                let impulse = segment
                    .arc
                    .angular_impulse(dt)
                    .map_err(|_| Error::NumericalFailure)?;
                let omega = segment.arc.angular_velocity();
                torque_work += (0..3).map(|k| impulse[k] * omega[k]).sum::<f64>();
            }
        }
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
            .chain(&torque.jerk)
            .chain(&torque.snap)
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
            .chain(&torque.jerk)
            .chain(&torque.snap)
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

    /// Work and world-origin moment of F(t)=force+force_rate*t at a material point.
    /// Rodrigues time moments integrate the same nominal rotation arcs, with
    /// stable small-angle series. No polynomial approximation of the arm is used.
    pub fn material_point_force_work(
        &self,
        time: f64,
        local: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
    ) -> Result<MaterialPointWork, Error> {
        self.moving_material_point_force_work(time, self, local, force, force_rate)
    }

    fn nominal_arc(&self, time: f64) -> Option<&crate::spin_path::Segment> {
        self.rotation.as_ref().map(|rotation| {
            let segments = rotation.segments();
            let index = segments
                .partition_point(|s| s.end_s <= time)
                .min(segments.len() - 1);
            &segments[index]
        })
    }

    /// Work on this receiver of an affine force applied at a source material point.
    /// Source and receiver share a time origin; both paths must cover the prefix.
    /// Opposite forces at the same source point have reciprocal world moments.
    pub fn moving_material_point_force_work(
        &self,
        time: f64,
        source: &RigidMotion,
        local: [f64; 3],
        force: [f64; 3],
        force_rate: [f64; 3],
    ) -> Result<MaterialPointWork, Error> {
        use crate::astrophysics_spin::{cross, rotate, rotating_arm_integrals};
        if local.iter().any(|x| !x.is_finite())
            || (source.initial.spin.is_none() && local != [0.; 3])
            || (self.initial.spin.is_none() && (self != source || local != [0.; 3]))
        {
            return Err(Error::InvalidInput);
        }
        source.sample(time)?;
        let zero = TorquePolynomial::constant([0.; 3]);
        let (com_force_work, _) = self.affine_wrench_work(time, force, force_rate, zero)?;
        if force
            .iter()
            .zip(force_rate)
            .any(|(f, r)| !r.mul_add(time, *f).is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        let mut angular_impulse =
            self.affine_wrench_angular_impulse(time, force, force_rate, zero)?;
        let mut torque_work = 0.;
        let mut knots = vec![0., time];
        for path in [source, self] {
            if let Some(rotation) = &path.rotation {
                knots.extend(
                    rotation
                        .segments()
                        .iter()
                        .filter(|s| s.end_s < time)
                        .map(|s| s.end_s),
                );
            }
        }
        knots.sort_by(f64::total_cmp);
        knots.dedup();
        for interval in knots.windows(2) {
            let start = interval[0];
            let dt = interval[1] - start;
            let a = source.sample(start)?;
            let b = self.sample(start)?;
            let arm = a.spin.map_or(local, |s| rotate(s.orientation, local));
            let source_omega = source
                .nominal_arc(start)
                .map_or([0.; 3], |s| s.arc.angular_velocity());
            let omega = self
                .nominal_arc(start)
                .map_or([0.; 3], |s| s.arc.angular_velocity());
            let (mut a0, mut a1) = rotating_arm_integrals(arm, source_omega, dt)
                .map_err(|_| Error::NumericalFailure)?;
            let aa = source.acceleration_at(start)?;
            let ab = self.acceleration_at(start)?;
            for k in 0..3 {
                let r = a.motion.position[k] - b.motion.position[k];
                let v = a.motion.velocity[k] - b.motion.velocity[k];
                let acceleration = aa[k] - ab[k];
                let jerk = source.jerk[k] - self.jerk[k];
                a0[k] += dt * (r + dt * (v / 2. + dt * (acceleration / 6. + dt * jerk / 24.)));
                a1[k] += dt
                    * (dt * (r / 2. + dt * (v / 3. + dt * (acceleration / 8. + dt * jerk / 30.))));
            }
            let current = std::array::from_fn(|k| force_rate[k].mul_add(start, force[k]));
            let constant = cross(a0, current);
            let changing = cross(a1, force_rate);
            for k in 0..3 {
                let moment = constant[k] + changing[k];
                angular_impulse[k] += moment;
                torque_work += omega[k] * moment;
            }
        }
        let total_work = com_force_work + torque_work;
        if !total_work.is_finite()
            || !torque_work.is_finite()
            || angular_impulse.iter().any(|x| !x.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(MaterialPointWork {
            com_force_work,
            torque_work,
            total_work,
            angular_impulse,
        })
    }

    /// Follow a material point using this path's existing COM and rotation owners.
    /// Arc derivatives use the represented constant angular velocity, rather
    /// than substituting physical endpoint angular velocity from momentum.
    pub fn sample_material_point(
        &self,
        time: f64,
        local: [f64; 3],
    ) -> Result<MaterialPointMotion, Error> {
        use crate::astrophysics_spin::{cross, rotate};
        if local.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let body = self.sample(time)?;
        let arm = body.spin.map_or(local, |s| rotate(s.orientation, local));
        let (omega, arc_interval_s) = self.nominal_arc(time).map_or(([0.; 3], None), |s| {
            (s.arc.angular_velocity(), Some([s.start_s, s.end_s]))
        });
        let velocity = cross(omega, arm);
        let acceleration = cross(omega, velocity);
        let jerk = cross(omega, acceleration);
        let com_acceleration = self.acceleration_at(time)?;
        let result = MaterialPointMotion {
            position: std::array::from_fn(|k| body.motion.position[k] + arm[k]),
            velocity: std::array::from_fn(|k| body.motion.velocity[k] + velocity[k]),
            acceleration: std::array::from_fn(|k| com_acceleration[k] + acceleration[k]),
            jerk: std::array::from_fn(|k| self.jerk[k] + jerk[k]),
            arc_interval_s,
        };
        if result
            .position
            .iter()
            .chain(&result.velocity)
            .chain(&result.acceleration)
            .chain(&result.jerk)
            .any(|x| !x.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
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
        Some(first.prepare_remainder(a, time_s, config)?)
    };
    let second_remainder = if remaining == 0. {
        None
    } else {
        Some(second.prepare_remainder(b, time_s, config)?)
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
