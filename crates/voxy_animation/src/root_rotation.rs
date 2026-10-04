//! Ordered quaternion trajectories. Endpoint quaternions do not encode winding.
use super::{AnimationError, Interpolation, Playback, QuatKey};
use glam::{DQuat, DVec3, DVec4, Quat, Vec4};
use std::sync::Arc;

/// Work/storage limits apply to the selected locomotion channel, not every bone.
pub const MAX_ROOT_ROTATION_KEYS: usize = 65_536;
/// Combined compiled keys across all selected channels of one immutable clip.
pub const MAX_ROOT_ROTATION_CACHE_KEYS: usize = 65_536;
pub const MAX_ROOT_ROTATION_SPANS: usize = 4096;

#[derive(Clone, Debug)]
enum Shape {
    Hold(DQuat),
    Arc { from: DQuat, axis: DVec3 },
    Cubic([DVec4; 4]),
    Step { from: DQuat, axis: DVec3 },
}
impl Shape {
    fn sample(&self, u: f64) -> Result<DQuat, AnimationError> {
        match self {
            Self::Hold(q) => Ok(*q),
            Self::Arc { from, axis } | Self::Step { from, axis } => {
                Ok((*from * DQuat::from_scaled_axis(*axis * u)).normalize())
            }
            Self::Cubic(control) => unit(bezier(*control, u)),
        }
    }
}

#[derive(Debug)]
struct Knot {
    time: f64,
    value: DQuat,
    shape: Shape,
}
#[derive(Debug)]
struct Curve {
    knots: Vec<Knot>,
    fallback: DQuat,
    duration: f64,
    playback: Playback,
    mode: Interpolation,
    origin: DQuat,
    cycle: DQuat,
    constant: bool,
    key_normalization_error: [f64;4],
    cubic_control_error: Option<[f64;4]>,
}

/// Immutable compiled channel. Clones share coefficients; paths preserve key
/// order, loop turns, STEP events and the actual normalized cubic polynomial.
#[derive(Clone, Debug)]
pub struct RootRotationCurve(Arc<Curve>);

/// A continuous span or one right-continuous STEP event. Orientations are
/// right-multiplied increments from the requested interval's starting orientation.
/// Axes use the channel's initial parent-local frame.
/// Cubic spans remain curves: replacing them by endpoint arcs loses their path.
#[derive(Clone, Debug)]
pub struct RootRotationSpan {
    start: f64,
    end: f64,
    shape: Shape,
    left: DQuat,
    right: DQuat,
    speed_bound: Option<f64>,
}
impl RootRotationSpan {
    /// Preserves the authored curve while changing its coordinate basis.
    pub(super) fn conjugated(&self, basis: DQuat) -> Self {
        let mut span = self.clone();
        span.left = (basis * span.left).normalize();
        span.right = (span.right * basis.conjugate()).normalize();
        span
    }

    #[must_use]
    pub const fn start(&self) -> f64 {
        self.start
    }
    #[must_use]
    pub const fn end(&self) -> f64 {
        self.end
    }
    #[must_use]
    pub fn is_step(&self) -> bool {
        matches!(self.shape, Shape::Step { .. })
    }

    pub(super) fn cubic_velocity_inputs(&self) -> Option<([[f64;4];4], DQuat, DQuat)> {
        match &self.shape {
            Shape::Cubic(control) => Some((control.map(|q|q.to_array()), self.left, self.right)),
            _ => None,
        }
    }
    pub(super) fn arc_velocity_inputs(&self) -> Option<(DQuat,DVec3,DQuat,DQuat)> {
        match &self.shape {
            Shape::Hold(from) => Some((*from,DVec3::ZERO,self.left,self.right)),
            Shape::Arc {from,axis} => Some((*from,*axis,self.left,self.right)),
            _ => None,
        }
    }
    /// Upper bound on interval-local angular speed in radians per clip second.
    /// STEP is instantaneous and returns None; it needs separate event admission.
    #[must_use]
    pub const fn angular_speed_bound(&self) -> Option<f64> {
        self.speed_bound
    }

