use super::{Interpolation, Playback, Vec3Key};
use glam::{DVec3, Vec3};

#[derive(Debug)]
struct Knot {
    time: f64,
    value: DVec3,
    area: DVec3,
    coefficients: [DVec3; 4],
    compilation_error: Result<[f64;3],super::AnimationError>,
}

/// Relative polynomial positions and prefix integrals; shared by clip clones.
#[derive(Debug)]
pub(super) struct RootCurve {
    knots: Vec<Knot>,
    duration: f64,
    playback: Playback,
    mode: Interpolation,
}

impl RootCurve {
    pub(super) fn new(
        keys: &[Vec3Key],
        mode: Interpolation,
        tangents: &[[Vec3; 2]],
        duration: f32,
        playback: Playback,
    ) -> Self {
        let origin = keys.first().map_or(DVec3::ZERO, |k| k.value.as_dvec3());
        let mut knots: Vec<Knot> = Vec::with_capacity(keys.len());
        let mut area = DVec3::ZERO;
        for (i, key) in keys.iter().enumerate() {
            let value = key.value.as_dvec3() - origin;
            let coefficients = if let Some(next) = keys.get(i + 1) {
                let end = next.value.as_dvec3() - origin;
                match mode {
                    Interpolation::Step => [value, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO],
                    Interpolation::Linear => [value, end - value, DVec3::ZERO, DVec3::ZERO],
                    Interpolation::CubicSpline => {
                        let dt = f64::from(next.time) - f64::from(key.time);
                        let out = tangents[i][1].as_dvec3() * dt;
                        let incoming = tangents[i + 1][0].as_dvec3() * dt;
                        [
                            value,
                            out,
                            (end - value) * 3. - out * 2. - incoming,
                            (value - end) * 2. + out + incoming,
                        ]
                    }
                }
            } else {
                [value, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO]
            };
            let next = keys.get(i+1);
            let compilation_error = crate::root_rigid::translation_coefficient_error_bounds(
                origin,key.value.as_dvec3(),next.map(|key|key.value.as_dvec3()),
                [f64::from(key.time),next.map_or(f64::from(key.time),|key|f64::from(key.time))],
                if mode == Interpolation::CubicSpline && next.is_some() {
                    [tangents[i][1].as_dvec3(),tangents[i+1][0].as_dvec3()]
                } else {[DVec3::ZERO;2]},mode,coefficients);
            knots.push(Knot {
                compilation_error,
                time: f64::from(key.time),
                value,
                area,
                coefficients,
            });
            if let Some(next) = keys.get(i + 1) {
                area += Self::segment_area(coefficients, 1.)
                    * (f64::from(next.time) - f64::from(key.time));
            }
        }
        Self {
            knots,
            duration: f64::from(duration),
            playback,
            mode,
        }
    }

    pub(super) fn compilation_error_bounds(&self) -> Result<[f64;3],super::AnimationError> {
        let mut bound = [0_f64;3];
        for knot in &self.knots {
            let error = knot.compilation_error.clone()?;
            for axis in 0..3 {bound[axis]=bound[axis].max(error[axis]);}
        }
        Ok(bound)
    }

    pub(super) fn piece_error_bounds(&self, start: f64, end: f64)
        -> Result<[f64;3],super::AnimationError> {
        if !start.is_finite() || !end.is_finite() || start<0. || end<start || end>self.duration {
            return Err(super::AnimationError::InvalidSampleTime);
        }
        let first = self.knots.partition_point(|knot|knot.time<=start);
        if first==0 {
            if self.knots.first().is_some_and(|knot|end>knot.time) {
                return Err(super::AnimationError::InvalidSampleTime);
            }
            return Ok([0.;3]);
        }
        let knot = &self.knots[first-1];
        let compilation = knot.compilation_error.clone()?;
        let Some(next) = self.knots.get(first) else {return Ok(compilation);};
        if end>next.time {return Err(super::AnimationError::InvalidSampleTime);}
        crate::root_rigid::translation_piece_error_bounds(knot.coefficients,compilation,
            [knot.time,next.time],[start,end],self.piece(start,end))
    }

