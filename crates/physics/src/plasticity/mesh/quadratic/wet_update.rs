//! Conservative momentum remap for fluid exchange in finite-elastic T10 solids.
use super::{ConsistentInertia, FiniteQuadraticDynamics, Vec3, dot};
#[derive(Clone, Debug)]
pub struct QuadraticWetUpdate {
    pub water_mass_change_kg: f64,
    pub water_momentum_kg_m_s: Vec3,
    pub support_impulse_n_s: Vec<Vec3>,
    pub carried_water_kinetic_j: f64,
    pub kinetic_transfer_loss_j: f64,
    pub elastic_parameter_work_j: f64,
    pub energy_defect_j: f64,
}
fn momentum(mass: &[Vec<f64>], velocities: &[Vec3]) -> Vec<Vec3> {
    mass.iter()
        .map(|row| {
            std::array::from_fn(|axis| row.iter().zip(velocities).map(|(m, v)| m * v[axis]).sum())
        })
        .collect()
}
impl FiniteQuadraticDynamics {
    /// Apply accepted per-cell water inventory and calibrated finite-elastic E/nu.
    /// Incoming velocity is a nodal field; outgoing water carries the old solid
    /// velocity. Reference volume is held fixed (no swelling). Reports boundary
    /// momentum, kinetic mixing/support loss and modulus parameter work explicitly.
    /// This preserves existing cohesive/history/contact state; finite elasticity
    /// remains elastic even if the calibration also contains yield/wear parameters.
    /// # Errors
    /// Invalid inventories/calibration/dimensions, inconsistent dry mass, singular
    /// inertia or invalid balance/overflow. The live body is unchanged on failure.
    pub fn apply_moisture(
        &mut self,
        water: &[crate::moisture::Cell],
        dry_mass_kg: &[f64],
        calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
    ) -> Result<QuadraticWetUpdate, &'static str> {
        let old = &self.inner;
        if water.len() != old.body.cells.len()
            || dry_mass_kg.len() != water.len()
            || calibration.len() != water.len()
            || incoming_velocity.len() != old.velocities.len()
            || incoming_velocity.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid quadratic wet update dimensions");
        }
        let before = old.energy()?;
        let mut candidate = old.clone();
        let mut target = momentum(&old.mass, &old.velocities);
        let mut water_momentum = [0.; 3];
        let mut carried = 0.;
        let mut materials = Vec::new();
        for (index, cell) in old.body.cells.iter().enumerate() {
            let old_water = old.densities[index] * cell.volume - dry_mass_kg[index];
            let scale = old.densities[index] * cell.volume;
            if !old_water.is_finite()
                || old_water < -1e-10 * scale
                || old_water > water[index].capacity_kg + 1e-10 * scale
            {
                return Err("inconsistent quadratic dry mass");
            }
            let density = water[index].wet_density_kg_m3(dry_mass_kg[index], cell.volume)?;
            let properties = water[index].properties(calibration[index])?;
            materials.push(crate::biomechanics::Material::from_young_poisson(
                properties.young_pa,
                properties.poisson,
            )?);
            let change = density - old.densities[index];
            candidate.densities[index] = density;
            let velocity = if change >= 0. {
                incoming_velocity
            } else {
                &old.velocities
            };
            let factor = cell.volume / 420. * change;
            for (i, &row) in cell.nodes.iter().enumerate() {
                for (j, &column) in cell.nodes.iter().enumerate() {
                    let mass = factor * super::super::mass_coefficient(i, j);
                    for axis in 0..3 {
                        let impulse = mass * velocity[column][axis];
                        target[row][axis] += impulse;
                        water_momentum[axis] += impulse;
                    }
                    carried += 0.5 * mass * dot(velocity[row], velocity[column]);
                }
            }
        }
        candidate.finite_materials = Some(materials);
        candidate.mass = candidate.body.consistent_mass(&candidate.densities)?;
        let reduced: Vec<Vec<f64>> = candidate
            .free_nodes
            .iter()
            .map(|&i| {
                candidate
                    .free_nodes
                    .iter()
                    .map(|&j| candidate.mass[i][j])
                    .collect()
            })
            .collect();
        candidate.inertia = ConsistentInertia::factor(&reduced)?;
        let restricted: Vec<_> = candidate.free_nodes.iter().map(|&i| target[i]).collect();
        let velocities = candidate.inertia.accelerations(&restricted)?;
        for (&node, &velocity) in candidate.free_nodes.iter().zip(&velocities) {
            candidate.velocities[node] = velocity;
        }
        candidate.last_coulomb = None;
        let after = candidate.energy()?;
        let actual = momentum(&candidate.mass, &candidate.velocities);
        let mut supports = vec![[0.; 3]; actual.len()];
        for node in 0..actual.len() {
            if !candidate.free_nodes.contains(&node) {
                supports[node] =
                    std::array::from_fn(|axis| actual[node][axis] - target[node][axis]);
            }
        }
        let report = balance_report(before, after, carried, water_momentum, supports)?;
        self.inner = candidate;
        Ok(report)
    }
}

