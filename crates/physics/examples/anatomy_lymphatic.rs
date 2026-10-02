//! Adaptive tissue/lymph exchange on imported anatomy; synthetic attachment ports
//! and material/network parameters, not anatomically registered lymph vessels.
use physics::{biomechanics::*, lymph::*};
use std::{io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/anatomy/hra-female/tetrahedra/left-ovary.vxtet")
        });
    let option = |prefix: &str, default: f64| -> Result<f64, Box<dyn std::error::Error>> {
        Ok(args
            .iter()
            .find_map(|a| a.strip_prefix(prefix))
            .map(str::parse)
            .transpose()?
            .unwrap_or(default))
    };
    let seconds = option("--seconds=", 1e-5)?;
    let max_step = option("--max-step=", seconds)?;
    let relative = option("--relative-tolerance=", 1e-3)?;
    let position_tolerance = option("--position-tolerance=", 1e-8)?;
    let tension = option("--tension=", 0.08)?;
    let external_offset = option("--external-offset=", 0.)?;
    let lymph_resistance = option("--lymph-interface-resistance=", 1e13)?;
    let density = option("--lymph-density=", 1000.)?;
    assert!(lymph_resistance.is_finite() && lymph_resistance > 0.);
    let force_tolerance = option("--solid-tolerance=", 1e-10)?;
    let steps: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--steps="))
        .map(str::parse)
        .transpose()?
        .unwrap_or(1);
    assert!((1..=100_000).contains(&steps));
    let output = args.iter().find_map(|a| a.strip_prefix("--output="));
    let fixed = args.iter().any(|a| a == "--fixed");
    let mesh = TetraMesh::from_bytes(&std::fs::read(&path)?)?;
    let low = mesh
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    let high = mesh
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let pins = mesh
        .points
        .iter()
        .map(|p| fixed || p[1] <= low + 0.05 * (high - low))
        .collect();
    let mut body = mesh.into_body(
        pins,
        &Material {
            shear_pa: 5000.,
            bulk_pa: 50_000.,
            fibers: vec![],
        },
    )?;
    let reference = body.positions().to_vec();
    let stores: Vec<_> = body
        .stresses_at(body.positions())?
        .iter()
        .map(|s| PoreFluid {
            reference_fluid_volume_m3: 0.5 * s.reference_volume_m3,
            fluid_volume_m3: 0.5002 * s.reference_volume_m3,
            storage_m3_per_pa: s.reference_volume_m3 / 100_000.,
            biot_coefficient: 0.8,
        })
        .collect();
    body.set_cell_pore_fluids(stores)?;
    let count = body.elements().len();
    let k = vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]; count];
    let model = body.deformed_darcy(&k, 0.001)?;
    // Ports are explicit cell identities, not an inferred vessel lumen. Choose
    // the first cell and the most distant cell centroid, independent of faces.
    let centroid = |i: usize| -> [f64; 3] {
        std::array::from_fn(|axis| {
            body.elements()[i]
                .nodes
                .iter()
                .map(|n| body.positions()[*n][axis])
                .sum::<f64>()
                / 4.
        })
    };
    let first = centroid(0);
    let drainage = (0..count)
        .max_by(|a, b| {
            let distance = |i| {
                (0..3)
                    .map(|d| (centroid(i)[d] - first[d]).powi(2))
                    .sum::<f64>()
            };
            distance(*a).total_cmp(&distance(*b))
        })
        .unwrap();
    let mut spaces: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| FluidSpace {
            reference_volume_m3: f.reference_fluid_volume_m3,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: f.storage_m3_per_pa,
            initial_volume_m3: f.fluid_volume_m3,
            initial_protein_kg: 10. * f.fluid_volume_m3,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    for (pressure, concentration) in [(100., 20.), (0., 5.)] {
        spaces.push(FluidSpace {
            reference_volume_m3: 1e-9,
            reference_pressure_pa: pressure,
            compliance_m3_per_pa: 1e-14,
            initial_volume_m3: 1e-9,
            initial_protein_kg: concentration * 1e-9,
            oncotic_pa_per_kg_m3: 0.,
        });
    }
    let mut edges: Vec<_> = model
        .faces()
        .iter()
        .map(|f| Exchange {
            from: f.owner,
            to: f
                .neighbor
                .expect("sealed Darcy model contains interior faces"),
            hydraulic_m3_per_pa_s: 0.,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: false,
        })
        .collect();
    let capillary_edge = edges.len();
    edges.push(Exchange {
        from: count,
        to: 0,
        hydraulic_m3_per_pa_s: 1e-13,
        reflection: 0.8,
        protein_permeability_m3_per_s: 1e-15,
        pump_head_pa: 0.,
        valve: false,
    });
    edges.push(Exchange {
        from: drainage,
        to: count + 1,
        hydraulic_m3_per_pa_s: 1. / lymph_resistance,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: true,
    });
    edges.push(Exchange {
        from: count + 1,
        to: count,
        hydraulic_m3_per_pa_s: 1. / lymph_resistance,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: true,
    });
    let mut network = LymphNetwork::new(spaces, edges)?;
    let laws = vec![
        OsmoticPressureLaw {
            linear: 8.,
            quadratic: 0.1,
            cubic: 0.001
        };
        count + 2
    ];
    let mut walls = vec![None; count + 2];
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-9,
        length_m: 0.001,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: external_offset,
        active_tension_n_per_m: tension,
    };
    let attachments = if args.iter().any(|a| a == "--uncoupled-wall") {
        vec![]
    } else {
        vec![LymphaticWallAttachment {
            compartment: count + 1,
            tissue_cell: drainage,
        }]
    };
    println!("lymph_wall_attachments={attachments:?} external_offset_pa={external_offset:.12e}");
    let hydraulic = if args.iter().any(|a| a == "--fixed-resistance") {
        vec![]
    } else {
        [capillary_edge + 1, capillary_edge + 2]
            .map(|edge| LymphaticHydraulicAttachment {
                edge,
                compartment: count + 1,
                segment_length_m: 0.5 * wall.length_m,
                viscosity_pa_s: 0.001,
            })
            .to_vec()
    };
    println!(
        "lymphatic_hydraulic_attachments={hydraulic:?} lymph_interface_resistance_pa_s_per_m3={lymph_resistance:.12e}"
    );
    let mut tissue = CellPoreTissue::new(body, (0..count).collect(), 32_000, force_tolerance)?;
    let totals = (network.total_volume(), network.total_protein());
    println!(
        "mesh={} cells={count} fixed={fixed} capillary_cell=0 drainage_cell={drainage} spaces={} edges={} phase_seconds={seconds:.12e} max_step_seconds={max_step:.12e}",
        path.display(),
        network.volumes().len(),
        network.edges().len()
    );
    let radial_adaptive = args.iter().any(|a| a == "--radial-adaptive");
    let radial_rings: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--radial-rings="))
        .unwrap_or("256")
        .parse()?;
    let radial_velocity_tolerance = option("--radial-velocity-tolerance=", 1e-7)?;
    let mut radial_profiles = Vec::new();
    if radial_adaptive {
        if hydraulic.is_empty() {
            return Err("radial mode requires explicit conduits".into());
        }
        for link in &hydraulic {
            radial_profiles.push(RadialExchange {
                edge: link.edge,
                length_m: link.segment_length_m,
                pipe: profile::RadialPipe::new(
                    wall.radius_m(wall.reference_volume_m3)?,
                    density,
                    link.viscosity_pa_s,
                    radial_rings,
                )?,
            });
        }
        println!(
            "radial_rings={radial_rings} radial_velocity_tolerance_m_per_s={radial_velocity_tolerance:.12e} conduit_radius_mode=fixed_reference"
        );
    }
    let inertial_adaptive = args.iter().any(|a| a == "--inertial-adaptive");
    let inertial = inertial_adaptive || args.iter().any(|a| a == "--inertial");
    let inertial_substeps: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--inertial-substeps="))
        .unwrap_or("1")
        .parse()?;
    if inertial_substeps == 0 || inertial_substeps > 100_000 {
        return Err("invalid inertial substeps".into());
    }
    let mut flow_history = vec![0.; network.edges().len()];
    println!(
        "transport_mode={} inertial_substeps={inertial_substeps}",
        if radial_adaptive {
            "adaptive_radial_profiles"
        } else if inertial_adaptive {
            "adaptive_backward_euler_inertial"
        } else if inertial {
            "backward_euler_inertial"
        } else {
            "adaptive_quasisteady"
        }
    );
    for step in 1..=steps {
        let active = if step % 2 == 1 { tension } else { 0. };
        walls[count + 1] = Some(LymphaticWallLaw {
            active_tension_n_per_m: active,
            ..wall
        });
        let report = if radial_adaptive {
            tissue.step_mixed_darcy_with_radial_profiles_adaptive(
                &mut network,
                &k,
                0.001,
                seconds,
                &laws,
                &walls,
                &attachments,
                &mut radial_profiles,
                AdaptiveTissueExchangeConfig {
                    exchange: AdaptiveExchangeConfig {
                        relative_tolerance: relative,
                        absolute_volume_tolerance_m3: 1e-18,
                        absolute_protein_tolerance_kg: 1e-17,
                        min_step_seconds: 1e-12,
                        max_step_seconds: max_step,
                        max_trials: 10000,
                    },
                    absolute_position_tolerance_m: position_tolerance,
                },
                radial_velocity_tolerance,
                1000,
                1e-20,
                1e-19,
            )?
        } else if inertial_adaptive {
            tissue.step_mixed_darcy_with_inertial_lymphatic_geometry_adaptive(
                &mut network,
                &k,
                0.001,
                seconds,
                &laws,
                &walls,
                &attachments,
                &hydraulic,
                density,
                &mut flow_history,
                AdaptiveTissueExchangeConfig {
                    exchange: AdaptiveExchangeConfig {
                        relative_tolerance: relative,
                        absolute_volume_tolerance_m3: 1e-18,
                        absolute_protein_tolerance_kg: 1e-17,
                        min_step_seconds: 1e-12,
                        max_step_seconds: max_step,
                        max_trials: 10000,
                    },
                    absolute_position_tolerance_m: position_tolerance,
                },
                1e-12,
                1000,
                1e-20,
                1e-19,
            )?
        } else if inertial {
            let h = seconds / inertial_substeps as f64;
            let mut exchange = ExchangeReport {
                substeps: 0,
                transferred_volume_m3: vec![0.; network.edges().len()],
                transferred_protein_kg: vec![0.; network.edges().len()],
                volume_drift_m3: 0.,
                protein_drift_kg: 0.,
            };
            for _ in 0..inertial_substeps {
                let part = tissue.step_mixed_darcy_with_inertial_lymphatic_geometry(
                    &mut network,
                    &k,
                    0.001,
                    h,
                    density,
                    &mut flow_history,
                    1000,
                    1e-20,
                    1e-19,
                    Some(&laws),
                    Some(&walls),
                    &attachments,
                    &hydraulic,
                )?;
                exchange.substeps += part.substeps;
                exchange.volume_drift_m3 += part.volume_drift_m3;
                exchange.protein_drift_kg += part.protein_drift_kg;
                for (sum, value) in exchange
                    .transferred_volume_m3
                    .iter_mut()
                    .zip(part.transferred_volume_m3)
                {
                    *sum += value;
                }
                for (sum, value) in exchange
                    .transferred_protein_kg
                    .iter_mut()
                    .zip(part.transferred_protein_kg)
                {
                    *sum += value;
                }
            }
            AdaptiveExchangeReport {
                exchange,
                accepted_steps: inertial_substeps,
                rejected_steps: 0,
                max_accepted_error_ratio: 0.,
            }
        } else {
            tissue.step_mixed_darcy_with_lymphatic_geometry_adaptive(
                &mut network,
                &k,
                0.001,
                seconds,
                Some(&laws),
                &walls,
                &attachments,
                &hydraulic,
                AdaptiveTissueExchangeConfig {
                    exchange: AdaptiveExchangeConfig {
                        relative_tolerance: relative,
                        absolute_volume_tolerance_m3: 1e-18,
                        absolute_protein_tolerance_kg: 1e-17,
                        min_step_seconds: 1e-12,
                        max_step_seconds: max_step,
                        max_trials: 10_000,
                    },
                    absolute_position_tolerance_m: position_tolerance,
                },
            )?
        };
        let time_error = if inertial && !inertial_adaptive && !radial_adaptive {
            "not_estimated".to_owned()
        } else {
            format!("{:.12e}", report.max_accepted_error_ratio)
        };
        let displacement = tissue
            .body()
            .positions()
            .iter()
            .zip(&reference)
            .map(|(a, b)| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt())
            .fold(0., f64::max);
        assert!((network.total_volume() - totals.0).abs() < 1e-10 * totals.0);
        assert!((network.total_protein() - totals.1).abs() < 1e-12 * totals.1);
        println!(
            "step={step} active_tension_n_per_m={active:.12e} accepted={} rejected={} max_error_ratio={time_error} capillary_transfer_m3={:.12e} lymph_inlet_transfer_m3={:.12e} lymph_return_transfer_m3={:.12e} water_drift_m3={:.12e} protein_drift_kg={:.12e} max_displacement_m={displacement:.12e}",
            report.accepted_steps,
            report.rejected_steps,
            report.exchange.transferred_volume_m3[capillary_edge],
            report.exchange.transferred_volume_m3[capillary_edge + 1],
            report.exchange.transferred_volume_m3[capillary_edge + 2],
            network.total_volume() - totals.0,
            network.total_protein() - totals.1
        );
        for link in &hydraulic {
            let phase_average = report.exchange.transferred_volume_m3[link.edge] / seconds;
            let d = link.diagnostics(
                walls[link.compartment].unwrap(),
                if radial_adaptive {
                    wall.reference_volume_m3
                } else {
                    network.volumes()[link.compartment]
                },
                density,
                phase_average,
                lymph_resistance,
                2. * seconds,
            )?;
            println!(
                "step={step} edge={} diagnostics_flow=phase_average radius_m={:.12e} velocity_m_per_s={:.12e} reynolds={:.12e} womersley={:.12e} inertance_pa_s2_per_m3={:.12e} relaxation_seconds={:.12e} omega_inertance_over_resistance={:.12e}",
                link.edge,
                d.radius_m,
                d.mean_velocity_m_per_s,
                d.reynolds,
                d.womersley,
                d.inertance_pa_s2_per_m3,
                d.relaxation_seconds,
                d.inertial_to_resistive_ratio
            );
        }
    }
    if let Some(path) = output {
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "cell,pressure_pa,fluid_volume_m3,protein_kg")?;
        let pressure = network.pressures();
        for i in 0..count {
            writeln!(
                file,
                "{i},{:.17e},{:.17e},{:.17e}",
                pressure[i],
                network.volumes()[i],
                network.protein_masses()[i]
            )?;
        }
        let mut file = std::fs::File::create(format!("{path}.network.csv"))?;
        writeln!(file, "compartment,pressure_pa,fluid_volume_m3,protein_kg")?;
        for i in count..count + 2 {
            writeln!(
                file,
                "{i},{:.17e},{:.17e},{:.17e}",
                pressure[i],
                network.volumes()[i],
                network.protein_masses()[i]
            )?;
        }
        let mut file = std::fs::File::create(format!("{path}.geometry.csv"))?;
        writeln!(
            file,
            "node,x_m,y_m,z_m,reference_x_m,reference_y_m,reference_z_m"
        )?;
        for (i, (p, r)) in tissue.body().positions().iter().zip(reference).enumerate() {
            writeln!(
                file,
                "{i},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
                p[0], p[1], p[2], r[0], r[1], r[2]
            )?;
        }
    }
    Ok(())
}
