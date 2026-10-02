//! Synthetic finite-compartment exchange with deforming anatomical lung tissue.
//! Port and clamp are numerical, not anatomically registered vascular attachments.
use physics::biomechanics::*;
use physics::circulation::{Circulation, Compartment, Vessel};
use std::io::Write;
fn main() {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/anatomy/hra-female/tetrahedra/right-lung-middle-envelope.vxtet")
        });
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let seconds: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--seconds="))
        .unwrap_or("0.0001")
        .parse()
        .expect("invalid step duration");
    let steps: usize = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--steps="))
        .unwrap_or("1")
        .parse()
        .expect("invalid step count");
    assert!(
        seconds.is_finite() && seconds > 0. && steps > 0 && steps <= 100_000,
        "step duration must be positive and finite; steps must be 1..=100000"
    );
    let output = arguments.iter().find_map(|a| a.strip_prefix("--output="));
    let mesh = TetraMesh::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    let pair = arguments.iter().any(|a| a == "--vascular-pair");
    let vascular = pair || arguments.iter().any(|a| a == "--vascular");
    let port = mesh.boundary[0];
    let centroid = |nodes: &[usize; 3]| -> [f64; 3] {
        std::array::from_fn(|axis| nodes.iter().map(|i| mesh.points[*i][axis]).sum::<f64>() / 3.)
    };
    let first_center = centroid(&port);
    let second_port = *mesh
        .boundary
        .iter()
        .max_by(|a, b| {
            let distance = |nodes: &[usize; 3]| {
                let center = centroid(nodes);
                (0..3)
                    .map(|i| (center[i] - first_center[i]).powi(2))
                    .sum::<f64>()
            };
            distance(a).total_cmp(&distance(b))
        })
        .unwrap();
    let mut vascular_ports = vec![VascularPorePort {
        nodes: port,
        compartment: 0,
    }];
    if pair {
        vascular_ports.push(VascularPorePort {
            nodes: second_port,
            compartment: 1,
        });
    }
    let interface_resistance: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--interface-resistance="))
        .unwrap_or("0")
        .parse()
        .expect("invalid interface resistance");
    assert!(
        interface_resistance.is_finite() && interface_resistance >= 0.,
        "interface resistance must be finite and nonnegative"
    );
    assert!(
        vascular || interface_resistance == 0.,
        "interface resistance requires vascular mode"
    );
    let reflection: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--protein-reflection="))
        .unwrap_or("0")
        .parse()
        .expect("invalid protein reflection");
    let diffusion: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--protein-diffusion="))
        .unwrap_or("0")
        .parse()
        .expect("invalid protein diffusion");
    assert!(
        reflection.is_finite()
            && (0. ..=1.).contains(&reflection)
            && diffusion.is_finite()
            && diffusion >= 0.,
        "invalid protein membrane coefficients"
    );
    assert!(
        vascular || (reflection == 0. && diffusion == 0.),
        "protein membranes require vascular mode"
    );
    let osmotic_slope: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--osmotic-slope="))
        .unwrap_or("0")
        .parse()
        .expect("invalid osmotic slope");
    assert!(
        osmotic_slope.is_finite() && osmotic_slope >= 0.,
        "invalid osmotic slope"
    );
    assert!(
        vascular || osmotic_slope == 0.,
        "osmosis requires vascular mode"
    );
    let coefficient = |flag: &str| -> f64 {
        arguments
            .iter()
            .find_map(|a| a.strip_prefix(flag))
            .unwrap_or("0")
            .parse()
            .expect("invalid osmotic coefficient")
    };
    let osmotic_law = OsmoticPressureLaw {
        linear: osmotic_slope,
        quadratic: coefficient("--osmotic-quadratic="),
        cubic: coefficient("--osmotic-cubic="),
    };
    osmotic_law.pressure_pa(0.).expect("invalid osmotic law");
    assert!(
        vascular || (osmotic_law.quadratic == 0. && osmotic_law.cubic == 0.),
        "osmosis requires vascular mode"
    );
    let membranes: Vec<_> = vascular_ports
        .iter()
        .map(|p| (p.nodes, reflection, diffusion))
        .collect();
    let resistances: Vec<_> = vascular_ports
        .iter()
        .map(|p| (p.nodes, interface_resistance))
        .collect();
    if vascular {
        println!(
            "vascular_ports={vascular_ports:?} interface_resistance_pa_s_per_m3={interface_resistance:.12e} reflection={reflection:.12e} diffusion_m3_per_s={diffusion:.12e} osmotic_slope_pa_m3_per_kg={osmotic_slope:.12e} osmotic_law={osmotic_law:?}"
        );
    }
    let reference = mesh.points.clone();
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
    let fixed = std::env::args().any(|a| a == "--fixed");
    let pins = mesh
        .points
        .iter()
        .map(|p| fixed || p[1] <= low + 0.05 * (high - low))
        .collect();
    let mut body = mesh
        .into_body(
            pins,
            &Material {
                shear_pa: 5000.,
                bulk_pa: 50_000.,
                fibers: vec![],
            },
        )
        .unwrap();
    let stores = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .map(|s| PoreFluid {
            reference_fluid_volume_m3: 0.5 * s.reference_volume_m3,
            fluid_volume_m3: 0.5 * s.reference_volume_m3 + 20. * s.reference_volume_m3 / 100_000.,
            storage_m3_per_pa: s.reference_volume_m3 / 100_000.,
            biot_coefficient: 0.8,
        })
        .collect();
    body.set_cell_pore_fluids(stores).unwrap();
    let count = body.elements().len();
    let k = vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]; count];
    let mut protein: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut reservoir = PoreReservoir {
        reference_volume_m3: 1e-9 - 100. * 1e-14,
        reference_pressure_pa: 0.,
        compliance_m3_per_pa: 1e-14,
        fluid_volume_m3: 1e-9,
        protein_kg: 2e-8,
    };
    let solid_tolerance: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--solid-tolerance="))
        .unwrap_or(if vascular {
            "0.0000000001"
        } else {
            "0.0000001"
        })
        .parse()
        .expect("invalid solid tolerance");
    let tissue_pressure_tolerance: f64 = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--tissue-pressure-tolerance="))
        .unwrap_or(if vascular { "0.000000001" } else { "0.000001" })
        .parse()
        .expect("invalid tissue pressure tolerance");
    let trace = arguments.iter().any(|a| a == "--trace");
    let vascular_iterations: usize = arguments
        .iter()
        .find_map(|a| a.strip_prefix("--vascular-iterations="))
        .unwrap_or("200")
        .parse()
        .expect("invalid vascular iteration count");
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 1e-9 - 100. * 1e-14,
                initial_volume_m3: 1e-9,
                initial_elastance_pa_per_m3: 1e14,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 1e-9 - if pair { 0. } else { 20. * 1e-14 },
                initial_volume_m3: 1e-9,
                initial_elastance_pa_per_m3: 1e14,
                initial_external_pressure_pa: 0.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1e12,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut blood_protein = vec![2e-8, 1e-8];
    let initial_fluid = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + if vascular {
            blood.total_volume()
        } else {
            reservoir.fluid_volume_m3
        };
    let initial_protein = protein.iter().sum::<f64>()
        + if vascular {
            blood_protein.iter().sum()
        } else {
            reservoir.protein_kg
        };
    for step in 1..=steps {
        let report = if vascular {
            let coupled = body
                .implicit_vascular_pore_ports_step_with_osmotic_law_and_observer(
                    &mut protein,
                    &mut blood,
                    &mut blood_protein,
                    &vascular_ports,
                    &resistances,
                    &membranes,
                    osmotic_law,
                    &k,
                    0.001,
                    seconds,
                    &[1e14; 2],
                    &[0.; 2],
                    VascularPoreConfig {
                        iterations: vascular_iterations,
                        relaxation: 0.5,
                        pressure_tolerance_pa: 1e-6,
                        concentration_tolerance_kg_per_m3: 1e-7,
                        tissue: ImplicitPoreConfig {
                            outer_iterations: 200,
                            pressure_tolerance_pa: tissue_pressure_tolerance,
                            relaxation: 0.5,
                            solid_iterations: 32000,
                            solid_tolerance_n: solid_tolerance,
                        },
                        circulation_iterations: 32,
                        circulation_tolerance_m3: 1e-22,
                    },
                    |iteration, pressure, concentration| {
                        if trace { println!("trial={iteration} pressure_residual_pa={pressure:.12e} concentration_residual_kg_per_m3={concentration:.12e}"); }
                    },
                )
                .unwrap();
            println!(
                "vascular_iterations={} vascular_pressure_residual_pa={:.12e} concentration_residual_kg_per_m3={:.12e} vessel_flow_m3_per_s={:.12e}",
                coupled.iterations,
                coupled.pressure_residual_pa,
                coupled.concentration_residual_kg_per_m3,
                blood.flows()[0]
            );
            let boundaries: Vec<_> = vascular_ports
                .iter()
                .map(|p| (p.nodes, blood.pressures()[p.compartment]))
                .collect();
            let model = body
                .deformed_darcy_with_boundaries(&k, 0.001, &boundaries)
                .unwrap();
            for port in &vascular_ports {
                let mut key = port.nodes;
                key.sort_unstable();
                let (face, q) = model
                    .faces()
                    .iter()
                    .zip(&coupled.tissue.flow.face_flows_m3_per_s)
                    .find(|(face, _)| {
                        let mut nodes = face.nodes;
                        nodes.sort_unstable();
                        nodes == key
                    })
                    .unwrap();
                let donor_concentration = if *q < 0. {
                    blood_protein[port.compartment] / blood.volumes()[port.compartment]
                } else {
                    protein[face.owner] / body.cell_pore_fluids()[face.owner].fluid_volume_m3
                };
                println!(
                    "step={step} compartment={} port={:?} tissue_outflow_m3_per_s={q:.12e} protein_to_blood_kg_per_s={:.12e}",
                    port.compartment,
                    port.nodes,
                    (1. - reflection) * q * donor_concentration
                        + diffusion
                            * (protein[face.owner]
                                / body.cell_pore_fluids()[face.owner].fluid_volume_m3
                                - blood_protein[port.compartment]
                                    / blood.volumes()[port.compartment])
                );
            }
            coupled.tissue
        } else {
            body.implicit_cell_pore_reservoir_step(
                &mut protein,
                &mut reservoir,
                &[port],
                &k,
                0.001,
                seconds,
                ImplicitPoreConfig {
                    outer_iterations: 200,
                    pressure_tolerance_pa: 1e-6,
                    relaxation: 0.5,
                    solid_iterations: 32_000,
                    solid_tolerance_n: solid_tolerance,
                },
            )
            .unwrap()
        };
        let fluid_drift = body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum::<f64>()
            + if vascular {
                blood.total_volume()
            } else {
                reservoir.fluid_volume_m3
            }
            - initial_fluid;
        let protein_drift = protein.iter().sum::<f64>()
            + if vascular {
                blood_protein.iter().sum()
            } else {
                reservoir.protein_kg
            }
            - initial_protein;
        let displacement = body
            .positions()
            .iter()
            .zip(&reference)
            .map(|(a, b)| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt())
            .fold(0., f64::max);
        assert!(
            fluid_drift.abs() < 1e-10 * initial_fluid
                && protein_drift.abs() < 1e-12 * initial_protein
        );
        println!(
            "step={step} time_s={:.12e} cells={count} fixed={fixed} port={port:?} outer_iterations={} force_residual_n={:.12e} pressure_residual_pa={:.12e} port_pressure_pa={:.12e} fluid_drift_m3={fluid_drift:.12e} protein_drift_kg={protein_drift:.12e} max_displacement_m={displacement:.12e}",
            step as f64 * seconds,
            report.iterations,
            report.solid.residual_n,
            report.pressure_residual_pa,
            if vascular {
                blood.pressures()[0]
            } else {
                reservoir.pressure_pa().unwrap()
            }
        );
    }
    if let Some(path) = output {
        let mut file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        writeln!(file, "cell,pressure_pa,fluid_volume_m3,protein_kg").unwrap();
        let stresses = body.stresses_at(body.positions()).unwrap();
        for (i, ((fluid, stress), mass)) in body
            .cell_pore_fluids()
            .iter()
            .zip(&stresses)
            .zip(&protein)
            .enumerate()
        {
            let pressure = (fluid.fluid_volume_m3
                - fluid.reference_fluid_volume_m3
                - fluid.biot_coefficient * stress.reference_volume_m3 * (stress.volume_ratio - 1.))
                / fluid.storage_m3_per_pa;
            writeln!(
                file,
                "{i},{pressure:.17e},{:.17e},{mass:.17e}",
                fluid.fluid_volume_m3
            )
            .unwrap();
        }
        file.flush().unwrap();
        if vascular {
            let mut file = std::io::BufWriter::new(
                std::fs::File::create(format!("{path}.blood.csv")).unwrap(),
            );
            writeln!(file, "compartment,pressure_pa,fluid_volume_m3,protein_kg").unwrap();
            for (i, ((pressure, volume), mass)) in blood
                .pressures()
                .iter()
                .zip(blood.volumes())
                .zip(&blood_protein)
                .enumerate()
            {
                writeln!(file, "{i},{pressure:.17e},{volume:.17e},{mass:.17e}").unwrap();
            }
            file.flush().unwrap();
        }
    }
}