fn balance_report(
    before: super::QuadraticEnergy,
    after: super::QuadraticEnergy,
    carried: f64,
    water_momentum: Vec3,
    supports: Vec<Vec3>,
) -> Result<QuadraticWetUpdate, &'static str> {
    let loss = before.kinetic_j + carried - after.kinetic_j;
    let tolerance = 1e-9 * (before.kinetic_j.abs() + carried.abs() + after.kinetic_j.abs()).max(1.);
    if !loss.is_finite() || loss < -tolerance {
        return Err("quadratic wet kinetic balance failure");
    }
    let parameter_work = after.elastic_j - before.elastic_j;
    let loss = loss.max(0.);
    let defect = after.kinetic_j - before.kinetic_j - carried + loss;
    for axis in 0..3 {
        let support: f64 = supports.iter().map(|p| p[axis]).sum();
        if (after.momentum_kg_m_s[axis]
            - before.momentum_kg_m_s[axis]
            - water_momentum[axis]
            - support)
            .abs()
            > 1e-8
                * (before.momentum_kg_m_s[axis].abs() + water_momentum[axis].abs() + support.abs())
                    .max(1.)
        {
            return Err("quadratic wet momentum balance failure");
        }
    }
    if [carried, parameter_work, defect]
        .iter()
        .chain(&water_momentum)
        .chain(supports.iter().flatten())
        .any(|x| !x.is_finite())
    {
        return Err("quadratic wet update overflow");
    }
    Ok(QuadraticWetUpdate {
        water_mass_change_kg: after.mass_kg - before.mass_kg,
        water_momentum_kg_m_s: water_momentum,
        support_impulse_n_s: supports,
        carried_water_kinetic_j: carried,
        kinetic_transfer_loss_j: loss,
        elastic_parameter_work_j: parameter_work,
        energy_defect_j: defect,
    })
}

