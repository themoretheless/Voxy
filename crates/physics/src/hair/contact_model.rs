//! Constraint generation within one staged contact solve.
//!
//! Rows discovered on a rejected full candidate belong to the tangent model,
//! never to geometry admission or published contact state. Rebuild the common
//! implicit solve at the original pose with these additional physical rows.
//! Retain them while retrying the same frozen pose. After an accepted pose
//! change, geometry and whole-path discovery own the next tangent model;
//! rejected-probe planes are not global halfspaces of a nonconvex surface.
use super::*;

/// Fixed-pose constraint generation and accepted nonlinear motion own
/// separate finite budgets. Adding a row is not a nonlinear pose update.
#[derive(Debug)]
pub(super) struct ContactWorkBudget {
    attempts: usize,
    accepted_steps: usize,
    attempts_at_pose: usize,
    accepted_step_limit: usize,
}
impl Default for ContactWorkBudget {
    fn default() -> Self {
        Self::with_accepted_step_limit(128).unwrap()
    }
}
impl ContactWorkBudget {
    pub(super) fn with_accepted_step_limit(limit: usize) -> Option<Self> {
        (1..=128).contains(&limit).then_some(Self {
            attempts: 0,
            accepted_steps: 0,
            attempts_at_pose: 0,
            accepted_step_limit: limit,
        })
    }
    pub(super) fn next_attempt(&mut self) -> Option<usize> {
        if self.accepted_steps >= self.accepted_step_limit || self.attempts_at_pose >= 128 {
            return None;
        }
        let iteration = self.attempts;
        self.attempts += 1;
        self.attempts_at_pose += 1;
        Some(iteration)
    }
    pub(super) fn accepted_step(&mut self) {
        self.accepted_steps += 1;
        self.attempts_at_pose = 0;
    }
    pub(super) fn counts(&self) -> (usize, usize, usize) {
        (self.attempts, self.accepted_steps, self.attempts_at_pose)
    }
}

/// One backtracking search owns immutable meshes, radius and collision modes.
/// Only identical binary64 centerlines and trajectory origins may reuse rows;
/// orientation still belongs to the freshly evaluated elastic objective.
#[derive(Default)]
pub(super) struct TrialGeometry {
    snapshot: Option<TrialGeometrySnapshot>,
}
struct TrialGeometrySnapshot {
    positions: Vec<(Vec<V>, Vec<V>)>,
    contacts: Vec<Vec<RodContact>>,
    pairs: Vec<contact::StrandResponse>,
}
impl TrialGeometry {
    pub(super) fn store(&mut self, rods: &[HairRod], pairs: &[contact::StrandResponse]) {
        self.snapshot = Some(TrialGeometrySnapshot {
            positions: rods
                .iter()
                .map(|r| (r.x.clone(), r.old_x.clone()))
                .collect(),
            contacts: rods.iter().map(|r| r.contacts.clone()).collect(),
            pairs: pairs.to_vec(),
        });
    }
    pub(super) fn restore(&self, rods: &mut [HairRod]) -> Option<Vec<contact::StrandResponse>> {
        let cached = self.snapshot.as_ref()?;
        let equal = |a: &[V], b: &[V]| {
            a.len() == b.len()
                && a.iter()
                    .flatten()
                    .zip(b.iter().flatten())
                    .all(|(a, b)| a.to_bits() == b.to_bits())
        };
        if rods.len() != cached.positions.len()
            || rods
                .iter()
                .zip(&cached.positions)
                .any(|(r, (x, old))| !equal(&r.x, x) || !equal(&r.old_x, old))
        {
            return None;
        }
        for (rod, rows) in rods.iter_mut().zip(&cached.contacts) {
            rod.contacts.clone_from(rows);
        }
        Some(cached.pairs.clone())
    }
}

