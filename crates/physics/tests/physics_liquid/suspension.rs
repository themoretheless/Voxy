use physics::suspension::{Carrier, Particle};
#[test]
fn settling_density_buoyancy_and_exact_interval_composition() {
    let medium = Carrier {
        density_kg_m3: 1000.,
        viscosity_pa_s: 0.001,
        velocity_m_s: [0.; 3],
    };
    let mut p = Particle::new(1e-6, 2000., [0.; 3], [0.; 3]).unwrap();
    let mut split = p.clone();
    let report = p.advance(1., medium, [0., -9.81, 0.]).unwrap();
    for _ in 0..100 {
        split.advance(0.01, medium, [0., -9.81, 0.]).unwrap();
    }
    let terminal = -2. * (2000. - 1000.) * 9.81 * 1e-12 / (9. * 0.001);
    assert!((p.velocity_m_s()[1] - terminal).abs() < 1e-15);
    assert!((p.position_m()[1] - split.position_m()[1]).abs() < 1e-15);
    assert!(
        (report.carrier_impulse_n_s[1] + p.mass_kg() * p.velocity_m_s()[1] - p.mass_kg() * (-9.81))
            .abs()
            < 1e-25
    );
    let mut neutral = Particle::new(1e-6, 1000., [0.; 3], [0.; 3]).unwrap();
    neutral.advance(1., medium, [0., -9.81, 0.]).unwrap();
    assert_eq!(neutral.velocity_m_s(), [0.; 3]);
    let mut light = Particle::new(1e-6, 500., [0.; 3], [0.; 3]).unwrap();
    light.advance(1., medium, [0., -9.81, 0.]).unwrap();
    assert!(light.velocity_m_s()[1] > 0.);
}
#[test]
fn invalid_regime_rolls_back_particle_mass_and_pose() {
    let mut p = Particle::new(0.01, 2000., [0.; 3], [1., 0., 0.]).unwrap();
    assert!(
        p.advance(
            1.,
            Carrier {
                density_kg_m3: 1000.,
                viscosity_pa_s: 0.001,
                velocity_m_s: [0.; 3]
            },
            [0.; 3]
        )
        .is_err()
    );
    assert_eq!(p.position_m(), [0.; 3]);
    assert_eq!(p.velocity_m_s(), [1., 0., 0.]);
}

#[test]
fn drag_heat_matches_lost_relative_energy_and_moving_carrier_work() {
    let mut p = Particle::new(1e-5, 2000., [0.; 3], [0.01, 0., 0.]).unwrap();
    let medium = Carrier {
        density_kg_m3: 1.,
        viscosity_pa_s: 1e-5,
        velocity_m_s: [0.; 3],
    };
    let initial = 0.5 * p.mass_kg() * 0.01_f64.powi(2);
    let report = p.advance(0.01, medium, [0.; 3]).unwrap();
    let remaining = 0.5 * p.mass_kg() * p.velocity_m_s()[0].powi(2);
    assert!((report.viscous_heat_j - (initial - remaining)).abs() < initial * 1e-12);
    assert!(report.viscous_heat_j > 0.);
    let mut moved = Particle::new(1e-5, 2000., [0.; 3], [1.01, 2., 3.]).unwrap();
    let moving = Carrier {
        velocity_m_s: [1., 2., 3.],
        ..medium
    };
    let translated = moved.advance(0.01, moving, [0.; 3]).unwrap();
    assert!((translated.viscous_heat_j - report.viscous_heat_j).abs() < initial * 1e-12);
    assert!(translated.energy_defect_j.abs() < initial * 1e-10);
}

#[test]
fn finite_carrier_receives_reaction_and_heat_conservatively() {
    use physics::suspension::FiniteCarrier;
    let mut p = Particle::new(1e-5, 2000., [0.; 3], [0.01, 0., 0.]).unwrap();
    let medium = Carrier {
        density_kg_m3: 1.,
        viscosity_pa_s: 1e-5,
        velocity_m_s: [0.; 3],
    };
    let mut carrier = FiniteCarrier::new(medium, 1e-9).unwrap();
    let initial_p = p.mass_kg() * 0.01;
    let initial_k = 0.5 * p.mass_kg() * 0.01_f64.powi(2);
    let mut split_p = p.clone();
    let mut split_c = carrier.clone();
    let report = p.exchange_drag(0.01, &mut carrier).unwrap();
    assert!(carrier.velocity_m_s()[0] > 0.);
    let final_p = p.mass_kg() * p.velocity_m_s()[0] + carrier.mass_kg() * carrier.velocity_m_s()[0];
    let final_k = 0.5 * p.mass_kg() * p.velocity_m_s()[0].powi(2)
        + 0.5 * carrier.mass_kg() * carrier.velocity_m_s()[0].powi(2);
    assert!((final_p - initial_p).abs() < initial_p * 1e-12);
    assert!((final_k + carrier.deposited_heat_j() - initial_k).abs() < initial_k * 1e-12);
    assert_eq!(report.deposited_heat_j, carrier.deposited_heat_j());
    for _ in 0..100 {
        split_p.exchange_drag(0.0001, &mut split_c).unwrap();
    }
    assert!((split_p.position_m()[0] - p.position_m()[0]).abs() < 1e-15);
    assert!((split_c.deposited_heat_j() - carrier.deposited_heat_j()).abs() < initial_k * 1e-12);
}