impl FiniteQuadraticDynamics {
    /// Atomically advance finite water supplies and apply the resulting wet mass
    /// and calibrated elasticity. Cell order is shared by moisture and solid mesh.
    /// # Errors
    /// Inconsistent previous inventories or any moisture/mechanical update failure;
    /// material water, supply water and mechanical state all remain unchanged.
    pub fn advance_moisture_supplies(
        &mut self,
        dt_s: f64,
        water: &mut crate::moisture::Body,
        supplies: &mut [crate::moisture::WaterSupply],
        dry_mass_kg: &[f64],
        calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
    ) -> Result<(crate::moisture::SupplyTransfer, QuadraticWetUpdate), &'static str> {
        if water.cells().len() != self.inner.body.cells.len()
            || dry_mass_kg.len() != water.cells().len()
        {
            return Err("inconsistent wet exchange cell mapping");
        }
        for (i, cell) in self.inner.body.cells.iter().enumerate() {
            let expected = dry_mass_kg[i] + water.cells()[i].water_kg;
            let actual = self.inner.densities[i] * cell.volume;
            if !expected.is_finite()
                || (actual - expected).abs() > 1e-10 * actual.max(expected).max(1e-300)
            {
                return Err("inconsistent previous wet inventory");
            }
        }
        let mut next_water = water.clone();
        let mut next_supplies = supplies.to_vec();
        let mut next_solid = self.clone();
        let transfer = next_water.advance_with_supplies(dt_s, &mut next_supplies)?;
        let mechanics = next_solid.apply_moisture(
            next_water.cells(),
            dry_mass_kg,
            calibration,
            incoming_velocity,
        )?;
        *water = next_water;
        supplies.copy_from_slice(&next_supplies);
        *self = next_solid;
        Ok((transfer, mechanics))
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticCohesiveWetUpdate {
    pub cohesive_parameter_work_j: f64,
    /// Newly exposed contact geometry may change its stored potential at fixed pose.
    pub surface_contact_parameter_work_j: f64,
    pub total_parameter_work_j: f64,
    pub fragments_before: usize,
    pub fragments_after: usize,
}
impl FiniteQuadraticDynamics {
    /// Apply explicitly supplied interface saturation at accepted fixed pose.
    /// Calibrations are per interface; no implicit cell-to-face averaging rule.
    /// Reports external parameter work; does not advance time or water inventory.
    /// # Errors
    /// Invalid dimensions/laws, healing, unsupported friction history or invalid
    /// newly exposed contact geometry. Body and all histories remain atomic.
    pub fn apply_cohesive_moisture(
        &mut self,
        saturation: &[f64],
        calibration: &[crate::moisture::CohesiveCalibration],
    ) -> Result<QuadraticCohesiveWetUpdate, &'static str> {
        if saturation.len() != self.inner.body.interfaces.len()
            || calibration.len() != saturation.len()
        {
            return Err("invalid cohesive moisture update dimensions");
        }
        let before = self.energy()?;
        let fragments_before = self.fragments()?.len();
        let mut next = self.clone();
        let mut work = 0.;
        for ((face, &s), &c) in next
            .inner
            .body
            .interfaces
            .iter_mut()
            .zip(saturation)
            .zip(calibration)
        {
            work += face.update_material_at(&next.inner.body.positions, c.at(s)?)?;
        }
        let after = next.energy()?;
        let contact_work = after.surface_contact_j - before.surface_contact_j;
        let defect = after.cohesive_stored_j - before.cohesive_stored_j - work;
        if !work.is_finite()
            || !contact_work.is_finite()
            || defect.abs()
                > 1e-10
                    * before
                        .cohesive_stored_j
                        .abs()
                        .max(after.cohesive_stored_j.abs())
                        .max(1e-12)
            || (after.fracture_dissipated_j - before.fracture_dissipated_j).abs()
                > 1e-10 * before.fracture_dissipated_j.abs().max(1e-12)
        {
            return Err("cohesive moisture energy balance failure");
        }
        let report = QuadraticCohesiveWetUpdate {
            cohesive_parameter_work_j: work,
            surface_contact_parameter_work_j: contact_work,
            total_parameter_work_j: work + contact_work,
            fragments_before,
            fragments_after: next.fragments()?.len(),
        };
        if !report.total_parameter_work_j.is_finite() {
            return Err("cohesive moisture parameter work overflow");
        }
        *self = next;
        Ok(report)
    }
}

impl FiniteQuadraticDynamics {
    /// Interface saturation from the two owning cells: explicit minus-side weight
    /// times its saturation plus the complementary plus-side contribution.
    /// This is a calibrated mixing choice, not a resolved through-face gradient.
    /// # Errors
    /// Invalid inventory, weight, dimensions or ambiguous interface ownership.
    pub fn cohesive_cell_saturations(
        &self,
        water: &[crate::moisture::Cell],
        minus_weights: &[f64],
    ) -> Result<Vec<f64>, &'static str> {
        let body = &self.inner.body;
        if water.len() != body.cells.len()
            || minus_weights.len() != body.interfaces.len()
            || minus_weights
                .iter()
                .any(|w| !w.is_finite() || !(0. ..=1.).contains(w))
        {
            return Err("invalid cohesive cell moisture dimensions or weights");
        }
        let saturation = water
            .iter()
            .map(|c| c.saturation())
            .collect::<Result<Vec<_>, _>>()?;
        body.interfaces
            .iter()
            .zip(minus_weights)
            .map(|(face, &weight)| {
                let owner = |nodes: [usize; 6]| -> Result<usize, &'static str> {
                    let owners = body
                        .cells
                        .iter()
                        .enumerate()
                        .filter(|(_, cell)| nodes.iter().all(|n| cell.nodes.contains(n)))
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    if owners.len() != 1 {
                        return Err("ambiguous wet cohesive cell ownership");
                    }
                    Ok(owners[0])
                };
                let (minus, plus) = face.sides();
                let a = owner(minus)?;
                let b = owner(plus)?;
                if a == b {
                    return Err("wet cohesive interface has same cell on both sides");
                }
                Ok(weight * saturation[a] + (1. - weight) * saturation[b])
            })
            .collect()
    }
    /// Joint accepted-water update of bulk mass/moduli and cohesive strength.
    /// Parameter work remains explicit in the two returned mechanical reports.
    /// # Errors
    /// Any bulk or cohesive failure rolls back mass, velocities, laws and histories.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_moisture_with_cohesion(
        &mut self,
        water: &[crate::moisture::Cell],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
    ) -> Result<(QuadraticWetUpdate, QuadraticCohesiveWetUpdate), &'static str> {
        let saturation = self.cohesive_cell_saturations(water, minus_weights)?;
        let mut next = self.clone();
        let bulk = next.apply_moisture(water, dry_mass_kg, bulk_calibration, incoming_velocity)?;
        let cohesive = next.apply_cohesive_moisture(&saturation, face_calibration)?;
        *self = next;
        Ok((bulk, cohesive))
    }
}

