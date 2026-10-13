//! Plan step B3: the wgpu medium transport kernel against the CPU reference
//! (`physics::medium`) for advection, diffusion and settling together.
//! Needs a real adapter.
use physics::liquid::{FiniteDropletGasGrid, VaporCell};
use physics::medium::{
    MediumSpecies, MediumSpeciesProperties, MediumStepControl, ParticipatingMedium,
};
use voxy_gpu::{GpuMediumProgram, MediumTransportJob};

fn gas(n: [usize; 3], h: f64) -> FiniteDropletGasGrid {
    let volume = h * h * h;
    let cells = (0..n[0] * n[1] * n[2])
        .map(|i| {
            let ijk = [i % n[0], (i / n[0]) % n[1], i / (n[0] * n[1])];
            VaporCell {
                mass: 1.2 * volume,
                volume,
                temperature: 300.,
                velocity: [
                    0.2 + 0.05 * (ijk[1] as f64).sin(),
                    -0.1 * (ijk[0] as f64 * 0.3).cos(),
                    0.15,
                ],
                specific_heat_cv: 718.,
            }
        })
        .collect();
    FiniteDropletGasGrid::new([0.; 3], [h; 3], n, cells).unwrap()
}

#[test]
fn wgpu_medium_transport_matches_the_cpu_reference() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("GPU required for the medium reference check");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(GpuMediumProgram::new(&device)).unwrap();

    let n = [16usize; 3];
    let h = 0.1;
    let species = MediumSpeciesProperties {
        kind: MediumSpecies::Dust,
        mass_extinction_m2_kg: 1.,
        albedo: 0.5,
        settling_velocity_m_s: 0.3,
        diffusivity_m2_s: 2e-4,
        buoyancy_per_kg_m3: 0.,
    };
    let mut medium = ParticipatingMedium::new(gas(n, h), vec![species]).unwrap();
    for i in 0..5 {
        medium
            .deposit(0, [0.45 + 0.1 * i as f64, 0.95, 0.55], 0.4 + 0.1 * i as f64)
            .unwrap();
        // A second row next to the floor exercises settling outflow.
        medium
            .deposit(0, [0.45 + 0.1 * i as f64, 0.05, 0.55], 0.2)
            .unwrap();
    }
    let gas_velocity: Vec<[f32; 3]> = medium
        .gas()
        .cells()
        .iter()
        .map(|c| c.velocity.map(|v| v as f32))
        .collect();
    let mass: Vec<f32> = medium.mass_kg(0).iter().map(|m| *m as f32).collect();
    let dt = 0.5;
    let report = medium
        .advance(
            dt,
            MediumStepControl {
                advance_gas: false,
                gravity: [0., -9.81, 0.],
                ..MediumStepControl::default()
            },
        )
        .unwrap();
    assert!(report.substeps >= 2, "{}", report.substeps);
    let result = program
        .transport(
            &queue,
            MediumTransportJob {
                shape: n,
                spacing: [h; 3],
                up: [0., 1., 0.],
                dt,
                substeps: report.substeps,
            },
            &species,
            &gas_velocity,
            &mass,
        )
        .unwrap();
    let reference = medium.mass_kg(0);
    let total: f64 = reference.iter().sum();
    let mut max_difference = 0.0f64;
    for (gpu, cpu) in result.mass_kg.iter().zip(reference) {
        max_difference = max_difference.max((f64::from(*gpu) - cpu).abs());
    }
    let gpu_total: f64 = result.mass_kg.iter().map(|m| f64::from(*m)).sum();
    println!(
        "medium: max cell difference {max_difference:e}, totals cpu {total:e} gpu {gpu_total:e}, settled cpu {:e} gpu {:e}",
        report.settled_kg[0], result.settled_kg
    );
    assert!(
        max_difference <= 1e-4 * total,
        "max difference {max_difference}"
    );
    assert!((gpu_total - total).abs() <= 1e-4 * total);
    assert!(
        report.settled_kg[0] > 0.1,
        "settling must drain mass through the floor"
    );
    assert!((f64::from(result.settled_kg) - report.settled_kg[0]).abs() <= 1e-4 * total);
}