#[test]
fn shared_carrier_cloud_is_order_independent_and_balanced() {
    use physics::suspension::{FiniteCarrier, exchange_drag_cloud};
    let mut particles = vec![
        Particle::new(1e-5, 2000., [0.; 3], [0.01, 0., 0.]).unwrap(),
        Particle::new(2e-5, 3000., [0.; 3], [-0.005, 0., 0.]).unwrap(),
    ];
    let mut carrier = FiniteCarrier::new(
        Carrier {
            density_kg_m3: 1.,
            viscosity_pa_s: 1e-5,
            velocity_m_s: [0.; 3],
        },
        1e-9,
    )
    .unwrap();
    let initial_p: f64 = particles
        .iter()
        .map(|p| p.mass_kg() * p.velocity_m_s()[0])
        .sum();
    let initial_k: f64 = particles
        .iter()
        .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s()[0].powi(2))
        .sum();
    let mut reversed = particles.iter().rev().cloned().collect::<Vec<_>>();
    let mut reversed_c = carrier.clone();
    let report = exchange_drag_cloud(&mut particles, &mut carrier, 0.01).unwrap();
    exchange_drag_cloud(&mut reversed, &mut reversed_c, 0.01).unwrap();
    let final_p: f64 = particles
        .iter()
        .map(|p| p.mass_kg() * p.velocity_m_s()[0])
        .sum::<f64>()
        + carrier.mass_kg() * carrier.velocity_m_s()[0];
    let final_k: f64 = particles
        .iter()
        .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s()[0].powi(2))
        .sum::<f64>()
        + 0.5 * carrier.mass_kg() * carrier.velocity_m_s()[0].powi(2);
    assert!((final_p - initial_p).abs() < initial_p.abs() * 1e-12);
    assert!(
        (final_k + report.viscous_heat_j + report.numerical_loss_j - initial_k).abs()
            < initial_k * 1e-12
    );
    for (a, b) in particles.iter().zip(reversed.iter().rev()) {
        assert!((a.velocity_m_s()[0] - b.velocity_m_s()[0]).abs() < 1e-15);
    }
    assert_eq!(carrier.velocity_m_s(), reversed_c.velocity_m_s());
}

