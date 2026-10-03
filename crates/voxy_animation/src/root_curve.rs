use super::{Interpolation, Playback, Vec3Key};
use glam::{DVec3, Vec3};

#[derive(Debug)]
struct Knot {
    time: f64,
    value: DVec3,
    area: DVec3,
    coefficients: [DVec3; 4],
}

/// Relative polynomial positions and prefix integrals; shared by clip clones.
#[derive(Debug)]
pub(super) struct RootCurve {
    knots: Vec<Knot>,
    duration: f64,
    playback: Playback,
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
            knots.push(Knot {
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
        }
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
