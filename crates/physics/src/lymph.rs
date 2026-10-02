//! Conservative lumped fluid/protein exchange in SI units.
//! Classical Starling/Kedem–Katchalsky approximation, not a resolved glycocalyx.
//! A node may represent plasma, interstitium or lymph; no fitted defaults.
pub mod profile;
/// Fixed-length cylindrical lymphangion with a passive collapse/stiffening law.
/// Parameters and active circumferential tension are explicit, not fitted defaults.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticWallLaw {
    pub reference_volume_m3: f64,
    pub length_m: f64,
    pub passive_pressure_scale_pa: f64,
    pub stiffening: f64,
    pub external_pressure_pa: f64,
    pub active_tension_n_per_m: f64,
}
impl LymphaticWallLaw {
    /// Radius from actual fluid volume at fixed length.
    pub fn radius_m(self, volume_m3: f64) -> Result<f64, &'static str> {
        if [
            self.reference_volume_m3,
            self.length_m,
            self.passive_pressure_scale_pa,
            self.stiffening,
            volume_m3,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || !self.external_pressure_pa.is_finite()
            || !self.active_tension_n_per_m.is_finite()
            || self.active_tension_n_per_m < 0.
        {
            return Err("invalid lymphatic wall state");
        }
        let radius = (volume_m3 / self.length_m / std::f64::consts::PI).sqrt();
        if !radius.is_finite() || radius <= 0. {
            return Err("lymphatic radius overflow");
        }
        Ok(radius)
    }
    /// P = Pext + scale*(exp(k*(r/r0-1))-(r0/r)^3) + Tactive/r.
    /// No clipping: collapse singularity or exponential overflow rejects the step.
    pub fn pressure_pa(self, volume_m3: f64) -> Result<f64, &'static str> {
        let radius = self.radius_m(volume_m3)?;
        let stretch = (volume_m3 / self.reference_volume_m3).sqrt();
        let passive = self.passive_pressure_scale_pa
            * ((self.stiffening * (stretch - 1.)).exp() - stretch.powi(-3));
        let p = self.external_pressure_pa + passive + self.active_tension_n_per_m / radius;
        if !p.is_finite() {
            return Err("lymphatic wall pressure overflow");
        }
        Ok(p)
    }
}
/// Poiseuille tube segment in series with an exchange edge's fixed resistance.
/// Radius follows an endpoint wall compartment; segment length is explicit.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticHydraulicAttachment {
    pub edge: usize,
    pub compartment: usize,
    pub segment_length_m: f64,
    pub viscosity_pa_s: f64,
}
/// Circular Newtonian segment diagnostics; not a validity guarantee or fitted physiology.
#[derive(Clone, Copy, Debug)]
pub struct LymphaticTubeDiagnostics {
    pub radius_m: f64,
    pub mean_velocity_m_per_s: f64,
    pub reynolds: f64,
    pub womersley: f64,
    pub tube_resistance_pa_s_per_m3: f64,
    pub inertance_pa_s2_per_m3: f64,
    pub relaxation_seconds: f64,
    pub inertial_to_resistive_ratio: f64,
}
impl LymphaticHydraulicAttachment {
    /// Backward-Euler segment momentum response at a supplied trial geometry.
    /// Returns signed flow and its pressure tangent. Pressure drive must include
    /// osmosis and active/external wall pressure before calling this method.
    /// An ideal valve may retain forward flow against reversed pressure while
    /// inertia decays. This response alone does not advance compartment inventory.
    /// # Errors
    /// Rejects invalid coefficients, geometry, time step or previous valve flow.
    pub fn momentum_response(
        self,
        wall: LymphaticWallLaw,
        volume_m3: f64,
        density_kg_per_m3: f64,
        fixed_resistance_pa_s_per_m3: f64,
        pressure_drive_pa: f64,
        previous_flow_m3_per_s: f64,
        seconds: f64,
        valve: bool,
    ) -> Result<(f64, f64), &'static str> {
        if !fixed_resistance_pa_s_per_m3.is_finite() || fixed_resistance_pa_s_per_m3 < 0. {
            return Err("invalid lymphatic series resistance");
        }
        let mut pipe = crate::circulation::Vessel::rigid_pipe(
            0,
            1,
            self.segment_length_m,
            wall.radius_m(volume_m3)?,
            self.viscosity_pa_s,
            density_kg_per_m3,
            0.,
        )?;
        pipe.resistance += fixed_resistance_pa_s_per_m3;
        pipe.valve = valve;
        pipe.flow(pressure_drive_pa, previous_flow_m3_per_s, seconds)
    }

    /// Diagnose supplied signed flow and forcing period with explicit density.
    /// Uses the same rigid-pipe coefficients as the inertial circulation solver.
    /// # Errors
    /// Invalid geometry, density, period, fixed resistance or derived overflow.
    pub fn diagnostics(
        self,
        wall: LymphaticWallLaw,
        volume_m3: f64,
        density_kg_per_m3: f64,
        flow_m3_per_s: f64,
        fixed_resistance_pa_s_per_m3: f64,
        forcing_period_seconds: f64,
    ) -> Result<LymphaticTubeDiagnostics, &'static str> {
        if !flow_m3_per_s.is_finite()
            || !fixed_resistance_pa_s_per_m3.is_finite()
            || fixed_resistance_pa_s_per_m3 < 0.
            || !forcing_period_seconds.is_finite()
            || forcing_period_seconds <= 0.
        {
            return Err("invalid lymphatic diagnostic parameters");
        }
        let radius = wall.radius_m(volume_m3)?;
        let pipe = crate::circulation::Vessel::rigid_pipe(
            0,
            1,
            self.segment_length_m,
            radius,
            self.viscosity_pa_s,
            density_kg_per_m3,
            0.,
        )?;
        let area = std::f64::consts::PI * radius * radius;
        let velocity = flow_m3_per_s / area;
        let reynolds = 2. * density_kg_per_m3 * radius * velocity.abs() / self.viscosity_pa_s;
        let omega = 2. * std::f64::consts::PI / forcing_period_seconds;
        let womersley = radius * (omega * density_kg_per_m3 / self.viscosity_pa_s).sqrt();
        let relaxation = pipe.inertance / (pipe.resistance + fixed_resistance_pa_s_per_m3);
        let ratio = omega * relaxation;
        if [velocity, reynolds, womersley, relaxation, ratio]
            .iter()
            .any(|v| !v.is_finite())
            || !(pipe.resistance + fixed_resistance_pa_s_per_m3).is_finite()
        {
            return Err("lymphatic diagnostic overflow");
        }
        Ok(LymphaticTubeDiagnostics {
            radius_m: radius,
            mean_velocity_m_per_s: velocity,
            reynolds,
            womersley,
            tube_resistance_pa_s_per_m3: pipe.resistance,
            inertance_pa_s2_per_m3: pipe.inertance,
            relaxation_seconds: relaxation,
            inertial_to_resistive_ratio: ratio,
        })
    }
}

