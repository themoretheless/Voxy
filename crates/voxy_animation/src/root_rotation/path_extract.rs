//! Ordered path extraction, Bezier subdivision and quaternion algebra helpers.
use super::*;

impl RootRotationCurve {
    /// Extracts an ordered path over [start,end], with relative span times.
    /// Held channels remain held; STEP events belong to (start,end]. Cubic spans
    /// are subdivided until a nonzero norm and an angular travel bound are proved
    /// from their Bezier control hull. No sampled-point test certifies a whole span.
    /// # Errors
    /// Bounds output to 1..=4096 spans. Invalid time, a singular quaternion, or
    /// exhausted span/subdivision work rejects the whole request without a prefix.
    // Exact equality assigns STEP boundaries once; epsilon comparisons duplicate events.
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn path(
        &self,
        start: f64,
        end: f64,
        max_spans: usize,
    ) -> Result<RootRotationPath, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end < start {
            return Err(AnimationError::InvalidSampleTime);
        }
        if !(1..=MAX_ROOT_ROTATION_SPANS).contains(&max_spans) {
            return Err(AnimationError::RootRotationBudget);
        }
        let initial = self.sample(start)?;
        let end_rotation = (initial.conjugate() * self.sample(end)?).normalize();
        let mut result = RootRotationPath {
            duration: end - start,
            spans: Vec::new(),
            end_rotation,
        };
        if start == end {
            return Ok(result);
        }
        if self.0.constant {
            result.spans.push(RootRotationSpan {
                start: 0.,
                end: end - start,
                shape: Shape::Hold(DQuat::IDENTITY),
                left: DQuat::IDENTITY,
                right: DQuat::IDENTITY,
                speed_bound: Some(0.),
            });
            return Ok(result);
        }
        let (mut cycle, mut phase) = self.cycle_phase(start)?;
        let active_end = if self.0.playback == Playback::Clamp {
            end.min(self.0.duration)
        } else {
            end
        };
        let mut cursor = start.min(active_end);
        let right = self.0.origin.conjugate();
        while cursor < active_end {
            let remaining = active_end - cursor;
            let phase_end = self.0.duration.min(phase + remaining);
            let left = initial.conjugate() * power(self.0.cycle, cycle);
            let base = cursor - phase - start;
            let mut at = phase;
            while at < phase_end {
                let upper = self.0.knots.partition_point(|k| k.time <= at);
                let next = self.0.knots.get(upper);
                let until = next.map_or(phase_end, |k| phase_end.min(k.time));
                let shape = if upper == 0 {
                    Shape::Hold(self.0.knots.first().map_or(self.0.fallback, |k| k.value))
                } else {
                    let knot = &self.0.knots[upper - 1];
                    if let Some(next) = next {
                        let dt = next.time - knot.time;
                        restrict(&knot.shape, (at - knot.time) / dt, (until - knot.time) / dt)?
                    } else {
                        Shape::Hold(knot.value)
                    }
                };
                append(
                    &mut result.spans,
                    RootRotationSpan {
                        start: base + at,
                        end: base + until,
                        shape,
                        left,
                        right,
                        speed_bound: None,
                    },
                    max_spans,
                    0,
                )?;
                if let Some(event) = next
                    .filter(|k| self.0.mode == Interpolation::Step && upper > 0 && k.time == until)
                {
                    let from = self.0.knots[upper - 1].value;
                    let axis = log(from.conjugate() * event.value);
                    if axis != DVec3::ZERO {
                        append(
                            &mut result.spans,
                            RootRotationSpan {
                                start: base + until,
                                end: base + until,
                                shape: Shape::Step { from, axis },
                                left,
                                right,
                                speed_bound: None,
                            },
                            max_spans,
                            0,
                        )?;
                    }
                }
                if until <= at {
                    return Err(AnimationError::RootRotationBudget);
                }
                at = until;
            }
            let next = cursor + (phase_end - phase);
            if next <= cursor {
                return Err(AnimationError::RootRotationBudget);
            }
            cursor = next;
            phase = 0.;
            cycle = cycle
                .checked_add(1)
                .ok_or(AnimationError::RootRotationBudget)?;
        }
        if self.0.playback == Playback::Clamp && end > self.0.duration {
            append(
                &mut result.spans,
                RootRotationSpan {
                    start: (self.0.duration - start).max(0.),
                    end: end - start,
                    shape: Shape::Hold(end_rotation),
                    left: DQuat::IDENTITY,
                    right: DQuat::IDENTITY,
                    speed_bound: Some(0.),
                },
                max_spans,
                0,
            )?;
        }
        Ok(result)
    }
}

