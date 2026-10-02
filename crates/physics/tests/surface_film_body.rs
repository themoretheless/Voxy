use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
use physics::surface_film::{Material, SurfaceFilm};
#[test]
fn finite_body_contact_preserves_normal_motion_and_converts_kinetic_loss_into_heat() {
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 2.0,
            ..Material::default()
        },
    )
    .unwrap();
    film.deposit(0, 0.1).unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [3.0, 1.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let before = body;
    let r = film
        .relax_sliding_body(0.1, 0.2, 0.1, [0.0, 1.0, 0.0], &mut body)
        .unwrap();
    assert!((body.mechanics.velocity[0] - 3.0 * (-0.05_f64).exp()).abs() < 1e-14);
    assert_eq!(body.mechanics.velocity[1], 1.0);
    assert!((r.substrate_impulse[0] + 2.0 * body.mechanics.velocity[0] - 6.0).abs() < 1e-14);
    let ke = |b: ThermalTranslatingBody| b.mechanics.velocity.iter().map(|v| v * v).sum::<f64>();
    assert!(
        (ke(body) + body.thermal_energy().unwrap() - ke(before) - before.thermal_energy().unwrap())
            .abs()
            < 1e-12
    );
    let snapshot = body;
    assert!(
        film.relax_sliding_body(0.1, 0.0, 0.1, [0.0, 1.0, 0.0], &mut body)
            .is_err()
    );
    assert_eq!(body.temperature, snapshot.temperature);
}
#[test]
fn rectangular_patch_clips_partial_triangle_and_requires_wet_gap() {
    use physics::surface_film::SlidingPatch;
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 2.0,
            ..Material::default()
        },
    )
    .unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.25, 0.1, 0.25],
            velocity: [3.0, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.25, 0.25],
    };
    let dry = film.relax_sliding_patch(0.1, patch, &mut body).unwrap();
    assert_eq!(dry.wetted_area, 0.0);
    assert_eq!(body.mechanics.velocity[0], 3.0);
    film.deposit(0, 0.05).unwrap();
    let wet = film.relax_sliding_patch(0.1, patch, &mut body).unwrap();
    assert!((wet.wetted_area - 0.25).abs() < 1e-14);
    assert!((body.mechanics.velocity[0] - 3.0 * (-0.25_f64).exp()).abs() < 1e-14);
    body.mechanics.position[0] = 2.0;
    assert_eq!(
        film.relax_sliding_patch(0.1, patch, &mut body)
            .unwrap()
            .wetted_area,
        0.0
    );
}
#[test]
fn coupled_patch_entrains_film_and_rolls_back_body_when_transport_rejects() {
    use physics::surface_film::SlidingPatch;
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            viscosity: 2.0,
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )
    .unwrap();
    film.deposit(0, 0.05).unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.5, 0.1, 0.5],
            velocity: [-0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.5, 0.5],
    };
    let original = body;
    let heights = film.thickness();
    assert!(film.step_sliding_patch(0.1, patch, &mut body, 0.0).is_err());
    assert_eq!(body, original);
    assert_eq!(film.thickness(), heights);
    film.step_sliding_patch(0.1, patch, &mut body, 0.001)
        .unwrap();
    assert!(film.thickness()[1] > 0.0);
    assert!((film.total_volume() - 0.05).abs() < 1e-15);
    assert!(body.mechanics.velocity[0].abs() < 0.3);
    assert!(body.temperature > 300.0);
}
#[test]
fn moving_contact_recomputes_overlap_and_preserves_body_energy() {
    use physics::surface_film::SlidingPatch;
    let mut f = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 2.0,
            ..Material::default()
        },
    )
    .unwrap();
    f.deposit(0, 0.05).unwrap();
    let mut b = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.25, 0.1, 0.25],
            velocity: [0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let old = b;
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.25, 0.25],
    };
    let r = f.advance_sliding_patch(0.1, patch, &mut b, 0.001).unwrap();
    assert_eq!(r.substeps, 100);
    assert!(b.mechanics.position[0] > old.mechanics.position[0]);
    assert!(r.mean_wetted_area < 0.25);
    assert!(b.mechanics.velocity[0] < old.mechanics.velocity[0]);
    let energy = |b: ThermalTranslatingBody| {
        b.thermal_energy().unwrap() + b.mechanics.velocity.iter().map(|v| v * v).sum::<f64>()
    };
    assert!((energy(b) - energy(old)).abs() < 1e-10);
    let snapshot = b;
    let heights = f.thickness();
    assert!(f.advance_sliding_patch(0.1, patch, &mut b, 0.0).is_err());
    assert_eq!(b, snapshot);
    assert_eq!(f.thickness(), heights);
}
#[test]
fn late_internal_heat_precision_failure_restores_entire_moving_step() {
    use physics::surface_film::SlidingPatch;
    let mut f = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 8000.0,
            ..Material::default()
        },
    )
    .unwrap();
    f.deposit(0, 0.05).unwrap();
    let mut b = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.25, 0.1, 0.25],
            velocity: [0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.25, 0.25],
    };
    let original = b;
    let volume = f.thickness();
    assert!(f.advance_sliding_patch(0.1, patch, &mut b, 0.001).is_err());
    assert_eq!(b, original);
    assert_eq!(f.thickness(), volume);
}

#[test]
fn moving_partial_overlap_converges_to_independent_continuous_drag_solution() {
    use physics::surface_film::SlidingPatch;
    // The clipped corner has area A(x)=1/4-(x-1/4)^2/2.
    // Integrating dv/dx=-mu*A/(mass*gap) gives this independent velocity law.
    let speed = |x: f64| {
        let d = x - 0.25;
        0.3 - 10.0 * (0.25 * d - d.powi(3) / 6.0)
    };
    let mut reference_x = 0.25;
    let dt = 0.1 / 10000.0;
    for _ in 0..10000 {
        let a = speed(reference_x);
        let b = speed(reference_x + dt * a / 2.0);
        let c = speed(reference_x + dt * b / 2.0);
        let d = speed(reference_x + dt * c);
        reference_x += dt * (a + 2.0 * b + 2.0 * c + d) / 6.0;
    }
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut film = SurfaceFilm::new(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            vec![[0, 1, 2]],
            Material {
                viscosity: 2.0,
                surface_tension: 0.0,
                wetting: 0.0,
                ..Material::default()
            },
        )
        .unwrap();
        film.deposit(0, 0.05).unwrap();
        let mut body = ThermalTranslatingBody {
            mechanics: TranslatingBody {
                position: [0.25, 0.1, 0.25],
                velocity: [0.3, 0.0, 0.0],
                mass: 2.0,
            },
            specific_heat: 4.0,
            temperature: 300.0,
        };
        film.advance_sliding_patch(
            0.1,
            SlidingPatch {
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                half_extents: [0.25, 0.25],
            },
            &mut body,
            step,
        )
        .unwrap();
        errors.push((body.mechanics.velocity[0] - speed(reference_x)).abs());
        assert!((body.mechanics.position[0] - reference_x).abs() < 1e-7);
    }
    for pair in errors.windows(2) {
        assert!(pair[1] < pair[0] * 0.6, "refinement errors: {errors:?}");
        assert!(pair[1] > pair[0] * 0.4, "refinement errors: {errors:?}");
    }
}