impl FiniteQuadraticDynamics {
    /// Advance finite water sources, bulk wet mechanics and interface damage in
    /// one transaction. Cell-to-face mixing weights are explicit. Reports water
    /// transfer and both mechanical energy exchanges separately.
    /// # Errors
    /// Any transport, inertia, constitutive or cohesive failure preserves the
    /// source inventories, material water and entire dynamic body.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_moisture_supplies_with_cohesion(
        &mut self,
        dt_s: f64,
        water: &mut crate::moisture::Body,
        supplies: &mut [crate::moisture::WaterSupply],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
    ) -> Result<
        (
            crate::moisture::SupplyTransfer,
            QuadraticWetUpdate,
            QuadraticCohesiveWetUpdate,
        ),
        &'static str,
    > {
        let mut next = self.clone();
        let mut next_water = water.clone();
        let mut next_supplies = supplies.to_vec();
        let (transfer, bulk) = next.advance_moisture_supplies(
            dt_s,
            &mut next_water,
            &mut next_supplies,
            dry_mass_kg,
            bulk_calibration,
            incoming_velocity,
        )?;
        let saturation = next.cohesive_cell_saturations(next_water.cells(), minus_weights)?;
        let cohesive = next.apply_cohesive_moisture(&saturation, face_calibration)?;
        *self = next;
        *water = next_water;
        supplies.copy_from_slice(&next_supplies);
        Ok((transfer, bulk, cohesive))
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticWetAdvance {
    pub transfer: crate::moisture::SupplyTransfer,
    pub bulk: QuadraticWetUpdate,
    pub cohesive: QuadraticCohesiveWetUpdate,
    pub motion: super::QuadraticAdvance,
}
impl FiniteQuadraticDynamics {
    /// One atomic operator-split interval: implicit uptake and material changes,
    /// then adaptive mechanical motion with the updated mass and laws.
    /// This is first-order moisture/motion splitting, not a monolithic solve.
    /// Boundary/parameter work is reported separately from motion energy defects.
    /// # Errors
    /// Any uptake, history migration or dynamic integration failure rolls back
    /// sources, water, mechanics and accepted positions for the whole interval.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_wet_loaded(
        &mut self,
        interval_s: f64,
        water: &mut crate::moisture::Body,
        supplies: &mut [crate::moisture::WaterSupply],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
        loads: &[Vec3],
        acceleration: Vec3,
        limits: super::QuadraticAdvanceLimits,
    ) -> Result<QuadraticWetAdvance, &'static str> {
        let mut next = self.clone();
        let mut next_water = water.clone();
        let mut next_supplies = supplies.to_vec();
        let (transfer, bulk, cohesive) = next.advance_moisture_supplies_with_cohesion(
            interval_s,
            &mut next_water,
            &mut next_supplies,
            dry_mass_kg,
            bulk_calibration,
            incoming_velocity,
            face_calibration,
            minus_weights,
        )?;
        let motion = next.advance_loaded(interval_s, loads, acceleration, limits)?;
        *self = next;
        *water = next_water;
        supplies.copy_from_slice(&next_supplies);
        Ok(QuadraticWetAdvance {
            transfer,
            bulk,
            cohesive,
            motion,
        })
    }
}

