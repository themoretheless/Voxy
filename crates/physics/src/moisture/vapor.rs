//! Closed isothermal activity exchange with explicit latent-energy inventory.
use super::{Body, Cell, Link};
#[derive(Clone, Debug)]
pub struct VaporReservoir {
    cell: Cell,
    latent_j_kg: f64,
    thermal_j: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct VaporLink {
    pub material_cell: usize,
    /// kg/s per activity difference. Material saturation is assumed to be its
    /// calibrated equilibrium activity; this is not a universal sorption law.
    pub conductance_kg_s: f64,
}
#[derive(Clone, Debug)]
pub struct VaporTransfer {
    pub material_water_change_kg: Vec<f64>,
    pub vapor_water_change_kg: f64,
    /// Positive for evaporation (withdrawn from thermal inventory).
    pub latent_exchange_j: f64,
    pub mass_defect_kg: f64,
    pub energy_defect_j: f64,
}
impl VaporReservoir {
    /// Capacity is saturated vapor mass at the caller's fixed temperature/volume.
    /// The finite thermal inventory supplies latent heat at that fixed temperature.
    pub fn new(
        capacity_kg: f64,
        water_kg: f64,
        latent_j_kg: f64,
        thermal_j: f64,
    ) -> Result<Self, &'static str> {
        Body::new(
            vec![Cell {
                capacity_kg,
                water_kg,
            }],
            vec![],
        )?;
        if !latent_j_kg.is_finite()
            || latent_j_kg <= 0.
            || !thermal_j.is_finite()
            || thermal_j < 0.
            || !(thermal_j + latent_j_kg * water_kg).is_finite()
        {
            return Err("invalid vapor latent energy inventory");
        }
        Ok(Self {
            cell: Cell {
                capacity_kg,
                water_kg,
            },
            latent_j_kg,
            thermal_j,
        })
    }
    pub fn water_kg(&self) -> f64 {
        self.cell.water_kg
    }
    pub fn activity(&self) -> f64 {
        self.cell.water_kg / self.cell.capacity_kg
    }
    pub fn thermal_j(&self) -> f64 {
        self.thermal_j
    }
    pub fn accounted_energy_j(&self) -> f64 {
        self.thermal_j + self.latent_j_kg * self.cell.water_kg
    }
}
impl Body {
    /// Implicit closed exchange with one finite vapor reservoir. Water and latent
    /// energy publish atomically. Insufficient latent heat rejects, allowing the
    /// caller to reduce dt; no water is deleted or silently clamped.
    /// No sensible heat, pressure, supersaturation or temperature evolution.
    pub fn advance_vapor(
        &mut self,
        dt_s: f64,
        vapor: &mut VaporReservoir,
        links: &[VaporLink],
    ) -> Result<VaporTransfer, &'static str> {
        let template = self.prepare_vapor_network(vapor, links)?;
        self.advance_vapor_prepared(dt_s, vapor, template)
    }
    fn prepare_vapor_network(
        &self,
        vapor: &VaporReservoir,
        links: &[VaporLink],
    ) -> Result<Body, &'static str> {
        let n = self.cells.len();
        let mut cells = self.cells.clone();
        cells.push(vapor.cell);
        let mut network_links = self.links.clone();
        for link in links {
            if link.material_cell >= n {
                return Err("invalid vapor material cell");
            }
            network_links.push(Link {
                cells: [link.material_cell, n],
                conductance_kg_s: link.conductance_kg_s,
            });
        }
        Body::new(cells, network_links)
    }
    fn advance_vapor_prepared(
        &mut self,
        dt_s: f64,
        vapor: &mut VaporReservoir,
        mut network: Body,
    ) -> Result<VaporTransfer, &'static str> {
        let n = self.cells.len();
        if network.cells.len() != n + 1 {
            return Err("vapor network owner size changed");
        }
        network.cells[..n].copy_from_slice(&self.cells);
        network.cells[n] = vapor.cell;
        super::validate_cells(&network.cells)?;
        network.advance(dt_s, &[])?;
        let changes: Vec<f64> = network.cells[..n]
            .iter()
            .zip(&self.cells)
            .map(|(a, b)| a.water_kg - b.water_kg)
            .collect();
        let vapor_change = network.cells[n].water_kg - vapor.cell.water_kg;
        let latent = vapor.latent_j_kg * vapor_change;
        let thermal = vapor.thermal_j - latent;
        if !thermal.is_finite()
            || thermal < 0.
            || !latent.is_finite()
            || (latent > 0. && thermal >= vapor.thermal_j)
            || (latent < 0. && thermal <= vapor.thermal_j)
        {
            return Err("insufficient or unrepresentable vapor thermal inventory");
        }
        let mass_defect = changes.iter().sum::<f64>() + vapor_change;
        let mass_scale = self.cells.iter().map(|c| c.water_kg).sum::<f64>() + vapor.cell.water_kg;
        let energy_before = vapor.accounted_energy_j();
        let energy_after = thermal + vapor.latent_j_kg * network.cells[n].water_kg;
        let energy_defect = energy_after - energy_before;
        if !mass_defect.is_finite()
            || !mass_scale.is_finite()
            || !energy_defect.is_finite()
            || mass_defect.abs() > 1e-10 * mass_scale.max(f64::MIN_POSITIVE)
            || energy_defect.abs() > 1e-10 * energy_before.max(f64::MIN_POSITIVE)
        {
            return Err("vapor water or latent energy balance failed");
        }
        self.cells.copy_from_slice(&network.cells[..n]);
        vapor.cell = network.cells[n];
        vapor.thermal_j = thermal;
        Ok(VaporTransfer {
            material_water_change_kg: changes,
            vapor_water_change_kg: vapor_change,
            latent_exchange_j: latent,
            mass_defect_kg: mass_defect,
            energy_defect_j: energy_defect,
        })
    }
}

