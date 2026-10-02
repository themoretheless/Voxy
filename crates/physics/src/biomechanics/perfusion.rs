//! Finite hydraulic reservoirs coupled to cell-resolved porous FEM tissues.
//! Generic carrier-fluid transport, not a calibrated blood microcirculation model.
use super::{Body, CellPoreTissue, Matrix};
use crate::lymph::{Exchange, ExchangeReport, FluidSpace, LymphNetwork};
/// Explicit cell solute inventory and linear osmotic coefficient; no protein defaults.
#[derive(Clone, Copy, Debug)]
pub struct PerfusionCellSolute {
    pub protein_kg: f64,
    pub oncotic_pa_per_kg_m3: f64,
}
/// Passive bidirectional hydraulic/protein exchange. Orientation sets ledger sign.
#[derive(Clone, Copy, Debug)]
pub struct PerfusionPort {
    pub tissue_cell: usize,
    pub reservoir: usize,
    pub reservoir_to_tissue: bool,
    pub hydraulic_m3_per_pa_s: f64,
    pub reflection: f64,
    pub protein_permeability_m3_per_s: f64,
}
#[derive(Clone, Debug)]
pub struct PorePerfusion {
    tissue: CellPoreTissue,
    network: LymphNetwork,
    permeability: Vec<Matrix>,
    viscosity: f64,
    reservoir_nodes: Vec<usize>,
    port_edges: Vec<usize>,
}
impl PorePerfusion {
    /// Construct a closed total-inventory system: each FEM cell plus finite
    /// external reservoirs. Interior faces use current-geometry RT0 Darcy flux.
    /// # Errors
    /// Missing cell stores, viscoelastic cells, invalid permeability/ports/reservoirs
    /// or solver controls. Reservoir indices refer to the supplied reservoir array.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        body: Body,
        permeability: Vec<Matrix>,
        viscosity: f64,
        cell_solute: Vec<PerfusionCellSolute>,
        reservoirs: Vec<FluidSpace>,
        ports: Vec<PerfusionPort>,
        iterations: usize,
        tolerance_n: f64,
    ) -> Result<Self, &'static str> {
        let n = body.elements().len();
        if reservoirs.is_empty()
            || ports.is_empty()
            || body.cell_pore_fluids().len() != n
            || cell_solute.len() != n
        {
            return Err("invalid pore perfusion stores");
        }
        let model = body.deformed_darcy(&permeability, viscosity)?;
        let mut spaces: Vec<_> = body
            .cell_pore_fluids()
            .iter()
            .zip(&cell_solute)
            .map(|(f, solute)| FluidSpace {
                reference_volume_m3: f.reference_fluid_volume_m3,
                initial_volume_m3: f.fluid_volume_m3,
                initial_protein_kg: solute.protein_kg,
                reference_pressure_pa: 0.,
                compliance_m3_per_pa: f.storage_m3_per_pa,
                oncotic_pa_per_kg_m3: solute.oncotic_pa_per_kg_m3,
            })
            .collect();
        let reservoir_nodes: Vec<_> = (n..n + reservoirs.len()).collect();
        spaces.extend(reservoirs);
        let edge = |from, to, hydraulic, reflection, protein| Exchange {
            from,
            to,
            hydraulic_m3_per_pa_s: hydraulic,
            reflection,
            protein_permeability_m3_per_s: protein,
            pump_head_pa: 0.,
            valve: false,
        };
        let mut edges = Vec::new();
        for face in model.faces() {
            let neighbor = face
                .neighbor
                .ok_or("perfusion requires sealed exterior Darcy faces")?;
            edges.push(edge(face.owner, neighbor, 0., 0., 0.));
        }
        let mut port_edges = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for port in ports {
            if port.tissue_cell >= n
                || port.reservoir >= reservoir_nodes.len()
                || !seen.insert((port.tissue_cell, port.reservoir))
            {
                return Err("invalid or duplicate perfusion port");
            }
            let reservoir = reservoir_nodes[port.reservoir];
            let (from, to) = if port.reservoir_to_tissue {
                (reservoir, port.tissue_cell)
            } else {
                (port.tissue_cell, reservoir)
            };
            port_edges.push(edges.len());
            edges.push(edge(
                from,
                to,
                port.hydraulic_m3_per_pa_s,
                port.reflection,
                port.protein_permeability_m3_per_s,
            ));
        }
        let network = LymphNetwork::new(spaces, edges)?;
        let tissue = CellPoreTissue::new(body, (0..n).collect(), iterations, tolerance_n)?;
        Ok(Self {
            tissue,
            network,
            permeability,
            viscosity,
            reservoir_nodes,
            port_edges,
        })
    }
    /// Atomic fluid/protein transport and elastic equilibrium. No reset/prescribed
    /// tissue inventory: finite reservoir pressures drive actual exchange flux.
    /// # Errors
    /// Invalid step, positivity/transport or FEM failure preserves both states.
    pub fn step(&mut self, seconds: f64, max_step: f64) -> Result<ExchangeReport, &'static str> {
        self.tissue.step_mixed_darcy(
            &mut self.network,
            &self.permeability,
            self.viscosity,
            seconds,
            max_step,
        )
    }
    /// Conservative SSPRK2 with mechanical equilibrium at both transport stages.
    /// # Errors
    /// Any failed predictor, transport or equilibrium leaves both states intact.
    pub fn step_second_order(
        &mut self,
        seconds: f64,
        max_step: f64,
    ) -> Result<ExchangeReport, &'static str> {
        self.tissue.step_mixed_darcy_second_order(
            &mut self.network,
            &self.permeability,
            self.viscosity,
            seconds,
            max_step,
        )
    }
    /// Step-doubling SSPRK2 controls water, protein and mechanical position error.
    /// Accepts two half steps; the complete requested interval commits atomically.
    /// # Errors
    /// Invalid tolerances, trial budget exhaustion or failed solve preserves state.
    pub fn step_adaptive(
        &mut self,
        seconds: f64,
        config: super::AdaptiveTissueExchangeConfig,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        if !config.absolute_position_tolerance_m.is_finite()
            || config.absolute_position_tolerance_m <= 0.
        {
            return Err("invalid adaptive perfusion position tolerance");
        }
        let (next, report) = crate::lymph::adaptive_exchange(
            self,
            seconds,
            config.exchange,
            |state, h| state.step_second_order(h, h),
            |state| state.network(),
            |coarse, fine| {
                let mut error = 0_f64;
                for (a, b) in coarse
                    .body()
                    .positions()
                    .iter()
                    .zip(fine.body().positions())
                {
                    let distance = a
                        .iter()
                        .zip(b)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    if !distance.is_finite() {
                        return Err("adaptive perfusion geometry overflow");
                    }
                    error = error.max(distance / (3. * config.absolute_position_tolerance_m));
                }
                Ok(error)
            },
        )?;
        *self = next;
        Ok(report)
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        self.tissue.body()
    }
    #[must_use]
    pub fn network(&self) -> &LymphNetwork {
        &self.network
    }
    #[must_use]
    pub fn reservoir_nodes(&self) -> &[usize] {
        &self.reservoir_nodes
    }
    #[must_use]
    pub fn port_edges(&self) -> &[usize] {
        &self.port_edges
    }
}