impl FiniteQuadraticDynamics {
    /// Atomically exchange material water with finite isothermal vapor, then
    /// update mass, elasticity and cohesive damage at the accepted fixed pose.
    /// Gas velocity is an explicit incoming nodal field. Outgoing water momentum
    /// and kinetic energy remain boundary transfers in the bulk report; the
    /// vapor reservoir stores water/latent heat, not gas momentum or sensible heat.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_moisture_vapor_with_cohesion(
        &mut self,
        dt_s: f64,
        water: &mut crate::moisture::Body,
        vapor: &mut crate::moisture::VaporReservoir,
        links: &[crate::moisture::VaporLink],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
    ) -> Result<
        (
            crate::moisture::VaporTransfer,
            QuadraticWetUpdate,
            QuadraticCohesiveWetUpdate,
        ),
        &'static str,
    > {
        if water.cells().len() != self.inner.body.cells.len()
            || dry_mass_kg.len() != water.cells().len()
        {
            return Err("inconsistent wet exchange cell mapping");
        }
        for (i, cell) in self.inner.body.cells.iter().enumerate() {
            let expected = dry_mass_kg[i] + water.cells()[i].water_kg;
            let actual = self.inner.densities[i] * cell.volume;
            if !expected.is_finite()
                || (actual - expected).abs() > 1e-10 * actual.max(expected).max(1e-300)
            {
                return Err("inconsistent previous wet inventory");
            }
        }
        let mut next = self.clone();
        let mut next_water = water.clone();
        let mut next_vapor = vapor.clone();
        let transfer = next_water.advance_vapor(dt_s, &mut next_vapor, links)?;
        let (bulk, cohesive) = next.apply_moisture_with_cohesion(
            next_water.cells(),
            dry_mass_kg,
            bulk_calibration,
            incoming_velocity,
            face_calibration,
            minus_weights,
        )?;
        *self = next;
        *water = next_water;
        *vapor = next_vapor;
        Ok((transfer, bulk, cohesive))
    }
}

impl FiniteQuadraticDynamics {
    /// Atomic first-order split interval: finite isothermal vapor exchange,
    /// material/cohesive update, then adaptive loaded motion. Boundary momentum
    /// and material parameter work are reported separately from dynamic defects.
    /// Any motion failure restores solid, material water and vapor inventories.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_vapor_loaded(
        &mut self,
        interval_s: f64,
        water: &mut crate::moisture::Body,
        vapor: &mut crate::moisture::VaporReservoir,
        links: &[crate::moisture::VaporLink],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
        loads: &[Vec3],
        acceleration: Vec3,
        limits: super::QuadraticAdvanceLimits,
    ) -> Result<
        (
            crate::moisture::VaporTransfer,
            QuadraticWetUpdate,
            QuadraticCohesiveWetUpdate,
            super::QuadraticAdvance,
        ),
        &'static str,
    > {
        let mut next = self.clone();
        let mut next_water = water.clone();
        let mut next_vapor = vapor.clone();
        let (transfer, bulk, cohesive) = next.advance_moisture_vapor_with_cohesion(
            interval_s,
            &mut next_water,
            &mut next_vapor,
            links,
            dry_mass_kg,
            bulk_calibration,
            incoming_velocity,
            face_calibration,
            minus_weights,
        )?;
        let motion = next.advance_loaded(interval_s, loads, acceleration, limits)?;
        *self = next;
        *water = next_water;
        *vapor = next_vapor;
        Ok((transfer, bulk, cohesive, motion))
    }
}

