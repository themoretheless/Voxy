//! Transactional fixed-point coupling of backward-Euler storage and elastic FEM.
use super::{Body, DarcyResponse, Equilibrium, Matrix, PoreReservoir};

#[derive(Clone, Copy, Debug)]
pub struct ImplicitPoreConfig {
    pub outer_iterations: usize,
    pub pressure_tolerance_pa: f64,
    pub relaxation: f64,
    pub solid_iterations: usize,
    pub solid_tolerance_n: f64,
}
#[derive(Clone, Debug)]
pub struct ImplicitPoreReport {
    pub iterations: usize,
    pub pressure_residual_pa: f64,
    pub solid: Equilibrium,
    pub flow: DarcyResponse,
}
impl Body {
    /// Commit elastic geometry, pore inventories and conservative protein together.
    /// Protein is indexed by tetrahedral cell and measured in kilograms.
    /// # Errors
    /// Any storage, solid or protein failure leaves both caller states unchanged.
    pub fn implicit_cell_pore_protein_step(
        &mut self,
        protein_kg: &mut [f64],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        let mut trial = self.clone();
        let report =
            trial.implicit_cell_pore_step(permeability, viscosity_pa_s, seconds, config)?;
        let model = trial.deformed_darcy(permeability, viscosity_pa_s)?;
        let volumes: Vec<_> = trial
            .cell_pore_fluids
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect();
        let next_protein = model.implicit_protein_step(
            protein_kg,
            &volumes,
            &report.flow.face_flows_m3_per_s,
            seconds,
        )?;
        // Input/output lengths were checked by the protein solve. No fallible
        // operation remains after this joint commit boundary.
        protein_kg.copy_from_slice(&next_protein);
        *self = trial;
        Ok(report)
    }
    /// Sealed backward-Euler pore transport coupled to current-geometry elastic FEM.
    /// Every iterate solves solid equilibrium with frozen material histories.
    /// Commit occurs only when both force and cell-storage equations converge.
    /// # Errors
    /// Rejects invalid controls, absent stores, viscous materials, bad permeability,
    /// invalid trial inventories or unconverged pressure/mechanics. Failure is atomic.
    pub fn implicit_cell_pore_step(
        &mut self,
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        self.implicit_pore_core(
            permeability,
            viscosity_pa_s,
            seconds,
            config,
            None,
            &[],
            &[],
        )
        .map(|(r, _)| r)
    }
    /// Joint elastic geometry, cell fluid/protein and finite-reservoir transaction.
    /// Explicit ports keep their vertex identities while their geometry deforms.
    /// # Errors
    /// Any solve/validation failure preserves body, cell protein and reservoir.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_cell_pore_reservoir_step(
        &mut self,
        protein_kg: &mut [f64],
        reservoir: &mut PoreReservoir,
        ports: &[[usize; 3]],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        let initial = *reservoir;
        let mut trial = self.clone();
        let (report, next) = trial.implicit_pore_core(
            permeability,
            viscosity_pa_s,
            seconds,
            config,
            Some((&initial, ports)),
            &[],
            &[],
        )?;
        let mut next = next.ok_or("missing implicit pore reservoir")?;
        let next_pressure = next.pressure_pa()?;
        let boundaries: Vec<_> = ports.iter().map(|p| (*p, next_pressure)).collect();
        let model =
            trial.deformed_darcy_with_boundaries(permeability, viscosity_pa_s, &boundaries)?;
        let volumes: Vec<_> = trial
            .cell_pore_fluids
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect();
        let (protein, reservoir_protein) = model.implicit_protein_reservoir_step(
            protein_kg,
            &volumes,
            &report.flow.face_flows_m3_per_s,
            seconds,
            initial.protein_kg,
            next.fluid_volume_m3,
        )?;
        next.protein_kg = reservoir_protein;
        next.pressure_pa()?;
        protein_kg.copy_from_slice(&protein);
        *reservoir = next;
        *self = trial;
        Ok(report)
    }
    /// Elastic tissue/storage solve at prescribed exterior pressures with protein.
    /// Ports are explicit boundary triangles; unlisted exterior faces are sealed.
    /// The supplied homogeneous exterior concentration is kg/m³. Inflow uses it;
    /// outflow uses the accepted tissue concentration. Exterior inventory is owned
    /// by the caller, which must account for the returned fluxes and mass change.
    /// # Errors
    /// Failure preserves geometry, fluid inventories and caller protein.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_cell_pore_boundary_protein_step(
        &mut self,
        protein_kg: &mut [f64],
        boundaries: &[([usize; 3], f64)],
        exterior_concentration_kg_per_m3: f64,
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        self.implicit_cell_pore_boundary_protein_step_with_concentrations(
            protein_kg,
            boundaries,
            &vec![exterior_concentration_kg_per_m3; boundaries.len()],
            permeability,
            viscosity_pa_s,
            seconds,
            config,
        )
    }
    /// Pressure-controlled exchange with a separate exterior concentration per port.
    /// Concentrations match the input boundary order, not assembled face order.
    /// Inflow uses the specified port concentration; outflow uses tissue protein.
    /// # Errors
    /// Invalid port/concentration assignments or any failed solve preserve state.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_cell_pore_boundary_protein_step_with_concentrations(
        &mut self,
        protein_kg: &mut [f64],
        boundaries: &[([usize; 3], f64)],
        exterior_concentrations_kg_per_m3: &[f64],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        self.implicit_cell_pore_boundary_protein_step_with_resistances(
            protein_kg,
            boundaries,
            exterior_concentrations_kg_per_m3,
            &[],
            permeability,
            viscosity_pa_s,
            seconds,
            config,
        )
    }
    /// Per-port pressure/concentration exchange through passive series resistance.
    /// Resistances are explicit boundary triangles and values in Pa s/m³.
    /// # Errors
    /// Invalid assignments or any failed subsolve preserve tissue and protein.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_cell_pore_boundary_protein_step_with_resistances(
        &mut self,
        protein_kg: &mut [f64],
        boundaries: &[([usize; 3], f64)],
        exterior_concentrations_kg_per_m3: &[f64],
        resistances: &[([usize; 3], f64)],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        let membranes: Vec<_> = exterior_concentrations_kg_per_m3
            .iter()
            .map(|c| super::ProteinMembrane {
                concentration_kg_per_m3: *c,
                reflection: 0.,
                diffusive_conductance_m3_per_s: 0.,
            })
            .collect();
        self.implicit_cell_pore_boundary_protein_step_with_membranes(
            protein_kg,
            boundaries,
            &membranes,
            resistances,
            permeability,
            viscosity_pa_s,
            seconds,
            config,
        )
    }
    /// Deforming pressure-controlled tissue with selective per-port protein membranes.
    /// # Errors
    /// Invalid membrane assignments or failed mechanics/transport preserve all state.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_cell_pore_boundary_protein_step_with_membranes(
        &mut self,
        protein_kg: &mut [f64],
        boundaries: &[([usize; 3], f64)],
        membranes: &[super::ProteinMembrane],
        resistances: &[([usize; 3], f64)],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
    ) -> Result<ImplicitPoreReport, &'static str> {
        if membranes.len() != boundaries.len()
            || membranes
                .iter()
                .any(|m| !m.concentration_kg_per_m3.is_finite() || m.concentration_kg_per_m3 < 0.)
        {
            return Err("invalid pore port concentration assignment");
        }
        let mut concentrations_by_port = std::collections::BTreeMap::new();
        for ((nodes, _), concentration) in boundaries.iter().zip(membranes) {
            let mut key = *nodes;
            key.sort_unstable();
            if concentrations_by_port.insert(key, *concentration).is_some() {
                return Err("duplicate pore protein port");
            }
        }
        let mut trial = self.clone();
        let (report, _) = trial.implicit_pore_core(
            permeability,
            viscosity_pa_s,
            seconds,
            config,
            None,
            boundaries,
            resistances,
        )?;
        let model = trial
            .deformed_darcy_with_boundaries(permeability, viscosity_pa_s, boundaries)?
            .with_added_boundary_resistances(resistances)?;
        let concentrations: Vec<_> = model
            .faces()
            .iter()
            .map(|f| {
                let mut key = f.nodes;
                key.sort_unstable();
                concentrations_by_port.get(&key).copied()
            })
            .collect();
        let volumes: Vec<_> = trial
            .cell_pore_fluids
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect();
        let protein = model.implicit_protein_step_with_membranes(
            protein_kg,
            &volumes,
            &report.flow.face_flows_m3_per_s,
            seconds,
            &concentrations,
        )?;
        protein_kg.copy_from_slice(&protein);
        *self = trial;
        Ok(report)
    }
    fn implicit_pore_core(
        &mut self,
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        config: ImplicitPoreConfig,
        reservoir: Option<(&PoreReservoir, &[[usize; 3]])>,
        prescribed: &[([usize; 3], f64)],
        resistances: &[([usize; 3], f64)],
    ) -> Result<(ImplicitPoreReport, Option<PoreReservoir>), &'static str> {
        if self.cell_pore_fluids.is_empty()
            || self.elements.iter().any(|e| e.viscoelastic.is_some())
            || config.outer_iterations == 0
            || config.outer_iterations > 10_000
            || !config.pressure_tolerance_pa.is_finite()
            || config.pressure_tolerance_pa <= 0.
            || !config.relaxation.is_finite()
            || config.relaxation <= 0.
            || config.relaxation > 1.
            || config.solid_iterations == 0
            || config.solid_iterations > 100_000
            || !config.solid_tolerance_n.is_finite()
            || config.solid_tolerance_n <= 0.
            || !seconds.is_finite()
            || seconds <= 0.
        {
            return Err("invalid implicit pore coupling");
        }
        let old: Vec<_> = self
            .cell_pore_fluids
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect();
        let storage: Vec<_> = self
            .cell_pore_fluids
            .iter()
            .map(|f| f.storage_m3_per_pa)
            .collect();
        let reservoir_pressure = match reservoir {
            Some((r, _)) => r.pressure_pa()?,
            None => 0.,
        };
        let boundaries: Vec<_> = reservoir.map_or_else(
            || prescribed.to_vec(),
            |(_, ports)| ports.iter().map(|p| (*p, reservoir_pressure)).collect(),
        );
        let mut old_pressure = vec![0.0; old.len()];
        let mut trial = self.clone();
        for iteration in 0..config.outer_iterations {
            let solid = trial.equilibrate(config.solid_iterations, config.solid_tolerance_n)?;
            if !solid.converged {
                return Err("implicit pore solid did not converge");
            }
            let current_pressure = trial.cell_pore_response_at(trial.positions())?.0;
            // Reconstruct old inventory pressure at the CURRENT solid volume.
            for ((((dst, p), f), old), s) in old_pressure
                .iter_mut()
                .zip(&current_pressure)
                .zip(&trial.cell_pore_fluids)
                .zip(&old)
                .zip(&storage)
            {
                *dst = p + (old - f.fluid_volume_m3) / s;
            }
            let model = trial
                .deformed_darcy_with_boundaries(permeability, viscosity_pa_s, &boundaries)?
                .with_added_boundary_resistances(resistances)?;
            let (next_pressure, next_reservoir_pressure, flow) = if let Some((r, _)) = reservoir {
                model.implicit_reservoir_step(
                    &old_pressure,
                    &storage,
                    seconds,
                    reservoir_pressure,
                    r.compliance_m3_per_pa,
                )?
            } else {
                let (p, q) = model.implicit_storage_step(&old_pressure, &storage, seconds)?;
                (p, 0., q)
            };
            let transferred: f64 = trial
                .cell_pore_fluids
                .iter()
                .zip(&old)
                .map(|(f, v)| f.fluid_volume_m3 - v)
                .sum();
            let mut residual = current_pressure
                .iter()
                .zip(&next_pressure)
                .map(|(a, b)| (a - b).abs())
                .fold(0_f64, f64::max);
            if let Some((r, _)) = reservoir {
                let current_reservoir_pressure =
                    reservoir_pressure - transferred / r.compliance_m3_per_pa;
                residual =
                    residual.max((current_reservoir_pressure - next_reservoir_pressure).abs());
            }
            if residual <= config.pressure_tolerance_pa {
                let next_reservoir = if let Some((r, _)) = reservoir {
                    let mut next = *r;
                    next.fluid_volume_m3 -= transferred;
                    next.pressure_pa()?;
                    Some(next)
                } else {
                    None
                };
                *self = trial;
                return Ok((
                    ImplicitPoreReport {
                        iterations: iteration + 1,
                        pressure_residual_pa: residual,
                        solid,
                        flow,
                    },
                    next_reservoir,
                ));
            }
            let mut fluids = trial.cell_pore_fluids.clone();
            for (((f, old), q), s) in fluids
                .iter_mut()
                .zip(&old)
                .zip(&flow.cell_outflows_m3_per_s)
                .zip(&storage)
            {
                let target = old - seconds * q;
                f.fluid_volume_m3 += config.relaxation * (target - f.fluid_volume_m3);
                if !f.fluid_volume_m3.is_finite() || !s.is_finite() {
                    return Err("implicit pore trial overflow");
                }
            }
            trial.set_cell_pore_fluids(fluids)?;
        }
        Err("implicit pore pressure did not converge")
    }
}