#[test]
fn cloud_time_refinement_reduces_numerical_loss_toward_exact_pair_solution() {
    use physics::suspension::{FiniteCarrier, exchange_drag_cloud};
    let original = Particle::new(1e-5, 2000., [0.; 3], [0.01, 0., 0.]).unwrap();
    let fluid = FiniteCarrier::new(
        Carrier {
            density_kg_m3: 1.,
            viscosity_pa_s: 1e-5,
            velocity_m_s: [0.; 3],
        },
        1e-9,
    )
    .unwrap();
    let mass = original.mass_kg();
    let total = mass + fluid.mass_kg();
    let rate = 6. * std::f64::consts::PI * 1e-5 * 1e-5 * (1. / mass + 1. / fluid.mass_kg());
    let exact = 0.01 * mass / total + 0.01 * fluid.mass_kg() / total * (-rate * 0.01).exp();
    let mut errors = Vec::new();
    let mut losses = Vec::new();
    for steps in [4, 16, 64] {
        let mut particles = [original.clone()];
        let mut carrier = fluid.clone();
        for _ in 0..steps {
            exchange_drag_cloud(&mut particles, &mut carrier, 0.01 / f64::from(steps)).unwrap();
        }
        errors.push((particles[0].velocity_m_s()[0] - exact).abs());
        losses.push(carrier.numerical_loss_j());
    }
    assert!(errors[1] < errors[0] && errors[2] < errors[1]);
    assert!(losses[1] < losses[0] && losses[2] < losses[1]);
}
#[test]
fn gravity_cloud_reaction_balances_total_weight_and_body_work() {
    use physics::suspension::{FiniteCarrier, exchange_gravity_cloud};
    let medium = Carrier {
        density_kg_m3: 1000.,
        viscosity_pa_s: 0.001,
        velocity_m_s: [0.; 3],
    };
    let mut carrier = FiniteCarrier::new(medium, 1e-13).unwrap();
    let mut cloud = vec![Particle::new(1e-6, 2000., [0.; 3], [0.; 3]).unwrap()];
    let dt = 1e-4;
    let g = -9.81;
    let mp = cloud[0].mass_kg();
    let mf = carrier.mass_kg();
    let report = exchange_gravity_cloud(&mut cloud, &mut carrier, dt, [0., g, 0.]).unwrap();
    let v = cloud[0].velocity_m_s()[1];
    let u = carrier.velocity_m_s()[1];
    assert!((mp * v + mf * u - (mp + mf) * g * dt).abs() < 1e-25);
    let beta_dt = 6. * std::f64::consts::PI * 0.001 * 1e-6 * dt;
    // Independent implicit equations with hydrostatic buoyancy on each phase.
    let ap = 0.5 * g;
    let af = (1. + 0.5 * mp / mf) * g;
    assert!((mp * v - dt * mp * ap + beta_dt * (v - u)).abs() < 1e-25);
    assert!((mf * u - dt * mf * af - beta_dt * (v - u)).abs() < 1e-25);
    let work = dt * (mp * ap * v + mf * af * u);
    let kinetic = 0.5 * (mp * v * v + mf * u * u);
    assert!((report.body_force_work_j - work).abs() < 1e-28);
    assert!((kinetic + report.viscous_heat_j + report.numerical_loss_j - work).abs() < 1e-28);
    let old_position = cloud[0].position_m();
    let old_velocity = cloud[0].velocity_m_s();
    let old_carrier_velocity = carrier.velocity_m_s();
    let old_heat = carrier.deposited_heat_j();
    assert!(exchange_gravity_cloud(&mut cloud, &mut carrier, dt, [0., f64::NAN, 0.]).is_err());
    assert_eq!(cloud[0].position_m(), old_position);
    assert_eq!(cloud[0].velocity_m_s(), old_velocity);
    assert_eq!(carrier.velocity_m_s(), old_carrier_velocity);
    assert_eq!(carrier.deposited_heat_j(), old_heat);
}
#[test]
fn nearly_comoving_cloud_balances_relative_energy_with_stored_velocity_roundoff() {
    use physics::suspension::{FiniteCarrier, exchange_drag_cloud};
    let base = 3e-8;
    let medium = Carrier {
        density_kg_m3: 1000.,
        viscosity_pa_s: 0.001,
        velocity_m_s: [base, 0., 0.],
    };
    let mut carrier = FiniteCarrier::new(medium, 1e-8).unwrap();
    let mut cloud = vec![Particle::new(1e-4, 2000., [0.; 3], [base + 1e-10, 0., 0.]).unwrap()];
    let old_v = cloud[0].velocity_m_s()[0];
    let mp = cloud[0].mass_kg();
    let mf = carrier.mass_kg();
    let report = exchange_drag_cloud(&mut cloud, &mut carrier, 0.1).unwrap();
    let v = cloud[0].velocity_m_s()[0];
    let u = carrier.velocity_m_s()[0];
    let momentum_error = mf * (u - base) + mp * (v - old_v);
    let bound = 2. * f64::EPSILON * (mf * (u.abs() + base.abs()) + mp * (v.abs() + old_v.abs()));
    assert!(momentum_error.abs() <= bound);
    let old_relative_k = 0.5 * mp * (old_v - base).powi(2);
    let relative_k = 0.5 * (mp * (v - base).powi(2) + mf * (u - base).powi(2));
    assert!(
        (relative_k - old_relative_k + report.viscous_heat_j + report.numerical_loss_j).abs()
            < 1e-10 * old_relative_k
    );
    assert!(report.viscous_heat_j > 0. && report.numerical_loss_j > 0.);
}

#[test]
fn cloud_relaxes_to_velocity_storage_precision_without_energy_gate_failure() {
    use physics::suspension::{FiniteCarrier, exchange_drag_cloud};
    let base = 3e-8;
    let medium = Carrier {
        density_kg_m3: 1000.,
        viscosity_pa_s: 0.001,
        velocity_m_s: [base, 0., 0.],
    };
    let mut carrier = FiniteCarrier::new(medium, 1e-8).unwrap();
    let mut cloud = vec![Particle::new(1e-4, 2000., [0.; 3], [base + 1e-10, 0., 0.]).unwrap()];
    let mp = cloud[0].mass_kg();
    let mf = carrier.mass_kg();
    let initial = 0.5 * mp * (cloud[0].velocity_m_s()[0] - base).powi(2);
    let mut loss = 0.;
    for _ in 0..200 {
        // This bridge owns cumulative heat independently, just as Liquid does.
        carrier = FiniteCarrier::new(
            Carrier {
                velocity_m_s: carrier.velocity_m_s(),
                ..medium
            },
            1e-8,
        )
        .unwrap();
        let report = exchange_drag_cloud(&mut cloud, &mut carrier, 0.1).unwrap();
        loss += report.viscous_heat_j + report.numerical_loss_j;
    }
    let v = cloud[0].velocity_m_s()[0];
    let u = carrier.velocity_m_s()[0];
    let final_energy = 0.5 * (mp * (v - base).powi(2) + mf * (u - base).powi(2));
    assert!((final_energy + loss - initial).abs() < 1e-10 * initial);
    assert!((v - u).abs() <= 4. * f64::EPSILON * base);
    assert!((mf * (u - base) + mp * (v - base) - mp * 1e-10).abs() < 1e-25);
}
