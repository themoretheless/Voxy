//! Full-body IK: several effector chains sharing one pelvis. Each pass pulls
//! the pelvis toward the weighted mean reach deficit, then solves every chain
//! with FABRIK. Deterministic and bounded; unreachable effectors report their
//! residual instead of failing.

use std::sync::Arc;

use glam::{DVec3, Quat, Vec3};

use super::fabrik::{FabrikSettings, IkChain};
use super::globals;
use crate::{AnimationError, Joint, Pose, Skeleton, rigs_match};

/// Chains that share one pelvis joint; chains may not overlap.
#[derive(Clone, Debug)]
pub struct FullBodyRig {
    rig: Arc<[Joint]>,
    pelvis: usize,
    chains: Vec<IkChain>,
}

impl FullBodyRig {
    /// # Errors
    /// Rejects an unknown pelvis, chains for another rig, overlapping chains,
    /// or a chain containing the pelvis.
    pub fn new(
        skeleton: &Skeleton,
        pelvis: u16,
        chains: Vec<IkChain>,
    ) -> Result<Self, AnimationError> {
        if usize::from(pelvis) >= skeleton.joints().len() || chains.is_empty() {
            return Err(AnimationError::InvalidIkChain);
        }
        let mut seen = vec![false; skeleton.joints().len()];
        for chain in &chains {
            if !rigs_match(chain.rig(), &skeleton.joints) {
                return Err(AnimationError::SkeletonMismatch);
            }
            for &joint in chain.joints() {
                if joint == usize::from(pelvis) || seen[joint] {
                    return Err(AnimationError::InvalidIkChain);
                }
                seen[joint] = true;
            }
        }
        Ok(Self {
            rig: skeleton.joints.clone(),
            pelvis: usize::from(pelvis),
            chains,
        })
    }

    #[must_use]
    pub fn chains(&self) -> &[IkChain] {
        &self.chains
    }
}

/// Target for one chain of the rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effector {
    pub chain: usize,
    pub position: Vec3,
    pub rotation: Option<Quat>,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullBodySettings {
    pub iterations: u8,
    /// Fraction of the weighted mean reach deficit applied to the pelvis per pass.
    pub pelvis_pull: f32,
    /// Bound on the total pelvis displacement, in model units.
    pub max_pelvis_offset: f32,
    pub fabrik: FabrikSettings,
}

impl Default for FullBodySettings {
    fn default() -> Self {
        Self {
            iterations: 4,
            pelvis_pull: 1.0,
            max_pelvis_offset: 0.5,
            fabrik: FabrikSettings::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FullBodyResult {
    /// Residual per effector, in effector order.
    pub residuals: Vec<f32>,
    pub pelvis_offset: Vec3,
    pub iterations: u8,
}

impl Pose {
    /// Solves all effectors with a shared pelvis correction. The source pose is
    /// never mutated on failure.
    ///
    /// # Errors
    /// Rejects foreign rigs, unknown or duplicate chains, nonfinite targets or
    /// settings, and chain degeneracies reported by FABRIK.
    pub fn solve_full_body(
        &self,
        rig: &FullBodyRig,
        effectors: &[Effector],
        settings: FullBodySettings,
    ) -> Result<(Self, FullBodyResult), AnimationError> {
        if !rigs_match(&self.rig, &rig.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !settings.pelvis_pull.is_finite()
            || !(0.0..=1.0).contains(&settings.pelvis_pull)
            || !settings.max_pelvis_offset.is_finite()
            || settings.max_pelvis_offset < 0.0
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let mut used = vec![false; rig.chains.len()];
        for effector in effectors {
            if effector.chain >= rig.chains.len()
                || used[effector.chain]
                || !effector.position.is_finite()
                || !effector.weight.is_finite()
                || !(0.0..=1.0).contains(&effector.weight)
            {
                return Err(AnimationError::InvalidIkTarget);
            }
            used[effector.chain] = true;
        }
        let skeleton = Skeleton {
            joints: rig.rig.clone(),
        };
        self.skin_matrices(&skeleton)?;
        let mut solved = self.clone();
        let original_pelvis = self.local[rig.pelvis].translation;
        let mut iterations = 0u8;
        let mut residuals = vec![0.0f32; effectors.len()];
        let total_weight: f32 = effectors.iter().map(|e| e.weight).sum();
        while iterations < settings.iterations {
            iterations += 1;
            // Pull the pelvis by the weighted mean reach deficit.
            if settings.pelvis_pull > 0.0 && total_weight > 0.0 {
                let world = globals(&solved)?;
                let mut pull = DVec3::ZERO;
                for effector in effectors {
                    let chain = &rig.chains[effector.chain];
                    let joints = chain.joints();
                    let base = world[joints[0]].w_axis.truncate();
                    let total: f64 = joints
                        .windows(2)
                        .map(|w| {
                            (world[w[1]].w_axis.truncate() - world[w[0]].w_axis.truncate()).length()
                        })
                        .sum();
                    let offset = effector.position.as_dvec3() - base;
                    let distance = offset.length();
                    if distance > total && distance > 0.0 {
                        pull += offset / distance * (distance - total) * f64::from(effector.weight);
                    }
                }
                pull *= f64::from(settings.pelvis_pull) / f64::from(total_weight);
                if pull.length() > 0.0 {
                    let current = solved.local[rig.pelvis].translation;
                    let mut next = current + pull.as_vec3();
                    let offset = next - original_pelvis;
                    if offset.length() > settings.max_pelvis_offset {
                        next = original_pelvis + offset.normalize() * settings.max_pelvis_offset;
                    }
                    solved.set_joint_translation(rig.pelvis, next)?;
                }
            }
            let mut all_reached = true;
            for (i, effector) in effectors.iter().enumerate() {
                let chain = &rig.chains[effector.chain];
                let (next, result) = solved.solve_fabrik(
                    chain,
                    effector.position,
                    effector.rotation,
                    FabrikSettings {
                        weight: settings.fabrik.weight * effector.weight,
                        ..settings.fabrik
                    },
                )?;
                solved = next;
                residuals[i] = result.residual;
                if result.residual > settings.fabrik.tolerance && effector.weight > 0.0 {
                    all_reached = false;
                }
            }
            if all_reached {
                break;
            }
        }
        solved.skin_matrices(&skeleton)?;
        Ok((
            solved.clone(),
            FullBodyResult {
                residuals,
                pelvis_offset: solved.local[rig.pelvis].translation - original_pelvis,
                iterations,
            },
        ))
    }
}
