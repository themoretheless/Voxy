//! Coupled numerical demonstration with synthetic rates, not a calibrated star.
use physics::{
    astrophysics_gas::{Boundary, Cell},
    astrophysics_nuclear::{Budget, Network, Nucleus, Reaclib, Reaction},
    astrophysics_spherical::{CompositionExterior, Sphere},
    astrophysics_spherical_radiation::{Heating, ReactiveSettings},
};
fn main() -> Result<(), String> {
    let mut dt = 0.01_f64;
    let mut shells = 16_usize;
    let mut duration = 1.0_f64;
    let mut tabulated = false;
    let mut hydrostatic = false;
    let mut balanced = false;
    let mut density_contrast = 0.0_f64;
    let mut polytrope_index = None;
    let mut use_polytrope_exterior = false;
    let mut polytrope_radius_fraction = 0.9_f64;
    let mut polytrope_radius_given = false;
    let mut opacity_file = None;
    let mut profile_file = None;
    let mut rate_file = None;
    let mut rate_min = None;
    let mut rate_max = None;
    let mut exterior_density = None;
    let mut exterior_temperature = 2e8_f64;
    let mut exterior_carbon = 0.0_f64;
    let mut exterior_velocity = 0.0_f64;
    let mut exterior_options = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if matches!(
            arg.as_str(),
            "--exterior-density"
                | "--exterior-temperature"
                | "--exterior-carbon"
                | "--exterior-velocity"
        ) {
            let value: f64 = args
                .next()
                .ok_or("exterior option requires a number")?
                .parse()
                .map_err(|_| "invalid exterior value")?;
            match arg.as_str() {
                "--exterior-density" => exterior_density = Some(value),
                "--exterior-temperature" => {
                    exterior_temperature = value;
                    exterior_options = true;
                }
                "--exterior-carbon" => {
                    exterior_carbon = value;
                    exterior_options = true;
                }
                _ => {
                    exterior_velocity = value;
                    exterior_options = true;
                }
            }
            continue;
        }
        if arg == "--polytrope-index" || arg == "--polytrope-radius-fraction" {
            let value: f64 = args
                .next()
                .ok_or("polytrope option requires a number")?
                .parse()
                .map_err(|_| "invalid polytrope value")?;
            if arg == "--polytrope-index" {
                polytrope_index = Some(value);
            } else {
                polytrope_radius_fraction = value;
                polytrope_radius_given = true;
            }
            continue;
        }
        if arg == "--shells" {
            shells = args
                .next()
                .ok_or("--shells requires an integer")?
                .parse()
                .map_err(|_| "invalid shell count")?;
            continue;
        }
        if arg == "--polytrope-exterior" {
            use_polytrope_exterior = true;
            continue;
        }
        if arg == "--profile-out" {
            profile_file = Some(args.next().ok_or("--profile-out requires a path")?);
            continue;
        }
        if arg == "--density-contrast" {
            density_contrast = args
                .next()
                .ok_or("--density-contrast requires a number")?
                .parse()
                .map_err(|_| "invalid density contrast")?;
            continue;
        }
        if arg == "--balanced" {
            balanced = true;
            continue;
        }
        if arg == "--hydrostatic" {
            hydrostatic = true;
            continue;
        }
        if arg == "--duration" {
            duration = args
                .next()
                .ok_or("--duration requires seconds")?
                .parse()
                .map_err(|_| "invalid duration")?;
            continue;
        }
        if arg == "--rate-file" {
            rate_file = Some(args.next().ok_or("--rate-file requires a path")?);
            continue;
        }
        if arg == "--rate-min" || arg == "--rate-max" {
            let value: f64 = args
                .next()
                .ok_or("rate bound requires kelvin")?
                .parse()
                .map_err(|_| "invalid rate temperature bound")?;
            if arg == "--rate-min" {
                rate_min = Some(value);
            } else {
                rate_max = Some(value);
            }
            continue;
        }
        if arg == "--opacity-table" {
            opacity_file = Some(args.next().ok_or("--opacity-table requires a path")?);
            continue;
        }
        if arg == "--tabulated" {
            tabulated = true;
            continue;
        }
        if arg != "--dt" {
            return Err(format!("unknown argument: {arg}"));
        }
        dt = args
            .next()
            .ok_or("--dt requires seconds")?
            .parse()
            .map_err(|_| "invalid dt")?;
    }
    if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
        return Err("dt must be in (0, 0.1] seconds".into());
    }
    if !duration.is_finite() || duration <= 0.0 {
        return Err("duration must be positive seconds".into());
    }
    if !density_contrast.is_finite() || density_contrast < 0.0 {
        return Err("density contrast must be finite and nonnegative".into());
    }
    if polytrope_index.is_some() && (hydrostatic || density_contrast != 0.0) {
        return Err("--polytrope-index supplies its own density and pressure; incompatible with --hydrostatic or nonzero --density-contrast".into());
    }
    if polytrope_radius_given && polytrope_index.is_none() {
        return Err("--polytrope-radius-fraction requires --polytrope-index".into());
    }
    if !polytrope_radius_fraction.is_finite()
        || polytrope_radius_fraction <= 0.0
        || polytrope_radius_fraction >= 1.0
    {
        return Err("polytrope radius fraction must be in (0,1)".into());
    }
    if !(2..=256).contains(&shells) {
        return Err("shell count must be in [2,256]; radiation work budgets still apply".into());
    }
    if use_polytrope_exterior
        && (polytrope_index.is_none() || exterior_density.is_some() || exterior_options)
    {
        return Err("--polytrope-exterior requires --polytrope-index and cannot combine with explicit exterior options".into());
    }
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
                binding_energy: 1e-12,
            },
        ],
        reactions: vec![Reaction {
            reactants: vec![3, 0],
            products: vec![0, 1],
            rate: Reaclib {
                sets: vec![[6e-4_f64.ln(), 0.0, 0.0, 0.0, 0.0, 0.0, 2.0]],
                min_temperature: 1e7,
                max_temperature: 1e9,
            },
            neutrino_fraction: 0.1,
        }],
    };
    if let Some(path) = rate_file {
        let min = rate_min.ok_or("--rate-file requires --rate-min in kelvin")?;
        let max = rate_max.ok_or("--rate-file requires --rate-max in kelvin")?;
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let rate = physics::astrophysics_reaclib::parse(&text, min, max, 1000)
            .map_err(|e| format!("{e:?}"))?;
        if rate.reactants != ["he4", "he4", "he4"] || rate.products != ["c12"] || rate.q_mev <= 0.0
        {
            return Err("example requires forward triple-alpha capture".into());
        }
        network.nuclei[1].binding_energy = rate.q_mev * physics::astrophysics_reaclib::MEV_JOULES;
        network.reactions.clear();
        network.reactions.push(
            rate.reaction(&network, &["he4", "c12"], 0.0, 1e-10)
                .map_err(|e| format!("{e:?}"))?,
        );
        eprintln!(
            "external triple-alpha fit; validity interval supplied by caller; opacity model requires separate calibration"
        );
    } else if rate_min.is_some() || rate_max.is_some() {
        return Err("--rate-min/--rate-max require --rate-file".into());
    }
    let mut rows = vec![vec![1.0, 0.0]; shells];
    let rho = 1e5;
    let mut sphere = Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy: network
                    .mixture(&rows[0])
                    .map_err(|e| format!("{e:?}"))?
                    .at(rho, 2e8)
                    .map_err(|e| format!("{e:?}"))?
                    .internal_energy_density
            };
            shells
        ],
        spacing: 1e7 / shells as f64,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Outflow,
    };
    if density_contrast > 0.0 {
        let shell_count = sphere.cells.len() as f64;
        for (i, cell) in sphere.cells.iter_mut().enumerate() {
            let a = i as f64 / shell_count;
            let b = (i + 1) as f64 / shell_count;
            let mean_r2 = 0.6
                * (b.powi(4) + b.powi(3) * a + b * b * a * a + b * a.powi(3) + a.powi(4))
                / (b * b + b * a + a * a);
            cell.density = rho * (1.0 + density_contrast * (1.0 - mean_r2));
            cell.energy = network
                .mixture(&rows[i])
                .map_err(|e| format!("{e:?}"))?
                .at(cell.density, 2e8)
                .map_err(|e| format!("{e:?}"))?
                .internal_energy_density;
        }
    }
    if hydrostatic {
        let state = network
            .mixture(&rows[0])
            .map_err(|e| format!("{e:?}"))?
            .at(rho, 2e8)
            .map_err(|e| format!("{e:?}"))?;
        sphere
            .initialize_hydrostatic(
                &rows,
                &network,
                state.gas_pressure + state.radiation_pressure,
            )
            .map_err(|e| format!("{e:?}"))?;
    }
    let mut polytrope_exterior = None;
    if let Some(index) = polytrope_index {
        use physics::astrophysics_star::{Scaling, lane_emden};
        let profile =
            lane_emden(index, 0.001, 20.0, 25000).map_err(|e| format!("polytrope: {e:?}"))?;
        let surface = profile
            .surface
            .ok_or("polytrope surface not reached within xi=20")?;
        let central = network
            .mixture(&rows[0])
            .map_err(|e| format!("{e:?}"))?
            .at(rho, 2e8)
            .map_err(|e| format!("{e:?}"))?;
        let scaling = Scaling::new(
            index,
            rho,
            central.gas_pressure + central.radiation_pressure,
            sphere.g,
        )
        .map_err(|e| format!("{e:?}"))?;
        sphere.spacing =
            scaling.length * surface.xi * polytrope_radius_fraction / sphere.cells.len() as f64;
        sphere
            .initialize_polytrope(&rows, &network, &profile, scaling, 32)
            .map_err(|e| format!("{e:?}"))?;
        if use_polytrope_exterior {
            let inner = sphere.spacing * sphere.cells.len() as f64 / scaling.length;
            let outer = inner + sphere.spacing / scaling.length;
            let avg = profile.shell_average(inner, outer, 32).map_err(|e| {
                format!("polytropic exterior must fit within the Lane-Emden profile: {e:?}")
            })?;
            let fractions = rows.last().unwrap().clone();
            let mixture = network.mixture(&fractions).map_err(|e| format!("{e:?}"))?;
            let density = rho * avg.density_over_central;
            let temperature = mixture
                .temperature_from_pressure(
                    density,
                    scaling.central_pressure * avg.pressure_over_central,
                )
                .map_err(|e| format!("{e:?}"))?;
            polytrope_exterior = Some(CompositionExterior {
                fractions,
                cell: Cell {
                    density,
                    momentum: 0.0,
                    energy: mixture
                        .at(density, temperature)
                        .map_err(|e| format!("{e:?}"))?
                        .internal_energy_density,
                },
            });
        }
        eprintln!(
            "polytrope index {index}; outer radius {} m; fraction {polytrope_radius_fraction} of Lane-Emden surface; central reference rho={rho} kg/m³, T=2e8 K",
            sphere.spacing * sphere.cells.len() as f64
        );
    }
    let exterior = if let Some(density) = exterior_density {
        if !density.is_finite()
            || density <= 0.0
            || !exterior_temperature.is_finite()
            || exterior_temperature <= 0.0
            || !exterior_carbon.is_finite()
            || !(0.0..=1.0).contains(&exterior_carbon)
            || !exterior_velocity.is_finite()
        {
            return Err("exterior requires positive density/temperature, carbon fraction in [0,1], and finite velocity".into());
        }
        let fractions = vec![1.0 - exterior_carbon, exterior_carbon];
        let state = network
            .mixture(&fractions)
            .map_err(|e| format!("{e:?}"))?
            .at(density, exterior_temperature)
            .map_err(|e| format!("{e:?}"))?;
        Some(CompositionExterior {
            fractions,
            cell: Cell {
                density,
                momentum: density * exterior_velocity,
                energy: state.internal_energy_density + 0.5 * density * exterior_velocity.powi(2),
            },
        })
    } else {
        if exterior_options {
            return Err("exterior options require --exterior-density".into());
        }
        polytrope_exterior
    };
    let reference = if balanced {
        let exterior = exterior
            .as_ref()
            .ok_or("--balanced requires an explicit exterior or --polytrope-exterior")?;
        let mixture = network
            .mixture(&exterior.fractions)
            .map_err(|e| format!("{e:?}"))?;
        let internal =
            exterior.cell.energy - 0.5 * exterior.cell.momentum.powi(2) / exterior.cell.density;
        let temperature = mixture
            .temperature(exterior.cell.density, internal)
            .map_err(|e| format!("{e:?}"))?;
        let state = mixture
            .at(exterior.cell.density, temperature)
            .map_err(|e| format!("{e:?}"))?;
        Some(
            sphere
                .initialize_balanced_exterior(
                    &rows,
                    &network,
                    state.gas_pressure + state.radiation_pressure,
                    exterior,
                )
                .map_err(|e| format!("balanced initialization: {e:?}"))?,
        )
    } else {
        None
    };
    if balanced {
        eprintln!(
            "balanced transport enabled; hydrostatic_residual columns describe the standard pressure stencil, not the balanced momentum operator"
        );
    }
    let initial = sphere
        .reactive_energy(&rows, &network)
        .map_err(|e| format!("{e:?}"))?;
    let settings = ReactiveSettings {
        hydro_max_step: dt / 4.0,
        hydro_steps: 10000,
        burn: Budget {
            max_step: dt / 10.0,
            steps: 10000,
            fit_evaluations: 10000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 1e-16,
            ambient: 0.0,
            rays_per_annulus: 4,
            max_segments: 1000000,
            max_step: dt / 4.0,
            max_steps: 1000,
        },
    };
    let table = if let Some(path) = opacity_file {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Some(
            physics::astrophysics_opacity::Table::from_csv(
                physics::astrophysics_opacity::Kind::GreyAbsorption,
                &text,
                1000000,
            )
            .map_err(|e| format!("{e:?}"))?,
        )
    } else if tabulated {
        let densities = vec![1e4_f64, 1e5, 1e6];
        let temperatures = vec![1e8_f64, 2e8, 4e8];
        let values = densities
            .iter()
            .flat_map(|rho| {
                temperatures
                    .iter()
                    .map(move |t| 1e-16 * (rho / 1e5) * (t / 2e8).powf(-3.5))
            })
            .collect();
        Some(
            physics::astrophysics_opacity::Table::new(
                physics::astrophysics_opacity::Kind::GreyAbsorption,
                densities,
                temperatures,
                values,
            )
            .map_err(|e| format!("{e:?}"))?,
        )
    } else {
        None
    };
    let mut time = 0.0;
    let mut escaping = 0.0;
    let mut photons = 0.0;
    let mut neutrinos = 0.0;
    let mut escaped_helium = 0.0;
    let mut escaped_carbon = 0.0;
    println!(
        "time_s,central_density_kg_m3,central_temperature_K,central_carbon_fraction,escaped_photons_J,escaped_neutrinos_J,relative_energy_error,radial_tau_two_thirds_radius_m,net_luminosity_W,photosphere_effective_temperature_K,max_hydrostatic_residual_m_s2,escaped_helium_kg,escaped_carbon_kg"
    );
    loop {
        let c = sphere.cells[0];
        let temperature = network
            .mixture(&rows[0])
            .map_err(|e| format!("{e:?}"))?
            .temperature(c.density, c.energy - 0.5 * c.momentum.powi(2) / c.density)
            .map_err(|e| format!("{e:?}"))?;
        let energy = sphere
            .reactive_energy(&rows, &network)
            .map_err(|e| format!("{e:?}"))?;
        let radius = sphere
            .composition_optical_depth_radius(
                &rows,
                &network,
                settings.radiation.opacity,
                table.as_ref(),
                2.0 / 3.0,
            )
            .map_err(|e| format!("{e:?}"))?;
        let luminosity = sphere
            .composition_radiation_rates(settings.radiation, &rows, &network, table.as_ref())
            .map_err(|e| format!("{e:?}"))?
            .luminosity;
        let effective = radius
            .map(|r| {
                physics::astrophysics_spherical_radiation::effective_temperature(luminosity, r)
            })
            .transpose()
            .map_err(|e| format!("{e:?}"))?
            .map_or_else(String::new, |t| t.to_string());
        let radius = radius.map_or_else(String::new, |r| r.to_string());
        let hydrostatic_residual = exterior
            .as_ref()
            .map_or_else(
                || sphere.hydrostatic_residual(&rows, &network),
                |e| sphere.hydrostatic_residual_exterior(&rows, &network, e),
            )
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .map(f64::abs)
            .fold(0.0, f64::max);
        println!(
            "{time},{},{temperature},{},{photons},{neutrinos},{},{radius},{luminosity},{effective},{hydrostatic_residual},{escaped_helium},{escaped_carbon}",
            c.density,
            rows[0][1],
            (energy + escaping - initial) / initial.abs()
        );
        if time >= duration {
            break;
        }
        let h = dt.min(duration - time);
        let report = if let Some(reference) = &reference {
            sphere.step_reactive_radiating_balanced(
                &mut rows,
                &network,
                h,
                settings,
                reference,
                exterior.as_ref(),
                table.as_ref(),
            )
        } else if let Some(exterior) = &exterior {
            sphere.step_reactive_radiating_exterior(
                &mut rows,
                &network,
                h,
                settings,
                exterior,
                table.as_ref(),
            )
        } else if let Some(table) = &table {
            sphere.step_reactive_tabulated(&mut rows, &network, h, settings, table)
        } else {
            sphere.step_reactive_radiating(&mut rows, &network, h, settings)
        }
        .map_err(|e| format!("t={time}: {e:?}"))?;
        escaped_helium += report.dynamics.escaped_species[0];
        escaped_carbon += report.dynamics.escaped_species[1];
        photons += report.radiation.escaped_energy;
        neutrinos += report.dynamics.escaped_neutrinos;
        escaping += report.radiation.escaped_energy
            + report.dynamics.escaped_neutrinos
            + report.dynamics.escaped_energy
            + report.dynamics.escaped_binding;
        time += h;
    }
    let error = (sphere
        .reactive_energy(&rows, &network)
        .map_err(|e| format!("{e:?}"))?
        + escaping
        - initial)
        / initial.abs();
    if let Some(path) = profile_file {
        use std::fmt::Write;
        let field = sphere.field().map_err(|e| format!("{e:?}"))?;
        let residual = exterior
            .as_ref()
            .map_or_else(
                || sphere.hydrostatic_residual(&rows, &network),
                |e| sphere.hydrostatic_residual_exterior(&rows, &network, e),
            )
            .map_err(|e| format!("{e:?}"))?;
        let mut profile = String::from(
            "time_s,inner_radius_m,outer_radius_m,density_kg_m3,temperature_K,gas_pressure_Pa,radiation_pressure_Pa,radial_velocity_m_s,helium_fraction,carbon_fraction,enclosed_mass_kg,gravity_m_s2,potential_J_kg,hydrostatic_residual_m_s2\n",
        );
        let mut a = 0.0_f64;
        let mut enclosed = 0.0;
        for (i, (cell, row)) in sphere.cells.iter().zip(&rows).enumerate() {
            let b = a + sphere.spacing;
            // Stable volume expression in shell thickness.
            let volume =
                4.0 * std::f64::consts::PI / 3.0 * sphere.spacing * (a * a + a * b + b * b);
            enclosed += cell.density * volume;
            let mix = network.mixture(row).map_err(|e| format!("{e:?}"))?;
            let velocity = cell.momentum / cell.density;
            let temperature = mix
                .temperature(
                    cell.density,
                    cell.energy - 0.5 * cell.density * velocity * velocity,
                )
                .map_err(|e| format!("{e:?}"))?;
            let state = mix
                .at(cell.density, temperature)
                .map_err(|e| format!("{e:?}"))?;
            writeln!(
                profile,
                "{time},{a},{b},{},{temperature},{},{},{velocity},{},{},{enclosed},{},{},{}",
                cell.density,
                state.gas_pressure,
                state.radiation_pressure,
                row[0],
                row[1],
                field.acceleration[i],
                field.potential[i],
                residual[i]
            )
            .map_err(|e| e.to_string())?;
            a = b;
        }
        std::fs::write(&path, profile).map_err(|e| format!("profile {path}: {e}"))?;
        eprintln!("final shell profile: {path}");
    }
    eprintln!("coupled numerical demonstration; duration {time} s; relative energy error {error}");
    Ok(())
}
