//! Cell-resolved finite-volume pore storage coupled to tetrahedral mechanics.
use super::{Body, PoreFluid, columns, det, dot, mm, sub};
impl Body {
    /// Assign independent fluid inventories to every tetrahedron. Replaces the
    /// uniform store. Each fluid node stores a reference-density-equivalent volume.
    /// # Errors
    /// Rejects wrong cell count, invalid storage/porosity or energy overflow.
    pub fn set_cell_pore_fluids(&mut self, fluids: Vec<PoreFluid>) -> Result<(), &'static str> {
        if fluids.len() != self.elements.len() || fluids.is_empty() {
            return Err("invalid cell pore count");
        }
        for (e, fluid) in self.elements.iter().zip(&fluids) {
            validate_fluid(*fluid, e.volume)?;
        }
        let mut trial = self.clone();
        trial.pore_fluid = None;
        trial.cell_pore_fluids = fluids;
        trial.pore_fields_at(trial.positions())?;
        *self = trial;
        Ok(())
    }
    #[must_use]
    pub fn cell_pore_fluids(&self) -> &[PoreFluid] {
        &self.cell_pore_fluids
    }
    /// Signed pore pressures and total storage energy (Pa, J).
    /// # Errors
    /// Rejects missing cell stores, inverted geometry or overflowing storage.
    pub fn cell_pore_response_at(&self, x: &[[f64; 3]]) -> Result<(Vec<f64>, f64), &'static str> {
        if self.cell_pore_fluids.len() != self.elements.len()
            || self.cell_pore_fluids.is_empty()
            || x.len() != self.positions.len()
            || x.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid cell pore state");
        }
        let mut pressures = Vec::with_capacity(self.elements.len());
        let mut energy = 0.;
        for (e, fluid) in self.elements.iter().zip(&self.cell_pore_fluids) {
            let [a, b, c, d] = e.nodes.map(|i| x[i]);
            let j = det(mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest));
            if !j.is_finite() || j <= 0. {
                return Err("invalid cell pore geometry");
            }
            let content = fluid.fluid_volume_m3
                - fluid.reference_fluid_volume_m3
                - fluid.biot_coefficient * e.volume * (j - 1.);
            let p = content / fluid.storage_m3_per_pa;
            energy += 0.5 * p * content;
            if !p.is_finite() || !energy.is_finite() {
                return Err("cell pore overflow");
            }
            pressures.push(p);
        }
        Ok((pressures, energy))
    }
    pub(super) fn pore_fields_at(&self, x: &[[f64; 3]]) -> Result<(Vec<f64>, f64), &'static str> {
        if self.cell_pore_fluids.is_empty() {
            let (p, energy) = self.pore_response_at(x)?;
            let alpha = self.pore_fluid.map_or(0., |f| f.biot_coefficient);
            Ok((vec![alpha * p; self.elements.len()], energy))
        } else {
            let (mut p, energy) = self.cell_pore_response_at(x)?;
            for (pressure, fluid) in p.iter_mut().zip(&self.cell_pore_fluids) {
                *pressure *= fluid.biot_coefficient;
            }
            Ok((p, energy))
        }
    }
    pub(super) fn pore_preconditioner(&self) -> Result<Vec<f64>, &'static str> {
        let mut diagonal = self.diagonal.clone();
        for bond in &self.tissue_bonds {
            for node in bond.nodes {
                diagonal[node] += bond.stiffness_n_m;
            }
        }
        for gap in &self.tissue_gaps {
            for node in gap.nodes {
                diagonal[node] += gap.stiffness_n_m;
            }
        }
        let reference_volume = self.reference_volume();
        for (index, e) in self.elements.iter().enumerate() {
            let store = if self.cell_pore_fluids.is_empty() {
                self.pore_fluid.map(|p| (p, reference_volume))
            } else {
                Some((self.cell_pore_fluids[index], e.volume))
            };
            if let Some((pore, volume)) = store {
                let stiffness = pore.biot_coefficient.powi(2) * volume / pore.storage_m3_per_pa;
                for k in 0..4 {
                    diagonal[e.nodes[k]] +=
                        e.volume * stiffness * dot(e.gradients[k], e.gradients[k]);
                }
            }
        }
        if diagonal.iter().any(|v| !v.is_finite()) {
            return Err("pore preconditioner overflow");
        }
        Ok(diagonal)
    }
}
pub(super) fn validate_fluid(fluid: PoreFluid, reference_volume: f64) -> Result<(), &'static str> {
    if !fluid.reference_fluid_volume_m3.is_finite()
        || fluid.reference_fluid_volume_m3 <= 0.
        || fluid.reference_fluid_volume_m3 > reference_volume
        || !fluid.fluid_volume_m3.is_finite()
        || fluid.fluid_volume_m3 <= 0.
        || !fluid.biot_coefficient.is_finite()
        || !(0. ..=1.).contains(&fluid.biot_coefficient)
        || !fluid.storage_m3_per_pa.is_finite()
        || fluid.storage_m3_per_pa <= 0.
    {
        return Err("invalid pore fluid storage");
    }
    Ok(())
}

