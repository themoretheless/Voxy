//! Transactional fixed-point vascular/pore pressure and protein coupling.
use super::{Body, ImplicitPoreConfig, ImplicitPoreReport, Matrix};
use crate::circulation::{Circulation, CirculationStep};

/// Explicit nonnegative virial polynomial Pi(C)=a*C+b*C²+c*C³.
/// C is kg/m³; coefficient units are Pa m³/kg, Pa m⁶/kg², Pa m⁹/kg³.
/// Coefficients require material calibration; no physiological defaults are supplied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OsmoticPressureLaw {
    pub linear: f64,
    pub quadratic: f64,
    pub cubic: f64,
}
impl OsmoticPressureLaw {
    /// Evaluate with checked coefficients and concentration; never clip overflow.
    pub fn pressure_pa(self, concentration_kg_per_m3: f64) -> Result<f64, &'static str> {
        if [
            self.linear,
            self.quadratic,
            self.cubic,
            concentration_kg_per_m3,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid osmotic pressure law");
        }
        let c = concentration_kg_per_m3;
        let pressure = c * (self.linear + c * (self.quadratic + c * self.cubic));
        if !pressure.is_finite() {
            return Err("osmotic pressure overflow");
        }
        Ok(pressure)
    }
    fn active(self) -> bool {
        self.linear > 0. || self.quadratic > 0. || self.cubic > 0.
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VascularPoreConfig {
    pub iterations: usize,
    pub relaxation: f64,
    pub pressure_tolerance_pa: f64,
    pub concentration_tolerance_kg_per_m3: f64,
    pub tissue: ImplicitPoreConfig,
    pub circulation_iterations: usize,
    pub circulation_tolerance_m3: f64,
}
/// Explicit tissue boundary triangle attached to a vascular compartment.
#[derive(Clone, Copy, Debug)]
pub struct VascularPorePort {
    pub nodes: [usize; 3],
    pub compartment: usize,
}
#[derive(Clone, Debug)]
pub struct VascularPoreReport {
    pub iterations: usize,
    pub pressure_residual_pa: f64,
    pub concentration_residual_kg_per_m3: f64,
    pub tissue_fluid_gain_m3: f64,
    pub tissue_protein_gain_kg: f64,
    pub tissue: ImplicitPoreReport,
    pub circulation: CirculationStep,
}
impl Body {
    /// Simultaneous elastic pore exchange with one vascular compartment.
    /// All ports share that compartment's accepted pressure/concentration. Each
    /// trial starts from the old state; only a converged joint state commits.
    /// Fluid is homogeneous, without plasma/red-cell partition or reflection.
    /// # Errors
    /// Invalid input, failed subsolves, depletion or unconverged coupling preserves
    /// tissue, circulation and both caller protein inventories.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_step(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        compartment: usize,
        ports: &[[usize; 3]],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
    ) -> Result<VascularPoreReport, &'static str> {
        self.implicit_vascular_pore_step_with_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            compartment,
            ports,
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            |_, _, _| {},
        )
    }
    /// Same joint solve with per-iteration pressure/concentration residuals.
    /// Observer receives iteration (one-based), Pa and kg/m³. It cannot mutate
    /// staged states; its own side effects are outside the state transaction.
    /// # Errors
    /// The same input, solve and conservation errors as the unobserved method.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_step_with_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        compartment: usize,
        ports: &[[usize; 3]],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        let mapped: Vec<_> = ports
            .iter()
            .map(|nodes| VascularPorePort {
                nodes: *nodes,
                compartment,
            })
            .collect();
        self.implicit_vascular_pore_ports_step_with_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            &mapped,
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            observer,
        )
    }
    /// Joint solve with independently assigned vascular compartments per port.
    /// Residuals are maxima across connected compartments. Each flux and donor
    /// protein transfer is credited to the compartment owning that exact port.
    /// # Errors
    /// Invalid mappings, duplicate ports or failed solves preserve all states.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_ports_step_with_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        ports: &[VascularPorePort],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        self.implicit_vascular_pore_ports_step_with_resistances_and_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            ports,
            &[],
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            observer,
        )
    }
    /// Joint vascular/tissue solve with explicit passive port resistances.
    /// Resistances add R*q pressure drop without altering donor protein rules.
    /// # Errors
    /// Bad resistance mappings or any failed solve preserve all accepted states.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_ports_step_with_resistances_and_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        ports: &[VascularPorePort],
        resistances: &[([usize; 3], f64)],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        self.implicit_vascular_pore_ports_step_with_membranes_and_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            ports,
            resistances,
            &[],
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            observer,
        )
    }
    /// Joint vascular/tissue exchange with reflection and diffusive conductance.
    /// Membrane entries are (triangle, reflection fraction, conductance m³/s).
    /// # Errors
    /// Bad mappings or any failed subsolve preserve all physical states.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_ports_step_with_membranes_and_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        ports: &[VascularPorePort],
        resistances: &[([usize; 3], f64)],
        membranes: &[([usize; 3], f64, f64)],
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        self.implicit_vascular_pore_ports_step_with_osmosis_and_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            ports,
            resistances,
            membranes,
            0.,
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            observer,
        )
    }
    /// Joint selective exchange with linear osmotic pressure Pi = slope*C.
    /// Slope is explicit Pa m³/kg, not an assumed measured protein property.
    /// The hydraulic port pressure is p_b - sigma*slope*(C_b-C_tissue).
    /// Accepted local tissue concentration and vascular state converge together.
    /// # Errors
    /// Invalid slope, overflow or any failed solve preserves all accepted states.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_ports_step_with_osmosis_and_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        ports: &[VascularPorePort],
        resistances: &[([usize; 3], f64)],
        membranes: &[([usize; 3], f64, f64)],
        osmotic_slope_pa_m3_per_kg: f64,
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        self.implicit_vascular_pore_ports_step_with_osmotic_law_and_observer(
            tissue_protein_kg,
            blood,
            blood_protein_kg,
            ports,
            resistances,
            membranes,
            OsmoticPressureLaw {
                linear: osmotic_slope_pa_m3_per_kg,
                quadratic: 0.,
                cubic: 0.,
            },
            permeability,
            viscosity_pa_s,
            seconds,
            elastance,
            external,
            config,
            observer,
        )
    }
    /// Simultaneous exchange with an explicit nonlinear osmotic pressure law.
    /// # Errors
    /// Invalid law, overflow or failed convergence preserves every accepted state.
    #[allow(clippy::too_many_arguments)]
    pub fn implicit_vascular_pore_ports_step_with_osmotic_law_and_observer<F>(
        &mut self,
        tissue_protein_kg: &mut [f64],
        blood: &mut Circulation,
        blood_protein_kg: &mut [f64],
        ports: &[VascularPorePort],
        resistances: &[([usize; 3], f64)],
        membranes: &[([usize; 3], f64, f64)],
        osmotic_law: OsmoticPressureLaw,
        permeability: &[Matrix],
        viscosity_pa_s: f64,
        seconds: f64,
        elastance: &[f64],
        external: &[f64],
        config: VascularPoreConfig,
        mut observer: F,
    ) -> Result<VascularPoreReport, &'static str>
    where
        F: FnMut(usize, f64, f64),
    {
        let n = blood.volumes().len();
        osmotic_law.pressure_pa(0.)?;
        if ports.iter().any(|p| p.compartment >= n)
            || blood_protein_kg.len() != n
            || ports.is_empty()
            || tissue_protein_kg.len() != self.cell_pore_fluids().len()
            || tissue_protein_kg
                .iter()
                .chain(blood_protein_kg.iter())
                .any(|m| !m.is_finite() || *m < 0.)
            || !(1..=10_000).contains(&config.iterations)
            || !config.relaxation.is_finite()
            || config.relaxation <= 0.
            || config.relaxation > 1.
            || !config.pressure_tolerance_pa.is_finite()
            || config.pressure_tolerance_pa <= 0.
            || !config.concentration_tolerance_kg_per_m3.is_finite()
            || config.concentration_tolerance_kg_per_m3 <= 0.
        {
            return Err("invalid vascular pore coupling");
        }
        let mut by_face = std::collections::BTreeMap::new();
        for port in ports {
            let mut key = port.nodes;
            key.sort_unstable();
            if by_face.insert(key, port.compartment).is_some() {
                return Err("duplicate vascular pore port");
            }
        }
        let connected: std::collections::BTreeSet<_> =
            ports.iter().map(|p| p.compartment).collect();
        let mut membrane_by_face = std::collections::BTreeMap::new();
        for (nodes, reflection, conductance) in membranes {
            let mut key = *nodes;
            key.sort_unstable();
            if !reflection.is_finite()
                || !(0. ..=1.).contains(reflection)
                || !conductance.is_finite()
                || *conductance < 0.
                || !by_face.contains_key(&key)
                || membrane_by_face
                    .insert(key, (*reflection, *conductance))
                    .is_some()
            {
                return Err("invalid vascular protein membrane");
            }
        }
        let old_fluid: Vec<_> = self
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect();
        let total_water = old_fluid.iter().sum::<f64>() + blood.total_volume();
        let total_protein = tissue_protein_kg
            .iter()
            .chain(blood_protein_kg.iter())
            .sum::<f64>();
        if !total_water.is_finite() || !total_protein.is_finite() {
            return Err("vascular pore inventory overflow");
        }
        let mut pressure = blood.pressures().to_vec();
        let mut concentration: Vec<_> = blood_protein_kg
            .iter()
            .zip(blood.volumes())
            .map(|(m, v)| m / v)
            .collect();
        let mut owners = vec![0; ports.len()];
        let mut local_concentration = vec![0.; ports.len()];
        if osmotic_law.active() {
            let initial_boundaries: Vec<_> = ports
                .iter()
                .map(|p| (p.nodes, pressure[p.compartment]))
                .collect();
            let model = self.deformed_darcy_with_boundaries(
                permeability,
                viscosity_pa_s,
                &initial_boundaries,
            )?;
            let owner_by_face: std::collections::BTreeMap<_, _> = model
                .faces()
                .iter()
                .filter(|f| f.neighbor.is_none())
                .map(|f| {
                    let mut key = f.nodes;
                    key.sort_unstable();
                    (key, f.owner)
                })
                .collect();
            for (i, p) in ports.iter().enumerate() {
                let mut key = p.nodes;
                key.sort_unstable();
                owners[i] = *owner_by_face.get(&key).ok_or("missing osmotic pore port")?;
                local_concentration[i] = tissue_protein_kg[owners[i]] / old_fluid[owners[i]];
            }
            if local_concentration.iter().any(|c| !c.is_finite()) {
                return Err("osmotic concentration overflow");
            }
        }
        for iteration in 0..config.iterations {
            let mut tissue_trial = self.clone();
            let mut tissue_mass = tissue_protein_kg.to_vec();
            let boundaries: Vec<_> = ports
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let mut key = p.nodes;
                    key.sort_unstable();
                    let reflection = membrane_by_face.get(&key).map_or(0., |m| m.0);
                    Ok((
                        p.nodes,
                        pressure[p.compartment]
                            - reflection
                                * (osmotic_law.pressure_pa(concentration[p.compartment])?
                                    - osmotic_law.pressure_pa(local_concentration[i])?),
                    ))
                })
                .collect::<Result<Vec<_>, &'static str>>()?;
            let concentrations: Vec<_> = ports
                .iter()
                .map(|p| {
                    let mut key = p.nodes;
                    key.sort_unstable();
                    let (reflection, conductance) =
                        membrane_by_face.get(&key).copied().unwrap_or((0., 0.));
                    super::ProteinMembrane {
                        concentration_kg_per_m3: concentration[p.compartment],
                        reflection,
                        diffusive_conductance_m3_per_s: conductance,
                    }
                })
                .collect();
            let tissue_report = tissue_trial
                .implicit_cell_pore_boundary_protein_step_with_membranes(
                    &mut tissue_mass,
                    &boundaries,
                    &concentrations,
                    resistances,
                    permeability,
                    viscosity_pa_s,
                    seconds,
                    config.tissue,
                )?;
            let fluid_gain = tissue_trial
                .cell_pore_fluids()
                .iter()
                .zip(&old_fluid)
                .map(|(f, old)| f.fluid_volume_m3 - old)
                .sum::<f64>();
            let protein_gain = tissue_mass
                .iter()
                .zip(tissue_protein_kg.iter())
                .map(|(m, old)| m - old)
                .sum::<f64>();
            let mut exchange_water = vec![0.; n];
            let mut exchange_protein = vec![0.; n];
            let model = tissue_trial.deformed_darcy_with_boundaries(
                permeability,
                viscosity_pa_s,
                &boundaries,
            )?;
            for (face, flow) in model
                .faces()
                .iter()
                .zip(&tissue_report.flow.face_flows_m3_per_s)
            {
                if face.neighbor.is_some() {
                    continue;
                }
                let mut key = face.nodes;
                key.sort_unstable();
                let compartment = *by_face.get(&key).ok_or("unmapped vascular pore face")?;
                let donor_concentration = if *flow < 0. {
                    concentration[compartment]
                } else {
                    tissue_mass[face.owner]
                        / tissue_trial.cell_pore_fluids()[face.owner].fluid_volume_m3
                };
                exchange_water[compartment] += seconds * flow;
                let (reflection, conductance) =
                    membrane_by_face.get(&key).copied().unwrap_or((0., 0.));
                let tissue_concentration = tissue_mass[face.owner]
                    / tissue_trial.cell_pore_fluids()[face.owner].fluid_volume_m3;
                exchange_protein[compartment] += seconds
                    * ((1. - reflection) * flow * donor_concentration
                        + conductance * (tissue_concentration - concentration[compartment]));
            }
            let mut blood_trial = blood.clone();
            let mut blood_mass = blood_protein_kg.to_vec();
            let circulation_report = blood_trial.step_with_protein_exchange(
                &mut blood_mass,
                seconds,
                elastance,
                external,
                &exchange_water,
                &exchange_protein,
                config.circulation_iterations,
                config.circulation_tolerance_m3,
            )?;
            let next_pressure = blood_trial.pressures();
            let next_concentration: Vec<_> = blood_mass
                .iter()
                .zip(blood_trial.volumes())
                .map(|(m, v)| m / v)
                .collect();
            if next_concentration
                .iter()
                .chain(next_pressure)
                .any(|v| !v.is_finite())
            {
                return Err("vascular pore response overflow");
            }
            let mut pressure_residual = connected
                .iter()
                .map(|i| (next_pressure[*i] - pressure[*i]).abs())
                .fold(0., f64::max);
            let mut concentration_residual = connected
                .iter()
                .map(|i| (next_concentration[*i] - concentration[*i]).abs())
                .fold(0., f64::max);
            let mut next_local = local_concentration.clone();
            if osmotic_law.active() {
                for (i, p) in ports.iter().enumerate() {
                    next_local[i] = tissue_mass[owners[i]]
                        / tissue_trial.cell_pore_fluids()[owners[i]].fluid_volume_m3;
                    let mut key = p.nodes;
                    key.sort_unstable();
                    let reflection = membrane_by_face.get(&key).map_or(0., |m| m.0);
                    let next_effective = next_pressure[p.compartment]
                        - reflection
                            * (osmotic_law.pressure_pa(next_concentration[p.compartment])?
                                - osmotic_law.pressure_pa(next_local[i])?);
                    if !next_local[i].is_finite() || !next_effective.is_finite() {
                        return Err("osmotic pore response overflow");
                    }
                    pressure_residual =
                        pressure_residual.max((next_effective - boundaries[i].1).abs());
                    if reflection > 0. {
                        concentration_residual = concentration_residual
                            .max((next_local[i] - local_concentration[i]).abs());
                    }
                }
            }
            observer(iteration + 1, pressure_residual, concentration_residual);
            if pressure_residual <= config.pressure_tolerance_pa
                && concentration_residual <= config.concentration_tolerance_kg_per_m3
            {
                let water_drift = tissue_trial
                    .cell_pore_fluids()
                    .iter()
                    .map(|f| f.fluid_volume_m3)
                    .sum::<f64>()
                    + blood_trial.total_volume()
                    - total_water;
                let mass_drift =
                    tissue_mass.iter().chain(blood_mass.iter()).sum::<f64>() - total_protein;
                if !water_drift.is_finite()
                    || !mass_drift.is_finite()
                    || water_drift.abs() > 1e-10 * total_water
                    || mass_drift.abs() > 1e-12 * total_protein
                {
                    return Err("vascular pore joint inventory imbalance");
                }
                tissue_protein_kg.copy_from_slice(&tissue_mass);
                blood_protein_kg.copy_from_slice(&blood_mass);
                *self = tissue_trial;
                *blood = blood_trial;
                return Ok(VascularPoreReport {
                    iterations: iteration + 1,
                    pressure_residual_pa: pressure_residual,
                    concentration_residual_kg_per_m3: concentration_residual,
                    tissue_fluid_gain_m3: fluid_gain,
                    tissue_protein_gain_kg: protein_gain,
                    tissue: tissue_report,
                    circulation: circulation_report,
                });
            }
            for (c, next) in local_concentration.iter_mut().zip(&next_local) {
                *c += config.relaxation * (*next - *c);
            }
            for i in &connected {
                pressure[*i] += config.relaxation * (next_pressure[*i] - pressure[*i]);
                concentration[*i] +=
                    config.relaxation * (next_concentration[*i] - concentration[*i]);
            }
        }
        Err("vascular pore coupling did not converge")
    }
}