pub(super) fn append(
    out: &mut Vec<RootRotationSpan>,
    mut span: RootRotationSpan,
    capacity: usize,
    depth: usize,
) -> Result<(), AnimationError> {
    if span.end <= span.start && !span.is_step() {
        return Err(AnimationError::RootRotationBudget);
    }
    if out.len() >= capacity || depth > 48 {
        return Err(AnimationError::RootRotationBudget);
    }
    span.speed_bound = match &span.shape {
        Shape::Hold(_) => Some(0.),
        Shape::Step { .. } => None,
        Shape::Arc { axis, .. } => Some(axis.length() / (span.end - span.start)),
        Shape::Cubic(control) => {
            // Normalize only after proving a positive lower norm on the whole
            // interval. The hull bounds |q'|; |omega| <= 2 |q'| / |q|.
            let norm = norm_lower_bound(*control);
            let derivative = 3.
                * control
                    .windows(2)
                    .map(|p| (p[1] - p[0]).length())
                    .fold(0_f64, f64::max);
            let travel = 2. * derivative / norm;
            if norm > 0. && travel <= std::f64::consts::FRAC_PI_2 {
                Some(travel / (span.end - span.start))
            } else {
                unit(bezier(*control, 0.5))?;
                let middle = (span.start + span.end) * 0.5;
                if middle <= span.start || middle >= span.end {
                    return Err(AnimationError::RootRotationBudget);
                }
                let (a, b) = split(*control, 0.5);
                let first = RootRotationSpan {
                    end: middle,
                    shape: Shape::Cubic(a),
                    ..span.clone()
                };
                let last = RootRotationSpan {
                    start: middle,
                    shape: Shape::Cubic(b),
                    ..span
                };
                append(out, first, capacity, depth + 1)?;
                return append(out, last, capacity, depth + 1);
            }
        }
    };
    out.push(span);
    Ok(())
}

pub(super) fn norm_lower_bound(c: [DVec4; 4]) -> f64 {
    let low = c.into_iter().fold(DVec4::splat(f64::INFINITY), DVec4::min);
    let high = c
        .into_iter()
        .fold(DVec4::splat(f64::NEG_INFINITY), DVec4::max);
    let box_distance = DVec4::from_array(std::array::from_fn(|i| {
        low[i].max(0.).max((-high[i]).max(0.))
    }))
    .length();
    let middle = bezier(c, 0.5);
    let projection = if middle.length() > 0. {
        let axis = middle.normalize();
        c.into_iter()
            .map(|v| v.dot(axis))
            .fold(f64::INFINITY, f64::min)
    } else {
        0.
    };
    let magnitude = c.into_iter().map(DVec4::length).fold(0_f64, f64::max);
    (box_distance.max(projection) - 256. * f64::EPSILON * magnitude).max(0.)
}
pub(super) fn restrict(shape: &Shape, start: f64, end: f64) -> Result<Shape, AnimationError> {
    Ok(match shape {
        Shape::Hold(q) => Shape::Hold(*q),
        Shape::Arc { axis, .. } => Shape::Arc {
            from: shape.sample(start)?,
            axis: *axis * (end - start),
        },
        Shape::Cubic(control) => {
            let first = split(*control, end).0;
            Shape::Cubic(if start == 0. {
                first
            } else {
                split(first, start / end).1
            })
        }
        Shape::Step { .. } => unreachable!("events are assembled separately"),
    })
}
pub(super) fn bezier(c: [DVec4; 4], t: f64) -> DVec4 {
    split(c, t).0[3]
}
pub(super) fn bezier_derivative(c: [DVec4; 4], t: f64) -> DVec4 {
    let a = (c[1] - c[0]) * 3.;
    let b = (c[2] - c[1]) * 3.;
    let d = (c[3] - c[2]) * 3.;
    a.lerp(b, t).lerp(b.lerp(d, t), t)
}
pub(super) fn split(c: [DVec4; 4], t: f64) -> ([DVec4; 4], [DVec4; 4]) {
    let a = c[0].lerp(c[1], t);
    let b = c[1].lerp(c[2], t);
    let d = c[2].lerp(c[3], t);
    let e = a.lerp(b, t);
    let f = b.lerp(d, t);
    let g = e.lerp(f, t);
    ([c[0], a, e, g], [g, f, d, c[3]])
}
pub(super) fn unit(value: DVec4) -> Result<DQuat, AnimationError> {
    let scale = value.abs().max_element();
    if !value.is_finite() || scale == 0. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    Ok(DQuat::from_array((value / scale).normalize().to_array()))
}
pub(super) fn double(q: Quat) -> DQuat {
    DQuat::from_array(q.to_array().map(f64::from)).normalize()
}
pub(super) fn log(mut q: DQuat) -> DVec3 {
    q = q.normalize();
    if q.w < 0. {
        q = -q;
    }
    let vector = DVec3::new(q.x, q.y, q.z);
    let length = vector.length();
    if length == 0. {
        DVec3::ZERO
    } else {
        vector * (2. * length.atan2(q.w) / length)
    }
}
pub(super) fn power_with_error(
    mut q: DQuat,
    mut error: [f64; 4],
    mut n: u64,
) -> Result<(DQuat, [f64; 4]), AnimationError> {
    let mut result = DQuat::IDENTITY;
    let mut result_error = [0.; 4];
    while n != 0 {
        if n & 1 != 0 {
            let next = (result * q).normalize();
            result_error = crate::root_rigid::quaternion_composition_evaluation_error_bounds(
                result.to_array(),
                result_error,
                q.to_array(),
                error,
                next.to_array(),
            )?;
            result = next;
        }
        n >>= 1;
        let next = (q * q).normalize();
        if n != 0 {
            error = crate::root_rigid::quaternion_composition_evaluation_error_bounds(
                q.to_array(),
                error,
                q.to_array(),
                error,
                next.to_array(),
            )?;
        }
        q = next;
    }
    Ok((result, result_error))
}

pub(super) fn power(mut q: DQuat, mut n: u64) -> DQuat {
    let mut result = DQuat::IDENTITY;
    while n != 0 {
        if n & 1 != 0 {
            result = (result * q).normalize();
        }
        n >>= 1;
        q = (q * q).normalize();
    }
    result
}
