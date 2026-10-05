//! Prescribed support motion with an independent reaction-work energy guard.
use super::{InertialBody, PlaneContact, PrescribedTriangleSurface, Vec3, cross, dot};
use std::sync::Arc;

/// Initial guess only: transport the current shape through a support patch.
/// All pins remain exact. The owner must still certify the complete trajectory,
/// solve equilibrium and independently admit work before publishing anything.
pub(super) fn support_transport_guess(
    positions: &[Vec3],
    end: &[Vec3],
    pinned: &[bool],
) -> Option<Vec<Vec3>> {
    use crate::biomechanics::{add, columns, inverse, mm, mv, scale, sub};
    if positions.len() != end.len()
        || positions.len() != pinned.len()
        || positions
            .iter()
            .chain(end)
            .flatten()
            .any(|v| !v.is_finite())
    {
        return None;
    }
    let pins: Vec<_> = pinned
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.then_some(i))
        .collect();
    let &a = pins.first()?;
    if pins.len() == positions.len() {
        return None;
    }
    let translated = || {
        let delta = sub(end[a], positions[a]);
        positions
            .iter()
            .enumerate()
            .map(|(i, p)| if pinned[i] { end[i] } else { add(*p, delta) })
            .collect::<Vec<_>>()
    };
    let transported = (|| {
        let &b = pins.iter().max_by(|&&i, &&j| {
            let p = sub(positions[i], positions[a]);
            let q = sub(positions[j], positions[a]);
            dot(p, p).total_cmp(&dot(q, q))
        })?;
        let old_b = sub(positions[b], positions[a]);
        let &c = pins.iter().max_by(|&&i, &&j| {
            let p = cross(old_b, sub(positions[i], positions[a]));
            let q = cross(old_b, sub(positions[j], positions[a]));
            dot(p, p).total_cmp(&dot(q, q))
        })?;
        let old_c = sub(positions[c], positions[a]);
        let new_b = sub(end[b], end[a]);
        let new_c = sub(end[c], end[a]);
        let old_normal = cross(old_b, old_c);
        let new_normal = cross(new_b, new_c);
        let old_area = dot(old_normal, old_normal).sqrt();
        let new_area = dot(new_normal, new_normal).sqrt();
        if !old_area.is_finite() || !new_area.is_finite() || old_area <= 0. || new_area <= 0. {
            return None;
        }
        let length = old_area.sqrt();
        // Normalize before inversion so conditioning is independent of metres.
        let old_basis = columns(
            scale(old_b, 1. / length),
            scale(old_c, 1. / length),
            scale(old_normal, 1. / old_area),
        );
        let new_basis = columns(
            scale(new_b, 1. / length),
            scale(new_c, 1. / length),
            scale(new_normal, new_area.sqrt() / (length * new_area)),
        );
        let motion = mm(new_basis, inverse(old_basis).ok()?);
        Some(
            positions
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if pinned[i] {
                        end[i]
                    } else {
                        add(end[a], mv(motion, sub(*p, positions[a])))
                    }
                })
                .collect::<Vec<_>>(),
        )
    })()
    .unwrap_or_else(translated);
    transported
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(transported)
}

