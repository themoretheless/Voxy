//! Animated property tracks (scalar, vector, color, boolean) for driving morph weights,
//! shader/material uniforms, wetness/sweat factors, and gameplay curves alongside skeletal poses.

mod asset_checkpoint;
pub use asset_checkpoint::{
    PreparedPropertyAssets, PropertyAssetCheckpoint, PropertyAssetError, PropertyAssetLimits,
};

use std::sync::Arc;

use glam::Vec3;

use crate::{AnimationError, Interpolation, Playback, TrackError};

fn sample_time(time: f32, duration: f32, playback: Playback) -> f32 {
    match playback {
        Playback::Loop => time.rem_euclid(duration),
        Playback::Clamp => time.clamp(0.0, duration),
    }
}

/// Maximum number of property tracks per [`PropertyClip`].
pub const MAX_PROPERTY_TRACKS: usize = 256;
/// Maximum number of keys in a single [`PropertyTrack`].
pub const MAX_PROPERTY_KEYS: usize = 16_384;

/// Typed value on a [`PropertyTrack`] or sampled [`PropertySet`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyValue {
    Float(f32),
    Vec3(Vec3),
    Color([f32; 4]),
    Bool(bool),
}

impl PropertyValue {
    #[must_use]
    pub fn is_valid(self) -> bool {
        match self {
            Self::Float(v) => v.is_finite(),
            Self::Vec3(v) => v.is_finite(),
            Self::Color(c) => c.into_iter().all(f32::is_finite),
            Self::Bool(_) => true,
        }
    }

    #[must_use]
    pub fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Float(_), Self::Float(_))
                | (Self::Vec3(_), Self::Vec3(_))
                | (Self::Color(_), Self::Color(_))
                | (Self::Bool(_), Self::Bool(_))
        )
    }

    /// Linearly blends `self` toward `other` by `weight` in `[0, 1]`.
    /// Boolean properties switch at `weight >= 0.5`.
    ///
    /// # Errors
    /// Rejects nonfinite weights, invalid values, or mismatched property kinds.
    pub fn blend(self, other: Self, weight: f32) -> Result<Self, AnimationError> {
        self.blend64(other, f64::from(weight))
    }

    fn blend64(self, other: Self, weight: f64) -> Result<Self, AnimationError> {
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        if !self.is_valid() || !other.is_valid() || !self.same_kind(other) {
            return Err(AnimationError::InvalidGraphParam);
        }
        let w = weight.clamp(0.0, 1.0);
        // Finite endpoints can have an unrepresentable f32 difference, even
        // when the interpolated value is representable (e.g. -MAX to MAX).
        let mix = |a: f32, b: f32| (f64::from(a) + (f64::from(b) - f64::from(a)) * w) as f32;
        let out = match (self, other) {
            (Self::Float(a), Self::Float(b)) => Self::Float(mix(a, b)),
            (Self::Vec3(a), Self::Vec3(b)) => {
                Self::Vec3(Vec3::from_array(std::array::from_fn(|i| mix(a[i], b[i]))))
            }
            (Self::Color(a), Self::Color(b)) => {
                Self::Color(std::array::from_fn(|i| mix(a[i], b[i])))
            }
            (Self::Bool(a), Self::Bool(b)) => Self::Bool(if w >= 0.5 { b } else { a }),
            _ => return Err(AnimationError::InvalidGraphParam),
        };
        if !out.is_valid() {
            return Err(AnimationError::InvalidGraphParam);
        }
        Ok(out)
    }

    /// Applies an additive offset `base + weight * (additive - reference)`.
    ///
    /// # Errors
    /// Rejects nonfinite weights, invalid values, or mismatched property kinds.
    pub fn additive(
        base: Self,
        additive: Self,
        reference: Self,
        weight: f32,
    ) -> Result<Self, AnimationError> {
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        if !base.is_valid()
            || !additive.is_valid()
            || !reference.is_valid()
            || !base.same_kind(additive)
            || !base.same_kind(reference)
        {
            return Err(AnimationError::InvalidGraphParam);
        }
        let w = weight.clamp(0.0, 1.0);
        let out = match (base, additive, reference) {
            (Self::Float(b), Self::Float(a), Self::Float(r)) => Self::Float(b + w * (a - r)),
            (Self::Vec3(b), Self::Vec3(a), Self::Vec3(r)) => Self::Vec3(b + w * (a - r)),
            (Self::Color(b), Self::Color(a), Self::Color(r)) => {
                Self::Color(std::array::from_fn(|i| b[i] + w * (a[i] - r[i])))
            }
            (Self::Bool(b), Self::Bool(a), Self::Bool(_)) => {
                Self::Bool(if w >= 0.5 { a } else { b })
            }
            _ => return Err(AnimationError::InvalidGraphParam),
        };
        if !out.is_valid() {
            return Err(AnimationError::InvalidGraphParam);
        }
        Ok(out)
    }
}

