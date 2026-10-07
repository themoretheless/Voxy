//! Diagnostic finite-inventory uptake weakening a preloaded cohesive interface.
use physics::{
    biomechanics::Material as Elastic,
    moisture::{
        Body, Calibration, Cell, CohesiveCalibration, CohesiveProperties, FilmSupply, Properties,
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
    sources: physics::surface_film::FilmMixture,
    bulk: Calibration,
    bond: CohesiveCalibration,
    initial_mass: f64,
    drying_vapor: Option<physics::moisture::VaporReservoir>,
    thermal: Option<(
        physics::moisture::ThermalVapor,
        physics::moisture::MaterialThermalStore,
    )>,
}
impl WetFemPreview {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_preload(0.0125)
    }
    fn with_preload(opening: f64) -> Result<Self, Box<dyn std::error::Error>> {
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
            .map(|i| [Some(0.), Some(0.), Some(if i < 10 { opening } else { 0. })])
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
        let source_surface = physics::surface_film::SurfaceFilm::new(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            vec![[0, 1, 2], [1, 3, 2]],
            physics::surface_film::Material::default(),
        )?;
        let mut sources = physics::surface_film::FilmMixture::new(
            source_surface,
            vec!["water".into()],
            vec![vec![1.]; 2],
        )?;
        sources.deposit_component_masses_batch(&[(0, vec![0.1]), (1, vec![0.1])])?;
        Ok(Self {
            dynamics,
            initial_mass,
            thermal: None,
            drying_vapor: None,
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
            sources,
        })
    }
    pub(crate) fn wet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.wet_with_expected_fragments(2)
    }
    fn wet_with_expected_fragments(
        &mut self,
        expected: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut next = self.clone();
        let n = next.dynamics.velocities().len();
        let transfer = next.water.advance_surface_film_supplies(
            1.,
            &mut next.sources,
            0,
            &[
                FilmSupply {
                    film_cell: 0,
                    material_cell: 0,
                    conductance_kg_s: 100.,
                },
                FilmSupply {
                    film_cell: 1,
                    material_cell: 1,
                    conductance_kg_s: 100.,
                },
            ],
        )?;
        let (bulk, fracture) = next.dynamics.apply_moisture_with_cohesion(
            next.water.cells(),
            &[1000. / 6.; 2],
            &[next.bulk; 2],
            &vec![[0.; 3]; n],
            &[next.bond],
            &[0.5],
        )?;
        let retained: f64 = next.water.cells().iter().map(|c| c.water_kg).sum();
        let supplied = next.sources.component_masses()?[0];
        if (retained + supplied - 0.2).abs() > 1e-12
            || (next.dynamics.energy()?.mass_kg - next.initial_mass - retained).abs() > 1e-10
            || bulk.energy_defect_j.abs() > 1e-10
            || transfer.mass_defect_kg.abs() > 1e-12
            || fracture.fragments_before != 1
            || fracture.fragments_after != expected
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
    /// Finite isothermal drying; broken interface history must remain broken.
    pub(crate) fn dry(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use physics::moisture::{VaporLink, VaporReservoir};
        let mut next = self.clone();
        let before: f64 = next.water.cells().iter().map(|c| c.water_kg).sum();
        let topology = next.topology()?;
        let mut vapor = next
            .drying_vapor
            .clone()
            .unwrap_or(VaporReservoir::new(1., 0., 2.4e6, 1e6)?);
        let total_before = before + vapor.water_kg();
        let energy_before = vapor.accounted_energy_j();
        let n = next.dynamics.velocities().len();
        let (transfer, bulk, fracture) = next.dynamics.advance_moisture_vapor_with_cohesion(
            1.,
            &mut next.water,
            &mut vapor,
            &[
                VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.01,
                },
                VaporLink {
                    material_cell: 1,
                    conductance_kg_s: 0.01,
                },
            ],
            &[1000. / 6.; 2],
            &[next.bulk; 2],
            &vec![[0.; 3]; n],
            &[next.bond],
            &[0.5],
        )?;
        let retained: f64 = next.water.cells().iter().map(|c| c.water_kg).sum();
        if retained >= before
            || transfer.vapor_water_change_kg <= 0.
            || (retained + vapor.water_kg() - total_before).abs() > 1e-12
            || (vapor.accounted_energy_j() - energy_before).abs() > 1e-8
            || (next.dynamics.energy()?.mass_kg - next.initial_mass - retained).abs() > 1e-10
            || next.topology()? != topology
            || fracture.fragments_after != fracture.fragments_before
        {
            return Err("drying FEM water/energy/history acceptance failed".into());
        }
        println!(
            "DRY FEM: retained_water_kg={retained} vapor_water_kg={} latent_exchange_j={} parameter_work_j={}",
            vapor.water_kg(),
            transfer.latent_exchange_j,
            bulk.elastic_parameter_work_j + fracture.total_parameter_work_j
        );
        next.drying_vapor = Some(vapor);
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
    fn partial_drying_loaded_transaction_moves_and_rolls_back_failed_interval() {
        use physics::moisture::{VaporLink, VaporReservoir};
        use physics::plasticity::mesh::QuadraticAdvanceLimits;
        let mut demo = WetFemPreview::with_preload(0.006).unwrap();
        demo.wet_with_expected_fragments(1).unwrap();
        let mut vapor = VaporReservoir::new(1., 0., 2.4e6, 1e6).unwrap();
        let n = demo.dynamics.velocities().len();
        let initial_positions = demo.dynamics.positions().to_vec();
        let initial_water: f64 = demo.water.cells().iter().map(|c| c.water_kg).sum();
        let initial_energy = vapor.accounted_energy_j();
        let limits = QuadraticAdvanceLimits {
            minimum_dt_s: 1e-8,
            maximum_dt_s: 1e-5,
            max_attempts: 100,
            energy_tolerance_j: 1e-8,
        };
        let run = |demo: &mut WetFemPreview, vapor: &mut VaporReservoir, dt, limits| {
            demo.dynamics.advance_vapor_loaded(
                dt,
                &mut demo.water,
                vapor,
                &[
                    VaporLink {
                        material_cell: 0,
                        conductance_kg_s: 0.01,
                    },
                    VaporLink {
                        material_cell: 1,
                        conductance_kg_s: 0.01,
                    },
                ],
                &[1000. / 6.; 2],
                &[demo.bulk; 2],
                &vec![[0.; 3]; n],
                &[demo.bond],
                &[0.5],
                &vec![[0.; 3]; n],
                [0., -9.81, 0.],
                limits,
            )
        };
        for _ in 0..3 {
            let (transfer, bulk, cohesive, motion) =
                run(&mut demo, &mut vapor, 1e-5, limits).unwrap();
            assert!(transfer.vapor_water_change_kg > 0.);
            assert!(transfer.mass_defect_kg.abs() < 1e-12);
            assert!(bulk.energy_defect_j.abs() < 1e-10);
            assert_eq!(cohesive.fragments_after, 1);
            assert!(!motion.substeps.is_empty());
            println!(
                "PARTIAL_DRYING_MOTION_RECEIPT vapor_change_kg={} water_defect_kg={} latent_defect_j={} dynamic_absolute_defect_j={} substeps={}",
                transfer.vapor_water_change_kg,
                transfer.mass_defect_kg,
                transfer.energy_defect_j,
                motion.absolute_energy_defect_j,
                motion.substeps.len()
            );
            let water: f64 = demo.water.cells().iter().map(|c| c.water_kg).sum();
            assert!((water + vapor.water_kg() - initial_water).abs() < 1e-12);
            assert!((vapor.accounted_energy_j() - initial_energy).abs() < 1e-8);
            assert!(
                (demo.dynamics.energy().unwrap().mass_kg - demo.initial_mass - water).abs() < 1e-10
            );
            assert_eq!(demo.topology().unwrap(), (6, 1));
        }
        assert_ne!(demo.dynamics.positions(), initial_positions.as_slice());
        let before = format!("{demo:?}{vapor:?}");
        let impossible = QuadraticAdvanceLimits {
            max_attempts: 1,
            ..limits
        };
        assert_eq!(
            run(&mut demo, &mut vapor, 1e-4, impossible).unwrap_err(),
            "quadratic adaptive attempt limit reached"
        );
        assert_eq!(format!("{demo:?}{vapor:?}"), before);
        println!(
            "PARTIAL_DRYING_MOTION_PASS interval_s=0.00003 accepted_intervals=3 late_failure_rollback=true"
        );
    }
    #[test]
    #[ignore = "manual native partial-drying motion timestep qualification"]
    fn partial_drying_motion_convergence() {
        run_partial_drying_motion_convergence(false);
    }
    #[test]
    #[ignore = "manual native partial-drying time-scaled energy qualification"]
    fn partial_drying_motion_time_scaled_convergence() {
        run_partial_drying_motion_convergence(true);
    }
    fn run_partial_drying_motion_convergence(time_scaled: bool) {
        println!(
            "PARTIAL_DRYING_ENERGY_POLICY time_scaled={time_scaled} reference_rate_j_s=0.001 base_step_budget_j=0.00000001"
        );
        use physics::moisture::{VaporLink, VaporReservoir};
        use physics::plasticity::mesh::QuadraticAdvanceLimits;
        let mut results = Vec::new();
        for level in 0..3 {
            let mut demo = WetFemPreview::with_preload(0.006).unwrap();
            demo.wet_with_expected_fragments(1).unwrap();
            let mut vapor = VaporReservoir::new(1., 0., 2.4e6, 1e6).unwrap();
            let initial_water: f64 = demo.water.cells().iter().map(|c| c.water_kg).sum();
            let energy = vapor.accounted_energy_j();
            let n = demo.dynamics.velocities().len();
            let steps = 10000 * (1 << level);
            let dt = 0.1 / steps as f64;
            let limits = QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: dt,
                max_attempts: 100,
                energy_tolerance_j: 1e-8,
            };
            let limits = if time_scaled {
                limits.with_interval_energy_rate(dt, 1e-3).unwrap()
            } else {
                limits
            };
            let mechanical = |demo: &WetFemPreview| {
                let e = demo.dynamics.energy().unwrap();
                assert_eq!(
                    e.hardening_j
                        + e.dissipated_j
                        + e.impact_dissipated_j
                        + e.contact_j
                        + e.surface_contact_j
                        + e.friction_dissipated_j
                        + e.cohesive_friction_dissipated_j
                        + e.cohesive_friction_released_j,
                    0.
                );
                e.kinetic_j + e.elastic_j + e.cohesive_stored_j + e.fracture_dissipated_j
            };
            let initial_mechanical = mechanical(&demo);
            let mut boundary_work = 0.;
            let mut maximum_independent_balance = 0_f64;
            let mut max_defect = 0_f64;
            for _ in 0..steps {
                let old_positions = demo.dynamics.positions().to_vec();
                let (transfer, bulk, cohesive, motion) = demo
                    .dynamics
                    .advance_vapor_loaded(
                        dt,
                        &mut demo.water,
                        &mut vapor,
                        &[
                            VaporLink {
                                material_cell: 0,
                                conductance_kg_s: 0.01,
                            },
                            VaporLink {
                                material_cell: 1,
                                conductance_kg_s: 0.01,
                            },
                        ],
                        &[1000. / 6.; 2],
                        &[demo.bulk; 2],
                        &vec![[0.; 3]; n],
                        &[demo.bond],
                        &[0.5],
                        &vec![[0.; 3]; n],
                        [0., -9.81, 0.],
                        limits,
                    )
                    .unwrap();
                let densities: Vec<_> = demo
                    .water
                    .cells()
                    .iter()
                    .map(|c| 1000. + 6. * c.water_kg)
                    .collect();
                let mass = demo.dynamics.body().consistent_mass(&densities).unwrap();
                let gravity_work: f64 = mass
                    .iter()
                    .enumerate()
                    .map(|(i, row)| {
                        row.iter().sum::<f64>()
                            * -9.81
                            * (demo.dynamics.positions()[i][1] - old_positions[i][1])
                    })
                    .sum();
                boundary_work += gravity_work + bulk.carried_water_kinetic_j
                    - bulk.kinetic_transfer_loss_j
                    + bulk.elastic_parameter_work_j
                    + cohesive.total_parameter_work_j;
                let balance = mechanical(&demo) - initial_mechanical - boundary_work;
                assert!(balance.is_finite());
                maximum_independent_balance = maximum_independent_balance.max(balance.abs());
                assert!(transfer.mass_defect_kg.abs() < 1e-12);
                max_defect = max_defect.max(motion.absolute_energy_defect_j);
                assert_eq!(demo.topology().unwrap(), (6, 1));
            }
            let water: f64 = demo.water.cells().iter().map(|c| c.water_kg).sum();
            let exact =
                initial_water / 6. + (initial_water - initial_water / 6.) * (-0.12_f64 * 0.1).exp();
            let error = (water - exact).abs();
            assert!((water + vapor.water_kg() - initial_water).abs() < 1e-11);
            assert!((vapor.accounted_energy_j() - energy).abs() < 1e-7);
            println!(
                "PARTIAL_DRYING_CONVERGENCE level={level} steps={steps} dt_s={dt} water_kg={water} exact_water_error_kg={error} maximum_dynamic_defect_j={max_defect}"
            );
            println!(
                "PARTIAL_DRYING_INDEPENDENT_ENERGY level={level} maximum_balance_j={maximum_independent_balance} final_balance_j={} reported_solver_defect_subtracted=false",
                mechanical(&demo) - initial_mechanical - boundary_work
            );
            results.push((
                error,
                demo.dynamics.positions().to_vec(),
                maximum_independent_balance,
                (mechanical(&demo) - initial_mechanical - boundary_work).abs(),
            ));
        }
        assert!(results[1].0 < results[0].0 && results[2].0 < results[1].0);
        if time_scaled {
            assert!(results[1].2 < results[0].2 && results[2].2 < results[1].2);
            assert!(results[1].3 < results[0].3 && results[2].3 < results[1].3);
        }
        let delta = |a: &[[f64; 3]], b: &[[f64; 3]]| {
            a.iter()
                .zip(b)
                .flat_map(|(a, b)| (0..3).map(move |i| (a[i] - b[i]).abs()))
                .fold(0_f64, f64::max)
        };
        let coarse = delta(&results[0].1, &results[1].1);
        let fine = delta(&results[1].1, &results[2].1);
        println!("PARTIAL_DRYING_POSITION_CONVERGENCE coarse_delta_m={coarse} fine_delta_m={fine}");
        assert!(fine < coarse);
    }
    #[test]
    fn partial_damage_drying_preserves_volume_history_and_finite_inventories() {
        let mut demo = WetFemPreview::with_preload(0.006).unwrap();
        demo.wet_with_expected_fragments(1).unwrap();
        assert_eq!(demo.topology().unwrap(), (6, 1));
        let before = demo.dynamics.energy().unwrap();
        assert!(before.fracture_dissipated_j > 0.);
        let geometry: Vec<_> = demo
            .mesh()
            .unwrap()
            .vertices()
            .iter()
            .map(|v| v.position)
            .collect();
        for _ in 0..5 {
            demo.dry().unwrap();
            assert_eq!(demo.topology().unwrap(), (6, 1));
            assert!(
                (demo.dynamics.energy().unwrap().fracture_dissipated_j
                    - before.fracture_dissipated_j)
                    .abs()
                    < 1e-12
            );
            assert_eq!(
                demo.mesh()
                    .unwrap()
                    .vertices()
                    .iter()
                    .map(|v| v.position)
                    .collect::<Vec<_>>(),
                geometry
            );
        }
    }
    #[test]
    fn drying_preserves_broken_topology_and_rolls_back_invalid_prior_inventory() {
        let mut demo = WetFemPreview::new().unwrap();
        demo.wet().unwrap();
        demo.dry().unwrap();
        assert_eq!(demo.topology().unwrap(), (8, 2));
        demo.dry().unwrap();
        assert_eq!(demo.topology().unwrap(), (8, 2));
        let mut invalid = WetFemPreview::new().unwrap();
        let before = format!("{invalid:?}");
        assert!(invalid.dry().is_err());
        assert_eq!(format!("{invalid:?}"), before);
    }
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
                // This diagnostic requires the original single fragment. A later
                // topology admission failure must retain film, water and mechanics.
                let before = format!("{demo:?}");
                assert!(demo.wet().is_err());
                assert_eq!(format!("{demo:?}"), before);
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
