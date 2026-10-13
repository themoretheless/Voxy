//! Forward-and-backward reaching IK over an arbitrary-length direct chain.
//! Only rotations change; translations and scales are preserved exactly as in
//! the two-bone solver. Limits clamp each joint after it is aimed.

use std::sync::Arc;

use glam::{DQuat, DVec3, Quat, Vec3};

use super::limits::{JointLimit, apply_limit};
use super::{between, globals, parent_basis, perpendicular, rotate, sign_basis};
use crate::{AnimationError, Joint, Pose, Skeleton, rigs_match};

/// Direct parent→child chain from `root` to `tip`, bound to a rig layout.
#[derive(Clone, Debug)]
pub struct IkChain {
    rig: Arc<[Joint]>,
    joints: Vec<usize>,
    limits: Vec<Option<JointLimit>>,
}

impl IkChain {
    /// Walks parents from `tip` up to `root`.
    ///
    /// # Errors
    /// Requires `root` to be a proper ancestor of `tip` with at least one link.
    pub fn new(skeleton: &Skeleton, root: u16, tip: u16) -> Result<Self, AnimationError> {
        let joints = skeleton.joints();
        if usize::from(tip) >= joints.len() || root >= tip {
            return Err(AnimationError::InvalidIkChain);
        }
        let mut chain = vec![usize::from(tip)];
        let mut current = tip;
        while current != root {
            let Some(parent) = joints[usize::from(current)].parent else {
                return Err(AnimationError::InvalidIkChain);
            };
            chain.push(usize::from(parent));
            current = parent;
        }
        chain.reverse();
        let limits = vec![None; chain.len()];
        Ok(Self {
            rig: skeleton.joints.clone(),
            joints: chain,
            limits,
        })
    }

    /// Sets the limit of the chain joint at `index` (0 = root).
    ///
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

    #[must_use]
    pub fn limits(&self) -> &[Option<JointLimit>] {
        &self.limits
    }

    pub(crate) fn rig(&self) -> &Arc<[Joint]> {
        &self.rig
    }
}

/// Cheap position passes per realization; convergence is geometric so a
/// fixed count suffices before rotations are rebuilt.
const POSITION_PASSES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FabrikSettings {
    /// Outer passes that rebuild rotations (and apply limits).
    pub iterations: u8,
    /// Model-space tip distance that stops iterating.
    pub tolerance: f32,
    /// Local-rotation blend weight in `[0, 1]`.
    pub weight: f32,
}

