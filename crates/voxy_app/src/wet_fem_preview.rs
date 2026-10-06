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
    thermal: Option<(
        physics::moisture::ThermalVapor,
        physics::moisture::MaterialThermalStore,
    )>,
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
            thermal: None,
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
    /// Diagnostic heat-only weakening through the production thermal/motion transaction.
    pub(crate) fn heat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use physics::{
            liquid::SaturationCurve,
            moisture::{
                MaterialThermalStore, ThermalCalibration, ThermalCohesiveCalibration, ThermalVapor,
            },
            plasticity::mesh::QuadraticAdvanceLimits,
        };
        let mut next = self.clone();
        let curve = SaturationCurve {
            reference_temperature: 300.,
            reference_pressure: 3500.,
            latent_heat: 2.4e6,
            vapor_gas_constant: 461.,
            min_temperature: 280.,
            max_temperature: 600.,
        };
        let mut vapor = ThermalVapor::new(600., 10000., 1., 0., curve)?;
        let mut material = MaterialThermalStore::new(1000., 300.)?;
        let heat_before = vapor.accounted_energy_j() + material.energy_j();
        let cold = CohesiveProperties {
            stiffness_pa_m: 1e6,
            closure_pa_m: 1e7,
            peak_pa: 1000.,
            fracture_j_m2: 10.,
        };
        let hot = CohesiveProperties {
            peak_pa: 500.,
            fracture_j_m2: 2.5,
            ..cold
        };
        let faces = ThermalCohesiveCalibration::new(
            300.,
            600.,
            CohesiveCalibration::new(cold, cold)?,
            CohesiveCalibration::new(hot, hot)?,
        )?;
        let cold_bulk = next.bulk.at(0.)?;
        let hot_bulk = Properties {
            young_pa: 5e4,
            ..cold_bulk
        };
        let bulk = ThermalCalibration::new(
            300.,
            600.,
            Calibration::new(cold_bulk, cold_bulk)?,
            Calibration::new(hot_bulk, hot_bulk)?,
        )?;
        let n = next.dynamics.velocities().len();
        let limits = QuadraticAdvanceLimits {
            minimum_dt_s: 1e-8,
            maximum_dt_s: 1e-5,
            max_attempts: 100,
            energy_tolerance_j: 1e-8,
        };
        let (water, heat, bulk, fracture, motion) =
            next.dynamics.advance_heated_vapor_loaded_calibrated(
                1e-5,
                &mut next.water,
                &mut vapor,
                &mut material,
                4200.,
                1e9,
                &[],
                &[1000. / 6.; 2],
                &[bulk; 2],
                &vec![[0.; 3]; n],
                &[faces],
                &[0.5],
                &vec![[0.; 3]; n],
                [0.; 3],
                limits,
            )?;
        if water.vapor_water_change_kg != 0.
            || (next.dynamics.energy()?.mass_kg - next.initial_mass).abs() > 1e-10
            || (vapor.accounted_energy_j() + material.energy_j() - heat_before).abs() > 1e-8
            || fracture.fragments_before != 1
            || fracture.fragments_after != 2
            || motion.substeps.is_empty()
        {
            return Err("heated FEM inventory/fracture/motion acceptance failed".into());
        }
        println!(
            "HEATED FEM: material_k={} vapor_k={} heat_received_by_vapor_j={heat} parameter_work_j={} substeps={}",
            material.temperature_k(),
            vapor.temperature_k(),
            bulk.elastic_parameter_work_j + fracture.total_parameter_work_j,
            motion.substeps.len()
        );
        next.thermal = Some((vapor, material));
        *self = next;
        Ok(())
    }
    /// Separate already broken pieces with explicit, balanced physical loads.
    pub(crate) fn separate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use physics::plasticity::mesh::QuadraticAdvanceLimits;
        let mut next = self.clone();
        let before = next.dynamics.fragments()?;
        if before.len() != 2 {
            return Err("fragment motion requires accepted fracture".into());
        }
        let n = next.dynamics.velocities().len();
        let densities: Vec<_> = next
            .water
            .cells()
            .iter()
            .map(|c| 1000. + 6. * c.water_kg)
            .collect();
        let mass = next.dynamics.body().consistent_mass(&densities)?;
        let mut acceleration = vec![[0.; 3]; n];
        for fragment in &before {
            let direction = if fragment.center_m[2] > 0. { 1. } else { -1. };
            for &node in &fragment.nodes {
                acceleration[node][2] = 4. * direction;
            }
        }
        let loads: Vec<_> = mass
            .iter()
            .map(|row| {
                std::array::from_fn(|a| row.iter().zip(&acceleration).map(|(m, x)| m * x[a]).sum())
            })
            .collect();
        let dt = 0.25;
        let receipt = next.dynamics.advance_loaded(
            dt,
            &loads,
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-7,
                maximum_dt_s: 0.0025,
                max_attempts: 20000,
                energy_tolerance_j: 1e-6,
            },
        )?;
        let after = next.dynamics.fragments()?;
        if after.len() != 2 {
            return Err("fragment motion changed admitted topology".into());
        }
        for axis in 0..3 {
            let before_p: f64 = before.iter().map(|f| f.momentum_kg_m_s[axis]).sum();
            let after_p: f64 = after.iter().map(|f| f.momentum_kg_m_s[axis]).sum();
            let impulse: f64 = loads.iter().map(|f| f[axis] * dt).sum();
            if (after_p - before_p - impulse).abs() > 1e-8 {
                return Err("FEM fragment motion momentum balance failed".into());
            }
        }

        for (old, new) in before.iter().zip(&after) {
            let a = acceleration[old.nodes[0]];
            for axis in 0..3 {
                let displacement = old.velocity_m_s[axis] * dt + 0.5 * a[axis] * dt * dt;
                if (new.center_m[axis] - old.center_m[axis] - displacement).abs() > 1e-8
                    || (new.velocity_m_s[axis] - old.velocity_m_s[axis] - a[axis] * dt).abs() > 1e-8
                {
                    return Err("FEM fragment motion differs from constant acceleration".into());
                }
            }
        }
        println!(
            "FEM MOTION: interval_s={dt} substeps={} centers_before={:?} centers_after={:?} energy_defect_j={}",
            receipt.substeps.len(),
            before.iter().map(|f| f.center_m).collect::<Vec<_>>(),
            after.iter().map(|f| f.center_m).collect::<Vec<_>>(),
            receipt.absolute_energy_defect_j
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admitted_fragment_motion_reaches_mesh_and_preserves_other_owners() {
        for heated in [false, true] {
            let mut demo = WetFemPreview::new().unwrap();
            let before = format!("{demo:?}");
            assert!(demo.separate().is_err());
            assert_eq!(format!("{demo:?}"), before);
            if heated {
                demo.heat().unwrap();
            } else {
                demo.wet().unwrap();
            }
            let water = format!("{:?}{:?}{:?}", demo.water, demo.sources, demo.thermal);
            let before = demo.mesh().unwrap();
            let positions: Vec<_> = before.vertices().iter().map(|v| v.position).collect();
            demo.separate().unwrap();
            assert_eq!(
                format!("{:?}{:?}{:?}", demo.water, demo.sources, demo.thermal),
                water
            );
            assert_eq!(demo.topology().unwrap(), (8, 2));
            let after = demo.mesh().unwrap();
            assert_eq!(before.indices(), after.indices());
            assert_ne!(
                positions,
                after
                    .vertices()
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>()
            );
        }
    }
}