/// Time-stamped keyframe on a [`PropertyTrack`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyKey {
    pub time: f32,
    pub value: PropertyValue,
}

/// Validated curve for a single named property over `[0, duration]`.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyTrack {
    name: Arc<str>,
    keys: Arc<[PropertyKey]>,
    interpolation: Interpolation,
    tangents: Arc<[(PropertyValue, PropertyValue)]>,
}

impl PropertyTrack {
    /// Creates a validated property track for a clip of `duration` seconds.
    ///
    /// # Errors
    /// Rejects empty names, empty or over-budget key lists, unsorted/out-of-range key times,
    /// nonfinite values, or mixed [`PropertyValue`] variants.
    pub fn new(
        name: impl Into<Arc<str>>,
        keys: Vec<PropertyKey>,
        duration: f32,
    ) -> Result<Self, AnimationError> {
        let name: Arc<str> = name.into();
        if name.trim().is_empty() {
            return Err(AnimationError::InvalidClipHeader);
        }
        if !duration.is_finite() || duration <= 0.0 {
            return Err(AnimationError::InvalidClipHeader);
        }
        if keys.is_empty() || keys.len() > MAX_PROPERTY_KEYS {
            return Err(AnimationError::InvalidTrack {
                joint: 0,
                reason: TrackError::InvalidValue,
            });
        }
        let first_kind = keys[0].value;
        let mut previous = None;
        for key in &keys {
            if !key.time.is_finite() || key.time < 0.0 || key.time > duration {
                return Err(AnimationError::InvalidTrack {
                    joint: 0,
                    reason: TrackError::InvalidTime,
                });
            }
            if let Some(prev) = previous
                && key.time <= prev
            {
                return Err(AnimationError::InvalidTrack {
                    joint: 0,
                    reason: TrackError::InvalidTime,
                });
            }
            if !key.value.is_valid() || !key.value.same_kind(first_kind) {
                return Err(AnimationError::InvalidTrack {
                    joint: 0,
                    reason: TrackError::InvalidValue,
                });
            }
            previous = Some(key.time);
        }
        Ok(Self {
            name,
            keys: keys.into(),
            interpolation: Interpolation::Linear,
            tangents: Arc::from([]),
        })
    }

    /// Creates a channel with explicit interpolation and incoming/outgoing
    /// derivatives per key. Cubic derivatives are per second, not per segment.
    /// Boolean tracks support only step interpolation through this constructor.
    ///
    /// # Errors
    /// Rejects invalid keys, mismatched or nonfinite derivatives, and derivatives
    /// supplied for non-cubic channels. Cubic values may overshoot their keys.
    pub fn new_with_interpolation(
        name: impl Into<Arc<str>>,
        keys: Vec<PropertyKey>,
        duration: f32,
        interpolation: Interpolation,
        tangents: Vec<(PropertyValue, PropertyValue)>,
    ) -> Result<Self, AnimationError> {
        let mut track = Self::new(name, keys, duration)?;
        let kind = track.keys[0].value;
        let cubic = interpolation == Interpolation::CubicSpline;
        if (matches!(kind, PropertyValue::Bool(_)) && interpolation != Interpolation::Step)
            || (cubic && tangents.len() != track.keys.len())
            || (!cubic && !tangents.is_empty())
            || tangents.iter().any(|(incoming, outgoing)| {
                !incoming.is_valid()
                    || !outgoing.is_valid()
                    || !kind.same_kind(*incoming)
                    || !kind.same_kind(*outgoing)
            })
        {
            return Err(AnimationError::InvalidGraphParam);
        }
        track.interpolation = interpolation;
        track.tangents = tangents.into();
        Ok(track)
    }