impl Default for FabrikSettings {
    fn default() -> Self {
        Self {
            iterations: 8,
            tolerance: 1e-4,
            weight: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FabrikResult {
    pub tip_position: Vec3,
    pub iterations: u8,
    /// Remaining model-space distance from the tip to the target.
    pub residual: f32,
    /// Target lies beyond the chain's total length.
    pub reach_clamped: bool,
}

fn positions(pose: &Pose, chain: &IkChain) -> Result<Vec<DVec3>, AnimationError> {
    let world = globals(pose)?;
    Ok(chain
        .joints
        .iter()
        .map(|&j| world[j].w_axis.truncate())
        .collect())
}

impl Pose {
    /// Aims the chain so its tip reaches `target`, optionally orienting the
    /// tip. The source pose is never mutated on failure.
    ///
    /// # Errors
    /// Rejects foreign rigs, nonfinite targets/settings, collapsed links and
    /// nonuniform ancestor scale.
    pub fn solve_fabrik(
        &self,
        chain: &IkChain,
        target: Vec3,
        tip_rotation: Option<Quat>,
        settings: FabrikSettings,
    ) -> Result<(Self, FabrikResult), AnimationError> {
        if !rigs_match(&self.rig, &chain.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !target.is_finite()
            || !settings.tolerance.is_finite()
            || settings.tolerance < 0.0
            || !settings.weight.is_finite()
            || !(0.0..=1.0).contains(&settings.weight)
            || tip_rotation.is_some_and(|q| !q.is_finite() || !q.is_normalized())
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let rig = Skeleton {
            joints: chain.rig.clone(),
        };
        self.skin_matrices(&rig)?;
        let tip = *chain.joints.last().ok_or(AnimationError::InvalidIkChain)?;
        parent_basis(self, tip)?;
        let initial = positions(self, chain)?;
        let lengths: Vec<f64> = initial.windows(2).map(|w| (w[1] - w[0]).length()).collect();
        if lengths.iter().any(|l| !l.is_finite() || *l == 0.0) {
            return Err(AnimationError::DegenerateIkChain);
        }
        let total: f64 = lengths.iter().sum();
        let target64 = target.as_dvec3();
        let base = initial[0];
        let reach_clamped = (target64 - base).length() > total;
        let mut solved = self.clone();
        let mut iterations = 0u8;
        let mut residual = (initial[initial.len() - 1] - target64).length();
        while iterations < settings.iterations && residual > f64::from(settings.tolerance) {
            iterations += 1;
            let mut p = positions(&solved, chain)?;
            let n = p.len();
            if reach_clamped {
                // Straight line toward the target.
                let dir = (target64 - base).normalize_or_zero();
                let dir = if dir == DVec3::ZERO {
                    (p[n - 1] - base).normalize_or_zero()
                } else {
                    dir
                };
                for i in 1..n {
                    p[i] = p[i - 1] + dir * lengths[i - 1];
                }
            } else {
                for _ in 0..POSITION_PASSES {
                    // Forward: tip to target, pull parents.
                    p[n - 1] = target64;
                    for i in (0..n - 1).rev() {
                        let d = (p[i] - p[i + 1]).normalize_or_zero();
                        p[i] = p[i + 1] + d * lengths[i];
                    }
                    // Backward: re-anchor the base, push children.
                    p[0] = base;
                    for i in 1..n {
                        let d = (p[i] - p[i - 1]).normalize_or_zero();
                        p[i] = p[i - 1] + d * lengths[i - 1];
                    }
                    if (p[n - 1] - target64).length() <= f64::from(settings.tolerance) {
                        break;
                    }
                }
            }
            // Realize the desired positions as rotations, root to tip, with
            // limits applied as each joint is aimed.
            for i in 0..n - 1 {
                let current = positions(&solved, chain)?;
                let joint = chain.joints[i];
                let from = current[i + 1] - current[i];
                let to = p[i + 1] - current[i];
                if from.length() == 0.0 || to.length() == 0.0 {
                    continue;
                }
                let plane = from.cross(to);
                let plane = if plane.length() < 1e-14 {
                    perpendicular(from.normalize(), DVec3::Y)
                } else {
                    plane
                };
                rotate(&mut solved, joint, between(from, to, plane)?)?;
                apply_limit(&mut solved, joint, chain.limits[i].as_ref())?;
            }
            let current = positions(&solved, chain)?;
            let next = (current[n - 1] - target64).length();
            let stalled = next >= residual - 1e-15 && !reach_clamped;
            residual = next;
            if stalled {
                // Limits or geometry block further progress.
                break;
            }
        }
        if let Some(rotation) = tip_rotation {
            let parent = parent_basis(&solved, tip)?;
            let target = DQuat::from_array(rotation.to_array().map(f64::from)).normalize();
            let local =
                (parent.conjugate() * target * sign_basis(solved.local[tip].scale).conjugate())
                    .normalize();
            solved.set_joint_rotation(
                tip,
                Quat::from_array(local.to_array().map(|v| v as f32)).normalize(),
            )?;
            apply_limit(
                &mut solved,
                tip,
                chain.limits[chain.joints.len() - 1].as_ref(),
            )?;
        }
        for &joint in &chain.joints {
            let old = self.local[joint].rotation;
            let new = solved.local[joint].rotation;
            solved.set_joint_rotation(
                joint,
                if settings.weight == 0.0 {
                    old
                } else if settings.weight == 1.0 {
                    new
                } else {
                    old.slerp(new, settings.weight).normalize()
                },
            )?;
        }
        solved.skin_matrices(&rig)?;
        let final_positions = positions(&solved, chain)?;
        let tip_position = final_positions[final_positions.len() - 1];
        if !tip_position.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok((
            solved,
            FabrikResult {
                tip_position: tip_position.as_vec3(),
                iterations,
                residual: (tip_position - target64).length() as f32,
                reach_clamped,
            },
        ))
    }
}