pub(super) struct ContactModel {
    mesh: Vec<Vec<RodContact>>,
    strands: Vec<contact::StrandResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trial_geometry_reuses_rotations_only_and_invalidates_changed_swept_positions() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            Default::default(),
        )
        .unwrap();
        rod.record_point_contact(2, [1., 0., 0.], [40e-6, 0., 0.02], ContactSource::Mesh(0));
        let original = rod.clone();
        let mut cache = TrialGeometry::default();
        assert!(cache.restore(std::slice::from_mut(&mut rod)).is_none());
        cache.store(std::slice::from_ref(&rod), &[]);
        rod.contacts.clear();
        rod.q[1][0] += 1e-6;
        assert!(cache.restore(std::slice::from_mut(&mut rod)).is_some());
        assert_eq!(
            format!("{:?}", rod.contacts),
            format!("{:?}", original.contacts)
        );
        rod.x[1][2] = f64::from_bits(rod.x[1][2].to_bits() + 1);
        assert!(cache.restore(std::slice::from_mut(&mut rod)).is_none());
        rod.x.clone_from(&original.x);
        rod.old_x[1][2] = f64::from_bits(rod.old_x[1][2].to_bits() + 1);
        assert!(cache.restore(std::slice::from_mut(&mut rod)).is_none());
        rod.old_x.clone_from(&original.old_x);
        rod.x[1][0] = -0.;
        assert!(
            cache.restore(std::slice::from_mut(&mut rod)).is_none(),
            "signed-zero changes must preserve bitwise query identity"
        );
    }
    #[test]
    fn early_restoration_budget_counts_poses_without_shortening_fixed_pose_discovery() {
        assert!(ContactWorkBudget::with_accepted_step_limit(0).is_none());
        assert!(ContactWorkBudget::with_accepted_step_limit(129).is_none());
        let mut budget = ContactWorkBudget::with_accepted_step_limit(32).unwrap();
        for pose in 0..32 {
            for refinement in 0..4 {
                assert_eq!(budget.next_attempt(), Some(pose * 4 + refinement));
            }
            budget.accepted_step();
        }
        assert_eq!(budget.counts(), (128, 32, 0));
        assert!(budget.next_attempt().is_none());
        let mut fixed = ContactWorkBudget::with_accepted_step_limit(32).unwrap();
        for _ in 0..128 {
            assert!(fixed.next_attempt().is_some());
        }
        assert!(fixed.next_attempt().is_none());
    }
    #[test]
    fn captured_model_refinements_do_not_exhaust_nonlinear_pose_budget() {
        // Actual final failed transaction of coupled frame2:31 accepted
        // updates and97 model refinements, at most4 refinements at one pose.
        let accepted = [
            0, 1, 2, 4, 6, 10, 14, 17, 21, 24, 27, 31, 35, 39, 44, 49, 54, 59, 64, 69, 74, 79, 84,
            89, 94, 99, 104, 109, 114, 119, 124,
        ];
        let mut budget = ContactWorkBudget::default();
        for attempt in 0..128 {
            assert_eq!(budget.next_attempt(), Some(attempt));
            if accepted.contains(&attempt) {
                budget.accepted_step();
            }
        }
        assert_eq!(budget.counts(), (128, 31, 3));
        assert_eq!(
            budget.next_attempt(),
            Some(128),
            "fresh contact rows must not consume the accepted-pose budget"
        );
        let mut frozen = ContactWorkBudget::default();
        for _ in 0..128 {
            assert!(frozen.next_attempt().is_some());
        }
        assert!(
            frozen.next_attempt().is_none(),
            "fixed-pose refinement must remain bounded"
        );
        let mut moving = ContactWorkBudget::default();
        for _ in 0..128 {
            assert!(moving.next_attempt().is_some());
            moving.accepted_step();
        }
        assert!(
            moving.next_attempt().is_none(),
            "accepted nonlinear steps must remain bounded"
        );
    }
    #[test]
    fn strand_counterexamples_have_frozen_pose_lifetime_and_no_old_impulse() {
        let mut model = ContactModel::new(2);
        let pair = contact::StrandResponse {
            a: (0, 1, 0.25),
            b: (1, 2, 0.75),
            normal: [1., 0., 0.],
            impulse: 17.,
        };
        assert_eq!(model.observe_strands(&[pair.clone(), pair.clone()]), 1);
        assert_eq!(model.strand_rows().len(), 1);
        assert_eq!(model.strand_rows()[0].impulse, 0.);
        let mut distinct = pair.clone();
        distinct.a.2 = 0.5;
        assert_eq!(model.observe_strands(&[distinct]), 1);
        assert_eq!(model.observe_strands(&[pair]), 0);
        model.invalidate_after_pose_change();
        assert!(model.strand_rows().is_empty());
    }
    #[test]
    fn accepted_pose_rebuild_excludes_old_opposite_probe_plane() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap();
        let row =
            rod.record_point_contact(2, [-1., 0., 0.], [-40e-6, 0.02, 0.], ContactSource::Mesh(0));
        rod.contacts[row].trajectory_time = Some(1.);
        let mut model = ContactModel::new(1);
        assert_eq!(
            model.observe_rejected_candidate(std::slice::from_ref(&rod)),
            1
        );
        rod.contacts.clear();
        model.install(std::slice::from_mut(&mut rod));
        assert_eq!(
            rod.contacts.len(),
            1,
            "same-pose retry must retain its counterexample"
        );
        // The accepted nonlinear move now has a newly queried outward plane.
        // Installing the preceding probe would demand the opposite halfspace.
        rod.x[2][0] = 40e-6;
        rod.contacts.clear();
        rod.record_point_contact(2, [1., 0., 0.], [40e-6, 0.02, 0.], ContactSource::Mesh(0));
        let before = format!("{rod:?}");
        model.invalidate_after_pose_change();
        model.install(std::slice::from_mut(&mut rod));
        assert_eq!(
            format!("{rod:?}"),
            before,
            "model refresh must preserve fresh geometry and pose"
        );
        assert_eq!(rod.contacts.len(), 1);
        assert_eq!(rod.contacts[0].physical_gap(rod.x[2]), 0.);
    }
    #[test]
    fn feasible_origin_is_not_excluded_by_a_tolerance_band_counterexample() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            HairMaterial::default(),
        )
        .unwrap();
        let origin = rod.x.clone();
        rod.record_contact(
            1,
            0.,
            [1., 0., 0.],
            [5e-11, 0., 0.01],
            ContactSource::Mesh(0),
        );
        let mut admitted = rod.contacts[0].clone();
        admitted.target[0] = -5e-11;
        rod.contacts.push(admitted.clone());
        rod.x[1][0] = -1e-6;
        let mut model = ContactModel::new(1);
        assert_eq!(
            model.observe_rejected_candidate_admitting_origin(
                std::slice::from_ref(&rod),
                [origin.as_slice()]
            ),
            1
        );
        rod.x = origin;
        rod.contacts.clear();
        model.install(std::slice::from_mut(&mut rod));
        assert_eq!(rod.contacts.len(), 1);
        assert_eq!(
            rod.contacts[0].target, rod.x[1],
            "the origin-anchored plane dominates the parallel weaker plane"
        );
        assert!(
            rod.contacts
                .iter()
                .all(|row| row.physical_gap(rod.x[1]) >= 0.),
            "a feasible origin must never receive a speculative correction load"
        );
    }
    #[test]
    fn time_scaled_band_contact_blocks_inward_motion_without_loading_origin() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            Default::default(),
        )
        .unwrap();
        let origin = rod.x.clone();
        rod.record_contact(
            1,
            0.4,
            [1., 0., 0.],
            [8.8e-11 / 0.015, 0., 0.014],
            ContactSource::Mesh(0),
        );
        let mut row = rod.contacts[0].clone();
        row.metric_scale = 0.015;
        row.trajectory_time = Some(0.015);
        rod.x[1][0] = -7e-9;
        rod.x[2][0] = -7e-9;
        let local = ContactModel::approaching_mesh_row(&row, &rod.x, &origin).unwrap();
        let point = |x: &[V]| add(mul(x[1], 0.6), mul(x[2], 0.4));
        assert!(row.physical_gap(point(&origin)) < 0.);
        assert!(row.physical_gap(point(&origin)) >= -1e-10);
        assert_eq!(local.physical_gap(point(&origin)), 0.);
        assert!(local.physical_gap(point(&rod.x)) < 0.);
        assert_eq!(local.normal, row.normal);
        assert_eq!(local.metric_scale, row.metric_scale);
        assert_eq!(local.fraction, row.fraction);
        assert_eq!(local.trajectory_time, row.trajectory_time);
        // Exercise the actual implicit constrained solve, not only row math.
        let mut unloaded = rod.clone();
        unloaded.x.clone_from(&origin);
        unloaded.contacts = vec![local.clone()];
        contact::reconcile_elastic_contact_positions_with_solver(
            std::slice::from_mut(&mut unloaded),
            &mut [],
            1e-6,
            40e-6,
            None,
        )
        .unwrap();
        assert!(
            unloaded
                .x
                .iter()
                .zip(&origin)
                .all(|(a, b)| len(sub(*a, *b)) < 1e-14),
            "a band-anchored model must not push an unloaded feasible fiber"
        );
        let mut loaded = unloaded.clone();
        for i in 1..loaded.x.len() {
            loaded.predicted_x[i][0] -= 5e-6;
        }
        let mut free = loaded.clone();
        free.contacts.clear();
        // This entry point intentionally leaves contact-free owners alone.
        // A distant nonbinding plane activates the same elastic owner without
        // restricting its inward candidate, as in the full-groom solve.
        free.record_contact(
            1,
            0.4,
            [1., 0., 0.],
            [-0.01, 0., 0.014],
            ContactSource::Mesh(1),
        );
        contact::reconcile_elastic_contact_positions_with_solver(
            std::slice::from_mut(&mut free),
            &mut [],
            1e-6,
            40e-6,
            None,
        )
        .unwrap();
        assert!(
            local.physical_gap(point(&free.x)) < -1e-10,
            "the forcing must expose a genuine blocked inward direction"
        );
        contact::reconcile_elastic_contact_positions_with_solver(
            std::slice::from_mut(&mut loaded),
            &mut [],
            1e-6,
            40e-6,
            None,
        )
        .unwrap();
        assert!(
            local.physical_gap(point(&loaded.x)) >= -1e-12,
            "the same forcing must respect the installed time-scaled plane"
        );
        assert_eq!(loaded.x[0], origin[0]);
        row.target[0] = 1.01e-10 / row.metric_scale;
        assert!(
            ContactModel::approaching_mesh_row(&row, &rod.x, &origin).is_none(),
            "an origin outside the original tolerance cannot be rebased into feasibility"
        );
    }
    #[test]
    fn distinct_rejected_planes_survive_tangent_model_installation() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            HairMaterial::default(),
        )
        .unwrap();
        rod.record_contact(
            1,
            0.,
            [1., 0., 0.],
            [1e-8, 0., 0.01],
            ContactSource::Mesh(0),
        );
        // Install distinct authored rows directly to isolate model ownership
        // from the separate geometric contact recorder.
        let mut second = rod.contacts[0].clone();
        second.normal = [1., 5e-13, 0.];
        rod.contacts.push(second);
        let original = rod.contacts.clone();
        let mut model = ContactModel::new(1);
        model.observe_rejected_candidate(std::slice::from_ref(&rod));
        rod.contacts.clear();
        model.install(std::slice::from_mut(&mut rod));
        // At this independent probe the first plane admits the point, while
        // the second excludes it. They cannot share a model row.
        let probe = [1e-8, -1000., 0.01];
        assert_eq!(original[0].physical_gap(probe), 0.);
        assert!(original[1].physical_gap(probe) < -1e-10);
        for expected in original {
            assert!(
                rod.contacts
                    .iter()
                    .any(|row| row.normal == expected.normal && row.target == expected.target),
                "tangent model discarded a distinct rejected plane"
            );
        }
        assert_eq!(
            model.observe_rejected_candidate(std::slice::from_ref(&rod)),
            0,
            "exact repeated probes must not duplicate the model"
        );
    }
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
    pub fn invalidate_after_pose_change(&mut self) {
        for rows in &mut self.mesh {
            rows.clear();
        }
        self.strands.clear();
    }
    pub fn new(count: usize) -> Self {
        Self {
            mesh: vec![Vec::new(); count],
            strands: Vec::new(),
        }
    }
    pub fn strand_rows(&self) -> &[contact::StrandResponse] {
        &self.strands
    }
    pub fn observe_strands(&mut self, rows: &[contact::StrandResponse]) -> usize {
        let mut added = 0;
        for row in rows {
            if self
                .strands
                .iter()
                .any(|old| old.a == row.a && old.b == row.b && old.normal == row.normal)
            {
                continue;
            }
            let mut row = row.clone();
            row.impulse = 0.;
            self.strands.push(row);
            added += 1;
        }
        added
    }
    pub fn install(&self, rods: &mut [HairRod]) {
        if std::env::var_os("VOXY_HAIR_ELASTIC_DESCENT_TRACE").is_some() {
            for (name, retained) in [("fresh", false), ("retained", true)] {
                let mut count = 0;
                let mut excludes_origin = 0;
                let mut outside_admission = 0;
                let mut minimum = f64::INFINITY;
                for (rod, rows) in rods.iter().zip(&self.mesh) {
                    let contacts = if retained {
                        rows.as_slice()
                    } else {
                        rod.contacts.as_slice()
                    };
                    for c in contacts
                        .iter()
                        .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                    {
                        let p = add(
                            mul(rod.x[c.segment], 1. - c.fraction),
                            mul(rod.x[c.segment + 1], c.fraction),
                        );
                        let gap = c.physical_gap(p);
                        count += 1;
                        excludes_origin += usize::from(gap < 0.);
                        outside_admission += usize::from(gap < -1e-10);
                        minimum = minimum.min(gap);
                    }
                }
                eprintln!(
                    "HAIR MODEL ORIGIN kind={name} rows={count} negative_gaps={excludes_origin} outside_admission={outside_admission} minimum_gap_m={minimum:e}"
                );
            }
        }
        for (rod, rows) in rods.iter_mut().zip(&self.mesh) {
            rod.contacts.extend(rows.iter().cloned());
        }
    }
    pub fn observe_rejected_candidate(&mut self, rods: &[HairRod]) -> usize {
        let rows: Vec<Vec<RodContact>> = rods
            .iter()
            .map(|rod| {
                rod.contacts
                    .iter()
                    .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                    .filter(|c| {
                        let p = add(
                            mul(rod.x[c.segment], 1. - c.fraction),
                            mul(rod.x[c.segment + 1], c.fraction),
                        );
                        c.physical_gap(p) < -1e-10
                    })
                    .cloned()
                    .collect()
            })
            .collect();
        self.observe_rows(&rows)
    }
    /// Build a local approaching-contact plane without a correction load at
    /// the already feasible origin. Within the existing admission band, pin
    /// the plane to that origin: it forbids further inward normal motion.
    /// The trial must violate the original physical row; whole-path admission
    /// still owns safety and this model is discarded after a pose change.
    pub(super) fn approaching_mesh_row(
        row: &RodContact,
        trial: &[V],
        origin: &[V],
    ) -> Option<RodContact> {
        if !matches!(row.source, ContactSource::Mesh(_)) {
            return None;
        }
        let point = |x: &[V]| {
            add(
                mul(x[row.segment], 1. - row.fraction),
                mul(x[row.segment + 1], row.fraction),
            )
        };
        let start = point(origin);
        let end = point(trial);
        let initial_gap = row.physical_gap(start);
        let trial_gap = row.physical_gap(end);
        if !initial_gap.is_finite()
            || !trial_gap.is_finite()
            || initial_gap < -1e-10
            || trial_gap >= -1e-10
        {
            return None;
        }
        let mut local = row.clone();
        if initial_gap < 0. {
            local.target = start;
        }
        // Exact subtraction from the stored anchor gives zero at the origin.
        // Recheck the rounded row: no positive load or nonblocking probe may
        // enter the local constrained solve.
        if local.physical_gap(start) < 0. || local.physical_gap(end) >= 0. {
            return None;
        }
        Some(local)
    }
    pub fn observe_rejected_candidate_admitting_origin<'a>(
        &mut self,
        rods: &[HairRod],
        origins: impl IntoIterator<Item = &'a [V]>,
    ) -> usize {
        let rows: Vec<Vec<RodContact>> = rods
            .iter()
            .zip(origins)
            .map(|(rod, origin)| {
                rod.contacts
                    .iter()
                    .filter_map(|row| Self::approaching_mesh_row(row, &rod.x, origin))
                    .collect()
            })
            .collect();
        self.observe_rows(&rows)
    }
    // Callers own locality and original-pose admission of these counterexamples.
    pub fn observe_rows(&mut self, candidates: &[Vec<RodContact>]) -> usize {
        let mut added = 0;
        for (contacts, rows) in candidates.iter().zip(&mut self.mesh) {
            for contact in contacts {
                // Canonical physical endpoint Jacobians, including the time
                // metric. Only equal endpoint Jacobians can share an owner;
                // nearby planes may exclude different candidate motions.
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
                        && (0..2).all(|end| jacobian(row, end) == jacobian(contact, end))
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
