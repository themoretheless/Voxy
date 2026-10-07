//! Spatial finite pure-vapor exchange using the existing interfacial solver.
use super::{Error, FiniteDropletGasGrid, Liquid, VaporExchangeAccuracy, VaporInterface, positive};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpatialVaporExchangeReport {
    /// Signed mass entering vapor (kg); negative for condensation.
    pub vapor_mass_change_kg: f64,
    pub exchanged_particles: usize,
    pub affected_cells: usize,
}
/// Explicit controls for the composed pure-vapor stage.
#[derive(Clone, Copy, Debug)]
pub struct SpatialVaporTransportControl {
    pub accuracy: VaporExchangeAccuracy,
    pub max_exchanges: usize,
    pub flow: super::GasGridFlowControl,
    pub heat: super::GasGridHeatControl,
}
impl Liquid {
    /// Atomic split stage: gas Euler transport, heat conduction, then phase exchange.
    /// All interfaces must describe the same pure vapor and the flow EOS gas constant.
    /// Particle motion and interface area remain caller-owned; this is first-order splitting.
    /// Any late phase error rolls back the earlier gas flow and thermal stages as well.
    pub fn exchange_vapor_grid_with_transport(
        &mut self,
        grid: &mut FiniteDropletGasGrid,
        exchanges: &[(usize, VaporInterface)],
        dt: f64,
        control: SpatialVaporTransportControl,
    ) -> Result<
        (
            SpatialVaporExchangeReport,
            super::GasGridFlowReport,
            super::GasGridHeatReport,
        ),
        Error,
    > {
        let SpatialVaporTransportControl {
            accuracy,
            max_exchanges,
            flow,
            heat,
        } = control;
        if exchanges.len() > max_exchanges {
            return Err(Error::PairBudget);
        }
        if exchanges.iter().any(|(_, interface)| {
            interface.curve.vapor_gas_constant != flow.gas_constant
                || exchanges.first().is_some_and(|(_, first)| {
                    first.curve.latent_heat != interface.curve.latent_heat
                })
        }) {
            return Err(Error::InvalidConfig);
        }
        let mut liquid = self.clone();
        let mut gas = grid.clone();
        let flow_report = gas.advance_euler(dt, flow)?;
        let heat_report = gas.conduct_heat(dt, heat)?;
        let phase_report =
            liquid.exchange_vapor_grid(&mut gas, exchanges, dt, accuracy, max_exchanges)?;
        *self = liquid;
        *grid = gas;
        Ok((phase_report, flow_report, heat_report))
    }

    /// Exchange explicitly exposed particles with their containing pure-vapor cells.
    /// Membership stays fixed for this stage; repeated particles and outside positions reject.
    /// Exchanges sharing a cell follow caller order (operator splitting, not a cloud equilibrium solve).
    /// No carrier-air composition, advection, nucleation or exposed-area inference is added.
    /// Fluid and every cell publish together, including after a late interfacial failure.
    pub fn exchange_vapor_grid(
        &mut self,
        grid: &mut FiniteDropletGasGrid,
        exchanges: &[(usize, VaporInterface)],
        dt: f64,
        accuracy: VaporExchangeAccuracy,
        max_exchanges: usize,
    ) -> Result<SpatialVaporExchangeReport, Error> {
        if !positive(dt)
            || max_exchanges == 0
            || !positive(accuracy.relative_tolerance)
            || !positive(accuracy.mass_tolerance)
            || !positive(accuracy.temperature_tolerance)
            || accuracy.max_attempts == 0
        {
            return Err(Error::InvalidConfig);
        }
        if exchanges.len() > max_exchanges {
            return Err(Error::PairBudget);
        }
        let mut selected = Vec::with_capacity(exchanges.len());
        let mut particles = std::collections::BTreeSet::new();
        let mut cells = std::collections::BTreeSet::new();
        for &(index, interface) in exchanges {
            let particle = self.particles.get(index).ok_or(Error::InvalidParticle)?;
            if !particles.insert(index) {
                return Err(Error::InvalidConfig);
            }
            let cell = grid
                .cell_index(particle.position)?
                .ok_or(Error::InvalidConfig)?;
            // Validate interface/state before cloning or advancing earlier pairs.
            let field = self
                .fields()
                .and_then(|fields| fields.get(index))
                .ok_or(Error::InvalidTransport)?;
            interface.mass_flux(field.temperature, grid.cells[cell])?;
            selected.push((index, cell, interface));
            cells.insert(cell);
        }
        let mut candidate = self.clone();
        let mut gas = grid.clone();
        let mut report = SpatialVaporExchangeReport {
            affected_cells: cells.len(),
            ..Default::default()
        };
        for (index, cell, interface) in selected {
            report.vapor_mass_change_kg +=
                candidate.exchange_vapor(index, &mut gas.cells[cell], interface, dt, accuracy)?;
            report.exchanged_particles += 1;
        }
        if !report.vapor_mass_change_kg.is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        *grid = gas;
        Ok(report)
    }
}
