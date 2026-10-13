//! Captured-state feasibility-correction experiment, never a runtime path.
use super::*;
use crate::hair::{HairSystem, contact, contact_replay, mesh_motion};

#[test]
fn feasible_contact_correction_can_raise_potential_from_infeasible_reference() {
    let dt = 1. / 240.;
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
        Default::default(),
    )
    .unwrap();
    let reference = (rod.x.clone(), rod.q.clone());
    let root = rod.x[0];
    assert_eq!(
        crate::hair::direct::structural_energy(&rod, dt).unwrap(),
        0.
    );
    let witness = rod.record_contact(
        1,
        0.,
        [1., 0., 0.],
        [1e-4, 0., 0.01],
        crate::hair::ContactSource::Mesh(0),
    );
    assert!(rod.contacts[witness].physical_gap(rod.x[1]) < 0.);
    let mut rods = vec![rod];
    let (mut rows, _) = position_constraints(&rods, &[], 40e-6).unwrap();
    let step = NativeNewtonStep::new(&rods, dt, 1e-14).unwrap();
    let direction = step.project(&mut rows, 1e-14).unwrap();
    for i in 1..rods[0].x.len() {
        rods[0].x[i] = add(rods[0].x[i], direction.linear[0][i]);
    }
    for i in 1..rods[0].q.len() {
        apply(&mut rods[0].q[i], direction.angular[0][i]);
    }
    assert_eq!(rods[0].x[0], root);
    assert!(rods[0].contacts[witness].physical_gap(rods[0].x[1]) >= -1e-14);
    let delta =
        crate::hair::direct::structural_delta(&rods[0], &reference.0, &reference.1, dt).unwrap();
    // The zero-potential rest pose is infeasible. A nonzero contact response
    // necessarily costs inertia/elastic potential; descent from that pose
    // cannot be a universal certificate for constrained contact restoration.
    assert!(delta.is_finite() && delta > 0.);
}

fn refresh(
    rods: &mut [HairRod],
    meshes: &[crate::hair::TriangleMesh],
    radius: f64,
    collision: bool,
    continuous: bool,
) -> Vec<StrandResponse> {
    let pairs = if collision {
        contact::refresh_strand_responses(rods, radius, &[])
    } else {
        Vec::new()
    };
    HairSystem::refresh_mesh_geometry(rods, meshes, radius, continuous).unwrap();
    pairs
}

fn save_local_terminal(
    rods: &[HairRod],
    meshes: &[crate::hair::TriangleMesh],
    dt: f64,
    radius: f64,
    collision: bool,
    continuous: bool,
    endpoint_merit: f64,
) {
    if let Some(path) = std::env::var_os("VOXY_HAIR_LOCAL_TERMINAL_EXPORT") {
        let path = std::path::PathBuf::from(path);
        contact_replay::save(
            &path,
            rods,
            meshes,
            dt,
            radius,
            collision,
            endpoint_merit,
            continuous,
        )
        .unwrap();
        let limits = safe_motion::correction_strand_limits(rods, radius).unwrap();
        let first = limits.iter().min_by(|a, b| a.2.total_cmp(&b.2));
        eprintln!(
            "LOCAL TERMINAL EXPORT path={path:?} remaining_pairs={} first={first:?}",
            limits.len()
        );
        let entries = limits
            .iter()
            .map(|(a, b, f)| format!("[{a},{b},{f:?}]"))
            .collect::<Vec<_>>()
            .join(",");
        std::fs::write(path.with_extension("limits.json"),format!("{{\"pairs\":[{entries}],\"energy_reference\":\"original input capture; not this terminal VHR\"}}\n")).unwrap();
    }
}