/// Adjust base constitutive rates once; base edge metadata is the fixed series element.
/// # Errors
/// Invalid mapping/state, nonpositive resistance or overflow leaves rates unchanged.
pub fn apply_wall_hydraulic_resistances(
    volumes: &[f64],
    proteins: &[f64],
    edges: &[Exchange],
    walls: &[Option<LymphaticWallLaw>],
    attachments: &[LymphaticHydraulicAttachment],
    rates: &mut [(f64, f64)],
) -> Result<(), &'static str> {
    if rates.len() != edges.len() || proteins.len() != volumes.len() || walls.len() != volumes.len()
    {
        return Err("invalid wall hydraulic dimensions");
    }
    if volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
        || proteins.iter().any(|m| !m.is_finite() || *m < 0.)
        || rates.iter().any(|(q, j)| !q.is_finite() || !j.is_finite())
    {
        return Err("invalid wall hydraulic state");
    }
    let mut linked = std::collections::BTreeSet::new();
    let mut added = std::collections::BTreeMap::<usize, f64>::new();
    for a in attachments {
        let e = edges.get(a.edge).ok_or("invalid wall hydraulic edge")?;
        let wall = walls
            .get(a.compartment)
            .and_then(|w| *w)
            .ok_or("missing hydraulic wall")?;
        if (e.from != a.compartment && e.to != a.compartment)
            || !a.segment_length_m.is_finite()
            || a.segment_length_m <= 0.
            || !a.viscosity_pa_s.is_finite()
            || a.viscosity_pa_s <= 0.
            || !e.hydraulic_m3_per_pa_s.is_finite()
            || e.hydraulic_m3_per_pa_s <= 0.
            || !e.reflection.is_finite()
            || !(0. ..=1.).contains(&e.reflection)
            || !e.protein_permeability_m3_per_s.is_finite()
            || e.protein_permeability_m3_per_s < 0.
            || !linked.insert((a.edge, a.compartment))
        {
            return Err("invalid wall hydraulic attachment");
        }
        let radius = wall.radius_m(volumes[a.compartment])?;
        let resistance =
            8. * a.viscosity_pa_s * a.segment_length_m / (std::f64::consts::PI * radius.powi(4));
        if !resistance.is_finite() || resistance <= 0. {
            return Err("wall hydraulic resistance overflow");
        }
        *added.entry(a.edge).or_default() += resistance;
    }
    // Stage every result: an invalid later edge must not partially change rates.
    let mut replacements = Vec::new();
    for (index, resistance) in added {
        let e = &edges[index];
        if e.from >= volumes.len() || e.to >= volumes.len() {
            return Err("invalid wall hydraulic endpoint");
        }
        let base = 1. / e.hydraulic_m3_per_pa_s;
        let total = base + resistance;
        if !total.is_finite() || total <= 0. {
            return Err("wall hydraulic resistance overflow");
        }
        let (old_q, _) = rates[index];
        let q = old_q * (base / total);
        let c0 = proteins[e.from] / volumes[e.from];
        let c1 = proteins[e.to] / volumes[e.to];
        if !c0.is_finite() || !c1.is_finite() {
            return Err("wall hydraulic concentration overflow");
        }
        if e.valve && old_q < 0. {
            return Err("invalid reverse valve flux");
        }
        let j = if e.valve && old_q == 0. {
            0.
        } else {
            (1. - e.reflection) * q * if q >= 0. { c0 } else { c1 }
                + e.protein_permeability_m3_per_s * (c0 - c1)
        };
        if !q.is_finite() || !j.is_finite() {
            return Err("wall hydraulic flux overflow");
        }
        replacements.push((index, (q, j)));
    }
    for (index, rate) in replacements {
        rates[index] = rate;
    }
    Ok(())
}