    /// Samples the entire span, including STEP's defined shortest event arc.
    /// # Errors
    /// Rejects fractions outside [0,1] or invalid quaternion normalization.
    pub fn sample(&self, fraction: f64) -> Result<DQuat, AnimationError> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        Ok((self.left * self.shape.sample(fraction)? * self.right).normalize())
    }

    /// Exact body-local arc for LINEAR/STEP/held values. Cubic returns None even
    /// when endpoints agree, preventing a consumer from silently flattening it.
    #[must_use]
    pub fn body_angular_displacement(&self) -> Option<DVec3> {
        match &self.shape {
            Shape::Cubic(_) => None,
            Shape::Hold(_) => Some(DVec3::ZERO),
            Shape::Arc { axis, .. } | Shape::Step { axis, .. } => {
                Some(self.right.conjugate() * *axis)
            }
        }
    }

    pub(super) fn with_times(&self, start: f64, end: f64) -> Result<Self, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end < start
            || (self.end > self.start && end <= start) {
            return Err(AnimationError::NumericalOverflow);
        }
        let mut span = self.clone();
        span.start = start;
        span.end = end;
        if self.end > self.start {
            let factor = (self.end-self.start)/(end-start);
            span.speed_bound = span.speed_bound.map(|bound| bound*factor);
            if span.speed_bound.is_some_and(|bound| !bound.is_finite()) {
                return Err(AnimationError::NumericalOverflow);
            }
        }
        Ok(span)
    }
    pub(super) fn shifted_and_postcomposed(&self, offset: f64, rotation: DQuat) -> Result<Self, AnimationError> {
        let mut span = self.with_times(self.start+offset, self.end+offset)?;
        span.right = (span.right*rotation).normalize();
        Ok(span)
    }
    pub(super) fn constant_velocity(start: f64, end: f64, from: DQuat, angular: DVec3)
        -> Result<Self, AnimationError> {
        let axis = from.conjugate() * angular * (end-start);
        let speed = angular.length();
        if !axis.is_finite() || !speed.is_finite() || end <= start {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Self { start, end, shape: Shape::Arc { from, axis },
            left:DQuat::IDENTITY, right:DQuat::IDENTITY, speed_bound:Some(speed) })
    }
    pub(super) fn held(start: f64, end: f64, rotation: DQuat) -> Self {
        Self {
            start,
            end,
            shape: Shape::Hold(rotation),
            left: DQuat::IDENTITY,
            right: DQuat::IDENTITY,
            speed_bound: Some(0.),
        }
    }
    pub(super) fn restricted(&self, start: f64, end: f64) -> Result<Self, AnimationError> {
        if self.is_step() || start < self.start || end > self.end || end <= start {
            return Err(AnimationError::RootRigidBudget);
        }
        let duration = self.end - self.start;
        Ok(Self {
            start,
            end,
            shape: restrict(
                &self.shape,
                (start - self.start) / duration,
                (end - self.start) / duration,
            )?,
            left: self.left,
            right: self.right,
            speed_bound: self.speed_bound,
        })
    }

    /// Analytic angular velocity in the interval-start frame; STEP has no finite derivative.
    /// # Errors
    /// Rejects an invalid sampling fraction or a singular cubic quaternion.
    pub fn angular_velocity(&self, fraction: f64) -> Result<Option<DVec3>, AnimationError> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        let dt = self.end - self.start;
        if dt == 0. { return Ok(None); }
        let velocity = match &self.shape {
            Shape::Step { .. } => return Ok(None),
            Shape::Hold(_) => DVec3::ZERO,
            Shape::Arc { from, axis } => *from * *axis / dt,
            Shape::Cubic(control) => {
                let value = bezier(*control, fraction);
                let derivative = bezier_derivative(*control, fraction) / dt;
                let q = unit(value)?;
                // q' q^-1 has the same imaginary part before/after removal of
                // the radial normalization derivative. Scale before division.
                let scaled = derivative / value.length();
                let product = DQuat::from_array(scaled.to_array()) * q.conjugate();
                DVec3::from_array([product.x, product.y, product.z]) * 2.
            }
        };
        let result = self.left * velocity;
        if !result.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Some(result))
    }

    /// Whole-span angular acceleration magnitude, per clip second squared.
    /// STEP has no finite derivative. Cubic bounds include normalization terms.
    /// # Errors
    /// Rejects unproved quaternion norms or overflowing derivative bounds.
    pub fn angular_acceleration_bound(&self) -> Result<Option<f64>, AnimationError> {
        let dt = self.end-self.start;
        if dt == 0. || self.is_step() { return Ok(None); }
        let Shape::Cubic(control) = &self.shape else { return Ok(Some(0.)); };
        let norm = norm_lower_bound(*control);
        if norm <= 0. { return Err(AnimationError::RootRotationBudget); }
        let first = control.windows(2).map(|p| 3.*(p[1]-p[0]).length()).fold(0_f64,f64::max)/dt;
        let second = control.windows(3).map(|p| 6.*(p[2]-2.*p[1]+p[0]).length()).fold(0_f64,f64::max)/(dt*dt);
        let bound = 2.*second/norm + 4.*(first/norm).powi(2);
        let control_norm = control.iter().map(|q| q.length()).fold(0_f64,f64::max);
        let guard = 4096.*f64::EPSILON*(bound+control_norm/norm/(dt*dt));
        let result = bound+guard;
        if !result.is_finite() { return Err(AnimationError::NumericalOverflow); }
        Ok(Some(result))
    }

    /// Whole-span speed bound for one rotated point, per clip second. Unlike a
    /// sphere bound, a tall point on a yaw axis contributes no height-dependent
    /// travel. STEP has no finite time derivative and returns None.
    /// # Errors
    /// Rejects nonfinite/overflowing vectors or an unproved cubic norm.
    pub fn point_speed_bound(&self, vector: DVec3) -> Result<Option<f64>, AnimationError> {
        let length = vector.length();
        if !vector.is_finite() || !length.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let vector = self.right * vector;
        let dt = self.end - self.start;
        let speed = match &self.shape {
            Shape::Step { .. } => return Ok(None),
            Shape::Hold(_) => 0.,
            Shape::Arc { axis, .. } => axis.cross(vector).length() / dt,
            Shape::Cubic(control) => {
                let scale = control
                    .iter()
                    .map(|q| q.abs().max_element())
                    .fold(0_f64, f64::max);
                let controls = control.map(|q| q / scale);
                let norm = norm_lower_bound(controls);
                if norm <= 0. {
                    return Err(AnimationError::InvalidRootRotationCurve);
                }
                let derivatives: [DVec4; 3] =
                    std::array::from_fn(|i| (controls[i + 1] - controls[i]) * 3.);
                let choose3 = [1., 3., 3., 1.];
                let choose2 = [1., 2., 1.];
                let choose5 = [1., 5., 10., 10., 5., 1.];
                let mut maximum = 0_f64;
                // omega_body = 2 Im(conj(q) q') / |q|^2. Its degree-five
                // numerator crossed with this point is a polynomial hull.
                for (degree, divisor) in choose5.into_iter().enumerate() {
                    let mut numerator = DVec3::ZERO;
                    for i in 0..4 {
                        if degree < i || degree - i >= 3 {
                            continue;
                        }
                        let j = degree - i;
                        let product = DQuat::from_array(controls[i].to_array()).conjugate()
                            * DQuat::from_array(derivatives[j].to_array());
                        let imaginary = DVec3::new(product.x, product.y, product.z);
                        numerator += imaginary * (2. * choose3[i] * choose2[j] / divisor);
                    }
                    maximum = maximum.max(numerator.cross(vector).length());
                }
                let control_norm = controls
                    .into_iter()
                    .map(DVec4::length)
                    .fold(0_f64, f64::max);
                let derivative_norm = derivatives
                    .into_iter()
                    .map(DVec4::length)
                    .fold(0_f64, f64::max);
                let guard = 2048. * f64::EPSILON * control_norm * derivative_norm * length;
                ((maximum + guard) / (norm * norm) / dt)
                    .min(self.speed_bound.unwrap_or(f64::INFINITY) * length)
            }
        };
        if !speed.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Some(speed))
    }

    /// Bounds `normal.dot(sample(u) * vector)` for every u in [0,1].
    /// This lets collision admission certify a complete curved trajectory rather
    /// than just its sampled poses. Cubic uses a degree-six rational Bezier hull;
    /// inconclusive denominator hulls fall back to the enclosing sphere.
    /// # Errors
    /// Rejects nonfinite vectors or overflowing projection magnitudes.
    pub fn projection_bounds(
        &self,
        vector: DVec3,
        normal: DVec3,
    ) -> Result<[f64; 2], AnimationError> {
        let radius = vector.length() * normal.length();
        if !vector.is_finite() || !normal.is_finite() || !radius.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let vector = self.right * vector;
        let normal = self.left.conjugate() * normal;
        let bounds = match &self.shape {
            Shape::Hold(q) => {
                let value = normal.dot(*q * vector);
                [value, value]
            }
            Shape::Arc { from, axis } | Shape::Step { from, axis } => {
                let angle = axis.length();
                if angle == 0. {
                    let value = normal.dot(*from * vector);
                    [value, value]
                } else {
                    let axis = *axis / angle;
                    let normal = from.conjugate() * normal;
                    let axial = axis * vector.dot(axis);
                    let a = normal.dot(vector - axial);
                    let b = normal.dot(axis.cross(vector));
                    let c = normal.dot(axial);
                    let evaluate = |u: f64| a * u.cos() + b * u.sin() + c;
                    let mut low = evaluate(0.).min(evaluate(angle));
                    let mut high = evaluate(0.).max(evaluate(angle));
                    for period in -2..=2 {
                        let u = b.atan2(a) + f64::from(period) * std::f64::consts::PI;
                        if u > 0. && u < angle {
                            low = low.min(evaluate(u));
                            high = high.max(evaluate(u));
                        }
                    }
                    [low, high]
                }
            }
            Shape::Cubic(control) => rational_projection(*control, vector, normal, radius),
        };
        let guard = 1024. * f64::EPSILON * radius;
        Ok([bounds[0] - guard, bounds[1] + guard])
    }
}

fn rational_projection(control: [DVec4; 4], vector: DVec3, normal: DVec3, radius: f64) -> [f64; 2] {
    // Degree-3 products become degree-6 Bernstein coefficients. The numerator
    // is the nonunit quaternion rotation quadratic; the denominator is |q|^2.
    let scale = control
        .into_iter()
        .map(|q| q.abs().max_element())
        .fold(0_f64, f64::max);
    let c = control.map(|q| q / scale);
    let diagonal = c
        .into_iter()
        .map(DVec4::length_squared)
        .fold(0_f64, f64::max);
    let denominator_error = 128. * f64::EPSILON * diagonal;
    let numerator_error = 512. * f64::EPSILON * diagonal * radius;
    let choose3 = [1., 3., 3., 1.];
    let choose6 = [1., 6., 15., 20., 15., 6., 1.];
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for (k, divisor) in choose6.into_iter().enumerate() {
        let mut denominator = 0.;
        let mut numerator = 0.;
        for i in 0..4 {
            if k < i || k - i >= 4 {
                continue;
            }
            let j = k - i;
            let weight = choose3[i] * choose3[j] / divisor;
            let a = c[i].truncate();
            let b = c[j].truncate();
            let rotated = vector * (c[i].w * c[j].w - a.dot(b))
                + a * b.dot(vector)
                + b * a.dot(vector)
                + b.cross(vector) * c[i].w
                + a.cross(vector) * c[j].w;
            denominator += weight * c[i].dot(c[j]);
            numerator += weight * normal.dot(rotated);
        }
        if denominator <= denominator_error {
            return [-radius, radius];
        }
        // With positive denominator coefficients the rational curve lies in
        // the coefficient-ratio hull. Enclose rounding before each division.
        for n in [numerator - numerator_error, numerator + numerator_error] {
            for d in [
                denominator - denominator_error,
                denominator + denominator_error,
            ] {
                let ratio = n / d;
                low = low.min(ratio);
                high = high.max(ratio);
            }
        }
    }
    [low.max(-radius), high.min(radius)]
}

