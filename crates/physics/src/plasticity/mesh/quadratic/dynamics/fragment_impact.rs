//! Owned single normal impact between two disconnected deformable fragments.
use super::QuadraticDynamics;
use crate::plasticity::mesh::{QuadraticNormalImpact, Vec3};
impl QuadraticDynamics {
    /// One explicitly identified coincident contact point per fragment. Each
    /// weight field sums to one and is supported on a different component;
    /// signed quadratic interpolation weights are allowed. Normal points from
    /// second to first. Free bodies only: supports need a reaction ledger.
    /// No pose correction, contact discovery, friction or simultaneous-contact
    /// solve is performed. Restitution loss is retained as a separate inventory.
    pub fn impact_fragments(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        self.impact_fragments_with_friction(first, second, normal, restitution, 0.)
    }
    /// Caller-identified contact with isotropic Coulomb impact friction.
    /// The owned loss inventory includes restitution and sliding dissipation.
    pub fn impact_fragments_with_friction(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
        friction: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        self.impact_fragment_constraint(first, second, normal, restitution, friction, None)
    }
    /// Frictionless proximity impulse along the line connecting two points.
    /// Its equal/opposite forces have zero total torque despite positive gap.
    /// This is a finite capture-distance model, not exact zero-gap impact.
    pub fn impact_fragments_along_gap(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
        maximum_gap_m: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        if !maximum_gap_m.is_finite() || maximum_gap_m <= 0. {
            return Err("invalid fragment impact capture distance");
        }
        self.impact_fragment_constraint(first, second, normal, restitution, 0., Some(maximum_gap_m))
    }
    fn impact_fragment_constraint(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
        friction: f64,
        maximum_gap_m: Option<f64>,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        let weights = self.fragment_contact_weights(first, second, normal, maximum_gap_m)?;
        let impact = self.inertia.frictional_impact(
            &self.velocities,
            &weights,
            normal,
            restitution,
            friction,
        )?;
        self.commit_fragment_impulse(
            &impact.velocities,
            &impact.nodal_impulse_n_s,
            impact.dissipated_j,
        )?;
        Ok(impact)
    }
    fn fragment_contact_weights(
        &self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        maximum_gap_m: Option<f64>,
    ) -> Result<Vec<f64>, &'static str> {
        let n = self.velocities.len();
        if self.free_nodes.len() != n
            || first.len() != n
            || second.len() != n
            || first.iter().chain(second).any(|v| !v.is_finite())
        {
            return Err("invalid free-fragment impact weights");
        }
        let groups = self.body.fragment_nodes();
        let mut owner = vec![usize::MAX; n];
        for (group, nodes) in groups.iter().enumerate() {
            for &node in nodes {
                owner[node] = group;
            }
        }
        let component = |weights: &[f64]| -> Result<(usize, usize), &'static str> {
            let node = weights
                .iter()
                .position(|v| *v != 0.)
                .ok_or("empty fragment contact weights")?;
            let sum: f64 = weights.iter().sum();
            let scale: f64 = weights.iter().map(|v| v.abs()).sum();
            if !sum.is_finite()
                || !scale.is_finite()
                || (sum - 1.).abs() > 1e-12 * scale
                || weights
                    .iter()
                    .enumerate()
                    .any(|(i, w)| *w != 0. && owner[i] != owner[node])
            {
                return Err("contact weights do not belong to one normalized fragment");
            }
            Ok((owner[node], node))
        };
        let (a, anchor) = component(first)?;
        let (b, _) = component(second)?;
        if a == b {
            return Err("impact requires disconnected fragments");
        }
        let origin = self.body.positions[anchor];
        let mut gap = [0.; 3];
        let mut scales = [0.; 3];
        for axis in 0..3 {
            gap[axis] = (0..n)
                .map(|i| (first[i] - second[i]) * (self.body.positions[i][axis] - origin[axis]))
                .sum();
            scales[axis] = (0..n)
                .map(|i| {
                    (first[i].abs() + second[i].abs())
                        * (self.body.positions[i][axis] - origin[axis]).abs()
                })
                .sum();
        }
        if gap.iter().chain(&scales).any(|x| !x.is_finite()) {
            return Err("fragment contact gap overflow");
        }
        if let Some(maximum) = maximum_gap_m {
            let length = gap[0].hypot(gap[1]).hypot(gap[2]);
            let axial: f64 = (0..3).map(|a| gap[a] * normal[a]).sum();
            let scale = scales.iter().sum::<f64>().max(length);
            if !length.is_finite()
                || !axial.is_finite()
                || !scale.is_finite()
                || length > maximum
                || axial < 0.
                || (0..3).any(|a| {
                    (gap[a] - axial * normal[a]).abs() > 1e-10 * scale.max(f64::MIN_POSITIVE)
                })
            {
                return Err("fragment gap does not align with impact normal");
            }
        } else if (0..3).any(|a| gap[a].abs() > 1e-10 * scales[a].max(f64::MIN_POSITIVE)) {
            return Err("fragment contact points are not coincident");
        }
        Ok(first.iter().zip(second).map(|(a, b)| a - b).collect())
    }
    fn commit_fragment_impulse(
        &mut self,
        velocities: &[Vec3],
        nodal_impulse_n_s: &[Vec3],
        dissipated_j: f64,
    ) -> Result<(), &'static str> {
        let n = self.velocities.len();
        let before = self.energy()?;
        let mut next = self.clone();
        next.velocities = velocities.to_vec();
        let loss = self.impact_dissipated_j + dissipated_j;
        if !loss.is_finite() || (dissipated_j > 0. && loss <= self.impact_dissipated_j) {
            return Err("unrepresentable fragment impact loss inventory");
        }
        next.impact_dissipated_j = loss;
        let after = next.energy()?;
        let defect = after.kinetic_j - before.kinetic_j + dissipated_j;
        let roundoff = 16. * f64::EPSILON * (before.kinetic_j.abs() + after.kinetic_j.abs());
        if !defect.is_finite()
            || !roundoff.is_finite()
            || defect.abs() > 1e-10 * dissipated_j.max(f64::MIN_POSITIVE) + roundoff
        {
            return Err("owned fragment impact energy mismatch");
        }
        for axis in 0..3 {
            let b = (axis + 1) % 3;
            let c = (axis + 2) % 3;
            let factor = 8. * (n * n + n) as f64 * f64::EPSILON;
            let mut p_bound = 0.;
            let mut l_bound = 0.;
            for (i, row) in self.mass.iter().enumerate() {
                for (j, m) in row.iter().enumerate() {
                    p_bound += factor
                        * m.abs()
                        * (self.velocities[j][axis].abs() + next.velocities[j][axis].abs());
                    l_bound += factor
                        * m.abs()
                        * (self.body.positions[i][b].abs()
                            * (self.velocities[j][c].abs() + next.velocities[j][c].abs())
                            + self.body.positions[i][c].abs()
                                * (self.velocities[j][b].abs() + next.velocities[j][b].abs()));
                }
            }
            let impulse_scale = nodal_impulse_n_s
                .iter()
                .flatten()
                .map(|x| x.abs())
                .sum::<f64>();
            let torque_scale = impulse_scale
                * self
                    .body
                    .positions
                    .iter()
                    .map(|p| p[b].abs() + p[c].abs())
                    .fold(0_f64, f64::max);
            if !p_bound.is_finite()
                || !l_bound.is_finite()
                || !torque_scale.is_finite()
                || (after.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis]).abs()
                    > 1e-10 * impulse_scale + p_bound
                || (after.angular_momentum_kg_m2_s[axis] - before.angular_momentum_kg_m2_s[axis])
                    .abs()
                    > 1e-10 * torque_scale + l_bound
            {
                return Err("owned fragment impact momentum mismatch");
            }
        }
        *self = next;
        Ok(())
    }
}
impl super::FiniteQuadraticDynamics {
    pub fn impact_fragments(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        self.inner
            .impact_fragments(first, second, normal, restitution)
    }
}

