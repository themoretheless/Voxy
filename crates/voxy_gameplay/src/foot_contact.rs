//! Per-foot contact authority. Update returns a candidate; publication is caller-owned.
#[path = "foot_contact_checkpoint.rs"]
mod checkpoint;
use super::{
    PhysicsError, SupportAnchor, SupportContact, SupportProbe, SupportQueryBudget, SupportWorld,
};
pub use checkpoint::{FootContactCheckpoint, FootContactCheckpointError};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Distances are world units. Acquisition is narrower than release (hysteresis).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FootContactSettings {
    pub probe_lift: f64,
    pub probe_drop: f64,
    pub plant_distance: f64,
    pub release_distance: f64,
    pub min_up_dot: f64,
}
impl Default for FootContactSettings {
    fn default() -> Self {
        Self {
            probe_lift: 0.2,
            probe_drop: 0.3,
            plant_distance: 0.08,
            release_distance: 0.35,
            min_up_dot: 0.7,
        }
    }
}
impl FootContactSettings {
    /// Validates authored support distances and slope admission without queries.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        let distances = [
            self.probe_lift,
            self.probe_drop,
            self.plant_distance,
            self.release_distance,
        ];
        if distances
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1e6).contains(v))
            || self.probe_lift + self.probe_drop > 1e6
            || self.release_distance < self.plant_distance
            || !self.min_up_dot.is_finite()
            || !(0. ..=1.).contains(&self.min_up_dot)
        {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FootContactInput {
    /// Animated sole before IK, in the physically accepted actor world frame.
    pub sole: DVec3,
    pub up: DVec3,
    pub grounded: bool,
    /// Authored stance/contact intent. False begins a new swing and rearms planting.
    pub plant: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootContactStatus {
    Swing,
    Airborne,
    Searching,
    Planted,
    Released,
    /// Failed support/reach cannot replant until swing or a new airborne landing.
    AwaitingSwing,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FootContactState {
    anchor: Option<SupportAnchor>,
    awaiting_swing: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootContactCandidate {
    pub state: FootContactState,
    pub contact: Option<SupportContact>,
    pub status: FootContactStatus,
}
/// Admitted replacement contact and the exact world-space projection correction.
/// Publish the state and corresponding corrected pose together after IK succeeds.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootPivotTransfer {
    pub state: FootContactState,
    pub contact: SupportContact,
    /// Translation of the proposed pivot to the admitted surface point.
    /// This is not the total leg IK correction.
    pub correction: DVec3,
}
impl FootPivotTransfer {
    /// Stages a renderable corrected frame with its palette and replacement state.
    /// Root-motion/transition metadata is preserved; no playback clock is advanced.
    /// A clamped target returns `None` without consuming or modifying the source.
    /// # Errors
    /// Rejects invalid targets, foreign skeletons and IK/palette failures.
    pub fn solve_frame(
        &self,
        source: &voxy_animation::AnimatorFrame,
        skeleton: &voxy_animation::Skeleton,
        chain: &voxy_animation::TwoBoneChain,
        actor_world: voxy_animation::RootRigidTransform,
        ankle_rotation_world: glam::DQuat,
        pivot_in_ankle: DVec3,
        pole: glam::Vec3,
    ) -> Result<
        Option<(voxy_animation::AnimatorFrame, FootContactState)>,
        voxy_animation::AnimationError,
    > {
        let target = self.ik_target(actor_world, ankle_rotation_world, pivot_in_ankle, pole)?;
        let (frame, report) = source.clone().with_two_bone_ik(skeleton, chain, target)?;
        if report.reach_clamped {
            return Ok(None);
        }
        Ok(Some((frame, self.state)))
    }

    /// Stages the corrected pose and replacement state using the existing IK solver.
    /// An unreachable target returns `None`; errors and rejection preserve the
    /// source pose and this candidate. No scene/contact publication occurs here.
    /// # Errors
    /// Propagates invalid target, foreign rig and IK geometry errors.
    pub fn solve_pose(
        &self,
        source: &voxy_animation::Pose,
        chain: &voxy_animation::TwoBoneChain,
        actor_world: voxy_animation::RootRigidTransform,
        ankle_rotation_world: glam::DQuat,
        pivot_in_ankle: DVec3,
        pole: glam::Vec3,
    ) -> Result<Option<(voxy_animation::Pose, FootContactState)>, voxy_animation::AnimationError>
    {
        let target = self.ik_target(actor_world, ankle_rotation_world, pivot_in_ankle, pole)?;
        let (pose, report) = source.solve_two_bone(chain, target)?;
        if report.reach_clamped {
            return Ok(None);
        }
        Ok(Some((pose, self.state)))
    }

    /// Builds the full-strength model-space ankle target for the admitted pivot.
    /// Uses the resolved contact directly, including projection correction. The
    /// caller must reject a clamped IK result before publishing this state.
    /// Actor and ankle orientation are rigid; pivot units include caller-owned sizing.
    /// # Errors
    /// Rejects invalid transforms/pivots/poles or unrepresentable f32 IK targets.
    pub fn ik_target(
        &self,
        actor_world: voxy_animation::RootRigidTransform,
        ankle_rotation_world: glam::DQuat,
        pivot_in_ankle: DVec3,
        pole: glam::Vec3,
    ) -> Result<voxy_animation::TwoBoneTarget, voxy_animation::AnimationError> {
        use voxy_animation::AnimationError;
        actor_world.transform_point(DVec3::ZERO)?;
        if !ankle_rotation_world.is_finite()
            || !ankle_rotation_world.is_normalized()
            || !pivot_in_ankle.is_finite()
            || !pole.is_finite()
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let inverse_actor = actor_world.rotation.conjugate();
        let rotation = (inverse_actor * ankle_rotation_world).normalize();
        let position = (inverse_actor * (self.contact.position - actor_world.translation)
            - rotation * pivot_in_ankle)
            .as_vec3();
        let rotation = glam::Quat::from_array(rotation.to_array().map(|v| v as f32)).normalize();
        if !position.is_finite() || !rotation.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(voxy_animation::TwoBoneTarget {
            position,
            rotation: Some(rotation),
            pole,
            weight: 1.,
        })
    }
}
impl FootContactState {
    /// Releases an unreachable IK target; rearm on swing or airborne landing.
    #[must_use]
    pub fn release_until_swing(&self) -> Self {
        Self {
            anchor: None,
            awaiting_swing: true,
        }
    }

    #[must_use]
    pub fn anchor(&self) -> Option<SupportAnchor> {
        self.anchor
    }
    /// Admits an explicitly requested replacement pivot on the current support.
    /// Returns `None` on a miss, inactive stance, missing old support or another
    /// support. The caller keeps the previous state in that case. This does not
    /// detect heel/toe events, alter the pose or silently release/rearm contact.
    /// `max_projection_distance` bounds the world-space ankle translation needed
    /// to project the proposed pivot onto the support, independent of the broader
    /// acquisition tolerance. Zero requires exact coincidence. No pose smoothing
    /// is implied. Publish the correction `contact.position - input.sole` together
    /// with the matching geometric pivot change.
    /// # Errors
    /// Invalid input, foreign snapshots and budget exhaustion preserve self.
    /// Work already charged to the shared query budget is not rolled back.
    pub fn prepare_pivot_transfer(
        &self,
        world: &SupportWorld,
        settings: FootContactSettings,
        max_projection_distance: f64,
        input: FootContactInput,
        budget: &mut SupportQueryBudget,
    ) -> Result<Option<FootPivotTransfer>, PhysicsError> {
        if !max_projection_distance.is_finite() || !(0. ..=1e6).contains(&max_projection_distance) {
            return Err(PhysicsError::InvalidMotion);
        }
        let candidate = Self::default().prepare(world, settings, input, budget)?;
        let Some(old_anchor) = self.anchor else {
            return Ok(None);
        };
        if self.awaiting_swing {
            return Ok(None);
        }
        let Some(old_contact) = world.resolve(old_anchor, budget)? else {
            return Ok(None);
        };
        let Some(new_contact) = candidate.contact else {
            return Ok(None);
        };
        if new_contact.position.distance(input.sole) > max_projection_distance {
            return Ok(None);
        }
        if old_contact.anchor.owner() != new_contact.anchor.owner()
            || old_contact.normal.dot(input.up) < settings.min_up_dot
        {
            return Ok(None);
        }
        Ok(Some(FootPivotTransfer {
            state: candidate.state,
            contact: new_contact,
            correction: new_contact.position - input.sole,
        }))
    }

    /// # Errors
    /// Invalid settings/input, foreign supports or budget exhaustion preserve self.
    /// Query budget is consumed during preparation and is not restored on failure.
    pub fn prepare(
        &self,
        world: &SupportWorld,
        settings: FootContactSettings,
        input: FootContactInput,
        budget: &mut SupportQueryBudget,
    ) -> Result<FootContactCandidate, PhysicsError> {
        settings.validate()?;
        if !input.sole.is_finite()
            || input.sole.abs().max_element() > 1e6
            || !input.up.is_finite()
            || (input.up.length_squared() - 1.).abs() > 1e-12
        {
            return Err(PhysicsError::InvalidMotion);
        }
        let empty = |status, awaiting_swing| FootContactCandidate {
            state: Self {
                anchor: None,
                awaiting_swing,
            },
            contact: None,
            status,
        };
        if !input.grounded {
            return Ok(empty(FootContactStatus::Airborne, false));
        }
        if !input.plant {
            return Ok(empty(FootContactStatus::Swing, false));
        }
        if self.awaiting_swing {
            return Ok(empty(FootContactStatus::AwaitingSwing, true));
        }
        if let Some(anchor) = self.anchor {
            let Some(contact) = world.resolve(anchor, budget)? else {
                return Ok(empty(FootContactStatus::Released, true));
            };
            if contact.normal.dot(input.up) < settings.min_up_dot
                || contact.position.distance(input.sole) > settings.release_distance
            {
                return Ok(empty(FootContactStatus::Released, true));
            }
            return Ok(FootContactCandidate {
                state: *self,
                contact: Some(contact),
                status: FootContactStatus::Planted,
            });
        }
        let contact = world
            .probe(
                SupportProbe {
                    origin: input.sole + input.up * settings.probe_lift,
                    direction: -input.up,
                    up: input.up,
                    max_distance: settings.probe_lift + settings.probe_drop,
                    min_up_dot: settings.min_up_dot,
                },
                budget,
            )?
            .filter(|contact| contact.position.distance(input.sole) <= settings.plant_distance);
        Ok(FootContactCandidate {
            state: Self {
                anchor: contact.map(|contact| contact.anchor),
                awaiting_swing: false,
            },
            status: if contact.is_some() {
                FootContactStatus::Planted
            } else {
                FootContactStatus::Searching
            },
            contact,
        })
    }
}
#[cfg(test)]
mod tests;
