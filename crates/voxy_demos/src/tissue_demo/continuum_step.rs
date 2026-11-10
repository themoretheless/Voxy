//! Continuum attachment widening, adaptive substepping, mannequin skeleton, and mesh generation.
use super::*;

#[allow(dead_code)]
impl TissueDemo {
    pub(super) fn widen_attachment_palette(palette: &[Mat4]) -> Result<Vec<DMat4>, &'static str> {
        palette
            .iter()
            .map(|matrix| {
                let mut wide = DMat4::from_cols_array(&matrix.to_cols_array().map(f64::from));
                Self::validate_attachment_matrix_with_tolerance(
                    &wide,
                    8. * f64::from(f32::EPSILON),
                )?;
                // transform_point3 uses affine positions; canonicalize only the unit
                // metadata after validating it at the source scalar's precision.
                wide.w_axis.w = 1.;
                Ok(wide)
            })
            .collect()
    }
    pub(super) fn validate_attachment_matrix(matrix: &DMat4) -> Result<(), &'static str> {
        Self::validate_attachment_matrix_with_tolerance(matrix, 8. * f64::EPSILON)
    }
    pub(super) fn validate_attachment_matrix_with_tolerance(
        matrix: &DMat4,
        unit_tolerance: f64,
    ) -> Result<(), &'static str> {
        if !matrix.is_finite() {
            return Err("nonfinite tissue attachment");
        }
        if matrix.x_axis.w != 0.
            || matrix.y_axis.w != 0.
            || matrix.z_axis.w != 0.
            // General matrix inversion can round the homogeneous unit entry.
            || (matrix.w_axis.w - 1.).abs() > unit_tolerance
        {
            return Err("nonaffine tissue attachment");
        }
        Ok(())
    }
    pub(super) fn attachment_targets(
        attachment: &TissueAttachment,
        palette: &[DMat4],
    ) -> Result<Vec<SupportTarget>, &'static str> {
        let TissueAttachment {
            joint,
            pins,
            overrides,
        } = attachment;
        let default = palette
            .get(*joint)
            .ok_or("missing tissue attachment bone")?;
        Self::validate_attachment_matrix(default)?;
        pins.iter()
            .map(|(node, rest)| {
                let transform = |bone: usize| -> Result<[f64; 3], &'static str> {
                    let matrix = palette.get(bone).ok_or("missing tissue attachment bone")?;
                    Self::validate_attachment_matrix(matrix)?;
                    Ok(matrix.transform_point3(DVec3::from_array(*rest)).to_array())
                };
                let position_m = if let Some(influences) = overrides.get(node) {
                    if influences.len() == 1 && influences[0].1 == 1. {
                        transform(influences[0].0)?
                    } else {
                        let mut result = [0.; 3];
                        for &(bone, weight) in influences {
                            let transformed = transform(bone)?;
                            for axis in 0..3 {
                                result[axis] += weight * transformed[axis];
                            }
                        }
                        result
                    }
                } else {
                    transform(*joint)?
                };
                if position_m.iter().any(|v| !v.is_finite()) {
                    return Err("nonfinite tissue attachment");
                }
                Ok(SupportTarget {
                    node: *node,
                    position_m,
                })
            })
            .collect()
    }
    pub(super) fn step_contact_region(
        body: &mut DemoTissue,
        attachment: &TissueAttachment,
        palette: &[DMat4],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
    ) -> Result<(), &'static str> {
        let targets = Self::attachment_targets(attachment, palette)?;
        Self::step_continuum_targets(body, &targets, conductivity_w_m_k, next_surface)
    }
    pub(super) fn step_continuum_targets(
        body: &mut DemoTissue,
        targets: &[SupportTarget],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
    ) -> Result<(), &'static str> {
        Self::step_continuum_targets_with_skin(
            body,
            targets,
            conductivity_w_m_k,
            next_surface,
            None,
        )
    }
    pub(super) fn step_continuum_targets_with_skin(
        body: &mut DemoTissue,
        targets: &[SupportTarget],
        conductivity_w_m_k: f64,
        next_surface: Option<&Arc<PrescribedTriangleSurface>>,
        next_skin: Option<&physics::biomechanics::StationaryEmbeddedContact>,
    ) -> Result<(), &'static str> {
        let DemoTissue::Continuum {
            dynamics,
            ledger,
            preferred_depth,
            energy_budget_rate_j_s,
            thermal_binding,
            cells,
            ..
        } = body
        else {
            return Err("body mode requires continuum tissue");
        };
        // Illustrative isotropic conductivity, not calibrated tissue data.
        // The outer frame candidate stages both thermal half-steps and all
        // mechanical trials. Pure conduction does not alter mechanical work.
        let binding = thermal_binding;
        let motion_budgets = std::env::var_os("VOXY_SUPPORT_MOTION_BUDGETS")
            .map(|value| {
                let text = value.to_str().ok_or("invalid support motion budgets")?;
                let (p, v) = text
                    .split_once(',')
                    .ok_or("invalid support motion budgets")?;
                let position = p
                    .parse::<f64>()
                    .map_err(|_| "invalid support motion budgets")?;
                let velocity = v
                    .parse::<f64>()
                    .map_err(|_| "invalid support motion budgets")?;
                if !position.is_finite()
                    || position <= 0.
                    || !velocity.is_finite()
                    || velocity <= 0.
                {
                    return Err("invalid support motion budgets");
                }
                if next_surface.is_some() || next_skin.is_some() {
                    return Err("support motion accuracy requires support-only path");
                }
                Ok((position, velocity))
            })
            .transpose()?;
        let explicit_accuracy = std::env::var_os("VOXY_EXPLICIT_SUPPORT_ACCURACY").is_some();
        if explicit_accuracy && motion_budgets.is_none() {
            return Err("explicit support accuracy requires motion budgets");
        }
        let trace_motion =
            motion_budgets.is_some() && std::env::var_os("VOXY_SUPPORT_ACCURACY_TRACE").is_some();
        let conductivity = vec![conductivity_w_m_k; cells.len()];
        let links = binding.internal_heat_contacts(dynamics, &conductivity)?;
        let first_conduction_defect = dynamics.conduct_maxwell_heat(&links, 1. / 480., 1e-10)?;
        let start: Vec<_> = targets
            .iter()
            .map(|p| dynamics.body().positions()[p.node])
            .collect();
        // Start one level coarser than the previous frame's finest accepted
        // step; each trial still passes the unchanged work/heat/path checks.
        let start_surface = dynamics.prescribed_surface().cloned();
        let start_skin = dynamics.body().stationary_embedded_contact().cloned();
        if next_surface.is_some() && start_surface.is_none() {
            return Err("surface motion requires installed contact");
        }
        let minimum_depth = std::env::var("VOXY_SUPPORT_MIN_DEPTH")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .map_err(|_| "invalid minimum support depth")
            })
            .transpose()?
            .unwrap_or(0);
        if minimum_depth > 10 {
            return Err("invalid minimum support depth");
        }
        let initial_depth = (*preferred_depth).max(minimum_depth);
        if motion_budgets.is_some() && initial_depth >= 12 {
            return Err("support accuracy refinement depth limit");
        }
        let subdivisions = 4_u32 << initial_depth;
        let mut frame_receipt = EnergyLedger::default();
        for substep in 1..=subdivisions {
            let fraction = f64::from(substep) / f64::from(subdivisions);
            let segment: Vec<_> = targets
                .iter()
                .zip(&start)
                .map(|(target, old)| SupportTarget {
                    node: target.node,
                    position_m: std::array::from_fn(|axis| {
                        if substep == subdivisions {
                            target.position_m[axis]
                        } else {
                            old[axis] + fraction * (target.position_m[axis] - old[axis])
                        }
                    }),
                })
                .collect();
            let segment_surface = next_surface
                .map(|next| {
                    let start = start_surface
                        .as_ref()
                        .ok_or("surface motion requires installed contact")?;
                    if substep == subdivisions {
                        return Ok(next.clone());
                    }
                    start
                        .with_positions(
                            start
                                .positions()
                                .iter()
                                .zip(next.positions())
                                .map(|(a, b)| {
                                    std::array::from_fn(|axis| {
                                        a[axis] + fraction * (b[axis] - a[axis])
                                    })
                                })
                                .collect(),
                        )
                        .map(Arc::new)
                })
                .transpose()?;
            let dt = 1. / (240. * f64::from(subdivisions));
            if let Some((position, velocity)) = motion_budgets {
                let advance = if explicit_accuracy {
                    InertialBody::step_viscoelastic_with_support_accuracy
                } else {
                    InertialBody::step_viscoelastic_implicit_with_support_accuracy
                };
                let receipt = advance(
                    dynamics,
                    Some(&segment),
                    dt,
                    *energy_budget_rate_j_s * dt,
                    position / f64::from(subdivisions),
                    velocity / f64::from(subdivisions),
                    1_usize << (12 - initial_depth),
                )?;
                if trace_motion {
                    println!(
                        "SUPPORT_ACCURACY_INTERVAL dt_s={:.17e} substeps={} trials={} position_difference_m={:.17e} velocity_difference_m_s={:.17e} position_budget_m={:.17e} velocity_budget_m_s={:.17e}",
                        dt,
                        receipt.substeps,
                        receipt.trial_steps,
                        receipt.position_difference_sum_m,
                        receipt.velocity_difference_sum_m_s,
                        position / f64::from(subdivisions),
                        velocity / f64::from(subdivisions)
                    );
                }
                frame_receipt.add(EnergyLedger {
                    support_work_j: receipt.step.support.support_work_j,
                    surface_work_j: receipt.step.support.surface_work_j,
                    heat_j: receipt.step.viscous_heat_j,
                    defect_j: receipt.step.total_energy_defect_j,
                    conduction_defect_j: 0.,
                    accepted_steps: receipt.substeps as u64,
                    // Coarse/fine evaluations are estimator work, not admitted steps.
                    // trial_steps are separately exposed by the opt-in trace above.
                    rejected_steps: 0,
                    max_refinement_depth: initial_depth + receipt.substeps.ilog2() as usize,
                });
                continue;
            }
            if let Some(next_skin) = next_skin {
                let skin = start_skin
                    .as_ref()
                    .ok_or("missing installed skin contact")?
                    .sample_linear_pose(next_skin, fraction)?;
                let receipt = dynamics
                    .step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
                        Some(&segment),
                        segment_surface.ok_or("skin motion requires native integration pose")?,
                        skin,
                        dt,
                        *energy_budget_rate_j_s * dt,
                        256,
                    )?;
                frame_receipt.add(EnergyLedger {
                    support_work_j: receipt.step.support.support_work_j,
                    surface_work_j: receipt.step.support.surface_work_j,
                    heat_j: receipt.step.viscous_heat_j,
                    defect_j: receipt.step.total_energy_defect_j,
                    conduction_defect_j: 0.,
                    accepted_steps: receipt.substeps as u64,
                    rejected_steps: u64::from(receipt.substeps.ilog2()),
                    max_refinement_depth: initial_depth + receipt.substeps.ilog2() as usize,
                });
            } else {
                frame_receipt.add(Self::advance_continuum_contact_budget(
                    dynamics,
                    &segment,
                    segment_surface,
                    dt,
                    initial_depth,
                    *energy_budget_rate_j_s,
                )?);
            }
        }
        let links = binding.internal_heat_contacts(dynamics, &conductivity)?;
        frame_receipt.conduction_defect_j =
            first_conduction_defect + dynamics.conduct_maxwell_heat(&links, 1. / 480., 1e-10)?;
        // Accuracy admission publishes two half-steps, so start two levels
        // below its finest depth to actually try one coarser physical step.
        *preferred_depth = frame_receipt
            .max_refinement_depth
            .saturating_sub(if motion_budgets.is_some() { 2 } else { 1 });
        ledger.add(frame_receipt);
        Ok(())
    }
    pub(super) fn advance_continuum(
        body: &mut InertialBody,
        targets: &[SupportTarget],
        dt: f64,
        depth: usize,
    ) -> Result<EnergyLedger, &'static str> {
        Self::advance_continuum_contact(body, targets, None, dt, depth)
    }
    pub(super) fn advance_continuum_contact(
        body: &mut InertialBody,
        targets: &[SupportTarget],
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        depth: usize,
    ) -> Result<EnergyLedger, &'static str> {
        Self::advance_continuum_contact_budget(body, targets, next_surface, dt, depth, 240. * 1e-5)
    }
    pub(super) fn advance_continuum_contact_budget(
        body: &mut InertialBody,
        targets: &[SupportTarget],
        next_surface: Option<Arc<PrescribedTriangleSurface>>,
        dt: f64,
        depth: usize,
        energy_budget_rate_j_s: f64,
    ) -> Result<EnergyLedger, &'static str> {
        // Retry a rejected mechanical step with a finer temporal partition.
        // The surrounding frame candidate owns all successful subdivisions.
        // The solver itself is atomic. A successful leaf needs no extra clone.
        let result = match &next_surface {
            Some(surface) => body.step_viscoelastic_implicit_with_surface_motion(
                Some(targets),
                surface.clone(),
                dt,
                energy_budget_rate_j_s * dt,
            ),
            None if std::env::var_os("VOXY_IMPLICIT_SUPPORTS").is_some() => body
                .step_viscoelastic_implicit_with_supports(
                    Some(targets),
                    dt,
                    energy_budget_rate_j_s * dt,
                ),
            None => body.step_viscoelastic(Some(targets), dt, energy_budget_rate_j_s * dt),
        };
        if let Err(error) = &result {
            record_refinement_rejection(error, depth);
        }
        match result {
            Ok(receipt) => Ok(EnergyLedger {
                support_work_j: receipt.support.support_work_j,
                surface_work_j: receipt.support.surface_work_j,
                heat_j: receipt.viscous_heat_j,
                defect_j: receipt.total_energy_defect_j,
                conduction_defect_j: 0.,
                accepted_steps: 1,
                rejected_steps: 0,
                max_refinement_depth: depth,
            }),
            Err(error) if voxy_cuda::is_tissue_search_backend_failure(error) => Err(error),
            // Imported supports can require finer steps than the procedural
            // fixture. Keep a finite refinement limit and unchanged per-time
            // energy admission; a rejected frame still commits nothing.
            Err(error) if depth >= 12 => {
                #[cfg(test)]
                if let (Some(directory), Some(next)) =
                    (std::env::var_os("VOXY_CONTACT_FIXTURE_DIR"), &next_surface)
                {
                    let capture = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let value =
                            contact_geometry_fixture(body, targets, next, dt, depth, error)?;
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory)?;
                        let position = body.body().positions()[0];
                        let name = format!(
                            "contact-{:016x}-{:016x}-{:016x}.json",
                            dt.to_bits(),
                            position[0].to_bits(),
                            position[1].to_bits()
                        );
                        let file = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(directory.join(name))?;
                        serde_json::to_writer(file, &value)?;
                        Ok(())
                    })();
                    if let Err(capture_error) = capture {
                        eprintln!("CONTACT_FIXTURE_ERROR {capture_error}");
                    }
                }
                if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                    // Optional probes must not replace the original rejection
                    // when a diagnostic subdivision itself fails.
                    let observation = (|| -> Result<(), &'static str> {
                        if let Some(surface) = body.prescribed_surface() {
                            let feature = surface.nearest_active_contact(
                                body.body().positions(),
                                &body.body().surface(),
                            )?;
                            let pinned_weight = feature.as_ref().map(|feature| {
                                feature
                                    .body_face
                                    .iter()
                                    .zip(feature.body_weights)
                                    .filter(|(node, _)| {
                                        targets.iter().any(|target| target.node == **node)
                                    })
                                    .map(|(_, weight)| weight)
                                    .sum::<f64>()
                            });
                            eprintln!(
                                "CONTACT_FEATURE before={feature:?} pinned_weight={pinned_weight:?}"
                            );
                        }
                        let mut diagnostic = body.clone();
                        let trial = match &next_surface {
                            Some(surface) => diagnostic
                                .step_viscoelastic_implicit_with_surface_motion(
                                    Some(targets),
                                    surface.clone(),
                                    dt,
                                    1e100,
                                ),
                            None => diagnostic.step_viscoelastic(Some(targets), dt, 1e100),
                        };
                        eprintln!(
                            "CONTACT_REJECTION dt={dt:.17e} depth={depth} budget_j={:.17e} error={error} before={:?} diagnostic_trial={trial:?} after={:?}",
                            energy_budget_rate_j_s * dt,
                            body.diagnostics(),
                            diagnostic.diagnostics()
                        );
                        if let Some(surface) = diagnostic.prescribed_surface() {
                            eprintln!(
                                "CONTACT_FEATURE after={:?}",
                                surface.nearest_active_contact(
                                    diagnostic.body().positions(),
                                    &diagnostic.body().surface()
                                )
                            );
                        }
                        if trial.is_ok()
                            && let (Some(start), Some(end)) =
                                (body.prescribed_surface(), diagnostic.prescribed_surface())
                        {
                            let faces = body.body().surface();
                            let initial = start.response(body.body().positions(), &faces)?;
                            let final_response =
                                end.response(diagnostic.body().positions(), &faces)?;
                            let mut integrands = Vec::new();
                            for sample in 0..=8 {
                                let fraction = sample as f64 / 8.;
                                let points: Vec<_> = body
                                    .body()
                                    .positions()
                                    .iter()
                                    .zip(diagnostic.body().positions())
                                    .map(|(a, b)| {
                                        std::array::from_fn(|axis| {
                                            a[axis] + fraction * (b[axis] - a[axis])
                                        })
                                    })
                                    .collect();
                                let surface = start.with_positions(
                                    start
                                        .positions()
                                        .iter()
                                        .zip(end.positions())
                                        .map(|(a, b)| {
                                            std::array::from_fn(|axis| {
                                                a[axis] + fraction * (b[axis] - a[axis])
                                            })
                                        })
                                        .collect(),
                                )?;
                                let response = surface.response(&points, &faces)?;
                                let mut work = 0.;
                                for (node, g) in response.body_gradient_n.iter().enumerate() {
                                    for axis in 0..3 {
                                        work += g[axis]
                                            * (diagnostic.body().positions()[node][axis]
                                                - body.body().positions()[node][axis]);
                                    }
                                }
                                for (node, g) in response.obstacle_gradient_n.iter().enumerate() {
                                    for axis in 0..3 {
                                        work += g[axis]
                                            * (end.positions()[node][axis]
                                                - start.positions()[node][axis]);
                                    }
                                }
                                integrands.push(work);
                            }
                            let simpson = (integrands[0]
                                + integrands[8]
                                + 4. * (integrands[1]
                                    + integrands[3]
                                    + integrands[5]
                                    + integrands[7])
                                + 2. * (integrands[2] + integrands[4] + integrands[6]))
                                / 24.;
                            let delta = final_response.potential_j - initial.potential_j;
                            let gauss = start.path_response(
                                end,
                                body.body().positions(),
                                diagnostic.body().positions(),
                                &faces,
                            )?;
                            let mut gauss_work = 0.;
                            let mut gauss_obstacle_work = 0.;
                            for (node, g) in gauss.body_gradient_n.iter().enumerate() {
                                for axis in 0..3 {
                                    gauss_work += g[axis]
                                        * (diagnostic.body().positions()[node][axis]
                                            - body.body().positions()[node][axis]);
                                }
                            }
                            for (node, g) in gauss.obstacle_gradient_n.iter().enumerate() {
                                for axis in 0..3 {
                                    gauss_obstacle_work += g[axis]
                                        * (end.positions()[node][axis]
                                            - start.positions()[node][axis]);
                                }
                            }
                            gauss_work += gauss_obstacle_work;
                            eprintln!(
                                "CONTACT_GAUSS_WORK work_j={gauss_work:.17e} quadrature_error_j={:.17e} obstacle_work_j={gauss_obstacle_work:.17e}",
                                delta - gauss_work
                            );
                            eprintln!(
                                "CONTACT_PATH_WORK contact_energy_change_j={delta:.17e} midpoint_work_j={:.17e} simpson_work_j={simpson:.17e} midpoint_error_j={:.17e} simpson_error_j={:.17e}",
                                integrands[4],
                                delta - integrands[4],
                                delta - simpson
                            );
                        }
                        for count in [2, 4] {
                            let mut proof = body.clone();
                            let mut total_defect = 0.;
                            let mut maximum_defect = 0.0_f64;
                            for step in 1..=count {
                                let fraction = step as f64 / count as f64;
                                let interpolated: Vec<_> = targets
                                    .iter()
                                    .map(|target| SupportTarget {
                                        node: target.node,
                                        position_m: std::array::from_fn(|axis| {
                                            body.body().positions()[target.node][axis]
                                                + fraction
                                                    * (target.position_m[axis]
                                                        - body.body().positions()[target.node]
                                                            [axis])
                                        }),
                                    })
                                    .collect();
                                let sub_surface = next_surface
                                    .as_ref()
                                    .map(|next| {
                                        let start = body
                                            .prescribed_surface()
                                            .ok_or("missing diagnostic contact")?;
                                        start
                                            .with_positions(
                                                start
                                                    .positions()
                                                    .iter()
                                                    .zip(next.positions())
                                                    .map(|(a, b)| {
                                                        std::array::from_fn(|axis| {
                                                            a[axis] + fraction * (b[axis] - a[axis])
                                                        })
                                                    })
                                                    .collect(),
                                            )
                                            .map(Arc::new)
                                    })
                                    .transpose()?;
                                let sub = match sub_surface {
                                    Some(surface) => proof
                                        .step_viscoelastic_implicit_with_surface_motion(
                                            Some(&interpolated),
                                            surface,
                                            dt / count as f64,
                                            1e100,
                                        ),
                                    None => proof.step_viscoelastic(
                                        Some(&interpolated),
                                        dt / count as f64,
                                        1e100,
                                    ),
                                }?;
                                total_defect += sub.total_energy_defect_j;
                                maximum_defect =
                                    maximum_defect.max(sub.support.energy_defect_j.abs());
                            }
                            eprintln!(
                                "CONTACT_CONVERGENCE subdivisions={count} total_defect_j={total_defect:.17e} maximum_leaf_defect_j={maximum_defect:.17e} mechanical_leaf_budget_j={:.17e}",
                                0.5 * energy_budget_rate_j_s * dt / count as f64
                            );
                        }
                        Ok(())
                    })();
                    if let Err(probe_error) = observation {
                        eprintln!(
                            "CONTACT_REJECTION_PROBE_FAILED original={error:?} probe={probe_error:?}"
                        );
                    }
                }
                Err(error)
            }
            Err(_) => {
                // Keep both children transactional: a successful first half
                // cannot escape when the second half fails.
                let mut candidate = body.clone();
                let middle: Vec<_> = targets
                    .iter()
                    .map(|target| SupportTarget {
                        node: target.node,
                        position_m: std::array::from_fn(|axis| {
                            (candidate.body().positions()[target.node][axis]
                                + target.position_m[axis])
                                * 0.5
                        }),
                    })
                    .collect();
                let middle_surface = next_surface
                    .as_ref()
                    .map(|next| {
                        let current = candidate
                            .prescribed_surface()
                            .ok_or("surface motion requires installed contact")?;
                        let positions = current
                            .positions()
                            .iter()
                            .zip(next.positions())
                            .map(|(a, b)| std::array::from_fn(|axis| a[axis] * 0.5 + b[axis] * 0.5))
                            .collect();
                        current.with_positions(positions).map(Arc::new)
                    })
                    .transpose()?;
                let mut receipt = Self::advance_continuum_contact_budget(
                    &mut candidate,
                    &middle,
                    middle_surface,
                    dt * 0.5,
                    depth + 1,
                    energy_budget_rate_j_s,
                )?;
                receipt.add(Self::advance_continuum_contact_budget(
                    &mut candidate,
                    targets,
                    next_surface,
                    dt * 0.5,
                    depth + 1,
                    energy_budget_rate_j_s,
                )?);
                receipt.rejected_steps += 1;
                *body = candidate;
                Ok(receipt)
            }
        }
    }
    /// Shared animation skeleton: root, then shoulder/elbow and hip/knee per side.
    pub(super) fn body_skeleton() -> Skeleton {
        let mut joints = vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }];
        for side in [-1.0, 1.0] {
            for (name, parent, origin, parent_origin) in [
                ("shoulder", 0, Vec3::new(side * 0.55, 0.89, 0.), Vec3::ZERO),
                (
                    "elbow",
                    joints.len(),
                    Vec3::new(side * 0.55, 0.35, 0.),
                    Vec3::new(side * 0.55, 0.89, 0.),
                ),
                ("hip", 0, Vec3::new(side * 0.2, -0.34, 0.), Vec3::ZERO),
                (
                    "knee",
                    joints.len() + 2,
                    Vec3::new(side * 0.2, -0.91, 0.),
                    Vec3::new(side * 0.2, -0.34, 0.),
                ),
            ] {
                joints.push(Joint {
                    name: format!("{side}_{name}").into(),
                    parent: Some(parent as u16),
                    bind_local: Transform {
                        translation: origin - parent_origin,
                        ..Transform::IDENTITY
                    },
                    inverse_bind: Mat4::from_translation(-origin),
                });
            }
        }
        Skeleton::new(joints).expect("fixed mannequin hierarchy")
    }
    pub(super) fn body_palette(&self) -> Vec<Mat4> {
        self.body_palette_at(self.time)
    }
    pub(super) fn body_palette_at(&self, time: f64) -> Vec<Mat4> {
        let skeleton = self.body_rig.as_deref().expect("body mode owns skeleton");
        let mut pose = skeleton.bind_pose();
        let phase = (time % 12.) as f32;
        let walking = if phase < 4. {
            (phase * std::f32::consts::TAU * 1.8).sin()
                * (std::f32::consts::PI * phase / 4.).sin().powi(2)
        } else {
            0.
        };
        let jumping = if (4.0..8.0).contains(&phase) {
            (Self::lift(time) / 0.48) as f32
        } else {
            0.
        };
        pose.set_joint_rotation(0, Quat::from_rotation_y(walking * 0.12))
            .expect("finite torso rotation");
        for (side, start) in [(-1., 1), (1., 5)] {
            pose.set_joint_rotation(
                start,
                Quat::from_rotation_x(-side * walking * 0.55 - jumping * 0.8),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 1,
                Quat::from_rotation_x(-0.25 * walking.abs() - jumping * 0.45),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 2,
                Quat::from_rotation_x(side * walking * 0.45 + jumping * 0.3),
            )
            .expect("finite joint rotation");
            pose.set_joint_rotation(
                start + 3,
                Quat::from_rotation_x(-0.5 * (side * walking).max(0.) - jumping * 0.6),
            )
            .expect("finite joint rotation");
        }
        pose.skin_matrices(skeleton)
            .expect("finite mannequin pose")
            .into_iter()
            .map(|matrix| Mat4::from_translation(Vec3::Y * Self::lift(time) as f32) * matrix)
            .collect()
    }
    pub(super) fn body_mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let palette = self.body_palette();
        // Two full mannequins, front and rear, retain a fixed camera and scale.
        for (view, x) in [(1.0, -1.25), (-1.0, 1.25)] {
            let mut ellipsoid = |center: [f64; 3], r: [f64; 3], color: [f32; 4], joint: usize| {
                let base = vertices.len() as u32;
                for j in 0..=16 {
                    let theta = j as f64 * std::f64::consts::PI / 16.0;
                    for k in 0..=24 {
                        let phi = k as f64 * std::f64::consts::TAU / 24.0;
                        let p = [
                            center[0] + r[0] * theta.sin() * phi.cos(),
                            center[1] + r[1] * theta.cos(),
                            center[2] + r[2] * theta.sin() * phi.sin(),
                        ];
                        let p =
                            palette[joint].transform_point3(Vec3::from_array(p.map(|v| v as f32)));
                        let normal = Vec3::new(
                            (theta.sin() * phi.cos() / r[0]) as f32,
                            (theta.cos() / r[1]) as f32,
                            (theta.sin() * phi.sin() / r[2]) as f32,
                        );
                        let normal = palette[joint].transform_vector3(normal).normalize()
                            * Vec3::new(view as f32, 1., view as f32);
                        let light =
                            0.3 + 0.7 * normal.dot(Vec3::new(-0.5, 0.8, 1.).normalize()).max(0.);
                        vertices.push(SceneVertex {
                            position: [x as f32 + p.x * view as f32, p.y, p.z * view as f32],
                            uv: [0.0; 2],
                            color: [
                                color[0] * light,
                                color[1] * light,
                                color[2] * light,
                                color[3],
                            ],
                        });
                    }
                }
                for j in 0..16 {
                    for k in 0..24 {
                        let a = base + j * 25 + k;
                        indices.extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
                    }
                }
            };
            let suit = [0.22, 0.48, 0.67, 1.0];
            ellipsoid([0.0, 0.38, 0.0], [0.39, 0.61, 0.22], suit, 0);
            ellipsoid([0.0, -0.24, 0.0], [0.41, 0.32, 0.25], suit, 0);
            ellipsoid(
                [0.0, 1.24, 0.0],
                [0.23, 0.28, 0.23],
                [0.7, 0.73, 0.77, 1.0],
                0,
            );
            for (side, start) in [(-1.0, 1), (1.0, 5)] {
                ellipsoid(
                    [side * 0.2, -0.625, 0.0],
                    [0.14, 0.285, 0.15],
                    suit,
                    start + 2,
                );
                ellipsoid(
                    [side * 0.2, -1.195, 0.0],
                    [0.13, 0.285, 0.14],
                    suit,
                    start + 3,
                );
                ellipsoid([side * 0.55, 0.62, 0.0], [0.12, 0.27, 0.12], suit, start);
                ellipsoid(
                    [side * 0.55, 0.08, 0.0],
                    [0.10, 0.27, 0.10],
                    suit,
                    start + 1,
                );
                ellipsoid(
                    [side * 0.2, -1.47, 0.09],
                    [0.15, 0.1, 0.23],
                    suit,
                    start + 3,
                );
            }
            for (i, (body, surface)) in self.bodies.iter().zip(&self.surfaces).enumerate() {
                let points = surface
                    .deform(body.positions())
                    .expect("validated tissue state");
                let color = if i < 2 {
                    [0.32, 0.67, 0.8, 1.0]
                } else {
                    [0.28, 0.58, 0.73, 1.0]
                };
                let normals = body
                    .smooth_boundary_normals()
                    .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
                let normals = surface
                    .deform(&normals)
                    .map_err(|_| voxy_render::SceneError::InvalidGeometry)?;
                for (p, normal) in points.iter().zip(normals) {
                    let normal = Vec3::from_array(normal.map(|v| v as f32)).normalize_or_zero()
                        * Vec3::new(view as f32, 1., view as f32);
                    let light =
                        0.3 + 0.7 * normal.dot(Vec3::new(-0.5, 0.8, 1.).normalize()).max(0.);
                    indices.push(vertices.len() as u32);
                    vertices.push(SceneVertex {
                        position: [(x + p[0] * view) as f32, p[1] as f32, (p[2] * view) as f32],
                        uv: [0.; 2],
                        color: [
                            color[0] * light,
                            color[1] * light,
                            color[2] * light,
                            color[3],
                        ],
                    });
                }
            }
        }
        SceneMesh::new(vertices, indices)
    }
}
