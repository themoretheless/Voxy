//! Diagnostic finite-inventory uptake weakening a preloaded cohesive interface.
use physics::{
    biomechanics::Material as Elastic,
    moisture::{
        Body, Calibration, Cell, CohesiveCalibration, CohesiveProperties, Properties, WaterSupply,
    },
    plasticity::{
        Material,
        mesh::{FiniteQuadraticDynamics, QuadraticBody},
    },
};
use voxy_render::SceneMesh;
#[derive(Clone, Debug)]
pub(crate) struct WetFemPreview {
    dynamics: FiniteQuadraticDynamics,
    water: Body,
    sources: [WaterSupply; 2],
    bulk: Calibration,
    bond: CohesiveCalibration,
    initial_mass: f64,
}
impl WetFemPreview {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dry = CohesiveProperties {
            stiffness_pa_m: 1e6,
            closure_pa_m: 1e7,
            peak_pa: 1000.,
            fracture_j_m2: 10.,
        };
        let bond = CohesiveCalibration::new(
            dry,
            CohesiveProperties {
                peak_pa: 500.,
                fracture_j_m2: 2.5,
                ..dry
            },
        )?;
        let mut body = QuadraticBody::from_linear_with_cohesive_faces(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0., 0., -1.],
            ],
            vec![
                ([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.)?),
                ([0, 2, 1, 4], Material::new(1e5, 0.3, 1e9, 0.)?),
            ],
            bond.at(0.)?,
        )?
        .body;
        let n = body.positions().len();
        let prescribed: Vec<_> = (0..n)
            .map(|i| [Some(0.), Some(0.), Some(if i < 10 { 0.0125 } else { 0. })])
            .collect();
        if !body
            .equilibrate(&vec![[0.; 3]; n], &prescribed, 4, 1e-7)?
            .converged
        {
            return Err("wet FEM preload failed".into());
        }
        let dynamics = FiniteQuadraticDynamics::new(
            body,
            vec![Elastic::from_young_poisson(1e5, 0.3)?; 2],
            &[1000.; 2],
            vec![[0.; 3]; n],
            &vec![false; n],
        )?;
        let initial_mass = dynamics.energy()?.mass_kg;
        let dry_bulk = Properties {
            young_pa: 1e5,
            poisson: 0.3,
            yield_pa: 1e9,
            hardening_pa: 0.,
            hardness_pa: 1e8,
            wear_coefficient: 1e-3,
        };
        Ok(Self {
            dynamics,
            initial_mass,
            bond,
            bulk: Calibration::new(
                dry_bulk,
                Properties {
                    young_pa: 5e4,
                    ..dry_bulk
                },
            )?,
            water: Body::new(
                vec![
                    Cell {
                        capacity_kg: 0.1,
                        water_kg: 0.
                    };
                    2
                ],
                vec![],
            )?,
            sources: [
                WaterSupply {
                    cell: 0,
                    water_kg: 0.1,
                    conductance_kg_s: 100.,
                },
                WaterSupply {
                    cell: 1,
                    water_kg: 0.1,
                    conductance_kg_s: 100.,
                },
            ],
        })
    }
    pub(crate) fn wet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut next = self.clone();
        let n = next.dynamics.velocities().len();
        let (transfer, bulk, fracture) = next.dynamics.advance_moisture_supplies_with_cohesion(
            1.,
            &mut next.water,
            &mut next.sources,
            &[1000. / 6.; 2],
            &[next.bulk; 2],
            &vec![[0.; 3]; n],
            &[next.bond],
            &[0.5],
        )?;
        let retained: f64 = next.water.cells().iter().map(|c| c.water_kg).sum();
        let supplied: f64 = next.sources.iter().map(|s| s.water_kg).sum();
        if (retained + supplied - 0.2).abs() > 1e-12
            || (next.dynamics.energy()?.mass_kg - next.initial_mass - retained).abs() > 1e-10
            || bulk.energy_defect_j.abs() > 1e-10
            || transfer.mass_defect_kg.abs() > 1e-12
            || fracture.fragments_before != 1
            || fracture.fragments_after != 2
        {
            return Err("wet FEM water/material balance failed".into());
        }
        println!(
            "WET FEM: retained_water_kg={retained}, supply_water_kg={supplied}, parameter_work_j={}",
            bulk.elastic_parameter_work_j + fracture.total_parameter_work_j
        );
        *self = next;
        Ok(())
    }
    pub(crate) fn topology(&self) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        Ok((
            self.dynamics
                .body()
                .exposed_faces_at(self.dynamics.positions())?
                .len(),
            self.dynamics.fragments()?.len(),
        ))
    }
    pub(crate) fn mesh(&self) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        crate::fem_surface::fem_surface_scene_mesh(
            self.dynamics.body(),
            2,
            1000,
            [0.35, 0.35, 0.],
            1.,
            |component, normal| {
                let shade = (0.35 + 0.65 * (0.6 * normal[0] + 0.8 * normal[1]).max(0.)) as f32;
                let base = if component == 0 {
                    [0.9, 0.5, 0.08]
                } else {
                    [0.08, 0.5, 0.95]
                };
                [base[0] * shade, base[1] * shade, base[2] * shade, 1.]
            },
        )
    }
}
