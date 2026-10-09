//! Constraint generation within one staged contact solve.
//!
//! Rows discovered on a rejected full candidate belong to the tangent model,
//! never to geometry admission or published contact state. Rebuild the common
//! implicit solve at the original pose with these additional physical rows.
//! Retain them through accepted increments of this solve so the next free step
//! cannot immediately forget a newly discovered obstacle.
use super::*;

pub(super) struct ContactModel {
    mesh: Vec<Vec<RodContact>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovered_future_contact_blocks_free_motion_without_publishing_probe_pose() {
        let mut rod = HairRod::new(
            vec![[1e-3, 0., 0.], [1e-3, 0., 0.1], [1e-3, 0., 0.2]],
            HairMaterial::default(),
        )
        .unwrap();
        for point in 1..rod.x.len() {
            rod.predicted_x[point][0] -= 2e-3;
        }
        // An unrelated, open plane lets the common elastic solve expose its
        // free candidate. It must never attract the fiber toward that plane.
        rod.record_contact(
            1,
            0.,
            [1., 0., 0.],
            [-0.01, 0., 0.1],
            ContactSource::Mesh(0),
        );
        let original = rod.clone();
        let mut candidate = vec![rod];
        contact::reconcile_elastic_contact_positions_with_solver(
            &mut candidate,
            &mut [],
            1. / 240.,
            40e-6,
            None,
        )
        .unwrap();
        assert!(
            candidate[0].x[2][0] < 0.5e-3,
            "fixture must expose the undiscovered collision"
        );
        let row = candidate[0].record_contact(
            1,
            1.,
            [1., 0., 0.],
            [0.5e-3, 0., 0.2],
            ContactSource::Mesh(1),
        );
        candidate[0].contacts[row].metric_scale = 0.25;
        candidate[0].contacts[row].trajectory_time = Some(0.25);
        let mut model = ContactModel::new(1);
        assert_eq!(model.observe_rejected_candidate(&candidate), 1);
        assert_eq!(
            model.observe_rejected_candidate(&candidate),
            0,
            "repeated rejected probes must not grow the model"
        );
        let mut staged = vec![original.clone()];
        model.install(&mut staged);
        assert_eq!(
            staged[0].x, original.x,
            "model assembly must not publish candidate positions"
        );
        assert_eq!(staged[0].q, original.q);
        contact::reconcile_elastic_contact_positions_with_solver(
            &mut staged,
            &mut [],
            1. / 240.,
            40e-6,
            None,
        )
        .unwrap();
        let future = &staged[0].contacts[1];
        assert!(
            future.physical_gap(staged[0].x[2]) >= -1e-11,
            "new time-scaled row must constrain the same free elastic solve"
        );
        assert_eq!(staged[0].x[0], original.x[0]);
        // Model rows are solve-local. A geometry refresh owns publication and
        // removes every speculative row, including the unrelated open plane.
        contact::refresh_mesh_constraints(&mut staged[0], &[], 40e-6);
        assert!(staged[0].contacts.is_empty());
    }
}
impl ContactModel {
    pub fn new(count: usize) -> Self {
        Self {
            mesh: vec![Vec::new(); count],
        }
    }
    pub fn install(&self, rods: &mut [HairRod]) {
        for (rod, rows) in rods.iter_mut().zip(&self.mesh) {
            rod.contacts.extend(rows.iter().cloned());
        }
    }
    pub fn observe_rejected_candidate(&mut self, rods: &[HairRod]) -> usize {
        let mut added = 0;
        for (rod, rows) in rods.iter().zip(&mut self.mesh) {
            for contact in rod
                .contacts
                .iter()
                .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
            {
                let p = add(
                    mul(rod.x[contact.segment], 1. - contact.fraction),
                    mul(rod.x[contact.segment + 1], contact.fraction),
                );
                if contact.physical_gap(p) >= -1e-10 {
                    continue;
                }
                // Canonical physical endpoint Jacobians, including the time
                // metric. A stricter parallel row replaces its weaker alias.
                let jacobian = |c: &RodContact, end: usize| {
                    mul(
                        c.normal,
                        c.metric_scale
                            * if end == 0 {
                                1. - c.fraction
                            } else {
                                c.fraction
                            },
                    )
                };
                if let Some(existing) = rows.iter_mut().find(|row| {
                    row.source == contact.source
                        && row.segment == contact.segment
                        && (0..2).all(|end| {
                            len(sub(jacobian(row, end), jacobian(contact, end))) <= 1e-12
                        })
                }) {
                    let offset = |c: &RodContact| c.metric_scale * dot(c.normal, c.target);
                    if offset(contact) > offset(existing) + 1e-12 {
                        *existing = contact.clone();
                        added += 1;
                    }
                } else {
                    rows.push(contact.clone());
                    added += 1;
                }
            }
        }
        added
    }
}
