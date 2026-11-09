//! Nonlinear position contact reconciliation, merit functions, and elastic descent checks for [`HairSystem`].
use super::{
    ContactSource, HairContactProjectionDiagnostic, HairLinearSolver, HairProjectionTrace, HairRod,
    HairSystem, TriangleMesh, contact, contact_model, contact_replay, direct, math::*, mesh_motion,
};

#[derive(Default)]
struct ReconcileTraceFlags {
    elastic_descent: bool,
    contact_parts: bool,
    nonlinear_merit: bool,
    contact_rows: bool,
    rejected_direction_export: Option<std::ffi::OsString>,
    replay_export: Option<std::ffi::OsString>,
}

impl ReconcileTraceFlags {
    fn from_env() -> Self {
        Self {
            elastic_descent: std::env::var_os("VOXY_HAIR_ELASTIC_DESCENT_TRACE").is_some(),
            contact_parts: std::env::var_os("VOXY_HAIR_CONTACT_PARTS_TRACE").is_some(),
            nonlinear_merit: std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some(),
            contact_rows: std::env::var_os("VOXY_HAIR_CONTACT_ROWS_TRACE").is_some(),
            rejected_direction_export: std::env::var_os(
                "VOXY_HAIR_REJECTED_CONTACT_DIRECTION_EXPORT",
            ),
            replay_export: std::env::var_os("VOXY_HAIR_CONTACT_REPLAY_EXPORT"),
        }
    }
}

