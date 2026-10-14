use super::*;
use crate::liquid::{GasGridBoundary, VaporCell};

fn gas(n: [usize; 3], h: f64, velocity: [f64; 3]) -> FiniteDropletGasGrid {
    let volume = h * h * h;
    let count = n[0] * n[1] * n[2];
    FiniteDropletGasGrid::new(
        [0.; 3],
        [h; 3],
        n,
        vec![
            VaporCell {
                mass: 1.2 * volume,
                volume,
                temperature: 300.,
                velocity,
                specific_heat_cv: 718.,
            };
            count
        ],
    )
    .unwrap()
}

fn species(kind: MediumSpecies) -> MediumSpeciesProperties {
    MediumSpeciesProperties {
        kind,
        mass_extinction_m2_kg: 2.,
        albedo: 0.5,
        settling_velocity_m_s: 0.,
        diffusivity_m2_s: 0.,
        buoyancy_per_kg_m3: 0.,
    }
}

fn still() -> MediumStepControl {
    MediumStepControl {
        advance_gas: false,
        gravity: [0., -9.81, 0.],
        ..MediumStepControl::default()
    }
}

fn centroid(medium: &ParticipatingMedium, s: usize) -> [f64; 3] {
    let shape = medium.gas().shape();
    let h = medium.gas().spacing();
    let mass = medium.mass_kg(s);
    let total: f64 = mass.iter().sum();
    let mut c = [0.; 3];
    for (i, m) in mass.iter().enumerate() {
        let ijk = [
            i % shape[0],
            (i / shape[0]) % shape[1],
            i / (shape[0] * shape[1]),
        ];
        for k in 0..3 {
            c[k] += m * (ijk[k] as f64 + 0.5) * h[k];
        }
    }
    c.map(|v| v / total)
}

#[test]
fn gaussian_diffusion_matches_the_analytic_kernel() {
    let n = 32;
    let h = 0.05;
    let d = 0.01;
    let mut medium = ParticipatingMedium::new(
        gas([n; 3], h, [0.; 3]),
        vec![MediumSpeciesProperties {
            diffusivity_m2_s: d,
            ..species(MediumSpecies::Smoke)
        }],
    )
    .unwrap();
    let center = [0.5 * n as f64 * h; 3];
    let shape = medium.gas().shape();
    let gaussian = |sigma2: f64, i: usize| {
        let ijk = [
            i % shape[0],
            (i / shape[0]) % shape[1],
            i / (shape[0] * shape[1]),
        ];
        let r2: f64 = (0..3)
            .map(|k| ((ijk[k] as f64 + 0.5) * h - center[k]).powi(2))
            .sum();
        (-r2 / (2. * sigma2)).exp() / (2. * std::f64::consts::PI * sigma2).powf(1.5)
    };
    // A resolved Gaussian (σ₀ = 3h) stays Gaussian with σ² = σ₀² + 2Dt.
    let sigma0 = 3. * h;
    let volume = h * h * h;
    for i in 0..shape[0] * shape[1] * shape[2] {
        let ijk = [
            i % shape[0],
            (i / shape[0]) % shape[1],
            i / (shape[0] * shape[1]),
        ];
        let point = [
            (ijk[0] as f64 + 0.5) * h,
            (ijk[1] as f64 + 0.5) * h,
            (ijk[2] as f64 + 0.5) * h,
        ];
        medium
            .deposit(0, point, gaussian(sigma0 * sigma0, i) * volume)
            .unwrap();
    }
    let total = medium.total_mass_kg(0);
    let t = 1.;
    let report = medium.advance(t, still()).unwrap();
    assert!((report.mass_after_kg[0] - total).abs() < 1e-12 * total);
    let sigma2 = sigma0 * sigma0 + 2. * d * t;
    let c = medium.concentration_kg_m3(0);
    let (mut num, mut den) = (0., 0.);
    for (i, value) in c.iter().enumerate() {
        let analytic = gaussian(sigma2, i) * total;
        num += (value - analytic).powi(2);
        den += analytic * analytic;
    }
    let l2 = (num / den).sqrt();
    assert!(l2 < 0.02, "L2 error {l2}");
}

