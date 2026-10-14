//! Procedural locomotion layer: a 2D blend space drives the cycle, ground
//! probes decide foot contacts, planted feet are locked in world space and
//! re-solved with exact two-bone IK, and acceleration leans the torso.
//! Everything is model space; the caller supplies the actor's displacement so
//! locked feet stay fixed while the actor moves.

use std::sync::Arc;

use glam::{DQuat, Mat4, Vec2, Vec3};

use crate::ik::rotate;
use crate::{
    AnimationError, BlendSpace1D, BlendSpace1DPlayer, BlendSpace2D, BlendSpace2DPlayer,
    PlantedLegsResult, Pose, Skeleton, TwoBoneChain, TwoBoneTarget,
};

/// Model-space ground query. Returns the hit point and surface normal.
pub trait GroundProbe {
    fn probe(&self, from: Vec3, down: Vec3) -> Option<(Vec3, Vec3)>;
}

/// Flat plane `up · x = height` for tests and simple worlds.
#[derive(Clone, Copy, Debug)]
pub struct FlatGround {
    pub up: Vec3,
    pub height: f32,
}

impl GroundProbe for FlatGround {
    fn probe(&self, from: Vec3, _down: Vec3) -> Option<(Vec3, Vec3)> {
        let along = from.dot(self.up);
        Some((from - self.up * (along - self.height), self.up))
    }
}

/// The cycle source: a 1D space keyed by forward speed, or a 2D space keyed
/// by (strafe, forward) velocity.
#[derive(Clone, Debug)]
pub enum GaitSpace {
    Forward(BlendSpace1DPlayer),
    Planar(BlendSpace2DPlayer),
}

impl GaitSpace {
    #[must_use]
    pub fn forward(space: Arc<BlendSpace1D>) -> Self {
        Self::Forward(BlendSpace1DPlayer::new(space, 0.0))
    }

    #[must_use]
    pub fn planar(space: Arc<BlendSpace2D>) -> Self {
        Self::Planar(BlendSpace2DPlayer::new(space, Vec2::ZERO))
    }

    fn phase(&self) -> f64 {
        match self {
            Self::Forward(p) => p.phase(),
            Self::Planar(p) => p.phase(),
        }
    }

    fn advance(&mut self, velocity: Vec2, dt: f32) -> Result<(), AnimationError> {
        match self {
            Self::Forward(p) => p.advance(velocity.y, dt),
            Self::Planar(p) => p.advance(velocity, dt),
        }
    }

