//! Composed root motion: translation and rotation share one authored interval.
use super::{
    AnimationError, MAX_ROOT_ROTATION_SPANS, Playback, RootRotationCurve, RootRotationSpan,
};
use crate::root_curve::RootCurve;
use glam::{DQuat, DVec3, Vec3};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootRigidTransform {
    pub translation: DVec3,
    pub rotation: DQuat,
}
impl RootRigidTransform {
    pub const IDENTITY: Self = Self {
        translation: DVec3::ZERO,
        rotation: DQuat::IDENTITY,
    };
    fn checked(self) -> Result<Self, AnimationError> {
        if !self.translation.is_finite()
            || !self.rotation.is_finite()
            || !self.rotation.is_normalized()
        {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(self)
    }
    /// Right composition: `a.compose(b)` applies b in a's rotated frame.
    /// # Errors
    /// Rejects invalid transforms or overflowing translation.
    pub fn compose(self, other: Self) -> Result<Self, AnimationError> {
        self.checked()?;
        other.checked()?;
        Self {
            translation: self.translation + self.rotation * other.translation,
            rotation: (self.rotation * other.rotation).normalize(),
        }
        .checked()
    }
    /// # Errors
    /// Rejects invalid transforms or overflowing translation.
    pub fn inverse(self) -> Result<Self, AnimationError> {
        self.checked()?;
        let rotation = self.rotation.conjugate();
        Self {
            translation: rotation * -self.translation,
            rotation,
        }
        .checked()
    }
    /// # Errors
    /// Rejects invalid transforms, points or numerical overflow.
    pub fn transform_point(self, point: DVec3) -> Result<DVec3, AnimationError> {
        self.checked()?;
        let result = self.translation + self.rotation * point;
        if !point.is_finite() || !result.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
}
fn power(
    mut value: RootRigidTransform,
    mut cycles: u64,
) -> Result<RootRigidTransform, AnimationError> {
    let mut result = RootRigidTransform::IDENTITY;
    while cycles > 0 {
        if cycles & 1 != 0 {
            result = result.compose(value)?;
        }
        cycles >>= 1;
        if cycles > 0 {
            value = value.compose(value)?;
        }
    }
    Ok(result)
}
#[derive(Debug)]
struct Curve {
    translation: Arc<RootCurve>,
    rotation: RootRotationCurve,
    origin: DVec3,
    bind: DVec3,
    duration: f64,
    playback: Playback,
}
/// Immutable selected-joint coefficients shared by clips and owners. Masks do
/// not create additional compiled key arrays; this object contains no clock.
#[derive(Clone, Debug)]
pub struct RootRigidCurve(Arc<Curve>);
#[derive(Clone, Debug)]
pub struct RootRigidSpan {
    rotation: RootRotationSpan,
    additive: [DVec3; 4],
    pivot: [DVec3; 4],
}
impl RootRigidSpan {
    #[must_use]
    pub fn start(&self) -> f64 {
        self.rotation.start()
    }
    #[must_use]
    pub fn end(&self) -> f64 {
        self.rotation.end()
    }
    #[must_use]
    pub fn is_step(&self) -> bool {
        self.start() == self.end()
    }
    #[must_use]
    pub fn rotation(&self) -> &RootRotationSpan {
        &self.rotation
    }
    /// Samples the actual curve. STEP bridges its simultaneous translation and
    /// shortest rotation event; replacing a continuous span by endpoint poses loses it.
    /// # Errors
    /// Rejects invalid fractions or numerical overflow.
    pub fn sample(&self, fraction: f64) -> Result<RootRigidTransform, AnimationError> {
        let rotation = self.rotation.sample(fraction)?;
        RootRigidTransform {
            translation: bezier(self.additive, fraction) - rotation * bezier(self.pivot, fraction),
            rotation,
        }
        .checked()
    }
    /// Encloses n dot (sample(u) * point) over the whole span. The position
    /// polynomial and rotated pivot retain matching Bernstein weights. Bounds
    /// combine proven constant-point rotation hulls, never sampled extrema.
    /// # Errors
    /// Rejects invalid points/normals or overflowing bounds.
    pub fn projection_bounds(
        &self,
        point: DVec3,
        normal: DVec3,
    ) -> Result<[f64; 2], AnimationError> {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for i in 0..4 {
            let bounds = self
                .rotation
                .projection_bounds(point - self.pivot[i], normal)?;
            let additive = normal.dot(self.additive[i]);
            low = low.min(additive + bounds[0]);
            high = high.max(additive + bounds[1]);
        }
        let guard = 4096.
            * f64::EPSILON
            * (low.abs().max(high.abs())
                + normal.length()
                    * (point.length()
                        + self
                            .additive
                            .iter()
                            .chain(&self.pivot)
                            .map(|v| v.length())
                            .fold(0_f64, f64::max)));
        let result = [low - guard, high + guard];
        if !result.into_iter().all(f64::is_finite) {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
    /// Speed per normalized span fraction, including simultaneous STEP bridges.
    /// # Errors
    /// Rejects invalid points or overflowing derivative bounds.
    pub fn point_speed_bound(&self, point: DVec3) -> Result<f64, AnimationError> {
        let mut angular = 0_f64;
        for pivot in self.pivot {
            let vector = point - pivot;
            let bound = self.rotation.point_speed_bound(vector)?;
            let speed = if let Some(speed) = bound {
                speed * (self.end() - self.start())
            } else {
                self.rotation
                    .body_angular_displacement()
                    .unwrap_or_default()
                    .cross(vector)
                    .length()
            };
            angular = angular.max(speed);
        }
        let derivative = |c: [DVec3; 4]| {
            c.windows(2)
                .map(|pair| 3. * (pair[1] - pair[0]).length())
                .fold(0_f64, f64::max)
        };
        let result = angular + derivative(self.additive) + derivative(self.pivot);
        if !result.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
}
fn bezier(c: [DVec3; 4], t: f64) -> DVec3 {
    let a = c[0].lerp(c[1], t);
    let b = c[1].lerp(c[2], t);
    let d = c[2].lerp(c[3], t);
    a.lerp(b, t).lerp(b.lerp(d, t), t)
}
#[derive(Clone, Debug)]
pub struct RootRigidPath {
    spans: Vec<RootRigidSpan>,
    duration: f64,
    end: RootRigidTransform,
}
impl RootRigidPath {
    #[must_use]
    pub fn spans(&self) -> &[RootRigidSpan] {
        &self.spans
    }
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.duration
    }
    #[must_use]
    pub fn end_transform(&self) -> RootRigidTransform {
        self.end
    }
    #[must_use]
    pub fn angular_travel_bound(&self) -> f64 {
        self.spans
            .iter()
            .map(|span| {
                span.rotation.angular_speed_bound().map_or_else(
                    || {
                        span.rotation
                            .body_angular_displacement()
                            .map_or(0., |v| v.length())
                    },
                    |speed| speed * (span.end() - span.start()),
                )
            })
            .sum()
    }
}
impl RootRigidCurve {
    pub(super) fn new(
        translation: Arc<RootCurve>,
        rotation: RootRotationCurve,
        origin: Vec3,
        bind: Vec3,
        duration: f32,
        playback: Playback,
    ) -> Result<Self, AnimationError> {
        Ok(Self(Arc::new(Curve {
            translation,
            rotation,
            origin: origin.as_dvec3(),
            bind: bind.as_dvec3(),
            duration: f64::from(duration),
            playback,
        })))
    }
    fn constant(&self, axes: [bool; 3]) -> bool {
        self.0.rotation.is_constant() && !self.0.translation.has_motion(axes)
    }
    fn pivot(&self, position: DVec3, axes: [bool; 3]) -> DVec3 {
        DVec3::from_array(std::array::from_fn(|i| {
            if axes[i] { self.0.bind[i] } else { position[i] }
        }))
    }
    fn adjusted_position(&self, position: DVec3, axes: [bool; 3]) -> DVec3 {
        position
            - DVec3::from_array(std::array::from_fn(|i| {
                if axes[i] {
                    self.0.origin[i] - self.0.bind[i]
                } else {
                    0.
                }
            }))
    }
    fn phase(&self, time: f64, axes: [bool; 3]) -> Result<RootRigidTransform, AnimationError> {
        let rotation = self.0.rotation.phase_rotation(time)?;
        let position = self.0.origin + self.0.translation.position(time);
        RootRigidTransform {
            rotation,
            translation: self.adjusted_position(position, axes)
                - rotation * self.pivot(position, axes),
        }
        .checked()
    }
    fn cycle_phase(&self, time: f64) -> Result<(u64, f64), AnimationError> {
        if !time.is_finite() || time < 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.0.playback == Playback::Clamp {
            return Ok((0, time.min(self.0.duration)));
        }
        let cycle = (time / self.0.duration).floor();
        if cycle > 9_007_199_254_740_991. {
            return Err(AnimationError::RootRigidBudget);
        }
        Ok((cycle as u64, time.rem_euclid(self.0.duration)))
    }
    /// Unwrapped composed transform relative to the first authored root factor.
    /// Each cycle transports the following cycle's displacement through its turn.
    /// Unselected translation axes remain in the displayed pose and become a
    /// moving rotation pivot; selected axes are extracted into the actor motion.
    /// # Errors
    /// Rejects invalid clocks, unrepresentable cycle indices or numerical overflow.
    pub fn sample(&self, time: f64, axes: [bool; 3]) -> Result<RootRigidTransform, AnimationError> {
        if !time.is_finite() || time < 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.constant(axes) {
            return Ok(RootRigidTransform::IDENTITY);
        }
        let (cycles, phase) = self.cycle_phase(time)?;
        let prefix = if cycles == 0 {
            RootRigidTransform::IDENTITY
        } else {
            power(self.phase(self.0.duration, axes)?, cycles)?
        };
        prefix.compose(self.phase(phase, axes)?)
    }
    fn span(
        &self,
        rotation: RootRotationSpan,
        positions: [DVec3; 4],
        prefix: RootRigidTransform,
        axes: [bool; 3],
    ) -> RootRigidSpan {
        RootRigidSpan {
            rotation,
            additive: positions
                .map(|p| prefix.translation + prefix.rotation * self.adjusted_position(p, axes)),
            pivot: positions.map(|p| self.pivot(p, axes)),
        }
    }
    fn prefix(
        &self,
        absolute: f64,
        initial_inverse: RootRigidTransform,
        cycle: RootRigidTransform,
    ) -> Result<(RootRigidTransform, f64), AnimationError> {
        let (n, phase) = self.cycle_phase(absolute)?;
        Ok((initial_inverse.compose(power(cycle, n)?)?, phase))
    }
    /// Complete ordered trajectory with exact polynomial/rotation spans. STEP
    /// events use (start,end], and simultaneous channel jumps form one event.
    /// A common old cycle prefix cancels before interval generation, avoiding
    /// subtraction of huge world displacements to recover a small tick.
    /// # Errors
    /// Invalid time, singular rotations, collapsed key times or span/loop work
    /// exhaustion reject the whole path without an accepted prefix.
    #[allow(clippy::too_many_lines)]
    pub fn path(
        &self,
        start: f64,
        end: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<RootRigidPath, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end < start {
            return Err(AnimationError::InvalidSampleTime);
        }
        if !(1..=MAX_ROOT_ROTATION_SPANS).contains(&max_spans) {
            return Err(AnimationError::RootRigidBudget);
        }
        let elapsed = end - start;
        if elapsed == 0. {
            self.sample(start, axes)?;
        }
        if self.constant(axes)
            || elapsed == 0.
            || (self.0.playback == Playback::Clamp && start >= self.0.duration)
        {
            return Ok(RootRigidPath {
                spans: vec![],
                duration: elapsed,
                end: RootRigidTransform::IDENTITY,
            });
        }
        let (_, start) = self.cycle_phase(start)?;
        let end = start + elapsed;
        if !end.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let rotations = self.0.rotation.path(start, end, max_spans)?;
        let initial = self.phase(start, axes)?.inverse()?;
        let cycle = if self.0.playback == Playback::Loop {
            self.phase(self.0.duration, axes)?
        } else {
            RootRigidTransform::IDENTITY
        };
        let end_transform = self
            .prefix(end, initial, cycle)?
            .0
            .compose(self.phase(self.cycle_phase(end)?.1, axes)?)?;
        let mut cuts = vec![0., elapsed];
        cuts.extend(
            rotations
                .spans()
                .iter()
                .flat_map(|span| [span.start(), span.end()]),
        );
        let loops = if self.0.playback == Playback::Loop {
            (end / self.0.duration).floor() as usize
        } else {
            0
        };
        if loops > max_spans {
            return Err(AnimationError::RootRigidBudget);
        }
        for n in 0..=loops {
            let base = n as f64 * self.0.duration;
            let from = (start - base).clamp(0., self.0.duration);
            let to = (end - base).clamp(0., self.0.duration);
            for key in self.0.translation.cuts(from, to, max_spans)? {
                let absolute = base + key;
                if key > 0.
                    && key < self.0.duration
                    && (absolute <= base || absolute >= base + self.0.duration)
                {
                    return Err(AnimationError::RootRigidBudget);
                }
                let relative = absolute - start;
                if relative > 0. && relative <= elapsed {
                    cuts.push(relative);
                }
            }
            let seam = base + self.0.duration - start;
            if seam > 0. && seam < elapsed {
                cuts.push(seam);
            }
            if cuts.len() > 4 * max_spans + 4 {
                return Err(AnimationError::RootRigidBudget);
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut spans = Vec::new();
        let mut cursor = 0;
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b <= a {
                return Err(AnimationError::RootRigidBudget);
            }
            while rotations
                .spans()
                .get(cursor)
                .is_some_and(|span| span.is_step() || span.end() <= a)
            {
                cursor += 1;
            }
            let rotation = rotations
                .spans()
                .get(cursor)
                .ok_or(AnimationError::RootRigidBudget)?
                .restricted(a, b)?;
            let interval_start = start + a;
            let (prefix, _) = self.prefix(interval_start, initial, cycle)?;
            let base = if self.0.playback == Playback::Loop {
                (interval_start / self.0.duration).floor() * self.0.duration
            } else {
                0.
            };
            let from = (start + a - base).min(self.0.duration);
            let to = (start + b - base).min(self.0.duration);
            let positions = self
                .0
                .translation
                .piece(from, to)
                .map(|p| p + self.0.origin);
            spans.push(self.span(rotation, positions, prefix, axes));
            let absolute = start + b;
            let (mut n, mut phase) = self.cycle_phase(absolute)?;
            if self.0.playback == Playback::Loop && phase == 0. && n > 0 {
                n -= 1;
                phase = self.0.duration;
            }
            let event = rotations
                .spans()
                .get(cursor + 1)
                .filter(|span| span.is_step() && span.start() == b);
            let jump = self.0.translation.jump(phase);
            if event.is_some() || jump.is_some() {
                let prefix = initial.compose(power(cycle, n)?)?;
                let rotation = if let Some(event) = event {
                    event.clone()
                } else {
                    RootRotationSpan::held(
                        b,
                        b,
                        prefix.rotation * self.0.rotation.phase_rotation(phase)?,
                    )
                };
                let values = jump.unwrap_or([self.0.translation.position(phase); 2]);
                let from = values[0] + self.0.origin;
                let to = values[1] + self.0.origin;
                let positions = [from, from.lerp(to, 1. / 3.), from.lerp(to, 2. / 3.), to];
                spans.push(self.span(rotation, positions, prefix, axes));
            }
            if spans.len() > max_spans {
                return Err(AnimationError::RootRigidBudget);
            }
        }
        Ok(RootRigidPath {
            spans,
            duration: elapsed,
            end: end_transform,
        })
    }
}

#[cfg(test)]
mod tests;