#[derive(Clone, Debug)]
pub struct RootRotationPath {
    duration: f64,
    spans: Vec<RootRotationSpan>,
    end_rotation: DQuat,
}
impl RootRotationPath {
    #[must_use]
    pub const fn duration(&self) -> f64 {
        self.duration
    }
    #[must_use]
    pub fn spans(&self) -> &[RootRotationSpan] {
        &self.spans
    }
    /// Composition result alone must not be used for collision admission.
    #[must_use]
    pub const fn end_rotation(&self) -> DQuat {
        self.end_rotation
    }
    /// Conservative angular travel including STEP event arcs, in radians.
    #[must_use]
    pub fn angular_travel_bound(&self) -> f64 {
        self.spans
            .iter()
            .map(|span| match &span.shape {
                Shape::Hold(_) => 0.,
                Shape::Arc { axis, .. } | Shape::Step { axis, .. } => axis.length(),
                Shape::Cubic(_) => {
                    span.speed_bound.unwrap_or(f64::INFINITY) * (span.end - span.start)
                }
            })
            .sum()
    }
}

impl RootRotationCurve {
    pub(super) fn new(
        keys: &[QuatKey],
        mode: Interpolation,
        tangents: &[[Vec4; 2]],
        fallback: Quat,
        duration: f32,
        playback: Playback,
    ) -> Result<Self, AnimationError> {
        if keys.len() > MAX_ROOT_ROTATION_KEYS {
            return Err(AnimationError::RootRotationBudget);
        }
        let fallback_source = fallback.to_array().map(f64::from);
        let fallback = double(fallback);
        let mut key_normalization_error = crate::root_rigid::quaternion_normalization_error_bounds(
            fallback_source,fallback.to_array())?;
        let mut cubic_control_error = (mode==Interpolation::CubicSpline).then_some([0_f64;4]);
        let mut knots = Vec::with_capacity(keys.len());
        for (i, key) in keys.iter().enumerate() {
            let value = double(key.value);
            let error = crate::root_rigid::quaternion_normalization_error_bounds(
                key.value.to_array().map(f64::from),value.to_array())?;
            for component in 0..4 {
                key_normalization_error[component] = key_normalization_error[component].max(error[component]);
            }
            let shape = if let Some(next) = keys.get(i + 1) {
                match mode {
                    Interpolation::Step => Shape::Hold(value),
                    Interpolation::Linear => Shape::Arc {
                        from: value,
                        axis: log(value.conjugate() * double(next.value)),
                    },
                    Interpolation::CubicSpline => {
                        let dt = f64::from(next.time) - f64::from(key.time);
                        let first = Vec4::from_array(key.value.to_array()).as_dvec4();
                        let last = Vec4::from_array(next.value.to_array()).as_dvec4();
                        Shape::Cubic([
                            first,
                            first + tangents[i][1].as_dvec4() * (dt / 3.),
                            last - tangents[i + 1][0].as_dvec4() * (dt / 3.),
                            last,
                        ])
                    }
                }
            } else {
                Shape::Hold(value)
            };
            if let Shape::Cubic(control) = &shape {
                let next = &keys[i+1];
                let error = crate::root_rigid::quaternion_cubic_control_error_bounds(
                    [key.value.to_array().map(f64::from),next.value.to_array().map(f64::from)],
                    [tangents[i][1].to_array().map(f64::from),tangents[i+1][0].to_array().map(f64::from)],
                    [f64::from(key.time),f64::from(next.time)],control.map(|value|value.to_array()))?;
                let bound = cubic_control_error.as_mut().ok_or(AnimationError::InvalidRootRotationCurve)?;
                for axis in 0..4 {bound[axis]=bound[axis].max(error[axis]);}
            }
            knots.push(Knot {
                time: f64::from(key.time),
                value,
                shape,
            });
        }
        let origin = knots.first().map_or(fallback, |k| k.value);
        let end = knots.last().map_or(fallback, |k| k.value);
        let constant = knots.iter().all(|k| match &k.shape {
            Shape::Hold(q) => log(origin.conjugate() * *q).length() == 0.,
            Shape::Arc { axis, .. } => axis.length() == 0.,
            Shape::Cubic(c) => {
                c.iter().all(|v| *v == c[0]) && log(origin.conjugate() * k.value).length() == 0.
            }
            Shape::Step { .. } => false,
        });
        Ok(Self(Arc::new(Curve {
            knots,
            fallback,
            duration: f64::from(duration),
            playback,
            mode,
            origin,
            cycle: (end * origin.conjugate()).normalize(),
            constant,
            key_normalization_error,
            cubic_control_error,
        })))
    }

    /// Enclosed errors of normalization from stored keys and bind fallback.
    pub fn key_normalization_error_bounds(&self) -> [f64;4] {
        self.0.key_normalization_error
    }

    /// Uniform raw quaternion polynomial error from stored-key cubic controls.
    /// Normalization amplifies this according to a separate positive norm proof.
    pub fn cubic_control_compilation_error_bounds(&self) -> Option<[f64;4]> {
        self.0.cubic_control_error
    }

    /// Normalized source/cached polynomial discrepancy over complete key spans.
    /// Fails when the simple convex-hull norm lower bound is insufficient.
    pub fn cubic_normalized_compilation_error_bounds(&self)
        -> Result<Option<[f64;4]>,AnimationError> {
        let Some(raw) = self.0.cubic_control_error else {return Ok(None);};
        let mut result = self.0.key_normalization_error;
        for knot in &self.0.knots {
            if let Shape::Cubic(control) = &knot.shape {
                let error = crate::root_rigid::quaternion_cubic_normalized_error_bounds(
                    control.map(|q|q.to_array()),raw)?;
                for axis in 0..4 {result[axis]=result[axis].max(error[axis]);}
            }
        }
        Ok(Some(result))
    }

