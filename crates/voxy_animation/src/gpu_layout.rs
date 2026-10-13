//! Plain-data layouts for GPU pose evaluation and the CPU reference that a
//! GPU implementation must match. Clips are uniformly resampled into
//! `samples × joints` TRS rows; a job blends two clips at a shared phase and
//! produces one skin palette per instance.
//!
//! All arithmetic here is `f32` in the same order a shader would use, so a
//! GPU palette can be compared against [`cpu_reference_palette`] tightly.

use glam::{Mat4, Quat, Vec3};

use crate::{AnimationClip, AnimationError, Skeleton, SoaSkeleton};

/// Marker for root joints in [`GpuSkeletonLayout::parents`].
pub const GPU_NO_PARENT: u32 = u32::MAX;

/// One resampled joint transform: 12 floats (translation, rotation, scale with padding).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct GpuJointKey {
    pub translation: [f32; 3],
    pub _pad0: f32,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub _pad1: f32,
}

impl GpuJointKey {
    pub const FLOATS: usize = 12;

    #[must_use]
    pub fn to_floats(self) -> [f32; Self::FLOATS] {
        let [tx, ty, tz] = self.translation;
        let [rx, ry, rz, rw] = self.rotation;
        let [sx, sy, sz] = self.scale;
        [tx, ty, tz, 0.0, rx, ry, rz, rw, sx, sy, sz, 0.0]
    }
}

/// Uniformly resampled clip; row `s` holds joints `s * joint_count ..`.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuClipTable {
    pub joint_count: u32,
    pub sample_count: u32,
    pub duration: f32,
    pub looping: bool,
    pub keys: Vec<GpuJointKey>,
}

impl GpuClipTable {
    /// Resamples `clip` at `rate` Hz (at least two samples).
    ///
    /// # Errors
    /// Rejects a nonpositive rate or a clip incompatible with `skeleton`.
    pub fn bake(
        clip: &AnimationClip,
        skeleton: &Skeleton,
        rate: f32,
    ) -> Result<Self, AnimationError> {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(AnimationError::InvalidClipHeader);
        }
        if !clip.is_compatible_with(skeleton) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let duration = clip.duration;
        let sample_count = ((duration * rate).ceil() as usize).max(1) + 1;
        let joint_count = skeleton.joints().len();
        let mut keys = Vec::with_capacity(sample_count * joint_count);
        for s in 0..sample_count {
            let phase = s as f64 / (sample_count - 1) as f64;
            let pose = clip.try_sample_phase(skeleton, phase)?;
            keys.extend(pose.local().iter().map(|t| GpuJointKey {
                translation: t.translation.to_array(),
                _pad0: 0.0,
                rotation: t.rotation.to_array(),
                scale: t.scale.to_array(),
                _pad1: 0.0,
            }));
        }
        Ok(Self {
            joint_count: joint_count as u32,
            sample_count: sample_count as u32,
            duration,
            looping: matches!(clip.playback, crate::Playback::Loop),
            keys,
        })
    }

    /// Flat float buffer, 12 floats per key.
    #[must_use]
    pub fn to_floats(&self) -> Vec<f32> {
        self.keys.iter().flat_map(|k| k.to_floats()).collect()
    }

    /// Header words: `[key_offset, sample_count, joint_count, looping]` with
    /// `duration` as the fifth word's bit pattern.
    #[must_use]
    pub fn header(&self, key_offset: u32) -> [u32; 8] {
        [
            key_offset,
            self.sample_count,
            self.joint_count,
            u32::from(self.looping),
            self.duration.to_bits(),
            0,
            0,
            0,
        ]
    }

    fn sample(&self, phase: f32, joint: usize, out: &mut (Vec3, Quat, Vec3)) {
        let phase = phase.clamp(0.0, 1.0);
        let scaled = phase * (self.sample_count - 1) as f32;
        let lower = (scaled.floor() as u32).min(self.sample_count - 1);
        let upper = (lower + 1).min(self.sample_count - 1);
        let t = scaled - lower as f32;
        let a = self.keys[(lower * self.joint_count) as usize + joint];
        let b = self.keys[(upper * self.joint_count) as usize + joint];
        let ta = Vec3::from_array(a.translation);
        let tb = Vec3::from_array(b.translation);
        let sa = Vec3::from_array(a.scale);
        let sb = Vec3::from_array(b.scale);
        let ra = Quat::from_array(a.rotation);
        let rb = Quat::from_array(b.rotation);
        *out = (ta + (tb - ta) * t, nlerp(ra, rb, t), sa + (sb - sa) * t);
    }
}