    #[must_use]
    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    #[must_use]
    pub fn tangents(&self) -> &[(PropertyValue, PropertyValue)] {
        &self.tangents
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn keys(&self) -> &[PropertyKey] {
        &self.keys
    }

    /// Samples this track at `time` (seconds) using the given `duration` and `playback` policy.
    #[must_use]
    pub fn sample(&self, time: f32, duration: f32, playback: Playback) -> PropertyValue {
        self.sample_at(f64::from(sample_time(time, duration, playback)))
    }

    /// Sample from an existing f64 animation clock without narrowing the time.
    /// # Errors
    /// Rejects invalid time/duration and nonfinite interpolated output.
    pub fn sample_clock(
        &self,
        time: f64,
        duration: f32,
        playback: Playback,
    ) -> Result<PropertyValue, AnimationError> {
        if !time.is_finite() || !duration.is_finite() || duration <= 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        let duration = f64::from(duration);
        let t = match playback {
            Playback::Loop => time.rem_euclid(duration),
            Playback::Clamp => time.clamp(0., duration),
        };
        let value = self.sample_at(t);
        if !value.is_valid() {
            return Err(AnimationError::InvalidGraphParam);
        }
        Ok(value)
    }

    fn sample_at(&self, t: f64) -> PropertyValue {
        let keys = &self.keys;
        if keys.len() == 1 || t <= f64::from(keys[0].time) {
            return keys[0].value;
        }
        let last = keys.len() - 1;
        if t >= f64::from(keys[last].time) {
            return keys[last].value;
        }
        let next = keys.partition_point(|k| f64::from(k.time) <= t);
        let prev = next - 1;
        let a = keys[prev];
        let b = keys[next];
        let span = f64::from(b.time) - f64::from(a.time);
        if span <= 0.0 {
            return b.value;
        }
        let alpha = ((t - f64::from(a.time)) / span).clamp(0.0, 1.0);
        // Bool tracks hold their previous value until the next keyframe time.
        if self.interpolation == Interpolation::Step || matches!(a.value, PropertyValue::Bool(_)) {
            a.value
        } else if self.interpolation == Interpolation::CubicSpline {
            let outgoing = self.tangents[prev].1;
            let incoming = self.tangents[next].0;
            // Evaluate in f64 so a finite f32 derivative times the segment
            // length does not overflow before cancellation in the polynomial.
            let t = alpha;
            let t2 = t * t;
            let t3 = t2 * t;
            let scalar = |a: f32, b: f32, outgoing: f32, incoming: f32| {
                ((2.0 * t3 - 3.0 * t2 + 1.0) * f64::from(a)
                    + (t3 - 2.0 * t2 + t) * span * f64::from(outgoing)
                    + (-2.0 * t3 + 3.0 * t2) * f64::from(b)
                    + (t3 - t2) * span * f64::from(incoming)) as f32
            };
            match (a.value, b.value, outgoing, incoming) {
                (
                    PropertyValue::Float(a),
                    PropertyValue::Float(b),
                    PropertyValue::Float(o),
                    PropertyValue::Float(i),
                ) => PropertyValue::Float(scalar(a, b, o, i)),
                (
                    PropertyValue::Vec3(a),
                    PropertyValue::Vec3(b),
                    PropertyValue::Vec3(o),
                    PropertyValue::Vec3(i),
                ) => PropertyValue::Vec3(Vec3::from_array(std::array::from_fn(|n| {
                    scalar(a[n], b[n], o[n], i[n])
                }))),
                (
                    PropertyValue::Color(a),
                    PropertyValue::Color(b),
                    PropertyValue::Color(o),
                    PropertyValue::Color(i),
                ) => PropertyValue::Color(std::array::from_fn(|n| scalar(a[n], b[n], o[n], i[n]))),
                _ => unreachable!("validated cubic property schema"),
            }
        } else {
            a.value.blend64(b.value, alpha).unwrap_or(a.value)
        }
    }
}

/// Evaluated snapshot of named properties at a single animation frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropertySet {
    entries: Vec<(Arc<str>, PropertyValue)>,
}

