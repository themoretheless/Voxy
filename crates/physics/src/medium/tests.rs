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