    fn pose(&self, skeleton: &Skeleton) -> Result<Pose, AnimationError> {
        match self {
            Self::Forward(p) => p.pose(skeleton),
            Self::Planar(p) => p.pose(skeleton),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GaitFeet {
    pub chains: [TwoBoneChain; 2],
    /// Model-space pole offset from each hip (knee bend direction).
    pub poles: [Vec3; 2],
    pub up: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaitSettings {
    /// Ankle height above the ground at or below which a foot is planted.
    pub contact_height: f32,
    /// Extra clearance before a planted foot is released, for hysteresis.
    pub release_height: f32,
    pub max_pelvis_offset: f32,
    pub extension_margin: f32,
    /// Joint tilted by acceleration; `None` disables leaning.
    pub lean_joint: Option<u16>,
    /// Radians of lean per unit of planar acceleration.
    pub lean_per_accel: f32,
    pub max_lean: f32,
    /// Time constant for lean smoothing, seconds.
    pub lean_smoothing: f32,
}

impl Default for GaitSettings {
    fn default() -> Self {
        Self {
            contact_height: 0.03,
            release_height: 0.06,
            max_pelvis_offset: 0.3,
            extension_margin: 0.0,
            lean_joint: None,
            lean_per_accel: 0.02,
            max_lean: 0.25,
            lean_smoothing: 0.15,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GaitFrame {
    pub pose: Pose,
    pub skin_matrices: Vec<Mat4>,
    /// Whether each foot is planted after this tick.
    pub contacts: [bool; 2],
    pub legs: PlantedLegsResult,
    /// Applied lean angle in radians.
    pub lean: f32,
}

#[derive(Clone, Debug)]
pub struct GaitController {
    player: GaitSpace,
    feet: GaitFeet,
    settings: GaitSettings,
    /// Model-space locked ankle position and ground normal per foot.
    locked: [Option<(Vec3, Vec3)>; 2],
    previous_velocity: Option<Vec2>,
    lean: Vec2,
    stride_scale: f32,
    cadence_scale: f32,
}

/// Locomotion gait regime classified from planar speed (m/s).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GaitRegime {
    Idle,
    Walk,
    Jog,
    Run,
}

impl GaitRegime {
    /// Classifies planar speed (m/s) into `Idle`, `Walk`, `Jog`, or `Run`.
    #[must_use]
    pub fn from_speed(speed_m_s: f32) -> Self {
        if !speed_m_s.is_finite() || speed_m_s < 0.15 {
            Self::Idle
        } else if speed_m_s < 2.2 {
            Self::Walk
        } else if speed_m_s < 4.2 {
            Self::Jog
        } else {
            Self::Run
        }
    }
}

/// Summary of scaling an in-place locomotion clip to match a requested ground speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrideScaleReport {
    pub authored_speed_m_s: f32,
    pub target_speed_m_s: f32,
    pub stride_scale: f32,
    pub cadence_scale: f32,
}

impl GaitController {
    /// # Errors
    /// Rejects nonfinite settings, a non-normalized up axis or chains sharing joints.
    pub fn new(
        space: GaitSpace,
        feet: GaitFeet,
        settings: GaitSettings,
    ) -> Result<Self, AnimationError> {
        let finite = settings.contact_height.is_finite()
            && settings.release_height.is_finite()
            && settings.release_height >= settings.contact_height
            && settings.max_pelvis_offset.is_finite()
            && settings.max_pelvis_offset >= 0.0
            && settings.extension_margin.is_finite()
            && settings.lean_per_accel.is_finite()
            && settings.max_lean.is_finite()
            && settings.max_lean >= 0.0
            && settings.lean_smoothing.is_finite()
            && settings.lean_smoothing >= 0.0
            && feet.up.is_finite()
            && feet.up.is_normalized()
            && feet.poles.iter().all(|p| p.is_finite());
        if !finite {
            return Err(AnimationError::InvalidIkTarget);
        }
        Ok(Self {
            player: space,
            feet,
            settings,
            locked: [None, None],
            previous_velocity: None,
            lean: Vec2::ZERO,
            stride_scale: 1.0,
            cadence_scale: 1.0,
        })
    }

    /// Configures dynamic hip-swing stride scaling and playback cadence scaling.
    ///
    /// # Errors
    /// Rejects nonfinite or nonpositive scale factors.
    pub fn set_stride_and_cadence_scale(
        &mut self,
        stride_scale: f32,
        cadence_scale: f32,
    ) -> Result<(), AnimationError> {
        if !stride_scale.is_finite()
            || stride_scale <= 0.0
            || !cadence_scale.is_finite()
            || cadence_scale <= 0.0
        {
            return Err(AnimationError::InvalidBlendWeight);
        }
        self.stride_scale = stride_scale;
        self.cadence_scale = cadence_scale;
        Ok(())
    }

    #[must_use]
    pub fn stride_and_cadence_scale(&self) -> (f32, f32) {
        (self.stride_scale, self.cadence_scale)
    }

    /// Current gait regime (`Idle`, `Walk`, `Jog`, `Run`) from the latest planar velocity.
    #[must_use]
    pub fn regime(&self) -> GaitRegime {
        self.previous_velocity
            .map_or(GaitRegime::Idle, |v| GaitRegime::from_speed(v.length()))
    }

    #[must_use]
    pub fn phase(&self) -> f64 {
        self.player.phase()
    }

    #[must_use]
    pub fn feet(&self) -> &GaitFeet {
        &self.feet
    }

    #[must_use]
    pub fn locked(&self) -> [Option<Vec3>; 2] {
        [self.locked[0].map(|l| l.0), self.locked[1].map(|l| l.0)]
    }

    /// Advances the cycle by `dt` at planar `velocity` (blend-space units).
    /// `actor_delta` is the actor's model-space displacement this tick; locked
    /// feet are shifted by its negation so they stay fixed in the world.
    ///
    /// # Errors
    /// Rejects nonfinite input, and propagates blend/IK errors; the controller
    /// state is unchanged on failure.
    pub fn advance(
        &mut self,
        skeleton: &Skeleton,
        velocity: Vec2,
        actor_delta: Vec3,
        dt: f32,
        ground: &dyn GroundProbe,
    ) -> Result<GaitFrame, AnimationError> {
        if !velocity.is_finite() || !actor_delta.is_finite() || !dt.is_finite() || dt < 0.0 {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut candidate = self.clone();
        let frame = candidate.advance_candidate(skeleton, velocity, actor_delta, dt, ground)?;
        *self = candidate;
        Ok(frame)
    }

    fn advance_candidate(
        &mut self,
        skeleton: &Skeleton,
        velocity: Vec2,
        actor_delta: Vec3,
        dt: f32,
        ground: &dyn GroundProbe,
    ) -> Result<GaitFrame, AnimationError> {
        self.player.advance(velocity, dt * self.cadence_scale)?;
        let mut pose = self.player.pose(skeleton)?;
        if (self.stride_scale - 1.0).abs() > 1e-6 {
            scale_pose_leg_swing(&mut pose, skeleton, &self.feet, self.stride_scale)?;
        }
        let up = self.feet.up;

        // Lean: smoothed planar acceleration tilts the torso about up × accel.
        let mut lean_angle = 0.0f32;
        if let Some(joint) = self.settings.lean_joint {
            let accel = match self.previous_velocity {
                Some(previous) if dt > 0.0 => (velocity - previous) / dt,
                _ => Vec2::ZERO,
            };
            let alpha = if self.settings.lean_smoothing > 0.0 {
                (dt / self.settings.lean_smoothing).clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.lean += (accel - self.lean) * alpha;
            let magnitude = self.lean.length();
            if magnitude > 1e-6 {
                lean_angle = (self.settings.lean_per_accel * magnitude).min(self.settings.max_lean);
                // Planar x → model x, planar y → model forward (−z).
                let forward = Vec3::new(self.lean.x, 0.0, -self.lean.y) / magnitude;
                let axis = up.cross(forward);
                if axis.length() > 1e-6 {
                    let delta =
                        DQuat::from_axis_angle(axis.normalize().as_dvec3(), f64::from(lean_angle));
                    rotate(&mut pose, usize::from(joint), delta)?;
                }
            }
        }
        self.previous_velocity = Some(velocity);

        // Contacts from ankle clearance; planted feet are locked in world space.
        let mut global = Vec::new();
        let mut skin = Vec::new();
        pose.skin_matrices_into(skeleton, &mut global, &mut skin)?;
        let mut targets = [TwoBoneTarget {
            position: Vec3::ZERO,
            pole: Vec3::ZERO,
            rotation: None,
            weight: 1.0,
        }; 2];
        let mut contacts = [false; 2];
        for foot in 0..2 {
            let chain = &self.feet.chains[foot];
            let [hip, _, ankle] = chain.joint_indices();
            let hip_position = global[hip].w_axis.truncate();
            let ankle_position = global[ankle].w_axis.truncate();
            let pole = hip_position + self.feet.poles[foot];
            if let Some((point, normal)) = self.locked[foot] {
                let shifted = point - actor_delta;
                self.locked[foot] = Some((shifted, normal));
            }
            let hit = ground.probe(ankle_position + up * 1.0, -up);
            let clearance = hit.map(|(point, _)| (ankle_position - point).dot(up));
            let planted = match (self.locked[foot], clearance) {
                (Some(_), Some(c)) => c <= self.settings.release_height,
                (None, Some(c)) => c <= self.settings.contact_height,
                (_, None) => false,
            };
            if planted {
                let locked = match self.locked[foot] {
                    Some(locked) => locked,
                    None => {
                        let (point, normal) = hit.ok_or(AnimationError::UnreachableFootPlant)?;
                        // Lock at the authored clearance so the sole stays on the ground.
                        let clearance = clearance.unwrap_or(0.0).max(0.0);
                        (point + up * clearance, normal)
                    }
                };
                self.locked[foot] = Some(locked);
                targets[foot] = TwoBoneTarget {
                    position: locked.0,
                    pole,
                    rotation: None,
                    weight: 1.0,
                };
                contacts[foot] = true;
            } else {
                self.locked[foot] = None;
                targets[foot] = TwoBoneTarget {
                    position: ankle_position,
                    pole,
                    rotation: None,
                    weight: 1.0,
                };
            }
        }
        let (pose, legs) = pose.solve_planted_legs_with_extension_margin(
            &self.feet.chains,
            targets,
            up,
            self.settings.max_pelvis_offset,
            self.settings.extension_margin,
        )?;
        let skin_matrices = pose.skin_matrices(skeleton)?;
        Ok(GaitFrame {
            pose,
            skin_matrices,
            contacts,
            legs,
            lean: lean_angle,
        })
    }
}

fn scale_pose_leg_swing(
    pose: &mut Pose,
    skeleton: &Skeleton,
    feet: &GaitFeet,
    stride_scale: f32,
) -> Result<(), AnimationError> {
    if !stride_scale.is_finite() || stride_scale <= 0.0 {
        return Err(AnimationError::InvalidBlendWeight);
    }
    let joints = skeleton.joints();
    for chain in &feet.chains {
        let [hip, _, _] = chain.joint_indices();
        let bind_rot = joints[hip].bind_local.rotation;
        let current_rot = pose.local()[hip].rotation;
        let delta = bind_rot.inverse() * current_rot;
        let (axis, angle) = delta.to_axis_angle();
        let scaled_angle = (angle * stride_scale).clamp(-1.4, 1.4);
        let scaled = if axis.is_finite() && axis.length_squared() > 1e-12 {
            (bind_rot * glam::Quat::from_axis_angle(axis.normalize(), scaled_angle)).normalize()
        } else {
            bind_rot
        };
        pose.set_joint_rotation(hip, scaled)?;
    }
    Ok(())
}

/// Measures the natural ground speed (m/s) of an in-place locomotion clip by integrating
/// the backward travel of whichever ankle is lower (in stance) along `-forward`.
///
/// # Errors
/// Rejects nonfinite/unnormalized `forward`, `samples < 4`, or pose evaluation errors.
pub fn measure_authored_stride_speed(
    clip: &crate::AnimationClip,
    skeleton: &Skeleton,
    feet: &GaitFeet,
    forward: Vec3,
    samples: usize,
) -> Result<f32, AnimationError> {
    if !forward.is_finite() || !forward.is_normalized() || samples < 4 {
        return Err(AnimationError::InvalidIkTarget);
    }
    let duration = clip.duration();
    let mut travel = 0.0f32;
    let mut previous: Option<[Vec3; 2]> = None;
    for s in 0..=samples {
        let t = duration * (s as f32 / samples as f32);
        let pose = clip.try_sample(skeleton, t)?;
        let ankles = ankle_positions(&pose, skeleton, feet)?;
        if let Some(prev) = previous {
            let lower = if ankles[0].dot(feet.up) < ankles[1].dot(feet.up) {
                0
            } else {
                1
            };
            let delta = prev[lower] - ankles[lower];
            travel += delta.dot(forward).max(0.0);
        }
        previous = Some(ankles);
    }
    Ok(travel / duration)
}

/// Scales an in-place locomotion clip's hip stride excursion and cycle duration so its
/// natural stance-ankle ground speed matches `target_speed_m_s`.
///
/// # Errors
/// Rejects nonpositive `target_speed_m_s`, `max_stride_scale < 1.0`, or clips with zero authored speed.
pub fn scale_clip_stride_to_speed(
    clip: &crate::AnimationClip,
    skeleton: &Skeleton,
    feet: &GaitFeet,
    forward: Vec3,
    target_speed_m_s: f32,
    max_stride_scale: f32,
    samples: usize,
) -> Result<(crate::AnimationClip, StrideScaleReport), AnimationError> {
    use crate::{JointTrack, QuatKey, Vec3Key};

    if !target_speed_m_s.is_finite()
        || target_speed_m_s <= 0.0
        || !max_stride_scale.is_finite()
        || max_stride_scale < 1.0
        || samples < 4
    {
        return Err(AnimationError::InvalidIkTarget);
    }
    let authored_speed_m_s = measure_authored_stride_speed(clip, skeleton, feet, forward, samples)?;
    if !authored_speed_m_s.is_finite() || authored_speed_m_s <= 1e-4 {
        return Err(AnimationError::InvalidClipHeader);
    }
    let ratio = target_speed_m_s / authored_speed_m_s;
    let stride_scale = ratio.sqrt().clamp(1.0 / max_stride_scale, max_stride_scale);
    let cadence_scale = ratio / stride_scale;
    let new_duration = clip.duration() / cadence_scale;
    if !new_duration.is_finite() || new_duration <= 1e-4 {
        return Err(AnimationError::InvalidClipHeader);
    }

    let mut tracks = vec![JointTrack::default(); skeleton.joints().len()];
    for s in 0..=samples {
        let alpha = s as f32 / samples as f32;
        let src_time = alpha * clip.duration();
        let dst_time = alpha * new_duration;
        let mut pose = clip.try_sample(skeleton, src_time)?;
        scale_pose_leg_swing(&mut pose, skeleton, feet, stride_scale)?;
        for (track, local) in tracks.iter_mut().zip(pose.local()) {
            track.translations.push(Vec3Key {
                time: dst_time,
                value: local.translation,
            });
            track.rotations.push(QuatKey {
                time: dst_time,
                value: local.rotation,
            });
            track.scales.push(Vec3Key {
                time: dst_time,
                value: local.scale,
            });
        }
    }
    let scaled_clip = crate::AnimationClip::new(
        format!("{}_scaled", clip.name()),
        new_duration,
        clip.playback,
        tracks,
        skeleton,
    )?;
    Ok((
        scaled_clip,
        StrideScaleReport {
            authored_speed_m_s,
            target_speed_m_s,
            stride_scale,
            cadence_scale,
        },
    ))
}

#[cfg(test)]
mod tests;

/// Model/world-space kinematic state of one proximal thigh segment (`hip -> knee`)
/// used to drive medial-thigh soft-tissue and scrotal/testicular contact during gait.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThighCapsuleState {
    pub hip_position_m: [f64; 3],
    pub knee_position_m: [f64; 3],
    pub hip_velocity_m_s: [f64; 3],
    pub knee_velocity_m_s: [f64; 3],
    pub radius_m: f64,
}

/// Coupled 6-DOF pelvis and bilateral thigh kinematic frame extracted from two consecutive
/// skeletal locomotion poses (`previous_pose` -> `current_pose` over `dt_s`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledPelvisThighFrame {
    pub pelvis_position_m: [f64; 3],
    pub pelvis_rotation_xyzw: [f64; 4],
    pub pelvis_velocity_m_s: [f64; 3],
    pub pelvis_acceleration_m_s2: [f64; 3],
    pub thighs: [ThighCapsuleState; 2],
    pub phase: f64,
    pub regime: GaitRegime,
}

/// Extracts a [`CoupledPelvisThighFrame`] from `previous_pose` and `current_pose` separated by `dt_s`.
///
/// # Errors
/// Rejects out-of-range `pelvis_joint`, nonfinite/nonpositive `dt_s` or `thigh_radius_m`,
/// nonfinite `previous_pelvis_velocity_m_s`, or invalid pose evaluation.
#[allow(clippy::too_many_arguments)]
pub fn extract_pelvis_thigh_frame(
    skeleton: &Skeleton,
    feet: &GaitFeet,
    previous_pose: &Pose,
    current_pose: &Pose,
    pelvis_joint: usize,
    previous_pelvis_velocity_m_s: [f64; 3],
    thigh_radius_m: f64,
    dt_s: f64,
    phase: f64,
    regime: GaitRegime,
) -> Result<CoupledPelvisThighFrame, AnimationError> {
    if pelvis_joint >= skeleton.joints().len()
        || !thigh_radius_m.is_finite()
        || thigh_radius_m <= 0.0
        || !dt_s.is_finite()
        || dt_s <= 0.0
        || !phase.is_finite()
        || previous_pelvis_velocity_m_s.iter().any(|v| !v.is_finite())
    {
        return Err(AnimationError::InvalidIkTarget);
    }
    let mut prev_global = Vec::new();
    let mut prev_skin = Vec::new();
    previous_pose.skin_matrices_into(skeleton, &mut prev_global, &mut prev_skin)?;

    let mut curr_global = Vec::new();
    let mut curr_skin = Vec::new();
    current_pose.skin_matrices_into(skeleton, &mut curr_global, &mut curr_skin)?;

    let to_f64_3 = |v: Vec3| [f64::from(v.x), f64::from(v.y), f64::from(v.z)];
    let prev_pelvis_pos = to_f64_3(prev_global[pelvis_joint].w_axis.truncate());
    let curr_pelvis_pos = to_f64_3(curr_global[pelvis_joint].w_axis.truncate());
    let pelvis_velocity_m_s =
        std::array::from_fn(|k| (curr_pelvis_pos[k] - prev_pelvis_pos[k]) / dt_s);
    let pelvis_acceleration_m_s2 = std::array::from_fn(|k| {
        (pelvis_velocity_m_s[k] - previous_pelvis_velocity_m_s[k]) / dt_s
    });
    let (_, rot, _) = curr_global[pelvis_joint].to_scale_rotation_translation();
    let rot = rot.normalize();
    let pelvis_rotation_xyzw = [
        f64::from(rot.x),
        f64::from(rot.y),
        f64::from(rot.z),
        f64::from(rot.w),
    ];

    let mut thighs = [ThighCapsuleState {
        hip_position_m: [0.0; 3],
        knee_position_m: [0.0; 3],
        hip_velocity_m_s: [0.0; 3],
        knee_velocity_m_s: [0.0; 3],
        radius_m: thigh_radius_m,
    }; 2];
    for side in 0..2 {
        let [hip, knee, _] = feet.chains[side].joint_indices();
        let prev_hip = to_f64_3(prev_global[hip].w_axis.truncate());
        let curr_hip = to_f64_3(curr_global[hip].w_axis.truncate());
        let prev_knee = to_f64_3(prev_global[knee].w_axis.truncate());
        let curr_knee = to_f64_3(curr_global[knee].w_axis.truncate());
        thighs[side] = ThighCapsuleState {
            hip_position_m: curr_hip,
            knee_position_m: curr_knee,
            hip_velocity_m_s: std::array::from_fn(|k| (curr_hip[k] - prev_hip[k]) / dt_s),
            knee_velocity_m_s: std::array::from_fn(|k| (curr_knee[k] - prev_knee[k]) / dt_s),
            radius_m: thigh_radius_m,
        };
    }

    Ok(CoupledPelvisThighFrame {
        pelvis_position_m: curr_pelvis_pos,
        pelvis_rotation_xyzw,
        pelvis_velocity_m_s,
        pelvis_acceleration_m_s2,
        thighs,
        phase,
        regime,
    })
}

/// Model-space ankle positions of both chains, for diagnostics and tests.
///
/// # Errors
/// Propagates palette evaluation errors.
pub fn ankle_positions(
    pose: &Pose,
    skeleton: &Skeleton,
    feet: &GaitFeet,
) -> Result<[Vec3; 2], AnimationError> {
    let mut global = Vec::new();
    let mut skin = Vec::new();
    pose.skin_matrices_into(skeleton, &mut global, &mut skin)?;
    let at = |chain: &TwoBoneChain| global[chain.joint_indices()[2]].w_axis.truncate();
    Ok([at(&feet.chains[0]), at(&feet.chains[1])])
}