    pub(super) fn phase_evaluation_error_bounds(&self,phase:f64)
        -> Result<[f64;3],super::AnimationError> {
        if !phase.is_finite() || phase<0. || phase>self.duration {
            return Err(super::AnimationError::InvalidSampleTime);
        }
        let upper = self.knots.partition_point(|knot|knot.time<=phase);
        if upper==0 {return Ok([0.;3]);}
        let knot = &self.knots[upper-1];
        let compilation = knot.compilation_error.clone()?;
        let Some(next) = self.knots.get(upper) else {return Ok(compilation);};
        crate::root_rigid::translation_phase_evaluation_error_bounds(knot.coefficients,
            compilation,[knot.time,next.time],phase,self.position(phase))
    }

    pub(super) fn interval_evaluation_error_bounds(&self,start:f64,end:f64)
        -> Result<[f64;3],super::AnimationError> {
        if !start.is_finite() || !end.is_finite() || start<0. || end<start || end>self.duration {
            return Err(super::AnimationError::InvalidSampleTime);
        }
        let upper = self.knots.partition_point(|knot|knot.time<=start);
        if upper==0 {
            if self.knots.first().is_some_and(|knot|end>knot.time) {
                return Err(super::AnimationError::InvalidSampleTime);
            }
            return Ok([0.;3]);
        }
        let knot = &self.knots[upper-1];
        let Some(next) = self.knots.get(upper) else {return knot.compilation_error.clone();};
        if end>next.time {return Err(super::AnimationError::InvalidSampleTime);}
        let mut error = crate::root_rigid::translation_interval_evaluation_error_bounds(knot.coefficients,
            knot.compilation_error.clone()?,[knot.time,next.time],[start,end])?;
        if end==next.time {
            let endpoint = next.compilation_error.clone()?;
            for axis in 0..3 {error[axis]=error[axis].max(endpoint[axis]);}
        }
        Ok(error)
    }

    fn segment_area(c: [DVec3; 4], u: f64) -> DVec3 {
        c[0] * u + c[1] * (u * u / 2.) + c[2] * (u * u * u / 3.) + c[3] * (u * u * u * u / 4.)
    }

    fn local(&self, time: f64) -> (DVec3, DVec3) {
        let Some(first) = self.knots.first() else {
            return (DVec3::ZERO, DVec3::ZERO);
        };
        if time < first.time {
            return (DVec3::ZERO, DVec3::ZERO);
        }
        let i = self.knots.partition_point(|k| k.time <= time) - 1;
        let k = &self.knots[i];
        if let Some(next) = self.knots.get(i + 1) {
            let dt = next.time - k.time;
            let u = (time - k.time) / dt;
            let c = k.coefficients;
            let value = ((c[3] * u + c[2]) * u + c[1]) * u + c[0];
            (value, k.area + Self::segment_area(c, u) * dt)
        } else {
            (k.value, k.area + k.value * (time - k.time))
        }
    }

    pub(super) fn position(&self, time: f64) -> DVec3 {
        self.local(time).0
    }
    pub(super) fn has_motion(&self, axes: [bool; 3]) -> bool {
        self.knots.iter().any(|knot| {
            knot.coefficients.iter().any(|value| {
                axes.into_iter()
                    .enumerate()
                    .any(|(i, active)| active && value[i] != 0.)
            })
        })
    }
    pub(super) fn cuts(
        &self,
        start: f64,
        end: f64,
        limit: usize,
    ) -> Result<Vec<f64>, super::AnimationError> {
        let first = self.knots.partition_point(|knot| knot.time <= start);
        let last = self.knots.partition_point(|knot| knot.time <= end);
        if last - first > limit {
            return Err(super::AnimationError::RootRigidBudget);
        }
        Ok(self.knots[first..last]
            .iter()
            .map(|knot| knot.time)
            .collect())
    }
    pub(super) fn jump(&self, time: f64) -> Option<[DVec3; 2]> {
        if self.mode != Interpolation::Step {
            return None;
        }
        let index = self.knots.partition_point(|knot| knot.time < time);
        if index == 0 || self.knots.get(index)?.time != time {
            return None;
        }
        let values = [self.knots[index - 1].value, self.knots[index].value];
        (values[0] != values[1]).then_some(values)
    }
    /// Exact Bernstein controls on a single key interval; the right endpoint
    /// remains the pre-event value for STEP, assigning the jump separately.
    pub(super) fn piece(&self, start: f64, end: f64) -> [DVec3; 4] {
        let first = self.knots.partition_point(|knot| knot.time <= start);
        if first == 0 {
            return [DVec3::ZERO; 4];
        }
        let knot = &self.knots[first - 1];
        let Some(next) = self.knots.get(first) else {
            return [knot.value; 4];
        };
        let delta = next.time - knot.time;
        let u = (start - knot.time) / delta;
        let v = (end - start) / delta;
        let c = knot.coefficients;
        let a = ((c[3] * u + c[2]) * u + c[1]) * u + c[0];
        let b = (c[1] + c[2] * (2. * u) + c[3] * (3. * u * u)) * v;
        let d = (c[2] + c[3] * (3. * u)) * (v * v);
        let e = c[3] * (v * v * v);
        [a, a + b / 3., a + b * (2. / 3.) + d / 3., a + b + d + e]
    }

