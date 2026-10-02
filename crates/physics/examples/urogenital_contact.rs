//! Synthetic compression with explicitly selected opposing inner-wall node pairs.
use physics::biomechanics::*;
use std::io::Write;
fn cavity_volume(body: &Body) -> f64 {
    body.cavities()[0]
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| body.positions()[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
// Reference-area dead traction: explicitly not a follower pressure law.
fn reference_pressure_forces(
    points: &[[f64; 3]],
    faces: &[[usize; 3]],
    pressure: f64,
) -> Vec<[f64; 3]> {
    let mut forces = vec![[0.; 3]; points.len()];
    for face in faces {
        let [a, b, c] = face.map(|i| points[i]);
        let u: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let v: [f64; 3] = std::array::from_fn(|k| c[k] - a[k]);
        let normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &node in face {
            for k in 0..3 {
                forces[node][k] -= pressure * normal[k] / 6.;
            }
        }
    }
    forces
}
fn install_pressure_caps(
    body: &mut Body,
    rings: usize,
    sectors: usize,
    segments: usize,
    first_anchor: usize,
    balanced: bool,
) -> Result<Vec<usize>, &'static str> {
    let side: Vec<_> = body
        .surface()
        .into_iter()
        .filter(|face| {
            face.iter()
                .all(|i| i % (rings * sectors) >= (rings - 1) * sectors)
        })
        .collect();
    let bottom = (rings - 1) * sectors;
    let top = segments * rings * sectors + bottom;
    let count = if balanced { sectors } else { 1 };
    let mut indices = Vec::new();
    for offset in 0..count {
        let anchor = (first_anchor + offset) % sectors;
        let mut faces = side.clone();
        for k in 1..sectors - 1 {
            let index = |k| (anchor + k) % sectors;
            faces.push([bottom + anchor, bottom + index(k + 1), bottom + index(k)]);
            faces.push([top + anchor, top + index(k), top + index(k + 1)]);
        }
        indices.push(body.cavities().len());
        body.add_gauge_pressure_cavity(Cavity {
            faces,
            pressure_pa: 0.,
        })?;
    }
    Ok(indices)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-urogenital-contact".into());
    let sectors = std::env::args()
        .nth(2)
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(8);
    if !(8..=128).contains(&sectors) || sectors % 4 != 0 {
        return Err("sector count must be 8..=128 and divisible by four".into());
    }
    std::fs::create_dir_all(&output)?;
    let reference_pressure = std::env::args()
        .find_map(|arg| arg.strip_prefix("--reference-pressure=").map(str::to_owned))
        .map(|value| value.parse::<f64>())
        .transpose()?;
    if reference_pressure.is_some_and(|p| !p.is_finite() || p < 0.) {
        return Err("reference pressure must be finite and nonnegative".into());
    }
    let reference_caps = std::env::args().any(|arg| arg == "--reference-caps");
    if reference_caps && reference_pressure.is_none() {
        return Err("reference caps require reference pressure".into());
    }
    let follower_pressure = std::env::args()
        .find_map(|arg| arg.strip_prefix("--follower-pressure=").map(str::to_owned))
        .map(|value| value.parse::<f64>())
        .transpose()?;
    if follower_pressure.is_some_and(|p| !p.is_finite() || p < 0.)
        || (follower_pressure.is_some() && reference_pressure.is_some())
    {
        return Err("choose one finite nonnegative external load pressure".into());
    }
    let cap_anchor = std::env::args()
        .find_map(|arg| arg.strip_prefix("--cap-anchor=").map(str::to_owned))
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(0);
    if cap_anchor >= sectors {
        return Err("cap anchor must be smaller than sector count".into());
    }
    let balanced_caps = std::env::args().any(|arg| arg == "--balanced-caps");
    if balanced_caps && follower_pressure.is_none() {
        return Err("balanced caps require follower pressure".into());
    }
    let stress_from =
        std::env::args().find_map(|arg| arg.strip_prefix("--stress-from=").map(str::to_owned));
    let specimen =
        std::env::args().find_map(|arg| arg.strip_prefix("--specimen=").map(str::to_owned));
    if specimen
        .as_deref()
        .is_some_and(|s| !["urethra", "vagina"].contains(&s))
    {
        return Err("specimen must be urethra or vagina".into());
    }
    let stress_stages: Vec<String> = std::env::args()
        .find_map(|arg| arg.strip_prefix("--stress-stages=").map(str::to_owned))
        .unwrap_or_else(|| "rest,barrier,released".into())
        .split(',')
        .map(str::to_owned)
        .collect();
    if stress_stages
        .iter()
        .any(|stage| !["rest", "compressed", "barrier", "released"].contains(&stage.as_str()))
    {
        return Err("invalid stress stage selection".into());
    }
    let radial = std::env::args()
        .find_map(|arg| arg.strip_prefix("--radial=").map(str::to_owned))
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    if !(1..=3).contains(&radial) {
        return Err("radial subdivisions must be 1..=3".into());
    }
    let rings = 4 * radial + 1;
    let segments = std::env::args()
        .find_map(|arg| arg.strip_prefix("--segments=").map(str::to_owned))
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(3);
    if !(3..=63).contains(&segments) || segments % 3 != 0 {
        return Err("segments must be 3..=63 and divisible by three".into());
    }
    let surface_only = std::env::args().any(|arg| arg == "--surface-only");
    let trace = std::env::args().any(|arg| arg == "--trace");
    let surface_mode = surface_only || std::env::args().any(|arg| arg == "--surface");
    let contact_mode = if surface_only {
        "surface_only"
    } else if surface_mode {
        "pairs_and_surface"
    } else {
        "selected_pairs"
    };
    let passive = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    let muscle = |direction| Material {
        fibers: vec![Fiber {
            direction,
            stiffness_pa: 50.,
            exponent: 2.,
            active_pa: 1000.,
        }],
        ..passive.clone()
    };
    let load_mode = if follower_pressure.is_some() {
        "closed_follower_pressure"
    } else if reference_pressure.is_some() {
        "reference_dead_traction"
    } else {
        "selected_point_forces"
    };
    let load_pressure_pa = follower_pressure.or(reference_pressure).unwrap_or(0.);
    if stress_from.is_none() {
        println!(
            "specimen,sectors,stage,lumen_volume_m3,tip_z_m,min_j,residual_n,min_pair_distance_m,barrier_energy_j,segments,contact_mode,radial,cap_anchor,balanced_caps,load_mode,load_pressure_pa,reference_caps"
        );
    }
    for (name, radii, length, scales) in [
        (
            "urethra",
            [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
            0.02,
            [1., 0.6],
        ),
        (
            "vagina",
            [0.008, 0.009, 0.01, 0.011, 0.012],
            0.03,
            [1., 0.3],
        ),
    ] {
        if specimen.as_deref().is_some_and(|s| s != name) {
            continue;
        }
        let layers = [
            passive.clone(),
            muscle([0., 0., 1.]),
            muscle([1., 0., 0.]),
            passive.clone(),
        ];
        let mut profiles = vec![layers; segments];
        for profile in &mut profiles[segments / 3..2 * segments / 3] {
            profile[3] = muscle([1., 0., 0.]);
        }
        let mut original = UrogenitalWallGeometry {
            radii_m: radii,
            length_m: length,
            sectors,
            segments,
        }
        .axial_wall_refined(scales, &profiles, radial)?;
        if let Some(source) = &stress_from {
            for stage in ["rest", "compressed", "barrier", "released"] {
                if stage != "rest" && !stress_stages.iter().any(|s| s == stage) {
                    continue;
                }
                let text = std::fs::read_to_string(format!("{source}/{name}-{stage}.obj"))?;
                let mut points = Vec::new();
                let mut faces = Vec::new();
                for line in text.lines() {
                    let fields: Vec<_> = line.split_whitespace().collect();
                    if fields.first() == Some(&"v") && fields.len() == 4 {
                        points.push([
                            fields[1].parse::<f64>()?,
                            fields[2].parse()?,
                            fields[3].parse()?,
                        ]);
                    } else if fields.first() == Some(&"f") && fields.len() == 4 {
                        faces.push([
                            fields[1]
                                .parse::<usize>()?
                                .checked_sub(1)
                                .ok_or("invalid face")?,
                            fields[2]
                                .parse::<usize>()?
                                .checked_sub(1)
                                .ok_or("invalid face")?,
                            fields[3]
                                .parse::<usize>()?
                                .checked_sub(1)
                                .ok_or("invalid face")?,
                        ]);
                    }
                }
                if faces != original.surface() {
                    return Err("stress source topology mismatch".into());
                }
                if stage == "rest" && points.as_slice() != original.rest_positions() {
                    return Err("stress source reference geometry mismatch".into());
                }
                if !stress_stages.iter().any(|s| s == stage) {
                    continue;
                }
                let stresses = original.stresses_at(&points)?;
                let mut file = std::io::BufWriter::new(std::fs::File::create(format!(
                    "{output}/{name}-{stage}-stress.csv"
                ))?);
                writeln!(
                    file,
                    "element,region,reference_volume_m3,j,pressure_pa,von_mises_pa,max_shear_pa,x_m,y_m,z_m,reference_z_m,base_nodes"
                )?;
                for (i, (e, stress)) in original.elements().iter().zip(stresses).enumerate() {
                    let center: [f64; 3] = std::array::from_fn(|k| {
                        e.nodes.iter().map(|n| points[*n][k]).sum::<f64>() / 4.
                    });
                    let reference_z = e
                        .nodes
                        .iter()
                        .map(|n| original.rest_positions()[*n][2])
                        .sum::<f64>()
                        / 4.;
                    let base_nodes = e
                        .nodes
                        .iter()
                        .filter(|n| original.rest_positions()[**n][2] == 0.)
                        .count();
                    writeln!(
                        file,
                        "{i},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{reference_z:.12e},{base_nodes}",
                        e.region,
                        stress.reference_volume_m3,
                        stress.volume_ratio,
                        stress.stress.pressure_pa,
                        stress.stress.von_mises_pa,
                        stress.stress.max_shear_pa,
                        center[0],
                        center[1],
                        center[2]
                    )?;
                }
            }
            continue;
        }
        let external_cavities = if follower_pressure.is_some() {
            install_pressure_caps(
                &mut original,
                rings,
                sectors,
                segments,
                cap_anchor,
                balanced_caps,
            )?
        } else {
            Vec::new()
        };
        let pairs: Vec<_> = (1..=segments)
            .map(|row| {
                [
                    row * rings * sectors + sectors / 4,
                    row * rings * sectors + 3 * sectors / 4,
                ]
            })
            .collect();
        let gaps: Vec<_> = pairs
            .iter()
            .map(|&nodes| {
                let distance =
                    (original.positions()[nodes[0]][1] - original.positions()[nodes[1]][1]).abs();
                TissueGap {
                    nodes,
                    minimum_distance_m: 0.1 * distance,
                    activation_gap_m: 0.95 * 0.9 * distance,
                    stiffness_n_m: 10.,
                }
            })
            .collect();
        let loads = if follower_pressure.is_some() {
            vec![[0.; 3]; original.positions().len()]
        } else if let Some(pressure) = reference_pressure {
            let outer_faces: Vec<_> = original
                .surface()
                .into_iter()
                .filter(|face| {
                    face.iter()
                        .all(|i| i % (rings * sectors) >= (rings - 1) * sectors)
                })
                .collect();
            eprintln!(
                "{name} reference_dead_pressure_pa={pressure} outer_faces={}",
                outer_faces.len()
            );
            if reference_caps {
                // Freeze exactly the averaged closed-hull follower load at rest.
                // Keeping this dead-load law fixed isolates cap inclusion.
                let mut closed = original.clone();
                let indices =
                    install_pressure_caps(&mut closed, rings, sectors, segments, 0, true)?;
                let mut forces = vec![[0.; 3]; original.positions().len()];
                for index in &indices {
                    let part = reference_pressure_forces(
                        original.rest_positions(),
                        &closed.cavities()[*index].faces,
                        pressure / indices.len() as f64,
                    );
                    for (force, part) in forces.iter_mut().zip(part) {
                        for k in 0..3 {
                            force[k] += part[k];
                        }
                    }
                }
                eprintln!("{name} reference_caps=averaged");
                forces
            } else {
                reference_pressure_forces(original.rest_positions(), &outer_faces, pressure)
            }
        } else {
            let mut forces = vec![[0.; 3]; original.positions().len()];
            let force = if name == "urethra" { 0.002 } else { 0.01 };
            for &[a, b] in &pairs {
                forces[a] = [0., -force, 0.];
                forces[b] = [0., force, 0.];
            }
            forces
        };
        let mut loaded_contact: Option<Body> = None;
        for stage in ["rest", "compressed", "barrier", "released"] {
            let mut body = if stage == "released" {
                loaded_contact
                    .take()
                    .ok_or("missing loaded contact state")?
            } else {
                original.clone()
            };
            if stage == "released" {
                for step in 1..=4 {
                    let factor = 1. - step as f64 / 4.;
                    for &index in &external_cavities {
                        body.set_gauge_pressure(
                            index,
                            -follower_pressure.unwrap() * factor / external_cavities.len() as f64,
                        )?;
                    }
                    for (node, load) in loads.iter().enumerate() {
                        body.set_force(node, load.map(|f| f * factor))?;
                    }
                    let mut solved = false;
                    for _ in 0..8 {
                        let unload = body.equilibrate_lbfgs_observed(100000, 1e-7, |i, e, r| {
                            if trace && i % 100 == 0 {
                                eprintln!("lbfgs_trace,{name},unload-{step},{i},{e:.12e},{r:.12e}");
                            }
                        })?;
                        if unload.converged {
                            solved = true;
                            break;
                        }
                        eprintln!(
                            "{name} factor={factor} iterations={} residual={:.6e}",
                            unload.iterations, unload.residual_n
                        );
                        if unload.iterations < 100000 {
                            break;
                        }
                    }
                    if !solved {
                        return Err(format!("{name} unload factor {factor} failed").into());
                    }
                }
            }
            if stage == "compressed" || stage == "barrier" {
                for &index in &external_cavities {
                    body.set_gauge_pressure(
                        index,
                        -follower_pressure.unwrap() / external_cavities.len() as f64,
                    )?;
                }
                for (node, load) in loads.iter().enumerate() {
                    body.set_force(node, *load)?;
                }
            }
            let plain = body.clone();
            if stage == "barrier" {
                if !surface_only {
                    body.add_tissue_gaps(&gaps)?;
                }
                if surface_mode {
                    body.set_surface_contacts(vec![TissueSurfaceContact {
                        faces: body.surface(),
                        minimum_distance_m: 1e-6,
                        activation_gap_m: 5e-5,
                        pair_stiffness_n_m: 1.,
                    }])?;
                }
            }
            let report = body.equilibrate_lbfgs_observed(100000, 1e-7, |i, e, r| {
                if trace && i % 100 == 0 {
                    eprintln!("lbfgs_trace,{name},{stage},{i},{e:.12e},{r:.12e}");
                }
            })?;
            if trace {
                eprintln!(
                    "lbfgs_final,{name},{stage},{},{:.12e}",
                    report.iterations, report.residual_n
                );
            }
            if !report.converged {
                return Err(format!("{name} {stage} did not converge: iterations={} residual_n={:.12e} min_j={:.12e}", report.iterations, report.residual_n, report.min_j).into());
            }
            let tip = body
                .positions()
                .iter()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            if stage == "barrier" {
                loaded_contact = Some(body.clone());
            }
            if let Some(distance) = body.minimum_surface_contact_distance()? {
                eprintln!("{name} {stage} minimum_surface_distance_m={distance:.12e}");
            }
            let minimum = pairs
                .iter()
                .map(|&[a, b]| {
                    (0..3)
                        .map(|k| (body.positions()[a][k] - body.positions()[b][k]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
                .fold(f64::INFINITY, f64::min);
            let barrier_energy =
                body.evaluate(body.positions())?.0 - plain.evaluate(body.positions())?.0;
            println!(
                "{name},{sectors},{stage},{:.12e},{tip:.12e},{:.12e},{:.12e},{minimum:.12e},{barrier_energy:.12e},{segments},{contact_mode},{radial},{cap_anchor},{balanced_caps},{load_mode},{load_pressure_pa:.12e},{reference_caps}",
                cavity_volume(&body),
                report.min_j,
                report.residual_n
            );
            let mut file = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/{name}-{stage}.obj"
            ))?);
            for p in body.positions() {
                writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
            }
            for face in body.surface() {
                writeln!(file, "f {} {} {}", face[0] + 1, face[1] + 1, face[2] + 1)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_pressure_integrates_area_and_is_invariant_to_patch_subdivision() {
        let points = [[0., 0., 0.], [2., 0., 0.], [0., 3., 0.], [1., 1.5, 0.]];
        let whole = reference_pressure_forces(&points, &[[0, 1, 2]], 5.);
        let split = reference_pressure_forces(&points, &[[0, 1, 3], [0, 3, 2]], 5.);
        for forces in [&whole, &split] {
            let sum: [f64; 3] = std::array::from_fn(|k| forces.iter().map(|f| f[k]).sum());
            assert_eq!(sum, [0., 0., -15.]);
            // First moment fixes the resultant at the area centroid.
            let moment: [f64; 3] =
                std::array::from_fn(|k| forces.iter().zip(points).map(|(f, p)| p[k] * f[2]).sum());
            assert_eq!(moment, [-10., -15., 0.]);
        }
    }
}

#[cfg(test)]
mod cap_tests {
    use super::*;
    #[test]
    fn frozen_closed_pressure_matches_follower_force_at_reference() {
        let material = Material::from_young_poisson(8000., 0.3).unwrap();
        let profiles = vec![std::array::from_fn(|_| material.clone()); 3];
        let mut body = UrogenitalWallGeometry {
            radii_m: [0.008, 0.009, 0.010, 0.011, 0.012],
            length_m: 0.03,
            sectors: 16,
            segments: 3,
        }
        .axial_wall_refined([1., 0.3], &profiles, 1)
        .unwrap();
        let (_, baseline) = body.evaluate(body.positions()).unwrap();
        let indices = install_pressure_caps(&mut body, 5, 16, 3, 0, true).unwrap();
        let mut frozen = vec![[0.; 3]; body.positions().len()];
        for &index in &indices {
            let part = reference_pressure_forces(
                body.rest_positions(),
                &body.cavities()[index].faces,
                20. / indices.len() as f64,
            );
            for (force, part) in frozen.iter_mut().zip(part) {
                for k in 0..3 {
                    force[k] += part[k];
                }
            }
            body.set_gauge_pressure(index, -20. / indices.len() as f64)
                .unwrap();
        }
        let (_, gradient) = body.evaluate(body.positions()).unwrap();
        for ((force, gradient), baseline) in frozen.iter().zip(gradient).zip(baseline) {
            for k in 0..3 {
                assert!((force[k] + gradient[k] - baseline[k]).abs() < 1e-12);
            }
        }
        for k in 0..3 {
            assert!(frozen.iter().map(|f| f[k]).sum::<f64>().abs() < 1e-12);
        }
    }
    #[test]
    fn planar_cap_fan_choice_keeps_volume_but_changes_nodal_pressure_loads() {
        let material = Material::from_young_poisson(8000., 0.3).unwrap();
        let profiles = vec![std::array::from_fn(|_| material.clone()); 3];
        let original = UrogenitalWallGeometry {
            radii_m: [0.008, 0.009, 0.010, 0.011, 0.012],
            length_m: 0.03,
            sectors: 16,
            segments: 3,
        }
        .axial_wall([1., 0.3], &profiles)
        .unwrap();
        let make = |anchor: usize| {
            let mut body = original.clone();
            let mut faces: Vec<_> = body
                .surface()
                .into_iter()
                .filter(|f| f.iter().all(|i| i % 80 >= 64))
                .collect();
            for k in 1..15 {
                let index = |k| (anchor + k) % 16;
                faces.push([64 + anchor, 64 + index(k + 1), 64 + index(k)]);
                faces.push([304 + anchor, 304 + index(k), 304 + index(k + 1)]);
            }
            body.add_gauge_pressure_cavity(Cavity {
                faces,
                pressure_pa: -20.,
            })
            .unwrap();
            body.evaluate(body.positions()).unwrap()
        };
        let (a, ga) = make(0);
        let (b, gb) = make(8);
        assert!((a - b).abs() < 1e-14);
        let difference: f64 = ga
            .iter()
            .zip(gb)
            .flat_map(|(a, b)| std::array::from_fn::<_, 3, _>(|k| (a[k] - b[k]).abs()))
            .sum();
        assert!(difference > 1e-5);
    }
}

#[cfg(test)]
mod balanced_tests {
    use super::*;
    #[test]
    fn balanced_caps_preserve_pressure_work_and_are_anchor_invariant_on_deformed_ring() {
        let material = Material::from_young_poisson(8000., 0.3).unwrap();
        let geometry = UrogenitalWallGeometry {
            radii_m: [0.008, 0.009, 0.010, 0.011, 0.012],
            length_m: 0.03,
            sectors: 16,
            segments: 3,
        };
        let reference = geometry
            .axial_wall(
                [1., 0.3],
                &vec![std::array::from_fn(|_| material.clone()); 3],
            )
            .unwrap();
        let make = |anchor| {
            let mut body = reference.clone();
            let indices = install_pressure_caps(&mut body, 5, 16, 3, anchor, true).unwrap();
            assert_eq!(indices.len(), 16);
            for index in indices {
                body.set_gauge_pressure(index, -20. / 16.).unwrap();
            }
            body
        };
        let a = make(0);
        let b = make(8);
        let mut x = reference.positions().to_vec();
        x[304][2] += 0.0001;
        let (ea, ga) = a.evaluate(&x).unwrap();
        let (eb, gb) = b.evaluate(&x).unwrap();
        assert!((ea - eb).abs() < 1e-13);
        for (a, b) in ga.iter().zip(gb) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 1e-13);
            }
        }
        for k in 0..3 {
            let mut plus = x.clone();
            let mut minus = x.clone();
            let h = 1e-7;
            plus[304][k] += h;
            minus[304][k] -= h;
            let derivative =
                (a.evaluate(&plus).unwrap().0 - a.evaluate(&minus).unwrap().0) / (2. * h);
            assert!((derivative - ga[304][k]).abs() < 1e-7);
        }
        let energy = a.evaluate(reference.positions()).unwrap().0
            - reference.evaluate(reference.positions()).unwrap().0;
        let expected =
            20. * 16. * 0.5 * (std::f64::consts::TAU / 16.).sin() * 0.012 * 0.012 * 0.3 * 0.03;
        assert!((energy - expected).abs() < 1e-14);
    }
}