/// Bounded local SQP restoration experiment. Rejected probes never become
/// contact state; every accepted iterate rebuilds its geometric model.
fn local_restoration(
    rods: &mut Vec<HairRod>,
    meshes: &[crate::hair::TriangleMesh],
    before: &[(Vec<V>, Vec<Q>)],
    dt: f64,
    radius: f64,
    collision: bool,
    continuous: bool,
    zero_free: bool,
) -> bool {
    let measure_path =
        collision && std::env::var("VOXY_HAIR_LOCAL_TRAJECTORY_MERIT").as_deref() == Ok("1");
    let limit = std::env::var("VOXY_HAIR_LOCAL_RESTORATION_STEPS")
        .map(|value| value.parse::<usize>().unwrap())
        .unwrap_or(32);
    assert!((1..=128).contains(&limit));
    for iteration in 0..=limit {
        let pairs = refresh(rods, meshes, radius, collision, continuous);
        let endpoint_merit = HairSystem::contact_merit(rods, &pairs, radius).unwrap();
        let path_merit = if measure_path {
            safe_motion::correction_trajectory_merit(rods, radius).unwrap()
        } else {
            0.
        };
        let merit = endpoint_merit + path_merit;
        eprintln!("LOCAL RESTORATION PARTS endpoint={endpoint_merit:e} trajectory={path_merit:e}");
        let work = HairSystem::elastic_energy_change(rods, before, dt).unwrap();
        let strain = rods
            .iter()
            .map(HairRod::max_relative_stretch)
            .fold(0., f64::max);
        let strand_path = !collision || contact::admit_staged_strands(rods, radius).is_ok();
        let mesh_path = !continuous || mesh_motion::admit(rods, meshes, radius).is_ok();
        let admitted = merit == 0. && work < 0. && strain <= 0.05 && strand_path && mesh_path;
        eprintln!(
            "LOCAL RESTORATION iteration={iteration} merit={merit:e} work_j={work:e} strain={strain:e} strand_path={strand_path} mesh_path={mesh_path} admitted={admitted}"
        );
        if admitted {
            save_local_terminal(
                rods,
                meshes,
                dt,
                radius,
                collision,
                continuous,
                endpoint_merit,
            );
            return true;
        }
        if iteration == limit {
            save_local_terminal(
                rods,
                meshes,
                dt,
                radius,
                collision,
                continuous,
                endpoint_merit,
            );
            return false;
        }
        let frozen_pose = rods.clone();
        let free = if zero_free {
            PositionIncrement {
                linear: rods.iter().map(|rod| vec![[0.; 3]; rod.x.len()]).collect(),
                angular: rods.iter().map(|rod| vec![[0.; 3]; rod.q.len()]).collect(),
            }
        } else {
            NativeNewtonStep::new(rods, dt, 1e-14).unwrap().free
        };
        let (mut rows, _) = position_constraints(rods, &pairs, radius).unwrap();
        if collision && std::env::var("VOXY_HAIR_LOCAL_TRAJECTORY_ROWS").as_deref() == Ok("1") {
            let mut start = rods.clone();
            for rod in &mut start {
                rod.x.clone_from(&rod.old_x);
            }
            let mut groups = pairs.clone();
            safe_motion::correction_trajectory_rows(&mut rows, rods, &start, &mut groups, radius)
                .unwrap();
        }
        let direction = square_root_projection::solve(&mut rows, rods, dt, free, 1e-14).unwrap();
        let (_, trust) = safe_motion::trust_components(rods, &direction, &pairs).unwrap();
        let mut accepted = false;
        for backtrack in 0..32 {
            let scale = 2f64.powi(-backtrack);
            let mut trial = frozen_pose.clone();
            for (r, rod) in trial.iter_mut().enumerate() {
                for i in 1..rod.x.len() {
                    rod.x[i] = add(rod.x[i], mul(direction.linear[r][i], scale * trust[r]));
                }
                for i in 1..rod.q.len() {
                    apply(
                        &mut rod.q[i],
                        mul(direction.angular[r][i], scale * trust[r]),
                    );
                }
            }
            let fresh = refresh(&mut trial, meshes, radius, collision, continuous);
            let next = HairSystem::contact_merit(&trial, &fresh, radius).unwrap()
                + if measure_path {
                    safe_motion::correction_trajectory_merit(&trial, radius).unwrap()
                } else {
                    0.
                };
            let next_strain = trial
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0., f64::max);
            let progress = if merit > 0. {
                next < merit
            } else {
                next == 0. && HairSystem::elastic_energy_change(&trial, before, dt).unwrap() < work
            };
            if progress && next_strain <= 0.05 {
                *rods = trial;
                accepted = true;
                eprintln!(
                    "LOCAL RESTORATION ACCEPT iteration={iteration} scale={scale:e} merit={next:e}"
                );
                break;
            }
        }
        if !accepted {
            eprintln!("LOCAL RESTORATION STOP iteration={iteration} reason=no_descending_probe");
            save_local_terminal(
                rods,
                meshes,
                dt,
                radius,
                collision,
                continuous,
                endpoint_merit,
            );
            return false;
        }
    }
    false
}

#[test]
#[ignore = "requires captured nonlinear failure VHR"]
fn captured_production_restoration_resumes_native_solver() {
    qualify_production_restoration(None);
}

#[test]
#[ignore = "requires captured nonlinear failure VHR"]
fn captured_production_restoration_rejects_bad_accelerator() {
    struct BadAccelerator {
        calls: usize,
    }
    impl crate::hair::HairLinearSolver for BadAccelerator {
        fn solve(
            &mut self,
            _: &[crate::hair::HairLinearSystem],
        ) -> Result<Vec<Vec<f64>>, &'static str> {
            panic!("coordinate recovery must retain the original free load");
        }
        fn joint_contact_coordinates_enabled(&self) -> bool {
            true
        }
        fn solve_joint_coordinates(
            &mut self,
            _: &[Vec<f64>],
            _: &[f64],
            _: f64,
        ) -> Option<(Vec<f64>, Vec<f64>)> {
            self.calls += 1;
            Some((vec![f64::NAN], vec![f64::NAN]))
        }
    }
    let mut backend = BadAccelerator { calls: 0 };
    qualify_production_restoration(Some(&mut backend));
    assert!(
        backend.calls > 0,
        "captured recovery never exercised the accelerator boundary"
    );
}

#[test]
#[ignore = "requires actual full-runtime contact entry VHR"]
fn captured_runtime_recovery_wrapper() {
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    assert!(collision && continuous);
    let roots = rods.iter().map(|r| r.x[0]).collect::<Vec<_>>();
    let began = std::time::Instant::now();
    let result = HairSystem::reconcile_positions_mode(
        &mut rods,
        &meshes,
        dt,
        radius,
        true,
        &mut Vec::new(),
        None,
        None,
        true,
    );
    eprintln!(
        "RUNTIME WRAPPER elapsed_s={} result={result:?}",
        began.elapsed().as_secs_f64()
    );
    result.unwrap();
    assert_recovery_geometry(&mut rods, &meshes, &roots, radius);
}

#[test]
#[ignore = "requires actual contact entry VHR; experimental feasible-strand preservation"]
fn captured_restoration_preserves_clear_strand_trajectory() {
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    assert!(collision && continuous);
    contact::admit_staged_strands(&rods, radius).unwrap();
    let roots = rods.iter().map(|r| r.x[0]).collect::<Vec<_>>();
    let began = std::time::Instant::now();
    let result = super::feasibility_restoration::restore_preserving_clear_strands(
        &mut rods, &meshes, dt, radius,
    );
    eprintln!(
        "FEASIBLE STRAND RESTORATION elapsed_s={} result={result:?}",
        began.elapsed().as_secs_f64()
    );
    assert!(
        result.unwrap(),
        "feasible-strand recovery did not establish a proposal"
    );
    assert_recovery_geometry(&mut rods, &meshes, &roots, radius);
    let before = rods
        .iter()
        .map(|r| (r.x.clone(), r.q.clone()))
        .collect::<Vec<_>>();
    HairSystem::reconcile_positions_attempt(
        &mut rods,
        &meshes,
        dt,
        radius,
        true,
        &mut Vec::new(),
        None,
        None,
        true,
        128,
    )
    .unwrap();
    assert_recovery_admission(&mut rods, &meshes, &before, &roots, dt, radius);
}

