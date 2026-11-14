use physics::liquid::{
    Config, DepositingImpact, FilmRebound, ImpactSpray, Liquid, LiquidField, Material, Particle,
    TransportMaterial,
};
use physics::surface_film::{FilmMixture, Material as FilmMaterial, SurfaceFilm};
fn mixture(wall: bool) -> FilmMixture {
    let mut points = vec![
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
    ];
    let mut triangles = vec![[0, 1, 2], [0, 2, 3]];
    if wall {
        points.extend([
            [0.02, -0.1, -0.1],
            [0.02, 0.1, -0.1],
            [0.02, 0.1, 0.1],
            [0.02, -0.1, 0.1],
        ]);
        triangles.extend([[4, 5, 6], [4, 6, 7]]);
    }
    let count = triangles.len();
    FilmMixture::new(
        SurfaceFilm::new(&points, triangles, FilmMaterial::default()).unwrap(),
        vec!["a".into(), "b".into()],
        vec![vec![0.5, 0.5]; count],
    )
    .unwrap()
}
fn fluid(y: f64) -> Liquid {
    let mut liquid = Liquid::new(
        [0.2, 0.0, 2.0]
            .into_iter()
            .enumerate()
            .map(|(i, x)| Particle {
                position: [x, y, 0.0],
                velocity: [0.0, if i == 1 { -2.0 } else { -0.1 }, 0.0],
                mass: 0.001,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_species(
            vec!["a".into(), "b".into()],
            vec![vec![0.9, 0.1], vec![0.2, 0.8], vec![0.5, 0.5]],
        )
        .unwrap();
    liquid
}
fn paths(liquid: &Liquid) -> Vec<[f64; 3]> {
    liquid
        .particles()
        .iter()
        .map(|p| [p.position[0], 0.02, 0.0])
        .collect()
}
fn model() -> DepositingImpact {
    DepositingImpact {
        capture_speed: 0.8,
        spray: ImpactSpray {
            rebound: FilmRebound {
                restitution: 0.5,
                friction: 0.1,
            },
            children: 4,
            position_radius: 0.001,
            surface_tension: 0.072,
            fragmentation_fraction: 0.8,
        },
    }
}
fn ke(liquid: &Liquid) -> f64 {
    liquid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
#[test]
fn finite_contacts_deposit_and_fragment_without_center_crossings() {
    let mut liquid = fluid(0.002);
    let original = liquid.clone();
    let mut film = mixture(false);
    let previous = paths(&liquid);
    assert!(previous.iter().zip(liquid.particles()).all(|(p, q)| {
        film.film()
            .first_segment_hit(*p, q.position)
            .unwrap()
            .is_none()
    }));
    let radii = liquid.equivalent_sphere_radii().unwrap();
    let report = liquid
        .depositing_impact_spheres_surface_mixture(&previous, &mut film, model(), &radii)
        .unwrap();
    assert_eq!(report.deposition.capture.particles, 1);
    assert_eq!(report.spray.impacts, 1);
    assert_eq!(report.spray.fragments_created, 4);
    assert!((liquid.mass() + film.film().total_mass() - original.mass()).abs() < 1e-15);
    for (k, mass) in liquid.species_totals().unwrap().unwrap().iter().enumerate() {
        assert!(
            (mass + film.component_masses().unwrap()[k]
                - original.species_totals().unwrap().unwrap()[k])
                .abs()
                < 1e-15
        );
    }
    assert!(
        (ke(&liquid)
            + report.spray.substrate_heat
            + report.spray.created_surface_energy
            + report.deposition.capture.absorbed.kinetic_energy
            - ke(&original))
        .abs()
            < 1e-15
    );
    assert!(
        (liquid.transport_totals().unwrap().unwrap().0
            + report.deposition.capture.absorbed.thermal_energy.unwrap()
            - original.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-9
    );
    for k in 0..3 {
        let momentum = |l: &Liquid| {
            l.particles()
                .iter()
                .map(|p| p.mass * p.velocity[k])
                .sum::<f64>()
        };
        assert!(
            (momentum(&liquid)
                + report.spray.substrate_impulse[k]
                + report.deposition.capture.absorbed.momentum[k]
                - momentum(&original))
            .abs()
                < 1e-15
        );
    }
    for (p, r) in liquid
        .particles()
        .iter()
        .zip(liquid.equivalent_sphere_radii().unwrap())
    {
        assert!(
            film.film()
                .first_sphere_hit(p.position, p.position, r)
                .unwrap()
                .is_none_or(|h| h.penetration < 1e-14)
        );
    }
}
#[test]
fn late_fragment_clearance_failure_rolls_back_proposed_deposition() {
    for wall in [false, true] {
        let mut liquid = fluid(if wall { 0.002 } else { 0.00005 });
        let initial = liquid.clone();
        let mut film = mixture(wall);
        let before = film.component_masses().unwrap();
        let mut controls = model();
        if wall {
            controls.spray.children = 64;
        }
        let radii = if wall {
            liquid.equivalent_sphere_radii().unwrap()
        } else {
            vec![0.0001; 3]
        };
        assert_eq!(
            liquid
                .depositing_impact_spheres_surface_mixture(
                    &paths(&liquid),
                    &mut film,
                    controls,
                    &radii
                )
                .unwrap_err(),
            "fragment sphere penetrates film mesh"
        );
        assert_eq!(liquid, initial);
        assert_eq!(film.component_masses().unwrap(), before);
        assert_eq!(film.film().total_mass(), 0.0);
    }
}