/// Well-mixed finite thermal store and ideal vapor in a fixed volume.
/// Constant heat capacity and latent heat; no gas motion or condensate droplets.
#[derive(Clone, Debug)]
pub struct ThermalVapor {
    reservoir: VaporReservoir,
    heat_capacity_j_k: f64,
    volume_m3: f64,
    curve: crate::liquid::SaturationCurve,
}
impl ThermalVapor {
    pub fn new(
        temperature_k: f64,
        heat_capacity_j_k: f64,
        volume_m3: f64,
        water_kg: f64,
        curve: crate::liquid::SaturationCurve,
    ) -> Result<Self, &'static str> {
        if !heat_capacity_j_k.is_finite()
            || heat_capacity_j_k <= 0.
            || !volume_m3.is_finite()
            || volume_m3 <= 0.
        {
            return Err("invalid thermal vapor geometry or capacity");
        }
        let capacity = vapor_capacity(curve, temperature_k, volume_m3)?;
        Ok(Self {
            reservoir: VaporReservoir::new(
                capacity,
                water_kg,
                curve.latent_heat,
                heat_capacity_j_k * temperature_k,
            )?,
            heat_capacity_j_k,
            volume_m3,
            curve,
        })
    }
    pub fn temperature_k(&self) -> f64 {
        self.reservoir.thermal_j / self.heat_capacity_j_k
    }
    pub fn water_kg(&self) -> f64 {
        self.reservoir.water_kg()
    }
    pub fn activity(&self) -> f64 {
        self.reservoir.activity()
    }
    pub fn accounted_energy_j(&self) -> f64 {
        self.reservoir.accounted_energy_j()
    }
}
fn vapor_capacity(
    curve: crate::liquid::SaturationCurve,
    temperature: f64,
    volume: f64,
) -> Result<f64, &'static str> {
    let pressure = curve
        .pressure(temperature)
        .map_err(|_| "thermal vapor saturation domain failure")?;
    let capacity = pressure / curve.vapor_gas_constant / temperature * volume;
    if !capacity.is_finite() || capacity <= 0. {
        return Err("thermal vapor capacity overflow");
    }
    Ok(capacity)
}
impl Body {
    /// First-order split: freeze saturation capacity for implicit activity exchange,
    /// then recover temperature from conserved sensible-plus-latent inventory and
    /// recompute capacity. Reject domain violations and supersaturation atomically.
    /// Heat capacity is an effective fixed store; transferred water sensible heat
    /// and solid temperature fields are not included in this approximation.
    pub fn advance_thermal_vapor(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        links: &[VaporLink],
    ) -> Result<VaporTransfer, &'static str> {
        let template = self.prepare_vapor_network(&vapor.reservoir, links)?;
        self.advance_thermal_vapor_prepared(dt_s, vapor, template)
    }
    fn advance_thermal_vapor_prepared(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        template: Body,
    ) -> Result<VaporTransfer, &'static str> {
        let mut next = self.clone();
        let mut gas = vapor.clone();
        let transfer = next.advance_vapor_prepared(dt_s, &mut gas.reservoir, template)?;
        let capacity = vapor_capacity(gas.curve, gas.temperature_k(), gas.volume_m3)?;
        if gas.water_kg() > capacity {
            return Err("thermal vapor supersaturation requires condensate model");
        }
        gas.reservoir.cell.capacity_kg = capacity;
        *self = next;
        *vapor = gas;
        Ok(transfer)
    }
}