#[derive(Clone, Copy, Debug)]
pub struct SupportTarget {
    pub node: usize,
    pub position_m: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct DrivenSupportStep {
    /// Derived sum of reaction and pin kinetic work. Separate components below
    /// retain information smaller than the sum's floating-point resolution.
    pub support_work_j: f64,
    /// Potential-gradient reaction work, excluding pin kinetic-energy change.
    pub reaction_work_j: f64,
    /// Work of changing prescribed pin velocities, included in support_work_j.
    pub pin_kinetic_work_j: f64,
    /// Sum of translation and rotation work of the prescribed penalty plane.
    /// This is an external actuator, not heat and not work of the pinned nodes.
    pub plane_work_j: f64,
    /// Offset-derivative work; remains separate even when the sum loses precision.
    pub plane_translation_work_j: f64,
    /// Angular-gradient work along the shortest normal rotation.
    pub plane_rotation_work_j: f64,
    /// Independent actuator work of prescribed triangle-obstacle vertex motion.
    pub surface_work_j: f64,
    /// Delta(K + U) minus independently evaluated pin and plane actuator work.
    pub energy_defect_j: f64,
}
impl InertialBody {
    /// Prescribes every pin's next position; free nodes use velocity Verlet.
    /// Supports follow a linear segment during this step, with velocity dx/dt.
    /// Changing from the previous prescribed velocity is an actuator impulse;
    /// its exact kinetic-energy change is included in the returned work.
    /// Elastic/dead-load/gravity reaction work uses endpoint trapezoidal forces.
    /// The energy tolerance bounds quadrature and integration defect; it does
    /// not replace timestep convergence or bound the supplied actuator work.
    /// # Errors
    /// Rejects missing/duplicate/free/nonfinite targets, invalid controls,
    /// inversion, gap-path crossing, constitutive failure or work defect.
    /// All failures preserve the complete original inertial state.
    pub fn step_with_support_targets(
        &mut self,
        targets: &[SupportTarget],
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        self.advance_supports(dt, energy_tolerance_j, Some(targets))
    }
    /// Advance an installed frictionless plane to its prescribed next offset.
    /// Normal and stiffness remain fixed; the offset follows a linear segment.
    /// Every pinned node needs a target when `targets` is present, otherwise pins
    /// must be stationary. Actuator work uses the contact offset-gradient at both
    /// endpoints, independently of the measured body energy change.
    /// # Errors
    /// Missing plane, invalid controls, rejected geometry or work defect preserve
    /// node state and the previous plane offset. History material uses the
    /// dedicated viscoelastic counterpart instead.
    pub fn step_with_moving_plane(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_offset_m: f64,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        let plane = self.plane_at_offset(next_offset_m)?;
        self.advance_supports_with_plane(dt, energy_tolerance_j, targets, Some(plane))
    }
    /// Advance an installed plane's offset and normal. The normal follows its
    /// shortest rotation and actuator torque work uses both endpoint gradients.
    /// Stiffness is unchanged. An antipodal normal needs intermediate controls
    /// because two endpoints do not identify its rotation axis.
    /// # Errors
    /// Invalid motion, unsupported material, path or work rejection is atomic.
    pub fn step_with_plane_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_plane: PlaneContact,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        self.advance_supports_with_plane(dt, energy_tolerance_j, targets, Some(next_plane))
    }
    /// Move the installed triangle obstacle along linear vertex trajectories.
    /// Supports may move together; work uses the obstacle's feature gradients.
    /// # Errors
    /// Missing/changed obstacle owner, failed swept separation or work admission
    /// preserves positions, velocities and the last admitted obstacle pose.
    pub fn step_with_surface_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next_surface: Arc<PrescribedTriangleSurface>,
        dt: f64,
        energy_tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        self.advance_supports_with_contacts(
            dt,
            energy_tolerance_j,
            targets,
            None,
            Some(next_surface),
        )
    }
    pub(super) fn plane_at_offset(&self, offset: f64) -> Result<PlaneContact, &'static str> {
        if !offset.is_finite() {
            return Err("invalid moving plane offset");
        }
        let mut plane = self
            .plane
            .ok_or("moving plane requires installed contact")?;
        plane.offset_m = offset;
        Ok(plane)
    }
    pub(super) fn require_stationary_supports(&self) -> Result<(), &'static str> {
        if self
            .body
            .pinned
            .iter()
            .zip(&self.velocities)
            .any(|(pin, v)| *pin && v.iter().any(|component| *component != 0.))
        {
            return Err("moving supports require prescribed support step");
        }
        Ok(())
    }
    pub(super) fn advance_supports(
        &mut self,
        dt: f64,
        energy_tolerance_j: f64,
        targets: Option<&[SupportTarget]>,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.advance_supports_with_plane(dt, energy_tolerance_j, targets, None)
    }
    pub(super) fn advance_supports_with_plane(
        &mut self,
        dt: f64,
        energy_tolerance_j: f64,
        targets: Option<&[SupportTarget]>,
        next_motion: Option<PlaneContact>,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.advance_supports_with_contacts(dt, energy_tolerance_j, targets, next_motion, None)
    }
    pub(super) fn advance_supports_with_contacts(
        &mut self,
        dt: f64,
        energy_tolerance_j: f64,
        targets: Option<&[SupportTarget]>,
        next_motion: Option<PlaneContact>,
        next_surface_motion: Option<Arc<PrescribedTriangleSurface>>,
    ) -> Result<DrivenSupportStep, &'static str> {
        if let Some(next) = &next_surface_motion {
            self.prescribed_surface
                .as_ref()
                .ok_or("surface motion requires installed contact")?
                .same_owner(next)?;
        }
        let next_surface = next_surface_motion
            .as_ref()
            .or(self.prescribed_surface.as_ref());
        let mut angular_increment = [0.; 3];
        let mut offset_increment = 0.;
        if let Some(next) = next_motion {
            let initial = self
                .plane
                .ok_or("moving plane requires installed contact")?;
            if next.stiffness_n_m != initial.stiffness_n_m {
                return Err("moving plane requires unchanged contact stiffness");
            }
            offset_increment = next.offset_m - initial.offset_m;
            let axis = cross(initial.normal, next.normal);
            let sine = axis[0].hypot(axis[1]).hypot(axis[2]);
            let cosine = dot(initial.normal, next.normal).clamp(-1., 1.);
            if sine <= 1e-12 && cosine < 0. {
                return Err("ambiguous antipodal plane rotation");
            }
            if sine > 0. {
                let angle = sine.atan2(cosine);
                angular_increment = axis.map(|value| value / sine * angle);
            }
        }
        let next_plane = next_motion.or(self.plane);
        if !dt.is_finite()
            || dt <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j <= 0.
        {
            return Err("invalid finite-deformation inertial step");
        }
        let prescribed = if let Some(targets) = targets {
            let count = self.body.pinned.iter().filter(|&&pin| pin).count();
            if count == 0 || targets.len() != count {
                return Err("incomplete prescribed support targets");
            }
            let mut positions = vec![None; self.body.positions.len()];
            for target in targets {
                if target.node >= positions.len()
                    || !self.body.pinned[target.node]
                    || positions[target.node].is_some()
                    || target.position_m.iter().any(|v| !v.is_finite())
                {
                    return Err("invalid prescribed support target");
                }
                positions[target.node] = Some(target.position_m);
            }
            Some(positions)
        } else {
            None
        };
        let initial = self.evaluate_at_contacts(
            &self.body.positions,
            self.plane,
            self.prescribed_surface.as_deref(),
        )?;
        let before = self.diagnostics_from_potential(initial.potential_j, initial.contact_j)?;
        // Only kinematic state changes during this frozen-history step. Stage
        // those two buffers; the outer viscoelastic transaction owns histories.
        let mut positions = self.body.positions.clone();
        let mut velocities = self.velocities.clone();
        for (node, force) in initial.gradient.iter().enumerate() {
            if self.body.pinned[node] {
                if let Some(prescribed_positions) = &prescribed {
                    let target =
                        prescribed_positions[node].ok_or("missing prescribed support target")?;
                    velocities[node] = std::array::from_fn(|axis| {
                        (target[axis] - self.body.positions[node][axis]) / dt
                    });
                    positions[node] = target;
                }
                continue;
            }
            for (axis, &component) in force.iter().enumerate() {
                velocities[node][axis] -=
                    0.5 * dt * (component / self.masses[node] - self.acceleration[axis]);
                positions[node][axis] += dt * velocities[node][axis];
            }
        }
        if !self.body.gap_path_is_open(&self.body.positions, &positions) {
            return Err("inertial tissue gap path crossing");
        }
        if !self.volume_path_is_open(&positions) {
            return Err("inertial tetrahedral path collapse");
        }
        if angular_increment != [0.; 3] {
            self.certify_plane_motion_path(&positions, next_plane.unwrap(), angular_increment)?;
        }
        if let Some(surface) = &self.prescribed_surface {
            if !surface.path_is_open(
                next_surface.unwrap(),
                &self.body.positions,
                &positions,
                &self.body.surface(),
            )? {
                return Err("inertial prescribed surface path crossing");
            }
        }
        let final_response =
            self.evaluate_at_contacts(&positions, next_plane, next_surface.map(AsRef::as_ref))?;
        let surface_work = if next_surface_motion.is_some() {
            self.prescribed_surface.as_ref().unwrap().motion_work(
                next_surface.unwrap(),
                &initial.surface_gradient,
                &final_response.surface_gradient,
            )?
        } else {
            0.
        };
        let translation_work = if offset_increment == 0. {
            0.
        } else {
            (0.5 * initial.plane_offset_gradient + 0.5 * final_response.plane_offset_gradient)
                * offset_increment
        };
        let rotation_work = if angular_increment == [0.; 3] {
            0.
        } else {
            let average = std::array::from_fn(|axis| {
                0.5 * initial.plane_rotation_gradient[axis]
                    + 0.5 * final_response.plane_rotation_gradient[axis]
            });
            dot(average, angular_increment)
        };
        let plane_work = translation_work + rotation_work;
        let lost_plane_component = if translation_work != 0. && plane_work == rotation_work {
            translation_work.abs()
        } else if rotation_work != 0. && plane_work == translation_work {
            rotation_work.abs()
        } else {
            0.
        };
        if lost_plane_component > energy_tolerance_j {
            return Err("unrepresentable plane actuator work");
        }
        for (node, force) in final_response.gradient.iter().enumerate() {
            if self.body.pinned[node] {
                continue;
            }
            for (axis, &component) in force.iter().enumerate() {
                velocities[node][axis] -=
                    0.5 * dt * (component / self.masses[node] - self.acceleration[axis]);
            }
        }
        let after = self.diagnostics_at(
            final_response.potential_j,
            final_response.contact_j,
            &positions,
            &velocities,
        )?;
        let mut reaction_work = 0.;
        let mut kinetic_work = 0.;
        if prescribed.is_some() {
            for (node, &pinned) in self.body.pinned.iter().enumerate() {
                if !pinned {
                    continue;
                }
                let displacement = std::array::from_fn(|axis| {
                    positions[node][axis] - self.body.positions[node][axis]
                });
                let reaction: Vec3 = std::array::from_fn(|axis| {
                    0.5 * (initial.gradient[node][axis] + final_response.gradient[node][axis])
                        - self.masses[node] * self.acceleration[axis]
                });
                reaction_work += dot(reaction, displacement);
                kinetic_work += 0.5
                    * self.masses[node]
                    * (dot(velocities[node], velocities[node])
                        - dot(self.velocities[node], self.velocities[node]));
            }
        }
        let work = reaction_work + kinetic_work;
        let lost_component = if reaction_work != 0. && work == kinetic_work {
            reaction_work.abs()
        } else if kinetic_work != 0. && work == reaction_work {
            kinetic_work.abs()
        } else {
            0.
        };
        if lost_component > energy_tolerance_j {
            return Err("unrepresentable prescribed support work");
        }
        let defect = if prescribed.is_some() {
            // Pin kinetic energy is exactly the actuator impulse work. Cancel
            // it analytically before summing, so a large prescribed velocity
            // cannot hide a small elastic/Verlet work defect by rounding.
            let free_kinetic_change: f64 = self
                .body
                .pinned
                .iter()
                .enumerate()
                .filter(|(_, pinned)| !**pinned)
                .map(|(node, _)| {
                    0.5 * self.masses[node]
                        * (dot(velocities[node], velocities[node])
                            - dot(self.velocities[node], self.velocities[node]))
                })
                .sum();
            free_kinetic_change + (after.potential_j - before.potential_j)
                - reaction_work
                - plane_work
                - surface_work
        } else {
            (after.kinetic_j - before.kinetic_j) + (after.potential_j - before.potential_j)
                - plane_work
                - surface_work
        };
        if !surface_work.is_finite()
            || !plane_work.is_finite()
            || !work.is_finite()
            || !kinetic_work.is_finite()
            || !defect.is_finite()
            || defect.abs() > energy_tolerance_j
        {
            return Err(if prescribed.is_some() {
                "finite-deformation inertial support work defect"
            } else {
                "finite-deformation inertial energy defect"
            });
        }
        self.body.positions = positions;
        self.velocities = velocities;
        self.plane = next_plane;
        if let Some(surface) = next_surface_motion {
            self.prescribed_surface = Some(surface);
        }
        Ok(DrivenSupportStep {
            support_work_j: work,
            reaction_work_j: reaction_work,
            pin_kinetic_work_j: kinetic_work,
            plane_work_j: plane_work,
            plane_translation_work_j: translation_work,
            plane_rotation_work_j: rotation_work,
            surface_work_j: surface_work,
            energy_defect_j: defect,
        })
    }
    // A rotating obstacle can enter and leave between force evaluations. Bound
    // the gap curvature along spherical normal motion and linear nodal drift.
    // Unknown intervals reject conservatively; the caller must subdivide them.
    fn certify_plane_motion_path(
        &self,
        positions: &[Vec3],
        next: PlaneContact,
        angle: Vec3,
    ) -> Result<(), &'static str> {
        let initial = self.plane.unwrap();
        let theta = angle[0].hypot(angle[1]).hypot(angle[2]);
        let offset_delta = next.offset_m - initial.offset_m;
        let mut boundary = vec![false; positions.len()];
        for face in self.body.surface() {
            for node in face {
                boundary[node] = true;
            }
        }
        for (node, on_surface) in boundary.into_iter().enumerate() {
            if !on_surface {
                continue;
            }
            let start = self.body.positions[node];
            let end = positions[node];
            let g0 = dot(initial.normal, start) - initial.offset_m;
            let g1 = dot(next.normal, end) - next.offset_m;
            // Existing endpoint contact is already represented in the force
            // quadrature. Certify intervals whose two endpoints appear open.
            if g0 < 0. || g1 < 0. {
                continue;
            }
            let drift: Vec3 = std::array::from_fn(|axis| end[axis] - start[axis]);
            let norm = |value: Vec3| value[0].hypot(value[1]).hypot(value[2]);
            let radius = norm(start).max(norm(end));
            let curvature = (theta * (theta * radius) + 2. * theta * norm(drift)) * (1. + 1e-10);
            let derivative0 = dot(cross(angle, initial.normal), start) + dot(initial.normal, drift)
                - offset_delta;
            let derivative1 =
                dot(cross(angle, next.normal), end) + dot(next.normal, drift) - offset_delta;
            if !curvature.is_finite() || !derivative0.is_finite() || !derivative1.is_finite() {
                return Err("unrepresentable rotating plane path");
            }
            let monotone = derivative0 - curvature >= 0.
                || derivative1 - curvature >= 0.
                || derivative0 + curvature <= 0.
                || derivative1 + curvature <= 0.;
            if !monotone && g0.min(g1) < curvature / 8. {
                return Err("unresolved swept rotating plane contact");
            }
        }
        Ok(())
    }
    /// Certify the minimum signed determinant along each linear drift segment.
    /// Endpoint-positive tetrahedra can still collapse halfway through a flip.
    pub(super) fn volume_path_is_open(&self, next: &[Vec3]) -> bool {
        let sub = |a: Vec3, b: Vec3| std::array::from_fn(|i| a[i] - b[i]);
        let triple = |a: Vec3, b: Vec3, c: Vec3| dot(a, super::cross(b, c));
        self.body.elements.iter().all(|element| {
            let [a, b, c, d] = element.nodes;
            let u = sub(self.body.positions[b], self.body.positions[a]);
            let v = sub(self.body.positions[c], self.body.positions[a]);
            let w = sub(self.body.positions[d], self.body.positions[a]);
            let du = sub(sub(next[b], next[a]), u);
            let dv = sub(sub(next[c], next[a]), v);
            let dw = sub(sub(next[d], next[a]), w);
            let mut coefficients = [
                triple(u, v, w),
                triple(du, v, w) + triple(u, dv, w) + triple(u, v, dw),
                triple(du, dv, w) + triple(du, v, dw) + triple(u, dv, dw),
                triple(du, dv, dw),
            ];
            if coefficients.iter().any(|c| !c.is_finite()) || coefficients[0] == 0. {
                return false;
            }
            let orientation = coefficients[0].signum();
            let scale = coefficients.iter().fold(0.0_f64, |s, c| s.max(c.abs()));
            for c in &mut coefficients {
                *c = (*c / scale) * orientation;
            }
            let [c0, c1, c2, c3] = coefficients;
            let evaluate = |t: f64| ((c3 * t + c2) * t + c1) * t + c0;
            let mut minimum = evaluate(0.).min(evaluate(1.));
            let mut sample = |t: f64| {
                if t.is_finite() && (0. ..1.).contains(&t) {
                    minimum = minimum.min(evaluate(t));
                }
            };
            let (a, b, c) = (3. * c3, 2. * c2, c1);
            if a == 0. {
                if b != 0. {
                    sample(-c / b);
                }
            } else {
                let discriminant = b * b - 4. * a * c;
                if discriminant >= 0. {
                    let q = -0.5
                        * (b + if b >= 0. {
                            discriminant.sqrt()
                        } else {
                            -discriminant.sqrt()
                        });
                    if q != 0. {
                        sample(q / a);
                        sample(c / q);
                    }
                }
            }
            let roundoff_margin =
                64. * f64::EPSILON * coefficients.iter().map(|c| c.abs()).sum::<f64>();
            minimum.is_finite() && minimum > roundoff_margin
        })
    }
}

