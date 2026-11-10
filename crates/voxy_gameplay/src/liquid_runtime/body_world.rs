use super::*;

impl SceneBodyWorld {
    fn support_point_shapes(
        &self,
        bodies: &[physics::contact::ContactBody],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<
        (
            physics::contact::ContactBody,
            crate::convex::AffineBox,
            Option<physics::contact::ContactBody>,
            crate::convex::AffineBox,
        ),
        physics::liquid::Error,
    > {
        use physics::liquid::Error;
        let key = FeatureKey::decode(geometry.feature.ok_or(Error::CollisionBackend)?)?;
        let first = *bodies.get(support.first).ok_or(Error::InvalidCollision)?;
        let shape = *self
            .templates
            .get(support.first)
            .and_then(|shapes| shapes.get(key.first))
            .ok_or(Error::InvalidCollision)?;
        let (second, obstacle) = if let Some(index) = support.second {
            (
                Some(*bodies.get(index).ok_or(Error::InvalidCollision)?),
                *self
                    .templates
                    .get(index)
                    .and_then(|shapes| shapes.get(key.second))
                    .ok_or(Error::InvalidCollision)?,
            )
        } else {
            (
                None,
                self.environment
                    .0
                    .0
                    .get(key.second)
                    .ok_or(Error::InvalidCollision)?
                    .shape,
            )
        };
        Ok((first, shape, second, obstacle))
    }
}
impl physics::liquid::LiquidBodyWorld for SceneBodyWorld {
    fn rigid_support_point_velocity(
        &self,
        bodies: &[physics::contact::ContactBody],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<[f64; 3], physics::liquid::Error> {
        let (first, shape, second, obstacle) =
            self.support_point_shapes(bodies, support, geometry)?;
        rigid_support::point_velocity(
            first,
            shape,
            second,
            obstacle,
            support.support.contact.point,
            geometry.tolerance_m + geometry.admission_error_m,
        )
    }

    fn rigid_support_point_motion(
        &self,
        bodies: &[physics::contact::ContactBody],
        loads: &[physics::contact::ContactWrench],
        rates: &[physics::contact::ContactWrench],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<Option<physics::liquid::RigidSupportPointMotion>, physics::liquid::Error> {
        use physics::liquid::Error;
        let (first, shape, second, obstacle) =
            self.support_point_shapes(bodies, support, geometry)?;
        let p = support.support.contact.point;
        let tolerance = geometry.tolerance_m + geometry.admission_error_m;
        let velocity =
            match rigid_support::point_velocity(first, shape, second, obstacle, p, tolerance) {
                Ok(velocity) => velocity,
                // A clipped-vertex kink has no unique derivative. The existing
                // common-point model is still available, but must independently
                // prove whole-interval ownership; no arbitrary vertex is chosen.
                Err(Error::CollisionBackend) => return Ok(None),
                Err(error) => return Err(error),
            };
        let pair = |all: &[physics::contact::ContactWrench]| -> Result<_, Error> {
            Ok([
                *all.get(support.first).ok_or(Error::InvalidCollision)?,
                if let Some(index) = support.second {
                    *all.get(index).ok_or(Error::InvalidCollision)?
                } else {
                    Default::default()
                },
            ])
        };
        let acceleration = match rigid_support::point_acceleration(
            first,
            shape,
            second,
            obstacle,
            p,
            tolerance,
            pair(loads)?,
        ) {
            Ok(value) => value,
            Err(Error::CollisionBackend) => return Ok(None),
            Err(error) => return Err(error),
        };
        let jerk = match rigid_support::point_jerk(
            first,
            shape,
            second,
            obstacle,
            p,
            tolerance,
            pair(loads)?,
            pair(rates)?,
        ) {
            Ok(value) => value,
            Err(Error::CollisionBackend) => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(physics::liquid::RigidSupportPointMotion {
            velocity,
            acceleration,
            jerk,
        }))
    }

    fn rigid_support_normal_acceleration(
        &self,
        bodies: &[physics::contact::ContactBody],
        loads: &[physics::contact::ContactWrench],
        support: physics::contact::NetworkSupport,
        geometry: physics::liquid::RigidSupportPoint,
    ) -> Result<Option<[f64; 3]>, physics::liquid::Error> {
        let physics::contact::SupportPlane::Rate { normal_rate } = support.support.plane else {
            return Ok(None);
        };
        let key = FeatureKey::decode(
            geometry
                .feature
                .ok_or(physics::liquid::Error::CollisionBackend)?,
        )?;
        let first_shape = *self
            .templates
            .get(support.first)
            .and_then(|shapes| shapes.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let second_shape = if let Some(j) = support.second {
            *self
                .templates
                .get(j)
                .and_then(|shapes| shapes.get(key.second))
                .ok_or(physics::liquid::Error::InvalidCollision)?
        } else {
            self.environment
                .0
                .0
                .get(key.second)
                .ok_or(physics::liquid::Error::InvalidCollision)?
                .shape
        };
        rigid_support::edge_normal_acceleration(
            bodies[support.first],
            first_shape,
            support.second.map(|j| bodies[j]),
            second_shape,
            key.axis,
            support.support.contact.normal,
            normal_rate,
            loads[support.first],
            support.second.map_or(Default::default(), |j| loads[j]),
        )
        .map(Some)
    }
    fn rigid_pair_support_contacts(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_pair_support_contacts_with_error(i, first, j, second, budget, 0.)
    }
    fn rigid_environment_support_contacts(
        &self,
        i: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_environment_support_contacts_with_error(i, body, budget, 0.)
    }
    fn rigid_pair_support_contacts_with_error(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let a = self.templates.get(i).ok_or(Error::InvalidCollision)?;
        let b = self.templates.get(j).ok_or(Error::InvalidCollision)?;
        if i == j || budget == 0 || a.len().checked_mul(b.len()).is_none_or(|n| n > budget) {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in a.iter().enumerate() {
            for (ib, obstacle) in b.iter().enumerate() {
                for (witness, normal, axis) in rigid_support::snapshot_with_error(
                    *first,
                    *shape,
                    Some(*second),
                    *obstacle,
                    error_m,
                )? {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        first,
                        &[*obstacle],
                        second,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane = rigid_support::plane(
                        *first,
                        *shape,
                        Some(*second),
                        *obstacle,
                        axis,
                        normal,
                    )?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn rigid_environment_support_contacts_with_error(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let shapes = self.templates.get(index).ok_or(Error::InvalidCollision)?;
        let walls = &self.environment.0.0;
        if budget == 0
            || shapes
                .len()
                .checked_mul(walls.len())
                .is_none_or(|n| n > budget)
        {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in shapes.iter().enumerate() {
            for (ib, wall) in walls.iter().enumerate() {
                for (witness, normal, axis) in
                    rigid_support::snapshot_with_error(*body, *shape, None, wall.shape, error_m)?
                {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let fixed = physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            mass: 1.,
                            position: [0.; 3],
                            velocity: [0.; 3],
                        },
                        spin: None,
                    };
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        body,
                        &[wall.shape],
                        &fixed,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane =
                        rigid_support::plane(*body, *shape, None, wall.shape, axis, normal)?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn sweep_supported_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        supported_shape_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            supports,
            None,
            budget,
            false,
        )
    }
    fn sweep_supported_rigid_environment_event(
        &self,
        i: usize,
        body: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        let fixed = sampling_frame([0.; 3], body.duration())?;
        let walls: Vec<_> = self.environment.0.0.iter().map(|w| w.shape).collect();
        supported_shape_events(
            body,
            &self.templates[i],
            &fixed,
            &walls,
            supports,
            None,
            budget,
            true,
        )
    }
    fn sweep_supported_rigid_pair_point_motion(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        motion: &[Option<physics::liquid::RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        supported_shape_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            supports,
            Some(motion),
            budget,
            false,
        )
    }
    fn sweep_supported_rigid_environment_point_motion(
        &self,
        i: usize,
        body: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        motion: &[Option<physics::liquid::RigidSupportPointMotion>],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        let fixed = sampling_frame([0.; 3], body.duration())?;
        let walls: Vec<_> = self.environment.0.0.iter().map(|wall| wall.shape).collect();
        supported_shape_events(
            body,
            &self.templates[i],
            &fixed,
            &walls,
            supports,
            Some(motion),
            budget,
            true,
        )
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_pair_patch(i, first, j, second, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        contact_patch_points(&[*a], first, &[*b], second, witness, normal, budget)
    }
    fn rigid_environment_patch_for_feature(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_environment_patch(index, body, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let shape = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &[*shape],
            body,
            &[wall.shape],
            &fixed,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_pair_supports(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_pair_patch_for_feature(
            i,
            first,
            j,
            second,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*first, *a, Some(*second), *b, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }
    fn rigid_environment_supports(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_environment_patch_for_feature(
            index,
            body,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*body, *a, None, wall.shape, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }

    fn rigid_pair_patch(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        contact_patch_points(
            &self.templates[i],
            first,
            &self.templates[j],
            second,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_environment_patch(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        if budget == 0
            || self.templates[index]
                .len()
                .checked_mul(self.environment.0.0.len())
                .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        let obstacles: Vec<_> = self.environment.0.0.iter().map(|b| b.shape).collect();
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &self.templates[index],
            body,
            &obstacles,
            &fixed,
            witness,
            normal,
            budget,
        )
    }

    fn sweep_particle_rigid_contact(
        &self,
        p: &physics::liquid::Particle,
        r: f64,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_particle_rigid_event(p, r, i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        a: &physics::rigid_motion::RigidMotion,
        j: usize,
        b: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_pair_event(i, a, j, b, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_environment_contact(
        &self,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_environment_event(i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_particle_motion_body_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() || particle.duration() != body.duration() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if body.has_constant_acceleration()
            && particle.has_constant_acceleration()
            && body.initial().spin.is_none()
            && particle.acceleration() == body.acceleration()
        {
            let state = body.initial().motion;
            let hit = self.sweep_particle_body_contact(
                p,
                radius,
                index,
                &physics::liquid::TranslatingBody {
                    mass: state.mass,
                    position: state.position,
                    velocity: state.velocity,
                },
                body.duration(),
                budget,
            )?;
            return transport_acceleration(hit, body.acceleration(), body.duration())
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_events(particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_particle_motion_environment_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if particle.has_constant_acceleration() && particle.acceleration() == [0.; 3] {
            return self
                .sweep_particle_environment_contact(p, radius, particle.duration(), budget)
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_environment_events(particle, &[shape], &self.environment, budget)
    }
    fn sweep_particle_rigid_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_particle_body_contact(
                    p,
                    radius,
                    index,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    body.duration(),
                    budget,
                )
                .map(Into::into);
        }
        let particle = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: p.mass,
                position: p.position,
                velocity: p.velocity,
            },
            spin: None,
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            body.duration(),
            physics::spin_path::Config {
                max_angular_error_rad: 1e-5,
                min_step_s: 1e-9,
                max_arcs: 1,
                max_trials: 1,
            },
        )
        .map_err(|_| physics::liquid::Error::CollisionBackend)?;
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_events(&particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if first.has_constant_acceleration()
            && second.has_constant_acceleration()
            && first.initial().spin.is_none()
            && second.initial().spin.is_none()
            && first.acceleration() == [0.; 3]
            && second.acceleration() == [0.; 3]
        {
            let a = first.initial().motion;
            let b = second.initial().motion;
            return self
                .sweep_body_pair_contact(
                    i,
                    &physics::liquid::TranslatingBody {
                        mass: a.mass,
                        position: a.position,
                        velocity: a.velocity,
                    },
                    j,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    first.duration(),
                    budget,
                )
                .map(Into::into);
        }
        trajectory_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            budget,
        )
    }
    fn sweep_rigid_environment_event(
        &self,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_body_environment_contact(
                    index,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    body.duration(),
                    budget,
                )
                .map(Into::into);
        }
        trajectory_environment_events(body, &self.templates[index], &self.environment, budget)
    }

    fn sweep_particle_body(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_body_contact(p, radius, index, body, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_pair(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_pair_contact(first_index, first, second_index, second, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_particle_environment(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_environment_contact(p, radius, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_environment(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_environment_contact(index, body, dt, budget)
            .map(|hit| hit.geometry)
    }

    fn sweep_particle_body_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index].len() > budget {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().map(|template| {
            let mut shape = *template;
            shape.center += glam::DVec3::from_array(body.position);
            affine_hit(
                shape,
                glam::DVec3::from_array(p.position),
                [
                    glam::DVec3::X * radius,
                    glam::DVec3::Y * radius,
                    glam::DVec3::Z * radius,
                ],
                (glam::DVec3::from_array(p.velocity) - glam::DVec3::from_array(body.velocity)) * dt,
            )
            .and_then(|hit| transport_contact(hit, glam::DVec3::from_array(body.velocity) * dt))
        }))
    }
    fn sweep_body_pair_contact(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[first_index]
            .len()
            .checked_mul(self.templates[second_index].len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[first_index].iter().flat_map(|template| {
            self.templates[second_index].iter().map(move |obstacle| {
                let mut shape = *obstacle;
                shape.center += glam::DVec3::from_array(second.position);
                affine_hit(
                    shape,
                    template.center + glam::DVec3::from_array(first.position),
                    template.edges,
                    (glam::DVec3::from_array(first.velocity)
                        - glam::DVec3::from_array(second.velocity))
                        * dt,
                )
                .and_then(|hit| {
                    transport_contact(hit, glam::DVec3::from_array(second.velocity) * dt)
                })
            })
        }))
    }
    fn sweep_particle_environment_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.environment
            .sweep_contact(p.position, radius, p.velocity.map(|v| v * dt), budget)
            .map_err(|_| physics::liquid::Error::CollisionBackend)
    }
    fn sweep_body_environment_contact(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index]
            .len()
            .checked_mul(self.environment.0.0.len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().flat_map(|template| {
            self.environment.0.0.iter().map(move |obstacle| {
                affine_hit(
                    obstacle.shape,
                    template.center + glam::DVec3::from_array(body.position),
                    template.edges,
                    glam::DVec3::from_array(body.velocity) * dt,
                )
            })
        }))
    }
    fn has_environment(&self) -> bool {
        !self.environment.0.0.is_empty()
    }
}