#[test]
fn uniform_advection_translates_the_centroid_exactly_and_conserves_mass() {
    let h = 0.1;
    let velocity = [0.3, 0., 0.];
    let mut medium = ParticipatingMedium::new(
        gas([40, 4, 4], h, velocity),
        vec![species(MediumSpecies::Fog)],
    )
    .unwrap();
    medium.deposit(0, [0.55, 0.25, 0.25], 2.).unwrap();
    medium.deposit(0, [0.75, 0.25, 0.25], 1.).unwrap();
    let before = centroid(&medium, 0);
    let t = 2.;
    let report = medium.advance(t, still()).unwrap();
    let after = centroid(&medium, 0);
    assert!(
        (after[0] - before[0] - velocity[0] * t).abs() < 1e-9,
        "{before:?} -> {after:?}"
    );
    assert!((after[1] - before[1]).abs() < 1e-12);
    assert!((report.mass_after_kg[0] - 3.).abs() < 1e-12);
    assert!(report.substeps >= 12);
}

#[test]
fn buoyancy_accelerates_the_gas_and_settling_drains_through_the_floor() {
    let h = 0.25;
    let mut medium = ParticipatingMedium::new(
        gas([2, 8, 2], h, [0.; 3]),
        vec![
            MediumSpeciesProperties {
                buoyancy_per_kg_m3: 4.,
                ..species(MediumSpecies::Smoke)
            },
            MediumSpeciesProperties {
                settling_velocity_m_s: 0.5,
                ..species(MediumSpecies::Dust)
            },
        ],
    )
    .unwrap();
    let volume = h * h * h;
    // Smoke in one cell: concentration 1/volume kg/m³.
    medium.deposit(0, [0.1, 1.1, 0.1], 1.).unwrap();
    // Uniform dust column: 0.5 kg per cell.
    for j in 0..8 {
        for i in 0..2 {
            for k in 0..2 {
                medium
                    .deposit(
                        1,
                        [
                            (i as f64 + 0.5) * h,
                            (j as f64 + 0.5) * h,
                            (k as f64 + 0.5) * h,
                        ],
                        0.5,
                    )
                    .unwrap();
            }
        }
    }
    let dt = 0.01;
    let report = medium.advance(dt, still()).unwrap();
    let cell = medium.gas().cell_index([0.1, 1.1, 0.1]).unwrap().unwrap();
    let expected_v = 4. * (1. / volume) * dt;
    assert!((medium.gas().cells()[cell].velocity[1] - expected_v).abs() < 1e-12);
    // Settling flux through the floor: v_s × c × A × dt over four bottom cells.
    let c = 0.5 / volume;
    let expected_out = 0.5 * c * (h * h) * dt * 4.;
    assert!(
        (report.settled_kg[1] - expected_out).abs() < 1e-9,
        "{} vs {expected_out}",
        report.settled_kg[1]
    );
    assert!(
        (report.mass_before_kg[1] - report.mass_after_kg[1] - report.settled_kg[1]).abs() < 1e-9
    );
    // Smoke mass is conserved with reflecting walls.
    assert!((report.mass_after_kg[0] - 1.).abs() < 1e-12);
}

