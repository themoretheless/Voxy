//! Support-relative contact anchors. Contact acquisition/release belongs to the
//! locomotion owner; this module neither queries physics nor owns scene objects.
use crate::{AnimationError, RootRigidTransform, TwoBoneTarget};
use glam::{DQuat, DVec3, Quat, Vec3};

/// A rigid foot contact relative to a caller-owned support.
/// Default targets keep the captured ankle orientation; explicit rotation targets
/// roll around the captured local pivot. Release when the gait lifts the foot.
/// Resolve the same support identity each tick, including on moving platforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootPlant {
    contact_in_support: RootRigidTransform,
    pivot_in_ankle: DVec3,
}
impl FootPlant {
    /// Captures an admitted ankle contact without retaining a scene reference.
    /// Both transforms are in the same world frame, in double precision.
    /// # Errors
    /// Rejects invalid transforms and relative-coordinate overflow.
    pub fn capture(
        ankle_world: RootRigidTransform,
        support_world: RootRigidTransform,
    ) -> Result<Self, AnimationError> {
        Self::capture_pivot(ankle_world, DVec3::ZERO, support_world)
    }

    /// Captures a rigid heel/toe pivot expressed in the ankle's local frame.
    /// The support point stays fixed while `target_with_rotation` rolls the foot.
    /// Pivot units include any caller-owned foot sizing; transforms are rigid.
    /// # Errors
    /// Rejects invalid transforms, a nonfinite pivot or relative-coordinate overflow.
    pub fn capture_pivot(
        ankle_world: RootRigidTransform,
        pivot_in_ankle: DVec3,
        support_world: RootRigidTransform,
    ) -> Result<Self, AnimationError> {
        ankle_world.transform_point(DVec3::ZERO)?;
        support_world.transform_point(DVec3::ZERO)?;
        if !pivot_in_ankle.is_finite() {
            return Err(AnimationError::InvalidIkTarget);
        }
        let inverse_rotation = support_world.rotation.conjugate();
        let relative_rotation = (inverse_rotation * ankle_world.rotation).normalize();
        // Resolve the pivot in relative coordinates before adding world origins.
        let contact_in_support = RootRigidTransform {
            translation: inverse_rotation * (ankle_world.translation - support_world.translation)
                + relative_rotation * pivot_in_ankle,
            rotation: relative_rotation,
        };
        contact_in_support.transform_point(DVec3::ZERO)?;
        Ok(Self {
            contact_in_support,
            pivot_in_ankle,
        })
    }

    /// Transfers a heel/toe pivot without changing the current solved ankle pose.
    /// The caller must admit the new support point before publishing this candidate:
    /// this geometric operation does not establish surface contact or choose gait events.
    /// The existing support identity must remain unchanged. Subsequent roll locks the
    /// new pivot; no release/reacquisition or second contact state machine is created.
    /// # Errors
    /// Rejects invalid support/orientation, nonfinite pivots and coordinate overflow.
    pub fn with_pivot_preserving_ankle(
        &self,
        support_world: RootRigidTransform,
        ankle_rotation_world: DQuat,
        new_pivot_in_ankle: DVec3,
    ) -> Result<Self, AnimationError> {
        support_world.transform_point(DVec3::ZERO)?;
        if !ankle_rotation_world.is_finite()
            || !ankle_rotation_world.is_normalized()
            || !new_pivot_in_ankle.is_finite()
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let rotation = (support_world.rotation.conjugate() * ankle_rotation_world).normalize();
        let translation = self.contact_in_support.translation
            + rotation * (new_pivot_in_ankle - self.pivot_in_ankle);
        let contact_in_support = RootRigidTransform {
            translation,
            rotation,
        };
        contact_in_support.transform_point(DVec3::ZERO)?;
        Ok(Self {
            contact_in_support,
            pivot_in_ankle: new_pivot_in_ankle,
        })
    }

    /// Resolves the planted ankle into the current actor's model coordinates.
    /// `pole` is already in model coordinates. Actor/support transforms are
    /// rigid; any authored bone scale remains the rig IK solver's responsibility.
    /// Use exact IK for this target. A weight below 1 intentionally blends away
    /// from the contact and cannot guarantee a locked endpoint.
    /// # Errors
    /// Rejects invalid transforms, pole, weight and unrepresentable model targets.
    pub fn target(
        &self,
        support_world: RootRigidTransform,
        actor_world: RootRigidTransform,
        pole: Vec3,
        weight: f32,
    ) -> Result<TwoBoneTarget, AnimationError> {
        self.target_with_rotation(
            support_world,
            actor_world,
            (support_world.rotation * self.contact_in_support.rotation).normalize(),
            pole,
            weight,
        )
    }

