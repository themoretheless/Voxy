//! Error-bounded clip compression: uniform resampling, constant-channel
//! elimination, Ramer–Douglas–Peucker key reduction and 16-bit range
//! quantization (smallest-three for rotations). The reported error is
//! measured in model space (joint positions and orientations), and channels
//! that would exceed the bound fall back to unquantized keys.

use std::sync::Arc;

use glam::{Quat, Vec3};

use crate::{
    AnimationClip, AnimationError, Joint, JointTrack, Playback, Pose, QuatKey, Skeleton, Transform,
    Vec3Key, rigs_match,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompressionSettings {
    /// Model-space joint position error bound, in model units.
    pub translation_error: f32,
    /// Model-space joint orientation error bound, in radians.
    pub rotation_error: f32,
    /// Relative scale error bound.
    pub scale_error: f32,
    /// Resampling rate in Hz.
    pub sample_rate: f32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            translation_error: 1e-3,
            rotation_error: 2e-3,
            scale_error: 1e-3,
            sample_rate: 30.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompressionReport {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub max_translation_error: f32,
    pub max_rotation_error: f32,
    pub max_scale_error: f32,
    /// Channels that kept unquantized keys to honour the bound.
    pub raw_channels: usize,
}

/// Range-quantized 3-vector: 16 bits per component.
#[derive(Clone, Debug, PartialEq)]
struct QuantizedVec3 {
    min: Vec3,
    max: Vec3,
    values: Vec<[u16; 3]>,
}

impl QuantizedVec3 {
    fn new(values: &[Vec3]) -> Self {
        let min = values.iter().fold(Vec3::INFINITY, |m, v| m.min(*v));
        let max = values.iter().fold(Vec3::NEG_INFINITY, |m, v| m.max(*v));
        let span = (max - min).max(Vec3::splat(f32::EPSILON));
        let packed = values
            .iter()
            .map(|v| {
                let t = ((*v - min) / span * 65535.0).round();
                [t.x as u16, t.y as u16, t.z as u16]
            })
            .collect();
        Self {
            min,
            max,
            values: packed,
        }
    }

    fn get(&self, index: usize) -> Vec3 {
        let q = self.values[index];
        let t = Vec3::new(f32::from(q[0]), f32::from(q[1]), f32::from(q[2])) / 65535.0;
        self.min + (self.max - self.min) * t
    }
}

/// Smallest-three quaternion: drop the largest component, store the other
/// three as 16-bit values in `[-1/√2, 1/√2]`, plus the dropped index.
#[derive(Clone, Copy, Debug, PartialEq)]
struct QuantizedQuat {
    components: [u16; 3],
    dropped: u8,
}

impl QuantizedQuat {
    const LIMIT: f32 = std::f32::consts::FRAC_1_SQRT_2;

    fn new(q: Quat) -> Self {
        let array = q.normalize().to_array();
        let dropped = (0..4)
            .max_by(|&a, &b| array[a].abs().total_cmp(&array[b].abs()))
            .unwrap_or(3);
        let sign = if array[dropped] < 0.0 { -1.0 } else { 1.0 };
        let mut components = [0u16; 3];
        let mut k = 0;
        for (i, value) in array.iter().enumerate() {
            if i == dropped {
                continue;
            }
            let t = ((value * sign / Self::LIMIT).clamp(-1.0, 1.0) + 1.0) * 0.5 * 65535.0;
            components[k] = t.round() as u16;
            k += 1;
        }
        Self {
            components,
            dropped: dropped as u8,
        }
    }

    fn get(self) -> Quat {
        let mut array = [0.0f32; 4];
        let mut k = 0;
        let mut sum = 0.0f32;
        for (i, slot) in array.iter_mut().enumerate() {
            if i == usize::from(self.dropped) {
                continue;
            }
            let v = (f32::from(self.components[k]) / 65535.0 * 2.0 - 1.0) * Self::LIMIT;
            *slot = v;
            sum += v * v;
            k += 1;
        }
        array[usize::from(self.dropped)] = (1.0 - sum).max(0.0).sqrt();
        Quat::from_array(array).normalize()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Vec3Channel {
    Constant(Vec3),
    Quantized {
        times: Vec<f32>,
        values: QuantizedVec3,
    },
    Raw {
        times: Vec<f32>,
        values: Vec<Vec3>,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum QuatChannel {
    Constant(Quat),
    Quantized {
        times: Vec<f32>,
        values: Vec<QuantizedQuat>,
    },
    Raw {
        times: Vec<f32>,
        values: Vec<Quat>,
    },
}

impl Vec3Channel {
    fn sample(&self, time: f32) -> Vec3 {
        match self {
            Self::Constant(v) => *v,
            Self::Quantized { times, values } => {
                let (i, j, t) = segment(times, time);
                values.get(i).lerp(values.get(j), t)
            }
            Self::Raw { times, values } => {
                let (i, j, t) = segment(times, time);
                values[i].lerp(values[j], t)
            }
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            Self::Constant(_) => 12,
            Self::Quantized { times, .. } => 24 + times.len() * (4 + 6),
            Self::Raw { times, .. } => times.len() * (4 + 12),
        }
    }

    fn keys(&self) -> Vec<Vec3Key> {
        match self {
            Self::Constant(v) => vec![Vec3Key {
                time: 0.0,
                value: *v,
            }],
            Self::Quantized { times, values } => times
                .iter()
                .enumerate()
                .map(|(i, &time)| Vec3Key {
                    time,
                    value: values.get(i),
                })
                .collect(),
            Self::Raw { times, values } => times
                .iter()
                .zip(values)
                .map(|(&time, &value)| Vec3Key { time, value })
                .collect(),
        }
    }
}

impl QuatChannel {
    fn sample(&self, time: f32) -> Quat {
        match self {
            Self::Constant(q) => *q,
            Self::Quantized { times, values } => {
                let (i, j, t) = segment(times, time);
                values[i].get().slerp(values[j].get(), t).normalize()
            }
            Self::Raw { times, values } => {
                let (i, j, t) = segment(times, time);
                values[i].slerp(values[j], t).normalize()
            }
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            Self::Constant(_) => 16,
            Self::Quantized { times, .. } => times.len() * (4 + 7),
            Self::Raw { times, .. } => times.len() * (4 + 16),
        }
    }

    fn keys(&self) -> Vec<QuatKey> {
        match self {
            Self::Constant(q) => vec![QuatKey {
                time: 0.0,
                value: *q,
            }],
            Self::Quantized { times, values } => times
                .iter()
                .zip(values)
                .map(|(&time, q)| QuatKey {
                    time,
                    value: q.get(),
                })
                .collect(),
            Self::Raw { times, values } => times
                .iter()
                .zip(values)
                .map(|(&time, &value)| QuatKey { time, value })
                .collect(),
        }
    }
}

/// Neighbouring key indices and interpolation factor for `time`.
fn segment(times: &[f32], time: f32) -> (usize, usize, f32) {
    if time <= times[0] {
        return (0, 0, 0.0);
    }
    let last = times.len() - 1;
    if time >= times[last] {
        return (last, last, 0.0);
    }
    let j = times.partition_point(|&t| t <= time);
    let i = j - 1;
    let span = times[j] - times[i];
    let t = if span > 0.0 {
        (time - times[i]) / span
    } else {
        0.0
    };
    (i, j, t)
}

#[derive(Clone, Debug)]
struct JointChannels {
    translation: Vec3Channel,
    rotation: QuatChannel,
    scale: Vec3Channel,
}

/// Compressed clip sampled by phase; `decompress` restores an [`AnimationClip`].
#[derive(Clone, Debug)]
pub struct CompressedClip {
    rig: Arc<[Joint]>,
    name: Arc<str>,
    duration: f32,
    playback: Playback,
    joints: Vec<JointChannels>,
}

impl CompressedClip {
    /// # Errors
    /// Rejects nonpositive/nonfinite settings.
    pub fn compress(
        clip: &AnimationClip,
        settings: &CompressionSettings,
    ) -> Result<(Self, CompressionReport), AnimationError> {
        if !settings.translation_error.is_finite()
            || settings.translation_error <= 0.0
            || !settings.rotation_error.is_finite()
            || settings.rotation_error <= 0.0
            || !settings.scale_error.is_finite()
            || settings.scale_error <= 0.0
            || !settings.sample_rate.is_finite()
            || settings.sample_rate <= 0.0
        {
            return Err(AnimationError::InvalidClipHeader);
        }
        let skeleton = Skeleton {
            joints: clip.rig.clone(),
        };
        let duration = clip.duration;
        let joint_count = clip.rig.len();
        let mut best: Option<(Self, CompressionReport)> = None;
        // Resampling error between samples is independent of key reduction, so
        // the sample rate doubles when tightening tolerances stops helping.
        let mut sample_rate = settings.sample_rate;
        'rates: for _rate_attempt in 0..4 {
            let sample_count = ((duration * sample_rate).ceil() as usize).max(1) + 1;
            let times: Vec<f32> = (0..sample_count)
                .map(|i| (duration * i as f32 / (sample_count - 1) as f32).min(duration))
                .collect();
            let samples: Vec<Pose> = times
                .iter()
                .map(|&t| clip.try_sample_local(&skeleton, t))
                .collect::<Result<_, _>>()?;
            // Local tolerances start at the model-space bounds and tighten
            // until the measured model-space error fits.
            let mut local_translation = settings.translation_error;
            let mut local_rotation = settings.rotation_error;
            let mut local_scale = settings.scale_error;
            let mut previous_error: Option<(f32, f32, f32)> = None;
            for _attempt in 0..6 {
                let mut joints = Vec::with_capacity(joint_count);
                let mut raw_channels = 0usize;
                for j in 0..joint_count {
                    let translations: Vec<Vec3> =
                        samples.iter().map(|p| p.local[j].translation).collect();
                    let rotations: Vec<Quat> =
                        samples.iter().map(|p| p.local[j].rotation).collect();
                    let scales: Vec<Vec3> = samples.iter().map(|p| p.local[j].scale).collect();
                    let translation = vec3_channel(&times, &translations, local_translation, false);
                    let rotation = quat_channel(&times, &rotations, local_rotation);
                    let scale = vec3_channel(&times, &scales, local_scale, true);
                    raw_channels += usize::from(matches!(translation, Vec3Channel::Raw { .. }))
                        + usize::from(matches!(rotation, QuatChannel::Raw { .. }))
                        + usize::from(matches!(scale, Vec3Channel::Raw { .. }));
                    joints.push(JointChannels {
                        translation,
                        rotation,
                        scale,
                    });
                }
                let compressed = Self {
                    rig: clip.rig.clone(),
                    name: clip.name.clone(),
                    duration,
                    playback: clip.playback,
                    joints,
                };
                let (te, re, se) = compressed.measure_error(clip, &skeleton, &times)?;
                let report = CompressionReport {
                    input_bytes: clip_byte_len(clip),
                    output_bytes: compressed.byte_len(),
                    max_translation_error: te,
                    max_rotation_error: re,
                    max_scale_error: se,
                    raw_channels,
                };
                let fits = te <= settings.translation_error
                    && re <= settings.rotation_error
                    && se <= settings.scale_error;
                let better = best
                    .as_ref()
                    .is_none_or(|(_, b)| worse_than(b, settings) > worse_than(&report, settings));
                if better {
                    best = Some((compressed, report));
                }
                if fits {
                    break 'rates;
                }
                // Tightening stopped helping: the residual is resampling error.
                if previous_error
                    .is_some_and(|(pt, pr, ps)| te >= pt * 0.9 && re >= pr * 0.9 && se >= ps * 0.9)
                {
                    break;
                }
                previous_error = Some((te, re, se));
                if te > settings.translation_error {
                    // Joint positions move through ancestor rotations too.
                    local_translation *= 0.5;
                    local_rotation *= 0.5;
                }
                if re > settings.rotation_error {
                    local_rotation *= 0.5;
                }
                if se > settings.scale_error {
                    local_scale *= 0.5;
                }
            }
            sample_rate *= 2.0;
        }
        best.ok_or(AnimationError::InvalidClipHeader)
    }

    /// Model-space error against `clip` at `times` and their midpoints.
    fn measure_error(
        &self,
        clip: &AnimationClip,
        skeleton: &Skeleton,
        times: &[f32],
    ) -> Result<(f32, f32, f32), AnimationError> {
        let mut probes: Vec<f32> = times.to_vec();
        probes.extend(times.windows(2).map(|w| (w[0] + w[1]) * 0.5));
        let (mut te, mut re, mut se) = (0.0f32, 0.0f32, 0.0f32);
        let mut ga = Vec::new();
        let mut gb = Vec::new();
        let mut skin = Vec::new();
        for &t in &probes {
            let original = clip.try_sample_local(skeleton, t)?;
            let decoded = self.sample_local(t);
            original.skin_matrices_into(skeleton, &mut ga, &mut skin)?;
            decoded.skin_matrices_into(skeleton, &mut gb, &mut skin)?;
            for (a, b) in ga.iter().zip(&gb) {
                let (sa, ra, ta) = a.to_scale_rotation_translation();
                let (sb, rb, tb) = b.to_scale_rotation_translation();
                te = te.max((ta - tb).length());
                re = re.max(quat_angle(ra, rb));
                se = se.max(
                    ((sb - sa) / sa.abs().max(Vec3::splat(f32::EPSILON)))
                        .abs()
                        .max_element(),
                );
            }
        }
        Ok((te, re, se))
    }

    fn sample_local(&self, time: f32) -> Pose {
        Pose {
            rig: self.rig.clone(),
            local: self
                .joints
                .iter()
                .map(|j| Transform {
                    translation: j.translation.sample(time),
                    rotation: j.rotation.sample(time),
                    scale: j.scale.sample(time),
                })
                .collect(),
        }
    }

    /// # Errors
    /// Rejects a foreign rig, a phase outside `[0, 1]` or an invalid pose.
    pub fn try_sample_phase(
        &self,
        skeleton: &Skeleton,
        phase: f64,
    ) -> Result<Pose, AnimationError> {
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !phase.is_finite() || !(0.0..=1.0).contains(&phase) {
            return Err(AnimationError::InvalidSampleTime);
        }
        let pose = self.sample_local((phase * f64::from(self.duration)) as f32);
        for (i, t) in pose.local.iter().enumerate() {
            if !t.is_valid() {
                return Err(AnimationError::InvalidPose(i));
            }
        }
        Ok(pose)
    }

    /// Rebuilds a linear-key clip from the kept keys; constant channels
    /// become single keys.
    ///
    /// # Errors
    /// Propagates clip validation errors.
    pub fn decompress(&self) -> Result<AnimationClip, AnimationError> {
        let skeleton = Skeleton {
            joints: self.rig.clone(),
        };
        let tracks = self
            .joints
            .iter()
            .map(|j| JointTrack {
                translations: j.translation.keys(),
                rotations: j.rotation.keys(),
                scales: j.scale.keys(),
            })
            .collect();
        AnimationClip::new(
            self.name.clone(),
            self.duration,
            self.playback,
            tracks,
            &skeleton,
        )
    }

    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.joints
            .iter()
            .map(|j| j.translation.byte_len() + j.rotation.byte_len() + j.scale.byte_len())
            .sum()
    }

    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Exact minimal rotation angle between two unit quaternions (glam's
/// `angle_between` uses an approximate arccosine).
pub(crate) fn quat_angle(a: Quat, b: Quat) -> f32 {
    // Chord-based form keeps precision for tiny angles where acos(dot) loses it.
    let b = if a.dot(b) < 0.0 { -b } else { b };
    let chord = (a - b).length() * 0.5;
    2.0 * chord.min(1.0).asin()
}

/// How far a report exceeds its bounds (0 when it fits), for picking the best attempt.
fn worse_than(report: &CompressionReport, settings: &CompressionSettings) -> f32 {
    (report.max_translation_error / settings.translation_error - 1.0).max(0.0)
        + (report.max_rotation_error / settings.rotation_error - 1.0).max(0.0)
        + (report.max_scale_error / settings.scale_error - 1.0).max(0.0)
}

fn clip_byte_len(clip: &AnimationClip) -> usize {
    clip.tracks
        .iter()
        .map(|t| t.translations.len() * 16 + t.rotations.len() * 20 + t.scales.len() * 16)
        .sum()
}

/// Ramer–Douglas–Peucker over uniformly sampled values with a caller
/// distance; returns kept sample indices (always first and last).
fn reduce<T: Copy>(
    times: &[f32],
    values: &[T],
    tolerance: f32,
    lerp: impl Fn(T, T, f32) -> T,
    distance: impl Fn(T, T) -> f32,
) -> Vec<usize> {
    let mut keep = vec![false; values.len()];
    keep[0] = true;
    keep[values.len() - 1] = true;
    let mut stack = vec![(0usize, values.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let span = times[b] - times[a];
        let mut worst = (0.0f32, a);
        for i in a + 1..b {
            let t = if span > 0.0 {
                (times[i] - times[a]) / span
            } else {
                0.0
            };
            let predicted = lerp(values[a], values[b], t);
            let d = distance(predicted, values[i]);
            if d > worst.0 {
                worst = (d, i);
            }
        }
        if worst.0 > tolerance {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    (0..values.len()).filter(|&i| keep[i]).collect()
}

fn vec3_channel(times: &[f32], values: &[Vec3], tolerance: f32, relative: bool) -> Vec3Channel {
    let distance = |a: Vec3, b: Vec3| {
        if relative {
            ((a - b) / b.abs().max(Vec3::splat(f32::EPSILON)))
                .abs()
                .max_element()
        } else {
            (a - b).length()
        }
    };
    if values
        .iter()
        .all(|v| distance(*v, values[0]) <= tolerance * 0.1)
    {
        return Vec3Channel::Constant(values[0]);
    }
    let kept = reduce(times, values, tolerance * 0.5, Vec3::lerp, distance);
    let kept_times: Vec<f32> = kept.iter().map(|&i| times[i]).collect();
    let kept_values: Vec<Vec3> = kept.iter().map(|&i| values[i]).collect();
    let quantized = QuantizedVec3::new(&kept_values);
    let quantization_error = (0..kept_values.len())
        .map(|i| distance(quantized.get(i), kept_values[i]))
        .fold(0.0f32, f32::max);
    if quantization_error <= tolerance * 0.5 {
        Vec3Channel::Quantized {
            times: kept_times,
            values: quantized,
        }
    } else {
        Vec3Channel::Raw {
            times: kept_times,
            values: kept_values,
        }
    }
}

fn quat_channel(times: &[f32], values: &[Quat], tolerance: f32) -> QuatChannel {
    let distance = quat_angle;
    if values
        .iter()
        .all(|q| distance(*q, values[0]) <= tolerance * 0.1)
    {
        return QuatChannel::Constant(values[0]);
    }
    let kept = reduce(
        times,
        values,
        tolerance * 0.5,
        |a, b, t| a.slerp(b, t).normalize(),
        distance,
    );
    let kept_times: Vec<f32> = kept.iter().map(|&i| times[i]).collect();
    let kept_values: Vec<Quat> = kept.iter().map(|&i| values[i]).collect();
    let quantized: Vec<QuantizedQuat> =
        kept_values.iter().map(|q| QuantizedQuat::new(*q)).collect();
    let quantization_error = quantized
        .iter()
        .zip(&kept_values)
        .map(|(q, v)| distance(q.get(), *v))
        .fold(0.0f32, f32::max);
    if quantization_error <= tolerance * 0.5 {
        QuatChannel::Quantized {
            times: kept_times,
            values: quantized,
        }
    } else {
        QuatChannel::Raw {
            times: kept_times,
            values: kept_values,
        }
    }
}

#[cfg(test)]
mod tests;