impl super::FiniteQuadraticDynamics {
    pub fn impact_fragments_with_friction(
        &mut self,
        first: &[f64],
        second: &[f64],
        normal: Vec3,
        restitution: f64,
        friction: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        self.inner
            .impact_fragments_with_friction(first, second, normal, restitution, friction)
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticFragmentImpactConstraint {
    pub first: Vec<f64>,
    pub second: Vec<f64>,
    pub normal: Vec3,
}
impl QuadraticDynamics {
    /// Joint perfectly inelastic, frictionless update of free fragments.
    /// Every contact is coincident or aligned with its bounded positive gap.
    /// Geometry, energy and total momentum are checked before one atomic commit.
    pub fn impact_fragment_contacts(
        &mut self,
        contacts: &[QuadraticFragmentImpactConstraint],
        maximum_gap_m: f64,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<crate::plasticity::mesh::QuadraticMultiImpact, &'static str> {
        self.impact_fragment_contacts_with_restitution(
            contacts,
            maximum_gap_m,
            0.,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
    /// Coupled normal restitution with atomic energy/momentum acceptance.
    pub fn impact_fragment_contacts_with_restitution(
        &mut self,
        contacts: &[QuadraticFragmentImpactConstraint],
        maximum_gap_m: f64,
        restitution: f64,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<crate::plasticity::mesh::QuadraticMultiImpact, &'static str> {
        if contacts.is_empty()
            || contacts.len() > 64
            || !maximum_gap_m.is_finite()
            || maximum_gap_m <= 0.
        {
            return Err("invalid owned coupled impact contacts");
        }
        let constraints: Result<Vec<_>, _> = contacts
            .iter()
            .map(|c| {
                let weights = self.fragment_contact_weights(
                    &c.first,
                    &c.second,
                    c.normal,
                    Some(maximum_gap_m),
                )?;
                Ok(crate::plasticity::mesh::QuadraticImpactConstraint {
                    weights,
                    normal: c.normal,
                })
            })
            .collect();
        let result = self.inertia.restitution_impacts(
            &self.velocities,
            &constraints?,
            restitution,
            velocity_tolerance_m_s,
            max_sweeps,
        )?;
        self.commit_fragment_impulse(
            &result.velocities,
            &result.nodal_impulse_n_s,
            result.dissipated_j,
        )?;
        Ok(result)
    }
}
impl super::FiniteQuadraticDynamics {
    pub fn impact_fragment_contacts(
        &mut self,
        contacts: &[QuadraticFragmentImpactConstraint],
        maximum_gap_m: f64,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<crate::plasticity::mesh::QuadraticMultiImpact, &'static str> {
        self.inner.impact_fragment_contacts(
            contacts,
            maximum_gap_m,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
}

impl super::FiniteQuadraticDynamics {
    pub fn impact_fragment_contacts_with_restitution(
        &mut self,
        contacts: &[QuadraticFragmentImpactConstraint],
        maximum_gap_m: f64,
        restitution: f64,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<crate::plasticity::mesh::QuadraticMultiImpact, &'static str> {
        self.inner.impact_fragment_contacts_with_restitution(
            contacts,
            maximum_gap_m,
            restitution,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
}