impl FiniteQuadraticDynamics {
    /// Atomic heat/water/material/motion interval with lumped temperatures.
    /// Thermal stores close water enthalpy; free-body water mixing loss heats
    /// material. Supported-body mixing loss, parameter work and other mechanical
    /// dissipation remain report terms. Laws do not depend on temperature yet.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_heated_vapor_loaded(
        &mut self,
        interval_s: f64,
        water: &mut crate::moisture::Body,
        vapor: &mut crate::moisture::ThermalVapor,
        thermal: &mut crate::moisture::MaterialThermalStore,
        water_specific_heat_j_kg_k: f64,
        conductance_w_k: f64,
        links: &[crate::moisture::VaporLink],
        dry_mass_kg: &[f64],
        bulk_calibration: &[crate::moisture::Calibration],
        incoming_velocity: &[Vec3],
        face_calibration: &[crate::moisture::CohesiveCalibration],
        minus_weights: &[f64],
        loads: &[Vec3],
        acceleration: Vec3,
        limits: super::QuadraticAdvanceLimits,
    ) -> Result<
        (
            crate::moisture::VaporTransfer,
            f64,
            QuadraticWetUpdate,
            QuadraticCohesiveWetUpdate,
            super::QuadraticAdvance,
        ),
        &'static str,
    > {
        if water.cells().len() != self.inner.body.cells.len()
            || dry_mass_kg.len() != water.cells().len()
        {
            return Err("inconsistent wet exchange cell mapping");
        }
        for (i, cell) in self.inner.body.cells.iter().enumerate() {
            let expected = dry_mass_kg[i] + water.cells()[i].water_kg;
            let actual = self.inner.densities[i] * cell.volume;
            if !expected.is_finite()
                || (actual - expected).abs() > 1e-10 * actual.max(expected).max(1e-300)
            {
                return Err("inconsistent previous wet inventory");
            }
        }
        let initial_kinetic = self.energy()?.kinetic_j;
        let initial_thermal = vapor.accounted_energy_j() + thermal.energy_j();
        let mut next = self.clone();
        let mut next_water = water.clone();
        let mut next_vapor = vapor.clone();
        let mut next_thermal = thermal.clone();
        let (transfer, heat) = next_water.advance_heated_vapor(
            interval_s,
            &mut next_vapor,
            &mut next_thermal,
            water_specific_heat_j_kg_k,
            conductance_w_k,
            links,
        )?;
        let (bulk, cohesive) = next.apply_moisture_with_cohesion(
            next_water.cells(),
            dry_mass_kg,
            bulk_calibration,
            incoming_velocity,
            face_calibration,
            minus_weights,
        )?;
        // Fully free-body mixing loss is deposited in the material store.
        // Supported-body loss includes constraint work and stays a report term.
        if next.inner.free_nodes.len() == next.inner.velocities.len() {
            next_thermal.deposit_heat(bulk.kinetic_transfer_loss_j)?;
        }
        let final_kinetic = next.energy()?.kinetic_j;
        let final_thermal = next_vapor.accounted_energy_j() + next_thermal.energy_j();
        let unconverted_loss = if next.inner.free_nodes.len() == next.inner.velocities.len() {
            0.
        } else {
            bulk.kinetic_transfer_loss_j
        };
        let exchange_defect = (final_kinetic - initial_kinetic) + (final_thermal - initial_thermal)
            - bulk.carried_water_kinetic_j
            + unconverted_loss;
        let scale = initial_thermal.abs()
            + final_thermal.abs()
            + initial_kinetic.abs()
            + final_kinetic.abs()
            + bulk.carried_water_kinetic_j.abs();
        if !scale.is_finite()
            || !exchange_defect.is_finite()
            || exchange_defect.abs() > 1e-12 * scale.max(f64::MIN_POSITIVE)
        {
            return Err("heated wet kinetic and thermal balance failure");
        }
        let motion = next.advance_loaded(interval_s, loads, acceleration, limits)?;
        *self = next;
        *water = next_water;
        *vapor = next_vapor;
        *thermal = next_thermal;
        Ok((transfer, heat, bulk, cohesive, motion))
    }
}

impl FiniteQuadraticDynamics {
    /// Apply bounded per-cell temperature/moisture calibration through the same
    /// wet inertia/constitutive transaction. Modulus energy change is explicit
    /// parameter work; it is not silently taken from thermal stores. No thermal
    /// expansion or temperature-dependent plastic evolution is implied.
    pub fn apply_thermal_moisture(
        &mut self,
        water: &[crate::moisture::Cell],
        dry_mass_kg: &[f64],
        calibration: &[crate::moisture::ThermalCalibration],
        temperatures_k: &[f64],
        incoming_velocity: &[Vec3],
    ) -> Result<QuadraticWetUpdate, &'static str> {
        if calibration.len() != water.len() || temperatures_k.len() != water.len() {
            return Err("invalid thermal wet calibration dimensions");
        }
        let laws = calibration
            .iter()
            .zip(temperatures_k)
            .map(|(&law, &t)| law.at_temperature(t))
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_moisture(water, dry_mass_kg, &laws, incoming_velocity)
    }
}

impl FiniteQuadraticDynamics {
    /// Update per-interface temperature/moisture laws using accepted-history
    /// migration. Preserves fracture work, rejects healing, and reports parameter
    /// work rather than silently taking energy from a thermal owner.
    pub fn apply_thermal_cohesive_moisture(
        &mut self,
        saturation: &[f64],
        calibration: &[crate::moisture::ThermalCohesiveCalibration],
        temperatures_k: &[f64],
    ) -> Result<QuadraticCohesiveWetUpdate, &'static str> {
        if saturation.len() != calibration.len() || temperatures_k.len() != calibration.len() {
            return Err("invalid thermal cohesive calibration dimensions");
        }
        let laws = calibration
            .iter()
            .zip(temperatures_k)
            .map(|(&law, &t)| law.at_temperature(t))
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_cohesive_moisture(saturation, &laws)
    }
}
