//! Distributed look-at: each chain joint rotates part of the way so that the
//! tip's local `forward` axis points at a model-space target.

use std::sync::Arc;

use glam::{DQuat, DVec3, Vec3};

use super::limits::{JointLimit, apply_limit};
use super::{between, globals, parent_basis, perpendicular, rotate};
use crate::{AnimationError, Joint, Pose, Skeleton, rigs_match};

/// Joints from base to head with per-joint shares of the total rotation.
#[derive(Clone, Debug)]
pub struct LookAtChain {
    rig: Arc<[Joint]>,
    joints: Vec<usize>,
    shares: Vec<f32>,
    limits: Vec<Option<JointLimit>>,
    /// Local axis of the last joint that should face the target.
    forward: Vec3,
    /// Model-space up used to resolve antiparallel aims.
    up: Vec3,
}

impl LookAtChain {
    /// `joints` must be ordered base→head and each a proper ancestor of the
    /// next (not necessarily direct); `shares` are nonnegative and sum to ≤ 1.
    ///
    /// # Errors
    /// Rejects an empty chain, bad ordering, nonfinite shares or axes.
    pub fn new(
        skeleton: &Skeleton,
        joints: &[u16],
        shares: &[f32],
        forward: Vec3,
        up: Vec3,
    ) -> Result<Self, AnimationError> {
        let rig = skeleton.joints();
        if joints.is_empty()
            || joints.len() != shares.len()
            || joints.iter().any(|&j| usize::from(j) >= rig.len())
            || !forward.is_finite()
            || !up.is_finite()
            || forward.length() < 1e-6
            || up.length() < 1e-6
        {
            return Err(AnimationError::InvalidIkChain);
        }
        for pair in joints.windows(2) {
            let mut current = rig[usize::from(pair[1])].parent;
            let mut found = false;
            while let Some(p) = current {
                if p == pair[0] {
                    found = true;
                    break;
                }
                current = rig[usize::from(p)].parent;
            }
            if !found {
                return Err(AnimationError::InvalidIkChain);
            }
        }
        let sum: f32 = shares.iter().sum();
        if shares.iter().any(|s| !s.is_finite() || *s < 0.0) || sum > 1.0 + 1e-6 {
            return Err(AnimationError::InvalidIkTarget);
        }
        Ok(Self {
            rig: skeleton.joints.clone(),
            joints: joints.iter().map(|&j| usize::from(j)).collect(),
            shares: shares.to_vec(),
            limits: vec![None; joints.len()],
            forward: forward.normalize(),
            up: up.normalize(),
        })
    }

    /// # Errors
    /// Rejects an index outside the chain.
    pub fn with_limit(mut self, index: usize, limit: JointLimit) -> Result<Self, AnimationError> {
        let slot = self
            .limits
            .get_mut(index)
            .ok_or(AnimationError::InvalidIkChain)?;
        *slot = Some(limit);
        Ok(self)
    }

    #[must_use]
    pub fn joints(&self) -> &[usize] {
        &self.joints
    }

    pub(crate) fn rig(&self) -> &Arc<[Joint]> {
        &self.rig
    }
}

/// Model-space forward direction of the chain head.
fn head_forward(pose: &Pose, chain: &LookAtChain) -> Result<(DVec3, DVec3), AnimationError> {
    let world = globals(pose)?;
    let head = world[chain.joints[chain.joints.len() - 1]];
    let position = head.w_axis.truncate();
    let forward = head.transform_vector3(chain.forward.as_dvec3());
    if forward.length() == 0.0 || !forward.is_finite() {
        return Err(AnimationError::DegenerateIkChain);
    }
    Ok((position, forward.normalize()))
}

impl Pose {
    /// Rotates the chain so the head faces `target`, each joint taking its
    /// share of the remaining angle, scaled by `weight`.
    ///
    /// # Errors
    /// Rejects foreign rigs, a nonfinite target or weight, a target at the
    /// head position, and nonuniform ancestor scale.
    pub fn solve_look_at(
        &self,
        chain: &LookAtChain,
        target: Vec3,
        weight: f32,
    ) -> Result<Self, AnimationError> {
        if !rigs_match(&self.rig, &chain.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !target.is_finite() || !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err(AnimationError::InvalidIkTarget);
        }
        let rig = Skeleton {
            joints: chain.rig.clone(),
        };
        self.skin_matrices(&rig)?;
        parent_basis(self, chain.joints[chain.joints.len() - 1])?;
        let mut solved = self.clone();
        if weight == 0.0 {
            return Ok(solved);
        }
        let target = target.as_dvec3();
        // Joint i takes its share of the rotation budget left after earlier
        // joints. The goal is recomputed per joint because rotating a parent
        // moves the head, which changes the direction to the target.
        let mut spent = 0.0f64;
        for (i, &joint) in chain.joints.iter().enumerate() {
            let share = f64::from(chain.shares[i]);
            let remaining = 1.0 - spent;
            if share <= 0.0 || remaining <= 1e-12 {
                continue;
            }
            let fraction = (share / remaining).min(1.0);
            spent += share;
            let (position, forward) = head_forward(&solved, chain)?;
            let to = target - position;
            if to.length() < 1e-9 {
                return Err(AnimationError::InvalidIkTarget);
            }
            let plane = forward.cross(to);
            let plane = if plane.length() < 1e-14 {
                perpendicular(forward, chain.up.as_dvec3())
            } else {
                plane
            };
            let full = between(forward, to, plane)?;
            let step = DQuat::IDENTITY
                .slerp(full, f64::from(weight) * fraction)
                .normalize();
            rotate(&mut solved, joint, step)?;
            apply_limit(&mut solved, joint, chain.limits[i].as_ref())?;
        }
        solved.skin_matrices(&rig)?;
        Ok(solved)
    }
}

/// Angle in radians between the head's forward axis and the target direction.
///
/// # Errors
/// Propagates pose evaluation errors.
pub fn look_at_error(
    pose: &Pose,
    chain: &LookAtChain,
    target: Vec3,
) -> Result<f64, AnimationError> {
    let (position, forward) = head_forward(pose, chain)?;
    let to = (target.as_dvec3() - position).normalize_or_zero();
    Ok(forward.dot(to).clamp(-1.0, 1.0).acos())
}