#[test]
#[ignore = "requires feasible VHR and original VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE"]
fn captured_feasible_recovery_native_continuation() {
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    let reference = std::env::var("VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE").unwrap();
    let (ref_dt, ref_radius, _, ref_collision, origin, ref_meshes, ref_continuous) =
        contact_replay::tests::read_replay(reference);
    assert_eq!(
        (dt, radius, collision, continuous),
        (ref_dt, ref_radius, ref_collision, ref_continuous)
    );
    assert!(collision && continuous);
    assert_eq!(rods.len(), origin.len());
    for (rod, old) in rods.iter().zip(&origin) {
        assert_eq!(rod.old_x, old.old_x);
        assert_eq!(rod.predicted_x, old.predicted_x);
        assert_eq!(rod.old_q, old.old_q);
        assert_eq!(rod.predicted_q, old.predicted_q);
        assert_eq!(rod.rest_x, old.rest_x);
        assert_eq!(rod.rest_q, old.rest_q);
    }
    assert_eq!(meshes.len(), ref_meshes.len());
    for (mesh, old) in meshes.iter().zip(&ref_meshes) {
        let (a, b) = (mesh.replay_geometry(), old.replay_geometry());
        assert!(a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && a.4 == b.4);
        let (mut fa, mut fb) = (a.3, b.3);
        fa.sort_unstable();
        fb.sort_unstable();
        assert_eq!(fa, fb);
    }
    let before = origin
        .iter()
        .map(|r| (r.x.clone(), r.q.clone()))
        .collect::<Vec<_>>();
    let roots = origin.iter().map(|r| r.x[0]).collect::<Vec<_>>();
    let pairs = refresh(&mut rods, &meshes, radius, true, true);
    assert_eq!(
        HairSystem::contact_merit(&rods, &pairs, radius).unwrap(),
        0.
    );
    contact::admit_staged_strands(&rods, radius).unwrap();
    mesh_motion::admit(&rods, &meshes, radius).unwrap();
    let feasible = rods
        .iter()
        .map(|r| (r.x.clone(), r.q.clone()))
        .collect::<Vec<_>>();
    eprintln!(
        "FEASIBLE CONTINUATION initial_original_work_j={:e}",
        HairSystem::elastic_energy_change(&rods, &before, dt).unwrap()
    );
    let result = HairSystem::reconcile_positions_attempt(
        &mut rods,
        &meshes,
        dt,
        radius,
        true,
        &mut Vec::new(),
        None,
        None,
        true,
        128,
    );
    let terminal = std::env::temp_dir().join(format!(
        "voxy-native-continuation-{}.vhr",
        std::process::id()
    ));
    let export = contact_replay::save_measured(&terminal, &rods, &meshes, dt, radius, true, true);
    eprintln!("FEASIBLE CONTINUATION result={result:?} terminal={terminal:?} export={export:?}");
    result.unwrap();
    eprintln!(
        "FEASIBLE CONTINUATION final_original_work_j={:e}",
        HairSystem::elastic_energy_change(&rods, &before, dt).unwrap()
    );
    assert_recovery_admission(&mut rods, &meshes, &feasible, &roots, dt, radius);
}

#[test]
#[ignore = "requires recovery VHR and original VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE"]
fn captured_recovery_implicit_energy_inventory() {
    let (dt, radius, _, collision, rods, _, continuous) =
        contact_replay::tests::load_replay_with_mode();
    let reference = std::env::var("VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE").unwrap();
    let (ref_dt, ref_radius, _, ref_collision, origin, _, ref_continuous) =
        contact_replay::tests::read_replay(reference);
    assert_eq!(
        (dt, radius, collision, continuous),
        (ref_dt, ref_radius, ref_collision, ref_continuous)
    );
    assert_eq!(rods.len(), origin.len());
    let mut rows = Vec::new();
    for (index, (rod, old)) in rods.iter().zip(&origin).enumerate() {
        assert_eq!(rod.predicted_x, old.predicted_x);
        assert_eq!(rod.predicted_q, old.predicted_q);
        assert_eq!(rod.old_x, old.old_x);
        assert_eq!(rod.old_q, old.old_q);
        let parts = crate::hair::direct::structural_delta_parts(rod, &old.x, &old.q, dt).unwrap();
        let delta = crate::hair::direct::structural_delta(rod, &old.x, &old.q, dt).unwrap();
        rows.push((index, delta, parts));
    }
    rows.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    for (rod, total, [inertia, elastic]) in &rows {
        eprintln!(
            "RECOVERY ENERGY {{\"rod\":{rod},\"total_j\":{total:e},\"inertia_j\":{inertia:e},\"elastic_j\":{elastic:e}}}"
        );
    }
    let before = origin
        .iter()
        .map(|r| (r.x.clone(), r.q.clone()))
        .collect::<Vec<_>>();
    eprintln!(
        "RECOVERY ENERGY TOTAL signed_original_potential_change_j={:e} scope=diagnostic_not_admission",
        HairSystem::elastic_energy_change(&rods, &before, dt).unwrap()
    );
}

fn qualify_production_restoration(solver: Option<&mut dyn crate::hair::HairLinearSolver>) {
    eprintln!(
        "RESTORATION QUALIFICATION experimental_native_current_witness_normals={}",
        std::env::var("VOXY_HAIR_NATIVE_CURRENT_WITNESS_NORMALS").as_deref() == Ok("1")
    );
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    assert!(collision && continuous);
    let roots = rods.iter().map(|r| r.x[0]).collect::<Vec<_>>();
    assert!(contact::restore_candidate(&mut rods, &meshes, dt, radius, solver).unwrap());
    assert_recovery_geometry(&mut rods, &meshes, &roots, radius);
    let before = rods
        .iter()
        .map(|r| (r.x.clone(), r.q.clone()))
        .collect::<Vec<_>>();
    safe_motion::with_current_native_normals(
        std::env::var("VOXY_HAIR_NATIVE_CURRENT_WITNESS_NORMALS").as_deref() == Ok("1"),
        || {
            HairSystem::reconcile_positions_attempt(
                &mut rods,
                &meshes,
                dt,
                radius,
                true,
                &mut Vec::new(),
                None,
                None,
                true,
                128,
            )
        },
    )
    .unwrap();
    assert_recovery_admission(&mut rods, &meshes, &before, &roots, dt, radius);
}