/// Spatial fluid inventory per elastic tetrahedron, mapped to distinct network nodes.
/// No viscous history update; both geometry and network commit transactionally.
/// Explicit interstitial-pressure attachment for an external lymphatic wall.
/// The wall's external_pressure_pa is an offset added to this cell's Biot pressure.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticWallAttachment {
    pub compartment: usize,
    pub tissue_cell: usize,
}
/// Temporal transport tolerances plus an absolute geometry discrepancy tolerance.
#[derive(Clone, Copy, Debug)]
pub struct AdaptiveTissueExchangeConfig {
    pub exchange: crate::lymph::AdaptiveExchangeConfig,
    pub absolute_position_tolerance_m: f64,
}
#[derive(Clone, Debug)]
pub struct CellPoreTissue {
    body: Body,
    fluid_nodes: Vec<usize>,
    iterations: usize,
    tolerance_n: f64,
}
impl CellPoreTissue {
    /// # Errors
    /// Rejects missing cell stores, duplicate mappings, viscous materials or solver options.
    pub fn new(
        body: Body,
        fluid_nodes: Vec<usize>,
        iterations: usize,
        tolerance_n: f64,
    ) -> Result<Self, &'static str> {
        let mut sorted = fluid_nodes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if body.cell_pore_fluids.is_empty()
            || fluid_nodes.len() != body.cell_pore_fluids.len()
            || sorted.len() != fluid_nodes.len()
            || body.elements.iter().any(|e| e.viscoelastic.is_some())
            || !(1..=100_000).contains(&iterations)
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
        {
            return Err("invalid cell pore tissue");
        }
        Ok(Self {
            body,
            fluid_nodes,
            iterations,
            tolerance_n,
        })
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }
    /// # Errors
    /// Rejects initial inventory mismatch, bad nodes or failed transport/FEM; no state commits.
    pub fn step(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        seconds: f64,
        max_step: f64,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        for (&node, fluid) in self.fluid_nodes.iter().zip(&self.body.cell_pore_fluids) {
            let v = *network
                .volumes()
                .get(node)
                .ok_or("invalid cell fluid node")?;
            if (v - fluid.fluid_volume_m3).abs() > 1e-10 * v.abs().max(fluid.fluid_volume_m3) {
                return Err("cell/network fluid volume mismatch");
            }
        }
        let mut body = self.body.clone();
        let mut next = network.clone();
        let report = next.step_with_pressure_law(seconds, max_step, |volumes, linear| {
            let mut fluids = body.cell_pore_fluids.clone();
            for (fluid, &node) in fluids.iter_mut().zip(&self.fluid_nodes) {
                fluid.fluid_volume_m3 = volumes[node];
            }
            body.set_cell_pore_fluids(fluids)?;
            let equilibrium = body.equilibrate(self.iterations, self.tolerance_n)?;
            if !equilibrium.converged {
                return Err("cell pore equilibrium did not converge");
            }
            let (pressures, _) = body.cell_pore_response_at(body.positions())?;
            let mut p = linear.to_vec();
            for (&node, pressure) in self.fluid_nodes.iter().zip(pressures) {
                p[node] = pressure;
            }
            Ok(p)
        })?;
        self.body = body;
        *network = next;
        Ok(report)
    }
}

impl Body {
    /// Construct a scalar-permeability Darcy edge from a shared reference face.
    /// Returned edge indices are cell indices; use a matching network mapping.
    /// Geometry/conductance is fixed in the reference configuration. Deformed
    /// permeability push-forward and nonorthogonal MPFA are not implemented here.
    /// # Errors
    /// Rejects nonadjacent cells, centers not normal to the face, degenerate geometry
    /// and invalid hydraulic parameters. No silently inconsistent oblique flux.
    pub fn reference_darcy_interface(
        &self,
        from: usize,
        to: usize,
        permeabilities_m2: [f64; 2],
        viscosity_pa_s: f64,
    ) -> Result<crate::lymph::Exchange, &'static str> {
        let a = self.elements.get(from).ok_or("invalid Darcy cell")?;
        let b = self.elements.get(to).ok_or("invalid Darcy cell")?;
        let face: Vec<_> = a
            .nodes
            .iter()
            .filter(|i| b.nodes.contains(i))
            .copied()
            .collect();
        if from == to || face.len() != 3 {
            return Err("Darcy cells must share one face");
        }
        let origin = self.rest[face[0]];
        let u = sub(self.rest[face[1]], origin);
        let v = sub(self.rest[face[2]], origin);
        let normal = super::cross(u, v);
        let norm = dot(normal, normal).sqrt();
        if !norm.is_finite() || norm <= 0. {
            return Err("degenerate Darcy face");
        }
        let normal = super::scale(normal, 1. / norm);
        // Translation-relative centroids keep the geometry computation stable.
        let center = super::scale(super::add(u, v), 1. / 3.);
        let centroid = |nodes: [usize; 4]| {
            super::scale(
                nodes
                    .into_iter()
                    .map(|i| sub(self.rest[i], origin))
                    .fold([0.; 3], super::add),
                0.25,
            )
        };
        let delta = [
            sub(centroid(a.nodes), center),
            sub(centroid(b.nodes), center),
        ];
        let signed = delta.map(|d| dot(d, normal));
        if signed[0] * signed[1] >= 0. {
            return Err("Darcy cell centers must straddle face");
        }
        for (d, s) in delta.into_iter().zip(signed) {
            let tangent = sub(d, super::scale(normal, s));
            if dot(tangent, tangent) > 1e-16 * dot(d, d) {
                return Err("nonorthogonal Darcy cells require MPFA");
            }
        }
        crate::lymph::Exchange::darcy(
            from,
            to,
            norm * 0.5,
            signed.map(f64::abs),
            permeabilities_m2,
            viscosity_pa_s,
        )
    }
}