    fn unwrapped(&self, time: f64) -> (DVec3, DVec3) {
        let (end, cycle_area) = self.local(self.duration);
        match self.playback {
            Playback::Clamp => {
                if time > self.duration {
                    (end, cycle_area + end * (time - self.duration))
                } else {
                    self.local(time.max(0.))
                }
            }
            Playback::Loop => {
                let cycles = (time / self.duration).floor();
                let phase = time - cycles * self.duration;
                let (position, area) = self.local(phase);
                (
                    position + end * cycles,
                    area + cycle_area * cycles
                        + end * (self.duration * cycles * (cycles - 1.) / 2. + cycles * phase),
                )
            }
        }
    }

    /// Integration by parts of w(t) dR(t), including right-continuous STEP jumps.
    /// Whole cycles are summed algebraically rather than iterated.
    #[cfg(test)]
    fn weighted_delta(&self, start: f32, end: f32, w0: f64, w1: f64) -> Vec3 {
        self.integral(f64::from(start), f64::from(end), w0, w1)
            .as_vec3()
    }

    pub(super) fn integral(&self, start: f64, end: f64, w0: f64, w1: f64) -> DVec3 {
        if end == start {
            return DVec3::ZERO;
        }
        let (p0, a0) = self.unwrapped(start);
        let (p1, a1) = self.unwrapped(end);
        let slope = (w1 - w0) / (end - start);
        (p1 - p0) * w1 - (a1 - a0 - p0 * (end - start)) * slope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubic_weighted_integral_matches_analytic_polynomial_and_substeps() {
        let curve = RootCurve::new(
            &[
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::X,
                },
            ],
            Interpolation::CubicSpline,
            &[[Vec3::ZERO; 2]; 2],
            1.,
            Playback::Clamp,
        );
        // R=3t^2-2t^3, w=t: integral 6t^2-6t^3 = 2t^3-1.5t^4.
        let actual = curve.weighted_delta(0., 0.5, 0., 0.5);
        assert!(actual.abs_diff_eq(Vec3::X * 0.15625, 1e-7));
        let split =
            curve.weighted_delta(0., 0.25, 0., 0.25) + curve.weighted_delta(0.25, 0.5, 0.25, 0.5);
        assert!(actual.abs_diff_eq(split, 1e-7));
        let full = curve.weighted_delta(0., 1., 0., 1.);
        assert!(full.abs_diff_eq(Vec3::X * 0.5, 1e-7));
        assert_eq!(curve.weighted_delta(1., 2., 0.5, 1.), Vec3::ZERO);
    }

    #[test]
    fn step_jumps_use_weight_at_event_including_loop_boundaries() {
        let keys = [
            Vec3Key {
                time: 0.,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 0.25,
                value: Vec3::X * 2.,
            },
            Vec3Key {
                time: 0.75,
                value: Vec3::X * 5.,
            },
        ];
        let curve = RootCurve::new(&keys, Interpolation::Step, &[], 1., Playback::Loop);
        assert!(
            curve
                .weighted_delta(0., 1., 0., 1.)
                .abs_diff_eq(Vec3::X * 2.75, 1e-6)
        );
        assert!(
            curve
                .weighted_delta(0., 4., 0., 1.)
                .abs_diff_eq(Vec3::X * 10.25, 1e-6)
        );
        let split = curve.weighted_delta(0., 0.25, 0., 0.25)
            + curve.weighted_delta(0.25, 0.75, 0.25, 0.75)
            + curve.weighted_delta(0.75, 1., 0.75, 1.);
        assert!(split.abs_diff_eq(Vec3::X * 2.75, 1e-6));
        let boundary = RootCurve::new(
            &[
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::X * 3.,
                },
            ],
            Interpolation::Step,
            &[],
            1.,
            Playback::Loop,
        );
        assert_eq!(boundary.weighted_delta(0., 1., 0., 1.), Vec3::X * 3.);
        assert_eq!(boundary.weighted_delta(1., 2., 0., 1.), Vec3::X * 3.);
    }

    #[test]
    fn million_cycles_are_summed_and_large_constant_offsets_cancel_exactly() {
        let duration = 1. / 1048576.;
        let curve = RootCurve::new(
            &[
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: duration,
                    value: Vec3::X,
                },
            ],
            Interpolation::Linear,
            &[],
            duration,
            Playback::Loop,
        );
        assert_eq!(curve.weighted_delta(0., 1., 0., 1.), Vec3::X * 524288.);
        let constant = RootCurve::new(
            &[
                Vec3Key {
                    time: 0.,
                    value: Vec3::splat(1e38),
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::splat(1e38),
                },
            ],
            Interpolation::Linear,
            &[],
            1.,
            Playback::Loop,
        );
        assert_eq!(constant.weighted_delta(0., 8., 0., 1.), Vec3::ZERO);
    }
}