impl PropertySet {
    /// Builds a sorted property set, rejecting duplicate or empty names and invalid values.
    ///
    /// # Errors
    /// Returns an error if any name is empty/duplicated or any value is nonfinite.
    pub fn new(mut entries: Vec<(Arc<str>, PropertyValue)>) -> Result<Self, AnimationError> {
        if entries.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimationError::InvalidGraphParam);
        }
        for (index, (name, value)) in entries.iter().enumerate() {
            if name.trim().is_empty() {
                return Err(AnimationError::InvalidJointName(index));
            }
            if !value.is_valid() {
                return Err(AnimationError::InvalidGraphParam);
            }
        }
        entries.sort_by(|a, b| a.0.as_ref().cmp(b.0.as_ref()));
        if entries.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(AnimationError::InvalidJointName(0));
        }
        Ok(Self { entries })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn entries(&self) -> &[(Arc<str>, PropertyValue)] {
        &self.entries
    }

    /// Looks up a property by name in $O(\log N)$ time.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<PropertyValue> {
        self.entries
            .binary_search_by(|(k, _)| k.as_ref().cmp(name))
            .ok()
            .map(|idx| self.entries[idx].1)
    }

    /// Looks up a float property by name.
    #[must_use]
    pub fn get_float(&self, name: &str) -> Option<f32> {
        match self.get(name)? {
            PropertyValue::Float(v) => Some(v),
            _ => None,
        }
    }

    /// Blends two [`PropertySet`]s with identical property schemas.
    ///
    /// # Errors
    /// Rejects mismatched property sets or invalid blend weights.
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Result<Self, AnimationError> {
        if a.entries.len() != b.entries.len() {
            return Err(AnimationError::InvalidGraphParam);
        }
        let mut entries = Vec::with_capacity(a.entries.len());
        for ((name_a, val_a), (name_b, val_b)) in a.entries.iter().zip(&b.entries) {
            if name_a != name_b {
                return Err(AnimationError::InvalidGraphParam);
            }
            entries.push((Arc::clone(name_a), val_a.blend(*val_b, weight)?));
        }
        Ok(Self { entries })
    }

    /// Applies an additive property delta `base + weight * (additive - reference)`.
    ///
    /// # Errors
    /// Rejects mismatched schemas or invalid weights.
    pub fn additive(
        base: &Self,
        additive: &Self,
        reference: &Self,
        weight: f32,
    ) -> Result<Self, AnimationError> {
        if base.entries.len() != additive.entries.len()
            || base.entries.len() != reference.entries.len()
        {
            return Err(AnimationError::InvalidGraphParam);
        }
        let mut entries = Vec::with_capacity(base.entries.len());
        for (((name_b, val_b), (name_a, val_a)), (name_r, val_r)) in base
            .entries
            .iter()
            .zip(&additive.entries)
            .zip(&reference.entries)
        {
            if name_b != name_a || name_b != name_r {
                return Err(AnimationError::InvalidGraphParam);
            }
            entries.push((
                Arc::clone(name_b),
                PropertyValue::additive(*val_b, *val_a, *val_r, weight)?,
            ));
        }
        Ok(Self { entries })
    }

    /// Blends an arbitrary number of weighted [`PropertySet`] samples (e.g. from a 1D/2D blend space).
    ///
    /// # Errors
    /// Rejects empty inputs, negative/nonfinite weights, zero total weight, or mismatched schemas.
    pub fn blend_weighted(samples: &[(&Self, f32)]) -> Result<Self, AnimationError> {
        let Some(&(first, first_w)) = samples.first() else {
            return Err(AnimationError::InvalidBlendWeight);
        };
        if !first_w.is_finite() || first_w < 0.0 {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let mut acc = first.clone();
        let mut total_w = first_w;
        for &(sample, w) in &samples[1..] {
            if !w.is_finite() || w < 0.0 {
                return Err(AnimationError::InvalidBlendWeight);
            }
            let new_total = total_w + w;
            if new_total > 0.0 {
                let alpha = w / new_total;
                acc = Self::blend(&acc, sample, alpha)?;
            }
            total_w = new_total;
        }
        if total_w <= 0.0 {
            return Err(AnimationError::InvalidBlendWeight);
        }
        Ok(acc)
    }
}

