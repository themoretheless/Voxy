//! Synthetic porous tetrahedron coupled to a closed fluid/protein loop.
use physics::{biomechanics::*, lymph::*};
fn main() {
    let h: f64 = std::env::args()
        .nth(1)
        .map_or(0.02, |s| s.parse().expect("maximum step seconds"));
    let mut body = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, true, true, false],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 1000.,
                bulk_pa: 10_000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let fluid = body.reference_volume() * 0.5;
    body.set_pore_fluid(PoreFluid {
        reference_fluid_volume_m3: fluid,
        fluid_volume_m3: fluid,
        biot_coefficient: 0.8,
        storage_m3_per_pa: 1e-11,
    })
    .unwrap();
    let space = |v, p| FluidSpace {
        reference_volume_m3: v,
        initial_volume_m3: v,
        initial_protein_kg: v * 10.,
        reference_pressure_pa: p,
        compliance_m3_per_pa: 1e-11,
        oncotic_pa_per_kg_m3: 0.,
    };
    let edge = |from, to, head, valve| Exchange {
        from,
        to,
        hydraulic_m3_per_pa_s: 1e-12,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: head,
        valve,
    };
    let mut net = LymphNetwork::new(
        vec![space(1e-6, 100.), space(fluid, 0.), space(1e-7, -20.)],
        vec![
            edge(0, 1, 0., false),
            edge(1, 2, 0., true),
            edge(2, 0, 150., true),
        ],
    )
    .unwrap();
    let v = net.total_volume();
    let m = net.total_protein();
    let mut tissue = PoreTissue::new(body, 1, 4000, 1e-9).unwrap();
    println!(
        "time_s,fluid_volume_m3,tissue_volume_m3,pore_pressure_pa,total_volume_drift_m3,total_protein_drift_kg"
    );
    for i in 1..=100 {
        tissue.step(&mut net, 0.2, h).unwrap();
        println!(
            "{:.3},{:.12e},{:.12e},{:.9},{:.12e},{:.12e}",
            i as f64 * 0.2,
            net.volumes()[1],
            tissue.body().volume_at(tissue.body().positions()).unwrap(),
            net.pressures()[1],
            net.total_volume() - v,
            net.total_protein() - m
        );
    }
}