impl CellPoreTissue {
    /// Arbitrary-mesh RT0 flux coupling. Permeabilities are symmetric positive
    /// definite reference tensors, pushed forward as F*Kref*F^T/J each trial.
    /// Existing network edges for each interior face are replaced by RT0 flux;
    /// additional capillary/lymph exchanges retain their original constitutive laws.
    /// # Errors
    /// Rejects topology/mapping mismatch, viscous history, invalid tensors,
    /// inverted geometry or failed FEM/flow solve. Both states roll back together.
    pub fn step_mixed_darcy(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_with_osmotic_laws(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            None,
        )
    }
    /// Conservative SSPRK2 RT0/FEM exchange without external wall overrides.
    /// Equilibrium and current-geometry flux are recomputed at each stage.
    /// # Errors
    /// Invalid mapping, predictor or failed subsolve rolls back both states.
    pub fn step_mixed_darcy_second_order(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_geometry_order(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            None,
            None,
            &[],
            &[],
            true,
        )
    }
    /// RT0/FEM tissue exchange with optional per-network-space osmotic laws.
    /// Darcy tissue faces stay unrestricted; capillary and lymph edges use the laws.
    /// # Errors
    /// Invalid law/topology or any failed subsolve preserves tissue and network.
    pub fn step_mixed_darcy_with_osmotic_laws(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_with_lymphatic_walls(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            laws,
            None,
        )
    }
    /// Coupled deforming tissue, nonlinear exchange and active lymphatic walls.
    /// Wall overrides apply only to external network spaces, never to FEM cells.
    /// # Errors
    /// Invalid mappings, wall overflow or failed subsolve preserves all state.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_lymphatic_walls(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_with_attached_lymphatic_walls(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            laws,
            walls,
            &[],
        )
    }
    /// Coupled wall pressure includes current pressure of explicitly attached tissue.
    /// # Errors
    /// Invalid/duplicate attachment, overflow or failed solve preserves all state.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_attached_lymphatic_walls(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
        attachments: &[LymphaticWallAttachment],
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_with_lymphatic_geometry(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            laws,
            walls,
            attachments,
            &[],
        )
    }
    /// Coupled current-volume wall pressures and laminar tube resistances.
    /// # Errors
    /// Invalid attachments, singular radius or failed solve preserves accepted states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_lymphatic_geometry(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
        attachments: &[LymphaticWallAttachment],
        hydraulic_attachments: &[crate::lymph::LymphaticHydraulicAttachment],
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        self.step_mixed_darcy_geometry_order(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            max_step,
            laws,
            walls,
            attachments,
            hydraulic_attachments,
            walls.is_some(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn step_mixed_darcy_geometry_order(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        max_step: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
        attachments: &[LymphaticWallAttachment],
        hydraulic_attachments: &[crate::lymph::LymphaticHydraulicAttachment],
        second_order: bool,
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        if !hydraulic_attachments.is_empty() {
            let mut rates = network.rates()?;
            crate::lymph::apply_wall_hydraulic_resistances(
                network.volumes(),
                network.protein_masses(),
                network.edges(),
                walls.ok_or("missing hydraulic wall array")?,
                hydraulic_attachments,
                &mut rates,
            )?;
        }
        let mut linked = std::collections::BTreeSet::new();
        for a in attachments {
            if a.tissue_cell >= self.fluid_nodes.len()
                || walls
                    .and_then(|w| w.get(a.compartment))
                    .is_none_or(|w| w.is_none())
                || !linked.insert(a.compartment)
            {
                return Err("invalid lymphatic wall attachment");
            }
        }
        if let Some(walls) = walls {
            let mut p = network.pressures();
            crate::lymph::apply_wall_pressures(network.volumes(), &mut p, walls)?;
            if self
                .fluid_nodes
                .iter()
                .any(|i| walls.get(*i).is_none_or(|wall| wall.is_some()))
            {
                return Err("lymphatic wall cannot override a tissue cell");
            }
        }
        if let Some(laws) = laws {
            network.rates_with_osmotic_laws(laws)?;
        }
        use std::cell::RefCell;
        for (&node, fluid) in self.fluid_nodes.iter().zip(&self.body.cell_pore_fluids) {
            let v = *network
                .volumes()
                .get(node)
                .ok_or("invalid mixed fluid node")?;
            if (v - fluid.fluid_volume_m3).abs() > 1e-10 * v.abs().max(fluid.fluid_volume_m3) {
                return Err("mixed pore inventory mismatch");
            }
        }
        let initial = self.body.deformed_darcy(permeabilities, viscosity_pa_s)?;
        let mut edge_index = std::collections::HashMap::new();
        for (index, edge) in network.edges().iter().enumerate() {
            let key = (edge.from.min(edge.to), edge.from.max(edge.to));
            let entry = edge_index.entry(key).or_insert((index, 0usize));
            entry.1 += 1;
        }
        let mut mapping = Vec::new();
        for face in initial.faces() {
            let a = self.fluid_nodes[face.owner];
            let b = self.fluid_nodes[face.neighbor.ok_or("mixed tissue uses sealed boundaries")?];
            let &(index, count) = edge_index
                .get(&(a.min(b), a.max(b)))
                .ok_or("each Darcy face requires one network edge")?;
            if count != 1 {
                return Err("each Darcy face requires one network edge");
            }
            let edge = &network.edges()[index];
            let orientation = if edge.from == a { 1. } else { -1. };
            if edge.valve
                || edge.reflection != 0.
                || edge.pump_head_pa != 0.
                || edge.protein_permeability_m3_per_s != 0.
            {
                return Err("Darcy face edge must represent unrestricted pore advection");
            }
            mapping.push((index, orientation));
        }
        let trial = RefCell::new(self.body.clone());
        let mut next = network.clone();
        let report = next.step_with_pressure_and_flux_laws_order(
            seconds,
            max_step,
            second_order,
            |volumes, linear| {
                let mut body = trial.borrow_mut();
                let mut fluids = body.cell_pore_fluids.clone();
                for (fluid, &node) in fluids.iter_mut().zip(&self.fluid_nodes) {
                    fluid.fluid_volume_m3 = volumes[node];
                }
                body.set_cell_pore_fluids(fluids)?;
                if !body
                    .equilibrate(self.iterations, self.tolerance_n)?
                    .converged
                {
                    return Err("mixed pore FEM nonconvergence");
                }
                let mut p = linear.to_vec();
                if let Some(walls) = walls {
                    crate::lymph::apply_wall_pressures(volumes, &mut p, walls)?;
                }
                let pressures = body.cell_pore_response_at(body.positions())?.0;
                for (&node, pressure) in self.fluid_nodes.iter().zip(pressures) {
                    p[node] = pressure;
                }
                for a in attachments {
                    let wall = walls.unwrap()[a.compartment].unwrap();
                    let external = wall.external_pressure_pa + p[self.fluid_nodes[a.tissue_cell]];
                    p[a.compartment] = crate::lymph::LymphaticWallLaw {
                        external_pressure_pa: external,
                        ..wall
                    }
                    .pressure_pa(volumes[a.compartment])?;
                }
                Ok(p)
            },
            |volumes, proteins, pressures, edges, default| {
                let body = trial.borrow();
                let model = body.deformed_darcy(permeabilities, viscosity_pa_s)?;
                let p: Vec<_> = self.fluid_nodes.iter().map(|i| pressures[*i]).collect();
                let response = model.response(&p)?;
                let mut rates = if let Some(laws) = laws {
                    crate::lymph::osmotic_exchange_rates(volumes, proteins, pressures, edges, laws)?
                } else {
                    default.to_vec()
                };
                if !hydraulic_attachments.is_empty() {
                    crate::lymph::apply_wall_hydraulic_resistances(
                        volumes,
                        proteins,
                        edges,
                        walls.unwrap(),
                        hydraulic_attachments,
                        &mut rates,
                    )?;
                }
                for (&(index, sign), q) in mapping.iter().zip(response.face_flows_m3_per_s) {
                    let q = q * sign;
                    let edge = &edges[index];
                    let donor = if q >= 0. { edge.from } else { edge.to };
                    rates[index] = (q, q * proteins[donor] / volumes[donor]);
                }
                Ok(rates)
            },
        )?;
        self.body = trial.into_inner();
        *network = next;
        Ok(report)
    }
    /// Joint backward-Euler inertial lymph, RT0 flow and equilibrated FEM tissue.
    /// # Errors
    /// Invalid mapping, positivity failure or nonconvergence preserves all states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_inertial_lymphatic_geometry(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        density: f64,
        flow_history: &mut [f64],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
        attachments: &[LymphaticWallAttachment],
        hydraulic_attachments: &[crate::lymph::LymphaticHydraulicAttachment],
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        if !hydraulic_attachments.is_empty() {
            let mut rates = network.rates()?;
            crate::lymph::apply_wall_hydraulic_resistances(
                network.volumes(),
                network.protein_masses(),
                network.edges(),
                walls.ok_or("missing hydraulic wall array")?,
                hydraulic_attachments,
                &mut rates,
            )?;
        }
        let mut linked = std::collections::BTreeSet::new();
        for a in attachments {
            if a.tissue_cell >= self.fluid_nodes.len()
                || walls
                    .and_then(|w| w.get(a.compartment))
                    .is_none_or(|w| w.is_none())
                || !linked.insert(a.compartment)
            {
                return Err("invalid lymphatic wall attachment");
            }
        }
        if let Some(walls) = walls {
            let mut p = network.pressures();
            crate::lymph::apply_wall_pressures(network.volumes(), &mut p, walls)?;
            if self
                .fluid_nodes
                .iter()
                .any(|i| walls.get(*i).is_none_or(|wall| wall.is_some()))
            {
                return Err("lymphatic wall cannot override a tissue cell");
            }
        }
        if let Some(laws) = laws {
            network.rates_with_osmotic_laws(laws)?;
        }
        use std::cell::RefCell;
        for (&node, fluid) in self.fluid_nodes.iter().zip(&self.body.cell_pore_fluids) {
            let v = *network
                .volumes()
                .get(node)
                .ok_or("invalid mixed fluid node")?;
            if (v - fluid.fluid_volume_m3).abs() > 1e-10 * v.abs().max(fluid.fluid_volume_m3) {
                return Err("mixed pore inventory mismatch");
            }
        }
        let initial = self.body.deformed_darcy(permeabilities, viscosity_pa_s)?;
        let mut edge_index = std::collections::HashMap::new();
        for (index, edge) in network.edges().iter().enumerate() {
            let key = (edge.from.min(edge.to), edge.from.max(edge.to));
            let entry = edge_index.entry(key).or_insert((index, 0usize));
            entry.1 += 1;
        }
        let mut mapping = Vec::new();
        for face in initial.faces() {
            let a = self.fluid_nodes[face.owner];
            let b = self.fluid_nodes[face.neighbor.ok_or("mixed tissue uses sealed boundaries")?];
            let &(index, count) = edge_index
                .get(&(a.min(b), a.max(b)))
                .ok_or("each Darcy face requires one network edge")?;
            if count != 1 {
                return Err("each Darcy face requires one network edge");
            }
            let edge = &network.edges()[index];
            let orientation = if edge.from == a { 1. } else { -1. };
            if edge.valve
                || edge.reflection != 0.
                || edge.pump_head_pa != 0.
                || edge.protein_permeability_m3_per_s != 0.
            {
                return Err("Darcy face edge must represent unrestricted pore advection");
            }
            mapping.push((index, orientation));
        }
        let trial = RefCell::new(self.body.clone());
        let mut next = network.clone();
        let report = next.step_with_inertial_callbacks(
            seconds,
            laws.ok_or("inertial lymph requires explicit osmotic laws")?,
            walls.ok_or("inertial lymph requires wall array")?,
            hydraulic_attachments,
            density,
            flow_history,
            max_iterations,
            volume_tolerance,
            protein_tolerance,
            |volumes, linear| {
                let mut body = trial.borrow_mut();
                let mut fluids = body.cell_pore_fluids.clone();
                for (fluid, &node) in fluids.iter_mut().zip(&self.fluid_nodes) {
                    fluid.fluid_volume_m3 = volumes[node];
                }
                body.set_cell_pore_fluids(fluids)?;
                if !body
                    .equilibrate(self.iterations, self.tolerance_n)?
                    .converged
                {
                    return Err("mixed pore FEM nonconvergence");
                }
                let mut p = linear.to_vec();
                if let Some(walls) = walls {
                    crate::lymph::apply_wall_pressures(volumes, &mut p, walls)?;
                }
                let pressures = body.cell_pore_response_at(body.positions())?.0;
                for (&node, pressure) in self.fluid_nodes.iter().zip(pressures) {
                    p[node] = pressure;
                }
                for a in attachments {
                    let wall = walls.unwrap()[a.compartment].unwrap();
                    let external = wall.external_pressure_pa + p[self.fluid_nodes[a.tissue_cell]];
                    p[a.compartment] = crate::lymph::LymphaticWallLaw {
                        external_pressure_pa: external,
                        ..wall
                    }
                    .pressure_pa(volumes[a.compartment])?;
                }
                Ok(p)
            },
            |volumes, proteins, pressures, edges, default| {
                let body = trial.borrow();
                let model = body.deformed_darcy(permeabilities, viscosity_pa_s)?;
                let p: Vec<_> = self.fluid_nodes.iter().map(|i| pressures[*i]).collect();
                let response = model.response(&p)?;
                let mut rates = default.to_vec();
                for (&(index, sign), q) in mapping.iter().zip(response.face_flows_m3_per_s) {
                    let q = q * sign;
                    let edge = &edges[index];
                    let donor = if q >= 0. { edge.from } else { edge.to };
                    rates[index] = (q, q * proteins[donor] / volumes[donor]);
                }
                Ok(rates)
            },
        )?;
        self.body = trial.into_inner();
        *network = next;
        Ok(report)
    }
    /// Joint equilibrated FEM/RT0 exchange through fixed-radius radial pipes.
    /// # Errors
    /// Invalid mapping or failed trial preserves tissue, network and radial histories.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_radial_profiles(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        profiles: &mut [crate::lymph::RadialExchange],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: Option<&[Option<crate::lymph::LymphaticWallLaw>]>,
        attachments: &[LymphaticWallAttachment],
    ) -> Result<crate::lymph::ExchangeReport, &'static str> {
        let mut linked = std::collections::BTreeSet::new();
        for a in attachments {
            if a.tissue_cell >= self.fluid_nodes.len()
                || walls
                    .and_then(|w| w.get(a.compartment))
                    .is_none_or(|w| w.is_none())
                || !linked.insert(a.compartment)
            {
                return Err("invalid lymphatic wall attachment");
            }
        }
        if let Some(walls) = walls {
            let mut p = network.pressures();
            crate::lymph::apply_wall_pressures(network.volumes(), &mut p, walls)?;
            if self
                .fluid_nodes
                .iter()
                .any(|i| walls.get(*i).is_none_or(|wall| wall.is_some()))
            {
                return Err("lymphatic wall cannot override a tissue cell");
            }
        }
        if let Some(laws) = laws {
            network.rates_with_osmotic_laws(laws)?;
        }
        use std::cell::RefCell;
        for (&node, fluid) in self.fluid_nodes.iter().zip(&self.body.cell_pore_fluids) {
            let v = *network
                .volumes()
                .get(node)
                .ok_or("invalid mixed fluid node")?;
            if (v - fluid.fluid_volume_m3).abs() > 1e-10 * v.abs().max(fluid.fluid_volume_m3) {
                return Err("mixed pore inventory mismatch");
            }
        }
        let initial = self.body.deformed_darcy(permeabilities, viscosity_pa_s)?;
        let mut edge_index = std::collections::HashMap::new();
        for (index, edge) in network.edges().iter().enumerate() {
            let key = (edge.from.min(edge.to), edge.from.max(edge.to));
            let entry = edge_index.entry(key).or_insert((index, 0usize));
            entry.1 += 1;
        }
        let mut mapping = Vec::new();
        for face in initial.faces() {
            let a = self.fluid_nodes[face.owner];
            let b = self.fluid_nodes[face.neighbor.ok_or("mixed tissue uses sealed boundaries")?];
            let &(index, count) = edge_index
                .get(&(a.min(b), a.max(b)))
                .ok_or("each Darcy face requires one network edge")?;
            if count != 1 {
                return Err("each Darcy face requires one network edge");
            }
            let edge = &network.edges()[index];
            let orientation = if edge.from == a { 1. } else { -1. };
            if edge.valve
                || edge.reflection != 0.
                || edge.pump_head_pa != 0.
                || edge.protein_permeability_m3_per_s != 0.
            {
                return Err("Darcy face edge must represent unrestricted pore advection");
            }
            mapping.push((index, orientation));
        }
        if profiles
            .iter()
            .any(|a| mapping.iter().any(|(edge, _)| *edge == a.edge))
        {
            return Err("radial pipe cannot override an internal Darcy face");
        }
        let trial = RefCell::new(self.body.clone());
        let mut next = network.clone();
        let report = next.step_with_radial_callbacks(
            seconds,
            laws.ok_or("radial tissue requires explicit osmotic laws")?,
            profiles,
            max_iterations,
            volume_tolerance,
            protein_tolerance,
            |volumes, linear| {
                let mut body = trial.borrow_mut();
                let mut fluids = body.cell_pore_fluids.clone();
                for (fluid, &node) in fluids.iter_mut().zip(&self.fluid_nodes) {
                    fluid.fluid_volume_m3 = volumes[node];
                }
                body.set_cell_pore_fluids(fluids)?;
                if !body
                    .equilibrate(self.iterations, self.tolerance_n)?
                    .converged
                {
                    return Err("mixed pore FEM nonconvergence");
                }
                let mut p = linear.to_vec();
                if let Some(walls) = walls {
                    crate::lymph::apply_wall_pressures(volumes, &mut p, walls)?;
                }
                let pressures = body.cell_pore_response_at(body.positions())?.0;
                for (&node, pressure) in self.fluid_nodes.iter().zip(pressures) {
                    p[node] = pressure;
                }
                for a in attachments {
                    let wall = walls.unwrap()[a.compartment].unwrap();
                    let external = wall.external_pressure_pa + p[self.fluid_nodes[a.tissue_cell]];
                    p[a.compartment] = crate::lymph::LymphaticWallLaw {
                        external_pressure_pa: external,
                        ..wall
                    }
                    .pressure_pa(volumes[a.compartment])?;
                }
                Ok(p)
            },
            |volumes, proteins, pressures, edges, default| {
                let body = trial.borrow();
                let model = body.deformed_darcy(permeabilities, viscosity_pa_s)?;
                let p: Vec<_> = self.fluid_nodes.iter().map(|i| pressures[*i]).collect();
                let response = model.response(&p)?;
                let mut rates = default.to_vec();
                for (&(index, sign), q) in mapping.iter().zip(response.face_flows_m3_per_s) {
                    let q = q * sign;
                    let edge = &edges[index];
                    let donor = if q >= 0. { edge.from } else { edge.to };
                    rates[index] = (q, q * proteins[donor] / volumes[donor]);
                }
                Ok(rates)
            },
        )?;
        self.body = trial.into_inner();
        *network = next;
        Ok(report)
    }
}
impl CellPoreTissue {
    /// Joint adaptive SSPRK2 tissue/lymph exchange with geometry error control.
    /// Accepted two half-steps carry their own equilibrated geometry and inventories.
    /// # Errors
    /// Invalid tolerances/mapping, failed FEM or exhausted steps preserve both states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_lymphatic_walls_adaptive(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: &[Option<crate::lymph::LymphaticWallLaw>],
        config: AdaptiveTissueExchangeConfig,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        self.step_mixed_darcy_with_attached_lymphatic_walls_adaptive(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            laws,
            walls,
            &[],
            config,
        )
    }
    /// Joint adaptive transport/geometry with interstitial wall-pressure feedback.
    /// # Errors
    /// Invalid mapping/tolerance or failed trial preserves both accepted states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_attached_lymphatic_walls_adaptive(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: &[Option<crate::lymph::LymphaticWallLaw>],
        attachments: &[LymphaticWallAttachment],
        config: AdaptiveTissueExchangeConfig,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        self.step_mixed_darcy_with_lymphatic_geometry_adaptive(
            network,
            permeabilities,
            viscosity_pa_s,
            seconds,
            laws,
            walls,
            attachments,
            &[],
            config,
        )
    }
    /// Joint adaptive current-geometry lymphatic pressure/resistance and FEM exchange.
    /// # Errors
    /// Invalid laws/tolerances or failed trial preserve geometry and fluid inventories.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_lymphatic_geometry_adaptive(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        laws: Option<&[super::OsmoticPressureLaw]>,
        walls: &[Option<crate::lymph::LymphaticWallLaw>],
        attachments: &[LymphaticWallAttachment],
        hydraulic_attachments: &[crate::lymph::LymphaticHydraulicAttachment],
        config: AdaptiveTissueExchangeConfig,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        if !config.absolute_position_tolerance_m.is_finite()
            || config.absolute_position_tolerance_m <= 0.
        {
            return Err("invalid adaptive tissue position tolerance");
        }
        let initial = (self.clone(), network.clone());
        let ((tissue, next), report) = crate::lymph::adaptive_exchange(
            &initial,
            seconds,
            config.exchange,
            |state, h| {
                state.0.step_mixed_darcy_with_lymphatic_geometry(
                    &mut state.1,
                    permeabilities,
                    viscosity_pa_s,
                    h,
                    h,
                    laws,
                    Some(walls),
                    attachments,
                    hydraulic_attachments,
                )
            },
            |state| &state.1,
            |coarse, fine| {
                let mut error = 0_f64;
                for (a, b) in coarse
                    .0
                    .body
                    .positions()
                    .iter()
                    .zip(fine.0.body.positions())
                {
                    let distance = dot(sub(*a, *b), sub(*a, *b)).sqrt();
                    if !distance.is_finite() {
                        return Err("adaptive tissue geometry overflow");
                    }
                    error = error.max((distance / 3.) / config.absolute_position_tolerance_m);
                }
                Ok(error)
            },
        )?;
        *self = tissue;
        *network = next;
        Ok(report)
    }
    /// First-order step doubling controls joint tissue geometry and inertial history.
    /// # Errors
    /// Invalid tolerances, failed solve or trial exhaustion preserves all three states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_inertial_lymphatic_geometry_adaptive(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        laws: &[super::OsmoticPressureLaw],
        walls: &[Option<crate::lymph::LymphaticWallLaw>],
        attachments: &[LymphaticWallAttachment],
        hydraulic: &[crate::lymph::LymphaticHydraulicAttachment],
        density: f64,
        flow_history: &mut [f64],
        config: AdaptiveTissueExchangeConfig,
        absolute_flow_tolerance: f64,
        max_iterations: usize,
        nonlinear_volume_tolerance: f64,
        nonlinear_protein_tolerance: f64,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        if !config.absolute_position_tolerance_m.is_finite()
            || config.absolute_position_tolerance_m <= 0.
            || !absolute_flow_tolerance.is_finite()
            || absolute_flow_tolerance <= 0.
        {
            return Err("invalid inertial tissue time tolerance");
        }
        let initial = (self.clone(), network.clone(), flow_history.to_vec());
        let (next, report) = crate::lymph::adaptive_exchange_order(
            &initial,
            seconds,
            config.exchange,
            1,
            |state, h| {
                state.0.step_mixed_darcy_with_inertial_lymphatic_geometry(
                    &mut state.1,
                    permeabilities,
                    viscosity_pa_s,
                    h,
                    density,
                    &mut state.2,
                    max_iterations,
                    nonlinear_volume_tolerance,
                    nonlinear_protein_tolerance,
                    Some(laws),
                    Some(walls),
                    attachments,
                    hydraulic,
                )
            },
            |state| &state.1,
            |coarse, fine| {
                let mut error = 0_f64;
                for (a, b) in coarse
                    .0
                    .body
                    .positions()
                    .iter()
                    .zip(fine.0.body.positions())
                {
                    let distance = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt();
                    error = error.max(distance / config.absolute_position_tolerance_m);
                }
                for (a, b) in coarse.2.iter().zip(&fine.2) {
                    let budget = absolute_flow_tolerance
                        + config.exchange.relative_tolerance * a.abs().max(b.abs());
                    if !budget.is_finite() || budget <= 0. {
                        return Err("inertial tissue flow budget overflow");
                    }
                    error = error.max((a - b).abs() / budget);
                }
                if !error.is_finite() {
                    return Err("inertial tissue geometry error overflow");
                }
                Ok(error)
            },
        )?;
        flow_history.copy_from_slice(&next.2);
        *self = next.0;
        *network = next.1;
        Ok(report)
    }
    /// Joint local error control of inventories, FEM positions and every radial velocity.
    /// # Errors
    /// Invalid controls, failed solve or exhausted trials preserves all three states.
    #[allow(clippy::too_many_arguments)]
    pub fn step_mixed_darcy_with_radial_profiles_adaptive(
        &mut self,
        network: &mut crate::lymph::LymphNetwork,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        laws: &[super::OsmoticPressureLaw],
        walls: &[Option<crate::lymph::LymphaticWallLaw>],
        attachments: &[LymphaticWallAttachment],
        profiles: &mut [crate::lymph::RadialExchange],
        config: AdaptiveTissueExchangeConfig,
        absolute_velocity_tolerance: f64,
        max_iterations: usize,
        nonlinear_volume_tolerance: f64,
        nonlinear_protein_tolerance: f64,
    ) -> Result<crate::lymph::AdaptiveExchangeReport, &'static str> {
        if !config.absolute_position_tolerance_m.is_finite()
            || config.absolute_position_tolerance_m <= 0.
            || !absolute_velocity_tolerance.is_finite()
            || absolute_velocity_tolerance <= 0.
        {
            return Err("invalid inertial tissue time tolerance");
        }
        let initial = (self.clone(), network.clone(), profiles.to_vec());
        let (next, report) = crate::lymph::adaptive_exchange_order(
            &initial,
            seconds,
            config.exchange,
            1,
            |state, h| {
                state.0.step_mixed_darcy_with_radial_profiles(
                    &mut state.1,
                    permeabilities,
                    viscosity_pa_s,
                    h,
                    &mut state.2,
                    max_iterations,
                    nonlinear_volume_tolerance,
                    nonlinear_protein_tolerance,
                    Some(laws),
                    Some(walls),
                    attachments,
                )
            },
            |state| &state.1,
            |coarse, fine| {
                let mut error = 0_f64;
                for (a, b) in coarse
                    .0
                    .body
                    .positions()
                    .iter()
                    .zip(fine.0.body.positions())
                {
                    let distance = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt();
                    error = error.max(distance / config.absolute_position_tolerance_m);
                }
                for (a, b) in coarse.2.iter().zip(&fine.2) {
                    for (u, v) in a
                        .pipe
                        .velocities_m_per_s()
                        .iter()
                        .zip(b.pipe.velocities_m_per_s())
                    {
                        let budget = absolute_velocity_tolerance
                            + config.exchange.relative_tolerance * u.abs().max(v.abs());
                        if !budget.is_finite() || budget <= 0. {
                            return Err("radial profile error budget overflow");
                        }
                        error = error.max((u - v).abs() / budget);
                    }
                }
                if !error.is_finite() {
                    return Err("inertial tissue geometry error overflow");
                }
                Ok(error)
            },
        )?;
        profiles.clone_from_slice(&next.2);
        *self = next.0;
        *network = next.1;
        Ok(report)
    }
}
impl Body {
    /// Current-geometry mixed Darcy operator with objective referential permeability.
    /// # Errors
    /// Rejects invalid tensor counts, deformation, permeability or mesh topology.
    pub fn deformed_darcy(
        &self,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
    ) -> Result<super::MixedDarcy, &'static str> {
        self.deformed_darcy_with_boundaries(permeabilities, viscosity_pa_s, &[])
    }
    /// Current geometry and transformed permeability, with explicit pressure ports.
    /// # Errors
    /// Rejects invalid tensors, geometry, duplicate ports or non-boundary faces.
    pub fn deformed_darcy_with_boundaries(
        &self,
        permeabilities: &[super::Matrix],
        viscosity_pa_s: f64,
        boundaries: &[([usize; 3], f64)],
    ) -> Result<super::MixedDarcy, &'static str> {
        if permeabilities.len() != self.elements.len() {
            return Err("invalid Darcy permeability count");
        }
        let mut current = Vec::with_capacity(permeabilities.len());
        for (e, k) in self.elements.iter().zip(permeabilities) {
            let [a, b, c, d] = e.nodes.map(|i| self.positions[i]);
            let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
            let j = det(f);
            if !j.is_finite() || j <= 0. {
                return Err("inverted Darcy body");
            }
            current.push(mm(mm(f, *k), super::transpose(f)).map(|r| r.map(|v| v / j)));
        }
        super::MixedDarcy::new(
            self.positions.clone(),
            self.elements.iter().map(|e| e.nodes).collect(),
            &current,
            viscosity_pa_s,
            boundaries,
        )
    }
}