/// Lumped material thermal store, with a fixed effective heat capacity.
#[derive(Clone, Debug)]
pub struct MaterialThermalStore {
    heat_capacity_j_k: f64,
    temperature_k: f64,
}
impl MaterialThermalStore {
    pub fn new(heat_capacity_j_k: f64, temperature_k: f64) -> Result<Self, &'static str> {
        if !heat_capacity_j_k.is_finite()
            || heat_capacity_j_k <= 0.
            || !temperature_k.is_finite()
            || temperature_k <= 0.
            || !(heat_capacity_j_k * temperature_k).is_finite()
        {
            return Err("invalid material thermal store");
        }
        Ok(Self {
            heat_capacity_j_k,
            temperature_k,
        })
    }
    pub fn temperature_k(&self) -> f64 {
        self.temperature_k
    }
    pub fn energy_j(&self) -> f64 {
        self.heat_capacity_j_k * self.temperature_k
    }
}
impl ThermalVapor {
    /// Exact two-store conduction with frozen capacities and W/K conductance.
    /// Positive return value is heat received by vapor. Domain/supersaturation
    /// failure restores both stores. No water transfer occurs in this operation.
    pub fn exchange_material_heat(
        &mut self,
        dt_s: f64,
        material: &mut MaterialThermalStore,
        conductance_w_k: f64,
    ) -> Result<f64, &'static str> {
        if !dt_s.is_finite() || dt_s <= 0. || !conductance_w_k.is_finite() || conductance_w_k < 0. {
            return Err("invalid material vapor heat exchange");
        }
        let cg = self.heat_capacity_j_k;
        let cm = material.heat_capacity_j_k;
        let reduced = 1. / (1. / cg + 1. / cm);
        let decay = dt_s * conductance_w_k / reduced;
        if !decay.is_finite() {
            return Err("thermal exchange rate overflow");
        }
        let heat = reduced * (material.temperature_k - self.temperature_k()) * (-(-decay).exp_m1());
        let thermal = self.reservoir.thermal_j + heat;
        let temperature = material.temperature_k - heat / cm;
        let capacity = vapor_capacity(self.curve, thermal / cg, self.volume_m3)?;
        let before = self.accounted_energy_j() + material.energy_j();
        let after = thermal + self.curve.latent_heat * self.water_kg() + cm * temperature;
        if !heat.is_finite()
            || !thermal.is_finite()
            || !temperature.is_finite()
            || temperature <= 0.
            || self.water_kg() > capacity
            || !before.is_finite()
            || !after.is_finite()
            || (after - before).abs() > 1e-12 * before.abs().max(f64::MIN_POSITIVE)
            || (heat != 0.
                && (thermal == self.reservoir.thermal_j || temperature == material.temperature_k))
        {
            return Err("material vapor heat balance or capacity failure");
        }
        self.reservoir.thermal_j = thermal;
        self.reservoir.cell.capacity_kg = capacity;
        material.temperature_k = temperature;
        Ok(heat)
    }
}