#[test]
fn extinction_grid_sums_species_and_mist_matches_droplet_cross_sections() {
    let h = 0.5;
    let radius = 1e-5;
    let mist = MediumSpeciesProperties::water_mist(radius, 2., 1000.);
    let mut medium = ParticipatingMedium::new(
        gas([2, 1, 1], h, [0.; 3]),
        vec![species(MediumSpecies::Smoke), mist],
    )
    .unwrap();
    medium.deposit(0, [0.25, 0.25, 0.25], 0.3).unwrap();
    // One droplet's worth of water mass in the second cell.
    let drop_mass = 1000. * (4. / 3.) * std::f64::consts::PI * radius.powi(3);
    medium.deposit(1, [0.75, 0.25, 0.25], drop_mass).unwrap();
    let grid = medium.extinction_grid().unwrap();
    let volume = h * h * h;
    assert!((grid.extinction_m_inverse()[0] - 2. * 0.3 / volume).abs() < 1e-12);
    let droplet_cross_section = 2. * std::f64::consts::PI * radius * radius / volume;
    assert!(
        (grid.extinction_m_inverse()[1] - droplet_cross_section).abs()
            < 1e-12 * droplet_cross_section
    );
    assert_eq!(grid.shape(), [2, 1, 1]);
    let albedo = medium.albedo_field();
    assert!((albedo[0] - 0.5).abs() < 1e-12 && (albedo[1] - 1.).abs() < 1e-12);
    assert!(!medium.deposit(0, [5., 0., 0.], 1.).unwrap());
    assert!(medium.deposit(7, [0.1, 0.1, 0.1], 1.).is_err());
    assert!(medium.advance(-1., still()).is_err());
    let tight = MediumStepControl {
        max_substeps: 1,
        ..still()
    };
    let mut fast = ParticipatingMedium::new(
        gas([8, 1, 1], 0.1, [5., 0., 0.]),
        vec![species(MediumSpecies::Fog)],
    )
    .unwrap();
    fast.deposit(0, [0.05, 0.05, 0.05], 1.).unwrap();
    assert_eq!(fast.advance(1., tight).err(), Some(Error::SubstepBudget));
    let _ = GasGridBoundary::Reflecting;
}

