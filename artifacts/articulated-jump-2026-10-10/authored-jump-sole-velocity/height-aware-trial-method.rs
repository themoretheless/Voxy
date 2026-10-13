    pub(crate) fn authored_jump_pose(
        &self,
        library: &crate::humanoid_locomotion::VoxyHumanoidLocomotion,
        time: f64,
    ) -> Result<Pose, Box<dyn std::error::Error>> {
        let pose = library.jump_pose(time, &self.skeleton)?;
        let motion = crate::jump_motion::sample(time);
        let contact_weight = if let Some(phase) = motion.flight {
            let ease = |x: f32| {
                let x = x.clamp(0., 1.);
                x * x * x * (10. + x * (-15. + 6. * x))
            };
            (1. - ease(phase / 0.20)).max(ease((phase - 0.80) / 0.20))
        } else { 1. };
        if contact_weight == 0. { return Ok(pose); }
        let palette = pose.skin_matrices(&self.skeleton)?;
        let actor = voxy_animation::RootRigidTransform {
            // Approach the actual actor height at release/touchdown as well
            // as the support pose. A fixed zero preparation height preserves
            // position but drops the actor velocity at the contact boundary.
            translation: glam::DVec3::Y * motion.height * if motion.flight.is_none() { 1. } else { f64::from(contact_weight) },
            rotation: glam::DQuat::IDENTITY,
        };
        let targets = [8usize, 14].map(|knee| {
            let rest = self.skeleton.joints()[knee].inverse_bind.inverse().transform_point3(Vec3::ZERO);
            let pole = palette[knee].transform_point3(rest);
            self.jump_plants[if knee == 8 {0} else {1}].target(
                voxy_animation::RootRigidTransform::IDENTITY, actor, pole, 1.)
        });
        let targets = [targets[0].clone()?, targets[1].clone()?];
        let (supported, _) = pose.solve_planted_legs(&self.jump_chains, targets, Vec3::Y, 0.10)?;
        Ok(Pose::blend(&pose, &supported, contact_weight)?)
    }
