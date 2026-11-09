//! Substep time integration, structural solve dispatch, and contact stabilization for [`HairSystem`].
use super::{
    HairLinearSolver, HairLinearSystem, HairPhaseDiagnostic, HairProjectionTrace, HairRod,
    HairStepProfile, HairSystem, RootPose, TriangleMesh, contact, direct, math::*, mesh_motion,
};

impl HairSystem {
    pub(super) fn solve_external_rods(
        rods: &mut [HairRod],
        dt: f64,
        solver: &mut dyn HairLinearSolver,
    ) -> Result<(), &'static str> {
        let systems = rods
            .iter_mut()
            .map(|rod| {
                let (matrix, rhs) = direct::assemble(rod, dt)?;
                let n = rhs.len();
                Ok(HairLinearSystem {
                    band_width: direct::BAND,
                    matrix,
                    rhs,
                    active: 6..n - 3,
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let corrections = solver.solve(&systems)?;
        if corrections.len() != systems.len() {
            return Err("hair accelerator changed batch size");
        }
        for (system, correction) in systems.iter().zip(&corrections) {
            system.validate_correction(correction)?;
        }
        for ((rod, system), correction) in rods.iter_mut().zip(systems).zip(corrections) {
            direct::apply_correction(rod, &correction, dt, &system.rhs)?;
            rod.solve_matrix = system.matrix;
            rod.solve_rhs = system.rhs;
        }
        Ok(())
    }

    pub(super) fn step_impl(
        &mut self,
        dt: f64,
        roots: &[RootPose],
        gravity: V,
        air_velocity: V,
        meshes: &[TriangleMesh],
        mut solver: Option<&mut dyn HairLinearSolver>,
    ) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt < 1e-6
            || dt > 1. / 20.
            || self.substeps == 0
            || self.substeps > 32
            || self.workers == 0
            || self.workers > 64
            || self.iterations == 0
            || self.iterations > 128
            || self.terminal_contact_iterations > 32
            || !self.contact_radius.is_finite()
            || self.contact_radius <= 0.
            || roots.len() != self.rods.len()
            || !finite(gravity)
            || !finite(air_velocity)
            || roots.iter().any(|r| {
                !finite(r.position)
                    || r.rotation.iter().any(|x| !x.is_finite())
                    || (r.rotation.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-5
            })
        {
            return Err("invalid hair step");
        }
        let swept_positions = self.swept_strand_positions
            || (self.continuous_mesh_admission
                && self.self_collision
                && self.joint_contact_positions);
        if swept_positions
            && (!self.joint_contact_positions
                || !self.self_collision
                || self.terminal_contact_iterations != 0)
        {
            return Err(
                "swept strand motion requires joint self contacts and no terminal structural bypass",
            );
        }
        if self.recover_friction_pressure && !self.joint_contact_positions {
            return Err("friction pressure recovery requires joint positions");
        }
        let timed_sampling = self.sample_collider_motion || self.continuous_mesh_admission;
        if timed_sampling
            && meshes
                .iter()
                .filter_map(TriangleMesh::motion_duration)
                .any(|duration| (duration - dt).abs() > 1e-12 * dt)
        {
            return Err("collider motion duration differs from hair step");
        }
        let mut sampled_meshes = (timed_sampling
            && (self.continuous_mesh_admission
                || meshes.iter().any(|mesh| mesh.motion_duration().is_some())))
        .then(|| meshes.to_vec());
        let mut sampled_vertices = sampled_meshes
            .as_ref()
            .map(|sampled| vec![Vec::new(); sampled.len()]);
        self.last_trace.clear();
        self.last_friction_trace.clear();
        self.last_projection_trace.clear();
        self.last_profile = HairStepProfile::default();
        let profiling = self.profiling;
        // Meshes are immutable for this step. Invalidate distance bounds before
        // every new call so animated/refitted or replaced surfaces cannot reuse them.
        for rod in &mut self.rods {
            for cache in &mut rod.clearance_cache {
                cache.fill(None);
            }
        }
        let substeps = self.substeps.max((dt * 240.0).ceil() as usize);
        self.last_step_substeps = substeps;
        let dt = dt / substeps as f64;
        if (self.swept_strand_positions || (self.continuous_mesh_admission && self.self_collision))
            && !self.swept_strand_initialized
        {
            // Authored guide curves are not necessarily collision-free. Admit
            // an initial pose transactionally, before continuous time evolution.
            // A numerical clearance margin prevents touching import geometry
            // from starting below the continuous-query precision band.
            let radius = self.contact_radius + 4e-10;
            let mut initial_meshes = meshes.to_vec();
            for (initial, source) in initial_meshes.iter_mut().zip(meshes) {
                if source.motion_duration().is_some() {
                    initial.sample_motion(source, 0.)?;
                }
            }
            let mut initial_reactions = Vec::new();
            if let Some(backend) = solver.as_mut().filter(|s| s.contact_responses_enabled()) {
                Self::reconcile_positions_with_solver(
                    &mut self.rods,
                    &initial_meshes,
                    dt,
                    radius,
                    true,
                    &mut initial_reactions,
                    Some(&mut **backend),
                    None,
                )?;
            } else {
                Self::reconcile_positions(
                    &mut self.rods,
                    &initial_meshes,
                    dt,
                    radius,
                    true,
                    &mut initial_reactions,
                    None,
                )?;
            }
            let admitted: Vec<Vec<V>> = self.rods.iter().map(|rod| rod.x.clone()).collect();
            if contact::strand_fraction(&self.rods, &admitted, self.contact_radius)? != 1.
                || self
                    .rods
                    .iter()
                    .any(|rod| rod.max_relative_stretch() > 0.05)
            {
                return Err("initial swept hair pose failed clearance or strain admission");
            }
            self.swept_strand_initialized = true;
            self.record_phase("initial-contact-admission", 0, 0);
        }
        for substep in 0..substeps {
            let meshes = if let Some(sampled) = &mut sampled_meshes {
                for ((sample, source), vertices) in sampled
                    .iter_mut()
                    .zip(meshes)
                    .zip(sampled_vertices.as_mut().expect("sampled collider scratch"))
                {
                    sample.sample_motion_interval(
                        source,
                        substep as f64 / substeps as f64,
                        (substep + 1) as f64 / substeps as f64,
                        vertices,
                    )?;
                }
                for rod in &mut self.rods {
                    for cache in &mut rod.clearance_cache {
                        cache.fill(None);
                    }
                }
                sampled.as_slice()
            } else {
                meshes
            };
            let mut strand_responses = Vec::new();
            for (rod, root) in self.rods.iter_mut().zip(roots) {
                // Interpolate moving attachments over substeps rather than teleporting on first substep.
                let t = 1. / (substeps - substep) as f64;
                let position = add(rod.x[0], mul(sub(root.position, rod.x[0]), t));
                let current = qm(rod.q[0], conj(rod.rest_q[0]));
                let rotation = qm(exp(mul(log(qm(root.rotation, conj(current))), t)), current);
                rod.predict(dt, RootPose { position, rotation }, gravity, air_velocity);
            }
            let mut structural_start = None;
            if swept_positions {
                // Keep prediction as an inertial target, not a published free
                // pose: that unrestricted move caused the captured tunnelling.
                let old: Vec<Vec<V>> = self.rods.iter().map(|rod| rod.old_x.clone()).collect();
                for rod in &mut self.rods {
                    rod.x[1..].copy_from_slice(&rod.old_x[1..]);
                    rod.q[1..].copy_from_slice(&rod.old_q[1..]);
                }
                let mut previous = self.rods.clone();
                for (rod, points) in previous.iter_mut().zip(old) {
                    rod.x = points;
                }
                // Root motion belongs to the first coupled structural path.
                // The staged new roots are the exact boundary conditions for
                // force assembly, not a separately published root-only pose.
                structural_start = Some(previous);
                for rod in &mut self.rods {
                    contact::refresh_mesh_constraints(rod, meshes, self.contact_radius);
                }
            }
            self.record_phase("predicted", substep, 0);
            // Guides are independent until shared self-contact. Keep workers
            // alive across each independent batch instead of spawning per iteration.
            let parallel = self.workers > 1 && self.rods.len() >= 8;
            let batch = if self.joint_contact_positions {
                1
            } else if parallel {
                if self.self_collision {
                    4
                } else {
                    self.iterations
                }
            } else {
                1
            };
            for first in (0..self.iterations).step_by(batch) {
                let end = (first + batch).min(self.iterations);
                let radius = self.contact_radius;
                let joint_positions = self.joint_contact_positions;
                if swept_positions {
                    let started = profiling.then(std::time::Instant::now);
                    // Newton iterations do not advance physical time. In the
                    // continuous pipeline every candidate shares this substep's
                    // original clock, including later structural corrections.
                    let motion_start = if self.continuous_mesh_admission || first == 0 {
                        structural_start.as_deref()
                    } else {
                        None
                    };
                    if let Some(backend) = solver.as_deref_mut() {
                        contact::advance_swept_strands(
                            &mut self.rods,
                            dt,
                            radius,
                            Some(backend),
                            &mut strand_responses,
                            motion_start,
                        )?;
                    } else {
                        contact::advance_swept_strands(
                            &mut self.rods,
                            dt,
                            radius,
                            None,
                            &mut strand_responses,
                            motion_start,
                        )?;
                    }
                    self.record_phase("structural", substep, first);
                    if let Some(started) = started {
                        self.last_profile.structural_ms += started.elapsed().as_secs_f64() * 1000.;
                    }
                    for rod in &mut self.rods {
                        contact::refresh_mesh_constraints(rod, meshes, radius);
                    }
                } else if let Some(solver) = solver.as_deref_mut() {
                    for iteration in first..end {
                        let started = profiling.then(std::time::Instant::now);
                        Self::solve_external_rods(&mut self.rods, dt, solver)?;
                        self.record_phase("structural", substep, iteration);
                        if let Some(started) = started {
                            self.last_profile.structural_ms +=
                                started.elapsed().as_secs_f64() * 1000.;
                        }
                        let started = profiling.then(std::time::Instant::now);
                        for rod in &mut self.rods {
                            if joint_positions {
                                contact::refresh_mesh_constraints(rod, meshes, radius);
                            } else {
                                contact::mesh_contacts(rod, meshes, radius);
                            }
                        }
                        if let Some(started) = started {
                            self.last_profile.mesh_contacts_ms +=
                                started.elapsed().as_secs_f64() * 1000.;
                        }
                    }
                } else if !parallel {
                    for (index, rod) in self.rods.iter_mut().enumerate() {
                        let started = profiling.then(std::time::Instant::now);
                        rod.solve(dt, first % 2 != 0)?;
                        if self.trace_rod == Some(index) {
                            self.last_trace.push(HairPhaseDiagnostic::capture(
                                "structural",
                                substep,
                                first,
                                index,
                                rod,
                            ));
                        }
                        if let Some(started) = started {
                            self.last_profile.structural_ms +=
                                started.elapsed().as_secs_f64() * 1000.;
                        }
                        let started = profiling.then(std::time::Instant::now);
                        if joint_positions {
                            contact::refresh_mesh_constraints(rod, meshes, radius);
                        } else {
                            contact::mesh_contacts(rod, meshes, radius);
                        }
                        if let Some(started) = started {
                            self.last_profile.mesh_contacts_ms +=
                                started.elapsed().as_secs_f64() * 1000.;
                        }
                    }
                } else {
                    let chunk = self.rods.len().div_ceil(self.workers);
                    let trace_rod = self.trace_rod;
                    let profiles = std::thread::scope(|scope| {
                        let mut workers = Vec::new();
                        for (chunk_index, rods) in self.rods.chunks_mut(chunk).enumerate() {
                            workers.push(scope.spawn(move || {
                                let mut profile = HairStepProfile::default();
                                let mut trace = Vec::new();
                                for (local_index, rod) in rods.iter_mut().enumerate() {
                                    for iteration in first..end {
                                        let started = profiling.then(std::time::Instant::now);
                                        rod.solve(dt, iteration % 2 != 0)?;
                                        let index = chunk_index * chunk + local_index;
                                        if trace_rod == Some(index) {
                                            trace.push(HairPhaseDiagnostic::capture(
                                                "structural",
                                                substep,
                                                iteration,
                                                index,
                                                rod,
                                            ));
                                        }
                                        if let Some(started) = started {
                                            profile.structural_ms +=
                                                started.elapsed().as_secs_f64() * 1000.;
                                        }
                                        let started = profiling.then(std::time::Instant::now);
                                        if joint_positions {
                                            contact::refresh_mesh_constraints(rod, meshes, radius);
                                        } else {
                                            contact::mesh_contacts(rod, meshes, radius);
                                        }
                                        if let Some(started) = started {
                                            profile.mesh_contacts_ms +=
                                                started.elapsed().as_secs_f64() * 1000.;
                                        }
                                    }
                                }
                                Ok::<_, &'static str>((profile, trace))
                            }));
                        }
                        workers
                            .into_iter()
                            .map(|worker| {
                                worker
                                    .join()
                                    .map_err(|_| "hair profile worker panicked")
                                    .and_then(|result| result)
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })?;
                    for (profile, trace) in profiles {
                        self.last_trace.extend(trace);
                        self.last_profile.structural_ms += profile.structural_ms;
                        self.last_profile.mesh_contacts_ms += profile.mesh_contacts_ms;
                    }
                }
                self.record_phase("structural-mesh", substep, end);
                if joint_positions && !self.self_collision {
                    let started = profiling.then(std::time::Instant::now);
                    let trace = if let Some(rod) = self.trace_rod {
                        Some(HairProjectionTrace {
                            rod,
                            substep,
                            structural_iteration: end,
                            output: &mut self.last_projection_trace,
                        })
                    } else {
                        None
                    };
                    if let Some(backend) = solver.as_mut().filter(|s| s.contact_responses_enabled())
                    {
                        Self::reconcile_positions_mode(
                            &mut self.rods,
                            meshes,
                            dt,
                            radius,
                            false,
                            &mut strand_responses,
                            Some(&mut **backend),
                            trace,
                            self.continuous_mesh_admission,
                        )?;
                    } else {
                        Self::reconcile_positions_mode(
                            &mut self.rods,
                            meshes,
                            dt,
                            radius,
                            false,
                            &mut strand_responses,
                            None,
                            trace,
                            self.continuous_mesh_admission,
                        )?;
                    }
                    if let Some(started) = started {
                        self.last_profile.mesh_contacts_ms +=
                            started.elapsed().as_secs_f64() * 1000.;
                    }
                }
                if self.self_collision
                    && (joint_positions || end % 4 == 0 || end == self.iterations)
                {
                    let started = profiling.then(std::time::Instant::now);
                    if joint_positions {
                        let trace = if let Some(rod) = self.trace_rod {
                            Some(HairProjectionTrace {
                                rod,
                                substep,
                                structural_iteration: end,
                                output: &mut self.last_projection_trace,
                            })
                        } else {
                            None
                        };
                        if let Some(backend) =
                            solver.as_mut().filter(|s| s.contact_responses_enabled())
                        {
                            Self::reconcile_positions_mode(
                                &mut self.rods,
                                meshes,
                                dt,
                                radius,
                                true,
                                &mut strand_responses,
                                Some(&mut **backend),
                                trace,
                                self.continuous_mesh_admission,
                            )?;
                        } else {
                            Self::reconcile_positions_mode(
                                &mut self.rods,
                                meshes,
                                dt,
                                radius,
                                true,
                                &mut strand_responses,
                                None,
                                trace,
                                self.continuous_mesh_admission,
                            )?;
                        }
                    } else {
                        strand_responses.extend(contact::self_contacts(&mut self.rods, radius));
                    }
                    if let Some(started) = started {
                        self.last_profile.self_contacts_ms +=
                            started.elapsed().as_secs_f64() * 1000.;
                    }
                    // A strand reaction can move a guide into the body after
                    // its independent mesh phase. Refresh those constraints
                    // before the shared velocity projection.
                    if self.joint_contact_velocities {
                        let started = profiling.then(std::time::Instant::now);
                        for rod in &mut self.rods {
                            contact::refresh_mesh_constraints(rod, meshes, radius);
                        }
                        if let Some(started) = started {
                            self.last_profile.mesh_contacts_ms +=
                                started.elapsed().as_secs_f64() * 1000.;
                        }
                    }
                }
                self.record_phase("contact-positions", substep, end);
            }
            // Additional coupled iterations: an elastic correction may reopen
            // penetration, so it must recheck geometry before publication.
            for _ in 0..self.terminal_contact_iterations {
                let started = profiling.then(std::time::Instant::now);
                if let Some(solver) = solver.as_deref_mut() {
                    Self::solve_external_rods(&mut self.rods, dt, solver)?;
                } else {
                    for rod in &mut self.rods {
                        direct::solve(rod, dt)?;
                    }
                }
                if let Some(started) = started {
                    self.last_profile.structural_ms += started.elapsed().as_secs_f64() * 1000.;
                }
                let started = profiling.then(std::time::Instant::now);
                for rod in &mut self.rods {
                    contact::refresh_mesh_constraints(rod, meshes, self.contact_radius);
                }
                if let Some(started) = started {
                    self.last_profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.;
                }
                if self.joint_contact_positions {
                    let started = profiling.then(std::time::Instant::now);
                    if let Some(backend) = solver.as_mut().filter(|s| s.contact_responses_enabled())
                    {
                        Self::reconcile_positions_mode(
                            &mut self.rods,
                            meshes,
                            dt,
                            self.contact_radius,
                            self.self_collision,
                            &mut strand_responses,
                            Some(&mut **backend),
                            None,
                            self.continuous_mesh_admission,
                        )?;
                    } else {
                        Self::reconcile_positions_mode(
                            &mut self.rods,
                            meshes,
                            dt,
                            self.contact_radius,
                            self.self_collision,
                            &mut strand_responses,
                            None,
                            None,
                            self.continuous_mesh_admission,
                        )?;
                    }
                    if let Some(started) = started {
                        self.last_profile.mesh_contacts_ms +=
                            started.elapsed().as_secs_f64() * 1000.;
                    }
                }
            }
            if self.self_collision {
                let started = profiling.then(std::time::Instant::now);
                strand_responses = contact::refresh_strand_responses(
                    &mut self.rods,
                    self.contact_radius,
                    &strand_responses,
                );
                if let Some(started) = started {
                    self.last_profile.self_contacts_ms += started.elapsed().as_secs_f64() * 1000.;
                }
            }
            if self.recover_friction_pressure {
                let started = profiling.then(std::time::Instant::now);
                if let Some(backend) = solver.as_mut().filter(|s| s.contact_responses_enabled()) {
                    contact::recover_friction_pressure(
                        &self.rods,
                        &mut strand_responses,
                        dt,
                        self.contact_radius,
                        Some(&mut **backend),
                    )?;
                } else {
                    contact::recover_friction_pressure(
                        &self.rods,
                        &mut strand_responses,
                        dt,
                        self.contact_radius,
                        None,
                    )?;
                }
                if let Some(started) = started {
                    self.last_profile.self_contacts_ms += started.elapsed().as_secs_f64() * 1000.;
                }
            }
            if self.continuous_mesh_admission {
                let started = profiling.then(std::time::Instant::now);
                let admission = mesh_motion::admit(&self.rods, meshes, self.contact_radius);
                if let Some(started) = started {
                    self.last_profile.mesh_contacts_ms += started.elapsed().as_secs_f64() * 1000.;
                }
                admission?;
                if self.self_collision {
                    contact::admit_staged_strands(&self.rods, self.contact_radius)?;
                }
                self.record_phase("continuous-mesh-admission", substep, self.iterations);
            }
            self.record_phase("before-finish", substep, self.iterations);
            for rod in &mut self.rods {
                rod.finish(dt);
            }
            self.record_phase("rod-velocities", substep, self.iterations);
            if self.trace_rod.is_some() {
                contact::finish_strand_contacts_with_diagnostics(
                    &mut self.rods,
                    &strand_responses,
                    dt,
                    substep,
                    Some(&mut self.last_friction_trace),
                );
            } else {
                contact::finish_strand_contacts(&mut self.rods, &strand_responses, dt);
            }
            self.record_phase("strand-friction", substep, self.iterations);
            if self.joint_contact_velocities {
                if let Some(backend) = solver.as_mut().filter(|s| s.contact_responses_enabled()) {
                    contact::stabilize_contact_velocities_with_solver(
                        &mut self.rods,
                        &strand_responses,
                        dt,
                        self.contact_radius,
                        Some(&mut **backend),
                    )?;
                } else {
                    contact::stabilize_contact_velocities(
                        &mut self.rods,
                        &strand_responses,
                        dt,
                        self.contact_radius,
                    )?;
                }
            }
            self.record_phase("finished", substep, self.iterations);
        }
        if self.rods.iter().any(|r| {
            r.x.iter()
                .chain(&r.velocity)
                .chain(&r.omega)
                .any(|p| !finite(*p))
                || r.q.iter().flatten().any(|q| !q.is_finite())
        }) {
            return Err("hair solver overflow");
        }
        Ok(())
    }
}
