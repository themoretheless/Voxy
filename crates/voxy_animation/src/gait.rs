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
        })
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
        self.player.advance(velocity, dt)?;
        let mut pose = self.player.pose(skeleton)?;
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

#[cfg(test)]
mod tests;

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
