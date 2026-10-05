//! Controlled CPU step measurement with full energy/path admission enabled.
use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, SupportTarget, TetraMesh,
    ViscoelasticOgden,
};
fn main() -> Result<(), &'static str> {
    for repeat in 0..3 {
        let mesh = TetraMesh::ellipsoid([0.; 3], [0.3, 0.36, 0.3], 1)?;
        let mut body = Body::new(
            mesh.points.clone(),
            (0..mesh.points.len()).map(|i| i == 3 || i == 6).collect(),
            mesh.cells
                .iter()
                .map(|&cell| {
                    (
                        cell,
                        Material {
                            shear_pa: 5000.,
                            bulk_pa: 1e6,
                            fibers: vec![],
                        },
                    )
                })
                .collect(),
        )?;
        let law = ViscoelasticOgden::new(
            vec![OgdenTerm {
                shear_pa: 5000.,
                exponent: 2.,
            }],
            1e6,
            vec![MaxwellBranch {
                shear_pa: 10000.,
                relaxation_seconds: 0.2,
            }],
        )?;
        body.set_viscoelastic_ogden_batch(
            &(0..mesh.cells.len())
                .map(|i| (i, law.clone()))
                .collect::<Vec<_>>(),
        )?;
        let mut body = InertialBody::new_viscoelastic_with_supports(
            body,
            &vec![1000.; mesh.cells.len()],
            vec![[0.; 3]; mesh.points.len()],
        )?;
        body.set_uniform_acceleration([0., -9.81, 0.])?;
        let mut heat = 0.;
        let mut work = 0.;
        let mut defect = 0.;
        let start = std::time::Instant::now();
        for step in 1..=1000 {
            let offset = 0.002 * (f64::from(step) / 3840. * 10.).sin();
            let targets: Vec<_> = [3, 6]
                .into_iter()
                .map(|node| SupportTarget {
                    node,
                    position_m: [
                        mesh.points[node][0] + offset,
                        mesh.points[node][1],
                        mesh.points[node][2],
                    ],
                })
                .collect();
            let receipt = body.step_viscoelastic(Some(&targets), 1. / 3840., 1e-6)?;
            heat += receipt.viscous_heat_j;
            work += receipt.support.support_work_j;
            defect += receipt.total_energy_defect_j;
        }
        let elapsed = start.elapsed().as_secs_f64();
        let diagnostics = body.diagnostics()?;
        println!(
            "repeat={repeat} seconds={elapsed:.9} position={:?} velocity={:?} potential={:.17} kinetic={:.17} heat={heat:.17} work={work:.17} defect={defect:.17}",
            body.body().positions()[0],
            body.velocities()[0],
            diagnostics.potential_j,
            diagnostics.kinetic_j
        );
    }
    Ok(())
}