#[cfg(test)]
mod transport_tests {
    use super::support_transport_guess;

    #[test]
    fn support_patch_transports_rotation_scale_and_translation() {
        let old = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0.2, 0.3, 0.4]];
        let transform = |p: [f64; 3]| [3. - 2. * p[1], 4. + 2. * p[0], 5. + 2. * p[2]];
        let end = old.map(transform);
        let got = support_transport_guess(&old, &end, &[true, true, true, false]).unwrap();
        for (actual, expected) in got.iter().zip(end) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn single_and_collinear_supports_use_translation() {
        let old = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [0., 1., 0.]];
        let end = old.map(|p| [p[0] + 2., p[1] - 3., p[2] + 4.]);
        for pins in [[true, false, false, false], [true, true, true, false]] {
            assert_eq!(support_transport_guess(&old, &end, &pins).unwrap(), end);
        }
    }

    #[test]
    fn invalid_or_unusable_supports_do_not_produce_a_guess() {
        let old = [[0.; 3], [1.; 3]];
        assert!(support_transport_guess(&old, &old, &[false; 2]).is_none());
        assert!(support_transport_guess(&old, &old, &[true; 2]).is_none());
        assert!(support_transport_guess(&old, &old[..1], &[true, false]).is_none());
        let mut invalid = old;
        invalid[1][0] = f64::NAN;
        assert!(support_transport_guess(&old, &invalid, &[true, false]).is_none());
        assert_eq!(old, [[0.; 3], [1.; 3]]);
    }
}