pub(crate) fn apply_wall_pressures(
    volumes: &[f64],
    pressures: &mut [f64],
    walls: &[Option<LymphaticWallLaw>],
) -> Result<(), &'static str> {
    if walls.len() != volumes.len() || pressures.len() != volumes.len() {
        return Err("invalid lymphatic wall count");
    }
    for ((v, p), wall) in volumes.iter().zip(pressures).zip(walls) {
        if let Some(wall) = wall {
            *p = wall.pressure_pa(*v)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct FluidSpace {
    pub reference_volume_m3: f64,
    pub reference_pressure_pa: f64,
    pub compliance_m3_per_pa: f64,
    pub initial_volume_m3: f64,
    pub initial_protein_kg: f64,
    /// Effective linear oncotic coefficient, Pa per (kg/m³).
    /// Must be supplied for the represented protein mixture.
    pub oncotic_pa_per_kg_m3: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Exchange {
    pub from: usize,
    pub to: usize,
    pub hydraulic_m3_per_pa_s: f64,
    pub reflection: f64,
    pub protein_permeability_m3_per_s: f64,
    /// Forward pressure supplied by an external pump; prescribed, not a muscle law.
    pub pump_head_pa: f64,
    pub valve: bool,
}
#[derive(Clone, Debug)]
pub struct LymphNetwork {
    spaces: Vec<FluidSpace>,
    edges: Vec<Exchange>,
    volumes: Vec<f64>,
    proteins: Vec<f64>,
    accepted_pressures: Option<Vec<f64>>,
    accepted_rates: Option<Vec<(f64, f64)>>,
}
/// Fixed rigid conduit attached to one network exchange edge.
#[derive(Clone, Debug)]
pub struct RadialExchange {
    pub edge: usize,
    pub length_m: f64,
    pub pipe: profile::RadialPipe,
}
#[derive(Clone, Debug)]
pub struct ExchangeReport {
    pub substeps: usize,
    pub transferred_volume_m3: Vec<f64>,
    pub transferred_protein_kg: Vec<f64>,
    pub volume_drift_m3: f64,
    pub protein_drift_kg: f64,
}
/// Local step-doubling tolerances, in SI units; no physiological defaults.
#[derive(Clone, Copy, Debug)]
pub struct AdaptiveExchangeConfig {
    pub relative_tolerance: f64,
    pub absolute_volume_tolerance_m3: f64,
    pub absolute_protein_tolerance_kg: f64,
    pub min_step_seconds: f64,
    pub max_step_seconds: f64,
    pub max_trials: usize,
}
#[derive(Clone, Debug)]
pub struct AdaptiveExchangeReport {
    pub exchange: ExchangeReport,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub max_accepted_error_ratio: f64,
}

/// Constitutive rates for explicitly supplied, space-specific nonlinear laws.
pub(crate) fn osmotic_exchange_rates(
    volumes: &[f64],
    proteins: &[f64],
    pressures: &[f64],
    edges: &[Exchange],
    laws: &[crate::biomechanics::OsmoticPressureLaw],
) -> Result<Vec<(f64, f64)>, &'static str> {
    let n = volumes.len();
    if proteins.len() != n
        || pressures.len() != n
        || laws.len() != n
        || volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
        || proteins.iter().any(|m| !m.is_finite() || *m < 0.)
        || pressures.iter().any(|p| !p.is_finite())
    {
        return Err("invalid osmotic exchange state");
    }
    let c: Vec<_> = proteins.iter().zip(volumes).map(|(m, v)| m / v).collect();
    let pi: Vec<_> = laws
        .iter()
        .zip(&c)
        .map(|(law, c)| law.pressure_pa(*c))
        .collect::<Result<_, _>>()?;
    edges
        .iter()
        .map(|e| {
            if e.from >= n || e.to >= n {
                return Err("invalid osmotic exchange edge");
            }
            let drive = pressures[e.from] - pressures[e.to] + e.pump_head_pa
                - e.reflection * (pi[e.from] - pi[e.to]);
            let q = e.hydraulic_m3_per_pa_s * if e.valve { drive.max(0.) } else { drive };
            let donor = if q >= 0. { c[e.from] } else { c[e.to] };
            let j = if e.valve && drive <= 0. {
                0.
            } else {
                (1. - e.reflection) * q * donor
                    + e.protein_permeability_m3_per_s * (c[e.from] - c[e.to])
            };
            if !drive.is_finite() || !q.is_finite() || !j.is_finite() {
                Err("osmotic exchange flux overflow")
            } else {
                Ok((q, j))
            }
        })
        .collect()
}

impl LymphNetwork {
    /// # Errors
    /// Rejects invalid topology, properties or nonfinite initial fluid state.
    pub fn new(spaces: Vec<FluidSpace>, edges: Vec<Exchange>) -> Result<Self, &'static str> {
        if spaces.len() < 2 || spaces.len() > 250_000 || edges.is_empty() || edges.len() > 1_000_000
        {
            return Err("invalid exchange network size");
        }
        for s in &spaces {
            if [
                s.reference_volume_m3,
                s.compliance_m3_per_pa,
                s.initial_volume_m3,
            ]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.)
                || !s.reference_pressure_pa.is_finite()
                || [s.initial_protein_kg, s.oncotic_pa_per_kg_m3]
                    .iter()
                    .any(|x| !x.is_finite() || *x < 0.)
            {
                return Err("invalid fluid space");
            }
        }
        for e in &edges {
            if e.from >= spaces.len()
                || e.to >= spaces.len()
                || e.from == e.to
                || !e.hydraulic_m3_per_pa_s.is_finite()
                || e.hydraulic_m3_per_pa_s < 0.
                || !e.reflection.is_finite()
                || !(0. ..=1.).contains(&e.reflection)
                || !e.protein_permeability_m3_per_s.is_finite()
                || e.protein_permeability_m3_per_s < 0.
                || !e.pump_head_pa.is_finite()
            {
                return Err("invalid exchange edge");
            }
        }
        let network = Self {
            accepted_pressures: None,
            accepted_rates: None,
            volumes: spaces.iter().map(|s| s.initial_volume_m3).collect(),
            proteins: spaces.iter().map(|s| s.initial_protein_kg).collect(),
            spaces,
            edges,
        };
        if !network.total_volume().is_finite() || !network.total_protein().is_finite() {
            return Err("fluid totals overflow");
        }
        network.rates()?;
        Ok(network)
    }
    #[must_use]
    pub fn volumes(&self) -> &[f64] {
        &self.volumes
    }
    #[must_use]
    pub fn protein_masses(&self) -> &[f64] {
        &self.proteins
    }
    #[must_use]
    pub fn total_volume(&self) -> f64 {
        self.volumes.iter().sum()
    }
    #[must_use]
    pub fn total_protein(&self) -> f64 {
        self.proteins.iter().sum()
    }
    #[must_use]
    pub fn pressures(&self) -> Vec<f64> {
        self.accepted_pressures
            .clone()
            .unwrap_or_else(|| self.linear_pressures())
    }
    fn linear_pressures(&self) -> Vec<f64> {
        self.spaces
            .iter()
            .zip(&self.volumes)
            .map(|(s, v)| {
                s.reference_pressure_pa + (v - s.reference_volume_m3) / s.compliance_m3_per_pa
            })
            .collect()
    }
    /// Simultaneous backward-Euler water, protein and segment momentum step.
    /// Flow history must contain one entry per edge; linked edges use inertia,
    /// others retain their osmotic resistance law. Nonconvergence is atomic.
    /// This wall-network step does not equilibrate an attached FEM tissue.
    /// # Errors
    /// Invalid inputs, nonpositive trial inventory or failed fixed-point solve.
    pub fn step_with_inertial_walls(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        walls: &[Option<LymphaticWallLaw>],
        links: &[LymphaticHydraulicAttachment],
        density: f64,
        flow_history: &mut [f64],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
    ) -> Result<ExchangeReport, &'static str> {
        self.step_with_inertial_callbacks(
            seconds,
            laws,
            walls,
            links,
            density,
            flow_history,
            max_iterations,
            volume_tolerance,
            protein_tolerance,
            |v, linear| {
                let mut p = linear.to_vec();
                apply_wall_pressures(v, &mut p, walls)?;
                Ok(p)
            },
            |_, _, _, _, rates| Ok(rates.to_vec()),
        )
    }
    pub(crate) fn step_with_inertial_callbacks<F, G>(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        walls: &[Option<LymphaticWallLaw>],
        links: &[LymphaticHydraulicAttachment],
        density: f64,
        flow_history: &mut [f64],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
        mut pressure_law: F,
        mut flux_law: G,
    ) -> Result<ExchangeReport, &'static str>
    where
        F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
        G: FnMut(
            &[f64],
            &[f64],
            &[f64],
            &[Exchange],
            &[(f64, f64)],
        ) -> Result<Vec<(f64, f64)>, &'static str>,
    {
        if !seconds.is_finite()
            || seconds <= 0.
            || flow_history.len() != self.edges.len()
            || flow_history.iter().any(|q| !q.is_finite())
            || max_iterations == 0
            || max_iterations > 100000
            || !volume_tolerance.is_finite()
            || volume_tolerance <= 0.
            || !protein_tolerance.is_finite()
            || protein_tolerance <= 0.
            || !density.is_finite()
            || density <= 0.
        {
            return Err("invalid inertial lymph step");
        }
        let mut seen = std::collections::BTreeSet::new();
        for a in links {
            let e = self.edges.get(a.edge).ok_or("invalid inertial edge")?;
            if !seen.insert(a.edge)
                || (e.from != a.compartment && e.to != a.compartment)
                || e.hydraulic_m3_per_pa_s <= 0.
                || walls.get(a.compartment).and_then(|w| *w).is_none()
            {
                return Err("invalid inertial wall link");
            }
        }
        let mut trial = self.clone();
        for _ in 0..max_iterations {
            let p = pressure_law(&trial.volumes, &trial.linear_pressures())?;
            let mut rates =
                osmotic_exchange_rates(&trial.volumes, &trial.proteins, &p, &self.edges, laws)?;
            for a in links {
                let e = self.edges[a.edge];
                let c0 = trial.proteins[e.from] / trial.volumes[e.from];
                let c1 = trial.proteins[e.to] / trial.volumes[e.to];
                let drive = p[e.from] - p[e.to] + e.pump_head_pa
                    - e.reflection
                        * (laws[e.from].pressure_pa(c0)? - laws[e.to].pressure_pa(c1)?);
                let (q, _) = a.momentum_response(
                    walls[a.compartment].unwrap(),
                    trial.volumes[a.compartment],
                    density,
                    1. / e.hydraulic_m3_per_pa_s,
                    drive,
                    flow_history[a.edge],
                    seconds,
                    e.valve,
                )?;
                let j = if e.valve && q == 0. {
                    0.
                } else {
                    (1. - e.reflection) * q * if q >= 0. { c0 } else { c1 }
                        + e.protein_permeability_m3_per_s * (c0 - c1)
                };
                rates[a.edge] = (q, j);
            }
            rates = flux_law(&trial.volumes, &trial.proteins, &p, &self.edges, &rates)?;
            trial.validate_rates(&rates)?;
            let mut v = self.volumes.clone();
            let mut m = self.proteins.clone();
            let mut dv = Vec::with_capacity(rates.len());
            let mut dm = Vec::with_capacity(rates.len());
            for (e, (q, j)) in self.edges.iter().zip(&rates) {
                let water = seconds * q;
                let protein = seconds * j;
                v[e.from] -= water;
                v[e.to] += water;
                m[e.from] -= protein;
                m[e.to] += protein;
                dv.push(water);
                dm.push(protein);
            }
            if v.iter().any(|x| !x.is_finite() || *x <= 0.)
                || m.iter().any(|x| !x.is_finite() || *x < 0.)
            {
                return Err("invalid inertial lymph trial inventory");
            }
            let error = v
                .iter()
                .zip(&trial.volumes)
                .map(|(a, b)| (a - b).abs() / volume_tolerance)
                .chain(
                    m.iter()
                        .zip(&trial.proteins)
                        .map(|(a, b)| (a - b).abs() / protein_tolerance),
                )
                .fold(0_f64, f64::max);
            if error <= 1. {
                // Commit the state at which pressure and momentum were evaluated;
                // its inventory residual is bounded by the supplied SI tolerances.
                let report = ExchangeReport {
                    substeps: 1,
                    transferred_volume_m3: dv,
                    transferred_protein_kg: dm,
                    volume_drift_m3: trial.total_volume() - self.total_volume(),
                    protein_drift_kg: trial.total_protein() - self.total_protein(),
                };
                trial.accepted_pressures = Some(p);
                trial.accepted_rates = Some(rates.clone());
                for (old, (q, _)) in flow_history.iter_mut().zip(rates) {
                    *old = q;
                }
                *self = trial;
                return Ok(report);
            }
            trial.volumes = v;
            trial.proteins = m;
        }
        Err("inertial lymph iteration limit")
    }
    /// Joint backward-Euler inventory exchange through fixed-radius resolved pipes.
    /// Profiles and network commit together; trial iterations restart old profiles.
    /// No wall motion, FEM coupling or automatic time control is supplied here.
    /// # Errors
    /// Invalid mapping, nonpositive inventory or nonconvergence preserves all state.
    pub fn step_with_radial_profiles(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        profiles: &mut [RadialExchange],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
    ) -> Result<ExchangeReport, &'static str> {
        self.step_with_radial_callbacks(
            seconds,
            laws,
            profiles,
            max_iterations,
            volume_tolerance,
            protein_tolerance,
            |_, linear| Ok(linear.to_vec()),
            |_, _, _, _, rates| Ok(rates.to_vec()),
        )
    }
    pub(crate) fn step_with_radial_callbacks<F, G>(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        profiles: &mut [RadialExchange],
        max_iterations: usize,
        volume_tolerance: f64,
        protein_tolerance: f64,
        mut pressure_law: F,
        mut flux_law: G,
    ) -> Result<ExchangeReport, &'static str>
    where
        F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
        G: FnMut(
            &[f64],
            &[f64],
            &[f64],
            &[Exchange],
            &[(f64, f64)],
        ) -> Result<Vec<(f64, f64)>, &'static str>,
    {
        if !seconds.is_finite()
            || seconds <= 0.
            || max_iterations == 0
            || max_iterations > 100000
            || !volume_tolerance.is_finite()
            || volume_tolerance <= 0.
            || !protein_tolerance.is_finite()
            || protein_tolerance <= 0.
        {
            return Err("invalid inertial lymph step");
        }
        let mut seen = std::collections::BTreeSet::new();
        for a in profiles.iter() {
            let e = self
                .edges
                .get(a.edge)
                .ok_or("invalid radial exchange edge")?;
            if !seen.insert(a.edge)
                || !e.hydraulic_m3_per_pa_s.is_finite()
                || e.hydraulic_m3_per_pa_s <= 0.
                || !a.length_m.is_finite()
                || a.length_m <= 0.
            {
                return Err("invalid radial exchange mapping");
            }
        }
        let mut trial = self.clone();
        for _ in 0..max_iterations {
            let p = pressure_law(&trial.volumes, &trial.linear_pressures())?;
            let mut rates =
                osmotic_exchange_rates(&trial.volumes, &trial.proteins, &p, &self.edges, laws)?;
            let mut staged_profiles = profiles.to_vec();
            for a in &mut staged_profiles {
                let e = self.edges[a.edge];
                let c0 = trial.proteins[e.from] / trial.volumes[e.from];
                let c1 = trial.proteins[e.to] / trial.volumes[e.to];
                let drive = p[e.from] - p[e.to] + e.pump_head_pa
                    - e.reflection
                        * (laws[e.from].pressure_pa(c0)? - laws[e.to].pressure_pa(c1)?);
                let (q, closed) = if e.valve {
                    let (response, _, reaction) = a.pipe.step_with_ideal_valve(
                        seconds,
                        drive,
                        a.length_m,
                        1. / e.hydraulic_m3_per_pa_s,
                    )?;
                    (
                        if reaction > 0. {
                            0.
                        } else {
                            response.flow_m3_per_s
                        },
                        reaction > 0. || response.flow_m3_per_s == 0.,
                    )
                } else {
                    (
                        a.pipe
                            .step_with_series_resistance(
                                seconds,
                                drive,
                                a.length_m,
                                1. / e.hydraulic_m3_per_pa_s,
                            )?
                            .0
                            .flow_m3_per_s,
                        false,
                    )
                };
                let j = if closed {
                    0.
                } else {
                    (1. - e.reflection) * q * if q >= 0. { c0 } else { c1 }
                        + e.protein_permeability_m3_per_s * (c0 - c1)
                };
                rates[a.edge] = (q, j);
            }
            rates = flux_law(&trial.volumes, &trial.proteins, &p, &self.edges, &rates)?;
            trial.validate_rates(&rates)?;
            let mut v = self.volumes.clone();
            let mut m = self.proteins.clone();
            let mut dv = Vec::with_capacity(rates.len());
            let mut dm = Vec::with_capacity(rates.len());
            for (e, (q, j)) in self.edges.iter().zip(&rates) {
                let water = seconds * q;
                let protein = seconds * j;
                v[e.from] -= water;
                v[e.to] += water;
                m[e.from] -= protein;
                m[e.to] += protein;
                dv.push(water);
                dm.push(protein);
            }
            if v.iter().any(|x| !x.is_finite() || *x <= 0.)
                || m.iter().any(|x| !x.is_finite() || *x < 0.)
            {
                return Err("invalid inertial lymph trial inventory");
            }
            let error = v
                .iter()
                .zip(&trial.volumes)
                .map(|(a, b)| (a - b).abs() / volume_tolerance)
                .chain(
                    m.iter()
                        .zip(&trial.proteins)
                        .map(|(a, b)| (a - b).abs() / protein_tolerance),
                )
                .fold(0_f64, f64::max);
            if error <= 1. {
                // Commit the state at which pressure and momentum were evaluated;
                // its inventory residual is bounded by the supplied SI tolerances.
                let report = ExchangeReport {
                    substeps: 1,
                    transferred_volume_m3: dv,
                    transferred_protein_kg: dm,
                    volume_drift_m3: trial.total_volume() - self.total_volume(),
                    protein_drift_kg: trial.total_protein() - self.total_protein(),
                };
                trial.accepted_pressures = Some(p);
                trial.accepted_rates = Some(rates.clone());
                profiles.clone_from_slice(&staged_profiles);
                *self = trial;
                return Ok(report);
            }
            trial.volumes = v;
            trial.proteins = m;
        }
        Err("inertial lymph iteration limit")
    }
    /// Step-doubled backward-Euler wall/network integration with momentum error.
    /// Both network and flow history roll back on whole-call failure.
    /// # Errors
    /// Invalid tolerance, failed nonlinear solve or exhausted time-step budget.
    pub fn step_with_inertial_walls_adaptive(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        walls: &[Option<LymphaticWallLaw>],
        links: &[LymphaticHydraulicAttachment],
        density: f64,
        flow_history: &mut [f64],
        config: AdaptiveExchangeConfig,
        absolute_flow_tolerance: f64,
        max_iterations: usize,
        nonlinear_volume_tolerance: f64,
        nonlinear_protein_tolerance: f64,
    ) -> Result<AdaptiveExchangeReport, &'static str> {
        if !absolute_flow_tolerance.is_finite() || absolute_flow_tolerance <= 0. {
            return Err("invalid adaptive flow tolerance");
        }
        let initial = (self.clone(), flow_history.to_vec());
        let (next, report) = adaptive_exchange_order(
            &initial,
            seconds,
            config,
            1,
            |state, h| {
                state.0.step_with_inertial_walls(
                    h,
                    laws,
                    walls,
                    links,
                    density,
                    &mut state.1,
                    max_iterations,
                    nonlinear_volume_tolerance,
                    nonlinear_protein_tolerance,
                )
            },
            |state| &state.0,
            |coarse, fine| {
                let mut error = 0_f64;
                for (a, b) in coarse.1.iter().zip(&fine.1) {
                    let budget =
                        absolute_flow_tolerance + config.relative_tolerance * a.abs().max(b.abs());
                    if !budget.is_finite() || budget <= 0. {
                        return Err("adaptive flow budget overflow");
                    }
                    error = error.max((a - b).abs() / budget);
                }
                Ok(error)
            },
        )?;
        flow_history.copy_from_slice(&next.1);
        *self = next.0;
        Ok(report)
    }
    /// Instantaneous volume (m³/s) and protein (kg/s) edge fluxes, signed from→to.
    /// # Errors
    /// Rejects nonfinite pressure/concentration or flux overflow.
    pub fn rates(&self) -> Result<Vec<(f64, f64)>, &'static str> {
        if let Some(rates) = &self.accepted_rates {
            return Ok(rates.clone());
        }
        self.rates_at(&self.pressures())
    }
    #[must_use]
    pub fn edges(&self) -> &[Exchange] {
        &self.edges
    }
    fn validate_rates(&self, rates: &[(f64, f64)]) -> Result<(), &'static str> {
        if rates.len() != self.edges.len()
            || rates.iter().any(|(q, j)| !q.is_finite() || !j.is_finite())
        {
            return Err("invalid constitutive flux response");
        }
        Ok(())
    }
    fn rates_at(&self, p: &[f64]) -> Result<Vec<(f64, f64)>, &'static str> {
        if p.len() != self.spaces.len() {
            return Err("invalid exchange pressure count");
        }
        let c: Vec<_> = self
            .proteins
            .iter()
            .zip(&self.volumes)
            .map(|(m, v)| m / v)
            .collect();
        if p.iter().chain(&c).any(|x| !x.is_finite()) {
            return Err("fluid state overflow");
        }
        self.edges
            .iter()
            .map(|e| {
                let drive = p[e.from] - p[e.to] + e.pump_head_pa
                    - e.reflection
                        * (self.spaces[e.from].oncotic_pa_per_kg_m3 * c[e.from]
                            - self.spaces[e.to].oncotic_pa_per_kg_m3 * c[e.to]);
                let q = e.hydraulic_m3_per_pa_s * if e.valve { drive.max(0.) } else { drive };
                // Donor concentration gives a conservative positive transport discretization.
                // Reflecting barriers retain the reflected fraction of advected protein.
                let donor = if q >= 0. { c[e.from] } else { c[e.to] };
                let j = if e.valve && drive <= 0. {
                    0.
                } else {
                    (1. - e.reflection) * q * donor
                        + e.protein_permeability_m3_per_s * (c[e.from] - c[e.to])
                };
                if !q.is_finite() || !j.is_finite() {
                    Err("exchange flux overflow")
                } else {
                    Ok((q, j))
                }
            })
            .collect()
    }
    /// Evaluate alternative per-space protein pressure laws at current state.
    /// # Errors
    /// Rejects invalid law dimensions, coefficients, concentrations or overflow.
    pub fn rates_with_osmotic_laws(
        &self,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
    ) -> Result<Vec<(f64, f64)>, &'static str> {
        osmotic_exchange_rates(
            &self.volumes,
            &self.proteins,
            &self.pressures(),
            &self.edges,
            laws,
        )
    }
    /// Conservative adaptive explicit exchange using per-space nonlinear laws.
    /// Accepted rates retain the selected law's response; no fitted defaults.
    /// # Errors
    /// Invalid law, overflow, depletion or substep failure preserves all state.
    pub fn step_with_osmotic_laws(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
    ) -> Result<ExchangeReport, &'static str> {
        self.rates_with_osmotic_laws(laws)?;
        self.step_with_pressure_and_flux_laws(
            seconds,
            max_step_seconds,
            |_, p| Ok(p.to_vec()),
            |v, m, p, e, _| osmotic_exchange_rates(v, m, p, e, laws),
        )
    }
    /// Conservative SSPRK2 passive/active wall and space-specific protein exchange.
    /// Both Euler stages enforce outgoing-inventory limits; no error-tolerance claim.
    /// Tension is held during this call; supply a new value for each activation phase.
    /// # Errors
    /// Invalid wall/law or failed integration preserves every accepted state.
    pub fn step_with_wall_and_osmotic_laws(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        walls: &[Option<LymphaticWallLaw>],
    ) -> Result<ExchangeReport, &'static str> {
        self.rates_with_osmotic_laws(laws)?;
        let mut initial = self.pressures();
        apply_wall_pressures(&self.volumes, &mut initial, walls)?;
        self.step_with_pressure_and_flux_laws_order(
            seconds,
            max_step_seconds,
            true,
            |v, p| {
                let mut p = p.to_vec();
                apply_wall_pressures(v, &mut p, walls)?;
                Ok(p)
            },
            |v, m, p, e, _| osmotic_exchange_rates(v, m, p, e, laws),
        )
    }
    /// Error-controlled wall exchange by SSPRK2 step doubling. Accepts two half
    /// steps without extrapolation, preserving positivity and conservative ledgers.
    /// Controls local volume/protein and integrated edge-transfer discrepancies;
    /// this is an error estimate, not a rigorous global-error bound.
    /// Activation and external pressure are held over this call.
    /// # Errors
    /// Invalid tolerances, failed laws, minimum-step/trial exhaustion or overflow
    /// preserve the entire original network, including earlier successful trials.
    pub fn step_with_wall_and_osmotic_laws_adaptive(
        &mut self,
        seconds: f64,
        laws: &[crate::biomechanics::OsmoticPressureLaw],
        walls: &[Option<LymphaticWallLaw>],
        config: AdaptiveExchangeConfig,
    ) -> Result<AdaptiveExchangeReport, &'static str> {
        let (next, report) = adaptive_exchange(
            self,
            seconds,
            config,
            |state, h| state.step_with_wall_and_osmotic_laws(h, h, laws, walls),
            |state| state,
            |_, _| Ok(0.),
        )?;
        *self = next;
        Ok(report)
    }
    /// Explicit conservative integration with user-controlled maximum step and
    /// outgoing-volume/protein positivity limits. No error-tolerance claim:
    /// verify temporal convergence by reducing `max_step_seconds`.
    /// Failure leaves the entire network unchanged.
    /// # Errors
    /// Rejects invalid time, nonfinite state, underflow or substep exhaustion.
    pub fn step(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
    ) -> Result<ExchangeReport, &'static str> {
        self.step_with_pressure_law(seconds, max_step_seconds, |_, p| Ok(p.to_vec()))
    }
    /// Replace compartment pressures with a coupled constitutive response.
    /// Callback receives fluid volumes and nominal linear pressures. It must
    /// return one finite pressure per space. External callback side effects are
    /// not rolled back by this method; adapters should use private trial states.
    /// # Errors
    /// Also rejects callback errors, invalid pressure dimensions or nonfinite pressures.
    pub fn step_with_pressure_law<F>(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
        law: F,
    ) -> Result<ExchangeReport, &'static str>
    where
        F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
    {
        self.step_with_pressure_and_flux_laws(
            seconds,
            max_step_seconds,
            law,
            |_, _, _, _, rates| Ok(rates.to_vec()),
        )
    }
    /// Coupled pressure and flux constitutive callbacks. Flow receives volumes,
    /// protein masses, actual pressures, edges and the default exchange rates.
    /// Accepted custom rates are retained for instantaneous flux diagnostics.
    /// External callback side effects require an adapter-owned transaction.
    /// # Errors
    /// Rejects callback errors, bad dimensions/nonfinite states or exhausted steps.
    pub fn step_with_pressure_and_flux_laws<F, G>(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
        law: F,
        flux: G,
    ) -> Result<ExchangeReport, &'static str>
    where
        F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
        G: FnMut(
            &[f64],
            &[f64],
            &[f64],
            &[Exchange],
            &[(f64, f64)],
        ) -> Result<Vec<(f64, f64)>, &'static str>,
    {
        self.step_with_pressure_and_flux_laws_order(seconds, max_step_seconds, false, law, flux)
    }
    /// Conservative SSPRK2 when selected; caller-owned callback effects require staging.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_with_pressure_and_flux_laws_order<F, G>(
        &mut self,
        seconds: f64,
        max_step_seconds: f64,
        second_order: bool,
        mut law: F,
        mut flux: G,
    ) -> Result<ExchangeReport, &'static str>
    where
        F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
        G: FnMut(
            &[f64],
            &[f64],
            &[f64],
            &[Exchange],
            &[(f64, f64)],
        ) -> Result<Vec<(f64, f64)>, &'static str>,
    {
        if !seconds.is_finite()
            || seconds <= 0.
            || !max_step_seconds.is_finite()
            || max_step_seconds <= 0.
        {
            return Err("invalid exchange time step");
        }
        let mut next = self.clone();
        let mut report = ExchangeReport {
            substeps: 0,
            transferred_volume_m3: vec![0.; self.edges.len()],
            transferred_protein_kg: vec![0.; self.edges.len()],
            volume_drift_m3: 0.,
            protein_drift_kg: 0.,
        };
        let mut elapsed = 0.;
        while elapsed < seconds {
            if report.substeps >= 100_000 {
                return Err("exchange substep limit");
            }
            let pressures = law(next.volumes(), &next.linear_pressures())?;
            let default = next.rates_at(&pressures)?;
            let rates = flux(
                next.volumes(),
                next.protein_masses(),
                &pressures,
                &next.edges,
                &default,
            )?;
            next.validate_rates(&rates)?;
            let mut out_v = vec![0.; next.spaces.len()];
            let mut out_m = out_v.clone();
            for (e, (q, j)) in next.edges.iter().zip(&rates) {
                out_v[if *q >= 0. { e.from } else { e.to }] += q.abs();
                out_m[if *j >= 0. { e.from } else { e.to }] += j.abs();
            }
            let mut h = max_step_seconds.min(seconds - elapsed);
            for i in 0..next.spaces.len() {
                if out_v[i] > 0. {
                    h = h.min(0.1 * next.volumes[i] / out_v[i]);
                }
                if out_m[i] > 0. {
                    h = h.min(0.1 * next.proteins[i] / out_m[i]);
                }
            }
            if !h.is_finite() || h <= 0. || elapsed + h <= elapsed {
                return Err("exchange step underflow");
            }
            let rates = if second_order {
                let mut accepted = None;
                for _ in 0..64 {
                    if !h.is_finite() || h <= 0. || elapsed + h <= elapsed {
                        return Err("exchange predictor underflow");
                    }
                    let mut predictor = next.clone();
                    for (e, (q, j)) in predictor.edges.iter().zip(&rates) {
                        predictor.volumes[e.from] -= h * q;
                        predictor.volumes[e.to] += h * q;
                        predictor.proteins[e.from] -= h * j;
                        predictor.proteins[e.to] += h * j;
                    }
                    if predictor.volumes.iter().any(|v| !v.is_finite() || *v <= 0.)
                        || predictor.proteins.iter().any(|m| !m.is_finite() || *m < 0.)
                    {
                        return Err("invalid exchange predictor");
                    }
                    let p = law(predictor.volumes(), &predictor.linear_pressures())?;
                    let default = predictor.rates_at(&p)?;
                    let stage = flux(
                        predictor.volumes(),
                        predictor.protein_masses(),
                        &p,
                        &predictor.edges,
                        &default,
                    )?;
                    predictor.validate_rates(&stage)?;
                    let mut out_v = vec![0.; predictor.spaces.len()];
                    let mut out_m = out_v.clone();
                    for (e, (q, j)) in predictor.edges.iter().zip(&stage) {
                        out_v[if *q >= 0. { e.from } else { e.to }] += q.abs();
                        out_m[if *j >= 0. { e.from } else { e.to }] += j.abs();
                    }
                    // A convex combination of two positive Euler stages preserves
                    // positivity without clipping or redistribution of either inventory.
                    if (0..out_v.len()).any(|i| {
                        h * out_v[i] > 0.1 * predictor.volumes[i]
                            || h * out_m[i] > 0.1 * predictor.proteins[i]
                    }) {
                        h *= 0.5;
                        continue;
                    }
                    accepted = Some(
                        rates
                            .iter()
                            .zip(stage)
                            .map(|((q, j), (q2, j2))| (0.5 * q + 0.5 * q2, 0.5 * j + 0.5 * j2))
                            .collect::<Vec<_>>(),
                    );
                    break;
                }
                accepted.ok_or("exchange predictor retry limit")?
            } else {
                rates
            };
            let mut dv = vec![0.; next.spaces.len()];
            let mut dm = dv.clone();
            for (k, (e, (q, j))) in next.edges.iter().zip(&rates).enumerate() {
                let v = h * q;
                let m = h * j;
                dv[e.from] -= v;
                dv[e.to] += v;
                dm[e.from] -= m;
                dm[e.to] += m;
                report.transferred_volume_m3[k] += v;
                report.transferred_protein_kg[k] += m;
            }
            for i in 0..next.spaces.len() {
                next.volumes[i] += dv[i];
                next.proteins[i] += dm[i];
                if !next.volumes[i].is_finite()
                    || next.volumes[i] <= 0.
                    || !next.proteins[i].is_finite()
                    || next.proteins[i] < 0.
                {
                    return Err("invalid updated fluid state");
                }
            }
            elapsed += h;
            report.substeps += 1;
        }
        let pressures = law(next.volumes(), &next.linear_pressures())?;
        let default = next.rates_at(&pressures)?;
        let rates = flux(
            next.volumes(),
            next.protein_masses(),
            &pressures,
            &next.edges,
            &default,
        )?;
        next.validate_rates(&rates)?;
        next.accepted_rates = Some(rates);
        next.accepted_pressures = Some(pressures);
        report.volume_drift_m3 = next.total_volume() - self.total_volume();
        report.protein_drift_kg = next.total_protein() - self.total_protein();
        if report
            .transferred_volume_m3
            .iter()
            .chain(&report.transferred_protein_kg)
            .chain([&report.volume_drift_m3, &report.protein_drift_kg])
            .any(|x| !x.is_finite())
        {
            return Err("exchange report overflow");
        }
        *self = next;
        Ok(report)
    }
}