/// Immutable collection of [`PropertyTrack`]s synchronized with an [`AnimationClip`](crate::AnimationClip).
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyClip {
    name: Arc<str>,
    duration: f32,
    playback: Playback,
    tracks: Arc<[PropertyTrack]>,
}

impl PropertyClip {
    /// Creates a validated property clip with unique track names sorted lexicographically.
    ///
    /// # Errors
    /// Rejects empty clip names, nonpositive duration, duplicate track names, or > [`MAX_PROPERTY_TRACKS`].
    pub fn new(
        name: impl Into<Arc<str>>,
        duration: f32,
        playback: Playback,
        mut tracks: Vec<PropertyTrack>,
    ) -> Result<Self, AnimationError> {
        let name: Arc<str> = name.into();
        if name.trim().is_empty() {
            return Err(AnimationError::InvalidClipHeader);
        }
        if !duration.is_finite() || duration <= 0.0 {
            return Err(AnimationError::InvalidClipHeader);
        }
        if tracks.len() > MAX_PROPERTY_TRACKS {
            return Err(AnimationError::InvalidClipHeader);
        }
        for (joint, track) in tracks.iter().enumerate() {
            if track.keys().iter().any(|k| k.time > duration) {
                return Err(AnimationError::InvalidTrack {
                    joint,
                    reason: TrackError::InvalidTime,
                });
            }
        }
        tracks.sort_by(|a, b| a.name().cmp(b.name()));
        if tracks.windows(2).any(|w| w[0].name() == w[1].name()) {
            return Err(AnimationError::InvalidJointName(0));
        }
        Ok(Self {
            name,
            duration,
            playback,
            tracks: tracks.into(),
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[must_use]
    pub fn playback(&self) -> Playback {
        self.playback
    }

    #[must_use]
    pub fn tracks(&self) -> &[PropertyTrack] {
        &self.tracks
    }

    /// Samples all property tracks at `time` (seconds).
    ///
    /// # Errors
    /// Rejects nonfinite sample times.
    pub fn sample(&self, time: f32) -> Result<PropertySet, AnimationError> {
        self.sample_clock(f64::from(time))
    }

    /// Uses the skeletal animator's existing f64 clock with the same loop/clamp policy.
    /// # Errors
    /// Rejects nonfinite times and interpolated values.
    pub fn sample_clock(&self, time: f64) -> Result<PropertySet, AnimationError> {
        if !time.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        let entries = self
            .tracks
            .iter()
            .map(|track| {
                Ok((
                    Arc::clone(&track.name),
                    track.sample_clock(time, self.duration, self.playback)?,
                ))
            })
            .collect::<Result<Vec<_>, AnimationError>>()?;
        let result = PropertySet { entries };
        if result.entries.iter().any(|(_, value)| !value.is_valid()) {
            return Err(AnimationError::InvalidGraphParam);
        }
        Ok(result)
    }

    /// Inclusive authored-phase preview independent of the runtime loop policy.
    /// Uses f64 time and includes the last key at phase one.
    /// # Errors
    /// Rejects phases outside [0,1], nonfinite phases or invalid sampled values.
    pub fn sample_authored_phase(&self, phase: f64) -> Result<PropertySet, AnimationError> {
        if !phase.is_finite() || !(0.0..=1.0).contains(&phase) {
            return Err(AnimationError::InvalidAnimationPhase);
        }
        let time = phase * f64::from(self.duration);
        let entries = self
            .tracks
            .iter()
            .map(|track| {
                Ok((
                    track.name.clone(),
                    track.sample_clock(time, self.duration, Playback::Clamp)?,
                ))
            })
            .collect::<Result<Vec<_>, AnimationError>>()?;
        PropertySet::new(entries)
    }

    /// Samples all property tracks at normalized `phase` in `[0, 1]`.
    ///
    /// # Errors
    /// Rejects nonfinite phases.
    pub fn sample_phase(&self, phase: f64) -> Result<PropertySet, AnimationError> {
        if !phase.is_finite() {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.sample_clock(phase * f64::from(self.duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_property_extremes_interpolate_without_overflow_or_fallback() {
        for (a, b, zero) in [
            (
                PropertyValue::Float(-f32::MAX),
                PropertyValue::Float(f32::MAX),
                PropertyValue::Float(0.),
            ),
            (
                PropertyValue::Vec3(Vec3::splat(-f32::MAX)),
                PropertyValue::Vec3(Vec3::splat(f32::MAX)),
                PropertyValue::Vec3(Vec3::ZERO),
            ),
            (
                PropertyValue::Color([-f32::MAX; 4]),
                PropertyValue::Color([f32::MAX; 4]),
                PropertyValue::Color([0.; 4]),
            ),
        ] {
            assert_eq!(a.blend(b, 0.5).unwrap(), zero);
            let track = PropertyTrack::new(
                "weight",
                vec![
                    PropertyKey { time: 0., value: a },
                    PropertyKey { time: 1., value: b },
                ],
                1.,
            )
            .unwrap();
            assert_eq!(track.sample(0.5, 1., Playback::Clamp), zero);
        }
    }

    #[test]
    fn cubic_property_derivatives_use_segment_seconds_and_preserve_overshoot() {
        let keys = vec![
            PropertyKey {
                time: 1.0,
                value: PropertyValue::Float(0.0),
            },
            PropertyKey {
                time: 3.0,
                value: PropertyValue::Float(1.0),
            },
        ];
        let tangents = vec![
            (PropertyValue::Float(0.0), PropertyValue::Float(4.0)),
            (PropertyValue::Float(0.0), PropertyValue::Float(0.0)),
        ];
        let cubic = PropertyTrack::new_with_interpolation(
            "weight",
            keys.clone(),
            4.0,
            Interpolation::CubicSpline,
            tangents,
        )
        .unwrap();
        // Hermite midpoint: .5 + .125 * 2 seconds * 4/second = 1.5.
        assert_eq!(
            cubic.sample(2.0, 4.0, Playback::Clamp),
            PropertyValue::Float(1.5)
        );
        assert_eq!(cubic.sample(1.0, 4.0, Playback::Clamp), keys[0].value);
        assert_eq!(cubic.sample(3.0, 4.0, Playback::Clamp), keys[1].value);
        let step =
            PropertyTrack::new_with_interpolation("weight", keys, 4.0, Interpolation::Step, vec![])
                .unwrap();
        assert_eq!(
            step.sample(2.99, 4.0, Playback::Clamp),
            PropertyValue::Float(0.0)
        );
        assert_eq!(
            step.sample(3.0, 4.0, Playback::Clamp),
            PropertyValue::Float(1.0)
        );
        let clip = PropertyClip::new("shape", 4.0, Playback::Loop, vec![cubic]).unwrap();
        assert_eq!(clip.sample(-2.0).unwrap().get_float("weight"), Some(1.5));
    }

    #[test]
    fn cubic_property_schema_and_overflow_are_rejected() {
        let keys = vec![
            PropertyKey {
                time: 0.0,
                value: PropertyValue::Float(0.0),
            },
            PropertyKey {
                time: 16.0,
                value: PropertyValue::Float(0.0),
            },
        ];
        assert!(
            PropertyTrack::new_with_interpolation(
                "x",
                keys.clone(),
                16.0,
                Interpolation::CubicSpline,
                vec![]
            )
            .is_err()
        );
        let wrong = vec![
            (
                PropertyValue::Vec3(Vec3::ZERO),
                PropertyValue::Vec3(Vec3::ZERO)
            );
            2
        ];
        assert!(
            PropertyTrack::new_with_interpolation(
                "x",
                keys.clone(),
                16.0,
                Interpolation::CubicSpline,
                wrong
            )
            .is_err()
        );
        let bad = vec![(PropertyValue::Float(0.0), PropertyValue::Float(f32::NAN)); 2];
        assert!(
            PropertyTrack::new_with_interpolation(
                "x",
                keys.clone(),
                16.0,
                Interpolation::CubicSpline,
                bad
            )
            .is_err()
        );
        let tangents = vec![
            (PropertyValue::Float(0.0), PropertyValue::Float(f32::MAX)),
            (PropertyValue::Float(-f32::MAX), PropertyValue::Float(0.0)),
        ];
        assert!(
            PropertyTrack::new_with_interpolation(
                "x",
                keys.clone(),
                16.0,
                Interpolation::Step,
                tangents.clone()
            )
            .is_err()
        );
        let track = PropertyTrack::new_with_interpolation(
            "x",
            keys,
            16.0,
            Interpolation::CubicSpline,
            tangents,
        )
        .unwrap();
        let clip = PropertyClip::new("shape", 16.0, Playback::Clamp, vec![track]).unwrap();
        assert!(clip.sample(8.0).is_err());
    }

    #[test]
    fn cubic_vector_and_color_properties_follow_the_same_curve() {
        for (name, zero, one, expected) in [
            (
                "vector",
                PropertyValue::Vec3(Vec3::ZERO),
                PropertyValue::Vec3(Vec3::ONE),
                PropertyValue::Vec3(Vec3::splat(0.15625)),
            ),
            (
                "color",
                PropertyValue::Color([0.0; 4]),
                PropertyValue::Color([1.0; 4]),
                PropertyValue::Color([0.15625; 4]),
            ),
        ] {
            let keys = vec![
                PropertyKey {
                    time: 0.0,
                    value: zero,
                },
                PropertyKey {
                    time: 2.0,
                    value: one,
                },
            ];
            let track = PropertyTrack::new_with_interpolation(
                name,
                keys,
                2.0,
                Interpolation::CubicSpline,
                vec![(zero, zero); 2],
            )
            .unwrap();
            assert_eq!(track.sample(0.5, 2.0, Playback::Clamp), expected);
        }
    }

    #[test]
    fn property_tracks_sample_blend_and_apply_additive_curves() {
        let wetness = PropertyTrack::new(
            "surface_wetness",
            vec![
                PropertyKey {
                    time: 0.0,
                    value: PropertyValue::Float(0.2),
                },
                PropertyKey {
                    time: 1.0,
                    value: PropertyValue::Float(0.8),
                },
            ],
            1.0,
        )
        .unwrap();
        let tint = PropertyTrack::new(
            "emission_tint",
            vec![
                PropertyKey {
                    time: 0.0,
                    value: PropertyValue::Color([1.0, 0.0, 0.0, 1.0]),
                },
                PropertyKey {
                    time: 1.0,
                    value: PropertyValue::Color([0.0, 1.0, 0.0, 1.0]),
                },
            ],
            1.0,
        )
        .unwrap();
        let foot_down = PropertyTrack::new(
            "left_foot_locked",
            vec![
                PropertyKey {
                    time: 0.0,
                    value: PropertyValue::Bool(true),
                },
                PropertyKey {
                    time: 0.5,
                    value: PropertyValue::Bool(false),
                },
            ],
            1.0,
        )
        .unwrap();

        let clip = PropertyClip::new(
            "locomotion_props",
            1.0,
            Playback::Loop,
            vec![wetness, tint, foot_down],
        )
        .unwrap();

        let s0 = clip.sample(0.0).unwrap();
        let s_mid = clip.sample(0.5).unwrap();
        let s1 = clip.sample(0.999).unwrap();

        assert!((s_mid.get_float("surface_wetness").unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(s0.get("left_foot_locked"), Some(PropertyValue::Bool(true)));
        assert_eq!(
            s_mid.get("left_foot_locked"),
            Some(PropertyValue::Bool(false))
        );

        let blended = PropertySet::blend(&s0, &s1, 0.25).unwrap();
        assert!((blended.get_float("surface_wetness").unwrap() - 0.35).abs() < 1e-3);

        let weighted = PropertySet::blend_weighted(&[(&s0, 0.75), (&s1, 0.25)]).unwrap();
        assert!((weighted.get_float("surface_wetness").unwrap() - 0.35).abs() < 1e-3);

        let add = PropertySet::additive(&s_mid, &s1, &s0, 0.5).unwrap();
        // 0.5 + 0.5 * (0.8 - 0.2) = 0.8
        assert!((add.get_float("surface_wetness").unwrap() - 0.8).abs() < 1e-3);
    }
}
