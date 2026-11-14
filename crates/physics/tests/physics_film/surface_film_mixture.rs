use physics::surface_film::{FilmMixture, Material, SurfaceFilm};
fn film(volumes: [f64; 2]) -> SurfaceFilm {
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )
    .unwrap();
    for (i, v) in volumes.into_iter().enumerate() {
        film.deposit(i, v).unwrap();
    }
    film
}
fn mixture(volumes: [f64; 2]) -> FilmMixture {
    FilmMixture::new(
        film(volumes),
        vec!["water".into(), "gel".into()],
        vec![vec![0.25, 0.75], vec![1.0, 0.0]],
    )
    .unwrap()
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 2e-13 * a.abs().max(b.abs()).max(1e-10),
        "{a} != {b}"
    );
}
fn same_height_to_roundoff(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        // Canonical kilograms require a final division by density; independent
        // volume transport has a different arithmetic path. Bound only rounding.
        assert!(
            (a - b).abs() <= 8. * f64::EPSILON * a.abs().max(b.abs()),
            "{a} != {b}"
        );
    }
}
#[test]
fn donor_composition_and_component_masses_follow_analytic_drainage() {
    let mut m = mixture([0.0005, 0.0002]);
    let before = m.component_masses().unwrap();
    m.step_with_advection(0.1, [0.0; 3], &[[-1.0, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    // Shared diagonal length sqrt(2), normal speed 1/sqrt(2), area 1/2:
    // forward Euler donor drainage V0(t+dt)=(1-2dt)*V0(t).
    let donor = 0.0005 * 0.998_f64.powi(100);
    let received = 0.0005 - donor;
    near(m.film().thickness()[0] * 0.5, donor);
    near(m.fractions()[0][1], 0.75);
    near(m.fractions()[1][1], received * 0.75 / (0.0002 + received));
    for (a, b) in before.into_iter().zip(m.component_masses().unwrap()) {
        near(a, b);
    }
}
#[test]
fn dry_fill_extreme_donor_limiting_and_weighted_deposit_preserve_inventory() {
    let mut m = mixture([0.0005, 0.0]);
    assert_eq!(m.fractions()[1], vec![0.0, 0.0]);
    m.step_with_advection(0.001, [0.0; 3], &[[-1e20, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    near(m.fractions()[1][1], 0.75);
    near(m.component_masses().unwrap()[1], 0.375);
    m.deposit(1, 0.0015, &[1.0, 0.0]).unwrap();
    near(m.fractions()[1][1], 0.1875);
    near(m.film().total_mass(), 2.0);
    near(m.component_masses().unwrap()[0], 1.625);
}
#[test]
fn invalid_driving_deposits_and_schema_are_atomic() {
    assert!(
        FilmMixture::new(
            film([0.0005, 0.0]),
            vec!["x".into(), "x".into()],
            vec![vec![1.0, 0.0]; 2]
        )
        .is_err()
    );
    assert!(FilmMixture::new(film([0.0005, 0.0]), vec!["x".into()], vec![vec![0.5]; 2]).is_err());
    let mut m = mixture([0.0005, 0.0002]);
    let h = m.film().thickness();
    let masses = m.component_masses().unwrap();
    let fractions = m.fractions();
    assert!(
        m.step_with_advection(0.1, [0.0; 3], &[[f64::NAN, 0.0, 0.0]; 2], 0.001)
            .is_err()
    );
    assert!(
        m.step_with_surface_shear(0.1, [0.0; 3], &[[0.0; 3]], 0.001)
            .is_err()
    );
    assert!(m.deposit(0, 0.1, &[0.5, 0.4]).is_err());
    assert!(m.deposit(0, f64::MAX, &[1.0, 0.0]).is_err());
    assert_eq!(m.film().thickness(), h);
    assert_eq!(m.component_masses().unwrap(), masses);
    assert_eq!(m.fractions(), fractions);
}
#[test]
fn composition_transport_leaves_mechanical_solution_identical() {
    for mode in 0..3 {
        let mut plain = film([0.0005, 0.0002]);
        let mut mixed = mixture([0.0005, 0.0002]);
        let old = mixed.component_masses().unwrap();
        let gravity = [-1.0, 0.0, 0.0];
        let stress = [[-0.2, 0.0, 0.0]; 2];
        match mode {
            0 => {
                plain.step_with_max_substep(0.01, gravity, 0.001).unwrap();
                mixed.step(0.01, gravity, 0.001).unwrap();
            }
            1 => {
                plain
                    .step_with_surface_shear(0.01, gravity, &stress, 0.001)
                    .unwrap();
                mixed
                    .step_with_surface_shear(0.01, gravity, &stress, 0.001)
                    .unwrap();
            }
            _ => {
                let model = physics::surface_film::FilmRheology {
                    consistency: 0.05,
                    flow_index: 0.7,
                    yield_stress: 0.01,
                    profile_samples: 128,
                };
                plain
                    .step_with_rheology(0.01, gravity, &stress, 0.001, model)
                    .unwrap();
                mixed
                    .step_with_rheology(0.01, gravity, &stress, 0.001, model)
                    .unwrap();
            }
        }
        same_height_to_roundoff(&plain.thickness(), &mixed.film().thickness());
        for (a, b) in old.into_iter().zip(mixed.component_masses().unwrap()) {
            near(a, b);
        }
        for row in mixed.fractions() {
            assert!(row.iter().all(|f| *f >= 0.0 && *f <= 1.0));
            near(row.iter().sum(), 1.0);
        }
    }
}

#[test]
fn branched_outgoing_flux_uses_one_donor_budget_for_all_components() {
    let mut f = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2], [1, 0, 3], [2, 1, 4], [0, 2, 5]],
        Material {
            surface_tension: 0.0,
            wetting: 1e10,
            ..Material::default()
        },
    )
    .unwrap();
    f.deposit(0, 0.0005).unwrap();
    let mut m = FilmMixture::new(f, vec!["a".into(), "b".into()], vec![vec![0.2, 0.8]; 4]).unwrap();
    m.step(0.001, [0.0; 3], 0.001).unwrap();
    let h = m.film().thickness();
    assert!(h[1..].iter().all(|v| *v > 0.0));
    assert!(h.iter().all(|v| *v >= 0.0));
    near(m.film().total_volume(), 0.0005);
    near(m.component_masses().unwrap()[0], 0.1);
    near(m.component_masses().unwrap()[1], 0.4);
    for row in &m.fractions()[1..] {
        near(row[1], 0.8);
    }
}

#[test]
fn diffusion_matches_unequal_volume_pair_solution_and_preserves_volume() {
    let mut m = mixture([0.0005, 0.0002]);
    let before = m.component_masses().unwrap();
    let heights = m.film().thickness();
    let harmonic_height = 2.0 * 0.001 * 0.0004 / (0.001 + 0.0004);
    let conductance = 3.0 * 2.0 * harmonic_height;
    let decay = (-conductance * (1.0 / 0.0005 + 1.0 / 0.0002) * 0.1_f64).exp();
    let average = 0.75 * 0.0005 / 0.0007;
    m.diffuse(0.1, 2.0, 0.001).unwrap();
    near(
        m.fractions()[0][1],
        average + 0.0002 / 0.0007 * 0.75 * decay,
    );
    near(
        m.fractions()[1][1],
        average - 0.0005 / 0.0007 * 0.75 * decay,
    );
    assert_eq!(m.film().thickness(), heights);
    for (a, b) in before.into_iter().zip(m.component_masses().unwrap()) {
        near(a, b);
    }
    let mut split = mixture([0.0005, 0.0002]);
    for _ in 0..2 {
        split.diffuse(0.05, 2.0, 0.001).unwrap();
    }
    for (a, b) in m
        .fractions()
        .iter()
        .flatten()
        .zip(split.fractions().iter().flatten())
    {
        near(*a, *b);
    }
}
#[test]
fn diffusion_handles_stiff_rates_dry_edges_and_invalid_controls() {
    let mut m = mixture([0.0005, 0.0]);
    let before = m.fractions();
    m.diffuse(0.1, 1e20, 0.001).unwrap();
    assert_eq!(m.fractions(), before);
    let mut m = mixture([0.0005, 0.0002]);
    m.diffuse(0.1, 1e20, 0.001).unwrap();
    near(m.fractions()[0][1], m.fractions()[1][1]);
    let before = m.fractions();
    let masses = m.component_masses().unwrap();
    for (dt, d, step) in [
        (0.1, -1.0, 0.001),
        (0.1, f64::NAN, 0.001),
        (0.1, 1.0, 0.0),
        (0.1, 1.0, 1e-10),
        (0.2, 1.0, 0.001),
    ] {
        assert!(m.diffuse(dt, d, step).is_err());
    }
    assert_eq!(m.fractions(), before);
    assert_eq!(m.component_masses().unwrap(), masses);
}
#[test]
fn multi_edge_diffusion_refines_quadratically_to_independent_graph_solution() {
    let derivative = |y: [f64; 3]| {
        [
            12.0 * (y[1] - y[0]),
            12.0 * (y[0] - y[1]) + 6.0 * (y[2] - y[1]),
            6.0 * (y[1] - y[2]),
        ]
    };
    let add = |y: [f64; 3], k: [f64; 3], scale: f64| std::array::from_fn(|i| y[i] + scale * k[i]);
    let mut reference = [1.0, 0.0, 0.25];
    let dt = 0.1 / 10000.0;
    for _ in 0..10000 {
        let a = derivative(reference);
        let b = derivative(add(reference, a, dt / 2.0));
        let c = derivative(add(reference, b, dt / 2.0));
        let d = derivative(add(reference, c, dt));
        reference = std::array::from_fn(|i| {
            reference[i] + dt * (a[i] + 2.0 * b[i] + 2.0 * c[i] + d[i]) / 6.0
        });
    }
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut f = SurfaceFilm::new(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 2.0],
            ],
            vec![[0, 1, 2], [1, 3, 2], [2, 3, 4]],
            Material::default(),
        )
        .unwrap();
        for i in 0..3 {
            f.deposit(i, 0.0005).unwrap();
        }
        let mut m = FilmMixture::new(
            f,
            vec!["a".into(), "b".into()],
            vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.25, 0.75]],
        )
        .unwrap();
        let initial = m.component_masses().unwrap();
        m.diffuse(0.1, 2.0, step).unwrap();
        errors.push(
            m.fractions()
                .iter()
                .zip(reference)
                .map(|(row, exact)| (row[0] - exact).abs())
                .fold(0.0, f64::max),
        );
        for (a, b) in initial.into_iter().zip(m.component_masses().unwrap()) {
            near(a, b);
        }
        assert!(
            m.fractions()
                .iter()
                .all(|row| row.iter().all(|y| *y >= 0.0 && *y <= 1.0))
        );
    }
    for pair in errors.windows(2) {
        assert!(
            pair[0] / pair[1] > 3.9 && pair[0] / pair[1] < 4.1,
            "diffusion errors: {errors:?}"
        );
    }
}