fn assert_recovery_admission(
    rods: &mut [HairRod],
    meshes: &[crate::hair::TriangleMesh],
    before: &[(Vec<V>, Vec<Q>)],
    roots: &[V],
    dt: f64,
    radius: f64,
) {
    assert_recovery_geometry(rods, meshes, roots, radius);
    assert!(HairSystem::elastic_energy_change(rods, before, dt).unwrap() <= 0.);
}

fn assert_recovery_geometry(
    rods: &mut [HairRod],
    meshes: &[crate::hair::TriangleMesh],
    roots: &[V],
    radius: f64,
) {
    let pairs = refresh(rods, meshes, radius, true, true);
    assert_eq!(HairSystem::contact_merit(rods, &pairs, radius).unwrap(), 0.);
    assert!(rods.iter().all(|r| r.max_relative_stretch() <= 0.05));
    contact::admit_staged_strands(rods, radius).unwrap();
    mesh_motion::admit(rods, meshes, radius).unwrap();
    for (rod, root) in rods.iter().zip(roots) {
        assert_eq!(rod.x[0], *root);
    }
}

#[test]
#[ignore = "requires actual nonlinear failure VHR; reports candidate gates, not qualification"]
fn captured_second_order_feasibility_candidate() {
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    let origin = rods.clone();
    let before: Vec<_> = rods
        .iter()
        .map(|rod| (rod.x.clone(), rod.q.clone()))
        .collect();
    let roots: Vec<_> = rods.iter().map(|rod| rod.x[0]).collect();
    let origin_pairs = refresh(&mut rods, &meshes, radius, collision, continuous);
    eprintln!(
        "SECOND ORDER ORIGIN merit={:e} mesh_path={}",
        HairSystem::contact_merit(&rods, &origin_pairs, radius).unwrap(),
        !continuous || mesh_motion::admit(&rods, &meshes, radius).is_ok()
    );
    let mut motion_start = origin.clone();
    for rod in &mut motion_start {
        rod.x.clone_from(&rod.old_x);
        rod.q.clone_from(&rod.old_q);
    }
    // The trial must be the same swept-admitted candidate that the outer
    // nonlinear mesh reconciliation sees, not an unguarded Newton step.
    let fraction = safe_motion::advance_contacts_with_pairs(
        &mut rods,
        dt,
        radius,
        None,
        &mut Vec::new(),
        (continuous && collision).then_some(motion_start.as_slice()),
        &[],
    )
    .unwrap();
    let scale: f64 = std::env::var("VOXY_HAIR_CORRECTION_TRIAL_SCALE")
        .map(|value| value.parse().unwrap())
        .unwrap_or(1.);
    assert!(scale.is_finite() && scale > 0. && scale <= 1.);
    if scale < 1. {
        for (rod, initial) in rods.iter_mut().zip(&origin) {
            for i in 1..rod.x.len() {
                rod.x[i] = add(initial.x[i], mul(sub(rod.x[i], initial.x[i]), scale));
            }
            for i in 1..rod.q.len() {
                let rotation = log(qm(rod.q[i], conj(initial.q[i])));
                rod.q[i] = initial.q[i];
                apply(&mut rod.q[i], mul(rotation, scale));
            }
        }
    }
    eprintln!("SECOND ORDER ORIGINAL CANDIDATE swept_fraction={fraction} trial_scale={scale}");
    if std::env::var("VOXY_HAIR_LOCAL_RESTORATION").as_deref() == Ok("1") {
        let zero_free = std::env::var("VOXY_HAIR_LOCAL_ZERO_FREE").as_deref() == Ok("1");
        let mut admitted = local_restoration(
            &mut rods, &meshes, &before, dt, radius, collision, continuous, zero_free,
        );
        if !admitted && std::env::var("VOXY_HAIR_LOCAL_TWO_PHASE").as_deref() == Ok("1") {
            eprintln!(
                "LOCAL RESTORATION PHASE feasibility_only original_energy_reference_retained=true"
            );
            admitted = local_restoration(
                &mut rods, &meshes, &before, dt, radius, collision, continuous, true,
            );
        }
        for (rod, root) in rods.iter().zip(&roots) {
            assert_eq!(rod.x[0], *root);
        }
        assert!(
            admitted,
            "local restoration failed independent physical and energy admission"
        );
        if std::env::var("VOXY_HAIR_LOCAL_RESUME_NATIVE").as_deref() == Ok("1") {
            eprintln!("NATIVE CONTINUATION BEGIN independent_gates=true");
            let started = std::time::Instant::now();
            let result = HairSystem::reconcile_positions_mode(
                &mut rods,
                &meshes,
                dt,
                radius,
                collision,
                &mut Vec::new(),
                None,
                None,
                continuous,
            );
            eprintln!(
                "NATIVE CONTINUATION END elapsed_s={} result={result:?}",
                started.elapsed().as_secs_f64()
            );
            assert!(
                result.is_ok(),
                "native tangent/contact reconciliation failed after feasible correction"
            );
            let fresh = refresh(&mut rods, &meshes, radius, collision, continuous);
            let merit = HairSystem::contact_merit(&rods, &fresh, radius).unwrap();
            let work = HairSystem::elastic_energy_change(&rods, &before, dt).unwrap();
            let strain = rods
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0., f64::max);
            let strand_path = !collision || contact::admit_staged_strands(&rods, radius).is_ok();
            let mesh_path = !continuous || mesh_motion::admit(&rods, &meshes, radius).is_ok();
            eprintln!(
                "NATIVE FINAL GATES merit={merit:e} original_work_j={work:e} strain={strain:e} strand_path={strand_path} mesh_path={mesh_path}"
            );
            assert!(merit == 0. && work < 0. && strain <= 0.05 && strand_path && mesh_path);
            for (rod, root) in rods.iter().zip(&roots) {
                assert_eq!(rod.x[0], *root);
            }
        }
        return;
    }
    if std::env::var("VOXY_HAIR_CORRECTION_DIRECTION_AUDIT").as_deref() == Ok("1") {
        let pairs = refresh(&mut rods, &meshes, radius, collision, continuous);
        let baseline = HairSystem::contact_merit(&rods, &pairs, radius).unwrap();
        let free = NativeNewtonStep::new(&rods, dt, 1e-14).unwrap().free;
        let projected = std::env::var("VOXY_HAIR_CORRECTION_AUDIT_PROJECTED").as_deref() == Ok("1");
        let direction = if projected {
            let (mut rows, _) = position_constraints(&rods, &pairs, radius).unwrap();
            square_root_projection::solve(&mut rows, &rods, dt, free, 1e-14).unwrap()
        } else {
            free
        };
        eprintln!("CONTACT DIRECTION AUDIT projected={projected}");
        let maximum = direction
            .linear
            .iter()
            .flatten()
            .map(|value| len(*value))
            .fold(0., f64::max);
        assert!(maximum.is_finite() && maximum > 0.);
        for epsilon in [1e-7, 1e-8, 1e-9, 1e-10] {
            let mut differences = Vec::new();
            for sign in [-1., 1.] {
                let mut probe = rods.clone();
                for (r, rod) in probe.iter_mut().enumerate() {
                    for i in 1..rod.x.len() {
                        rod.x[i] = add(
                            rod.x[i],
                            mul(direction.linear[r][i], sign * epsilon / maximum),
                        );
                    }
                }
                let frozen = HairSystem::contact_merit(&probe, &pairs, radius).unwrap();
                let frozen_mesh = HairSystem::contact_merit(&probe, &[], radius).unwrap();
                let fresh_pairs = refresh(&mut probe, &meshes, radius, collision, continuous);
                let refreshed = HairSystem::contact_merit(&probe, &fresh_pairs, radius).unwrap();
                let refreshed_mesh = HairSystem::contact_merit(&probe, &[], radius).unwrap();
                assert!(frozen.is_finite() && refreshed.is_finite());
                differences.push((frozen, refreshed, frozen_mesh, refreshed_mesh));
            }
            let frozen_derivative = (differences[1].0 - differences[0].0) / (2. * epsilon);
            let refreshed_derivative = (differences[1].1 - differences[0].1) / (2. * epsilon);
            eprintln!(
                "CONTACT DIRECTION AUDIT epsilon_m={epsilon:e} baseline={baseline:e} frozen_derivative={frozen_derivative:e} refreshed_derivative={refreshed_derivative:e} frozen_minus={:e} frozen_plus={:e} refreshed_minus={:e} refreshed_plus={:e}",
                differences[0].0, differences[1].0, differences[0].1, differences[1].1
            );
            let frozen_mesh_derivative = (differences[1].2 - differences[0].2) / (2. * epsilon);
            let refreshed_mesh_derivative = (differences[1].3 - differences[0].3) / (2. * epsilon);
            eprintln!(
                "CONTACT DIRECTION PARTS epsilon_m={epsilon:e} frozen_mesh={frozen_mesh_derivative:e} refreshed_mesh={refreshed_mesh_derivative:e} frozen_strand={:e} refreshed_strand={:e}",
                frozen_derivative - frozen_mesh_derivative,
                refreshed_derivative - refreshed_mesh_derivative
            );
        }
        return;
    }
    // IPC OGC sizes its broad-phase trust region from predicted motion and
    // the contact band. This test uses that radius only as a proposal bound;
    // it does not import IPC's collision certificate for finite-radius rods.
    let predicted_motion = rods
        .iter()
        .zip(&origin)
        .flat_map(|(rod, old)| rod.x.iter().zip(&old.x).map(|(x, y)| len(sub(*x, *y))))
        .fold(0., f64::max);
    let trust_radius = predicted_motion.max(2. * radius);
    let bound_proposal = std::env::var("VOXY_HAIR_CORRECTION_TRUST_REGION").as_deref() == Ok("1");
    assert!(
        !bound_proposal || preserve_free_switch(),
        "trust proposal requires original-pose increments"
    );
    let frozen = square_root_projection::FrozenSystems::new(&origin, dt);
    let freeze_operator = match std::env::var("VOXY_HAIR_CORRECTION_FREEZE_OPERATOR").as_deref() {
        Ok("1") => true,
        Ok("0") | Err(_) => false,
        _ => panic!("correction operator switch must be 0 or 1"),
    };
    eprintln!("SECOND ORDER OPERATOR frozen={freeze_operator}");
    let preserve_free = std::env::var("VOXY_HAIR_CORRECTION_PRESERVE_FREE").as_deref() == Ok("1");
    let original_free =
        preserve_free.then(|| NativeNewtonStep::new(&origin, dt, 1e-14).unwrap().free);
    let retain_rows = std::env::var("VOXY_HAIR_CORRECTION_RETAIN_ROWS").as_deref() == Ok("1");
    assert!(
        !retain_rows || preserve_free,
        "retained rows require original-pose coordinates"
    );
    let mut retained: Vec<([Entry; 4], f64)> = Vec::new();
    let correction_limit = if retain_rows { 16 } else { 4 };

    // Compare zero-load feasibility repair and objective-preserving correction.
    // The bounded iteration count never replaces physical admission.
    let mut admitted = false;
    for correction in 0..=correction_limit {
        let pairs = refresh(&mut rods, &meshes, radius, collision, continuous);
        let merit = HairSystem::contact_merit(&rods, &pairs, radius).unwrap();
        let work = HairSystem::elastic_energy_change(&rods, &before, dt).unwrap();
        let strain = rods
            .iter()
            .map(HairRod::max_relative_stretch)
            .fold(0., f64::max);
        let mut start = origin.clone();
        for rod in &mut start {
            rod.x.clone_from(&rod.old_x);
        }
        let end: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
        let strand_path = !collision || contact::strand_fraction(&start, &end, radius) == Ok(1.);
        let mesh_path = !continuous || mesh_motion::admit(&rods, &meshes, radius).is_ok();
        let candidate_admitted =
            merit == 0. && work < 0. && strain <= 0.05 && strand_path && mesh_path;
        admitted = candidate_admitted;
        eprintln!(
            "SECOND ORDER CANDIDATE correction={correction} merit={merit:e} work_j={work:e} strain={strain:e} strand_path={strand_path} mesh_path={mesh_path} candidate_admitted={candidate_admitted}"
        );
        for (rod, root) in rods.iter().zip(&roots) {
            assert_eq!(rod.x[0], *root);
        }
        assert!(merit.is_finite() && work.is_finite() && strain.is_finite());
        if candidate_admitted || correction == correction_limit {
            break;
        }
        let (mut rows, _) = position_constraints(&rods, &pairs, radius).unwrap();
        if std::env::var("VOXY_HAIR_CORRECTION_TRAJECTORY_ROWS").as_deref() == Ok("1") {
            let mut groups = pairs.clone();
            let count = rows.len();
            safe_motion::correction_trajectory_rows(&mut rows, &rods, &start, &mut groups, radius)
                .unwrap();
            eprintln!("SECOND ORDER TRAJECTORY ROWS added={}", rows.len() - count);
        }
        let zero = PositionIncrement {
            linear: rods.iter().map(|rod| vec![[0.; 3]; rod.x.len()]).collect(),
            angular: rods.iter().map(|rod| vec![[0.; 3]; rod.q.len()]).collect(),
        };
        let mut increment = if let Some(original_free) = &original_free {
            let total = PositionIncrement {
                linear: rods
                    .iter()
                    .zip(&origin)
                    .map(|(rod, old)| rod.x.iter().zip(&old.x).map(|(x, y)| sub(*x, *y)).collect())
                    .collect(),
                angular: rods
                    .iter()
                    .zip(&origin)
                    .map(|(rod, old)| {
                        rod.q
                            .iter()
                            .zip(&old.q)
                            .map(|(q, p)| log(qm(*q, conj(*p))))
                            .collect()
                    })
                    .collect(),
            };
            let mut free = original_free.clone();
            let mut involved = vec![false; rods.len()];
            for row in &mut rows {
                // Express refreshed nonlinear residual as a total step from
                // the frozen origin. Preserve the original elastic RHS.
                row.bound += row.speed(&total);
            }
            if retain_rows {
                // Store affine rows in the common original-pose coordinates.
                // Reinsert through canonical coalescing, never copy cached
                // responses or multipliers from a different projection.
                for (entries, bound) in &retained {
                    add_constraint(&mut rows, *entries, *bound).unwrap();
                }
                let origin_safe =
                    std::env::var("VOXY_HAIR_CORRECTION_ORIGIN_SAFE").as_deref() == Ok("1");
                if origin_safe {
                    rows.retain(|row| row.bound <= 0.);
                }
                retained = rows.iter().map(|row| (row.entries, row.bound)).collect();
                eprintln!("SECOND ORDER RETAINED ROWS count={}", retained.len());
                let excluding = retained.iter().filter(|(_, bound)| *bound > 0.).count();
                let maximum_bound = retained
                    .iter()
                    .map(|(_, bound)| *bound)
                    .fold(f64::NEG_INFINITY, f64::max);
                eprintln!(
                    "SECOND ORDER ORIGIN ROWS excluding={excluding} maximum_bound_m={maximum_bound:e}"
                );
            }
            for row in &rows {
                for entry in row.entries {
                    if entry.gradient.iter().any(|&v| v != 0.) {
                        involved[entry.rod] = true;
                    }
                }
            }
            for (r, used) in involved.into_iter().enumerate() {
                for value in &mut free.linear[r] {
                    *value = if used { mul(*value, scale) } else { [0.; 3] };
                }
                for value in &mut free.angular[r] {
                    *value = if used { mul(*value, scale) } else { [0.; 3] };
                }
            }
            let increment =
                square_root_projection::solve_prepared(&mut rows, &frozen, free, 1e-14).unwrap();
            for (rod, initial) in rods.iter_mut().zip(&origin) {
                rod.x.clone_from(&initial.x);
                rod.q.clone_from(&initial.q);
            }
            increment
        } else if freeze_operator {
            square_root_projection::solve_prepared(&mut rows, &frozen, zero, 1e-14).unwrap()
        } else {
            square_root_projection::solve(&mut rows, &rods, dt, zero, 1e-14).unwrap()
        };
        if bound_proposal {
            let maximum = increment
                .linear
                .iter()
                .flatten()
                .map(|value| len(*value))
                .fold(0., f64::max);
            let factor = if maximum > trust_radius {
                trust_radius / maximum
            } else {
                1.
            };
            for value in increment
                .linear
                .iter_mut()
                .flatten()
                .chain(increment.angular.iter_mut().flatten())
            {
                *value = mul(*value, factor);
            }
            eprintln!(
                "SECOND ORDER TRUST radius_m={trust_radius:e} proposed_m={maximum:e} factor={factor:e}"
            );
        }
        for (r, rod) in rods.iter_mut().enumerate() {
            for i in 1..rod.x.len() {
                rod.x[i] = add(rod.x[i], increment.linear[r][i]);
            }
            for i in 1..rod.q.len() {
                apply(&mut rod.q[i], increment.angular[r][i]);
            }
        }
    }
    if std::env::var("VOXY_HAIR_CORRECTION_REQUIRE_ADMISSION").as_deref() == Ok("1") {
        assert!(
            admitted,
            "captured correction failed physical and energy admission"
        );
    }
}