impl Exchange {
    /// Isotropic Darcy interface with two half-cell resistances in series.
    /// Distances are positive normal distances to the shared face. This two-point
    /// law is consistent for orthogonal cell centers, not arbitrary oblique grids.
    /// Protein follows donor advection; there is no osmotic membrane or valve.
    /// # Errors
    /// Rejects invalid geometry, permeability/viscosity or conductance overflow.
    pub fn darcy(
        from: usize,
        to: usize,
        area_m2: f64,
        distances_m: [f64; 2],
        permeabilities_m2: [f64; 2],
        viscosity_pa_s: f64,
    ) -> Result<Self, &'static str> {
        if from == to
            || !area_m2.is_finite()
            || area_m2 <= 0.
            || !viscosity_pa_s.is_finite()
            || viscosity_pa_s <= 0.
            || distances_m.iter().any(|x| !x.is_finite() || *x <= 0.)
            || permeabilities_m2.iter().any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid Darcy interface");
        }
        let conductance = if permeabilities_m2.contains(&0.) {
            0.
        } else {
            let resistance = viscosity_pa_s
                * (distances_m[0] / permeabilities_m2[0] + distances_m[1] / permeabilities_m2[1])
                / area_m2;
            if !resistance.is_finite() || resistance <= 0. {
                return Err("Darcy resistance overflow");
            }
            1. / resistance
        };
        if !conductance.is_finite() {
            return Err("Darcy conductance overflow");
        }
        Ok(Self {
            from,
            to,
            hydraulic_m3_per_pa_s: conductance,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: false,
        })
    }
}

