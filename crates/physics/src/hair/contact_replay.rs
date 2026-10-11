//! Diagnostic nonlinear replay. Spatial acceleration caches are rebuilt.
use super::*;
use std::io::{self, Write};
fn number(w: &mut impl Write, x: f64) -> io::Result<()> {
    w.write_all(&x.to_le_bytes())
}
fn count(w: &mut impl Write, x: usize) -> io::Result<()> {
    w.write_all(&(x as u64).to_le_bytes())
}
fn vectors<const N: usize>(w: &mut impl Write, x: &[[f64; N]]) -> io::Result<()> {
    count(w, x.len())?;
    for v in x {
        for x in v {
            number(w, *x)?;
        }
    }
    Ok(())
}
pub(super) fn save(
    path: &std::path::Path,
    rods: &[HairRod],
    meshes: &[TriangleMesh],
    dt: f64,
    radius: f64,
    self_collision: bool,
    merit: f64,
    continuous_mesh: bool,
) -> io::Result<()> {
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    w.write_all(if continuous_mesh { b"VHR2" } else { b"VHR1" })?;
    number(&mut w, dt)?;
    number(&mut w, radius)?;
    number(&mut w, merit)?;
    count(&mut w, usize::from(self_collision))?;
    if continuous_mesh {
        count(&mut w, 1)?;
    }
    count(&mut w, rods.len())?;
    for r in rods {
        let m = r.material;
        for x in [
            m.radius,
            m.density,
            m.young_modulus,
            m.poisson_ratio,
            m.damping,
            m.friction,
            m.air_drag,
        ] {
            number(&mut w, x)?;
        }
        for v in [
            &r.rest_x,
            &r.x,
            &r.velocity,
            &r.omega,
            &r.old_x,
            &r.predicted_x,
        ] {
            vectors(&mut w, v)?;
        }
        for v in [&r.rest_q, &r.q, &r.old_q, &r.predicted_q, &r.rest_relative] {
            vectors(&mut w, v)?;
        }
    }
    count(&mut w, meshes.len())?;
    for mesh in meshes {
        let (current, previous, velocity, faces, closed) = mesh.replay_geometry();
        vectors(&mut w, &current)?;
        vectors(&mut w, &previous)?;
        vectors(&mut w, &velocity)?;
        count(&mut w, faces.len())?;
        for f in faces {
            for i in f {
                count(&mut w, i)?;
            }
        }
        count(&mut w, usize::from(closed))?;
    }
    w.flush()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn n(r: &mut impl Read) -> f64 {
        let mut b = [0; 8];
        r.read_exact(&mut b).unwrap();
        f64::from_le_bytes(b)
    }
    fn c(r: &mut impl Read) -> usize {
        let mut b = [0; 8];
        r.read_exact(&mut b).unwrap();
        let n = u64::from_le_bytes(b) as usize;
        assert!(n <= 10_000_000);
        n
    }
    fn v<const N: usize>(r: &mut impl Read) -> Vec<[f64; N]> {
        (0..c(r)).map(|_| std::array::from_fn(|_| n(r))).collect()
    }
    #[test]
    #[ignore = "requires actual rejected nonlinear contact capture"]
    fn replay_rejected_contact_geometry() {
        run_replay(false);
    }
    #[test]
    #[ignore = "qualification of corrected geometry on historical rejected capture"]
    fn captured_contact_reaches_feasible_geometry() {
        run_replay(true);
    }
    #[test]
    #[ignore = "requires full-model captured contact geometry"]
    fn captured_interior_gradient_matches_translation_derivative() {
        let (_, radius, _, _, mut rods, meshes) = load_replay();
        for rod in &mut rods {
            contact::refresh_mesh_constraints(rod, &meshes, radius);
        }
        let (r, witness) = rods
            .iter()
            .enumerate()
            .flat_map(|(r, rod)| {
                rod.contacts
                    .iter()
                    .filter(|c| matches!(c.source, ContactSource::Mesh(_)) && c.metric_scale < 0.05)
                    .map(move |c| (r, c))
            })
            .min_by(|(ra, a), (rb, b)| {
                let gap = |r: usize, c: &RodContact| {
                    c.physical_gap(add(
                        mul(rods[r].x[c.segment], 1. - c.fraction),
                        mul(rods[r].x[c.segment + 1], c.fraction),
                    ))
                };
                gap(*ra, a).total_cmp(&gap(*rb, b))
            })
            .expect("capture must contain a weak interior envelope gradient");
        let witness = witness.clone();
        let rod = &rods[r];
        let gap = |candidate: &HairRod| {
            candidate
                .contacts
                .iter()
                .filter(|c| {
                    c.segment == witness.segment && matches!(c.source, ContactSource::Mesh(_))
                })
                .map(|c| {
                    c.physical_gap(add(
                        mul(candidate.x[c.segment], 1. - c.fraction),
                        mul(candidate.x[c.segment + 1], c.fraction),
                    ))
                })
                .fold(f64::INFINITY, f64::min)
        };
        let expected = mul(witness.normal, witness.metric_scale);
        eprintln!(
            "ENVELOPE TRANSLATION rod={r} segment={} fraction={} gap={} gradient={expected:?}",
            witness.segment,
            witness.fraction,
            gap(rod)
        );
        // Check independent endpoint motion as well as common translation:
        // the witness interpolation weights belong to the physical Jacobian.
        for endpoint in [None, Some(witness.segment), Some(witness.segment + 1)] {
            let weight = match endpoint {
                None => 1.,
                Some(i) if i == witness.segment => 1. - witness.fraction,
                Some(_) => witness.fraction,
            };
            let expected = mul(expected, weight);
            for epsilon in [1e-6, 1e-7, 1e-8] {
                let actual: V = std::array::from_fn(|axis| {
                    let mut displaced = [rod.clone(), rod.clone()];
                    for (candidate, sign) in displaced.iter_mut().zip([-1., 1.]) {
                        for (i, x) in candidate.x.iter_mut().enumerate() {
                            if endpoint.is_none_or(|point| point == i) {
                                x[axis] += sign * epsilon;
                            }
                        }
                        contact::refresh_mesh_constraints(candidate, &meshes, radius);
                    }
                    (gap(&displaced[1]) - gap(&displaced[0])) / (2. * epsilon)
                });
                eprintln!(
                    "ENVELOPE TRANSLATION endpoint={endpoint:?} epsilon={epsilon:e} derivative={actual:?} expected={expected:?} error={}",
                    len(sub(actual, expected))
                );
                if epsilon == 1e-8 {
                    assert!(
                        len(sub(actual, expected)) < 5e-6,
                        "captured envelope gradient must match refreshed geometric endpoint derivative"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires full-model captured moving mesh"]
    fn captured_mesh_sweep_matches_exhaustive_faces() {
        let (_, radius, _, _, rods, meshes) = load_replay();
        let rod = &rods[51];
        let motions: Vec<_> = (0..rod.x.len() - 1)
            .map(|i| CapsuleMotion {
                start: [rod.old_x[i], rod.old_x[i + 1]],
                end: [rod.x[i], rod.x[i + 1]],
                radius,
            })
            .collect();
        for mesh in &meshes {
            let actual = mesh
                .swept_capsule_contacts(&motions, Default::default())
                .unwrap();
            let (current, previous, _, faces, _) = mesh.replay_geometry();
            let mut expected = Vec::new();
            for (i, &motion) in motions.iter().enumerate() {
                for &face in &faces {
                    let query = sweep_capsule_triangle(
                        motion,
                        TriangleMotion {
                            start: face.map(|v| previous[v]),
                            end: face.map(|v| current[v]),
                        },
                        Default::default(),
                    )
                    .unwrap();
                    if query != CapsuleSweep::Clear {
                        expected.push((i, face, query));
                    }
                }
            }
            expected.sort_by_key(|(i, face, _)| (*i, *face));
            assert_eq!(
                actual, expected,
                "temporal BVH must retain every exhaustive full-model contact"
            );
            assert!(
                !actual.is_empty(),
                "the historical failed rod must have surface contacts"
            );
            let initial = actual
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::InitialContact { .. }))
                .count();
            let approaching = actual
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::Approach { .. }))
                .count();
            let limited = actual
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::IterationLimit { .. }))
                .count();
            eprintln!(
                "CAPTURED MESH SWEEP faces={} segments={} contacts={} initial={initial} approach={approaching} iteration_limit={limited}",
                faces.len(),
                motions.len(),
                actual.len()
            );
        }
    }
    #[test]
    #[ignore = "requires all captured full-model guide motions"]
    fn captured_full_groom_mesh_sweep() {
        let (_, radius, _, _, rods, meshes, _) = load_replay_with_mode();
        let ids: Vec<_> = rods
            .iter()
            .enumerate()
            .flat_map(|(r, rod)| (0..rod.x.len() - 1).map(move |i| (r, i)))
            .collect();
        let motions: Vec<_> = rods
            .iter()
            .flat_map(|rod| {
                (0..rod.x.len() - 1).map(move |i| {
                    let mut start = [rod.old_x[i], rod.old_x[i + 1]];
                    let mut end = [rod.x[i], rod.x[i + 1]];
                    if i == 0 {
                        start[0] = add(mul(start[0], 0.85), mul(start[1], 0.15));
                        end[0] = add(mul(end[0], 0.85), mul(end[1], 0.15));
                    }
                    CapsuleMotion { start, end, radius }
                })
            })
            .collect();
        for mesh in &meshes {
            // Clone while cold; never query this template. Each later clone
            // has the original uncached full-density geometry and topology.
            let cold_template=mesh.clone();
            let start = std::time::Instant::now();
            let contacts = mesh
                .swept_capsule_contacts(&motions, Default::default())
                .unwrap();
            let elapsed = start.elapsed();
            let initial = contacts
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::InitialContact { .. }))
                .count();
            let approaching = contacts
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::Approach { .. }))
                .count();
            let limited = contacts
                .iter()
                .filter(|(_, _, q)| matches!(q, CapsuleSweep::IterationLimit { .. }))
                .count();
            eprintln!(
                "FULL GROOM MESH SWEEP rods={} segments={} contacts={} initial={initial} approach={approaching} iteration_limit={limited} cpu_query={elapsed:?}",
                rods.len(),
                motions.len(),
                contacts.len()
            );
            assert!(!contacts.is_empty());
            if std::env::var_os("VOXY_HAIR_MESH_BOUNDS_BENCHMARK").is_some() {
                for repeat in 0..7 {
                    let cold=cold_template.clone();
                    let timed=|candidate:&TriangleMesh| {
                        let start=std::time::Instant::now();
                        let found=candidate.swept_capsule_contacts(&motions,Default::default()).unwrap();
                        let elapsed=start.elapsed();
                        assert_eq!(found,contacts,"cache changed a full-density contact query");
                        elapsed.as_secs_f64()*1000.
                    };
                    let (cold_ms,warm_ms)=if repeat%2==0 {(timed(&cold),timed(mesh))} else {let warm=timed(mesh);(timed(&cold),warm)};
                    eprintln!("MESH BOUNDS CACHE PAIR repeat={repeat} cold_ms={cold_ms} warm_ms={warm_ms}");
                }
            }
            for (index, face, query) in contacts {
                if matches!(query, CapsuleSweep::IterationLimit { .. }) {
                    eprintln!(
                        "FULL GROOM MESH SWEEP LIMITED rod={} segment={} face={face:?} outcome={query:?}",
                        ids[index].0, ids[index].1
                    );
                }
                if let CapsuleSweep::Approach {
                    fraction, gap_m, ..
                } = query
                {
                    assert!(fraction.is_finite() && (0. ..=1.).contains(&fraction));
                    assert!(gap_m >= 0. && gap_m <= 2e-10);
                }
            }
        }
    }
    #[test]
    fn continuous_replay_retains_mode_and_authoritative_motion_endpoints() {
        let rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap();
        let vertices = [[0., 0., 1.], [1., 0., 1.], [0., 1., 1.]];
        let mut mesh = TriangleMesh::new(&vertices, &[[0, 1, 2]]).unwrap();
        mesh.refit_with_timestep(&vertices.map(|p| add(p, [0., 0., 0.01])), 1. / 240.)
            .unwrap();
        let path = std::env::temp_dir().join(format!(
            "voxy-continuous-replay-{}-{}.vhr",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        save(
            &path,
            &[rod.clone()],
            &[mesh.clone()],
            1. / 240.,
            40e-6,
            false,
            0.,
            true,
        )
        .unwrap();
        let (_, _, _, _, rods, meshes, mode) = read_replay(&path);
        std::fs::remove_file(path).unwrap();
        assert!(mode);
        assert_eq!(rods[0].x, rod.x);
        assert_eq!(meshes[0].replay_geometry().0, mesh.replay_geometry().0);
        assert_eq!(meshes[0].replay_geometry().1, mesh.replay_geometry().1);
    }
    fn load_replay() -> (f64, f64, f64, bool, Vec<HairRod>, Vec<TriangleMesh>) {
        let (dt, radius, merit, collision, rods, meshes, continuous) = load_replay_with_mode();
        assert!(
            !continuous,
            "this legacy diagnostic does not cover continuous mesh contact mode"
        );
        (dt, radius, merit, collision, rods, meshes)
    }
    fn load_replay_with_mode() -> (f64, f64, f64, bool, Vec<HairRod>, Vec<TriangleMesh>, bool) {
        let path = std::env::var("VOXY_HAIR_CONTACT_REPLAY_FILE").unwrap();
        read_replay(path)
    }
    fn read_replay(
        path: impl AsRef<std::path::Path>,
    ) -> (f64, f64, f64, bool, Vec<HairRod>, Vec<TriangleMesh>, bool) {
        let mut reader = std::io::BufReader::new(std::fs::File::open(path).unwrap());
        let mut magic = [0; 4];
        reader.read_exact(&mut magic).unwrap();
        assert!(&magic == b"VHR1" || &magic == b"VHR2");
        let dt = n(&mut reader);
        let radius = n(&mut reader);
        let expected = n(&mut reader);
        let self_collision = c(&mut reader) != 0;
        let continuous_mesh = if &magic == b"VHR2" {
            let flag = c(&mut reader);
            assert_eq!(flag, 1);
            true
        } else {
            false
        };
        let mut rods = Vec::new();
        for _ in 0..c(&mut reader) {
            let material = HairMaterial {
                radius: n(&mut reader),
                density: n(&mut reader),
                young_modulus: n(&mut reader),
                poisson_ratio: n(&mut reader),
                damping: n(&mut reader),
                friction: n(&mut reader),
                air_drag: n(&mut reader),
            };
            let mut rod = HairRod::new(v(&mut reader), material).unwrap();
            rod.x = v(&mut reader);
            rod.velocity = v(&mut reader);
            rod.omega = v(&mut reader);
            rod.old_x = v(&mut reader);
            rod.predicted_x = v(&mut reader);
            rod.rest_q = v(&mut reader);
            rod.q = v(&mut reader);
            rod.old_q = v(&mut reader);
            rod.predicted_q = v(&mut reader);
            rod.rest_relative = v(&mut reader);
            rods.push(rod);
        }
        let mut meshes = Vec::new();
        for _ in 0..c(&mut reader) {
            let positions = v(&mut reader);
            let previous = v(&mut reader);
            let velocity = v(&mut reader);
            let faces: Vec<[usize; 3]> = (0..c(&mut reader))
                .map(|_| std::array::from_fn(|_| c(&mut reader)))
                .collect();
            let closed = c(&mut reader) != 0;
            let mut mesh = TriangleMesh::new(&positions, &faces).unwrap();
            if closed {
                mesh.enable_closed_feature_normals().unwrap();
            }
            mesh.restore_replay_motion(&previous, &velocity);
            meshes.push(mesh);
        }
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap(),
            0,
            "unexpected capture trailing bytes"
        );
        (
            dt,
            radius,
            expected,
            self_collision,
            rods,
            meshes,
            continuous_mesh,
        )
    }
    #[test]
    #[ignore = "requires rejected complete-step admission capture"]
    fn captured_elastic_contact_preserves_strain_admission() {
        let (dt, radius, expected, self_collision, mut rods, mut meshes, continuous_mesh) = load_replay_with_mode();
        // Legacy admission captures saved full-frame collider endpoints while
        // rod.old_x belonged to the LAST substep. This explicit diagnostic
        // correction is only for a capture with a verified source substep count.
        if let Ok(value)=std::env::var("VOXY_HAIR_ADMISSION_CAPTURE_FRAME_SUBSTEPS") {
            assert!(continuous_mesh,"time normalization requires continuous capture");
            let count:usize=value.parse().unwrap();assert!((2..=32).contains(&count));
            let start=(count-1) as f64/count as f64;
            for mesh in &mut meshes {
                let (current,previous,velocity,_,_)=mesh.replay_geometry();
                let previous:Vec<_>=previous.iter().zip(&current).map(|(a,b)|add(mul(*a,1.-start),mul(*b,start))).collect();
                mesh.restore_replay_motion(&previous,&velocity);
            }
            eprintln!("ADMISSION CAPTURE LAST SUBSTEP start_fraction={start} verified_substeps={count}");
        }
        let roots: Vec<_> = rods.iter().map(|rod| rod.x[0]).collect();
        let mut history = Vec::new();
        if self_collision {
            history = contact::refresh_strand_responses(&mut rods, radius, &[]);
        }
        HairSystem::refresh_mesh_geometry(&mut rods,&meshes,radius,continuous_mesh).unwrap();
        let actual = HairSystem::contact_merit(&rods, &history, radius).unwrap();
        assert!(
            (actual - expected).abs() <= 1e-12 * expected.max(1e-20),
            "admission replay geometry changed"
        );
        assert!(
            rods.iter().any(|rod| rod.max_relative_stretch() > 0.05),
            "capture must reproduce rejected strain"
        );
        if let Some(path)=std::env::var_os("VOXY_HAIR_NORMALIZED_ADMISSION_REPLAY_EXPORT") {
            super::save(std::path::Path::new(&path),&rods,&meshes,dt,radius,self_collision,actual,continuous_mesh).unwrap();
        }
        for iteration in 0..64 {
            for rod in &mut rods {
                contact::refresh_mesh_constraints(rod, &meshes, radius);
                direct::solve(rod, dt).expect("captured elastic correction");
            }
            HairSystem::reconcile_positions_mode(
                &mut rods, &meshes, dt, radius, self_collision,
                &mut history, None, None, continuous_mesh,
            )
            .expect("captured contact correction");
            let strain = rods
                .iter()
                .map(HairRod::max_relative_stretch)
                .fold(0., f64::max);
            eprintln!("ELASTIC CONTACT REPLAY iteration={iteration} maximum_strain={strain:e}");
            if strain <= 0.05 {
                if let Some(path)=std::env::var_os("VOXY_HAIR_ELASTIC_FINAL_REPLAY_EXPORT") {
                    let pairs=if self_collision {contact::refresh_strand_responses(&mut rods,radius,&[])} else {Vec::new()};
                    HairSystem::refresh_mesh_geometry(&mut rods,&meshes,radius,continuous_mesh).unwrap();
                    let merit=HairSystem::contact_merit(&rods,&pairs,radius).unwrap();
                    super::save(std::path::Path::new(&path),&rods,&meshes,dt,radius,self_collision,merit,continuous_mesh).unwrap();
                }
                if self_collision && std::env::var_os("VOXY_HAIR_ELASTIC_STRAND_SWEEP_AUDIT").is_some() {
                    let end:Vec<_>=rods.iter().map(|rod|rod.x.clone()).collect();
                    let mut start=rods.clone();
                    for rod in &mut start {rod.x.clone_from(&rod.old_x);}
                    let fraction=contact::strand_fraction(&start,&end,radius).expect("captured whole-step strand motion must admit");
                    eprintln!("ELASTIC CONTACT REPLAY WHOLE STRAND SWEEP fraction={fraction}");
                    assert_eq!(fraction,1.,"feasible endpoints alone cannot qualify strand trajectories");
                }
                for (rod, root) in rods.iter().zip(&roots) {
                    assert_eq!(rod.x[0], *root);
                }
                return;
            }
        }
        panic!("coupled elastic/contact solve must satisfy unchanged strain admission");
    }
    fn run_replay(require_feasible: bool) {
        let (dt, radius, expected, self_collision, mut rods, meshes, continuous_mesh) =
            load_replay_with_mode();
        if let Ok(probe) = std::env::var("VOXY_HAIR_REPLAY_PROBE") {
            let (rod, segment) = probe.split_once(':').expect("probe is rod:segment");
            let rod = &rods[rod.parse::<usize>().unwrap()];
            let i = segment.parse::<usize>().unwrap();
            for t in [
                0.,
                0.85,
                0.86,
                0.88,
                0.9,
                0.95,
                0.97,
                0.98,
                0.99,
                0.991091841156475,
                1.,
            ] {
                let p = add(mul(rod.x[i], 1. - t), mul(rod.x[i + 1], t));
                for mesh in &meshes {
                    eprintln!(
                        "CAPTURED INTERVAL PROBE fraction={t} signed_distance={:?} ray_inside={:?}",
                        mesh.signed_distance_closed(p),
                        mesh.contains_closed_surface(p)
                    );
                }
            }
        }
        let pairs = if self_collision {
            contact::refresh_strand_responses(&mut rods, radius, &[])
        } else {
            Vec::new()
        };
        for rod in &mut rods {
            contact::refresh_mesh_constraints(rod, &meshes, radius);
        }
        if continuous_mesh {
            super::super::mesh_motion::refresh(&mut rods, &meshes, radius).unwrap();
        }
        let actual = HairSystem::contact_merit(&rods, &pairs, radius).unwrap();
        eprintln!(
            "REPLAY MERIT captured={expected:e} rebuilt={actual:e} rods={} meshes={}",
            rods.len(),
            meshes.len()
        );
        if !require_feasible {
            assert!(
                (actual - expected).abs() <= 1e-12 * expected.max(1e-20),
                "rebuilt geometry must match captured merit"
            );
        }
        let roots: Vec<_> = rods.iter().map(|rod| rod.x[0]).collect();
        let result = HairSystem::reconcile_positions_mode(
            &mut rods,
            &meshes,
            dt,
            radius,
            self_collision,
            &mut Vec::new(),
            None,
            None,
            continuous_mesh,
        );
        eprintln!("REPLAY RESULT {result:?}");
        if require_feasible {
            result.expect("corrected captured geometry must converge");
            for (rod, root) in rods.iter().zip(roots) {
                assert_eq!(rod.x[0], root);
                assert!(
                    rod.max_relative_stretch() <= 0.05,
                    "contact solve must preserve original strain admission"
                );
            }
        } else {
            assert_eq!(
                result,
                Err("joint hair contact direction has no descending geometry step")
            );
        }
        if let Some(path)=std::env::var_os("VOXY_HAIR_REPLAY_FINAL_STATE_EXPORT") {
            let mut out=std::io::BufWriter::new(std::fs::File::create(path).unwrap());
            for rod in &rods {vectors(&mut out,&rod.x).unwrap();vectors(&mut out,&rod.q).unwrap();}
            out.flush().unwrap();
        }
    }
    #[test]
    #[ignore = "exports original full-density systems from a diagnostic capture"]
    fn export_captured_structural_batch() {
        let (dt, radius, _, self_collision, mut rods, meshes) = load_replay();
        if self_collision {
            contact::refresh_strand_responses(&mut rods, radius, &[]);
        }
        for rod in &mut rods {
            contact::refresh_mesh_constraints(rod, &meshes, radius);
        }
        let mut rows = Vec::new();
        for rod in &rods {
            let system = rod.linear_system(dt).unwrap();
            assert!(
                system
                    .matrix
                    .iter()
                    .chain(&system.rhs)
                    .all(|x| x.is_finite())
            );
            let native = system.solve_native().unwrap();
            system.validate_correction(&native).unwrap();
            rows.push(format!("{{\"band_width\":{},\"active_start\":{},\"active_end\":{},\"matrix\":{:?},\"rhs\":{:?}}}",system.band_width,system.active.start,system.active.end,system.matrix,system.rhs));
        }
        let path = std::env::var("VOXY_HAIR_STRUCTURAL_BATCH_EXPORT").unwrap();
        std::fs::write(path, format!("[{}]", rows.join(","))).unwrap();
        eprintln!("CAPTURED STRUCTURAL EXPORT systems={} dt={dt}", rods.len());
    }
    #[test]
    #[ignore = "paired broad-phase benchmark on actual full-density capture"]
    fn captured_strand_refresh_matches_reference_and_profiles() {
        let (_, radius, _, self_collision, rods, _) = load_replay();
        assert!(self_collision);
        let roots: Vec<_> = rods.iter().map(|r| r.x.clone()).collect();
        let mut reference = rods.clone();
        let expected = contact::reference_strand_refresh(&mut reference, radius);
        let mut candidate = rods;
        let actual = contact::refresh_strand_responses(&mut candidate, radius, &[]);
        assert_eq!(actual.len(), expected.len());
        for (a, b) in actual.iter().zip(&expected) {
            assert_eq!(
                (a.a, a.b, a.normal, a.impulse),
                (b.a, b.b, b.normal, b.impulse),
                "pair geometry and order changed"
            );
        }
        for ((a, b), x) in candidate.iter().zip(&reference).zip(roots) {
            assert_eq!(
                format!("{:?}", a.contacts),
                format!("{:?}", b.contacts),
                "contact planes changed"
            );
            assert_eq!(a.x, x, "refresh moved a guide");
        }
        let mut times = [Vec::new(), Vec::new()];
        for repeat in 0..12 {
            for mode in [repeat % 2, 1 - repeat % 2] {
                let started = std::time::Instant::now();
                let result = if mode == 0 {
                    contact::reference_strand_refresh(&mut reference, radius)
                } else {
                    contact::refresh_strand_responses(&mut candidate, radius, &[])
                };
                times[mode].push(started.elapsed().as_secs_f64() * 1000.);
                assert_eq!(result.len(), expected.len());
                std::hint::black_box(result);
            }
        }
        for t in &mut times {
            t.sort_by(f64::total_cmp);
        }
        eprintln!(
            "CAPTURED STRAND REFRESH pairs={} reference_median_ms={} filtered_median_ms={}",
            expected.len(),
            times[0][6],
            times[1][6]
        );
    }
}
