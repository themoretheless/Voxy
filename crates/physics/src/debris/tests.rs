use super::*;
use crate::plasticity::mesh::convex_hull;

/// Unit cube piece of 1 kg centred at `center`.
fn cube(center: Vec3, velocity: Vec3, spin: Vec3) -> DebrisPiece {
    let mut points = Vec::new();
    for sx in [-0.5, 0.5] {
        for sy in [-0.5, 0.5] {
            for sz in [-0.5, 0.5] {
                points.push([center[0] + sx, center[1] + sy, center[2] + sz]);
            }
        }
    }
    let (hull, hull_faces) = convex_hull(&points).unwrap();
    let inertia = 1. / 6.;
    DebrisPiece {
        nodes: Vec::new(),
        hull,
        hull_faces,
        mass_kg: 1.,
        inertia_kg_m2: [[inertia, 0., 0.], [0., inertia, 0.], [0., 0., inertia]],
        center_m: center,
        velocity_m_s: velocity,
        angular_velocity_rad_s: spin,
        kinetic_j: 0.5 * dot(velocity, velocity) + 0.5 * inertia * dot(spin, spin),
        deformation_kinetic_j: 0.,
    }
}

fn ground(friction: f64) -> DebrisGround {
    DebrisGround {
        normal: [0., 1., 0.],
        offset_m: 0.,
        friction,
    }
}

fn audit(ledger: &DebrisStepReport) -> f64 {
    ledger.kinetic_j + ledger.potential_j + ledger.dissipated_j
        - ledger.correction_j
        - ledger.integration_j
}

#[test]
fn free_fall_is_exact_and_rotation_stays_orthonormal() {
    let mut bodies =
        vec![DebrisBody::new(cube([0., 10., 0.], [1., 0., 0.], [0., 3., 1.])).unwrap()];
    let mut ledger = DebrisStepReport::default();
    let gravity = [0., -9.81, 0.];
    let dt = 1e-3;
    let initial = bodies[0].kinetic_j() + 9.81 * 10.;
    for _ in 0..500 {
        step_debris(&mut bodies, dt, gravity, &ground(0.5), &mut ledger).unwrap();
    }
    let t = 500. * dt;
    let expected_y = 10. - 0.5 * 9.81 * t * t;
    assert!((bodies[0].position_m[1] - expected_y).abs() < 1e-9);
    assert!((bodies[0].position_m[0] - 0.5).abs() < 1e-9);
    assert_eq!(ledger.contacts, 0);
    assert_eq!(ledger.dissipated_j, 0.);
    assert!(
        ledger.integration_j.abs() < 1e-9 * initial,
        "{}",
        ledger.integration_j
    );
    assert!((audit(&ledger) - initial).abs() < 1e-9 * initial);
    let r = bodies[0].rotation;
    let product = mat_mul(r, transpose(r));
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1. } else { 0. };
            assert!((product[i][j] - expected).abs() < 1e-12);
        }
    }
    // Spin magnitude is conserved for a sphere-like inertia with no torque.
    assert!((norm(bodies[0].angular_velocity_rad_s) - 10_f64.sqrt()).abs() < 1e-12);
}

#[test]
fn landing_rests_on_the_plane_and_the_ledger_closes() {
    let gravity = [0., -9.81, 0.];
    let run = |dt: f64| {
        let mut bodies =
            vec![DebrisBody::new(cube([0., 3., 0.], [0., 0., 0.], [0., 0., 0.])).unwrap()];
        let mut ledger = DebrisStepReport::default();
        let initial = bodies[0].kinetic_j() + 9.81 * 3.;
        let steps = (2. / dt).round() as usize;
        for _ in 0..steps {
            step_debris(&mut bodies, dt, gravity, &ground(0.5), &mut ledger).unwrap();
            let total = audit(&ledger);
            assert!(
                (total - initial).abs() < 1e-9 * initial,
                "ledger drift {}",
                total - initial
            );
        }
        assert!(
            (bodies[0].position_m[1] - 0.5).abs() < 1e-6,
            "{}",
            bodies[0].position_m[1]
        );
        assert!(norm(bodies[0].velocity_m_s) < 1e-6);
        assert!(ledger.dissipated_j > 0.9 * 9.81 * 2.5);
        assert!(ledger.contacts >= 1);
        (ledger.integration_j.abs(), initial)
    };
    let (coarse, initial) = run(1e-3);
    let (fine, _) = run(5e-4);
    // The residual is small and first order in the step.
    assert!(coarse < 5e-3 * initial, "{coarse}");
    assert!(fine < 0.6 * coarse, "{fine} vs {coarse}");
}

#[test]
fn inclined_plane_sticks_below_and_slides_above_the_friction_angle() {
    let angle: f64 = 0.3;
    let normal = [-angle.sin(), angle.cos(), 0.];
    let slide = |friction: f64| {
        // Start resting on the plane: centre at half a cube height along the normal.
        let start = scale(normal, 0.5 + 1e-9);
        let mut body = DebrisBody::new(cube(start, [0.; 3], [0.; 3])).unwrap();
        // Align the cube with the slope so a whole face rests on it.
        let (s, c) = angle.sin_cos();
        body.rotation = [[c, -s, 0.], [s, c, 0.], [0., 0., 1.]];
        let mut bodies = vec![body];
        let ground = DebrisGround {
            normal,
            offset_m: 0.,
            friction,
        };
        let mut ledger = DebrisStepReport::default();
        for _ in 0..1000 {
            step_debris(&mut bodies, 1e-3, [0., -9.81, 0.], &ground, &mut ledger).unwrap();
        }
        norm(sub(bodies[0].position_m, start))
    };
    let sticky = slide(angle.tan() * 1.5);
    let slippery = slide(angle.tan() * 0.5);
    assert!(sticky < 1e-6, "stuck block moved {sticky}");
    // Half the gravity component along the slope remains: a = g sin θ (1 - μ/tan θ).
    let expected = 0.5 * 9.81 * angle.sin() * 0.5;
    assert!(
        (slippery - expected).abs() < 0.03 * expected,
        "{slippery} vs {expected}"
    );
}

#[test]
fn invalid_input_is_rejected() {
    let mut bodies = vec![DebrisBody::new(cube([0., 1., 0.], [0.; 3], [0.; 3])).unwrap()];
    let mut ledger = DebrisStepReport::default();
    assert!(step_debris(&mut bodies, -1., [0.; 3], &ground(0.5), &mut ledger).is_err());
    assert!(step_debris(&mut bodies, f64::NAN, [0.; 3], &ground(0.5), &mut ledger).is_err());
    let bad = DebrisGround {
        normal: [0.; 3],
        offset_m: 0.,
        friction: 0.5,
    };
    assert!(step_debris(&mut bodies, 0.1, [0.; 3], &bad, &mut ledger).is_err());
    let mut singular = cube([0.; 3], [0.; 3], [0.; 3]);
    singular.inertia_kg_m2 = [[0.; 3]; 3];
    assert!(DebrisBody::new(singular).is_err());
}