impl Body {
    /// Closed lumped material/gas enthalpy exchange. Both current heat capacities
    /// include their existing water; water has the same constant specific heat
    /// in both phases. Incoming sensible enthalpy uses the donor's initial T.
    /// Evaporation draws latent heat from material; condensation returns it.
    /// Activity transport freezes gas capacity for this first-order split step.
    /// Spatial temperatures, distinct phase heat capacities and droplets remain
    /// outside this model. Invalid post-transfer capacity/domain rolls back all.
    pub fn advance_enthalpy_vapor(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        material: &mut MaterialThermalStore,
        water_specific_heat_j_kg_k: f64,
        links: &[VaporLink],
    ) -> Result<VaporTransfer, &'static str> {
        let cp = water_specific_heat_j_kg_k;
        if !cp.is_finite() || cp <= 0. {
            return Err("invalid exchanged water specific heat");
        }
        let material_water = self.cells.iter().map(|cell| cell.water_kg).sum::<f64>();
        let gas_dry_capacity = vapor.heat_capacity_j_k - cp * vapor.water_kg();
        let material_dry_capacity = material.heat_capacity_j_k - cp * material_water;
        if !material_water.is_finite()
            || !gas_dry_capacity.is_finite()
            || gas_dry_capacity <= 0.
            || !material_dry_capacity.is_finite()
            || material_dry_capacity <= 0.
        {
            return Err("water heat capacity exceeds effective thermal store");
        }
        let mut next = self.clone();
        let mut gas = vapor.clone();
        // Reuse the implicit mass network; physical enthalpy is checked below.
        let n = next.cells.len();
        let mut cells = next.cells.clone();
        cells.push(vapor.reservoir.cell);
        let mut network_links = next.links.clone();
        for link in links {
            if link.material_cell >= n {
                return Err("invalid vapor material cell");
            }
            network_links.push(Link {
                cells: [link.material_cell, n],
                conductance_kg_s: link.conductance_kg_s,
            });
        }
        let mut network = Body::new(cells, network_links)?;
        network.advance(dt_s, &[])?;
        let delta = network.cells[n].water_kg - vapor.water_kg();
        let changes: Vec<_> = network.cells[..n]
            .iter()
            .zip(&self.cells)
            .map(|(a, b)| a.water_kg - b.water_kg)
            .collect();
        let gas_activity = network.cells[n].water_kg / network.cells[n].capacity_kg;
        let mut sensible = 0.;
        let mut flux_sum = 0.;
        let mut flux_scale = 0.;
        for link in links {
            let cell = network.cells[link.material_cell];
            let flux =
                dt_s * link.conductance_kg_s * (cell.water_kg / cell.capacity_kg - gas_activity);
            let donor_t = if flux >= 0. {
                material.temperature_k
            } else {
                vapor.temperature_k()
            };
            sensible += flux * cp * donor_t;
            flux_sum += flux;
            flux_scale += flux.abs();
        }
        if !sensible.is_finite()
            || !flux_sum.is_finite()
            || !flux_scale.is_finite()
            || (flux_sum - delta).abs()
                > 1e-10 * (flux_scale + vapor.water_kg()).max(f64::MIN_POSITIVE)
        {
            return Err("thermal vapor link flux balance failure");
        }
        let latent = delta * vapor.curve.latent_heat;
        let cg = vapor.heat_capacity_j_k + delta * cp;
        let cm = material.heat_capacity_j_k - delta * cp;
        let eg = vapor.reservoir.thermal_j + sensible;
        let em = material.energy_j() - sensible - latent;
        if [cg, cm, eg, em, em / cm]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.)
        {
            return Err("invalid post-transfer thermal inventory");
        }
        let capacity = vapor_capacity(vapor.curve, eg / cg, vapor.volume_m3)?;
        if network.cells[n].water_kg > capacity {
            return Err("thermal vapor supersaturation requires condensate model");
        }
        let before = vapor.accounted_energy_j() + material.energy_j();
        let after = eg + vapor.curve.latent_heat * network.cells[n].water_kg + em;
        let energy_defect = after - before;
        let mass_defect = changes.iter().sum::<f64>() + delta;
        let water_scale = self.cells.iter().map(|c| c.water_kg).sum::<f64>() + vapor.water_kg();
        if !before.is_finite()
            || !after.is_finite()
            || !mass_defect.is_finite()
            || energy_defect.abs() > 1e-12 * before.abs().max(f64::MIN_POSITIVE)
            || mass_defect.abs() > 1e-10 * water_scale.max(f64::MIN_POSITIVE)
        {
            return Err("closed enthalpy vapor balance failure");
        }
        next.cells.copy_from_slice(&network.cells[..n]);
        gas.reservoir.cell = Cell {
            capacity_kg: capacity,
            water_kg: network.cells[n].water_kg,
        };
        gas.reservoir.thermal_j = eg;
        gas.heat_capacity_j_k = cg;
        *self = next;
        *vapor = gas;
        material.heat_capacity_j_k = cm;
        material.temperature_k = em / cm;
        Ok(VaporTransfer {
            material_water_change_kg: changes,
            vapor_water_change_kg: delta,
            latent_exchange_j: latent,
            mass_defect_kg: mass_defect,
            energy_defect_j: energy_defect,
        })
    }
}