fn preserve_free_switch() -> bool {
    std::env::var("VOXY_HAIR_CORRECTION_PRESERVE_FREE").as_deref() == Ok("1")
}

#[test]
#[ignore = "requires terminal VHR; inventories blockers without solving or claiming admission"]
fn captured_terminal_strand_blockers() {
    let (_, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    assert!(collision);
    let pairs = refresh(&mut rods, &meshes, radius, collision, continuous);
    let endpoint = HairSystem::contact_merit(&rods, &pairs, radius).unwrap();
    // Preserve the canonical per-segment maximum used by contact_merit.
    // A trajectory row's physical_gap already includes its time metric;
    // reporting it as an ordinary endpoint distance would be misleading.
    let mut mesh_merit = 0.;
    for (r, rod) in rods.iter().enumerate() {
        let mut depths = vec![0.; rod.lengths.len()];
        for segment in 0..rod.lengths.len() {
            let worst = rod
                .contacts
                .iter()
                .filter(|c| {
                    c.segment == segment && matches!(c.source, crate::hair::ContactSource::Mesh(_))
                })
                .map(|c| {
                    let point = add(
                        mul(rod.x[segment], 1. - c.fraction),
                        mul(rod.x[segment + 1], c.fraction),
                    );
                    (c, c.physical_gap(point))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((c, gap)) = worst {
                assert!(gap.is_finite());
                let excess = (-gap - 1e-10).max(0.);
                depths[segment] = excess;
                if excess > 0. {
                    eprintln!(
                        "TERMINAL MESH WITNESS rod={r} segment={segment} fraction={} physical_gap_m={gap:e} excess_m={excess:e} metric_scale={} trajectory_time={:?} source={:?}",
                        c.fraction, c.metric_scale, c.trajectory_time, c.source
                    );
                }
            }
        }
        mesh_merit += depths.iter().map(|depth| depth * depth).sum::<f64>();
    }
    assert!(mesh_merit.is_finite() && mesh_merit <= endpoint);
    eprintln!("TERMINAL MESH ACCOUNTING merit={mesh_merit:e} endpoint_total={endpoint:e}");
    let mut start = rods.clone();
    for rod in &mut start {
        rod.x.clone_from(&rod.old_x);
        rod.q.clone_from(&rod.old_q);
    }
    let (_, trajectory) = safe_motion::restoration_model(&rods, &start, &pairs, radius).unwrap();
    eprintln!(
        "TERMINAL PHYSICAL INVENTORY endpoint_merit={endpoint:e} witnessed_trajectory_merit={trajectory:e} mesh_path={:?} strand_path={:?} qualification_pass_inferred=false",
        mesh_motion::admit(&rods, &meshes, radius),
        contact::admit_staged_strands(&rods, radius)
    );
    let limits = safe_motion::correction_strand_limits(&rods, radius).unwrap();
    for (a, b, fraction) in &limits {
        assert!(
            *a < rods.len()
                && *b < rods.len()
                && fraction.is_finite()
                && *fraction > 0.
                && *fraction < 1.
        );
    }
    eprintln!(
        "TERMINAL STRAND BLOCKERS count={} first={:?}",
        limits.len(),
        limits.iter().min_by(|a, b| a.2.total_cmp(&b.2))
    );
}

#[test]
#[ignore = "requires terminal VHR and original VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE"]
fn captured_terminal_feasibility_repair() {
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    let reference = std::env::var("VOXY_HAIR_ORIGINAL_ENERGY_REFERENCE").unwrap();
    let (ref_dt, ref_radius, _, ref_collision, origin, ref_meshes, ref_continuous) =
        contact_replay::tests::read_replay(reference);
    assert_eq!(
        (dt, radius, collision, continuous),
        (ref_dt, ref_radius, ref_collision, ref_continuous)
    );
    assert_eq!(rods.len(), origin.len());
    for (rod, old) in rods.iter().zip(&origin) {
        assert_eq!(rod.rest_x, old.rest_x);
        assert_eq!(rod.old_x, old.old_x);
        assert_eq!(rod.predicted_x, old.predicted_x);
        assert_eq!(rod.x[0], old.x[0]);
    }
    assert_eq!(meshes.len(), ref_meshes.len());
    for (mesh, old) in meshes.iter().zip(&ref_meshes) {
        let a = mesh.replay_geometry();
        let b = old.replay_geometry();
        assert!(
            a.0 == b.0,
            "terminal/reference current collider vertices differ"
        );
        assert!(
            a.1 == b.1,
            "terminal/reference previous collider vertices differ"
        );
        assert!(a.2 == b.2, "terminal/reference collider velocities differ");
        assert_eq!(a.4, b.4);
        let order_changed = a.3 != b.3;
        let mut faces_a = a.3;
        let mut faces_b = b.3;
        faces_a.sort_unstable();
        faces_b.sort_unstable();
        assert!(
            faces_a == faces_b,
            "terminal/reference collider topology differs"
        );
        eprintln!(
            "TERMINAL REFERENCE collider_geometry_equal=true face_order_changed={order_changed}"
        );
    }
    let before = origin
        .iter()
        .map(|rod| (rod.x.clone(), rod.q.clone()))
        .collect::<Vec<_>>();
    let zero_free = std::env::var("VOXY_HAIR_LOCAL_ZERO_FREE").as_deref() == Ok("1");
    assert!(
        local_restoration(
            &mut rods, &meshes, &before, dt, radius, collision, continuous, zero_free
        ),
        "terminal repair failed original-energy and independent physical admission"
    );
    for (rod, old) in rods.iter().zip(&origin) {
        assert_eq!(rod.x[0], old.x[0]);
    }
}

#[test]
#[ignore = "requires VHR material/predictions and VDP1 complete rejected direction"]
fn captured_direction_energy_curve() {
    use std::io::Read;
    let (dt, radius, _, collision, mut rods, meshes, continuous) =
        contact_replay::tests::load_replay_with_mode();
    let audit_geometry = std::env::var("VOXY_HAIR_DIRECTION_GEOMETRY").as_deref() == Ok("1");
    let mut input =
        std::fs::File::open(std::env::var("VOXY_HAIR_DIRECTION_POSES").unwrap()).unwrap();
    let mut magic = [0; 4];
    input.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"VDP1");
    let read_count = |input: &mut std::fs::File| {
        let mut bytes = [0; 8];
        input.read_exact(&mut bytes).unwrap();
        usize::try_from(u64::from_le_bytes(bytes)).unwrap()
    };
    fn vectors<const N: usize>(input: &mut std::fs::File, count: usize) -> Vec<[f64; N]> {
        (0..count)
            .map(|_| {
                std::array::from_fn(|_| {
                    let mut bytes = [0; 8];
                    input.read_exact(&mut bytes).unwrap();
                    let value = f64::from_le_bytes(bytes);
                    assert!(value.is_finite());
                    value
                })
            })
            .collect()
    }
    assert_eq!(read_count(&mut input), rods.len());
    let mut before = Vec::new();
    let mut full = Vec::new();
    for rod in &rods {
        let n = read_count(&mut input);
        assert_eq!(n, rod.x.len());
        let x = vectors::<3>(&mut input, n);
        let q = vectors::<4>(&mut input, n - 1);
        let fx = vectors::<3>(&mut input, n);
        let fq = vectors::<4>(&mut input, n - 1);
        assert_eq!(x[0], fx[0], "direction moved prescribed root");
        before.push((x, q));
        full.push((fx, fq));
    }
    let mut extra = [0];
    assert_eq!(input.read(&mut extra).unwrap(), 0);
    for (rod, (x, q)) in rods.iter_mut().zip(&before) {
        rod.x.clone_from(x);
        rod.q.clone_from(q);
    }
    let origin = rods.clone();
    for exponent in [0, 1, 4, 8, 16, 24, 28, 32, 36, 40, 44, 48, 52] {
        let scale = 2f64.powi(-exponent);
        let mut slope = 0.;
        let mut changed = 0;
        for ((rod, (x, q)), (fx, fq)) in rods.iter_mut().zip(&before).zip(&full) {
            assert_eq!(q[0], fq[0], "direction moved prescribed root orientation");
            for i in 1..x.len() {
                rod.x[i] = if exponent == 0 {
                    fx[i]
                } else {
                    add(x[i], mul(sub(fx[i], x[i]), scale))
                };
                changed += usize::from(rod.x[i] != x[i]);
            }
            for i in 1..q.len() {
                rod.q[i] = q[i];
                if exponent == 0 {
                    rod.q[i] = fq[i];
                } else {
                    apply(&mut rod.q[i], mul(log(qm(fq[i], conj(q[i]))), scale));
                }
                changed += usize::from(rod.q[i] != q[i]);
            }
        }
        if exponent == 0 {
            for ((rod, (x, q)), (fx, fq)) in origin.iter().zip(&before).zip(&full) {
                let mut staged = rod.clone();
                staged.contacts.clear();
                let (_, rhs) = crate::hair::direct::assemble(&mut staged, dt).unwrap();
                for i in 1..x.len() {
                    slope -= dot(
                        [rhs[i * 6], rhs[i * 6 + 1], rhs[i * 6 + 2]],
                        sub(fx[i], x[i]),
                    );
                }
                for i in 1..q.len() {
                    slope -= dot(
                        [rhs[i * 6 + 3], rhs[i * 6 + 4], rhs[i * 6 + 5]],
                        log(qm(fq[i], conj(q[i]))),
                    );
                }
            }
            assert!(slope.is_finite());
            eprintln!("DIRECTION ENERGY original_assembly_slope_j={slope:e}");
        }
        let work = HairSystem::elastic_energy_change(&rods, &before, dt).unwrap();
        let reverse_pose = rods
            .iter()
            .map(|r| (r.x.clone(), r.q.clone()))
            .collect::<Vec<_>>();
        let reverse = HairSystem::elastic_energy_change(&origin, &reverse_pose, dt).unwrap();
        assert_eq!(work, -reverse, "signed energy change must reverse exactly");
        eprintln!(
            "DIRECTION ENERGY scale={scale:e} changed_blocks={changed} original_potential_change_j={work:e} scope=energy_only_not_geometry_or_convergence"
        );
        if audit_geometry {
            let mut probe = rods.clone();
            let pairs = refresh(&mut probe, &meshes, radius, collision, continuous);
            let merit = HairSystem::contact_merit(&probe, &pairs, radius).unwrap();
            // Evaluate each refreshed trial plane at the frozen origin too.
            // An admissible nonlinear origin need not belong to a rejected
            // trial's local halfspace; such a plane cannot simply be retained.
            let mut worst_mesh: Option<(usize, usize, f64, f64, f64, f64, Option<f64>)> = None;
            let mut origin_excluding_rows = 0usize;
            let mut origin_outside_tolerance_rows = 0usize;
            for (r, rod) in probe.iter().enumerate() {
                for row in rod
                    .contacts
                    .iter()
                    .filter(|row| matches!(row.source, ContactSource::Mesh(_)))
                {
                    let point = |x: &[V]| {
                        add(
                            mul(x[row.segment], 1. - row.fraction),
                            mul(x[row.segment + 1], row.fraction),
                        )
                    };
                    let gap = row.physical_gap(point(&rod.x));
                    let origin_gap = row.physical_gap(point(&before[r].0));
                    assert!(gap.is_finite() && origin_gap.is_finite());
                    origin_excluding_rows += usize::from(origin_gap < 0.);
                    origin_outside_tolerance_rows += usize::from(origin_gap < -1e-10);
                    if worst_mesh.as_ref().is_none_or(|worst| gap < worst.3) {
                        worst_mesh = Some((
                            r,
                            row.segment,
                            row.fraction,
                            gap,
                            origin_gap,
                            row.metric_scale,
                            row.trajectory_time,
                        ));
                    }
                }
            }
            eprintln!(
                "DIRECTION LIMITING MESH scale={scale:e} origin_excluding_rows={origin_excluding_rows} origin_outside_tolerance_rows={origin_outside_tolerance_rows} worst=(rod,segment,fraction,gap,origin_gap,metric_scale,time) {worst_mesh:?}"
            );
            let strain = probe
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0., f64::max);
            let strands = !collision || contact::admit_staged_strands(&probe, radius).is_ok();
            let mesh = !continuous || mesh_motion::admit(&probe, &meshes, radius).is_ok();
            eprintln!(
                "DIRECTION GEOMETRY scale={scale:e} merit={merit:e} strain={strain:e} strand_path={strands} mesh_path={mesh} descending_feasible={} scope=direction_only_not_tangent_completion",
                merit == 0. && strain <= 0.05 && strands && mesh && work < 0.
            );
        }
    }
}