/// Shortest-path normalized linear interpolation, as in the shader.
#[must_use]
pub fn nlerp(a: Quat, b: Quat, t: f32) -> Quat {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    let x = a.x + (b.x - a.x) * t;
    let y = a.y + (b.y - a.y) * t;
    let z = a.z + (b.z - a.z) * t;
    let w = a.w + (b.w - a.w) * t;
    let len = (x * x + y * y + z * z + w * w).sqrt();
    Quat::from_xyzw(x / len, y / len, z / len, w / len)
}

/// Skeleton topology for the shader: parent indices and inverse binds.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSkeletonLayout {
    pub parents: Vec<u32>,
    pub inverse_binds: Vec<Mat4>,
}

impl GpuSkeletonLayout {
    #[must_use]
    pub fn new(skeleton: &SoaSkeleton) -> Self {
        Self {
            parents: skeleton
                .parents()
                .iter()
                .map(|&p| {
                    if p == crate::SOA_NO_PARENT {
                        GPU_NO_PARENT
                    } else {
                        u32::from(p)
                    }
                })
                .collect(),
            inverse_binds: skeleton.inverse_binds().to_vec(),
        }
    }
}

/// One instance's blend: clip `a` toward clip `b` by `weight` at `phase`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct GpuBlendJob {
    pub instance: u32,
    pub clip_a: u32,
    pub clip_b: u32,
    pub phase: f32,
    pub weight: f32,
    pub _pad: [u32; 3],
}

impl GpuBlendJob {
    #[must_use]
    pub fn new(instance: u32, clip_a: u32, clip_b: u32, phase: f32, weight: f32) -> Self {
        Self {
            instance,
            clip_a,
            clip_b,
            phase,
            weight,
            _pad: [0; 3],
        }
    }

    /// Eight words: three indices, phase and weight bit patterns, padding.
    #[must_use]
    pub fn to_words(self) -> [u32; 8] {
        [
            self.instance,
            self.clip_a,
            self.clip_b,
            self.phase.to_bits(),
            self.weight.to_bits(),
            0,
            0,
            0,
        ]
    }
}

/// CPU evaluation of one job, in the shader's arithmetic order.
///
/// # Errors
/// Rejects out-of-range clip indices, mismatched joint counts or a
/// nonfinite phase/weight.
pub fn cpu_reference_palette(
    layout: &GpuSkeletonLayout,
    tables: &[GpuClipTable],
    job: &GpuBlendJob,
) -> Result<Vec<Mat4>, AnimationError> {
    let a = tables
        .get(job.clip_a as usize)
        .ok_or(AnimationError::InvalidClipHeader)?;
    let b = tables
        .get(job.clip_b as usize)
        .ok_or(AnimationError::InvalidClipHeader)?;
    let joint_count = layout.parents.len();
    if a.joint_count as usize != joint_count || b.joint_count as usize != joint_count {
        return Err(AnimationError::SkeletonMismatch);
    }
    if !job.phase.is_finite() || !job.weight.is_finite() {
        return Err(AnimationError::InvalidBlendWeight);
    }
    let weight = job.weight.clamp(0.0, 1.0);
    let mut globals: Vec<Mat4> = Vec::with_capacity(joint_count);
    let mut palette = Vec::with_capacity(joint_count);
    let mut sa = (Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
    let mut sb = sa;
    for joint in 0..joint_count {
        a.sample(job.phase, joint, &mut sa);
        b.sample(job.phase, joint, &mut sb);
        let translation = sa.0 + (sb.0 - sa.0) * weight;
        let rotation = nlerp(sa.1, sb.1, weight);
        let scale = sa.2 + (sb.2 - sa.2) * weight;
        let local = Mat4::from_scale_rotation_translation(scale, rotation, translation);
        let parent = layout.parents[joint];
        let global = if parent == GPU_NO_PARENT {
            local
        } else {
            globals[parent as usize] * local
        };
        palette.push(global * layout.inverse_binds[joint]);
        globals.push(global);
    }
    Ok(palette)
}

#[cfg(test)]
mod tests;
