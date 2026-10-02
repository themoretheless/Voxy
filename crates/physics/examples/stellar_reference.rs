//! Physical REACLIB/free-free reference probe; prescribed radius and surface
//! pressure, fixed He/C composition, no claim of matching an observed star.
use physics::{
    astrophysics_equilibrium::Search,
    astrophysics_gas::{Boundary, Cell},
    astrophysics_nuclear::{Network, Nucleus},
    astrophysics_opacity::FreeFree,
    astrophysics_reaclib::{MEV_JOULES, parse},
    astrophysics_spherical::Sphere,
    astrophysics_spherical_radiation::FreeFreeSpectrum,
};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, fallback: f64| {
        args.get(i).map_or(Ok(fallback), |s| {
            s.parse::<f64>().map_err(|e| e.to_string())
        })
    };
    let n_value = number(0, 4.0)?;
    if n_value.fract() != 0.0 || !(2.0..=64.0).contains(&n_value) {
        return Err("shells must be 2..64".into());
    }
    let n = n_value as usize;
    let radius = number(1, 1e8)?;
    let rho = number(2, 1e6)?;
    let temperature = number(3, 2e8)?;
    let surface_temperature = number(4, 1e6)?;
    let surface_density = number(5, 1.0)?;
    let bins = number(6, 32.0)?;
    let iterations = number(10, 150.0)?;
    if iterations.fract() != 0.0 || !(1.0..=2000.0).contains(&iterations) {
        return Err("iterations must be 1..2000".into());
    }
    let iterations = iterations as usize;
    let evaluation_budget = iterations * (2 * n + 1 + 120) + 1;
    if bins.fract() != 0.0 || !(4.0..=1024.0).contains(&bins) {
        return Err("bins must be 4..1024".into());
    }
    let rate = parse(
        include_str!("../tests/data/triple_alpha_fy05.reaclib"),
        1e6,
        1e9,
        3,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: rate.q_mev * MEV_JOULES,
            },
        ],
        reactions: vec![],
    };
    network.reactions.push(
        rate.reaction(&network, &["he4", "c12"], 0.0, 1e-10)
            .map_err(|e| format!("{e:?}"))?,
    );
    let rows = vec![vec![1.0, 0.0]; n];
    let mixture = network.mixture(&rows[0]).map_err(|e| format!("{e:?}"))?;
    let initial = mixture.at(rho, temperature).map_err(|e| format!("{e:?}"))?;
    let outer = mixture
        .at(surface_density, surface_temperature)
        .map_err(|e| format!("{e:?}"))?;
    let mut sphere = Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy: initial.internal_energy_density
            };
            n
        ],
        spacing: radius / n_value,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Reflecting,
    };
    sphere
        .initialize_hydrostatic(
            &rows,
            &network,
            outer.gas_pressure + outer.radiation_pressure,
        )
        .map_err(|e| format!("{e:?}"))?;
    // Optional seed/output paths follow the seven numeric arguments. A dash
    // means no seed. Conservatively remap piecewise-constant shell primitives.
    if let Some(path) = args.get(7).filter(|p| p.as_str() != "-") {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut seed = Vec::<[f64; 4]>::new();
        for line in text.lines().skip(1) {
            let row = line
                .split(',')
                .map(|v| v.parse::<f64>().map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            if row.len() != 4
                || row.iter().any(|v| !v.is_finite())
                || row[0] < 0.0
                || row[1] <= row[0]
                || row[2] <= 0.0
                || row[3] <= 0.0
            {
                return Err("invalid seed shell".into());
            }
            let previous = seed.last().map_or(0.0, |r| r[1]);
            if (row[0] - previous).abs() > 1e-12 * radius {
                return Err("noncontiguous seed".into());
            }
            seed.push([row[0], row[1], row[2], row[3]]);
        }
        if seed.len() < 2 || (seed.last().unwrap()[1] - radius).abs() > 1e-12 * radius {
            return Err("seed radius mismatch".into());
        }
        let volume =
            |a: f64, b: f64| 4.0 * std::f64::consts::PI / 3.0 * (b - a) * (a * a + a * b + b * b);
        for (i, cell) in sphere.cells.iter_mut().enumerate() {
            let a = i as f64 * sphere.spacing;
            let b = a + sphere.spacing;
            let mut mass = 0.0;
            let mut energy = 0.0;
            for row in &seed {
                let lo = a.max(row[0]);
                let hi = b.min(row[1]);
                if hi > lo {
                    let v = volume(lo, hi);
                    mass += v * row[2];
                    energy += v * row[3];
                }
            }
            cell.density = mass / volume(a, b);
            cell.energy = energy / volume(a, b);
        }
        let seed_mass: f64 = seed.iter().map(|r| volume(r[0], r[1]) * r[2]).sum();
        let seed_energy: f64 = seed.iter().map(|r| volume(r[0], r[1]) * r[3]).sum();
        let totals = sphere.totals().map_err(|e| format!("{e:?}"))?;
        let mass_error = (totals[0] / seed_mass - 1.0).abs();
        let energy_error = (totals[2] / seed_energy - 1.0).abs();
        eprintln!("remap: mass_error={mass_error} thermal_energy_error={energy_error}");
        if mass_error > 1e-12 || energy_error > 1e-12 {
            return Err("nonconservative seed remap".into());
        }
    }
    let spectrum = FreeFreeSpectrum {
        surface: physics::astrophysics_spherical_radiation::SpectralSurface::EddingtonApproximation,
        absorption: FreeFree {
            gaunt_factor: 1.0,
            min_temperature: 1e6,
            max_temperature: 1e9,
        },
        min_frequency: 1e12,
        max_frequency: 1e22,
        bins: bins as usize,
        ambient_temperature: 0.0,
    };
    let initial_rates = sphere
        .thermal_rates_free_free(&rows, &network, spectrum, 4, 100_000_000, n * 3)
        .map_err(|e| format!("{e:?}"))?;
    eprintln!(
        "initial: mass={} nuclear={} luminosity={} residual={}",
        sphere.totals().unwrap()[0],
        initial_rates.nuclear.iter().sum::<f64>(),
        initial_rates.luminosity,
        initial_rates.relative_imbalance().unwrap()
    );
    let absolute_power_tolerance = number(9, initial_rates.nuclear.iter().sum::<f64>() * 1e-6)?;
    let ray_budget = 2 * n * (n + 1) * 4 * (bins as usize) * evaluation_budget;
    let report = sphere
        .equilibrate_stellar_free_free(
            &rows,
            &network,
            outer.gas_pressure + outer.radiation_pressure,
            spectrum,
            4,
            ray_budget,
            Search {
                absolute_power_tolerance,
                min_density: 1e-6,
                max_density: 1e10,
                min_temperature: 1.01e6,
                max_temperature: 9.99e8,
                relative_tolerance: 1e-6,
                iterations,
                evaluations: evaluation_budget,
                fit_evaluations: 3 * n * evaluation_budget,
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    println!(
        "shells,radius_m,mass_kg,luminosity_W,hydrostatic_residual,thermal_residual,absolute_power_tolerance_W,maximum_net_power_W,iterations,evaluations"
    );
    println!(
        "{n},{radius},{},{},{},{},{},{},{},{}",
        sphere.totals().unwrap()[0],
        report.thermal.luminosity,
        report.hydrostatic_residual,
        report.thermal_residual,
        absolute_power_tolerance,
        report.maximum_net_power,
        report.iterations,
        report.evaluations
    );
    if let Some(path) = args.get(8) {
        let mut csv = String::from("inner_m,outer_m,density_kg_m3,energy_J_m3\n");
        for (i, cell) in sphere.cells.iter().enumerate() {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                i as f64 * sphere.spacing,
                (i + 1) as f64 * sphere.spacing,
                cell.density,
                cell.energy
            ));
        }
        std::fs::write(path, csv).map_err(|e| e.to_string())?;
    }
    for cell in &sphere.cells {
        eprintln!(
            "rho={} T={}",
            cell.density,
            mixture.temperature(cell.density, cell.energy).unwrap()
        );
    }
    Ok(())
}