#[test]
fn composition_viscosity_changes_shear_flow_and_tracks_diffusive_mixing() {
    let mut m = mixture([0.0005, 0.0002]);
    m.configure_viscosities(Some(vec![0.001, 1.0])).unwrap();
    let mu = 0.001_f64.powf(0.25);
    near(m.effective_viscosities()[0], mu);
    near(m.effective_viscosities()[1], 0.001);
    let expected = 0.001 * 0.2 * 0.001_f64.powi(2) / (2.0 * mu);
    m.step_with_surface_shear(0.001, [0.0; 3], &[[-0.2, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    assert!((m.film().thickness()[1] * 0.5 - 0.0002 - expected).abs() < 3e-19);
    m.diffuse(0.1, 2.0, 0.001).unwrap();
    for (row, mu) in m.fractions().iter().zip(m.effective_viscosities()) {
        near(mu, (row[0] * 0.001_f64.ln()).exp());
    }
}
#[test]
fn viscosity_feedback_is_recomputed_each_internal_interval() {
    let mut internal = mixture([0.0005, 0.0002]);
    let mut external = mixture([0.0005, 0.0002]);
    for m in [&mut internal, &mut external] {
        m.configure_viscosities(Some(vec![0.001, 1.0])).unwrap();
    }
    let original = internal.effective_viscosities();
    let stress = [[1.0, 0.0, 0.0]; 2];
    internal
        .step_with_surface_shear(0.1, [-1.0, 0.0, 0.0], &stress, 0.001)
        .unwrap();
    for _ in 0..100 {
        external
            .step_with_surface_shear(0.001, [-1.0, 0.0, 0.0], &stress, 0.001)
            .unwrap();
    }
    assert_eq!(internal.film().thickness(), external.film().thickness());
    assert_eq!(internal.fractions(), external.fractions());
    assert_ne!(original, internal.effective_viscosities());
}
#[test]
fn viscosity_configuration_is_atomic_bounded_and_explicitly_conflicts_with_rheology() {
    let mut m = mixture([0.0005, 0.0002]);
    m.configure_viscosities(Some(vec![0.001, 1.0])).unwrap();
    let before = m.effective_viscosities();
    for values in [
        vec![1.0],
        vec![0.0, 1.0],
        vec![f64::NAN, 1.0],
        vec![f64::INFINITY, 1.0],
    ] {
        assert!(m.configure_viscosities(Some(values)).is_err());
        assert_eq!(m.effective_viscosities(), before);
    }
    let h = m.film().thickness();
    let fractions = m.fractions();
    let model = physics::surface_film::FilmRheology {
        consistency: 0.05,
        flow_index: 1.0,
        yield_stress: 0.0,
        profile_samples: 16,
    };
    assert!(
        m.step_with_rheology(0.01, [0.0; 3], &[[0.0; 3]; 2], 0.001, model)
            .is_err()
    );
    assert_eq!(m.film().thickness(), h);
    assert_eq!(m.fractions(), fractions);
    m.configure_viscosities(Some(vec![f64::from_bits(1), f64::MAX]))
        .unwrap();
    assert!(
        m.effective_viscosities()
            .iter()
            .all(|v| v.is_finite() && *v > 0.0)
    );
    m.configure_viscosities(None).unwrap();
    assert_eq!(
        m.effective_viscosities(),
        vec![m.film().material().viscosity; 2]
    );
}

fn slider_body() -> physics::liquid::ThermalTranslatingBody {
    physics::liquid::ThermalTranslatingBody {
        mechanics: physics::liquid::TranslatingBody {
            position: [0.5, 0.05, 0.5],
            velocity: [0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    }
}
fn slider_patch() -> physics::surface_film::SlidingPatch {
    physics::surface_film::SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.5, 0.5],
    }
}
fn slider_mixture() -> FilmMixture {
    FilmMixture::new(
        film([0.05, 0.05]),
        vec!["a".into(), "b".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )
    .unwrap()
}
#[test]
fn moving_mixture_patch_uses_local_viscous_drag_and_transports_composition() {
    for (values, exponent) in [
        (vec![0.01, 0.1], 0.00055_f64),
        (vec![0.001, 0.002], 0.000015),
    ] {
        let mut m = slider_mixture();
        m.configure_viscosities(Some(values)).unwrap();
        let mut body = slider_body();
        let before = body;
        let masses = m.component_masses().unwrap();
        let report = m
            .advance_sliding_patch(0.001, slider_patch(), &mut body, 0.001)
            .unwrap();
        // Both overlaps are 0.5 m² at physical gap 0.05 m; changing viscosity
        // must not alter that gap or spuriously fail its wet-volume validation.
        near(body.mechanics.velocity[0], 0.3 * (-exponent).exp());
        near(report.mean_wetted_area, 1.0);
        assert!(m.fractions()[0][1] > 0.0);
        assert!(body.mechanics.position[0] > before.mechanics.position[0]);
        for (a, b) in masses.into_iter().zip(m.component_masses().unwrap()) {
            near(a, b);
        }
        let ke = |b: physics::liquid::ThermalTranslatingBody| {
            b.mechanics.velocity.iter().map(|v| v * v).sum::<f64>()
        };
        assert!(
            (ke(body) + body.thermal_energy().unwrap()
                - ke(before)
                - before.thermal_energy().unwrap())
            .abs()
                < 1e-10
        );
        assert!(
            (2.0 * (body.mechanics.velocity[0] - before.mechanics.velocity[0])
                + report.substrate_impulse[0])
                .abs()
                < 1e-15
        );
    }
}
#[test]
fn mixture_patch_recomputes_state_and_retains_uniform_film_compatibility() {
    let mut internal = slider_mixture();
    let mut external = slider_mixture();
    for m in [&mut internal, &mut external] {
        m.configure_viscosities(Some(vec![0.01, 0.1])).unwrap();
    }
    let mut a = slider_body();
    let mut b = a;
    internal
        .advance_sliding_patch(0.1, slider_patch(), &mut a, 0.001)
        .unwrap();
    for _ in 0..100 {
        external
            .advance_sliding_patch(0.001, slider_patch(), &mut b, 0.001)
            .unwrap();
    }
    assert_eq!(a, b);
    assert_eq!(internal.film().thickness(), external.film().thickness());
    assert_eq!(internal.fractions(), external.fractions());
    let mut plain = film([0.05, 0.05]);
    let mut mixed = slider_mixture();
    let mut a = slider_body();
    let mut b = a;
    let ra = plain
        .advance_sliding_patch(0.1, slider_patch(), &mut a, 0.001)
        .unwrap();
    let rb = mixed
        .advance_sliding_patch(0.1, slider_patch(), &mut b, 0.001)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(ra, rb);
    same_height_to_roundoff(&plain.thickness(), &mixed.film().thickness());
}
#[test]
fn late_mixture_patch_heat_failure_rolls_back_volume_composition_and_body() {
    let mut m = slider_mixture();
    m.configure_viscosities(Some(vec![1000.0, 1000.0])).unwrap();
    let mut body = slider_body();
    let original = body;
    let h = m.film().thickness();
    let fractions = m.fractions();
    let masses = m.component_masses().unwrap();
    // Exponent 10 per first interval: initial heat is representable, later tiny
    // residual damping heat is below the stored body's temperature precision.
    assert!(
        m.advance_sliding_patch(0.1, slider_patch(), &mut body, 0.001)
            .is_err()
    );
    assert_eq!(body, original);
    assert_eq!(m.film().thickness(), h);
    assert_eq!(m.fractions(), fractions);
    assert_eq!(m.component_masses().unwrap(), masses);
}