#[test]
fn film_drying_moves_solvent_mass_into_the_containing_gas_cell() {
    use crate::liquid::{
        SaturationCurve, SolutionVaporInterface, VaporExchangeAccuracy, VaporInterface,
    };
    use crate::surface_film::{FilmMixture, Material, SurfaceFilm, ThermalFilmMixture};
    let mut surface = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        vec![[0, 1, 2]],
        Material {
            density: 1.,
            ..Material::default()
        },
    )
    .unwrap();
    surface.deposit(0, 1.).unwrap();
    let mut film = ThermalFilmMixture::new(
        FilmMixture::new(
            surface,
            vec!["solvent".into(), "residue".into()],
            vec![vec![0.7, 0.3]],
        )
        .unwrap(),
        vec![2., 3.],
        &[10.],
    )
    .unwrap();
    let model = SolutionVaporInterface {
        interface: VaporInterface {
            curve: SaturationCurve {
                reference_temperature: 10.,
                reference_pressure: 10.,
                latent_heat: 100.,
                vapor_gas_constant: 1.,
                min_temperature: 5.,
                max_temperature: 20.,
            },
            area: 0.1,
            accommodation: 0.01,
        },
        solvent: 0,
        molar_masses: vec![1., 1.],
    };
    // Dry gas at the film temperature: the solvent evaporates into it.
    let volume = 1.;
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 1e-9,
                volume,
                temperature: 10.,
                velocity: [0.; 3],
                specific_heat_cv: 1.,
            };
            2
        ],
    )
    .unwrap();
    let mut medium = ParticipatingMedium::new(gas, vec![species(MediumSpecies::Mist)]).unwrap();
    let film_mass_before: f64 = film.mixture().component_masses().unwrap().iter().sum();
    let gas_before: f64 = medium.gas().cells().iter().map(|c| c.mass).sum();
    let mut velocity = [0.; 3];
    let exchange = medium
        .exchange_film_vapor(
            &mut film,
            0,
            &mut velocity,
            [1.5, 0.5, 0.5],
            &model,
            0.1,
            VaporExchangeAccuracy::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(exchange.cell, 1);
    assert!(exchange.transferred_kg > 0., "{exchange:?}");
    let film_mass_after: f64 = film.mixture().component_masses().unwrap().iter().sum();
    let gas_after: f64 = medium.gas().cells().iter().map(|c| c.mass).sum();
    assert!((film_mass_before - film_mass_after - exchange.transferred_kg).abs() < 1e-12);
    assert!((gas_after - gas_before - exchange.transferred_kg).abs() < 1e-12);
    assert_eq!(
        medium.gas().cells()[0].mass,
        1e-9,
        "the other cell is untouched"
    );
    // Outside the grid nothing happens.
    let untouched = film.clone();
    assert!(
        medium
            .exchange_film_vapor(
                &mut film,
                0,
                &mut velocity,
                [9., 9., 9.],
                &model,
                0.1,
                VaporExchangeAccuracy::default()
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(
        film.mixture().component_masses().unwrap(),
        untouched.mixture().component_masses().unwrap()
    );
}

#[test]
fn multi_liquid_jets_splash_into_mist_wet_smear_and_dry_conserving_mass() {
    use crate::friction;
    use crate::liquid::{
        Config, DepositingImpact, DropletCoalescenceControl, DropletFlight, DropletLifecycle,
        EmissionPulse, FilmImpactControl, FilmRebound, ImpactSpray, Liquid, LiquidField, Material,
        Particle, ParticleInput, PulsedEmitter, SaturationCurve, SolutionVaporInterface,
        TransportMaterial, VaporExchangeAccuracy, VaporInterface,
    };
    use crate::surface_film::{
        FilmMixture, Material as FilmMaterial, SmearContact, SmearRegime, SurfaceFilm,
        ThermalFilmMixture,
    };

    let mut smear_works = Vec::new();
    for fluid_mat in [Material::WATER, Material::GLYCERIN] {
        let mut liquid = Liquid::new(
            vec![],
            vec![fluid_mat],
            Config {
                smoothing_radius: 0.16,
                particle_radius: 0.005,
                ..Config::default()
            },
        )
        .unwrap();
        liquid
            .configure_transport(
                vec![],
                vec![TransportMaterial {
                    specific_heat: 728.0,
                    conductivity: 0.0,
                    diffusivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        liquid
            .configure_species(vec!["solvent".into()], vec![])
            .unwrap();
        liquid.configure_droplet_population(Some(vec![])).unwrap();

        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 0.05,
                volume: 0.001,
                speed: 4.5,
            }],
            ParticleInput {
                particle: Particle {
                    position: [0.5, 0.22, 0.5],
                    velocity: [0.0; 3],
                    mass: 1.0,
                    material: 0,
                },
                field: Some(LiquidField {
                    temperature: 300.0,
                    concentration: 0.0,
                }),
                phase_fraction: None,
            },
        )
        .unwrap();
        emitter.direction = [0.0, -1.0, 0.0];
        emitter.nozzle_radius = 0.03;
        emitter.particle_volume = 0.0001;
        emitter.density = fluid_mat.rest_density;

        let points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let triangles = vec![[0, 1, 2], [0, 2, 3]];
        let surface = SurfaceFilm::new(
            &points,
            triangles.clone(),
            FilmMaterial {
                density: fluid_mat.rest_density,
                viscosity: fluid_mat.viscosity,
                wetting: 1e-5,
                surface_tension: 0.072,
            },
        )
        .unwrap();
        let mixture =
            FilmMixture::new(surface, vec!["solvent".into()], vec![vec![1.0]; 2]).unwrap();
        let mut thermal_film = ThermalFilmMixture::new(mixture, vec![728.0], &[300.0; 2]).unwrap();

        let gas_grid = FiniteDropletGasGrid::new(
            [0.0, -0.1, 0.0],
            [1.0, 0.6, 1.0],
            [1, 1, 1],
            vec![VaporCell {
                mass: 1e-6,
                volume: 0.6,
                temperature: 300.0,
                velocity: [0.0; 3],
                specific_heat_cv: 718.0,
            }],
        )
        .unwrap();
        let mut medium = ParticipatingMedium::new(
            gas_grid,
            vec![MediumSpeciesProperties::water_mist(
                0.01,
                2.0,
                fluid_mat.rest_density,
            )],
        )
        .unwrap();
        let initial_gas_mass: f64 = medium.gas().cells().iter().map(|c| c.mass).sum();

        let dt = 1.0 / 120.0;
        let mut emitted_mass = 0.0;
        let mut total_captured = 0;
        let mut total_fragments = 0;
        for _ in 0..12 {
            let ex = emitter
                .advance_with_species(&mut liquid, dt, &[1.0])
                .unwrap();
            emitted_mass += ex.added.mass;
            let initial_velocities: Vec<_> =
                liquid.particles().iter().map(|p| p.velocity).collect();
            let previous: Vec<_> = liquid.particles().iter().map(|p| p.position).collect();
            liquid.step_carrier_and_droplets(dt, None).unwrap();
            let radii = liquid.equivalent_sphere_radii().unwrap();
            let report = liquid
                .depositing_impact_spheres_thermal_surface_mixture_lifecycle(
                    &previous,
                    &mut thermal_film,
                    DepositingImpact {
                        capture_speed: 3.0,
                        spray: ImpactSpray {
                            rebound: FilmRebound {
                                restitution: 0.45,
                                friction: 0.1,
                            },
                            children: 2,
                            position_radius: 0.01,
                            surface_tension: 0.072,
                            fragmentation_fraction: 0.25,
                        },
                    },
                    &radii,
                    FilmImpactControl {
                        dt,
                        max_contacts_per_lineage: 16,
                        max_events: 1024,
                    },
                    &DropletLifecycle {
                        mass_fractions: Some(vec![0.3, 0.7]),
                        splash_onset: None,
                        capture_growth_contact: true,
                        free_surface_side: Some(1.0),
                        capture_film_immersion: true,
                        flight: Some(DropletFlight {
                            initial_velocities,
                            acceleration: [0.0, -9.81, 0.0],
                            max_feature_checks: 4096,
                        }),
                        coalescence: Some(DropletCoalescenceControl {
                            dt,
                            surface_tension: 0.072,
                            maximum_normal_speed: 0.5,
                            max_events: 1024,
                        }),
                    },
                )
                .unwrap();
            total_captured += report.impact.deposition.capture.particles;
            total_fragments += report.impact.spray.fragments_created;
        }
        assert!(total_captured > 0 && total_fragments > 0);

        // Absorb airborne spray droplets into the participating medium's Mist species.
        let absorbed = medium.absorb_spray_droplets(0, &mut liquid, 0.05).unwrap();
        assert!(absorbed.absorbed_count > 0);
        assert!((absorbed.absorbed_mass_kg - absorbed.exchange.removed.mass).abs() < 1e-14);
        assert!((medium.total_mass_kg(0) - absorbed.absorbed_mass_kg).abs() < 1e-14);

        // Smear the wetted film with a sliding contact and record Couette work.
        let mut smear_film = thermal_film.mixture().film().clone();
        let friction_mat = friction::Material::new(1e7, 1e7, 0.5).unwrap();
        let mut friction_states = vec![friction::State::default(); 2];
        let smear_report = smear_film
            .advance_smear(
                0.01,
                SmearContact {
                    normal: [0.0, 1.0, 0.0],
                    body_velocity_m_s: [0.05, 0.0, 0.0],
                },
                &[50_000.0, 50_000.0],
                &friction_mat,
                &mut friction_states,
                [0.0, -9.81, 0.0],
                0.001,
            )
            .unwrap();
        assert!(smear_report.regimes.contains(&SmearRegime::Lubricated));
        assert!(smear_report.slip_work_j > 0.0);
        smear_works.push(smear_report.slip_work_j);

        // Dry the wetted film into the participating medium's gas grid.
        let vapor_model = SolutionVaporInterface {
            interface: VaporInterface {
                curve: SaturationCurve {
                    reference_temperature: 300.0,
                    reference_pressure: 100.0,
                    latent_heat: 1000.0,
                    vapor_gas_constant: 10.0,
                    min_temperature: 200.0,
                    max_temperature: 400.0,
                },
                area: 0.05,
                accommodation: 1e-4,
            },
            solvent: 0,
            molar_masses: vec![1.0],
        };
        let wet_cell = thermal_film
            .mixture()
            .film()
            .thickness()
            .iter()
            .position(|&h| h > 0.0)
            .unwrap();
        let mut cell_vel = [0.0; 3];
        let drying = medium
            .exchange_film_vapor(
                &mut thermal_film,
                wet_cell,
                &mut cell_vel,
                [0.5, 0.05, 0.5],
                &vapor_model,
                0.01,
                VaporExchangeAccuracy::default(),
            )
            .unwrap()
            .unwrap();
        assert!(drying.transferred_kg > 0.0);

        let total_accounted = liquid.mass()
            + thermal_film.mixture().film().total_mass()
            + medium.total_mass_kg(0)
            + (medium.gas().cells().iter().map(|c| c.mass).sum::<f64>() - initial_gas_mass);
        assert!(
            (total_accounted - emitted_mass).abs() < 1e-11,
            "total_accounted={total_accounted} vs emitted_mass={emitted_mass}"
        );
    }
    // High-viscosity glycerin (1.412 Pa*s) requires more lubricated shear work than water (0.001 Pa*s).
    assert!(smear_works[1] > smear_works[0] * 100.0, "{smear_works:?}");
}

#[test]
fn mixture_density_fog_condensation_and_wet_scavenging_conserve_mass() {
    let h = 0.5;
    let mut medium = ParticipatingMedium::new(
        gas([2, 2, 1], h, [0.0; 3]),
        vec![
            species(MediumSpecies::Smoke),
            species(MediumSpecies::Fog),
            MediumSpeciesProperties {
                settling_velocity_m_s: 0.4,
                ..species(MediumSpecies::Dust)
            },
            MediumSpeciesProperties::water_mist(2e-5, 2.0, 1000.0),
        ],
    )
    .unwrap();

    // Deposit Smoke (0), Dust (2), and Mist (3) in cell 0 ([0.25, 0.25, 0.25])
    let p0 = [0.25, 0.25, 0.25];
    medium.deposit(0, p0, 0.02).unwrap();
    medium.deposit(2, p0, 0.15).unwrap();
    medium.deposit(3, p0, 0.10).unwrap();

    let densities = medium.mixture_density_kg_m3();
    assert!(densities[0] > densities[1]);

    // Richardson-Zaki hindered settling reduces settling velocity in dense dust cell 0 relative to dilute cell 1
    let hindered = medium
        .hindered_settling_velocities_m_s(2, 2400.0, 4.65)
        .unwrap();
    assert!(hindered[0] < hindered[1]);

    // Vapor-to-fog condensation in cool supersaturated gas
    let cond = medium
        .condense_or_evaporate_fog(
            0.2,
            FogCondensationConfig {
                fog_species: 1,
                saturation_density_at_ref_kg_m3: 0.5,
                reference_temperature_k: 300.0,
                clausius_clapeyron_exponent: 12.0,
                relaxation_rate_s_inv: 2.0,
                min_gas_fraction_floor: 0.2,
            },
        )
        .unwrap();
    assert!(cond.net_condensed_fog_mass_kg > 0.0);
    assert!(medium.total_mass_kg(1) > 0.0);
    assert!(
        (cond.total_gas_plus_fog_before_kg - cond.total_gas_plus_fog_after_kg).abs() < 1e-12
    );

    // Wet scavenging of Smoke (0) and Dust (2) by water Mist (3)
    let total_suspended_before: f64 = (0..4).map(|s| medium.total_mass_kg(s)).sum();
    let scavenge = medium
        .scavenge_dust_and_smoke_by_mist(3, &[0, 2], 1.5, 0.25)
        .unwrap();
    assert!(scavenge.total_scavenged_mass_kg > 0.0);
    assert!(scavenge.scavenged_mass_per_species_kg[0] > 0.0);
    assert!(scavenge.scavenged_mass_per_species_kg[2] > 0.0);
    let total_suspended_after: f64 = (0..4).map(|s| medium.total_mass_kg(s)).sum();
    assert!((total_suspended_before - total_suspended_after).abs() < 1e-12);
}