    /// Keeps the captured support pivot fixed while following an authored ankle
    /// orientation in world coordinates. Heel strike/toe-off can rotate naturally;
    /// the locomotion owner chooses pivots and contact transitions explicitly.
    /// # Errors
    /// Rejects invalid rigid transforms/orientation, pole, weight and target overflow.
    pub fn target_with_rotation(
        &self,
        support_world: RootRigidTransform,
        actor_world: RootRigidTransform,
        ankle_rotation_world: DQuat,
        pole: Vec3,
        weight: f32,
    ) -> Result<TwoBoneTarget, AnimationError> {
        support_world.transform_point(DVec3::ZERO)?;
        actor_world.transform_point(DVec3::ZERO)?;
        if !pole.is_finite() || !weight.is_finite() || !(0. ..=1.).contains(&weight) {
            return Err(AnimationError::InvalidIkTarget);
        }
        if !ankle_rotation_world.is_finite() || !ankle_rotation_world.is_normalized() {
            return Err(AnimationError::InvalidIkTarget);
        }
        let inverse_actor = actor_world.rotation.conjugate();
        let rotation = (inverse_actor * ankle_rotation_world).normalize();
        let position = (inverse_actor
            * ((support_world.translation - actor_world.translation)
                + support_world.rotation * self.contact_in_support.translation)
            - rotation * self.pivot_in_ankle)
            .as_vec3();
        let rotation = Quat::from_array(rotation.to_array().map(|v| v as f32)).normalize();
        if !position.is_finite() || !rotation.is_finite() || !rotation.is_normalized() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(TwoBoneTarget {
            position,
            pole,
            rotation: Some(rotation),
            weight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_contact_survives_actor_motion_and_large_shared_origin() {
        let origin = DVec3::new(1e10, -1e10, 1e10);
        let support = RootRigidTransform {
            translation: origin,
            rotation: DQuat::from_rotation_y(0.7),
        };
        let ankle = support
            .compose(RootRigidTransform {
                translation: DVec3::new(0.25, 0.1, -0.4),
                rotation: DQuat::from_rotation_x(0.3),
            })
            .unwrap();
        let plant = FootPlant::capture(ankle, support).unwrap();
        for step in 0..100 {
            let actor = RootRigidTransform {
                translation: origin + DVec3::new(step as f64 * 0.01, 0.2, 0.3),
                rotation: DQuat::from_rotation_y(step as f64 * 0.01),
            };
            let target = plant.target(support, actor, Vec3::Z, 1.).unwrap();
            let reconstructed = actor.transform_point(target.position.as_dvec3()).unwrap();
            assert!(reconstructed.distance(ankle.translation) < 4e-6);
            let orientation = actor.rotation * target.rotation.unwrap().as_dquat();
            assert!(orientation.dot(ankle.rotation).abs() > 1. - 1e-7);
        }
    }

    #[test]
    fn moving_support_transports_anchor_and_common_world_motion_cancels() {
        let support = RootRigidTransform {
            translation: DVec3::new(0.3, 0.2, -0.1),
            rotation: DQuat::from_rotation_y(0.4),
        };
        let local = RootRigidTransform {
            translation: DVec3::new(0.1, 0.2, 0.3),
            rotation: DQuat::from_rotation_x(0.2),
        };
        let plant = FootPlant::capture(support.compose(local).unwrap(), support).unwrap();
        let actor = RootRigidTransform::IDENTITY;
        let world_move = RootRigidTransform {
            translation: DVec3::new(2., 3., 4.),
            rotation: DQuat::from_rotation_z(0.6),
        };
        let a = plant.target(support, actor, Vec3::Z, 1.).unwrap();
        let b = plant
            .target(
                world_move.compose(support).unwrap(),
                world_move.compose(actor).unwrap(),
                Vec3::Z,
                1.,
            )
            .unwrap();
        assert!(a.position.abs_diff_eq(b.position, 1e-6));
        assert!(a.rotation.unwrap().dot(b.rotation.unwrap()).abs() > 1. - 1e-6);
        let moved = world_move.compose(support).unwrap();
        let target = plant.target(moved, actor, Vec3::Z, 1.).unwrap();
        assert!(
            target
                .position
                .as_dvec3()
                .distance(moved.transform_point(local.translation).unwrap())
                < 1e-6
        );
    }

    #[test]
    fn invalid_inputs_reject_without_changing_anchor() {
        let identity = RootRigidTransform::IDENTITY;
        let plant = FootPlant::capture(identity, identity).unwrap();
        let saved = plant;
        let invalid = RootRigidTransform {
            rotation: DQuat::from_xyzw(0., 0., 0., 2.),
            ..identity
        };
        assert!(FootPlant::capture(invalid, identity).is_err());
        assert!(plant.target(invalid, identity, Vec3::Z, 1.).is_err());
        assert!(plant.target(identity, invalid, Vec3::Z, 1.).is_err());
        assert!(
            plant
                .target(identity, identity, Vec3::splat(f32::NAN), 1.)
                .is_err()
        );
        for weight in [f32::NAN, -0.1, 1.1] {
            assert!(plant.target(identity, identity, Vec3::Z, weight).is_err());
        }
        let distant = RootRigidTransform {
            translation: DVec3::splat(1e100),
            ..identity
        };
        assert!(plant.target(distant, identity, Vec3::Z, 1.).is_err());
        assert_eq!(plant, saved);
    }
}

#[cfg(test)]
mod pivot_tests {
    use super::*;
    #[test]
    fn heel_and_toe_pivots_remain_fixed_while_ankle_rolls_on_moving_support() {
        for pivot in [DVec3::new(0., -0.08, -0.04), DVec3::new(0., -0.08, 0.14)] {
            let support = RootRigidTransform {
                translation: DVec3::new(2., 3., 4.),
                rotation: DQuat::from_rotation_y(0.4),
            };
            let ankle = support
                .compose(RootRigidTransform {
                    translation: DVec3::new(0.2, 0.08, 0.),
                    rotation: DQuat::IDENTITY,
                })
                .unwrap();
            let plant = FootPlant::capture_pivot(ankle, pivot, support).unwrap();
            let contact_local = support
                .inverse()
                .unwrap()
                .transform_point(ankle.transform_point(pivot).unwrap())
                .unwrap();
            for step in 0..60 {
                let current_support = RootRigidTransform {
                    translation: support.translation + DVec3::X * step as f64 * 0.01,
                    rotation: DQuat::from_rotation_y(0.4 + step as f64 * 0.01),
                };
                let actor = RootRigidTransform {
                    translation: DVec3::new(1., 0., 1.),
                    rotation: DQuat::from_rotation_y(-0.3),
                };
                let roll = current_support.rotation * DQuat::from_rotation_x(step as f64 * 0.015);
                let target = plant
                    .target_with_rotation(current_support, actor, roll, Vec3::Z, 1.)
                    .unwrap();
                let resolved = RootRigidTransform {
                    translation: actor.transform_point(target.position.as_dvec3()).unwrap(),
                    rotation: actor.rotation * target.rotation.unwrap().as_dquat(),
                };
                let expected = current_support.transform_point(contact_local).unwrap();
                assert!(resolved.transform_point(pivot).unwrap().distance(expected) < 1e-6);
                assert!(resolved.rotation.dot(roll).abs() > 1. - 1e-7);
            }
        }
    }
    #[test]
    fn pivot_transfer_preserves_pose_then_rolls_about_new_point() {
        let support = RootRigidTransform {
            translation: DVec3::new(2., 3., 4.),
            rotation: DQuat::from_rotation_y(0.7),
        };
        let ankle = support
            .compose(RootRigidTransform {
                translation: DVec3::new(0.2, 0.08, 0.),
                rotation: DQuat::IDENTITY,
            })
            .unwrap();
        let heel = DVec3::new(0., -0.08, -0.04);
        let toe = DVec3::new(0., -0.08, 0.14);
        let plant = FootPlant::capture_pivot(ankle, heel, support).unwrap();
        let actor = RootRigidTransform::IDENTITY;
        let roll = support.rotation * DQuat::from_rotation_x(0.3);
        let before = plant
            .target_with_rotation(support, actor, roll, Vec3::Z, 1.)
            .unwrap();
        let transferred = plant
            .with_pivot_preserving_ankle(support, roll, toe)
            .unwrap();
        let after = transferred.target(support, actor, Vec3::Z, 1.).unwrap();
        assert!(before.position.abs_diff_eq(after.position, 1e-6));
        assert!(before.rotation.unwrap().dot(after.rotation.unwrap()).abs() > 1. - 1e-6);
        let new_contact = before.position.as_dvec3() + roll * toe;
        let further_roll = support.rotation * DQuat::from_rotation_x(0.6);
        let next = transferred
            .target_with_rotation(support, actor, further_roll, Vec3::Z, 1.)
            .unwrap();
        assert!((next.position.as_dvec3() + further_roll * toe).distance(new_contact) < 1e-6);
        let saved = plant;
        assert!(
            plant
                .with_pivot_preserving_ankle(support, roll, DVec3::splat(f64::NAN))
                .is_err()
        );
        assert!(
            plant
                .with_pivot_preserving_ankle(support, DQuat::from_xyzw(0., 0., 0., 2.), toe)
                .is_err()
        );
        assert_eq!(plant, saved);
    }

    #[test]
    fn transferred_pivot_follows_platform_and_is_world_origin_independent() {
        let heel = DVec3::new(0., -0.08, -0.04);
        let toe = DVec3::new(0., -0.08, 0.14);
        let mut reference = None;
        for origin in [DVec3::ZERO, DVec3::new(1e12, -1e12, 1e12)] {
            let support = RootRigidTransform {
                translation: origin,
                rotation: DQuat::from_rotation_y(0.4),
            };
            // Common origins deliberately cancel before any local pivot arithmetic.
            let ankle = RootRigidTransform {
                translation: origin,
                rotation: support.rotation,
            };
            let plant = FootPlant::capture_pivot(ankle, heel, support).unwrap();
            let rotation = support.rotation * DQuat::from_rotation_x(0.2);
            let transferred = plant
                .with_pivot_preserving_ankle(support, rotation, toe)
                .unwrap();
            let actor = RootRigidTransform {
                translation: origin,
                rotation: DQuat::IDENTITY,
            };
            let before = plant
                .target_with_rotation(support, actor, rotation, Vec3::Z, 1.)
                .unwrap();
            let after = transferred.target(support, actor, Vec3::Z, 1.).unwrap();
            assert!(before.position.abs_diff_eq(after.position, 1e-7));
            if let Some(previous) = reference {
                assert_eq!(after.position, previous);
            }
            reference = Some(after.position);
            let moved_support = RootRigidTransform {
                translation: origin + DVec3::new(0.5, 0.25, -0.5),
                rotation: DQuat::from_rotation_y(1.1),
            };
            let rolled = moved_support.rotation * DQuat::from_rotation_x(0.5);
            let result = transferred
                .target_with_rotation(moved_support, actor, rolled, Vec3::Z, 1.)
                .unwrap();
            // Compare support-relative coordinates; adding the large origin would
            // discard local precision and hide a bad result behind a loose tolerance.
            let actual = moved_support.rotation.conjugate()
                * (result.position.as_dvec3() - (moved_support.translation - actor.translation)
                    + rolled * toe);
            assert!(actual.distance(transferred.contact_in_support.translation) < 1e-6);
            let returned = transferred
                .with_pivot_preserving_ankle(support, rotation, heel)
                .unwrap();
            assert!(
                returned
                    .contact_in_support
                    .translation
                    .distance(plant.contact_in_support.translation)
                    < 1e-15
            );
        }
        let plant = FootPlant::capture_pivot(
            RootRigidTransform::IDENTITY,
            DVec3::splat(-f64::MAX),
            RootRigidTransform::IDENTITY,
        )
        .unwrap();
        let saved = plant;
        assert!(
            plant
                .with_pivot_preserving_ankle(
                    RootRigidTransform::IDENTITY,
                    DQuat::IDENTITY,
                    DVec3::splat(f64::MAX)
                )
                .is_err()
        );
        assert_eq!(plant, saved);
    }

    #[test]
    fn zero_pivot_preserves_captured_target_and_invalid_roll_rejects() {
        let identity = RootRigidTransform::IDENTITY;
        let plant = FootPlant::capture(identity, identity).unwrap();
        let explicit = FootPlant::capture_pivot(identity, DVec3::ZERO, identity).unwrap();
        assert_eq!(plant, explicit);
        assert_eq!(
            plant
                .target(identity, identity, Vec3::Z, 1.)
                .unwrap()
                .position,
            Vec3::ZERO
        );
        assert!(FootPlant::capture_pivot(identity, DVec3::splat(f64::NAN), identity).is_err());
        assert!(
            plant
                .target_with_rotation(
                    identity,
                    identity,
                    DQuat::from_xyzw(0., 0., 0., 2.),
                    Vec3::Z,
                    1.
                )
                .is_err()
        );
    }
}
