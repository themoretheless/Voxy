//! Synthetic two-cell spatial Darcy diffusion with local elastic swelling.
use physics::{biomechanics::*, lymph::*};
fn main() {
    let h: f64 = std::env::args()
        .nth(1)
        .map_or(0.01, |s| s.parse().expect("maximum substep seconds"));
    let x = vec![
        [0., 0., 0.],
        [0.01, 0., 0.],
        [0., 0.01, 0.],
        [0.01 / 3., 0.01 / 3., 0.01],
        [0.01 / 3., 0.01 / 3., -0.01],
    ];
    let m = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let mut body = Body::new(
        x,
        vec![true, true, true, false, false],
        vec![([0, 1, 2, 3], m.clone()), ([0, 1, 2, 4], m)],
    )
    .unwrap();
    let vf = body.reference_volume() / 4.;
    let store = |extra| PoreFluid {
        reference_fluid_volume_m3: vf,
        fluid_volume_m3: vf + extra,
        biot_coefficient: 0.8,
        storage_m3_per_pa: 1e-11,
    };
    body.set_cell_pore_fluids(vec![store(1e-9), store(0.)])
        .unwrap();
    let spaces = body
        .cell_pore_fluids()
        .iter()
        .map(|p| FluidSpace {
            reference_volume_m3: p.reference_fluid_volume_m3,
            initial_volume_m3: p.fluid_volume_m3,
            initial_protein_kg: 10. * p.fluid_volume_m3,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: p.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    let edge = body
        .reference_darcy_interface(0, 1, [1e-12; 2], 0.001)
        .unwrap();
    let mut net = LymphNetwork::new(spaces, vec![edge]).unwrap();
    let v = net.total_volume();
    let mass = net.total_protein();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
    println!(
        "time_s,p0_pa,p1_pa,fluid0_m3,fluid1_m3,tissue0_m3,tissue1_m3,total_fluid_drift_m3,total_protein_drift_kg"
    );
    for i in 1..=100 {
        tissue.step(&mut net, 0.1, h).unwrap();
        let stresses = tissue
            .body()
            .stresses_at(tissue.body().positions())
            .unwrap();
        let volumes = stresses
            .iter()
            .map(|s| s.reference_volume_m3 * s.volume_ratio)
            .collect::<Vec<_>>();
        let p = net.pressures();
        println!(
            "{:.3},{:.9},{:.9},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
            f64::from(i) * 0.1,
            p[0],
            p[1],
            net.volumes()[0],
            net.volumes()[1],
            volumes[0],
            volumes[1],
            net.total_volume() - v,
            net.total_protein() - mass
        );
    }
}