    pub(super) fn phase_key_times(&self)->impl Iterator<Item=f64>+'_ {
        self.0.knots.iter().map(|k|k.time)
    }
    /// Exact-source angular speed on a single cubic key interval. Uses a whole-key
    /// Bernstein bound; returns None for other interpolation modes.
    pub fn cubic_source_angular_speed_bound(&self,start:f64,end:f64)
        ->Result<Option<f64>,AnimationError> {
        if !start.is_finite() || !end.is_finite() || start<0. || end<start || end>self.0.duration {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(error)=self.0.cubic_control_error else {return Ok(None);};
        let upper=self.0.knots.partition_point(|k|k.time<=start);
        if upper==0 {
            if self.0.knots.first().is_some_and(|k|end>k.time) {return Err(AnimationError::InvalidSampleTime);}
            return Ok(Some(0.));
        }
        let knot=&self.0.knots[upper-1];
        let Some(next)=self.0.knots.get(upper) else {return Ok(Some(0.));};
        if end>next.time {return Err(AnimationError::InvalidSampleTime);}
        let Shape::Cubic(control)=&knot.shape else {return Ok(Some(0.));};
        Ok(Some(crate::root_rigid::quaternion_cubic_source_speed_bound(
            control.map(|q|q.to_array()),error,[knot.time,next.time])?))
    }

    /// Normalized discrepancy through stored-key cubic restriction, before
    /// left/right frame composition and rounded runtime pose evaluation.
    pub fn cubic_piece_compilation_error_bounds(&self,start:f64,end:f64)
        -> Result<Option<[f64;4]>,AnimationError> {
        if !start.is_finite() || !end.is_finite() || start<0. || end<=start || end>self.0.duration {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(raw) = self.0.cubic_control_error else {return Ok(None);};
        let upper = self.0.knots.partition_point(|key|key.time<=start);
        if upper==0 {
            if self.0.knots.first().is_some_and(|key|end>key.time) {
                return Err(AnimationError::InvalidSampleTime);
            }
            return Ok(Some(self.0.key_normalization_error));
        }
        let knot = &self.0.knots[upper-1];
        let Some(next) = self.0.knots.get(upper) else {return Ok(Some(self.0.key_normalization_error));};
        if end>next.time {return Err(AnimationError::InvalidSampleTime);}
        let Shape::Cubic(control) = &knot.shape else {return Ok(Some(self.0.key_normalization_error));};
        let duration = next.time-knot.time;
        let Shape::Cubic(stored) = restrict(&knot.shape,(start-knot.time)/duration,(end-knot.time)/duration)?
            else {return Err(AnimationError::InvalidRootRotationCurve);};
        let error = crate::root_rigid::quaternion_cubic_restriction_error_bounds(
            control.map(|value|value.to_array()),raw,[knot.time,next.time],[start,end],
            stored.map(|value|value.to_array()))?;
        Ok(Some(error))
    }

    /// Discrepancy of the actual local cubic sample from exact source keys.
    /// The supplied phase is exact; clock/loop mapping is a separate obligation.
    pub fn cubic_phase_evaluation_error_bounds(&self,phase:f64)
        -> Result<Option<[f64;4]>,AnimationError> {
        if !phase.is_finite() || phase<0. || phase>self.0.duration {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(raw) = self.0.cubic_control_error else {return Ok(None);};
        let upper = self.0.knots.partition_point(|key|key.time<=phase);
        if upper==0 {return Ok(Some(self.0.key_normalization_error));}
        let from = &self.0.knots[upper-1];
        let Some(to) = self.0.knots.get(upper) else {return Ok(Some(self.0.key_normalization_error));};
        let Shape::Cubic(control) = &from.shape else {return Ok(Some(self.0.key_normalization_error));};
        let evaluated = self.local(phase)?;
        crate::root_rigid::quaternion_cubic_phase_evaluation_error_bounds(
            control.map(|value|value.to_array()),raw,[from.time,to.time],phase,
            evaluated.to_array()).map(Some)
    }

    /// Uniform actual cubic evaluation discrepancy over one closed key interval.
    /// Exact supplied phases; no relative-frame or cycle composition included.
    pub fn cubic_interval_evaluation_error_bounds(&self,start:f64,end:f64)
        -> Result<Option<[f64;4]>,AnimationError> {
        if !start.is_finite() || !end.is_finite() || start<0. || end<start || end>self.0.duration {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(raw)=self.0.cubic_control_error else {return Ok(None);};
        let upper=self.0.knots.partition_point(|key|key.time<=start);
        if upper==0 {
            if self.0.knots.first().is_some_and(|key|end>key.time) {
                return Err(AnimationError::InvalidSampleTime);
            }
            return Ok(Some(self.0.key_normalization_error));
        }
        let from=&self.0.knots[upper-1];
        let Some(to)=self.0.knots.get(upper) else {return Ok(Some(self.0.key_normalization_error));};
        if end>to.time {return Err(AnimationError::InvalidSampleTime);}
        let Shape::Cubic(control)=&from.shape else {return Ok(Some(self.0.key_normalization_error));};
        let mut error=crate::root_rigid::quaternion_cubic_interval_evaluation_error_bounds(
            control.map(|value|value.to_array()),raw,[from.time,to.time],[start,end])?;
        if end==to.time {
            for axis in 0..4 {error[axis]=error[axis].max(self.0.key_normalization_error[axis]);}
        }
        Ok(Some(error))
    }

    /// Uniform source-to-runtime relative cubic orientation on one key interval.
    pub fn cubic_relative_interval_evaluation_error_bounds(&self,start:f64,end:f64)
        ->Result<Option<[f64;4]>,AnimationError> {
        let Some(error)=self.cubic_interval_evaluation_error_bounds(start,end)? else {return Ok(None);};
        crate::root_rigid::quaternion_normalized_composition_uniform_error(error,
            self.0.origin.conjugate().to_array(),self.0.key_normalization_error).map(Some)
    }

    /// Pointwise source-to-runtime cubic orientation relative to its first key.
    /// Carries both sample and origin normalization through real composition.
    pub fn cubic_relative_phase_evaluation_error_bounds(&self,phase:f64)
        -> Result<Option<[f64;4]>,AnimationError> {
        let Some(error)=self.cubic_phase_evaluation_error_bounds(phase)? else {return Ok(None);};
        let local=self.local(phase)?;
        let origin=self.0.origin.conjugate();
        let evaluated=self.phase_rotation(phase)?;
        crate::root_rigid::quaternion_composition_evaluation_error_bounds(
            local.to_array(),error,origin.to_array(),self.0.key_normalization_error,
            evaluated.to_array()).map(Some)
    }

    /// Uniform loop-sample error for an explicit cycle and local key interval.
    /// Matches sample(): power*local is unnormalized, then origin composition
    /// and final normalization. Wall-clock mapping is a separate obligation.
    pub fn cubic_cycle_interval_evaluation_error_bounds(&self,cycles:u64,start:f64,end:f64)
        ->Result<Option<[f64;4]>,AnimationError> {
        if cycles>9007199254740991 || (self.0.playback==Playback::Clamp && cycles!=0) {
            return Err(AnimationError::RootRotationBudget);
        }
        let Some(local_error)=self.cubic_interval_evaluation_error_bounds(start,end)? else {return Ok(None);};
        let initial=self.0.origin.conjugate();
        let end=self.0.knots.last().map_or(self.0.fallback,|key|key.value);
        let cycle_error=crate::root_rigid::quaternion_composition_evaluation_error_bounds(
            end.to_array(),self.0.key_normalization_error,initial.to_array(),
            self.0.key_normalization_error,self.0.cycle.to_array())?;
        let (_,prefix_error)=power_with_error(self.0.cycle,cycle_error,cycles)?;
        let intermediate=crate::root_rigid::quaternion_composition_uniform_error(prefix_error,local_error)?;
        crate::root_rigid::quaternion_normalized_composition_uniform_error(intermediate,
            initial.to_array(),self.0.key_normalization_error).map(Some)
    }

    /// Pointwise cubic sample at an explicit cycle count and exact local phase.
    /// Cycle/phase selection from wall time remains a separate proof obligation.
    pub fn cubic_cycle_phase_evaluation_error_bounds(&self,cycles:u64,phase:f64)
        -> Result<Option<[f64;4]>,AnimationError> {
        if self.0.playback==Playback::Clamp && cycles!=0 {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(local_error)=self.cubic_phase_evaluation_error_bounds(phase)? else {return Ok(None);};
        let initial=self.0.origin.conjugate();
        let end=self.0.knots.last().map_or(self.0.fallback,|key|key.value);
        let cycle_error=crate::root_rigid::quaternion_composition_evaluation_error_bounds(
            end.to_array(),self.0.key_normalization_error,initial.to_array(),
            self.0.key_normalization_error,self.0.cycle.to_array())?;
        let (prefix,prefix_error)=power_with_error(self.0.cycle,cycle_error,cycles)?;
        let local=self.local(phase)?;
        // Match sample(): the intermediate product is not normalized.
        let intermediate=prefix*local;
        let intermediate_error=crate::root_rigid::quaternion_composition_evaluation_error_bounds(
            prefix.to_array(),prefix_error,local.to_array(),local_error,intermediate.to_array())?;
        let evaluated=(intermediate*initial).normalize();
        crate::root_rigid::quaternion_composition_evaluation_error_bounds(
            intermediate.to_array(),intermediate_error,initial.to_array(),
            self.0.key_normalization_error,evaluated.to_array()).map(Some)
    }

    fn local(&self, time: f64) -> Result<DQuat, AnimationError> {
        let data = &self.0;
        let upper = data.knots.partition_point(|k| k.time <= time);
        if upper == 0 {
            return Ok(data.knots.first().map_or(data.fallback, |k| k.value));
        }
        let from = &data.knots[upper - 1];
        let Some(to) = data.knots.get(upper) else {
            return Ok(from.value);
        };
        from.shape
            .sample((time - from.time) / (to.time - from.time))
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn cycle_phase(&self, time: f64) -> Result<(u64, f64), AnimationError> {
        let proof=crate::enclose_root_cycle_phase(time,self.0.duration as f32,self.0.playback)
            .map_err(|error|if error==AnimationError::RootRigidBudget {
                AnimationError::RootRotationBudget
            } else {error})?;
        Ok((proof.cycle(),proof.phase()))
    }

    pub(super) fn phase_rotation(&self, time: f64) -> Result<DQuat, AnimationError> {
        Ok((self.local(time)? * self.0.origin.conjugate()).normalize())
    }
    pub(super) fn is_constant(&self) -> bool {
        self.0.constant
    }

    /// Unwrapped parent-local orientation relative to the first authored value.
    /// Each loop composes its full rotation; the seam contributes no reset snap.
    /// # Errors
    /// Rejects invalid time, a singular cubic sample or an unrepresentable loop index.
    pub fn sample(&self, time: f64) -> Result<DQuat, AnimationError> {
        let (cycle, phase) = self.cycle_phase(time)?;
        Ok(
            (power(self.0.cycle, cycle) * self.local(phase)? * self.0.origin.conjugate())
                .normalize(),
        )
    }

    /// Extracts an ordered path over [start,end], with relative span times.
    /// Held channels remain held; STEP events belong to (start,end]. Cubic spans
    /// are subdivided until a nonzero norm and an angular travel bound are proved
    /// from their Bezier control hull. No sampled-point test certifies a whole span.
    /// # Errors
    /// Bounds output to 1..=4096 spans. Invalid time, a singular quaternion, or
    /// exhausted span/subdivision work rejects the whole request without a prefix.
    // Exact equality assigns STEP boundaries once; epsilon comparisons duplicate events.
    #[allow(clippy::float_cmp, clippy::too_many_lines)]
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

fn append(
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

fn norm_lower_bound(c: [DVec4; 4]) -> f64 {
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
fn restrict(shape: &Shape, start: f64, end: f64) -> Result<Shape, AnimationError> {
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
fn bezier(c: [DVec4; 4], t: f64) -> DVec4 {
    split(c, t).0[3]
}
#[allow(clippy::many_single_char_names)]
fn bezier_derivative(c: [DVec4; 4], t: f64) -> DVec4 {
    let a = (c[1] - c[0]) * 3.;
    let b = (c[2] - c[1]) * 3.;
    let d = (c[3] - c[2]) * 3.;
    a.lerp(b, t).lerp(b.lerp(d, t), t)
}
#[allow(clippy::many_single_char_names)]
fn split(c: [DVec4; 4], t: f64) -> ([DVec4; 4], [DVec4; 4]) {
    let a = c[0].lerp(c[1], t);
    let b = c[1].lerp(c[2], t);
    let d = c[2].lerp(c[3], t);
    let e = a.lerp(b, t);
    let f = b.lerp(d, t);
    let g = e.lerp(f, t);
    ([c[0], a, e, g], [g, f, d, c[3]])
}
fn unit(value: DVec4) -> Result<DQuat, AnimationError> {
    let scale = value.abs().max_element();
    if !value.is_finite() || scale == 0. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    Ok(DQuat::from_array((value / scale).normalize().to_array()))
}
fn double(q: Quat) -> DQuat {
    DQuat::from_array(q.to_array().map(f64::from)).normalize()
}
fn log(mut q: DQuat) -> DVec3 {
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
fn power_with_error(mut q:DQuat,mut error:[f64;4],mut n:u64)
    -> Result<(DQuat,[f64;4]),AnimationError> {
    let mut result=DQuat::IDENTITY;
    let mut result_error=[0.;4];
    while n!=0 {
        if n&1!=0 {
            let next=(result*q).normalize();
            result_error=crate::root_rigid::quaternion_composition_evaluation_error_bounds(
                result.to_array(),result_error,q.to_array(),error,next.to_array())?;
            result=next;
        }
        n>>=1;
        let next=(q*q).normalize();
        if n!=0 {
            error=crate::root_rigid::quaternion_composition_evaluation_error_bounds(
                q.to_array(),error,q.to_array(),error,next.to_array())?;
        }
        q=next;
    }
    Ok((result,result_error))
}

fn power(mut q: DQuat, mut n: u64) -> DQuat {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnimationClip, Joint, JointTangents, JointTrack, Skeleton, TrackInterpolation, Transform,
    };
    use glam::Mat4;
    fn rig() -> Skeleton {
        Skeleton::new(vec![Joint {
            name: Arc::from("locomotion"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap()
    }
    fn clip(
        keys: Vec<QuatKey>,
        mode: Interpolation,
        tangents: Vec<[Vec4; 2]>,
        playback: Playback,
    ) -> AnimationClip {
        AnimationClip::new_with_tangents(
            "turn",
            1.,
            playback,
            vec![JointTrack {
                rotations: keys,
                ..Default::default()
            }],
            vec![TrackInterpolation {
                rotation: mode,
                ..Default::default()
            }],
            vec![JointTangents {
                rotation: tangents,
                ..Default::default()
            }],
            &rig(),
        )
        .unwrap()
    }
    fn keys(values: &[Quat]) -> Vec<QuatKey> {
        values
            .iter()
            .enumerate()
            .map(|(i, q)| QuatKey {
                time: i as f32 / (values.len() - 1) as f32,
                value: *q,
            })
            .collect()
    }
    fn same(a: DQuat, b: DQuat, tolerance: f64) {
        assert!(
            log(a.conjugate() * b).length() <= tolerance,
            "{a:?} != {b:?}"
        );
    }
    fn follow(path: &RootRotationPath, mut initial: DQuat) -> DQuat {
        for span in path.spans() {
            same(initial, span.sample(0.).unwrap(), 1e-10);
            if let Some(axis) = span.body_angular_displacement() {
                same(
                    initial * DQuat::from_scaled_axis(axis),
                    span.sample(1.).unwrap(),
                    1e-10,
                );
            }
            initial = span.sample(1.).unwrap();
        }
        same(initial, path.end_rotation(), 1e-10);
        initial
    }

    #[test]
    fn full_turn_is_not_lost_at_equal_endpoints_and_substeps_compose() {
        let values: Vec<_> = (0..=4)
            .map(|i| Quat::from_rotation_y(i as f32 * std::f32::consts::FRAC_PI_2))
            .collect();
        let curve = clip(keys(&values), Interpolation::Linear, vec![], Playback::Loop)
            .root_rotation_curve(0)
            .unwrap();
        let path = curve.path(0., 3., 32).unwrap();
        assert_eq!(path.spans().len(), 12);
        assert!((path.angular_travel_bound() - 3. * std::f64::consts::TAU).abs() < 1e-6);
        same(follow(&path, DQuat::IDENTITY), DQuat::IDENTITY, 1e-6);
        let mut result = DQuat::IDENTITY;
        for i in 0..24 {
            let part = curve
                .path(f64::from(i) / 8., f64::from(i + 1) / 8., 4)
                .unwrap();
            follow(&part, DQuat::IDENTITY);
            result = (result * part.end_rotation()).normalize();
        }
        same(result, path.end_rotation(), 1e-10);
    }

    #[test]
    fn noncommuting_keys_and_cycle_transforms_preserve_order_and_large_clocks() {
        let values = [
            Quat::IDENTITY,
            Quat::from_rotation_x(0.8),
            Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.8),
            Quat::from_rotation_z(-0.4) * Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.8),
        ];
        let curve = clip(keys(&values), Interpolation::Linear, vec![], Playback::Loop)
            .root_rotation_curve(0)
            .unwrap();
        let cycle = double(values[3]);
        same(
            curve.sample(2.75).unwrap(),
            cycle * cycle * curve.local(0.75).unwrap(),
            1e-12,
        );
        let path = curve.path(0.2, 2.8, 32).unwrap();
        follow(&path, DQuat::IDENTITY);
        let split = curve.path(0.2, 1.3, 32).unwrap().end_rotation()
            * curve.path(1.3, 2.8, 32).unwrap().end_rotation();
        same(split, path.end_rotation(), 1e-10);
        let start = 1_048_576.25;
        let tiny = curve.path(start, start + 1. / 60., 4).unwrap();
        assert!(log(tiny.end_rotation()).length() > 0.01);
        follow(&tiny, DQuat::IDENTITY);
    }

    #[test]
    fn step_events_are_right_continuous_and_not_repeated_at_start_or_seam() {
        let curve = clip(
            keys(&[
                Quat::IDENTITY,
                Quat::from_rotation_x(0.7),
                Quat::from_rotation_y(-0.9),
            ]),
            Interpolation::Step,
            vec![],
            Playback::Loop,
        )
        .root_rotation_curve(0)
        .unwrap();
        let full = curve.path(0., 2., 16).unwrap();
        assert_eq!(full.spans().iter().filter(|s| s.is_step()).count(), 4);
        follow(&full, DQuat::IDENTITY);
        let first = curve.path(0., 0.5, 8).unwrap();
        let second = curve.path(0.5, 1., 8).unwrap();
        assert_eq!(first.spans().iter().filter(|s| s.is_step()).count(), 1);
        assert_eq!(second.spans().iter().filter(|s| s.is_step()).count(), 1);
        same(
            first.end_rotation() * second.end_rotation(),
            curve.path(0., 1., 8).unwrap().end_rotation(),
            1e-12,
        );
        let event = full.spans().iter().find(|s| s.is_step()).unwrap();
        assert_eq!(event.start(), event.end());
        assert_eq!(event.angular_speed_bound(), None);
        assert_eq!(event.angular_velocity(0.5).unwrap(), None);
    }

    #[test]
    fn cubic_equal_endpoints_keep_excursion_and_bound_speed_between_samples() {
        // Both ends are identity; tangents make a multi-axis interior excursion.
        let out = Vec4::new(3., 4., -2., 0.7);
        let incoming = Vec4::new(-4., 1., 3., -0.2);
        let animation = clip(
            keys(&[Quat::IDENTITY, Quat::IDENTITY]),
            Interpolation::CubicSpline,
            vec![[Vec4::ZERO, out], [incoming, Vec4::ZERO]],
            Playback::Clamp,
        );
        let curve = animation.root_rotation_curve(0).unwrap();
        let path = curve.path(0., 1., 256).unwrap();
        assert!(path.spans().len() > 1);
        same(path.end_rotation(), DQuat::IDENTITY, 1e-12);
        assert!(
            path.spans()
                .iter()
                .all(|s| s.body_angular_displacement().is_none())
        );
        let mut maximum_excursion = 0_f64;
        for span in path.spans() {
            let bound = span.angular_speed_bound().unwrap();
            assert!(bound * (span.end() - span.start()) <= std::f64::consts::FRAC_PI_2);
            for i in 0..=200 {
                let u = f64::from(i) / 200.;
                let t = span.start() + u * (span.end() - span.start());
                let independent = crate::sample_quat(
                    &animation.tracks[0].rotations,
                    t as f32,
                    Quat::IDENTITY,
                    Interpolation::CubicSpline,
                    &animation.tangents[0].rotation,
                );
                let actual = span.sample(u).unwrap();
                same(actual, double(independent), 2e-6);
                maximum_excursion = maximum_excursion.max(log(actual).length());
                let velocity = span.angular_velocity(u).unwrap().unwrap();
                assert!(velocity.length() <= bound * (1. + 1e-12));
                if i > 0 && i < 200 {
                    let h = 1e-5;
                    let numerical =
                        log(span.sample(u + h).unwrap() * span.sample(u - h).unwrap().conjugate())
                            / (2. * h * (span.end() - span.start()));
                    assert!((numerical - velocity).length() < 1e-7);
                }
            }
        }
        assert!(maximum_excursion > 1.);
        follow(&path, DQuat::IDENTITY);
    }

    #[test]
    fn near_singular_cubic_is_subdivided_but_a_hidden_zero_rejects_the_path() {
        let zero = clip(
            keys(&[Quat::IDENTITY, -Quat::IDENTITY]),
            Interpolation::CubicSpline,
            vec![[Vec4::ZERO; 2]; 2],
            Playback::Clamp,
        )
        .root_rotation_curve(0)
        .unwrap();
        // Endpoint-only pose validation would miss the zero at the midpoint.
        assert!(zero.sample(0.).is_ok() && zero.sample(1.).is_ok());
        assert_eq!(
            zero.path(0., 1., 256).unwrap_err(),
            AnimationError::InvalidRootRotationCurve
        );
        let offset = Vec4::X * 0.01;
        let near = clip(
            keys(&[Quat::IDENTITY, -Quat::IDENTITY]),
            Interpolation::CubicSpline,
            vec![[Vec4::ZERO, offset], [-offset, Vec4::ZERO]],
            Playback::Clamp,
        )
        .root_rotation_curve(0)
        .unwrap();
        let path = near.path(0., 1., 256).unwrap();
        assert!(path.spans().len() > 10);
        assert!(path.angular_travel_bound() >= std::f64::consts::TAU);
        for span in path.spans() {
            for i in 0..=100 {
                assert!(
                    span.angular_velocity(f64::from(i) / 100.)
                        .unwrap()
                        .unwrap()
                        .length()
                        <= span.angular_speed_bound().unwrap() * (1. + 1e-12)
                );
            }
        }
        follow(&path, DQuat::IDENTITY);
    }

    #[test]
    fn clamp_holds_key_ranges_and_paused_interval_has_no_events() {
        let animation = clip(
            vec![
                QuatKey {
                    time: 0.2,
                    value: Quat::from_rotation_y(0.1),
                },
                QuatKey {
                    time: 0.8,
                    value: Quat::from_rotation_y(0.5),
                },
            ],
            Interpolation::Linear,
            vec![],
            Playback::Clamp,
        );
        let curve = animation.root_rotation_curve(0).unwrap();
        let path = curve.path(0., 3., 8).unwrap();
        assert_eq!(path.duration(), 3.);
        assert_eq!(path.spans().last().unwrap().end(), 3.);
        follow(&path, DQuat::IDENTITY);
        same(path.end_rotation(), DQuat::from_rotation_y(0.4), 1e-7);
        let held = curve.path(2., 3., 1).unwrap();
        assert_eq!(held.angular_travel_bound(), 0.);
        follow(&held, DQuat::IDENTITY);
        assert!(curve.path(0.4, 0.4, 1).unwrap().spans().is_empty());
    }

    #[test]
    fn budgets_reject_whole_request_without_flattening_or_repeating_work() {
        let animation = clip(
            keys(&[Quat::IDENTITY, Quat::from_rotation_x(1.)]),
            Interpolation::Linear,
            vec![],
            Playback::Loop,
        );
        let curve = animation.root_rotation_curve(0).unwrap();
        assert_eq!(
            curve.path(0., 1_000_000., 2).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        assert_eq!(
            curve.path(0., 1., 0).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        assert_eq!(
            curve.path(0., 1., MAX_ROOT_ROTATION_SPANS + 1).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        for invalid in [f64::NAN, f64::INFINITY, -1.] {
            assert_eq!(
                curve.sample(invalid).unwrap_err(),
                AnimationError::InvalidSampleTime
            );
        }
        assert_eq!(
            curve.sample(1e18).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        assert_eq!(
            animation.root_rotation_curve(1).unwrap_err(),
            AnimationError::InvalidRootMotionJoint(1)
        );
        let path = curve.path(0., 1., 1).unwrap();
        assert!(path.spans()[0].sample(1.01).is_err());
        let axis = path.spans()[0].body_angular_displacement().unwrap();
        assert!((axis - DVec3::X).length() < 1e-7);
        let empty = AnimationClip::new(
            "hold",
            1.,
            Playback::Loop,
            vec![JointTrack::default()],
            &rig(),
        )
        .unwrap();
        assert_eq!(
            empty
                .root_rotation_curve(0)
                .unwrap()
                .path(0., 1e6, 1)
                .unwrap()
                .spans()
                .len(),
            1
        );
        assert!(Arc::ptr_eq(&curve.0, &curve.clone().0));
    }

    #[test]
    fn projection_hulls_enclose_curved_motion_and_certify_grounded_yaw() {
        let first = Quat::from_rotation_z(0.3);
        let animation = clip(
            keys(&[
                first,
                Quat::from_rotation_x(0.8),
                Quat::from_rotation_y(-0.4),
            ]),
            Interpolation::CubicSpline,
            vec![
                [Vec4::ZERO, Vec4::new(3., -1., 2., 0.5)],
                [Vec4::new(1., 3., -2., 0.4), Vec4::new(-2., 1., 4., -0.3)],
                [Vec4::new(2., -3., 1., 0.1), Vec4::ZERO],
            ],
            Playback::Loop,
        );
        let path = animation
            .root_rotation_curve(0)
            .unwrap()
            .path(0.17, 2.8, 512)
            .unwrap();
        for span in path.spans() {
            for (vector, normal) in [
                (DVec3::new(0.4, 0.1, 0.02), DVec3::Y),
                (
                    DVec3::new(-0.2, 0.8, -0.3),
                    DVec3::new(0.4, -0.8, 0.2).normalize(),
                ),
                (DVec3::new(1e-9, 4e-9, -2e-9), DVec3::X),
            ] {
                let [low, high] = span.projection_bounds(vector, normal).unwrap();
                let speed = span.point_speed_bound(vector).unwrap().unwrap();
                for i in 0..=500 {
                    let value = normal.dot(span.sample(f64::from(i) / 500.).unwrap() * vector);
                    assert!(value >= low && value <= high, "{low} <= {value} <= {high}");
                    if i > 0 && i < 500 {
                        let u = f64::from(i) / 500.;
                        let h = 1e-5;
                        let velocity = (span.sample(u + h).unwrap() * vector
                            - span.sample(u - h).unwrap() * vector)
                            / (2. * h * (span.end() - span.start()));
                        assert!(velocity.length() <= speed * (1. + 1e-8) + 1e-10 * vector.length());
                    }
                }
            }
        }
        let yaw = clip(
            keys(&[Quat::IDENTITY, Quat::from_rotation_y(0.7)]),
            Interpolation::CubicSpline,
            vec![[Vec4::ZERO, Vec4::Y * 0.5], [Vec4::Y * 0.3, Vec4::ZERO]],
            Playback::Clamp,
        );
        for span in yaw
            .root_rotation_curve(0)
            .unwrap()
            .path(0., 1., 256)
            .unwrap()
            .spans()
        {
            let [low, high] = span
                .projection_bounds(DVec3::new(0.4, 0.1, 0.02), DVec3::Y)
                .unwrap();
            assert!(low <= 0.1 && high >= 0.1);
            assert!(
                high - low < 1e-11,
                "floor certificate too loose: {low}..{high}"
            );
            assert!(
                span.point_speed_bound(DVec3::new(0.4, 1000., 0.02))
                    .unwrap()
                    .unwrap()
                    < 10.
            );
        }
        for mode in [Interpolation::Linear, Interpolation::Step] {
            let path = clip(
                keys(&[Quat::IDENTITY, Quat::from_rotation_z(1.2)]),
                mode,
                vec![],
                Playback::Clamp,
            )
            .root_rotation_curve(0)
            .unwrap()
            .path(0., 1., 8)
            .unwrap();
            for span in path.spans() {
                let [low, high] = span.projection_bounds(DVec3::X, DVec3::Y).unwrap();
                for i in 0..=1000 {
                    let value = DVec3::Y.dot(span.sample(f64::from(i) / 1000.).unwrap() * DVec3::X);
                    assert!(value >= low && value <= high);
                }
            }
        }
    }

    #[test]
    fn selected_channel_key_budget_is_enforced_at_compilation() {
        let channel: Vec<_> = (0..=MAX_ROOT_ROTATION_KEYS)
            .map(|i| QuatKey {
                time: i as f32 / MAX_ROOT_ROTATION_KEYS as f32,
                value: Quat::IDENTITY,
            })
            .collect();
        let oversized = clip(
            channel.clone(),
            Interpolation::Linear,
            vec![],
            Playback::Loop,
        );
        assert_eq!(
            oversized.root_rotation_curve(0).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        let admitted = clip(
            channel[..MAX_ROOT_ROTATION_KEYS].to_vec(),
            Interpolation::Linear,
            vec![],
            Playback::Loop,
        );
        let curve = admitted.root_rotation_curve(0).unwrap();
        assert!(Arc::ptr_eq(
            &curve.0,
            &admitted.root_rotation_curve(0).unwrap().0
        ));
        assert!(Arc::ptr_eq(
            &curve.0,
            &admitted.clone().root_rotation_curve(0).unwrap().0
        ));
        assert_eq!(curve.path(0., 1e6, 1).unwrap().angular_travel_bound(), 0.);
    }

    #[test]
    fn concurrent_owners_compile_one_shared_curve_without_sharing_path_state() {
        let animation = Arc::new(clip(
            keys(&[Quat::IDENTITY, Quat::from_rotation_x(1.)]),
            Interpolation::Linear,
            vec![],
            Playback::Loop,
        ));
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let clip = animation.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    clip.root_rotation_curve(0).unwrap()
                })
            })
            .collect();
        let curves: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert!(
            curves
                .iter()
                .all(|curve| Arc::ptr_eq(&curves[0].0, &curve.0))
        );
        let first = curves[0].path(0., 0.25, 2).unwrap();
        let later = curves[1].path(0., 0.75, 2).unwrap();
        same(first.end_rotation(), DQuat::from_rotation_x(0.25), 1e-7);
        same(later.end_rotation(), DQuat::from_rotation_x(0.75), 1e-7);
        same(first.end_rotation(), DQuat::from_rotation_x(0.25), 1e-7);
    }

    #[test]
    fn selecting_many_channels_cannot_multiply_the_compiled_key_budget() {
        let skeleton = Skeleton::new(vec![
            Joint {
                name: Arc::from("root"),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: Arc::from("child"),
                parent: Some(0),
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
        ])
        .unwrap();
        let channel: Vec<_> = (0..40_000)
            .map(|i| QuatKey {
                time: i as f32 / 40_000.,
                value: Quat::IDENTITY,
            })
            .collect();
        let animation = AnimationClip::new(
            "budget",
            1.,
            Playback::Loop,
            vec![
                JointTrack {
                    rotations: channel.clone(),
                    ..Default::default()
                },
                JointTrack {
                    rotations: channel,
                    ..Default::default()
                },
            ],
            &skeleton,
        )
        .unwrap();
        let accepted = animation.root_rotation_curve(0).unwrap();
        assert_eq!(
            animation.root_rotation_curve(1).unwrap_err(),
            AnimationError::RootRotationBudget
        );
        assert!(Arc::ptr_eq(
            &accepted.0,
            &animation.clone().root_rotation_curve(0).unwrap().0
        ));
        assert_eq!(accepted.path(0., 1., 1).unwrap().angular_travel_bound(), 0.);
    }
}

#[cfg(test)]
mod key_normalization_proof_tests {
    use super::*;
    #[test]
    fn normalization_proof_preserves_exact_zero_key_components() {
        let keys = [QuatKey {time:0.,value:Quat::from_xyzw(0.,0.7,0.,0.7)},
            QuatKey {time:1.,value:Quat::from_xyzw(0.,1.,0.,0.)}];
        for mode in [Interpolation::Step,Interpolation::Linear,Interpolation::CubicSpline] {
            let curve = RootRotationCurve::new(&keys,mode,&[[Vec4::ZERO;2];2],
                Quat::IDENTITY,1.,Playback::Clamp).unwrap();
            let errors = curve.key_normalization_error_bounds();
            assert_eq!(errors[0],0.);
            assert_eq!(errors[2],0.);
            let unit = 0.5_f64.sqrt();
            let rounded = curve.0.knots[0].value;
            assert!((rounded.y-unit).abs()<=errors[1]);
            assert!((rounded.w-unit).abs()<=errors[3]);
            assert!(errors.iter().all(|error|error.is_finite() && *error<1e-12));
        }
        assert!(crate::root_rigid::quaternion_normalization_error_bounds([0.;4],[0.;4]).is_err());
        assert!(crate::root_rigid::quaternion_normalization_error_bounds([f64::NAN;4],[0.;4]).is_err());
    }
}

#[cfg(test)]
mod cubic_compilation_proof_tests {
    use super::*;
    #[test]
    fn cubic_controls_cover_duration_tangent_rounding_and_preserve_zero_axes() {
        let keys = [QuatKey {time:0.,value:Quat::IDENTITY},
            QuatKey {time:0.3,value:Quat::from_xyzw(0.,0.5,0.,0.5)}];
        let tangents = [[Vec4::ZERO,Vec4::new(0.,0.1,0.,0.)],
            [Vec4::new(0.,-0.2,0.,0.),Vec4::ZERO]];
        let curve = RootRotationCurve::new(&keys,Interpolation::CubicSpline,&tangents,
            Quat::IDENTITY,1.,Playback::Clamp).unwrap();
        let error = curve.cubic_control_compilation_error_bounds().unwrap();
        assert_eq!(error[0],0.);
        assert_eq!(error[2],0.);
        assert_eq!(error[3],0.);
        assert!(error[1]>0. && error[1]<1e-12);
        let normalized = curve.cubic_normalized_compilation_error_bounds().unwrap().unwrap();
        let uniform=curve.cubic_interval_evaluation_error_bounds(0.,f64::from(keys[1].time)).unwrap().unwrap();
        assert_eq!(uniform[0],0.);assert_eq!(uniform[2],0.);
        assert!(uniform.iter().all(|value|value.is_finite() && *value<1e-12));
        assert!(curve.cubic_interval_evaluation_error_bounds(0.,0.5).is_err());

        let restricted = curve.cubic_piece_compilation_error_bounds(0.125,0.25).unwrap().unwrap();
        assert_eq!(restricted[0],0.);
        assert_eq!(restricted[2],0.);
        assert!(restricted.iter().all(|value|value.is_finite() && *value<1e-12));
        assert!(curve.cubic_piece_compilation_error_bounds(0.125,0.5).is_err());
        for step in 0..=32 {
            let phase = f64::from(keys[1].time)*f64::from(step)/32.;
            let evaluation = curve.cubic_phase_evaluation_error_bounds(phase).unwrap().unwrap();
            println!("CUBIC_UNIFORM_SAMPLE {:?}",(phase,curve.local(phase).unwrap().to_array(),uniform));

            assert_eq!(evaluation[0],0.);
            assert_eq!(evaluation[2],0.);
            assert!(evaluation.iter().all(|error|error.is_finite() && *error<1e-12));
        }
        assert!(curve.cubic_phase_evaluation_error_bounds(f64::NAN).is_err());
        assert!(curve.cubic_phase_evaluation_error_bounds(-0.1).is_err());


        assert_eq!(normalized[0],0.);
        assert_eq!(normalized[2],0.);
        assert!(normalized.iter().all(|error|error.is_finite() && *error<1e-12));
        let singular = [[0.,0.,0.,1.],[0.,0.,0.,1.],
            [0.,0.,0.,-1.],[0.,0.,0.,-1.]];
        assert!(crate::root_rigid::quaternion_cubic_normalized_error_bounds(singular,[0.;4]).is_err());
        assert!(crate::root_rigid::quaternion_cubic_normalized_error_bounds([[0.,0.,0.,1.];4],[1.;4]).is_err());

        let linear = RootRotationCurve::new(&keys,Interpolation::Linear,&[],
            Quat::IDENTITY,1.,Playback::Clamp).unwrap();
        assert!(linear.cubic_control_compilation_error_bounds().is_none());
        assert!(crate::root_rigid::quaternion_cubic_control_error_bounds(
            [[0.;4];2],[[0.;4];2],[1.,0.],[[0.;4];4]).is_err());
    }
}

#[cfg(test)]
mod relative_phase_proof_tests {
    use super::*;
    #[test]
    fn cubic_relative_phase_compares_nonidentity_origin_without_losing_axis_zeros() {
        let keys=[QuatKey {time:0.,value:Quat::from_xyzw(0.,0.5,0.,0.5)},
            QuatKey {time:1.,value:Quat::from_xyzw(0.,1.,0.,0.)}];
        let curve=RootRotationCurve::new(&keys,Interpolation::CubicSpline,
            &[[Vec4::ZERO;2];2],Quat::IDENTITY,1.,Playback::Loop).unwrap();
        let error=curve.cubic_relative_phase_evaluation_error_bounds(0.5).unwrap().unwrap();
        let actual=curve.phase_rotation(0.5).unwrap();
        // Raw midpoint is (0,3/4,0,1/4); removing initial (0,1,0,1)/sqrt(2)
        // gives the independent relative reference (0,1,0,2)/sqrt(5).
        let expected=[0.,1./5_f64.sqrt(),0.,2./5_f64.sqrt()];
        for axis in 0..4 {assert!((actual.to_array()[axis]-expected[axis]).abs()<=error[axis]);}
        assert_eq!(error[0],0.);assert_eq!(error[2],0.);
        assert!(error.iter().all(|value|value.is_finite() && *value<1e-12));
        assert!(curve.cubic_relative_phase_evaluation_error_bounds(f64::NAN).is_err());
        let cycle_error=curve.cubic_cycle_phase_evaluation_error_bounds(4,0.5).unwrap().unwrap();
        let actual=curve.sample(4.5).unwrap().to_array();
        for axis in 0..4 {assert!((actual[axis]+expected[axis]).abs()<=cycle_error[axis]);}
        assert_eq!(cycle_error[0],0.);assert_eq!(cycle_error[2],0.);
        assert!(cycle_error.iter().all(|value|value.is_finite() && *value<1e-11));
        assert!(curve.cubic_cycle_phase_evaluation_error_bounds(1,f64::NAN).is_err());

    }
}

#[cfg(test)]
mod source_angular_speed_tests {
    use super::*;
    #[test]
    fn normalized_cubic_source_speed_covers_analytic_rotation() {
        let keys=[QuatKey {time:0.,value:Quat::IDENTITY},
            QuatKey {time:1.,value:Quat::from_xyzw(0.,1.,0.,1.)}];
        let curve=RootRotationCurve::new(&keys,Interpolation::CubicSpline,
            &[[Vec4::ZERO;2];2],Quat::IDENTITY,1.,Playback::Clamp).unwrap();
        let bound=curve.cubic_source_angular_speed_bound(0.,1.).unwrap().unwrap();
        for i in 0..=64 {
            let u=f64::from(i)/64.;let y=u*u*(3.-2.*u);
            let exact=12.*u*(1.-u)/(1.+y*y);
            assert!(exact<=bound);
        }
        assert_eq!(curve.cubic_source_angular_speed_bound(1.,1.).unwrap(),Some(0.));
        assert!(curve.cubic_source_angular_speed_bound(-1.,0.5).is_err());
        assert!(crate::root_rigid::quaternion_cubic_source_speed_bound([[0.;4];4],[0.;4],[0.,1.]).is_err());
    }
}

#[cfg(test)]
mod uniform_relative_composition_tests {
    use super::*;
    #[test]
    fn relative_cubic_uniform_error_covers_nonidentity_origin_composition() {
        let curve=RootRotationCurve::new(&[QuatKey {time:0.,value:Quat::from_xyzw(0.,0.5,0.,0.5)},
            QuatKey {time:1.,value:Quat::from_xyzw(0.,1.,0.,0.)}],Interpolation::CubicSpline,
            &[[Vec4::ZERO;2];2],Quat::IDENTITY,1.,Playback::Loop).unwrap();
        let uniform=curve.cubic_relative_interval_evaluation_error_bounds(0.,0.5).unwrap().unwrap();
        assert!(uniform.iter().all(|e|e.is_finite() && *e<1e-10));
        for i in 0..=32 {
            let phase=f64::from(i)/64.;
            let point=curve.cubic_relative_phase_evaluation_error_bounds(phase).unwrap().unwrap();
            for axis in 0..4 {assert!(point[axis]<=uniform[axis]);}
        }
        for cycles in [0,3,7] {
            let bound=curve.cubic_cycle_interval_evaluation_error_bounds(cycles,0.,0.5).unwrap().unwrap();
            assert!(bound.iter().all(|e|e.is_finite() && *e<1e-9));
            for i in 0..=32 {
                let point=curve.cubic_cycle_phase_evaluation_error_bounds(cycles,f64::from(i)/64.).unwrap().unwrap();
                for axis in 0..4 {assert!(point[axis]<=bound[axis]);}
            }
        }
        assert!(curve.cubic_cycle_interval_evaluation_error_bounds(u64::MAX,0.,0.5).is_err());
        assert!(crate::root_rigid::quaternion_normalized_composition_uniform_error([1.;4],[0.,0.,0.,1.],[0.;4]).is_err());
    }
}
