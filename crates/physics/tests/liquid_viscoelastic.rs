use physics::liquid::*;
const I: Conformation = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn law() -> MaxwellFluid {
    MaxwellFluid {
        modulus: 2.,
        relaxation_time: 0.1,
    }
}
fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() < tol, "{a} != {b}");
}
fn fluid() -> Liquid {
    let particles = [
        [-0.02, -0.02, -0.02],
        [0.02, -0.02, -0.02],
        [-0.02, 0.02, -0.02],
        [-0.02, -0.02, 0.02],
        [0.02, 0.02, 0.02],
    ]
    .into_iter()
    .map(|position| Particle {
        position,
        velocity: [0.; 3],
        mass: 0.001,
        material: 0,
    })
    .collect();
    let mut f = Liquid::new(
        particles,
        vec![Material {
            rest_density: 1000.,
            sound_speed: 0.1,
            viscosity: 0.,
        }],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    f.set_maxwell_fluid(0, Some(law())).unwrap();
    f
}
#[test]
fn exact_quiescent_relaxation_and_free_energy_to_heat() {
    let c = [[4., 0., 0.], [0., 0.5, 0.], [0., 0., 2.]];
    let (next, q) = law().advance_conformation(c, [[0.; 3]; 3], 0.07).unwrap();
    for a in 0..3 {
        for b in 0..3 {
            close(
                next[a][b],
                I[a][b] + (-0.7_f64).exp() * (c[a][b] - I[a][b]),
                1e-13,
            );
        }
    }
    close(
        law().energy_density(c).unwrap(),
        law().energy_density(next).unwrap() + q,
        1e-13,
    );
}
#[test]
fn rotation_is_objective_and_does_not_create_polymer_heat() {
    let l = [[0., -3., 0.], [3., 0., 0.], [0., 0., 0.]];
    let (next, q) = law().advance_conformation(I, l, 0.4).unwrap();
    for a in 0..3 {
        for b in 0..3 {
            close(next[a][b], I[a][b], 1e-13);
        }
    }
    close(q, 0., 1e-13);
    let mut f = fluid();
    // An initially anisotropic tensor rotates while its eigenvalues relax.
    let c = [[4., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    f.set_conformations(vec![c; 5]).unwrap();
    let (rotated, _) = law().advance_conformation(c, l, 0.03).unwrap();
    let angle = 0.09_f64;
    let amp = 3. * (-0.3_f64).exp();
    close(rotated[0][1], amp * angle.cos() * angle.sin(), 1e-12);
}
#[test]
fn shear_matches_analytic_oldroyd_b_with_second_order_time_refinement() {
    let error = |n: usize| {
        let dt = 0.1 / n as f64;
        let mut c = I;
        for _ in 0..n {
            c = law()
                .advance_conformation(c, [[0., 4., 0.], [0., 0., 0.], [0., 0., 0.]], dt)
                .unwrap()
                .0;
        }
        let xy = 0.4 * (1. - (-1_f64).exp());
        let xx = 1. + 0.32 * (1. - 2. * (-1_f64).exp());
        (c[0][1] - xy).abs() + (c[0][0] - xx).abs()
    };
    let a = error(16);
    let b = error(32);
    let c = error(64);
    assert!(a / b > 3.8 && b / c > 3.8, "{a} {b} {c}");
}
#[test]
fn coupled_stress_accelerates_particles_and_conserves_linear_and_angular_momentum() {
    let mut f = fluid();
    f.set_conformations(vec![[[2., 0.3, 0.], [0.3, 1., 0.], [0., 0., 1.]]; 5])
        .unwrap();
    f.step(0.0001, None).unwrap();
    let mut momentum = [0.; 3];
    let mut angular = [0.; 3];
    for p in f.particles() {
        for k in 0..3 {
            momentum[k] += p.mass * p.velocity[k];
            angular[k] += p.mass
                * (p.position[(k + 1) % 3] * p.velocity[(k + 2) % 3]
                    - p.position[(k + 2) % 3] * p.velocity[(k + 1) % 3]);
        }
    }
    assert!(
        f.particles()
            .iter()
            .any(|p| p.velocity.iter().any(|v| v.abs() > 1e-7))
    );
    for v in momentum.into_iter().chain(angular) {
        close(v, 0., 1e-14);
    }
}
#[test]
fn coupled_mechanical_energy_error_decreases_with_time_step() {
    let energy = |f: &Liquid| {
        f.polymer_energy().unwrap()
            + f.polymer_relaxation_heat()
            + f.particles()
                .iter()
                .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
                .sum::<f64>()
    };
    let error = |n: usize| {
        let mut f = fluid();
        f.set_conformations(vec![[[2., 0.2, 0.], [0.2, 1., 0.], [0., 0., 1.]]; 5])
            .unwrap();
        let start = energy(&f);
        for _ in 0..n {
            f.step(0.003 / n as f64, None).unwrap();
        }
        (energy(&f) - start).abs() / start
    };
    let a = error(8);
    let b = error(16);
    let c = error(32);
    eprintln!("polymer coupled energy relative error: {a} {b} {c}");
    assert!(a / b > 1.7 && b / c > 1.7 && c < 0.001, "{a} {b} {c}");
}
#[test]
fn isolated_particle_relaxation_heats_transport_without_motion() {
    let mut f = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 0.01,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    f.configure_transport(
        vec![LiquidField {
            temperature: 300.,
            concentration: 0.,
        }],
        vec![TransportMaterial {
            conductivity: 0.,
            diffusivity: 0.,
            specific_heat: 2.,
            mixing_group: 0,
        }],
    )
    .unwrap();
    f.set_maxwell_fluid(0, Some(law())).unwrap();
    f.set_conformations(vec![[[4., 0., 0.], [0., 1., 0.], [0., 0., 1.]]])
        .unwrap();
    let initial = f.polymer_energy().unwrap() + f.transport_totals().unwrap().unwrap().0;
    f.step(0.01, None).unwrap();
    close(
        initial,
        f.polymer_energy().unwrap() + f.transport_totals().unwrap().unwrap().0,
        1e-12,
    );
    assert!(f.fields().unwrap()[0].temperature > 300.);
    assert_eq!(f.particles()[0].velocity, [0.; 3]);
}
#[test]
fn split_and_merge_preserve_memory_and_account_for_mixing_energy() {
    let mut f = fluid();
    let c = [[4., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    f.set_conformations(vec![c; 5]).unwrap();
    let before = f.polymer_energy().unwrap();
    f.split_droplet(
        0,
        DropletSplit {
            children: 3,
            axis: [0., 1., 0.],
            position_radius: 0.001,
            surface_tension: 0.,
            available_energy: 0.,
        },
    )
    .unwrap();
    close(before, f.polymer_energy().unwrap(), 1e-18);
    assert!(f.conformations().iter().all(|value| *value == c));
    let mut tensors = f.conformations().to_vec();
    tensors[0] = I;
    f.set_conformations(tensors).unwrap();
    let before = f.polymer_energy().unwrap();
    let report = f.merge_droplets(&[0, 1], 0.).unwrap();
    close(
        before,
        f.polymer_energy().unwrap() + report.released_polymer_energy,
        1e-18,
    );
    assert!(report.released_polymer_energy > 0.);
    let before = f.polymer_energy().unwrap();
    let report = f.exchange_particles(&[0], &[]).unwrap();
    close(
        before,
        f.polymer_energy().unwrap() + report.removed.polymer_energy,
        1e-18,
    );
}
#[test]
fn invalid_memory_and_late_substep_failure_are_atomic() {
    let mut f = fluid();
    let original = f.clone();
    assert!(
        f.set_conformations(vec![[[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]; 5])
            .is_err()
    );
    assert_eq!(f, original);
    let mut f = Liquid::new(
        original.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    f.set_maxwell_fluid(0, Some(law())).unwrap();
    f.set_conformations(vec![[[4., 0., 0.], [0., 1., 0.], [0., 0., 1.]]; 5])
        .unwrap();
    let original = f.clone();
    assert!(f.step(1., None).is_err());
    assert_eq!(f, original);
}

#[test]
fn thin_filament_resolves_longitudinal_stress_without_spurious_rotation_strain() {
    let create = |rotate: bool| {
        let mut f = Liquid::new(
            [-0.02, 0.02]
                .into_iter()
                .map(|x| Particle {
                    position: [x, 0., 0.],
                    velocity: if rotate { [0., 3. * x, 0.] } else { [0.; 3] },
                    mass: 0.001,
                    material: 0,
                })
                .collect(),
            vec![Material {
                sound_speed: 0.1,
                viscosity: 0.,
                ..Material::WATER
            }],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
        )
        .unwrap();
        f.set_maxwell_fluid(0, Some(law())).unwrap();
        f
    };
    let mut f = create(false);
    f.set_conformations(vec![[[4., 0.3, 0.], [0.3, 1., 0.], [0., 0., 1.]]; 2])
        .unwrap();
    f.step(0.001, None).unwrap();
    assert!(f.particles()[0].velocity[0] > 0. && f.particles()[1].velocity[0] < 0.);
    close(f.particles()[0].velocity[1], 0., 1e-15);
    close(
        f.particles()[0].velocity[0] + f.particles()[1].velocity[0],
        0.,
        1e-15,
    );
    let mut f = create(true);
    f.step(0.001, None).unwrap();
    for c in f.conformations() {
        for a in 0..3 {
            for b in 0..3 {
                close(c[a][b], I[a][b], 1e-12);
            }
        }
    }
    close(f.polymer_relaxation_heat(), 0., 1e-15);
}

#[test]
fn planar_cloud_rigid_rotation_preserves_relaxed_conformation() {
    let points = [
        [-0.02, 0., -0.02],
        [0.02, 0., -0.02],
        [-0.02, 0., 0.02],
        [0.02, 0., 0.02],
    ];
    let mut f = Liquid::new(
        points
            .into_iter()
            .map(|p| Particle {
                position: p,
                velocity: [0., 2. * p[0] - 3. * p[2], 0.],
                mass: 0.001,
                material: 0,
            })
            .collect(),
        vec![Material {
            sound_speed: 0.1,
            viscosity: 0.,
            ..Material::WATER
        }],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    f.set_maxwell_fluid(0, Some(law())).unwrap();
    f.step(0.001, None).unwrap();
    for c in f.conformations() {
        for a in 0..3 {
            for b in 0..3 {
                close(c[a][b], I[a][b], 1e-12);
            }
        }
    }
}