/// Shared step-doubling controller for staged network or coupled FEM/network states.
pub(crate) fn adaptive_exchange<T: Clone>(
    initial: &T,
    seconds: f64,
    config: AdaptiveExchangeConfig,
    advance: impl FnMut(&mut T, f64) -> Result<ExchangeReport, &'static str>,
    network: impl Fn(&T) -> &LymphNetwork,
    extra_error: impl Fn(&T, &T) -> Result<f64, &'static str>,
) -> Result<(T, AdaptiveExchangeReport), &'static str> {
    adaptive_exchange_order(initial, seconds, config, 2, advance, network, extra_error)
}

pub(crate) fn adaptive_exchange_order<T: Clone>(
    initial: &T,
    seconds: f64,
    config: AdaptiveExchangeConfig,
    order: u32,
    mut advance: impl FnMut(&mut T, f64) -> Result<ExchangeReport, &'static str>,
    network: impl Fn(&T) -> &LymphNetwork,
    extra_error: impl Fn(&T, &T) -> Result<f64, &'static str>,
) -> Result<(T, AdaptiveExchangeReport), &'static str> {
    if !(1..=2).contains(&order) {
        return Err("unsupported adaptive integration order");
    }
    if !seconds.is_finite()
        || seconds <= 0.
        || !config.relative_tolerance.is_finite()
        || config.relative_tolerance < 0.
        || [
            config.absolute_volume_tolerance_m3,
            config.absolute_protein_tolerance_kg,
            config.min_step_seconds,
            config.max_step_seconds,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
        || config.min_step_seconds > config.max_step_seconds
        || config.max_trials == 0
        || config.max_trials > 1_000_000
    {
        return Err("invalid adaptive exchange configuration");
    }
    let mut next = initial.clone();
    let mut report = AdaptiveExchangeReport {
        exchange: ExchangeReport {
            substeps: 0,
            transferred_volume_m3: vec![0.; network(initial).edges.len()],
            transferred_protein_kg: vec![0.; network(initial).edges.len()],
            volume_drift_m3: 0.,
            protein_drift_kg: 0.,
        },
        accepted_steps: 0,
        rejected_steps: 0,
        max_accepted_error_ratio: 0.,
    };
    let mut elapsed = 0.;
    let mut h = config.max_step_seconds.min(seconds);
    while elapsed < seconds {
        if report.accepted_steps + report.rejected_steps >= config.max_trials {
            return Err("adaptive exchange trial limit");
        }
        h = h.min(seconds - elapsed);
        if h <= 0. || elapsed + h <= elapsed {
            return Err("adaptive exchange step underflow");
        }
        // A failed trial owns only clones, including momentum and FEM history.
        let attempt = (|| {
            let mut coarse = next.clone();
            let whole = advance(&mut coarse, h)?;
            let mut fine = next.clone();
            let first = advance(&mut fine, 0.5 * h)?;
            let second = advance(&mut fine, 0.5 * h)?;
            Ok::<_, &'static str>((coarse, fine, whole, first, second))
        })();
        let (coarse, fine, whole, first, second) = match attempt {
            Ok(states) => states,
            Err(reason)
                if order == 1
                    && matches!(
                        reason,
                        "inertial lymph iteration limit" | "invalid inertial lymph trial inventory"
                    ) =>
            {
                report.rejected_steps += 1;
                if 0.5 * h < config.min_step_seconds {
                    return Err(reason);
                }
                h *= 0.5;
                continue;
            }
            Err(reason) => return Err(reason),
        };
        // Richardson's factor assumes one fixed SSPRK2 step versus exactly
        // two fixed half-steps. Internal positivity subdivisions break that
        // comparison and can alias the two trajectories; reject and shorten.
        if whole.substeps != 1 || first.substeps != 1 || second.substeps != 1 {
            report.rejected_steps += 1;
            if 0.5 * h < config.min_step_seconds {
                return Err("adaptive exchange minimum step");
            }
            h *= 0.5;
            continue;
        }
        let discrepancy = |a: f64, b: f64, scale: f64, absolute: f64| {
            let budget = absolute + config.relative_tolerance * scale;
            if !budget.is_finite() || budget <= 0. {
                f64::INFINITY
            } else {
                ((a - b).abs() / ((1_u32 << order) - 1) as f64) / budget
            }
        };
        let mut error = 0_f64;
        for i in 0..network(&next).volumes.len() {
            error = error.max(discrepancy(
                network(&coarse).volumes[i],
                network(&fine).volumes[i],
                network(&next).volumes[i]
                    .abs()
                    .max(network(&fine).volumes[i].abs()),
                config.absolute_volume_tolerance_m3,
            ));
            error = error.max(discrepancy(
                network(&coarse).proteins[i],
                network(&fine).proteins[i],
                network(&next).proteins[i]
                    .abs()
                    .max(network(&fine).proteins[i].abs()),
                config.absolute_protein_tolerance_kg,
            ));
        }
        for i in 0..network(initial).edges.len() {
            let v = first.transferred_volume_m3[i] + second.transferred_volume_m3[i];
            let m = first.transferred_protein_kg[i] + second.transferred_protein_kg[i];
            error = error.max(discrepancy(
                whole.transferred_volume_m3[i],
                v,
                whole.transferred_volume_m3[i].abs().max(v.abs()),
                config.absolute_volume_tolerance_m3,
            ));
            error = error.max(discrepancy(
                whole.transferred_protein_kg[i],
                m,
                whole.transferred_protein_kg[i].abs().max(m.abs()),
                config.absolute_protein_tolerance_kg,
            ));
        }
        let extra = extra_error(&coarse, &fine)?;
        if !extra.is_finite() || extra < 0. {
            return Err("invalid adaptive coupled error");
        }
        error = error.max(extra);
        if !error.is_finite() {
            return Err("adaptive exchange error overflow");
        }
        if error > 1. {
            report.rejected_steps += 1;
            let reduced = h * (0.9 * error.powf(-1. / f64::from(order + 1))).clamp(0.1, 0.5);
            if reduced < config.min_step_seconds {
                return Err("adaptive exchange minimum step");
            }
            h = reduced;
            continue;
        }
        for i in 0..network(initial).edges.len() {
            report.exchange.transferred_volume_m3[i] +=
                first.transferred_volume_m3[i] + second.transferred_volume_m3[i];
            report.exchange.transferred_protein_kg[i] +=
                first.transferred_protein_kg[i] + second.transferred_protein_kg[i];
        }
        report.exchange.substeps += first.substeps + second.substeps;
        report.accepted_steps += 1;
        report.max_accepted_error_ratio = report.max_accepted_error_ratio.max(error);
        next = fine;
        elapsed += h;
        let growth = if error == 0. {
            2.
        } else {
            (0.9 * error.powf(-1. / f64::from(order + 1))).clamp(0.5, 2.)
        };
        h = (h * growth).clamp(config.min_step_seconds, config.max_step_seconds);
    }
    report.exchange.volume_drift_m3 =
        network(&next).total_volume() - network(initial).total_volume();
    report.exchange.protein_drift_kg =
        network(&next).total_protein() - network(initial).total_protein();
    if report
        .exchange
        .transferred_volume_m3
        .iter()
        .chain(&report.exchange.transferred_protein_kg)
        .chain([
            &report.exchange.volume_drift_m3,
            &report.exchange.protein_drift_kg,
        ])
        .any(|v| !v.is_finite())
    {
        return Err("adaptive exchange report overflow");
    }
    Ok((next, report))
}
