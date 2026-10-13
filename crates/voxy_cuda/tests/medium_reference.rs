//! Plan step B4: the CUDA medium transport kernel against the CPU reference.
//! Runs only with the `cuda` feature on a host with an NVIDIA device.
#![cfg(feature = "cuda")]
use physics::liquid::{FiniteDropletGasGrid, VaporCell};
use physics::medium::{
    MediumSpecies, MediumSpeciesProperties, MediumStepControl, ParticipatingMedium,
};
use voxy_cuda::{CudaCompute, pack_medium_job, unpack_medium_result};

#[test]
fn cuda_medium_transport_matches_the_cpu_reference() {
    let compute = CudaCompute::new(0, 64 * 1024 * 1024).expect("CUDA device required");
    let n = [16usize; 3];
    let h = 0.1;
    let volume = h * h * h;
    let cells = (0..n[0] * n[1] * n[2])
        .map(|i| VaporCell {
            mass: 1.2 * volume,
            volume,
            temperature: 300.,
            velocity: [0.2, -0.1 * ((i % 16) as f64 * 0.3).cos(), 0.15],
            specific_heat_cv: 718.,
        })
        .collect();
    let species = MediumSpeciesProperties {
        kind: MediumSpecies::Dust,
        mass_extinction_m2_kg: 1.,
        albedo: 0.5,
        settling_velocity_m_s: 0.3,
        diffusivity_m2_s: 2e-4,
        buoyancy_per_kg_m3: 0.,
    };
    let mut medium = ParticipatingMedium::new(
        FiniteDropletGasGrid::new([0.; 3], [h; 3], n, cells).unwrap(),
        vec![species],
    )
    .unwrap();
    for i in 0..5 {
        medium
            .deposit(0, [0.45 + 0.1 * i as f64, 0.95, 0.55], 0.5)
            .unwrap();
        medium
            .deposit(0, [0.45 + 0.1 * i as f64, 0.05, 0.55], 0.2)
            .unwrap();
    }
    let velocity: Vec<[f32; 3]> = medium
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
                ..MediumStepControl::default()
            },
        )
        .unwrap();
    let words = pack_medium_job(
        n,
        [h as f32; 3],
        [0., 1., 0.],
        report.substeps,
        dt as f32,
        species.settling_velocity_m_s as f32,
        species.diffusivity_m2_s as f32,
        &velocity,
        &mass,
    )
    .unwrap();
    let output = compute.medium_transport(&words).unwrap();
    let (gpu, settled) = unpack_medium_result(&output).unwrap();
    let total: f64 = medium.mass_kg(0).iter().sum();
    let max_difference = gpu
        .iter()
        .zip(medium.mass_kg(0))
        .map(|(g, c)| (f64::from(*g) - c).abs())
        .fold(0., f64::max);
    assert!(max_difference <= 1e-4 * total, "{max_difference}");
    assert!((f64::from(settled) - report.settled_kg[0]).abs() <= 1e-4 * total);
}