#[cfg(test)]
mod compilation_error_tests {
    use super::*;
    #[test]
    fn translation_compilation_proof_covers_lost_key_units_and_cubic_tangents() {
        let large = 2_f32.powi(100);
        for mode in [Interpolation::Step,Interpolation::Linear,Interpolation::CubicSpline] {
            let curve = RootCurve::new(&[
                Vec3Key {time:0.,value:Vec3::X*large},
                Vec3Key {time:0.75,value:Vec3::X},
            ],mode,&[[Vec3::ZERO,Vec3::X*0.25],[Vec3::X*0.5,Vec3::ZERO]],
                1.,Playback::Clamp);
            let error = curve.compilation_error_bounds().unwrap();
            // At the final key, exact relative coordinate is 1-2^100 while the
            // rounded cache stores -2^100; the lost unit must remain covered.
            assert!(error[0]>=1.);
            assert_eq!(error[1],0.);
            assert_eq!(error[2],0.);
            let piece = curve.piece_error_bounds(0.125,0.625).unwrap();
            if mode == Interpolation::Step {assert_eq!(piece,[0.;3]);}
            else {assert!(piece[0]>=1.);}
            assert_eq!(piece[1],0.);
            assert_eq!(piece[2],0.);
            assert!(curve.piece_error_bounds(0.125,0.875).is_err());
            assert!(curve.piece_error_bounds(f64::NAN,0.5).is_err());
            let evaluation = curve.phase_evaluation_error_bounds(0.5).unwrap();
            assert_eq!(evaluation[1],0.);
            assert_eq!(evaluation[2],0.);
            if mode==Interpolation::Step {assert_eq!(evaluation,[0.;3]);}
            else {assert!(evaluation[0]>=1.);}
            assert!(curve.phase_evaluation_error_bounds(-0.5).is_err());
            let uniform = curve.interval_evaluation_error_bounds(0.,0.75).unwrap();
            assert!(uniform[0]>=1.);
            assert_eq!(uniform[1],0.);
            assert_eq!(uniform[2],0.);



        }
        let empty = RootCurve::new(&[],Interpolation::Linear,&[],1.,Playback::Clamp);
        assert_eq!(empty.compilation_error_bounds().unwrap(),[0.;3]);
    }
}

#[cfg(test)]
mod uniform_evaluation_tests {
    use super::*;
    #[test]
    fn whole_interval_horner_error_preserves_zero_axes_and_covers_dyadic_curve() {
        let curve = RootCurve::new(&[
            Vec3Key {time:0.,value:Vec3::ZERO},Vec3Key {time:0.75,value:Vec3::X*0.75}],
            Interpolation::CubicSpline,&[[Vec3::ZERO,Vec3::X*0.25],
                [Vec3::X*0.5,Vec3::ZERO]],1.,Playback::Clamp);
        let error = curve.interval_evaluation_error_bounds(0.,0.75).unwrap();
        assert_eq!(error[1],0.);
        assert_eq!(error[2],0.);
        assert!(error[0]>0. && error[0]<1e-12);
        for i in 0..=64 {
            let u = f64::from(i)/64.;
            // All coefficients and sampled parameters are exact dyadics, small
            // enough that this expanded independent evaluation is exact in f64.
            let exact = 0.1875*u+1.5*u*u-0.9375*u*u*u;
            let actual = curve.position(0.75*u).x;
            assert!((exact-actual).abs()<=error[0]);
        }
        assert!(curve.interval_evaluation_error_bounds(0.,0.875).is_err());
        assert!(curve.interval_evaluation_error_bounds(f64::NAN,0.5).is_err());
    }
}