impl HairSystem {
    pub(super) fn reconcile_positions(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        dt: f64,
        radius: f64,
        self_collision: bool,
        history: &mut Vec<contact::StrandResponse>,
        trace: Option<HairProjectionTrace<'_>>,
    ) -> Result<(), &'static str> {
        Self::reconcile_positions_with_solver(
            rods,
            meshes,
            dt,
            radius,
            self_collision,
            history,
            None,
            trace,
        )
    }

    pub(super) fn reconcile_positions_with_solver(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        dt: f64,
        radius: f64,
        self_collision: bool,
        history: &mut Vec<contact::StrandResponse>,
        solver: Option<&mut dyn HairLinearSolver>,
        trace: Option<HairProjectionTrace<'_>>,
    ) -> Result<(), &'static str> {
        Self::reconcile_positions_mode(
            rods,
            meshes,
            dt,
            radius,
            self_collision,
            history,
            solver,
            trace,
            false,
        )
    }

    pub(super) fn refresh_mesh_geometry(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        radius: f64,
        continuous: bool,
    ) -> Result<(), &'static str> {
        for rod in rods.iter_mut() {
            contact::refresh_mesh_constraints(rod, meshes, radius);
        }
        if continuous {
            mesh_motion::refresh(rods, meshes, radius)?;
        }
        Ok(())
    }

    pub(super) fn reconcile_positions_mode(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        dt: f64,
        radius: f64,
        self_collision: bool,
        history: &mut Vec<contact::StrandResponse>,
        solver: Option<&mut dyn HairLinearSolver>,
        trace: Option<HairProjectionTrace<'_>>,
        continuous_mesh: bool,
    ) -> Result<(), &'static str> {
        Self::reconcile_positions_mode_with_first_stage_limit(
            rods,
            meshes,
            dt,
            radius,
            self_collision,
            history,
            solver,
            trace,
            continuous_mesh,
            32,
        )
    }

    pub(super) fn reconcile_positions_mode_with_first_stage_limit(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        dt: f64,
        radius: f64,
        self_collision: bool,
        history: &mut Vec<contact::StrandResponse>,
        mut solver: Option<&mut dyn HairLinearSolver>,
        mut trace: Option<HairProjectionTrace<'_>>,
        continuous_mesh: bool,
        first_stage_limit: usize,
    ) -> Result<(), &'static str> {
        if !(1..=128).contains(&first_stage_limit) {
            return Err("invalid initial contact work budget");
        }
        let flags = ReconcileTraceFlags::from_env();
        let original_history_len = history.len();
        // Keep the original pose for fixed roots. An infeasible pose is not an
        // objective-descent reference; minimization starts at the admitted
        // restoration proposal, never at an exhausted Newton iterate.
        let recovery_origin = (continuous_mesh && self_collision).then(|| rods.to_vec());
        let mut feasible_reference: Option<Vec<(Vec<V>, Vec<Q>)>> = None;
        if recovery_origin.is_some() {
            if let Some(path) = &flags.replay_export {
                let path = std::path::Path::new(path).with_extension("recovery-entry.vhr");
                match contact_replay::save_measured(&path, rods, meshes, dt, radius, true, true) {
                    Ok(()) => eprintln!("HAIR CONTACT ENTRY EXPORT path={}", path.display()),
                    Err(error) => eprintln!("HAIR CONTACT ENTRY EXPORT ERROR {error}"),
                }
            }
        }
        for attempt in 0..2 {
            // A bounded first stage hands difficult continuous self-contact
            // to verified restoration instead of spending the entire native
            // budget on short updates. The resumed solve keeps its full budget
            // and must establish ordinary tangent and physical admission.
            let accepted_step_limit = if attempt == 0 && continuous_mesh && self_collision {
                first_stage_limit
            } else {
                128
            };
            let current_trace = trace.as_mut().map(|trace| HairProjectionTrace {
                rod: trace.rod,
                substep: trace.substep,
                structural_iteration: trace.structural_iteration,
                output: &mut *trace.output,
            });
            let result = match solver.as_mut() {
                Some(backend) => Self::reconcile_positions_attempt(
                    rods,
                    meshes,
                    dt,
                    radius,
                    self_collision,
                    history,
                    Some(&mut **backend),
                    current_trace,
                    continuous_mesh,
                    accepted_step_limit,
                ),
                None => Self::reconcile_positions_attempt(
                    rods,
                    meshes,
                    dt,
                    radius,
                    self_collision,
                    history,
                    None,
                    current_trace,
                    continuous_mesh,
                    accepted_step_limit,
                ),
            };
            match result {
                Ok(()) => {
                    if attempt != 0 {
                        let origin = recovery_origin.as_ref().unwrap();
                        let reference = feasible_reference.as_ref().unwrap();
                        // Native convergence is necessary, but the complete
                        // minimization must retain the admitted feasible
                        // reference and complete physical trajectory admission.
                        if Self::elastic_energy_change(rods, reference, dt)? > 0. {
                            return Err("contact recovery increased feasible-reference potential");
                        }
                        if rods
                            .iter()
                            .zip(origin)
                            .any(|(rod, start)| rod.x[0] != start.x[0])
                        {
                            return Err("contact recovery moved a prescribed root");
                        }
                        contact::admit_staged_strands(rods, radius)?;
                        mesh_motion::admit(rods, meshes, radius)?;
                    }
                    return Ok(());
                }
                Err(error) => {
                    if attempt != 0
                        || !continuous_mesh
                        || !self_collision
                        || error != "joint hair contact position linearization did not converge"
                    {
                        return Err(error);
                    }
                    if let Some(path) = &flags.replay_export {
                        let path = std::path::Path::new(path);
                        for (extension, state) in [
                            (
                                "recovery-input.vhr",
                                recovery_origin.as_ref().unwrap().as_slice(),
                            ),
                            ("recovery-terminal.vhr", &*rods),
                        ] {
                            if let Err(export_error) = contact_replay::save_measured(
                                &path.with_extension(extension),
                                state,
                                meshes,
                                dt,
                                radius,
                                true,
                                true,
                            ) {
                                eprintln!(
                                    "HAIR CONTACT RECOVERY EXPORT ERROR {extension}: {export_error}"
                                );
                            }
                        }
                    }
                    let mut candidate = recovery_origin.as_ref().unwrap().clone();
                    let restored = match solver.as_mut() {
                        Some(backend) => contact::restore_candidate(
                            &mut candidate,
                            meshes,
                            dt,
                            radius,
                            Some(&mut **backend),
                        ),
                        None => {
                            contact::restore_candidate(&mut candidate, meshes, dt, radius, None)
                        }
                    };
                    match restored {
                        Ok(true) => {
                            feasible_reference = Some(
                                candidate
                                    .iter()
                                    .map(|rod| (rod.x.clone(), rod.q.clone()))
                                    .collect::<Vec<_>>(),
                            );
                            if let Some(path) = &flags.replay_export {
                                let path = std::path::Path::new(path)
                                    .with_extension("recovery-proposal.vhr");
                                // Recovery already measured zero contact merit
                                // and admitted both complete physical paths.
                                if let Err(export_error) = contact_replay::save(
                                    &path, &candidate, meshes, dt, radius, true, 0., true,
                                ) {
                                    eprintln!(
                                        "HAIR CONTACT RECOVERY EXPORT ERROR proposal: {export_error}"
                                    );
                                }
                            }
                            rods.clone_from_slice(&candidate);
                            // Failed-attempt reactions must not leak into the
                            // resumed solve. A proposal is never completion.
                            history.truncate(original_history_len);
                            if flags.nonlinear_merit {
                                eprintln!(
                                    "HAIR CONTACT RECOVERY admitted=true native_reconciliation_required=true"
                                );
                            }
                        }
                        Ok(false) => {
                            if flags.nonlinear_merit {
                                eprintln!(
                                    "HAIR CONTACT RECOVERY admitted=false bounded_proposal_exhausted=true"
                                );
                            }
                            return Err(error);
                        }
                        Err(recovery_error) => {
                            if flags.nonlinear_merit {
                                eprintln!(
                                    "HAIR CONTACT RECOVERY admitted=false error={recovery_error:?}"
                                );
                            }
                            return Err(error);
                        }
                    }
                }
            }
        }
        unreachable!("bounded native contact attempts return inside loop")
    }

    pub(super) fn reconcile_positions_attempt(
        rods: &mut [HairRod],
        meshes: &[TriangleMesh],
        dt: f64,
        radius: f64,
        self_collision: bool,
        history: &mut Vec<contact::StrandResponse>,
        mut solver: Option<&mut dyn HairLinearSolver>,
        mut trace: Option<HairProjectionTrace<'_>>,
        continuous_mesh: bool,
        accepted_step_limit: usize,
    ) -> Result<(), &'static str> {
        let flags = ReconcileTraceFlags::from_env();
        let mut last_worst = None;
        let mut first_gap = 0.;
        let mut merit_history = Vec::with_capacity(128);
        let mut energy_change_history = Vec::with_capacity(128);
        let mut model = contact_model::ContactModel::new(rods.len());
        let motion_start = (continuous_mesh && self_collision).then(|| {
            let mut start = rods.to_vec();
            for rod in &mut start {
                rod.x.clone_from(&rod.old_x);
                rod.q.clone_from(&rod.old_q);
            }
            start
        });
        let mut work =
            contact_model::ContactWorkBudget::with_accepted_step_limit(accepted_step_limit)
                .ok_or("invalid hair contact work budget")?;
        // This attempt owns immutable meshes, radius, collision modes and rod
        // topology. A rollback can prepare the next fixed-pose query once.
        let mut refinement_geometry = contact_model::TrialGeometry::default();
        let mut worst_scratch: Vec<(f64, f64)> = Vec::new();
        while let Some(iteration) = work.next_attempt() {
            let mut current = if let Some(pairs) = refinement_geometry.restore(rods) {
                pairs
            } else {
                let pairs = if self_collision {
                    contact::refresh_strand_responses(rods, radius, &[])
                } else {
                    Vec::new()
                };
                Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
                pairs
            };
            let merit = Self::contact_merit(rods, &current, radius)?;
            let feasible_energies = if merit == 0. {
                Some(
                    rods.iter()
                        .map(|rod| direct::structural_energy(rod, dt))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            let feasible_energy = feasible_energies
                .as_ref()
                .map(|values| values.iter().sum::<f64>());
            let energy_trace = flags.elastic_descent;
            let energy_gradients = if energy_trace && merit == 0. {
                if let Some(start) = &motion_start {
                    let end: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
                    let path = contact::strand_fraction(start, &end, radius);
                    eprintln!(
                        "HAIR ELASTIC ORIGIN PATH iteration={iteration} endpoint_merit={merit:e} strand_fraction={path:?}"
                    );
                }
                Some(
                    rods.iter()
                        .map(|rod| {
                            let mut staged = rod.clone();
                            staged.contacts.clear();
                            direct::assemble(&mut staged, dt).map(|(_, rhs)| rhs)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            if flags.contact_parts {
                Self::trace_contact_parts_with_flag(
                    rods,
                    &current,
                    radius,
                    iteration,
                    0.,
                    flags.contact_rows,
                )?;
            }
            let before: Vec<_> = rods
                .iter()
                .map(|rod| (rod.x.clone(), rod.q.clone()))
                .collect();
            let before_contacts = Some(
                rods.iter()
                    .map(|rod| rod.contacts.clone())
                    .collect::<Vec<_>>(),
            );
            let mut snapshot = trace.as_ref().map(|trace| {
                HairContactProjectionDiagnostic::begin(trace, iteration, rods, &current)
            });
            let before_pairs = current.clone();
            if continuous_mesh {
                model.install(rods);
            }
            let projection = if let Some(start) = &motion_start {
                let mut reactions = Vec::new();
                let result = if let Some(backend) = solver.as_mut() {
                    contact::reconcile_swept_contact_owners_with_pairs(
                        rods,
                        dt,
                        radius,
                        Some(&mut **backend),
                        &mut reactions,
                        Some(start),
                        model.strand_rows(),
                    )
                } else {
                    contact::reconcile_swept_contact_owners_with_pairs(
                        rods,
                        dt,
                        radius,
                        None,
                        &mut reactions,
                        Some(start),
                        model.strand_rows(),
                    )
                };
                current = reactions;
                result.map(|fraction| fraction == 1.)
            } else if continuous_mesh {
                if let Some(backend) = solver.as_mut() {
                    contact::reconcile_elastic_contact_positions_with_solver(
                        rods,
                        &mut current,
                        dt,
                        radius,
                        Some(&mut **backend),
                    )
                } else {
                    contact::reconcile_elastic_contact_positions_with_solver(
                        rods,
                        &mut current,
                        dt,
                        radius,
                        None,
                    )
                }
            } else if let Some(backend) = solver.as_mut() {
                contact::reconcile_contact_positions_with_solver(
                    rods,
                    &mut current,
                    dt,
                    radius,
                    Some(&mut **backend),
                )
            } else {
                contact::reconcile_contact_positions(rods, &mut current, dt, radius)
            };
            let mut complete = projection.inspect_err(|_| {
                if let Some(path) = &flags.replay_export {
                    let path = std::path::Path::new(path);
                    if let Err(export_error) = contact_replay::save(
                        path,
                        rods,
                        meshes,
                        dt,
                        radius,
                        self_collision,
                        merit,
                        continuous_mesh,
                    ) {
                        eprintln!("HAIR CONTACT REPLAY EXPORT ERROR {export_error}");
                    }
                    let mut rows = Vec::new();
                    for (r, rod) in rods.iter().enumerate() {
                        for c in &rod.contacts {
                            if !matches!(c.source, ContactSource::Mesh(_)) {
                                continue;
                            }
                            rows.push(format!(
                                "{{\"rod\":{r},\"segment\":{},\"fraction\":{:?},\"metric_scale\":{:?},\"time\":{},\"normal\":{:?},\"target\":{:?},\"endpoints\":{:?}}}",
                                c.segment,
                                c.fraction,
                                c.metric_scale,
                                c.trajectory_time.map_or("null".to_owned(), |t| format!("{t:?}")),
                                c.normal,
                                c.target,
                                &rod.x[c.segment..=c.segment + 1]
                            ));
                        }
                    }
                    if let Err(export_error) =
                        std::fs::write(path.with_extension("contacts.json"), format!("[{}]", rows.join(",")))
                    {
                        eprintln!("HAIR CONTACT ROW EXPORT ERROR {export_error}");
                    }
                }
            })?;
            let mut fresh = if self_collision {
                contact::refresh_strand_responses(rods, radius, &[])
            } else {
                Vec::new()
            };
            Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
            let full_merit = Self::contact_merit(rods, &fresh, radius)?;
            let full_merit = if continuous_mesh
                && self_collision
                && complete
                && merit == 0.
                && full_merit > 0.
            {
                // Retain the feasible pre-step energy reference,
                // repair the complete trial, then re-query every physical gate.
                // Failure leaves the ordinary full trial and backtracking intact.
                let mut candidate = rods.to_vec();
                let restored = match solver.as_mut() {
                    Some(backend) => contact::restore_second_order_candidate(
                        &mut candidate,
                        meshes,
                        dt,
                        radius,
                        &before,
                        Some(&mut **backend),
                    ),
                    None => contact::restore_second_order_candidate(
                        &mut candidate,
                        meshes,
                        dt,
                        radius,
                        &before,
                        None,
                    ),
                };
                match restored {
                    Ok(true) => {
                        rods.clone_from_slice(&candidate);
                        fresh = contact::refresh_strand_responses(rods, radius, &[]);
                        Self::refresh_mesh_geometry(rods, meshes, radius, true)?;
                        let corrected = Self::contact_merit(rods, &fresh, radius)?;
                        if flags.nonlinear_merit {
                            eprintln!(
                                "HAIR CONTACT SOC iteration={iteration} admitted=true uncorrected_merit={full_merit:e} corrected_merit={corrected:e}"
                            );
                        }
                        corrected
                    }
                    result => {
                        if flags.nonlinear_merit {
                            eprintln!(
                                "HAIR CONTACT SOC iteration={iteration} admitted=false result={result:?}"
                            );
                        }
                        full_merit
                    }
                }
            } else {
                full_merit
            };
            if let Some(gradients) = &energy_gradients {
                let mut slope = 0.;
                let mut max_translation_m = 0f64;
                let mut max_rotation_rad = 0f64;
                for ((rod, (x, q)), rhs) in rods.iter().zip(&before).zip(gradients) {
                    for i in 1..rod.x.len() {
                        let delta = sub(rod.x[i], x[i]);
                        max_translation_m = max_translation_m.max(len(delta));
                        for axis in 0..3 {
                            slope -= rhs[i * 6 + axis] * delta[axis];
                        }
                    }
                    for i in 1..rod.q.len() {
                        let delta = log(qm(rod.q[i], conj(q[i])));
                        max_rotation_rad = max_rotation_rad.max(len(delta));
                        for axis in 0..3 {
                            slope -= rhs[i * 6 + 3 + axis] * delta[axis];
                        }
                    }
                }
                eprintln!(
                    "HAIR ELASTIC DIRECTION iteration={iteration} energy_j={:?} slope_j={slope:e} full_merit={full_merit:e} max_translation_m={max_translation_m:e} max_rotation_rad={max_rotation_rad:e} max_velocity_change_m_s={:e} max_angular_velocity_change_rad_s={:e}",
                    feasible_energy,
                    max_translation_m / dt,
                    max_rotation_rad / dt
                );
            }
            if flags.contact_parts {
                Self::trace_contact_parts_with_flag(
                    rods,
                    &fresh,
                    radius,
                    iteration,
                    1.,
                    flags.contact_rows,
                )?;
            }
            let mut accepted_scale = 1.;
            // Audit the full step separately from shortened trials, as in
            // IPC's feasibility/energy diagnostics. This is observation only:
            // constrained optimality must be established before changing the
            // completion gate, especially after initial overlap restoration.
            if energy_trace && full_merit == 0. {
                if let Some(before_energy) = feasible_energy {
                    let energy = rods
                        .iter()
                        .map(|rod| direct::structural_energy(rod, dt))
                        .collect::<Result<Vec<_>, _>>()?
                        .iter()
                        .sum::<f64>();
                    eprintln!(
                        "HAIR ELASTIC FULL iteration={iteration} before_j={before_energy:e} trial_j={energy:e} delta_j={:e}",
                        energy - before_energy
                    );
                }
            }
            if full_merit > merit {
                let full: Vec<_> = rods
                    .iter()
                    .map(|rod| (rod.x.clone(), rod.q.clone()))
                    .collect();
                let mut slope = 0.;
                for (r, contacts) in before_contacts.as_ref().unwrap().iter().enumerate() {
                    worst_scratch.clear();
                    worst_scratch.resize(rods[r].lengths.len(), (0f64, 0f64));
                    for c in contacts
                        .iter()
                        .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                    {
                        let i = c.segment;
                        let t = c.fraction;
                        let p = add(mul(before[r].0[i], 1. - t), mul(before[r].0[i + 1], t));
                        let d = add(
                            mul(sub(full[r].0[i], before[r].0[i]), 1. - t),
                            mul(sub(full[r].0[i + 1], before[r].0[i + 1]), t),
                        );
                        let depth = (-c.physical_gap(p) - 1e-10).max(0.);
                        let derivative = -dot(c.normal, d) * c.metric_scale;
                        if depth > worst_scratch[i].0 {
                            worst_scratch[i] = (depth, derivative);
                        } else if depth == worst_scratch[i].0 {
                            worst_scratch[i].1 = worst_scratch[i].1.max(derivative);
                        }
                    }
                    slope += worst_scratch
                        .iter()
                        .map(|(depth, derivative)| 2. * depth * derivative)
                        .sum::<f64>();
                }
                for pair in &before_pairs {
                    let (a, i, u) = pair.a;
                    let (b, j, v) = pair.b;
                    let pa = add(mul(before[a].0[i], 1. - u), mul(before[a].0[i + 1], u));
                    let pb = add(mul(before[b].0[j], 1. - v), mul(before[b].0[j + 1], v));
                    let da = add(
                        mul(sub(full[a].0[i], before[a].0[i]), 1. - u),
                        mul(sub(full[a].0[i + 1], before[a].0[i + 1]), u),
                    );
                    let db = add(
                        mul(sub(full[b].0[j], before[b].0[j]), 1. - v),
                        mul(sub(full[b].0[j + 1], before[b].0[j + 1]), v),
                    );
                    let distance = len(sub(pa, pb));
                    let derivative = if distance > 0. {
                        dot(sub(pa, pb), sub(da, db)) / distance
                    } else {
                        len(sub(da, db))
                    };
                    slope -= 2. * (2. * radius - distance - 1e-10).max(0.) * derivative;
                }
                let backtrack_scale = |scale: f64, value: f64| {
                    let denominator = 2. * (value - merit - scale * slope);
                    let candidate = if slope < 0. && denominator > 0. {
                        -slope * scale * scale / denominator
                    } else {
                        0.5 * scale
                    };
                    if candidate.is_finite() {
                        candidate.clamp(0.1 * scale, 0.5 * scale)
                    } else {
                        0.5 * scale
                    }
                };
                let mut scale = backtrack_scale(1., full_merit);
                let mut last_trial_scale = scale;
                let mut accepted = None;
                let mut nearby_counterexample = None;
                let mut nearby_pair_counterexample = None;
                let mut trial_geometry = contact_model::TrialGeometry::default();
                // Closest features can change at different scales. Search the
                // actual refreshed geometry rather than testing only one half step.
                // Twelve halvings can stop before a representable feasible
                // prefix exists (captured frame4 still had1.84nm excess).
                // Bound work by binary64 precision, not an arbitrary scale
                // floor; stop earlier when rounding leaves the pose unchanged.
                for _backtrack in 1..=f64::MANTISSA_DIGITS {
                    last_trial_scale = scale;
                    for ((rod, (x, q)), (full_x, full_q)) in rods.iter_mut().zip(&before).zip(&full)
                    {
                        for i in 1..rod.x.len() {
                            rod.x[i] = add(x[i], mul(sub(full_x[i], x[i]), scale));
                        }
                        for i in 1..rod.q.len() {
                            rod.q[i] = q[i];
                            apply(&mut rod.q[i], mul(log(qm(full_q[i], conj(q[i]))), scale));
                        }
                    }
                    if rods
                        .iter()
                        .zip(&before)
                        .all(|(rod, (x, q))| rod.x == *x && rod.q == *q)
                    {
                        break;
                    }
                    let trial_pairs = if let Some(pairs) = trial_geometry.restore(rods) {
                        if flags.nonlinear_merit {
                            eprintln!(
                                "HAIR CONTACT TRIAL GEOMETRY iteration={iteration} scale={scale:e} reused=true"
                            );
                        }
                        pairs
                    } else {
                        let pairs = if self_collision {
                            contact::refresh_strand_responses(rods, radius, &[])
                        } else {
                            Vec::new()
                        };
                        Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
                        trial_geometry.store(rods, &pairs);
                        pairs
                    };
                    let trial_merit = Self::contact_merit(rods, &trial_pairs, radius)?;
                    if flags.contact_parts {
                        Self::trace_contact_parts_with_flag(
                            rods,
                            &trial_pairs,
                            radius,
                            iteration,
                            scale,
                            flags.contact_rows,
                        )?;
                    }
                    // Feasibility has no strictly decreasing direction once
                    // its merit is zero. Continue the actual implicit elastic
                    // minimization, while preserving exactly zero excess.
                    let elastic_descent = if trial_merit == 0. {
                        if let Some(before_energy) = feasible_energy {
                            let residual_delta = Self::elastic_energy_change(rods, &before, dt)?;
                            if energy_trace {
                                let energies = rods
                                    .iter()
                                    .map(|rod| direct::structural_energy(rod, dt))
                                    .collect::<Result<Vec<_>, _>>()?;
                                let energy = energies.iter().sum::<f64>();
                                let delta = energies
                                    .iter()
                                    .zip(feasible_energies.as_ref().unwrap())
                                    .map(|(a, b)| a - b)
                                    .sum::<f64>();
                                eprintln!(
                                    "HAIR ELASTIC TRIAL iteration={iteration} scale={scale:e} before_j={before_energy:e} trial_j={energy:e} global_delta_j={:e} rod_delta_sum_j={delta:e} residual_delta_j={residual_delta:e}",
                                    energy - before_energy
                                );
                            }
                            residual_delta < 0.
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if (trial_merit < merit || elastic_descent) && trial_merit < full_merit {
                        accepted = Some((scale, trial_pairs));
                        break;
                    }
                    if continuous_mesh {
                        // Keep only the nearest rejected trial, and only
                        // planes that admit the frozen pose. This supplies
                        // approaching contacts without importing a far-side
                        // plane that already excludes the pose being solved.
                        let rows = rods
                            .iter()
                            .zip(&before)
                            .map(|(rod, (x, _))| {
                                rod.contacts
                                    .iter()
                                    .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                                    .filter_map(|c| {
                                        if merit == 0. {
                                            contact_model::ContactModel::approaching_mesh_row(
                                                c, &rod.x, x,
                                            )
                                        } else {
                                            let p = add(
                                                mul(rod.x[c.segment], 1. - c.fraction),
                                                mul(rod.x[c.segment + 1], c.fraction),
                                            );
                                            let origin = add(
                                                mul(x[c.segment], 1. - c.fraction),
                                                mul(x[c.segment + 1], c.fraction),
                                            );
                                            (c.physical_gap(p) < -1e-10
                                                && c.physical_gap(origin) >= -1e-10)
                                                .then(|| c.clone())
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        // A smaller failed trial may contain only the already
                        // violated origin rows. It is not a counterexample to
                        // the approaching-contact model and must not erase
                        // the nearest trial that actually supplied one.
                        if rows.iter().any(|rows| !rows.is_empty()) {
                            nearby_counterexample = Some(rows);
                        }
                    }
                    if continuous_mesh && self_collision {
                        let rows: Vec<_> = trial_pairs
                            .iter()
                            .filter(|pair| {
                                let (a, i, u) = pair.a;
                                let (b, j, v) = pair.b;
                                let point = |positions: &[V], segment: usize, t: f64| {
                                    add(
                                        mul(positions[segment], 1. - t),
                                        mul(positions[segment + 1], t),
                                    )
                                };
                                let trial = sub(point(&rods[a].x, i, u), point(&rods[b].x, j, v));
                                let origin =
                                    sub(point(&before[a].0, i, u), point(&before[b].0, j, v));
                                len(trial) < 2. * radius - 1e-10
                                    && dot(origin, pair.normal)
                                        >= 2. * radius - (if merit == 0. { 0. } else { 1e-10 })
                            })
                            .cloned()
                            .collect();
                        if !rows.is_empty() {
                            nearby_pair_counterexample = Some(rows);
                        }
                    }
                    scale = backtrack_scale(scale, trial_merit);
                }
                let added_pairs = nearby_pair_counterexample
                    .as_ref()
                    .map_or(0, |rows| model.observe_strands(rows));
                let added_mesh = nearby_counterexample
                    .as_ref()
                    .map_or(0, |rows| model.observe_rows(rows));
                // A fully certified feasible descent changes the nonlinear pose.
                // Rejected larger-step probe planes belong to the preceding
                // pose and are invalidated below after this accepted move.
                // A newly discovered plane pinned to the feasible origin
                // must participate in the next tangent solve. Accepting an
                // infinitesimal prefix here would immediately discard it and
                // repeat the same blocked direction at the next pose.
                let origin_tangent = added_mesh > 0
                    && nearby_counterexample.as_ref().is_some_and(|rows| {
                        rows.iter().zip(&before).any(|(rows, (x, _))| {
                            rows.iter().any(|c| {
                                let p = add(
                                    mul(x[c.segment], 1. - c.fraction),
                                    mul(x[c.segment + 1], c.fraction),
                                );
                                c.physical_gap(p) == 0.
                            })
                        })
                    });
                let certified_prefix =
                    if added_mesh + added_pairs > 0 && merit == 0. && !origin_tangent {
                        if let Some((_, pairs)) = &accepted {
                            Self::contact_merit(rods, pairs, radius)? == 0.
                                && Self::feasible_elastic_prefix_admitted_with_flag(
                                    rods,
                                    meshes,
                                    &before,
                                    dt,
                                    radius,
                                    self_collision,
                                    flags.nonlinear_merit,
                                )?
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                if certified_prefix && flags.nonlinear_merit {
                    eprintln!(
                        "HAIR CONTACT FEASIBLE PREFIX iteration={iteration} probe_rows={} probe_pairs={} tangent_complete=false",
                        added_mesh, added_pairs
                    );
                }
                if added_mesh + added_pairs > 0 && !certified_prefix {
                    for (rod, (x, q)) in rods.iter_mut().zip(&before) {
                        rod.x.clone_from(x);
                        rod.q.clone_from(q);
                    }
                    // Preserve fresh-query ordering: strand rows, then mesh rows.
                    let rollback_pairs = if self_collision {
                        contact::refresh_strand_responses(rods, radius, &[])
                    } else {
                        Vec::new()
                    };
                    Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
                    refinement_geometry.store(rods, &rollback_pairs);
                    if flags.nonlinear_merit {
                        eprintln!(
                            "HAIR CONTACT APPROACH MODEL iteration={iteration} added_rows={added_mesh} added_pairs={added_pairs} descent_found={}",
                            accepted.is_some()
                        );
                    }
                    continue;
                }
                if let Some((scale, pairs)) = accepted {
                    fresh = pairs;
                    accepted_scale = scale;
                    for response in &mut current {
                        response.impulse *= scale;
                    }
                    complete = false;
                } else {
                    // Try local geometry descent before enriching the tangent
                    // model. A far rejected candidate can lie across another
                    // surface feature; its outward plane is not a global
                    // halfspace valid at the original pose. Only the final
                    // nearby failed trial supplies a local counterexample.
                    if continuous_mesh {
                        let trial_merit = Self::contact_merit(rods, &[], radius)?;
                        let added = if merit == 0. {
                            model.observe_rejected_candidate_admitting_origin(
                                rods,
                                before.iter().map(|(x, _)| x.as_slice()),
                            )
                        } else {
                            model.observe_rejected_candidate(rods)
                        };
                        if added > 0 {
                            for (rod, (x, q)) in rods.iter_mut().zip(&before) {
                                rod.x.clone_from(x);
                                rod.q.clone_from(q);
                            }
                            // Preserve fresh-query ordering: strand rows, then mesh rows.
                            let rollback_pairs = if self_collision {
                                contact::refresh_strand_responses(rods, radius, &[])
                            } else {
                                Vec::new()
                            };
                            Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
                            refinement_geometry.store(rods, &rollback_pairs);
                            if flags.nonlinear_merit {
                                eprintln!(
                                    "HAIR CONTACT LOCAL MODEL iteration={iteration} added_rows={added} before={merit:e} rejected_mesh={trial_merit:e} trial_scale={last_trial_scale:e}"
                                );
                            }
                            continue;
                        }
                    }
                    // Do not publish a known worsening increment or its
                    // reactions. The enclosing step transaction preserves all
                    // public state; retain the pre-increment staged pose too.
                    if let (Some(path), Some(contacts)) =
                        (&flags.rejected_direction_export, &before_contacts)
                    {
                        let mut rows = Vec::new();
                        for (r, list) in contacts.iter().enumerate() {
                            for c in list
                                .iter()
                                .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                            {
                                let i = c.segment;
                                let t = c.fraction;
                                let p =
                                    add(mul(before[r].0[i], 1. - t), mul(before[r].0[i + 1], t));
                                let end = add(mul(full[r].0[i], 1. - t), mul(full[r].0[i + 1], t));
                                let metric_scale = c.metric_scale;
                                rows.push(format!(
                                    "{{\"rod\":{r},\"segment\":{i},\"fraction\":{t:?},\"metric_scale\":{metric_scale:?},\"normal\":{:?},\"target\":{:?},\"gap\":{:?},\"directional_gap\":{:?}}}",
                                    c.normal,
                                    c.target,
                                    c.physical_gap(p),
                                    c.metric_scale * dot(sub(end, p), c.normal)
                                ));
                            }
                        }
                        let bx: Vec<_> = before.iter().map(|pose| &pose.0).collect();
                        let fx: Vec<_> = full.iter().map(|pose| &pose.0).collect();
                        let bq: Vec<_> = before.iter().map(|pose| &pose.1).collect();
                        let fq: Vec<_> = full.iter().map(|pose| &pose.1).collect();
                        let mut trial_rows = Vec::new();
                        for (r, rod) in rods.iter().enumerate() {
                            for c in rod
                                .contacts
                                .iter()
                                .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
                            {
                                let t = c.fraction;
                                let i = c.segment;
                                let p = add(mul(rod.x[i], 1. - t), mul(rod.x[i + 1], t));
                                trial_rows.push(format!(
                                    "{{\"rod\":{r},\"segment\":{i},\"fraction\":{t:?},\"gap\":{:?},\"metric_scale\":{:?},\"normal\":{:?}}}",
                                    c.physical_gap(p),
                                    c.metric_scale,
                                    c.normal
                                ));
                            }
                        }
                        let payload = format!(
                            "{{\"iteration\":{iteration},\"before_merit\":{merit:?},\"full_merit\":{full_merit:?},\"before_positions\":{bx:?},\"full_positions\":{fx:?},\"before_orientations\":{bq:?},\"full_orientations\":{fq:?},\"mesh_contacts\":[{}],\"smallest_trial_scale\":{last_trial_scale:?},\"smallest_trial_mesh_contacts\":[{}]}}",
                            rows.join(","),
                            trial_rows.join(",")
                        );
                        if let Err(error) = std::fs::write(path, payload) {
                            eprintln!("HAIR CONTACT DIRECTION EXPORT ERROR: {error}");
                        }
                    }
                    for (rod, (x, q)) in rods.iter_mut().zip(before) {
                        rod.x = x;
                        rod.q = q;
                    }
                    Self::refresh_mesh_geometry(rods, meshes, radius, continuous_mesh)?;
                    if let Some(path) = &flags.replay_export {
                        if let Err(error) = contact_replay::save(
                            std::path::Path::new(path),
                            rods,
                            meshes,
                            dt,
                            radius,
                            self_collision,
                            merit,
                            continuous_mesh,
                        ) {
                            eprintln!("HAIR CONTACT REPLAY EXPORT ERROR: {error}");
                        }
                    }
                    if flags.nonlinear_merit {
                        eprintln!(
                            "HAIR NONLINEAR REJECT iteration={iteration} before={merit:e} full={full_merit:e} reason=no_descent"
                        );
                    }
                    return Err("joint hair contact direction has no descending geometry step");
                }
            }
            if flags.nonlinear_merit {
                let accepted_merit = Self::contact_merit(rods, &fresh, radius)?;
                eprintln!(
                    "HAIR NONLINEAR MERIT iteration={iteration} before={merit:e} full={full_merit:e} accepted={accepted_merit:e} scale={accepted_scale:e} tangent_complete={complete}"
                );
            }
            // Speculative contact planes describe the preceding frozen pose.
            // Carrying them across an accepted nonlinear move can combine
            // opposing sides of the same surface into an invalid tangent
            // model. Fresh geometry and whole-path CCD rediscover obstacles.
            if rods
                .iter()
                .zip(&before)
                .any(|(rod, (x, q))| rod.x != *x || rod.q != *q)
            {
                model.invalidate_after_pose_change();
            }
            let accepted_merit = Self::contact_merit(rods, &fresh, radius)?;
            work.accepted_step();
            merit_history.push(accepted_merit);
            // Store signed transition energies, not rounded total energies.
            // The progress window then tests actual work over its eight moves.
            energy_change_history.push(if accepted_merit == 0. && feasible_energy.is_some() {
                Some(Self::elastic_energy_change(rods, &before, dt)?)
            } else {
                None
            });
            // The common trust scale preserves paired reactions. Re-query all
            // geometry before the next nonlinear increment rather than fixing
            // one strand to the other's preceding position.
            if let Some(snapshot) = snapshot.as_mut() {
                snapshot.record_reactions(&current);
            }
            history.extend(current);
            // A solved tangent problem is not proof of separation in the new
            // nonlinear geometry: rotations can change a closest feature or
            // create another pair. Admit only a freshly queried feasible pose.
            let strand_admitted = if self_collision {
                contact::strand_geometry_admitted(rods, &fresh, radius)?
            } else {
                true
            };
            if let (Some(trace), Some(mut snapshot)) = (trace.as_mut(), snapshot) {
                snapshot.complete(rods);
                trace.output.push(snapshot);
            }
            let mut unresolved = !strand_admitted;
            let mut worst = None;
            for (index, rod) in rods.iter().enumerate() {
                for contact in &rod.contacts {
                    let i = contact.segment;
                    let t = contact.fraction;
                    let position = add(mul(rod.x[i], 1. - t), mul(rod.x[i + 1], t));
                    let gap = contact.physical_gap(position);
                    if !gap.is_finite() {
                        return Err("joint hair contact geometry overflow");
                    }
                    unresolved |= gap < -1e-10;
                    if gap
                        < worst.as_ref().map_or(
                            0.,
                            |value: &(usize, usize, f64, f64, ContactSource, V, V, V, V, V)| {
                                value.3
                            },
                        )
                    {
                        worst = Some((
                            index,
                            i,
                            t,
                            gap,
                            contact.source,
                            rod.x[0],
                            rod.x[i],
                            rod.x[i + 1],
                            contact.normal,
                            contact.target,
                        ));
                    }
                }
            }
            for rod in rods.iter_mut() {
                rod.contacts
                    .retain(|contact| matches!(contact.source, ContactSource::Mesh(_)));
            }
            if flags.nonlinear_merit {
                eprintln!(
                    "HAIR NONLINEAR ADMISSION iteration={iteration} tangent_complete={complete} strand_admitted={strand_admitted} unresolved={unresolved} worst_gap_m={:?}",
                    worst.as_ref().map(|value| value.3)
                );
            }
            if complete && !unresolved {
                if continuous_mesh {
                    mesh_motion::admit(rods, meshes, radius)?;
                }
                return Ok(());
            }
            if iteration == 0 {
                first_gap = worst.as_ref().map_or(0., |value| value.3);
            }
            last_worst = worst;
            // Once contact excess is zero, actual elastic energy owns progress.
            // Zero contact merit cannot contract further. This is only a work
            // budget: tangent completion and every physical gate remain above.
            if iteration >= 31 && !Self::nonlinear_progress(&merit_history, &energy_change_history)
            {
                break;
            }
        }
        if let Some(path) = &flags.replay_export {
            let pairs = if self_collision {
                contact::refresh_strand_responses(rods, radius, &[])
            } else {
                Vec::new()
            };
            if let Ok(merit) = Self::contact_merit(rods, &pairs, radius) {
                if let Err(error) = contact_replay::save(
                    std::path::Path::new(path),
                    rods,
                    meshes,
                    dt,
                    radius,
                    self_collision,
                    merit,
                    continuous_mesh,
                ) {
                    eprintln!("HAIR CONTACT REPLAY EXPORT ERROR: {error}");
                }
            }
        }
        if flags.nonlinear_merit {
            let (attempts, accepted_steps, attempts_at_pose) = work.counts();
            eprintln!(
                "HAIR NONLINEAR WORK BUDGET attempts={attempts} accepted_steps={accepted_steps} attempts_at_pose={attempts_at_pose} accepted_step_limit={accepted_step_limit} fixed_pose_attempt_limit=128"
            );
            eprintln!("HAIR NONLINEAR MERIT HISTORY {merit_history:?}");
        }
        eprintln!(
            "HAIR NONLINEAR CONTACT NONCONVERGENCE first_gap_m={first_gap} worst=(rod,segment,fraction,gap,source,root,a,b,normal,target) {last_worst:?}"
        );
        Err("joint hair contact position linearization did not converge")
    }

    #[cfg(test)]
    pub(super) fn feasible_elastic_prefix_admitted(
        rods: &[HairRod],
        meshes: &[TriangleMesh],
        before: &[(Vec<V>, Vec<Q>)],
        dt: f64,
        radius: f64,
        self_collision: bool,
    ) -> Result<bool, &'static str> {
        let trace = std::env::var_os("VOXY_HAIR_NONLINEAR_MERIT_TRACE").is_some();
        Self::feasible_elastic_prefix_admitted_with_flag(
            rods,
            meshes,
            before,
            dt,
            radius,
            self_collision,
            trace,
        )
    }

    fn feasible_elastic_prefix_admitted_with_flag(
        rods: &[HairRod],
        meshes: &[TriangleMesh],
        before: &[(Vec<V>, Vec<Q>)],
        dt: f64,
        radius: f64,
        self_collision: bool,
        trace: bool,
    ) -> Result<bool, &'static str> {
        if rods.len() != before.len()
            || rods
                .iter()
                .zip(before)
                .any(|(rod, (x, q))| rod.x[0] != x[0] || rod.q[0] != q[0])
        {
            if trace {
                eprintln!("HAIR CONTACT PREFIX CHECK admitted=false gate=prescribed_roots");
            }
            return Ok(false);
        }
        let stretch = rods
            .iter()
            .map(HairRod::max_relative_stretch)
            .try_fold(0f64, |a, b| b.is_finite().then_some(a.max(b)));
        if stretch.is_none_or(|value| value > 0.05) {
            if trace {
                eprintln!(
                    "HAIR CONTACT PREFIX CHECK admitted=false gate=stretch stretch={stretch:?}"
                );
            }
            return Ok(false);
        }
        let change = Self::elastic_energy_change(rods, before, dt)?;
        if !change.is_finite() || change >= 0. {
            if trace {
                eprintln!(
                    "HAIR CONTACT PREFIX CHECK admitted=false gate=energy change_j={change:e}"
                );
            }
            return Ok(false);
        }
        if self_collision {
            if let Err(error) = contact::admit_staged_strands(rods, radius) {
                if trace {
                    eprintln!(
                        "HAIR CONTACT PREFIX CHECK admitted=false gate=strand_path change_j={change:e} error={error:?}"
                    );
                }
                return Ok(false);
            }
        }
        if let Err(error) = mesh_motion::admit(rods, meshes, radius) {
            if trace {
                eprintln!(
                    "HAIR CONTACT PREFIX CHECK admitted=false gate=mesh_path change_j={change:e} error={error:?}"
                );
            }
            return Ok(false);
        }
        if trace {
            let translation = rods
                .iter()
                .zip(before)
                .flat_map(|(rod, (x, _))| rod.x.iter().zip(x).map(|(a, b)| len(sub(*a, *b))))
                .fold(0f64, f64::max);
            let rotation = rods
                .iter()
                .zip(before)
                .flat_map(|(rod, (_, q))| {
                    rod.q.iter().zip(q).map(|(a, b)| len(log(qm(*a, conj(*b)))))
                })
                .fold(0f64, f64::max);
            eprintln!(
                "HAIR CONTACT PREFIX CHECK admitted=true gate=complete change_j={change:e} max_translation_m={translation:e} max_rotation_rad={rotation:e}"
            );
        }
        Ok(true)
    }

    fn trace_contact_parts_with_flag(
        rods: &[HairRod],
        pairs: &[contact::StrandResponse],
        radius: f64,
        iteration: usize,
        scale: f64,
        trace_rows: bool,
    ) -> Result<(), &'static str> {
        let mut mesh_merit = 0.;
        let mut worst = None;
        for (r, rod) in rods.iter().enumerate() {
            for c in rod
                .contacts
                .iter()
                .filter(|c| matches!(c.source, ContactSource::Mesh(_)))
            {
                let p = add(
                    mul(rod.x[c.segment], 1. - c.fraction),
                    mul(rod.x[c.segment + 1], c.fraction),
                );
                let gap = c.physical_gap(p);
                if trace_rows {
                    eprintln!(
                        "HAIR CONTACT ROW {{\"iteration\":{iteration},\"scale\":{scale:?},\"rod\":{r},\"segment\":{},\"fraction\":{:?},\"time\":{},\"gap\":{gap:?},\"normal\":{:?},\"target\":{:?}}}",
                        c.segment,
                        c.fraction,
                        c.trajectory_time
                            .map_or("null".to_owned(), |t| format!("{t:?}")),
                        c.normal,
                        c.target
                    );
                }
                mesh_merit += gap.min(0.).powi(2);
                if gap
                    < worst
                        .as_ref()
                        .map_or(0., |v: &(usize, usize, f64, f64, V, V)| v.3)
                {
                    worst = Some((r, c.segment, c.fraction, gap, c.normal, c.target));
                }
            }
        }
        let mut strand_merit = 0.;
        for pair in pairs {
            let (a, i, s) = pair.a;
            let (b, j, t) = pair.b;
            let pa = add(mul(rods[a].x[i], 1. - s), mul(rods[a].x[i + 1], s));
            let pb = add(mul(rods[b].x[j], 1. - t), mul(rods[b].x[j + 1], t));
            strand_merit += (len(sub(pa, pb)) - 2. * radius).min(0.).powi(2);
        }
        if !(mesh_merit + strand_merit).is_finite() {
            return Err("contact merit diagnostic overflow");
        }
        eprintln!(
            "HAIR CONTACT PARTS iteration={iteration} scale={scale:e} mesh_raw={mesh_merit:e} strand={strand_merit:e} worst_mesh={worst:?}"
        );
        Ok(())
    }

    pub(super) fn elastic_energy_change(
        rods: &[HairRod],
        before: &[(Vec<V>, Vec<Q>)],
        dt: f64,
    ) -> Result<f64, &'static str> {
        if rods.len() != before.len() {
            return Err("hair energy pose count mismatch");
        }
        let (sum, correction) = rods.iter().zip(before).try_fold(
            (0f64, 0f64),
            |(sum, correction), (rod, (x, q))| {
                let term = direct::structural_delta(rod, x, q, dt)?;
                let next = sum + term;
                let roundoff = if sum.abs() >= term.abs() {
                    (sum - next) + term
                } else {
                    (term - next) + sum
                };
                Ok::<_, &'static str>((next, correction + roundoff))
            },
        )?;
        let delta = sum + correction;
        if !delta.is_finite() {
            return Err("joint hair elastic energy difference overflow");
        }
        Ok(delta)
    }

    pub(super) fn nonlinear_progress(merits: &[f64], changes: &[Option<f64>]) -> bool {
        if changes.len() != merits.len() {
            return false;
        }
        if merits.len() < 9 {
            return true;
        }
        let end = merits.len() - 1;
        let begin = end - 8;
        if merits[begin] > 0. {
            return merits[end] <= 0.9 * merits[begin];
        }
        if !merits[begin..=end].iter().all(|value| *value == 0.) {
            return false;
        }
        let mut sum = 0f64;
        let mut correction = 0f64;
        // Entry i is the move into pose i; exclude the move into the first
        // pose of the window. No monotonic decrease of every move is assumed.
        for change in &changes[begin + 1..=end] {
            let Some(change) = change.filter(|value| value.is_finite()) else {
                return false;
            };
            let next = sum + change;
            correction += if sum.abs() >= change.abs() {
                (sum - next) + change
            } else {
                (change - next) + sum
            };
            sum = next;
        }
        let change = sum + correction;
        change.is_finite() && change < 0.
    }

    pub(super) fn contact_merit(
        rods: &[HairRod],
        pairs: &[contact::StrandResponse],
        radius: f64,
    ) -> Result<f64, &'static str> {
        let mut merit = 0.;
        let mut heap_depth: Vec<f64> = Vec::new();
        for rod in rods {
            let mut stack_depth = [0f64; 64];
            let segment_depth: &mut [f64] = if rod.lengths.len() <= stack_depth.len() {
                &mut stack_depth[..rod.lengths.len()]
            } else {
                heap_depth.clear();
                heap_depth.resize(rod.lengths.len(), 0f64);
                &mut heap_depth[..]
            };
            for contact in &rod.contacts {
                if !matches!(contact.source, ContactSource::Mesh(_)) {
                    continue;
                }
                let p = add(
                    mul(rod.x[contact.segment], 1. - contact.fraction),
                    mul(rod.x[contact.segment + 1], contact.fraction),
                );
                let gap = contact.physical_gap(p);
                // Discovery adds a row only beyond the physical gap tolerance.
                // Measure excess violation so activation is continuous at that
                // same boundary instead of adding a tolerance-squared jump.
                segment_depth[contact.segment] = segment_depth[contact.segment].max(-gap - 1e-10);
                if !gap.is_finite() {
                    return Err("contact merit overflow");
                }
            }
            merit += segment_depth.iter().map(|depth| depth * depth).sum::<f64>();
        }
        for pair in pairs {
            let (a, i, s) = pair.a;
            let (b, j, t) = pair.b;
            let pa = add(mul(rods[a].x[i], 1. - s), mul(rods[a].x[i + 1], s));
            let pb = add(mul(rods[b].x[j], 1. - t), mul(rods[b].x[j + 1], t));
            let gap = len(sub(pa, pb)) - 2. * radius;
            merit += (-gap - 1e-10).max(0.).powi(2);
            if !gap.is_finite() {
                return Err("contact merit overflow");
            }
        }
        if !merit.is_finite() {
            return Err("contact merit overflow");
        }
        Ok(merit)
    }
}