impl Body {
    /// Atomic split interval: half conduction, full donor-enthalpy mass exchange,
    /// half conduction with updated capacities. The mass solver remains first
    /// order; symmetric placement of heat does not make this a second-order solve.
    pub fn advance_heated_vapor(
        &mut self,
        dt_s: f64,
        vapor: &mut ThermalVapor,
        material: &mut MaterialThermalStore,
        water_specific_heat_j_kg_k: f64,
        conductance_w_k: f64,
        links: &[VaporLink],
    ) -> Result<(VaporTransfer, f64), &'static str> {
        let mut next = self.clone();
        let mut gas = vapor.clone();
        let mut solid = material.clone();
        let first = gas.exchange_material_heat(0.5 * dt_s, &mut solid, conductance_w_k)?;
        let transfer = next.advance_enthalpy_vapor(
            dt_s,
            &mut gas,
            &mut solid,
            water_specific_heat_j_kg_k,
            links,
        )?;
        let last = gas.exchange_material_heat(0.5 * dt_s, &mut solid, conductance_w_k)?;
        let heat = first + last;
        let before = vapor.accounted_energy_j() + material.energy_j();
        let after = gas.accounted_energy_j() + solid.energy_j();
        if !heat.is_finite()
            || !before.is_finite()
            || !after.is_finite()
            || (after - before).abs() > 1e-12 * before.abs().max(f64::MIN_POSITIVE)
        {
            return Err("heated vapor interval energy balance failure");
        }
        *self = next;
        *vapor = gas;
        *material = solid;
        Ok((transfer, heat))
    }
}

impl MaterialThermalStore {
    /// Deposit nonnegative heat into the fixed-capacity material store.
    /// Reject unrepresentable positive increments without modifying the store.
    pub fn deposit_heat(&mut self, heat_j: f64) -> Result<(), &'static str> {
        let before = self.energy_j();
        let after = before + heat_j;
        let temperature = after / self.heat_capacity_j_k;
        if !heat_j.is_finite()
            || heat_j < 0.
            || !after.is_finite()
            || !temperature.is_finite()
            || temperature <= 0.
            || (heat_j > 0. && (after <= before || temperature <= self.temperature_k))
        {
            return Err("invalid or unrepresentable deposited material heat");
        }
        self.temperature_k = temperature;
        Ok(())
    }
}

mod adaptive;
pub use adaptive::{ThermalVaporAccuracy, ThermalVaporStep};
